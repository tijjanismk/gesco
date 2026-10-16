//! Métadonnées des documents produits (§8.1 du blueprint, R-45).

use gesco_core::commun::{Horodatage, Id};
use gesco_core::core::{Etablissement, Utilisateur};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

/// Un PDF produit par le moteur documentaire, avec la version du modèle qui l'a composé.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Document {
    /// Identifiant technique.
    pub id: Id<Document>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Type de document (`recu`, `bulletin`, `test`…).
    pub type_document: String,
    /// Entité rattachée.
    pub entite: String,
    /// Identifiant de l'entité rattachée.
    pub entite_id: String,
    /// Chemin du fichier produit.
    pub chemin: String,
    /// Empreinte SHA-256 du fichier, en hexadécimal.
    pub empreinte_sha256: String,
    /// Taille en octets.
    pub taille: i64,
    /// Identifiant du modèle.
    pub modele_id: String,
    /// Version du modèle au moment de la composition.
    pub modele_version: i64,
    /// Instant de production.
    pub produit_le: Horodatage,
    /// Producteur.
    pub produit_par: Option<Id<Utilisateur>>,
}

#[derive(FromRow)]
struct Ligne {
    id: String,
    etablissement_id: String,
    #[sqlx(rename = "type")]
    type_document: String,
    entite: String,
    entite_id: String,
    chemin: String,
    empreinte_sha256: String,
    taille: i64,
    modele_id: String,
    modele_version: i64,
    produit_le: i64,
    produit_par: Option<String>,
}

impl Ligne {
    fn vers_entite(self) -> Resultat<Document> {
        Ok(Document {
            id: lire_id("document", "id", &self.id)?,
            etablissement_id: lire_id("document", "etablissement_id", &self.etablissement_id)?,
            type_document: self.type_document,
            entite: self.entite,
            entite_id: self.entite_id,
            chemin: self.chemin,
            empreinte_sha256: self.empreinte_sha256,
            taille: self.taille,
            modele_id: self.modele_id,
            modele_version: self.modele_version,
            produit_le: Horodatage::depuis_millis_epoque(self.produit_le),
            produit_par: self
                .produit_par
                .as_deref()
                .map(|p| lire_id("document", "produit_par", p))
                .transpose()?,
        })
    }
}

/// Compose `SELECT <colonnes> FROM <table> <suite>` à partir de littéraux : sqlx refuse les
/// chaînes SQL dynamiques, et les colonnes ne sont écrites qu'une fois.
macro_rules! selectionner {
    ($suite:literal) => {
        concat!("SELECT id, etablissement_id, type, entite, entite_id, chemin, empreinte_sha256, taille, modele_id, modele_version, produit_le, produit_par FROM document ", $suite)
    };
}

/// Enregistre un document produit.
pub async fn inserer(
    cnx: &mut Connexion,
    document: &Document,
    contexte: &Contexte,
) -> Resultat<()> {
    if document.etablissement_id != contexte.etablissement_id {
        return Err(ErreurBase::Cloisonnement {
            entite: "document",
            id: document.id.vers_chaine(),
        });
    }
    sqlx::query(
        "INSERT INTO document (id, etablissement_id, type, entite, entite_id, chemin, empreinte_sha256, taille, modele_id,
            modele_version, produit_le, produit_par, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?13, ?14, ?15)",
    )
    .bind(document.id.vers_chaine())
    .bind(document.etablissement_id.vers_chaine())
    .bind(&document.type_document)
    .bind(&document.entite)
    .bind(&document.entite_id)
    .bind(&document.chemin)
    .bind(&document.empreinte_sha256)
    .bind(document.taille)
    .bind(&document.modele_id)
    .bind(document.modele_version)
    .bind(document.produit_le.millis_epoque())
    .bind(document.produit_par.map(|p| p.vers_chaine()))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Lit un document de l'établissement courant.
pub async fn lire(
    cnx: &mut Connexion,
    id: Id<Document>,
    contexte: &Contexte,
) -> Resultat<Document> {
    sqlx::query_as::<_, Ligne>(selectionner!(
        "WHERE id = ?1 AND etablissement_id = ?2 AND supprime_le IS NULL"
    ))
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .ok_or_else(|| ErreurBase::Introuvable {
        entite: "document",
        id: id.vers_chaine(),
    })?
    .vers_entite()
}

/// Documents rattachés à une entité.
pub async fn par_entite(
    cnx: &mut Connexion,
    entite: &str,
    entite_id: &str,
    contexte: &Contexte,
) -> Resultat<Vec<Document>> {
    sqlx::query_as::<_, Ligne>(selectionner!("WHERE etablissement_id = ?1 AND entite = ?2 AND entite_id = ?3 AND supprime_le IS NULL ORDER BY produit_le"))
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(entite)
    .bind(entite_id)
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(Ligne::vers_entite)
    .collect()
}
