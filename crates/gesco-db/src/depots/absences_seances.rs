//! Absences par séance (SPEC_VERSION_1_1 §1, étape 6, T-71).

use gesco_core::commun::{DateCivile, Id};
use gesco_core::pedagogie::emploi_du_temps::Seance;
use gesco_core::scolarite::Inscription;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::erreur::Resultat;
use crate::id::lire_id;

/// Séances d'un jour où un élève est noté absent.
pub async fn absents_du_jour(
    cnx: &mut Connexion,
    date: DateCivile,
    contexte: &Contexte,
) -> Resultat<Vec<(Id<Inscription>, Id<Seance>)>> {
    let lignes: Vec<(String, String)> = sqlx::query_as(
        "SELECT inscription_id, seance_id FROM absence_seance
         WHERE etablissement_id = ?1 AND date = ?2 AND absent = 1",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(date.to_string())
    .fetch_all(cnx)
    .await?;
    lignes
        .into_iter()
        .map(|(i, s)| {
            Ok((
                lire_id("absence_seance", "inscription_id", &i)?,
                lire_id("absence_seance", "seance_id", &s)?,
            ))
        })
        .collect()
}

/// Coche ou décoche l'absence d'un élève à une séance d'un jour.
pub async fn marquer(
    cnx: &mut Connexion,
    id_si_nouveau: &str,
    inscription_id: Id<Inscription>,
    date: DateCivile,
    seance_id: Id<Seance>,
    absent: bool,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "INSERT INTO absence_seance (id, etablissement_id, inscription_id, date, seance_id, absent,
            cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?7, ?8, ?9)
         ON CONFLICT (inscription_id, date, seance_id) DO UPDATE SET absent = excluded.absent,
            mis_a_jour_le = excluded.mis_a_jour_le, mis_a_jour_par = excluded.mis_a_jour_par,
            identifiant_poste = excluded.identifiant_poste",
    )
    .bind(id_si_nouveau)
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(inscription_id.vers_chaine())
    .bind(date.to_string())
    .bind(seance_id.vers_chaine())
    .bind(i64::from(absent))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}
