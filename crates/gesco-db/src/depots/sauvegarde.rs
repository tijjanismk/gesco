//! Historique des sauvegardes (§18 du blueprint).

use gesco_core::commun::{Horodatage, Id};
use gesco_core::core::Etablissement;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

/// Issue d'une sauvegarde.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ResultatSauvegarde {
    /// Archive produite et empreinte vérifiée.
    Reussie,
    /// Échec, détail dans le journal d'audit.
    Echouee,
}

impl ResultatSauvegarde {
    const fn nom(&self) -> &'static str {
        match self {
            Self::Reussie => "reussie",
            Self::Echouee => "echouee",
        }
    }
}

/// Trace d'une sauvegarde.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Sauvegarde {
    /// Identifiant technique.
    pub id: Id<Sauvegarde>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Instant.
    pub horodatage: Horodatage,
    /// Chemin de l'archive.
    pub destination: String,
    /// Taille de l'archive en octets.
    pub taille: i64,
    /// Empreinte SHA-256 de l'archive.
    pub empreinte_sha256: String,
    /// Version du schéma de la base sauvegardée.
    pub version_schema: i64,
    /// Version de l'application.
    pub version_applicative: String,
    /// Issue.
    pub resultat: ResultatSauvegarde,
    /// Dernier test de restauration réussi sur cette archive (§18.3).
    pub dernier_test_restauration: Option<Horodatage>,
}

#[derive(FromRow)]
struct Ligne {
    id: String,
    etablissement_id: String,
    horodatage: i64,
    destination: String,
    taille: i64,
    empreinte_sha256: String,
    version_schema: i64,
    version_applicative: String,
    resultat: String,
    dernier_test_restauration: Option<i64>,
}

impl Ligne {
    fn vers_entite(self) -> Resultat<Sauvegarde> {
        Ok(Sauvegarde {
            id: lire_id("sauvegarde", "id", &self.id)?,
            etablissement_id: lire_id("sauvegarde", "etablissement_id", &self.etablissement_id)?,
            horodatage: Horodatage::depuis_millis_epoque(self.horodatage),
            destination: self.destination,
            taille: self.taille,
            empreinte_sha256: self.empreinte_sha256,
            version_schema: self.version_schema,
            version_applicative: self.version_applicative,
            resultat: match self.resultat.as_str() {
                "reussie" => ResultatSauvegarde::Reussie,
                "echouee" => ResultatSauvegarde::Echouee,
                autre => return Err(ErreurBase::illisible("sauvegarde", "resultat", autre)),
            },
            dernier_test_restauration: self
                .dernier_test_restauration
                .map(Horodatage::depuis_millis_epoque),
        })
    }
}

/// Compose `SELECT <colonnes> FROM <table> <suite>` à partir de littéraux : sqlx refuse les
/// chaînes SQL dynamiques, et les colonnes ne sont écrites qu'une fois.
macro_rules! selectionner {
    ($suite:literal) => {
        concat!("SELECT id, etablissement_id, horodatage, destination, taille, empreinte_sha256, version_schema, version_applicative, resultat, dernier_test_restauration FROM sauvegarde ", $suite)
    };
}

/// Enregistre une sauvegarde.
pub async fn inserer(
    cnx: &mut Connexion,
    sauvegarde: &Sauvegarde,
    contexte: &Contexte,
) -> Resultat<()> {
    if sauvegarde.etablissement_id != contexte.etablissement_id {
        return Err(ErreurBase::Cloisonnement {
            entite: "sauvegarde",
            id: sauvegarde.id.vers_chaine(),
        });
    }
    sqlx::query(
        "INSERT INTO sauvegarde (id, etablissement_id, horodatage, destination, taille, empreinte_sha256, version_schema,
            version_applicative, resultat, dernier_test_restauration, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
    )
    .bind(sauvegarde.id.vers_chaine())
    .bind(sauvegarde.etablissement_id.vers_chaine())
    .bind(sauvegarde.horodatage.millis_epoque())
    .bind(&sauvegarde.destination)
    .bind(sauvegarde.taille)
    .bind(&sauvegarde.empreinte_sha256)
    .bind(sauvegarde.version_schema)
    .bind(&sauvegarde.version_applicative)
    .bind(sauvegarde.resultat.nom())
    .bind(sauvegarde.dernier_test_restauration.map(|h| h.millis_epoque()))
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Note un test de restauration réussi sur une archive.
pub async fn noter_test_restauration(
    cnx: &mut Connexion,
    id: Id<Sauvegarde>,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query("UPDATE sauvegarde SET dernier_test_restauration = ?1 WHERE id = ?2 AND etablissement_id = ?3")
        .bind(contexte.maintenant.millis_epoque())
        .bind(id.vers_chaine())
        .bind(contexte.etablissement_id.vers_chaine())
        .execute(cnx)
        .await?;
    Ok(())
}

/// Sauvegarde par chemin d'archive.
pub async fn par_destination(
    cnx: &mut Connexion,
    destination: &str,
    contexte: &Contexte,
) -> Resultat<Option<Sauvegarde>> {
    sqlx::query_as::<_, Ligne>(selectionner!(
        "WHERE etablissement_id = ?1 AND destination = ?2 ORDER BY horodatage DESC LIMIT 1"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(destination)
    .fetch_optional(cnx)
    .await?
    .map(Ligne::vers_entite)
    .transpose()
}

/// Sauvegardes de l'établissement, la plus récente d'abord.
pub async fn lister(
    cnx: &mut Connexion,
    limite: i64,
    contexte: &Contexte,
) -> Resultat<Vec<Sauvegarde>> {
    sqlx::query_as::<_, Ligne>(selectionner!(
        "WHERE etablissement_id = ?1 ORDER BY horodatage DESC LIMIT ?2"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(limite)
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(Ligne::vers_entite)
    .collect()
}

/// Dernière sauvegarde réussie.
pub async fn derniere_reussie(
    cnx: &mut Connexion,
    contexte: &Contexte,
) -> Resultat<Option<Sauvegarde>> {
    sqlx::query_as::<_, Ligne>(selectionner!(
        "WHERE etablissement_id = ?1 AND resultat = 'reussie' ORDER BY horodatage DESC LIMIT 1"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .map(Ligne::vers_entite)
    .transpose()
}
