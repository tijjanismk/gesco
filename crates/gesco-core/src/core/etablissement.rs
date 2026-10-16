//! Hiérarchie Organisation → Établissement (→ Campus), présente dès l'origine (ADR-006).

use serde::{Deserialize, Serialize};

use crate::commun::Id;

/// Propriétaire d'un ou plusieurs établissements.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Organisation {
    /// Identifiant technique.
    pub id: Id<Organisation>,
    /// Raison sociale ou nom du promoteur.
    pub nom: String,
}

/// Statut administratif de l'établissement, utilisé par les états CPS/SE.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum StatutEtablissement {
    /// École publique.
    Public,
    /// École privée.
    Prive,
    /// École communautaire.
    Communautaire,
    /// Medersa.
    Medersa,
}

impl StatutEtablissement {
    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Prive => "prive",
            Self::Communautaire => "communautaire",
            Self::Medersa => "medersa",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        [
            Self::Public,
            Self::Prive,
            Self::Communautaire,
            Self::Medersa,
        ]
        .into_iter()
        .find(|s| s.nom() == nom)
    }
}

/// Établissement scolaire : l'unité de cloisonnement de toutes les données métier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Etablissement {
    /// Identifiant technique.
    pub id: Id<Etablissement>,
    /// Organisation propriétaire.
    pub organisation_id: Id<Organisation>,
    /// Nom officiel, imprimé en en-tête des documents.
    pub nom: String,
    /// Statut administratif.
    pub statut: StatutEtablissement,
    /// Code de l'Académie d'enseignement (remontées CPS/SE).
    pub code_academie: String,
    /// Code du Centre d'animation pédagogique.
    pub code_cap: String,
    /// Adresse postale imprimée.
    pub adresse: String,
    /// Téléphones de l'établissement.
    pub telephones: Vec<String>,
    /// Mentions légales imprimées au pied des documents.
    pub mentions_legales: String,
    /// Multiple d'arrondi commercial des montants (§3.7), en francs.
    pub multiple_arrondi: i64,
}
