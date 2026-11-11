//! Deux personnes sur la même fiche (SPEC_PHASE_11_A §6.1) : la lecture rend l'instant de
//! dernière modification, la commande de modification le renvoie, et l'enregistrement ne se fait
//! que s'il n'a pas bougé. Sinon l'erreur nomme la personne et l'heure ; rien n'est écrit.

use gesco_db::Connexion;
use gesco_db::depots::fraicheur;
pub use gesco_db::depots::fraicheur::Ligne;

use crate::erreur::{ErreurService, Resultat};

/// Instant de dernière modification de la fiche que composent ces lignes, en millisecondes ;
/// zéro si aucune n'existe.
pub async fn instant(cnx: &mut Connexion, lignes: &[Ligne]) -> Resultat<i64> {
    Ok(fraicheur::derniere(cnx, lignes).await?.map_or(0, |d| d.le))
}

/// Refuse l'enregistrement si la fiche a changé depuis l'instant lu avec elle. À appeler dans
/// l'unité de travail qui écrit, avant la première écriture : les écritures se suivent une à
/// une, aucune autre ne peut donc se glisser entre la vérification et l'enregistrement.
pub async fn verifier(cnx: &mut Connexion, lignes: &[Ligne], lue_le: i64) -> Resultat<()> {
    let Some(derniere) = fraicheur::derniere(cnx, lignes).await? else {
        return Ok(());
    };
    if derniere.le == lue_le {
        return Ok(());
    }
    let par = match &derniere.par {
        Some(id) => fraicheur::nom_de(cnx, id).await?,
        None => None,
    };
    Err(ErreurService::ModifieeEntreTemps {
        par,
        le: derniere.le,
    })
}
