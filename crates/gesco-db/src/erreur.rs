//! Erreurs de la couche de persistance.

use thiserror::Error;

/// Erreur rendue par la couche de persistance.
#[derive(Debug, Error)]
pub enum ErreurBase {
    /// Erreur du moteur SQLite ou du pilote.
    #[error("base de données : {0}")]
    Sqlx(#[from] sqlx::Error),
    /// Échec d'une migration.
    #[error("migration : {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    /// Donnée persistée illisible (JSON, ULID, énumération inconnue).
    #[error("donnée persistée illisible dans {table}.{colonne} : {detail}")]
    DonneeIllisible {
        /// Table concernée.
        table: &'static str,
        /// Colonne concernée.
        colonne: &'static str,
        /// Détail technique.
        detail: String,
    },
    /// L'entité demandée n'existe pas dans le périmètre de l'établissement.
    #[error("{entite} introuvable : {id}")]
    Introuvable {
        /// Nom de l'entité.
        entite: &'static str,
        /// Identifiant demandé.
        id: String,
    },
    /// Une ligne appartient à un autre établissement que celui du contexte (R-12).
    #[error("cloisonnement : {entite} {id} n'appartient pas à l'établissement courant")]
    Cloisonnement {
        /// Nom de l'entité.
        entite: &'static str,
        /// Identifiant concerné.
        id: String,
    },
    /// La chaîne d'audit est rompue (R-27).
    #[error("chaîne d'audit rompue à la ligne {ligne_id} (position {index})")]
    ChaineRompue {
        /// Position de la rupture.
        index: usize,
        /// Identifiant de la ligne fautive.
        ligne_id: String,
    },
    /// La vérification d'intégrité SQLite a échoué.
    #[error("intégrité de la base : {0}")]
    Integrite(String),
    /// Sérialisation JSON.
    #[error("sérialisation : {0}")]
    Json(#[from] serde_json::Error),
}

impl ErreurBase {
    pub(crate) fn illisible(
        table: &'static str,
        colonne: &'static str,
        detail: impl ToString,
    ) -> Self {
        Self::DonneeIllisible {
            table,
            colonne,
            detail: detail.to_string(),
        }
    }
}

/// Résultat de la couche de persistance.
pub type Resultat<T> = Result<T, ErreurBase>;
