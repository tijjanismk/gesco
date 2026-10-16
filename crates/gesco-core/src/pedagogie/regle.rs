//! Règles d'évaluation d'une année et d'un cycle (SPEC_PHASE_4 §2.4, §8.2 du blueprint),
//! versionnées : un bulletin se recalcule avec la version qui l'a produit (RC-4). Aucune valeur
//! n'est écrite en dur ailleurs que dans les défauts ci-dessous, que l'école modifie.

use serde::{Deserialize, Serialize};

use crate::commun::{Horodatage, Id};
use crate::core::{AnneeScolaire, Etablissement};
use crate::scolarite::Cycle;

/// Type d'évaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum TypeEvaluation {
    /// Devoir, interrogation : la moyenne des devoirs fait la note de classe (T-28).
    Devoir,
    /// Composition de fin de période (mensuelle au 1er cycle).
    Composition,
}

impl TypeEvaluation {
    /// Les deux types.
    pub const TOUS: [Self; 2] = [Self::Devoir, Self::Composition];

    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Devoir => "devoir",
            Self::Composition => "composition",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::TOUS.into_iter().find(|t| t.nom() == nom)
    }
}

/// Type d'évaluation permis dans le cycle, avec son barème usuel en centièmes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct TypeAutorise {
    /// Type.
    pub type_evaluation: TypeEvaluation,
    /// Barème proposé à la création d'une évaluation, en centièmes (`2000` pour 20).
    pub bareme: i32,
}

/// Formule de la moyenne d'une matière sur une période (T-28).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum FormuleMatiere {
    /// Note de classe (moyenne des devoirs), puis (classe + 2 × composition) ÷ 3 ; sans devoir la
    /// composition seule, sans composition la note de classe seule. Défaut.
    ClasseEtComposition,
    /// Somme des notes sur somme des barèmes, ramenée au barème du niveau (§8.4bis).
    SommeDesBaremes,
}

/// Traitement d'un absent non justifié (T-29, RC-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum TraitementAbsence {
    /// Exclu du calcul, numérateur et dénominateur. Défaut.
    Exclu,
    /// Compté zéro.
    Zero,
}

/// Arrondi des moyennes au centième, au choix de chaque école (réponse du 11 octobre 2026).
/// Le blueprint (RC-1) tronquait toujours ; la troncature reste le défaut.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, specta::Type,
)]
#[serde(rename_all = "snake_case")]
pub enum ArrondiMoyenne {
    /// On coupe : 13,358 donne 13,35. Défaut.
    #[default]
    Troncature,
    /// Au plus proche, la moitié vers le haut : 13,355 donne 13,36, 13,354 donne 13,35.
    AuPlusProche,
    /// Par excès : 13,351 donne 13,36.
    ParExces,
}

/// Où la moyenne annuelle s'imprime ; elle est toujours calculée et enregistrée (T-33).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ImpressionMoyenneAnnuelle {
    /// Dans les seules classes d'examen (DEF, BAC, CAP, BT). Défaut.
    ClassesExamen,
    /// Dans toutes les classes.
    Toutes,
}

/// Mention attribuée à partir d'une moyenne, sur le barème du niveau (T-31).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Mention {
    /// Moyenne minimale, en centièmes du barème du niveau.
    pub seuil: i32,
    /// Libellé imprimé.
    pub libelle: String,
}

/// Contenu d'une règle d'évaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ContenuRegle {
    /// Types d'évaluation permis et barèmes usuels.
    pub types: Vec<TypeAutorise>,
    /// Formule de la moyenne de matière.
    pub formule: FormuleMatiere,
    /// Traitement de l'absent non justifié.
    pub absent_non_justifie: TraitementAbsence,
    /// Reporter la moyenne de matière telle qu'imprimée (deux décimales, selon `arrondi`) dans
    /// la moyenne générale (Q-20) ; sinon elle y entre en pleine précision.
    pub tronquer_avant_report: bool,
    /// Arrondi des moyennes au centième.
    #[serde(default)]
    pub arrondi: ArrondiMoyenne,
    /// Mentions, de la plus haute à la plus basse.
    pub mentions: Vec<Mention>,
    /// Impression de la moyenne annuelle.
    pub moyenne_annuelle: ImpressionMoyenneAnnuelle,
    /// Classer l'élève arrivé en cours d'année dès sa première période (Q-29).
    pub classer_arrivee_en_cours: bool,
}

/// Règle d'une année et d'un cycle, dans une version donnée.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct RegleEvaluation {
    /// Identifiant technique.
    pub id: Id<RegleEvaluation>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Année.
    pub annee_id: Id<AnneeScolaire>,
    /// Cycle.
    pub cycle: Cycle,
    /// Version, à partir de 1 ; chaque modification en crée une nouvelle.
    pub version: i64,
    /// Contenu.
    pub contenu: ContenuRegle,
    /// Instant de la version.
    pub le: Horodatage,
}

/// Barème du bulletin d'un cycle, en centièmes : 10 au 1er cycle, 20 ensuite (§8.4bis), y compris
/// dans l'enseignement technique (SPEC_PHASE_6 §10) ; aucun au jardin.
pub const fn bareme_du_cycle(cycle: Cycle) -> Option<i32> {
    match cycle {
        Cycle::Jardin => None,
        Cycle::PremierCycle => Some(1000),
        Cycle::SecondCycle | Cycle::Lycee | Cycle::Technique => Some(2000),
    }
}

/// Mentions fixes, en pour cent du barème : Excellent à 18 sur 20, Très bien à 16, Bien à 14,
/// Assez bien à 12, Passable à 10. Elles ne changent pas d'une école à l'autre (réponse du
/// 11 octobre 2026) ; seules les mentions sous Passable se règlent.
const MENTIONS_FIXES: [(i32, &str); 5] = [
    (90, "Excellent"),
    (80, "Très bien"),
    (70, "Bien"),
    (60, "Assez bien"),
    (50, "Passable"),
];

fn mention(bareme: i32, pour_cent: i32, libelle: &str) -> Mention {
    Mention {
        seuil: bareme / 100 * pour_cent,
        libelle: libelle.to_owned(),
    }
}

/// Mentions fixes d'un barème, de la plus haute à Passable.
pub fn mentions_fixes(bareme: i32) -> Vec<Mention> {
    MENTIONS_FIXES
        .iter()
        .map(|(p, l)| mention(bareme, *p, l))
        .collect()
}

/// Échelle livrée (T-31) : les mentions fixes, puis Médiocre à 8 sur 20 et Insuffisant en
/// dessous, que l'école change (Insuffisant, Nul…) selon son usage.
fn mentions_par_defaut(bareme: i32) -> Vec<Mention> {
    let mut m = mentions_fixes(bareme);
    m.push(mention(bareme, 40, "Médiocre"));
    m.push(mention(bareme, 0, "Insuffisant"));
    m
}

impl ContenuRegle {
    /// Règle livrée pour un cycle ; aucune au jardin.
    pub fn par_defaut(cycle: Cycle) -> Option<Self> {
        let bareme = bareme_du_cycle(cycle)?;
        let types = if cycle == Cycle::PremierCycle {
            vec![TypeAutorise {
                type_evaluation: TypeEvaluation::Composition,
                bareme: 1000,
            }]
        } else {
            vec![
                TypeAutorise {
                    type_evaluation: TypeEvaluation::Devoir,
                    bareme: 2000,
                },
                TypeAutorise {
                    type_evaluation: TypeEvaluation::Composition,
                    bareme: 2000,
                },
            ]
        };
        Some(Self {
            types,
            formule: FormuleMatiere::ClasseEtComposition,
            absent_non_justifie: TraitementAbsence::Exclu,
            tronquer_avant_report: true,
            arrondi: ArrondiMoyenne::Troncature,
            mentions: mentions_par_defaut(bareme),
            moyenne_annuelle: ImpressionMoyenneAnnuelle::ClassesExamen,
            classer_arrivee_en_cours: true,
        })
    }

    /// Barème usuel d'un type, s'il est permis.
    pub fn bareme_de(&self, type_evaluation: TypeEvaluation) -> Option<i32> {
        self.types
            .iter()
            .find(|t| t.type_evaluation == type_evaluation)
            .map(|t| t.bareme)
    }

    /// Mention d'une moyenne en centièmes : la première dont le seuil est atteint.
    pub fn mention(&self, moyenne: i32) -> Option<&str> {
        self.mentions
            .iter()
            .find(|m| moyenne >= m.seuil)
            .map(|m| m.libelle.as_str())
    }
}

/// Vérifie une règle saisie par l'école pour un cycle.
pub fn verifier_regle(cycle: Cycle, contenu: &ContenuRegle) -> Result<(), &'static str> {
    let bareme_niveau = bareme_du_cycle(cycle).ok_or("regle.cycle_sans_notes")?;
    if contenu.bareme_de(TypeEvaluation::Composition).is_none() {
        return Err("regle.composition_obligatoire");
    }
    for (i, t) in contenu.types.iter().enumerate() {
        if contenu
            .types
            .iter()
            .take(i)
            .any(|u| u.type_evaluation == t.type_evaluation)
        {
            return Err("regle.type_en_double");
        }
        if !(100..=10_000).contains(&t.bareme) {
            return Err("regle.bareme_invalide");
        }
    }
    if contenu.mentions.len() < MENTIONS_FIXES.len()
        || contenu
            .mentions
            .iter()
            .zip(mentions_fixes(bareme_niveau))
            .any(|(saisie, fixe)| *saisie != fixe)
    {
        return Err("regle.mentions_hautes_fixes");
    }
    let mut precedent: Option<i32> = None;
    for m in &contenu.mentions {
        if m.libelle.trim().is_empty() || m.libelle.chars().count() > 30 {
            return Err("regle.mention_libelle_invalide");
        }
        if !(0..=bareme_niveau).contains(&m.seuil) || precedent.is_some_and(|p| m.seuil >= p) {
            return Err("regle.mentions_non_decroissantes");
        }
        precedent = Some(m.seuil);
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn les_regles_livrees_sont_valides() {
        for cycle in [
            Cycle::PremierCycle,
            Cycle::SecondCycle,
            Cycle::Lycee,
            Cycle::Technique,
        ] {
            let r = ContenuRegle::par_defaut(cycle).unwrap_or_else(|| unreachable!());
            assert_eq!(verifier_regle(cycle, &r), Ok(()), "{cycle:?}");
        }
        assert!(ContenuRegle::par_defaut(Cycle::Jardin).is_none());
        // L'enseignement technique reprend la règle du lycée (SPEC_PHASE_6 §10).
        assert_eq!(
            ContenuRegle::par_defaut(Cycle::Technique),
            ContenuRegle::par_defaut(Cycle::Lycee)
        );
    }

    #[test]
    fn le_premier_cycle_compose_seul_sur_dix() {
        let r = ContenuRegle::par_defaut(Cycle::PremierCycle).unwrap_or_else(|| unreachable!());
        assert_eq!(r.bareme_de(TypeEvaluation::Composition), Some(1000));
        assert_eq!(r.bareme_de(TypeEvaluation::Devoir), None);
        assert_eq!(r.mention(900), Some("Excellent"));
        assert_eq!(r.mention(899), Some("Très bien"));
        assert_eq!(r.mention(450), Some("Médiocre"));
        assert_eq!(r.mention(150), Some("Insuffisant"));
    }

    #[test]
    fn mentions_sur_vingt() {
        let r = ContenuRegle::par_defaut(Cycle::Lycee).unwrap_or_else(|| unreachable!());
        let seuils: Vec<i32> = r.mentions.iter().map(|m| m.seuil).collect();
        assert_eq!(seuils, vec![1800, 1600, 1400, 1200, 1000, 800, 0]);
        assert_eq!(mentions_fixes(1000).last().map(|m| m.seuil), Some(500));
        assert_eq!(r.mention(1835), Some("Excellent"));
        assert_eq!(r.mention(1250), Some("Assez bien"));
    }

    #[test]
    fn une_regle_saisie_se_controle() {
        let base = ContenuRegle::par_defaut(Cycle::SecondCycle).unwrap_or_else(|| unreachable!());
        let mut r = base.clone();
        r.types
            .retain(|t| t.type_evaluation == TypeEvaluation::Devoir);
        assert_eq!(
            verifier_regle(Cycle::SecondCycle, &r),
            Err("regle.composition_obligatoire")
        );
        let mut r = base.clone();
        r.types.push(r.types[0]);
        assert_eq!(
            verifier_regle(Cycle::SecondCycle, &r),
            Err("regle.type_en_double")
        );
        let mut r = base.clone();
        r.types[0].bareme = 0;
        assert_eq!(
            verifier_regle(Cycle::SecondCycle, &r),
            Err("regle.bareme_invalide")
        );
        // Excellent à Passable ne changent pas.
        let mut r = base.clone();
        r.mentions.swap(0, 1);
        assert_eq!(
            verifier_regle(Cycle::SecondCycle, &r),
            Err("regle.mentions_hautes_fixes")
        );
        let mut r = base.clone();
        r.mentions[4].libelle = "Moyen".to_owned();
        assert_eq!(
            verifier_regle(Cycle::SecondCycle, &r),
            Err("regle.mentions_hautes_fixes")
        );
        // Sous Passable, l'école choisit : Insuffisant puis Nul, mais dans l'ordre.
        let mut r = base.clone();
        r.mentions[6] = Mention {
            seuil: 400,
            libelle: "Insuffisant".to_owned(),
        };
        r.mentions.push(Mention {
            seuil: 0,
            libelle: "Nul".to_owned(),
        });
        assert_eq!(verifier_regle(Cycle::SecondCycle, &r), Ok(()));
        let mut r = base;
        r.mentions[6].seuil = 1100;
        assert_eq!(
            verifier_regle(Cycle::SecondCycle, &r),
            Err("regle.mentions_non_decroissantes")
        );
        assert_eq!(
            verifier_regle(
                Cycle::Jardin,
                &ContenuRegle::par_defaut(Cycle::Lycee).unwrap_or_else(|| unreachable!())
            ),
            Err("regle.cycle_sans_notes")
        );
    }
}
