//! Routes de l'API du portail des élèves : entrer, lire sans code, sortir, accueil, catalogue,
//! contenu, fichiers, PDF et avancement. La session
//! de l'élève voyage dans un témoin `HttpOnly`, `SameSite=Strict` ; ni le code ni le jeton ne
//! vont au journal.

use std::pin::Pin;
use std::task::{Context, Poll};

use axum::Json;
use axum::body::{Body, Bytes, HttpBody};
use axum::extract::{Path, State};
use axum::http::header::{
    ACCEPT_RANGES, CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_RANGE, CONTENT_TYPE, COOKIE, RANGE,
    SET_COOKIE,
};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use gesco_services::portail::devoirs::DemandeRendu;
use gesco_services::portail::entree::{DemandeEntrer, DemandeLireSansCode, Entree};
use gesco_services::portail::lecture::{FichierEnvoye, FichierLuPortail, Plage, ReponseLecture};
use gesco_services::portail::quiz::DemandeTentative;
use gesco_services::portail::suivi::DemandeAvancement;
use http_body_util::Full;
use hyper::body::{Frame, SizeHint};
use serde_json::json;
use tokio::sync::OwnedSemaphorePermit;

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

/// Réponse d'une route de lecture qui n'a rien lu.
fn sans_lecture<T>(r: &ReponseLecture<T>) -> Option<Response> {
    match r {
        ReponseLecture::Ferme => Some(StatusCode::SERVICE_UNAVAILABLE.into_response()),
        ReponseLecture::SansSession => Some(StatusCode::UNAUTHORIZED.into_response()),
        ReponseLecture::Introuvable => Some(StatusCode::NOT_FOUND.into_response()),
        ReponseLecture::Occupe => Some(super::occupe_api()),
        ReponseLecture::Lu(_) => None,
    }
}

/// Vrai pour un identifiant de contenu : lettres, chiffres, tiret.
fn identifiant_sur(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|o| o.is_ascii_alphanumeric() || o == b'-')
}

/// Le catalogue de l'élève.
pub(crate) async fn catalogue(State(etat): State<EtatPortail>, entetes: HeaderMap) -> Response {
    let jeton = jeton(&entetes);
    match etat.application.portail_catalogue(jeton.as_deref()).await {
        Ok(ReponseLecture::Lu(catalogue)) => Json(catalogue).into_response(),
        Ok(autre) => sans_lecture(&autre).unwrap_or_else(panne),
        Err(_) => panne(),
    }
}

/// Un contenu visible, en arbre de blocs.
pub(crate) async fn contenu(
    State(etat): State<EtatPortail>,
    Path(id): Path<String>,
    entetes: HeaderMap,
) -> Response {
    if !identifiant_sur(&id) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let jeton = jeton(&entetes);
    match etat
        .application
        .portail_contenu(jeton.as_deref(), &id)
        .await
    {
        Ok(ReponseLecture::Lu(contenu)) => Json(contenu).into_response(),
        Ok(autre) => sans_lecture(&autre).unwrap_or_else(panne),
        Err(_) => panne(),
    }
}

/// Plage d'octets demandée (`Range: bytes=début-fin`) ; seule la première plage est lue.
fn plage(entetes: &HeaderMap) -> Option<Plage> {
    let valeur = entetes.get(RANGE)?.to_str().ok()?;
    let (debut, fin) = valeur
        .trim()
        .strip_prefix("bytes=")?
        .split(',')
        .next()?
        .split_once('-')?;
    let debut: u64 = debut.trim().parse().ok()?;
    let fin = match fin.trim() {
        "" => None,
        f => Some(f.parse::<u64>().ok()?),
    };
    if fin.is_some_and(|f| f < debut) {
        return None;
    }
    Some((debut, fin))
}

/// Un fichier appelé par un contenu visible : entier, ou un morceau d'un son ou d'une vidéo.
/// Son nom est son empreinte : il ne change jamais et se garde en cache.
pub(crate) async fn fichier(
    State(etat): State<EtatPortail>,
    Path(empreinte): Path<String>,
    entetes: HeaderMap,
) -> Response {
    let jeton = jeton(&entetes);
    let lu = etat
        .application
        .portail_fichier(jeton.as_deref(), &empreinte, plage(&entetes))
        .await;
    let (morceau, place) = match lu {
        Ok(ReponseLecture::Lu(FichierEnvoye {
            lu: FichierLuPortail::Servi(m),
            place,
        })) => (m, place),
        Ok(ReponseLecture::Lu(FichierEnvoye {
            lu: FichierLuPortail::HorsPlage { taille },
            ..
        })) => {
            let mut reponse = StatusCode::RANGE_NOT_SATISFIABLE.into_response();
            if let Ok(v) = HeaderValue::from_str(&format!("bytes */{taille}")) {
                reponse.headers_mut().insert(CONTENT_RANGE, v);
            }
            return reponse;
        }
        Ok(ReponseLecture::Lu(FichierEnvoye {
            lu: FichierLuPortail::Absent,
            ..
        })) => {
            return StatusCode::NOT_FOUND.into_response();
        }
        Ok(autre) => return sans_lecture(&autre).unwrap_or_else(panne),
        Err(_) => return panne(),
    };
    let fin = (morceau.debut + morceau.octets.len() as u64).saturating_sub(1);
    let partiel = morceau.partiel;
    let plage_servie = format!("bytes {}-{fin}/{}", morceau.debut, morceau.taille);
    let type_mime = HeaderValue::from_str(&morceau.type_mime)
        .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream"));
    let statut = if partiel {
        StatusCode::PARTIAL_CONTENT
    } else {
        StatusCode::OK
    };
    // Un morceau de vidéo garde sa place jusqu'à ce que le corps soit envoyé et lâché.
    let corps = match place {
        Some(place) => Body::new(CorpsTenu {
            corps: Full::new(Bytes::from(morceau.octets)),
            _place: place,
        }),
        None => Body::from(morceau.octets),
    };
    let mut reponse = (statut, corps).into_response();
    let h = reponse.headers_mut();
    h.insert(CONTENT_TYPE, type_mime);
    h.insert(ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    h.insert(
        CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=31536000, immutable"),
    );
    h.insert(CONTENT_DISPOSITION, HeaderValue::from_static("inline"));
    if partiel && let Ok(v) = HeaderValue::from_str(&plage_servie) {
        h.insert(CONTENT_RANGE, v);
    }
    reponse
}

/// Corps d'une réponse qui tient une place (vidéo) jusqu'à ce qu'il soit lâché, c'est-à-dire
/// envoyé ou abandonné par le téléphone.
struct CorpsTenu {
    corps: Full<Bytes>,
    _place: OwnedSemaphorePermit,
}

impl HttpBody for CorpsTenu {
    type Data = Bytes;
    type Error = std::convert::Infallible;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        Pin::new(&mut self.corps).poll_frame(cx)
    }

    fn is_end_stream(&self) -> bool {
        self.corps.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.corps.size_hint()
    }
}

/// Nom de fichier pour l'en-tête `Content-Disposition` : une forme ASCII et la forme UTF-8.
fn disposition_pdf(nom: &str) -> HeaderValue {
    let ascii: String = nom
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let encode: String = nom
        .bytes()
        .map(|o| {
            if o.is_ascii_alphanumeric() || matches!(o, b'-' | b'_' | b'.') {
                char::from(o).to_string()
            } else {
                format!("%{o:02X}")
            }
        })
        .collect();
    HeaderValue::from_str(&format!(
        "attachment; filename=\"{ascii}\"; filename*=UTF-8''{encode}"
    ))
    .unwrap_or_else(|_| HeaderValue::from_static("attachment; filename=\"Lecon.pdf\""))
}

/// Le PDF « leçon complète » d'un contenu visible.
pub(crate) async fn pdf(
    State(etat): State<EtatPortail>,
    Path(id): Path<String>,
    entetes: HeaderMap,
) -> Response {
    if !identifiant_sur(&id) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let jeton = jeton(&entetes);
    match etat.application.portail_pdf(jeton.as_deref(), &id).await {
        Ok(ReponseLecture::Lu((nom, pdf))) => {
            let mut reponse = pdf.into_response();
            let h = reponse.headers_mut();
            h.insert(CONTENT_TYPE, HeaderValue::from_static("application/pdf"));
            h.insert(CONTENT_DISPOSITION, disposition_pdf(&nom));
            reponse
        }
        Ok(autre) => sans_lecture(&autre).unwrap_or_else(panne),
        Err(_) => panne(),
    }
}

/// Un geste d'avancement : leçon ouverte ou finie. Rien n'est jamais refusé à l'élève pour le
/// suivi : sans code ou suivi éteint, la réponse est la même et rien ne s'écrit.
pub(crate) async fn avancement(
    State(etat): State<EtatPortail>,
    entetes: HeaderMap,
    Json(demande): Json<DemandeAvancement>,
) -> Response {
    if !identifiant_sur(&demande.contenu_id) || demande.version.len() > 128 {
        return StatusCode::NOT_FOUND.into_response();
    }
    let jeton = jeton(&entetes);
    match etat
        .application
        .portail_avancement(jeton.as_deref(), &demande)
        .await
    {
        Ok(ReponseLecture::Lu(())) => StatusCode::NO_CONTENT.into_response(),
        Ok(autre) => sans_lecture(&autre).unwrap_or_else(panne),
        Err(_) => panne(),
    }
}

/// Le quiz d'un contenu visible.
pub(crate) async fn quiz(
    State(etat): State<EtatPortail>,
    Path(id): Path<String>,
    entetes: HeaderMap,
) -> Response {
    if !identifiant_sur(&id) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let jeton = jeton(&entetes);
    match etat.application.portail_quiz(jeton.as_deref(), &id).await {
        Ok(ReponseLecture::Lu(q)) => Json(q).into_response(),
        Ok(autre) => sans_lecture(&autre).unwrap_or_else(panne),
        Err(_) => panne(),
    }
}

/// « Réviser le chapitre » : dix questions tirées des quiz du chapitre.
pub(crate) async fn revision(
    State(etat): State<EtatPortail>,
    Path(id): Path<String>,
    entetes: HeaderMap,
) -> Response {
    if !identifiant_sur(&id) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let jeton = jeton(&entetes);
    match etat
        .application
        .portail_revision(jeton.as_deref(), &id)
        .await
    {
        Ok(ReponseLecture::Lu(q)) => Json(q).into_response(),
        Ok(autre) => sans_lecture(&autre).unwrap_or_else(panne),
        Err(_) => panne(),
    }
}

/// Un quiz passé jusqu'au bout. Comme l'avancement, rien n'est refusé à l'élève : sans code ou
/// suivi éteint, la réponse est la même et rien ne s'écrit.
pub(crate) async fn tentative(
    State(etat): State<EtatPortail>,
    entetes: HeaderMap,
    Json(demande): Json<DemandeTentative>,
) -> Response {
    let sures = demande.contenu_id.as_deref().is_none_or(identifiant_sur)
        && demande.chapitre_id.as_deref().is_none_or(identifiant_sur)
        && demande
            .reponses
            .iter()
            .all(|r| identifiant_sur(&r.contenu_id) && r.version.len() <= 128);
    if !sures {
        return StatusCode::NOT_FOUND.into_response();
    }
    let jeton = jeton(&entetes);
    match etat
        .application
        .portail_tentative(jeton.as_deref(), &demande)
        .await
    {
        Ok(ReponseLecture::Lu(())) => StatusCode::NO_CONTENT.into_response(),
        Ok(autre) => sans_lecture(&autre).unwrap_or_else(panne),
        Err(_) => panne(),
    }
}

/// « Mon avancement » de l'élève entré avec son code.
pub(crate) async fn mon_avancement(
    State(etat): State<EtatPortail>,
    entetes: HeaderMap,
) -> Response {
    let jeton = jeton(&entetes);
    match etat
        .application
        .portail_mon_avancement(jeton.as_deref())
        .await
    {
        Ok(ReponseLecture::Lu(a)) => Json(a).into_response(),
        Ok(autre) => sans_lecture(&autre).unwrap_or_else(panne),
        Err(_) => panne(),
    }
}

/// « Mes devoirs » de l'élève.
pub(crate) async fn mes_devoirs(State(etat): State<EtatPortail>, entetes: HeaderMap) -> Response {
    let jeton = jeton(&entetes);
    match etat.application.portail_mes_devoirs(jeton.as_deref()).await {
        Ok(ReponseLecture::Lu(d)) => Json(d).into_response(),
        Ok(autre) => sans_lecture(&autre).unwrap_or_else(panne),
        Err(_) => panne(),
    }
}

/// Un devoir de l'élève, ouvert.
pub(crate) async fn devoir(
    State(etat): State<EtatPortail>,
    Path(id): Path<String>,
    entetes: HeaderMap,
) -> Response {
    if !identifiant_sur(&id) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let jeton = jeton(&entetes);
    match etat.application.portail_devoir(jeton.as_deref(), &id).await {
        Ok(ReponseLecture::Lu(d)) => Json(d).into_response(),
        Ok(autre) => sans_lecture(&autre).unwrap_or_else(panne),
        Err(_) => panne(),
    }
}

/// L'élève rend un devoir. Reçu : 200 avec l'instant ; refusé (clos, photo qui n'en est pas
/// une, trop de photos…) : 422 avec la raison, que la page dit en une phrase.
pub(crate) async fn rendre(
    State(etat): State<EtatPortail>,
    entetes: HeaderMap,
    Json(demande): Json<DemandeRendu>,
) -> Response {
    if !identifiant_sur(&demande.devoir_id) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let jeton = jeton(&entetes);
    match etat
        .application
        .portail_rendre(jeton.as_deref(), &demande)
        .await
    {
        Ok(ReponseLecture::Lu(Ok(fait))) => Json(fait).into_response(),
        Ok(ReponseLecture::Lu(Err(refus))) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({ "refus": refus })),
        )
            .into_response(),
        Ok(autre) => sans_lecture(&autre).unwrap_or_else(panne),
        Err(_) => panne(),
    }
}

/// Une photo d'un rendu de l'élève. Retirée pour libérer la place : 410.
pub(crate) async fn photo_rendu(
    State(etat): State<EtatPortail>,
    Path((id, ordre)): Path<(String, String)>,
    entetes: HeaderMap,
) -> Response {
    let Ok(ordre) = ordre.parse::<i64>() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if !identifiant_sur(&id) || !(1..=5).contains(&ordre) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let jeton = jeton(&entetes);
    match etat
        .application
        .portail_photo_rendu(jeton.as_deref(), &id, ordre)
        .await
    {
        Ok(ReponseLecture::Lu(photo)) => match photo.octets {
            Some(octets) => {
                let mut reponse = octets.into_response();
                let h = reponse.headers_mut();
                h.insert(CONTENT_TYPE, HeaderValue::from_static(photo.type_mime));
                h.insert(CACHE_CONTROL, HeaderValue::from_static("private, no-store"));
                h.insert(CONTENT_DISPOSITION, HeaderValue::from_static("inline"));
                reponse
            }
            None => StatusCode::GONE.into_response(),
        },
        Ok(autre) => sans_lecture(&autre).unwrap_or_else(panne),
        Err(_) => panne(),
    }
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
    fn la_plage_se_lit() {
        let mut h = HeaderMap::new();
        assert_eq!(plage(&h), None);
        h.insert(RANGE, HeaderValue::from_static("bytes=0-"));
        assert_eq!(plage(&h), Some((0, None)));
        h.insert(RANGE, HeaderValue::from_static("bytes=100-199"));
        assert_eq!(plage(&h), Some((100, Some(199))));
        h.insert(RANGE, HeaderValue::from_static("bytes=9-3"));
        assert_eq!(plage(&h), None);
        h.insert(RANGE, HeaderValue::from_static("lignes=1-2"));
        assert_eq!(plage(&h), None);
    }

    #[test]
    fn le_nom_du_pdf_passe_dans_l_en_tete() {
        let v = disposition_pdf("La racine carrée.pdf");
        let texte = v.to_str().unwrap_or_default();
        assert!(texte.contains("filename=\"La racine carr_e.pdf\""));
        assert!(texte.contains("filename*=UTF-8''La%20racine%20carr%C3%A9e.pdf"));
        assert!(!identifiant_sur("../x"));
        assert!(identifiant_sur("01J9ZK3Q-abc"));
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
