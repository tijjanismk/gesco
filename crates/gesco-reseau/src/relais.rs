//! Le poste relié (SPEC_PHASE_11_A §5) : il transmet chaque commande de son écran au poste
//! principal par le canal chiffré, en n'acceptant que l'empreinte retenue à l'appairage. Il ne
//! garde ni base ni clé de données.
//!
//! - [`apercevoir`] lit la version et l'empreinte du poste principal, avant l'appairage : la
//!   personne compare l'empreinte courte à celle de l'écran « Postes de l'école ».
//! - [`appairer`] échange le code contre le secret du poste.
//! - [`Relais`] transmet les commandes ; pour celles qui manient un fichier, c'est le contenu
//!   qui voyage (§3.3), lu ou écrit sur le disque de ce poste.
//!
//! Une commande qui n'atteint pas le poste principal rend [`CLE_INJOIGNABLE`] ; rien n'est mis
//! de côté pour plus tard.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use gesco_core::core::reseau::{empreinte_certificat, empreinte_courte};
use gesco_services::ErreurCommande;
use gesco_services::commandes::{Commande, Fichier};
use gesco_services::fichiers_relies::{CHAMP_CONTENU, CHAMP_NOM, CHAMP_SORTIE, coder, decoder};
use gesco_services::postes::AppairageConclu;
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::header::{AUTHORIZATION, CONTENT_TYPE, HOST};
use hyper::{Method, Request, StatusCode};
use hyper_util::rt::TokioIo;
use rustls::ClientConfig;
use rustls::pki_types::ServerName;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;

use crate::chiffrement::{
    AUTRE_EMPREINTE, NOM_CERTIFICAT, configuration_client, configuration_decouverte,
};
use crate::{
    DemandeAppairage, ENTETE_POSTE, ENTETE_SECRET, ENTETE_VERSION, EtatServeur, PORT_PAR_DEFAUT,
};

/// Clé d'erreur d'une commande qui n'atteint pas le poste principal.
pub const CLE_INJOIGNABLE: &str = "reseau.poste_principal_injoignable";

/// Clé d'erreur d'un poste principal qui ne présente plus l'empreinte retenue.
pub const CLE_EMPREINTE_DIFFERENTE: &str = "reseau.empreinte_differente";

/// Clé d'erreur d'une adresse de poste principal qui ne se lit pas.
pub const CLE_ADRESSE_INVALIDE: &str = "reseau.adresse_invalide";

/// Clé d'erreur d'un fichier de ce poste qui ne se lit pas ou ne s'écrit pas.
pub const CLE_FICHIER_INACCESSIBLE: &str = "reseau.fichier_inaccessible";

/// Clé d'erreur d'un fichier trop gros pour voyager.
pub const CLE_FICHIER_TROP_GROS: &str = "reseau.fichier_trop_gros";

/// Taille maximale d'un fichier qui voyage : codé, il tient dans le corps admis par le poste
/// principal.
pub const TAILLE_MAX_FICHIER: u64 = 20 * 1024 * 1024;

/// Temps laissé pour joindre le poste principal et établir le canal chiffré.
pub const DELAI_CONNEXION: Duration = Duration::from_secs(5);

/// Attente d'une réponse : un lot de bulletins prend du temps (SPEC_PHASE_11_A §3.1).
pub const DELAI_REPONSE: Duration = Duration::from_secs(300);

/// Attente d'une réponse courte : l'état du poste principal, l'appairage.
const DELAI_COURT: Duration = Duration::from_secs(15);

fn erreur(cle: &str, detail: Option<String>) -> ErreurCommande {
    ErreurCommande {
        cle: cle.to_owned(),
        jusqua: None,
        detail,
    }
}

fn injoignable(detail: impl ToString) -> ErreurCommande {
    erreur(CLE_INJOIGNABLE, Some(detail.to_string()))
}

/// Adresse complète du poste principal, d'après ce que la personne a saisi : l'adresse seule
/// (le port habituel s'ajoute) ou l'adresse et le port.
pub fn adresse_complete(saisie: &str) -> Result<String, ErreurCommande> {
    let saisie = saisie.trim();
    let (hote, port) = match saisie.rsplit_once(':') {
        Some((hote, port)) => match port.parse::<u16>() {
            Ok(port) if port != 0 => (hote, port),
            _ => return Err(erreur(CLE_ADRESSE_INVALIDE, None)),
        },
        None => (saisie, PORT_PAR_DEFAUT),
    };
    let permis = |c: char| c.is_ascii_alphanumeric() || c == '.' || c == '-';
    if hote.is_empty() || !hote.chars().all(permis) {
        return Err(erreur(CLE_ADRESSE_INVALIDE, None));
    }
    Ok(format!("{hote}:{port}"))
}

/// Ce qu'un poste retient de son appairage : de quoi joindre le poste principal et s'en faire
/// reconnaître. Tout cela se range dans le trousseau du poste.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Liaison {
    /// Adresse et port du poste principal.
    pub adresse: String,
    /// Empreinte complète du certificat du poste principal.
    pub empreinte: String,
    /// Identifiant de ce poste, donné par le poste principal.
    pub poste_id: String,
    /// Secret de ce poste.
    pub secret: String,
}

impl std::fmt::Debug for Liaison {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Le secret ne s'écrit nulle part, pas même dans un message d'erreur.
        f.debug_struct("Liaison")
            .field("adresse", &self.adresse)
            .field("poste_id", &self.poste_id)
            .finish_non_exhaustive()
    }
}

/// Ce que le poste principal montre de lui avant l'appairage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Apercu {
    /// Adresse complète, port compris.
    pub adresse: String,
    /// Version de GESCO sur le poste principal.
    pub version: String,
    /// Empreinte complète de son certificat.
    pub empreinte: String,
    /// Empreinte courte, à comparer à l'œil avec l'écran du poste principal.
    pub empreinte_courte: String,
}

/// Réponse brute du poste principal.
struct Reponse {
    statut: StatusCode,
    corps: Bytes,
    /// Certificat présenté par le poste principal.
    certificat: Option<Vec<u8>>,
}

impl Reponse {
    /// Le corps d'une réponse heureuse, ou l'erreur que le poste principal a rendue.
    fn lire<T: for<'a> Deserialize<'a>>(&self) -> Result<T, ErreurCommande> {
        if self.statut == StatusCode::OK {
            serde_json::from_slice(&self.corps).map_err(injoignable)
        } else {
            Err(serde_json::from_slice::<ErreurCommande>(&self.corps)
                .unwrap_or_else(|_| injoignable(format!("réponse {}", self.statut))))
        }
    }
}

/// Vrai si le canal a échoué parce que le poste principal présente une autre empreinte.
fn autre_empreinte(echec: &std::io::Error) -> bool {
    echec
        .get_ref()
        .and_then(|source| source.downcast_ref::<rustls::Error>())
        .is_some_and(|e| matches!(e, rustls::Error::General(texte) if texte == AUTRE_EMPREINTE))
}

/// Un appel au poste principal : une connexion chiffrée, une requête, une réponse.
async fn echanger(
    configuration: Arc<ClientConfig>,
    adresse: &str,
    requete: Request<Full<Bytes>>,
    attente: Duration,
) -> Result<Reponse, ErreurCommande> {
    let flux = tokio::time::timeout(DELAI_CONNEXION, TcpStream::connect(adresse))
        .await
        .map_err(|_| injoignable("délai de connexion dépassé"))?
        .map_err(injoignable)?;
    let nom = ServerName::try_from(NOM_CERTIFICAT).map_err(injoignable)?;
    let canal = tokio::time::timeout(
        DELAI_CONNEXION,
        TlsConnector::from(configuration).connect(nom, flux),
    )
    .await
    .map_err(|_| injoignable("délai du canal chiffré dépassé"))?
    .map_err(|echec| {
        if autre_empreinte(&echec) {
            erreur(CLE_EMPREINTE_DIFFERENTE, None)
        } else {
            injoignable(echec)
        }
    })?;
    let certificat = canal
        .get_ref()
        .1
        .peer_certificates()
        .and_then(|liste| liste.first())
        .map(|der| der.as_ref().to_vec());
    let (mut envoi, connexion) = hyper::client::conn::http1::handshake(TokioIo::new(canal))
        .await
        .map_err(injoignable)?;
    let tache = tokio::spawn(connexion);
    let recu = tokio::time::timeout(attente, async {
        let reponse = envoi.send_request(requete).await.map_err(injoignable)?;
        let statut = reponse.status();
        let corps = reponse
            .into_body()
            .collect()
            .await
            .map_err(injoignable)?
            .to_bytes();
        Ok::<_, ErreurCommande>((statut, corps))
    })
    .await;
    tache.abort();
    let (statut, corps) = recu.map_err(|_| injoignable("délai de réponse dépassé"))??;
    Ok(Reponse {
        statut,
        corps,
        certificat,
    })
}

fn requete(
    methode: Method,
    chemin: &str,
    version: Option<&str>,
    corps: Option<&Value>,
) -> hyper::http::request::Builder {
    let mut requete = Request::builder()
        .method(methode)
        .uri(chemin)
        .header(HOST, NOM_CERTIFICAT);
    if let Some(version) = version {
        requete = requete.header(ENTETE_VERSION, version);
    }
    if corps.is_some() {
        requete = requete.header(CONTENT_TYPE, "application/json");
    }
    requete
}

fn corps(valeur: Option<&Value>) -> Full<Bytes> {
    Full::new(valeur.map_or_else(Bytes::new, |v| Bytes::from(v.to_string())))
}

/// Lit la version et l'empreinte du poste principal à cette adresse, avant tout appairage.
pub async fn apercevoir(saisie: &str) -> Result<Apercu, ErreurCommande> {
    let adresse = adresse_complete(saisie)?;
    let configuration = configuration_decouverte().map_err(injoignable)?;
    let demande = requete(Method::GET, "/api/etat", None, None)
        .body(corps(None))
        .map_err(injoignable)?;
    let reponse = echanger(configuration, &adresse, demande, DELAI_COURT).await?;
    let etat: EtatServeur = reponse.lire()?;
    let empreinte = reponse
        .certificat
        .as_deref()
        .map(empreinte_certificat)
        .ok_or_else(|| injoignable("aucun certificat présenté"))?;
    Ok(Apercu {
        adresse,
        version: etat.version,
        empreinte_courte: empreinte_courte(&empreinte),
        empreinte,
    })
}

/// Relie ce poste : échange le code affiché sur le poste principal contre le secret du poste.
/// Le canal n'accepte que l'empreinte que la personne vient de comparer.
pub async fn appairer(
    apercu: &Apercu,
    version: &str,
    code: &str,
    nom: &str,
) -> Result<(Liaison, AppairageConclu), ErreurCommande> {
    let configuration = configuration_client(&apercu.empreinte).map_err(injoignable)?;
    let demande = serde_json::to_value(DemandeAppairage {
        code: code.to_owned(),
        nom: nom.to_owned(),
    })
    .map_err(injoignable)?;
    let envoi = requete(
        Method::POST,
        "/api/appairage",
        Some(version),
        Some(&demande),
    )
    .body(corps(Some(&demande)))
    .map_err(injoignable)?;
    let reponse = echanger(configuration, &apercu.adresse, envoi, DELAI_COURT).await?;
    let conclu: AppairageConclu = reponse.lire()?;
    let liaison = Liaison {
        adresse: apercu.adresse.clone(),
        empreinte: apercu.empreinte.clone(),
        poste_id: conclu.poste_id.clone(),
        secret: conclu.secret.clone(),
    };
    Ok((liaison, conclu))
}

/// Le relais d'un poste relié vers son poste principal.
pub struct Relais {
    liaison: Liaison,
    version: String,
    configuration: Arc<ClientConfig>,
    attente: Duration,
}

impl Relais {
    /// Relais vers le poste principal de cette liaison, pour un poste de cette version.
    pub fn nouveau(liaison: Liaison, version: &str) -> Result<Self, ErreurCommande> {
        let configuration = configuration_client(&liaison.empreinte).map_err(injoignable)?;
        Ok(Self {
            liaison,
            version: version.to_owned(),
            configuration,
            attente: DELAI_REPONSE,
        })
    }

    /// Le même relais avec une autre attente de réponse : les essais n'attendent pas cinq
    /// minutes.
    #[must_use]
    pub fn avec_attente(mut self, attente: Duration) -> Self {
        self.attente = attente;
        self
    }

    /// Adresse du poste principal.
    pub fn adresse(&self) -> &str {
        &self.liaison.adresse
    }

    /// Vérifie que le poste principal répond, et qu'il est de la même version que ce poste.
    pub async fn verifier(&self) -> Result<(), ErreurCommande> {
        let demande = requete(Method::GET, "/api/etat", None, None)
            .body(corps(None))
            .map_err(injoignable)?;
        let reponse = echanger(
            Arc::clone(&self.configuration),
            &self.liaison.adresse,
            demande,
            DELAI_COURT,
        )
        .await?;
        let etat: EtatServeur = reponse.lire()?;
        if etat.version == self.version {
            Ok(())
        } else {
            Err(erreur(crate::CLE_VERSION_DIFFERENTE, Some(etat.version)))
        }
    }

    /// Transmet une commande au poste principal et rend sa réponse, comme la commande locale.
    pub async fn executer(
        &self,
        nom: &str,
        jeton: Option<&str>,
        mut entree: Value,
    ) -> Result<Value, ErreurCommande> {
        let fichier = Commande::par_nom(nom).and_then(Commande::fichier);
        let mut a_ecrire = None;
        match fichier {
            Some(Fichier::Lu(champ)) => {
                if let Some(chemin) = chemin_dans(&entree, champ) {
                    let envoi = lire_pour_envoi(&chemin).await?;
                    remplacer(&mut entree, champ, envoi);
                }
            }
            Some(Fichier::Ecrit(champ)) => {
                if let Some(chemin) = chemin_dans(&entree, champ) {
                    // Seul le nom du fichier part : le chemin est celui de ce poste.
                    let nom_fichier = nom_du_fichier(&chemin);
                    remplacer(&mut entree, champ, Value::String(nom_fichier));
                    a_ecrire = Some((champ, chemin));
                }
            }
            None => {}
        }
        let mut envoi = requete(
            Method::POST,
            &format!("/api/commandes/{nom}"),
            Some(&self.version),
            Some(&entree),
        )
        .header(ENTETE_POSTE, &self.liaison.poste_id)
        .header(ENTETE_SECRET, &self.liaison.secret);
        if let Some(jeton) = jeton {
            envoi = envoi.header(AUTHORIZATION, format!("Bearer {jeton}"));
        }
        let envoi = envoi.body(corps(Some(&entree))).map_err(injoignable)?;
        let reponse = echanger(
            Arc::clone(&self.configuration),
            &self.liaison.adresse,
            envoi,
            self.attente,
        )
        .await?;
        let mut recu: Value = reponse.lire()?;
        let Some((champ, chemin)) = a_ecrire else {
            return Ok(recu);
        };
        if let Some(contenu) = recu.get(CHAMP_CONTENU).and_then(Value::as_str) {
            let octets = decoder(contenu).ok_or_else(|| injoignable("fichier reçu illisible"))?;
            tokio::fs::write(&chemin, octets)
                .await
                .map_err(|e| erreur(CLE_FICHIER_INACCESSIBLE, Some(e.to_string())))?;
        }
        let mut sortie = recu
            .get_mut(CHAMP_SORTIE)
            .map(Value::take)
            .unwrap_or(Value::Null);
        // Si la sortie redit le chemin, c'est celui de ce poste, pas le dossier de travail du
        // poste principal.
        if let Some(redit) = sortie.get_mut(champ)
            && redit.is_string()
        {
            *redit = Value::String(chemin.to_string_lossy().into_owned());
        }
        Ok(sortie)
    }
}

fn chemin_dans(entree: &Value, champ: &str) -> Option<PathBuf> {
    entree.get(champ).and_then(Value::as_str).map(PathBuf::from)
}

fn remplacer(entree: &mut Value, champ: &str, valeur: Value) {
    if let Some(place) = entree.get_mut(champ) {
        *place = valeur;
    }
}

fn nom_du_fichier(chemin: &Path) -> String {
    chemin
        .file_name()
        .map(|nom| nom.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Lit un fichier de ce poste pour l'envoyer : son nom et son contenu.
async fn lire_pour_envoi(chemin: &Path) -> Result<Value, ErreurCommande> {
    let inaccessible = |e: std::io::Error| erreur(CLE_FICHIER_INACCESSIBLE, Some(e.to_string()));
    let taille = tokio::fs::metadata(chemin)
        .await
        .map_err(inaccessible)?
        .len();
    if taille > TAILLE_MAX_FICHIER {
        return Err(erreur(CLE_FICHIER_TROP_GROS, None));
    }
    let octets = tokio::fs::read(chemin).await.map_err(inaccessible)?;
    Ok(json!({ CHAMP_NOM: nom_du_fichier(chemin), CHAMP_CONTENU: coder(&octets) }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l_adresse_saisie_recoit_le_port_habituel() {
        assert_eq!(
            adresse_complete(" 192.168.1.20 ").ok().as_deref(),
            Some("192.168.1.20:47800")
        );
        assert_eq!(
            adresse_complete("192.168.1.20:5000").ok().as_deref(),
            Some("192.168.1.20:5000")
        );
        assert_eq!(
            adresse_complete("poste-direction").ok().as_deref(),
            Some("poste-direction:47800")
        );
        for fausse in [
            "",
            "192.168.1.20:",
            "192.168.1.20:0",
            "a b",
            "http://x",
            ":47800",
        ] {
            assert_eq!(
                adresse_complete(fausse).err().map(|e| e.cle),
                Some(CLE_ADRESSE_INVALIDE.to_owned()),
                "{fausse}"
            );
        }
    }
}
