//! Recouvrement (SPEC_PHASE_3 §9, UC-05) : ce qu'un élève doit à ce jour, depuis quand, et dans
//! quelle tranche d'ancienneté (T-20). Règles pures, sans E/S (R-03).

use serde::{Deserialize, Serialize};

use crate::commun::{DateCivile, ErreurMontant, Montant};

/// Tranche d'ancienneté d'un impayé, comptée depuis la plus ancienne échéance passée et non
/// soldée (T-20). Ses bornes en jours sont celles que l'école règle ([`Tranches`]).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, specta::Type,
)]
#[serde(rename_all = "snake_case")]
pub enum TrancheAnciennete {
    /// De 0 jour à la première limite.
    Premiere,
    /// Jusqu'à la deuxième limite.
    Deuxieme,
    /// Jusqu'à la troisième limite.
    Troisieme,
    /// Au-delà de la troisième limite.
    Derniere,
}

impl TrancheAnciennete {
    /// Toutes les tranches, de la plus récente à la plus ancienne.
    pub const TOUTES: &'static [Self] = &[
        Self::Premiere,
        Self::Deuxieme,
        Self::Troisieme,
        Self::Derniere,
    ];
}

/// Réglage de l'école qui porte les limites des tranches.
pub const PARAMETRE_TRANCHES: &str = "facturation.tranches_impayes";

/// Limites des tranches d'ancienneté, en jours, que chaque école règle : certaines renvoient un
/// élève dès 7 à 15 jours de retard, d'autres attendent pour ne pas harceler les familles
/// (porteur du projet, 31 octobre 2026). Par défaut 15, 30 et 60 : 0 à 15 jours, 16 à 30, 31 à
/// 60, plus de 60.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tranches {
    limites: [i64; 3],
}

impl Tranches {
    /// Limites livrées.
    pub const DEFAUT: Self = Self {
        limites: [15, 30, 60],
    };

    /// Plus grande limite : un an.
    pub const LIMITE_MAX: i64 = 365;

    /// Lit le réglage `15,30,60` : trois nombres de jours croissants, de 1 à un an.
    pub fn lire(texte: &str) -> Option<Self> {
        let nombres: Vec<i64> = texte
            .split(',')
            .map(|n| n.trim().parse().ok())
            .collect::<Option<_>>()?;
        let limites: [i64; 3] = nombres.try_into().ok()?;
        let croissantes = limites.windows(2).all(|p| matches!(p, [a, b] if a < b));
        (croissantes && limites.iter().all(|n| (1..=Self::LIMITE_MAX).contains(n)))
            .then_some(Self { limites })
    }

    /// Le réglage tel qu'il s'enregistre.
    pub fn texte(&self) -> String {
        let [a, b, c] = self.limites;
        format!("{a},{b},{c}")
    }

    /// Tranche d'un nombre de jours de retard ; une échéance du jour compte pour 0.
    pub const fn classer(&self, jours: i64) -> TrancheAnciennete {
        let [a, b, c] = self.limites;
        if jours <= a {
            TrancheAnciennete::Premiere
        } else if jours <= b {
            TrancheAnciennete::Deuxieme
        } else if jours <= c {
            TrancheAnciennete::Troisieme
        } else {
            TrancheAnciennete::Derniere
        }
    }

    /// Bornes d'une tranche en jours : de, et jusqu'à (comprise) ; sans fin pour la dernière,
    /// qui commence après la troisième limite.
    pub const fn bornes(&self, tranche: TrancheAnciennete) -> (i64, Option<i64>) {
        let [a, b, c] = self.limites;
        match tranche {
            TrancheAnciennete::Premiere => (0, Some(a)),
            TrancheAnciennete::Deuxieme => (a + 1, Some(b)),
            TrancheAnciennete::Troisieme => (b + 1, Some(c)),
            TrancheAnciennete::Derniere => (c, None),
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
    /// Tranche d'ancienneté, selon les limites livrées ; la liste des impayés la reclasse selon
    /// celles de l'école.
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
            tranche: Tranches::DEFAUT.classer(jours),
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
    fn les_tranches_livrees_sont_15_30_60() {
        use TrancheAnciennete as T;
        let tranches: Vec<_> = [0, 15, 16, 30, 31, 60, 61, 400]
            .into_iter()
            .map(|j| Tranches::DEFAUT.classer(j))
            .collect();
        assert_eq!(
            tranches,
            vec![
                T::Premiere,
                T::Premiere,
                T::Deuxieme,
                T::Deuxieme,
                T::Troisieme,
                T::Troisieme,
                T::Derniere,
                T::Derniere,
            ]
        );
        assert_eq!(Tranches::DEFAUT.bornes(T::Deuxieme), (16, Some(30)));
        assert_eq!(Tranches::DEFAUT.bornes(T::Derniere), (60, None));
    }

    #[test]
    fn l_ecole_regle_ses_tranches() {
        let strictes = Tranches::lire("7, 15,30").unwrap_or_else(|| unreachable!());
        assert_eq!(strictes.texte(), "7,15,30");
        assert_eq!(strictes.classer(8), TrancheAnciennete::Deuxieme);
        assert_eq!(strictes.classer(31), TrancheAnciennete::Derniere);
        assert_eq!(
            Tranches::lire(&Tranches::DEFAUT.texte()),
            Some(Tranches::DEFAUT)
        );
        // Trois nombres croissants, d'un jour à un an.
        for refuse in [
            "",
            "15,30",
            "15,30,60,90",
            "30,15,60",
            "15,15,60",
            "0,30,60",
            "15,30,400",
            "a,b,c",
        ] {
            assert_eq!(Tranches::lire(refuse), None, "{refuse}");
        }
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
                tranche: TrancheAnciennete::Derniere,
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
                    prop_assert_eq!(i.tranche, Tranches::DEFAUT.classer(i.jours));
                    prop_assert_eq!(i.plus_ancienne.jours_depuis_epoque() - origine, decalage - i.jours);
                }
                Ok(None) => prop_assert!(echeances.iter().all(|(dt, r)| *dt > aujourd_hui || *r <= Montant::ZERO)),
                Err(_) => prop_assert!(false),
            }
        }
    }
}
