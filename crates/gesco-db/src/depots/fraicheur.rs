//! Instant de dernière modification d'une fiche (SPEC_PHASE_11_A §6.1). Une fiche tient souvent
//! sur plusieurs lignes (l'élève et son identité, le responsable, son identité et son lien) :
//! son instant est le plus récent de ces lignes, et son auteur celui de cette ligne-là.

use sqlx::Row;

use crate::Connexion;
use crate::erreur::Resultat;

/// Ligne qui compose une fiche : sa table et les colonnes qui la désignent. Les noms viennent
/// du code, jamais de l'écran.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ligne {
    /// Table.
    pub table: &'static str,
    /// Colonnes de la clé et leurs valeurs.
    pub cles: Vec<(&'static str, String)>,
}

impl Ligne {
    /// Ligne désignée par sa colonne `id`.
    pub fn par_id(table: &'static str, id: String) -> Self {
        Self {
            table,
            cles: vec![("id", id)],
        }
    }
}

/// Dernière modification d'une fiche.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerniereModification {
    /// Instant, millisecondes depuis l'époque.
    pub le: i64,
    /// Utilisateur qui l'a faite ; absent pour le système.
    pub par: Option<String>,
}

/// Dernière modification parmi les lignes d'une fiche ; `None` si aucune n'existe.
pub async fn derniere(
    cnx: &mut Connexion,
    lignes: &[Ligne],
) -> Resultat<Option<DerniereModification>> {
    let mut derniere: Option<DerniereModification> = None;
    for ligne in lignes {
        let conditions = ligne
            .cles
            .iter()
            .enumerate()
            .map(|(i, (colonne, _))| format!("{colonne} = ?{}", i + 1))
            .collect::<Vec<_>>()
            .join(" AND ");
        let requete = format!(
            "SELECT mis_a_jour_le, mis_a_jour_par FROM {} WHERE {conditions}
             ORDER BY mis_a_jour_le DESC LIMIT 1",
            ligne.table
        );
        // Noms de table et de colonnes écrits dans le code, valeurs liées : rien ne vient de
        // l'écran dans le texte de la requête.
        let mut q = sqlx::query(sqlx::AssertSqlSafe(requete));
        for (_, valeur) in &ligne.cles {
            q = q.bind(valeur);
        }
        if let Some(trouvee) = q.fetch_optional(&mut *cnx).await? {
            let le: i64 = trouvee.try_get(0)?;
            let par: Option<String> = trouvee.try_get(1)?;
            if derniere.as_ref().is_none_or(|d| le > d.le) {
                derniere = Some(DerniereModification { le, par });
            }
        }
    }
    Ok(derniere)
}

/// Nom affiché d'un utilisateur, pour dire qui a modifié la fiche.
pub async fn nom_de(cnx: &mut Connexion, utilisateur_id: &str) -> Resultat<Option<String>> {
    Ok(
        sqlx::query_scalar("SELECT nom_affiche FROM utilisateur WHERE id = ?1")
            .bind(utilisateur_id)
            .fetch_optional(cnx)
            .await?,
    )
}
