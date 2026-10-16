//! Réglages de la paie de l'école (SPEC_PHASE_7 §2, Q-73, Q-74) : mois payés d'un engagement
//! mensuel, mois de vacances, prorata d'un mois incomplet.

use serde::{Deserialize, Serialize};

use crate::rh::Mois;

/// Paramètre : un nouvel engagement mensuel est payé douze mois (vrai) ou sur l'année scolaire.
pub const PARAMETRE_DOUZE_MOIS: &str = "paie.douze_mois";
/// Paramètre : mois de vacances, non payés aux engagements de l'année scolaire (`7,8,9`).
pub const PARAMETRE_MOIS_VACANCES: &str = "paie.mois_vacances";
/// Paramètre : règle du prorata d'un mois incomplet.
pub const PARAMETRE_PRORATA: &str = "paie.prorata";

/// Mois de vacances livrés : juillet, août et septembre, soit neuf mois payés (T-52).
pub const MOIS_VACANCES_DEFAUT: &str = "7,8,9";

/// Règle du prorata d'un mois incomplet (Q-74) ; la retenue d'une journée d'absence la suit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum Prorata {
    /// Trente trentièmes pour un mois entier, moins un par jour non couvert.
    Trentieme,
    /// Jours couverts sur jours du mois.
    Calendrier,
    /// Jours ouvrables couverts sur jours ouvrables du mois.
    Ouvrables,
}

impl Prorata {
    /// Les trois règles.
    pub const TOUTES: [Self; 3] = [Self::Trentieme, Self::Calendrier, Self::Ouvrables];

    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Trentieme => "trentieme",
            Self::Calendrier => "calendrier",
            Self::Ouvrables => "ouvrables",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::TOUTES.into_iter().find(|p| p.nom() == nom.trim())
    }
}

/// Lit les mois de vacances : numéros de 1 à 12 séparés par des virgules, sans doublon ; vide,
/// aucun. Rend les mois triés.
pub fn lire_mois_vacances(texte: &str) -> Option<Vec<u8>> {
    let mut mois = Vec::new();
    for morceau in texte.split(',').map(str::trim).filter(|m| !m.is_empty()) {
        let m: u8 = morceau.parse().ok()?;
        if !(1..=12).contains(&m) || mois.contains(&m) {
            return None;
        }
        mois.push(m);
    }
    mois.sort_unstable();
    Some(mois)
}

/// Écrit les mois de vacances comme le paramètre les garde.
pub fn ecrire_mois_vacances(mois: &[u8]) -> String {
    let mut tries = mois.to_vec();
    tries.sort_unstable();
    tries.dedup();
    tries
        .iter()
        .map(u8::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

/// Vrai si un engagement mensuel paie ce mois : toujours s'il est payé douze mois, sinon hors
/// des mois de vacances de l'école.
pub fn mois_paye(paye_douze_mois: bool, mois_vacances: &[u8], mois: Mois) -> bool {
    paye_douze_mois || !mois_vacances.contains(&mois.numero())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_mois_de_vacances_se_lisent() {
        assert_eq!(lire_mois_vacances("7,8,9"), Some(vec![7, 8, 9]));
        assert_eq!(lire_mois_vacances(" 9, 7 ,8 "), Some(vec![7, 8, 9]));
        assert_eq!(lire_mois_vacances(""), Some(vec![]));
        assert_eq!(lire_mois_vacances("7,7"), None);
        assert_eq!(lire_mois_vacances("13"), None);
        assert_eq!(lire_mois_vacances("juillet"), None);
        assert_eq!(ecrire_mois_vacances(&[9, 7, 8, 7]), "7,8,9");
    }

    #[test]
    fn neuf_mois_ou_douze() {
        let mois = |t| Mois::analyser(t).unwrap_or_else(|| unreachable!());
        assert!(mois_paye(true, &[7, 8, 9], mois("2027-08")));
        assert!(!mois_paye(false, &[7, 8, 9], mois("2027-08")));
        assert!(mois_paye(false, &[7, 8, 9], mois("2027-06")));
        assert_eq!(Prorata::depuis_nom("ouvrables"), Some(Prorata::Ouvrables));
        assert_eq!(Prorata::depuis_nom("au_hasard"), None);
    }
}
