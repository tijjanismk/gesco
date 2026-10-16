//! Absences et retards (SPEC_PHASE_4 §6, T-37) : relevés par demi-journée, saisis par classe et
//! par jour, justifiés ensuite. Le bulletin porte, pour la période, les demi-journées d'absence,
//! dont justifiées, et les retards (T-32).

use serde::{Deserialize, Serialize};

use crate::commun::{DateCivile, Id};
use crate::pedagogie::{AffectationEnseignement, Periode};
use crate::scolarite::{Classe, Inscription};

/// Demi-journée du relevé.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum DemiJournee {
    /// Matin.
    Matin,
    /// Après-midi.
    ApresMidi,
}

impl DemiJournee {
    /// Les deux demi-journées, dans l'ordre du jour.
    pub const TOUTES: [Self; 2] = [Self::Matin, Self::ApresMidi];

    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Matin => "matin",
            Self::ApresMidi => "apres_midi",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::TOUTES.into_iter().find(|d| d.nom() == nom)
    }
}

/// Absence ou retard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum NatureAbsence {
    /// Absent toute la demi-journée.
    Absence,
    /// Arrivé en retard.
    Retard,
}

impl NatureAbsence {
    /// Les deux natures.
    pub const TOUTES: [Self; 2] = [Self::Absence, Self::Retard];

    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Absence => "absence",
            Self::Retard => "retard",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::TOUTES.into_iter().find(|n| n.nom() == nom)
    }
}

/// Retard le plus long qu'on note : au-delà, l'élève est absent de la demi-journée.
pub const RETARD_MAXIMUM: i32 = 240;

/// Absence ou retard d'un élève sur une demi-journée.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Absence {
    /// Inscription de l'élève.
    pub inscription_id: Id<Inscription>,
    /// Classe au jour du relevé.
    pub classe_id: Id<Classe>,
    /// Jour.
    pub date: DateCivile,
    /// Matin ou après-midi.
    pub demi_journee: DemiJournee,
    /// Absence ou retard.
    pub nature: NatureAbsence,
    /// Durée du retard en minutes, si elle est connue.
    pub minutes: Option<i32>,
    /// Justifiée (`attendance:justify`).
    pub justifiee: bool,
    /// Motif de la justification : maladie, décès…
    pub motif: String,
    /// Pièce présentée : mot des parents, certificat médical…
    pub justificatif: String,
}

/// Ce qui est tapé dans une case du relevé.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaisieAbsence {
    /// Case vide : présent.
    Present,
    /// `a` : absent.
    Absent,
    /// `r`, ou `r15` pour quinze minutes : en retard.
    Retard(Option<i32>),
}

/// Lit une case du relevé : rien (présent), `a` (absent), `r` ou `r15` (retard, en minutes).
pub fn lire_saisie_absence(texte: &str) -> Result<SaisieAbsence, &'static str> {
    let t: String = texte.to_lowercase().split_whitespace().collect();
    match t.as_str() {
        "" | "p" => return Ok(SaisieAbsence::Present),
        "a" | "abs" => return Ok(SaisieAbsence::Absent),
        "r" => return Ok(SaisieAbsence::Retard(None)),
        _ => {}
    }
    let minutes = t
        .strip_prefix('r')
        .filter(|m| !m.is_empty() && m.len() <= 3 && m.bytes().all(|b| b.is_ascii_digit()))
        .ok_or("absence.illisible")?;
    let n: i32 = minutes.parse().map_err(|_| "absence.illisible")?;
    if !(1..=RETARD_MAXIMUM).contains(&n) {
        return Err("absence.retard_invalide");
    }
    Ok(SaisieAbsence::Retard(Some(n)))
}

/// Texte d'une case : rien, `a`, `r` ou `r15`.
pub fn texte_absence(absence: Option<&Absence>) -> String {
    match absence {
        None => String::new(),
        Some(a) => match (a.nature, a.minutes) {
            (NatureAbsence::Absence, _) => "a".to_owned(),
            (NatureAbsence::Retard, None) => "r".to_owned(),
            (NatureAbsence::Retard, Some(m)) => format!("r{m}"),
        },
    }
}

impl SaisieAbsence {
    /// Nature et durée enregistrées ; rien pour un présent.
    pub const fn nature(self) -> Option<(NatureAbsence, Option<i32>)> {
        match self {
            Self::Present => None,
            Self::Absent => Some((NatureAbsence::Absence, None)),
            Self::Retard(m) => Some((NatureAbsence::Retard, m)),
        }
    }
}

/// Vérifie qu'un jour se relève : ni dans l'avenir, ni hors des périodes du niveau, ni dans une
/// période verrouillée (SPEC_PHASE_4 §2.3).
pub fn verifier_jour(
    date: DateCivile,
    aujourd_hui: DateCivile,
    periodes_du_niveau: &[Periode],
) -> Result<(), &'static str> {
    if date > aujourd_hui {
        return Err("absence.date_future");
    }
    let periode = periodes_du_niveau
        .iter()
        .find(|p| p.debut <= date && date <= p.fin)
        .ok_or("absence.hors_periode")?;
    if periode.verrouillee {
        return Err("periode.verrouillee");
    }
    Ok(())
}

/// Totaux d'un élève sur une période, pour le bulletin (T-32).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct TotauxAbsences {
    /// Demi-journées d'absence.
    pub demi_journees: u32,
    /// Dont justifiées.
    pub justifiees: u32,
    /// Retards.
    pub retards: u32,
    /// Minutes de retard connues.
    pub minutes_de_retard: u32,
}

/// Totaux des absences et retards d'un élève entre deux dates incluses.
pub fn totaux(absences: &[Absence], debut: DateCivile, fin: DateCivile) -> TotauxAbsences {
    absences
        .iter()
        .filter(|a| debut <= a.date && a.date <= fin)
        .fold(TotauxAbsences::default(), |mut t, a| {
            match a.nature {
                NatureAbsence::Absence => {
                    t.demi_journees += 1;
                    if a.justifiee {
                        t.justifiees += 1;
                    }
                }
                NatureAbsence::Retard => {
                    t.retards += 1;
                    t.minutes_de_retard += u32::try_from(a.minutes.unwrap_or(0)).unwrap_or(0);
                }
            }
            t
        })
}

/// Classes où un utilisateur relève les absences.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PorteeAbsences {
    /// Toutes : la direction, le surveillant, le secrétariat, qui justifient aussi.
    Etablissement,
    /// Celles où il enseigne : un enseignant avec son compte.
    Classes {
        /// Classes autorisées.
        classes: Vec<Id<Classe>>,
    },
}

impl PorteeAbsences {
    /// Portée d'un utilisateur : qui justifie relève partout ; un compte lié à un enseignant, dans
    /// les classes de ses affectations ; tout autre compte, nulle part.
    pub fn de(justifie: bool, affectations: &[AffectationEnseignement]) -> Self {
        if justifie {
            Self::Etablissement
        } else {
            let mut classes: Vec<Id<Classe>> = affectations.iter().map(|a| a.classe_id).collect();
            classes.sort();
            classes.dedup();
            Self::Classes { classes }
        }
    }

    /// Vrai si l'utilisateur relève les absences de cette classe.
    pub fn autorise(&self, classe: Id<Classe>) -> bool {
        match self {
            Self::Etablissement => true,
            Self::Classes { classes } => classes.contains(&classe),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pedagogie::TypePeriode;

    fn d(t: &str) -> DateCivile {
        DateCivile::analyser(t).unwrap_or_else(|| unreachable!())
    }

    fn id<T>(n: u128) -> Id<T> {
        Id::depuis_ulid(ulid::Ulid::from(n))
    }

    fn absence(
        date: &str,
        nature: NatureAbsence,
        minutes: Option<i32>,
        justifiee: bool,
    ) -> Absence {
        Absence {
            inscription_id: id(1),
            classe_id: id(2),
            date: d(date),
            demi_journee: DemiJournee::Matin,
            nature,
            minutes,
            justifiee,
            motif: String::new(),
            justificatif: String::new(),
        }
    }

    #[test]
    fn le_releve_lit_les_codes() {
        assert_eq!(lire_saisie_absence(""), Ok(SaisieAbsence::Present));
        assert_eq!(lire_saisie_absence(" A "), Ok(SaisieAbsence::Absent));
        assert_eq!(lire_saisie_absence("r"), Ok(SaisieAbsence::Retard(None)));
        assert_eq!(
            lire_saisie_absence("R15"),
            Ok(SaisieAbsence::Retard(Some(15)))
        );
        assert_eq!(
            lire_saisie_absence("r 20"),
            Ok(SaisieAbsence::Retard(Some(20)))
        );
        assert_eq!(lire_saisie_absence("r0"), Err("absence.retard_invalide"));
        assert_eq!(lire_saisie_absence("r300"), Err("absence.retard_invalide"));
        assert_eq!(lire_saisie_absence("x"), Err("absence.illisible"));
        assert_eq!(lire_saisie_absence("r1h"), Err("absence.illisible"));
    }

    #[test]
    fn une_case_se_relit_telle_qu_ecrite() {
        for texte in ["", "a", "r", "r15"] {
            let saisie = lire_saisie_absence(texte).unwrap_or_else(|_| unreachable!());
            let relue = saisie
                .nature()
                .map(|(nature, minutes)| absence("2026-10-05", nature, minutes, false));
            assert_eq!(texte_absence(relue.as_ref()), texte);
        }
    }

    #[test]
    fn les_totaux_comptent_la_periode_seule() {
        let absences = [
            absence("2026-10-05", NatureAbsence::Absence, None, true),
            absence("2026-10-06", NatureAbsence::Absence, None, false),
            absence("2026-10-07", NatureAbsence::Retard, Some(15), false),
            absence("2026-10-08", NatureAbsence::Retard, None, false),
            // Hors période.
            absence("2026-09-30", NatureAbsence::Absence, None, false),
        ];
        assert_eq!(
            totaux(&absences, d("2026-10-01"), d("2026-10-31")),
            TotauxAbsences {
                demi_journees: 2,
                justifiees: 1,
                retards: 2,
                minutes_de_retard: 15,
            }
        );
    }

    #[test]
    fn un_jour_se_releve_dans_une_periode_ouverte() {
        let mut periode = Periode {
            id: id(1),
            etablissement_id: id(2),
            annee_id: id(3),
            niveau_id: id(4),
            rang: 1,
            type_periode: TypePeriode::Mensuelle,
            debut: d("2026-10-01"),
            fin: d("2026-10-31"),
            ponderation: 1,
            verrouillee: false,
        };
        let aujourd_hui = d("2026-10-11");
        let jour = |date, p: &Periode| verifier_jour(d(date), aujourd_hui, std::slice::from_ref(p));
        assert_eq!(jour("2026-10-05", &periode), Ok(()));
        assert_eq!(jour("2026-10-12", &periode), Err("absence.date_future"));
        assert_eq!(jour("2026-09-30", &periode), Err("absence.hors_periode"));
        periode.verrouillee = true;
        assert_eq!(jour("2026-10-05", &periode), Err("periode.verrouillee"));
    }

    #[test]
    fn la_portee_suit_les_affectations() {
        let affectation = |classe: u128| AffectationEnseignement {
            id: id(90 + classe),
            etablissement_id: id(1),
            annee_id: id(2),
            enseignant_id: id(3),
            classe_id: id(classe),
            matiere_id: id(classe + 50),
        };
        let enseignant = PorteeAbsences::de(false, &[affectation(10), affectation(10)]);
        assert_eq!(
            enseignant,
            PorteeAbsences::Classes {
                classes: vec![id(10)]
            }
        );
        assert!(enseignant.autorise(id(10)));
        assert!(!enseignant.autorise(id(11)));
        assert!(PorteeAbsences::de(true, &[]).autorise(id(11)));
    }
}
