//! Paie (SPEC_PHASE_7) : éléments de paie de l'école et leurs versions datées, attributions,
//! réglages des mois payés et du prorata. Le noyau applique des éléments ; il ne connaît aucune
//! règle fiscale ou sociale d'un pays (§8.7 du blueprint).

pub mod avance;
pub mod calcul;
pub mod element;
pub mod mois;
pub mod paiement;
pub mod reglages;

pub use avance::{Avance, StatutAvance};
pub use element::{
    Attribution, Calcul, ElementPaie, NatureElement, ReductionFamille, TrancheBareme,
    VersionElement,
};
pub use mois::{AbsenceConfirmee, BulletinPaie, PaieMois, SaisieMois, StatutPaie};
pub use paiement::{ObjetPaiement, PaiementPersonnel, SourcePaiement};
pub use reglages::Prorata;
