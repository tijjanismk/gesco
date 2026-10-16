//! Date civile `AAAA-MM-JJ` : un jour du calendrier, sans heure ni fuseau. Une date de naissance
//! n'est pas un instant : elle ne doit pas glisser d'un jour selon le fuseau du poste.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::horodatage::Horodatage;

/// Jour du calendrier grégorien.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DateCivile {
    annee: i32,
    mois: u8,
    jour: u8,
}

const fn bissextile(annee: i32) -> bool {
    (annee % 4 == 0 && annee % 100 != 0) || annee % 400 == 0
}

const fn jours_du_mois(annee: i32, mois: u8) -> u8 {
    match mois {
        2 if bissextile(annee) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

impl DateCivile {
    /// Construit une date valide du calendrier ; `None` sinon (31 avril, 29 février non
    /// bissextile, année hors de 1900 à 2999).
    pub const fn nouvelle(annee: i32, mois: u8, jour: u8) -> Option<Self> {
        if annee < 1900 || annee > 2999 || mois < 1 || mois > 12 || jour < 1 {
            return None;
        }
        if jour > jours_du_mois(annee, mois) {
            return None;
        }
        Some(Self { annee, mois, jour })
    }

    /// Lit `AAAA-MM-JJ`.
    pub fn analyser(texte: &str) -> Option<Self> {
        let mut morceaux = texte.trim().split('-');
        let (Some(a), Some(m), Some(j), None) = (
            morceaux.next(),
            morceaux.next(),
            morceaux.next(),
            morceaux.next(),
        ) else {
            return None;
        };
        if a.len() != 4 || m.len() != 2 || j.len() != 2 {
            return None;
        }
        Self::nouvelle(a.parse().ok()?, m.parse().ok()?, j.parse().ok()?)
    }

    /// Année.
    pub const fn annee(&self) -> i32 {
        self.annee
    }

    /// Mois, de 1 à 12.
    pub const fn mois(&self) -> u8 {
        self.mois
    }

    /// Jour du mois.
    pub const fn jour(&self) -> u8 {
        self.jour
    }

    /// Jours écoulés depuis le 1er janvier 1970, pour comparer deux dates.
    pub const fn jours_depuis_epoque(&self) -> i64 {
        // Algorithme de Howard Hinnant (« days_from_civil »).
        let annee = (self.annee as i64) - if self.mois <= 2 { 1 } else { 0 };
        let ere = annee.div_euclid(400);
        let annee_de_l_ere = annee - ere * 400;
        let mois = self.mois as i64;
        let jour_de_l_annee =
            (153 * (mois + if mois > 2 { -3 } else { 9 }) + 2) / 5 + self.jour as i64 - 1;
        let jour_de_l_ere =
            annee_de_l_ere * 365 + annee_de_l_ere / 4 - annee_de_l_ere / 100 + jour_de_l_annee;
        ere * 146_097 + jour_de_l_ere - 719_468
    }

    /// Date d'un nombre de jours depuis le 1er janvier 1970 ; `None` hors de 1900 à 2999.
    pub fn depuis_jours_epoque(jours: i64) -> Option<Self> {
        // Algorithme de Howard Hinnant (« civil_from_days »).
        let z = jours + 719_468;
        let ere = z.div_euclid(146_097);
        let jour_de_l_ere = z - ere * 146_097;
        let annee_de_l_ere = (jour_de_l_ere - jour_de_l_ere / 1460 + jour_de_l_ere / 36_524
            - jour_de_l_ere / 146_096)
            / 365;
        let jour_de_l_annee =
            jour_de_l_ere - (365 * annee_de_l_ere + annee_de_l_ere / 4 - annee_de_l_ere / 100);
        let mp = (5 * jour_de_l_annee + 2) / 153;
        let jour = jour_de_l_annee - (153 * mp + 2) / 5 + 1;
        let mois = if mp < 10 { mp + 3 } else { mp - 9 };
        let annee = annee_de_l_ere + ere * 400 + i64::from(mois <= 2);
        Self::nouvelle(
            i32::try_from(annee).ok()?,
            u8::try_from(mois).ok()?,
            u8::try_from(jour).ok()?,
        )
    }

    /// La date décalée d'un nombre de jours, en avant ou en arrière.
    pub fn decaler(&self, jours: i64) -> Option<Self> {
        Self::depuis_jours_epoque(self.jours_depuis_epoque() + jours)
    }

    /// Jour de la semaine : 0 pour lundi, 6 pour dimanche.
    pub const fn jour_de_semaine(&self) -> u8 {
        // Le 1er janvier 1970 était un jeudi.
        (self.jours_depuis_epoque() + 3).rem_euclid(7) as u8
    }

    /// Jours de `self` à `autre` : positif si `autre` vient après.
    pub const fn jours_jusqua(&self, autre: Self) -> i64 {
        autre.jours_depuis_epoque() - self.jours_depuis_epoque()
    }

    /// Jour civil UTC d'un instant : l'heure du Mali.
    pub fn du_jour(instant: Horodatage) -> Option<Self> {
        let civil = instant.civil_utc();
        Self::nouvelle(i32::try_from(civil.annee).ok()?, civil.mois, civil.jour)
    }
}

impl fmt::Display for DateCivile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.annee, self.mois, self.jour)
    }
}

impl Serialize for DateCivile {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for DateCivile {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let texte = String::deserialize(deserializer)?;
        Self::analyser(&texte)
            .ok_or_else(|| serde::de::Error::custom(format!("date civile invalide : {texte}")))
    }
}

// Transmise au frontend sous la forme `AAAA-MM-JJ`.
impl specta::Type for DateCivile {
    fn definition(_types: &mut specta::Types) -> specta::datatype::DataType {
        specta::datatype::DataType::Primitive(specta::datatype::Primitive::str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_jours_se_comptent_dans_les_deux_sens() {
        let jour = |a, m, j| DateCivile::nouvelle(a, m, j).unwrap_or_else(|| unreachable!());
        let d = jour(2026, 10, 15);
        assert_eq!(
            d.decaler(17).map(|x| x.to_string()),
            Some("2026-11-01".to_owned())
        );
        assert_eq!(
            d.decaler(-15).map(|x| x.to_string()),
            Some("2026-09-30".to_owned())
        );
        let fevrier = jour(2028, 2, 28);
        assert_eq!(
            fevrier.decaler(1).map(|x| x.to_string()),
            Some("2028-02-29".to_owned())
        );
        assert_eq!(
            fevrier.decaler(2).map(|x| x.to_string()),
            Some("2028-03-01".to_owned())
        );
        assert_eq!(d.jours_jusqua(jour(2027, 10, 15)), 365);
        for jours in [-25_567_i64, 0, 1, 20_000, 375_000] {
            let x = DateCivile::depuis_jours_epoque(jours).map(|x| x.jours_depuis_epoque());
            assert_eq!(x, Some(jours));
        }
        assert!(DateCivile::depuis_jours_epoque(-30_000).is_none());
    }

    #[test]
    fn les_jours_de_la_semaine() {
        let jour = |t: &str| DateCivile::analyser(t).map(|d| d.jour_de_semaine());
        assert_eq!(jour("1970-01-01"), Some(3));
        assert_eq!(jour("2026-10-19"), Some(0));
        assert_eq!(jour("2026-10-24"), Some(5));
        assert_eq!(jour("2026-10-25"), Some(6));
    }

    #[test]
    fn calendrier() {
        assert!(DateCivile::analyser("2012-02-29").is_some());
        assert!(DateCivile::analyser("2013-02-29").is_none());
        assert!(DateCivile::analyser("2014-04-31").is_none());
        assert!(DateCivile::analyser("2014-4-3").is_none());
        assert!(DateCivile::analyser("1899-12-31").is_none());
        assert!(DateCivile::analyser("2014-04-30-1").is_none());
        assert_eq!(
            DateCivile::analyser(" 2014-04-30 ").map(|d| d.to_string()),
            Some("2014-04-30".to_owned())
        );
    }

    #[test]
    fn jours_depuis_epoque() {
        let jours = |t: &str| DateCivile::analyser(t).map(|d| d.jours_depuis_epoque());
        assert_eq!(jours("1970-01-01"), Some(0));
        assert_eq!(jours("2000-03-01"), Some(11_017));
        assert_eq!(jours("2026-10-07"), Some(20_733));
        assert_eq!(
            DateCivile::du_jour(Horodatage::depuis_millis_epoque(20_733 * 86_400_000 + 5)),
            DateCivile::analyser("2026-10-07")
        );
    }

    #[test]
    fn serialisation() {
        let d = DateCivile::nouvelle(2015, 1, 9);
        assert_eq!(
            serde_json::to_string(&d).ok(),
            Some("\"2015-01-09\"".to_owned())
        );
        assert!(serde_json::from_str::<DateCivile>("\"2015-13-09\"").is_err());
    }
}
