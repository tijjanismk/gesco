//! Erreurs du moteur documentaire.

use thiserror::Error;

/// Erreur rendue par le moteur documentaire.
#[derive(Debug, Error)]
pub enum ErreurDocument {
    /// Aucun modèle ne porte cet identifiant.
    #[error("modèle inconnu : {0}")]
    ModeleInconnu(String),
    /// Le manifeste d'un modèle est illisible ou incohérent.
    #[error("manifeste du modèle {modele} invalide : {detail}")]
    ManifesteInvalide {
        /// Modèle concerné.
        modele: String,
        /// Détail.
        detail: String,
    },
    /// Une variable requise manque : on refuse plutôt que d'imprimer un document à trou (§10.3).
    #[error("variable requise absente pour le modèle {modele} : {variable}")]
    VariableManquante {
        /// Modèle concerné.
        modele: String,
        /// Variable absente.
        variable: String,
    },
    /// Une variable fournie n'est pas déclarée par le modèle (faute de frappe probable).
    #[error("variable non déclarée par le modèle {modele} : {variable}")]
    VariableInconnue {
        /// Modèle concerné.
        modele: String,
        /// Variable en trop.
        variable: String,
    },
    /// Les données ne forment pas un objet JSON.
    #[error("les données du modèle {0} doivent être un objet")]
    DonneesInvalides(String),
    /// Typst a refusé le modèle.
    #[error("composition du modèle {modele} : {detail}")]
    Composition {
        /// Modèle concerné.
        modele: String,
        /// Premier diagnostic de Typst.
        detail: String,
    },
    /// L'export PDF a échoué.
    #[error("export PDF du modèle {modele} : {detail}")]
    Export {
        /// Modèle concerné.
        modele: String,
        /// Premier diagnostic.
        detail: String,
    },
    /// Clé de document de lot inutilisable comme nom de fichier.
    #[error("clé de document invalide dans le lot : {0}")]
    CleLotInvalide(String),
    /// Lecture ou écriture de fichier.
    #[error("fichier : {0}")]
    Fichier(#[from] std::io::Error),
    /// Une ligne de la liste à exporter n'a pas la largeur des en-têtes (0 : aucune colonne).
    #[error("ligne {ligne} de la liste exportée de travers")]
    ListeIrreguliere {
        /// Numéro de la ligne fautive, 1 pour la première ; 0 quand les en-têtes manquent.
        ligne: usize,
    },
    /// La liste dépasse ce qu'un classeur exporté peut contenir.
    #[error("liste trop grande pour un classeur")]
    ListeTropGrande,
    /// Le classeur Excel n'a pas pu être écrit.
    #[error("classeur : {0}")]
    Classeur(String),
}

impl From<rust_xlsxwriter::XlsxError> for ErreurDocument {
    fn from(erreur: rust_xlsxwriter::XlsxError) -> Self {
        Self::Classeur(erreur.to_string())
    }
}

impl From<std::num::TryFromIntError> for ErreurDocument {
    fn from(_: std::num::TryFromIntError) -> Self {
        Self::ListeTropGrande
    }
}

impl ErreurDocument {
    /// Clé de traduction du message destiné à l'utilisateur (R-39).
    pub const fn cle_traduction(&self) -> &'static str {
        match self {
            Self::ModeleInconnu(_) => "document.modele_inconnu",
            Self::ManifesteInvalide { .. } => "document.manifeste_invalide",
            Self::VariableManquante { .. } => "document.variable_manquante",
            Self::VariableInconnue { .. } => "document.variable_inconnue",
            Self::DonneesInvalides(_) => "document.donnees_invalides",
            Self::Composition { .. } | Self::Export { .. } => "document.composition",
            Self::CleLotInvalide(_) => "document.cle_lot_invalide",
            Self::Fichier(_) => "document.fichier",
            Self::ListeIrreguliere { .. } => "export.liste_irreguliere",
            Self::ListeTropGrande => "export.liste_trop_grande",
            Self::Classeur(_) => "export.classeur",
        }
    }
}
