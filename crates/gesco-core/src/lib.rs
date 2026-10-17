//! Noyau métier de GESCO.
//!
//! Cette crate ne lit ni n'écrit rien : ni base, ni réseau, ni fichier, ni horloge (R-03).
//! Elle reçoit des données et retourne des résultats, ce qui la rend testable sans environnement.

pub mod commun;
pub mod core;
pub mod finance;
pub mod notification;
pub mod paie;
pub mod pedagogie;
pub mod rh;
pub mod scolarite;
pub mod tresorerie;
