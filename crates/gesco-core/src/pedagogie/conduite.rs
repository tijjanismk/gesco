//! Note de conduite (porteur du projet, 24 octobre 2026) : au 2e cycle, au lycée et dans
//! l'enseignement technique, une note par période sur le barème du niveau, que l'école active
//! comme un module. Selon l'école, la direction ou le surveillant la saisit, ou GESCO la calcule
//! d'après le registre de discipline et les absences : départ au barème, un retrait par
//! sanction, par exclusion temporaire, par demi-journée d'absence non justifiée et par retard,
//! sans descendre sous zéro. Une note calculée se corrige avec un motif. Elle compte dans la
//! moyenne générale avec son coefficient, ou s'imprime à part, au choix de l'école.
//! Une note saisie par erreur se retire avec un motif (R-14).

use serde::{Deserialize, Serialize};

use crate::commun::Id;
use crate::core::{Etablissement, Utilisateur};
use crate::pedagogie::Periode;
use crate::pedagogie::evaluation::{SaisieNote, lire_saisie};
use crate::scolarite::{Cycle, Inscription};

/// Réglages de la note de conduite, dans les paramètres de l'école.
pub mod parametres {
    /// Saisie par la direction ou calculée par GESCO.
    pub const MODE: &str = "conduite.mode";
    /// Vrai si la note compte dans la moyenne générale.
    pub const COMPTEE: &str = "conduite.comptee";
    /// Coefficient de la note quand elle compte.
    pub const COEFFICIENT: &str = "conduite.coefficient";
    /// Points retirés pour un avertissement.
    pub const RETRAIT_AVERTISSEMENT: &str = "conduite.retrait_avertissement";
    /// Points retirés pour un blâme.
    pub const RETRAIT_BLAME: &str = "conduite.retrait_blame";
    /// Points retirés pour une retenue.
    pub const RETRAIT_RETENUE: &str = "conduite.retrait_retenue";
    /// Points retirés pour une convocation des parents.
    pub const RETRAIT_CONVOCATION: &str = "conduite.retrait_convocation";
    /// Points retirés pour une exclusion temporaire.
    pub const RETRAIT_EXCLUSION: &str = "conduite.retrait_exclusion";
    /// Points retirés par demi-journée d'absence non justifiée.
    pub const RETRAIT_ABSENCE: &str = "conduite.retrait_absence";
    /// Points retirés par retard.
    pub const RETRAIT_RETARD: &str = "conduite.retrait_retard";
}

/// Coefficient livré.
pub const COEFFICIENT_DEFAUT: i64 = 1;
/// Coefficients permis.
pub const COEFFICIENTS: std::ops::RangeInclusive<i64> = 1..=10;
/// Longueur maximale d'un motif.
pub const MOTIF_MAX: usize = 200;

/// Les retraits livrés, en points, tels qu'ils s'écrivent dans les paramètres.
pub const RETRAITS_PAR_DEFAUT: [(&str, &str); 7] = [
    (parametres::RETRAIT_AVERTISSEMENT, "2"),
    (parametres::RETRAIT_BLAME, "4"),
    (parametres::RETRAIT_RETENUE, "1"),
    (parametres::RETRAIT_CONVOCATION, "1"),
    (parametres::RETRAIT_EXCLUSION, "5"),
    (parametres::RETRAIT_ABSENCE, "0,5"),
    (parametres::RETRAIT_RETARD, "0,25"),
];

/// Saisie par la direction ou le surveillant, ou calculée par GESCO.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ModeConduite {
    /// La direction ou le surveillant saisit la note de chaque élève.
    Manuelle,
    /// GESCO calcule la note d'après la discipline et les absences ; elle se corrige.
    Automatique,
}

impl ModeConduite {
    /// Les deux modes.
    pub const TOUS: [Self; 2] = [Self::Manuelle, Self::Automatique];
    /// Mode livré : rien n'est noté sans une décision de l'école.
    pub const DEFAUT: Self = Self::Manuelle;

    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Manuelle => "manuelle",
            Self::Automatique => "automatique",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::TOUS.into_iter().find(|m| m.nom() == nom)
    }
}

/// Vrai pour un cycle qui a une note de conduite : le 2e cycle, le lycée et le technique.
pub const fn cycle_note(cycle: Cycle) -> bool {
    matches!(cycle, Cycle::SecondCycle | Cycle::Lycee | Cycle::Technique)
}

/// Lit des points écrits comme une note (`2`, `0,5`, `0.25`) : de 0 à 20, au centième.
pub fn lire_points(texte: &str) -> Option<i32> {
    match lire_saisie(texte, 2000) {
        Ok(SaisieNote::Valeur(v)) => Some(v),
        _ => None,
    }
}

/// Points retirés pour chaque fait, en centièmes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Retraits {
    /// Par avertissement.
    pub avertissement: i32,
    /// Par blâme.
    pub blame: i32,
    /// Par retenue.
    pub retenue: i32,
    /// Par convocation des parents.
    pub convocation: i32,
    /// Par exclusion temporaire.
    pub exclusion: i32,
    /// Par demi-journée d'absence non justifiée.
    pub absence: i32,
    /// Par retard.
    pub retard: i32,
}

impl Retraits {
    /// Les retraits livrés.
    pub fn par_defaut() -> Self {
        let lu = |cle: &str| {
            RETRAITS_PAR_DEFAUT
                .iter()
                .find(|(c, _)| *c == cle)
                .and_then(|(_, t)| lire_points(t))
                .unwrap_or(0)
        };
        Self {
            avertissement: lu(parametres::RETRAIT_AVERTISSEMENT),
            blame: lu(parametres::RETRAIT_BLAME),
            retenue: lu(parametres::RETRAIT_RETENUE),
            convocation: lu(parametres::RETRAIT_CONVOCATION),
            exclusion: lu(parametres::RETRAIT_EXCLUSION),
            absence: lu(parametres::RETRAIT_ABSENCE),
            retard: lu(parametres::RETRAIT_RETARD),
        }
    }
}

/// Ce qui pèse sur la conduite d'un élève pendant une période.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DecompteConduite {
    /// Avertissements.
    pub avertissements: u32,
    /// Blâmes.
    pub blames: u32,
    /// Retenues.
    pub retenues: u32,
    /// Convocations des parents.
    pub convocations: u32,
    /// Exclusions temporaires.
    pub exclusions: u32,
    /// Demi-journées d'absence non justifiées.
    pub absences: u32,
    /// Retards.
    pub retards: u32,
}

/// Note calculée, en centièmes : le barème moins les retraits, jamais sous zéro.
pub fn note_automatique(bareme: i32, retraits: &Retraits, d: &DecompteConduite) -> i32 {
    let fois = |n: u32, points: i32| i64::from(n) * i64::from(points.max(0));
    let retire = fois(d.avertissements, retraits.avertissement)
        + fois(d.blames, retraits.blame)
        + fois(d.retenues, retraits.retenue)
        + fois(d.convocations, retraits.convocation)
        + fois(d.exclusions, retraits.exclusion)
        + fois(d.absences, retraits.absence)
        + fois(d.retards, retraits.retard);
    i32::try_from((i64::from(bareme) - retire).max(0)).unwrap_or(0)
}

/// Note de conduite saisie pour un élève et une période : la note du mode manuel, ou la
/// correction d'une note calculée.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct NoteConduite {
    /// Identifiant technique.
    pub id: Id<NoteConduite>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Inscription de l'élève.
    pub inscription_id: Id<Inscription>,
    /// Période.
    pub periode_id: Id<Periode>,
    /// Note, en centièmes du barème du niveau.
    pub note: i32,
    /// Motif : obligatoire pour corriger une note calculée.
    pub motif: String,
    /// Qui l'a saisie.
    pub saisie_par: Id<Utilisateur>,
    /// Retirée, avec son motif.
    pub annulee: bool,
    /// Motif du retrait.
    pub motif_annulation: String,
}

/// Lit une note de conduite saisie et vérifie son motif : obligatoire pour corriger une note
/// calculée, facultatif pour une note saisie.
pub fn lire_note(
    texte: &str,
    bareme: i32,
    motif: &str,
    mode: ModeConduite,
) -> Result<i32, &'static str> {
    let saisie = lire_saisie(texte, bareme).map_err(|e| {
        if e == "note.illisible" {
            "conduite.note_illisible"
        } else {
            e
        }
    })?;
    let note = match saisie {
        SaisieNote::Valeur(v) => v,
        _ => return Err("conduite.note_illisible"),
    };
    let motif = motif.trim();
    if motif.chars().count() > MOTIF_MAX {
        return Err("conduite.motif_trop_long");
    }
    if mode == ModeConduite::Automatique && motif.is_empty() {
        return Err("conduite.motif_obligatoire");
    }
    Ok(note)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_note_part_du_bareme_et_ne_descend_pas_sous_zero() {
        let r = Retraits::par_defaut();
        assert_eq!(r.absence, 50);
        assert_eq!(r.retard, 25);
        assert_eq!(
            note_automatique(2000, &r, &DecompteConduite::default()),
            2000
        );
        // Un avertissement, une retenue, trois demi-journées et deux retards :
        // 20 − 2 − 1 − 1,5 − 0,5 = 15.
        let d = DecompteConduite {
            avertissements: 1,
            retenues: 1,
            absences: 3,
            retards: 2,
            ..DecompteConduite::default()
        };
        assert_eq!(note_automatique(2000, &r, &d), 1500);
        let lourd = DecompteConduite {
            blames: 3,
            exclusions: 2,
            ..DecompteConduite::default()
        };
        assert_eq!(note_automatique(2000, &r, &lourd), 0);
    }

    #[test]
    fn une_note_calculee_ne_se_corrige_qu_avec_un_motif() {
        assert_eq!(
            lire_note("14,5", 2000, "", ModeConduite::Manuelle),
            Ok(1450)
        );
        assert_eq!(
            lire_note("14,5", 2000, " ", ModeConduite::Automatique),
            Err("conduite.motif_obligatoire")
        );
        assert_eq!(
            lire_note(
                "16",
                2000,
                "Bonne tenue au conseil",
                ModeConduite::Automatique
            ),
            Ok(1600)
        );
        assert_eq!(
            lire_note("21", 2000, "", ModeConduite::Manuelle),
            Err("note.hors_bareme")
        );
        assert_eq!(
            lire_note("a", 2000, "", ModeConduite::Manuelle),
            Err("conduite.note_illisible")
        );
        assert_eq!(lire_points("0,25"), Some(25));
        assert_eq!(lire_points("x"), None);
        assert!(cycle_note(Cycle::Lycee) && !cycle_note(Cycle::PremierCycle));
        for m in ModeConduite::TOUS {
            assert_eq!(ModeConduite::depuis_nom(m.nom()), Some(m));
        }
    }
}
