//! Années scolaires.

use gesco_core::commun::{Horodatage, Id};
use gesco_core::core::{AnneeScolaire, Etablissement, MachineAEtats, StatutAnnee};
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::depots::booleen;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

#[derive(FromRow)]
struct Ligne {
    id: String,
    etablissement_id: String,
    libelle: String,
    debut: i64,
    fin: i64,
    statut: String,
    est_courante: i64,
}

impl Ligne {
    fn vers_entite(self) -> Resultat<AnneeScolaire> {
        Ok(AnneeScolaire {
            id: lire_id("annee_scolaire", "id", &self.id)?,
            etablissement_id: lire_id(
                "annee_scolaire",
                "etablissement_id",
                &self.etablissement_id,
            )?,
            libelle: self.libelle,
            debut: Horodatage::depuis_millis_epoque(self.debut),
            fin: Horodatage::depuis_millis_epoque(self.fin),
            statut: StatutAnnee::depuis_nom(&self.statut)
                .ok_or_else(|| ErreurBase::illisible("annee_scolaire", "statut", &self.statut))?,
            est_courante: booleen(self.est_courante),
        })
    }
}

/// Compose `SELECT <colonnes> FROM <table> <suite>` à partir de littéraux : sqlx refuse les
/// chaînes SQL dynamiques, et les colonnes ne sont écrites qu'une fois.
macro_rules! selectionner {
    ($suite:literal) => {
        concat!("SELECT id, etablissement_id, libelle, debut, fin, statut, est_courante FROM annee_scolaire ", $suite)
    };
}

/// Crée une année dans l'établissement du contexte.
pub async fn inserer(
    cnx: &mut Connexion,
    annee: &AnneeScolaire,
    contexte: &Contexte,
) -> Resultat<()> {
    verifier_etablissement(annee.etablissement_id, contexte, annee.id)?;
    sqlx::query(
        "INSERT INTO annee_scolaire (id, etablissement_id, libelle, debut, fin, statut, est_courante,
            cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?8, ?9, ?10)",
    )
    .bind(annee.id.vers_chaine())
    .bind(annee.etablissement_id.vers_chaine())
    .bind(&annee.libelle)
    .bind(annee.debut.millis_epoque())
    .bind(annee.fin.millis_epoque())
    .bind(annee.statut.nom())
    .bind(i64::from(annee.est_courante))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Lit une année de l'établissement du contexte.
pub async fn lire(
    cnx: &mut Connexion,
    id: Id<AnneeScolaire>,
    contexte: &Contexte,
) -> Resultat<AnneeScolaire> {
    sqlx::query_as::<_, Ligne>(selectionner!(
        "WHERE id = ?1 AND etablissement_id = ?2 AND supprime_le IS NULL"
    ))
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .ok_or_else(|| ErreurBase::Introuvable {
        entite: "annee_scolaire",
        id: id.vers_chaine(),
    })?
    .vers_entite()
}

/// Année courante de l'établissement, s'il y en a une.
pub async fn courante(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Option<AnneeScolaire>> {
    sqlx::query_as::<_, Ligne>(selectionner!(
        "WHERE etablissement_id = ?1 AND est_courante = 1 AND supprime_le IS NULL"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .map(Ligne::vers_entite)
    .transpose()
}

/// Toutes les années de l'établissement, la plus récente d'abord.
pub async fn lister(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<AnneeScolaire>> {
    sqlx::query_as::<_, Ligne>(selectionner!(
        "WHERE etablissement_id = ?1 AND supprime_le IS NULL ORDER BY debut DESC"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(Ligne::vers_entite)
    .collect()
}

/// Applique une transition déjà vérifiée par le noyau (R-30) : le dépôt n'écrit que l'état.
pub async fn ecrire_statut(
    cnx: &mut Connexion,
    id: Id<AnneeScolaire>,
    statut: StatutAnnee,
    contexte: &Contexte,
) -> Resultat<()> {
    let modifiees = sqlx::query(
        "UPDATE annee_scolaire SET statut = ?1, mis_a_jour_le = ?2, mis_a_jour_par = ?3
         WHERE id = ?4 AND etablissement_id = ?5 AND supprime_le IS NULL",
    )
    .bind(statut.nom())
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?
    .rows_affected();
    if modifiees == 0 {
        return Err(ErreurBase::Introuvable {
            entite: "annee_scolaire",
            id: id.vers_chaine(),
        });
    }
    Ok(())
}

/// Désigne l'année courante ; l'ancienne cesse de l'être dans la même requête.
pub async fn definir_courante(
    cnx: &mut Connexion,
    id: Id<AnneeScolaire>,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE annee_scolaire SET est_courante = 0, mis_a_jour_le = ?1, mis_a_jour_par = ?2
         WHERE etablissement_id = ?3 AND est_courante = 1 AND supprime_le IS NULL",
    )
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(&mut *cnx)
    .await?;
    let modifiees = sqlx::query(
        "UPDATE annee_scolaire SET est_courante = 1, mis_a_jour_le = ?1, mis_a_jour_par = ?2
         WHERE id = ?3 AND etablissement_id = ?4 AND supprime_le IS NULL",
    )
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?
    .rows_affected();
    if modifiees == 0 {
        return Err(ErreurBase::Introuvable {
            entite: "annee_scolaire",
            id: id.vers_chaine(),
        });
    }
    Ok(())
}

fn verifier_etablissement(
    proprietaire: Id<Etablissement>,
    contexte: &Contexte,
    id: Id<AnneeScolaire>,
) -> Resultat<()> {
    if proprietaire != contexte.etablissement_id {
        return Err(ErreurBase::Cloisonnement {
            entite: "annee_scolaire",
            id: id.vers_chaine(),
        });
    }
    Ok(())
}

/// Récrit le libellé et les bornes d'une année ; le statut ne change que par `ecrire_statut`.
pub async fn modifier(
    cnx: &mut Connexion,
    annee: &AnneeScolaire,
    contexte: &Contexte,
) -> Resultat<()> {
    verifier_etablissement(annee.etablissement_id, contexte, annee.id)?;
    sqlx::query(
        "UPDATE annee_scolaire SET libelle = ?1, debut = ?2, fin = ?3, mis_a_jour_le = ?4, mis_a_jour_par = ?5
         WHERE id = ?6 AND etablissement_id = ?7 AND supprime_le IS NULL",
    )
    .bind(&annee.libelle)
    .bind(annee.debut.millis_epoque())
    .bind(annee.fin.millis_epoque())
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(annee.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Supprime logiquement une année.
pub async fn supprimer(
    cnx: &mut Connexion,
    id: Id<AnneeScolaire>,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE annee_scolaire SET supprime_le = ?1, supprime_par = ?2, mis_a_jour_le = ?1, mis_a_jour_par = ?2
         WHERE id = ?3 AND etablissement_id = ?4 AND supprime_le IS NULL",
    )
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}
