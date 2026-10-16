//! Matières de l'établissement et référentiel livré (SPEC_PHASE_4 §2.1, Q-54) : au 1er cycle,
//! les matières de l'emploi du temps des écoles (réponse du 13 octobre 2026), les cinq domaines
//! du curriculum 2011 restant disponibles (Q-23) ; une liste par défaut ensuite. L'école
//! modifie, ajoute et désactive ; aucun coefficient n'est écrit en dur (T-27). L'enseignement
//! technique a ses matières de pratique et de stage, à ajouter aux programmes de chaque filière
//! (SPEC_PHASE_6 §10, Q-72).

use serde::{Deserialize, Serialize};

use crate::commun::Id;
use crate::core::Etablissement;

/// Groupe d'une matière, pour l'ordre et les regroupements du bulletin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum GroupeMatiere {
    /// Lettres, langues, sciences humaines.
    Litteraire,
    /// Mathématiques et sciences.
    Scientifique,
    /// Le reste : éducation physique, arts, développement de la personne.
    Autre,
}

impl GroupeMatiere {
    /// Les trois groupes.
    pub const TOUS: [Self; 3] = [Self::Litteraire, Self::Scientifique, Self::Autre];

    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Litteraire => "litteraire",
            Self::Scientifique => "scientifique",
            Self::Autre => "autre",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::TOUS.into_iter().find(|g| g.nom() == nom)
    }
}

/// Matière enseignée dans l'établissement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Matiere {
    /// Identifiant technique.
    pub id: Id<Matiere>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Code court, stable : `MATH`, `LC`.
    pub code: String,
    /// Libellé complet.
    pub libelle: String,
    /// Libellé court, pour la colonne du bulletin.
    pub libelle_court: String,
    /// Groupe.
    pub groupe: GroupeMatiere,
    /// Ordre d'affichage.
    pub ordre: i64,
    /// Faux pour une matière que l'école n'enseigne plus : elle reste pour les notes passées.
    pub active: bool,
}

/// Vérifie le code et les libellés d'une matière.
pub fn verifier_matiere(
    code: &str,
    libelle: &str,
    libelle_court: &str,
) -> Result<(), &'static str> {
    let code = code.trim();
    if code.is_empty()
        || code.chars().count() > 10
        || !code
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-')
    {
        return Err("matiere.code_invalide");
    }
    let libelle = libelle.trim();
    if libelle.is_empty() || libelle.chars().count() > 60 {
        return Err("matiere.libelle_invalide");
    }
    let court = libelle_court.trim();
    if court.is_empty() || court.chars().count() > 16 {
        return Err("matiere.libelle_court_invalide");
    }
    Ok(())
}

/// Matière du référentiel livré.
#[derive(Debug, Clone, Copy)]
pub struct MatiereLivree {
    /// Code.
    pub code: &'static str,
    /// Libellé.
    pub libelle: &'static str,
    /// Libellé court.
    pub libelle_court: &'static str,
    /// Groupe.
    pub groupe: GroupeMatiere,
    /// Volume horaire hebdomadaire indicatif au 1er cycle, en minutes (curriculum 2011, Q-37).
    pub minutes_premier_cycle: Option<i64>,
}

const fn livree(
    code: &'static str,
    libelle: &'static str,
    libelle_court: &'static str,
    groupe: GroupeMatiere,
    minutes_premier_cycle: Option<i64>,
) -> MatiereLivree {
    MatiereLivree {
        code,
        libelle,
        libelle_court,
        groupe,
        minutes_premier_cycle,
    }
}

/// Référentiel livré, dans l'ordre d'affichage. Au 1er cycle, les cinq domaines du curriculum
/// 2011 et leurs horaires (§16.5 du blueprint : 12 h de LC, 7 h de SMT, 1 h 30 de SH et d'Arts,
/// 3 h de DP) ; ensuite une liste que l'école corrige (T-27).
pub const MATIERES_LIVREES: &[MatiereLivree] = {
    use GroupeMatiere::{Autre, Litteraire, Scientifique};
    &[
        // Matières du 1er cycle, d'après l'emploi du temps transmis le 13 octobre 2026 (T-38) ;
        // l'ordre du bulletin est celui de `programme_premier_cycle`.
        livree("LANG", "Langage", "Langage", Litteraire, None),
        livree("LECT", "Lecture", "Lecture", Litteraire, None),
        livree("ELOC", "Élocution", "Élocution", Litteraire, None),
        livree("VOCA", "Vocabulaire", "Vocabulaire", Litteraire, None),
        livree("GRAM", "Grammaire", "Grammaire", Litteraire, None),
        livree("CONJ", "Conjugaison", "Conjugaison", Litteraire, None),
        livree(
            "ORTH",
            "Initiation à l'orthographe",
            "Orthographe",
            Litteraire,
            None,
        ),
        livree("DICT", "Dictée", "Dictée", Litteraire, None),
        livree(
            "REDAC",
            "Rédaction (expression écrite)",
            "Rédaction",
            Litteraire,
            None,
        ),
        livree("ECRI", "Écriture", "Écriture", Litteraire, None),
        livree("RECIT", "Récitation", "Récitation", Litteraire, None),
        livree("NUM", "Numération", "Numération", Scientifique, None),
        livree("MES", "Mesure", "Mesure", Scientifique, None),
        livree("GEOM", "Géométrie", "Géométrie", Scientifique, None),
        livree(
            "PROB",
            "Problèmes pratiques",
            "Problèmes",
            Scientifique,
            None,
        ),
        livree(
            "SCOB",
            "Sciences d'observation",
            "Sciences",
            Scientifique,
            None,
        ),
        livree("HIST", "Histoire", "Histoire", Litteraire, None),
        livree("GEO", "Géographie", "Géographie", Litteraire, None),
        livree(
            "SENS",
            "Exercices sensoriels",
            "Ex. sensoriels",
            Autre,
            None,
        ),
        livree("ECOF", "Économie familiale", "Écon. familiale", Autre, None),
        livree("TECH", "Technologie", "Technologie", Autre, None),
        livree("DESS", "Dessin", "Dessin", Autre, None),
        livree("CHANT", "Chant", "Chant", Autre, None),
        livree(
            "LC",
            "Langue et communication",
            "Langue et comm.",
            Litteraire,
            Some(720),
        ),
        livree(
            "SMT",
            "Sciences, mathématiques et technologie",
            "Sc. et maths",
            Scientifique,
            Some(420),
        ),
        livree(
            "SH",
            "Sciences humaines",
            "Sc. humaines",
            Litteraire,
            Some(90),
        ),
        livree("ARTS", "Arts", "Arts", Autre, Some(90)),
        livree(
            "DP",
            "Développement de la personne",
            "Dév. personne",
            Autre,
            Some(180),
        ),
        livree("FR", "Français", "Français", Litteraire, None),
        livree("ANG", "Anglais", "Anglais", Litteraire, None),
        livree("MATH", "Mathématiques", "Maths", Scientifique, None),
        livree("PC", "Physique-chimie", "Phys.-chimie", Scientifique, None),
        livree(
            "SVT",
            "Sciences de la vie et de la terre",
            "SVT",
            Scientifique,
            None,
        ),
        livree("HG", "Histoire-géographie", "Hist.-géo.", Litteraire, None),
        livree(
            "ECM",
            "Éducation civique et morale",
            "ECM",
            Litteraire,
            None,
        ),
        livree("PHILO", "Philosophie", "Philosophie", Litteraire, None),
        livree("ECO", "Économie", "Économie", Litteraire, None),
        livree("EPS", "Éducation physique et sportive", "EPS", Autre, None),
        // Enseignement technique et professionnel (Q-72) ; la technologie est celle du 1er cycle.
        livree(
            "DTECH",
            "Dessin technique",
            "Dessin tech.",
            Scientifique,
            None,
        ),
        livree(
            "TPA",
            "Travaux pratiques d'atelier",
            "TP atelier",
            Autre,
            None,
        ),
        livree("STAGE", "Stage en entreprise", "Stage", Autre, None),
        livree("LEGIS", "Législation", "Législation", Litteraire, None),
        livree(
            "GEST",
            "Gestion et économie d'entreprise",
            "Gestion",
            Litteraire,
            None,
        ),
        livree(
            "HSE",
            "Hygiène et sécurité",
            "Hygiène-sécurité",
            Autre,
            None,
        ),
    ]
};

/// Matières techniques livrées, que l'école ajoute au programme de ses filières.
pub const MATIERES_TECHNIQUES: [&str; 7] =
    ["TECH", "DTECH", "TPA", "STAGE", "LEGIS", "GEST", "HSE"];

/// Vrai pour un niveau du CAP ou du BT (code national `CAP1`…`BT4`).
fn niveau_technique(niveau: &str) -> bool {
    niveau.starts_with("CAP") || niveau.starts_with("BT")
}

/// 1re et 2e années : les matières de leur emploi du temps, dans l'ordre du bulletin.
const PROGRAMME_1_2: &[&str] = &[
    "LANG", "LECT", "ORTH", "ECRI", "MATH", "SENS", "RECIT", "DESS", "CHANT", "TECH", "ECM", "EPS",
];
/// 3e année.
const PROGRAMME_3: &[&str] = &[
    "LECT", "ELOC", "VOCA", "GRAM", "CONJ", "DICT", "REDAC", "ECRI", "RECIT", "NUM", "MES", "GEOM",
    "PROB", "ECOF", "TECH", "DESS", "CHANT", "ECM", "EPS",
];
/// 4e, 5e et 6e années : celles de la 3e, avec l'histoire, la géographie et les sciences.
const PROGRAMME_4_6: &[&str] = &[
    "LECT", "ELOC", "VOCA", "GRAM", "CONJ", "DICT", "REDAC", "ECRI", "RECIT", "NUM", "MES", "GEOM",
    "PROB", "SCOB", "HIST", "GEO", "ECOF", "TECH", "DESS", "CHANT", "ECM", "EPS",
];

/// Matières du programme livré d'une année du 1er cycle (rang 1 à 6), dans l'ordre du bulletin,
/// d'après l'emploi du temps des écoles (réponse du 13 octobre 2026). La morale et l'éducation
/// civique y sont réunies, la dictée préparée et la dictée de contrôle aussi.
pub fn programme_premier_cycle(rang: u8) -> Option<&'static [&'static str]> {
    match rang {
        1 | 2 => Some(PROGRAMME_1_2),
        3 => Some(PROGRAMME_3),
        4..=6 => Some(PROGRAMME_4_6),
        _ => None,
    }
}

/// Vrai si la matière livrée figure au programme par défaut d'un niveau (code national) et d'une
/// série (code, au lycée). L'école corrige ensuite.
pub fn au_programme_par_defaut(matiere: &str, niveau: &str, serie: Option<&str>) -> bool {
    // Au CAP et au BT, les matières générales ; la pratique et le stage dépendent de la filière.
    if niveau_technique(niveau) {
        return matches!(matiere, "FR" | "ANG" | "MATH" | "PC");
    }
    let rang: u8 = match niveau.parse() {
        Ok(r) => r,
        // Jardin : pas de notes (Q-03).
        Err(_) => return false,
    };
    if let Some(programme) = programme_premier_cycle(rang) {
        return programme.contains(&matiere);
    }
    match matiere {
        "FR" | "ANG" | "MATH" | "PC" | "SVT" | "HG" | "EPS" => (7..=12).contains(&rang),
        "ECM" => (7..=10).contains(&rang),
        "PHILO" => (11..=12).contains(&rang),
        "ECO" => matches!(serie, Some("SES" | "TSEco")),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn le_referentiel_livre_est_coherent() {
        let codes: HashSet<_> = MATIERES_LIVREES.iter().map(|m| m.code).collect();
        assert_eq!(codes.len(), MATIERES_LIVREES.len());
        for m in MATIERES_LIVREES {
            assert_eq!(
                verifier_matiere(m.code, m.libelle, m.libelle_court),
                Ok(()),
                "{}",
                m.code
            );
        }
        // 26 h 30 par semaine au 1er cycle, dont 1 h 30 de récréation : 25 h de cours.
        let minutes: i64 = MATIERES_LIVREES
            .iter()
            .filter_map(|m| m.minutes_premier_cycle)
            .sum();
        assert_eq!(minutes, 25 * 60);
    }

    #[test]
    fn le_programme_par_defaut_suit_les_cycles() {
        // Au 1er cycle, les matières de l'emploi du temps ; les domaines restent disponibles.
        assert!(au_programme_par_defaut("LANG", "1", None));
        assert!(au_programme_par_defaut("MATH", "2", None));
        assert!(!au_programme_par_defaut("HIST", "3", None));
        assert!(au_programme_par_defaut("HIST", "4", None));
        assert!(au_programme_par_defaut("ECM", "6", None));
        assert!(!au_programme_par_defaut("LC", "1", None));
        assert!(!au_programme_par_defaut("LC", "7", None));
        for rang in 1..=6 {
            for code in programme_premier_cycle(rang).unwrap_or_default() {
                assert!(MATIERES_LIVREES.iter().any(|m| m.code == *code), "{code}");
            }
        }
        assert!(au_programme_par_defaut("MATH", "9", None));
        assert!(!au_programme_par_defaut("MATH", "PS", None));
        assert!(au_programme_par_defaut("PHILO", "12", Some("TSE")));
        assert!(!au_programme_par_defaut("PHILO", "10", None));
        assert!(au_programme_par_defaut("ECO", "11", Some("SES")));
        assert!(!au_programme_par_defaut("ECO", "11", Some("S")));
        // Au CAP et au BT, les matières générales ; la pratique s'ajoute par filière.
        assert!(au_programme_par_defaut("FR", "CAP1", Some("ELEC")));
        assert!(au_programme_par_defaut("PC", "BT4", None));
        assert!(!au_programme_par_defaut("SVT", "BT1", None));
        for code in MATIERES_TECHNIQUES {
            assert!(MATIERES_LIVREES.iter().any(|m| m.code == code), "{code}");
            assert!(
                !au_programme_par_defaut(code, "CAP1", Some("ELEC")),
                "{code}"
            );
        }
    }

    #[test]
    fn une_matiere_se_controle() {
        assert_eq!(verifier_matiere("INFO", "Informatique", "Info"), Ok(()));
        assert_eq!(
            verifier_matiere("info", "Informatique", "Info"),
            Err("matiere.code_invalide")
        );
        assert_eq!(
            verifier_matiere("INFO", " ", "Info"),
            Err("matiere.libelle_invalide")
        );
        assert_eq!(
            verifier_matiere("INFO", "Informatique", "Informatique appliquée"),
            Err("matiere.libelle_court_invalide")
        );
    }
}
