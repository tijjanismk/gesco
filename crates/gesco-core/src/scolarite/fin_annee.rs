//! Fin d'année (SPEC_PHASE_10 §3, UC-06) : moyenne de passage de chaque cycle, décision proposée
//! d'après la moyenne annuelle, niveau où l'élève revient l'année suivante.
//!
//! La règle de passage vient du CAP ou de l'académie ; l'école la reporte dans le réglage de son
//! cycle (T-65). GESCO ne fait que proposer : rien ne passe sans la validation de la direction
//! (Q-30).

use super::academique::Cycle;
use super::inscription::DecisionFinAnnee;
use crate::pedagogie::regle::bareme_du_cycle;

/// Préfixe des réglages de moyenne de passage, un par cycle noté : `passage.premier_cycle`…
pub const PREFIXE_PASSAGE: &str = "passage.";

/// Réglage de la moyenne de passage d'un cycle ; aucun au jardin, où l'enfant passe toujours.
pub fn cle_passage(cycle: Cycle) -> Option<String> {
    bareme_du_cycle(cycle).map(|_| format!("{PREFIXE_PASSAGE}{}", cycle.nom()))
}

/// Moyenne de passage livrée : la moitié du barème, 5 sur 10 au 1er cycle, 10 sur 20 ensuite.
pub fn passage_par_defaut(cycle: Cycle) -> Option<i32> {
    bareme_du_cycle(cycle).map(|b| b / 2)
}

/// Vrai si une moyenne de passage sort du barème de son cycle.
pub fn passage_hors_bornes(cle: &str, valeur: i64) -> bool {
    let bareme = cle
        .strip_prefix(PREFIXE_PASSAGE)
        .and_then(Cycle::depuis_nom)
        .and_then(bareme_du_cycle);
    bareme.is_none_or(|b| !(0..=i64::from(b)).contains(&valeur))
}

/// Décision proposée : au jardin, l'enfant passe ; ailleurs, admis à la moyenne de passage,
/// redoublant en dessous ; sans moyenne annuelle, rien n'est proposé.
pub fn proposer(cycle: Cycle, moyenne_annuelle: Option<i32>, passage: i32) -> DecisionFinAnnee {
    if cycle == Cycle::Jardin {
        return DecisionFinAnnee::Admis;
    }
    match moyenne_annuelle {
        Some(m) if m >= passage => DecisionFinAnnee::Admis,
        Some(_) => DecisionFinAnnee::Redouble,
        None => DecisionFinAnnee::NonStatue,
    }
}

/// Niveaux qui se suivent, par leur code national : du jardin au lycée, puis le CAP et le BT.
const PROGRESSIONS: &[&[&str]] = &[
    &[
        "PS", "MS", "GS", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12",
    ],
    &["CAP1", "CAP2"],
    &["BT1", "BT2", "BT3", "BT4"],
];

/// Niveau qui suit, par son code ; aucun après la 12e année, le CAP 2 ou le BT 4.
pub fn code_suivant(code: &str) -> Option<&'static str> {
    PROGRESSIONS.iter().find_map(|suite| {
        let rang = suite.iter().position(|c| *c == code)?;
        suite.get(rang + 1).copied()
    })
}

/// Niveau où l'élève revient l'année suivante : le suivant s'il est admis, le même s'il
/// redouble. Aucun pour un exclu, une décision en attente, ou l'admis du dernier niveau.
pub fn code_de_retour(code: &str, decision: DecisionFinAnnee) -> Option<String> {
    match decision {
        DecisionFinAnnee::Admis => code_suivant(code).map(str::to_owned),
        DecisionFinAnnee::Redouble => Some(code.to_owned()),
        DecisionFinAnnee::NonStatue | DecisionFinAnnee::Exclu | DecisionFinAnnee::Rattrapage => {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use DecisionFinAnnee::{Admis, Exclu, NonStatue, Rattrapage, Redouble};

    #[test]
    fn la_decision_suit_la_moyenne_de_passage_du_cycle() {
        let premier = passage_par_defaut(Cycle::PremierCycle).unwrap_or_default();
        assert_eq!(premier, 500);
        assert_eq!(passage_par_defaut(Cycle::Lycee), Some(1000));
        assert_eq!(passage_par_defaut(Cycle::Jardin), None);
        assert_eq!(proposer(Cycle::PremierCycle, Some(500), premier), Admis);
        assert_eq!(proposer(Cycle::PremierCycle, Some(499), premier), Redouble);
        assert_eq!(proposer(Cycle::PremierCycle, None, premier), NonStatue);
        // Au jardin, sans moyenne, l'enfant passe à la section suivante.
        assert_eq!(proposer(Cycle::Jardin, None, 0), Admis);
    }

    #[test]
    fn la_moyenne_de_passage_reste_dans_le_bareme() {
        assert_eq!(
            cle_passage(Cycle::SecondCycle).as_deref(),
            Some("passage.second_cycle")
        );
        assert_eq!(cle_passage(Cycle::Jardin), None);
        assert!(!passage_hors_bornes("passage.premier_cycle", 1000));
        assert!(passage_hors_bornes("passage.premier_cycle", 1001));
        assert!(!passage_hors_bornes("passage.technique", 2000));
        assert!(passage_hors_bornes("passage.lycee", -1));
        assert!(passage_hors_bornes("passage.jardin", 0));
    }

    #[test]
    fn l_eleve_revient_au_niveau_suivant_ou_au_meme() {
        assert_eq!(code_suivant("GS"), Some("1"));
        assert_eq!(code_suivant("6"), Some("7"));
        assert_eq!(code_suivant("12"), None);
        assert_eq!(code_suivant("CAP1"), Some("CAP2"));
        assert_eq!(code_suivant("CAP2"), None);
        assert_eq!(code_suivant("BT3"), Some("BT4"));
        assert_eq!(code_de_retour("MS", Admis).as_deref(), Some("GS"));
        assert_eq!(code_de_retour("4", Redouble).as_deref(), Some("4"));
        assert_eq!(code_de_retour("4", Exclu), None);
        assert_eq!(code_de_retour("4", NonStatue), None);
        assert_eq!(code_de_retour("4", Rattrapage), None);
        assert_eq!(code_de_retour("12", Admis), None);
    }
}
