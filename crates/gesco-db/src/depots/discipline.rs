//! Discipline (SPEC_VERSION_1_1 §1, étape 3) : les sanctions du dossier des élèves.

use gesco_core::commun::{DateCivile, Id};
use gesco_core::core::AnneeScolaire;
use gesco_core::pedagogie::Periode;
use gesco_core::scolarite::discipline::{NatureSanction, Sanction};
use gesco_core::scolarite::{Classe, Inscription};
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::depots::booleen;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

#[derive(FromRow)]
struct LigneSanction {
    id: String,
    etablissement_id: String,
    inscription_id: String,
    nature: String,
    date: String,
    motif: String,
    decidee_par: String,
    periode_id: Option<String>,
    saisie_par: String,
    annulee: i64,
    motif_annulation: String,
}

impl LigneSanction {
    fn vers_entite(self) -> Resultat<Sanction> {
        let t = "sanction";
        Ok(Sanction {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            inscription_id: lire_id(t, "inscription_id", &self.inscription_id)?,
            nature: NatureSanction::depuis_nom(&self.nature)
                .ok_or_else(|| ErreurBase::illisible(t, "nature", &self.nature))?,
            date: DateCivile::analyser(&self.date)
                .ok_or_else(|| ErreurBase::illisible(t, "date", &self.date))?,
            motif: self.motif,
            decidee_par: self.decidee_par,
            periode_id: self
                .periode_id
                .as_deref()
                .map(|p| lire_id(t, "periode_id", p))
                .transpose()?,
            saisie_par: lire_id(t, "saisie_par", &self.saisie_par)?,
            annulee: booleen(self.annulee),
            motif_annulation: self.motif_annulation,
        })
    }
}

macro_rules! selectionner_sanction {
    ($suite:literal) => {
        concat!(
            "SELECT s.id, s.etablissement_id, s.inscription_id, s.nature, s.date, s.motif,
                s.decidee_par, s.periode_id, s.saisie_par, s.annulee, s.motif_annulation
             FROM sanction s ",
            $suite
        )
    };
}

/// Sanctions d'un élève pour une inscription, annulées comprises, par date.
pub async fn sanctions_de(
    cnx: &mut Connexion,
    inscription_id: Id<Inscription>,
    contexte: &Contexte,
) -> Resultat<Vec<Sanction>> {
    sqlx::query_as::<_, LigneSanction>(selectionner_sanction!(
        "WHERE s.etablissement_id = ?1 AND s.inscription_id = ?2 ORDER BY s.date, s.cree_le"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(inscription_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneSanction::vers_entite)
    .collect()
}

/// Sanctions d'une année, annulées comprises, la plus récente d'abord.
pub async fn sanctions_de_l_annee(
    cnx: &mut Connexion,
    annee_id: Id<AnneeScolaire>,
    contexte: &Contexte,
) -> Resultat<Vec<Sanction>> {
    sqlx::query_as::<_, LigneSanction>(selectionner_sanction!(
        "JOIN inscription i ON i.id = s.inscription_id
         WHERE s.etablissement_id = ?1 AND i.annee_id = ?2 ORDER BY s.date DESC, s.cree_le DESC"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(annee_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneSanction::vers_entite)
    .collect()
}

/// Sanctions non annulées du conseil de classe d'une période.
pub async fn du_conseil(
    cnx: &mut Connexion,
    periode_id: Id<Periode>,
    contexte: &Contexte,
) -> Resultat<Vec<Sanction>> {
    sqlx::query_as::<_, LigneSanction>(selectionner_sanction!(
        "WHERE s.etablissement_id = ?1 AND s.periode_id = ?2 AND s.annulee = 0
         ORDER BY s.date, s.cree_le"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(periode_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneSanction::vers_entite)
    .collect()
}

/// Une sanction.
pub async fn sanction(
    cnx: &mut Connexion,
    id: Id<Sanction>,
    contexte: &Contexte,
) -> Resultat<Sanction> {
    sqlx::query_as::<_, LigneSanction>(selectionner_sanction!(
        "WHERE s.id = ?1 AND s.etablissement_id = ?2"
    ))
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .ok_or_else(|| ErreurBase::Introuvable {
        entite: "sanction",
        id: id.vers_chaine(),
    })?
    .vers_entite()
}

/// Enregistre une sanction.
pub async fn inserer(cnx: &mut Connexion, s: &Sanction, contexte: &Contexte) -> Resultat<()> {
    if s.etablissement_id != contexte.etablissement_id {
        return Err(ErreurBase::Cloisonnement {
            entite: "sanction",
            id: s.id.vers_chaine(),
        });
    }
    sqlx::query(
        "INSERT INTO sanction (id, etablissement_id, inscription_id, nature, date, motif,
            decidee_par, periode_id, saisie_par, annulee, motif_annulation, cree_le, cree_par,
            mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?12, ?13, ?14)",
    )
    .bind(s.id.vers_chaine())
    .bind(s.etablissement_id.vers_chaine())
    .bind(s.inscription_id.vers_chaine())
    .bind(s.nature.nom())
    .bind(s.date.to_string())
    .bind(&s.motif)
    .bind(&s.decidee_par)
    .bind(s.periode_id.map(|p| p.vers_chaine()))
    .bind(s.saisie_par.vers_chaine())
    .bind(i64::from(s.annulee))
    .bind(&s.motif_annulation)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Annule une sanction, avec son motif.
pub async fn annuler(
    cnx: &mut Connexion,
    id: Id<Sanction>,
    motif: &str,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE sanction SET annulee = 1, motif_annulation = ?1, mis_a_jour_le = ?2,
            mis_a_jour_par = ?3, identifiant_poste = ?4
         WHERE id = ?5 AND etablissement_id = ?6",
    )
    .bind(motif)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Sanctions non annulées des élèves d'une classe, par date.
pub async fn de_la_classe(
    cnx: &mut Connexion,
    classe_id: Id<Classe>,
    contexte: &Contexte,
) -> Resultat<Vec<Sanction>> {
    sqlx::query_as::<_, LigneSanction>(selectionner_sanction!(
        "JOIN inscription i ON i.id = s.inscription_id
         WHERE s.etablissement_id = ?1 AND i.classe_id = ?2 AND s.annulee = 0
         ORDER BY s.date, s.cree_le"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(classe_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneSanction::vers_entite)
    .collect()
}
