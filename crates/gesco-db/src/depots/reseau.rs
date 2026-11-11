//! Réseau local de l'école et postes reliés (SPEC_PHASE_11_A §4). Un poste ne se supprime pas,
//! il se révoque (R-14).

use gesco_core::commun::{Horodatage, Id};
use gesco_core::core::PosteRelie;
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::depots::horodatage_optionnel;
use crate::erreur::Resultat;
use crate::id::lire_id;

/// Port du poste principal tant que l'école n'en règle pas un autre.
pub const PORT_PAR_DEFAUT: u16 = 47_800;

/// État du réseau local d'un établissement, tel que la base le garde.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReseauLocal {
    /// Le poste principal écoute les postes reliés.
    pub actif: bool,
    /// Port d'écoute.
    pub port: u16,
    /// Certificat du poste principal (DER) et sa clé privée (PKCS#8, DER), fabriqués ensemble.
    pub certificat: Option<(Vec<u8>, Vec<u8>)>,
    /// Instant de fabrication du certificat.
    pub fabrique_le: Option<Horodatage>,
}

impl Default for ReseauLocal {
    fn default() -> Self {
        Self {
            actif: false,
            port: PORT_PAR_DEFAUT,
            certificat: None,
            fabrique_le: None,
        }
    }
}

#[derive(FromRow)]
struct LigneReseau {
    actif: i64,
    port: i64,
    certificat: Option<Vec<u8>>,
    cle: Option<Vec<u8>>,
    fabrique_le: Option<i64>,
}

/// État du réseau local ; l'état par défaut (éteint, sans certificat) si rien n'a été réglé.
pub async fn lire(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<ReseauLocal> {
    let ligne = sqlx::query_as::<_, LigneReseau>(
        "SELECT actif, port, certificat, cle, fabrique_le FROM reseau_local
         WHERE etablissement_id = ?1",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?;
    Ok(ligne.map_or_else(ReseauLocal::default, |l| ReseauLocal {
        actif: l.actif != 0,
        port: u16::try_from(l.port).unwrap_or(PORT_PAR_DEFAUT),
        certificat: l.certificat.zip(l.cle),
        fabrique_le: horodatage_optionnel(l.fabrique_le),
    }))
}

/// Enregistre l'état du réseau local, à la place du précédent.
pub async fn ecrire(
    cnx: &mut Connexion,
    reseau: &ReseauLocal,
    contexte: &Contexte,
) -> Resultat<()> {
    let (certificat, cle) = match &reseau.certificat {
        Some((certificat, cle)) => (Some(certificat.as_slice()), Some(cle.as_slice())),
        None => (None, None),
    };
    sqlx::query(
        "INSERT INTO reseau_local (etablissement_id, actif, port, certificat, cle, fabrique_le,
            cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?7, ?8, ?9)
         ON CONFLICT (etablissement_id) DO UPDATE SET actif = ?2, port = ?3, certificat = ?4,
            cle = ?5, fabrique_le = ?6, mis_a_jour_le = ?7, mis_a_jour_par = ?8,
            identifiant_poste = ?9",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(i64::from(reseau.actif))
    .bind(i64::from(reseau.port))
    .bind(certificat)
    .bind(cle)
    .bind(reseau.fabrique_le.map(|h| h.millis_epoque()))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

#[derive(FromRow)]
struct LignePoste {
    id: String,
    etablissement_id: String,
    nom: String,
    empreinte_secret: String,
    relie_le: i64,
    relie_par: Option<String>,
    derniere_connexion: Option<i64>,
    revoque_le: Option<i64>,
    revoque_par: Option<String>,
}

impl LignePoste {
    fn vers_entite(self) -> Resultat<PosteRelie> {
        let t = "poste_relie";
        Ok(PosteRelie {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            nom: self.nom,
            empreinte_secret: self.empreinte_secret,
            relie_le: Horodatage::depuis_millis_epoque(self.relie_le),
            relie_par: self
                .relie_par
                .as_deref()
                .map(|u| lire_id(t, "relie_par", u))
                .transpose()?,
            derniere_connexion: horodatage_optionnel(self.derniere_connexion),
            revoque_le: horodatage_optionnel(self.revoque_le),
            revoque_par: self
                .revoque_par
                .as_deref()
                .map(|u| lire_id(t, "revoque_par", u))
                .transpose()?,
        })
    }
}

macro_rules! selectionner_postes {
    ($suite:literal) => {
        concat!(
            "SELECT id, etablissement_id, nom, empreinte_secret, relie_le, relie_par, \
             derniere_connexion, revoque_le, revoque_par FROM poste_relie ",
            $suite
        )
    };
}

/// Tous les postes reliés, en service ou révoqués, du plus récent au plus ancien.
pub async fn postes(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<PosteRelie>> {
    sqlx::query_as::<_, LignePoste>(selectionner_postes!(
        "WHERE etablissement_id = ?1 ORDER BY relie_le DESC, id DESC"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LignePoste::vers_entite)
    .collect()
}

/// Un poste relié par son identifiant, en service ou non.
pub async fn poste(
    cnx: &mut Connexion,
    id: Id<PosteRelie>,
    contexte: &Contexte,
) -> Resultat<Option<PosteRelie>> {
    sqlx::query_as::<_, LignePoste>(selectionner_postes!(
        "WHERE id = ?1 AND etablissement_id = ?2"
    ))
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .map(LignePoste::vers_entite)
    .transpose()
}

/// Enregistre un poste qui vient d'être relié.
pub async fn inserer_poste(
    cnx: &mut Connexion,
    p: &PosteRelie,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "INSERT INTO poste_relie (id, etablissement_id, nom, empreinte_secret, relie_le, relie_par,
            derniere_connexion, revoque_le, revoque_par, cree_le, cree_par, mis_a_jour_le,
            mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, NULL, NULL, ?7, ?8, ?7, ?8, ?9)",
    )
    .bind(p.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(&p.nom)
    .bind(&p.empreinte_secret)
    .bind(p.relie_le.millis_epoque())
    .bind(p.relie_par.map(|u| u.vers_chaine()))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Révoque un poste en service. Rend faux s'il était déjà révoqué ou inconnu.
pub async fn revoquer_poste(
    cnx: &mut Connexion,
    id: Id<PosteRelie>,
    contexte: &Contexte,
) -> Resultat<bool> {
    let resultat = sqlx::query(
        "UPDATE poste_relie SET revoque_le = ?1, revoque_par = ?2, mis_a_jour_le = ?1,
            mis_a_jour_par = ?2, identifiant_poste = ?3
         WHERE id = ?4 AND etablissement_id = ?5 AND revoque_le IS NULL",
    )
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(resultat.rows_affected() > 0)
}

/// Note le dernier appel reçu d'un poste en service.
pub async fn noter_connexion(
    cnx: &mut Connexion,
    id: Id<PosteRelie>,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE poste_relie SET derniere_connexion = ?1
         WHERE id = ?2 AND etablissement_id = ?3 AND revoque_le IS NULL",
    )
    .bind(contexte.maintenant.millis_epoque())
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}
