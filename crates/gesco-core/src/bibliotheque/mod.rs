//! Bibliothèque de cours (SPEC_PHASE_12_LOT_1) : format des leçons, paquet livré, règles des
//! versions, des retraits et de la publication. Règles pures, sans E/S (R-03).

pub mod catalogue;
pub mod classes;
pub mod ecriture;
pub mod fichiers;
pub mod lecon;
pub mod lecture;
pub mod paquet;
pub mod reglages;
pub mod word;

pub use catalogue::{
    BilanInstallation, ChapitreBibliotheque, ContenuBibliotheque, OrigineBibliotheque,
    PaquetInstalle, correspondances_par_code,
};
pub use classes::{ClasseChoisie, classes_de_l_annee, visible_pour_classe};
pub use ecriture::{
    EtatVersion, LigneNumerotee, PLAN_LECON, RangeeComparaison, TEXTE_MAX, comparer_lignes,
    images_a_ajouter,
};
pub use fichiers::{
    MORCEAU_VIDEO, SorteFichier, SvgRefuse, TAILLE_DOCUMENT_MAX, TAILLE_IMAGE_MAX, TAILLE_SON_MAX,
    est_empreinte, nettoyer_svg, reconnaitre, reconnaitre_son,
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
pub use word::{ErreurWord, ImageWord, LeconWord, NoteWord, TAILLE_WORD_MAX, convertir_docx};
