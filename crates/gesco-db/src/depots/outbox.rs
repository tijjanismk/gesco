//! File d'événements sortants (ADR-009).

use gesco_core::commun::{Horodatage, Id};
use gesco_core::core::{Evenement, Poste, TypeEvenement};
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

pub(crate) async fn inserer(cnx: &mut Connexion, evenement: &Evenement) -> Resultat<()> {
    sqlx::query(
        "INSERT INTO outbox (id, type_evenement, version, entite, entite_id, etablissement_id, acteur_id, horodatage,
            identifiant_poste, charge_utile, correlation_id, consomme_le)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, NULL)",
    )
    .bind(evenement.id.vers_chaine())
    .bind(evenement.type_evenement.nom())
    .bind(i64::from(evenement.version))
    .bind(&evenement.entite)
    .bind(&evenement.entite_id)
    .bind(evenement.etablissement_id.vers_chaine())
    .bind(evenement.acteur_id.map(|a| a.vers_chaine()))
    .bind(evenement.horodatage.millis_epoque())
    .bind(&evenement.poste.0)
    .bind(serde_json::to_string(&evenement.charge_utile)?)
    .bind(&evenement.correlation_id)
    .execute(cnx)
    .await?;
    Ok(())
}

#[derive(FromRow)]
struct Ligne {
    id: String,
    type_evenement: String,
    version: i64,
    entite: String,
    entite_id: String,
    etablissement_id: String,
    acteur_id: Option<String>,
    horodatage: i64,
    identifiant_poste: String,
    charge_utile: String,
    correlation_id: String,
    consomme_le: Option<i64>,
}

impl Ligne {
    fn vers_entite(self) -> Resultat<Evenement> {
        Ok(Evenement {
            id: lire_id("outbox", "id", &self.id)?,
            type_evenement: TypeEvenement::depuis_nom(&self.type_evenement).ok_or_else(|| {
                ErreurBase::illisible("outbox", "type_evenement", &self.type_evenement)
            })?,
            version: u16::try_from(self.version)
                .map_err(|e| ErreurBase::illisible("outbox", "version", e))?,
            entite: self.entite,
            entite_id: self.entite_id,
            etablissement_id: lire_id("outbox", "etablissement_id", &self.etablissement_id)?,
            acteur_id: self
                .acteur_id
                .as_deref()
                .map(|a| lire_id("outbox", "acteur_id", a))
                .transpose()?,
            horodatage: Horodatage::depuis_millis_epoque(self.horodatage),
            poste: Poste(self.identifiant_poste),
            charge_utile: serde_json::from_str(&self.charge_utile)?,
            correlation_id: self.correlation_id,
            consomme_le: self.consomme_le.map(Horodatage::depuis_millis_epoque),
        })
    }
}

/// Compose `SELECT <colonnes> FROM <table> <suite>` à partir de littéraux : sqlx refuse les
/// chaînes SQL dynamiques, et les colonnes ne sont écrites qu'une fois.
macro_rules! selectionner {
    ($suite:literal) => {
        concat!("SELECT id, type_evenement, version, entite, entite_id, etablissement_id, acteur_id, horodatage, identifiant_poste, charge_utile, correlation_id, consomme_le FROM outbox ", $suite)
    };
}

/// Événements non consommés, dans l'ordre d'émission.
pub async fn non_consommes(
    cnx: &mut Connexion,
    limite: i64,
    contexte: &Contexte,
) -> Resultat<Vec<Evenement>> {
    sqlx::query_as::<_, Ligne>(selectionner!(
        "WHERE etablissement_id = ?1 AND consomme_le IS NULL ORDER BY rowid LIMIT ?2"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(limite)
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(Ligne::vers_entite)
    .collect()
}

/// Tous les événements d'une corrélation (ceux d'une même opération).
pub async fn par_correlation(
    cnx: &mut Connexion,
    correlation_id: &str,
    contexte: &Contexte,
) -> Resultat<Vec<Evenement>> {
    sqlx::query_as::<_, Ligne>(selectionner!(
        "WHERE etablissement_id = ?1 AND correlation_id = ?2 ORDER BY rowid"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(correlation_id)
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(Ligne::vers_entite)
    .collect()
}

/// Marque des événements comme consommés. Seule mutation autorisée sur la table.
pub async fn marquer_consommes(
    cnx: &mut Connexion,
    ids: &[Id<Evenement>],
    contexte: &Contexte,
) -> Resultat<()> {
    for id in ids {
        sqlx::query("UPDATE outbox SET consomme_le = ?1 WHERE id = ?2 AND etablissement_id = ?3 AND consomme_le IS NULL")
            .bind(contexte.maintenant.millis_epoque())
            .bind(id.vers_chaine())
            .bind(contexte.etablissement_id.vers_chaine())
            .execute(&mut *cnx)
            .await?;
    }
    Ok(())
}
