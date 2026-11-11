//! Origine d'un appel à la frontière (SPEC_PHASE_11_A §3.2) : l'écran de ce poste, ou un poste
//! relié que le serveur du réseau local a reconnu. Elle donne le nom de poste écrit dans le
//! journal d'audit et les événements (R-13), et borne ce qu'un poste relié peut demander.
//!
//! L'origine accompagne le calcul de la commande au lieu d'être passée de main en main : le
//! noyau la lit là où il compose le contexte d'une écriture, sans que chaque service la reçoive.

use gesco_core::core::Poste;

/// D'où vient un appel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origine {
    /// L'écran de ce poste : ses écritures portent le nom du poste lui-même.
    Locale,
    /// Un poste relié, sous le nom que le poste principal lui connaît.
    Relie(Poste),
}

impl Origine {
    /// Vrai pour un appel venu d'un poste relié.
    pub fn est_reliee(&self) -> bool {
        matches!(self, Self::Relie(_))
    }
}

tokio::task_local! {
    static ORIGINE: Origine;
}

/// Mène un calcul sous une origine : tout ce qu'il écrit porte le poste de cette origine.
pub(crate) async fn dans<F: Future>(origine: Origine, calcul: F) -> F::Output {
    ORIGINE.scope(origine, calcul).await
}

/// Origine du calcul en cours ; locale hors d'un appel à la frontière (tâches de fond, essais).
pub(crate) fn courante() -> Origine {
    ORIGINE.try_with(Clone::clone).unwrap_or(Origine::Locale)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn l_origine_suit_le_calcul_et_ne_deborde_pas() {
        assert_eq!(courante(), Origine::Locale);
        let relie = Origine::Relie(Poste("caisse".to_owned()));
        let vue = dans(relie.clone(), async {
            tokio::task::yield_now().await;
            courante()
        })
        .await;
        assert_eq!(vue, relie);
        assert_eq!(courante(), Origine::Locale);
    }
}
