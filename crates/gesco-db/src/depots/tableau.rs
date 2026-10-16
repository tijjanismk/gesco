//! Requêtes de synthèse du tableau de bord : des comptes, jamais de détail.

use gesco_core::commun::{DateCivile, Id};
use gesco_core::core::AnneeScolaire;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::erreur::Resultat;

/// Inscriptions d'une classe par sexe et par état.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LigneEffectif {
    /// Libellé de la classe.
    pub classe: String,
    /// Libellé du niveau.
    pub niveau: String,
    /// Cycle, nom persisté.
    pub cycle: String,
    /// Ordre du niveau.
    pub ordre: i64,
    /// Sexe, nom persisté ; vide s'il n'est pas renseigné.
    pub sexe: String,
    /// État de l'inscription, nom persisté.
    pub statut: String,
    /// Nombre d'inscriptions.
    pub nombre: i64,
}

/// Ligne lue : classe, niveau, cycle, ordre du niveau, sexe, état, nombre.
type LigneBrute = (String, String, String, i64, Option<String>, String, i64);

/// Inscriptions de l'année par classe, sexe et état.
pub async fn effectifs(
    cnx: &mut Connexion,
    annee_id: Id<AnneeScolaire>,
    contexte: &Contexte,
) -> Resultat<Vec<LigneEffectif>> {
    let lignes: Vec<LigneBrute> = sqlx::query_as(
        "SELECT c.libelle, n.libelle, n.cycle, n.ordre, p.sexe, i.statut, count(*)
         FROM inscription i
         JOIN classe c ON c.id = i.classe_id
         JOIN niveau n ON n.id = c.niveau_id
         JOIN eleve e ON e.id = i.eleve_id
         JOIN personne p ON p.id = e.personne_id
         WHERE i.annee_id = ?1 AND i.etablissement_id = ?2
         GROUP BY c.id, p.sexe, i.statut
         ORDER BY n.ordre, c.libelle",
    )
    .bind(annee_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?;
    Ok(lignes
        .into_iter()
        .map(
            |(classe, niveau, cycle, ordre, sexe, statut, nombre)| LigneEffectif {
                classe,
                niveau,
                cycle,
                ordre,
                sexe: sexe.unwrap_or_default(),
                statut,
                nombre,
            },
        )
        .collect())
}

async fn compter(
    cnx: &mut Connexion,
    requete: &'static str,
    parametre: String,
    contexte: &Contexte,
) -> Resultat<u32> {
    let (n,): (i64,) = sqlx::query_as(requete)
        .bind(parametre)
        .bind(contexte.etablissement_id.vers_chaine())
        .fetch_one(cnx)
        .await?;
    Ok(u32::try_from(n).unwrap_or(0))
}

/// Demi-journées d'absence et retards relevés un jour, toutes classes : (absences, retards).
pub async fn absences_du_jour(
    cnx: &mut Connexion,
    jour: DateCivile,
    contexte: &Contexte,
) -> Resultat<(u32, u32)> {
    let absences = compter(
        cnx,
        "SELECT count(*) FROM absence WHERE date = ?1 AND etablissement_id = ?2 AND retiree = 0 AND nature = 'absence'",
        jour.to_string(),
        contexte,
    )
    .await?;
    let retards = compter(
        cnx,
        "SELECT count(*) FROM absence WHERE date = ?1 AND etablissement_id = ?2 AND retiree = 0 AND nature = 'retard'",
        jour.to_string(),
        contexte,
    )
    .await?;
    Ok((absences, retards))
}

/// Classes dont le relevé des absences est fait ce jour.
pub async fn releves_du_jour(
    cnx: &mut Connexion,
    jour: DateCivile,
    contexte: &Contexte,
) -> Resultat<u32> {
    compter(
        cnx,
        "SELECT count(*) FROM releve_absences WHERE date = ?1 AND etablissement_id = ?2",
        jour.to_string(),
        contexte,
    )
    .await
}

/// Évaluations de l'année encore en saisie.
pub async fn evaluations_en_saisie(
    cnx: &mut Connexion,
    annee_id: Id<AnneeScolaire>,
    contexte: &Contexte,
) -> Resultat<u32> {
    compter(
        cnx,
        "SELECT count(*) FROM evaluation WHERE annee_id = ?1 AND etablissement_id = ?2 AND statut = 'en_saisie'",
        annee_id.vers_chaine(),
        contexte,
    )
    .await
}

/// Bulletins de l'année dans un état : `calcule` (à valider) ou `valide` (à publier).
pub async fn bulletins_en_etat(
    cnx: &mut Connexion,
    annee_id: Id<AnneeScolaire>,
    statut: &'static str,
    contexte: &Contexte,
) -> Resultat<u32> {
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM bulletin b JOIN classe c ON c.id = b.classe_id
         WHERE c.annee_id = ?1 AND b.etablissement_id = ?2 AND b.statut = ?3",
    )
    .bind(annee_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(statut)
    .fetch_one(cnx)
    .await?;
    Ok(u32::try_from(n).unwrap_or(0))
}

/// Exclusions en cours un jour : temporaires qui couvrent ce jour, définitives non révoquées.
pub async fn exclusions_en_cours(
    cnx: &mut Connexion,
    jour: DateCivile,
    contexte: &Contexte,
) -> Resultat<u32> {
    compter(
        cnx,
        "SELECT count(*) FROM exclusion WHERE statut = 'en_cours' AND etablissement_id = ?2
            AND (nature = 'definitive' OR (du <= ?1 AND au >= ?1))",
        jour.to_string(),
        contexte,
    )
    .await
}
