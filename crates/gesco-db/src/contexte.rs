//! Contexte d'exécution d'une opération : qui, où, quand.

use gesco_core::commun::{Horodatage, Id};
use gesco_core::core::{Etablissement, Poste, Utilisateur};

/// Tout ce qu'un dépôt doit savoir pour cloisonner (R-12), horodater et tracer (R-13) une
/// écriture. Construit par la couche frontière, jamais par un dépôt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contexte {
    /// Établissement courant : filtre de toutes les requêtes.
    pub etablissement_id: Id<Etablissement>,
    /// Utilisateur à l'origine ; absent pour le système (démarrage, sauvegarde planifiée).
    pub utilisateur_id: Option<Id<Utilisateur>>,
    /// Code du rôle actif.
    pub role_actif: Option<String>,
    /// Poste physique.
    pub poste: Poste,
    /// Empreinte de la machine, pour le journal d'audit.
    pub empreinte_machine: String,
    /// Instant de l'opération, lu une fois par la frontière.
    pub maintenant: Horodatage,
    /// Identifiant de corrélation partagé par l'audit et l'`outbox` de l'opération.
    pub correlation_id: String,
}

impl Contexte {
    /// Identifiant de l'utilisateur en texte, pour `cree_par` et `mis_a_jour_par`.
    pub fn auteur(&self) -> Option<String> {
        self.utilisateur_id.map(|u| u.vers_chaine())
    }
}
