//! Organisations et établissements.

use gesco_core::commun::Id;
use gesco_core::core::{Etablissement, Organisation, StatutEtablissement};
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

/// Crée une organisation. Hors cloisonnement par nature : elle précède tout établissement.
pub async fn inserer_organisation(
    cnx: &mut Connexion,
    organisation: &Organisation,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "INSERT INTO organisation (id, nom, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?3, ?4, ?5)",
    )
    .bind(organisation.id.vers_chaine())
    .bind(&organisation.nom)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Crée un établissement.
pub async fn inserer(
    cnx: &mut Connexion,
    etablissement: &Etablissement,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "INSERT INTO etablissement (id, organisation_id, nom, statut, code_academie, code_cap, adresse, telephones,
            mentions_legales, multiple_arrondi, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?11, ?12, ?13)",
    )
    .bind(etablissement.id.vers_chaine())
    .bind(etablissement.organisation_id.vers_chaine())
    .bind(&etablissement.nom)
    .bind(etablissement.statut.nom())
    .bind(&etablissement.code_academie)
    .bind(&etablissement.code_cap)
    .bind(&etablissement.adresse)
    .bind(serde_json::to_string(&etablissement.telephones)?)
    .bind(&etablissement.mentions_legales)
    .bind(etablissement.multiple_arrondi)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

#[derive(FromRow)]
struct Ligne {
    id: String,
    organisation_id: String,
    nom: String,
    statut: String,
    code_academie: String,
    code_cap: String,
    adresse: String,
    telephones: String,
    mentions_legales: String,
    multiple_arrondi: i64,
}

impl Ligne {
    fn vers_entite(self) -> Resultat<Etablissement> {
        Ok(Etablissement {
            id: lire_id("etablissement", "id", &self.id)?,
            organisation_id: lire_id("etablissement", "organisation_id", &self.organisation_id)?,
            nom: self.nom,
            statut: StatutEtablissement::depuis_nom(&self.statut)
                .ok_or_else(|| ErreurBase::illisible("etablissement", "statut", &self.statut))?,
            code_academie: self.code_academie,
            code_cap: self.code_cap,
            adresse: self.adresse,
            telephones: serde_json::from_str(&self.telephones)?,
            mentions_legales: self.mentions_legales,
            multiple_arrondi: self.multiple_arrondi,
        })
    }
}

/// Compose `SELECT <colonnes> FROM <table> <suite>` à partir de littéraux : sqlx refuse les
/// chaînes SQL dynamiques, et les colonnes ne sont écrites qu'une fois.
macro_rules! selectionner {
    ($suite:literal) => {
        concat!("SELECT id, organisation_id, nom, statut, code_academie, code_cap, adresse, telephones, mentions_legales, multiple_arrondi FROM etablissement ", $suite)
    };
}

/// Lit un établissement par identifiant.
pub async fn lire(cnx: &mut Connexion, id: Id<Etablissement>) -> Resultat<Etablissement> {
    sqlx::query_as::<_, Ligne>(selectionner!("WHERE id = ?1 AND supprime_le IS NULL"))
        .bind(id.vers_chaine())
        .fetch_optional(cnx)
        .await?
        .ok_or_else(|| ErreurBase::Introuvable {
            entite: "etablissement",
            id: id.vers_chaine(),
        })?
        .vers_entite()
}

/// Liste les établissements d'une organisation : la seule lecture qui traverse le cloisonnement,
/// réservée à la sélection d'établissement à l'ouverture (ADR-006).
pub async fn lister_par_organisation(
    cnx: &mut Connexion,
    organisation_id: Id<Organisation>,
) -> Resultat<Vec<Etablissement>> {
    sqlx::query_as::<_, Ligne>(selectionner!(
        "WHERE organisation_id = ?1 AND supprime_le IS NULL ORDER BY nom"
    ))
    .bind(organisation_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(Ligne::vers_entite)
    .collect()
}

/// Établissements présents dans la base du poste, par nom. Seule requête sans filtre
/// d'établissement (dérogation R-12) : au démarrage, avant toute session, l'application doit
/// savoir quel établissement elle sert.
pub async fn installes(cnx: &mut Connexion) -> Resultat<Vec<Etablissement>> {
    let ids: Vec<String> =
        sqlx::query_scalar("SELECT id FROM etablissement WHERE supprime_le IS NULL ORDER BY nom")
            .fetch_all(&mut *cnx)
            .await?;
    let mut etablissements = Vec::with_capacity(ids.len());
    for id in ids {
        let id = crate::id::lire_id("etablissement", "id", &id)?;
        etablissements.push(lire(&mut *cnx, id).await?);
    }
    Ok(etablissements)
}

/// Récrit la fiche de l'établissement du contexte (SPEC_PHASE_1 §2.1). L'organisation et le
/// multiple d'arrondi ne changent pas ici.
pub async fn modifier(
    cnx: &mut Connexion,
    etablissement: &Etablissement,
    contexte: &Contexte,
) -> Resultat<()> {
    if etablissement.id != contexte.etablissement_id {
        return Err(ErreurBase::Cloisonnement {
            entite: "etablissement",
            id: etablissement.id.vers_chaine(),
        });
    }
    sqlx::query(
        "UPDATE etablissement SET nom = ?1, statut = ?2, code_academie = ?3, code_cap = ?4, adresse = ?5,
            telephones = ?6, mentions_legales = ?7, mis_a_jour_le = ?8, mis_a_jour_par = ?9, identifiant_poste = ?10
         WHERE id = ?11 AND supprime_le IS NULL",
    )
    .bind(&etablissement.nom)
    .bind(etablissement.statut.nom())
    .bind(&etablissement.code_academie)
    .bind(&etablissement.code_cap)
    .bind(&etablissement.adresse)
    .bind(serde_json::to_string(&etablissement.telephones)?)
    .bind(&etablissement.mentions_legales)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(etablissement.id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Logo de l'établissement du contexte : octets et format (`png` ou `jpeg`).
pub async fn logo(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Option<(Vec<u8>, String)>> {
    let ligne: Option<(Option<Vec<u8>>, Option<String>)> = sqlx::query_as(
        "SELECT logo, logo_format FROM etablissement WHERE id = ?1 AND supprime_le IS NULL",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?;
    Ok(match ligne {
        Some((Some(octets), Some(format))) => Some((octets, format)),
        _ => None,
    })
}

/// Pose ou retire le logo de l'établissement du contexte.
pub async fn definir_logo(
    cnx: &mut Connexion,
    logo: Option<(&[u8], &str)>,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE etablissement SET logo = ?1, logo_format = ?2, mis_a_jour_le = ?3, mis_a_jour_par = ?4, identifiant_poste = ?5
         WHERE id = ?6 AND supprime_le IS NULL",
    )
    .bind(logo.map(|(octets, _)| octets))
    .bind(logo.map(|(_, format)| format))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}
