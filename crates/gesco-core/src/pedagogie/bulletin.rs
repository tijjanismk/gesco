//! Bulletin d'un élève pour une période (SPEC_PHASE_4 §7.3, §10.4 du blueprint) : son contenu est
//! figé au calcul, complété des appréciations, puis validé et publié. Publié, il est immuable ;
//! une correction en crée une nouvelle version, l'ancienne est marquée remplacée et reste
//! consultable. Le bulletin se publie et s'imprime quel que soit le solde de l'élève (T-36).

use serde::{Deserialize, Serialize};

use crate::commun::{DateCivile, Horodatage, Id};
use crate::core::{Etablissement, MachineAEtats};
use crate::pedagogie::assiduite::TotauxAbsences;
use crate::pedagogie::calcul::Rang;
use crate::pedagogie::{Matiere, Periode};
use crate::scolarite::{Classe, Cycle, Inscription};

/// État d'un bulletin. Le « brouillon » du blueprint n'est pas enregistré : un bulletin existe
/// dès son premier calcul.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum StatutBulletin {
    /// Calculé à partir des notes ; se recalcule tant qu'il n'est pas publié.
    Calcule,
    /// Validé par la direction.
    Valide,
    /// Publié : immuable.
    Publie,
    /// Remplacé par une version corrigée publiée ; reste consultable.
    Remplace,
}

impl MachineAEtats for StatutBulletin {
    const ETATS: &'static [Self] = &[Self::Calcule, Self::Valide, Self::Publie, Self::Remplace];
    const TRANSITIONS: &'static [(Self, Self)] = &[
        (Self::Calcule, Self::Valide),
        // Une note corrigée après validation : le bulletin est recalculé et se revalide.
        (Self::Valide, Self::Calcule),
        (Self::Valide, Self::Publie),
        (Self::Publie, Self::Remplace),
    ];

    fn nom(&self) -> &'static str {
        match self {
            Self::Calcule => "calcule",
            Self::Valide => "valide",
            Self::Publie => "publie",
            Self::Remplace => "remplace",
        }
    }
}

impl StatutBulletin {
    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::ETATS.iter().copied().find(|s| s.nom() == nom)
    }

    /// Vrai si le contenu ne change plus.
    pub const fn fige(self) -> bool {
        matches!(self, Self::Publie | Self::Remplace)
    }
}

/// Ligne d'une matière, telle qu'imprimée. Valeurs en centièmes du barème du niveau.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct LigneBulletin {
    /// Matière.
    pub matiere_id: Id<Matiere>,
    /// Libellé de la matière.
    pub matiere: String,
    /// Enseignant de la matière dans la classe.
    pub enseignant: String,
    /// Note de classe (moyenne des devoirs).
    pub note_de_classe: Option<i32>,
    /// Composition.
    pub composition: Option<i32>,
    /// Moyenne de la période.
    pub moyenne: Option<i32>,
    /// Coefficient.
    pub coefficient: i64,
    /// Moyenne × coefficient.
    pub points: Option<i64>,
    /// Moyenne de la classe dans la matière.
    pub moyenne_classe: Option<i32>,
    /// Appréciation de l'enseignant.
    pub appreciation: String,
}

/// Note de conduite imprimée : une ligne du tableau quand elle compte dans la moyenne générale,
/// une ligne du bilan sinon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ConduiteBulletin {
    /// Note, en centièmes du barème du niveau.
    pub note: i32,
    /// Coefficient.
    pub coefficient: i64,
    /// Note × coefficient, quand elle compte dans la moyenne générale ; rien sinon.
    pub points: Option<i64>,
}

/// Contenu figé d'un bulletin : tout ce qui s'imprime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ContenuBulletin {
    /// Prénoms et nom de l'élève.
    pub eleve: String,
    /// Matricule.
    pub matricule: String,
    /// Classe.
    pub classe: String,
    /// Niveau.
    pub niveau: String,
    /// Cycle : décide du modèle imprimé.
    pub cycle: Cycle,
    /// Année scolaire.
    pub annee: String,
    /// « Octobre », « 1er trimestre ».
    pub periode: String,
    /// Barème du niveau, en centièmes (10 ou 20).
    pub bareme: i32,
    /// Matières du programme.
    pub lignes: Vec<LigneBulletin>,
    /// Total des points.
    pub total_points: i64,
    /// Somme des coefficients des matières notées.
    pub total_coefficients: i64,
    /// Moyenne générale.
    pub moyenne: Option<i32>,
    /// Rang.
    pub rang: Option<Rang>,
    /// Mention.
    pub mention: Option<String>,
    /// Élèves de la classe.
    pub effectif: u32,
    /// Élèves qui ont composé : ceux qui ont une moyenne (réponse du 11 octobre 2026).
    pub ont_compose: u32,
    /// Moyenne du premier.
    pub premier: Option<i32>,
    /// Moyenne du dernier.
    pub dernier: Option<i32>,
    /// Moyenne de la classe.
    pub moyenne_classe: Option<i32>,
    /// Absences et retards de la période (T-32).
    pub absences: TotauxAbsences,
    /// Appréciation générale de la direction.
    pub appreciation_generale: String,
    /// Moyenne annuelle, calculée et enregistrée à la dernière période de l'année pour toutes
    /// les classes (T-33).
    pub moyenne_annuelle: Option<i32>,
    /// Rang sur la moyenne annuelle.
    #[serde(default)]
    pub rang_annuel: Option<Rang>,
    /// Vrai si la moyenne annuelle s'imprime : classes d'examen (9e, 12e) par défaut, ou toutes
    /// selon la règle de l'école (T-33).
    #[serde(default)]
    pub moyenne_annuelle_imprimee: bool,
    /// Date d'arrivée d'un élève arrivé pendant la période (Q-29).
    #[serde(default)]
    pub arrive_le: Option<DateCivile>,
    /// Avertissement et blâme du conseil de classe de la période (T-70) :
    /// « Blâme : bavardages répétés ».
    #[serde(default)]
    pub sanctions_conseil: Vec<String>,
    /// Note de conduite de la période (2e cycle, lycée, technique), quand l'école l'a activée.
    #[serde(default)]
    pub conduite: Option<ConduiteBulletin>,
}

/// Bulletin d'un élève, dans une version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Bulletin {
    /// Identifiant technique, encodé dans le QR de vérification.
    pub id: Id<Bulletin>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Inscription de l'élève.
    pub inscription_id: Id<Inscription>,
    /// Période.
    pub periode_id: Id<Periode>,
    /// Classe.
    pub classe_id: Id<Classe>,
    /// Version, à partir de 1 ; chaque correction en crée une nouvelle.
    pub version: i64,
    /// État.
    pub statut: StatutBulletin,
    /// Contenu.
    pub contenu: ContenuBulletin,
    /// Version de la règle d'évaluation qui l'a calculé (RC-4).
    pub regle_version: i64,
    /// Motif de la correction qui a créé cette version.
    pub motif_correction: String,
    /// Dernier calcul.
    pub calcule_le: Horodatage,
    /// Publication.
    pub publie_le: Option<Horodatage>,
}

/// Ce que le bulletin imprimé affiche, au choix de l'école et pour chaque cycle (réponses des 12
/// et 14 octobre 2026) : une case par colonne du tableau des matières et par ligne du bilan. La
/// matière et sa moyenne s'impriment toujours. Le réglage ne touche qu'à l'impression, jamais aux
/// notes ni aux moyennes ; il s'applique à chaque impression, réimpression comprise. Chaque
/// entrée : (réglage, affiché par défaut, propre aux bulletins trimestriels).
pub const REGLAGES_IMPRESSION: [(&str, bool, bool); 14] = [
    ("colonne.note_de_classe", true, true),
    ("colonne.composition", true, true),
    ("colonne.coefficient", true, true),
    ("colonne.points", true, true),
    ("colonne.moyenne_classe", false, false),
    ("colonne.appreciation", true, false),
    ("afficher.moyenne_generale", true, false),
    ("afficher.ont_compose", true, false),
    ("afficher.rang", true, false),
    ("afficher.mention", true, false),
    ("afficher.premier_dernier", true, false),
    ("afficher.moyenne_classe", true, false),
    ("afficher.absences", true, false),
    ("afficher.appreciation_generale", true, false),
];

/// Cycles qui ont un bulletin de notes, chacun avec ses réglages d'impression.
pub const CYCLES_A_BULLETIN: [Cycle; 4] = [
    Cycle::PremierCycle,
    Cycle::SecondCycle,
    Cycle::Lycee,
    Cycle::Technique,
];

/// Réglages d'impression d'un cycle : (paramètre, affiché par défaut). Le 1er cycle, mensuel sur
/// 10 sans coefficient, n'a pas les colonnes des bulletins trimestriels.
pub fn reglages_impression(cycle: Cycle) -> Vec<(String, bool)> {
    REGLAGES_IMPRESSION
        .iter()
        .filter(|(_, _, trimestriel)| !(cycle == Cycle::PremierCycle && *trimestriel))
        .map(|(suffixe, defaut, _)| (format!("bulletin.{}.{suffixe}", cycle.nom()), *defaut))
        .collect()
}

/// Moyenne d'une période reprise du relevé officiel d'une autre école, pour un élève arrivé
/// après elle (Q-28) : jamais les notes de détail, seulement la moyenne, avec sa source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct MoyenneReprise {
    /// Inscription de l'élève.
    pub inscription_id: Id<Inscription>,
    /// Période qu'il a suivie ailleurs.
    pub periode_id: Id<Periode>,
    /// Moyenne générale de la période, en centièmes du barème du niveau.
    pub moyenne: i32,
    /// École qui a délivré le relevé.
    pub etablissement_origine: String,
    /// Référence du relevé présenté.
    pub piece: String,
}

/// Vérifie une moyenne reprise : dans le barème du niveau, école d'origine indiquée.
pub fn verifier_reprise(r: &MoyenneReprise, bareme: i32) -> Result<(), &'static str> {
    if !(0..=bareme).contains(&r.moyenne) {
        return Err("moyenne_reprise.hors_bareme");
    }
    let origine = r.etablissement_origine.trim();
    if origine.is_empty() || origine.chars().count() > 120 || r.piece.chars().count() > 120 {
        return Err("moyenne_reprise.origine_obligatoire");
    }
    Ok(())
}

/// Vrai si l'élève arrive pendant cette période (Q-29) : inscrit après son début, hors de la
/// première période de l'année, où une inscription tardive n'est pas une arrivée.
pub fn arrive_pendant(inscrit_le: DateCivile, periode: &Periode) -> bool {
    periode.rang > 1 && inscrit_le > periode.debut && inscrit_le <= periode.fin
}

/// Vrai si l'élève n'a pas suivi cette période : il est arrivé après sa fin (Q-27).
pub fn non_suivie(inscrit_le: DateCivile, periode: &Periode) -> bool {
    inscrit_le > periode.fin
}

/// Rang imprimé : « 1er », « 2e », « 3e ex ».
pub fn texte_rang(rang: Rang) -> String {
    let base = if rang.rang == 1 {
        "1er".to_owned()
    } else {
        format!("{}e", rang.rang)
    };
    if rang.ex_aequo {
        format!("{base} ex")
    } else {
        base
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::transition::verifier_exhaustivement;

    #[test]
    fn etats_du_bulletin() {
        verifier_exhaustivement::<StatutBulletin>();
        assert!(StatutBulletin::Publie.fige());
        assert!(!StatutBulletin::Valide.fige());
        assert_eq!(
            StatutBulletin::depuis_nom("remplace"),
            Some(StatutBulletin::Remplace)
        );
    }

    #[test]
    fn le_rang_s_ecrit_comme_au_crayon() {
        let r = |rang, ex_aequo| texte_rang(Rang { rang, ex_aequo });
        assert_eq!(r(1, false), "1er");
        assert_eq!(r(1, true), "1er ex");
        assert_eq!(r(3, true), "3e ex");
        assert_eq!(r(5, false), "5e");
    }

    #[test]
    fn arrivee_et_periodes_non_suivies() {
        fn d(t: &str) -> DateCivile {
            DateCivile::analyser(t).unwrap_or_else(|| unreachable!())
        }
        fn id<T>(n: u128) -> Id<T> {
            Id::depuis_ulid(ulid::Ulid::from(n))
        }
        let periode = |rang: i64, debut: &str, fin: &str| Periode {
            id: id(u128::try_from(rang).unwrap_or(0)),
            etablissement_id: id(10),
            annee_id: id(11),
            niveau_id: id(12),
            rang,
            type_periode: crate::pedagogie::TypePeriode::Trimestre,
            debut: d(debut),
            fin: d(fin),
            ponderation: 1,
            verrouillee: false,
        };
        let t1 = periode(1, "2026-10-01", "2026-12-31");
        let t2 = periode(2, "2027-01-01", "2027-03-31");
        // Inscrit le 4 octobre : une inscription tardive, pas une arrivée.
        assert!(!arrive_pendant(d("2026-10-04"), &t1));
        assert!(!non_suivie(d("2026-10-04"), &t1));
        // Arrivé le 15 janvier : le 1er trimestre n'est pas suivi, il arrive pendant le 2e.
        assert!(non_suivie(d("2027-01-15"), &t1));
        assert!(arrive_pendant(d("2027-01-15"), &t2));
        let reprise = |moyenne, origine: &str| MoyenneReprise {
            inscription_id: id(1),
            periode_id: id(2),
            moyenne,
            etablissement_origine: origine.to_owned(),
            piece: String::new(),
        };
        assert_eq!(
            verifier_reprise(&reprise(1250, "École Mamadou Konaté"), 2000),
            Ok(())
        );
        assert_eq!(
            verifier_reprise(&reprise(2100, "École Mamadou Konaté"), 2000),
            Err("moyenne_reprise.hors_bareme")
        );
        assert_eq!(
            verifier_reprise(&reprise(1250, " "), 2000),
            Err("moyenne_reprise.origine_obligatoire")
        );
    }
}
