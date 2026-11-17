//! Portail des élèves sur le Wi-Fi de l'école (SPEC_PHASE_12_LOT_4) : ses réglages, ce que le
//! serveur du portail doit servir, et ce que la direction en voit. Le serveur lui-même est dans
//! `gesco-reseau` ; il n'appelle que les fonctions écrites ici pour lui, sous
//! [`crate::Origine::Portail`], jamais une commande de gestion.

pub mod codes;
pub mod entree;

use std::sync::Mutex;

use gesco_core::bibliotheque::portail::{
    self as regles, DUREE_SESSION_DEFAUT, GrainSuivi, adresse_a_donner,
};
use gesco_core::bibliotheque::reglages::SUIVI_ELEVES;
use gesco_core::commun::Id;
use gesco_core::core::parametre::hors_bornes;
use gesco_core::core::{Etablissement, Operation, Parametre, ValeurParametre};
use gesco_db::depots::{parametre, reseau};
use gesco_db::unite::Durabilite;
use gesco_db::{Connexion, Contexte};
use serde::{Deserialize, Serialize};
use tokio::sync::watch;

use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;
use crate::session::Session;

/// Réglages du portail des élèves, tels que la direction les voit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ReglagesPortail {
    /// « Les élèves lisent les cours sur leurs appareils ».
    pub eleves: bool,
    /// Suivi des élèves par leurs maîtres.
    pub suivi: bool,
    /// Ce que le maître voit du suivi : `eleve` ou `classe`.
    pub grain: String,
    /// Minutes sans activité avant qu'un élève doive entrer de nouveau.
    pub duree_session_min: i64,
    /// Les vidéos se regardent sur les appareils des élèves.
    pub videos: bool,
    /// Le PDF d'une leçon s'enregistre depuis le portail.
    pub pdf: bool,
}

/// Ce que la direction voit du portail : ses réglages, ce qu'il faut allumer d'abord, et
/// l'adresse à donner aux élèves quand il écoute.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct EtatPortail {
    /// Réglages.
    pub reglages: ReglagesPortail,
    /// Le serveur du réseau local est allumé.
    pub reseau_allume: bool,
    /// Le module Bibliothèque est allumé.
    pub module_allume: bool,
    /// Port écouté en ce moment ; aucun quand le portail n'écoute pas.
    pub port: Option<u16>,
    /// Adresses à donner aux élèves (`http://…`), la première d'abord.
    pub adresses: Vec<String>,
}

/// Ce que le portail vit pendant que l'application tourne : le port écouté, et le signal qui dit
/// au serveur de relire s'il doit servir.
#[derive(Debug)]
pub struct PortailVivant {
    ecoute: Mutex<Option<u16>>,
    signal: watch::Sender<u64>,
    /// Sessions des élèves entrés.
    pub sessions: entree::SessionsPortail,
    /// Vérifications de code menées en même temps, au plus.
    pub verifications: tokio::sync::Semaphore,
}

impl Default for PortailVivant {
    fn default() -> Self {
        let (signal, _) = watch::channel(0);
        Self {
            ecoute: Mutex::new(None),
            signal,
            sessions: entree::SessionsPortail::default(),
            verifications: tokio::sync::Semaphore::new(
                gesco_core::bibliotheque::portail::VERIFICATIONS_SIMULTANEES,
            ),
        }
    }
}

impl PortailVivant {
    /// Suit les changements des réglages du portail.
    pub fn suivre(&self) -> watch::Receiver<u64> {
        self.signal.subscribe()
    }

    /// Dit au serveur du portail de relire s'il doit servir.
    pub fn signaler(&self) {
        self.signal.send_modify(|n| *n = n.wrapping_add(1));
    }

    /// Retient le port écouté, ou qu'il n'écoute plus.
    pub fn declarer_ecoute(&self, port: Option<u16>) {
        if let Ok(mut e) = self.ecoute.lock() {
            *e = port;
        }
    }

    /// Port écouté en ce moment.
    pub fn port(&self) -> Option<u16> {
        self.ecoute.lock().ok().and_then(|e| *e)
    }
}

/// Service du portail des élèves.
pub struct ServicePortail<'a> {
    noyau: &'a Noyau,
}

impl<'a> ServicePortail<'a> {
    /// Service sur ce noyau.
    pub fn nouveau(noyau: &'a Noyau) -> Self {
        Self { noyau }
    }

    async fn connexion(&self) -> Resultat<gesco_db::ConnexionEmpruntee> {
        Ok(self
            .noyau
            .base()
            .pool()
            .acquire()
            .await
            .map_err(gesco_db::ErreurBase::from)?)
    }

    /// Réglages du portail, lus dans la base.
    pub(crate) async fn lire_reglages(
        cnx: &mut Connexion,
        contexte: &Contexte,
    ) -> Resultat<ReglagesPortail> {
        let grain = parametre::texte(cnx, regles::SUIVI_GRAIN, "eleve", contexte).await?;
        Ok(ReglagesPortail {
            eleves: parametre::booleen_param(cnx, regles::ELEVES, false, contexte).await?,
            suivi: parametre::booleen_param(cnx, SUIVI_ELEVES, true, contexte).await?,
            grain: GrainSuivi::depuis_nom(&grain)
                .unwrap_or_default()
                .nom()
                .to_owned(),
            duree_session_min: parametre::entier(
                cnx,
                regles::DUREE_SESSION_MIN,
                DUREE_SESSION_DEFAUT,
                contexte,
            )
            .await?,
            videos: parametre::booleen_param(cnx, regles::VIDEOS, true, contexte).await?,
            pdf: parametre::booleen_param(cnx, regles::PDF, true, contexte).await?,
        })
    }

    /// Vrai quand le portail doit écouter : la direction l'a allumé, le module Bibliothèque est
    /// allumé et le serveur du réseau local aussi.
    pub async fn a_servir(&self, etablissement_id: Id<Etablissement>) -> Resultat<bool> {
        let contexte = self.noyau.contexte(etablissement_id, None);
        let mut cnx = self.connexion().await?;
        let eleves = parametre::booleen_param(&mut cnx, regles::ELEVES, false, &contexte).await?;
        let module =
            parametre::booleen_param(&mut cnx, "module.bibliotheque", true, &contexte).await?;
        let reseau = reseau::lire(&mut cnx, &contexte).await?.actif;
        Ok(eleves && module && reseau)
    }

    /// État du portail pour l'écran de la direction ; `adresses` sont les adresses de ce poste
    /// sur le réseau de l'école.
    pub async fn etat(
        &self,
        session: &Session,
        port: Option<u16>,
        adresses: &[String],
    ) -> Resultat<EtatPortail> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let reglages = Self::lire_reglages(&mut cnx, &contexte).await?;
        let module_allume =
            parametre::booleen_param(&mut cnx, "module.bibliotheque", true, &contexte).await?;
        let reseau_allume = reseau::lire(&mut cnx, &contexte).await?.actif;
        let adresses = match port {
            Some(port) => adresses
                .iter()
                .map(|hote| adresse_a_donner(hote, port))
                .collect(),
            None => Vec::new(),
        };
        Ok(EtatPortail {
            reglages,
            reseau_allume,
            module_allume,
            port,
            adresses,
        })
    }

    /// Règle le portail. Chaque réglage changé est écrit et tracé ; allumer ou éteindre le
    /// portail se lit au journal comme tel.
    pub async fn regler(
        &self,
        session: &Session,
        demande: &ReglagesPortail,
    ) -> Resultat<ReglagesPortail> {
        let voulus = [
            (regles::ELEVES, ValeurParametre::Booleen(demande.eleves)),
            (SUIVI_ELEVES, ValeurParametre::Booleen(demande.suivi)),
            (
                regles::SUIVI_GRAIN,
                ValeurParametre::Texte(demande.grain.clone()),
            ),
            (
                regles::DUREE_SESSION_MIN,
                ValeurParametre::Entier(demande.duree_session_min),
            ),
            (regles::VIDEOS, ValeurParametre::Booleen(demande.videos)),
            (regles::PDF, ValeurParametre::Booleen(demande.pdf)),
        ];
        if voulus.iter().any(|(cle, valeur)| hors_bornes(cle, valeur)) {
            return Err(ErreurService::Regle {
                cle: "parametre.hors_bornes",
            });
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        for (cle, valeur) in voulus {
            let avant = parametre::lire(unite.connexion(), cle, &contexte).await?;
            if avant.as_ref().map(|p| &p.valeur) == Some(&valeur) {
                continue;
            }
            let action = match (cle, &valeur) {
                (regles::ELEVES, ValeurParametre::Booleen(true)) => "portail.allumer",
                (regles::ELEVES, _) => "portail.eteindre",
                (regles::SUIVI_GRAIN | SUIVI_ELEVES, _) => "portail.suivi_regler",
                _ => "portail.regler",
            };
            parametre::definir(
                unite.connexion(),
                &Parametre {
                    etablissement_id: session.etablissement_id,
                    cle: cle.to_owned(),
                    valeur: valeur.clone(),
                    modifiable_par_utilisateur: true,
                },
                &contexte,
            )
            .await?;
            unite.auditer(
                "parametre",
                cle,
                Operation::Modification,
                avant.map(|p| serde_json::json!({ "valeur": p.valeur })),
                Some(serde_json::json!({ "valeur": valeur })),
                action,
            );
        }
        let reglages = Self::lire_reglages(unite.connexion(), &contexte).await?;
        self.noyau.terminer(unite).await?;
        Ok(reglages)
    }
}
