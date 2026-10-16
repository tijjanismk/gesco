//! Absences et retards (SPEC_PHASE_4 §6). Une absence ne se supprime jamais (R-14).

use gesco_core::commun::{DateCivile, Horodatage, Id};
use gesco_core::pedagogie::{Absence, DemiJournee, NatureAbsence};
use gesco_core::scolarite::{Classe, Inscription};
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::depots::booleen;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

#[derive(FromRow)]
struct LigneAbsence {
    inscription_id: String,
    classe_id: String,
    date: String,
    demi_journee: String,
    nature: String,
    minutes: Option<i64>,
    justifiee: i64,
    motif: String,
    justificatif: String,
}

impl LigneAbsence {
    fn vers_entite(self) -> Resultat<Absence> {
        let t = "absence";
        Ok(Absence {
            inscription_id: lire_id(t, "inscription_id", &self.inscription_id)?,
            classe_id: lire_id(t, "classe_id", &self.classe_id)?,
            date: DateCivile::analyser(&self.date)
                .ok_or_else(|| ErreurBase::illisible(t, "date", &self.date))?,
            demi_journee: DemiJournee::depuis_nom(&self.demi_journee)
                .ok_or_else(|| ErreurBase::illisible(t, "demi_journee", &self.demi_journee))?,
            nature: NatureAbsence::depuis_nom(&self.nature)
                .ok_or_else(|| ErreurBase::illisible(t, "nature", &self.nature))?,
            minutes: self
                .minutes
                .map(|m| {
                    i32::try_from(m).map_err(|_| ErreurBase::illisible(t, "minutes", m.to_string()))
                })
                .transpose()?,
            justifiee: booleen(self.justifiee),
            motif: self.motif,
            justificatif: self.justificatif,
        })
    }
}

macro_rules! selectionner_absence {
    ($suite:literal) => {
        concat!(
            "SELECT inscription_id, classe_id, date, demi_journee, nature, minutes, justifiee, motif, justificatif
             FROM absence ",
            $suite
        )
    };
}

/// Absences et retards en vigueur d'une classe entre deux dates incluses, par date.
pub async fn de_la_classe(
    cnx: &mut Connexion,
    classe_id: Id<Classe>,
    debut: DateCivile,
    fin: DateCivile,
    contexte: &Contexte,
) -> Resultat<Vec<Absence>> {
    sqlx::query_as::<_, LigneAbsence>(selectionner_absence!(
        "WHERE classe_id = ?1 AND date BETWEEN ?2 AND ?3 AND etablissement_id = ?4 AND retiree = 0
         ORDER BY date, demi_journee DESC"
    ))
    .bind(classe_id.vers_chaine())
    .bind(debut.to_string())
    .bind(fin.to_string())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneAbsence::vers_entite)
    .collect()
}

/// Absence en vigueur d'un élève sur une demi-journée.
pub async fn lire(
    cnx: &mut Connexion,
    inscription_id: Id<Inscription>,
    date: DateCivile,
    demi_journee: DemiJournee,
    contexte: &Contexte,
) -> Resultat<Option<Absence>> {
    sqlx::query_as::<_, LigneAbsence>(selectionner_absence!(
        "WHERE inscription_id = ?1 AND date = ?2 AND demi_journee = ?3 AND etablissement_id = ?4
            AND retiree = 0"
    ))
    .bind(inscription_id.vers_chaine())
    .bind(date.to_string())
    .bind(demi_journee.nom())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .map(LigneAbsence::vers_entite)
    .transpose()
}

/// Écrit l'absence d'un élève sur une demi-journée : la crée, la remplace, ou la retire (`None`)
/// sans la supprimer.
pub async fn ecrire(
    cnx: &mut Connexion,
    inscription_id: Id<Inscription>,
    classe_id: Id<Classe>,
    date: DateCivile,
    demi_journee: DemiJournee,
    absence: Option<&Absence>,
    contexte: &Contexte,
) -> Resultat<()> {
    let (nature, minutes, justifiee, motif, justificatif, retiree) = match absence {
        Some(a) => (
            a.nature,
            a.minutes,
            a.justifiee,
            a.motif.as_str(),
            a.justificatif.as_str(),
            false,
        ),
        None => (NatureAbsence::Absence, None, false, "", "", true),
    };
    sqlx::query(
        "INSERT INTO absence (inscription_id, date, demi_journee, etablissement_id, classe_id, nature, minutes,
            justifiee, motif, justificatif, retiree, saisie_le, saisie_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
         ON CONFLICT (inscription_id, date, demi_journee) DO UPDATE SET classe_id = excluded.classe_id,
            nature = excluded.nature, minutes = excluded.minutes, justifiee = excluded.justifiee,
            motif = excluded.motif, justificatif = excluded.justificatif, retiree = excluded.retiree,
            saisie_le = excluded.saisie_le, saisie_par = excluded.saisie_par,
            identifiant_poste = excluded.identifiant_poste",
    )
    .bind(inscription_id.vers_chaine())
    .bind(date.to_string())
    .bind(demi_journee.nom())
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(classe_id.vers_chaine())
    .bind(nature.nom())
    .bind(minutes)
    .bind(i64::from(justifiee))
    .bind(motif)
    .bind(justificatif)
    .bind(i64::from(retiree))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Justifie une absence ou un retard en vigueur.
pub async fn justifier(
    cnx: &mut Connexion,
    inscription_id: Id<Inscription>,
    date: DateCivile,
    demi_journee: DemiJournee,
    motif: &str,
    justificatif: &str,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE absence SET justifiee = 1, motif = ?1, justificatif = ?2, justifiee_le = ?3, justifiee_par = ?4,
            identifiant_poste = ?5
         WHERE inscription_id = ?6 AND date = ?7 AND demi_journee = ?8 AND etablissement_id = ?9
            AND retiree = 0",
    )
    .bind(motif)
    .bind(justificatif)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(inscription_id.vers_chaine())
    .bind(date.to_string())
    .bind(demi_journee.nom())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Note que le relevé d'une classe est fait pour ce jour, par l'auteur du contexte.
pub async fn noter_releve(
    cnx: &mut Connexion,
    classe_id: Id<Classe>,
    date: DateCivile,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "INSERT INTO releve_absences (classe_id, date, etablissement_id, fait_le, fait_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT (classe_id, date) DO UPDATE SET fait_le = excluded.fait_le,
            fait_par = excluded.fait_par, identifiant_poste = excluded.identifiant_poste",
    )
    .bind(classe_id.vers_chaine())
    .bind(date.to_string())
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Relevé fait d'une classe pour un jour : quand, et par qui (nom affiché du compte).
pub async fn releve(
    cnx: &mut Connexion,
    classe_id: Id<Classe>,
    date: DateCivile,
    contexte: &Contexte,
) -> Resultat<Option<(Horodatage, String)>> {
    let ligne: Option<(i64, Option<String>)> = sqlx::query_as(
        "SELECT r.fait_le, u.nom_affiche FROM releve_absences r
         LEFT JOIN utilisateur u ON u.id = r.fait_par
         WHERE r.classe_id = ?1 AND r.date = ?2 AND r.etablissement_id = ?3",
    )
    .bind(classe_id.vers_chaine())
    .bind(date.to_string())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?;
    Ok(ligne.map(|(le, par)| {
        (
            Horodatage::depuis_millis_epoque(le),
            par.unwrap_or_default(),
        )
    }))
}

/// Nombre de jours relevés pour une classe entre deux dates incluses.
pub async fn jours_releves(
    cnx: &mut Connexion,
    classe_id: Id<Classe>,
    debut: DateCivile,
    fin: DateCivile,
    contexte: &Contexte,
) -> Resultat<u32> {
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM releve_absences
         WHERE classe_id = ?1 AND date BETWEEN ?2 AND ?3 AND etablissement_id = ?4",
    )
    .bind(classe_id.vers_chaine())
    .bind(debut.to_string())
    .bind(fin.to_string())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_one(cnx)
    .await?;
    Ok(u32::try_from(n).unwrap_or(0))
}
