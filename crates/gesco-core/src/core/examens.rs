//! Résultats aux examens (SPEC_PHASE_9 §8.5) : élève par élève — inscrit, présent, admis,
//! mention —, ou en totaux par sexe. Tout est facultatif. Un résultat se range sans contradiction :
//! un élève admis s'est présenté, un élève présent était inscrit, sans que rien ne soit refusé.

use serde::{Deserialize, Serialize};

use crate::scolarite::{DiplomeTerminal, Niveau};

/// Longueur maximale d'une mention.
pub const MENTION_MAX: usize = 40;
/// Clé du réglage « L'école présente ses élèves de 6e année à l'examen d'entrée en 7e ».
pub const ENTREE_7E: &str = "examens.entree_7e";

/// Examen dont l'école note les résultats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum Examen {
    /// Examen d'entrée en 7e année, en 6e année, quand l'école l'organise.
    #[serde(rename = "entree_7e")]
    Entree7e,
    /// Diplôme d'études fondamentales, en 9e année.
    Def,
    /// Baccalauréat.
    Bac,
    /// Certificat d'aptitude professionnelle.
    Cap,
    /// Brevet de technicien.
    Bt,
}

impl Examen {
    /// Tous les examens, dans l'ordre d'affichage.
    pub const TOUS: [Self; 5] = [Self::Entree7e, Self::Def, Self::Bac, Self::Cap, Self::Bt];

    /// Nom stable, rangé dans la base.
    pub const fn nom(self) -> &'static str {
        match self {
            Self::Entree7e => "entree_7e",
            Self::Def => "def",
            Self::Bac => "bac",
            Self::Cap => "cap",
            Self::Bt => "bt",
        }
    }

    /// Examen d'un nom de la base.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::TOUS.into_iter().find(|e| e.nom() == nom)
    }

    /// Examen que passent les élèves d'un niveau : son diplôme terminal, ou l'entrée en 7e pour la
    /// 6e année quand l'école l'organise.
    pub fn du_niveau(niveau: &Niveau, entree_7e: bool) -> Option<Self> {
        match niveau.diplome_terminal {
            DiplomeTerminal::Def => Some(Self::Def),
            DiplomeTerminal::Bac => Some(Self::Bac),
            DiplomeTerminal::Cap => Some(Self::Cap),
            DiplomeTerminal::Bt => Some(Self::Bt),
            DiplomeTerminal::Aucun => (entree_7e && niveau.code == "6").then_some(Self::Entree7e),
        }
    }
}

/// Résultat d'un élève, rangé sans contradiction : admis entraîne présent, présent entraîne
/// inscrit ; ne pas être inscrit efface la présence et l'admission, ne pas être présent efface
/// l'admission. Une case laissée vide reste vide.
pub fn ranger_resultat(
    inscrit: Option<bool>,
    present: Option<bool>,
    admis: Option<bool>,
) -> (Option<bool>, Option<bool>, Option<bool>) {
    let mut inscrit = inscrit;
    let mut present = present;
    let mut admis = admis;
    if admis == Some(true) {
        present = Some(true);
    }
    if present == Some(true) {
        inscrit = Some(true);
    }
    if inscrit == Some(false) {
        present = Some(false);
        admis = Some(false);
    }
    if present == Some(false) {
        admis = Some(false);
    }
    (inscrit, present, admis)
}

/// Mention nettoyée, ou la clé du refus.
pub fn verifier_mention(mention: &str) -> Result<String, &'static str> {
    let m = mention.split_whitespace().collect::<Vec<_>>().join(" ");
    if m.chars().count() > MENTION_MAX {
        return Err("examens.mention_trop_longue");
    }
    Ok(m)
}

/// Taux de réussite lisible, une décimale, virgule française : « 85,7 % » ; rien sans présent.
pub fn taux(admis: u32, presentes: u32) -> Option<String> {
    if presentes == 0 {
        return None;
    }
    let dixiemes = (u64::from(admis) * 1000 + u64::from(presentes) / 2) / u64::from(presentes);
    Some(format!("{},{} %", dixiemes / 10, dixiemes % 10))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_resultat_se_range_sans_contradiction() {
        assert_eq!(
            ranger_resultat(None, None, Some(true)),
            (Some(true), Some(true), Some(true))
        );
        assert_eq!(
            ranger_resultat(Some(false), Some(true), None),
            (Some(true), Some(true), None)
        );
        assert_eq!(
            ranger_resultat(Some(false), None, Some(false)),
            (Some(false), Some(false), Some(false))
        );
        assert_eq!(
            ranger_resultat(Some(true), Some(false), None),
            (Some(true), Some(false), Some(false))
        );
        assert_eq!(ranger_resultat(None, None, None), (None, None, None));
    }

    #[test]
    fn le_taux_s_ecrit_a_la_francaise() {
        assert_eq!(taux(6, 7), Some("85,7 %".into()));
        assert_eq!(taux(1, 1), Some("100,0 %".into()));
        assert_eq!(taux(0, 3), Some("0,0 %".into()));
        assert_eq!(taux(0, 0), None);
    }

    #[test]
    fn les_noms_sont_stables() {
        for e in Examen::TOUS {
            assert_eq!(Examen::depuis_nom(e.nom()), Some(e));
        }
        assert!(verifier_mention(&"x".repeat(41)).is_err());
        assert_eq!(verifier_mention(" Assez  bien "), Ok("Assez bien".into()));
    }
}
