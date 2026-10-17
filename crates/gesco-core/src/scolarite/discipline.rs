//! Discipline (SPEC_VERSION_1_1 §1, étape 3, T-70) : le dossier de discipline d'un élève —
//! avertissement, blâme, retenue, convocation des parents —, daté, motivé, avec qui l'a décidé.
//! L'avertissement et le blâme du conseil de classe, rattachés à une période, paraissent sur le
//! bulletin de la période. L'exclusion temporaire et la radiation restent celles de la phase 4.
//! Une sanction saisie par erreur s'annule avec un motif (R-14).

use serde::{Deserialize, Serialize};

use crate::commun::{DateCivile, Id};
use crate::core::{Etablissement, Utilisateur};
use crate::pedagogie::Periode;
use crate::scolarite::Inscription;

/// Longueur maximale d'un motif ou de l'auteur d'une décision.
pub const TEXTE_SANCTION_MAX: usize = 200;

/// Nature d'une sanction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum NatureSanction {
    /// Avertissement, de la direction ou du conseil de classe.
    Avertissement,
    /// Blâme, de la direction ou du conseil de classe.
    Blame,
    /// Retenue : heures de travail en dehors des cours.
    Retenue,
    /// Convocation des parents.
    ConvocationParents,
}

impl NatureSanction {
    /// Toutes les natures.
    pub const TOUTES: [Self; 4] = [
        Self::Avertissement,
        Self::Blame,
        Self::Retenue,
        Self::ConvocationParents,
    ];

    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Avertissement => "avertissement",
            Self::Blame => "blame",
            Self::Retenue => "retenue",
            Self::ConvocationParents => "convocation_parents",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::TOUTES.into_iter().find(|n| n.nom() == nom)
    }

    /// Libellé imprimé.
    pub const fn libelle(&self) -> &'static str {
        match self {
            Self::Avertissement => "Avertissement",
            Self::Blame => "Blâme",
            Self::Retenue => "Retenue",
            Self::ConvocationParents => "Convocation des parents",
        }
    }

    /// Vrai pour une sanction que le conseil de classe peut porter sur le bulletin.
    pub const fn du_conseil(&self) -> bool {
        matches!(self, Self::Avertissement | Self::Blame)
    }
}

/// Sanction du dossier de discipline d'un élève.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Sanction {
    /// Identifiant technique.
    pub id: Id<Sanction>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Inscription de l'élève.
    pub inscription_id: Id<Inscription>,
    /// Nature.
    pub nature: NatureSanction,
    /// Jour de la décision.
    pub date: DateCivile,
    /// Motif.
    pub motif: String,
    /// Qui l'a décidée : la direction, le conseil de classe, le surveillant général.
    pub decidee_par: String,
    /// Période du conseil de classe, pour un avertissement ou un blâme porté sur le bulletin.
    pub periode_id: Option<Id<Periode>>,
    /// Qui l'a saisie.
    pub saisie_par: Id<Utilisateur>,
    /// Annulée, avec son motif.
    pub annulee: bool,
    /// Motif de l'annulation.
    pub motif_annulation: String,
}

/// Vérifie une sanction : un motif, qui l'a décidée ; seuls l'avertissement et le blâme se
/// rattachent à la période d'un conseil de classe.
pub fn verifier_sanction(
    nature: NatureSanction,
    motif: &str,
    decidee_par: &str,
    periode: bool,
) -> Result<(), &'static str> {
    for texte in [motif, decidee_par] {
        let t = texte.trim();
        if t.is_empty() || t.chars().count() > TEXTE_SANCTION_MAX {
            return Err("sanction.texte_invalide");
        }
    }
    if periode && !nature.du_conseil() {
        return Err("sanction.periode_reservee_au_conseil");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seuls_l_avertissement_et_le_blame_vont_au_bulletin() {
        assert_eq!(
            verifier_sanction(
                NatureSanction::Blame,
                "Bavardages répétés",
                "Conseil de classe",
                true
            ),
            Ok(())
        );
        assert_eq!(
            verifier_sanction(
                NatureSanction::Retenue,
                "Retard",
                "Surveillant général",
                true
            ),
            Err("sanction.periode_reservee_au_conseil")
        );
        assert_eq!(
            verifier_sanction(NatureSanction::Avertissement, " ", "Direction", false),
            Err("sanction.texte_invalide")
        );
        for n in NatureSanction::TOUTES {
            assert_eq!(NatureSanction::depuis_nom(n.nom()), Some(n));
        }
    }
}
