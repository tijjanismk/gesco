//! Effets de bord collectés pendant une transaction et exécutés après son commit (R-18).

use std::sync::Mutex;

use gesco_db::Contexte;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::erreur::{ErreurService, Resultat};

/// Un effet de bord déclaré par un service. La transaction ne l'exécute jamais elle-même.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Effet {
    /// Composer un document par le moteur documentaire.
    ComposerDocument {
        /// Type de document (reçu, bulletin, certificat de clé de secours…).
        type_document: String,
        /// Entité rattachée.
        entite: String,
        /// Identifiant de l'entité rattachée.
        entite_id: String,
        /// Données passées au modèle.
        donnees: serde_json::Value,
    },
    /// Porter un message à l'attention de la direction (file de notification, phase 5).
    AvertirDirection {
        /// Clé de traduction du message.
        cle_message: String,
        /// Données du message.
        donnees: serde_json::Value,
    },
}

/// Exécute les effets une fois la transaction validée.
pub trait ExecuteurEffets: Send + Sync {
    /// Exécute un effet. Une erreur n'annule pas la transaction, déjà validée : elle est
    /// journalisée par l'appelant et l'effet est rejoué par la file concernée.
    fn executer(&self, contexte: &Contexte, effet: Effet) -> Resultat<()>;
}

/// Exécuteur qui enregistre les effets sans rien faire : socle de la phase 0.5 et tests.
#[derive(Debug, Default)]
pub struct JournalEffets(Mutex<Vec<Effet>>);

impl JournalEffets {
    /// Effets reçus jusqu'ici, dans l'ordre.
    pub fn effets(&self) -> Vec<Effet> {
        self.0.lock().map(|v| v.clone()).unwrap_or_default()
    }
}

impl ExecuteurEffets for JournalEffets {
    fn executer(&self, _contexte: &Contexte, effet: Effet) -> Resultat<()> {
        if let Ok(mut v) = self.0.lock() {
            v.push(effet);
        }
        Ok(())
    }
}

/// Réception des effets, consommée par la tâche de fond (`documents::traiter_effets`).
pub type RecepteurEffets = mpsc::UnboundedReceiver<(Contexte, Effet)>;

/// Exécuteur de production : chaque effet est remis, avec le contexte de l'opération qui l'a
/// déclaré, à une tâche de fond. La commande rend la main sans attendre la composition d'un PDF.
#[derive(Debug, Clone)]
pub struct FileEffets(mpsc::UnboundedSender<(Contexte, Effet)>);

impl FileEffets {
    /// File et son récepteur.
    pub fn nouvelle() -> (Self, RecepteurEffets) {
        let (envoi, reception) = mpsc::unbounded_channel();
        (Self(envoi), reception)
    }
}

impl ExecuteurEffets for FileEffets {
    fn executer(&self, contexte: &Contexte, effet: Effet) -> Resultat<()> {
        self.0
            .send((contexte.clone(), effet))
            .map_err(|_| ErreurService::Regle {
                cle: "effets.file_fermee",
            })
    }
}
