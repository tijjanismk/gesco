//! Bibliothèque de cours (SPEC_PHASE_12_LOT_1) : format des leçons, règles des versions, des
//! retraits et de la publication. Règles pures, sans E/S (R-03).

pub mod lecon;

pub use lecon::{Alignement, Bloc, Enligne, FormuleLecon, Lecon, SourceImage, analyser};
