//! Sessions ouvertes sur le poste. Plusieurs personnes se succèdent au même clavier (§24.2 du
//! blueprint, RA-13) : chaque session se verrouille seule après inactivité et se rouvre par code
//! PIN, sans repasser par le mot de passe.

use std::collections::HashMap;
use std::sync::Mutex;

use gesco_core::commun::{Horodatage, Id};
use gesco_core::core::{Etablissement, Utilisateur};
use serde::{Deserialize, Serialize};

/// Jeton opaque d'une session, remis au frontend.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub struct Jeton(pub String);

/// Session d'un utilisateur sur ce poste.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Session {
    /// Jeton de la session.
    pub jeton: Jeton,
    /// Utilisateur connecté.
    pub utilisateur_id: Id<Utilisateur>,
    /// Nom affiché en permanence dans le bandeau (§6.2).
    pub nom_affiche: String,
    /// Établissement de la session.
    pub etablissement_id: Id<Etablissement>,
    /// Rôle actif, pour l'audit.
    pub role_actif: Option<String>,
    /// Ouverture de la session.
    pub ouverte_le: Horodatage,
    /// Dernière action enregistrée.
    pub derniere_activite: Horodatage,
    /// Verrouillée : un code PIN est attendu avant toute commande.
    pub verrouillee: bool,
    /// L'utilisateur a un code PIN ; sinon l'écran verrouillé demande son mot de passe.
    pub pin_defini: bool,
}

/// Codes PIN faux tolérés sur une session verrouillée avant de la fermer : au-delà, un PIN de
/// quatre chiffres deviendrait devinable (SPEC_PHASE_1 §7).
pub const ESSAIS_PIN: u32 = 3;

/// Durées de la politique de session, en millisecondes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PolitiqueSession {
    /// Inactivité avant verrouillage automatique (deux minutes par défaut, §6.2).
    pub verrouillage_apres_millis: i64,
    /// Inactivité avant fermeture définitive de la session.
    pub expiration_apres_millis: i64,
}

impl Default for PolitiqueSession {
    fn default() -> Self {
        Self {
            verrouillage_apres_millis: 2 * 60 * 1000,
            expiration_apres_millis: 8 * 60 * 60 * 1000,
        }
    }
}

/// Registre des sessions du poste, en mémoire : une session ne survit pas à l'application.
#[derive(Debug, Default)]
pub struct Sessions {
    sessions: Mutex<HashMap<Jeton, Session>>,
    echecs_pin: Mutex<HashMap<Jeton, u32>>,
}

impl Sessions {
    /// Enregistre une session.
    pub fn ouvrir(&self, session: Session) {
        if let Ok(mut s) = self.sessions.lock() {
            s.insert(session.jeton.clone(), session);
        }
    }

    /// Applique la politique à l'instant donné : verrouille les sessions inactives, ferme les
    /// sessions expirées. Retourne les sessions fermées.
    pub fn appliquer(&self, politique: PolitiqueSession, maintenant: Horodatage) -> Vec<Session> {
        let Ok(mut s) = self.sessions.lock() else {
            return Vec::new();
        };
        let mut fermees = Vec::new();
        s.retain(|_, session| {
            let inactivite = session
                .derniere_activite
                .difference_millis(&maintenant)
                .unwrap_or(i64::MAX);
            if inactivite >= politique.expiration_apres_millis {
                fermees.push(session.clone());
                return false;
            }
            if inactivite >= politique.verrouillage_apres_millis {
                session.verrouillee = true;
            }
            true
        });
        fermees
    }

    /// Lit une session, verrouillée ou non.
    pub fn lire(&self, jeton: &Jeton) -> Option<Session> {
        self.sessions
            .lock()
            .ok()
            .and_then(|s| s.get(jeton).cloned())
    }

    /// Marque une activité : rafraîchit l'horodatage, sauf si la session est verrouillée.
    pub fn toucher(&self, jeton: &Jeton, maintenant: Horodatage) -> Option<Session> {
        let mut s = self.sessions.lock().ok()?;
        let session = s.get_mut(jeton)?;
        if !session.verrouillee {
            session.derniere_activite = maintenant;
        }
        Some(session.clone())
    }

    /// Verrouille la session.
    pub fn verrouiller(&self, jeton: &Jeton) -> Option<Session> {
        let mut s = self.sessions.lock().ok()?;
        let session = s.get_mut(jeton)?;
        session.verrouillee = true;
        Some(session.clone())
    }

    /// Déverrouille la session après vérification du PIN par l'appelant.
    pub fn deverrouiller(&self, jeton: &Jeton, maintenant: Horodatage) -> Option<Session> {
        let mut s = self.sessions.lock().ok()?;
        let session = s.get_mut(jeton)?;
        session.verrouillee = false;
        session.derniere_activite = maintenant;
        Some(session.clone())
    }

    /// Ferme la session.
    pub fn fermer(&self, jeton: &Jeton) -> Option<Session> {
        self.effacer_echecs_pin(jeton);
        self.sessions.lock().ok()?.remove(jeton)
    }

    /// Compte un code PIN faux et rend le total pour cette session.
    pub fn noter_echec_pin(&self, jeton: &Jeton) -> u32 {
        self.echecs_pin.lock().map_or(ESSAIS_PIN, |mut e| {
            let n = e.entry(jeton.clone()).or_insert(0);
            *n = n.saturating_add(1);
            *n
        })
    }

    /// Remet à zéro les PIN faux d'une session rouverte ou fermée.
    pub fn effacer_echecs_pin(&self, jeton: &Jeton) {
        if let Ok(mut e) = self.echecs_pin.lock() {
            e.remove(jeton);
        }
    }

    /// Note qu'un utilisateur vient de définir son PIN, sur toutes ses sessions.
    pub fn marquer_pin_defini(&self, utilisateur_id: Id<Utilisateur>) {
        if let Ok(mut s) = self.sessions.lock() {
            s.values_mut()
                .filter(|x| x.utilisateur_id == utilisateur_id)
                .for_each(|x| x.pin_defini = true);
        }
    }

    /// Ferme toutes les sessions d'un utilisateur : compte désactivé ou mot de passe réinitialisé.
    pub fn fermer_de(&self, utilisateur_id: Id<Utilisateur>) -> Vec<Session> {
        self.sessions.lock().map_or_else(
            |_| Vec::new(),
            |mut s| {
                let jetons: Vec<Jeton> = s
                    .values()
                    .filter(|x| x.utilisateur_id == utilisateur_id)
                    .map(|x| x.jeton.clone())
                    .collect();
                jetons.iter().filter_map(|j| s.remove(j)).collect()
            },
        )
    }

    /// Sessions présentes sur le poste : c'est la liste de l'écran verrouillé (bascule rapide).
    pub fn lister(&self) -> Vec<Session> {
        self.sessions
            .lock()
            .map(|s| {
                let mut v: Vec<Session> = s.values().cloned().collect();
                v.sort_by_key(|s| std::cmp::Reverse(s.derniere_activite));
                v
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gesco_db::nouvel_id;

    fn session(jeton: &str, activite: i64) -> Session {
        Session {
            jeton: Jeton(jeton.to_owned()),
            utilisateur_id: nouvel_id(),
            nom_affiche: "Awa".to_owned(),
            etablissement_id: nouvel_id(),
            role_actif: Some("secretaire".to_owned()),
            ouverte_le: Horodatage::depuis_millis_epoque(0),
            derniere_activite: Horodatage::depuis_millis_epoque(activite),
            verrouillee: false,
            pin_defini: true,
        }
    }

    #[test]
    fn verrouillage_puis_expiration_par_inactivite() {
        let sessions = Sessions::default();
        sessions.ouvrir(session("a", 0));
        let politique = PolitiqueSession {
            verrouillage_apres_millis: 100,
            expiration_apres_millis: 1000,
        };
        assert!(
            sessions
                .appliquer(politique, Horodatage::depuis_millis_epoque(50))
                .is_empty()
        );
        assert!(
            !sessions
                .lire(&Jeton("a".to_owned()))
                .map(|s| s.verrouillee)
                .unwrap_or(true)
        );
        sessions.appliquer(politique, Horodatage::depuis_millis_epoque(100));
        assert!(
            sessions
                .lire(&Jeton("a".to_owned()))
                .map(|s| s.verrouillee)
                .unwrap_or(false)
        );
        let fermees = sessions.appliquer(politique, Horodatage::depuis_millis_epoque(1000));
        assert_eq!(fermees.len(), 1);
        assert!(sessions.lire(&Jeton("a".to_owned())).is_none());
    }

    #[test]
    fn toucher_ne_rafraichit_pas_une_session_verrouillee() {
        let sessions = Sessions::default();
        sessions.ouvrir(session("a", 0));
        sessions.verrouiller(&Jeton("a".to_owned()));
        let s = sessions.toucher(
            &Jeton("a".to_owned()),
            Horodatage::depuis_millis_epoque(500),
        );
        assert_eq!(s.map(|s| s.derniere_activite.millis_epoque()), Some(0));
        let s = sessions.deverrouiller(
            &Jeton("a".to_owned()),
            Horodatage::depuis_millis_epoque(600),
        );
        assert_eq!(
            s.map(|s| (s.verrouillee, s.derniere_activite.millis_epoque())),
            Some((false, 600))
        );
    }

    #[test]
    fn la_liste_de_l_ecran_verrouille_est_triee_par_activite_recente() {
        let sessions = Sessions::default();
        sessions.ouvrir(session("ancienne", 10));
        sessions.ouvrir(session("recente", 20));
        let noms: Vec<String> = sessions.lister().into_iter().map(|s| s.jeton.0).collect();
        assert_eq!(noms, vec!["recente".to_owned(), "ancienne".to_owned()]);
    }
}
