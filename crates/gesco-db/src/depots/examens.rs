//! Résultats aux examens, par élève et en totaux (SPEC_PHASE_9 §8.5). Rien ne se supprime : une
//! ligne se vide.

use gesco_core::commun::Id;
use gesco_core::core::{AnneeScolaire, Examen};
use gesco_core::scolarite::Inscription;
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::erreur::Resultat;
use crate::id::nouvel_id;

/// Résultat d'un élève à un examen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultatExamen {
    /// Inscription de l'élève dans la classe d'examen.
    pub inscription_id: String,
    /// Examen.
    pub examen: Examen,
    /// Inscrit à l'examen.
    pub inscrit: Option<bool>,
    /// Présent.
    pub present: Option<bool>,
    /// Admis.
    pub admis: Option<bool>,
    /// Mention ; vide s'il n'y en a pas.
    pub mention: String,
}

#[derive(FromRow)]
struct LigneResultat {
    inscription_id: String,
    examen: String,
    inscrit: Option<i64>,
    present: Option<i64>,
    admis: Option<i64>,
    mention: String,
}

fn booleen(v: Option<i64>) -> Option<bool> {
    v.map(|v| v != 0)
}

/// Résultats des inscriptions d'une année.
pub async fn resultats(
    cnx: &mut Connexion,
    annee_id: Id<AnneeScolaire>,
    contexte: &Contexte,
) -> Resultat<Vec<ResultatExamen>> {
    let lignes: Vec<LigneResultat> = sqlx::query_as(
        "SELECT r.inscription_id, r.examen, r.inscrit, r.present, r.admis, r.mention
         FROM resultat_examen r JOIN inscription i ON i.id = r.inscription_id
         WHERE i.annee_id = ?1 AND r.etablissement_id = ?2",
    )
    .bind(annee_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?;
    Ok(lignes
        .into_iter()
        .filter_map(|l| {
            Some(ResultatExamen {
                examen: Examen::depuis_nom(&l.examen)?,
                inscription_id: l.inscription_id,
                inscrit: booleen(l.inscrit),
                present: booleen(l.present),
                admis: booleen(l.admis),
                mention: l.mention,
            })
        })
        .collect())
}

/// Écrit le résultat d'une inscription : la ligne existante est récrite, sinon elle est créée.
/// Rend la ligne d'avant, pour l'audit.
pub async fn ecrire_resultat(
    cnx: &mut Connexion,
    r: &ResultatExamen,
    contexte: &Contexte,
) -> Resultat<Option<ResultatExamen>> {
    let avant: Option<LigneResultat> = sqlx::query_as(
        "SELECT inscription_id, examen, inscrit, present, admis, mention FROM resultat_examen
         WHERE inscription_id = ?1 AND etablissement_id = ?2",
    )
    .bind(&r.inscription_id)
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(&mut *cnx)
    .await?;
    let b = |v: Option<bool>| v.map(i64::from);
    if avant.is_some() {
        sqlx::query(
            "UPDATE resultat_examen SET examen = ?1, inscrit = ?2, present = ?3, admis = ?4,
                mention = ?5, mis_a_jour_le = ?6, mis_a_jour_par = ?7, identifiant_poste = ?8
             WHERE inscription_id = ?9 AND etablissement_id = ?10",
        )
        .bind(r.examen.nom())
        .bind(b(r.inscrit))
        .bind(b(r.present))
        .bind(b(r.admis))
        .bind(&r.mention)
        .bind(contexte.maintenant.millis_epoque())
        .bind(contexte.auteur())
        .bind(&contexte.poste.0)
        .bind(&r.inscription_id)
        .bind(contexte.etablissement_id.vers_chaine())
        .execute(cnx)
        .await?;
    } else {
        let id: Id<Inscription> = nouvel_id();
        sqlx::query(
            "INSERT INTO resultat_examen (id, etablissement_id, inscription_id, examen, inscrit,
                present, admis, mention, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par,
                identifiant_poste)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?9, ?10, ?11)",
        )
        .bind(id.vers_chaine())
        .bind(contexte.etablissement_id.vers_chaine())
        .bind(&r.inscription_id)
        .bind(r.examen.nom())
        .bind(b(r.inscrit))
        .bind(b(r.present))
        .bind(b(r.admis))
        .bind(&r.mention)
        .bind(contexte.maintenant.millis_epoque())
        .bind(contexte.auteur())
        .bind(&contexte.poste.0)
        .execute(cnx)
        .await?;
    }
    Ok(avant.and_then(|l| {
        Some(ResultatExamen {
            examen: Examen::depuis_nom(&l.examen)?,
            inscription_id: l.inscription_id,
            inscrit: booleen(l.inscrit),
            present: booleen(l.present),
            admis: booleen(l.admis),
            mention: l.mention,
        })
    }))
}

/// Totaux d'un examen pour un sexe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TotalExamen {
    /// Examen.
    pub examen: Examen,
    /// Sexe, nom persisté.
    pub sexe: String,
    /// Inscrits.
    pub inscrits: Option<u32>,
    /// Présentés.
    pub presentes: Option<u32>,
    /// Admis.
    pub admis: Option<u32>,
}

#[derive(FromRow)]
struct LigneTotal {
    examen: String,
    sexe: String,
    inscrits: Option<i64>,
    presentes: Option<i64>,
    admis: Option<i64>,
}

fn nombre(v: Option<i64>) -> Option<u32> {
    v.and_then(|v| u32::try_from(v).ok())
}

/// Totaux d'une année.
pub async fn totaux(
    cnx: &mut Connexion,
    annee_id: Id<AnneeScolaire>,
    contexte: &Contexte,
) -> Resultat<Vec<TotalExamen>> {
    let lignes: Vec<LigneTotal> = sqlx::query_as(
        "SELECT examen, sexe, inscrits, presentes, admis FROM total_examen
         WHERE annee_id = ?1 AND etablissement_id = ?2",
    )
    .bind(annee_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?;
    Ok(lignes
        .into_iter()
        .filter_map(|l| {
            Some(TotalExamen {
                examen: Examen::depuis_nom(&l.examen)?,
                sexe: l.sexe,
                inscrits: nombre(l.inscrits),
                presentes: nombre(l.presentes),
                admis: nombre(l.admis),
            })
        })
        .collect())
}

/// Écrit les totaux d'un examen pour un sexe. Rend les totaux d'avant, pour l'audit.
pub async fn ecrire_total(
    cnx: &mut Connexion,
    annee_id: Id<AnneeScolaire>,
    t: &TotalExamen,
    contexte: &Contexte,
) -> Resultat<Option<TotalExamen>> {
    let avant: Option<LigneTotal> = sqlx::query_as(
        "SELECT examen, sexe, inscrits, presentes, admis FROM total_examen
         WHERE annee_id = ?1 AND examen = ?2 AND sexe = ?3 AND etablissement_id = ?4",
    )
    .bind(annee_id.vers_chaine())
    .bind(t.examen.nom())
    .bind(&t.sexe)
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(&mut *cnx)
    .await?;
    let n = |v: Option<u32>| v.map(i64::from);
    if avant.is_some() {
        sqlx::query(
            "UPDATE total_examen SET inscrits = ?1, presentes = ?2, admis = ?3, mis_a_jour_le = ?4,
                mis_a_jour_par = ?5, identifiant_poste = ?6
             WHERE annee_id = ?7 AND examen = ?8 AND sexe = ?9 AND etablissement_id = ?10",
        )
        .bind(n(t.inscrits))
        .bind(n(t.presentes))
        .bind(n(t.admis))
        .bind(contexte.maintenant.millis_epoque())
        .bind(contexte.auteur())
        .bind(&contexte.poste.0)
        .bind(annee_id.vers_chaine())
        .bind(t.examen.nom())
        .bind(&t.sexe)
        .bind(contexte.etablissement_id.vers_chaine())
        .execute(cnx)
        .await?;
    } else {
        let id: Id<AnneeScolaire> = nouvel_id();
        sqlx::query(
            "INSERT INTO total_examen (id, etablissement_id, annee_id, examen, sexe, inscrits,
                presentes, admis, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par,
                identifiant_poste)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?9, ?10, ?11)",
        )
        .bind(id.vers_chaine())
        .bind(contexte.etablissement_id.vers_chaine())
        .bind(annee_id.vers_chaine())
        .bind(t.examen.nom())
        .bind(&t.sexe)
        .bind(n(t.inscrits))
        .bind(n(t.presentes))
        .bind(n(t.admis))
        .bind(contexte.maintenant.millis_epoque())
        .bind(contexte.auteur())
        .bind(&contexte.poste.0)
        .execute(cnx)
        .await?;
    }
    Ok(avant.and_then(|l| {
        Some(TotalExamen {
            examen: Examen::depuis_nom(&l.examen)?,
            sexe: l.sexe,
            inscrits: nombre(l.inscrits),
            presentes: nombre(l.presentes),
            admis: nombre(l.admis),
        })
    }))
}
