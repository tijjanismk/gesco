//! Résultats d'une classe sur une période (SPEC_PHASE_4 §7.1) : pour chaque élève, chaque
//! matière du programme avec sa note de classe, sa composition, sa moyenne, son coefficient et
//! ses points ; puis le total, la moyenne générale, le rang et la mention. Pas de rang dans une
//! matière : seule la moyenne générale classe les élèves (réponse du 12 octobre 2026). Jamais saisis,
//! toujours recalculés à partir des notes avec le moteur de [`crate::pedagogie::calcul`].

use serde::{Deserialize, Serialize};

use crate::commun::Id;
use crate::pedagogie::calcul::{
    Depassement, Fraction, NoteCalcul, Rang, Resume, classer, moyenne_annuelle, moyenne_generale,
    moyenne_matiere, note_de_classe,
};
use crate::pedagogie::{ContenuRegle, FormuleMatiere, Matiere, TypeEvaluation};
use crate::scolarite::Inscription;

/// Notes d'un élève sur la période, matière par matière.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotesEleve {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Notes, chacune avec sa matière.
    pub notes: Vec<(Id<Matiere>, NoteCalcul)>,
    /// Faux pour un élève arrivé pendant la période quand l'école ne le classe pas encore
    /// (Q-29) : il a sa moyenne, sans rang, et ne déplace pas le rang des autres.
    pub classable: bool,
    /// Note de conduite de la période, en centièmes du barème ; rien sans note de conduite.
    pub conduite: Option<i32>,
}

/// Ligne d'une matière sur le bulletin d'un élève. Toutes les valeurs sont en centièmes du
/// barème du niveau, telles qu'imprimées (arrondi de l'école).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct LigneMatiere {
    /// Matière.
    pub matiere_id: Id<Matiere>,
    /// Coefficient (1 au 1er cycle).
    pub coefficient: i64,
    /// Note de classe : la moyenne des devoirs.
    pub note_de_classe: Option<i32>,
    /// Composition.
    pub composition: Option<i32>,
    /// Moyenne de la matière ; rien si elle n'est pas notée (Q-59).
    pub moyenne: Option<i32>,
    /// Moyenne × coefficient.
    pub points: Option<i64>,
}

/// Résultat d'un élève sur la période.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ResultatEleve {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Matières du programme, dans son ordre.
    pub matieres: Vec<LigneMatiere>,
    /// Total des points des matières notées.
    pub total_points: i64,
    /// Somme des coefficients des matières notées.
    pub total_coefficients: i64,
    /// Note de conduite.
    pub conduite: Option<i32>,
    /// Note de conduite × son coefficient, quand elle compte dans la moyenne générale.
    pub points_conduite: Option<i64>,
    /// Moyenne générale.
    pub moyenne: Option<i32>,
    /// Rang dans la classe ; rien pour un élève sans moyenne.
    pub rang: Option<Rang>,
    /// Mention de l'école.
    pub mention: Option<String>,
}

/// Moyenne de la classe dans une matière.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct MoyenneDeClasse {
    /// Matière.
    pub matiere_id: Id<Matiere>,
    /// Moyenne des moyennes des élèves notés.
    pub moyenne: Option<i32>,
}

/// Résultats d'une classe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ResultatsClasse {
    /// Élèves, dans l'ordre reçu.
    pub eleves: Vec<ResultatEleve>,
    /// Effectif classé, moyenne du premier et du dernier.
    pub resume: Resume,
    /// Élèves qui ont composé : ceux qui ont une moyenne, classés ou non.
    pub ont_compose: u32,
    /// Moyenne générale de la classe : la moyenne des moyennes générales.
    pub moyenne_classe: Option<i32>,
    /// Moyenne de la classe dans chaque matière.
    pub moyennes_matieres: Vec<MoyenneDeClasse>,
}

fn moyenne_des(valeurs: &[i32], regle: &ContenuRegle) -> Result<Option<i32>, Depassement> {
    if valeurs.is_empty() {
        return Ok(None);
    }
    let somme: i128 = valeurs.iter().map(|v| i128::from(*v)).sum();
    let n = i128::try_from(valeurs.len()).map_err(|_| Depassement)?;
    Ok(Some(Fraction::nouvelle(somme, n)?.arrondie(regle.arrondi)))
}

/// Calcule les résultats d'une classe : `matieres` est le programme (matière, coefficient), dans
/// l'ordre du bulletin ; `bareme` celui du niveau. `coefficient_conduite` est celui de la note de
/// conduite quand elle compte dans la moyenne générale : elle s'y ajoute comme une matière, pour
/// un élève noté dans au moins une matière.
pub fn calculer_classe(
    regle: &ContenuRegle,
    bareme: i32,
    matieres: &[(Id<Matiere>, i64)],
    eleves: &[NotesEleve],
    coefficient_conduite: Option<i64>,
) -> Result<ResultatsClasse, Depassement> {
    let arrondi = regle.arrondi;
    let absent = regle.absent_non_justifie;
    let report = regle.tronquer_avant_report.then_some(arrondi);
    let mut resultats = Vec::with_capacity(eleves.len());
    for eleve in eleves {
        let mut lignes = Vec::with_capacity(matieres.len());
        let mut pour_generale = Vec::with_capacity(matieres.len());
        for (matiere_id, coefficient) in matieres {
            let notes: Vec<NoteCalcul> = eleve
                .notes
                .iter()
                .filter(|(m, _)| m == matiere_id)
                .map(|(_, n)| *n)
                .collect();
            let compositions: Vec<NoteCalcul> = notes
                .iter()
                .filter(|n| n.type_evaluation == TypeEvaluation::Composition)
                .copied()
                .collect();
            let exacte = moyenne_matiere(&notes, regle.formule, absent, bareme)?;
            let moyenne = exacte.map(|f| f.arrondie(arrondi));
            pour_generale.push((exacte, *coefficient));
            lignes.push(LigneMatiere {
                matiere_id: *matiere_id,
                coefficient: *coefficient,
                note_de_classe: note_de_classe(&notes, absent, bareme)?
                    .map(|f| f.arrondie(arrondi)),
                composition: moyenne_matiere(
                    &compositions,
                    FormuleMatiere::SommeDesBaremes,
                    absent,
                    bareme,
                )?
                .map(|f| f.arrondie(arrondi)),
                points: moyenne.map(|m| i64::from(m) * coefficient),
                moyenne,
            });
        }
        let notees = lignes.iter().filter(|l| l.moyenne.is_some());
        let mut total_points: i64 = notees.clone().filter_map(|l| l.points).sum();
        let mut total_coefficients: i64 = notees.clone().map(|l| l.coefficient).sum();
        let points_conduite = match (eleve.conduite, coefficient_conduite) {
            (Some(note), Some(coefficient)) if coefficient > 0 && notees.count() > 0 => {
                pour_generale.push((Some(Fraction::entiere(note)), coefficient));
                let points = i64::from(note) * coefficient;
                total_points += points;
                total_coefficients += coefficient;
                Some(points)
            }
            _ => None,
        };
        let moyenne = moyenne_generale(&pour_generale, report)?.map(|f| f.arrondie(arrondi));
        resultats.push(ResultatEleve {
            inscription_id: eleve.inscription_id,
            matieres: lignes,
            total_points,
            total_coefficients,
            conduite: eleve.conduite,
            points_conduite,
            mention: moyenne.and_then(|m| regle.mention(m)).map(str::to_owned),
            moyenne,
            rang: None,
        });
    }
    let classables: Vec<Option<i32>> = resultats
        .iter()
        .zip(eleves)
        .map(|(r, e)| r.moyenne.filter(|_| e.classable))
        .collect();
    let (rangs, resume) = classer(&classables);
    for (r, rang) in resultats.iter_mut().zip(rangs) {
        r.rang = rang;
    }
    let mut moyennes_matieres = Vec::with_capacity(matieres.len());
    for (i, (matiere_id, _)) in matieres.iter().enumerate() {
        let colonne: Vec<Option<i32>> = resultats
            .iter()
            .map(|r| r.matieres.get(i).and_then(|l| l.moyenne))
            .collect();
        moyennes_matieres.push(MoyenneDeClasse {
            matiere_id: *matiere_id,
            moyenne: moyenne_des(&colonne.into_iter().flatten().collect::<Vec<_>>(), regle)?,
        });
    }
    let generales: Vec<i32> = resultats.iter().filter_map(|r| r.moyenne).collect();
    Ok(ResultatsClasse {
        ont_compose: u32::try_from(generales.len()).unwrap_or(u32::MAX),
        moyenne_classe: moyenne_des(&generales, regle)?,
        eleves: resultats,
        resume,
        moyennes_matieres,
    })
}

/// Moyenne et rang annuels d'un élève (T-33).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Annuelle {
    /// Moyenne annuelle, selon l'arrondi de l'école.
    pub moyenne: Option<i32>,
    /// Rang sur la moyenne annuelle.
    pub rang: Option<Rang>,
}

/// Moyennes annuelles d'une classe (T-33, §8.4bis niveau 3) : pour chaque élève, ses moyennes de
/// période telles qu'imprimées avec le poids de chaque période ; une période non suivie vaut
/// rien et sort du calcul (Q-27), une moyenne reprise d'une autre école y entre (Q-28). Tous les
/// élèves qui ont une moyenne annuelle sont classés sur les périodes qu'ils ont suivies (Q-29).
pub fn moyennes_annuelles(
    regle: &ContenuRegle,
    periodes_par_eleve: &[Vec<(Option<i32>, i64)>],
) -> Result<Vec<Annuelle>, Depassement> {
    let moyennes = periodes_par_eleve
        .iter()
        .map(|p| Ok(moyenne_annuelle(p)?.map(|f| f.arrondie(regle.arrondi))))
        .collect::<Result<Vec<_>, Depassement>>()?;
    let (rangs, _) = classer(&moyennes);
    Ok(moyennes
        .into_iter()
        .zip(rangs)
        .map(|(moyenne, rang)| Annuelle { moyenne, rang })
        .collect())
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::scolarite::Cycle;

    fn id<T>(n: u128) -> Id<T> {
        Id::depuis_ulid(ulid::Ulid::from(n))
    }

    fn note(type_evaluation: TypeEvaluation, valeur: Option<i32>, bareme: i32) -> NoteCalcul {
        NoteCalcul {
            type_evaluation,
            bareme,
            ponderation: 1,
            valeur,
            absent: valeur.is_none(),
            justifie: false,
            dispense: false,
        }
    }

    const MATH: u128 = 20;
    const FR: u128 = 21;
    const ANG: u128 = 22;

    fn notes(i: u128, liste: &[(u128, TypeEvaluation, Option<i32>)]) -> NotesEleve {
        NotesEleve {
            inscription_id: id(i),
            notes: liste
                .iter()
                .map(|(m, t, v)| (id(*m), note(*t, *v, 2000)))
                .collect(),
            classable: true,
            conduite: None,
        }
    }

    #[test]
    fn une_classe_du_second_cycle() {
        use TypeEvaluation::{Composition as K, Devoir as D};
        let regle = ContenuRegle::par_defaut(Cycle::SecondCycle).unwrap_or_else(|| unreachable!());
        let programme = [(id(MATH), 4), (id(FR), 4), (id(ANG), 2)];
        let eleves = [
            // Cas 2 en maths : devoirs 12 et 15, composition 11 → 11,83 ; français 14 ; anglais
            // non noté (Q-59).
            notes(
                1,
                &[
                    (MATH, D, Some(1200)),
                    (MATH, D, Some(1500)),
                    (MATH, K, Some(1100)),
                    (FR, K, Some(1400)),
                ],
            ),
            notes(
                2,
                &[
                    (MATH, K, Some(1600)),
                    (FR, K, Some(1000)),
                    (ANG, K, Some(1200)),
                ],
            ),
            notes(
                3,
                &[
                    (MATH, K, Some(1300)),
                    (FR, K, Some(1300)),
                    (ANG, K, Some(800)),
                ],
            ),
            // Absent partout : ni moyenne, ni rang.
            notes(4, &[(MATH, K, None)]),
        ];
        let r = calculer_classe(&regle, 2000, &programme, &eleves, None)
            .unwrap_or_else(|_| unreachable!());
        let a = &r.eleves[0];
        assert_eq!(a.matieres[0].note_de_classe, Some(1350));
        assert_eq!(a.matieres[0].composition, Some(1100));
        assert_eq!(a.matieres[0].moyenne, Some(1183));
        assert_eq!(a.matieres[0].points, Some(4732));
        assert_eq!(a.matieres[2].moyenne, None);
        // (47,32 + 56) ÷ 8 = 12,915 → 12,91.
        assert_eq!((a.total_points, a.total_coefficients), (10_332, 8));
        assert_eq!(a.moyenne, Some(1291));
        assert_eq!(a.mention.as_deref(), Some("Assez bien"));
        // B : (64 + 40 + 24) ÷ 10 = 12,80 ; C : (52 + 52 + 16) ÷ 10 = 12,00.
        assert_eq!(r.eleves[1].moyenne, Some(1280));
        assert_eq!(r.eleves[2].moyenne, Some(1200));
        assert_eq!(
            r.eleves[0].rang,
            Some(Rang {
                rang: 1,
                ex_aequo: false
            })
        );
        assert_eq!(
            r.eleves[2].rang,
            Some(Rang {
                rang: 3,
                ex_aequo: false
            })
        );
        assert_eq!(r.eleves[3].moyenne, None);
        assert_eq!(r.eleves[3].rang, None);
        assert_eq!(
            r.resume,
            Resume {
                classes: 3,
                premier: Some(1291),
                dernier: Some(1200)
            }
        );
        // (12,91 + 12,80 + 12,00) ÷ 3 = 12,57.
        assert_eq!(r.moyenne_classe, Some(1257));
        // Moyenne de la classe en anglais : (12 + 8) ÷ 2 = 10.
        assert_eq!(r.moyennes_matieres[2].moyenne, Some(1000));

        // Note de conduite comptée au coefficient 1 : A a 16 de conduite, (103,32 + 16) ÷ 9 =
        // 13,25 ; D, sans aucune matière notée, n'a toujours pas de moyenne.
        let mut avec_conduite = eleves.clone();
        avec_conduite[0].conduite = Some(1600);
        avec_conduite[3].conduite = Some(2000);
        let c = calculer_classe(&regle, 2000, &programme, &avec_conduite, Some(1))
            .unwrap_or_else(|_| unreachable!());
        assert_eq!(
            (c.eleves[0].total_points, c.eleves[0].total_coefficients),
            (11_932, 9)
        );
        assert_eq!(c.eleves[0].points_conduite, Some(1600));
        assert_eq!(c.eleves[0].moyenne, Some(1325));
        assert_eq!(c.eleves[3].moyenne, None);
        assert_eq!(c.eleves[3].points_conduite, None);
        // Imprimée à part, elle ne touche ni aux totaux ni à la moyenne.
        let a_part = calculer_classe(&regle, 2000, &programme, &avec_conduite, None)
            .unwrap_or_else(|_| unreachable!());
        assert_eq!(a_part.eleves[0].conduite, Some(1600));
        assert_eq!(a_part.eleves[0].points_conduite, None);
        assert_eq!(a_part.eleves[0].moyenne, Some(1291));
    }

    #[test]
    fn une_classe_du_premier_cycle_sur_dix() {
        let regle = ContenuRegle::par_defaut(Cycle::PremierCycle).unwrap_or_else(|| unreachable!());
        // Cas 1 : cinq domaines sans coefficient, 7,5 ; 6 ; 8 ; 9 ; 7 → 7,50, « Bien ».
        let programme: Vec<(Id<Matiere>, i64)> = (30..35).map(|m| (id(m), 1)).collect();
        let eleve = NotesEleve {
            inscription_id: id(1),
            notes: [750, 600, 800, 900, 700]
                .iter()
                .zip(30..35)
                .map(|(v, m)| (id(m), note(TypeEvaluation::Composition, Some(*v), 1000)))
                .collect(),
            classable: true,
            conduite: None,
        };
        let r = calculer_classe(&regle, 1000, &programme, &[eleve], None)
            .unwrap_or_else(|_| unreachable!());
        assert_eq!(r.eleves[0].total_points, 3750);
        assert_eq!(r.eleves[0].total_coefficients, 5);
        assert_eq!(r.eleves[0].moyenne, Some(750));
        assert_eq!(r.eleves[0].mention.as_deref(), Some("Bien"));
        assert_eq!(r.resume.classes, 1);
    }

    #[test]
    fn un_eleve_arrive_peut_rester_non_classe() {
        let regle = ContenuRegle::par_defaut(Cycle::SecondCycle).unwrap_or_else(|| unreachable!());
        let programme = [(id(MATH), 1)];
        let mut arrive = notes(3, &[(MATH, TypeEvaluation::Composition, Some(1900))]);
        arrive.classable = false;
        let eleves = [
            notes(1, &[(MATH, TypeEvaluation::Composition, Some(1200))]),
            notes(2, &[(MATH, TypeEvaluation::Composition, Some(1000))]),
            arrive,
        ];
        let r = calculer_classe(&regle, 2000, &programme, &eleves, None)
            .unwrap_or_else(|_| unreachable!());
        // L'arrivé a sa moyenne, sans rang ; le premier reste premier.
        assert_eq!(r.eleves[2].moyenne, Some(1900));
        assert_eq!(r.eleves[2].rang, None);
        assert_eq!(r.eleves[0].rang.map(|x| x.rang), Some(1));
        assert_eq!(r.resume.classes, 2);
        assert_eq!(r.ont_compose, 3);
    }

    #[test]
    fn moyennes_annuelles_du_cas_8() {
        let regle = ContenuRegle::par_defaut(Cycle::SecondCycle).unwrap_or_else(|| unreachable!());
        let a = moyennes_annuelles(
            &regle,
            &[
                // Trois trimestres : 12,42.
                vec![(Some(1177), 1), (Some(1240), 1), (Some(1310), 1)],
                // Arrivé au 2e trimestre, le 1er non suivi : 12,75.
                vec![(None, 1), (Some(1240), 1), (Some(1310), 1)],
                // Arrivé au 2e trimestre avec une moyenne reprise de 12,50 : 12,66.
                vec![(Some(1250), 1), (Some(1240), 1), (Some(1310), 1)],
            ],
        )
        .unwrap_or_else(|_| unreachable!());
        assert_eq!(a[0].moyenne, Some(1242));
        assert_eq!(a[1].moyenne, Some(1275));
        assert_eq!(a[2].moyenne, Some(1266));
        assert_eq!(a[1].rang.map(|x| x.rang), Some(1));
        assert_eq!(a[0].rang.map(|x| x.rang), Some(3));
        // École à deux trimestres : 10,50.
        let deux = moyennes_annuelles(&regle, &[vec![(Some(1000), 1), (Some(1101), 1)]])
            .unwrap_or_else(|_| unreachable!());
        assert_eq!(deux[0].moyenne, Some(1050));
    }

    use proptest::prelude::*;

    /// Une classe tirée au hasard : pour chaque élève, (matière, devoir ou composition, note ou
    /// absence) sur 20.
    fn classe() -> impl Strategy<Value = Vec<Vec<(u128, bool, Option<i32>)>>> {
        prop::collection::vec(
            prop::collection::vec(
                (20u128..23, any::<bool>(), prop::option::of(0i32..=2000)),
                0..6,
            ),
            1..12,
        )
    }

    fn entrees(classe: &[Vec<(u128, bool, Option<i32>)>]) -> Vec<NotesEleve> {
        classe
            .iter()
            .enumerate()
            .map(|(i, notes)| NotesEleve {
                inscription_id: id(u128::try_from(i).unwrap_or(0) + 1),
                notes: notes
                    .iter()
                    .map(|(m, devoir, v)| {
                        let t = if *devoir {
                            TypeEvaluation::Devoir
                        } else {
                            TypeEvaluation::Composition
                        };
                        (id(*m), note(t, *v, 2000))
                    })
                    .collect(),
                classable: true,
                conduite: None,
            })
            .collect()
    }

    proptest! {
        /// RC-5 : recalculer une période à partir des mêmes notes redonne exactement les mêmes
        /// résultats, et le résultat d'un élève ne dépend pas de sa place dans la liste.
        #[test]
        fn recalculer_redonne_les_memes_resultats(c in classe()) {
            let regle = ContenuRegle::par_defaut(Cycle::SecondCycle).unwrap_or_else(|| unreachable!());
            let programme = [(id(MATH), 4), (id(FR), 3), (id(ANG), 2)];
            let eleves = entrees(&c);
            let une = calculer_classe(&regle, 2000, &programme, &eleves, None).unwrap_or_else(|_| unreachable!());
            let deux = calculer_classe(&regle, 2000, &programme, &eleves, None).unwrap_or_else(|_| unreachable!());
            prop_assert_eq!(&une, &deux);
            let mut inverses = eleves.clone();
            inverses.reverse();
            let retournee = calculer_classe(&regle, 2000, &programme, &inverses, None).unwrap_or_else(|_| unreachable!());
            for r in &une.eleves {
                let meme = retournee.eleves.iter().find(|x| x.inscription_id == r.inscription_id);
                prop_assert_eq!(Some(r), meme);
            }
            prop_assert_eq!(une.resume, retournee.resume);
            // Toute moyenne reste dans le barème.
            for r in &une.eleves {
                prop_assert!(r.moyenne.is_none_or(|m| (0..=2000).contains(&m)));
            }
        }
    }
}
