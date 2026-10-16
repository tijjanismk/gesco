//! Élèves et responsables (SPEC_PHASE_2 §4) : fiche, matricule interne, téléphones canoniques,
//! photo, responsables réutilisés d'une fratrie à l'autre.

use gesco_core::commun::{DateCivile, Id};
use gesco_core::core::{MachineAEtats, Operation, TypeDocumentNumerote, TypeEvenement};
use gesco_core::scolarite::matricule::{self, ReglesMatricule, parametres as reglages};
use gesco_core::scolarite::personne::{matricule_national, millesime, texte_propre};
use gesco_core::scolarite::recherche::{self, IdentiteComparee, Recherche, doublon_probable};
use gesco_core::scolarite::{
    Eleve, NatureActe, Personne, Qualites, Responsable, Sexe, StatutInscription, telephone,
};
use gesco_db::depots::{academique, annee_scolaire, inscriptions, parametre, personnes};
use gesco_db::unite::Durabilite;
use gesco_db::{Contexte, UniteDeTravail, nouvel_id};
use serde::{Deserialize, Serialize};

use crate::effets::Effet;
use crate::erreur::{ErreurService, Resultat};
use crate::image;
use crate::noyau::Noyau;
use crate::numerotation;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Taille maximale d'une photo : 100 Ko (T-10).
const PHOTO_MAX: usize = 100 * 1024;

/// Résultats rendus par une recherche.
const RESULTATS_MAX: usize = 200;

/// Identité saisie à l'écran ; le noyau la nettoie et la vérifie (R-02).
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct SaisieIdentite {
    /// Nom de famille.
    pub nom: String,
    /// Prénoms.
    pub prenoms: String,
    /// Sexe.
    pub sexe: Option<Sexe>,
    /// Date de naissance `AAAA-MM-JJ`, vide si inconnue.
    pub date_naissance: String,
    /// Date approximative ou issue d'un jugement supplétif.
    pub date_naissance_estimee: bool,
    /// Lieu de naissance.
    pub lieu_naissance: String,
    /// Nationalité.
    pub nationalite: String,
    /// Adresse.
    pub adresse: String,
    /// Téléphones, sous n'importe quelle forme.
    pub telephones: Vec<String>,
}

/// Fiche élève saisie.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct SaisieEleve {
    /// Identité.
    pub identite: SaisieIdentite,
    /// Matricule interne, quand l'école le saisit elle-même ; ignoré quand GESCO le génère.
    pub matricule_interne: String,
    /// Matricule national, vide au fondamental.
    pub matricule_national: String,
    /// Pièce d'état civil.
    pub acte_nature: Option<NatureActe>,
    /// Numéro de l'acte ou du jugement.
    pub acte_numero: String,
    /// Date de l'acte `AAAA-MM-JJ`, vide si inconnue.
    pub acte_date: String,
    /// École d'origine.
    pub ecole_origine: String,
}

/// Responsable saisi.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct SaisieResponsable {
    /// Identité.
    pub identite: SaisieIdentite,
    /// Profession.
    pub profession: String,
    /// Employeur.
    pub employeur: String,
}

/// Responsable d'un élève, avec les qualités de leur lien.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct ResponsableDeLEleve {
    /// Responsable.
    pub responsable: Responsable,
    /// Identité.
    pub identite: Personne,
    /// Qualités du lien.
    pub qualites: Qualites,
}

/// Fiche élève complète.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct FicheEleve {
    /// Élève.
    pub eleve: Eleve,
    /// Identité.
    pub identite: Personne,
    /// Photo en URL de données, pour l'affichage.
    pub photo: Option<String>,
    /// Responsables, dans l'ordre d'appel.
    pub responsables: Vec<ResponsableDeLEleve>,
    /// Inscriptions, la plus récente d'abord.
    pub inscriptions: Vec<crate::inscriptions::InscriptionAffichee>,
}

/// Ligne d'une liste d'élèves : le nécessaire pour reconnaître l'élève (R-24).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct EleveTrouve {
    /// Élève.
    pub id: Id<Eleve>,
    /// Matricule interne.
    pub matricule_interne: String,
    /// Matricule national.
    pub matricule_national: Option<String>,
    /// Nom.
    pub nom: String,
    /// Prénoms.
    pub prenoms: String,
    /// Sexe.
    pub sexe: Option<Sexe>,
    /// Date de naissance.
    pub date_naissance: Option<DateCivile>,
    /// Classe de l'année en cours ; vide si l'élève n'y est pas inscrit.
    pub classe: String,
    /// État de cette inscription, nom persisté ; vide sans inscription.
    pub statut_inscription: String,
}

/// Texte cherché.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeRecherche {
    /// Nom, prénoms, matricule ou téléphone.
    pub texte: String,
}

/// Élève désigné.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeEleve {
    /// Élève.
    pub id: Id<Eleve>,
}

/// Rectification d'une fiche élève.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeModificationEleve {
    /// Élève.
    pub id: Id<Eleve>,
    /// Fiche saisie.
    pub saisie: SaisieEleve,
}

/// Photo d'un élève, ou son retrait.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandePhoto {
    /// Élève.
    pub id: Id<Eleve>,
    /// Image PNG ou JPEG en base64 ; absente pour retirer la photo.
    pub contenu_base64: Option<String>,
}

/// Lien d'un responsable existant ou nouveau à un élève.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeRattachement {
    /// Élève.
    pub eleve_id: Id<Eleve>,
    /// Responsable déjà connu, retrouvé par la recherche.
    pub responsable_id: Option<Id<Responsable>>,
    /// Nouveau responsable, si aucun n'a été retrouvé.
    pub saisie: Option<SaisieResponsable>,
    /// Qualités du lien.
    pub qualites: Qualites,
}

/// Rectification d'un responsable et de son lien.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeModificationResponsable {
    /// Élève.
    pub eleve_id: Id<Eleve>,
    /// Responsable.
    pub responsable_id: Id<Responsable>,
    /// Responsable saisi.
    pub saisie: SaisieResponsable,
    /// Qualités du lien.
    pub qualites: Qualites,
}

/// Lien désigné.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeDetachement {
    /// Élève.
    pub eleve_id: Id<Eleve>,
    /// Responsable.
    pub responsable_id: Id<Responsable>,
}

/// Réglages de la scolarité utiles aux écrans de saisie.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct ReglagesScolarite {
    /// Vrai : la secrétaire saisit le matricule interne.
    pub matricule_manuel: bool,
}

/// Élève déjà lié à un responsable trouvé.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct EnfantLie {
    /// Élève.
    pub id: Id<Eleve>,
    /// Nom complet.
    pub nom: String,
}

/// Identité en cours de saisie, comparée aux fiches existantes.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeDoublons {
    /// Nom.
    pub nom: String,
    /// Prénoms.
    pub prenoms: String,
    /// Date de naissance `AAAA-MM-JJ`.
    pub date_naissance: String,
    /// Fiche à écarter : celle que l'on modifie.
    pub sauf: Option<Id<Eleve>>,
}

/// Responsable trouvé, avec les élèves auxquels il est déjà lié.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct ResponsableTrouve {
    /// Responsable.
    pub responsable: Responsable,
    /// Identité.
    pub identite: Personne,
    /// Élèves déjà liés : la fratrie.
    pub enfants: Vec<EnfantLie>,
}

fn date_saisie(texte: &str, cle: &'static str) -> Resultat<Option<DateCivile>> {
    if texte.trim().is_empty() {
        return Ok(None);
    }
    DateCivile::analyser(texte).map(Some).ok_or(regle(cle))
}

/// Identité nettoyée : textes bornés, dates lues, téléphones canoniques.
pub(crate) fn identite(saisie: &SaisieIdentite, base: Personne) -> Resultat<Personne> {
    let texte = |t: &str| texte_propre(t, "personne.texte_trop_long").map_err(regle);
    Ok(Personne {
        nom: texte(&saisie.nom)?,
        prenoms: texte(&saisie.prenoms)?,
        sexe: saisie.sexe,
        date_naissance: date_saisie(&saisie.date_naissance, "personne.date_invalide")?,
        date_naissance_estimee: saisie.date_naissance_estimee,
        lieu_naissance: texte(&saisie.lieu_naissance)?,
        nationalite: texte(&saisie.nationalite)?,
        adresse: texte(&saisie.adresse)?,
        telephones: telephone::normaliser_liste(&saisie.telephones).map_err(regle)?,
        ..base
    })
}

pub(crate) fn nouvelle_personne(session: &Session) -> Personne {
    Personne {
        id: nouvel_id(),
        etablissement_id: session.etablissement_id,
        nom: String::new(),
        prenoms: String::new(),
        sexe: None,
        date_naissance: None,
        date_naissance_estimee: false,
        lieu_naissance: String::new(),
        nationalite: String::new(),
        adresse: String::new(),
        telephones: Vec::new(),
    }
}

pub(crate) fn champs_eleve(saisie: &SaisieEleve, base: Eleve) -> Resultat<Eleve> {
    Ok(Eleve {
        matricule_national: matricule_national(&saisie.matricule_national).map_err(regle)?,
        acte_nature: saisie.acte_nature,
        acte_numero: texte_propre(&saisie.acte_numero, "personne.texte_trop_long")
            .map_err(regle)?,
        acte_date: date_saisie(&saisie.acte_date, "eleve.acte_date_invalide")?,
        ecole_origine: texte_propre(&saisie.ecole_origine, "personne.texte_trop_long")
            .map_err(regle)?,
        ..base
    })
}

fn trouve(eleve: Eleve, identite: Personne) -> EleveTrouve {
    EleveTrouve {
        id: eleve.id,
        matricule_interne: eleve.matricule_interne,
        matricule_national: eleve.matricule_national,
        nom: identite.nom,
        prenoms: identite.prenoms,
        sexe: identite.sexe,
        date_naissance: identite.date_naissance,
        classe: String::new(),
        statut_inscription: String::new(),
    }
}

fn verifier_qualites(qualites: &Qualites) -> Resultat<()> {
    if qualites.ordre_priorite < 1 {
        return Err(regle("responsable.ordre_invalide"));
    }
    Ok(())
}

/// Règles lues une fois avant d'insérer des élèves.
pub(crate) struct Insertion {
    /// L'école saisit elle-même le matricule interne.
    pub manuel: bool,
    /// Règles de génération.
    pub regles: ReglesMatricule,
    /// Millésime de l'année courante, s'il y en a une.
    pub millesime: Option<String>,
}

/// Service des élèves et des responsables.
pub struct ServiceEleves<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceEleves<'n> {
    /// Service adossé au noyau.
    pub const fn nouveau(noyau: &'n Noyau) -> Self {
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

    /// Fiche complète d'un élève.
    pub async fn fiche(&self, session: &Session, id: Id<Eleve>) -> Resultat<FicheEleve> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let (eleve, identite) = personnes::eleve(&mut cnx, id, &contexte).await?;
        let photo = personnes::photo(&mut cnx, identite.id, &contexte)
            .await?
            .map(|(octets, format)| image::url_de_donnees(&octets, &format));
        let mut responsables = Vec::new();
        for (_, responsable_id, qualites) in
            personnes::liens_de_l_eleve(&mut cnx, id, &contexte).await?
        {
            let (responsable, identite) =
                personnes::responsable(&mut cnx, responsable_id, &contexte).await?;
            responsables.push(ResponsableDeLEleve {
                responsable,
                identite,
                qualites,
            });
        }
        drop(cnx);
        Ok(FicheEleve {
            inscriptions: crate::inscriptions::ServiceInscriptions::nouveau(self.noyau)
                .de_l_eleve(session, id)
                .await?,
            eleve,
            identite,
            photo,
            responsables,
        })
    }

    /// Élèves dont le nom et les prénoms approchent la recherche (SPEC_PHASE_2 §6.2), du plus
    /// sûr au plus lâche ; un matricule saisi en entier trouve sa fiche en tête.
    pub async fn rechercher(&self, session: &Session, texte: &str) -> Resultat<Vec<EleveTrouve>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let cherche = Recherche::nouvelle(texte);
        let matricule = recherche::normaliser(texte);
        let porte = |eleve: &Eleve| {
            !matricule.is_empty()
                && (recherche::normaliser(&eleve.matricule_interne) == matricule
                    || eleve
                        .matricule_national
                        .as_deref()
                        .is_some_and(|m| recherche::normaliser(m) == matricule))
        };
        // Le matricule d'une fiche absorbée par une fusion mène à la fiche conservée (§8).
        let conservees: Vec<Id<Eleve>> = personnes::eleves_absorbes(&mut cnx, &contexte)
            .await?
            .iter()
            .filter(|(eleve, _)| porte(eleve))
            .filter_map(|(eleve, _)| eleve.fusionne_dans)
            .collect();
        let mut trouves: Vec<((u8, u32), Eleve, Personne)> = personnes::eleves(&mut cnx, &contexte)
            .await?
            .into_iter()
            .filter_map(|(eleve, identite)| {
                let par_matricule = porte(&eleve) || conservees.contains(&eleve.id);
                let rang = if par_matricule {
                    Some((0, 0))
                } else {
                    cherche.rang(&identite.nom_complet())
                };
                rang.map(|r| (r, eleve, identite))
            })
            .collect();
        trouves.sort_by_key(|t| t.0);
        let mut liste: Vec<EleveTrouve> = trouves
            .into_iter()
            .take(RESULTATS_MAX)
            .map(|(_, eleve, identite)| trouve(eleve, identite))
            .collect();
        // La classe de l'année en cours, pour situer l'élève d'un coup d'œil.
        if let Some(annee) = annee_scolaire::courante(&mut cnx, &contexte).await? {
            let classes = academique::classes(&mut cnx, annee.id, &contexte).await?;
            let inscriptions = inscriptions::de_l_annee(&mut cnx, annee.id, &contexte).await?;
            for l in &mut liste {
                // L'inscription en cours prime sur une inscription close (transfert, abandon).
                let siennes = inscriptions.iter().filter(|i| i.eleve_id == l.id);
                let i = siennes.clone().find(|i| {
                    matches!(
                        i.statut,
                        StatutInscription::Validee | StatutInscription::Brouillon
                    )
                });
                if let Some(i) = i.or_else(|| siennes.clone().next_back()) {
                    l.classe = classes
                        .iter()
                        .find(|c| c.id == i.classe_id)
                        .map(|c| c.libelle.clone())
                        .unwrap_or_default();
                    i.statut.nom().clone_into(&mut l.statut_inscription);
                }
            }
        }
        Ok(liste)
    }

    /// Fiches proches d'une identité en cours de saisie (SPEC_PHASE_2 §6.3) : proposées, jamais
    /// fusionnées d'elles-mêmes.
    pub async fn doublons_probables(
        &self,
        session: &Session,
        demande: &DemandeDoublons,
    ) -> Resultat<Vec<EleveTrouve>> {
        let date = DateCivile::analyser(&demande.date_naissance);
        let saisie = IdentiteComparee {
            nom: &demande.nom,
            prenoms: &demande.prenoms,
            date_naissance: date,
        };
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        Ok(personnes::eleves(&mut cnx, &contexte)
            .await?
            .into_iter()
            .filter(|(eleve, identite)| {
                Some(eleve.id) != demande.sauf
                    && doublon_probable(
                        saisie,
                        IdentiteComparee {
                            nom: &identite.nom,
                            prenoms: &identite.prenoms,
                            date_naissance: identite.date_naissance,
                        },
                    )
            })
            .take(20)
            .map(|(eleve, identite)| trouve(eleve, identite))
            .collect())
    }

    async fn regles_matricule(&self, session: &Session) -> Resultat<(bool, ReglesMatricule)> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let defaut = ReglesMatricule::default();
        let manuel =
            parametre::booleen_param(&mut cnx, reglages::SAISIE_MANUELLE, false, &contexte).await?;
        Ok((
            manuel,
            ReglesMatricule {
                prefixe: parametre::texte(&mut cnx, reglages::PREFIXE, "", &contexte).await?,
                chiffres_annee: parametre::entier(
                    &mut cnx,
                    reglages::CHIFFRES_ANNEE,
                    defaut.chiffres_annee,
                    &contexte,
                )
                .await?,
                chiffres: parametre::entier(
                    &mut cnx,
                    reglages::CHIFFRES,
                    defaut.chiffres,
                    &contexte,
                )
                .await?,
                remise_annuelle: parametre::booleen_param(
                    &mut cnx,
                    reglages::REMISE_ANNUELLE,
                    defaut.remise_annuelle,
                    &contexte,
                )
                .await?,
            },
        ))
    }

    /// Réglages utiles aux écrans de saisie.
    pub async fn reglages(&self, session: &Session) -> Resultat<ReglagesScolarite> {
        Ok(ReglagesScolarite {
            matricule_manuel: self.regles_matricule(session).await?.0,
        })
    }

    async fn verifier_matricule_saisi(
        &self,
        session: &Session,
        matricule: &str,
        sauf: Option<Id<Eleve>>,
    ) -> Resultat<()> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        if personnes::matricule_interne_pris(&mut cnx, matricule, sauf, &contexte).await? {
            return Err(regle("eleve.matricule_pris"));
        }
        Ok(())
    }

    async fn verifier_matricule_national(&self, session: &Session, eleve: &Eleve) -> Resultat<()> {
        let Some(matricule) = &eleve.matricule_national else {
            return Ok(());
        };
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        if personnes::matricule_national_pris(&mut cnx, matricule, Some(eleve.id), &contexte)
            .await?
        {
            return Err(regle("eleve.matricule_national_pris"));
        }
        Ok(())
    }

    /// Règles de matricule et millésime de l'année courante, lus une fois avant d'insérer un
    /// ou plusieurs élèves.
    pub(crate) async fn preparer_insertion(&self, session: &Session) -> Resultat<Insertion> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let (manuel, regles) = self.regles_matricule(session).await?;
        let mut cnx = self.connexion().await?;
        let millesime = annee_scolaire::courante(&mut cnx, &contexte)
            .await?
            .and_then(|a| millesime(&a.libelle).map(str::to_owned));
        Ok(Insertion {
            manuel,
            regles,
            millesime,
        })
    }

    /// Fiche élève vierge, à compléter par la saisie.
    pub(crate) fn eleve_vierge(session: &Session, personne: &Personne) -> Eleve {
        Eleve {
            id: nouvel_id(),
            etablissement_id: session.etablissement_id,
            personne_id: personne.id,
            matricule_interne: String::new(),
            matricule_national: None,
            acte_nature: None,
            acte_numero: String::new(),
            acte_date: None,
            ecole_origine: String::new(),
            fusionne_dans: None,
        }
    }

    /// Insère une personne et sa fiche élève dans une unité de travail déjà ouverte : matricule
    /// saisi (déjà vérifié) ou généré selon les règles, sans trou dans la séquence ; audit et
    /// événement. L'import (§10) en enchaîne des centaines dans une seule transaction.
    pub(crate) async fn inserer_dans(
        unite: &mut UniteDeTravail<Effet>,
        contexte: &Contexte,
        insertion: &Insertion,
        personne: &Personne,
        brouillon: Eleve,
        saisi: Option<String>,
        origine: &'static str,
    ) -> Resultat<Eleve> {
        let matricule_interne = match saisi {
            Some(m) => m,
            None => {
                let millesime = insertion
                    .millesime
                    .as_deref()
                    .ok_or(regle("eleve.annee_absente"))?;
                let exercice = insertion.regles.exercice(millesime);
                // Un matricule saisi à la main, ou produit avant un changement de règles, peut
                // déjà exister : le numéro suivant est pris.
                let mut tentatives = 0;
                loop {
                    let compteur = numerotation::attribuer_compteur(
                        unite,
                        TypeDocumentNumerote::MatriculeEleve,
                        &exercice,
                    )
                    .await?;
                    let m = insertion.regles.formater(millesime, compteur);
                    if !personnes::matricule_interne_pris(unite.connexion(), &m, None, contexte)
                        .await?
                    {
                        break m;
                    }
                    tentatives += 1;
                    if tentatives >= 1000 {
                        return Err(regle("eleve.matricule_pris"));
                    }
                }
            }
        };
        let eleve = Eleve {
            matricule_interne,
            ..brouillon
        };
        personnes::inserer_personne(unite.connexion(), personne, contexte).await?;
        personnes::inserer_eleve(unite.connexion(), &eleve, contexte).await?;
        let id = eleve.id.vers_chaine();
        unite.auditer(
            "eleve",
            &id,
            Operation::Creation,
            None,
            Some(serde_json::json!({ "eleve": eleve, "identite": personne })),
            origine,
        );
        unite.emettre(
            TypeEvenement::FicheEleveCreee,
            "eleve",
            &id,
            serde_json::json!({ "matricule_interne": eleve.matricule_interne }),
        );
        Ok(eleve)
    }

    /// Crée une fiche élève. Son matricule interne est saisi par l'école ou généré selon ses
    /// règles, sans trou dans la séquence (T-01 révisé).
    pub async fn creer(&self, session: &Session, saisie: &SaisieEleve) -> Resultat<FicheEleve> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let personne = identite(&saisie.identite, nouvelle_personne(session))?;
        personne
            .verifier_eleve(contexte.maintenant)
            .map_err(regle)?;
        let insertion = self.preparer_insertion(session).await?;
        let saisi = if insertion.manuel {
            let m = matricule::saisi(&saisie.matricule_interne).map_err(regle)?;
            self.verifier_matricule_saisi(session, &m, None).await?;
            Some(m)
        } else {
            None
        };
        let brouillon = champs_eleve(saisie, Self::eleve_vierge(session, &personne))?;
        self.verifier_matricule_national(session, &brouillon)
            .await?;
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let eleve = Self::inserer_dans(
            &mut unite,
            &contexte,
            &insertion,
            &personne,
            brouillon,
            saisi,
            "eleves.creer",
        )
        .await?;
        self.noyau.terminer(unite).await?;
        self.fiche(session, eleve.id).await
    }

    async fn vivant(&self, session: &Session, id: Id<Eleve>) -> Resultat<(Eleve, Personne)> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let fiche = personnes::eleve(&mut cnx, id, &contexte).await?;
        if fiche.0.fusionne_dans.is_some() {
            return Err(regle("eleve.absorbe"));
        }
        Ok(fiche)
    }

    /// Rectifie la fiche d'un élève. Le matricule interne ne change que si l'école le saisit
    /// elle-même : une faute de frappe se corrige ; un matricule généré ne bouge jamais.
    pub async fn modifier(
        &self,
        session: &Session,
        id: Id<Eleve>,
        saisie: &SaisieEleve,
    ) -> Resultat<FicheEleve> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let (eleve_avant, personne_avant) = self.vivant(session, id).await?;
        let personne = identite(&saisie.identite, personne_avant.clone())?;
        personne
            .verifier_eleve(contexte.maintenant)
            .map_err(regle)?;
        let mut eleve = champs_eleve(saisie, eleve_avant.clone())?;
        if self.regles_matricule(session).await?.0 {
            let m = matricule::saisi(&saisie.matricule_interne).map_err(regle)?;
            if m != eleve_avant.matricule_interne {
                self.verifier_matricule_saisi(session, &m, Some(id)).await?;
                eleve.matricule_interne = m;
            }
        }
        self.verifier_matricule_national(session, &eleve).await?;
        let avant = serde_json::json!({ "eleve": eleve_avant, "identite": personne_avant });
        let apres = serde_json::json!({ "eleve": eleve, "identite": personne });
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        personnes::modifier_personne(unite.connexion(), &personne, &contexte).await?;
        personnes::modifier_eleve(unite.connexion(), &eleve, &contexte).await?;
        if eleve.matricule_interne != eleve_avant.matricule_interne {
            personnes::renumeroter(unite.connexion(), id, &eleve.matricule_interne, &contexte)
                .await?;
        }
        let id_texte = id.vers_chaine();
        let changes = champs_changes(&avant, &apres);
        unite.auditer(
            "eleve",
            &id_texte,
            Operation::Modification,
            Some(avant),
            Some(apres),
            "eleves.modifier",
        );
        unite.emettre(
            TypeEvenement::FicheEleveRectifiee,
            "eleve",
            &id_texte,
            serde_json::json!({ "matricule_interne": eleve.matricule_interne, "champs": changes }),
        );
        self.noyau.terminer(unite).await?;
        self.fiche(session, id).await
    }

    /// Pose une photo PNG ou JPEG de 100 Ko au plus, ou la retire (`None`).
    pub async fn definir_photo(
        &self,
        session: &Session,
        id: Id<Eleve>,
        contenu_base64: Option<&str>,
    ) -> Resultat<FicheEleve> {
        let photo = contenu_base64
            .map(|texte| {
                image::decoder(
                    texte,
                    PHOTO_MAX,
                    &image::Refus {
                        illisible: "eleve.photo_illisible",
                        trop_lourde: "eleve.photo_trop_lourde",
                        format: "eleve.photo_format",
                    },
                )
            })
            .transpose()?;
        let (_, personne) = self.vivant(session, id).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let avant = personnes::photo(unite.connexion(), personne.id, &contexte)
            .await?
            .map(|(octets, format)| image::trace(&octets, &format));
        personnes::definir_photo(
            unite.connexion(),
            personne.id,
            photo.as_ref().map(|i| (i.octets.as_slice(), i.format)),
            &contexte,
        )
        .await?;
        unite.auditer(
            "eleve_photo",
            &id.vers_chaine(),
            Operation::Modification,
            avant,
            photo.as_ref().map(|i| image::trace(&i.octets, i.format)),
            "eleves.photo",
        );
        self.noyau.terminer(unite).await?;
        self.fiche(session, id).await
    }

    /// Responsables dont un téléphone ou le nom correspond : une saisie qui se lit comme un
    /// numéro cherche par téléphone, sinon par nom.
    pub async fn rechercher_responsables(
        &self,
        session: &Session,
        texte: &str,
    ) -> Resultat<Vec<ResponsableTrouve>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let numero = telephone::normaliser(texte).ok();
        let cherche = Recherche::nouvelle(texte);
        if numero.is_none() && cherche.est_vide() {
            return Ok(Vec::new());
        }
        let eleves = personnes::eleves(&mut cnx, &contexte).await?;
        let liens = personnes::liens(&mut cnx, &contexte).await?;
        let mut trouves: Vec<((u8, u32), Responsable, Personne)> =
            personnes::responsables(&mut cnx, &contexte)
                .await?
                .into_iter()
                .filter_map(|(responsable, identite)| {
                    let rang = match &numero {
                        Some(n) => identite.telephones.contains(n).then_some((0, 0)),
                        None => cherche.rang(&identite.nom_complet()),
                    };
                    rang.map(|r| (r, responsable, identite))
                })
                .collect();
        trouves.sort_by_key(|t| t.0);
        Ok(trouves
            .into_iter()
            .take(20)
            .map(|(_, responsable, identite)| ResponsableTrouve {
                enfants: liens
                    .iter()
                    .filter(|(_, r, _)| *r == responsable.id)
                    .filter_map(|(e, _, _)| eleves.iter().find(|(x, _)| x.id == *e))
                    .map(|(eleve, p)| EnfantLie {
                        id: eleve.id,
                        nom: p.nom_complet(),
                    })
                    .collect(),
                responsable,
                identite,
            })
            .collect())
    }

    fn responsable_saisi(
        saisie: &SaisieResponsable,
        responsable: Responsable,
        personne: Personne,
    ) -> Resultat<(Responsable, Personne)> {
        let personne = identite(&saisie.identite, personne)?;
        personne.verifier_responsable().map_err(regle)?;
        let texte = |t: &str| texte_propre(t, "personne.texte_trop_long").map_err(regle);
        Ok((
            Responsable {
                profession: texte(&saisie.profession)?,
                employeur: texte(&saisie.employeur)?,
                ..responsable
            },
            personne,
        ))
    }

    /// Lie à l'élève un responsable existant (`responsable_id`) ou nouveau (`saisie`).
    pub async fn rattacher(
        &self,
        session: &Session,
        eleve_id: Id<Eleve>,
        responsable_id: Option<Id<Responsable>>,
        saisie: Option<&SaisieResponsable>,
        qualites: Qualites,
    ) -> Resultat<FicheEleve> {
        verifier_qualites(&qualites)?;
        self.vivant(session, eleve_id).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let nouveau = match (responsable_id, saisie) {
            (Some(id), None) => {
                let mut cnx = self.connexion().await?;
                personnes::responsable(&mut cnx, id, &contexte).await?;
                if personnes::liens_de_l_eleve(&mut cnx, eleve_id, &contexte)
                    .await?
                    .iter()
                    .any(|(_, r, _)| *r == id)
                {
                    return Err(regle("responsable.deja_lie"));
                }
                None
            }
            (None, Some(saisie)) => {
                let personne = nouvelle_personne(session);
                let (responsable, personne) = Self::responsable_saisi(
                    saisie,
                    Responsable {
                        id: nouvel_id(),
                        etablissement_id: session.etablissement_id,
                        personne_id: personne.id,
                        profession: String::new(),
                        employeur: String::new(),
                    },
                    personne,
                )?;
                Some((
                    responsable,
                    Personne {
                        sexe: qualites.lien.sexe().or(personne.sexe),
                        ..personne
                    },
                ))
            }
            _ => return Err(regle("responsable.choix_ambigu")),
        };
        let responsable_id = nouveau
            .as_ref()
            .map(|(r, _)| r.id)
            .or(responsable_id)
            .ok_or(regle("responsable.choix_ambigu"))?;
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        if let Some((responsable, personne)) = &nouveau {
            personnes::inserer_personne(unite.connexion(), personne, &contexte).await?;
            personnes::inserer_responsable(unite.connexion(), responsable, &contexte).await?;
            unite.auditer(
                "responsable",
                &responsable.id.vers_chaine(),
                Operation::Creation,
                None,
                Some(serde_json::json!({ "responsable": responsable, "identite": personne })),
                "eleves.rattacher",
            );
        }
        personnes::lier(
            unite.connexion(),
            eleve_id,
            responsable_id,
            &qualites,
            &contexte,
        )
        .await?;
        let cle_lien = format!(
            "{}:{}",
            eleve_id.vers_chaine(),
            responsable_id.vers_chaine()
        );
        unite.auditer(
            "eleve_responsable",
            &cle_lien,
            Operation::Creation,
            None,
            Some(serde_json::to_value(qualites)?),
            "eleves.rattacher",
        );
        unite.emettre(
            TypeEvenement::ResponsableRattache,
            "eleve",
            &eleve_id.vers_chaine(),
            serde_json::json!({ "responsable_id": responsable_id, "qualites": qualites }),
        );
        self.noyau.terminer(unite).await?;
        self.fiche(session, eleve_id).await
    }

    /// Rectifie l'identité d'un responsable et les qualités de son lien avec l'élève.
    pub async fn modifier_responsable(
        &self,
        session: &Session,
        eleve_id: Id<Eleve>,
        responsable_id: Id<Responsable>,
        saisie: &SaisieResponsable,
        qualites: Qualites,
    ) -> Resultat<FicheEleve> {
        verifier_qualites(&qualites)?;
        self.vivant(session, eleve_id).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let (avant_responsable, avant_personne, avant_qualites) = {
            let mut cnx = self.connexion().await?;
            let (r, p) = personnes::responsable(&mut cnx, responsable_id, &contexte).await?;
            let q = personnes::liens_de_l_eleve(&mut cnx, eleve_id, &contexte)
                .await?
                .into_iter()
                .find(|(_, id, _)| *id == responsable_id)
                .map(|(_, _, q)| q)
                .ok_or(regle("responsable.non_lie"))?;
            (r, p, q)
        };
        let (responsable, personne) =
            Self::responsable_saisi(saisie, avant_responsable.clone(), avant_personne.clone())?;
        let personne = Personne {
            sexe: qualites.lien.sexe().or(personne.sexe),
            ..personne
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        personnes::modifier_personne(unite.connexion(), &personne, &contexte).await?;
        personnes::modifier_responsable(unite.connexion(), &responsable, &contexte).await?;
        personnes::modifier_lien(
            unite.connexion(),
            eleve_id,
            responsable_id,
            &qualites,
            &contexte,
        )
        .await?;
        unite.auditer(
            "responsable",
            &responsable_id.vers_chaine(),
            Operation::Modification,
            Some(
                serde_json::json!({ "responsable": avant_responsable, "identite": avant_personne }),
            ),
            Some(serde_json::json!({ "responsable": responsable, "identite": personne })),
            "eleves.modifier_responsable",
        );
        unite.auditer(
            "eleve_responsable",
            &format!(
                "{}:{}",
                eleve_id.vers_chaine(),
                responsable_id.vers_chaine()
            ),
            Operation::Modification,
            Some(serde_json::to_value(avant_qualites)?),
            Some(serde_json::to_value(qualites)?),
            "eleves.modifier_responsable",
        );
        self.noyau.terminer(unite).await?;
        self.fiche(session, eleve_id).await
    }

    /// Retire le lien entre l'élève et un responsable ; le responsable reste pour la fratrie.
    pub async fn detacher(
        &self,
        session: &Session,
        eleve_id: Id<Eleve>,
        responsable_id: Id<Responsable>,
    ) -> Resultat<FicheEleve> {
        self.vivant(session, eleve_id).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let qualites = {
            let mut cnx = self.connexion().await?;
            personnes::liens_de_l_eleve(&mut cnx, eleve_id, &contexte)
                .await?
                .into_iter()
                .find(|(_, id, _)| *id == responsable_id)
                .map(|(_, _, q)| q)
                .ok_or(regle("responsable.non_lie"))?
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        personnes::detacher(unite.connexion(), eleve_id, responsable_id, &contexte).await?;
        unite.auditer(
            "eleve_responsable",
            &format!(
                "{}:{}",
                eleve_id.vers_chaine(),
                responsable_id.vers_chaine()
            ),
            Operation::Suppression,
            Some(serde_json::to_value(qualites)?),
            None,
            "eleves.detacher",
        );
        unite.emettre(
            TypeEvenement::ResponsableDetache,
            "eleve",
            &eleve_id.vers_chaine(),
            serde_json::json!({ "responsable_id": responsable_id }),
        );
        self.noyau.terminer(unite).await?;
        self.fiche(session, eleve_id).await
    }
}

/// Chemins des champs changés entre deux fiches (`identite.nom`, `eleve.matricule_national`).
fn champs_changes(avant: &serde_json::Value, apres: &serde_json::Value) -> Vec<String> {
    let mut changes = Vec::new();
    for partie in ["eleve", "identite"] {
        if let (Some(a), Some(b)) = (
            avant.get(partie).and_then(serde_json::Value::as_object),
            apres.get(partie).and_then(serde_json::Value::as_object),
        ) {
            changes.extend(
                b.iter()
                    .filter(|(cle, valeur)| a.get(*cle) != Some(*valeur))
                    .map(|(cle, _)| format!("{partie}.{cle}")),
            );
        }
    }
    changes
}
