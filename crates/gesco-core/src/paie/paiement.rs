//! Paiements du personnel (SPEC_PHASE_7 §7, T-58, T-59) : le net d'un bulletin validé ou une
//! avance, payés en une ou plusieurs fois, en espèces de la caisse ou hors caisse. Rien ne bloque
//! un paiement : ni une caisse fermée, ni un tiroir qui ne suffit pas ; seul le dépassement de ce
//! qui reste dû est refusé. Un paiement saisi par erreur s'annule avec un motif.

use serde::{Deserialize, Serialize};

use crate::commun::{Horodatage, Id, Montant};
use crate::core::{Etablissement, Utilisateur};
use crate::finance::{ModePaiement, SessionCaisse};
use crate::paie::avance::Avance;
use crate::paie::mois::BulletinPaie;
use crate::rh::MembrePersonnel;

/// Ce qui est payé.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ObjetPaiement {
    /// Le net d'un bulletin de paie validé.
    Salaire,
    /// Une avance accordée.
    Avance,
}

impl ObjetPaiement {
    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Salaire => "salaire",
            Self::Avance => "avance",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        [Self::Salaire, Self::Avance]
            .into_iter()
            .find(|o| o.nom() == nom)
    }
}

/// D'où sort l'argent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum SourcePaiement {
    /// Espèces du tiroir de la session de caisse ouverte de qui paie.
    Caisse,
    /// Hors de la caisse : mobile money, banque, ou espèces d'une autre source (Q-77).
    HorsCaisse,
}

impl SourcePaiement {
    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Caisse => "caisse",
            Self::HorsCaisse => "hors_caisse",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        [Self::Caisse, Self::HorsCaisse]
            .into_iter()
            .find(|s| s.nom() == nom)
    }
}

/// Paiement d'un membre du personnel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct PaiementPersonnel {
    /// Identifiant technique.
    pub id: Id<PaiementPersonnel>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Membre payé.
    pub membre_id: Id<MembrePersonnel>,
    /// Salaire ou avance.
    pub objet: ObjetPaiement,
    /// Bulletin payé, pour un salaire.
    pub bulletin_id: Option<Id<BulletinPaie>>,
    /// Avance versée.
    pub avance_id: Option<Id<Avance>>,
    /// Montant.
    pub montant: Montant,
    /// Instant.
    pub le: Horodatage,
    /// Mode.
    pub mode: ModePaiement,
    /// Référence de la transaction, hors espèces.
    pub reference: String,
    /// Source de l'argent.
    pub source: SourcePaiement,
    /// Session de caisse d'où sortent les espèces.
    pub session_id: Option<Id<SessionCaisse>>,
    /// Qui paie.
    pub par: Id<Utilisateur>,
    /// Annulé.
    pub annule: bool,
    /// Motif de l'annulation.
    pub motif_annulation: String,
}

/// Vérifie un paiement : un montant positif qui ne dépasse pas ce qui reste dû, la référence
/// hors espèces, des espèces seulement pour la caisse.
pub fn verifier_paiement(
    montant: Montant,
    reste: Montant,
    mode: ModePaiement,
    reference: &str,
    source: SourcePaiement,
) -> Result<(), &'static str> {
    if montant.francs() <= 0 {
        return Err("paiement.montant_invalide");
    }
    if montant.francs() > reste.francs() {
        return Err("paiement.superieur_au_reste");
    }
    if mode.exige_reference() && reference.trim().is_empty() {
        return Err("paiement.reference_obligatoire");
    }
    if source == SourcePaiement::Caisse && mode != ModePaiement::Especes {
        return Err("paiement.caisse_especes");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_paiement_ne_depasse_pas_le_reste() {
        let f = Montant::depuis_francs;
        let caisse = SourcePaiement::Caisse;
        let dehors = SourcePaiement::HorsCaisse;
        assert_eq!(
            verifier_paiement(f(50_000), f(93_400), ModePaiement::Especes, "", caisse),
            Ok(())
        );
        assert_eq!(
            verifier_paiement(f(100_000), f(93_400), ModePaiement::Especes, "", caisse),
            Err("paiement.superieur_au_reste")
        );
        assert_eq!(
            verifier_paiement(f(0), f(93_400), ModePaiement::Especes, "", caisse),
            Err("paiement.montant_invalide")
        );
        assert_eq!(
            verifier_paiement(f(10_000), f(93_400), ModePaiement::OrangeMoney, " ", dehors),
            Err("paiement.reference_obligatoire")
        );
        assert_eq!(
            verifier_paiement(
                f(10_000),
                f(93_400),
                ModePaiement::OrangeMoney,
                "OM-1",
                caisse
            ),
            Err("paiement.caisse_especes")
        );
        assert_eq!(
            verifier_paiement(f(10_000), f(93_400), ModePaiement::Especes, "", dehors),
            Ok(())
        );
    }
}
