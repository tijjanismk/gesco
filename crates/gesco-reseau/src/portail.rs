//! Serveur du portail des élèves (SPEC_PHASE_12_LOT_4 §3 ; ADR « portail des élèves séparé de
//! la gestion » et « en clair sur le Wi-Fi ») : un second routeur, sur son propre port, avec ses
//! seules routes. Aucune ne mène à la route des commandes ni à l'appairage des postes : un
//! téléphone n'atteint jamais la gestion.
//!
//! Le portail écoute en clair : il ne sert que des leçons, et le code d'un élève n'ouvre rien
//! d'autre. Il ne répond qu'aux adresses du réseau privé, comme le serveur des postes.

use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use std::path::{Path as Chemin, PathBuf};
use std::sync::Arc;

use axum::Router;
use axum::extract::{Path, Request, State};
use axum::http::header::{
    ACCEPT_ENCODING, CACHE_CONTROL, CONTENT_ENCODING, CONTENT_SECURITY_POLICY, CONTENT_TYPE,
    REFERRER_POLICY, RETRY_AFTER, VARY, X_CONTENT_TYPE_OPTIONS, X_FRAME_OPTIONS,
};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::{MethodRouter, get, post};
use gesco_core::bibliotheque::portail::{
    ATTENTE_PLACE_FICHIER_MS, ATTENTE_PLACE_MS, RELANCE_S, REQUETES_SIMULTANEES,
};
use gesco_services::Application;
use gesco_services::portail::lecture::prendre_place;
use tokio::net::TcpListener;
use tokio::sync::watch;

use crate::adresse_privee;

mod api;

/// Port du portail quand il est libre : l'adresse à taper reste courte.
pub const PORT_PREFERE: u16 = gesco_core::bibliotheque::portail::PORT_PREFERE;

/// Port du portail quand le 80 est pris.
pub const PORT_SECOURS: u16 = gesco_core::bibliotheque::portail::PORT_SECOURS;

/// Politique de contenu : rien ne vient d'ailleurs que du portail lui-même. Les styles en ligne
/// sont admis pour les formules (KaTeX place ses glyphes par des styles).
const POLITIQUE_CONTENU: &str = "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; media-src 'self' blob:; font-src 'self'; connect-src 'self'; object-src 'none'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'";

/// Page servie à la racine tant que les écrans des élèves ne sont pas livrés avec le poste :
/// elle dit seulement que le portail répond, pour un essai depuis un vrai téléphone.
const PAGE_ESSAI: &str = r#"<!doctype html>
<html lang="fr">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>GESCO</title>
<style>
body { margin: 0; font-family: sans-serif; background: #ffffff; color: #1b2a3a; }
main { max-width: 32rem; margin: 0 auto; padding: 2rem 1rem; }
h1 { font-size: 2rem; color: #123a63; margin: 0 0 1rem; }
p { font-size: 1.125rem; line-height: 1.5; }
</style>
</head>
<body>
<main>
<h1>GESCO répond</h1>
<p>Ce téléphone atteint l'ordinateur de l'école.</p>
<p>Les cours s'afficheront ici.</p>
</main>
</body>
</html>
"#;

/// Page servie à la racine quand le poste est occupé : la phrase des écrans de l'élève, et la
/// page se relance seule (`RELANCE_S` secondes) ; rien ne vient d'ailleurs.
const PAGE_OCCUPEE: &str = r#"<!doctype html>
<html lang="fr">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta http-equiv="refresh" content="5">
<title>Les cours</title>
<style>
body { margin: 0; font-family: sans-serif; background: #f1f3f7; color: #1e2533; }
main { max-width: 32rem; margin: 0 auto; padding: 2rem 1rem; }
p { font-size: 1.25rem; line-height: 1.5; text-align: center; }
@media (prefers-color-scheme: dark) { body { background: #0f1828; color: #e3e8f2; } }
</style>
</head>
<body>
<main>
<p role="status">Beaucoup d'élèves sont connectés. Réessaie dans un instant.</p>
</main>
</body>
</html>
"#;

/// Routes du portail, et rien d'autre (SPEC_PHASE_12_LOT_4 §3) : méthode et chemin. Le
/// routeur est construit depuis cette liste ; un essai vérifie qu'elle est complète et que les
/// routes de la gestion n'y répondent pas.
pub const ROUTES_PORTAIL: &[(&str, &str)] = &[
    ("GET", "/"),
    ("GET", "/assets/{fichier}"),
    ("GET", "/eleve/api/accueil"),
    ("POST", "/eleve/api/entrer"),
    ("POST", "/eleve/api/lire-sans-code"),
    ("POST", "/eleve/api/sortir"),
    ("GET", "/eleve/api/catalogue"),
    ("GET", "/eleve/api/contenu/{id}"),
    ("GET", "/eleve/api/fichier/{empreinte}"),
    ("GET", "/eleve/api/pdf/{id}"),
    ("POST", "/eleve/api/avancement"),
    ("GET", "/eleve/api/mon-avancement"),
    ("GET", "/eleve/api/quiz/{id}"),
    ("GET", "/eleve/api/revision/{id}"),
    ("POST", "/eleve/api/tentative"),
];

/// Ce que chaque route du portail connaît.
#[derive(Clone)]
pub struct EtatPortail {
    application: Arc<Application>,
    /// Dossier des écrans des élèves construits (`index.html`, `assets/`) ; sans lui, la racine
    /// sert la page d'essai.
    pages: Option<Arc<PathBuf>>,
    /// Places des requêtes servies en même temps ([`REQUETES_SIMULTANEES`]).
    places: Arc<tokio::sync::Semaphore>,
}

/// Les routes du portail, avec leur méthode et leur chemin, dans l'ordre de [`ROUTES_PORTAIL`].
fn routes() -> Vec<(&'static str, &'static str, MethodRouter<EtatPortail>)> {
    vec![
        ("GET", "/", get(racine)),
        ("GET", "/assets/{fichier}", get(fichier_des_pages)),
        ("GET", "/eleve/api/accueil", get(api::accueil)),
        ("POST", "/eleve/api/entrer", post(api::entrer)),
        (
            "POST",
            "/eleve/api/lire-sans-code",
            post(api::lire_sans_code),
        ),
        ("POST", "/eleve/api/sortir", post(api::sortir)),
        ("GET", "/eleve/api/catalogue", get(api::catalogue)),
        ("GET", "/eleve/api/contenu/{id}", get(api::contenu)),
        ("GET", "/eleve/api/fichier/{empreinte}", get(api::fichier)),
        ("GET", "/eleve/api/pdf/{id}", get(api::pdf)),
        ("POST", "/eleve/api/avancement", post(api::avancement)),
        ("GET", "/eleve/api/mon-avancement", get(api::mon_avancement)),
        ("GET", "/eleve/api/quiz/{id}", get(api::quiz)),
        ("GET", "/eleve/api/revision/{id}", get(api::revision)),
        ("POST", "/eleve/api/tentative", post(api::tentative)),
    ]
}

/// Routeur du portail : ses seules routes, et les en-têtes de chaque réponse. `pages` est le
/// dossier des écrans des élèves construits, s'il est livré.
pub fn routeur_portail(application: Arc<Application>, pages: Option<PathBuf>) -> Router {
    routeur_borne(application, pages, REQUETES_SIMULTANEES)
}

/// Routeur du portail avec `places` requêtes servies en même temps (les essais en prennent
/// moins pour voir la borne).
fn routeur_borne(application: Arc<Application>, pages: Option<PathBuf>, places: usize) -> Router {
    let etat = EtatPortail {
        application,
        pages: pages.map(Arc::new),
        places: Arc::new(tokio::sync::Semaphore::new(places)),
    };
    let mut routeur = Router::new();
    for (_, chemin, route) in routes() {
        routeur = routeur.route(chemin, route);
    }
    routeur
        .fallback(introuvable)
        .layer(axum::middleware::from_fn_with_state(etat.clone(), borner))
        .layer(axum::middleware::map_response(entetes))
        .with_state(etat)
}

/// Borne les requêtes servies en même temps (ADR « charge ») : chacune prend une place,
/// rendue quand la réponse est prête. Sans place libre après l'attente permise : la page
/// d'attente à la racine, 429 ailleurs, avec `Retry-After`. La gestion passe d'abord : le
/// portail ne prend jamais plus que ses places.
async fn borner(State(etat): State<EtatPortail>, requete: Request, suite: Next) -> Response {
    let chemin = requete.uri().path();
    let attente = if chemin.starts_with("/assets/") {
        ATTENTE_PLACE_FICHIER_MS
    } else {
        ATTENTE_PLACE_MS
    };
    let racine = chemin == "/";
    let Some(_place) = prendre_place(&etat.places, attente).await else {
        return if racine { occupe_page() } else { occupe_api() };
    };
    suite.run(requete).await
}

/// En-tête `Retry-After` du poste occupé.
fn relance() -> HeaderValue {
    HeaderValue::from(RELANCE_S)
}

/// Réponse « occupé » de l'API et des fichiers : 429, réessayer dans quelques secondes. Les
/// écrans de l'élève la montrent comme « Beaucoup d'élèves sont connectés ».
pub(crate) fn occupe_api() -> Response {
    let mut reponse = StatusCode::TOO_MANY_REQUESTS.into_response();
    reponse.headers_mut().insert(RETRY_AFTER, relance());
    reponse
}

/// Page d'attente de la racine, qui se relance seule.
fn occupe_page() -> Response {
    let mut reponse = (
        StatusCode::TOO_MANY_REQUESTS,
        [(
            CONTENT_TYPE,
            HeaderValue::from_static("text/html; charset=utf-8"),
        )],
        PAGE_OCCUPEE,
    )
        .into_response();
    reponse.headers_mut().insert(RETRY_AFTER, relance());
    reponse
}

/// En-têtes de toute réponse du portail : politique de contenu stricte, aucune page dans un
/// cadre, aucune adresse transmise ; les réponses de l'API ne se gardent pas en cache.
async fn entetes(mut reponse: Response) -> Response {
    let h = reponse.headers_mut();
    h.insert(
        CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(POLITIQUE_CONTENU),
    );
    h.insert(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    h.insert(X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    h.insert(REFERRER_POLICY, HeaderValue::from_static("no-referrer"));
    if !h.contains_key(CACHE_CONTROL) {
        h.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    }
    reponse
}

async fn introuvable() -> Response {
    StatusCode::NOT_FOUND.into_response()
}

/// La page des élèves, ou la page d'essai tant qu'elle n'est pas livrée.
async fn racine(State(etat): State<EtatPortail>, entetes: HeaderMap) -> Response {
    if let Some(pages) = &etat.pages
        && let Some(reponse) = servir_fichier(pages, "index.html", &entetes, false).await
    {
        return reponse;
    }
    (
        [(
            CONTENT_TYPE,
            HeaderValue::from_static("text/html; charset=utf-8"),
        )],
        PAGE_ESSAI,
    )
        .into_response()
}

/// Un fichier des écrans des élèves : son nom porte son empreinte (Vite), il ne change jamais.
async fn fichier_des_pages(
    State(etat): State<EtatPortail>,
    Path(fichier): Path<String>,
    entetes: HeaderMap,
) -> Response {
    let Some(pages) = &etat.pages else {
        return introuvable().await;
    };
    if !nom_de_fichier_sur(&fichier) {
        return introuvable().await;
    }
    match servir_fichier(pages, &format!("assets/{fichier}"), &entetes, true).await {
        Some(reponse) => reponse,
        None => introuvable().await,
    }
}

/// Vrai pour un nom de fichier simple : lettres, chiffres, point, tiret, souligné ; jamais un
/// chemin qui remonte.
fn nom_de_fichier_sur(nom: &str) -> bool {
    !nom.is_empty()
        && !nom.starts_with('.')
        && nom.len() <= 128
        && nom
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

/// Type d'un fichier des pages, d'après son extension.
fn type_de_fichier(nom: &str) -> &'static str {
    match nom.rsplit('.').next().unwrap_or_default() {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ttf" => "font/ttf",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "ico" => "image/x-icon",
        "webmanifest" | "json" => "application/manifest+json",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

/// Sert un fichier du dossier des pages, compressé quand une version `.gz` existe et que le
/// navigateur l'accepte. `None` si le fichier n'existe pas.
async fn servir_fichier(
    pages: &Chemin,
    relatif: &str,
    entetes: &HeaderMap,
    permanent: bool,
) -> Option<Response> {
    let chemin = pages.join(relatif);
    let accepte_gzip = entetes
        .get(ACCEPT_ENCODING)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.contains("gzip"));
    let compresse = PathBuf::from(format!("{}.gz", chemin.display()));
    let (contenu, gzip) = match accepte_gzip {
        true => match tokio::fs::read(&compresse).await {
            Ok(contenu) => (contenu, true),
            Err(_) => (tokio::fs::read(&chemin).await.ok()?, false),
        },
        false => (tokio::fs::read(&chemin).await.ok()?, false),
    };
    let mut reponse = contenu.into_response();
    let h = reponse.headers_mut();
    h.insert(
        CONTENT_TYPE,
        HeaderValue::from_static(type_de_fichier(relatif)),
    );
    h.insert(
        CACHE_CONTROL,
        HeaderValue::from_static(if permanent {
            "public, max-age=31536000, immutable"
        } else {
            "no-cache"
        }),
    );
    h.insert(VARY, HeaderValue::from_static("Accept-Encoding"));
    if gzip {
        h.insert(CONTENT_ENCODING, HeaderValue::from_static("gzip"));
    }
    Some(reponse)
}

/// Sert le portail sur l'écoute donnée jusqu'à ce que `arret` se termine. Une connexion venue
/// d'une adresse qui n'est pas du réseau privé est fermée sans réponse.
pub async fn servir_portail(
    ecoute: TcpListener,
    application: Arc<Application>,
    pages: Option<PathBuf>,
    arret: impl Future<Output = ()> + Send,
) -> std::io::Result<()> {
    servir_portail_avec_places(ecoute, application, pages, REQUETES_SIMULTANEES, arret).await
}

/// [`servir_portail`] avec `places` requêtes servies en même temps : les essais en prennent
/// une seule pour voir la page d'attente sans lancer cinquante requêtes.
#[doc(hidden)]
pub async fn servir_portail_avec_places(
    ecoute: TcpListener,
    application: Arc<Application>,
    pages: Option<PathBuf>,
    places: usize,
    arret: impl Future<Output = ()> + Send,
) -> std::io::Result<()> {
    let routes = routeur_borne(application, pages, places);
    tokio::pin!(arret);
    loop {
        let (flux, adresse) = tokio::select! {
            () = &mut arret => return Ok(()),
            accepte = ecoute.accept() => match accepte {
                Ok(connexion) => connexion,
                Err(_) => continue,
            },
        };
        if !adresse_privee(adresse.ip()) {
            continue;
        }
        let routes = routes.clone();
        tokio::spawn(crate::servir_connexion(flux, adresse, routes));
    }
}

/// Ouvre l'écoute du portail sur le premier port libre parmi `ports`.
async fn ecouter(ou: IpAddr, ports: &[u16]) -> Option<TcpListener> {
    for port in ports {
        if let Ok(ecoute) = TcpListener::bind(SocketAddr::new(ou, *port)).await {
            return Some(ecoute);
        }
    }
    None
}

/// Tient le serveur du portail en accord avec ses réglages : il écoute quand la direction l'a
/// allumé (avec le module Bibliothèque et le réseau local), s'arrête quand elle l'éteint. Il
/// essaie les ports dans l'ordre donné (80, puis 47 801). Rend l'adresse réellement écoutée,
/// ou `None` quand le portail est arrêté.
pub fn superviser_portail(
    application: Arc<Application>,
    ou: IpAddr,
    ports: Vec<u16>,
    pages: Option<PathBuf>,
) -> watch::Receiver<Option<SocketAddr>> {
    let (publier, adresse) = watch::channel(None);
    tokio::spawn(async move {
        let mut reglages = application.suivre_portail();
        let mut reseau = application.suivre_reseau();
        loop {
            reglages.borrow_and_update();
            reseau.borrow_and_update();
            if application.portail_a_servir().await {
                match ecouter(ou, &ports).await {
                    Some(ecoute) => {
                        let ecoutee = ecoute.local_addr().ok();
                        application.declarer_ecoute_portail(ecoutee.map(|a| a.port()));
                        application.noter_au_journal_technique(&format!(
                            "portail ecoute port={}",
                            ecoutee.map(|a| a.port()).unwrap_or_default()
                        ));
                        let _ = publier.send(ecoutee);
                        let mut suivi_reglages = reglages.clone();
                        let mut suivi_reseau = reseau.clone();
                        let pour_arret = Arc::clone(&application);
                        let _ = servir_portail(
                            ecoute,
                            Arc::clone(&application),
                            pages.clone(),
                            async move {
                                // Un réglage qui ne l'éteint pas laisse le portail écouter.
                                loop {
                                    tokio::select! {
                                        r = suivi_reglages.changed() => if r.is_err() { return },
                                        r = suivi_reseau.changed() => if r.is_err() { return },
                                    }
                                    if !pour_arret.portail_a_servir().await {
                                        return;
                                    }
                                }
                            },
                        )
                        .await;
                        application.declarer_ecoute_portail(None);
                        application.noter_au_journal_technique("portail arrete");
                        let _ = publier.send(None);
                        continue;
                    }
                    None => application.noter_au_journal_technique(&format!(
                        "portail ecoute impossible ports={ports:?}"
                    )),
                }
            }
            tokio::select! {
                r = reglages.changed() => if r.is_err() { return },
                r = reseau.changed() => if r.is_err() { return },
            }
        }
    });
    adresse
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_routeur_suit_la_liste_ecrite() {
        let construites: Vec<(&str, &str)> = routes()
            .into_iter()
            .map(|(methode, chemin, _)| (methode, chemin))
            .collect();
        assert_eq!(construites, ROUTES_PORTAIL);
        for (_, chemin) in ROUTES_PORTAIL {
            assert!(
                *chemin == "/" || chemin.starts_with("/assets/") || chemin.starts_with("/eleve/"),
                "{chemin}"
            );
        }
    }

    #[test]
    fn seuls_les_noms_simples_sont_servis() {
        assert!(nom_de_fichier_sur("index-4f2a.js"));
        assert!(nom_de_fichier_sur("andika-regular.woff2"));
        for faux in ["", "..", "../secret", ".env", "a/b", "a\\b", "%2e%2e"] {
            assert!(!nom_de_fichier_sur(faux), "{faux}");
        }
    }

    #[test]
    fn la_page_d_attente_se_relance_au_rythme_annonce() {
        assert!(PAGE_OCCUPEE.contains(&format!("content=\"{RELANCE_S}\"")));
        assert!(
            PAGE_OCCUPEE.contains("Beaucoup d'élèves sont connectés. Réessaie dans un instant.")
        );
        // Rien ne vient d'ailleurs : ni script, ni police, ni image.
        assert!(!PAGE_OCCUPEE.contains("<script") && !PAGE_OCCUPEE.contains("//"));
        let r = occupe_api();
        assert_eq!(r.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(
            r.headers().get(RETRY_AFTER),
            Some(&HeaderValue::from(RELANCE_S))
        );
        let r = occupe_page();
        assert_eq!(r.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(
            r.headers().get(RETRY_AFTER),
            Some(&HeaderValue::from(RELANCE_S))
        );
    }

    #[test]
    fn chaque_fichier_a_son_type() {
        assert_eq!(type_de_fichier("index.html"), "text/html; charset=utf-8");
        assert_eq!(type_de_fichier("assets/a.woff2"), "font/woff2");
        assert_eq!(type_de_fichier("x.bin"), "application/octet-stream");
    }
}
