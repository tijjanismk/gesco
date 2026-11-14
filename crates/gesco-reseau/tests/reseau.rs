//! Le poste principal répond par le réseau comme à son propre écran (SPEC_PHASE_11_A §0.3,
//! critères 1 à 6 et 9) : mêmes résultats et mêmes refus par les deux chemins, écritures signées
//! du poste appelant, session liée à son poste, commandes réservées au poste principal ; puis un
//! poste se relie par le canal chiffré, se fait reconnaître à son secret, et se révoque.
//!
//! Le poste principal s'appelle « poste-principal ». Les essais du contrat passent par une
//! écoute en clair sur la machine elle-même, où tout appel est attribué au poste « caisse » ;
//! ceux de l'appairage passent par le superviseur et son canal chiffré.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;

use gesco_core::commun::Horodatage;
use gesco_core::core::reseau::{empreinte_certificat, empreinte_courte};
use gesco_core::core::{Poste, licence};
use gesco_reseau::chiffrement::{NOM_CERTIFICAT, configuration_client};
use gesco_reseau::{
    CLE_POSTE_INCONNU, CLE_TROP_D_ESSAIS, CLE_VERSION_DIFFERENTE, ENTETE_POSTE, ENTETE_SECRET,
    ENTETE_VERSION, EtatServeur, REFUS_MAX_PAR_MINUTE, Reconnaissance, ecouter_boucle_locale,
    relais, servir, superviser,
};
use gesco_services::cles::TrousseauMemoire;
use gesco_services::commandes::{COMMANDES, Lieu};
use gesco_services::session::{Jeton, PolitiqueSession};
use gesco_services::{Application, HorlogeFixe, OptionsApplication};
use rustls::pki_types::ServerName;
use serde_json::{Value, json};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{oneshot, watch};
use tokio_rustls::TlsConnector;

const MOT_DE_PASSE: &str = "premier-mot-de-passe";
const VERSION: &str = "2.0.0-essai";
const POSTE_PRINCIPAL: &str = "poste-principal";
const POSTE_RELIE: &str = "caisse";
/// Entrée qu'aucune commande n'accepte : la garde passe, puis l'entrée est refusée, sans effet.
const ENTREE_REFUSEE: &str = "__entree_refusee__";

/// Un poste principal d'essai et son écoute en clair. L'écoute s'arrête quand la structure tombe.
struct Ecole {
    application: Arc<Application>,
    adresse: SocketAddr,
    _arret: oneshot::Sender<()>,
    _dossier: tempfile::TempDir,
}

async fn ecole() -> Ecole {
    let dossier = tempfile::tempdir().unwrap();
    let application = Arc::new(
        Application::demarrer(OptionsApplication {
            racine: dossier.path().to_path_buf(),
            poste: Poste(POSTE_PRINCIPAL.to_owned()),
            empreinte_machine: "machine-test".to_owned(),
            trousseau: Arc::new(TrousseauMemoire::default()),
            horloge: Arc::new(HorlogeFixe::a(Horodatage::depuis_millis_epoque(
                1_791_101_730_250,
            ))),
            politique_session: PolitiqueSession::default(),
            version_applicative: VERSION.to_owned(),
            cle_licence: licence::cle_publique(&[42; 32]),
        })
        .await
        .unwrap(),
    );
    let ecoute = ecouter_boucle_locale(0).await.unwrap();
    let adresse = ecoute.local_addr().unwrap();
    let (arret, attente) = oneshot::channel::<()>();
    tokio::spawn(servir(
        ecoute,
        Arc::clone(&application),
        Reconnaissance::BoucleLocale(Poste(POSTE_RELIE.to_owned())),
        None,
        async move {
            let _ = attente.await;
        },
    ));
    Ecole {
        application,
        adresse,
        _arret: arret,
        _dossier: dossier,
    }
}

/// Réponse d'un appel par le réseau : le code HTTP et le corps JSON.
struct Reponse {
    statut: u16,
    corps: Value,
}

/// Appel HTTP/1.1 écrit à la main, sans dépendance, sur un flux déjà ouvert (en clair ou
/// chiffré) : une requête, puis la connexion se ferme.
async fn echanger<F: AsyncRead + AsyncWrite + Unpin>(
    mut flux: F,
    methode: &str,
    chemin: &str,
    entetes: &[(&str, String)],
    corps: Option<&str>,
) -> Reponse {
    let mut requete =
        format!("{methode} {chemin} HTTP/1.1\r\nHost: poste-principal\r\nConnection: close\r\n");
    // Sauf mention contraire, l'appelant est de la version du poste principal.
    if !entetes.iter().any(|(nom, _)| *nom == ENTETE_VERSION) {
        requete.push_str(&format!("{ENTETE_VERSION}: {VERSION}\r\n"));
    }
    for (nom, valeur) in entetes {
        requete.push_str(&format!("{nom}: {valeur}\r\n"));
    }
    if let Some(corps) = corps {
        requete.push_str(&format!(
            "Content-Type: application/json\r\nContent-Length: {}\r\n",
            corps.len()
        ));
    }
    requete.push_str("\r\n");
    requete.push_str(corps.unwrap_or_default());
    flux.write_all(requete.as_bytes()).await.unwrap();
    let mut brut = Vec::new();
    // Le canal chiffré peut se fermer sans adieu : ce qui a été lu suffit.
    let _ = flux.read_to_end(&mut brut).await;
    let texte = String::from_utf8(brut).unwrap();
    let (tete, corps) = texte.split_once("\r\n\r\n").unwrap();
    let statut = tete
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap();
    Reponse {
        statut,
        corps: serde_json::from_str(corps).unwrap_or(Value::Null),
    }
}

/// Appel en clair, sur l'écoute de la machine elle-même.
async fn http(
    adresse: SocketAddr,
    methode: &str,
    chemin: &str,
    jeton: Option<&Jeton>,
    corps: Option<&str>,
) -> Reponse {
    let flux = TcpStream::connect(adresse).await.unwrap();
    let entetes: Vec<(&str, String)> = jeton
        .map(|j| ("Authorization", format!("Bearer {}", j.0)))
        .into_iter()
        .collect();
    echanger(flux, methode, chemin, &entetes, corps).await
}

fn resultat(reponse: Reponse) -> Result<Value, String> {
    if reponse.statut == 200 {
        Ok(reponse.corps)
    } else {
        Err(reponse.corps["cle"].as_str().unwrap().to_owned())
    }
}

impl Ecole {
    /// La commande, demandée à l'écran du poste principal.
    async fn sur_place(
        &self,
        nom: &str,
        jeton: Option<&Jeton>,
        entree: Value,
    ) -> Result<Value, String> {
        self.application
            .executer(nom, jeton.cloned(), entree)
            .await
            .map_err(|erreur| erreur.cle)
    }

    /// La même commande, demandée par le réseau.
    async fn par_reseau(
        &self,
        nom: &str,
        jeton: Option<&Jeton>,
        entree: Value,
    ) -> Result<Value, String> {
        resultat(
            http(
                self.adresse,
                "POST",
                &format!("/api/commandes/{nom}"),
                jeton,
                Some(&entree.to_string()),
            )
            .await,
        )
    }

    async fn installer(&self) {
        self.sur_place(
            "premier_demarrage",
            None,
            json!({
                "organisation": "Groupe scolaire Askia",
                "etablissement_nom": "Lycée Askia Mohamed",
                "statut": "prive",
                "code_academie": "AE-BKO-RD",
                "code_cap": "CAP-01",
                "adresse": "Hamdallaye ACI 2000, Bamako",
                "telephones": ["+223 20 22 00 00"],
                "administrateur_identifiant": "admin",
                "administrateur_nom": "Awa Traoré",
                "administrateur_mot_de_passe": MOT_DE_PASSE,
                "cycles": ["premier_cycle"],
            }),
        )
        .await
        .unwrap();
        // La case du certificat ne se note qu'une fois, juste après la création : on la note ici
        // pour que la suite ne dépende pas de l'ordre des appels.
        self.sur_place("certificat_confirmer_initial", None, Value::Null)
            .await
            .unwrap();
    }

    fn identifiants() -> Value {
        json!({ "identifiant": "admin", "mot_de_passe": MOT_DE_PASSE })
    }

    async fn connecter_sur_place(&self) -> Jeton {
        let session = self
            .sur_place("connexion", None, Self::identifiants())
            .await
            .unwrap();
        serde_json::from_value(session["jeton"].clone()).unwrap()
    }

    async fn connecter_par_reseau(&self) -> Jeton {
        let session = self
            .par_reseau("connexion", None, Self::identifiants())
            .await
            .unwrap();
        assert_eq!(session["poste"], POSTE_RELIE);
        serde_json::from_value(session["jeton"].clone()).unwrap()
    }

    /// Compare, pour chaque commande du contrat, la réponse de l'écran et celle du réseau.
    async fn comparer_tout_le_contrat(
        &self,
        sur_place: Option<&Jeton>,
        reseau: Option<&Jeton>,
        entree: &Value,
    ) {
        for commande in COMMANDES {
            let attendu = self
                .sur_place(commande.nom, sur_place, entree.clone())
                .await;
            let obtenu = self.par_reseau(commande.nom, reseau, entree.clone()).await;
            if commande.lieu() == Lieu::PostePrincipal {
                assert_eq!(
                    obtenu,
                    Err("commande.poste_principal_seulement".to_owned()),
                    "{} devrait être réservée au poste principal",
                    commande.nom
                );
            } else {
                assert_eq!(obtenu, attendu, "{} ne répond pas pareil", commande.nom);
            }
        }
    }
}

#[tokio::test]
async fn l_etat_donne_la_version_du_poste_principal() {
    let ecole = ecole().await;
    let reponse = http(ecole.adresse, "GET", "/api/etat", None, None).await;
    assert_eq!(reponse.statut, 200);
    let etat: EtatServeur = serde_json::from_value(reponse.corps).unwrap();
    assert_eq!(etat.version, VERSION);
    let ailleurs = http(ecole.adresse, "GET", "/api/autre", None, None).await;
    assert_eq!(ailleurs.statut, 404);
}

#[tokio::test]
async fn sans_session_tout_le_contrat_repond_pareil_par_les_deux_chemins() {
    let ecole = ecole().await;
    // Avant le premier démarrage, puis une fois l'école créée : aucun jeton, donc les mêmes
    // refus, et les mêmes réponses pour les quelques commandes publiques sans entrée.
    ecole
        .comparer_tout_le_contrat(None, None, &json!(ENTREE_REFUSEE))
        .await;
    ecole.installer().await;
    ecole
        .comparer_tout_le_contrat(None, None, &json!(ENTREE_REFUSEE))
        .await;
    for nom in ["etat_demarrage", "sessions_du_poste"] {
        let attendu = ecole.sur_place(nom, None, Value::Null).await;
        assert!(attendu.is_ok(), "{nom} : {attendu:?}");
        assert_eq!(ecole.par_reseau(nom, None, Value::Null).await, attendu);
    }
}

#[tokio::test]
async fn avec_une_session_la_garde_laisse_passer_les_memes_commandes() {
    let ecole = ecole().await;
    ecole.installer().await;
    let sur_place = ecole.connecter_sur_place().await;
    let reseau = ecole.connecter_par_reseau().await;
    // L'administrateur a tous les droits : la garde passe partout, puis l'entrée est refusée
    // avant tout effet. Une commande oubliée par l'un des chemins répondrait autrement.
    ecole
        .comparer_tout_le_contrat(Some(&sur_place), Some(&reseau), &json!(ENTREE_REFUSEE))
        .await;
    let courante = ecole
        .par_reseau("session_courante", Some(&reseau), Value::Null)
        .await
        .unwrap();
    assert_eq!(courante["session"]["nom_affiche"], "Awa Traoré");
}

#[tokio::test]
async fn une_ecriture_porte_le_nom_du_poste_qui_l_a_demandee() {
    let ecole = ecole().await;
    ecole.installer().await;
    let sur_place = ecole.connecter_sur_place().await;
    ecole.connecter_par_reseau().await;
    let journal = ecole
        .sur_place("audit_lister", Some(&sur_place), json!({ "page": 0 }))
        .await
        .unwrap();
    let postes_des_connexions: Vec<&str> = journal["lignes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|ligne| ligne["apres"]["evenement"] == "connexion")
        .map(|ligne| ligne["poste"].as_str().unwrap())
        .collect();
    // De la plus récente à la plus ancienne : la connexion par le réseau, puis celle de l'écran.
    assert_eq!(postes_des_connexions, vec![POSTE_RELIE, POSTE_PRINCIPAL]);
}

#[tokio::test]
async fn une_session_ne_vaut_que_depuis_le_poste_qui_l_a_ouverte() {
    let ecole = ecole().await;
    ecole.installer().await;
    let sur_place = ecole.connecter_sur_place().await;
    let reseau = ecole.connecter_par_reseau().await;
    let invalide = Err("session.invalide".to_owned());
    assert_eq!(
        ecole
            .par_reseau("session_courante", Some(&sur_place), Value::Null)
            .await,
        invalide
    );
    assert_eq!(
        ecole
            .sur_place("session_courante", Some(&reseau), Value::Null)
            .await,
        invalide
    );
    // L'écran verrouillé de chaque poste ne montre que les sessions de ce poste.
    let vues = |sessions: Value| -> Vec<String> {
        sessions
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["jeton"].as_str().unwrap().to_owned())
            .collect()
    };
    let du_reseau = ecole
        .par_reseau("sessions_du_poste", None, Value::Null)
        .await
        .unwrap();
    assert_eq!(vues(du_reseau), vec![reseau.0.clone()]);
    let de_l_ecran = ecole
        .sur_place("sessions_du_poste", None, Value::Null)
        .await
        .unwrap();
    assert_eq!(vues(de_l_ecran), vec![sur_place.0.clone()]);
    // Chacune reste valable chez elle.
    assert!(
        ecole
            .par_reseau("session_courante", Some(&reseau), Value::Null)
            .await
            .is_ok()
    );
    assert!(
        ecole
            .sur_place("session_courante", Some(&sur_place), Value::Null)
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn ce_qui_depend_du_poste_principal_se_refuse_par_le_reseau() {
    let ecole = ecole().await;
    let reserve = Err("commande.poste_principal_seulement".to_owned());
    // Même le premier démarrage : une école ne se crée pas depuis un poste relié.
    assert_eq!(
        ecole.par_reseau("premier_demarrage", None, json!({})).await,
        reserve
    );
    ecole.installer().await;
    let reseau = ecole.connecter_par_reseau().await;
    assert_eq!(
        ecole
            .par_reseau("sauvegarde_creer", Some(&reseau), Value::Null)
            .await,
        reserve
    );
    // Le même compte, sur place, sauvegarde.
    let sur_place = ecole.connecter_sur_place().await;
    assert!(
        ecole
            .sur_place("sauvegarde_creer", Some(&sur_place), Value::Null)
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn un_corps_illisible_et_une_commande_inconnue_sont_refuses_proprement() {
    let ecole = ecole().await;
    let illisible = http(
        ecole.adresse,
        "POST",
        "/api/commandes/etat_demarrage",
        None,
        Some("{ ceci n'est pas du JSON"),
    )
    .await;
    assert_eq!(illisible.statut, 422);
    assert_eq!(illisible.corps["cle"], "commande.entree_invalide");
    assert_eq!(
        ecole.par_reseau("inexistante", None, Value::Null).await,
        Err("commande.inconnue".to_owned())
    );
}

// --- Postes reliés par le canal chiffré -------------------------------------------------------

/// Une école installée, le réseau allumé, servie par le superviseur sur un port libre.
struct EcoleReliee {
    ecole: Ecole,
    admin: Jeton,
    serveur: watch::Receiver<Option<SocketAddr>>,
}

async fn ecole_reliee() -> EcoleReliee {
    let ecole = ecole().await;
    ecole.installer().await;
    let admin = ecole.connecter_sur_place().await;
    ecole
        .sur_place("reseau_activer", Some(&admin), Value::Null)
        .await
        .unwrap();
    let serveur = superviser(Arc::clone(&ecole.application), |_port| {
        SocketAddr::from((Ipv4Addr::LOCALHOST, 0))
    });
    EcoleReliee {
        ecole,
        admin,
        serveur,
    }
}

impl EcoleReliee {
    /// Adresse où le serveur écoute, dès qu'il écoute.
    async fn adresse(&mut self) -> SocketAddr {
        (*self.serveur.wait_for(Option::is_some).await.unwrap()).unwrap()
    }

    /// Empreinte du certificat que le poste principal garde dans sa base.
    async fn empreinte(&self) -> String {
        let a_servir = self.ecole.application.reseau_a_servir().await.unwrap();
        let (certificat, _) = a_servir.unwrap().certificat.unwrap();
        empreinte_certificat(&certificat)
    }

    async fn sur_place(&self, nom: &str, entree: Value) -> Value {
        self.ecole
            .sur_place(nom, Some(&self.admin), entree)
            .await
            .unwrap()
    }

    /// Code d'appairage affiché sur le poste principal, sans son espace.
    async fn code(&self) -> String {
        self.sur_place("poste_appairage_ouvrir", Value::Null).await["appairage"]["code"]
            .as_str()
            .unwrap()
            .replace(' ', "")
    }
}

/// Appel par le canal chiffré, en n'acceptant que le poste principal de cette empreinte.
/// `Err` si le canal ne s'établit pas.
async fn chiffre(
    adresse: SocketAddr,
    empreinte: &str,
    methode: &str,
    chemin: &str,
    entetes: &[(&str, String)],
    corps: Option<&str>,
) -> Result<Reponse, String> {
    let connecteur = TlsConnector::from(configuration_client(empreinte).unwrap());
    let flux = TcpStream::connect(adresse).await.unwrap();
    let nom = ServerName::try_from(NOM_CERTIFICAT).unwrap();
    let flux = connecteur
        .connect(nom, flux)
        .await
        .map_err(|erreur| erreur.to_string())?;
    Ok(echanger(flux, methode, chemin, entetes, corps).await)
}

/// Ce qu'un poste relié garde après son appairage, et ses appels.
struct PosteEssai {
    adresse: SocketAddr,
    empreinte: String,
    id: String,
    secret: String,
}

impl PosteEssai {
    async fn appeler(&self, nom: &str, jeton: Option<&Jeton>, entree: Value) -> Reponse {
        let mut entetes = vec![
            (ENTETE_POSTE, self.id.clone()),
            (ENTETE_SECRET, self.secret.clone()),
        ];
        if let Some(jeton) = jeton {
            entetes.push(("Authorization", format!("Bearer {}", jeton.0)));
        }
        chiffre(
            self.adresse,
            &self.empreinte,
            "POST",
            &format!("/api/commandes/{nom}"),
            &entetes,
            Some(&entree.to_string()),
        )
        .await
        .unwrap()
    }
}

async fn relier(e: &mut EcoleReliee, nom: &str) -> PosteEssai {
    let adresse = e.adresse().await;
    let empreinte = e.empreinte().await;
    let code = e.code().await;
    let reponse = chiffre(
        adresse,
        &empreinte,
        "POST",
        "/api/appairage",
        &[],
        Some(&json!({ "code": code, "nom": nom }).to_string()),
    )
    .await
    .unwrap();
    assert_eq!(reponse.statut, 200, "{}", reponse.corps);
    PosteEssai {
        adresse,
        empreinte,
        id: reponse.corps["poste_id"].as_str().unwrap().to_owned(),
        secret: reponse.corps["secret"].as_str().unwrap().to_owned(),
    }
}

#[tokio::test]
async fn un_poste_se_relie_par_le_canal_chiffre_et_travaille_sous_son_nom() {
    let mut e = ecole_reliee().await;
    let adresse = e.adresse().await;
    let empreinte = e.empreinte().await;
    // L'écran du poste principal montre l'empreinte courte de ce même certificat.
    e.ecole
        .application
        .declarer_adresses_du_poste(Arc::new(|| vec!["192.168.1.20".to_owned()]));
    let etat = e.sur_place("reseau_etat_lire", Value::Null).await;
    assert_eq!(etat["empreinte_courte"], empreinte_courte(&empreinte));
    assert_eq!(etat["adresses"], json!(["192.168.1.20"]));
    assert_eq!(etat["connectes"], json!([]));

    // Un code faux ne relie rien.
    e.code().await;
    let refuse = chiffre(
        adresse,
        &empreinte,
        "POST",
        "/api/appairage",
        &[],
        Some(&json!({ "code": "ZZZZZZZZ", "nom": "Caisse" }).to_string()),
    )
    .await
    .unwrap();
    assert_eq!(refuse.statut, 422);
    assert_eq!(refuse.corps["cle"], "reseau.code_faux");

    let poste = relier(&mut e, "Caisse").await;
    assert_eq!(poste.secret.len(), 64);

    // Le poste ouvre une session et travaille : ses écritures portent son nom.
    let session = poste
        .appeler("connexion", None, Ecole::identifiants())
        .await;
    assert_eq!(session.statut, 200, "{}", session.corps);
    assert_eq!(session.corps["poste"], "Caisse");
    let jeton: Jeton = serde_json::from_value(session.corps["jeton"].clone()).unwrap();
    let courante = poste
        .appeler("session_courante", Some(&jeton), Value::Null)
        .await;
    assert_eq!(courante.corps["session"]["nom_affiche"], "Awa Traoré");
    let journal = e.sur_place("audit_lister", json!({ "page": 0 })).await;
    let derniere_connexion = journal["lignes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|ligne| ligne["apres"]["evenement"] == "connexion")
        .unwrap();
    assert_eq!(derniere_connexion["poste"], "Caisse");
    // L'écran du poste principal le montre connecté.
    let etat = e.sur_place("reseau_etat_lire", Value::Null).await;
    assert_eq!(etat["connectes"], json!([poste.id]));

    // Sans identité, ou avec un secret faux : refusé.
    let anonyme = chiffre(
        adresse,
        &empreinte,
        "POST",
        "/api/commandes/etat_demarrage",
        &[],
        Some("null"),
    )
    .await
    .unwrap();
    assert_eq!(anonyme.statut, 401);
    assert_eq!(anonyme.corps["cle"], CLE_POSTE_INCONNU);
    let faux = PosteEssai {
        secret: "0".repeat(64),
        adresse,
        empreinte: empreinte.clone(),
        id: poste.id.clone(),
    };
    assert_eq!(
        faux.appeler("etat_demarrage", None, Value::Null)
            .await
            .statut,
        401
    );

    // Un poste qui attend une autre empreinte refuse ce poste principal : le canal ne s'établit pas.
    let autre = empreinte_certificat(b"un autre poste principal");
    assert!(
        chiffre(adresse, &autre, "GET", "/api/etat", &[], None)
            .await
            .is_err()
    );

    // Révoqué : refusé à l'appel suivant, et sa session n'existe plus.
    e.sur_place("poste_revoquer", json!({ "poste_id": poste.id }))
        .await;
    let apres = poste
        .appeler("session_courante", Some(&jeton), Value::Null)
        .await;
    assert_eq!(apres.statut, 401);
    assert_eq!(apres.corps["cle"], CLE_POSTE_INCONNU);
    let etat = e.sur_place("reseau_etat_lire", Value::Null).await;
    assert_eq!(etat["connectes"], json!([]));
    assert_eq!(etat["en_service"], 0);
}

#[tokio::test]
async fn des_essais_au_hasard_mettent_l_adresse_en_attente() {
    let mut e = ecole_reliee().await;
    let poste = relier(&mut e, "Caisse").await;
    let curieux = PosteEssai {
        secret: "0".repeat(64),
        adresse: poste.adresse,
        empreinte: poste.empreinte.clone(),
        id: poste.id.clone(),
    };
    for _ in 0..REFUS_MAX_PAR_MINUTE {
        let refuse = curieux.appeler("etat_demarrage", None, Value::Null).await;
        assert_eq!(refuse.statut, 401);
    }
    // Au-delà, l'adresse n'est plus écoutée : ni pour un secret, ni pour un code.
    let attente = curieux.appeler("etat_demarrage", None, Value::Null).await;
    assert_eq!(attente.statut, 429);
    assert_eq!(attente.corps["cle"], CLE_TROP_D_ESSAIS);
    let code = chiffre(
        poste.adresse,
        &poste.empreinte,
        "POST",
        "/api/appairage",
        &[],
        Some(&json!({ "code": "ZZZZZZZZ", "nom": "Autre" }).to_string()),
    )
    .await
    .unwrap();
    assert_eq!(code.statut, 429);
}

#[tokio::test]
async fn le_serveur_suit_le_reseau_allume_eteint_cle_renouvelee() {
    let mut e = ecole_reliee().await;
    let poste = relier(&mut e, "Caisse").await;
    let premiere = poste.empreinte.clone();
    assert_eq!(
        poste
            .appeler("etat_demarrage", None, Value::Null)
            .await
            .statut,
        200
    );

    // Réseau éteint : le serveur n'écoute plus.
    e.sur_place("reseau_eteindre", Value::Null).await;
    e.serveur.wait_for(Option::is_none).await.unwrap();
    assert!(TcpStream::connect(poste.adresse).await.is_err());

    // Rallumé : même certificat, le poste déjà relié travaille de nouveau.
    e.sur_place("reseau_activer", Value::Null).await;
    let adresse = e.adresse().await;
    assert_eq!(e.empreinte().await, premiere);
    let revenu = PosteEssai { adresse, ..poste };
    assert_eq!(
        revenu
            .appeler("etat_demarrage", None, Value::Null)
            .await
            .statut,
        200
    );

    // Clé renouvelée : le serveur repart avec un autre certificat ; l'ancien poste, qui attend
    // l'ancienne empreinte, n'établit plus le canal, et il est révoqué.
    e.sur_place("reseau_cle_renouveler", Value::Null).await;
    e.serveur.wait_for(Option::is_none).await.unwrap();
    let adresse = e.adresse().await;
    let seconde = e.empreinte().await;
    assert_ne!(seconde, premiere);
    assert!(
        chiffre(adresse, &premiere, "GET", "/api/etat", &[], None)
            .await
            .is_err()
    );
    let etat = e.sur_place("reseau_etat_lire", Value::Null).await;
    assert_eq!(etat["en_service"], 0);
    assert_eq!(etat["empreinte_courte"], empreinte_courte(&seconde));

    // Relié de nouveau avec la nouvelle empreinte, il travaille.
    let nouveau = relier(&mut e, "Caisse").await;
    assert_eq!(nouveau.empreinte, seconde);
    assert_eq!(
        nouveau
            .appeler("etat_demarrage", None, Value::Null)
            .await
            .statut,
        200
    );
}

// --- Le poste relié et son relais ---------------------------------------------------------------

/// Relie un poste comme le fait son écran : aperçu de l'empreinte, code, puis relais.
async fn relais_de(e: &mut EcoleReliee, nom: &str) -> (relais::Liaison, relais::Relais) {
    let adresse = e.adresse().await.to_string();
    let apercu = relais::apercevoir(&adresse).await.unwrap();
    // L'empreinte courte lue par le poste est celle que montre l'écran du poste principal.
    let etat = e.sur_place("reseau_etat_lire", Value::Null).await;
    assert_eq!(etat["empreinte_courte"], apercu.empreinte_courte);
    assert_eq!(apercu.version, VERSION);
    let code = e.code().await;
    let (liaison, conclu) = relais::appairer(&apercu, VERSION, &code, nom)
        .await
        .unwrap();
    assert_eq!(conclu.nom, nom);
    let relais = relais::Relais::nouveau(liaison.clone(), VERSION).unwrap();
    (liaison, relais)
}

async fn connecter_par_relais(relais: &relais::Relais) -> String {
    let session = relais
        .executer("connexion", None, Ecole::identifiants())
        .await
        .unwrap();
    session["jeton"].as_str().unwrap().to_owned()
}

#[tokio::test]
async fn le_relais_transmet_les_commandes_et_le_contenu_des_fichiers() {
    let mut e = ecole_reliee().await;
    let (liaison, relais) = relais_de(&mut e, "Caisse").await;
    relais.verifier().await.unwrap();
    let jeton = connecter_par_relais(&relais).await;

    // La session dit le rôle du poste, de chaque côté ; le poste principal compte ses postes.
    let courante = relais
        .executer("session_courante", Some(&jeton), Value::Null)
        .await
        .unwrap();
    assert_eq!(courante["role_poste"], "relie");
    assert_eq!(courante["session"]["poste"], "Caisse");
    let sur_place = e.sur_place("session_courante", Value::Null).await;
    assert_eq!(sur_place["role_poste"], "principal");
    assert_eq!(e.ecole.application.postes_connectes().await, 1);

    // Une commande qui écrit un fichier : il arrive sur le disque du poste relié, au chemin
    // choisi, et la réponse redit ce chemin.
    let disque = tempfile::tempdir().unwrap();
    let classeur = disque.path().join("liste des classes.xlsx");
    let ecrit = relais
        .executer(
            "exporter_xlsx",
            Some(&jeton),
            json!({
                "chemin": classeur.to_string_lossy(),
                "titre": "Classes",
                "entetes": ["Classe", "Effectif"],
                "lignes": [["7e A", "42"], ["8e A", "39"]],
            }),
        )
        .await
        .unwrap();
    assert_eq!(ecrit["lignes"], 2);
    assert_eq!(
        ecrit["chemin"].as_str().unwrap(),
        classeur.to_string_lossy()
    );
    let octets = std::fs::read(&classeur).unwrap();
    assert!(octets.starts_with(b"PK"), "un classeur est une archive ZIP");

    // Une commande qui lit un fichier : son contenu part du poste relié.
    let annee = relais
        .executer(
            "annee_creer",
            Some(&jeton),
            json!({ "libelle": "2026-2027", "debut": 1_790_812_800_000_i64, "fin": 1_816_992_000_000_i64 }),
        )
        .await
        .unwrap();
    relais
        .executer("annee_activer", Some(&jeton), json!({ "id": annee["id"] }))
        .await
        .unwrap();
    let eleves = disque.path().join("eleves.csv");
    std::fs::write(
        &eleves,
        "nom;prenoms;sexe;date_naissance;classe\nTraoré;Awa;F;12/03/2014;7e A\n",
    )
    .unwrap();
    let analyse = relais
        .executer(
            "import_eleves_analyser",
            Some(&jeton),
            json!({ "chemin": eleves.to_string_lossy() }),
        )
        .await
        .unwrap();
    assert_eq!(
        analyse["champs"],
        json!(["nom", "prenoms", "sexe", "date_naissance", "classe"])
    );
    // Le refus du gestionnaire revient tel quel, et un fichier absent se dit sur ce poste.
    let pdf = disque.path().join("eleves.pdf");
    std::fs::write(&pdf, b"%PDF-").unwrap();
    let refus = relais
        .executer(
            "import_eleves_analyser",
            Some(&jeton),
            json!({ "chemin": pdf.to_string_lossy() }),
        )
        .await
        .unwrap_err();
    assert_eq!(refus.cle, "import.format_inconnu");
    let absent = relais
        .executer(
            "import_eleves_analyser",
            Some(&jeton),
            json!({ "chemin": disque.path().join("absent.csv").to_string_lossy() }),
        )
        .await
        .unwrap_err();
    assert_eq!(absent.cle, relais::CLE_FICHIER_INACCESSIBLE);

    // Rien ne reste sur le poste principal.
    let travail = e.ecole._dossier.path().join("temp").join("reseau");
    let restes = std::fs::read_dir(&travail)
        .map(Iterator::count)
        .unwrap_or(0);
    assert_eq!(
        restes, 0,
        "le dossier de travail est vide après chaque commande"
    );

    // Un chemin brut présenté par un poste relié n'atteint aucun gestionnaire.
    let poste = PosteEssai {
        adresse: e.adresse().await,
        empreinte: liaison.empreinte.clone(),
        id: liaison.poste_id.clone(),
        secret: liaison.secret.clone(),
    };
    let brut = poste
        .appeler(
            "import_eleves_analyser",
            Some(&Jeton(jeton.clone())),
            json!({ "chemin": eleves.to_string_lossy() }),
        )
        .await;
    assert_eq!(brut.statut, 422);
    assert_eq!(brut.corps["cle"], "commande.entree_invalide");
    // Sans session, la garde refuse d'abord : rien n'est déposé pour un inconnu.
    let sans_session = poste
        .appeler(
            "import_eleves_analyser",
            None,
            json!({ "chemin": { "nom": "eleves.csv", "contenu": "bm9t" } }),
        )
        .await;
    assert_eq!(sans_session.corps["cle"], "session.invalide");
}

#[tokio::test]
async fn un_poste_d_une_autre_version_est_refuse_avec_la_version_attendue() {
    let mut e = ecole_reliee().await;
    let (liaison, _) = relais_de(&mut e, "Caisse").await;
    let ancien_id = liaison.poste_id.clone();
    let ancien = relais::Relais::nouveau(liaison, "1.2.7").unwrap();
    let refus = ancien
        .executer("etat_demarrage", None, Value::Null)
        .await
        .unwrap_err();
    assert_eq!(refus.cle, CLE_VERSION_DIFFERENTE);
    assert_eq!(refus.detail.as_deref(), Some(VERSION));
    let refus = ancien.verifier().await.unwrap_err();
    assert_eq!(refus.cle, CLE_VERSION_DIFFERENTE);
    assert_eq!(refus.detail.as_deref(), Some(VERSION));

    // L'appairage aussi : un poste d'une autre version ne se relie pas.
    e.sur_place("poste_revoquer", json!({ "poste_id": ancien_id }))
        .await;
    let adresse = e.adresse().await.to_string();
    let apercu = relais::apercevoir(&adresse).await.unwrap();
    let code = e.code().await;
    let refus = relais::appairer(&apercu, "1.2.7", &code, "Ancien")
        .await
        .unwrap_err();
    assert_eq!(refus.cle, CLE_VERSION_DIFFERENTE);
    // Le code n'a pas servi : le même poste, mis à la bonne version, se relie avec lui.
    assert!(
        relais::appairer(&apercu, VERSION, &code, "Nouveau")
            .await
            .is_ok()
    );

    // Sans dire sa version, un appel est refusé de même.
    let muet = chiffre(
        e.adresse().await,
        &apercu.empreinte,
        "POST",
        "/api/commandes/etat_demarrage",
        &[(ENTETE_VERSION, String::new())],
        Some("null"),
    )
    .await
    .unwrap();
    assert_eq!(muet.statut, 426);
    assert_eq!(muet.corps["detail"], VERSION);
}

#[tokio::test]
async fn poste_principal_eteint_le_relais_le_dit_puis_reprend() {
    let mut e = ecole_reliee().await;
    let (liaison, relais) = relais_de(&mut e, "Caisse").await;
    let jeton = connecter_par_relais(&relais).await;

    // Réseau éteint : la commande n'atteint pas le poste principal, et rien n'est mis de côté.
    e.sur_place("reseau_eteindre", Value::Null).await;
    e.serveur.wait_for(Option::is_none).await.unwrap();
    let coupe = relais
        .executer("session_courante", Some(&jeton), Value::Null)
        .await
        .unwrap_err();
    assert_eq!(coupe.cle, relais::CLE_INJOIGNABLE);
    assert_eq!(
        relais.verifier().await.unwrap_err().cle,
        relais::CLE_INJOIGNABLE
    );

    // Rallumé : le même poste, sans se relier de nouveau, retrouve sa session.
    e.sur_place("reseau_activer", Value::Null).await;
    let revenu = relais::Relais::nouveau(
        relais::Liaison {
            adresse: e.adresse().await.to_string(),
            ..liaison.clone()
        },
        VERSION,
    )
    .unwrap();
    revenu.verifier().await.unwrap();
    let courante = revenu
        .executer("session_courante", Some(&jeton), Value::Null)
        .await
        .unwrap();
    assert_eq!(courante["session"]["poste"], "Caisse");

    // Clé du réseau renouvelée : le poste refuse ce poste principal, qui n'a plus l'empreinte
    // retenue.
    e.sur_place("reseau_cle_renouveler", Value::Null).await;
    e.serveur.wait_for(Option::is_none).await.unwrap();
    let change = relais::Relais::nouveau(
        relais::Liaison {
            adresse: e.adresse().await.to_string(),
            ..liaison
        },
        VERSION,
    )
    .unwrap();
    assert_eq!(
        change.verifier().await.unwrap_err().cle,
        relais::CLE_EMPREINTE_DIFFERENTE
    );
}

#[tokio::test]
async fn un_appareil_qui_ne_repond_pas_rend_poste_principal_injoignable() {
    // Un appareil qui accepte la connexion et ne dit jamais rien.
    let ecoute = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();
    let adresse = ecoute.local_addr().unwrap();
    tokio::spawn(async move {
        let mut gardes = Vec::new();
        while let Ok((flux, _)) = ecoute.accept().await {
            gardes.push(flux);
        }
    });
    let refus = relais::apercevoir(&adresse.to_string()).await.unwrap_err();
    assert_eq!(refus.cle, relais::CLE_INJOIGNABLE);
}

/// PNG d'essai (4 × 4 pixels) en base64, à la place d'une photo : au jardin, l'inscription en
/// demande une.
const PHOTO: &str = "iVBORw0KGgoAAAANSUhEUgAAAAQAAAAECAIAAAAmkwkpAAAAEElEQVR4nGOQc1gPRwzEcQCsUxDRqgYFyQAAAABJRU5ErkJggg==";

/// SPEC_PHASE_11_B critère 13 : par le réseau, les commandes du jardin donnent le même résultat
/// et le même refus que sur place ; deux postes qui modifient la même observation ne s'écrasent
/// pas en silence.
#[tokio::test]
async fn le_jardin_repond_pareil_par_le_reseau_et_ne_s_ecrase_pas() {
    let ecole = ecole().await;
    let ok = |r: Result<Value, String>, nom: &str| r.unwrap_or_else(|e| panic!("{nom} : {e}"));
    ok(
        ecole
            .sur_place(
                "premier_demarrage",
                None,
                json!({
                    "organisation": "Groupe scolaire Askia",
                    "etablissement_nom": "École Askia",
                    "statut": "prive",
                    "code_academie": "AE-BKO-RD",
                    "code_cap": "CAP-01",
                    "adresse": "Hamdallaye ACI 2000, Bamako",
                    "telephones": ["+223 20 22 00 00"],
                    "administrateur_identifiant": "admin",
                    "administrateur_nom": "Awa Traoré",
                    "administrateur_mot_de_passe": MOT_DE_PASSE,
                    "cycles": ["jardin", "premier_cycle"],
                }),
            )
            .await,
        "premier_demarrage",
    );
    ok(
        ecole
            .sur_place("certificat_confirmer_initial", None, Value::Null)
            .await,
        "certificat_confirmer_initial",
    );
    let sp = ecole.connecter_sur_place().await;
    let rs = ecole.connecter_par_reseau().await;
    let ici = |nom: &'static str, entree: Value| {
        let ecole = &ecole;
        let sp = &sp;
        async move { ok(ecole.sur_place(nom, Some(sp), entree).await, nom) }
    };
    let annee = ici(
        "annee_creer",
        json!({ "libelle": "2026-2027", "debut": 1_790_812_800_000_i64, "fin": 1_816_992_000_000_i64 }),
    )
    .await;
    ici("annee_activer", json!({ "id": annee["id"] })).await;
    // Les frais de l'année, sans quoi aucune inscription ne se valide.
    let niveaux = ici("niveaux_lister", Value::Null).await;
    let grille = ici(
        "grille_creer",
        json!({ "annee_id": annee["id"], "copie_de": null }),
    )
    .await;
    let lignes: Vec<Value> = niveaux["niveaux"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| json!({ "niveau_id": n["id"], "serie_id": null, "type_frais": "inscription",
            "libelle": "Frais d'inscription", "montant": 10_000, "obligatoire": true, "exigibilite": "une_fois" }))
        .collect();
    let plans: Vec<Value> = grille["contenu"]["plans"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| json!({ "libelle": p["libelle"], "par_defaut": p["par_defaut"], "tranches": p["tranches"] }))
        .collect();
    ici(
        "grille_modifier",
        json!({ "id": grille["grille"]["id"], "lue_le": grille["modifiee_le"],
            "libelle": grille["grille"]["libelle"], "lignes": lignes, "plans": plans }),
    )
    .await;
    ici(
        "grille_mettre_en_vigueur",
        json!({ "id": grille["grille"]["id"] }),
    )
    .await;
    let niveau = |code: &str| {
        niveaux["niveaux"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["code"] == code)
            .unwrap()["id"]
            .clone()
    };
    let ms = ici(
        "classe_creer",
        json!({ "annee_id": annee["id"], "niveau_id": niveau("MS"), "serie_id": null, "libelle": "MS A" }),
    )
    .await["id"]
        .clone();
    let premiere = ici(
        "classe_creer",
        json!({ "annee_id": annee["id"], "niveau_id": niveau("1"), "serie_id": null, "libelle": "1re A" }),
    )
    .await["id"]
        .clone();
    // Un enfant au jardin : photo, mère joignable qui vient le chercher.
    let fiche = ici(
        "eleve_creer",
        json!({ "identite": { "nom": "Diarra", "prenoms": "Aminata", "sexe": "feminin",
            "date_naissance": "2022-03-02", "date_naissance_estimee": false, "lieu_naissance": "Bamako",
            "nationalite": "Malienne", "adresse": "", "telephones": [] },
            "matricule_interne": "J/2022/001", "matricule_national": "", "acte_nature": "acte",
            "acte_numero": "", "acte_date": "", "ecole_origine": "" }),
    )
    .await;
    ici(
        "responsable_rattacher",
        json!({ "eleve_id": fiche["eleve"]["id"], "responsable_id": null,
            "saisie": { "identite": { "nom": "Diarra", "prenoms": "Mariam", "sexe": "feminin",
                "date_naissance": "", "date_naissance_estimee": false, "lieu_naissance": "",
                "nationalite": "", "adresse": "", "telephones": ["76 12 34 56"] },
                "profession": "", "employeur": "" },
            "qualites": { "lien": "mere", "est_responsable_legal": true, "est_payeur": true,
                "recoit_notifications": true, "vient_chercher": true, "ordre_priorite": 1 } }),
    )
    .await;
    ici(
        "eleve_photo_definir",
        json!({ "id": fiche["eleve"]["id"], "contenu_base64": PHOTO }),
    )
    .await;
    ici(
        "inscription_creer",
        json!({ "eleve_id": fiche["eleve"]["id"], "classe_id": ms, "date_inscription": "",
            "redoublant": false, "etablissement_origine": "" }),
    )
    .await;
    assert_eq!(
        ici("inscriptions_valider_classe", json!({ "id": ms })).await["validees"],
        1
    );
    let ped = ici("pedagogie_lire", json!({ "id": annee["id"] })).await;
    let trimestre = ped["periodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["periode"]["niveau_id"] == niveau("MS") && p["periode"]["rang"] == 1)
        .unwrap()["periode"]["id"]
        .clone();
    let grille_obs = json!({ "classe_id": ms, "periode_id": trimestre });

    // Les lectures donnent la même chose par les deux chemins.
    for (nom, entree) in [
        ("jardin_lire", Value::Null),
        ("observations_grille", grille_obs.clone()),
        ("maitres_jardin_lister", json!({ "id": annee["id"] })),
        ("portee_notes_lire", Value::Null),
    ] {
        assert_eq!(
            ecole.par_reseau(nom, Some(&rs), entree.clone()).await,
            ecole.sur_place(nom, Some(&sp), entree).await,
            "{nom}"
        );
    }
    // Les refus aussi : une classe à notes, une compétence inconnue.
    let g = ici("observations_grille", grille_obs.clone()).await;
    let enfant = g["enfants"][0]["inscription_id"].clone();
    let competence = g["competences"][0]["id"].clone();
    for (nom, entree) in [
        (
            "observations_grille",
            json!({ "classe_id": premiere, "periode_id": trimestre }),
        ),
        (
            "observation_saisir",
            json!({ "classe_id": ms, "periode_id": trimestre, "competence_id": enfant,
                "inscription_id": enfant, "niveau": "acquis", "lue_le": 0 }),
        ),
        (
            "observation_apprecier",
            json!({ "classe_id": ms, "periode_id": trimestre, "inscription_id": enfant,
                "domaine_id": null, "texte": "x".repeat(301) }),
        ),
    ] {
        let local = ecole.sur_place(nom, Some(&sp), entree.clone()).await;
        assert!(local.is_err(), "{nom}");
        assert_eq!(
            ecole.par_reseau(nom, Some(&rs), entree).await,
            local,
            "{nom}"
        );
    }

    // Le poste relié pose une case ; l'écran, qui l'avait lue vide, ne l'écrase pas.
    let case = |niveau: &str, lue_le: i64| {
        json!({ "classe_id": ms, "periode_id": trimestre, "competence_id": competence,
            "inscription_id": enfant, "niveau": niveau, "lue_le": lue_le })
    };
    let posee = ok(
        ecole
            .par_reseau("observation_saisir", Some(&rs), case("acquis", 0))
            .await,
        "observation_saisir",
    );
    assert_eq!(posee["niveau"], "acquis");
    assert_eq!(
        ecole
            .sur_place("observation_saisir", Some(&sp), case("non_acquis", 0))
            .await,
        Err("fiche.modifiee_entre_temps".to_owned())
    );
    // Et l'inverse : le poste relié qui a lu avant l'écran est refusé à son tour.
    let lue = posee["modifiee_le"].as_i64().unwrap();
    ok(
        ecole
            .sur_place("observation_saisir", Some(&sp), case("en_cours", lue))
            .await,
        "observation_saisir",
    );
    assert_eq!(
        ecole
            .par_reseau("observation_saisir", Some(&rs), case("non_acquis", lue))
            .await,
        Err("fiche.modifiee_entre_temps".to_owned())
    );
    // « Toute la colonne » et l'appréciation par le réseau, puis le livret préparé par le
    // réseau se lit pareil des deux côtés.
    ok(
        ecole
            .par_reseau(
                "observations_colonne",
                Some(&rs),
                json!({ "classe_id": ms, "periode_id": trimestre,
                    "competence_id": g["competences"][1]["id"], "niveau": "en_cours", "lue_le": 0 }),
            )
            .await,
        "observations_colonne",
    );
    ok(
        ecole
            .par_reseau(
                "observation_apprecier",
                Some(&rs),
                json!({ "classe_id": ms, "periode_id": trimestre, "inscription_id": enfant,
                    "domaine_id": null, "texte": "Un bon trimestre." }),
            )
            .await,
        "observation_apprecier",
    );
    let livrets = ok(
        ecole
            .par_reseau("resultats_calculer", Some(&rs), grille_obs.clone())
            .await,
        "resultats_calculer",
    );
    assert_eq!(
        livrets["bulletins"][0]["bulletin"]["contenu"]["forme"],
        "livret"
    );
    assert_eq!(
        ecole
            .par_reseau("bulletins_lire", Some(&rs), grille_obs.clone())
            .await,
        ecole
            .sur_place("bulletins_lire", Some(&sp), grille_obs)
            .await
    );
}
