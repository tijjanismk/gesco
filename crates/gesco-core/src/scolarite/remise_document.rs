//! Remise des documents (SPEC_PHASE_5 §6, blueprint §8.5bis) : un bulletin publié n'est pas un
//! bulletin remis. Chaque bulletin publié naît « à remettre » ; il se remet à un responsable, à
//! l'élève, à une personne autorisée ou à un tiers, et se signe ; resté à remettre quand la
//! période suivante est publiée, il est « non retiré ». Les bulletins non retirés à sept jours
//! forment une liste, qui recoupe celle des impayés.

use serde::{Deserialize, Serialize};

use crate::commun::{Horodatage, Id};
use crate::core::{Etablissement, Utilisateur};
use crate::pedagogie::Periode;
use crate::pedagogie::bulletin::Bulletin;
use crate::scolarite::{Inscription, Responsable};

/// Jours après la publication au-delà desquels un bulletin à remettre est signalé non retiré.
pub const JOURS_AVANT_RAPPEL: i64 = 7;

/// Longueur maximale d'un nom saisi à la remise.
pub const NOM_MAX: usize = 120;

/// État de la remise d'un document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum StatutRemiseDocument {
    /// Publié, pas encore remis.
    ARemettre,
    /// Remis, pas encore signé.
    Remis,
    /// Remis et signé.
    Signe,
    /// Resté à remettre quand la période suivante a été publiée.
    NonRetire,
}

impl StatutRemiseDocument {
    /// Tous les états.
    pub const TOUS: [Self; 4] = [Self::ARemettre, Self::Remis, Self::Signe, Self::NonRetire];

    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::ARemettre => "a_remettre",
            Self::Remis => "remis",
            Self::Signe => "signe",
            Self::NonRetire => "non_retire",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::TOUS.into_iter().find(|s| s.nom() == nom)
    }

    /// Vrai si le document attend encore d'être remis.
    pub const fn en_attente(&self) -> bool {
        matches!(self, Self::ARemettre | Self::NonRetire)
    }
}

/// À qui le document a été remis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum RemisA {
    /// Un responsable de l'élève.
    Responsable,
    /// L'élève lui-même.
    Eleve,
    /// Une personne autorisée à retirer pour l'élève.
    PersonneAutorisee,
    /// Une autre personne, nommée.
    Tiers,
}

impl RemisA {
    /// Tous.
    pub const TOUS: [Self; 4] = [
        Self::Responsable,
        Self::Eleve,
        Self::PersonneAutorisee,
        Self::Tiers,
    ];

    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Responsable => "responsable",
            Self::Eleve => "eleve",
            Self::PersonneAutorisee => "personne_autorisee",
            Self::Tiers => "tiers",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::TOUS.into_iter().find(|r| r.nom() == nom)
    }
}

/// Remise d'un bulletin publié.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct RemiseDocument {
    /// Identifiant technique.
    pub id: Id<RemiseDocument>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Bulletin en vigueur, la dernière version publiée.
    pub bulletin_id: Id<Bulletin>,
    /// Inscription de l'élève.
    pub inscription_id: Id<Inscription>,
    /// Période.
    pub periode_id: Id<Periode>,
    /// État.
    pub statut: StatutRemiseDocument,
    /// À qui il a été remis.
    pub remis_a: Option<RemisA>,
    /// Responsable qui l'a reçu.
    pub responsable_id: Option<Id<Responsable>>,
    /// Nom de qui l'a retiré, pour une personne autorisée ou un tiers.
    pub nom_retrait: String,
    /// Nom de qui a signé.
    pub signataire: String,
    /// Remise.
    pub remis_le: Option<Horodatage>,
    /// Signature.
    pub signe_le: Option<Horodatage>,
    /// Agent qui l'a remis.
    pub remis_par: Option<Id<Utilisateur>>,
    /// Publication de la version en vigueur.
    pub publie_le: Horodatage,
}

/// Vérifie une remise : un responsable désigné, ou le nom de qui retire pour une personne
/// autorisée ou un tiers ; un signataire pour un document signé. L'erreur est la clé du message.
pub fn verifier_remise(
    remis_a: RemisA,
    responsable: bool,
    nom_retrait: &str,
    signe: bool,
    signataire: &str,
) -> Result<(), &'static str> {
    match remis_a {
        RemisA::Responsable if !responsable => return Err("remise.responsable_exige"),
        RemisA::PersonneAutorisee | RemisA::Tiers if nom_retrait.trim().is_empty() => {
            return Err("remise.nom_exige");
        }
        _ => {}
    }
    if nom_retrait.trim().chars().count() > NOM_MAX || signataire.trim().chars().count() > NOM_MAX {
        return Err("remise.nom_trop_long");
    }
    if signe && signataire.trim().is_empty() {
        return Err("remise.signataire_exige");
    }
    Ok(())
}

/// Jours écoulés depuis la publication, et vrai si le bulletin est à signaler non retiré.
pub fn attente(remise: &RemiseDocument, maintenant: Horodatage) -> (i64, bool) {
    let jours = (maintenant.millis_epoque() - remise.publie_le.millis_epoque()) / 86_400_000;
    (
        jours,
        remise.statut.en_attente()
            && (jours >= JOURS_AVANT_RAPPEL || remise.statut == StatutRemiseDocument::NonRetire),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noms_persistes() {
        for s in StatutRemiseDocument::TOUS {
            assert_eq!(StatutRemiseDocument::depuis_nom(s.nom()), Some(s));
        }
        for r in RemisA::TOUS {
            assert_eq!(RemisA::depuis_nom(r.nom()), Some(r));
        }
    }

    #[test]
    fn remises_verifiees() {
        assert_eq!(
            verifier_remise(RemisA::Responsable, true, "", false, ""),
            Ok(())
        );
        assert_eq!(
            verifier_remise(RemisA::Responsable, false, "", false, ""),
            Err("remise.responsable_exige")
        );
        assert_eq!(verifier_remise(RemisA::Eleve, false, "", false, ""), Ok(()));
        assert_eq!(
            verifier_remise(RemisA::Tiers, false, " ", false, ""),
            Err("remise.nom_exige")
        );
        assert_eq!(
            verifier_remise(RemisA::PersonneAutorisee, false, "Oncle Bakary", true, ""),
            Err("remise.signataire_exige")
        );
        assert_eq!(
            verifier_remise(
                RemisA::PersonneAutorisee,
                false,
                "Oncle Bakary",
                true,
                "Bakary Diallo"
            ),
            Ok(())
        );
    }
}
