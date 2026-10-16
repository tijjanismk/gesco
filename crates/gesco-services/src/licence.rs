//! Licence du poste (spécification §13, blueprint §26.5) : installation d'un fichier signé et
//! évaluation de son état. Aucune situation de licence ne bloque l'édition Essentielle ni l'accès
//! aux données ; l'état sert l'affichage et, plus tard, l'activation des modules connectés.

use gesco_core::commun::{Horodatage, Id};
use gesco_core::core::licence::{self, ErreurLicence, EtatLicence};
use gesco_core::core::{Edition, Etablissement, Licence, Operation, StatutLicence, TypeEvenement};
use gesco_db::depots::licence as depot;
use gesco_db::nouvel_id;
use gesco_db::unite::Durabilite;
use serde::{Deserialize, Serialize};

use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;
use crate::session::Session;

/// Clé publique de l'éditeur, en hexadécimal. La clé privée correspondante ne quitte jamais le
/// poste de l'éditeur (outil `gesco-licence`).
const CLE_PUBLIQUE_EDITEUR_HEX: &str = include_str!("../cle-publique-licence.txt");

/// Clé publique de l'éditeur, contre laquelle toute licence est vérifiée.
pub fn cle_publique_editeur() -> Resultat<[u8; 32]> {
    hex::decode(CLE_PUBLIQUE_EDITEUR_HEX.trim())
        .ok()
        .and_then(|o| o.try_into().ok())
        .ok_or(ErreurService::Licence(ErreurLicence::CleInvalide))
}

/// Licence telle que l'écran Licence l'affiche.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct EtatLicenceAffiche {
    /// Empreinte de ce poste, à communiquer à l'éditeur pour obtenir une licence.
    pub empreinte_poste: String,
    /// Situation et modules connectés.
    pub etat: EtatLicence,
    /// Contenu de la licence installée, quand elle est authentique.
    pub licence: Option<Licence>,
    /// Empreinte du poste auquel une licence libre s'est attachée en s'installant.
    pub empreinte_attachee: Option<String>,
    /// Instant d'installation.
    pub installee_le: Option<Horodatage>,
}

/// Service de licence.
pub struct ServiceLicence<'a> {
    noyau: &'a Noyau,
    cle_editeur: [u8; 32],
}

impl<'a> ServiceLicence<'a> {
    /// Service sur le noyau, vérifiant contre la clé publique donnée.
    pub fn nouveau(noyau: &'a Noyau, cle_editeur: [u8; 32]) -> Self {
        Self { noyau, cle_editeur }
    }

    /// État de la licence. Le premier constat d'un écart d'empreinte est enregistré : c'est le
    /// départ de la grâce des modules connectés, qui ne doit pas repartir à chaque démarrage.
    pub async fn etat(&self, etablissement_id: Id<Etablissement>) -> Resultat<EtatLicenceAffiche> {
        let contexte = self.noyau.contexte(etablissement_id, None);
        let empreinte_poste = self.noyau.empreinte_machine().to_owned();
        let enregistree = {
            let mut cnx = self.connexion().await?;
            depot::courante(&mut cnx, &contexte).await?
        };
        let Some(enregistree) = enregistree else {
            return Ok(EtatLicenceAffiche {
                empreinte_poste,
                etat: EtatLicence::ABSENTE,
                licence: None,
                empreinte_attachee: None,
                installee_le: None,
            });
        };
        // Une ligne illisible (clé de l'éditeur changée, base modifiée à la main) compte comme
        // une absence : l'école continue de travailler.
        let Ok(contenu) = licence::verifier(&enregistree.contenu, &self.cle_editeur) else {
            return Ok(EtatLicenceAffiche {
                empreinte_poste,
                etat: EtatLicence::ABSENTE,
                licence: None,
                empreinte_attachee: None,
                installee_le: Some(enregistree.installee_le),
            });
        };
        let attachee = enregistree.empreinte_attachee.as_deref();
        let ecart = contenu
            .empreinte_liee(attachee)
            .is_some_and(|liee| liee != empreinte_poste);
        let ecart_depuis = match (ecart, enregistree.ecart_empreinte_depuis) {
            (true, None) => {
                self.noter_ecart(
                    etablissement_id,
                    Some(contexte.maintenant),
                    &contenu,
                    attachee,
                )
                .await?;
                Some(contexte.maintenant)
            }
            (false, Some(_)) => {
                self.noter_ecart(etablissement_id, None, &contenu, attachee)
                    .await?;
                None
            }
            (_, depuis) => depuis,
        };
        Ok(EtatLicenceAffiche {
            etat: licence::evaluer(
                &contenu,
                attachee,
                &empreinte_poste,
                ecart_depuis,
                contexte.maintenant,
            ),
            empreinte_poste,
            empreinte_attachee: contenu
                .est_libre()
                .then(|| enregistree.empreinte_attachee.clone())
                .flatten(),
            licence: Some(contenu),
            installee_le: Some(enregistree.installee_le),
        })
    }

    /// Installe un fichier de licence après vérification de sa signature.
    pub async fn installer(&self, session: &Session, texte: &str) -> Resultat<EtatLicenceAffiche> {
        let contenu = licence::verifier(texte.trim(), &self.cle_editeur)?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let id = nouvel_id();
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        depot::remplacer(
            unite.connexion(),
            id,
            &contenu.licence_id,
            texte.trim(),
            self.noyau.empreinte_machine(),
            &contexte,
        )
        .await?;
        let resume = serde_json::json!({
            "licence_id": contenu.licence_id,
            "edition": contenu.edition,
            "modules": contenu.modules,
            "echeance": contenu.echeance,
            "libre": contenu.est_libre(),
            "empreinte_poste": self.noyau.empreinte_machine(),
        });
        unite.auditer(
            "licence",
            &id.vers_chaine(),
            Operation::Creation,
            None,
            Some(resume.clone()),
            "licence.installer",
        );
        unite.emettre(
            TypeEvenement::LicenceInstallee,
            "licence",
            &id.vers_chaine(),
            resume,
        );
        self.noyau.terminer(unite).await?;
        self.etat(session.etablissement_id).await
    }

    /// Clé d'avertissement du bandeau, quand la licence demande une action.
    pub async fn avertissement(
        &self,
        etablissement_id: Id<Etablissement>,
    ) -> Resultat<Option<&'static str>> {
        let etat = self.etat(etablissement_id).await?;
        Ok(match etat.etat.statut {
            StatutLicence::Absente => Some("avertissement.licence_absente"),
            StatutLicence::AReemettre if etat.licence.as_ref().is_some_and(Licence::est_libre) => {
                Some("avertissement.licence_a_reinstaller")
            }
            StatutLicence::AReemettre => Some("avertissement.licence_a_reemettre"),
            StatutLicence::Echue => (etat.licence.map(|l| l.edition) == Some(Edition::Connectee))
                .then_some("avertissement.licence_echue"),
            StatutLicence::Valide => None,
        })
    }

    async fn noter_ecart(
        &self,
        etablissement_id: Id<Etablissement>,
        depuis: Option<Horodatage>,
        contenu: &Licence,
        attachee: Option<&str>,
    ) -> Resultat<()> {
        let contexte = self.noyau.contexte(etablissement_id, None);
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        depot::noter_ecart(unite.connexion(), depuis, &contexte).await?;
        unite.auditer(
            "licence",
            &contenu.licence_id,
            Operation::Modification,
            None,
            Some(serde_json::json!({
                "ecart_empreinte_depuis": depuis,
                "empreinte_licence": contenu.empreinte_liee(attachee),
                "empreinte_poste": self.noyau.empreinte_machine(),
            })),
            "licence.ecart_empreinte",
        );
        self.noyau.terminer(unite).await?;
        Ok(())
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
}
