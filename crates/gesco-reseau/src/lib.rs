//! Serveur du réseau local (ADR-014, ADR du 10 novembre 2026, SPEC_PHASE_11_A §3 et §4) : le
//! poste principal répond aux postes reliés. Une seule route de commandes, qui remet chaque appel
//! à la frontière de `gesco-services` : même garde, mêmes gestionnaires que l'écran du poste.
//!
//! Un poste se relie par la route d'appairage, avec le code affiché sur le poste principal ; il
//! reçoit un secret, qu'il présente ensuite à chaque appel. Le canal est chiffré par le
//! certificat du poste principal ([`chiffrement`]). Le superviseur ([`superviser`]) démarre et
//! arrête l'écoute quand la direction allume ou éteint le réseau.

pub mod chiffrement;

use std::collections::HashMap;
use std::future::Future;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::rejection::JsonRejection;
use axum::extract::{ConnectInfo, DefaultBodyLimit, Path, State};
use axum::http::header::AUTHORIZATION;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use gesco_core::core::Poste;
use gesco_services::session::Jeton;
use gesco_services::{Application, ErreurCommande, Origine};
use hyper_util::rt::TokioIo;
use rustls::ServerConfig;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpListener;
use tokio::sync::watch;
use tokio_rustls::TlsAcceptor;
use tower_service::Service;

/// Port du poste principal, tant que l'école n'en règle pas un autre.
pub const PORT_PAR_DEFAUT: u16 = 47_800;

/// Taille maximale du corps d'un appel : photos, pièces et fichiers d'import.
pub const TAILLE_MAX_CORPS: usize = 32 * 1024 * 1024;

/// Clé d'erreur d'un appel venu d'un poste que le poste principal ne reconnaît pas.
pub const CLE_POSTE_INCONNU: &str = "reseau.poste_inconnu";

/// Clé d'erreur d'une adresse qui vient d'être refusée trop de fois : elle attend.
pub const CLE_TROP_D_ESSAIS: &str = "reseau.trop_d_essais";

/// Refus admis d'une même adresse en une minute (code d'appairage faux, poste non reconnu) ;
/// au-delà, elle n'est plus écoutée jusqu'à la fin de la minute.
pub const REFUS_MAX_PAR_MINUTE: u32 = 10;

/// Durée pendant laquelle les refus d'une adresse se comptent.
const FENETRE_REFUS: Duration = Duration::from_secs(60);

/// En-tête où un poste relié présente son identifiant.
pub const ENTETE_POSTE: &str = "x-gesco-poste";

/// En-tête où un poste relié présente son secret.
pub const ENTETE_SECRET: &str = "x-gesco-secret";

/// Clé d'erreur d'un corps qui n'est pas du JSON : la même que pour une entrée mal formée.
const CLE_ENTREE_INVALIDE: &str = "commande.entree_invalide";

/// Temps laissé à un appareil pour établir le canal chiffré avant d'être abandonné.
const DELAI_CANAL: Duration = Duration::from_secs(10);

/// Comment le serveur reconnaît le poste qui appelle. Le nom rendu signe ses écritures au
/// journal d'audit : il ne vient jamais d'un nom que le poste appelant choisirait lui-même.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reconnaissance {
    /// Essais sur une seule machine : tout appel venu de la machine elle-même est attribué à ce
    /// poste. Sans secret ni chiffrement : ne sert jamais sur le réseau.
    BoucleLocale(Poste),
    /// Postes reliés : chaque appel présente l'identifiant et le secret reçus à l'appairage, et
    /// vient d'une adresse du réseau privé.
    Postes,
}

/// Vrai pour une adresse de la machine elle-même ou d'un réseau privé : le poste principal ne
/// répond jamais à une adresse d'Internet.
pub fn adresse_privee(adresse: IpAddr) -> bool {
    match adresse {
        IpAddr::V4(v4) => v4.is_loopback() || v4.is_private() || v4.is_link_local(),
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return adresse_privee(IpAddr::V4(v4));
            }
            let tete = v6.segments().first().copied().unwrap_or(0);
            // Adresses locales uniques (fc00::/7) et de lien (fe80::/10).
            v6.is_loopback() || tete & 0xfe00 == 0xfc00 || tete & 0xffc0 == 0xfe80
        }
    }
}

/// Ce que le serveur dit de lui-même sans rien demander : de quoi refuser un poste relié d'une
/// autre version avant tout appel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EtatServeur {
    /// Version de GESCO sur le poste principal.
    pub version: String,
}

/// Ce qu'envoie un poste qui veut se relier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DemandeAppairage {
    /// Code affiché sur le poste principal.
    pub code: String,
    /// Nom à donner au poste.
    pub nom: String,
}

/// Refus comptés par adresse (SPEC_PHASE_11_A §4.3) : un appareil qui essaie des codes ou des
/// secrets au hasard est mis en attente, sans gêner les autres postes.
#[derive(Debug, Default)]
struct Refus {
    /// Par adresse : nombre de refus, et début de la minute où ils se comptent.
    comptes: Mutex<HashMap<IpAddr, (u32, Instant)>>,
}

impl Refus {
    /// Vrai si cette adresse a été refusée trop de fois dans la minute.
    fn en_attente(&self, adresse: IpAddr, maintenant: Instant) -> bool {
        self.comptes.lock().is_ok_and(|comptes| {
            comptes.get(&adresse).is_some_and(|(nombre, debut)| {
                *nombre >= REFUS_MAX_PAR_MINUTE
                    && maintenant.saturating_duration_since(*debut) < FENETRE_REFUS
            })
        })
    }

    /// Compte un refus de plus pour cette adresse, et oublie les minutes écoulées.
    fn noter(&self, adresse: IpAddr, maintenant: Instant) {
        let Ok(mut comptes) = self.comptes.lock() else {
            return;
        };
        comptes
            .retain(|_, (_, debut)| maintenant.saturating_duration_since(*debut) < FENETRE_REFUS);
        let compte = comptes.entry(adresse).or_insert((0, maintenant));
        compte.0 = compte.0.saturating_add(1);
    }
}

#[derive(Clone)]
struct Etat {
    application: Arc<Application>,
    reconnaissance: Reconnaissance,
    refus: Arc<Refus>,
}

/// Routes du poste principal : l'état, l'appairage, et la route unique des commandes.
pub fn routeur(application: Arc<Application>, reconnaissance: Reconnaissance) -> Router {
    Router::new()
        .route("/api/etat", get(etat))
        .route("/api/appairage", post(appairage))
        .route("/api/commandes/{nom}", post(commande))
        .layer(DefaultBodyLimit::max(TAILLE_MAX_CORPS))
        .with_state(Etat {
            application,
            reconnaissance,
            refus: Arc::default(),
        })
}

async fn etat(State(etat): State<Etat>) -> Json<EtatServeur> {
    Json(EtatServeur {
        version: etat.application.version_applicative().to_owned(),
    })
}

fn refus(statut: StatusCode, cle: &str) -> Response {
    let erreur = ErreurCommande {
        cle: cle.to_owned(),
        jusqua: None,
        detail: None,
    };
    (statut, Json(erreur)).into_response()
}

/// Jeton de session porté par l'en-tête `Authorization: Bearer …`.
fn jeton(entetes: &HeaderMap) -> Option<Jeton> {
    entetes
        .get(AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(|valeur| Jeton(valeur.to_owned()))
}

/// Identifiant et secret présentés par un poste relié.
fn identite(entetes: &HeaderMap) -> Option<(&str, &str)> {
    let poste = entetes.get(ENTETE_POSTE)?.to_str().ok()?;
    let secret = entetes.get(ENTETE_SECRET)?.to_str().ok()?;
    Some((poste, secret))
}

impl Etat {
    /// Poste qui appelle, s'il est reconnu.
    async fn reconnaitre(&self, adresse: SocketAddr, entetes: &HeaderMap) -> Option<Poste> {
        match &self.reconnaissance {
            Reconnaissance::BoucleLocale(poste) => {
                adresse.ip().is_loopback().then(|| poste.clone())
            }
            Reconnaissance::Postes => {
                if !adresse_privee(adresse.ip()) {
                    return None;
                }
                let (poste, secret) = identite(entetes)?;
                self.application.reconnaitre_poste(poste, secret).await
            }
        }
    }
}

async fn appairage(
    State(etat): State<Etat>,
    ConnectInfo(adresse): ConnectInfo<SocketAddr>,
    demande: Result<Json<DemandeAppairage>, JsonRejection>,
) -> Response {
    if !adresse_privee(adresse.ip()) {
        return refus(StatusCode::FORBIDDEN, CLE_POSTE_INCONNU);
    }
    if etat.refus.en_attente(adresse.ip(), Instant::now()) {
        return refus(StatusCode::TOO_MANY_REQUESTS, CLE_TROP_D_ESSAIS);
    }
    let Ok(Json(demande)) = demande else {
        return refus(StatusCode::UNPROCESSABLE_ENTITY, CLE_ENTREE_INVALIDE);
    };
    match etat
        .application
        .conclure_appairage(&demande.code, &demande.nom)
        .await
    {
        Ok(conclu) => Json(conclu).into_response(),
        Err(erreur) => {
            etat.refus.noter(adresse.ip(), Instant::now());
            etat.application.noter_au_journal_technique(&format!(
                "reseau appairage refuse adresse={} cle={}",
                adresse.ip(),
                erreur.cle
            ));
            (StatusCode::UNPROCESSABLE_ENTITY, Json(erreur)).into_response()
        }
    }
}

async fn commande(
    State(etat): State<Etat>,
    ConnectInfo(adresse): ConnectInfo<SocketAddr>,
    Path(nom): Path<String>,
    entetes: HeaderMap,
    entree: Result<Json<serde_json::Value>, JsonRejection>,
) -> Response {
    if etat.refus.en_attente(adresse.ip(), Instant::now()) {
        return refus(StatusCode::TOO_MANY_REQUESTS, CLE_TROP_D_ESSAIS);
    }
    let Some(poste) = etat.reconnaitre(adresse, &entetes).await else {
        etat.refus.noter(adresse.ip(), Instant::now());
        etat.application.noter_au_journal_technique(&format!(
            "reseau poste non reconnu adresse={} commande={nom}",
            adresse.ip()
        ));
        return refus(StatusCode::UNAUTHORIZED, CLE_POSTE_INCONNU);
    };
    let Ok(Json(entree)) = entree else {
        return refus(StatusCode::UNPROCESSABLE_ENTITY, CLE_ENTREE_INVALIDE);
    };
    let resultat = etat
        .application
        .executer_depuis(Origine::Relie(poste), &nom, jeton(&entetes), entree)
        .await;
    match resultat {
        Ok(sortie) => Json(sortie).into_response(),
        Err(erreur) => (StatusCode::UNPROCESSABLE_ENTITY, Json(erreur)).into_response(),
    }
}

/// Parmi des adresses, celles qu'on donne aux postes à relier : IPv4 de réseau privé, hors la
/// machine elle-même ; les adresses que Windows se donne faute de routeur (169.254) en dernier.
fn adresses_a_donner(trouvees: impl Iterator<Item = IpAddr>) -> Vec<String> {
    let mut retenues: Vec<Ipv4Addr> = trouvees
        .filter_map(|adresse| match adresse {
            IpAddr::V4(v4) if adresse_privee(adresse) && !v4.is_loopback() => Some(v4),
            _ => None,
        })
        .collect();
    retenues.sort_by_key(|v4| (v4.is_link_local(), *v4));
    retenues.dedup();
    retenues.iter().map(ToString::to_string).collect()
}

/// Adresses de cet ordinateur sur le réseau de l'école, d'après son propre nom : ce que la
/// direction lit sur le poste principal et saisit sur le poste à relier. Vide si l'ordinateur
/// n'est sur aucun réseau.
pub fn adresses_du_poste(nom: &str) -> Vec<String> {
    match (nom, 0).to_socket_addrs() {
        Ok(trouvees) => adresses_a_donner(trouvees.map(|adresse| adresse.ip())),
        Err(_) => Vec::new(),
    }
}

/// Ouvre l'écoute sur la machine elle-même ; le port 0 laisse le système en choisir un.
pub async fn ecouter_boucle_locale(port: u16) -> std::io::Result<TcpListener> {
    TcpListener::bind((Ipv4Addr::LOCALHOST, port)).await
}

/// Sert une connexion : chaque appel connaît l'adresse d'où il vient.
async fn servir_connexion<F>(flux: F, adresse: SocketAddr, routes: Router)
where
    F: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let service =
        hyper::service::service_fn(move |mut requete: hyper::Request<hyper::body::Incoming>| {
            requete.extensions_mut().insert(ConnectInfo(adresse));
            routes.clone().call(requete)
        });
    // Une connexion coupée par l'autre bout n'est pas une erreur à remonter.
    let _ = hyper::server::conn::http1::Builder::new()
        .serve_connection(TokioIo::new(flux), service)
        .await;
}

/// Sert les routes sur l'écoute donnée jusqu'à ce que `arret` se termine. Avec `chiffrement`,
/// chaque connexion passe d'abord par le canal chiffré ; sans lui, elle est en clair, ce qui ne
/// convient qu'à la machine elle-même.
pub async fn servir(
    ecoute: TcpListener,
    application: Arc<Application>,
    reconnaissance: Reconnaissance,
    chiffrement: Option<Arc<ServerConfig>>,
    arret: impl Future<Output = ()> + Send,
) -> std::io::Result<()> {
    let routes = routeur(application, reconnaissance);
    let accepteur = chiffrement.map(TlsAcceptor::from);
    tokio::pin!(arret);
    loop {
        let (flux, adresse) = tokio::select! {
            () = &mut arret => return Ok(()),
            accepte = ecoute.accept() => match accepte {
                Ok(connexion) => connexion,
                // Une connexion ratée à l'arrivée n'arrête pas le serveur.
                Err(_) => continue,
            },
        };
        let routes = routes.clone();
        let accepteur = accepteur.clone();
        tokio::spawn(async move {
            match accepteur {
                Some(accepteur) => {
                    if let Ok(Ok(chiffre)) =
                        tokio::time::timeout(DELAI_CANAL, accepteur.accept(flux)).await
                    {
                        servir_connexion(chiffre, adresse, routes).await;
                    }
                }
                None => servir_connexion(flux, adresse, routes).await,
            }
        });
    }
}

/// Ce que le serveur doit servir : le port et le réglage du canal chiffré. Fabrique le
/// certificat et le dépose dans la base s'il n'existe pas encore.
async fn preparer(application: &Application) -> Option<(u16, Arc<ServerConfig>)> {
    let a_servir = application.reseau_a_servir().await.ok()??;
    let (certificat, cle) = match a_servir.certificat {
        Some(paire) => paire,
        None => {
            let (certificat, cle) = chiffrement::fabriquer_certificat().ok()?;
            application
                .enregistrer_certificat_reseau(certificat, cle)
                .await
                .ok()?;
            // Relu dans la base : si un autre l'a déposé entre-temps, c'est le sien qui compte.
            application.reseau_a_servir().await.ok()??.certificat?
        }
    };
    let configuration = chiffrement::configuration_serveur(certificat, cle).ok()?;
    Some((a_servir.port, configuration))
}

/// Tient le serveur du réseau local en accord avec l'état du réseau : il écoute quand la
/// direction l'a allumé, s'arrête quand elle l'éteint, et repart avec un autre certificat quand
/// la clé du réseau est renouvelée. `ou_ecouter` donne l'adresse d'écoute pour le port réglé.
/// Rend l'adresse réellement écoutée, ou `None` quand le serveur est arrêté.
pub fn superviser(
    application: Arc<Application>,
    ou_ecouter: impl Fn(u16) -> SocketAddr + Send + 'static,
) -> watch::Receiver<Option<SocketAddr>> {
    let (publier, adresse) = watch::channel(None);
    tokio::spawn(async move {
        let mut suivi = application.suivre_reseau();
        loop {
            // Marqué comme vu avant de lire l'état : un changement survenu pendant la lecture
            // relance un tour.
            suivi.borrow_and_update();
            if let Some((port, configuration)) = preparer(&application).await {
                match TcpListener::bind(ou_ecouter(port)).await {
                    Ok(ecoute) => {
                        let _ = publier.send(ecoute.local_addr().ok());
                        let mut changement = suivi.clone();
                        let _ = servir(
                            ecoute,
                            Arc::clone(&application),
                            Reconnaissance::Postes,
                            Some(configuration),
                            async move {
                                let _ = changement.changed().await;
                            },
                        )
                        .await;
                        let _ = publier.send(None);
                        continue;
                    }
                    Err(erreur) => application.noter_au_journal_technique(&format!(
                        "reseau ecoute impossible port={port} detail={erreur}"
                    )),
                }
            }
            if suivi.changed().await.is_err() {
                return;
            }
        }
    });
    adresse
}

#[cfg(test)]
mod tests {
    use std::net::Ipv6Addr;

    use axum::http::HeaderValue;

    use super::*;

    #[test]
    fn seules_les_adresses_privees_sont_admises() {
        for privee in [
            "127.0.0.1",
            "10.0.0.7",
            "172.16.4.2",
            "192.168.1.20",
            "169.254.3.3",
        ] {
            let adresse: IpAddr = privee.parse().unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
            assert!(adresse_privee(adresse), "{privee}");
        }
        for publique in ["8.8.8.8", "172.32.0.1", "41.73.96.10", "0.0.0.0"] {
            let adresse: IpAddr = publique.parse().unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST));
            assert!(!adresse_privee(adresse), "{publique}");
        }
        assert!(adresse_privee(IpAddr::V6(Ipv6Addr::LOCALHOST)));
        assert!(adresse_privee(IpAddr::V6(Ipv6Addr::new(
            0xfd00, 0, 0, 0, 0, 0, 0, 1
        ))));
        assert!(adresse_privee(IpAddr::V6(Ipv6Addr::new(
            0xfe80, 0, 0, 0, 0, 0, 0, 1
        ))));
        assert!(!adresse_privee(IpAddr::V6(Ipv6Addr::new(
            0x2001, 0xdb8, 0, 0, 0, 0, 0, 1
        ))));
        // Une adresse IPv4 écrite en IPv6 se juge comme l'adresse IPv4.
        assert!(adresse_privee(IpAddr::V6(
            Ipv4Addr::new(192, 168, 1, 20).to_ipv6_mapped()
        )));
        assert!(!adresse_privee(IpAddr::V6(
            Ipv4Addr::new(8, 8, 8, 8).to_ipv6_mapped()
        )));
    }

    #[test]
    fn une_adresse_refusee_trop_de_fois_attend_la_fin_de_la_minute() {
        let refus = Refus::default();
        let curieux = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 66));
        let caisse = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20));
        let depart = Instant::now();
        for essai in 0..REFUS_MAX_PAR_MINUTE {
            assert!(!refus.en_attente(curieux, depart), "essai {essai}");
            refus.noter(curieux, depart);
        }
        assert!(refus.en_attente(curieux, depart));
        assert!(refus.en_attente(curieux, depart + Duration::from_secs(59)));
        // Les autres postes ne sont pas gênés, et la minute passée, l'adresse est écoutée.
        assert!(!refus.en_attente(caisse, depart));
        assert!(!refus.en_attente(curieux, depart + FENETRE_REFUS));
        // Un refus après la minute repart de zéro.
        refus.noter(curieux, depart + FENETRE_REFUS);
        assert!(!refus.en_attente(curieux, depart + FENETRE_REFUS));
    }

    #[test]
    fn on_ne_donne_que_les_adresses_du_reseau_de_l_ecole() {
        let trouvees = [
            "fe80::1",
            "169.254.7.7",
            "192.168.1.20",
            "127.0.0.1",
            "41.73.96.10",
            "10.0.0.7",
            "192.168.1.20",
        ]
        .into_iter()
        .filter_map(|texte| texte.parse::<IpAddr>().ok());
        assert_eq!(
            adresses_a_donner(trouvees),
            vec!["10.0.0.7", "192.168.1.20", "169.254.7.7"]
        );
        assert!(adresses_du_poste("nom qui n'existe pas").is_empty());
    }

    #[test]
    fn le_jeton_et_l_identite_se_lisent_dans_les_en_tetes() {
        let mut entetes = HeaderMap::new();
        assert_eq!(jeton(&entetes), None);
        assert_eq!(identite(&entetes), None);
        entetes.insert(AUTHORIZATION, HeaderValue::from_static("Bearer abc"));
        assert_eq!(jeton(&entetes), Some(Jeton("abc".to_owned())));
        entetes.insert(AUTHORIZATION, HeaderValue::from_static("Basic abc"));
        assert_eq!(jeton(&entetes), None);
        entetes.insert(
            ENTETE_POSTE,
            HeaderValue::from_static("01ARZ3NDEKTSV4RRFFQ69G5FAV"),
        );
        assert_eq!(identite(&entetes), None, "le secret manque");
        entetes.insert(ENTETE_SECRET, HeaderValue::from_static("secret"));
        assert_eq!(
            identite(&entetes),
            Some(("01ARZ3NDEKTSV4RRFFQ69G5FAV", "secret"))
        );
    }
}
