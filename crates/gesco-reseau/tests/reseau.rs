//! Le poste principal répond par le réseau comme à son propre écran (SPEC_PHASE_11_A §0.3,
//! critères 1, 2, 6 et 9) : mêmes résultats et mêmes refus par les deux chemins, écritures
//! signées du poste appelant, session liée à son poste, commandes réservées au poste principal.
//!
//! Le serveur écoute la machine elle-même sur un port libre ; le poste qui appelle par le
//! réseau s'appelle « caisse », le poste principal « poste-principal ».

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::net::SocketAddr;
use std::sync::Arc;

use gesco_core::commun::Horodatage;
use gesco_core::core::{Poste, licence};
use gesco_reseau::{BoucleLocale, CLE_POSTE_INCONNU, EtatServeur, ecouter_boucle_locale, servir};
use gesco_services::cles::TrousseauMemoire;
use gesco_services::commandes::{COMMANDES, Lieu};
use gesco_services::session::{Jeton, PolitiqueSession};
use gesco_services::{Application, HorlogeFixe, OptionsApplication};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::oneshot;

const MOT_DE_PASSE: &str = "premier-mot-de-passe";
const VERSION: &str = "2.0.0-essai";
const POSTE_PRINCIPAL: &str = "poste-principal";
const POSTE_RELIE: &str = "caisse";
/// Entrée qu'aucune commande n'accepte : la garde passe, puis l'entrée est refusée, sans effet.
const ENTREE_REFUSEE: &str = "__entree_refusee__";

/// Un poste principal d'essai et son serveur. Le serveur s'arrête quand la structure tombe.
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
        Arc::new(BoucleLocale::nouvelle(Poste(POSTE_RELIE.to_owned()))),
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

/// Appel HTTP/1.1 écrit à la main, sans dépendance : une requête, puis la connexion se ferme.
async fn http(
    adresse: SocketAddr,
    methode: &str,
    chemin: &str,
    jeton: Option<&Jeton>,
    corps: Option<&str>,
) -> Reponse {
    let mut flux = TcpStream::connect(adresse).await.unwrap();
    let mut requete =
        format!("{methode} {chemin} HTTP/1.1\r\nHost: {adresse}\r\nConnection: close\r\n");
    if let Some(jeton) = jeton {
        requete.push_str(&format!("Authorization: Bearer {}\r\n", jeton.0));
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
    flux.read_to_end(&mut brut).await.unwrap();
    let texte = String::from_utf8(brut).unwrap();
    let (entetes, corps) = texte.split_once("\r\n\r\n").unwrap();
    let statut = entetes
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap();
    Reponse {
        statut,
        corps: serde_json::from_str(corps).unwrap_or(Value::Null),
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
        let reponse = http(
            self.adresse,
            "POST",
            &format!("/api/commandes/{nom}"),
            jeton,
            Some(&entree.to_string()),
        )
        .await;
        if reponse.statut == 200 {
            Ok(reponse.corps)
        } else {
            Err(reponse.corps["cle"].as_str().unwrap().to_owned())
        }
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
            } else if commande.fichier().is_some() {
                // Tant que le contenu des fichiers ne voyage pas (étape 3), aucun chemin venu
                // du réseau n'atteint un gestionnaire.
                assert_eq!(
                    obtenu,
                    Err("commande.fichier_non_transporte".to_owned()),
                    "{} manie un fichier et devrait être refusée par le réseau",
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
        .sur_place(
            "audit_lister",
            Some(&sur_place),
            json!({
                "utilisateur_id": null, "entite": null, "operation": null,
                "depuis": null, "jusqua": null, "page": 0,
            }),
        )
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
    // La clé d'un poste non reconnu existe pour l'étape 2 ; ici tout appel vient de la machine.
    assert_eq!(CLE_POSTE_INCONNU, "reseau.poste_inconnu");
}
