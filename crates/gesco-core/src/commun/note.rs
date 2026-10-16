//! Note en centièmes de point, entier signé 32 bits (R-11bis, RC-6).
//!
//! Une note de 13,50 sur 20 vaut `1350` sur `2000`. La note est stockée telle qu'écrite par le
//! correcteur, avec le barème de son évaluation (RC-2). La normalisation vers le barème du niveau
//! n'intervient qu'au calcul de moyenne, par une seule division entière finale (RC-1, RC-7).
//!
//! ```compile_fail
//! use gesco_core::commun::Note;
//! let _ = Note::nouvelle(13.5, 20.0);
//! ```

use std::cmp::Ordering;
use std::fmt;

use serde::{Deserialize, Serialize};

use super::erreur::ErreurNote;

/// Note attribuée sur un barème, toutes deux en centièmes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Note {
    valeur_centiemes: i32,
    bareme_centiemes: i32,
}

impl Note {
    /// Construit une note en vérifiant `0 ≤ valeur ≤ barème` et `barème > 0`.
    pub fn nouvelle(valeur_centiemes: i32, bareme_centiemes: i32) -> Result<Self, ErreurNote> {
        if bareme_centiemes <= 0 {
            return Err(ErreurNote::BaremeInvalide(bareme_centiemes));
        }
        if valeur_centiemes < 0 || valeur_centiemes > bareme_centiemes {
            return Err(ErreurNote::HorsBareme {
                valeur_centiemes,
                bareme_centiemes,
            });
        }
        Ok(Self {
            valeur_centiemes,
            bareme_centiemes,
        })
    }

    /// Valeur en centièmes.
    pub const fn valeur_centiemes(&self) -> i32 {
        self.valeur_centiemes
    }

    /// Barème en centièmes.
    pub const fn bareme_centiemes(&self) -> i32 {
        self.bareme_centiemes
    }

    /// Compare deux notes de même barème. Deux barèmes différents ne se comparent pas sans
    /// normalisation explicite.
    pub fn comparer(&self, autre: &Self) -> Result<Ordering, ErreurNote> {
        if self.bareme_centiemes != autre.bareme_centiemes {
            return Err(ErreurNote::BaremesDifferents(
                self.bareme_centiemes,
                autre.bareme_centiemes,
            ));
        }
        Ok(self.valeur_centiemes.cmp(&autre.valeur_centiemes))
    }

    /// Ramène la note au barème demandé par une division entière, c'est-à-dire **tronquée** au
    /// centième (RC-1). Pour une moyenne de plusieurs notes, passer par [`SommePonderee`] afin
    /// que la troncature n'intervienne qu'une fois, à la fin (RC-7).
    pub fn ramener_a(&self, bareme_cible_centiemes: i32) -> Result<Self, ErreurNote> {
        SommePonderee::nouvelle()
            .ajouter(*self, 1)?
            .moyenne(bareme_cible_centiemes)
    }

    /// Affichage à deux décimales, virgule française : `13,50`.
    pub fn formater(&self) -> String {
        format!(
            "{},{:02}",
            self.valeur_centiemes / 100,
            self.valeur_centiemes % 100
        )
    }
}

impl fmt::Display for Note {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.formater(), self.bareme_centiemes / 100)
    }
}

/// Accumulateur `Σ(note × pondération)` et `Σ(barème × pondération)` en `i64`.
///
/// C'est la forme exacte de la formule du §8.4bis : la moyenne sur un barème cible vaut
/// `points × barème_cible ÷ barème_total`, une seule division entière, tronquée (RC-1, RC-6).
/// Avec tous les poids à 1, un devoir sur 20 et une composition sur 40 donnent
/// `(devoir + composition) ÷ 3` : le barème porte la pondération.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SommePonderee {
    points: i64,
    bareme: i64,
    nombre: u32,
}

impl SommePonderee {
    /// Accumulateur vide.
    pub const fn nouvelle() -> Self {
        Self {
            points: 0,
            bareme: 0,
            nombre: 0,
        }
    }

    /// Ajoute une note affectée d'une pondération strictement positive.
    pub fn ajouter(mut self, note: Note, ponderation: i64) -> Result<Self, ErreurNote> {
        if ponderation <= 0 {
            return Err(ErreurNote::PonderationInvalide(ponderation));
        }
        let points = i64::from(note.valeur_centiemes)
            .checked_mul(ponderation)
            .and_then(|p| self.points.checked_add(p))
            .ok_or(ErreurNote::Debordement)?;
        let bareme = i64::from(note.bareme_centiemes)
            .checked_mul(ponderation)
            .and_then(|b| self.bareme.checked_add(b))
            .ok_or(ErreurNote::Debordement)?;
        self.points = points;
        self.bareme = bareme;
        self.nombre = self.nombre.checked_add(1).ok_or(ErreurNote::Debordement)?;
        Ok(self)
    }

    /// Nombre de notes accumulées.
    pub const fn nombre(&self) -> u32 {
        self.nombre
    }

    /// Moyenne sur le barème cible, tronquée au centième (RC-1). Les absents n'ont pas été
    /// ajoutés : ils sont exclus du numérateur comme du dénominateur (RC-3).
    pub fn moyenne(&self, bareme_cible_centiemes: i32) -> Result<Note, ErreurNote> {
        if bareme_cible_centiemes <= 0 {
            return Err(ErreurNote::BaremeInvalide(bareme_cible_centiemes));
        }
        if self.nombre == 0 {
            return Err(ErreurNote::AucuneNote);
        }
        let numerateur = self
            .points
            .checked_mul(i64::from(bareme_cible_centiemes))
            .ok_or(ErreurNote::Debordement)?;
        let valeur = numerateur
            .checked_div(self.bareme)
            .ok_or(ErreurNote::Debordement)?;
        let valeur = i32::try_from(valeur).map_err(|_| ErreurNote::Debordement)?;
        Note::nouvelle(valeur, bareme_cible_centiemes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn n(v: i32, b: i32) -> Note {
        Note::nouvelle(v, b).unwrap_or(Note {
            valeur_centiemes: 0,
            bareme_centiemes: 1,
        })
    }

    #[test]
    fn bornes_de_construction() {
        assert!(Note::nouvelle(0, 2000).is_ok());
        assert!(Note::nouvelle(2000, 2000).is_ok());
        assert_eq!(
            Note::nouvelle(2001, 2000),
            Err(ErreurNote::HorsBareme {
                valeur_centiemes: 2001,
                bareme_centiemes: 2000
            })
        );
        assert_eq!(
            Note::nouvelle(-1, 2000),
            Err(ErreurNote::HorsBareme {
                valeur_centiemes: -1,
                bareme_centiemes: 2000
            })
        );
        assert_eq!(Note::nouvelle(0, 0), Err(ErreurNote::BaremeInvalide(0)));
    }

    #[test]
    fn devoir_sur_20_et_composition_sur_40_comptent_pour_trois() {
        // Devoir 12/20, composition 30/40 : (12 + 30) ÷ 3 = 14,00 — le calcul du cahier.
        let moyenne = SommePonderee::nouvelle()
            .ajouter(n(1200, 2000), 1)
            .and_then(|s| s.ajouter(n(3000, 4000), 1))
            .and_then(|s| s.moyenne(2000));
        assert_eq!(moyenne, Ok(n(1400, 2000)));
    }

    #[test]
    fn troncature_jamais_arrondi() {
        // 13,09 + 27,00 = 40,09 sur 60, ramené sur 20 = 13,3633… → 13,36, jamais 13,37 (RC-1).
        let moyenne = SommePonderee::nouvelle()
            .ajouter(n(1309, 2000), 1)
            .and_then(|s| s.ajouter(n(2700, 4000), 1))
            .and_then(|s| s.moyenne(2000));
        assert_eq!(moyenne, Ok(n(1336, 2000)));
        // 7/30 ramené sur 20 : 4,666… → 4,66, pas 4,67.
        assert_eq!(n(700, 3000).ramener_a(2000), Ok(n(466, 2000)));
    }

    #[test]
    fn premier_cycle_trois_compositions_sur_dix() {
        // Trois compositions /10 : 7, 8,5 et 6 → (7 + 8,5 + 6) ÷ 3 = 7,16 (tronqué).
        let moyenne = SommePonderee::nouvelle()
            .ajouter(n(700, 1000), 1)
            .and_then(|s| s.ajouter(n(850, 1000), 1))
            .and_then(|s| s.ajouter(n(600, 1000), 1))
            .and_then(|s| s.moyenne(1000));
        assert_eq!(moyenne, Ok(n(716, 1000)));
    }

    #[test]
    fn ponderation_explicite_equivaut_a_repeter_la_note() {
        let ponderee = SommePonderee::nouvelle()
            .ajouter(n(1500, 2000), 3)
            .and_then(|s| s.ajouter(n(900, 2000), 1))
            .and_then(|s| s.moyenne(2000));
        let repetee = SommePonderee::nouvelle()
            .ajouter(n(1500, 2000), 1)
            .and_then(|s| s.ajouter(n(1500, 2000), 1))
            .and_then(|s| s.ajouter(n(1500, 2000), 1))
            .and_then(|s| s.ajouter(n(900, 2000), 1))
            .and_then(|s| s.moyenne(2000));
        assert_eq!(ponderee, repetee);
        assert_eq!(ponderee, Ok(n(1350, 2000)));
    }

    #[test]
    fn erreurs_de_calcul() {
        assert_eq!(
            SommePonderee::nouvelle().moyenne(2000),
            Err(ErreurNote::AucuneNote)
        );
        assert_eq!(
            SommePonderee::nouvelle().ajouter(n(1, 1), 0),
            Err(ErreurNote::PonderationInvalide(0))
        );
        assert_eq!(
            n(1000, 2000).comparer(&n(500, 1000)),
            Err(ErreurNote::BaremesDifferents(2000, 1000))
        );
        assert_eq!(n(1000, 2000).comparer(&n(1200, 2000)), Ok(Ordering::Less));
    }

    #[test]
    fn formatage() {
        assert_eq!(n(1350, 2000).formater(), "13,50");
        assert_eq!(n(5, 1000).formater(), "0,05");
        assert_eq!(n(1350, 2000).to_string(), "13,50/20");
    }

    proptest! {
        #[test]
        fn la_moyenne_reste_dans_le_bareme_et_est_tronquee(
            notes in prop::collection::vec((0i32..=4000, 1i64..=5), 1..40),
            bareme_cible in prop::sample::select(vec![1000i32, 2000])
        ) {
            let mut somme = SommePonderee::nouvelle();
            let mut points: i128 = 0;
            let mut total: i128 = 0;
            for (valeur, pond) in &notes {
                let note = n(*valeur, 4000);
                somme = somme.ajouter(note, *pond).unwrap_or(somme);
                points += i128::from(*valeur) * i128::from(*pond);
                total += 4000 * i128::from(*pond);
            }
            let moyenne = somme.moyenne(bareme_cible);
            prop_assert!(moyenne.is_ok());
            if let Ok(m) = moyenne {
                prop_assert!(m.valeur_centiemes() >= 0 && m.valeur_centiemes() <= bareme_cible);
                let attendu = points * i128::from(bareme_cible) / total;
                prop_assert_eq!(i128::from(m.valeur_centiemes()), attendu);
            }
        }
    }
}
