//! Périodes de notes d'une année, par niveau (SPEC_PHASE_4 §2.3, §24.6 du blueprint) : neuf mois
//! au 1er cycle (Q-22), trois trimestres ensuite, ou deux trimestres pour l'école qui n'en fait
//! que deux (T-26). Au 2e cycle et au lycée, jamais de semestre (réponse du 11 octobre 2026) ;
//! l'enseignement technique compte en deux semestres, ou en trois ou deux trimestres, au choix
//! de l'école niveau par niveau (SPEC_PHASE_6 §10, Q-71).

use serde::{Deserialize, Serialize};

use crate::commun::{DateCivile, Id};
use crate::core::{AnneeScolaire, Etablissement};
use crate::scolarite::{Cycle, Niveau};

/// Nature d'une période.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum TypePeriode {
    /// Un mois, une composition (1er cycle).
    Mensuelle,
    /// Un trimestre.
    Trimestre,
    /// Un semestre, dans l'enseignement technique seulement.
    Semestre,
}

impl TypePeriode {
    /// Les trois natures.
    pub const TOUS: [Self; 3] = [Self::Mensuelle, Self::Trimestre, Self::Semestre];

    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Mensuelle => "mensuelle",
            Self::Trimestre => "trimestre",
            Self::Semestre => "semestre",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::TOUS.into_iter().find(|t| t.nom() == nom)
    }
}

/// Période de notes d'un niveau.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Periode {
    /// Identifiant technique.
    pub id: Id<Periode>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Année.
    pub annee_id: Id<AnneeScolaire>,
    /// Niveau.
    pub niveau_id: Id<Niveau>,
    /// Rang dans l'année, à partir de 1.
    pub rang: i64,
    /// Nature.
    pub type_periode: TypePeriode,
    /// Premier jour.
    pub debut: DateCivile,
    /// Dernier jour, compris.
    pub fin: DateCivile,
    /// Poids dans la moyenne annuelle (1 par défaut).
    pub ponderation: i64,
    /// Vrai si plus aucune note ni absence n'y entre.
    pub verrouillee: bool,
}

const MOIS: [&str; 12] = [
    "Janvier",
    "Février",
    "Mars",
    "Avril",
    "Mai",
    "Juin",
    "Juillet",
    "Août",
    "Septembre",
    "Octobre",
    "Novembre",
    "Décembre",
];

impl Periode {
    /// Libellé imprimé : « Octobre », « 1er trimestre », « 2e semestre ».
    pub fn libelle(&self) -> String {
        let ordinal = if self.rang == 1 {
            "1er".to_owned()
        } else {
            format!("{}e", self.rang)
        };
        match self.type_periode {
            TypePeriode::Mensuelle => MOIS
                .get(usize::from(self.debut.mois()).saturating_sub(1))
                .map_or_else(|| ordinal.clone(), |m| (*m).to_owned()),
            TypePeriode::Trimestre => format!("{ordinal} trimestre"),
            TypePeriode::Semestre => format!("{ordinal} semestre"),
        }
    }
}

fn dernier_jour(annee: i32, mois: u8) -> Option<DateCivile> {
    (28..=31)
        .rev()
        .find_map(|jour| DateCivile::nouvelle(annee, mois, jour))
}

fn mois_entiers(debuts: &[(i32, u8)], fin: (i32, u8)) -> Option<Vec<(DateCivile, DateCivile)>> {
    let mut bornes: Vec<(i32, u8)> = debuts.to_vec();
    bornes.push(fin);
    bornes
        .windows(2)
        .map(|w| match w {
            [(a, m), (a2, m2)] => {
                let debut = DateCivile::nouvelle(*a, *m, 1)?;
                // Dernier jour du mois qui précède le début suivant.
                let (ap, mp) = if *m2 == 1 {
                    (a2 - 1, 12)
                } else {
                    (*a2, m2 - 1)
                };
                Some((debut, dernier_jour(ap, mp)?))
            }
            _ => None,
        })
        .collect()
}

/// Découpage livré d'un cycle : deux semestres dans l'enseignement technique, trois trimestres
/// ailleurs ; le 1er cycle compte en mois, quel que soit ce découpage.
pub const fn decoupage_par_defaut(cycle: Cycle) -> (TypePeriode, u8) {
    match cycle {
        Cycle::Technique => (TypePeriode::Semestre, 2),
        _ => (TypePeriode::Trimestre, 3),
    }
}

/// Vérifie le découpage qu'une école choisit pour un niveau : trois ou deux trimestres au 2e
/// cycle et au lycée ; deux semestres, trois ou deux trimestres dans l'enseignement technique ;
/// rien à choisir au jardin ni au 1er cycle. L'erreur est la clé du message.
pub fn verifier_decoupage(
    cycle: Cycle,
    nature: TypePeriode,
    nombre: u8,
) -> Result<(), &'static str> {
    match (cycle, nature, nombre) {
        (Cycle::Jardin | Cycle::PremierCycle, _, _) => Err("periode.nombre_fixe"),
        (_, TypePeriode::Trimestre, 2 | 3) | (Cycle::Technique, TypePeriode::Semestre, 2) => Ok(()),
        (Cycle::SecondCycle | Cycle::Lycee, TypePeriode::Semestre, _) => {
            Err("periode.semestre_technique")
        }
        _ => Err("periode.nombre_invalide"),
    }
}

/// Calendrier par défaut d'un cycle pour l'année qui commence en `rentree` (2026 pour 2026-2027) :
/// au 1er cycle neuf mois d'octobre à juin ; ensuite trois trimestres, ou deux périodes
/// (octobre-janvier, février-juin) si l'école n'en fait que deux (`nombre` = 2), trimestres ou
/// semestres selon `nature`. Rien pour le jardin (Q-03).
pub fn calendrier_par_defaut(
    cycle: Cycle,
    rentree: i32,
    nature: TypePeriode,
    nombre: u8,
) -> Option<(TypePeriode, Vec<(DateCivile, DateCivile)>)> {
    let suivante = rentree + 1;
    match cycle {
        Cycle::Jardin => None,
        Cycle::PremierCycle => {
            let debuts = [
                (rentree, 10),
                (rentree, 11),
                (rentree, 12),
                (suivante, 1),
                (suivante, 2),
                (suivante, 3),
                (suivante, 4),
                (suivante, 5),
                (suivante, 6),
            ];
            Some((
                TypePeriode::Mensuelle,
                mois_entiers(&debuts, (suivante, 7))?,
            ))
        }
        Cycle::SecondCycle | Cycle::Lycee | Cycle::Technique => {
            if nombre == 2 {
                let debuts = [(rentree, 10), (suivante, 2)];
                Some((nature, mois_entiers(&debuts, (suivante, 7))?))
            } else {
                let debuts = [(rentree, 10), (suivante, 1), (suivante, 4)];
                Some((nature, mois_entiers(&debuts, (suivante, 7))?))
            }
        }
    }
}

/// Vérifie un calendrier corrigé par l'école : dans l'ordre, sans chevauchement, chaque période
/// de son début à sa fin, et un poids de 1 à 10.
pub fn verifier_calendrier(periodes: &[(DateCivile, DateCivile, i64)]) -> Result<(), &'static str> {
    if periodes.is_empty() {
        return Err("periode.aucune");
    }
    let mut precedente: Option<DateCivile> = None;
    for (debut, fin, ponderation) in periodes {
        if fin < debut {
            return Err("periode.bornes_inversees");
        }
        if precedente.is_some_and(|p| *debut <= p) {
            return Err("periode.chevauchement");
        }
        if !(1..=10).contains(ponderation) {
            return Err("periode.ponderation_invalide");
        }
        precedente = Some(*fin);
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn d(t: &str) -> DateCivile {
        DateCivile::analyser(t).unwrap_or_else(|| unreachable!())
    }

    #[test]
    fn le_premier_cycle_a_neuf_mois() {
        let (nature, mois) =
            calendrier_par_defaut(Cycle::PremierCycle, 2026, TypePeriode::Trimestre, 3)
                .unwrap_or_else(|| unreachable!());
        assert_eq!(nature, TypePeriode::Mensuelle);
        assert_eq!(mois.len(), 9);
        assert_eq!(mois[0], (d("2026-10-01"), d("2026-10-31")));
        assert_eq!(mois[4], (d("2027-02-01"), d("2027-02-28")));
        assert_eq!(mois[8], (d("2027-06-01"), d("2027-06-30")));
    }

    #[test]
    fn trois_ou_deux_trimestres() {
        let (nature, t) =
            calendrier_par_defaut(Cycle::SecondCycle, 2026, TypePeriode::Trimestre, 3)
                .unwrap_or_else(|| unreachable!());
        assert_eq!(nature, TypePeriode::Trimestre);
        assert_eq!(
            t,
            vec![
                (d("2026-10-01"), d("2026-12-31")),
                (d("2027-01-01"), d("2027-03-31")),
                (d("2027-04-01"), d("2027-06-30")),
            ]
        );
        let (nature, s) = calendrier_par_defaut(Cycle::Lycee, 2027, TypePeriode::Trimestre, 2)
            .unwrap_or_else(|| unreachable!());
        assert_eq!(nature, TypePeriode::Trimestre);
        assert_eq!(
            s,
            vec![
                (d("2027-10-01"), d("2028-01-31")),
                (d("2028-02-01"), d("2028-06-30"))
            ]
        );
        assert!(calendrier_par_defaut(Cycle::Jardin, 2026, TypePeriode::Trimestre, 3).is_none());
    }

    #[test]
    fn deux_semestres_dans_l_enseignement_technique() {
        let (nature, nombre) = decoupage_par_defaut(Cycle::Technique);
        assert_eq!((nature, nombre), (TypePeriode::Semestre, 2));
        assert_eq!(
            decoupage_par_defaut(Cycle::Lycee),
            (TypePeriode::Trimestre, 3)
        );
        let (nature, s) = calendrier_par_defaut(Cycle::Technique, 2026, nature, nombre)
            .unwrap_or_else(|| unreachable!());
        assert_eq!(nature, TypePeriode::Semestre);
        assert_eq!(
            s,
            vec![
                (d("2026-10-01"), d("2027-01-31")),
                (d("2027-02-01"), d("2027-06-30"))
            ]
        );
        // Les choix de l'école, niveau par niveau.
        use TypePeriode::{Mensuelle, Semestre, Trimestre};
        assert_eq!(verifier_decoupage(Cycle::Technique, Semestre, 2), Ok(()));
        assert_eq!(verifier_decoupage(Cycle::Technique, Trimestre, 3), Ok(()));
        assert_eq!(verifier_decoupage(Cycle::Technique, Trimestre, 2), Ok(()));
        assert_eq!(
            verifier_decoupage(Cycle::Technique, Semestre, 3),
            Err("periode.nombre_invalide")
        );
        assert_eq!(
            verifier_decoupage(Cycle::Technique, Mensuelle, 9),
            Err("periode.nombre_invalide")
        );
        assert_eq!(verifier_decoupage(Cycle::Lycee, Trimestre, 2), Ok(()));
        assert_eq!(
            verifier_decoupage(Cycle::Lycee, Semestre, 2),
            Err("periode.semestre_technique")
        );
        assert_eq!(
            verifier_decoupage(Cycle::SecondCycle, Trimestre, 4),
            Err("periode.nombre_invalide")
        );
        assert_eq!(
            verifier_decoupage(Cycle::PremierCycle, Trimestre, 3),
            Err("periode.nombre_fixe")
        );
    }

    #[test]
    fn libelles_imprimes() {
        let p = |rang, type_periode, debut| Periode {
            id: Id::depuis_ulid(ulid::Ulid::from(1u128)),
            etablissement_id: Id::depuis_ulid(ulid::Ulid::from(2u128)),
            annee_id: Id::depuis_ulid(ulid::Ulid::from(3u128)),
            niveau_id: Id::depuis_ulid(ulid::Ulid::from(4u128)),
            rang,
            type_periode,
            debut: d(debut),
            fin: d(debut),
            ponderation: 1,
            verrouillee: false,
        };
        assert_eq!(
            p(4, TypePeriode::Mensuelle, "2027-01-01").libelle(),
            "Janvier"
        );
        assert_eq!(
            p(1, TypePeriode::Trimestre, "2026-10-01").libelle(),
            "1er trimestre"
        );
        assert_eq!(
            p(2, TypePeriode::Semestre, "2027-02-01").libelle(),
            "2e semestre"
        );
    }

    #[test]
    fn un_calendrier_corrige_se_controle() {
        let ok = [
            (d("2026-10-01"), d("2026-12-20"), 1),
            (d("2027-01-04"), d("2027-03-31"), 2),
        ];
        assert_eq!(verifier_calendrier(&ok), Ok(()));
        assert_eq!(verifier_calendrier(&[]), Err("periode.aucune"));
        assert_eq!(
            verifier_calendrier(&[(d("2026-12-01"), d("2026-10-01"), 1)]),
            Err("periode.bornes_inversees")
        );
        assert_eq!(
            verifier_calendrier(&[
                (d("2026-10-01"), d("2026-12-31"), 1),
                (d("2026-12-31"), d("2027-03-31"), 1)
            ]),
            Err("periode.chevauchement")
        );
        assert_eq!(
            verifier_calendrier(&[(d("2026-10-01"), d("2026-12-31"), 0)]),
            Err("periode.ponderation_invalide")
        );
    }
}
