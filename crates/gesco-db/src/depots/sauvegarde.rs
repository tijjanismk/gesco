//! Historique des sauvegardes (§18 du blueprint).

use gesco_core::commun::{Horodatage, Id};
use gesco_core::core::{Etablissement, EtatEnvoi, SorteDestination};
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
    /// Sorte de la destination.
    pub sorte: SorteDestination,
    /// État d'envoi d'une copie hors de l'école ; vide pour les autres sortes.
    pub etat_envoi: Option<EtatEnvoi>,
    /// Instant de la dernière vérification de cet état.
    pub etat_verifie_le: Option<Horodatage>,
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
    sorte: String,
    etat_envoi: Option<String>,
    etat_verifie_le: Option<i64>,
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
            sorte: SorteDestination::depuis_nom(&self.sorte)
                .ok_or_else(|| ErreurBase::illisible("sauvegarde", "sorte", &self.sorte))?,
            etat_envoi: match self.etat_envoi {
                None => None,
                Some(etat) => Some(
                    EtatEnvoi::depuis_nom(&etat)
                        .ok_or_else(|| ErreurBase::illisible("sauvegarde", "etat_envoi", &etat))?,
                ),
            },
            etat_verifie_le: self.etat_verifie_le.map(Horodatage::depuis_millis_epoque),
        })
    }
}

/// Compose `SELECT <colonnes> FROM <table> <suite>` à partir de littéraux : sqlx refuse les
/// chaînes SQL dynamiques, et les colonnes ne sont écrites qu'une fois.
macro_rules! selectionner {
    ($suite:literal) => {
        concat!("SELECT id, etablissement_id, horodatage, destination, taille, empreinte_sha256, version_schema, version_applicative, resultat, dernier_test_restauration, sorte, etat_envoi, etat_verifie_le FROM sauvegarde ", $suite)
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
            version_applicative, resultat, dernier_test_restauration, identifiant_poste, sorte, etat_envoi,
            etat_verifie_le)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
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
    .bind(sauvegarde.sorte.nom())
    .bind(sauvegarde.etat_envoi.map(EtatEnvoi::nom))
    .bind(sauvegarde.etat_verifie_le.map(|h| h.millis_epoque()))
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

/// Dernière sauvegarde réussie hors des dossiers synchronisés : le test de restauration la lit
/// en entier, ce qui ferait télécharger une copie que le fournisseur a libérée du disque.
pub async fn derniere_reussie(
    cnx: &mut Connexion,
    contexte: &Contexte,
) -> Resultat<Option<Sauvegarde>> {
    sqlx::query_as::<_, Ligne>(selectionner!(
        "WHERE etablissement_id = ?1 AND resultat = 'reussie' AND sorte <> 'synchronise'
         ORDER BY horodatage DESC LIMIT 1"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .map(Ligne::vers_entite)
    .transpose()
}

/// Copies que GESCO a bien déposées dans les dossiers synchronisés pour cet établissement, et
/// qui n'en ont pas été retirées : le nettoyage part d'elles, sans lire le dossier. La plus
/// récente d'abord.
pub async fn deposees_hors_ecole(
    cnx: &mut Connexion,
    contexte: &Contexte,
) -> Resultat<Vec<Sauvegarde>> {
    sqlx::query_as::<_, Ligne>(selectionner!(
        "WHERE etablissement_id = ?1 AND sorte = 'synchronise' AND resultat = 'reussie'
           AND (etat_envoi IS NULL OR etat_envoi <> 'absente')
         ORDER BY horodatage DESC"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(Ligne::vers_entite)
    .collect()
}

/// Instant de la dernière copie hors de l'école : bien déposée si `deposee`, sinon le dernier
/// essai, réussi ou non.
pub async fn derniere_hors_ecole(
    cnx: &mut Connexion,
    deposee: bool,
    contexte: &Contexte,
) -> Resultat<Option<Horodatage>> {
    let instant: Option<i64> = sqlx::query_scalar(
        "SELECT MAX(horodatage) FROM sauvegarde
         WHERE etablissement_id = ?1 AND sorte = 'synchronise' AND (?2 = 0 OR resultat = 'reussie')",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(i64::from(deposee))
    .fetch_one(cnx)
    .await?;
    Ok(instant.map(Horodatage::depuis_millis_epoque))
}

/// Note l'état d'envoi d'une copie hors de l'école et l'instant de sa vérification.
pub async fn noter_etat_envoi(
    cnx: &mut Connexion,
    id: Id<Sauvegarde>,
    etat: EtatEnvoi,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE sauvegarde SET etat_envoi = ?1, etat_verifie_le = ?2
         WHERE id = ?3 AND etablissement_id = ?4 AND sorte = 'synchronise'",
    )
    .bind(etat.nom())
    .bind(contexte.maintenant.millis_epoque())
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}
