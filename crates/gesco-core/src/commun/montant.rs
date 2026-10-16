//! Montant en francs CFA, entier signé 64 bits (ADR-005, R-11).
//!
//! Aucune conversion depuis ou vers un flottant n'existe, délibérément. Toute division rend un
//! quotient **et** un reste ; l'arrondi commercial est une opération séparée, nommée, paramétrée.
//!
//! ```compile_fail
//! use gesco_core::commun::Montant;
//! let _ = Montant::depuis_francs(1.5);
//! ```
//!
//! ```compile_fail
//! use gesco_core::commun::Montant;
//! let m = Montant::depuis_francs(10);
//! let _ = m / 3;
//! ```

use std::fmt;

use serde::{Deserialize, Serialize};

use super::erreur::ErreurMontant;

/// Montant en francs CFA. Le franc est l'unité ; il n'existe pas de centime.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Default,
    Serialize,
    Deserialize,
    specta::Type,
)]
#[serde(transparent)]
pub struct Montant(i64);

/// Résultat d'une division : quotient entier et reste, jamais un arrondi silencieux.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DivisionMontant {
    /// Part entière, tronquée vers zéro.
    pub quotient: Montant,
    /// Ce qui n'a pas pu être réparti. Même signe que le dividende.
    pub reste: Montant,
}

/// Sens de l'arrondi commercial.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModeArrondi {
    /// Au multiple le plus proche ; à égale distance, vers le haut en valeur absolue.
    AuPlusProche,
    /// Au multiple supérieur ou égal (en valeur absolue).
    Superieur,
    /// Au multiple inférieur ou égal (en valeur absolue).
    Inferieur,
}

impl Montant {
    /// Montant nul.
    pub const ZERO: Self = Self(0);

    /// Construit depuis un nombre entier de francs.
    pub const fn depuis_francs(francs: i64) -> Self {
        Self(francs)
    }

    /// Valeur en francs.
    pub const fn francs(&self) -> i64 {
        self.0
    }

    /// Vrai si strictement négatif.
    pub const fn est_negatif(&self) -> bool {
        self.0 < 0
    }

    /// Vrai si nul.
    pub const fn est_nul(&self) -> bool {
        self.0 == 0
    }

    /// Addition sans débordement silencieux.
    pub fn ajouter(&self, autre: Self) -> Result<Self, ErreurMontant> {
        self.0
            .checked_add(autre.0)
            .map(Self)
            .ok_or(ErreurMontant::Debordement)
    }

    /// Soustraction sans débordement silencieux.
    pub fn soustraire(&self, autre: Self) -> Result<Self, ErreurMontant> {
        self.0
            .checked_sub(autre.0)
            .map(Self)
            .ok_or(ErreurMontant::Debordement)
    }

    /// Multiplication par un entier (nombre d'échéances, de mois, d'élèves).
    pub fn multiplier(&self, facteur: i64) -> Result<Self, ErreurMontant> {
        self.0
            .checked_mul(facteur)
            .map(Self)
            .ok_or(ErreurMontant::Debordement)
    }

    /// Opposé, pour les contre-passations (ADR-010).
    pub fn oppose(&self) -> Result<Self, ErreurMontant> {
        self.0
            .checked_neg()
            .map(Self)
            .ok_or(ErreurMontant::Debordement)
    }

    /// Division entière avec reste explicite. `quotient × diviseur + reste == self`.
    pub fn diviser(&self, diviseur: i64) -> Result<DivisionMontant, ErreurMontant> {
        if diviseur == 0 {
            return Err(ErreurMontant::DivisionParZero);
        }
        let quotient = self
            .0
            .checked_div(diviseur)
            .ok_or(ErreurMontant::Debordement)?;
        let reste = self
            .0
            .checked_rem(diviseur)
            .ok_or(ErreurMontant::Debordement)?;
        Ok(DivisionMontant {
            quotient: Self(quotient),
            reste: Self(reste),
        })
    }

    /// Répartit le montant en `parts` échéances : les premières absorbent le reste, un franc
    /// chacune, de sorte que la somme des parts égale exactement le montant.
    pub fn repartir(&self, parts: usize) -> Result<Vec<Self>, ErreurMontant> {
        let nombre = i64::try_from(parts).map_err(|_| ErreurMontant::Debordement)?;
        let division = self.diviser(nombre)?;
        let reste_abs = division.reste.0.unsigned_abs();
        let unite = if self.0 < 0 { -1 } else { 1 };
        (0..parts)
            .map(|i| {
                let complement = if u64::try_from(i).map(|i| i < reste_abs).unwrap_or(false) {
                    unite
                } else {
                    0
                };
                division.quotient.ajouter(Self(complement))
            })
            .collect()
    }

    /// Arrondi commercial au `multiple` demandé (5 ou 25 F en pratique, §3.7). Le multiple est un
    /// paramètre d'établissement ; le noyau ne connaît aucune valeur par défaut.
    pub fn arrondir_commercial(
        &self,
        multiple: i64,
        mode: ModeArrondi,
    ) -> Result<Self, ErreurMontant> {
        if multiple <= 0 {
            return Err(ErreurMontant::MultipleInvalide(multiple));
        }
        let division = self.diviser(multiple)?;
        if division.reste.est_nul() {
            return Ok(*self);
        }
        let base = division.quotient.multiplier(multiple)?;
        let signe: i64 = if self.0 < 0 { -1 } else { 1 };
        let reste_abs = division.reste.0.abs();
        let monter = match mode {
            ModeArrondi::Superieur => true,
            ModeArrondi::Inferieur => false,
            ModeArrondi::AuPlusProche => {
                reste_abs.checked_mul(2).ok_or(ErreurMontant::Debordement)? >= multiple
            }
        };
        if monter {
            base.ajouter(Self(
                signe
                    .checked_mul(multiple)
                    .ok_or(ErreurMontant::Debordement)?,
            ))
        } else {
            Ok(base)
        }
    }

    /// Part `numerateur / denominateur` du montant, en arithmétique entière : quotient tronqué
    /// vers zéro et reste, jamais d'arrondi implicite (R-11). Sert au pourcentage d'une remise et
    /// au prorata d'une arrivée en cours d'année ; l'arrondi commercial est une étape à part.
    pub fn part(
        &self,
        numerateur: i64,
        denominateur: i64,
    ) -> Result<DivisionMontant, ErreurMontant> {
        if denominateur == 0 {
            return Err(ErreurMontant::DivisionParZero);
        }
        let produit = i128::from(self.0) * i128::from(numerateur);
        let den = i128::from(denominateur);
        let quotient = i64::try_from(produit / den).map_err(|_| ErreurMontant::Debordement)?;
        let reste = i64::try_from(produit % den).map_err(|_| ErreurMontant::Debordement)?;
        Ok(DivisionMontant {
            quotient: Self(quotient),
            reste: Self(reste),
        })
    }

    /// Pourcentage entier du montant : `part(pour_cent, 100)`.
    pub fn pourcentage(&self, pour_cent: i64) -> Result<DivisionMontant, ErreurMontant> {
        self.part(pour_cent, 100)
    }

    /// Montant en toutes lettres, suivi de « franc(s) CFA » : la mention qui fait foi sur un reçu
    /// (SPEC_PHASE_3 §2). Un montant négatif commence par « moins ».
    pub fn en_lettres(&self) -> Result<String, ErreurMontant> {
        let valeur = self.0.unsigned_abs();
        let mots = super::lettres::en_lettres(valeur).ok_or(ErreurMontant::TropGrandPourLettres)?;
        let unite = if valeur <= 1 {
            "franc CFA"
        } else {
            "francs CFA"
        };
        let signe = if self.0 < 0 { "moins " } else { "" };
        Ok(format!("{signe}{mots} {unite}"))
    }

    /// Formatage français : séparateur de milliers (espace fine insécable) et unité.
    pub fn formater(&self) -> String {
        let chiffres = self.0.unsigned_abs().to_string();
        let mut groupes = Vec::new();
        let octets = chiffres.as_bytes();
        let mut fin = octets.len();
        while fin > 0 {
            let debut = fin.saturating_sub(3);
            groupes.push(&chiffres[debut..fin]);
            fin = debut;
        }
        groupes.reverse();
        let signe = if self.0 < 0 { "−" } else { "" };
        format!("{signe}{}\u{202F}F", groupes.join("\u{202F}"))
    }
}

impl fmt::Display for Montant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.formater())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn f(v: i64) -> Montant {
        Montant::depuis_francs(v)
    }

    #[test]
    fn operations_de_base() {
        assert_eq!(f(1_000).ajouter(f(250)), Ok(f(1_250)));
        assert_eq!(f(1_000).soustraire(f(1_250)), Ok(f(-250)));
        assert_eq!(f(12_500).multiplier(3), Ok(f(37_500)));
        assert_eq!(f(500).oppose(), Ok(f(-500)));
        assert_eq!(f(i64::MAX).ajouter(f(1)), Err(ErreurMontant::Debordement));
        assert_eq!(f(i64::MIN).oppose(), Err(ErreurMontant::Debordement));
    }

    #[test]
    fn division_rend_quotient_et_reste() {
        assert_eq!(
            f(100_000).diviser(3),
            Ok(DivisionMontant {
                quotient: f(33_333),
                reste: f(1)
            })
        );
        assert_eq!(f(10).diviser(0), Err(ErreurMontant::DivisionParZero));
        assert_eq!(f(i64::MIN).diviser(-1), Err(ErreurMontant::Debordement));
    }

    #[test]
    fn repartition_exacte_sur_les_echeances() {
        assert_eq!(
            f(100_000).repartir(3),
            Ok(vec![f(33_334), f(33_333), f(33_333)])
        );
        assert_eq!(f(-7).repartir(3), Ok(vec![f(-3), f(-2), f(-2)]));
        assert_eq!(f(10).repartir(0), Err(ErreurMontant::DivisionParZero));
    }

    #[test]
    fn arrondi_commercial() {
        assert_eq!(
            f(12_503).arrondir_commercial(5, ModeArrondi::AuPlusProche),
            Ok(f(12_505))
        );
        assert_eq!(
            f(12_502).arrondir_commercial(5, ModeArrondi::AuPlusProche),
            Ok(f(12_500))
        );
        assert_eq!(
            f(12_501).arrondir_commercial(25, ModeArrondi::Superieur),
            Ok(f(12_525))
        );
        assert_eq!(
            f(12_524).arrondir_commercial(25, ModeArrondi::Inferieur),
            Ok(f(12_500))
        );
        assert_eq!(
            f(12_500).arrondir_commercial(25, ModeArrondi::Superieur),
            Ok(f(12_500))
        );
        assert_eq!(
            f(-13).arrondir_commercial(5, ModeArrondi::AuPlusProche),
            Ok(f(-15))
        );
        assert_eq!(
            f(10).arrondir_commercial(0, ModeArrondi::Superieur),
            Err(ErreurMontant::MultipleInvalide(0))
        );
    }

    #[test]
    fn formatage_francais() {
        assert_eq!(f(0).formater(), "0\u{202F}F");
        assert_eq!(f(950).formater(), "950\u{202F}F");
        assert_eq!(f(1_250_000).formater(), "1\u{202F}250\u{202F}000\u{202F}F");
        assert_eq!(f(-45_000).formater(), "−45\u{202F}000\u{202F}F");
    }

    #[test]
    fn serde_en_entier() {
        assert_eq!(serde_json::to_string(&f(1_500)).unwrap_or_default(), "1500");
        assert!(serde_json::from_str::<Montant>("15.5").is_err());
    }

    #[test]
    fn parts_et_pourcentages() {
        assert_eq!(
            f(150_000).pourcentage(10),
            Ok(DivisionMontant {
                quotient: f(15_000),
                reste: f(0)
            })
        );
        // 7 mois restants sur 9 : 116 666 F, reste 6 (en neuvièmes de franc).
        assert_eq!(
            f(150_000).part(7, 9),
            Ok(DivisionMontant {
                quotient: f(116_666),
                reste: f(6)
            })
        );
        assert_eq!(f(10).part(1, 0), Err(ErreurMontant::DivisionParZero));
        assert_eq!(f(i64::MAX).part(2, 1), Err(ErreurMontant::Debordement));
        assert_eq!(
            f(i64::MAX).part(3, 3),
            Ok(DivisionMontant {
                quotient: f(i64::MAX),
                reste: f(0)
            })
        );
    }

    #[test]
    fn montants_en_lettres() {
        assert_eq!(
            f(125_000).en_lettres(),
            Ok("cent vingt-cinq mille francs CFA".to_owned())
        );
        assert_eq!(f(1).en_lettres(), Ok("un franc CFA".to_owned()));
        assert_eq!(f(0).en_lettres(), Ok("zéro franc CFA".to_owned()));
        assert_eq!(
            f(-5_000).en_lettres(),
            Ok("moins cinq mille francs CFA".to_owned())
        );
        assert_eq!(
            f(i64::MAX).en_lettres(),
            Err(ErreurMontant::TropGrandPourLettres)
        );
    }

    proptest! {
        #[test]
        fn part_reconstitue_le_produit(m in -1_000_000_000_000i64..1_000_000_000_000, num in 0i64..1_000, den in 1i64..1_000) {
            let r = f(m).part(num, den);
            prop_assert!(r.is_ok());
            if let Ok(DivisionMontant { quotient, reste }) = r {
                prop_assert_eq!(i128::from(quotient.francs()) * i128::from(den) + i128::from(reste.francs()), i128::from(m) * i128::from(num));
                prop_assert!(reste.francs().abs() < den);
            }
        }

        #[test]
        fn tout_montant_d_ecole_s_ecrit_en_lettres(m in 0i64..=999_999_999_999) {
            let r = f(m).en_lettres();
            prop_assert!(r.is_ok());
            if let Ok(texte) = r {
                prop_assert!(texte.ends_with("CFA"));
                prop_assert!(!texte.contains("  "));
            }
        }

        #[test]
        fn division_reconstitue_le_dividende(d in any::<i64>(), q in any::<i64>().prop_filter("non nul", |q| *q != 0 && *q != -1)) {
            let r = f(d).diviser(q);
            prop_assert!(r.is_ok());
            if let Ok(DivisionMontant { quotient, reste }) = r {
                prop_assert_eq!(quotient.francs().wrapping_mul(q).wrapping_add(reste.francs()), d);
                prop_assert!(reste.francs().abs() < q.abs());
            }
        }

        #[test]
        fn repartition_somme_exacte(m in -1_000_000_000i64..1_000_000_000, parts in 1usize..200) {
            let parts_calculees = f(m).repartir(parts);
            prop_assert!(parts_calculees.is_ok());
            if let Ok(v) = parts_calculees {
                prop_assert_eq!(v.len(), parts);
                prop_assert_eq!(v.iter().map(Montant::francs).sum::<i64>(), m);
                let min = v.iter().map(Montant::francs).min().unwrap_or(0);
                let max = v.iter().map(Montant::francs).max().unwrap_or(0);
                prop_assert!(max - min <= 1);
            }
        }

        #[test]
        fn arrondi_est_un_multiple_et_reste_proche(m in -1_000_000_000i64..1_000_000_000, multiple in 1i64..1_000) {
            for mode in [ModeArrondi::AuPlusProche, ModeArrondi::Superieur, ModeArrondi::Inferieur] {
                let r = f(m).arrondir_commercial(multiple, mode);
                prop_assert!(r.is_ok());
                if let Ok(a) = r {
                    prop_assert_eq!(a.francs() % multiple, 0);
                    prop_assert!((a.francs() - m).abs() < multiple);
                }
            }
        }
    }
}
