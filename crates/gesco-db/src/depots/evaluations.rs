//! Évaluations et notes (SPEC_PHASE_4 §4). Une note ne se supprime jamais (R-14).

use gesco_core::commun::{DateCivile, Id};
use gesco_core::core::MachineAEtats;
use gesco_core::pedagogie::{
    Evaluation, Matiere, NoteEleve, Periode, StatutEvaluation, TypeEvaluation,
};
use gesco_core::scolarite::{Classe, Inscription};
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::depots::booleen;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

#[derive(FromRow)]
struct LigneEvaluation {
    id: String,
    etablissement_id: String,
    annee_id: String,
    periode_id: String,
    classe_id: String,
    matiere_id: String,
    type_evaluation: String,
    libelle: String,
    date: String,
    bareme: i64,
    ponderation: i64,
    enseignant_id: Option<String>,
    statut: String,
    motif_deverrouillage: String,
}

impl LigneEvaluation {
    fn vers_entite(self) -> Resultat<Evaluation> {
        let t = "evaluation";
        Ok(Evaluation {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            annee_id: lire_id(t, "annee_id", &self.annee_id)?,
            periode_id: lire_id(t, "periode_id", &self.periode_id)?,
            classe_id: lire_id(t, "classe_id", &self.classe_id)?,
            matiere_id: lire_id(t, "matiere_id", &self.matiere_id)?,
            type_evaluation: TypeEvaluation::depuis_nom(&self.type_evaluation).ok_or_else(
                || ErreurBase::illisible(t, "type_evaluation", &self.type_evaluation),
            )?,
            date: DateCivile::analyser(&self.date)
                .ok_or_else(|| ErreurBase::illisible(t, "date", &self.date))?,
            bareme: i32::try_from(self.bareme)
                .map_err(|_| ErreurBase::illisible(t, "bareme", self.bareme.to_string()))?,
            ponderation: i32::try_from(self.ponderation).map_err(|_| {
                ErreurBase::illisible(t, "ponderation", self.ponderation.to_string())
            })?,
            enseignant_id: self
                .enseignant_id
                .as_deref()
                .map(|e| lire_id(t, "enseignant_id", e))
                .transpose()?,
            statut: StatutEvaluation::depuis_nom(&self.statut)
                .ok_or_else(|| ErreurBase::illisible(t, "statut", &self.statut))?,
            libelle: self.libelle,
            motif_deverrouillage: self.motif_deverrouillage,
        })
    }
}

macro_rules! selectionner_evaluation {
    ($suite:literal) => {
        concat!(
            "SELECT id, etablissement_id, annee_id, periode_id, classe_id, matiere_id, type_evaluation, libelle,
                date, bareme, ponderation, enseignant_id, statut, motif_deverrouillage FROM evaluation ",
            $suite
        )
    };
}

/// Enregistre une évaluation.
pub async fn inserer(cnx: &mut Connexion, e: &Evaluation, contexte: &Contexte) -> Resultat<()> {
    if e.etablissement_id != contexte.etablissement_id {
        return Err(ErreurBase::Cloisonnement {
            entite: "evaluation",
            id: e.id.vers_chaine(),
        });
    }
    sqlx::query(
        "INSERT INTO evaluation (id, etablissement_id, annee_id, periode_id, classe_id, matiere_id, type_evaluation,
            libelle, date, bareme, ponderation, enseignant_id, statut, motif_deverrouillage,
            cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?15, ?16, ?17)",
    )
    .bind(e.id.vers_chaine())
    .bind(e.etablissement_id.vers_chaine())
    .bind(e.annee_id.vers_chaine())
    .bind(e.periode_id.vers_chaine())
    .bind(e.classe_id.vers_chaine())
    .bind(e.matiere_id.vers_chaine())
    .bind(e.type_evaluation.nom())
    .bind(&e.libelle)
    .bind(e.date.to_string())
    .bind(e.bareme)
    .bind(e.ponderation)
    .bind(e.enseignant_id.map(|x| x.vers_chaine()))
    .bind(e.statut.nom())
    .bind(&e.motif_deverrouillage)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Récrit l'état et le motif de déverrouillage d'une évaluation.
pub async fn ecrire_statut(
    cnx: &mut Connexion,
    e: &Evaluation,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE evaluation SET statut = ?1, motif_deverrouillage = ?2, mis_a_jour_le = ?3, mis_a_jour_par = ?4,
            identifiant_poste = ?5
         WHERE id = ?6 AND etablissement_id = ?7",
    )
    .bind(e.statut.nom())
    .bind(&e.motif_deverrouillage)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(e.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Lit une évaluation.
pub async fn lire(
    cnx: &mut Connexion,
    id: Id<Evaluation>,
    contexte: &Contexte,
) -> Resultat<Evaluation> {
    sqlx::query_as::<_, LigneEvaluation>(selectionner_evaluation!(
        "WHERE id = ?1 AND etablissement_id = ?2"
    ))
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .ok_or_else(|| ErreurBase::Introuvable {
        entite: "evaluation",
        id: id.vers_chaine(),
    })?
    .vers_entite()
}

/// Évaluations d'une classe dans une période, par date.
pub async fn de_la_classe(
    cnx: &mut Connexion,
    classe_id: Id<Classe>,
    periode_id: Id<Periode>,
    contexte: &Contexte,
) -> Resultat<Vec<Evaluation>> {
    sqlx::query_as::<_, LigneEvaluation>(selectionner_evaluation!(
        "WHERE classe_id = ?1 AND periode_id = ?2 AND etablissement_id = ?3 ORDER BY date, cree_le"
    ))
    .bind(classe_id.vers_chaine())
    .bind(periode_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneEvaluation::vers_entite)
    .collect()
}

/// Nombre d'évaluations d'une matière d'une classe, toutes périodes : une affectation qui en a
/// reste visible.
pub async fn nombre_pour(
    cnx: &mut Connexion,
    classe_id: Id<Classe>,
    matiere_id: Id<Matiere>,
    contexte: &Contexte,
) -> Resultat<i64> {
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM evaluation WHERE classe_id = ?1 AND matiere_id = ?2 AND etablissement_id = ?3",
    )
    .bind(classe_id.vers_chaine())
    .bind(matiere_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_one(cnx)
    .await?;
    Ok(n)
}

#[derive(FromRow)]
struct LigneNote {
    evaluation_id: String,
    inscription_id: String,
    valeur: Option<i64>,
    absent: i64,
    justifie: i64,
    dispense: i64,
}

impl LigneNote {
    fn vers_entite(self) -> Resultat<NoteEleve> {
        let t = "note";
        Ok(NoteEleve {
            evaluation_id: lire_id(t, "evaluation_id", &self.evaluation_id)?,
            inscription_id: lire_id(t, "inscription_id", &self.inscription_id)?,
            valeur: self
                .valeur
                .map(|v| {
                    i32::try_from(v).map_err(|_| ErreurBase::illisible(t, "valeur", v.to_string()))
                })
                .transpose()?,
            absent: booleen(self.absent),
            justifie: booleen(self.justifie),
            dispense: booleen(self.dispense),
        })
    }
}

/// Notes en vigueur d'une évaluation.
pub async fn notes(
    cnx: &mut Connexion,
    evaluation_id: Id<Evaluation>,
    contexte: &Contexte,
) -> Resultat<Vec<NoteEleve>> {
    sqlx::query_as::<_, LigneNote>(
        "SELECT evaluation_id, inscription_id, valeur, absent, justifie, dispense FROM note
         WHERE evaluation_id = ?1 AND etablissement_id = ?2 AND retiree = 0",
    )
    .bind(evaluation_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneNote::vers_entite)
    .collect()
}

/// Écrit la note d'un élève : la crée, la remplace, ou la retire (`None`) sans la supprimer.
pub async fn ecrire_note(
    cnx: &mut Connexion,
    evaluation_id: Id<Evaluation>,
    inscription_id: Id<Inscription>,
    note: Option<&NoteEleve>,
    contexte: &Contexte,
) -> Resultat<()> {
    let (valeur, absent, justifie, dispense, retiree) = match note {
        Some(n) => (n.valeur, n.absent, n.justifie, n.dispense, false),
        None => (None, false, false, false, true),
    };
    sqlx::query(
        "INSERT INTO note (evaluation_id, inscription_id, etablissement_id, valeur, absent, justifie, dispense,
            retiree, saisie_le, saisie_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
         ON CONFLICT (evaluation_id, inscription_id) DO UPDATE SET valeur = excluded.valeur,
            absent = excluded.absent, justifie = excluded.justifie, dispense = excluded.dispense,
            retiree = excluded.retiree, saisie_le = excluded.saisie_le, saisie_par = excluded.saisie_par,
            identifiant_poste = excluded.identifiant_poste",
    )
    .bind(evaluation_id.vers_chaine())
    .bind(inscription_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(valeur)
    .bind(i64::from(absent))
    .bind(i64::from(justifie))
    .bind(i64::from(dispense))
    .bind(i64::from(retiree))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}
