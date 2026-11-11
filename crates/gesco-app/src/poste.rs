//! Rôle du poste (SPEC_PHASE_11_A §2 et §5). Un poste a sa base et répond à son écran, ou il
//! est **relié** : il n'a ni base ni clé de données et transmet chaque commande au poste
//! principal de l'école.
//!
//! Le rôle vit dans un réglage du poste, `poste.json`, à côté de la base et hors d'elle ; ce
//! que le poste retient du poste principal (adresse, empreinte complète, identifiant, secret)
//! est rangé dans le trousseau de Windows. Entre seul et principal, c'est la base qui tranche :
//! le poste est principal quand le réseau de l'école y est allumé.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gesco_core::core::Poste;
use gesco_reseau::relais::{self, Apercu, Liaison, Relais};
use gesco_services::cles::Trousseau;
use gesco_services::session::{Jeton, PolitiqueSession};
use gesco_services::{
    Application, ErreurCommande, EspaceDonnees, Horloge, HorlogeSysteme, OptionsApplication,
    Resultat,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::async_runtime::JoinHandle;
use tokio::sync::RwLock;

use crate::trousseau::CoffreLiaison;

/// Réglage du poste, à la racine de son espace de données.
const FICHIER_REGLAGE: &str = "poste.json";

/// Rôle écrit dans le réglage d'un poste relié. Sans réglage, le poste a sa base.
const ROLE_RELIE: &str = "relie";

/// Commande de l'écran qui note une erreur au journal technique : sur un poste relié, elle
/// s'écrit dans le journal de ce poste, pas dans celui du poste principal.
const COMMANDE_JOURNAL: &str = "journal_technique_noter";

/// Refus qui tiennent à la liaison elle-même : ils se notent au journal technique du poste
/// relié, une fois par changement d'état.
const CLES_DE_LIAISON: &[&str] = &[
    relais::CLE_INJOIGNABLE,
    relais::CLE_EMPREINTE_DIFFERENTE,
    gesco_reseau::CLE_VERSION_DIFFERENTE,
    gesco_reseau::CLE_POSTE_INCONNU,
    gesco_reseau::CLE_TROP_D_ESSAIS,
];

#[derive(Serialize, Deserialize)]
struct Reglage {
    role: String,
    /// Nom donné au poste à l'appairage, tel que le poste principal le connaît.
    nom: String,
}

/// Ce que l'écran sait du poste, avant même toute session.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct InfoPoste {
    /// Vrai pour un poste relié.
    relie: bool,
    /// Nom du poste : celui de l'appairage pour un poste relié, celui de l'ordinateur sinon.
    nom: String,
    /// Nom de l'ordinateur, proposé comme nom de poste.
    nom_ordinateur: String,
    /// Adresse du poste principal, pour un poste relié.
    adresse: Option<String>,
    /// Version de GESCO sur ce poste.
    version: String,
}

/// Ce que l'écran montre du poste principal avant de s'y relier.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct ApercuAffiche {
    adresse: String,
    version: String,
    /// À comparer à l'œil avec l'écran « Postes de l'école » du poste principal.
    empreinte_courte: String,
    /// Faux quand le poste principal est d'une autre version : le poste ne se reliera pas.
    meme_version: bool,
}

/// Ce qu'il faut pour démarrer GESCO sur ce poste, dans l'un ou l'autre rôle.
pub(crate) struct Demarrage {
    pub(crate) racine: PathBuf,
    pub(crate) en_essai: bool,
    pub(crate) nom_ordinateur: String,
    pub(crate) version: String,
    pub(crate) trousseau: Arc<dyn Trousseau>,
    pub(crate) coffre: Arc<dyn CoffreLiaison>,
    pub(crate) horloge: Arc<dyn Horloge>,
    pub(crate) cle_licence: [u8; 32],
}

enum Mode {
    /// Le poste a sa base : seul ou principal.
    Local {
        application: Arc<Application>,
        taches: Vec<JoinHandle<()>>,
    },
    /// Le poste transmet tout au poste principal.
    Relie { relais: Arc<Relais>, nom: String },
}

/// Le poste et son rôle du moment.
pub(crate) struct EtatPoste {
    demarrage: Demarrage,
    mode: RwLock<Mode>,
    /// Poste principal aperçu, dont l'empreinte attend d'être comparée puis acceptée.
    apercu: Mutex<Option<Apercu>>,
    /// Dernier refus de la liaison noté au journal ; `None` quand elle marche.
    liaison: Mutex<Option<String>>,
    /// La personne a confirmé la fermeture malgré les postes connectés.
    fermeture_confirmee: AtomicBool,
}

fn erreur(cle: &str) -> ErreurCommande {
    ErreurCommande {
        cle: cle.to_owned(),
        jusqua: None,
        detail: None,
        modifiee: None,
    }
}

/// Sauvegarde planifiée et test de restauration mensuel, vérifiés chaque minute (§11.1, §11.5).
async fn taches_periodiques(application: Arc<Application>) {
    let mut minute = tokio::time::interval(Duration::from_secs(60));
    loop {
        minute.tick().await;
        application.taches_periodiques().await;
    }
}

/// File des SMS, toutes les cinq secondes (SPEC_PHASE_5 §3) : l'utilisateur n'attend jamais
/// l'envoi. Un échec se retente au passage suivant.
async fn file_sms(application: Arc<Application>) {
    let mut tic = tokio::time::interval(Duration::from_secs(5));
    loop {
        tic.tick().await;
        let _ = application.traiter_file_sms().await;
    }
}

/// Serveur du réseau local (SPEC_PHASE_11_A §3.1) : il écoute dès que la direction allume le
/// réseau et s'arrête quand elle l'éteint. En mode d'essai, il n'écoute que la machine elle-même :
/// les essais de bout en bout ne font pas paraître la question du pare-feu de Windows.
fn servir_le_reseau(application: &Arc<Application>, poste: String, essai: bool) {
    use std::net::{Ipv4Addr, SocketAddr};
    application.declarer_adresses_du_poste(Arc::new(move || {
        if essai {
            vec![Ipv4Addr::LOCALHOST.to_string()]
        } else {
            gesco_reseau::adresses_du_poste(&poste)
        }
    }));
    let ou = if essai {
        Ipv4Addr::LOCALHOST
    } else {
        Ipv4Addr::UNSPECIFIED
    };
    let application = Arc::clone(application);
    // Le superviseur lance sa tâche : il lui faut le moteur asynchrone autour de lui.
    tauri::async_runtime::spawn(async move {
        gesco_reseau::superviser(application, move |port| SocketAddr::from((ou, port)));
    });
}

impl Demarrage {
    fn reglage(&self) -> PathBuf {
        self.racine.join(FICHIER_REGLAGE)
    }

    fn noter(&self, ligne: &str) {
        gesco_services::journal_technique::noter(&self.racine, HorlogeSysteme.maintenant(), ligne);
    }

    /// Liaison d'un poste relié : le réglage dit « relié » et le trousseau garde la liaison.
    fn liaison(&self) -> Option<(Liaison, String)> {
        let reglage: Reglage =
            serde_json::from_str(&std::fs::read_to_string(self.reglage()).ok()?).ok()?;
        if reglage.role != ROLE_RELIE {
            return None;
        }
        match self.coffre.lire() {
            Ok(Some(contenu)) => match serde_json::from_str(&contenu) {
                Ok(liaison) => Some((liaison, reglage.nom)),
                Err(_) => {
                    self.noter("poste relie : liaison illisible dans le trousseau");
                    None
                }
            },
            Ok(None) => {
                self.noter("poste relie : aucune liaison dans le trousseau");
                None
            }
            Err(e) => {
                self.noter(&format!("poste relie : trousseau inaccessible detail={e}"));
                None
            }
        }
    }

    /// Démarre le poste avec sa base : le noyau, ses tâches de fond et le serveur du réseau.
    async fn local(&self) -> Resultat<Mode> {
        let options = OptionsApplication {
            racine: self.racine.clone(),
            empreinte_machine: crate::empreinte::du_poste(&self.nom_ordinateur),
            poste: Poste(self.nom_ordinateur.clone()),
            trousseau: Arc::clone(&self.trousseau),
            horloge: Arc::clone(&self.horloge),
            politique_session: PolitiqueSession::default(),
            version_applicative: self.version.clone(),
            cle_licence: self.cle_licence,
        };
        let application = Arc::new(Application::demarrer(options).await?);
        servir_le_reseau(&application, self.nom_ordinateur.clone(), self.en_essai);
        let taches = vec![
            tauri::async_runtime::spawn(taches_periodiques(Arc::clone(&application))),
            tauri::async_runtime::spawn(file_sms(Arc::clone(&application))),
        ];
        Ok(Mode::Local {
            application,
            taches,
        })
    }

    /// Démarre le poste relié : aucun noyau, aucune base, seulement le relais.
    fn relie(&self, liaison: Liaison, nom: String) -> Result<Mode, ErreurCommande> {
        std::fs::create_dir_all(&self.racine).map_err(|e| ErreurCommande {
            cle: "erreur.interne".to_owned(),
            jusqua: None,
            detail: Some(e.to_string()),
            modifiee: None,
        })?;
        self.noter(&format!(
            "demarrage version={} role=relie poste={nom} principal={}",
            self.version, liaison.adresse
        ));
        let relais = Relais::nouveau(liaison, &self.version)?;
        Ok(Mode::Relie {
            relais: Arc::new(relais),
            nom,
        })
    }
}

/// Efface la base vide et les clés qu'un premier lancement a préparées : un poste relié ne
/// garde ni base ni clé de données. N'est appelé que pour un poste sans établissement.
fn effacer_base_vide(racine: &Path) {
    let espace = EspaceDonnees::nouveau(racine);
    let base = espace.base();
    let mut fichiers = vec![base.clone(), espace.cles()];
    for suffixe in ["-wal", "-shm"] {
        let mut nom = base.clone().into_os_string();
        nom.push(suffixe);
        fichiers.push(PathBuf::from(nom));
    }
    for fichier in fichiers {
        let _ = std::fs::remove_file(fichier);
    }
}

impl EtatPoste {
    /// Démarre le poste dans le rôle que dit son réglage.
    pub(crate) async fn demarrer(demarrage: Demarrage) -> Resultat<Self> {
        let mode = match demarrage.liaison() {
            Some((liaison, nom)) => match demarrage.relie(liaison, nom) {
                Ok(mode) => mode,
                Err(e) => {
                    demarrage.noter(&format!("poste relie : relais impossible cle={}", e.cle));
                    demarrage.local().await?
                }
            },
            None => demarrage.local().await?,
        };
        Ok(Self {
            demarrage,
            mode: RwLock::new(mode),
            apercu: Mutex::new(None),
            liaison: Mutex::new(None),
            fermeture_confirmee: AtomicBool::new(false),
        })
    }

    /// Ce que l'écran sait du poste.
    pub(crate) async fn info(&self) -> InfoPoste {
        self.info_de(&*self.mode.read().await)
    }

    fn info_de(&self, mode: &Mode) -> InfoPoste {
        let (relie, nom, adresse) = match mode {
            Mode::Local { .. } => (false, self.demarrage.nom_ordinateur.clone(), None),
            Mode::Relie { relais, nom } => (true, nom.clone(), Some(relais.adresse().to_owned())),
        };
        InfoPoste {
            relie,
            nom,
            nom_ordinateur: self.demarrage.nom_ordinateur.clone(),
            adresse,
            version: self.demarrage.version.clone(),
        }
    }

    /// Note au journal technique du poste relié un changement d'état de la liaison : coupée,
    /// refusée, rétablie. Un état qui dure ne se note qu'une fois.
    fn suivre_liaison<T>(&self, resultat: &Result<T, ErreurCommande>) {
        let refus = match resultat {
            Err(e) if CLES_DE_LIAISON.contains(&e.cle.as_str()) => Some(e),
            _ => None,
        };
        let Ok(mut dernier) = self.liaison.lock() else {
            return;
        };
        let cle = refus.map(|e| e.cle.clone());
        if *dernier == cle {
            return;
        }
        match refus {
            Some(e) => self.demarrage.noter(&format!(
                "liaison refus cle={}{}",
                e.cle,
                e.detail
                    .as_deref()
                    .map(|d| format!(" detail={d}"))
                    .unwrap_or_default()
            )),
            None => self.demarrage.noter("liaison retablie"),
        }
        *dernier = cle;
    }

    /// Exécute une commande de l'écran : sur place, ou par le poste principal.
    pub(crate) async fn executer(
        &self,
        nom: &str,
        jeton: Option<String>,
        entree: Value,
    ) -> Result<Value, ErreurCommande> {
        let relais = match &*self.mode.read().await {
            Mode::Local { application, .. } => {
                let application = Arc::clone(application);
                return application.executer(nom, jeton.map(Jeton), entree).await;
            }
            Mode::Relie { relais, .. } => Arc::clone(relais),
        };
        if nom == COMMANDE_JOURNAL {
            let message = entree
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or_default();
            self.demarrage.noter(&format!("ecran {message}"));
            return Ok(Value::Null);
        }
        let resultat = relais.executer(nom, jeton.as_deref(), entree).await;
        self.suivre_liaison(&resultat);
        resultat
    }

    /// Vérifie la liaison d'un poste relié ; toujours bonne pour un poste qui a sa base.
    pub(crate) async fn verifier_liaison(&self) -> Result<(), ErreurCommande> {
        let relais = match &*self.mode.read().await {
            Mode::Local { .. } => return Ok(()),
            Mode::Relie { relais, .. } => Arc::clone(relais),
        };
        let resultat = relais.verifier().await;
        self.suivre_liaison(&resultat);
        resultat
    }

    /// Lit la version et l'empreinte du poste principal à cette adresse, et les retient : la
    /// personne compare l'empreinte courte avant de saisir le code.
    pub(crate) async fn apercevoir(&self, adresse: &str) -> Result<ApercuAffiche, ErreurCommande> {
        let apercu = relais::apercevoir(adresse).await?;
        let affiche = ApercuAffiche {
            adresse: apercu.adresse.clone(),
            version: apercu.version.clone(),
            empreinte_courte: apercu.empreinte_courte.clone(),
            meme_version: apercu.version == self.demarrage.version,
        };
        if let Ok(mut garde) = self.apercu.lock() {
            *garde = Some(apercu);
        }
        Ok(affiche)
    }

    /// Relie ce poste au poste principal aperçu, avec le code qu'il affiche (SPEC_PHASE_11_A
    /// §2). Seulement au premier lancement : un poste qui a un établissement ne l'abandonne pas.
    pub(crate) async fn rejoindre(
        &self,
        code: &str,
        nom: &str,
    ) -> Result<InfoPoste, ErreurCommande> {
        let apercu = self
            .apercu
            .lock()
            .ok()
            .and_then(|garde| garde.clone())
            .ok_or_else(|| erreur("poste.apercu_absent"))?;
        let mut mode = self.mode.write().await;
        let Mode::Local { application, .. } = &*mode else {
            return Err(erreur("poste.deja_relie"));
        };
        let etat = application
            .executer("etat_demarrage", None, Value::Null)
            .await?;
        if etat.get("phase").and_then(Value::as_str) != Some("premier_demarrage") {
            return Err(erreur("poste.deja_un_etablissement"));
        }
        if apercu.version != self.demarrage.version {
            return Err(ErreurCommande {
                cle: gesco_reseau::CLE_VERSION_DIFFERENTE.to_owned(),
                jusqua: None,
                detail: Some(apercu.version),
                modifiee: None,
            });
        }
        let code: String = code.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
        let (liaison, conclu) =
            relais::appairer(&apercu, &self.demarrage.version, &code.to_uppercase(), nom).await?;
        let contenu = serde_json::to_string(&liaison).map_err(|e| ErreurCommande {
            cle: "erreur.interne".to_owned(),
            jusqua: None,
            detail: Some(e.to_string()),
            modifiee: None,
        })?;
        // Sans trousseau, le secret serait perdu à la fermeture : le poste ne se dit pas relié.
        self.demarrage.coffre.ecrire(&contenu).map_err(|e| {
            self.demarrage
                .noter(&format!("poste relie : trousseau refuse detail={e}"));
            erreur("poste.trousseau_inaccessible")
        })?;
        let reglage = Reglage {
            role: ROLE_RELIE.to_owned(),
            nom: conclu.nom.clone(),
        };
        let ecrit = serde_json::to_string_pretty(&reglage)
            .map_err(std::io::Error::other)
            .and_then(|texte| std::fs::write(self.demarrage.reglage(), texte));
        if let Err(e) = ecrit {
            let _ = self.demarrage.coffre.effacer();
            return Err(ErreurCommande {
                cle: "erreur.interne".to_owned(),
                jusqua: None,
                detail: Some(e.to_string()),
                modifiee: None,
            });
        }
        let relie = self.demarrage.relie(liaison, conclu.nom)?;
        // Le noyau du premier lancement s'arrête : ses tâches, puis sa base vide.
        if let Mode::Local {
            application,
            taches,
        } = std::mem::replace(&mut *mode, relie)
        {
            for tache in taches {
                tache.abort();
            }
            let _ = application.fermer().await;
            effacer_base_vide(&self.demarrage.racine);
        }
        if let Ok(mut garde) = self.apercu.lock() {
            *garde = None;
        }
        Ok(self.info_de(&mode))
    }

    /// Quitte le poste principal : efface la liaison et ramène le poste au premier lancement.
    pub(crate) async fn quitter(&self) -> Result<InfoPoste, ErreurCommande> {
        let mut mode = self.mode.write().await;
        if matches!(&*mode, Mode::Relie { .. }) {
            let local = self.demarrage.local().await?;
            if let Err(e) = self.demarrage.coffre.effacer() {
                self.demarrage
                    .noter(&format!("poste relie : liaison non effacee detail={e}"));
            }
            let _ = std::fs::remove_file(self.demarrage.reglage());
            *mode = local;
            if let Ok(mut garde) = self.liaison.lock() {
                *garde = None;
            }
            self.demarrage.noter("poste relie : liaison quittee");
        }
        Ok(self.info_de(&mode))
    }

    /// Postes reliés qui ont une session ouverte sur ce poste principal ; zéro tant que la
    /// fermeture a été confirmée ou que ce poste n'est pas principal.
    pub(crate) async fn postes_a_prevenir(&self) -> u32 {
        if self.fermeture_confirmee.load(Ordering::Relaxed) {
            return 0;
        }
        match &*self.mode.read().await {
            Mode::Local { application, .. } => application.postes_connectes().await,
            Mode::Relie { .. } => 0,
        }
    }

    /// La personne ferme le poste principal en connaissance de cause.
    pub(crate) fn confirmer_fermeture(&self) {
        self.fermeture_confirmee.store(true, Ordering::Relaxed);
    }

    /// Fermeture de GESCO : sauvegarde et fermeture de la base, pour un poste qui en a une.
    pub(crate) async fn fermer(&self) {
        if let Mode::Local { application, .. } = &*self.mode.read().await {
            // Sauvegarde à la fermeture (§11.1). Son échec ne retient pas l'application : la
            // sauvegarde planifiée suivante le rattrapera.
            let _ = application.fermer().await;
        }
    }
}
