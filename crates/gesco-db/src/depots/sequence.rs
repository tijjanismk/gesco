//! Numérotation séquentielle sans trou (R-46). L'attribution se fait dans la transaction de
//! l'opération, sous le verrou d'écriture pris par `BEGIN IMMEDIATE`.

use gesco_core::core::{FormatNumero, TypeDocumentNumerote};

use crate::Connexion;
use crate::contexte::Contexte;
use crate::erreur::{ErreurBase, Resultat};

/// Attribue le prochain numéro de la séquence `(établissement, type, exercice)` et le compose.
/// La séquence est créée à la première demande avec le format par défaut.
pub async fn prochain_numero(
    cnx: &mut Connexion,
    type_document: TypeDocumentNumerote,
    exercice: &str,
    contexte: &Contexte,
) -> Resultat<String> {
    let (compteur, format) = prochain_compteur(cnx, type_document, exercice, contexte).await?;
    Ok(format.formater(exercice, compteur))
}

/// Avance le compteur de la séquence et le rend avec son format enregistré : pour un numéro dont
/// la forme suit d'autres règles, comme le matricule d'un élève.
pub async fn prochain_compteur(
    cnx: &mut Connexion,
    type_document: TypeDocumentNumerote,
    exercice: &str,
    contexte: &Contexte,
) -> Resultat<(u64, FormatNumero)> {
    let format_defaut = serde_json::to_string(&FormatNumero::par_defaut(type_document))?;
    sqlx::query(
        "INSERT INTO sequence_numerotation (etablissement_id, type_document, exercice, compteur, format)
         VALUES (?1, ?2, ?3, 0, ?4)
         ON CONFLICT(etablissement_id, type_document, exercice) DO NOTHING",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(type_document.code())
    .bind(exercice)
    .bind(format_defaut)
    .execute(&mut *cnx)
    .await?;
    let (compteur, format): (i64, String) = sqlx::query_as(
        "UPDATE sequence_numerotation SET compteur = compteur + 1
         WHERE etablissement_id = ?1 AND type_document = ?2 AND exercice = ?3
         RETURNING compteur, format",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(type_document.code())
    .bind(exercice)
    .fetch_one(&mut *cnx)
    .await?;
    let format: FormatNumero = serde_json::from_str(&format)?;
    let compteur = u64::try_from(compteur)
        .map_err(|e| ErreurBase::illisible("sequence_numerotation", "compteur", e))?;
    Ok((compteur, format))
}

/// Valeur courante du compteur, sans l'incrémenter.
pub async fn compteur(
    cnx: &mut Connexion,
    type_document: TypeDocumentNumerote,
    exercice: &str,
    contexte: &Contexte,
) -> Resultat<i64> {
    let valeur: Option<i64> = sqlx::query_scalar(
        "SELECT compteur FROM sequence_numerotation WHERE etablissement_id = ?1 AND type_document = ?2 AND exercice = ?3",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(type_document.code())
    .bind(exercice)
    .fetch_optional(cnx)
    .await?;
    Ok(valeur.unwrap_or(0))
}

/// Un numéro attribué puis abandonné.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumeroAbandonne {
    /// Numéro formaté.
    pub numero: String,
    /// Type de document.
    pub type_document: TypeDocumentNumerote,
    /// Exercice de la séquence.
    pub exercice: String,
    /// Motif de l'abandon, obligatoire.
    pub motif: String,
}

/// Enregistre l'abandon d'un numéro (R-46). Le numéro reste consommé dans la séquence.
pub async fn abandonner(
    cnx: &mut Connexion,
    abandon: &NumeroAbandonne,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "INSERT INTO numero_abandonne (numero, etablissement_id, type_document, exercice, motif,
            abandonne_le, abandonne_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
    )
    .bind(&abandon.numero)
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(abandon.type_document.code())
    .bind(&abandon.exercice)
    .bind(&abandon.motif)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Numéros abandonnés d'une séquence, dans l'ordre d'attribution.
pub async fn abandonnes(
    cnx: &mut Connexion,
    type_document: TypeDocumentNumerote,
    exercice: &str,
    contexte: &Contexte,
) -> Resultat<Vec<String>> {
    Ok(sqlx::query_scalar(
        "SELECT numero FROM numero_abandonne
         WHERE etablissement_id = ?1 AND type_document = ?2 AND exercice = ?3 ORDER BY numero",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(type_document.code())
    .bind(exercice)
    .fetch_all(cnx)
    .await?)
}
