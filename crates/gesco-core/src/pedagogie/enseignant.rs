//! Enseignants et affectations (SPEC_PHASE_4 §3) : qui enseigne quelle matière dans quelle
//! classe, et donc qui peut y saisir des notes (§11.3 du blueprint). Plusieurs enseignants
//! peuvent partager une matière d'une classe (T-22) ; un vacataire est un enseignant comme les
//! autres (T-24) ; au 1er cycle, le maître reçoit toutes les matières de sa classe (T-25).

use serde::{Deserialize, Serialize};

use crate::commun::Id;
use crate::core::{AnneeScolaire, Etablissement, Utilisateur};
use crate::pedagogie::Matiere;
use crate::scolarite::{Classe, Personne};

/// Statut d'un enseignant dans l'établissement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum StatutEnseignant {
    /// Enseignant de l'établissement.
    Titulaire,
    /// Vacataire, qui peut enseigner ailleurs (T-24).
    Vacataire,
}

impl StatutEnseignant {
    /// Les deux statuts.
    pub const TOUS: [Self; 2] = [Self::Titulaire, Self::Vacataire];

    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Titulaire => "titulaire",
            Self::Vacataire => "vacataire",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::TOUS.into_iter().find(|s| s.nom() == nom)
    }
}

/// Enseignant : une personne, un statut, et un compte s'il saisit lui-même ses notes (Q-55). Le
/// dossier du personnel (phase 6) le complétera sans toucher à ses affectations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Enseignant {
    /// Identifiant technique.
    pub id: Id<Enseignant>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Personne : nom, prénoms, téléphones.
    pub personne_id: Id<Personne>,
    /// Statut.
    pub statut: StatutEnseignant,
    /// Compte utilisateur lié, pour saisir ses notes.
    pub utilisateur_id: Option<Id<Utilisateur>>,
    /// Faux pour un enseignant parti : ses affectations passées restent.
    pub actif: bool,
}

/// Affectation d'un enseignant à une matière d'une classe pour une année.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct AffectationEnseignement {
    /// Identifiant technique.
    pub id: Id<AffectationEnseignement>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Année.
    pub annee_id: Id<AnneeScolaire>,
    /// Classe.
    pub classe_id: Id<Classe>,
    /// Matière.
    pub matiere_id: Id<Matiere>,
    /// Enseignant.
    pub enseignant_id: Id<Enseignant>,
}

/// Où un utilisateur peut saisir des notes (§11.3 du blueprint).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PorteeNotes {
    /// Partout dans l'établissement : la direction, qui saisit à partir du relevé (T-23).
    Etablissement,
    /// Dans ses seuls couples classe et matière : un enseignant avec son compte.
    Couples {
        /// Couples autorisés.
        couples: Vec<(Id<Classe>, Id<Matiere>)>,
    },
}

impl PorteeNotes {
    /// Portée d'un utilisateur : la direction (qui valide les notes) saisit partout ; un compte
    /// lié à un enseignant, dans ses affectations ; tout autre compte, nulle part.
    pub fn de(valide_les_notes: bool, affectations: &[AffectationEnseignement]) -> Self {
        if valide_les_notes {
            Self::Etablissement
        } else {
            Self::Couples {
                couples: affectations
                    .iter()
                    .map(|a| (a.classe_id, a.matiere_id))
                    .collect(),
            }
        }
    }

    /// Vrai si l'utilisateur peut saisir dans cette matière de cette classe.
    pub fn autorise(&self, classe: Id<Classe>, matiere: Id<Matiere>) -> bool {
        match self {
            Self::Etablissement => true,
            Self::Couples { couples } => couples.contains(&(classe, matiere)),
        }
    }
}

/// Matières du programme d'une classe que personne n'enseigne encore : celles que reçoit le
/// maître affecté à sa classe d'un geste (T-25). Une matière déjà confiée à un autre le reste.
pub fn matieres_a_confier(
    programme: &[Id<Matiere>],
    affectations_de_la_classe: &[AffectationEnseignement],
) -> Vec<Id<Matiere>> {
    programme
        .iter()
        .copied()
        .filter(|m| !affectations_de_la_classe.iter().any(|a| a.matiere_id == *m))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id<T>(n: u128) -> Id<T> {
        Id::depuis_ulid(ulid::Ulid::from(n))
    }

    fn affectation(classe: u128, matiere: u128, enseignant: u128) -> AffectationEnseignement {
        AffectationEnseignement {
            id: id(1000 + classe * 100 + matiere),
            etablissement_id: id(1),
            annee_id: id(2),
            classe_id: id(classe),
            matiere_id: id(matiere),
            enseignant_id: id(enseignant),
        }
    }

    #[test]
    fn la_direction_saisit_partout_l_enseignant_dans_ses_classes() {
        let siennes = [affectation(10, 20, 7), affectation(11, 20, 7)];
        let enseignant = PorteeNotes::de(false, &siennes);
        assert!(enseignant.autorise(id(10), id(20)));
        assert!(!enseignant.autorise(id(10), id(21)));
        assert!(!enseignant.autorise(id(12), id(20)));
        assert!(PorteeNotes::de(true, &[]).autorise(id(12), id(21)));
        assert!(!PorteeNotes::de(false, &[]).autorise(id(10), id(20)));
    }

    #[test]
    fn le_maitre_recoit_les_matieres_que_personne_n_enseigne() {
        let programme = [id(20), id(21), id(22)];
        // L'anglais (22) est déjà confié à un autre maître.
        let deja = [affectation(10, 22, 8)];
        assert_eq!(matieres_a_confier(&programme, &deja), vec![id(20), id(21)]);
        assert_eq!(matieres_a_confier(&programme, &[]).len(), 3);
    }
}
