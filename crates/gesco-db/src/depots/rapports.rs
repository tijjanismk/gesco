//! Rapports de l'école et leurs textes (SPEC_PHASE_9 §8.6). Rien ne se supprime : un rapport
//! rouvert passe à sa version suivante, un texte se vide.

use gesco_core::commun::Id;
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::nouvel_id;

/// Un rapport tel que la base le range.
#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct LigneRapport {
    /// Identifiant.
    pub id: String,
    /// Année.
    pub annee_id: String,
    /// Type (`succinct_rentree`, `rentree`).
    pub type_rapport: String,
    /// Périmètre (`fondamental`).
    pub perimetre: String,
    /// État (`preparation`, `arrete`).
    pub etat: String,
    /// Version : 1, puis une de plus à chaque réouverture.
    pub version: i64,
    /// Date d'arrêt des chiffres, AAAA-MM-JJ.
    pub date_arret: String,
    /// Lieu de « Fait à ».
    pub lieu: String,
    /// Date de signature, AAAA-MM-JJ.
    pub date_signature: Option<String>,
    /// Qualité du signataire.
    pub qualite_signataire: String,
    /// Document imprimé à l'arrêt.
    pub document_id: Option<String>,
    /// Contenu figé à l'arrêt, en JSON.
    pub contenu_fige: Option<String>,
    /// Dernière modification, millisecondes.
    pub mis_a_jour_le: i64,
}

const COLONNES: &str = "id, annee_id, type_rapport, perimetre, etat, version, date_arret, lieu,
    date_signature, qualite_signataire, document_id, contenu_fige, mis_a_jour_le";

/// Rapports d'une année.
pub async fn de_l_annee(
    cnx: &mut Connexion,
    annee_id: &str,
    contexte: &Contexte,
) -> Resultat<Vec<LigneRapport>> {
    Ok(
        sqlx::query_as::<_, LigneRapport>(sqlx::AssertSqlSafe(format!(
            "SELECT {COLONNES} FROM rapport_ecole WHERE annee_id = ?1 AND etablissement_id = ?2
         ORDER BY type_rapport"
        )))
        .bind(annee_id)
        .bind(contexte.etablissement_id.vers_chaine())
        .fetch_all(cnx)
        .await?,
    )
}

/// Un rapport.
pub async fn lire(cnx: &mut Connexion, id: &str, contexte: &Contexte) -> Resultat<LigneRapport> {
    sqlx::query_as::<_, LigneRapport>(sqlx::AssertSqlSafe(format!(
        "SELECT {COLONNES} FROM rapport_ecole WHERE id = ?1 AND etablissement_id = ?2"
    )))
    .bind(id)
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .ok_or_else(|| ErreurBase::Introuvable {
        entite: "rapport_ecole",
        id: id.to_owned(),
    })
}

/// Le rapport d'un type pour une année et un périmètre, s'il existe.
pub async fn du_type(
    cnx: &mut Connexion,
    annee_id: &str,
    type_rapport: &str,
    perimetre: &str,
    contexte: &Contexte,
) -> Resultat<Option<LigneRapport>> {
    Ok(
        sqlx::query_as::<_, LigneRapport>(sqlx::AssertSqlSafe(format!(
            "SELECT {COLONNES} FROM rapport_ecole
         WHERE annee_id = ?1 AND type_rapport = ?2 AND perimetre = ?3 AND etablissement_id = ?4"
        )))
        .bind(annee_id)
        .bind(type_rapport)
        .bind(perimetre)
        .bind(contexte.etablissement_id.vers_chaine())
        .fetch_optional(cnx)
        .await?,
    )
}

/// Le dernier rapport, toutes années, qui a donné un lieu ou une qualité de signataire : ses
/// réglages sont proposés au rapport suivant.
pub async fn dernier_regle(
    cnx: &mut Connexion,
    contexte: &Contexte,
) -> Resultat<Option<LigneRapport>> {
    Ok(
        sqlx::query_as::<_, LigneRapport>(sqlx::AssertSqlSafe(format!(
            "SELECT {COLONNES} FROM rapport_ecole
         WHERE etablissement_id = ?1 AND (lieu <> '' OR qualite_signataire <> '')
         ORDER BY mis_a_jour_le DESC LIMIT 1"
        )))
        .bind(contexte.etablissement_id.vers_chaine())
        .fetch_optional(cnx)
        .await?,
    )
}

/// Ce qu'il faut pour créer un rapport.
#[derive(Debug, Clone, Copy)]
pub struct NouveauRapport<'a> {
    /// Année.
    pub annee_id: &'a str,
    /// Type.
    pub type_rapport: &'a str,
    /// Périmètre.
    pub perimetre: &'a str,
    /// Date d'arrêt proposée.
    pub date_arret: &'a str,
    /// Lieu proposé.
    pub lieu: &'a str,
    /// Qualité du signataire proposée.
    pub qualite_signataire: &'a str,
}

/// Crée un rapport en préparation ; rend son identifiant.
pub async fn inserer(
    cnx: &mut Connexion,
    n: NouveauRapport<'_>,
    contexte: &Contexte,
) -> Resultat<String> {
    let id: Id<LigneRapport> = nouvel_id();
    let id = id.vers_chaine();
    sqlx::query(
        "INSERT INTO rapport_ecole (id, etablissement_id, annee_id, type_rapport, perimetre,
            date_arret, lieu, qualite_signataire, cree_le, cree_par, mis_a_jour_le,
            mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?9, ?10, ?11)",
    )
    .bind(&id)
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(n.annee_id)
    .bind(n.type_rapport)
    .bind(n.perimetre)
    .bind(n.date_arret)
    .bind(n.lieu)
    .bind(n.qualite_signataire)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(id)
}

/// Réglages d'un rapport en préparation : date d'arrêt, lieu, date et qualité du signataire.
pub async fn regler(
    cnx: &mut Connexion,
    id: &str,
    date_arret: &str,
    lieu: &str,
    date_signature: Option<&str>,
    qualite_signataire: &str,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE rapport_ecole SET date_arret = ?1, lieu = ?2, date_signature = ?3,
            qualite_signataire = ?4, mis_a_jour_le = ?5, mis_a_jour_par = ?6,
            identifiant_poste = ?7
         WHERE id = ?8 AND etablissement_id = ?9",
    )
    .bind(date_arret)
    .bind(lieu)
    .bind(date_signature)
    .bind(qualite_signataire)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(id)
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Arrête un rapport en préparation : son document et son contenu figé. Rend faux s'il était
/// déjà arrêté (rien n'est écrit).
pub async fn arreter(
    cnx: &mut Connexion,
    id: &str,
    document_id: &str,
    contenu: &str,
    contexte: &Contexte,
) -> Resultat<bool> {
    let fait = sqlx::query(
        "UPDATE rapport_ecole SET etat = 'arrete', document_id = ?1, contenu_fige = ?2,
            arrete_le = ?3, arrete_par = ?4, mis_a_jour_le = ?3, mis_a_jour_par = ?4,
            identifiant_poste = ?5
         WHERE id = ?6 AND etablissement_id = ?7 AND etat = 'preparation'",
    )
    .bind(document_id)
    .bind(contenu)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(id)
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(fait.rows_affected() == 1)
}

/// Rouvre un rapport arrêté : version suivante, de nouveau en préparation. Le document de
/// l'arrêt reste dans `document`. Rend faux s'il n'était pas arrêté.
pub async fn rouvrir(cnx: &mut Connexion, id: &str, contexte: &Contexte) -> Resultat<bool> {
    let fait = sqlx::query(
        "UPDATE rapport_ecole SET etat = 'preparation', version = version + 1,
            document_id = NULL, contenu_fige = NULL, arrete_le = NULL, arrete_par = NULL,
            mis_a_jour_le = ?1, mis_a_jour_par = ?2, identifiant_poste = ?3
         WHERE id = ?4 AND etablissement_id = ?5 AND etat = 'arrete'",
    )
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(id)
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(fait.rows_affected() == 1)
}

/// Un texte de section.
#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct TexteRapport {
    /// Section.
    pub section: String,
    /// Texte.
    pub texte: String,
    /// Dernière modification, millisecondes.
    pub mis_a_jour_le: i64,
}

/// Textes d'un rapport.
pub async fn textes(
    cnx: &mut Connexion,
    rapport_id: &str,
    contexte: &Contexte,
) -> Resultat<Vec<TexteRapport>> {
    Ok(sqlx::query_as::<_, TexteRapport>(
        "SELECT section, texte, mis_a_jour_le FROM texte_rapport
         WHERE rapport_id = ?1 AND etablissement_id = ?2",
    )
    .bind(rapport_id)
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?)
}

/// Écrit le texte d'une section : la ligne existante est récrite, sinon elle est créée. Rend le
/// texte d'avant, pour l'audit.
pub async fn ecrire_texte(
    cnx: &mut Connexion,
    rapport_id: &str,
    section: &str,
    texte: &str,
    contexte: &Contexte,
) -> Resultat<Option<String>> {
    let avant: Option<(String, String)> = sqlx::query_as(
        "SELECT id, texte FROM texte_rapport
         WHERE rapport_id = ?1 AND section = ?2 AND etablissement_id = ?3",
    )
    .bind(rapport_id)
    .bind(section)
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(&mut *cnx)
    .await?;
    match avant {
        Some((id, ancien)) => {
            sqlx::query(
                "UPDATE texte_rapport SET texte = ?1, mis_a_jour_le = ?2, mis_a_jour_par = ?3,
                    identifiant_poste = ?4 WHERE id = ?5",
            )
            .bind(texte)
            .bind(contexte.maintenant.millis_epoque())
            .bind(contexte.auteur())
            .bind(&contexte.poste.0)
            .bind(id)
            .execute(cnx)
            .await?;
            Ok(Some(ancien))
        }
        None => {
            let id: Id<TexteRapport> = nouvel_id();
            sqlx::query(
                "INSERT INTO texte_rapport (id, etablissement_id, rapport_id, section, texte,
                    cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?6, ?7, ?8)",
            )
            .bind(id.vers_chaine())
            .bind(contexte.etablissement_id.vers_chaine())
            .bind(rapport_id)
            .bind(section)
            .bind(texte)
            .bind(contexte.maintenant.millis_epoque())
            .bind(contexte.auteur())
            .bind(&contexte.poste.0)
            .execute(cnx)
            .await?;
            Ok(None)
        }
    }
}
