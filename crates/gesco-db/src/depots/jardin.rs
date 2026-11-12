//! Domaines et compétences du jardin d'enfants (SPEC_PHASE_11_B §3.1). Rien ne se supprime : un
//! domaine ou une compétence se retire (`retire_le`, `retiree_le`) et se remet.

use gesco_core::commun::Id;
use gesco_core::pedagogie::{Competence, DomaineCompetence};
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::depots::{booleen, horodatage_optionnel};
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

fn cloisonner(entite: &'static str, id: String, contexte: &Contexte, de: String) -> Resultat<()> {
    if de != contexte.etablissement_id.vers_chaine() {
        return Err(ErreurBase::Cloisonnement { entite, id });
    }
    Ok(())
}

// --- Domaines -------------------------------------------------------------------------------

#[derive(FromRow)]
struct LigneDomaine {
    id: String,
    etablissement_id: String,
    code: Option<String>,
    libelle: String,
    ordre: i64,
    livre: i64,
    retire_le: Option<i64>,
}

impl LigneDomaine {
    fn vers_entite(self) -> Resultat<DomaineCompetence> {
        let t = "domaine_competence";
        Ok(DomaineCompetence {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            code: self.code,
            libelle: self.libelle,
            ordre: self.ordre,
            livre: booleen(self.livre),
            retire_le: horodatage_optionnel(self.retire_le),
        })
    }
}

/// Domaines de l'établissement, retirés compris, dans l'ordre d'affichage.
pub async fn domaines(
    cnx: &mut Connexion,
    contexte: &Contexte,
) -> Resultat<Vec<DomaineCompetence>> {
    sqlx::query_as::<_, LigneDomaine>(
        "SELECT id, etablissement_id, code, libelle, ordre, livre, retire_le FROM domaine_competence
         WHERE etablissement_id = ?1 ORDER BY ordre, libelle",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneDomaine::vers_entite)
    .collect()
}

/// Enregistre un domaine.
pub async fn inserer_domaine(
    cnx: &mut Connexion,
    d: &DomaineCompetence,
    contexte: &Contexte,
) -> Resultat<()> {
    cloisonner(
        "domaine_competence",
        d.id.vers_chaine(),
        contexte,
        d.etablissement_id.vers_chaine(),
    )?;
    sqlx::query(
        "INSERT INTO domaine_competence (id, etablissement_id, code, libelle, ordre, livre,
            retire_le, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?8, ?9, ?10)",
    )
    .bind(d.id.vers_chaine())
    .bind(d.etablissement_id.vers_chaine())
    .bind(d.code.as_deref())
    .bind(&d.libelle)
    .bind(d.ordre)
    .bind(i64::from(d.livre))
    .bind(d.retire_le.map(|h| h.millis_epoque()))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Récrit un domaine : libellé, ordre, retrait.
pub async fn ecrire_domaine(
    cnx: &mut Connexion,
    d: &DomaineCompetence,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE domaine_competence SET libelle = ?1, ordre = ?2, retire_le = ?3,
            mis_a_jour_le = ?4, mis_a_jour_par = ?5, identifiant_poste = ?6
         WHERE id = ?7 AND etablissement_id = ?8",
    )
    .bind(&d.libelle)
    .bind(d.ordre)
    .bind(d.retire_le.map(|h| h.millis_epoque()))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(d.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

// --- Compétences ----------------------------------------------------------------------------

#[derive(FromRow)]
struct LigneCompetence {
    id: String,
    etablissement_id: String,
    domaine_id: String,
    niveau_id: String,
    libelle: String,
    ordre: i64,
    livree: i64,
    retiree_le: Option<i64>,
}

impl LigneCompetence {
    fn vers_entite(self) -> Resultat<Competence> {
        let t = "competence";
        Ok(Competence {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            domaine_id: lire_id(t, "domaine_id", &self.domaine_id)?,
            niveau_id: lire_id(t, "niveau_id", &self.niveau_id)?,
            libelle: self.libelle,
            ordre: self.ordre,
            livree: booleen(self.livree),
            retiree_le: horodatage_optionnel(self.retiree_le),
        })
    }
}

/// Compétences de l'établissement, retirées comprises, dans l'ordre de chaque domaine.
pub async fn competences(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<Competence>> {
    sqlx::query_as::<_, LigneCompetence>(
        "SELECT id, etablissement_id, domaine_id, niveau_id, libelle, ordre, livree, retiree_le
         FROM competence WHERE etablissement_id = ?1 ORDER BY niveau_id, domaine_id, ordre, libelle",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneCompetence::vers_entite)
    .collect()
}

/// Une compétence de l'établissement.
pub async fn competence(
    cnx: &mut Connexion,
    id: Id<Competence>,
    contexte: &Contexte,
) -> Resultat<Competence> {
    sqlx::query_as::<_, LigneCompetence>(
        "SELECT id, etablissement_id, domaine_id, niveau_id, libelle, ordre, livree, retiree_le
         FROM competence WHERE etablissement_id = ?1 AND id = ?2",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .ok_or_else(|| ErreurBase::Introuvable {
        entite: "competence",
        id: id.vers_chaine(),
    })?
    .vers_entite()
}

/// Enregistre une compétence.
pub async fn inserer_competence(
    cnx: &mut Connexion,
    c: &Competence,
    contexte: &Contexte,
) -> Resultat<()> {
    cloisonner(
        "competence",
        c.id.vers_chaine(),
        contexte,
        c.etablissement_id.vers_chaine(),
    )?;
    sqlx::query(
        "INSERT INTO competence (id, etablissement_id, domaine_id, niveau_id, libelle, ordre,
            livree, retiree_le, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par,
            identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?9, ?10, ?11)",
    )
    .bind(c.id.vers_chaine())
    .bind(c.etablissement_id.vers_chaine())
    .bind(c.domaine_id.vers_chaine())
    .bind(c.niveau_id.vers_chaine())
    .bind(&c.libelle)
    .bind(c.ordre)
    .bind(i64::from(c.livree))
    .bind(c.retiree_le.map(|h| h.millis_epoque()))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Récrit une compétence : domaine, libellé, ordre, retrait. La section ne change pas.
pub async fn ecrire_competence(
    cnx: &mut Connexion,
    c: &Competence,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE competence SET domaine_id = ?1, libelle = ?2, ordre = ?3, retiree_le = ?4,
            mis_a_jour_le = ?5, mis_a_jour_par = ?6, identifiant_poste = ?7
         WHERE id = ?8 AND etablissement_id = ?9",
    )
    .bind(c.domaine_id.vers_chaine())
    .bind(&c.libelle)
    .bind(c.ordre)
    .bind(c.retiree_le.map(|h| h.millis_epoque()))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(c.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}
