//! Routes de l'API du portail des élèves : entrer, lire sans code, sortir, accueil. La session
//! de l'élève voyage dans un témoin `HttpOnly`, `SameSite=Strict` ; ni le code ni le jeton ne
//! vont au journal.

use axum::Json;
use axum::extract::State;
use axum::http::header::{COOKIE, SET_COOKIE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use gesco_services::portail::entree::{DemandeEntrer, DemandeLireSansCode, Entree};
use serde_json::json;

use super::EtatPortail;

/// Nom du témoin de session de l'élève.
pub(crate) const TEMOIN: &str = "gesco_eleve";

/// Jeton lu dans le témoin de la requête.
pub(crate) fn jeton(entetes: &HeaderMap) -> Option<String> {
    entetes
        .get_all(COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|paire| paire.trim().split_once('='))
        .find(|(nom, _)| *nom == TEMOIN)
        .map(|(_, valeur)| valeur.to_owned())
        .filter(|j| j.len() == 64 && j.bytes().all(|o| o.is_ascii_hexdigit()))
}

/// Témoin d'une session : sans durée pour « Ce n'est pas mon téléphone », la session finit
/// avec le navigateur ; sinon il dure une journée, le poste principal fermant la session après
/// la durée sans activité.
fn temoin(jeton: &str, ephemere: bool) -> HeaderValue {
    let duree = if ephemere { "" } else { "; Max-Age=86400" };
    HeaderValue::from_str(&format!(
        "{TEMOIN}={jeton}; Path=/; HttpOnly; SameSite=Strict{duree}"
    ))
    .unwrap_or_else(|_| HeaderValue::from_static(""))
}

/// Réponse d'une panne : rien de plus que « réessaie ».
fn panne() -> Response {
    StatusCode::INTERNAL_SERVER_ERROR.into_response()
}

/// Ce que lit le téléphone en arrivant.
pub(crate) async fn accueil(State(etat): State<EtatPortail>, entetes: HeaderMap) -> Response {
    let jeton = jeton(&entetes);
    match etat.application.portail_accueil(jeton.as_deref()).await {
        Ok(Some(accueil)) => Json(accueil).into_response(),
        Ok(None) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
        Err(_) => panne(),
    }
}

/// L'élève entre avec son matricule et son code.
pub(crate) async fn entrer(
    State(etat): State<EtatPortail>,
    Json(demande): Json<DemandeEntrer>,
) -> Response {
    match etat.application.portail_entrer(&demande).await {
        Ok(Some(Entree::Entre { jeton, session })) => {
            let mut reponse = Json(json!({ "eleve": session.vue() })).into_response();
            reponse
                .headers_mut()
                .insert(SET_COOKIE, temoin(&jeton, session.ephemere));
            reponse
        }
        Ok(Some(Entree::Refuse(refus))) => {
            (StatusCode::FORBIDDEN, Json(json!({ "refus": refus }))).into_response()
        }
        Ok(None) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
        Err(_) => panne(),
    }
}

/// L'élève lit sans code, au niveau choisi.
pub(crate) async fn lire_sans_code(
    State(etat): State<EtatPortail>,
    Json(demande): Json<DemandeLireSansCode>,
) -> Response {
    match etat.application.portail_lire_sans_code(&demande).await {
        Ok(Some((jeton, session))) => {
            let mut reponse = Json(json!({ "eleve": session.vue() })).into_response();
            reponse
                .headers_mut()
                .insert(SET_COOKIE, temoin(&jeton, session.ephemere));
            reponse
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(_) => panne(),
    }
}

/// L'élève sort : la session est fermée et le témoin effacé.
pub(crate) async fn sortir(State(etat): State<EtatPortail>, entetes: HeaderMap) -> Response {
    if let Some(jeton) = jeton(&entetes) {
        etat.application.portail_sortir(&jeton);
    }
    let mut reponse = StatusCode::NO_CONTENT.into_response();
    reponse.headers_mut().insert(
        SET_COOKIE,
        HeaderValue::from_static("gesco_eleve=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0"),
    );
    reponse
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_jeton_se_lit_dans_le_temoin() {
        let bon = "a".repeat(64);
        let mut h = HeaderMap::new();
        h.insert(
            COOKIE,
            HeaderValue::from_str(&format!("autre=1; {TEMOIN}={bon}"))
                .unwrap_or_else(|_| HeaderValue::from_static("")),
        );
        assert_eq!(jeton(&h), Some(bon));
        let mut faux = HeaderMap::new();
        faux.insert(COOKIE, HeaderValue::from_static("gesco_eleve=../../x"));
        assert_eq!(jeton(&faux), None);
    }

    #[test]
    fn le_temoin_est_http_only_et_strict() {
        let t = temoin(&"b".repeat(64), true);
        let texte = t.to_str().unwrap_or_default();
        assert!(texte.contains("HttpOnly") && texte.contains("SameSite=Strict"));
        assert!(!texte.contains("Max-Age"));
        let t = temoin(&"b".repeat(64), false);
        assert!(t.to_str().unwrap_or_default().contains("Max-Age=86400"));
    }
}
