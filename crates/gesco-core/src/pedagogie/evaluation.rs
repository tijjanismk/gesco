//! Évaluations et notes (SPEC_PHASE_4 §4, §8.4 et §10.4 du blueprint) : une évaluation d'une
//! matière d'une classe dans une période, et la note de chaque élève, stockée telle qu'écrite
//! avec le barème de l'évaluation (RC-2). L'absent n'est pas un zéro (RC-3).

use serde::{Deserialize, Serialize};

use crate::commun::{DateCivile, Id};
use crate::core::{AnneeScolaire, Etablissement, MachineAEtats};
use crate::pedagogie::{ContenuRegle, Enseignant, Matiere, Periode, TypeEvaluation};
use crate::scolarite::{Classe, Inscription};

/// État d'une évaluation. Le blueprint (§10.4) prévoyait aussi « brouillon » et « publiée » :
/// la saisie commence dès la création, et c'est le bulletin qui se publie (SPEC_PHASE_4 §4.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum StatutEvaluation {
    /// Les notes se saisissent et se corrigent.
    EnSaisie,
    /// Validée par la direction : plus aucune note ne change sans déverrouillage motivé.
    Validee,
}

impl MachineAEtats for StatutEvaluation {
    const ETATS: &'static [Self] = &[Self::EnSaisie, Self::Validee];
    const TRANSITIONS: &'static [(Self, Self)] = &[
        (Self::EnSaisie, Self::Validee),
        // Déverrouillage motivé (`grades:unlock`).
        (Self::Validee, Self::EnSaisie),
    ];

    fn nom(&self) -> &'static str {
        match self {
            Self::EnSaisie => "en_saisie",
            Self::Validee => "validee",
        }
    }
}

/// Évaluation : un devoir ou une composition d'une matière d'une classe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Evaluation {
    /// Identifiant technique.
    pub id: Id<Evaluation>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Année.
    pub annee_id: Id<AnneeScolaire>,
    /// Période.
    pub periode_id: Id<Periode>,
    /// Classe.
    pub classe_id: Id<Classe>,
    /// Matière.
    pub matiere_id: Id<Matiere>,
    /// Devoir ou composition.
    pub type_evaluation: TypeEvaluation,
    /// « Devoir 1 », « Composition de mars ».
    pub libelle: String,
    /// Jour de l'évaluation.
    pub date: DateCivile,
    /// Barème, en centièmes (`2000` pour 20).
    pub bareme: i32,
    /// Pondération (1 par défaut).
    pub ponderation: i32,
    /// Enseignant correcteur, s'il est connu.
    pub enseignant_id: Option<Id<Enseignant>>,
    /// État.
    pub statut: StatutEvaluation,
    /// Motif du dernier déverrouillage.
    pub motif_deverrouillage: String,
}

/// Note d'un élève à une évaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct NoteEleve {
    /// Évaluation.
    pub evaluation_id: Id<Evaluation>,
    /// Inscription de l'élève.
    pub inscription_id: Id<Inscription>,
    /// Note en centièmes ; absente pour un absent ou un dispensé.
    pub valeur: Option<i32>,
    /// Absent.
    pub absent: bool,
    /// Absence justifiée.
    pub justifie: bool,
    /// Dispensé.
    pub dispense: bool,
}

/// Ce que la direction tape dans la grille pour un élève.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaisieNote {
    /// Une note, en centièmes.
    Valeur(i32),
    /// `a` : absent, sans justification à ce jour.
    Absent,
    /// `aj` : absent justifié.
    AbsentJustifie,
    /// `d` : dispensé.
    Dispense,
    /// Case vidée : la note est retirée.
    Effacee,
}

/// Lit une saisie de la grille (Q-57) : `13`, `13,5`, `13.50`, `a`, `aj`, `d`, ou rien. Refuse
/// une note hors du barème ou plus fine que le centième.
pub fn lire_saisie(texte: &str, bareme: i32) -> Result<SaisieNote, &'static str> {
    let t = texte.trim().to_lowercase();
    match t.as_str() {
        "" => return Ok(SaisieNote::Effacee),
        "a" | "abs" => return Ok(SaisieNote::Absent),
        "aj" => return Ok(SaisieNote::AbsentJustifie),
        "d" | "disp" => return Ok(SaisieNote::Dispense),
        _ => {}
    }
    let (entier, decimales) = match t.split_once([',', '.']) {
        Some((e, d)) => (e, d),
        None => (t.as_str(), ""),
    };
    if entier.is_empty()
        || entier.len() > 3
        || decimales.len() > 2
        || !entier.bytes().all(|b| b.is_ascii_digit())
        || !decimales.bytes().all(|b| b.is_ascii_digit())
    {
        return Err("note.illisible");
    }
    let e: i32 = entier.parse().map_err(|_| "note.illisible")?;
    let d: i32 = if decimales.is_empty() {
        0
    } else {
        format!("{decimales:0<2}")
            .parse()
            .map_err(|_| "note.illisible")?
    };
    let valeur = e * 100 + d;
    if valeur > bareme {
        return Err("note.hors_bareme");
    }
    Ok(SaisieNote::Valeur(valeur))
}

impl SaisieNote {
    /// Note enregistrée pour cette saisie ; rien pour une case vidée.
    pub const fn en_note(
        self,
        evaluation_id: Id<Evaluation>,
        inscription_id: Id<Inscription>,
    ) -> Option<NoteEleve> {
        let (valeur, absent, justifie, dispense) = match self {
            Self::Valeur(v) => (Some(v), false, false, false),
            Self::Absent => (None, true, false, false),
            Self::AbsentJustifie => (None, true, true, false),
            Self::Dispense => (None, false, false, true),
            Self::Effacee => return None,
        };
        Some(NoteEleve {
            evaluation_id,
            inscription_id,
            valeur,
            absent,
            justifie,
            dispense,
        })
    }
}

/// Texte d'une note dans la grille et dans le classeur exporté : `13,5`, `20`, `a`, `aj`, `d`.
pub fn texte_note(note: &NoteEleve) -> String {
    if note.dispense {
        return "d".to_owned();
    }
    if note.absent {
        return if note.justifie { "aj" } else { "a" }.to_owned();
    }
    match note.valeur {
        Some(v) if v % 100 == 0 => (v / 100).to_string(),
        Some(v) => {
            let decimales = format!("{:02}", v % 100);
            format!("{},{}", v / 100, decimales.trim_end_matches('0'))
        }
        None => String::new(),
    }
}

/// Vérifie une évaluation à sa création : type permis dans le cycle, barème et pondération
/// raisonnables, libellé, date dans la période, période ouverte.
pub fn verifier_evaluation(
    regle: &ContenuRegle,
    periode: &Periode,
    type_evaluation: TypeEvaluation,
    libelle: &str,
    date: DateCivile,
    bareme: i32,
    ponderation: i32,
) -> Result<(), &'static str> {
    if regle.bareme_de(type_evaluation).is_none() {
        return Err("evaluation.type_interdit");
    }
    if libelle.trim().is_empty() || libelle.chars().count() > 60 {
        return Err("evaluation.libelle_invalide");
    }
    if !(100..=10_000).contains(&bareme) {
        return Err("evaluation.bareme_invalide");
    }
    if !(1..=10).contains(&ponderation) {
        return Err("evaluation.ponderation_invalide");
    }
    if periode.verrouillee {
        return Err("periode.verrouillee");
    }
    if date < periode.debut || date > periode.fin {
        return Err("evaluation.hors_periode");
    }
    Ok(())
}

/// Vrai si chaque élève a une note, une absence ou une dispense : l'évaluation peut être validée.
pub fn complete(inscriptions: &[Id<Inscription>], notes: &[NoteEleve]) -> bool {
    inscriptions
        .iter()
        .all(|i| notes.iter().any(|n| n.inscription_id == *i))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::transition::verifier_exhaustivement;
    use crate::pedagogie::TypePeriode;
    use crate::scolarite::Cycle;

    fn d(t: &str) -> DateCivile {
        DateCivile::analyser(t).unwrap_or_else(|| unreachable!())
    }

    fn id<T>(n: u128) -> Id<T> {
        Id::depuis_ulid(ulid::Ulid::from(n))
    }

    #[test]
    fn etats_de_l_evaluation() {
        verifier_exhaustivement::<StatutEvaluation>();
    }

    #[test]
    fn la_grille_lit_notes_et_codes() {
        assert_eq!(lire_saisie("13", 2000), Ok(SaisieNote::Valeur(1300)));
        assert_eq!(lire_saisie("13,5", 2000), Ok(SaisieNote::Valeur(1350)));
        assert_eq!(lire_saisie("13.25", 2000), Ok(SaisieNote::Valeur(1325)));
        assert_eq!(lire_saisie("0", 2000), Ok(SaisieNote::Valeur(0)));
        assert_eq!(lire_saisie("20", 2000), Ok(SaisieNote::Valeur(2000)));
        assert_eq!(lire_saisie(" A ", 2000), Ok(SaisieNote::Absent));
        assert_eq!(lire_saisie("aj", 2000), Ok(SaisieNote::AbsentJustifie));
        assert_eq!(lire_saisie("D", 2000), Ok(SaisieNote::Dispense));
        assert_eq!(lire_saisie("", 2000), Ok(SaisieNote::Effacee));
        assert_eq!(lire_saisie("20,5", 2000), Err("note.hors_bareme"));
        assert_eq!(lire_saisie("11", 1000), Err("note.hors_bareme"));
        assert_eq!(lire_saisie("13,555", 2000), Err("note.illisible"));
        assert_eq!(lire_saisie("-2", 2000), Err("note.illisible"));
        assert_eq!(lire_saisie("douze", 2000), Err("note.illisible"));
    }

    #[test]
    fn une_note_se_relit_telle_qu_ecrite() {
        for texte in ["13,5", "20", "9,25", "0", "a", "aj", "d"] {
            let note = lire_saisie(texte, 2000)
                .ok()
                .and_then(|s| s.en_note(id(1), id(2)))
                .unwrap_or_else(|| unreachable!());
            assert_eq!(texte_note(&note), texte);
        }
    }

    #[test]
    fn une_absence_n_est_jamais_un_zero() {
        let n = SaisieNote::Absent
            .en_note(id(1), id(2))
            .unwrap_or_else(|| unreachable!());
        assert_eq!(n.valeur, None);
        assert!(n.absent && !n.justifie);
        assert_eq!(SaisieNote::Effacee.en_note(id(1), id(2)), None);
    }

    #[test]
    fn une_evaluation_se_controle() {
        let regle = ContenuRegle::par_defaut(Cycle::PremierCycle).unwrap_or_else(|| unreachable!());
        let mut periode = Periode {
            id: id(1),
            etablissement_id: id(2),
            annee_id: id(3),
            niveau_id: id(4),
            rang: 2,
            type_periode: TypePeriode::Mensuelle,
            debut: d("2026-11-01"),
            fin: d("2026-11-30"),
            ponderation: 1,
            verrouillee: false,
        };
        let ok = |t, date, bareme| {
            verifier_evaluation(
                &regle,
                &periode,
                t,
                "Composition de novembre",
                d(date),
                bareme,
                1,
            )
        };
        assert_eq!(ok(TypeEvaluation::Composition, "2026-11-20", 1000), Ok(()));
        // Pas de devoir au 1er cycle (§8.4bis).
        assert_eq!(
            ok(TypeEvaluation::Devoir, "2026-11-20", 1000),
            Err("evaluation.type_interdit")
        );
        assert_eq!(
            ok(TypeEvaluation::Composition, "2026-12-02", 1000),
            Err("evaluation.hors_periode")
        );
        assert_eq!(
            ok(TypeEvaluation::Composition, "2026-11-20", 0),
            Err("evaluation.bareme_invalide")
        );
        periode.verrouillee = true;
        assert_eq!(
            verifier_evaluation(
                &regle,
                &periode,
                TypeEvaluation::Composition,
                "x",
                d("2026-11-20"),
                1000,
                1
            ),
            Err("periode.verrouillee")
        );
    }

    #[test]
    fn la_validation_attend_chaque_eleve() {
        let note = |i| NoteEleve {
            evaluation_id: id(9),
            inscription_id: id(i),
            valeur: Some(1000),
            absent: false,
            justifie: false,
            dispense: false,
        };
        let eleves = [id(1), id(2)];
        assert!(!complete(&eleves, &[note(1)]));
        assert!(complete(&eleves, &[note(1), note(2)]));
    }
}
