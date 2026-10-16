//! Trésorerie (SPEC_PHASE_8) : catégories de recettes et de dépenses, comptes de l'école.

use gesco_core::commun::{DateCivile, Id, Montant};
use gesco_core::tresorerie::{CategorieTresorerie, CompteTresorerie, NatureCompte, Sens};
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::depots::booleen;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

fn cloisonne<T>(entite: &'static str, id: Id<T>, etablissement: bool) -> Resultat<()> {
    if etablissement {
        Ok(())
    } else {
        Err(ErreurBase::Cloisonnement {
            entite,
            id: id.vers_chaine(),
        })
    }
}

#[derive(FromRow)]
struct LigneCategorie {
    id: String,
    etablissement_id: String,
    code: Option<String>,
    sens: String,
    libelle: String,
    ordre: i64,
    active: i64,
}

impl LigneCategorie {
    fn vers_entite(self) -> Resultat<CategorieTresorerie> {
        let t = "categorie_tresorerie";
        Ok(CategorieTresorerie {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            code: self.code,
            sens: Sens::depuis_nom(&self.sens)
                .ok_or_else(|| ErreurBase::illisible(t, "sens", &self.sens))?,
            libelle: self.libelle,
            ordre: self.ordre,
            active: booleen(self.active),
        })
    }
}

/// Catégories de l'école, recettes puis dépenses, dans l'ordre.
pub async fn categories(
    cnx: &mut Connexion,
    contexte: &Contexte,
) -> Resultat<Vec<CategorieTresorerie>> {
    sqlx::query_as::<_, LigneCategorie>(
        "SELECT id, etablissement_id, code, sens, libelle, ordre, active FROM categorie_tresorerie
         WHERE etablissement_id = ?1 ORDER BY sens DESC, ordre, libelle",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneCategorie::vers_entite)
    .collect()
}

/// Enregistre une catégorie.
pub async fn inserer_categorie(
    cnx: &mut Connexion,
    c: &CategorieTresorerie,
    contexte: &Contexte,
) -> Resultat<()> {
    cloisonne(
        "categorie_tresorerie",
        c.id,
        c.etablissement_id == contexte.etablissement_id,
    )?;
    sqlx::query(
        "INSERT INTO categorie_tresorerie (id, etablissement_id, code, sens, libelle, ordre, active,
            cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?8, ?9, ?10)",
    )
    .bind(c.id.vers_chaine())
    .bind(c.etablissement_id.vers_chaine())
    .bind(c.code.as_deref())
    .bind(c.sens.nom())
    .bind(&c.libelle)
    .bind(c.ordre)
    .bind(i64::from(c.active))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Récrit le libellé, l'ordre et l'état d'une catégorie.
pub async fn ecrire_categorie(
    cnx: &mut Connexion,
    c: &CategorieTresorerie,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE categorie_tresorerie SET libelle = ?1, ordre = ?2, active = ?3,
            mis_a_jour_le = ?4, mis_a_jour_par = ?5, identifiant_poste = ?6
         WHERE id = ?7 AND etablissement_id = ?8",
    )
    .bind(&c.libelle)
    .bind(c.ordre)
    .bind(i64::from(c.active))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(c.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

#[derive(FromRow)]
struct LigneCompte {
    id: String,
    etablissement_id: String,
    nature: String,
    libelle: String,
    organisme: String,
    numero: String,
    solde_ouverture: i64,
    ouvert_le: String,
    actif: i64,
}

impl LigneCompte {
    fn vers_entite(self) -> Resultat<CompteTresorerie> {
        let t = "compte_tresorerie";
        Ok(CompteTresorerie {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            nature: NatureCompte::depuis_nom(&self.nature)
                .ok_or_else(|| ErreurBase::illisible(t, "nature", &self.nature))?,
            libelle: self.libelle,
            organisme: self.organisme,
            numero: self.numero,
            solde_ouverture: Montant::depuis_francs(self.solde_ouverture),
            ouvert_le: DateCivile::analyser(&self.ouvert_le)
                .ok_or_else(|| ErreurBase::illisible(t, "ouvert_le", &self.ouvert_le))?,
            actif: booleen(self.actif),
        })
    }
}

/// Comptes de l'école, la caisse d'abord.
pub async fn comptes(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<CompteTresorerie>> {
    sqlx::query_as::<_, LigneCompte>(
        "SELECT id, etablissement_id, nature, libelle, organisme, numero, solde_ouverture,
            ouvert_le, actif
         FROM compte_tresorerie WHERE etablissement_id = ?1
         ORDER BY nature <> 'caisse', nature, libelle",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneCompte::vers_entite)
    .collect()
}

/// Enregistre un compte.
pub async fn inserer_compte(
    cnx: &mut Connexion,
    c: &CompteTresorerie,
    contexte: &Contexte,
) -> Resultat<()> {
    cloisonne(
        "compte_tresorerie",
        c.id,
        c.etablissement_id == contexte.etablissement_id,
    )?;
    sqlx::query(
        "INSERT INTO compte_tresorerie (id, etablissement_id, nature, libelle, organisme, numero,
            solde_ouverture, ouvert_le, actif, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par,
            identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?10, ?11, ?12)",
    )
    .bind(c.id.vers_chaine())
    .bind(c.etablissement_id.vers_chaine())
    .bind(c.nature.nom())
    .bind(&c.libelle)
    .bind(&c.organisme)
    .bind(&c.numero)
    .bind(c.solde_ouverture.francs())
    .bind(c.ouvert_le.to_string())
    .bind(i64::from(c.actif))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Récrit un compte : libellé, organisme, numéro, solde et jour d'ouverture, état.
pub async fn ecrire_compte(
    cnx: &mut Connexion,
    c: &CompteTresorerie,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE compte_tresorerie SET libelle = ?1, organisme = ?2, numero = ?3,
            solde_ouverture = ?4, ouvert_le = ?5, actif = ?6, mis_a_jour_le = ?7,
            mis_a_jour_par = ?8, identifiant_poste = ?9
         WHERE id = ?10 AND etablissement_id = ?11",
    )
    .bind(&c.libelle)
    .bind(&c.organisme)
    .bind(&c.numero)
    .bind(c.solde_ouverture.francs())
    .bind(c.ouvert_le.to_string())
    .bind(i64::from(c.actif))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(c.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}
