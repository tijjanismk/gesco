//! Note de conduite : la note saisie d'un élève pour une période, ou la correction d'une note
//! calculée. Une note remplacée ou saisie par erreur est retirée avec un motif.

use gesco_core::commun::Id;
use gesco_core::pedagogie::Periode;
use gesco_core::pedagogie::conduite::NoteConduite;
use gesco_core::scolarite::Inscription;
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::depots::booleen;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

#[derive(FromRow)]
struct LigneNoteConduite {
    id: String,
    etablissement_id: String,
    inscription_id: String,
    periode_id: String,
    note: i64,
    motif: String,
    saisie_par: String,
    annulee: i64,
    motif_annulation: String,
}

impl LigneNoteConduite {
    fn vers_entite(self) -> Resultat<NoteConduite> {
        let t = "note_conduite";
        Ok(NoteConduite {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            inscription_id: lire_id(t, "inscription_id", &self.inscription_id)?,
            periode_id: lire_id(t, "periode_id", &self.periode_id)?,
            note: i32::try_from(self.note)
                .map_err(|_| ErreurBase::illisible(t, "note", self.note.to_string()))?,
            motif: self.motif,
            saisie_par: lire_id(t, "saisie_par", &self.saisie_par)?,
            annulee: booleen(self.annulee),
            motif_annulation: self.motif_annulation,
        })
    }
}

macro_rules! selectionner_note {
    ($suite:literal) => {
        concat!(
            "SELECT id, etablissement_id, inscription_id, periode_id, note, motif, saisie_par,
                annulee, motif_annulation
             FROM note_conduite ",
            $suite
        )
    };
}

/// Notes en vigueur d'une période, tous élèves confondus.
pub async fn de_la_periode(
    cnx: &mut Connexion,
    periode_id: Id<Periode>,
    contexte: &Contexte,
) -> Resultat<Vec<NoteConduite>> {
    sqlx::query_as::<_, LigneNoteConduite>(selectionner_note!(
        "WHERE etablissement_id = ?1 AND periode_id = ?2 AND annulee = 0"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(periode_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneNoteConduite::vers_entite)
    .collect()
}

/// Note en vigueur d'un élève pour une période.
pub async fn en_vigueur(
    cnx: &mut Connexion,
    inscription_id: Id<Inscription>,
    periode_id: Id<Periode>,
    contexte: &Contexte,
) -> Resultat<Option<NoteConduite>> {
    sqlx::query_as::<_, LigneNoteConduite>(selectionner_note!(
        "WHERE etablissement_id = ?1 AND inscription_id = ?2 AND periode_id = ?3 AND annulee = 0"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(inscription_id.vers_chaine())
    .bind(periode_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .map(LigneNoteConduite::vers_entite)
    .transpose()
}

/// Enregistre une note de conduite.
pub async fn inserer(cnx: &mut Connexion, n: &NoteConduite, contexte: &Contexte) -> Resultat<()> {
    if n.etablissement_id != contexte.etablissement_id {
        return Err(ErreurBase::Cloisonnement {
            entite: "note_conduite",
            id: n.id.vers_chaine(),
        });
    }
    sqlx::query(
        "INSERT INTO note_conduite (id, etablissement_id, inscription_id, periode_id, note, motif,
            saisie_par, annulee, motif_annulation, cree_le, cree_par, mis_a_jour_le,
            mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?10, ?11, ?12)",
    )
    .bind(n.id.vers_chaine())
    .bind(n.etablissement_id.vers_chaine())
    .bind(n.inscription_id.vers_chaine())
    .bind(n.periode_id.vers_chaine())
    .bind(i64::from(n.note))
    .bind(&n.motif)
    .bind(n.saisie_par.vers_chaine())
    .bind(i64::from(n.annulee))
    .bind(&n.motif_annulation)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Retire une note de conduite, avec son motif.
pub async fn annuler(
    cnx: &mut Connexion,
    id: Id<NoteConduite>,
    motif: &str,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE note_conduite SET annulee = 1, motif_annulation = ?1, mis_a_jour_le = ?2,
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
