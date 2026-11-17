//! Codes des élèves pour le portail (ADR « identité de l'élève ») : billets d'une classe, code
//! remis à un élève, affiche de classe. Le code n'est lisible qu'au moment où le billet est
//! produit ; la base n'en garde que l'empreinte, le journal ne le voit jamais.

use gesco_core::bibliotheque::portail::BILLETS_PAR_PAGE;
use gesco_core::commun::{Horodatage, Id};
use gesco_core::core::Operation;
use gesco_core::notification::consultation::{self, code_lisible};
use gesco_core::scolarite::{Classe, Eleve};
use gesco_db::depots::portail as depot;
use gesco_db::nouvel_id;
use gesco_db::unite::Durabilite;
use gesco_docs::{Piece, qr};
use serde::{Deserialize, Serialize};

use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;
use crate::session::Session;

/// Une classe dont on veut les codes du portail.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct DemandeCodesClasse {
    /// Classe.
    pub classe_id: Id<Classe>,
}

/// Combien d'élèves d'une classe ont leur code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct CodesClasse {
    /// Élèves inscrits dans la classe.
    pub eleves: u32,
    /// Élèves qui ont un code.
    pub avec_code: u32,
}

/// Billets d'une classe à produire.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct DemandeBilletsPortail {
    /// Classe.
    pub classe_id: Id<Classe>,
    /// Pour tous les élèves (les anciens codes ne marchent plus) ; sinon pour ceux qui n'ont pas
    /// de code.
    pub tous: bool,
}

/// Un élève dont on lit ou remet le code.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct DemandeCodeEleve {
    /// Élève.
    pub eleve_id: Id<Eleve>,
}

/// État du code d'un élève, sur sa fiche.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct CodeEleveEtat {
    /// Date de remise du code actif (`AAAA-MM-JJ`) ; aucune s'il n'a pas de code.
    pub remis_le: Option<String>,
    /// L'élève est inscrit dans une classe de l'année en cours : un code peut lui être remis.
    pub inscrit: bool,
}

/// Billets produits, en PDF (base64) ; le code n'est écrit nulle part ailleurs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct BilletsPortail {
    /// PDF des billets, en base64 ; vide si aucun élève n'en avait besoin.
    pub pdf: String,
    /// Nombre de billets.
    pub billets: u32,
}

/// Affiche de classe, en PDF (base64).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct AffichePortail {
    /// PDF de l'affiche, en base64.
    pub pdf: String,
}

/// Ce qu'il faut composer : données du modèle et pièces jointes (codes QR).
pub struct AComposer {
    /// Données du modèle.
    pub donnees: serde_json::Value,
    /// Pièces (code QR de l'adresse).
    pub pieces: Vec<Piece>,
    /// Nombre de billets.
    pub billets: u32,
}

/// Service des codes du portail.
pub struct ServiceCodesPortail<'a> {
    noyau: &'a Noyau,
}

/// Date courte d'un instant : `JJ/MM/AAAA`.
fn date_courte(instant: Horodatage) -> String {
    let c = instant.civil_utc();
    format!("{:02}/{:02}/{:04}", c.jour, c.mois, c.annee)
}

/// Pièce du code QR de l'adresse, si le portail en a une.
fn qr_adresse(adresse: Option<&str>) -> Resultat<Vec<Piece>> {
    match adresse {
        Some(adresse) => {
            let mut piece = qr::piece(adresse)?;
            piece.nom = "qr-portail.svg".to_owned();
            Ok(vec![piece])
        }
        None => Ok(Vec::new()),
    }
}

impl<'a> ServiceCodesPortail<'a> {
    /// Service sur ce noyau.
    pub fn nouveau(noyau: &'a Noyau) -> Self {
        Self { noyau }
    }

    async fn connexion(&self) -> Resultat<gesco_db::ConnexionEmpruntee> {
        Ok(self
            .noyau
            .base()
            .pool()
            .acquire()
            .await
            .map_err(gesco_db::ErreurBase::from)?)
    }

    /// Combien d'élèves de la classe ont leur code.
    pub async fn classe(
        &self,
        session: &Session,
        demande: &DemandeCodesClasse,
    ) -> Resultat<CodesClasse> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let eleves =
            depot::eleves_de_la_classe(&mut cnx, &demande.classe_id.vers_chaine(), &contexte)
                .await?;
        Ok(CodesClasse {
            eleves: u32::try_from(eleves.len()).unwrap_or(u32::MAX),
            avec_code: u32::try_from(eleves.iter().filter(|e| e.code_remis_le.is_some()).count())
                .unwrap_or(u32::MAX),
        })
    }

    /// État du code d'un élève.
    pub async fn etat(
        &self,
        session: &Session,
        demande: &DemandeCodeEleve,
    ) -> Resultat<CodeEleveEtat> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let id = demande.eleve_id.vers_chaine();
        let inscrit = depot::eleve_billet(&mut cnx, &id, &contexte)
            .await?
            .is_some();
        let remis_le = depot::code_remis_le(&mut cnx, &id, &contexte)
            .await?
            .map(|ms| {
                gesco_core::bibliotheque::portail::jour_de(Horodatage::depuis_millis_epoque(ms))
            });
        Ok(CodeEleveEtat { remis_le, inscrit })
    }

    /// Remet un code neuf à chacun de ces élèves (l'ancien ne marche plus) et prépare leurs
    /// billets. Les empreintes se calculent avant d'ouvrir l'écriture.
    async fn remettre(
        &self,
        session: &Session,
        eleves: Vec<depot::EleveBillet>,
        adresse: Option<&str>,
        action: &str,
        entite: (&str, String),
    ) -> Resultat<AComposer> {
        let mut nouveaux = Vec::with_capacity(eleves.len());
        for eleve in eleves {
            let code = consultation::code_depuis_octets(&crate::alea::octets::<
                { consultation::LONGUEUR_CODE },
            >()?);
            let a_hacher = code.clone();
            let empreinte = tokio::task::spawn_blocking(move || {
                crate::secrets::empreinte_code_eleve(&a_hacher)
            })
            .await
            .map_err(|e| ErreurService::Secret(e.to_string()))??;
            nouveaux.push((eleve, code, empreinte));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        let mut remplaces = 0_u32;
        for (eleve, _, empreinte) in &nouveaux {
            let id = nouvel_id::<()>().vers_chaine();
            if depot::remplacer_code(
                unite.connexion(),
                &id,
                &eleve.eleve_id,
                empreinte,
                &contexte,
            )
            .await?
            .is_some()
            {
                remplaces += 1;
            }
        }
        let billets = u32::try_from(nouveaux.len()).unwrap_or(u32::MAX);
        if billets > 0 {
            unite.auditer(
                entite.0,
                &entite.1,
                Operation::Modification,
                None,
                Some(serde_json::json!({
                    "billets": billets,
                    "codes_remplaces": remplaces,
                    "eleves": nouveaux.iter().map(|(e, _, _)| e.eleve_id.clone()).collect::<Vec<_>>(),
                })),
                action,
            );
        }
        self.noyau.terminer(unite).await?;
        let remis_le = date_courte(contexte.maintenant);
        let billets_json: Vec<serde_json::Value> = nouveaux
            .iter()
            .map(|(e, code, _)| {
                serde_json::json!({
                    "prenoms": e.prenoms,
                    "nom": e.nom,
                    "classe": e.classe,
                    "matricule": e.matricule,
                    "code": code_lisible(code),
                })
            })
            .collect();
        let pieces = qr_adresse(adresse)?;
        Ok(AComposer {
            donnees: serde_json::json!({
                "adresse": adresse.unwrap_or_default(),
                "qr": pieces.first().map(Piece::chemin),
                "remis_le": remis_le,
                "par_page": BILLETS_PAR_PAGE,
                "billets": billets_json,
            }),
            pieces,
            billets,
        })
    }

    /// Billets d'une classe : pour les élèves sans code, ou pour tous (réimprimer remplace les
    /// codes).
    pub async fn billets(
        &self,
        session: &Session,
        demande: &DemandeBilletsPortail,
        adresse: Option<&str>,
    ) -> Resultat<AComposer> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let classe = demande.classe_id.vers_chaine();
        let eleves = {
            let mut cnx = self.connexion().await?;
            depot::eleves_de_la_classe(&mut cnx, &classe, &contexte).await?
        };
        let eleves: Vec<_> = eleves
            .into_iter()
            .filter(|e| demande.tous || e.code_remis_le.is_none())
            .collect();
        self.remettre(
            session,
            eleves,
            adresse,
            "portail.billets",
            ("classe", classe),
        )
        .await
    }

    /// Remet un code neuf à un élève inscrit cette année ; l'ancien ne marche plus.
    pub async fn remettre_un(
        &self,
        session: &Session,
        demande: &DemandeCodeEleve,
        adresse: Option<&str>,
    ) -> Resultat<AComposer> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let id = demande.eleve_id.vers_chaine();
        let eleve = {
            let mut cnx = self.connexion().await?;
            depot::eleve_billet(&mut cnx, &id, &contexte).await?
        };
        let Some(eleve) = eleve else {
            return Err(ErreurService::Regle {
                cle: "portail.eleve_non_inscrit",
            });
        };
        self.remettre(
            session,
            vec![eleve],
            adresse,
            "portail.code_remettre",
            ("eleve", id),
        )
        .await
    }

    /// Données de l'affiche de classe : l'adresse en grand, son code QR, trois étapes.
    pub fn affiche(adresse: &str) -> Resultat<AComposer> {
        let pieces = qr_adresse(Some(adresse))?;
        Ok(AComposer {
            donnees: serde_json::json!({
                "adresse": adresse,
                "qr": pieces.first().map(Piece::chemin),
            }),
            pieces,
            billets: 0,
        })
    }
}
