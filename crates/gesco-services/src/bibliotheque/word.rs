//! Importer une leçon depuis Word (SPEC_PHASE_12_LOT_2 §4 ; ADR « importer une leçon Word ») :
//! le `.docx` choisi devient le texte du brouillon d'un cours ; ses images se rangent comme celles
//! du bouton Image. Le brouillon suit ensuite le circuit habituel (soumission, publication).
//!
//! Le cours n'a pas de brouillon : il en commence un, parti de la version visible comme
//! « Commencer un brouillon ». Il en a un, de l'auteur : son texte est remplacé, après la
//! confirmation de l'écran, à la révision que l'éditeur a lue.

use std::path::Path;

use gesco_core::bibliotheque::paquet::empreinte;
use gesco_core::bibliotheque::{
    ErreurWord, NatureContenu, NoteWord, TAILLE_WORD_MAX, TEXTE_MAX, convertir_docx,
};
use gesco_core::core::{Operation, TypeEvenement};
use gesco_db::depots::bibliotheque as depot;
use gesco_db::unite::Durabilite;
use serde::{Deserialize, Serialize};

use super::ecrire::{DemandeContenuBibliotheque, EcritureOuverte, nombre};
use super::{ServiceBibliotheque, regle};
use crate::erreur::Resultat;
use crate::session::Session;

/// Importer un fichier Word dans le brouillon d'un cours.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeImportWord {
    /// Identifiant du cours.
    pub contenu_id: String,
    /// Chemin du fichier `.docx`.
    pub chemin: String,
    /// Brouillon ouvert dans l'éditeur, dont le texte sera remplacé ; aucun si le cours n'en a
    /// pas.
    pub version_id: Option<String>,
    /// Révision de ce brouillon lue par l'éditeur.
    pub revision: Option<u32>,
}

/// Le cours après l'import, et ce que l'auteur lit sous l'aperçu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ImportWord {
    /// Le cours ouvert pour l'écriture, avec son brouillon.
    pub ecriture: EcritureOuverte,
    /// Ce qui n'est pas passé.
    pub notes: Vec<NoteWord>,
}

impl ServiceBibliotheque<'_> {
    /// Importe un `.docx` dans le brouillon d'un cours de la portée de l'utilisateur.
    pub async fn importer_word(
        &self,
        session: &Session,
        demande: &DemandeImportWord,
    ) -> Resultat<ImportWord> {
        let chemin = Path::new(&demande.chemin);
        if std::fs::metadata(chemin)?.len() > TAILLE_WORD_MAX {
            return Err(regle("bibliotheque.word_trop_lourd"));
        }
        let octets = std::fs::read(chemin)?;
        let lecon = convertir_docx(&octets).map_err(|e| {
            regle(match e {
                ErreurWord::AncienFormat => "bibliotheque.word_ancien_format",
                ErreurWord::Illisible => "bibliotheque.word_illisible",
                ErreurWord::TropLourd => "bibliotheque.word_trop_lourd",
            })
        })?;
        if lecon.texte.len() > TEXTE_MAX {
            return Err(regle("bibliotheque.texte_trop_long"));
        }
        let nom_fichier = chemin
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();

        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let portee = self.portee(session).await?;
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let contenu =
            Self::contenu_permis(unite.connexion(), &portee, &demande.contenu_id, &contexte)
                .await?;
        if contenu.nature == NatureContenu::Document {
            return Err(regle("bibliotheque.word_pas_un_cours"));
        }
        let versions = depot::versions(unite.connexion(), &demande.contenu_id, &contexte).await?;
        if versions.iter().any(|v| v.etat == "soumise") {
            return Err(regle("bibliotheque.en_attente_de_validation"));
        }
        let brouillon = versions.iter().find(|v| v.etat == "brouillon");
        let attendu = (demande.version_id.as_deref(), demande.revision);
        if let Some(b) = brouillon {
            if attendu != (Some(b.id.as_str()), Some(nombre(b.revision))) {
                return Err(regle("bibliotheque.modifiee_ailleurs"));
            }
            if b.auteur_id != session.utilisateur_id.vers_chaine() {
                return Err(regle("bibliotheque.brouillon_d_un_autre"));
            }
            if b.retour_livre != 0 {
                return Err(regle("bibliotheque.retour_sans_texte"));
            }
        } else if attendu.0.is_some() {
            return Err(regle("bibliotheque.modifiee_ailleurs"));
        }

        for image in &lecon.images {
            Self::enregistrer_dans(
                &mut unite,
                &contexte,
                image.sorte,
                &image.nom,
                &image.octets,
            )
            .await?;
        }

        let version_id = match brouillon {
            Some(b) => {
                let empreinte_texte = empreinte(lecon.texte.as_bytes());
                if !depot::reecrire_brouillon(
                    unite.connexion(),
                    &b.id,
                    b.revision,
                    &lecon.texte,
                    &empreinte_texte,
                    &contexte,
                )
                .await?
                {
                    return Err(regle("bibliotheque.modifiee_ailleurs"));
                }
                unite.auditer(
                    "version_contenu",
                    &b.id,
                    Operation::Modification,
                    Some(serde_json::json!({ "revision": b.revision, "texte": b.texte })),
                    Some(serde_json::json!({ "revision": b.revision + 1, "texte": lecon.texte })),
                    "bibliotheque.import_word",
                );
                unite.emettre(
                    TypeEvenement::VersionContenuModifiee,
                    "version_contenu",
                    &b.id,
                    serde_json::json!({
                        "contenu_id": demande.contenu_id,
                        "numero": b.numero,
                        "etat": "brouillon",
                    }),
                );
                b.id.clone()
            }
            None => {
                let (depart_version, depart_livre) = match self.texte_visible(&contenu, &versions) {
                    (Some(_), Some(v)) => (Some(v.id.clone()), v.depart_livre_sha256.clone()),
                    (Some(_), None) => (None, contenu.empreinte.clone()),
                    (None, _) => (None, None),
                };
                self.creer_brouillon(
                    &mut unite,
                    session,
                    &demande.contenu_id,
                    &lecon.texte,
                    false,
                    depart_version.as_deref(),
                    depart_livre.as_deref(),
                    "bibliotheque.import_word",
                )
                .await?
            }
        };
        // Le geste lui-même, avec le fichier d'où vient le texte.
        unite.auditer(
            "version_contenu",
            &version_id,
            Operation::Acces,
            None,
            Some(serde_json::json!({
                "contenu_id": demande.contenu_id,
                "fichier": nom_fichier,
                "images": lecon.images.len(),
                "notes": lecon.notes,
            })),
            "bibliotheque.import_word",
        );
        self.noyau.terminer(unite).await?;
        let ecriture = self
            .ecriture(
                session,
                &DemandeContenuBibliotheque {
                    contenu_id: demande.contenu_id.clone(),
                },
            )
            .await?;
        Ok(ImportWord {
            ecriture,
            notes: lecon.notes,
        })
    }
}
