//! Licence installée (spécification §13). Le dépôt garde le fichier ; sa vérification est l'affaire
//! du noyau, à chaque lecture.

use gesco_core::commun::{Horodatage, Id};
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::depots::horodatage_optionnel;
use crate::erreur::Resultat;
use crate::id::lire_id;

/// Licence enregistrée pour l'établissement courant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LicenceEnregistree {
    /// Identifiant technique de la ligne.
    pub id: Id<LicenceEnregistree>,
    /// Identifiant attribué par l'éditeur.
    pub licence_id: String,
    /// Fichier signé, tel qu'il a été importé.
    pub contenu: String,
    /// Empreinte du poste qui a installé la licence : celle à laquelle une licence libre s'attache.
    pub empreinte_attachee: Option<String>,
    /// Premier constat d'une empreinte machine différente de celle de la licence.
    pub ecart_empreinte_depuis: Option<Horodatage>,
    /// Instant d'installation.
    pub installee_le: Horodatage,
}

#[derive(FromRow)]
struct Ligne {
    id: String,
    licence_id: String,
    contenu: String,
    empreinte_attachee: Option<String>,
    ecart_empreinte_depuis: Option<i64>,
    cree_le: i64,
}

/// Licence en vigueur, s'il y en a une.
pub async fn courante(
    cnx: &mut Connexion,
    contexte: &Contexte,
) -> Resultat<Option<LicenceEnregistree>> {
    let Some(ligne) = sqlx::query_as::<_, Ligne>(
        "SELECT id, licence_id, contenu, empreinte_attachee, ecart_empreinte_depuis, cree_le FROM licence
         WHERE etablissement_id = ?1 AND supprime_le IS NULL",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    else {
        return Ok(None);
    };
    Ok(Some(LicenceEnregistree {
        id: lire_id("licence", "id", &ligne.id)?,
        licence_id: ligne.licence_id,
        contenu: ligne.contenu,
        empreinte_attachee: ligne.empreinte_attachee,
        ecart_empreinte_depuis: horodatage_optionnel(ligne.ecart_empreinte_depuis),
        installee_le: Horodatage::depuis_millis_epoque(ligne.cree_le),
    }))
}

/// Installe une licence sur le poste d'empreinte donnée ; celle qui était en vigueur est supprimée
/// logiquement.
pub async fn remplacer(
    cnx: &mut Connexion,
    id: Id<LicenceEnregistree>,
    licence_id: &str,
    contenu: &str,
    empreinte_poste: &str,
    contexte: &Contexte,
) -> Resultat<()> {
    let etablissement = contexte.etablissement_id.vers_chaine();
    let maintenant = contexte.maintenant.millis_epoque();
    sqlx::query(
        "UPDATE licence SET supprime_le = ?2, supprime_par = ?3, mis_a_jour_le = ?2, mis_a_jour_par = ?3
         WHERE etablissement_id = ?1 AND supprime_le IS NULL",
    )
    .bind(&etablissement)
    .bind(maintenant)
    .bind(contexte.auteur())
    .execute(&mut *cnx)
    .await?;
    sqlx::query(
        "INSERT INTO licence (id, etablissement_id, licence_id, contenu, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par,
            identifiant_poste, empreinte_attachee)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?5, ?6, ?7, ?8)",
    )
    .bind(id.vers_chaine())
    .bind(&etablissement)
    .bind(licence_id)
    .bind(contenu)
    .bind(maintenant)
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(empreinte_poste)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Note ou efface le constat d'écart d'empreinte de la licence en vigueur.
pub async fn noter_ecart(
    cnx: &mut Connexion,
    depuis: Option<Horodatage>,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE licence SET ecart_empreinte_depuis = ?2, mis_a_jour_le = ?3, mis_a_jour_par = ?4
         WHERE etablissement_id = ?1 AND supprime_le IS NULL",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(depuis.map(|d| d.millis_epoque()))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .execute(cnx)
    .await?;
    Ok(())
}
