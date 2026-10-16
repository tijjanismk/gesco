//! Types fondamentaux partagés par tous les domaines.
//!
//! Ils encodent les décisions non négociables du blueprint : montants entiers en francs (ADR-005),
//! notes entières en centièmes (RC-6), identifiants ULID typés (ADR-004), temps passé en
//! paramètre (R-03). Aucun flottant n'existe sur ces types, ni en entrée ni en sortie.

pub mod date;
pub mod erreur;
pub mod horodatage;
pub mod id;
pub mod lettres;
pub mod montant;
pub mod note;

pub use date::DateCivile;
pub use erreur::{CleTraduction, ErreurHorodatage, ErreurId, ErreurMontant, ErreurNote};
pub use horodatage::{DateHeureCivile, Horodatage};
pub use id::Id;
pub use montant::{DivisionMontant, ModeArrondi, Montant};
pub use note::{Note, SommePonderee};
