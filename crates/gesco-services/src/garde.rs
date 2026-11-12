//! Garde de permission : point de passage unique entre la frontière et les services (R-21).
//! Trois dimensions, vérifiées côté Rust uniquement (R-22) :
//! `permission ∧ portée_établissement ∧ portée_ressource`.

use std::sync::Arc;

use gesco_core::commun::Id;
use gesco_core::core::Utilisateur;

use crate::commandes::{Acces, Commande};
use crate::erreur::{ErreurService, MotifRefus, Resultat};
use crate::noyau::Noyau;
use crate::session::{Jeton, Session};

/// Ressource visée par une commande, pour la portée ressource (§11.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ressource {
    /// Type de ressource (`classe`, `matiere`…).
    pub type_ressource: &'static str,
    /// Identifiant de la ressource.
    pub id: String,
}

/// Décide si un utilisateur a la portée sur une ressource. Point d'extension de la phase 2 :
/// `Scolarite::classes_autorisees(utilisateur)` en sera l'implémentation.
pub trait PorteeRessource: Send + Sync {
    /// `true` si l'utilisateur peut agir sur la ressource.
    fn autorise(&self, utilisateur_id: Id<Utilisateur>, ressource: &Ressource) -> bool;
}

/// Portée de la phase 0.5 : aucune ressource n'existe encore, tout est accordé (§7.2).
#[derive(Debug, Default, Clone, Copy)]
pub struct PorteeTotale;

impl PorteeRessource for PorteeTotale {
    fn autorise(&self, _utilisateur_id: Id<Utilisateur>, _ressource: &Ressource) -> bool {
        true
    }
}

/// Garde adossée au noyau.
pub struct Garde<'n> {
    noyau: &'n Noyau,
    portee: Arc<dyn PorteeRessource>,
}

impl<'n> Garde<'n> {
    /// Garde avec la portée ressource donnée.
    pub fn nouvelle(noyau: &'n Noyau, portee: Arc<dyn PorteeRessource>) -> Self {
        Self { noyau, portee }
    }

    /// Autorise l'appel d'une commande par le porteur d'un jeton. Retourne la session rafraîchie.
    pub async fn autoriser(
        &self,
        commande: &Commande,
        jeton: Option<&Jeton>,
        ressource: Option<&Ressource>,
    ) -> Resultat<Option<Session>> {
        let Acces::Public = commande.acces else {
            let jeton =
                jeton.ok_or(ErreurService::Authentification(MotifRefus::SessionInvalide))?;
            let session = self.session_active(jeton)?;
            let contexte = self
                .noyau
                .contexte(session.etablissement_id, Some(&session));
            let droits = self
                .noyau
                .droits_de(session.utilisateur_id, &contexte)
                .await?;
            if droits.maitre {
                // Le réglage éteint ferme la session d'un maître à sa commande suivante ; allumé,
                // il ne demande que ce que la liste lui ouvre (SPEC_PHASE_11_A §7).
                if !droits.maitres_admis {
                    self.noyau.sessions().fermer(jeton);
                    return Err(ErreurService::Authentification(MotifRefus::SessionInvalide));
                }
                if !commande.du_maitre() {
                    return Err(ErreurService::Regle {
                        cle: crate::maitres::REFUS_COMMANDE,
                    });
                }
            }
            if let Acces::Permission(permission) = commande.acces {
                if !droits.permissions.contains(&permission) {
                    return Err(ErreurService::permission(permission));
                }
                if let Some(ressource) = ressource
                    && !self.portee.autorise(session.utilisateur_id, ressource)
                {
                    return Err(ErreurService::PorteeRefusee {
                        ressource: format!("{}:{}", ressource.type_ressource, ressource.id),
                    });
                }
            }
            return Ok(Some(session));
        };
        Ok(None)
    }

    /// Session ouverte et non verrouillée, activité rafraîchie.
    pub fn session_active(&self, jeton: &Jeton) -> Resultat<Session> {
        let maintenant = self.noyau.horloge().maintenant();
        let session = self
            .noyau
            .sessions()
            .toucher(jeton, maintenant)
            .ok_or(ErreurService::Authentification(MotifRefus::SessionInvalide))?;
        if session.verrouillee {
            return Err(ErreurService::Authentification(
                MotifRefus::SessionVerrouillee,
            ));
        }
        Ok(session)
    }
}
