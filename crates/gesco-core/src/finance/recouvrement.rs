//! Recouvrement (SPEC_PHASE_3 §9, UC-05) : ce qu'un élève doit à ce jour, depuis quand, et dans
//! quelle tranche d'ancienneté (T-20). Règles pures, sans E/S (R-03).

use serde::{Deserialize, Serialize};

use crate::commun::{DateCivile, ErreurMontant, Montant};

/// Tranche d'ancienneté d'un impayé, comptée depuis la plus ancienne échéance passée et non
/// soldée (T-20).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, specta::Type,
)]
pub enum TrancheAnciennete {
    /// De 0 à 30 jours.
    #[serde(rename = "de_0_a_30")]
    De0A30,
    /// De 31 à 60 jours.
    #[serde(rename = "de_31_a_60")]
    De31A60,
    /// De 61 à 90 jours.
    #[serde(rename = "de_61_a_90")]
    De61A90,
    /// Plus de 90 jours.
    #[serde(rename = "plus_de_90")]
    PlusDe90,
}

impl TrancheAnciennete {
    /// Toutes les tranches, de la plus récente à la plus ancienne.
    pub const TOUTES: &'static [Self] =
        &[Self::De0A30, Self::De31A60, Self::De61A90, Self::PlusDe90];

    /// Tranche d'un nombre de jours de retard ; une échéance du jour compte pour 0.
    pub const fn depuis_jours(jours: i64) -> Self {
        if jours <= 30 {
            Self::De0A30
        } else if jours <= 60 {
            Self::De31A60
        } else if jours <= 90 {
            Self::De61A90
        } else {
            Self::PlusDe90
        }
    }
}

/// Impayé d'une facture à une date.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Impaye {
    /// Reste dû sur les échéances passées (date du jour comprise).
    pub echu: Montant,
    /// Reste dû total, échéances à venir comprises.
    pub total: Montant,
    /// Plus ancienne échéance passée et non soldée.
    pub plus_ancienne: DateCivile,
    /// Jours écoulés depuis cette échéance.
    pub jours: i64,
    /// Tranche d'ancienneté.
    pub tranche: TrancheAnciennete,
}

/// Impayé d'une facture d'après ses échéances dues `(date, reste)`, à la date `aujourd_hui` :
/// rien si aucune échéance passée n'a de reste.
pub fn impaye(
    echeances: &[(DateCivile, Montant)],
    aujourd_hui: DateCivile,
) -> Result<Option<Impaye>, ErreurMontant> {
    let mut echu = Montant::ZERO;
    let mut total = Montant::ZERO;
    let mut plus_ancienne: Option<DateCivile> = None;
    for (date, reste) in echeances {
        if *reste <= Montant::ZERO {
            continue;
        }
        total = total.ajouter(*reste)?;
        if *date <= aujourd_hui {
            echu = echu.ajouter(*reste)?;
            if plus_ancienne.is_none_or(|p| *date < p) {
                plus_ancienne = Some(*date);
            }
        }
    }
    Ok(plus_ancienne.map(|p| {
        let jours = aujourd_hui.jours_depuis_epoque() - p.jours_depuis_epoque();
        Impaye {
            echu,
            total,
            plus_ancienne: p,
            jours,
            tranche: TrancheAnciennete::depuis_jours(jours),
        }
    }))
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn d(texte: &str) -> DateCivile {
        DateCivile::analyser(texte).unwrap_or_else(|| unreachable!())
    }

    fn f(v: i64) -> Montant {
        Montant::depuis_francs(v)
    }

    #[test]
    fn les_tranches_suivent_t_20() {
        let tranches: Vec<_> = [0, 30, 31, 60, 61, 90, 91, 400]
            .into_iter()
            .map(TrancheAnciennete::depuis_jours)
            .collect();
        assert_eq!(
            tranches,
            vec![
                TrancheAnciennete::De0A30,
                TrancheAnciennete::De0A30,
                TrancheAnciennete::De31A60,
                TrancheAnciennete::De31A60,
                TrancheAnciennete::De61A90,
                TrancheAnciennete::De61A90,
                TrancheAnciennete::PlusDe90,
                TrancheAnciennete::PlusDe90,
            ]
        );
    }

    #[test]
    fn l_impaye_part_de_la_plus_ancienne_echeance_non_soldee() {
        let echeances = [
            (d("2026-10-01"), f(0)),
            (d("2026-10-15"), f(20_000)),
            (d("2027-01-15"), f(45_000)),
            (d("2027-04-15"), f(45_000)),
        ];
        let i = impaye(&echeances, d("2027-01-20")).ok().flatten();
        assert_eq!(
            i,
            Some(Impaye {
                echu: f(65_000),
                total: f(110_000),
                plus_ancienne: d("2026-10-15"),
                jours: 97,
                tranche: TrancheAnciennete::PlusDe90,
            })
        );
        // Rien d'échu avant la première échéance non soldée ; une échéance du jour compte.
        assert_eq!(impaye(&echeances, d("2026-10-14")), Ok(None));
        let du_jour = impaye(&echeances, d("2026-10-15")).ok().flatten();
        assert_eq!(du_jour.map(|i| (i.jours, i.echu)), Some((0, f(20_000))));
    }

    proptest! {
        #[test]
        fn l_echu_ne_depasse_jamais_le_total(
            restes in prop::collection::vec((0i64..400, 0i64..200_000), 0..12),
            decalage in 0i64..400,
        ) {
            let origine = d("2026-09-01").jours_depuis_epoque();
            let date = |n: i64| {
                DateCivile::du_jour(crate::commun::Horodatage::depuis_millis_epoque((origine + n) * 86_400_000))
                    .unwrap_or_else(|| unreachable!())
            };
            let echeances: Vec<_> = restes.iter().map(|(j, r)| (date(*j), f(*r))).collect();
            let aujourd_hui = date(decalage);
            match impaye(&echeances, aujourd_hui) {
                Ok(Some(i)) => {
                    prop_assert!(i.echu <= i.total);
                    prop_assert!(i.echu > Montant::ZERO);
                    prop_assert!(i.jours >= 0);
                    prop_assert_eq!(i.tranche, TrancheAnciennete::depuis_jours(i.jours));
                    prop_assert_eq!(i.plus_ancienne.jours_depuis_epoque() - origine, decalage - i.jours);
                }
                Ok(None) => prop_assert!(echeances.iter().all(|(dt, r)| *dt > aujourd_hui || *r <= Montant::ZERO)),
                Err(_) => prop_assert!(false),
            }
        }
    }
}
