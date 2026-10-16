//! Journal d'audit : ajout seul, chaîné (R-26, R-27). Les lignes arrivent scellées depuis
//! [`crate::UniteDeTravail`] ; ce dépôt ne calcule aucune empreinte. L'ordre de la chaîne est l'ordre
//! d'insertion (`rowid`) : deux lignes écrites dans la même milliseconde ont des ULID non ordonnés.

use gesco_core::commun::{Horodatage, Id};
use gesco_core::core::{EmpreinteAudit, Etablissement, LigneAudit, Operation, Poste};
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

/// Dernière empreinte de l'établissement, ou l'amorce si le journal est vide.
pub async fn derniere_empreinte(
    cnx: &mut Connexion,
    etablissement_id: Id<Etablissement>,
) -> Resultat<EmpreinteAudit> {
    let derniere: Option<String> =
        sqlx::query_scalar("SELECT empreinte FROM journal_audit WHERE etablissement_id = ?1 ORDER BY rowid DESC LIMIT 1")
            .bind(etablissement_id.vers_chaine())
            .fetch_optional(cnx)
            .await?;
    match derniere {
        Some(hex) => EmpreinteAudit::depuis_hex(&hex)
            .ok_or_else(|| ErreurBase::illisible("journal_audit", "empreinte", hex)),
        None => Ok(EmpreinteAudit::amorce(etablissement_id)),
    }
}

/// Écrit une ligne scellée.
pub(crate) async fn inserer(cnx: &mut Connexion, ligne: &LigneAudit) -> Resultat<()> {
    sqlx::query(
        "INSERT INTO journal_audit (id, horodatage, utilisateur_id, role_actif, etablissement_id, entite, entite_id, operation,
            avant, apres, contexte, poste, empreinte_machine, correlation_id, empreinte_precedente, empreinte)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
    )
    .bind(ligne.id.vers_chaine())
    .bind(ligne.horodatage.millis_epoque())
    .bind(ligne.utilisateur_id.map(|u| u.vers_chaine()))
    .bind(&ligne.role_actif)
    .bind(ligne.etablissement_id.vers_chaine())
    .bind(&ligne.entite)
    .bind(&ligne.entite_id)
    .bind(ligne.operation.nom())
    .bind(ligne.avant.as_ref().map(serde_json::to_string).transpose()?)
    .bind(ligne.apres.as_ref().map(serde_json::to_string).transpose()?)
    .bind(&ligne.contexte)
    .bind(&ligne.poste.0)
    .bind(&ligne.empreinte_machine)
    .bind(&ligne.correlation_id)
    .bind(ligne.empreinte_precedente.vers_hex())
    .bind(ligne.empreinte.vers_hex())
    .execute(cnx)
    .await?;
    Ok(())
}

#[derive(FromRow)]
struct Ligne {
    id: String,
    horodatage: i64,
    utilisateur_id: Option<String>,
    role_actif: Option<String>,
    etablissement_id: String,
    entite: String,
    entite_id: String,
    operation: String,
    avant: Option<String>,
    apres: Option<String>,
    contexte: String,
    poste: String,
    empreinte_machine: String,
    correlation_id: String,
    empreinte_precedente: String,
    empreinte: String,
}

impl Ligne {
    fn vers_entite(self) -> Resultat<LigneAudit> {
        Ok(LigneAudit {
            id: lire_id("journal_audit", "id", &self.id)?,
            horodatage: Horodatage::depuis_millis_epoque(self.horodatage),
            utilisateur_id: self
                .utilisateur_id
                .as_deref()
                .map(|u| lire_id("journal_audit", "utilisateur_id", u))
                .transpose()?,
            role_actif: self.role_actif,
            etablissement_id: lire_id("journal_audit", "etablissement_id", &self.etablissement_id)?,
            entite: self.entite,
            entite_id: self.entite_id,
            operation: Operation::depuis_nom(&self.operation).ok_or_else(|| {
                ErreurBase::illisible("journal_audit", "operation", &self.operation)
            })?,
            avant: self
                .avant
                .as_deref()
                .map(serde_json::from_str)
                .transpose()?,
            apres: self
                .apres
                .as_deref()
                .map(serde_json::from_str)
                .transpose()?,
            contexte: self.contexte,
            poste: Poste(self.poste),
            empreinte_machine: self.empreinte_machine,
            correlation_id: self.correlation_id,
            empreinte_precedente: EmpreinteAudit::depuis_hex(&self.empreinte_precedente)
                .ok_or_else(|| {
                    ErreurBase::illisible(
                        "journal_audit",
                        "empreinte_precedente",
                        &self.empreinte_precedente,
                    )
                })?,
            empreinte: EmpreinteAudit::depuis_hex(&self.empreinte).ok_or_else(|| {
                ErreurBase::illisible("journal_audit", "empreinte", &self.empreinte)
            })?,
        })
    }
}

/// Compose `SELECT <colonnes> FROM <table> <suite>` à partir de littéraux : sqlx refuse les
/// chaînes SQL dynamiques, et les colonnes ne sont écrites qu'une fois.
macro_rules! selectionner {
    ($suite:literal) => {
        concat!("SELECT id, horodatage, utilisateur_id, role_actif, etablissement_id, entite, entite_id, operation, avant, apres, contexte, poste, empreinte_machine, correlation_id, empreinte_precedente, empreinte FROM journal_audit ", $suite)
    };
}

/// Les `limite` dernières lignes de l'établissement, de la plus récente à la plus ancienne.
pub async fn dernieres(
    cnx: &mut Connexion,
    limite: i64,
    contexte: &Contexte,
) -> Resultat<Vec<LigneAudit>> {
    sqlx::query_as::<_, Ligne>(selectionner!(
        "WHERE etablissement_id = ?1 ORDER BY rowid DESC LIMIT ?2"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(limite)
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(Ligne::vers_entite)
    .collect()
}

/// Critères de recherche dans le journal ; un critère absent ne filtre pas.
#[derive(Debug, Clone, Default)]
pub struct FiltreAudit {
    /// Auteur de la mutation.
    pub utilisateur_id: Option<String>,
    /// Entité touchée (`utilisateur`, `role`…).
    pub entite: Option<String>,
    /// Opération, par son nom persisté.
    pub operation: Option<&'static str>,
    /// Instant de début inclus, en millisecondes.
    pub depuis: Option<i64>,
    /// Instant de fin exclu, en millisecondes.
    pub jusqua: Option<i64>,
}

/// Une page du journal filtré, de la plus récente à la plus ancienne ligne. Rend une ligne de
/// plus que demandé quand une page suivante existe.
pub async fn rechercher(
    cnx: &mut Connexion,
    filtre: &FiltreAudit,
    taille: i64,
    decalage: i64,
    contexte: &Contexte,
) -> Resultat<Vec<LigneAudit>> {
    sqlx::query_as::<_, Ligne>(selectionner!(
        "WHERE etablissement_id = ?1
           AND (?2 IS NULL OR utilisateur_id = ?2)
           AND (?3 IS NULL OR entite = ?3)
           AND (?4 IS NULL OR operation = ?4)
           AND (?5 IS NULL OR horodatage >= ?5)
           AND (?6 IS NULL OR horodatage < ?6)
         ORDER BY rowid DESC LIMIT ?7 OFFSET ?8"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(filtre.utilisateur_id.as_deref())
    .bind(filtre.entite.as_deref())
    .bind(filtre.operation)
    .bind(filtre.depuis)
    .bind(filtre.jusqua)
    .bind(taille + 1)
    .bind(decalage)
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(Ligne::vers_entite)
    .collect()
}

/// Entités présentes dans le journal de l'établissement, pour le filtre de l'écran.
pub async fn entites(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<String>> {
    Ok(sqlx::query_scalar(
        "SELECT DISTINCT entite FROM journal_audit WHERE etablissement_id = ?1 ORDER BY entite",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?)
}

/// Historique d'une entité, chronologique.
pub async fn par_entite(
    cnx: &mut Connexion,
    entite: &str,
    entite_id: &str,
    contexte: &Contexte,
) -> Resultat<Vec<LigneAudit>> {
    sqlx::query_as::<_, Ligne>(selectionner!(
        "WHERE etablissement_id = ?1 AND entite = ?2 AND entite_id = ?3 ORDER BY rowid"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(entite)
    .bind(entite_id)
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(Ligne::vers_entite)
    .collect()
}

/// Vérifie la chaîne complète d'un établissement (R-27). `limite` borne le nombre de lignes
/// vérifiées depuis la fin ; `None` vérifie tout.
pub async fn verifier_chaine(
    cnx: &mut Connexion,
    limite: Option<i64>,
    contexte: &Contexte,
) -> Resultat<usize> {
    let lignes = match limite {
        Some(n) => {
            let mut l = dernieres(cnx, n, contexte).await?;
            l.reverse();
            l
        }
        None => {
            sqlx::query_as::<_, Ligne>(selectionner!("WHERE etablissement_id = ?1 ORDER BY rowid"))
                .bind(contexte.etablissement_id.vers_chaine())
                .fetch_all(cnx)
                .await?
                .into_iter()
                .map(Ligne::vers_entite)
                .collect::<Resultat<Vec<_>>>()?
        }
    };
    // Une vérification partielle part de l'empreinte précédente de sa première ligne.
    let amorce = match (limite, lignes.first()) {
        (Some(_), Some(premiere)) => premiere.empreinte_precedente,
        _ => EmpreinteAudit::amorce(contexte.etablissement_id),
    };
    gesco_core::core::audit::verifier_chaine(&lignes, amorce).map_err(|r| {
        ErreurBase::ChaineRompue {
            index: r.index,
            ligne_id: r.ligne_id.vers_chaine(),
        }
    })?;
    Ok(lignes.len())
}
