//! Départs et exclusions (SPEC_PHASE_4 §9, réponse du 13 octobre 2026) : transfert vers une autre
//! école, exclusion définitive révocable, exclusion temporaire avec date de retour. Rien ne
//! s'efface : une exclusion révoquée ou levée reste dans l'historique de l'élève.

use serde::{Deserialize, Serialize};

use crate::commun::{DateCivile, Id};
use crate::core::{Etablissement, MachineAEtats};
use crate::finance::Echeance;
use crate::scolarite::Inscription;

/// Exclusion temporaire ou définitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum NatureExclusion {
    /// Quelques jours, avec une date de retour : l'élève reste inscrit.
    Temporaire,
    /// L'élève quitte l'école ; la direction peut revenir sur la décision et le réintégrer.
    Definitive,
}

impl NatureExclusion {
    /// Les deux natures.
    pub const TOUTES: [Self; 2] = [Self::Temporaire, Self::Definitive];

    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Temporaire => "temporaire",
            Self::Definitive => "definitive",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::TOUTES.into_iter().find(|n| n.nom() == nom)
    }
}

/// État d'une exclusion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum StatutExclusion {
    /// Décidée ; une exclusion temporaire prend fin d'elle-même à sa date de retour.
    EnCours,
    /// Exclusion temporaire levée avant son terme, avec motif.
    Levee,
    /// Exclusion définitive annulée par décision motivée : l'élève est réintégré.
    Revoquee,
}

impl MachineAEtats for StatutExclusion {
    const ETATS: &'static [Self] = &[Self::EnCours, Self::Levee, Self::Revoquee];
    const TRANSITIONS: &'static [(Self, Self)] = &[
        (Self::EnCours, Self::Levee),
        (Self::EnCours, Self::Revoquee),
    ];

    fn nom(&self) -> &'static str {
        match self {
            Self::EnCours => "en_cours",
            Self::Levee => "levee",
            Self::Revoquee => "revoquee",
        }
    }
}

impl StatutExclusion {
    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::ETATS.iter().copied().find(|s| s.nom() == nom)
    }
}

/// Exclusion d'un élève.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Exclusion {
    /// Identifiant technique.
    pub id: Id<Exclusion>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Inscription de l'élève.
    pub inscription_id: Id<Inscription>,
    /// Temporaire ou définitive.
    pub nature: NatureExclusion,
    /// Premier jour d'exclusion.
    pub du: DateCivile,
    /// Dernier jour d'une exclusion temporaire ; rien pour une définitive.
    pub au: Option<DateCivile>,
    /// Motif.
    pub motif: String,
    /// Décision : conseil de discipline, décision de la direction, sa date…
    pub decision: String,
    /// État.
    pub statut: StatutExclusion,
    /// Jour de la levée ou de la réintégration.
    pub leve_le: Option<DateCivile>,
    /// Motif de la levée ou de la réintégration.
    pub motif_levee: String,
    /// Échéances annulées par une exclusion définitive, rétablies à la réintégration.
    pub echeances_annulees: Vec<Id<Echeance>>,
}

/// Plus longue exclusion temporaire, en jours : au-delà, c'est une exclusion définitive.
pub const EXCLUSION_TEMPORAIRE_MAXIMUM: i64 = 30;

/// Vérifie une exclusion à sa décision.
pub fn verifier_exclusion(
    nature: NatureExclusion,
    du: DateCivile,
    au: Option<DateCivile>,
    motif: &str,
    inscrit_le: DateCivile,
) -> Result<(), &'static str> {
    if motif.trim().is_empty() || motif.chars().count() > 300 {
        return Err("exclusion.motif_obligatoire");
    }
    if du < inscrit_le {
        return Err("depart.avant_inscription");
    }
    match (nature, au) {
        (NatureExclusion::Temporaire, None) => Err("exclusion.retour_obligatoire"),
        (NatureExclusion::Temporaire, Some(au)) if au < du => Err("exclusion.bornes_inversees"),
        (NatureExclusion::Temporaire, Some(au))
            if au.jours_depuis_epoque() - du.jours_depuis_epoque() + 1
                > EXCLUSION_TEMPORAIRE_MAXIMUM =>
        {
            Err("exclusion.trop_longue")
        }
        (NatureExclusion::Definitive, Some(_)) => Err("exclusion.definitive_sans_retour"),
        _ => Ok(()),
    }
}

/// Vrai si une exclusion temporaire tient l'élève hors de la classe ce jour-là.
pub fn exclu_le(exclusions: &[Exclusion], jour: DateCivile) -> bool {
    exclusions.iter().any(|e| {
        e.nature == NatureExclusion::Temporaire
            && e.du <= jour
            && e.au.is_some_and(|au| jour <= au)
            && match e.statut {
                StatutExclusion::EnCours => true,
                StatutExclusion::Levee => e.leve_le.is_some_and(|l| jour < l),
                StatutExclusion::Revoquee => false,
            }
    })
}

/// Vérifie un départ vers une autre école : après l'inscription, destination et motif indiqués.
pub fn verifier_depart(
    date_depart: DateCivile,
    inscrit_le: DateCivile,
    destination: &str,
) -> Result<(), &'static str> {
    if date_depart < inscrit_le {
        return Err("depart.avant_inscription");
    }
    if destination.trim().is_empty() || destination.chars().count() > 120 {
        return Err("transfert.destination_obligatoire");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::transition::verifier_exhaustivement;

    fn d(t: &str) -> DateCivile {
        DateCivile::analyser(t).unwrap_or_else(|| unreachable!())
    }

    fn id<T>(n: u128) -> Id<T> {
        Id::depuis_ulid(ulid::Ulid::from(n))
    }

    fn temporaire(du: &str, au: &str, statut: StatutExclusion, leve_le: Option<&str>) -> Exclusion {
        Exclusion {
            id: id(1),
            etablissement_id: id(2),
            inscription_id: id(3),
            nature: NatureExclusion::Temporaire,
            du: d(du),
            au: Some(d(au)),
            motif: "Bagarre".to_owned(),
            decision: String::new(),
            statut,
            leve_le: leve_le.map(d),
            motif_levee: String::new(),
            echeances_annulees: Vec::new(),
        }
    }

    #[test]
    fn etats_de_l_exclusion() {
        verifier_exhaustivement::<StatutExclusion>();
    }

    #[test]
    fn une_exclusion_se_controle() {
        let inscrit = d("2026-10-04");
        let ok = |nature, du, au: Option<&str>, motif| {
            verifier_exclusion(nature, d(du), au.map(d), motif, inscrit)
        };
        assert_eq!(
            ok(
                NatureExclusion::Temporaire,
                "2026-11-02",
                Some("2026-11-04"),
                "Bagarre"
            ),
            Ok(())
        );
        assert_eq!(
            ok(NatureExclusion::Definitive, "2026-11-02", None, "Vol"),
            Ok(())
        );
        assert_eq!(
            ok(NatureExclusion::Temporaire, "2026-11-02", None, "Bagarre"),
            Err("exclusion.retour_obligatoire")
        );
        assert_eq!(
            ok(
                NatureExclusion::Temporaire,
                "2026-11-04",
                Some("2026-11-02"),
                "Bagarre"
            ),
            Err("exclusion.bornes_inversees")
        );
        assert_eq!(
            ok(
                NatureExclusion::Temporaire,
                "2026-11-02",
                Some("2026-12-15"),
                "Bagarre"
            ),
            Err("exclusion.trop_longue")
        );
        assert_eq!(
            ok(NatureExclusion::Definitive, "2026-11-02", None, " "),
            Err("exclusion.motif_obligatoire")
        );
        assert_eq!(
            ok(NatureExclusion::Definitive, "2026-09-02", None, "Vol"),
            Err("depart.avant_inscription")
        );
    }

    #[test]
    fn l_exclusion_temporaire_couvre_ses_jours() {
        let e = [temporaire(
            "2026-11-02",
            "2026-11-04",
            StatutExclusion::EnCours,
            None,
        )];
        assert!(!exclu_le(&e, d("2026-11-01")));
        assert!(exclu_le(&e, d("2026-11-02")));
        assert!(exclu_le(&e, d("2026-11-04")));
        assert!(!exclu_le(&e, d("2026-11-05")));
        // Levée le 3 : exclu le 2 seulement.
        let levee = [temporaire(
            "2026-11-02",
            "2026-11-04",
            StatutExclusion::Levee,
            Some("2026-11-03"),
        )];
        assert!(exclu_le(&levee, d("2026-11-02")));
        assert!(!exclu_le(&levee, d("2026-11-03")));
    }

    #[test]
    fn un_depart_se_controle() {
        let inscrit = d("2026-10-04");
        assert_eq!(
            verifier_depart(d("2027-01-10"), inscrit, "Lycée de Kati"),
            Ok(())
        );
        assert_eq!(
            verifier_depart(d("2026-09-10"), inscrit, "Lycée de Kati"),
            Err("depart.avant_inscription")
        );
        assert_eq!(
            verifier_depart(d("2027-01-10"), inscrit, " "),
            Err("transfert.destination_obligatoire")
        );
    }
}
