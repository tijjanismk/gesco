//! Persistance : repositories, migrations et cloisonnement par établissement (ADR-003, ADR-006).
//!
//! Tout accès aux tables passe par un dépôt de cette crate, qui applique le filtre
//! `etablissement_id` (R-12) et refuse toute suppression physique sur les entités protégées
//! (R-14). Les écritures métier passent par [`UniteDeTravail`], qui garantit qu'une mutation, sa
//! ligne d'audit et son événement `outbox` sont validés ensemble (R-17).

pub mod base;
pub mod contexte;
pub mod depots;
pub mod erreur;
pub mod id;
pub mod unite;

pub use base::{Base, OptionsBase};
pub use contexte::Contexte;
pub use erreur::ErreurBase;
pub use id::nouvel_id;
pub use unite::UniteDeTravail;

/// Connexion SQLite, type commun à tous les dépôts.
pub type Connexion = sqlx::SqliteConnection;

/// Connexion empruntée au pool, rendue à sa libération.
pub type ConnexionEmpruntee = sqlx::pool::PoolConnection<sqlx::Sqlite>;
