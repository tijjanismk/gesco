//! Domaine SCOLARITÉ : référentiel académique, élèves, responsables, inscriptions. Entités,
//! invariants et transitions ; la persistance est dans `gesco-db`.

pub mod academique;
pub mod depart;
pub mod fin_annee;
pub mod fusion;
pub mod import_eleves;
pub mod inscription;
pub mod matricule;
pub mod personne;
pub mod recherche;
pub mod telephone;

pub use academique::{Classe, Cycle, DiplomeTerminal, ModeEvaluation, Niveau, Serie};
pub use depart::{Exclusion, NatureExclusion, StatutExclusion};
pub use inscription::{DecisionFinAnnee, Inscription, StatutInscription, TypeInscription};
pub use personne::{Eleve, Lien, NatureActe, Personne, Qualites, Responsable, Sexe};
