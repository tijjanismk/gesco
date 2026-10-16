//! Départs et exclusions (SPEC_PHASE_4 §9). Une exclusion ne se supprime jamais (R-14).

use gesco_core::commun::{DateCivile, Id};
use gesco_core::core::MachineAEtats;
use gesco_core::finance::Echeance;
use gesco_core::scolarite::{Exclusion, Inscription, NatureExclusion, StatutExclusion};
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

#[derive(FromRow)]
struct LigneExclusion {
    id: String,
    etablissement_id: String,
    inscription_id: String,
    nature: String,
    du: String,
    au: Option<String>,
    motif: String,
    decision: String,
    statut: String,
    leve_le: Option<String>,
    motif_levee: String,
    echeances_annulees: String,
}

impl LigneExclusion {
    fn vers_entite(self) -> Resultat<Exclusion> {
        let t = "exclusion";
        let date = |colonne: &'static str, texte: &str| {
            DateCivile::analyser(texte).ok_or_else(|| ErreurBase::illisible(t, colonne, texte))
        };
        let ids: Vec<String> = serde_json::from_str(&self.echeances_annulees)?;
        Ok(Exclusion {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            inscription_id: lire_id(t, "inscription_id", &self.inscription_id)?,
            nature: NatureExclusion::depuis_nom(&self.nature)
                .ok_or_else(|| ErreurBase::illisible(t, "nature", &self.nature))?,
            du: date("du", &self.du)?,
            au: self.au.as_deref().map(|a| date("au", a)).transpose()?,
            motif: self.motif,
            decision: self.decision,
            statut: StatutExclusion::depuis_nom(&self.statut)
                .ok_or_else(|| ErreurBase::illisible(t, "statut", &self.statut))?,
            leve_le: self
                .leve_le
                .as_deref()
                .map(|l| date("leve_le", l))
                .transpose()?,
            motif_levee: self.motif_levee,
            echeances_annulees: ids
                .iter()
                .map(|i| lire_id::<Echeance>(t, "echeances_annulees", i))
                .collect::<Resultat<Vec<_>>>()?,
        })
    }
}

macro_rules! selectionner_exclusion {
    ($suite:literal) => {
        concat!(
            "SELECT id, etablissement_id, inscription_id, nature, du, au, motif, decision, statut, leve_le,
                motif_levee, echeances_annulees FROM exclusion ",
            $suite
        )
    };
}

fn ids_en_json(ids: &[Id<Echeance>]) -> Resultat<String> {
    Ok(serde_json::to_string(
        &ids.iter().map(|i| i.vers_chaine()).collect::<Vec<_>>(),
    )?)
}

/// Enregistre une exclusion.
pub async fn inserer_exclusion(
    cnx: &mut Connexion,
    e: &Exclusion,
    contexte: &Contexte,
) -> Resultat<()> {
    if e.etablissement_id != contexte.etablissement_id {
        return Err(ErreurBase::Cloisonnement {
            entite: "exclusion",
            id: e.id.vers_chaine(),
        });
    }
    sqlx::query(
        "INSERT INTO exclusion (id, etablissement_id, inscription_id, nature, du, au, motif, decision, statut, leve_le,
            motif_levee, echeances_annulees, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?13, ?14, ?15)",
    )
    .bind(e.id.vers_chaine())
    .bind(e.etablissement_id.vers_chaine())
    .bind(e.inscription_id.vers_chaine())
    .bind(e.nature.nom())
    .bind(e.du.to_string())
    .bind(e.au.map(|a| a.to_string()))
    .bind(&e.motif)
    .bind(&e.decision)
    .bind(e.statut.nom())
    .bind(e.leve_le.map(|l| l.to_string()))
    .bind(&e.motif_levee)
    .bind(ids_en_json(&e.echeances_annulees)?)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Récrit l'état d'une exclusion levée ou révoquée.
pub async fn ecrire_levee(cnx: &mut Connexion, e: &Exclusion, contexte: &Contexte) -> Resultat<()> {
    sqlx::query(
        "UPDATE exclusion SET statut = ?1, leve_le = ?2, motif_levee = ?3, mis_a_jour_le = ?4, mis_a_jour_par = ?5,
            identifiant_poste = ?6
         WHERE id = ?7 AND etablissement_id = ?8",
    )
    .bind(e.statut.nom())
    .bind(e.leve_le.map(|l| l.to_string()))
    .bind(&e.motif_levee)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(e.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Lit une exclusion.
pub async fn exclusion(
    cnx: &mut Connexion,
    id: Id<Exclusion>,
    contexte: &Contexte,
) -> Resultat<Exclusion> {
    sqlx::query_as::<_, LigneExclusion>(selectionner_exclusion!(
        "WHERE id = ?1 AND etablissement_id = ?2"
    ))
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .ok_or_else(|| ErreurBase::Introuvable {
        entite: "exclusion",
        id: id.vers_chaine(),
    })?
    .vers_entite()
}

/// Exclusions d'un élève, la plus ancienne d'abord.
pub async fn exclusions(
    cnx: &mut Connexion,
    inscription_id: Id<Inscription>,
    contexte: &Contexte,
) -> Resultat<Vec<Exclusion>> {
    sqlx::query_as::<_, LigneExclusion>(selectionner_exclusion!(
        "WHERE inscription_id = ?1 AND etablissement_id = ?2 ORDER BY du, cree_le"
    ))
    .bind(inscription_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneExclusion::vers_entite)
    .collect()
}

/// Inscrit le départ d'un élève : son nouvel état, la date, l'école de destination et le motif ;
/// une réintégration efface la date de départ (`None`).
pub async fn ecrire_depart(
    cnx: &mut Connexion,
    i: &Inscription,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE inscription SET statut = ?1, date_depart = ?2, etablissement_destination = ?3, motif_depart = ?4,
            mis_a_jour_le = ?5, mis_a_jour_par = ?6, identifiant_poste = ?7
         WHERE id = ?8 AND etablissement_id = ?9",
    )
    .bind(i.statut.nom())
    .bind(i.date_depart.map(|d| d.to_string()))
    .bind(&i.etablissement_destination)
    .bind(&i.motif_depart)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(i.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Enregistre le numéro du certificat de transfert d'un élève.
pub async fn inserer_certificat(
    cnx: &mut Connexion,
    inscription_id: Id<Inscription>,
    numero: &str,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "INSERT INTO certificat_transfert (inscription_id, etablissement_id, numero, delivre_le, delivre_par,
            identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )
    .bind(inscription_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(numero)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Numéro du certificat de transfert d'un élève, s'il a été délivré.
pub async fn certificat(
    cnx: &mut Connexion,
    inscription_id: Id<Inscription>,
    contexte: &Contexte,
) -> Resultat<Option<String>> {
    let ligne: Option<(String,)> = sqlx::query_as(
        "SELECT numero FROM certificat_transfert WHERE inscription_id = ?1 AND etablissement_id = ?2",
    )
    .bind(inscription_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?;
    Ok(ligne.map(|(n,)| n))
}
