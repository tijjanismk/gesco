//! Serveur du réseau local (ADR-014, ADR du 10 novembre 2026, SPEC_PHASE_11_A §3) : le poste
//! principal répond aux postes reliés. Une seule route de commandes, qui remet chaque appel à
//! la frontière de `gesco-services` : même garde, mêmes gestionnaires que l'écran du poste.
//!
//! Étape 1 : le serveur n'écoute que la machine elle-même. L'appairage, le canal chiffré et le
//! secret de chaque poste viennent à l'étape 2, derrière l'interface [`Reconnaissance`].

use std::future::Future;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;

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
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

/// Port du poste principal, tant que l'école n'en règle pas un autre.
pub const PORT_PAR_DEFAUT: u16 = 47_800;

/// Taille maximale du corps d'un appel : photos, pièces et fichiers d'import.
pub const TAILLE_MAX_CORPS: usize = 32 * 1024 * 1024;

/// Clé d'erreur d'un appel venu d'un poste que le poste principal ne reconnaît pas.
pub const CLE_POSTE_INCONNU: &str = "reseau.poste_inconnu";

/// Clé d'erreur d'un corps qui n'est pas du JSON : la même que pour une entrée mal formée.
const CLE_ENTREE_INVALIDE: &str = "commande.entree_invalide";

/// Reconnaît le poste qui appelle. Le nom rendu signe ses écritures au journal d'audit : il ne
/// vient jamais d'un nom que le poste appelant choisirait lui-même.
pub trait Reconnaissance: Send + Sync + 'static {
    /// Poste reconnu derrière cette adresse et ces en-têtes, ou `None` pour un inconnu.
    fn reconnaitre(&self, adresse: SocketAddr, entetes: &HeaderMap) -> Option<Poste>;
}

/// Reconnaissance de l'étape 1 : seule la machine elle-même est admise, sous un nom fixe.
#[derive(Debug, Clone)]
pub struct BoucleLocale {
    poste: Poste,
}

impl BoucleLocale {
    /// Tout appel venu de la machine elle-même est attribué à ce poste.
    pub fn nouvelle(poste: Poste) -> Self {
        Self { poste }
    }
}

impl Reconnaissance for BoucleLocale {
    fn reconnaitre(&self, adresse: SocketAddr, _entetes: &HeaderMap) -> Option<Poste> {
        adresse.ip().is_loopback().then(|| self.poste.clone())
    }
}

/// Ce que le serveur dit de lui-même sans rien demander : de quoi refuser un poste relié d'une
/// autre version avant tout appel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EtatServeur {
    /// Version de GESCO sur le poste principal.
    pub version: String,
}

#[derive(Clone)]
struct Etat {
    application: Arc<Application>,
    reconnaissance: Arc<dyn Reconnaissance>,
}

/// Routes du poste principal : l'état, et la route unique des commandes.
pub fn routeur(application: Arc<Application>, reconnaissance: Arc<dyn Reconnaissance>) -> Router {
    Router::new()
        .route("/api/etat", get(etat))
        .route("/api/commandes/{nom}", post(commande))
        .layer(DefaultBodyLimit::max(TAILLE_MAX_CORPS))
        .with_state(Etat {
            application,
            reconnaissance,
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

async fn commande(
    State(etat): State<Etat>,
    ConnectInfo(adresse): ConnectInfo<SocketAddr>,
    Path(nom): Path<String>,
    entetes: HeaderMap,
    entree: Result<Json<serde_json::Value>, JsonRejection>,
) -> Response {
    let Some(poste) = etat.reconnaissance.reconnaitre(adresse, &entetes) else {
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

/// Ouvre l'écoute sur la machine elle-même ; le port 0 laisse le système en choisir un.
pub async fn ecouter_boucle_locale(port: u16) -> std::io::Result<TcpListener> {
    TcpListener::bind((Ipv4Addr::LOCALHOST, port)).await
}

/// Sert les routes sur l'écoute donnée jusqu'à ce que `arret` se termine ; les appels en cours
/// s'achèvent avant le retour.
pub async fn servir(
    ecoute: TcpListener,
    application: Arc<Application>,
    reconnaissance: Arc<dyn Reconnaissance>,
    arret: impl Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    let service =
        routeur(application, reconnaissance).into_make_service_with_connect_info::<SocketAddr>();
    axum::serve(ecoute, service)
        .with_graceful_shutdown(arret)
        .await
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    #[test]
    fn la_boucle_locale_n_admet_que_la_machine_elle_meme() {
        let reconnaissance = BoucleLocale::nouvelle(Poste("caisse".to_owned()));
        let entetes = HeaderMap::new();
        let locale = SocketAddr::from((Ipv4Addr::LOCALHOST, 50_000));
        assert_eq!(
            reconnaissance.reconnaitre(locale, &entetes),
            Some(Poste("caisse".to_owned()))
        );
        let voisine = SocketAddr::from((Ipv4Addr::new(192, 168, 1, 20), 50_000));
        assert_eq!(reconnaissance.reconnaitre(voisine, &entetes), None);
    }

    #[test]
    fn le_jeton_se_lit_dans_l_en_tete_d_autorisation() {
        let mut entetes = HeaderMap::new();
        assert_eq!(jeton(&entetes), None);
        entetes.insert(AUTHORIZATION, HeaderValue::from_static("Bearer abc"));
        assert_eq!(jeton(&entetes), Some(Jeton("abc".to_owned())));
        entetes.insert(AUTHORIZATION, HeaderValue::from_static("Basic abc"));
        assert_eq!(jeton(&entetes), None);
    }
}
