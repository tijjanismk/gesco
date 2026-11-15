//! Bibliothèque de cours (SPEC_PHASE_12_LOT_1) : format des leçons, paquet livré, règles des
//! versions, des retraits et de la publication. Règles pures, sans E/S (R-03).

pub mod catalogue;
pub mod ecriture;
pub mod lecon;
pub mod lecture;
pub mod paquet;
pub mod reglages;

pub use catalogue::{
    BilanInstallation, ChapitreBibliotheque, ContenuBibliotheque, OrigineBibliotheque,
    PaquetInstalle, correspondances_par_code,
};
pub use ecriture::{
    EtatVersion, LigneNumerotee, PLAN_LECON, RangeeComparaison, TEXTE_MAX, comparer_lignes,
    images_a_ajouter,
};
pub use lecon::{Alignement, Bloc, Enligne, FormuleLecon, Lecon, SourceImage, analyser};
pub use lecture::{
    ChapitreCatalogue, CibleRetrait, ContenuCatalogue, EntreesCatalogue, EtatContenu, GrainRetrait,
    MatiereCatalogue, MatiereReference, NiveauCatalogue, NiveauReference, PartieCatalogue,
    PlaceContenu, SerieCatalogue, SerieReference, construire_catalogue, places_du_catalogue,
    ranger, requete_recherche,
};
pub use paquet::{
    ChapitrePaquet, ContenuPaquet, DisciplinePaquet, ErreurPaquet, ExamenSujet, ManifestePaquet,
    NatureContenu, PaquetLu, comparer_versions, lire_paquet, marques_interdites,
};
