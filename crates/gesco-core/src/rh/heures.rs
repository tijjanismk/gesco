//! État mensuel des heures des vacataires (SPEC_PHASE_6 §4, T-41) : pour chaque engagement de
//! vacation qui couvre le mois, les heures prévues et les heures faites, visées par la direction ;
//! la paie (phase 7) paie les heures visées au taux de l'engagement.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::commun::{DateCivile, Horodatage, Id, Montant};
use crate::core::transition::MachineAEtats;
use crate::core::{Etablissement, Utilisateur};
use crate::rh::{Engagement, MembrePersonnel, NatureEngagement, Remuneration};

/// Mois du calendrier, `AAAA-MM`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Mois {
    annee: i32,
    mois: u8,
}

impl Mois {
    /// Mois valide ; `None` hors de 1 à 12 ou de 1900 à 2999.
    pub fn nouveau(annee: i32, mois: u8) -> Option<Self> {
        DateCivile::nouvelle(annee, mois, 1).map(|_| Self { annee, mois })
    }

    /// Lit `AAAA-MM`.
    pub fn analyser(texte: &str) -> Option<Self> {
        let (a, m) = texte.trim().split_once('-')?;
        if a.len() != 4 || m.len() != 2 {
            return None;
        }
        Self::nouveau(a.parse().ok()?, m.parse().ok()?)
    }

    /// Année.
    pub const fn annee(&self) -> i32 {
        self.annee
    }

    /// Numéro du mois, de 1 à 12.
    pub const fn numero(&self) -> u8 {
        self.mois
    }

    /// Mois d'un jour.
    pub const fn de(jour: DateCivile) -> Self {
        Self {
            annee: jour.annee(),
            mois: jour.mois(),
        }
    }

    /// Premier jour.
    pub fn premier_jour(&self) -> Option<DateCivile> {
        DateCivile::nouvelle(self.annee, self.mois, 1)
    }

    /// Dernier jour.
    pub fn dernier_jour(&self) -> Option<DateCivile> {
        let (a, m) = if self.mois == 12 {
            (self.annee + 1, 1)
        } else {
            (self.annee, self.mois + 1)
        };
        DateCivile::nouvelle(a, m, 1)?.decaler(-1)
    }

    /// Mois précédent ; `None` avant janvier 1900.
    pub fn precedent(&self) -> Option<Self> {
        if self.mois == 1 {
            Self::nouveau(self.annee - 1, 12)
        } else {
            Self::nouveau(self.annee, self.mois - 1)
        }
    }
}

impl fmt::Display for Mois {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}", self.annee, self.mois)
    }
}

impl Serialize for Mois {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Mois {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let texte = String::deserialize(deserializer)?;
        Self::analyser(&texte)
            .ok_or_else(|| serde::de::Error::custom(format!("mois invalide : {texte}")))
    }
}

// Transmis au frontend sous la forme `AAAA-MM`.
impl specta::Type for Mois {
    fn definition(_types: &mut specta::Types) -> specta::datatype::DataType {
        specta::datatype::DataType::Primitive(specta::datatype::Primitive::str)
    }
}

/// État d'une ligne d'heures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum StatutHeures {
    /// En saisie : les heures se corrigent.
    EnSaisie,
    /// Visée par la direction : elle ne change plus, la paie la reprend.
    Visee,
}

impl MachineAEtats for StatutHeures {
    const ETATS: &'static [Self] = &[Self::EnSaisie, Self::Visee];
    const TRANSITIONS: &'static [(Self, Self)] =
        &[(Self::EnSaisie, Self::Visee), (Self::Visee, Self::EnSaisie)];

    fn nom(&self) -> &'static str {
        match self {
            Self::EnSaisie => "en_saisie",
            Self::Visee => "visee",
        }
    }
}

impl StatutHeures {
    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::ETATS.iter().copied().find(|s| s.nom() == nom)
    }
}

/// Heures d'un engagement de vacation pour un mois.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct HeuresVacation {
    /// Identifiant technique.
    pub id: Id<HeuresVacation>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Engagement de vacation.
    pub engagement_id: Id<Engagement>,
    /// Membre.
    pub membre_id: Id<MembrePersonnel>,
    /// Mois.
    pub mois: Mois,
    /// Heures prévues, en minutes.
    pub minutes_prevues: i64,
    /// Heures faites, en minutes ; absentes tant qu'elles ne sont pas saisies.
    pub minutes_faites: Option<i64>,
    /// Observation : cours manqués, rattrapages.
    pub observation: String,
    /// État.
    pub statut: StatutHeures,
    /// Instant du visa.
    pub vise_le: Option<Horodatage>,
    /// Qui a visé.
    pub vise_par: Option<Id<Utilisateur>>,
    /// Motif du dernier déverrouillage.
    pub motif_deverrouillage: String,
}

/// Semaines de cours proposées pour un mois : la direction corrige les heures prévues (Q-66).
pub const SEMAINES_PAR_MOIS: i64 = 4;

/// Plus d'heures qu'on n'en peut faire dans un mois : 300 heures.
pub const MINUTES_PAR_MOIS_MAX: i64 = 300 * 60;

/// Vrai si l'engagement est une vacation qui couvre au moins un jour du mois.
pub fn vacation_du_mois(e: &Engagement, mois: Mois) -> bool {
    let (Some(premier), Some(dernier)) = (mois.premier_jour(), mois.dernier_jour()) else {
        return false;
    };
    e.nature == NatureEngagement::Vacation
        && e.debut <= dernier
        && e.dernier_jour().is_none_or(|f| f >= premier)
}

/// Heures prévues proposées : heures par semaine × semaines de cours du mois.
pub fn minutes_prevues(e: &Engagement) -> i64 {
    match e.remuneration {
        Remuneration::Horaire {
            minutes_par_semaine,
            ..
        } => minutes_par_semaine.saturating_mul(SEMAINES_PAR_MOIS),
        _ => 0,
    }
}

/// Lit des heures saisies : `12`, `12,5`, `12.25`, `12h30`, `12 h 30` ; vide, aucune. Au quart
/// d'heure près, 300 heures au plus. L'erreur est la clé du message.
pub fn lire_heures(texte: &str) -> Result<Option<i64>, &'static str> {
    let t: String = texte
        .trim()
        .to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    if t.is_empty() {
        return Ok(None);
    }
    let minutes = if let Some((h, m)) = t.split_once('h') {
        let heures: i64 = h.parse().map_err(|_| "heures.illisibles")?;
        let minutes: i64 = if m.is_empty() {
            0
        } else {
            m.parse().map_err(|_| "heures.illisibles")?
        };
        if !(0..60).contains(&minutes) {
            return Err("heures.illisibles");
        }
        heures.checked_mul(60).and_then(|x| x.checked_add(minutes))
    } else {
        let (entier, decimales) = t.replace(',', ".").split_once('.').map_or_else(
            || (t.clone(), String::new()),
            |(e, d)| (e.to_owned(), d.to_owned()),
        );
        let heures: i64 = entier.parse().map_err(|_| "heures.illisibles")?;
        let fraction = match decimales.as_str() {
            "" | "0" | "00" => 0,
            "25" => 15,
            "5" | "50" => 30,
            "75" => 45,
            d if d.chars().all(|c| c.is_ascii_digit()) => return Err("heures.quart_d_heure"),
            _ => return Err("heures.illisibles"),
        };
        heures.checked_mul(60).and_then(|x| x.checked_add(fraction))
    }
    .ok_or("heures.illisibles")?;
    if minutes < 0 {
        return Err("heures.illisibles");
    }
    if minutes % 15 != 0 {
        return Err("heures.quart_d_heure");
    }
    if minutes > MINUTES_PAR_MOIS_MAX {
        return Err("heures.trop_nombreuses");
    }
    Ok(Some(minutes))
}

/// Heures écrites comme on les saisit : `12`, `12,5`, `12,25`.
pub fn ecrire_heures(minutes: i64) -> String {
    let (h, m) = (minutes / 60, minutes % 60);
    match m {
        0 => h.to_string(),
        15 => format!("{h},25"),
        30 => format!("{h},5"),
        45 => format!("{h},75"),
        _ => format!("{h}h{m:02}"),
    }
}

/// Montant des heures au taux de l'heure, tronqué au franc (R-11) : la paie le reprend.
pub fn montant_des_heures(taux: Montant, minutes: i64) -> Option<Montant> {
    taux.part(minutes, 60).ok().map(|d| d.quotient)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rh::StatutEngagement;

    fn jour(a: i32, m: u8, j: u8) -> DateCivile {
        DateCivile::nouvelle(a, m, j).unwrap_or_else(|| unreachable!())
    }

    fn vacation(debut: DateCivile, fin: Option<DateCivile>) -> Engagement {
        Engagement {
            id: Id::depuis_octets([1; 16]),
            etablissement_id: Id::depuis_octets([2; 16]),
            membre_id: Id::depuis_octets([3; 16]),
            nature: NatureEngagement::Vacation,
            debut,
            fin,
            fonction: "Professeur".to_owned(),
            remuneration: Remuneration::Horaire {
                taux: Montant::depuis_francs(2_500),
                minutes_par_semaine: 6 * 60 + 30,
            },
            paye_douze_mois: true,
            statut: StatutEngagement::EnCours,
            arrete_le: None,
            motif_fin: String::new(),
            precedent_id: None,
            note: String::new(),
        }
    }

    #[test]
    fn un_mois_se_lit_et_connait_ses_bornes() {
        let fevrier = Mois::analyser("2028-02");
        assert_eq!(fevrier.map(|m| m.to_string()), Some("2028-02".to_owned()));
        assert_eq!(
            fevrier.and_then(|m| m.dernier_jour()),
            Some(jour(2028, 2, 29))
        );
        assert_eq!(
            Mois::analyser("2026-12").and_then(|m| m.dernier_jour()),
            Some(jour(2026, 12, 31))
        );
        assert_eq!(Mois::analyser("2026-13"), None);
        assert_eq!(Mois::analyser("26-10"), None);
        assert_eq!(Mois::de(jour(2026, 10, 14)).to_string(), "2026-10");
        assert_eq!(
            Mois::analyser("2027-01").and_then(|m| m.precedent()),
            Mois::analyser("2026-12")
        );
        assert_eq!(
            Mois::analyser("2026-10").and_then(|m| m.precedent()),
            Mois::analyser("2026-09")
        );
        assert_eq!(Mois::analyser("1900-01").and_then(|m| m.precedent()), None);
    }

    #[test]
    fn la_table_des_etats_est_exhaustive() {
        use StatutHeures::*;
        for depuis in StatutHeures::ETATS {
            for vers in StatutHeures::ETATS {
                let permise = matches!((depuis, vers), (EnSaisie, Visee) | (Visee, EnSaisie));
                assert_eq!(
                    depuis.transiter(*vers).is_ok(),
                    permise,
                    "{depuis:?} → {vers:?}"
                );
            }
        }
    }

    #[test]
    fn une_vacation_couvre_les_mois_de_sa_periode() {
        let v = vacation(jour(2026, 10, 14), Some(jour(2027, 7, 31)));
        let mois = |t| Mois::analyser(t).unwrap_or_else(|| unreachable!());
        assert!(!vacation_du_mois(&v, mois("2026-09")));
        assert!(vacation_du_mois(&v, mois("2026-10")));
        assert!(vacation_du_mois(&v, mois("2027-07")));
        assert!(!vacation_du_mois(&v, mois("2027-08")));
        let mut arretee = v.clone();
        arretee.statut = StatutEngagement::Termine;
        arretee.arrete_le = Some(jour(2026, 11, 30));
        assert!(!vacation_du_mois(&arretee, mois("2026-12")));
        assert_eq!(minutes_prevues(&v), 26 * 60);
    }

    #[test]
    fn les_heures_se_saisissent_au_quart_d_heure() {
        assert_eq!(lire_heures(""), Ok(None));
        assert_eq!(lire_heures("12"), Ok(Some(720)));
        assert_eq!(lire_heures("12,5"), Ok(Some(750)));
        assert_eq!(lire_heures(" 12.25 "), Ok(Some(735)));
        assert_eq!(lire_heures("12h30"), Ok(Some(750)));
        assert_eq!(lire_heures("12 h 45"), Ok(Some(765)));
        assert_eq!(lire_heures("12h"), Ok(Some(720)));
        assert_eq!(lire_heures("0"), Ok(Some(0)));
        assert_eq!(lire_heures("12,3"), Err("heures.quart_d_heure"));
        assert_eq!(lire_heures("12h20"), Err("heures.quart_d_heure"));
        assert_eq!(lire_heures("douze"), Err("heures.illisibles"));
        assert_eq!(lire_heures("-2"), Err("heures.illisibles"));
        assert_eq!(lire_heures("301"), Err("heures.trop_nombreuses"));
        for minutes in [0, 15, 30, 45, 720, 735, 750, 765] {
            assert_eq!(lire_heures(&ecrire_heures(minutes)), Ok(Some(minutes)));
        }
    }

    #[test]
    fn le_montant_des_heures_est_tronque_au_franc() {
        let taux = Montant::depuis_francs(2_501);
        assert_eq!(montant_des_heures(taux, 15).map(|m| m.francs()), Some(625));
        assert_eq!(
            montant_des_heures(Montant::depuis_francs(2_500), 24 * 60 + 30).map(|m| m.francs()),
            Some(61_250)
        );
    }
}
