//! Compléments pour les rapports (SPEC_PHASE_9 §8.2, §8.3) : ce que les formulaires demandent
//! et que les fiches ne disaient pas. Tout est facultatif ; rien ne touche à la paie ni n'exige
//! une pièce au dossier. Le statut du personnel n'est jamais vide à l'affichage : tant que
//! personne ne l'a saisi, il se déduit de la fiche.

use serde::{Deserialize, Serialize};

use crate::commun::Id;
use crate::core::Etablissement;

/// Liste de valeurs rangées dans la base sous un nom stable.
macro_rules! liste {
    ($(#[$doc:meta])* $nom:ident { $($(#[$vdoc:meta])* $variante:ident = $texte:literal,)+ }) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
        #[serde(rename_all = "snake_case")]
        pub enum $nom {
            $($(#[$vdoc])* $variante,)+
        }

        impl $nom {
            /// Toutes les valeurs, dans l'ordre des formulaires.
            pub const TOUS: &'static [Self] = &[$(Self::$variante,)+];

            /// Nom stable, rangé dans la base.
            pub const fn nom(self) -> &'static str {
                match self {
                    $(Self::$variante => $texte,)+
                }
            }

            /// Valeur d'un nom de la base ; rien pour un nom inconnu.
            pub fn depuis_nom(nom: &str) -> Option<Self> {
                Self::TOUS.iter().copied().find(|v| v.nom() == nom)
            }
        }
    };
}

liste! {
    /// Statut d'un membre du personnel pour les rapports : une seule liste, que chaque
    /// formulaire regroupe selon ses colonnes.
    StatutRapport {
        /// Fonctionnaire de l'État.
        FonctionnaireEtat = "fonctionnaire_etat",
        /// Fonctionnaire des collectivités.
        FonctionnaireCollectivites = "fonctionnaire_collectivites",
        /// Contractuel de l'État.
        ContractuelEtat = "contractuel_etat",
        /// Contractuel des collectivités.
        ContractuelCollectivites = "contractuel_collectivites",
        /// Contractuel de l'école : privé, « local », « fonds local ».
        ContractuelEcole = "contractuel_ecole",
        /// Vacataire ou prestataire.
        Vacataire = "vacataire",
        /// Élève-maître ou stagiaire.
        EleveMaitre = "eleve_maitre",
        /// Autre.
        Autre = "autre",
    }
}

impl StatutRapport {
    /// Statut déduit de la fiche quand personne ne l'a saisi : fonctionnaire coché →
    /// fonctionnaire de l'État ; engagement en cours de vacation → vacataire ; de stage →
    /// élève-maître ; sinon contractuel de l'école.
    pub fn deduit(fonctionnaire: bool, nature_engagement: Option<&str>) -> Self {
        if fonctionnaire {
            return Self::FonctionnaireEtat;
        }
        match nature_engagement {
            Some("vacation") => Self::Vacataire,
            Some("stage") => Self::EleveMaitre,
            _ => Self::ContractuelEcole,
        }
    }

    /// Permanent au sens du rapport succinct : fonctionnaire de l'État ou des collectivités.
    pub const fn permanent(self) -> bool {
        matches!(
            self,
            Self::FonctionnaireEtat | Self::FonctionnaireCollectivites
        )
    }
}

liste! {
    /// Spécialité d'un enseignant du second cycle.
    Specialite {
        /// Lettres, histoire, géographie.
        Lhg = "lhg",
        /// Mathématiques, physique, chimie.
        Mpc = "mpc",
        /// Sciences naturelles, physique, chimie.
        Snpc = "snpc",
        /// Langues étrangères.
        Ldm = "ldm",
        /// Éducation physique et sportive.
        Eps = "eps",
        /// Autres.
        Autre = "autre",
    }
}

impl Specialite {
    /// Spécialité que suggère le nom d'une matière ; rien quand il ne dit rien.
    pub fn d_apres_matiere(matiere: &str) -> Option<Self> {
        let m = matiere.to_lowercase();
        let contient = |mots: &[&str]| mots.iter().any(|x| m.contains(x));
        if contient(&["eps", "éducation physique", "education physique", "sport"]) {
            Some(Self::Eps)
        } else if contient(&[
            "anglais", "allemand", "espagnol", "arabe", "russe", "langue",
        ]) {
            Some(Self::Ldm)
        } else if contient(&["biolog", "sciences nat", "svt", "sciences de la vie"]) {
            Some(Self::Snpc)
        } else if contient(&["math", "physique", "chimie"]) {
            Some(Self::Mpc)
        } else if contient(&[
            "français",
            "francais",
            "lettre",
            "histoire",
            "géograph",
            "geograph",
            "lecture",
            "rédaction",
            "grammaire",
        ]) {
            Some(Self::Lhg)
        } else {
            None
        }
    }

    /// Spécialité proposée pour un maître : celle de toutes ses matières, quand elles tombent
    /// toutes dans une même spécialité ; rien sinon.
    pub fn proposee<'a>(matieres: impl IntoIterator<Item = &'a str>) -> Option<Self> {
        let mut retenue: Option<Self> = None;
        for m in matieres {
            let s = Self::d_apres_matiere(m)?;
            match retenue {
                Some(r) if r != s => return None,
                _ => retenue = Some(s),
            }
        }
        retenue
    }
}

liste! {
    /// Diplôme le plus élevé, dans la liste de l'enquête, du plus haut au plus bas.
    DiplomeEleve {
        /// Doctorat.
        Doctorat = "doctorat",
        /// Master.
        Master = "master",
        /// DEA.
        Dea = "dea",
        /// Diplôme de l'ENSUP.
        Ensup = "ensup",
        /// Ingénieur.
        Ingenieur = "ingenieur",
        /// Maîtrise.
        Maitrise = "maitrise",
        /// Licence.
        Licence = "licence",
        /// DEUG ou BTS.
        DeugBts = "deug_bts",
        /// BT.
        Bt = "bt",
        /// Baccalauréat.
        Baccalaureat = "baccalaureat",
        /// CAP.
        Cap = "cap",
        /// Diplôme d'IFM.
        Ifm = "ifm",
        /// Autre.
        Autre = "autre",
    }
}

impl DiplomeEleve {
    /// Diplôme proposé d'après les diplômes saisis en texte libre : le plus élevé que l'on y
    /// reconnaît ; rien sinon. Le texte reste tel quel.
    pub fn propose(diplomes: &str) -> Option<Self> {
        let t = diplomes.to_lowercase();
        let mots: Vec<&str> = t
            .split(|c: char| !c.is_alphanumeric())
            .filter(|m| !m.is_empty())
            .collect();
        let a = |x: &str| mots.contains(&x);
        let trouve = |d: Self| match d {
            Self::Doctorat => t.contains("doctorat"),
            Self::Master => t.contains("master"),
            Self::Dea => a("dea"),
            Self::Ensup => t.contains("ensup"),
            Self::Ingenieur => t.contains("ingénieur") || t.contains("ingenieur"),
            Self::Maitrise => t.contains("maîtrise") || t.contains("maitrise"),
            Self::Licence => t.contains("licence"),
            Self::DeugBts => a("deug") || a("bts"),
            Self::Bt => a("bt"),
            Self::Baccalaureat => t.contains("bac"),
            Self::Cap => a("cap"),
            Self::Ifm => a("ifm") || t.contains("ifm"),
            Self::Autre => false,
        };
        Self::TOUS.iter().copied().find(|d| trouve(*d))
    }
}

liste! {
    /// Fonctionnement d'une classe.
    Fonctionnement {
        /// Une classe, un maître, une salle.
        Simple = "simple",
        /// Deux années d'études dans la même classe.
        DoubleDivision = "double_division",
        /// Deux groupes qui se partagent la salle dans la journée.
        DoubleVacation = "double_vacation",
    }
}

liste! {
    /// Pédagogie appliquée dans une classe.
    PedagogieClasse {
        /// Classique.
        Classique = "classique",
        /// Pédagogie convergente.
        PedagogieConvergente = "pedagogie_convergente",
        /// Curriculum.
        Curriculum = "curriculum",
    }
}

impl Fonctionnement {
    /// Valeur d'un libellé de la fiche de l'établissement (« Double division »).
    pub fn depuis_libelle(libelle: &str) -> Option<Self> {
        Self::depuis_nom(&libelle.to_lowercase().replace(' ', "_"))
    }
}

impl PedagogieClasse {
    /// Valeur d'un libellé de la fiche de l'établissement (« Pédagogie convergente »).
    pub fn depuis_libelle(libelle: &str) -> Option<Self> {
        Self::depuis_nom(&libelle.to_lowercase().replace('é', "e").replace(' ', "_"))
    }
}

liste! {
    /// Type de difficulté d'un élève, dans la liste de l'enquête.
    TypeDifficulte {
        /// Voir.
        Vue = "vue",
        /// Entendre.
        Ouie = "ouie",
        /// Marcher, se déplacer.
        MotriciteGlobale = "motricite_globale",
        /// Se servir de ses mains.
        MotriciteFine = "motricite_fine",
        /// Apprendre, comprendre.
        Intellect = "intellect",
        /// Communiquer.
        Communication = "communication",
        /// Se comporter, se faire des amis.
        Comportement = "comportement",
        /// Plusieurs difficultés.
        Multiple = "multiple",
        /// Autre.
        Autre = "autre",
    }
}

liste! {
    /// Degré d'une difficulté.
    DegreDifficulte {
        /// Quelques difficultés.
        Quelques = "quelques",
        /// Beaucoup de difficultés.
        Beaucoup = "beaucoup",
        /// N'y parvient pas.
        Impossible = "impossible",
    }
}

/// Difficulté notée pour un élève. Confidentielle : lue et écrite avec un droit propre, jamais
/// dans une liste nominative.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DifficulteEleve {
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Élève.
    pub eleve_id: String,
    /// Type.
    pub type_difficulte: TypeDifficulte,
    /// Degré ; vide quand la difficulté a été retirée.
    pub degre: Option<DegreDifficulte>,
    /// Note libre.
    pub note: String,
}

/// Longueur maximale de la note d'une difficulté.
pub const NOTE_DIFFICULTE_MAX: usize = 300;

/// Plus petite année de recrutement admise.
pub const ANNEE_RECRUTEMENT_MIN: i32 = 1950;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_statut_se_deduit_de_la_fiche() {
        assert_eq!(
            StatutRapport::deduit(true, Some("vacation")),
            StatutRapport::FonctionnaireEtat
        );
        assert_eq!(
            StatutRapport::deduit(false, Some("vacation")),
            StatutRapport::Vacataire
        );
        assert_eq!(
            StatutRapport::deduit(false, Some("stage")),
            StatutRapport::EleveMaitre
        );
        assert_eq!(
            StatutRapport::deduit(false, None),
            StatutRapport::ContractuelEcole
        );
        assert!(StatutRapport::FonctionnaireCollectivites.permanent());
        assert!(!StatutRapport::ContractuelEcole.permanent());
    }

    #[test]
    fn la_specialite_se_propose_quand_les_matieres_s_accordent() {
        assert_eq!(
            Specialite::proposee(["Mathématiques", "Physique-chimie"]),
            Some(Specialite::Mpc)
        );
        assert_eq!(Specialite::proposee(["Mathématiques", "Anglais"]), None);
        assert_eq!(Specialite::proposee(["Dessin"]), None);
        assert_eq!(Specialite::proposee([]), None);
    }

    #[test]
    fn le_diplome_se_propose_d_apres_le_texte() {
        assert_eq!(
            DiplomeEleve::propose("Bac 2008, Licence en lettres"),
            Some(DiplomeEleve::Licence)
        );
        assert_eq!(
            DiplomeEleve::propose("Diplôme de fin d'études IFM"),
            Some(DiplomeEleve::Ifm)
        );
        assert_eq!(
            DiplomeEleve::propose("CAP mécanique"),
            Some(DiplomeEleve::Cap)
        );
        assert_eq!(DiplomeEleve::propose(""), None);
    }

    #[test]
    fn les_libelles_de_la_fiche_se_relisent() {
        assert_eq!(
            Fonctionnement::depuis_libelle("Double vacation"),
            Some(Fonctionnement::DoubleVacation)
        );
        assert_eq!(
            PedagogieClasse::depuis_libelle("Pédagogie convergente"),
            Some(PedagogieClasse::PedagogieConvergente)
        );
        for t in TypeDifficulte::TOUS {
            assert_eq!(TypeDifficulte::depuis_nom(t.nom()), Some(*t));
        }
    }
}
