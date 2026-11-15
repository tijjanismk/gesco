//! Bibliothèque de cours (SPEC_PHASE_12_LOT_1) : format des leçons, paquet livré, règles des
//! versions, des retraits et de la publication. Règles pures, sans E/S (R-03).

pub mod lecon;
pub mod paquet;
pub mod reglages;

pub use lecon::{Alignement, Bloc, Enligne, FormuleLecon, Lecon, SourceImage, analyser};
pub use paquet::{
    ChapitrePaquet, ContenuPaquet, DisciplinePaquet, ErreurPaquet, ExamenSujet, ManifestePaquet,
    NatureContenu, PaquetLu, comparer_versions, lire_paquet, marques_interdites,
};
