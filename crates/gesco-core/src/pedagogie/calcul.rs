//! Moteur de calcul des moyennes (SPEC_PHASE_4 §5, §8.4bis du blueprint). Il reproduit le calcul
//! que l'enseignant fait au crayon : fractions exactes en centièmes (RC-6), aucun flottant, aucune
//! valeur intermédiaire arrondie (RC-7) sauf la moyenne de matière avant la moyenne générale
//! quand l'école le veut (Q-20). L'affichage au centième suit l'arrondi de l'école : troncature
//! par défaut, au plus proche ou par excès (réponse du 11 octobre 2026, qui assouplit RC-1).

use serde::{Deserialize, Serialize};

use crate::pedagogie::regle::{ArrondiMoyenne, FormuleMatiere, TraitementAbsence, TypeEvaluation};

/// Le calcul déborde : des barèmes ou des coefficients hors de toute mesure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Depassement;

/// Fraction exacte, en centièmes de point : `num / den` centièmes, `den > 0`, réduite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fraction {
    num: i128,
    den: i128,
}

const fn pgcd(mut a: i128, mut b: i128) -> i128 {
    a = a.abs();
    b = b.abs();
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    if a == 0 { 1 } else { a }
}

impl Fraction {
    /// `num / den` centièmes, réduite ; refuse un dénominateur nul ou négatif.
    pub fn nouvelle(num: i128, den: i128) -> Result<Self, Depassement> {
        if den <= 0 {
            return Err(Depassement);
        }
        let g = pgcd(num, den);
        Ok(Self {
            num: num / g,
            den: den / g,
        })
    }

    /// Un nombre entier de centièmes.
    pub const fn entiere(centiemes: i32) -> Self {
        Self {
            num: centiemes as i128,
            den: 1,
        }
    }

    fn plus(self, autre: Self) -> Result<Self, Depassement> {
        let num = self
            .num
            .checked_mul(autre.den)
            .and_then(|a| {
                autre
                    .num
                    .checked_mul(self.den)
                    .and_then(|b| a.checked_add(b))
            })
            .ok_or(Depassement)?;
        let den = self.den.checked_mul(autre.den).ok_or(Depassement)?;
        Self::nouvelle(num, den)
    }

    fn fois(self, entier: i128) -> Result<Self, Depassement> {
        Self::nouvelle(self.num.checked_mul(entier).ok_or(Depassement)?, self.den)
    }

    fn divise(self, entier: i128) -> Result<Self, Depassement> {
        Self::nouvelle(self.num, self.den.checked_mul(entier).ok_or(Depassement)?)
    }

    /// Valeur tronquée au centième (RC-1) : on coupe, on ne regarde pas le chiffre suivant.
    pub fn tronquee(self) -> i32 {
        i32::try_from(self.num.div_euclid(self.den)).unwrap_or(i32::MAX)
    }

    /// Valeur au centième selon l'arrondi de l'école, en arithmétique entière.
    pub fn arrondie(self, arrondi: ArrondiMoyenne) -> i32 {
        let bas = self.num.div_euclid(self.den);
        let reste = self.num.rem_euclid(self.den);
        let haut = match arrondi {
            ArrondiMoyenne::Troncature => false,
            // La moitié monte : 2 × reste ≥ dénominateur.
            ArrondiMoyenne::AuPlusProche => reste.saturating_mul(2) >= self.den,
            ArrondiMoyenne::ParExces => reste > 0,
        };
        i32::try_from(if haut { bas + 1 } else { bas }).unwrap_or(i32::MAX)
    }
}

/// Écrit des centièmes avec deux décimales : `1335` → « 13,35 ».
pub fn ecrire(centiemes: i32) -> String {
    let signe = if centiemes < 0 { "-" } else { "" };
    let v = centiemes.unsigned_abs();
    format!("{signe}{},{:02}", v / 100, v % 100)
}

/// Note d'un élève à une évaluation, telle que saisie (RC-2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct NoteCalcul {
    /// Type de l'évaluation.
    pub type_evaluation: TypeEvaluation,
    /// Barème de l'évaluation, en centièmes.
    pub bareme: i32,
    /// Pondération de l'évaluation (1 par défaut).
    pub ponderation: i32,
    /// Note en centièmes ; absente pour un absent, un dispensé ou une note non saisie.
    pub valeur: Option<i32>,
    /// Absent.
    pub absent: bool,
    /// Absence justifiée.
    pub justifie: bool,
    /// Dispensé : jamais compté.
    pub dispense: bool,
}

/// Note retenue pour le calcul : (note, barème, pondération), ou rien.
fn retenue(n: &NoteCalcul, absent: TraitementAbsence) -> Option<(i128, i128, i128)> {
    if n.dispense || n.bareme <= 0 || n.ponderation <= 0 {
        return None;
    }
    match n.valeur {
        Some(v) if !n.absent => Some((
            i128::from(v),
            i128::from(n.bareme),
            i128::from(n.ponderation),
        )),
        // RC-3 : l'absent n'est pas un zéro, sauf si l'école l'a voulu pour l'absence non justifiée.
        _ if n.absent && !n.justifie && absent == TraitementAbsence::Zero => {
            Some((0, i128::from(n.bareme), i128::from(n.ponderation)))
        }
        _ => None,
    }
}

/// Σ notes × pondérations ÷ Σ barèmes × pondérations, ramené au barème du niveau (§8.4bis,
/// niveau 1) ; rien si aucune note ne compte.
fn somme_des_baremes(
    notes: &[&NoteCalcul],
    absent: TraitementAbsence,
    bareme_niveau: i32,
) -> Result<Option<Fraction>, Depassement> {
    let mut points: i128 = 0;
    let mut baremes: i128 = 0;
    for (v, b, p) in notes.iter().filter_map(|n| retenue(n, absent)) {
        points = v
            .checked_mul(p)
            .and_then(|x| points.checked_add(x))
            .ok_or(Depassement)?;
        baremes = b
            .checked_mul(p)
            .and_then(|x| baremes.checked_add(x))
            .ok_or(Depassement)?;
    }
    if baremes == 0 {
        return Ok(None);
    }
    Ok(Some(Fraction::nouvelle(
        points
            .checked_mul(i128::from(bareme_niveau))
            .ok_or(Depassement)?,
        baremes,
    )?))
}

/// Moyenne d'une matière sur une période, en centièmes exacts du barème du niveau ; rien si
/// aucune note ne compte (« non noté », Q-59).
pub fn moyenne_matiere(
    notes: &[NoteCalcul],
    formule: FormuleMatiere,
    absent: TraitementAbsence,
    bareme_niveau: i32,
) -> Result<Option<Fraction>, Depassement> {
    let toutes: Vec<&NoteCalcul> = notes.iter().collect();
    match formule {
        FormuleMatiere::SommeDesBaremes => somme_des_baremes(&toutes, absent, bareme_niveau),
        FormuleMatiere::ClasseEtComposition => {
            let devoirs: Vec<&NoteCalcul> = notes
                .iter()
                .filter(|n| n.type_evaluation == TypeEvaluation::Devoir)
                .collect();
            let compositions: Vec<&NoteCalcul> = notes
                .iter()
                .filter(|n| n.type_evaluation == TypeEvaluation::Composition)
                .collect();
            let classe = somme_des_baremes(&devoirs, absent, bareme_niveau)?;
            let composition = somme_des_baremes(&compositions, absent, bareme_niveau)?;
            match (classe, composition) {
                (Some(c), Some(k)) => Ok(Some(c.plus(k.fois(2)?)?.divise(3)?)),
                (Some(c), None) => Ok(Some(c)),
                (None, Some(k)) => Ok(Some(k)),
                (None, None) => Ok(None),
            }
        }
    }
}

/// Note de classe d'une matière : la moyenne des devoirs, ramenée au barème du niveau.
pub fn note_de_classe(
    notes: &[NoteCalcul],
    absent: TraitementAbsence,
    bareme_niveau: i32,
) -> Result<Option<Fraction>, Depassement> {
    let devoirs: Vec<&NoteCalcul> = notes
        .iter()
        .filter(|n| n.type_evaluation == TypeEvaluation::Devoir)
        .collect();
    somme_des_baremes(&devoirs, absent, bareme_niveau)
}

/// Moyenne générale (§8.4bis, niveau 2) : Σ moyenne × coefficient ÷ Σ coefficients, sur les seules
/// matières notées. `report` (Q-20) reporte la moyenne de matière telle qu'imprimée, selon
/// l'arrondi de l'école ; `None` la reporte en pleine précision.
pub fn moyenne_generale(
    matieres: &[(Option<Fraction>, i64)],
    report: Option<ArrondiMoyenne>,
) -> Result<Option<Fraction>, Depassement> {
    let mut somme = Fraction::entiere(0);
    let mut coefficients: i128 = 0;
    for (moyenne, coefficient) in matieres {
        let Some(m) = moyenne else { continue };
        if *coefficient <= 0 {
            continue;
        }
        let reportee = match report {
            Some(arrondi) => Fraction::entiere(m.arrondie(arrondi)),
            None => *m,
        };
        somme = somme.plus(reportee.fois(i128::from(*coefficient))?)?;
        coefficients = coefficients
            .checked_add(i128::from(*coefficient))
            .ok_or(Depassement)?;
    }
    if coefficients == 0 {
        return Ok(None);
    }
    Ok(Some(somme.divise(coefficients)?))
}

/// Moyenne annuelle (§8.4bis, niveau 3 ; T-33) : moyennes générales des périodes suivies, telles
/// qu'imprimées, pondérées par le poids de chaque période ; une période non suivie (rien) sort du
/// calcul (Q-27).
pub fn moyenne_annuelle(periodes: &[(Option<i32>, i64)]) -> Result<Option<Fraction>, Depassement> {
    let mut somme: i128 = 0;
    let mut poids: i128 = 0;
    for (moyenne, ponderation) in periodes {
        let Some(m) = moyenne else { continue };
        if *ponderation <= 0 {
            continue;
        }
        somme = i128::from(*m)
            .checked_mul(i128::from(*ponderation))
            .and_then(|x| somme.checked_add(x))
            .ok_or(Depassement)?;
        poids = poids
            .checked_add(i128::from(*ponderation))
            .ok_or(Depassement)?;
    }
    if poids == 0 {
        return Ok(None);
    }
    Ok(Some(Fraction::nouvelle(somme, poids)?))
}

/// Rang d'un élève : « 3e », ou « 3e ex » à égalité ; le suivant saute (T-30).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Rang {
    /// Rang, à partir de 1.
    pub rang: u32,
    /// Vrai à égalité avec au moins un autre élève.
    pub ex_aequo: bool,
}

/// Résumé d'un classement : effectif classé, moyenne du premier et du dernier (réponse du
/// 11 octobre 2026).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Resume {
    /// Élèves classés : ceux qui ont une moyenne.
    pub classes: u32,
    /// Moyenne du premier, en centièmes.
    pub premier: Option<i32>,
    /// Moyenne du dernier, en centièmes.
    pub dernier: Option<i32>,
}

/// Classe des moyennes telles qu'imprimées (tronquées) : rang = 1 + nombre de moyennes
/// strictement supérieures ; à égalité, même rang et mention « ex » ; un élève sans moyenne n'est
/// pas classé.
pub fn classer(moyennes: &[Option<i32>]) -> (Vec<Option<Rang>>, Resume) {
    let classees: Vec<i32> = moyennes.iter().flatten().copied().collect();
    let rangs = moyennes
        .iter()
        .map(|m| {
            m.map(|v| Rang {
                rang: u32::try_from(classees.iter().filter(|x| **x > v).count() + 1)
                    .unwrap_or(u32::MAX),
                ex_aequo: classees.iter().filter(|x| **x == v).count() > 1,
            })
        })
        .collect();
    let resume = Resume {
        classes: u32::try_from(classees.len()).unwrap_or(u32::MAX),
        premier: classees.iter().max().copied(),
        dernier: classees.iter().min().copied(),
    };
    (rangs, resume)
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    //! Cas de référence (SPEC_PHASE_4 §8, `docs/cas-de-reference-calcul.md`) : chaque résultat
    //! est celui du calcul à la main.
    use super::*;
    use proptest::prelude::*;

    const SUR_20: i32 = 2000;
    const SUR_10: i32 = 1000;

    fn note(type_evaluation: TypeEvaluation, valeur: i32, bareme: i32) -> NoteCalcul {
        NoteCalcul {
            type_evaluation,
            bareme,
            ponderation: 1,
            valeur: Some(valeur),
            absent: false,
            justifie: false,
            dispense: false,
        }
    }

    fn devoir(v: i32) -> NoteCalcul {
        note(TypeEvaluation::Devoir, v, SUR_20)
    }

    fn compo(v: i32, bareme: i32) -> NoteCalcul {
        note(TypeEvaluation::Composition, v, bareme)
    }

    fn absent(justifie: bool) -> NoteCalcul {
        NoteCalcul {
            valeur: None,
            absent: true,
            justifie,
            ..devoir(0)
        }
    }

    fn classe_compo(notes: &[NoteCalcul], abs: TraitementAbsence) -> Option<i32> {
        moyenne_matiere(notes, FormuleMatiere::ClasseEtComposition, abs, SUR_20)
            .ok()
            .flatten()
            .map(Fraction::tronquee)
    }

    #[test]
    fn cas_1_premier_cycle_cinq_domaines_sans_coefficient() {
        // LC 7,5 ; SMT 6 ; SH 8 ; Arts 9 ; DP 7, compositions sur 10 : 37,5 ÷ 5 = 7,50.
        let domaines: Vec<(Option<Fraction>, i64)> = [750, 600, 800, 900, 700]
            .iter()
            .map(|v| {
                let m = moyenne_matiere(
                    &[compo(*v, SUR_10)],
                    FormuleMatiere::ClasseEtComposition,
                    TraitementAbsence::Exclu,
                    SUR_10,
                )
                .ok()
                .flatten();
                (m, 1)
            })
            .collect();
        let m = moyenne_generale(&domaines, Some(ArrondiMoyenne::Troncature))
            .ok()
            .flatten()
            .map(Fraction::tronquee);
        assert_eq!(m, Some(750));
        assert_eq!(ecrire(750), "7,50");
    }

    #[test]
    fn cas_2_deux_devoirs_et_une_composition() {
        // Devoirs 12 et 15 : note de classe 13,50 ; composition 11 sur 20 :
        // (13,50 + 2 × 11) ÷ 3 = 35,50 ÷ 3 = 11,8333… → 11,83.
        let notes = [devoir(1200), devoir(1500), compo(1100, SUR_20)];
        assert_eq!(
            note_de_classe(&notes, TraitementAbsence::Exclu, SUR_20)
                .ok()
                .flatten()
                .map(Fraction::tronquee),
            Some(1350)
        );
        assert_eq!(classe_compo(&notes, TraitementAbsence::Exclu), Some(1183));
        // La même matière par la somme des barèmes, composition sur 40 (22 sur 40) :
        // (12 + 15 + 22) ÷ (80 ÷ 20) = 49 ÷ 4 = 12,25. Les deux formules divergent.
        let somme = [devoir(1200), devoir(1500), compo(2200, 4000)];
        let m = moyenne_matiere(
            &somme,
            FormuleMatiere::SommeDesBaremes,
            TraitementAbsence::Exclu,
            SUR_20,
        )
        .ok()
        .flatten()
        .map(Fraction::tronquee);
        assert_eq!(m, Some(1225));
    }

    #[test]
    fn cas_3_moyenne_generale_avec_coefficients() {
        // Maths 11,83 (coef 4), français 14 (4), anglais 9,50 (2), physique-chimie 10,25 (3) :
        // (47,32 + 56 + 19 + 30,75) ÷ 13 = 153,07 ÷ 13 = 11,7746… → 11,77.
        let maths = moyenne_matiere(
            &[devoir(1200), devoir(1500), compo(1100, SUR_20)],
            FormuleMatiere::ClasseEtComposition,
            TraitementAbsence::Exclu,
            SUR_20,
        )
        .ok()
        .flatten();
        let f = |v: i32| Some(Fraction::entiere(v));
        let matieres = [(maths, 4), (f(1400), 4), (f(950), 2), (f(1025), 3)];
        let tronquee = moyenne_generale(&matieres, Some(ArrondiMoyenne::Troncature))
            .ok()
            .flatten()
            .map(Fraction::tronquee);
        assert_eq!(tronquee, Some(1177));
        // Sans troncature avant report, les maths comptent 11,8333… : 153,0733… ÷ 13 = 11,7748…
        let exacte = moyenne_generale(&matieres, None)
            .ok()
            .flatten()
            .map(Fraction::tronquee);
        assert_eq!(exacte, Some(1177));
    }

    #[test]
    fn cas_4_absent_non_justifie_exclu_ou_zero() {
        // Devoir 1 : 12 ; devoir 2 : absent ; composition 11.
        let notes = [devoir(1200), absent(false), compo(1100, SUR_20)];
        // Exclu : note de classe 12 → (12 + 22) ÷ 3 = 11,333… → 11,33.
        assert_eq!(classe_compo(&notes, TraitementAbsence::Exclu), Some(1133));
        // Zéro : note de classe (12 + 0) ÷ 2 = 6 → (6 + 22) ÷ 3 = 9,333… → 9,33.
        assert_eq!(classe_compo(&notes, TraitementAbsence::Zero), Some(933));
        // Une absence justifiée n'est jamais un zéro.
        let justifie = [devoir(1200), absent(true), compo(1100, SUR_20)];
        assert_eq!(classe_compo(&justifie, TraitementAbsence::Zero), Some(1133));
    }

    #[test]
    fn cas_5_dispense_et_matiere_non_notee() {
        // Dispensé d'EPS : la matière n'est pas notée et sort de la moyenne générale, coefficient
        // compris : (14 × 4 + 10 × 2) ÷ 6 = 76 ÷ 6 = 12,666… → 12,66.
        let eps = moyenne_matiere(
            &[NoteCalcul {
                dispense: true,
                valeur: None,
                ..compo(0, SUR_20)
            }],
            FormuleMatiere::ClasseEtComposition,
            TraitementAbsence::Zero,
            SUR_20,
        );
        assert_eq!(eps, Ok(None));
        let matieres = [
            (Some(Fraction::entiere(1400)), 4),
            (Some(Fraction::entiere(1000)), 2),
            (None, 1),
        ];
        let m = moyenne_generale(&matieres, Some(ArrondiMoyenne::Troncature))
            .ok()
            .flatten()
            .map(Fraction::tronquee);
        assert_eq!(m, Some(1266));
    }

    #[test]
    fn cas_6_ex_aequo_le_suivant_saute() {
        let (rangs, resume) = classer(&[Some(1425), Some(1250), Some(1250), Some(1100), None]);
        let r = |rang, ex_aequo| Some(Rang { rang, ex_aequo });
        assert_eq!(
            rangs,
            vec![r(1, false), r(2, true), r(2, true), r(4, false), None]
        );
        assert_eq!(
            resume,
            Resume {
                classes: 4,
                premier: Some(1425),
                dernier: Some(1100)
            }
        );
    }

    #[test]
    fn cas_7_troncature_jamais_d_arrondi() {
        // Devoirs 13,25 et 13,50 : note de classe 13,375 ; composition 13,35 :
        // (13,375 + 26,70) ÷ 3 = 40,075 ÷ 3 = 13,358333… → 13,35 (un arrondi aurait donné 13,36).
        let notes = [devoir(1325), devoir(1350), compo(1335, SUR_20)];
        assert_eq!(classe_compo(&notes, TraitementAbsence::Exclu), Some(1335));
        assert_eq!(ecrire(1335), "13,35");
    }

    #[test]
    fn cas_9_chaque_ecole_choisit_son_arrondi() {
        // 13,3583… (cas 7) et 11,8333… (cas 2) selon les trois arrondis.
        let cas7 = moyenne_matiere(
            &[devoir(1325), devoir(1350), compo(1335, SUR_20)],
            FormuleMatiere::ClasseEtComposition,
            TraitementAbsence::Exclu,
            SUR_20,
        )
        .ok()
        .flatten()
        .unwrap_or_else(|| unreachable!());
        let cas2 = moyenne_matiere(
            &[devoir(1200), devoir(1500), compo(1100, SUR_20)],
            FormuleMatiere::ClasseEtComposition,
            TraitementAbsence::Exclu,
            SUR_20,
        )
        .ok()
        .flatten()
        .unwrap_or_else(|| unreachable!());
        assert_eq!(cas7.arrondie(ArrondiMoyenne::Troncature), 1335);
        assert_eq!(cas7.arrondie(ArrondiMoyenne::AuPlusProche), 1336);
        assert_eq!(cas7.arrondie(ArrondiMoyenne::ParExces), 1336);
        assert_eq!(cas2.arrondie(ArrondiMoyenne::Troncature), 1183);
        assert_eq!(cas2.arrondie(ArrondiMoyenne::AuPlusProche), 1183);
        assert_eq!(cas2.arrondie(ArrondiMoyenne::ParExces), 1184);
        // La moitié monte ; une valeur exacte ne bouge jamais.
        let moitie = Fraction::nouvelle(26_711, 2).unwrap_or_else(|_| unreachable!());
        assert_eq!(moitie.arrondie(ArrondiMoyenne::AuPlusProche), 13_356);
        assert_eq!(
            Fraction::entiere(1250).arrondie(ArrondiMoyenne::ParExces),
            1250
        );
    }

    #[test]
    fn cas_8_moyenne_annuelle_des_trimestres_suivis() {
        // Trois trimestres : (11,77 + 12,40 + 13,10) ÷ 3 = 37,27 ÷ 3 = 12,4233… → 12,42.
        let annee = moyenne_annuelle(&[(Some(1177), 1), (Some(1240), 1), (Some(1310), 1)]);
        assert_eq!(annee.ok().flatten().map(Fraction::tronquee), Some(1242));
        // Arrivé au 2e trimestre : (12,40 + 13,10) ÷ 2 = 12,75 (Q-27).
        let arrive = moyenne_annuelle(&[(None, 1), (Some(1240), 1), (Some(1310), 1)]);
        assert_eq!(arrive.ok().flatten().map(Fraction::tronquee), Some(1275));
        // Deux trimestres seulement : la même formule.
        let deux = moyenne_annuelle(&[(Some(1000), 1), (Some(1101), 1)]);
        assert_eq!(deux.ok().flatten().map(Fraction::tronquee), Some(1050));
    }

    proptest! {
        #[test]
        fn une_moyenne_reste_dans_le_bareme(
            notes in prop::collection::vec((0i32..=2000, any::<bool>(), any::<bool>()), 0..8),
            compo_v in proptest::option::of(0i32..=2000),
            formule in prop::sample::select(vec![FormuleMatiere::ClasseEtComposition, FormuleMatiere::SommeDesBaremes]),
        ) {
            let mut liste: Vec<NoteCalcul> = notes
                .iter()
                .map(|(v, abs, just)| NoteCalcul { valeur: if *abs { None } else { Some(*v) }, absent: *abs, justifie: *just, ..devoir(0) })
                .collect();
            if let Some(c) = compo_v {
                liste.push(compo(c, SUR_20));
            }
            for abs in [TraitementAbsence::Exclu, TraitementAbsence::Zero] {
                if let Ok(Some(m)) = moyenne_matiere(&liste, formule, abs, SUR_20) {
                    let t = m.tronquee();
                    prop_assert!((0..=SUR_20).contains(&t));
                }
            }
        }

        #[test]
        fn un_devoir_et_une_composition_sur_quarante_les_deux_formules_concordent(
            d in 0i32..=2000,
            c in 0i32..=2000,
        ) {
            // §8.4bis : un devoir sur 20 et une composition sur 40 (2 × c) donnent (d + 2c) ÷ 3
            // par les deux formules.
            let classe = moyenne_matiere(&[devoir(d), compo(c, SUR_20)], FormuleMatiere::ClasseEtComposition, TraitementAbsence::Exclu, SUR_20);
            let somme = moyenne_matiere(&[devoir(d), compo(2 * c, 4000)], FormuleMatiere::SommeDesBaremes, TraitementAbsence::Exclu, SUR_20);
            prop_assert_eq!(classe, somme);
        }

        #[test]
        fn les_arrondis_s_ordonnent_et_restent_a_un_centieme(num in 0i128..400_000, den in 1i128..1000) {
            let f = Fraction::nouvelle(num, den).unwrap_or_else(|_| unreachable!());
            let t = f.arrondie(ArrondiMoyenne::Troncature);
            let p = f.arrondie(ArrondiMoyenne::AuPlusProche);
            let e = f.arrondie(ArrondiMoyenne::ParExces);
            prop_assert!(t <= p && p <= e && e - t <= 1);
            prop_assert_eq!(t, f.tronquee());
        }

        #[test]
        fn le_rang_suit_les_moyennes(moyennes in prop::collection::vec(proptest::option::of(0i32..=2000), 0..40)) {
            let (rangs, resume) = classer(&moyennes);
            prop_assert_eq!(resume.classes as usize, moyennes.iter().flatten().count());
            for (i, a) in moyennes.iter().enumerate() {
                for (j, b) in moyennes.iter().enumerate() {
                    if let (Some(a), Some(b), Some(ra), Some(rb)) = (a, b, rangs[i], rangs[j]) {
                        prop_assert_eq!(a > b, ra.rang < rb.rang);
                        prop_assert_eq!(a == b, ra.rang == rb.rang);
                    }
                }
                prop_assert_eq!(a.is_none(), rangs[i].is_none());
            }
        }
    }
}
