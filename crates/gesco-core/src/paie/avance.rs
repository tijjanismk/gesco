//! Avances sur salaire (SPEC_PHASE_7 §4, T-55) : un montant remboursé en mensualités, retenues
//! chaque mois sur la paie jusqu'au solde. La mensualité est tronquée au franc ; la dernière solde
//! l'avance. Une avance qui n'a encore rien remboursé s'annule avec un motif.

use serde::{Deserialize, Serialize};

use crate::commun::{DateCivile, Id, Montant};
use crate::core::{Etablissement, MachineAEtats};
use crate::rh::{MembrePersonnel, Mois};

/// Nombre de mensualités au plus : deux ans.
pub const MENSUALITES_MAX: i64 = 24;

/// Longueur maximale du motif d'une avance ou de son annulation.
pub const MOTIF_AVANCE_MAX: usize = 120;

/// État d'une avance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum StatutAvance {
    /// En cours de remboursement.
    EnCours,
    /// Entièrement remboursée.
    Soldee,
    /// Annulée avant tout remboursement.
    Annulee,
}

impl MachineAEtats for StatutAvance {
    const ETATS: &'static [Self] = &[Self::EnCours, Self::Soldee, Self::Annulee];
    const TRANSITIONS: &'static [(Self, Self)] = &[
        (Self::EnCours, Self::Soldee),
        (Self::EnCours, Self::Annulee),
    ];

    fn nom(&self) -> &'static str {
        match self {
            Self::EnCours => "en_cours",
            Self::Soldee => "soldee",
            Self::Annulee => "annulee",
        }
    }
}

impl StatutAvance {
    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::ETATS.iter().copied().find(|s| s.nom() == nom)
    }
}

/// Avance accordée à un membre.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Avance {
    /// Identifiant technique.
    pub id: Id<Avance>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Membre.
    pub membre_id: Id<MembrePersonnel>,
    /// Montant avancé.
    pub montant: Montant,
    /// Nombre de mois de remboursement.
    pub mensualites: i64,
    /// Premier mois retenu sur la paie.
    pub premier_mois: Mois,
    /// Jour où elle est accordée.
    pub accordee_le: DateCivile,
    /// Motif.
    pub motif: String,
    /// État.
    pub statut: StatutAvance,
    /// Motif de l'annulation.
    pub motif_annulation: String,
}

impl Avance {
    /// Mensualité prévue : le montant divisé par le nombre de mois, tronqué au franc.
    pub fn mensualite(&self) -> Montant {
        self.montant
            .part(1, self.mensualites.max(1))
            .map(|d| d.quotient)
            .unwrap_or(self.montant)
    }

    /// Ce que la paie retient ce mois : la mensualité, ou tout le reste quand il ne tient plus
    /// dans deux mensualités, pour que la dernière solde l'avance.
    pub fn a_retenir(&self, reste: Montant) -> Montant {
        let mensualite = self.mensualite();
        if reste.francs() < mensualite.francs().saturating_mul(2) {
            reste
        } else {
            mensualite
        }
    }
}

/// Vérifie une avance : un montant, de un à vingt-quatre mois, un motif.
pub fn verifier_avance(
    montant: Montant,
    mensualites: i64,
    motif: &str,
) -> Result<(), &'static str> {
    if montant.francs() <= 0 {
        return Err("avance.montant_invalide");
    }
    if !(1..=MENSUALITES_MAX).contains(&mensualites) {
        return Err("avance.mensualites_invalides");
    }
    let motif = motif.trim();
    if motif.is_empty() || motif.chars().count() > MOTIF_AVANCE_MAX {
        return Err("avance.motif_obligatoire");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn avance(montant: i64, mensualites: i64) -> Avance {
        Avance {
            id: Id::depuis_octets([1; 16]),
            etablissement_id: Id::depuis_octets([2; 16]),
            membre_id: Id::depuis_octets([3; 16]),
            montant: Montant::depuis_francs(montant),
            mensualites,
            premier_mois: Mois::nouveau(2026, 10).unwrap_or_else(|| unreachable!()),
            accordee_le: DateCivile::nouvelle(2026, 10, 1).unwrap_or_else(|| unreachable!()),
            motif: "Rentrée des enfants".to_owned(),
            statut: StatutAvance::EnCours,
            motif_annulation: String::new(),
        }
    }

    #[test]
    fn la_derniere_mensualite_solde_l_avance() {
        let a = avance(25_000, 3);
        assert_eq!(a.mensualite(), Montant::depuis_francs(8_333));
        let mut reste = a.montant;
        let mut retenues = Vec::new();
        while reste.francs() > 0 {
            let r = a.a_retenir(reste);
            retenues.push(r.francs());
            reste = Montant::depuis_francs(reste.francs() - r.francs());
        }
        assert_eq!(retenues, vec![8_333, 8_333, 8_334]);
        assert_eq!(
            avance(30_000, 3)
                .a_retenir(Montant::depuis_francs(30_000))
                .francs(),
            10_000
        );
        assert_eq!(
            avance(30_000, 1)
                .a_retenir(Montant::depuis_francs(30_000))
                .francs(),
            30_000
        );
    }

    #[test]
    fn une_avance_a_un_montant_une_duree_et_un_motif() {
        let m = Montant::depuis_francs(30_000);
        assert_eq!(verifier_avance(m, 3, "Rentrée"), Ok(()));
        assert_eq!(
            verifier_avance(Montant::ZERO, 3, "Rentrée"),
            Err("avance.montant_invalide")
        );
        assert_eq!(
            verifier_avance(m, 25, "Rentrée"),
            Err("avance.mensualites_invalides")
        );
        assert_eq!(verifier_avance(m, 3, " "), Err("avance.motif_obligatoire"));
        assert!(
            StatutAvance::EnCours
                .transiter(StatutAvance::Annulee)
                .is_ok()
        );
        assert!(
            StatutAvance::Soldee
                .transiter(StatutAvance::EnCours)
                .is_err()
        );
    }
}
