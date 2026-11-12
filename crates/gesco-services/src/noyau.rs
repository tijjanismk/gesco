//! Le noyau applicatif : base, horloge, identité du poste, sessions et exécuteur d'effets.
//! Les services s'y adossent ; la frontière (`gesco-app`) le détient en état partagé.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use gesco_core::commun::Id;
use gesco_core::core::{Etablissement, Permission, Poste, Utilisateur};
use gesco_db::depots::{parametre, role};
use gesco_db::unite::Durabilite;
use gesco_db::{Base, Connexion, Contexte, UniteDeTravail, nouvel_id};

use crate::effets::{Effet, ExecuteurEffets, JournalEffets};
use crate::erreur::Resultat;
use crate::horloge::{Horloge, HorlogeSysteme};
use crate::origine::{self, Origine};
use crate::session::{PolitiqueSession, Session, Sessions, SessionsDuPoste};

/// Options d'assemblage du noyau.
pub struct OptionsNoyau {
    /// Base ouverte et migrée.
    pub base: Base,
    /// Nom du poste (ADR-014 : identifie l'origine d'une écriture).
    pub poste: Poste,
    /// Empreinte machine, portée par l'audit et la licence.
    pub empreinte_machine: String,
    /// Horloge ; [`HorlogeSysteme`] en production.
    pub horloge: Arc<dyn Horloge>,
    /// Exécuteur des effets après commit.
    pub effets: Arc<dyn ExecuteurEffets>,
    /// Politique de session.
    pub politique_session: PolitiqueSession,
}

impl OptionsNoyau {
    /// Options de production sur une base donnée, avec le journal d'effets de la phase 0.5.
    pub fn production(base: Base, poste: Poste, empreinte_machine: String) -> Self {
        Self {
            base,
            poste,
            empreinte_machine,
            horloge: Arc::new(HorlogeSysteme),
            effets: Arc::new(JournalEffets::default()),
            politique_session: PolitiqueSession::default(),
        }
    }
}

/// Paramètre de l'inactivité avant verrouillage, en secondes.
pub const PARAMETRE_VERROUILLAGE: &str = "session.verrouillage_inactivite_s";
/// Paramètre de l'inactivité avant fermeture de la session, en secondes.
pub const PARAMETRE_EXPIRATION: &str = "session.expiration_s";

/// Noyau applicatif partagé.
pub struct Noyau {
    base: Base,
    poste: Poste,
    empreinte_machine: String,
    horloge: Arc<dyn Horloge>,
    effets: Arc<dyn ExecuteurEffets>,
    sessions: Sessions,
    politique_session: std::sync::RwLock<PolitiqueSession>,
    permissions: PermissionsGardees,
}

/// Permissions des utilisateurs, gardées en mémoire entre deux écritures. Toute unité de travail
/// validée les oublie : un rôle modifié, retiré, un compte désactivé prennent effet à la commande
/// suivante, comme quand elles étaient relues à chaque fois.
#[derive(Default)]
struct PermissionsGardees {
    /// Compte des écritures validées : une lecture commencée avant une écriture n'est pas gardée.
    ecritures: AtomicU64,
    lues: Mutex<HashMap<CleDePermissions, DroitsDuCompte>>,
}

/// Ce que la garde retient d'un compte entre deux écritures.
#[derive(Debug, Clone)]
pub struct DroitsDuCompte {
    /// Permissions du compte.
    pub permissions: Arc<Vec<Permission>>,
    /// Compte de maître : il ne porte que le rôle Enseignant (SPEC_PHASE_11_A §7).
    pub maitre: bool,
    /// L'école a ouvert la saisie des notes aux maîtres.
    pub maitres_admis: bool,
}

/// Établissement et utilisateur dont les permissions sont gardées.
type CleDePermissions = (String, String);

impl PermissionsGardees {
    fn oublier(&self) {
        self.ecritures.fetch_add(1, Ordering::SeqCst);
        if let Ok(mut lues) = self.lues.lock() {
            lues.clear();
        }
    }
}

impl Noyau {
    /// Assemble le noyau.
    pub fn assembler(options: OptionsNoyau) -> Self {
        Self {
            base: options.base,
            poste: options.poste,
            empreinte_machine: options.empreinte_machine,
            horloge: options.horloge,
            effets: options.effets,
            sessions: Sessions::default(),
            politique_session: std::sync::RwLock::new(options.politique_session),
            permissions: PermissionsGardees::default(),
        }
    }

    /// Permissions d'un utilisateur dans l'établissement du contexte. Lues en base la première
    /// fois, puis gardées jusqu'à la prochaine écriture validée, quelle qu'elle soit.
    pub async fn permissions_de(
        &self,
        cnx: &mut Connexion,
        utilisateur_id: Id<Utilisateur>,
        contexte: &Contexte,
    ) -> Resultat<Arc<Vec<Permission>>> {
        Ok(self
            .droits_lus(Some(cnx), utilisateur_id, contexte)
            .await?
            .permissions)
    }

    /// Droits d'un compte : ses permissions, et s'il est un compte de maître que l'école admet
    /// ou non. Gardés comme les permissions ; la base n'est lue qu'à la première demande.
    pub async fn droits_de(
        &self,
        utilisateur_id: Id<Utilisateur>,
        contexte: &Contexte,
    ) -> Resultat<DroitsDuCompte> {
        self.droits_lus(None, utilisateur_id, contexte).await
    }

    async fn droits_lus(
        &self,
        cnx: Option<&mut Connexion>,
        utilisateur_id: Id<Utilisateur>,
        contexte: &Contexte,
    ) -> Resultat<DroitsDuCompte> {
        let cle = (
            contexte.etablissement_id.vers_chaine(),
            utilisateur_id.vers_chaine(),
        );
        if let Some(gardes) = self
            .permissions
            .lues
            .lock()
            .ok()
            .and_then(|lues| lues.get(&cle).cloned())
        {
            return Ok(gardes);
        }
        let avant = self.permissions.ecritures.load(Ordering::SeqCst);
        let lus = match cnx {
            Some(cnx) => Self::lire_droits(cnx, utilisateur_id, contexte).await?,
            None => {
                let mut cnx = self
                    .base
                    .pool()
                    .acquire()
                    .await
                    .map_err(gesco_db::ErreurBase::from)?;
                Self::lire_droits(&mut cnx, utilisateur_id, contexte).await?
            }
        };
        if let Ok(mut gardes) = self.permissions.lues.lock()
            && self.permissions.ecritures.load(Ordering::SeqCst) == avant
        {
            gardes.insert(cle, lus.clone());
        }
        Ok(lus)
    }

    async fn lire_droits(
        cnx: &mut Connexion,
        utilisateur_id: Id<Utilisateur>,
        contexte: &Contexte,
    ) -> Resultat<DroitsDuCompte> {
        let permissions = Arc::new(role::permissions_de(cnx, utilisateur_id, contexte).await?);
        let maitre =
            crate::maitres::est_maitre(&role::roles_de(cnx, utilisateur_id, contexte).await?);
        // Le réglage ne se lit que pour un compte de maître : lui seul en dépend.
        let maitres_admis = maitre
            && parametre::booleen_param(cnx, crate::maitres::PARAMETRE_SAISIE, false, contexte)
                .await?;
        Ok(DroitsDuCompte {
            permissions,
            maitre,
            maitres_admis,
        })
    }

    /// Base de données.
    pub fn base(&self) -> &Base {
        &self.base
    }

    /// Ce poste : celui qui détient la base.
    pub fn poste(&self) -> &Poste {
        &self.poste
    }

    /// Poste d'où vient l'appel en cours : ce poste, ou le poste relié que le serveur du réseau
    /// local a reconnu (SPEC_PHASE_11_A §3.2). C'est lui qui signe les écritures.
    pub fn poste_appelant(&self) -> Poste {
        match origine::courante() {
            Origine::Locale => self.poste.clone(),
            Origine::Relie(poste) => poste,
        }
    }

    /// Empreinte de la machine, comparée à celle de la licence.
    pub fn empreinte_machine(&self) -> &str {
        &self.empreinte_machine
    }

    /// Horloge.
    pub fn horloge(&self) -> &dyn Horloge {
        self.horloge.as_ref()
    }

    /// Sessions du poste appelant, après application de la politique d'inactivité à l'instant
    /// courant : une session ne se lit ni ne se manie que depuis le poste qui l'a ouverte.
    pub fn sessions(&self) -> SessionsDuPoste<'_> {
        self.sessions
            .appliquer(self.politique_session(), self.horloge.maintenant());
        self.sessions.du_poste(self.poste_appelant())
    }

    /// Postes d'où une session est ouverte en ce moment, sans doublon.
    pub fn postes_en_session(&self) -> Vec<Poste> {
        self.sessions
            .appliquer(self.politique_session(), self.horloge.maintenant());
        let mut postes: Vec<Poste> = self
            .sessions
            .lister()
            .into_iter()
            .map(|s| s.poste)
            .collect();
        postes.sort_by(|a, b| a.0.cmp(&b.0));
        postes.dedup();
        postes
    }

    /// Ferme les sessions ouvertes depuis un poste relié qui vient d'être révoqué ; rend leur
    /// nombre.
    pub fn fermer_sessions_du_poste(&self, poste: &Poste) -> usize {
        self.sessions.fermer_du_poste(poste).len()
    }

    /// Politique de session en vigueur.
    pub fn politique_session(&self) -> PolitiqueSession {
        self.politique_session
            .read()
            .map_or_else(|e| *e.into_inner(), |p| *p)
    }

    /// Remplace la politique de session, d'après les paramètres de l'établissement.
    pub fn regler_politique_session(&self, politique: PolitiqueSession) {
        match self.politique_session.write() {
            Ok(mut p) => *p = politique,
            Err(e) => *e.into_inner() = politique,
        }
    }

    /// Lit la politique de session dans les paramètres de l'établissement (en secondes) et
    /// l'applique : sans elle, les durées réglées par l'école resteraient lettre morte.
    pub async fn charger_politique_session(
        &self,
        etablissement_id: Id<Etablissement>,
    ) -> Resultat<()> {
        let defaut = PolitiqueSession::default();
        let contexte = self.contexte(etablissement_id, None);
        let mut cnx = self
            .base
            .pool()
            .acquire()
            .await
            .map_err(gesco_db::ErreurBase::from)?;
        let secondes = |millis: i64| millis / 1000;
        let verrouillage = gesco_db::depots::parametre::entier(
            &mut cnx,
            PARAMETRE_VERROUILLAGE,
            secondes(defaut.verrouillage_apres_millis),
            &contexte,
        )
        .await?;
        let expiration = gesco_db::depots::parametre::entier(
            &mut cnx,
            PARAMETRE_EXPIRATION,
            secondes(defaut.expiration_apres_millis),
            &contexte,
        )
        .await?;
        self.regler_politique_session(PolitiqueSession {
            verrouillage_apres_millis: verrouillage.saturating_mul(1000),
            expiration_apres_millis: expiration.saturating_mul(1000),
        });
        Ok(())
    }

    /// Contexte d'exécution pour un utilisateur (ou pour le système quand `session` est `None`).
    pub fn contexte(
        &self,
        etablissement_id: Id<Etablissement>,
        session: Option<&Session>,
    ) -> Contexte {
        Contexte {
            etablissement_id,
            utilisateur_id: session.map(|s| s.utilisateur_id),
            role_actif: session.and_then(|s| s.role_actif.clone()),
            poste: self.poste_appelant(),
            empreinte_machine: self.empreinte_machine.clone(),
            maintenant: self.horloge.maintenant(),
            correlation_id: nouvel_id::<Utilisateur>().vers_chaine(),
        }
    }

    /// Ouvre une unité de travail.
    pub async fn commencer(
        &self,
        contexte: Contexte,
        durabilite: Durabilite,
    ) -> Resultat<UniteDeTravail<Effet>> {
        Ok(UniteDeTravail::commencer(&self.base, contexte, durabilite).await?)
    }

    /// Valide l'unité de travail puis exécute ses effets, strictement après le commit (R-18).
    /// Retourne le nombre d'effets exécutés sans erreur.
    pub async fn terminer(&self, unite: UniteDeTravail<Effet>) -> Resultat<usize> {
        let contexte = unite.contexte().clone();
        let effets = unite.valider().await;
        // Validée ou non, l'écriture a pu toucher un rôle ou un compte : les permissions gardées
        // se relisent.
        self.permissions.oublier();
        let effets = effets?;
        let mut reussis = 0;
        for effet in effets {
            if self.effets.executer(&contexte, effet).is_ok() {
                reussis += 1;
            }
        }
        Ok(reussis)
    }
}
