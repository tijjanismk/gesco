//! Rubriques et réponses de la fiche de l'établissement (SPEC_PHASE_9 §8.1). Rien ne se
//! supprime : une rubrique se désactive (`retiree_le`) ; une réponse se vide (`valeur` nulle).

use gesco_core::commun::Id;
use gesco_core::core::fiche::SEPARATEUR;
use gesco_core::core::{AnneeScolaire, RubriqueFiche, ThemeRubrique, TypeReponse};
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::depots::{booleen, horodatage_optionnel};
use crate::erreur::{ErreurBase, Resultat};
use crate::id::{lire_id, nouvel_id};

#[derive(FromRow)]
struct LigneRubrique {
    id: String,
    etablissement_id: String,
    code: Option<String>,
    theme: String,
    libelle: String,
    aide: String,
    type_reponse: String,
    choix: String,
    annuelle: i64,
    livree: i64,
    ordre: i64,
    retiree_le: Option<i64>,
}

impl LigneRubrique {
    fn vers_entite(self) -> Resultat<RubriqueFiche> {
        let t = "rubrique_fiche";
        Ok(RubriqueFiche {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            code: self.code,
            theme: ThemeRubrique::depuis_nom(&self.theme),
            libelle: self.libelle,
            aide: self.aide,
            type_reponse: TypeReponse::depuis_nom(&self.type_reponse),
            choix: self
                .choix
                .split(SEPARATEUR)
                .filter(|c| !c.is_empty())
                .map(str::to_owned)
                .collect(),
            annuelle: booleen(self.annuelle),
            livree: booleen(self.livree),
            ordre: self.ordre,
            retiree_le: horodatage_optionnel(self.retiree_le),
        })
    }
}

fn choix(r: &RubriqueFiche) -> String {
    r.choix.join(&SEPARATEUR.to_string())
}

/// Rubriques de l'établissement, désactivées comprises, par thème puis dans leur ordre.
pub async fn rubriques(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<RubriqueFiche>> {
    sqlx::query_as::<_, LigneRubrique>(
        "SELECT id, etablissement_id, code, theme, libelle, aide, type_reponse, choix, annuelle,
                livree, ordre, retiree_le
         FROM rubrique_fiche WHERE etablissement_id = ?1 ORDER BY theme, ordre, libelle",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneRubrique::vers_entite)
    .collect()
}

/// Une rubrique de l'établissement.
pub async fn rubrique(
    cnx: &mut Connexion,
    id: Id<RubriqueFiche>,
    contexte: &Contexte,
) -> Resultat<RubriqueFiche> {
    sqlx::query_as::<_, LigneRubrique>(
        "SELECT id, etablissement_id, code, theme, libelle, aide, type_reponse, choix, annuelle,
                livree, ordre, retiree_le
         FROM rubrique_fiche WHERE id = ?1 AND etablissement_id = ?2",
    )
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .ok_or_else(|| ErreurBase::Introuvable {
        entite: "rubrique_fiche",
        id: id.vers_chaine(),
    })?
    .vers_entite()
}

/// Enregistre une rubrique.
pub async fn inserer_rubrique(
    cnx: &mut Connexion,
    r: &RubriqueFiche,
    contexte: &Contexte,
) -> Resultat<()> {
    if r.etablissement_id != contexte.etablissement_id {
        return Err(ErreurBase::Cloisonnement {
            entite: "rubrique_fiche",
            id: r.id.vers_chaine(),
        });
    }
    sqlx::query(
        "INSERT INTO rubrique_fiche (id, etablissement_id, code, theme, libelle, aide, type_reponse,
            choix, annuelle, livree, ordre, retiree_le, cree_le, cree_par, mis_a_jour_le,
            mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?13, ?14, ?15)",
    )
    .bind(r.id.vers_chaine())
    .bind(r.etablissement_id.vers_chaine())
    .bind(r.code.as_deref())
    .bind(r.theme.nom())
    .bind(&r.libelle)
    .bind(&r.aide)
    .bind(r.type_reponse.nom())
    .bind(choix(r))
    .bind(i64::from(r.annuelle))
    .bind(i64::from(r.livree))
    .bind(r.ordre)
    .bind(r.retiree_le.map(|h| h.millis_epoque()))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Récrit une rubrique : thème, libellé, aide, type, choix, portée, ordre, désactivation.
pub async fn ecrire_rubrique(
    cnx: &mut Connexion,
    r: &RubriqueFiche,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE rubrique_fiche SET theme = ?1, libelle = ?2, aide = ?3, type_reponse = ?4,
            choix = ?5, annuelle = ?6, ordre = ?7, retiree_le = ?8, mis_a_jour_le = ?9,
            mis_a_jour_par = ?10, identifiant_poste = ?11
         WHERE id = ?12 AND etablissement_id = ?13",
    )
    .bind(r.theme.nom())
    .bind(&r.libelle)
    .bind(&r.aide)
    .bind(r.type_reponse.nom())
    .bind(choix(r))
    .bind(i64::from(r.annuelle))
    .bind(r.ordre)
    .bind(r.retiree_le.map(|h| h.millis_epoque()))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(r.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Une réponse rangée : la rubrique, l'année (vide pour une rubrique permanente), la valeur
/// (vide pour une réponse vidée).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValeurFiche {
    /// Rubrique.
    pub rubrique_id: Id<RubriqueFiche>,
    /// Année d'une réponse annuelle.
    pub annee_id: Option<Id<AnneeScolaire>>,
    /// Valeur ; vide pour une réponse vidée.
    pub valeur: Option<String>,
}

#[derive(FromRow)]
struct LigneValeur {
    rubrique_id: String,
    annee_id: Option<String>,
    valeur: Option<String>,
}

/// Toutes les réponses de l'établissement, toutes années.
pub async fn valeurs(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<ValeurFiche>> {
    let t = "valeur_fiche";
    sqlx::query_as::<_, LigneValeur>(
        "SELECT rubrique_id, annee_id, valeur FROM valeur_fiche WHERE etablissement_id = ?1",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(|l| {
        Ok(ValeurFiche {
            rubrique_id: lire_id(t, "rubrique_id", &l.rubrique_id)?,
            annee_id: l
                .annee_id
                .as_deref()
                .map(|a| lire_id(t, "annee_id", a))
                .transpose()?,
            valeur: l.valeur,
        })
    })
    .collect()
}

/// Écrit une réponse : la ligne existante est récrite, sinon une ligne est créée. Rend la valeur
/// d'avant, pour l'audit.
pub async fn ecrire_valeur(
    cnx: &mut Connexion,
    v: &ValeurFiche,
    contexte: &Contexte,
) -> Resultat<Option<String>> {
    let annee = v.annee_id.map(|a| a.vers_chaine());
    let existante: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT id, valeur FROM valeur_fiche
         WHERE rubrique_id = ?1 AND ifnull(annee_id, '') = ifnull(?2, '') AND etablissement_id = ?3",
    )
    .bind(v.rubrique_id.vers_chaine())
    .bind(annee.as_deref())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(&mut *cnx)
    .await?;
    match existante {
        Some((id, avant)) => {
            sqlx::query(
                "UPDATE valeur_fiche SET valeur = ?1, mis_a_jour_le = ?2, mis_a_jour_par = ?3,
                    identifiant_poste = ?4 WHERE id = ?5",
            )
            .bind(v.valeur.as_deref())
            .bind(contexte.maintenant.millis_epoque())
            .bind(contexte.auteur())
            .bind(&contexte.poste.0)
            .bind(id)
            .execute(cnx)
            .await?;
            Ok(avant)
        }
        None => {
            let id: Id<ValeurFiche> = nouvel_id();
            sqlx::query(
                "INSERT INTO valeur_fiche (id, etablissement_id, rubrique_id, annee_id, valeur,
                    cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?6, ?7, ?8)",
            )
            .bind(id.vers_chaine())
            .bind(contexte.etablissement_id.vers_chaine())
            .bind(v.rubrique_id.vers_chaine())
            .bind(annee.as_deref())
            .bind(v.valeur.as_deref())
            .bind(contexte.maintenant.millis_epoque())
            .bind(contexte.auteur())
            .bind(&contexte.poste.0)
            .execute(cnx)
            .await?;
            Ok(None)
        }
    }
}
