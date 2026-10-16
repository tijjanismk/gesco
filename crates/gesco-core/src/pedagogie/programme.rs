//! Programme d'un niveau (et d'une série au lycée) pour une année (SPEC_PHASE_4 §2.2) : les
//! matières enseignées, leur coefficient et leur volume horaire indicatif (Q-37). Pas de
//! coefficient au 1er cycle (Q-16) ; ailleurs, l'école les saisit (T-27).

use serde::{Deserialize, Serialize};

use crate::commun::Id;
use crate::core::{AnneeScolaire, Etablissement};
use crate::pedagogie::Matiere;
use crate::scolarite::{Cycle, Niveau, Serie};

/// Matière au programme d'un niveau.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct LigneProgramme {
    /// Identifiant technique.
    pub id: Id<LigneProgramme>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Année.
    pub annee_id: Id<AnneeScolaire>,
    /// Niveau.
    pub niveau_id: Id<Niveau>,
    /// Série, au lycée.
    pub serie_id: Option<Id<Serie>>,
    /// Matière.
    pub matiere_id: Id<Matiere>,
    /// Coefficient ; absent au 1er cycle, et tant que l'école ne l'a pas saisi ailleurs.
    pub coefficient: Option<i64>,
    /// Volume horaire hebdomadaire indicatif, en minutes ; jamais bloquant (Q-37).
    pub minutes_hebdo: Option<i64>,
    /// Ordre sur le bulletin.
    pub ordre: i64,
}

/// Ce que l'école saisit pour une matière du programme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct SaisieProgramme {
    /// Matière.
    pub matiere_id: Id<Matiere>,
    /// Coefficient.
    pub coefficient: Option<i64>,
    /// Minutes par semaine.
    pub minutes_hebdo: Option<i64>,
}

/// Vérifie le programme d'un niveau : chaque matière une fois, coefficient de 1 à 20 hors du
/// 1er cycle et aucun au 1er cycle, volume de 0 à 50 heures.
pub fn verifier_programme(cycle: Cycle, lignes: &[SaisieProgramme]) -> Result<(), &'static str> {
    if cycle == Cycle::Jardin {
        return Err("programme.jardin_sans_notes");
    }
    for (i, l) in lignes.iter().enumerate() {
        if lignes.iter().take(i).any(|m| m.matiere_id == l.matiere_id) {
            return Err("programme.matiere_en_double");
        }
        match (cycle, l.coefficient) {
            (Cycle::PremierCycle, Some(_)) => return Err("programme.coefficient_premier_cycle"),
            (_, Some(c)) if !(1..=20).contains(&c) => return Err("programme.coefficient_invalide"),
            _ => {}
        }
        if l.minutes_hebdo.is_some_and(|m| !(0..=3000).contains(&m)) {
            return Err("programme.volume_invalide");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ligne(n: u128, coefficient: Option<i64>) -> SaisieProgramme {
        SaisieProgramme {
            matiere_id: Id::depuis_ulid(ulid::Ulid::from(n)),
            coefficient,
            minutes_hebdo: Some(240),
        }
    }

    #[test]
    fn un_programme_se_controle() {
        assert_eq!(
            verifier_programme(Cycle::SecondCycle, &[ligne(1, Some(4)), ligne(2, None)]),
            Ok(())
        );
        assert_eq!(
            verifier_programme(Cycle::PremierCycle, &[ligne(1, None)]),
            Ok(())
        );
        assert_eq!(
            verifier_programme(Cycle::PremierCycle, &[ligne(1, Some(2))]),
            Err("programme.coefficient_premier_cycle")
        );
        assert_eq!(
            verifier_programme(Cycle::Lycee, &[ligne(1, Some(0))]),
            Err("programme.coefficient_invalide")
        );
        assert_eq!(
            verifier_programme(Cycle::Lycee, &[ligne(1, Some(2)), ligne(1, Some(3))]),
            Err("programme.matiere_en_double")
        );
        let mut trop = ligne(3, None);
        trop.minutes_hebdo = Some(4000);
        assert_eq!(
            verifier_programme(Cycle::Lycee, &[trop]),
            Err("programme.volume_invalide")
        );
        assert_eq!(
            verifier_programme(Cycle::Jardin, &[]),
            Err("programme.jardin_sans_notes")
        );
    }
}
