//! Trésorerie (SPEC_PHASE_8) : catégories de recettes et de dépenses, comptes de l'école (la
//! caisse, les comptes en banque, les comptes mobile money). L'école sait ce qui entre et ce qui
//! sort, par catégorie, sur chaque compte (T-62, T-63) ; ce n'est pas une comptabilité générale.

pub mod depense;
pub mod fournisseur;
pub mod livre;
pub mod recette;

use serde::{Deserialize, Serialize};

use crate::commun::{DateCivile, Id, Montant};
use crate::core::Etablissement;

pub use depense::{Depense, StatutDepense};

/// Longueur maximale d'un libellé de catégorie ou de compte.
pub const LIBELLE_MAX: usize = 60;

/// Sens d'une catégorie.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum Sens {
    /// L'argent entre.
    Recette,
    /// L'argent sort.
    Depense,
}

impl Sens {
    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Recette => "recette",
            Self::Depense => "depense",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        [Self::Recette, Self::Depense]
            .into_iter()
            .find(|s| s.nom() == nom)
    }
}

/// Catégorie de recette ou de dépense, propre à l'école ; les livrées ont un code stable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct CategorieTresorerie {
    /// Identifiant technique.
    pub id: Id<CategorieTresorerie>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Code d'une catégorie livrée ; absent pour une catégorie de l'école.
    pub code: Option<String>,
    /// Recette ou dépense.
    pub sens: Sens,
    /// Libellé affiché, modifiable.
    pub libelle: String,
    /// Ordre d'affichage.
    pub ordre: i64,
    /// Faux pour une catégorie retirée : les écritures qui la portent la gardent.
    pub active: bool,
}

/// Catégories livrées : code, sens et libellé. Les recettes suivent les types de frais de la
/// phase 3 ; « Salaires » reçoit les salaires et les avances payés (SPEC_PHASE_8 §6).
pub const CATEGORIES_LIVREES: [(&str, Sens, &str); 18] = [
    ("frais_inscription", Sens::Recette, "Frais d'inscription"),
    ("frais_scolarite", Sens::Recette, "Scolarité"),
    ("frais_cantine", Sens::Recette, "Cantine"),
    ("frais_transport", Sens::Recette, "Transport"),
    ("frais_tenue", Sens::Recette, "Tenue"),
    ("frais_examen", Sens::Recette, "Frais d'examen"),
    ("frais_autre", Sens::Recette, "Autres frais"),
    ("autres_recettes", Sens::Recette, "Dons et autres recettes"),
    ("salaires", Sens::Depense, "Salaires"),
    ("loyer", Sens::Depense, "Loyer"),
    ("eau_electricite", Sens::Depense, "Électricité et eau"),
    ("fournitures", Sens::Depense, "Fournitures"),
    ("entretien", Sens::Depense, "Entretien et réparations"),
    ("cantine", Sens::Depense, "Cantine (vivres)"),
    ("transport", Sens::Depense, "Transport"),
    ("impots", Sens::Depense, "Impôts et taxes"),
    ("frais_bancaires", Sens::Depense, "Frais bancaires"),
    ("divers", Sens::Depense, "Divers"),
];

/// Code de la catégorie des salaires.
pub const CATEGORIE_SALAIRES: &str = "salaires";

/// Nature d'un compte de trésorerie.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum NatureCompte {
    /// La caisse de l'école, faite de ses sessions : une seule.
    Caisse,
    /// Un compte en banque.
    Banque,
    /// Un compte mobile money (Orange Money, Moov Money, Wave).
    MobileMoney,
}

impl NatureCompte {
    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Caisse => "caisse",
            Self::Banque => "banque",
            Self::MobileMoney => "mobile_money",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        [Self::Caisse, Self::Banque, Self::MobileMoney]
            .into_iter()
            .find(|n| n.nom() == nom)
    }
}

/// Compte de trésorerie de l'école.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct CompteTresorerie {
    /// Identifiant technique.
    pub id: Id<CompteTresorerie>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Nature.
    pub nature: NatureCompte,
    /// Libellé : « Caisse », « BDM Hamdallaye », « Orange Money de l'école ».
    pub libelle: String,
    /// Banque ou opérateur.
    pub organisme: String,
    /// Numéro de compte ou de téléphone, tel qu'écrit.
    pub numero: String,
    /// Solde au jour d'ouverture dans GESCO.
    pub solde_ouverture: Montant,
    /// Jour d'ouverture dans GESCO : les mouvements comptent à partir de lui.
    pub ouvert_le: DateCivile,
    /// Faux pour un compte fermé : il garde ses mouvements.
    pub actif: bool,
}

/// Vérifie un libellé de catégorie ou de compte.
pub fn verifier_libelle(libelle: &str) -> Result<(), &'static str> {
    let l = libelle.trim();
    if l.is_empty() || l.chars().count() > LIBELLE_MAX {
        return Err("tresorerie.libelle_invalide");
    }
    Ok(())
}

/// Vérifie un compte : un libellé, une caisse unique, un organisme pour une banque ou un mobile
/// money ; un solde d'ouverture positif ou nul.
pub fn verifier_compte(compte: &CompteTresorerie) -> Result<(), &'static str> {
    verifier_libelle(&compte.libelle)?;
    if compte.nature != NatureCompte::Caisse && compte.organisme.trim().is_empty() {
        return Err("tresorerie.organisme_obligatoire");
    }
    if compte.solde_ouverture.est_negatif() {
        return Err("tresorerie.solde_negatif");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn les_categories_livrees_ont_un_code_unique_et_couvrent_les_frais() {
        let codes: HashSet<&str> = CATEGORIES_LIVREES.iter().map(|(c, ..)| *c).collect();
        assert_eq!(codes.len(), CATEGORIES_LIVREES.len());
        assert!(codes.contains(CATEGORIE_SALAIRES));
        for frais in [
            "inscription",
            "scolarite",
            "cantine",
            "transport",
            "tenue",
            "examen",
            "autre",
        ] {
            assert!(codes.contains(format!("frais_{frais}").as_str()), "{frais}");
        }
    }

    #[test]
    fn un_compte_en_banque_a_sa_banque() {
        let mut c = CompteTresorerie {
            id: Id::depuis_octets([1; 16]),
            etablissement_id: Id::depuis_octets([2; 16]),
            nature: NatureCompte::Banque,
            libelle: "Compte courant".to_owned(),
            organisme: String::new(),
            numero: "ML010 01001 0123456789 12".to_owned(),
            solde_ouverture: Montant::depuis_francs(250_000),
            ouvert_le: DateCivile::nouvelle(2026, 10, 1).unwrap_or_else(|| unreachable!()),
            actif: true,
        };
        assert_eq!(verifier_compte(&c), Err("tresorerie.organisme_obligatoire"));
        c.organisme = "BDM".to_owned();
        assert_eq!(verifier_compte(&c), Ok(()));
        c.solde_ouverture = Montant::depuis_francs(-1);
        assert_eq!(verifier_compte(&c), Err("tresorerie.solde_negatif"));
        assert_eq!(verifier_libelle(" "), Err("tresorerie.libelle_invalide"));
    }
}
