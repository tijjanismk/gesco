//! Rapports de l'école (SPEC_PHASE_9 §3, §4, §5, §8.6) : le rapport succinct de rentrée, le
//! rapport de rentrée et le rapport de fermeture du fondamental. Un rapport tient ses textes rédigés par la direction ; ses
//! tableaux se calculent à sa date d'arrêt et ne se retouchent pas (règle 9). Arrêté, il ne bouge
//! plus (règle 10) ; rouvert, il passe à sa version suivante et l'ancien document reste.

use serde::{Deserialize, Serialize};

use crate::commun::DateCivile;

/// Type d'un rapport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum TypeRapport {
    /// Rapport succinct de rentrée, une page, quinze jours après la rentrée.
    SuccinctRentree,
    /// Rapport de rentrée : partie rédigée et tableaux.
    Rentree,
    /// Rapport de fermeture : partie rédigée, résultats et tableaux de fin d'année.
    Fermeture,
}

impl TypeRapport {
    /// Tous les types, dans l'ordre de l'écran.
    pub const TOUS: &'static [Self] = &[Self::SuccinctRentree, Self::Rentree, Self::Fermeture];

    /// Nom rangé dans la base.
    pub const fn nom(self) -> &'static str {
        match self {
            Self::SuccinctRentree => "succinct_rentree",
            Self::Rentree => "rentree",
            Self::Fermeture => "fermeture",
        }
    }

    /// Type d'un nom de la base ; rien pour un nom inconnu.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::TOUS.iter().copied().find(|t| t.nom() == nom)
    }

    /// Titre imprimé.
    pub const fn titre(self) -> &'static str {
        match self {
            Self::SuccinctRentree => "Rapport succinct de rentrée",
            Self::Rentree => "Rapport de rentrée",
            Self::Fermeture => "Rapport de fermeture",
        }
    }

    /// Sections rédigées, dans l'ordre du modèle.
    pub const fn sections(self) -> &'static [SectionModele] {
        match self {
            Self::SuccinctRentree => &[],
            Self::Rentree => SECTIONS_RENTREE,
            Self::Fermeture => SECTIONS_FERMETURE,
        }
    }

    /// Date d'arrêt proposée à la création : quinze jours après le premier jour de l'année pour
    /// les rapports de rentrée, le dernier jour de l'année pour le rapport de fermeture.
    pub fn date_arret_proposee(self, rentree: DateCivile, fin: DateCivile) -> DateCivile {
        match self {
            Self::SuccinctRentree | Self::Rentree => rentree.decaler(15).unwrap_or(rentree),
            Self::Fermeture => fin,
        }
    }
}

/// État d'un rapport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum EtatRapport {
    /// En préparation : textes et réglages se modifient, les tableaux suivent les données.
    Preparation,
    /// Arrêté : chiffres et textes figés avec le document.
    Arrete,
}

impl EtatRapport {
    /// Nom rangé dans la base.
    pub const fn nom(self) -> &'static str {
        match self {
            Self::Preparation => "preparation",
            Self::Arrete => "arrete",
        }
    }

    /// État d'un nom de la base ; « en préparation » pour un nom inconnu.
    pub fn depuis_nom(nom: &str) -> Self {
        if nom == "arrete" {
            Self::Arrete
        } else {
            Self::Preparation
        }
    }
}

/// Une section rédigée d'un modèle de rapport.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SectionModele {
    /// Code stable, rangé avec le texte.
    pub code: &'static str,
    /// Titre imprimé.
    pub titre: &'static str,
    /// Vrai quand GESCO propose à côté des chiffres à recopier.
    pub aidee: bool,
}

const fn section(code: &'static str, titre: &'static str, aidee: bool) -> SectionModele {
    SectionModele { code, titre, aidee }
}

/// Sections rédigées du rapport de rentrée (SPEC_PHASE_9 §4.1).
const SECTIONS_RENTREE: &[SectionModele] = &[
    section("introduction", "Introduction", true),
    section("preparation", "Préparation de la rentrée", false),
    section("deroulement", "Déroulement de la rentrée", false),
    section("organisation", "Organisation pédagogique", false),
    section("partenaires", "Rapports avec les partenaires", false),
    section("conclusion", "Conclusion", false),
];

/// Sections rédigées du rapport de fermeture (SPEC_PHASE_9 §5.1), dans l'ordre du modèle.
const SECTIONS_FERMETURE: &[SectionModele] = &[
    section("introduction", "Introduction", false),
    section(
        "deroulement",
        "Conditions de déroulement de l'année scolaire",
        false,
    ),
    section("difficultes", "Difficultés", false),
    section("solutions", "Solutions préconisées", false),
    section("etat_des_lieux", "État des lieux", true),
    section("besoins", "Besoins", true),
    section("examens", "Organisation des examens", false),
    section("partenaires", "Rapports avec les partenaires", false),
    section("resultats", "Résultats scolaires", true),
    section("conclusion", "Conclusion", false),
];

/// Vrai si le code est celui d'une section du type.
pub fn section_connue(type_rapport: TypeRapport, code: &str) -> bool {
    type_rapport.sections().iter().any(|s| s.code == code)
}

/// Longueur la plus grande d'un texte de section, en caractères.
pub const TEXTE_MAX: usize = 20_000;

/// Qualité du signataire proposée quand aucun rapport n'en a encore donné.
pub const QUALITE_PAR_DEFAUT: &str = "Le Directeur";

/// Vérifie un texte de section : rien n'est exigé, seule la longueur est bornée.
pub fn verifier_texte(texte: &str) -> Result<String, &'static str> {
    if texte.chars().count() > TEXTE_MAX {
        return Err("rapports.texte_trop_long");
    }
    Ok(texte.trim_end().to_owned())
}

/// Vérifie un réglage court (lieu, qualité du signataire).
pub fn verifier_reglage(texte: &str) -> Result<String, &'static str> {
    let t = texte.trim();
    if t.chars().count() > 120 {
        return Err("rapports.reglage_trop_long");
    }
    Ok(t.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_noms_se_relisent() {
        for t in TypeRapport::TOUS {
            assert_eq!(TypeRapport::depuis_nom(t.nom()), Some(*t));
        }
        assert_eq!(
            TypeRapport::depuis_nom("fermeture"),
            Some(TypeRapport::Fermeture)
        );
        assert_eq!(TypeRapport::depuis_nom("enquete"), None);
        assert_eq!(EtatRapport::depuis_nom("arrete"), EtatRapport::Arrete);
        assert_eq!(EtatRapport::depuis_nom("?"), EtatRapport::Preparation);
    }

    #[test]
    fn la_date_proposee_est_quinze_jours_apres_la_rentree() {
        let dates = DateCivile::analyser("2026-10-01").zip(DateCivile::analyser("2027-06-30"));
        let proposee = |t: TypeRapport| {
            dates.map(|(rentree, fin)| t.date_arret_proposee(rentree, fin).to_string())
        };
        assert_eq!(
            proposee(TypeRapport::SuccinctRentree).as_deref(),
            Some("2026-10-16")
        );
        assert_eq!(
            proposee(TypeRapport::Fermeture).as_deref(),
            Some("2027-06-30")
        );
    }

    #[test]
    fn le_rapport_de_fermeture_suit_le_modele() {
        let s = TypeRapport::Fermeture.sections();
        assert_eq!(s.len(), 10);
        let aidees: Vec<&str> = s.iter().filter(|x| x.aidee).map(|x| x.code).collect();
        assert_eq!(aidees, ["etat_des_lieux", "besoins", "resultats"]);
        assert!(section_connue(TypeRapport::Fermeture, "solutions"));
        assert!(!section_connue(TypeRapport::Fermeture, "preparation"));
    }

    #[test]
    fn le_rapport_de_rentree_a_six_sections_et_l_introduction_est_aidee() {
        let s = TypeRapport::Rentree.sections();
        assert_eq!(s.len(), 6);
        assert!(
            s.first()
                .is_some_and(|x| x.aidee && x.code == "introduction")
        );
        assert_eq!(s.iter().filter(|x| x.aidee).count(), 1);
        assert!(TypeRapport::SuccinctRentree.sections().is_empty());
        assert!(section_connue(TypeRapport::Rentree, "conclusion"));
        assert!(!section_connue(TypeRapport::SuccinctRentree, "conclusion"));
    }

    #[test]
    fn un_texte_trop_long_est_refuse() {
        assert_eq!(
            verifier_texte(
                "Bonne rentrée.  
"
            ),
            Ok("Bonne rentrée.".to_owned())
        );
        assert!(verifier_texte(&"a".repeat(TEXTE_MAX + 1)).is_err());
        assert!(verifier_reglage(&"b".repeat(121)).is_err());
        assert_eq!(verifier_reglage("  Kati "), Ok("Kati".to_owned()));
    }
}
