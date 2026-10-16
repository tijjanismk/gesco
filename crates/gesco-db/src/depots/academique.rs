//! Niveaux, séries et classes (SPEC_PHASE_2 §2, §3).

use gesco_core::commun::Id;
use gesco_core::core::{AnneeScolaire, Etablissement};
use gesco_core::scolarite::{Classe, Cycle, DiplomeTerminal, ModeEvaluation, Niveau, Serie};
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::depots::booleen;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

fn verifier_etablissement(
    entite: &'static str,
    proprietaire: Id<Etablissement>,
    id: String,
    contexte: &Contexte,
) -> Resultat<()> {
    if proprietaire != contexte.etablissement_id {
        return Err(ErreurBase::Cloisonnement { entite, id });
    }
    Ok(())
}

fn introuvable(entite: &'static str, id: String) -> ErreurBase {
    ErreurBase::Introuvable { entite, id }
}

// --- Niveaux -------------------------------------------------------------------------------

#[derive(FromRow)]
struct LigneNiveau {
    id: String,
    etablissement_id: String,
    code: String,
    libelle: String,
    cycle: String,
    ordre: i64,
    mode_evaluation: String,
    diplome_terminal: String,
    actif: i64,
}

impl LigneNiveau {
    fn vers_entite(self) -> Resultat<Niveau> {
        Ok(Niveau {
            id: lire_id("niveau", "id", &self.id)?,
            etablissement_id: lire_id("niveau", "etablissement_id", &self.etablissement_id)?,
            cycle: Cycle::depuis_nom(&self.cycle)
                .ok_or_else(|| ErreurBase::illisible("niveau", "cycle", &self.cycle))?,
            mode_evaluation: ModeEvaluation::depuis_nom(&self.mode_evaluation).ok_or_else(
                || ErreurBase::illisible("niveau", "mode_evaluation", &self.mode_evaluation),
            )?,
            diplome_terminal: DiplomeTerminal::depuis_nom(&self.diplome_terminal).ok_or_else(
                || ErreurBase::illisible("niveau", "diplome_terminal", &self.diplome_terminal),
            )?,
            code: self.code,
            libelle: self.libelle,
            ordre: self.ordre,
            actif: booleen(self.actif),
        })
    }
}

macro_rules! selectionner_niveau {
    ($suite:literal) => {
        concat!(
            "SELECT id, etablissement_id, code, libelle, cycle, ordre, mode_evaluation, diplome_terminal, actif FROM niveau ",
            $suite
        )
    };
}

/// Crée un niveau dans l'établissement du contexte.
pub async fn inserer_niveau(
    cnx: &mut Connexion,
    niveau: &Niveau,
    contexte: &Contexte,
) -> Resultat<()> {
    verifier_etablissement(
        "niveau",
        niveau.etablissement_id,
        niveau.id.vers_chaine(),
        contexte,
    )?;
    sqlx::query(
        "INSERT INTO niveau (id, etablissement_id, code, libelle, cycle, ordre, mode_evaluation, diplome_terminal, actif,
            cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?10, ?11, ?12)",
    )
    .bind(niveau.id.vers_chaine())
    .bind(niveau.etablissement_id.vers_chaine())
    .bind(&niveau.code)
    .bind(&niveau.libelle)
    .bind(niveau.cycle.nom())
    .bind(niveau.ordre)
    .bind(niveau.mode_evaluation.nom())
    .bind(niveau.diplome_terminal.nom())
    .bind(i64::from(niveau.actif))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Niveaux de l'établissement, dans l'ordre d'affichage.
pub async fn niveaux(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<Niveau>> {
    sqlx::query_as::<_, LigneNiveau>(selectionner_niveau!(
        "WHERE etablissement_id = ?1 AND supprime_le IS NULL ORDER BY ordre"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneNiveau::vers_entite)
    .collect()
}

/// Lit un niveau de l'établissement du contexte.
pub async fn niveau(cnx: &mut Connexion, id: Id<Niveau>, contexte: &Contexte) -> Resultat<Niveau> {
    sqlx::query_as::<_, LigneNiveau>(selectionner_niveau!(
        "WHERE id = ?1 AND etablissement_id = ?2 AND supprime_le IS NULL"
    ))
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .ok_or_else(|| introuvable("niveau", id.vers_chaine()))?
    .vers_entite()
}

/// Récrit le libellé et l'état d'un niveau ; code, cycle et ordre sont nationaux.
pub async fn modifier_niveau(
    cnx: &mut Connexion,
    niveau: &Niveau,
    contexte: &Contexte,
) -> Resultat<()> {
    verifier_etablissement(
        "niveau",
        niveau.etablissement_id,
        niveau.id.vers_chaine(),
        contexte,
    )?;
    sqlx::query(
        "UPDATE niveau SET libelle = ?1, actif = ?2, mis_a_jour_le = ?3, mis_a_jour_par = ?4, identifiant_poste = ?5
         WHERE id = ?6 AND etablissement_id = ?7 AND supprime_le IS NULL",
    )
    .bind(&niveau.libelle)
    .bind(i64::from(niveau.actif))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(niveau.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

// --- Séries --------------------------------------------------------------------------------

#[derive(FromRow)]
struct LigneSerie {
    id: String,
    etablissement_id: String,
    niveau_id: String,
    code: String,
    libelle: String,
    actif: i64,
}

impl LigneSerie {
    fn vers_entite(self) -> Resultat<Serie> {
        Ok(Serie {
            id: lire_id("serie", "id", &self.id)?,
            etablissement_id: lire_id("serie", "etablissement_id", &self.etablissement_id)?,
            niveau_id: lire_id("serie", "niveau_id", &self.niveau_id)?,
            code: self.code,
            libelle: self.libelle,
            actif: booleen(self.actif),
        })
    }
}

macro_rules! selectionner_serie {
    ($suite:literal) => {
        concat!(
            "SELECT id, etablissement_id, niveau_id, code, libelle, actif FROM serie ",
            $suite
        )
    };
}

/// Crée une série dans l'établissement du contexte.
pub async fn inserer_serie(
    cnx: &mut Connexion,
    serie: &Serie,
    contexte: &Contexte,
) -> Resultat<()> {
    verifier_etablissement(
        "serie",
        serie.etablissement_id,
        serie.id.vers_chaine(),
        contexte,
    )?;
    sqlx::query(
        "INSERT INTO serie (id, etablissement_id, niveau_id, code, libelle, actif,
            cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?7, ?8, ?9)",
    )
    .bind(serie.id.vers_chaine())
    .bind(serie.etablissement_id.vers_chaine())
    .bind(serie.niveau_id.vers_chaine())
    .bind(&serie.code)
    .bind(&serie.libelle)
    .bind(i64::from(serie.actif))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Séries de l'établissement.
pub async fn series(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<Serie>> {
    sqlx::query_as::<_, LigneSerie>(selectionner_serie!(
        "WHERE etablissement_id = ?1 AND supprime_le IS NULL ORDER BY rowid"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneSerie::vers_entite)
    .collect()
}

/// Lit une série de l'établissement du contexte.
pub async fn serie(cnx: &mut Connexion, id: Id<Serie>, contexte: &Contexte) -> Resultat<Serie> {
    sqlx::query_as::<_, LigneSerie>(selectionner_serie!(
        "WHERE id = ?1 AND etablissement_id = ?2 AND supprime_le IS NULL"
    ))
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .ok_or_else(|| introuvable("serie", id.vers_chaine()))?
    .vers_entite()
}

/// Récrit le libellé et l'état d'une série.
pub async fn modifier_serie(
    cnx: &mut Connexion,
    serie: &Serie,
    contexte: &Contexte,
) -> Resultat<()> {
    verifier_etablissement(
        "serie",
        serie.etablissement_id,
        serie.id.vers_chaine(),
        contexte,
    )?;
    sqlx::query(
        "UPDATE serie SET libelle = ?1, actif = ?2, mis_a_jour_le = ?3, mis_a_jour_par = ?4, identifiant_poste = ?5
         WHERE id = ?6 AND etablissement_id = ?7 AND supprime_le IS NULL",
    )
    .bind(&serie.libelle)
    .bind(i64::from(serie.actif))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(serie.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

// --- Classes -------------------------------------------------------------------------------

#[derive(FromRow)]
struct LigneClasse {
    id: String,
    etablissement_id: String,
    annee_id: String,
    niveau_id: String,
    serie_id: Option<String>,
    libelle: String,
}

impl LigneClasse {
    fn vers_entite(self) -> Resultat<Classe> {
        Ok(Classe {
            id: lire_id("classe", "id", &self.id)?,
            etablissement_id: lire_id("classe", "etablissement_id", &self.etablissement_id)?,
            annee_id: lire_id("classe", "annee_id", &self.annee_id)?,
            niveau_id: lire_id("classe", "niveau_id", &self.niveau_id)?,
            serie_id: self
                .serie_id
                .as_deref()
                .map(|s| lire_id("classe", "serie_id", s))
                .transpose()?,
            libelle: self.libelle,
        })
    }
}

macro_rules! selectionner_classe {
    ($suite:literal) => {
        concat!(
            "SELECT id, etablissement_id, annee_id, niveau_id, serie_id, libelle FROM classe ",
            $suite
        )
    };
}

/// Crée une classe dans l'établissement du contexte.
pub async fn inserer_classe(
    cnx: &mut Connexion,
    classe: &Classe,
    contexte: &Contexte,
) -> Resultat<()> {
    verifier_etablissement(
        "classe",
        classe.etablissement_id,
        classe.id.vers_chaine(),
        contexte,
    )?;
    sqlx::query(
        "INSERT INTO classe (id, etablissement_id, annee_id, niveau_id, serie_id, libelle,
            cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?7, ?8, ?9)",
    )
    .bind(classe.id.vers_chaine())
    .bind(classe.etablissement_id.vers_chaine())
    .bind(classe.annee_id.vers_chaine())
    .bind(classe.niveau_id.vers_chaine())
    .bind(classe.serie_id.map(|s| s.vers_chaine()))
    .bind(&classe.libelle)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Classes d'une année, par libellé.
pub async fn classes(
    cnx: &mut Connexion,
    annee_id: Id<AnneeScolaire>,
    contexte: &Contexte,
) -> Resultat<Vec<Classe>> {
    sqlx::query_as::<_, LigneClasse>(selectionner_classe!(
        "WHERE etablissement_id = ?1 AND annee_id = ?2 AND supprime_le IS NULL ORDER BY libelle"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(annee_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneClasse::vers_entite)
    .collect()
}

/// Lit une classe de l'établissement du contexte.
pub async fn classe(cnx: &mut Connexion, id: Id<Classe>, contexte: &Contexte) -> Resultat<Classe> {
    sqlx::query_as::<_, LigneClasse>(selectionner_classe!(
        "WHERE id = ?1 AND etablissement_id = ?2 AND supprime_le IS NULL"
    ))
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .ok_or_else(|| introuvable("classe", id.vers_chaine()))?
    .vers_entite()
}

/// Vrai si une autre classe non supprimée de l'année porte ce libellé.
pub async fn libelle_pris(
    cnx: &mut Connexion,
    annee_id: Id<AnneeScolaire>,
    libelle: &str,
    sauf: Option<Id<Classe>>,
    contexte: &Contexte,
) -> Resultat<bool> {
    let trouve: Option<String> = sqlx::query_scalar(
        "SELECT id FROM classe WHERE etablissement_id = ?1 AND annee_id = ?2 AND libelle = ?3
           AND supprime_le IS NULL AND (?4 IS NULL OR id <> ?4) LIMIT 1",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(annee_id.vers_chaine())
    .bind(libelle)
    .bind(sauf.map(|s| s.vers_chaine()))
    .fetch_optional(cnx)
    .await?;
    Ok(trouve.is_some())
}

/// Récrit le libellé d'une classe.
pub async fn renommer_classe(
    cnx: &mut Connexion,
    id: Id<Classe>,
    libelle: &str,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE classe SET libelle = ?1, mis_a_jour_le = ?2, mis_a_jour_par = ?3, identifiant_poste = ?4
         WHERE id = ?5 AND etablissement_id = ?6 AND supprime_le IS NULL",
    )
    .bind(libelle)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Supprime logiquement une classe.
pub async fn supprimer_classe(
    cnx: &mut Connexion,
    id: Id<Classe>,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE classe SET supprime_le = ?1, supprime_par = ?2, mis_a_jour_le = ?1, mis_a_jour_par = ?2, identifiant_poste = ?3
         WHERE id = ?4 AND etablissement_id = ?5 AND supprime_le IS NULL",
    )
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}
