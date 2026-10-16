//! Machines à états explicites (ADR-011, R-29, R-30).
//!
//! Chaque processus déclare ses états et la table de ses transitions autorisées. Les services ne
//! modifient jamais un `statut` directement : ils demandent la transition, qui vérifie qu'elle
//! existe. Les tests de transitions sont générés depuis la table.

use thiserror::Error;

/// Transition refusée.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ErreurTransition {
    /// Aucune transition déclarée entre les deux états.
    #[error("transition interdite : {depuis} → {vers}")]
    Interdite {
        /// État de départ.
        depuis: &'static str,
        /// État demandé.
        vers: &'static str,
    },
}

/// Un type à états nommés muni d'une table de transitions.
pub trait MachineAEtats: Copy + Eq + Sized + 'static {
    /// Tous les états, dans l'ordre du cycle de vie.
    const ETATS: &'static [Self];
    /// Couples (depuis, vers) autorisés.
    const TRANSITIONS: &'static [(Self, Self)];

    /// Nom de l'état, pour les messages et la persistance.
    fn nom(&self) -> &'static str;

    /// Vérifie et effectue la transition.
    fn transiter(self, vers: Self) -> Result<Self, ErreurTransition> {
        if Self::TRANSITIONS.contains(&(self, vers)) {
            Ok(vers)
        } else {
            Err(ErreurTransition::Interdite {
                depuis: self.nom(),
                vers: vers.nom(),
            })
        }
    }

    /// États accessibles depuis `self`.
    fn suivants(self) -> Vec<Self> {
        Self::TRANSITIONS
            .iter()
            .filter(|(d, _)| *d == self)
            .map(|(_, v)| *v)
            .collect()
    }

    /// Retrouve un état depuis son nom persisté.
    fn depuis_nom(nom: &str) -> Option<Self> {
        Self::ETATS.iter().copied().find(|e| e.nom() == nom)
    }
}

/// Vérifie exhaustivement une machine : chaque couple d'états est soit autorisé, soit refusé, et
/// le résultat de `transiter` est cohérent avec la table. Utilisé par les tests de chaque machine.
pub fn verifier_exhaustivement<M: MachineAEtats + std::fmt::Debug>() {
    for depuis in M::ETATS {
        for vers in M::ETATS {
            let attendu = M::TRANSITIONS.contains(&(*depuis, *vers));
            let obtenu = depuis.transiter(*vers).is_ok();
            assert_eq!(attendu, obtenu, "{depuis:?} → {vers:?}");
        }
        assert_eq!(M::depuis_nom(depuis.nom()), Some(*depuis));
    }
}
