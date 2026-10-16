//! Instant UTC en millisecondes depuis l'époque Unix.
//!
//! Le noyau ne lit jamais l'horloge (R-03) : tout horodatage lui est passé en paramètre, ce qui
//! rend déterministes les calculs de périodes, d'échéances et d'ancienneté.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::erreur::ErreurHorodatage;

/// Instant UTC, précision milliseconde. L'affichage en heure locale est l'affaire de l'interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Horodatage(i64);

impl Horodatage {
    /// Construit depuis des millisecondes écoulées depuis le 1er janvier 1970 UTC.
    pub const fn depuis_millis_epoque(millis: i64) -> Self {
        Self(millis)
    }

    /// Millisecondes depuis l'époque Unix.
    pub const fn millis_epoque(&self) -> i64 {
        self.0
    }

    /// Vrai si `self` précède strictement `autre`.
    pub const fn avant(&self, autre: &Self) -> bool {
        self.0 < autre.0
    }

    /// `autre − self`, en millisecondes (négatif si `autre` est antérieur).
    pub fn difference_millis(&self, autre: &Self) -> Result<i64, ErreurHorodatage> {
        autre
            .0
            .checked_sub(self.0)
            .ok_or(ErreurHorodatage::HorsPlage)
    }

    /// Décale de `millis` millisecondes, en refusant le débordement.
    pub fn plus_millis(&self, millis: i64) -> Result<Self, ErreurHorodatage> {
        self.0
            .checked_add(millis)
            .map(Self)
            .ok_or(ErreurHorodatage::HorsPlage)
    }

    /// Date et heure civiles UTC (année, mois, jour, heure, minute, seconde, milliseconde).
    pub fn civil_utc(&self) -> DateHeureCivile {
        let millis_jour: i64 = 86_400_000;
        let jours = self.0.div_euclid(millis_jour);
        let reste = self.0.rem_euclid(millis_jour);
        let (annee, mois, jour) = jours_vers_civil(jours);
        DateHeureCivile {
            annee,
            mois,
            jour,
            heure: u8::try_from(reste / 3_600_000).unwrap_or(0),
            minute: u8::try_from((reste / 60_000) % 60).unwrap_or(0),
            seconde: u8::try_from((reste / 1_000) % 60).unwrap_or(0),
            milliseconde: u16::try_from(reste % 1_000).unwrap_or(0),
        }
    }

    /// Représentation RFC 3339 en UTC, avec millisecondes : `2026-10-04T08:15:30.250Z`.
    pub fn vers_rfc3339(&self) -> String {
        let c = self.civil_utc();
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
            c.annee, c.mois, c.jour, c.heure, c.minute, c.seconde, c.milliseconde
        )
    }
}

impl fmt::Display for Horodatage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.vers_rfc3339())
    }
}

/// Décomposition civile d'un instant UTC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateHeureCivile {
    /// Année du calendrier grégorien proleptique.
    pub annee: i64,
    /// Mois, de 1 à 12.
    pub mois: u8,
    /// Jour du mois, de 1 à 31.
    pub jour: u8,
    /// Heure, de 0 à 23.
    pub heure: u8,
    /// Minute, de 0 à 59.
    pub minute: u8,
    /// Seconde, de 0 à 59.
    pub seconde: u8,
    /// Milliseconde, de 0 à 999.
    pub milliseconde: u16,
}

/// Convertit un nombre de jours depuis l'époque Unix en (année, mois, jour) grégoriens.
/// Algorithme de Howard Hinnant, exact sur tout l'intervalle représentable en `i64`.
fn jours_vers_civil(jours: i64) -> (i64, u8, u8) {
    let z = jours + 719_468;
    let ere = z.div_euclid(146_097);
    let jour_ere = z.rem_euclid(146_097);
    let annee_ere = (jour_ere - jour_ere / 1_460 + jour_ere / 36_524 - jour_ere / 146_096) / 365;
    let jour_annee = jour_ere - (365 * annee_ere + annee_ere / 4 - annee_ere / 100);
    let mois_decale = (5 * jour_annee + 2) / 153;
    let jour = jour_annee - (153 * mois_decale + 2) / 5 + 1;
    let mois = if mois_decale < 10 {
        mois_decale + 3
    } else {
        mois_decale - 9
    };
    let annee = annee_ere + ere * 400 + i64::from(mois <= 2);
    (
        annee,
        u8::try_from(mois).unwrap_or(1),
        u8::try_from(jour).unwrap_or(1),
    )
}

// Millisecondes depuis l'époque : un entier, exporté en `number` (exact jusqu'en l'an 287 396).
impl specta::Type for Horodatage {
    fn definition(types: &mut specta::Types) -> specta::datatype::DataType {
        <i64 as specta::Type>::definition(types)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoque_unix() {
        assert_eq!(
            Horodatage::depuis_millis_epoque(0).vers_rfc3339(),
            "1970-01-01T00:00:00.000Z"
        );
    }

    #[test]
    fn date_connue_avec_millisecondes() {
        // 2026-10-04 08:15:30.250 UTC
        let h = Horodatage::depuis_millis_epoque(1_791_101_730_250);
        assert_eq!(h.vers_rfc3339(), "2026-10-04T08:15:30.250Z");
    }

    #[test]
    fn annee_bissextile_et_fin_de_siecle() {
        // 2000-02-29 (bissextile) et 1900-03-01 (1900 non bissextile) via des jours connus.
        let bissextile = Horodatage::depuis_millis_epoque(951_782_400_000);
        assert_eq!(bissextile.vers_rfc3339(), "2000-02-29T00:00:00.000Z");
        let avant_epoque = Horodatage::depuis_millis_epoque(-2_203_891_200_000);
        assert_eq!(avant_epoque.vers_rfc3339(), "1900-03-01T00:00:00.000Z");
    }

    #[test]
    fn differences_et_decalages() {
        let a = Horodatage::depuis_millis_epoque(1_000);
        let b = Horodatage::depuis_millis_epoque(4_500);
        assert!(a.avant(&b) && !b.avant(&a));
        assert_eq!(a.difference_millis(&b), Ok(3_500));
        assert_eq!(b.difference_millis(&a), Ok(-3_500));
        assert_eq!(
            a.plus_millis(500),
            Ok(Horodatage::depuis_millis_epoque(1_500))
        );
        assert_eq!(
            Horodatage::depuis_millis_epoque(i64::MAX).plus_millis(1),
            Err(ErreurHorodatage::HorsPlage)
        );
    }

    #[test]
    fn serde_transparent() {
        let h = Horodatage::depuis_millis_epoque(123);
        assert_eq!(serde_json::to_string(&h).unwrap_or_default(), "123");
    }
}
