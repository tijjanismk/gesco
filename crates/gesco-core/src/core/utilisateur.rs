//! Utilisateurs et postes.

use serde::{Deserialize, Serialize};

use super::etablissement::Etablissement;
use crate::commun::{Horodatage, Id};

/// Poste physique identifié ; porté par chaque ligne synchronisable (R-13).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(transparent)]
pub struct Poste(pub String);

/// Compte d'un utilisateur de l'établissement. Les secrets (empreintes du mot de passe et du PIN)
/// ne font pas partie de l'entité : ils vivent dans la couche d'authentification et n'apparaissent
/// jamais dans une sérialisation (R-25).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Utilisateur {
    /// Identifiant technique.
    pub id: Id<Utilisateur>,
    /// Établissement de rattachement.
    pub etablissement_id: Id<Etablissement>,
    /// Identifiant de connexion, unique par établissement.
    pub identifiant: String,
    /// Nom affiché en permanence dans le bandeau (RA-13).
    pub nom_affiche: String,
    /// Compte actif.
    pub actif: bool,
    /// Changement de mot de passe exigé à la prochaine connexion.
    pub doit_changer_mot_de_passe: bool,
    /// Dernière connexion réussie.
    pub derniere_connexion: Option<Horodatage>,
    /// Échecs consécutifs d'authentification.
    pub echecs_consecutifs: u32,
    /// Compte verrouillé jusqu'à cet instant après échecs répétés.
    pub verrouille_jusqua: Option<Horodatage>,
}

impl Utilisateur {
    /// Vrai si une tentative de connexion est recevable à l'instant donné.
    pub fn peut_tenter_connexion(&self, maintenant: Horodatage) -> bool {
        self.actif
            && self
                .verrouille_jusqua
                .is_none_or(|fin| !maintenant.avant(&fin))
    }

    /// Durée de verrouillage après `echecs` échecs consécutifs : rien avant le `seuil`
    /// (paramètre `connexion.echecs_avant_verrouillage`, 3 par défaut), puis 30 s doublées à
    /// chaque échec, plafonnées à une heure.
    pub const fn duree_verrouillage_millis(echecs: u32, seuil: u32) -> Option<i64> {
        if echecs < seuil {
            return None;
        }
        let paliers = echecs - seuil;
        let base: i64 = 30_000;
        let duree = if paliers >= 7 {
            3_600_000
        } else {
            base << paliers
        };
        Some(if duree > 3_600_000 { 3_600_000 } else { duree })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verrouillage_progressif() {
        assert_eq!(Utilisateur::duree_verrouillage_millis(0, 3), None);
        assert_eq!(Utilisateur::duree_verrouillage_millis(2, 3), None);
        assert_eq!(Utilisateur::duree_verrouillage_millis(3, 3), Some(30_000));
        assert_eq!(Utilisateur::duree_verrouillage_millis(4, 3), Some(60_000));
        assert_eq!(
            Utilisateur::duree_verrouillage_millis(9, 3),
            Some(1_920_000)
        );
        assert_eq!(
            Utilisateur::duree_verrouillage_millis(10, 3),
            Some(3_600_000)
        );
        assert_eq!(
            Utilisateur::duree_verrouillage_millis(200, 3),
            Some(3_600_000)
        );
        assert_eq!(Utilisateur::duree_verrouillage_millis(4, 5), None);
        assert_eq!(Utilisateur::duree_verrouillage_millis(5, 5), Some(30_000));
    }
}
