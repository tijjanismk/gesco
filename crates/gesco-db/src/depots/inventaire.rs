//! Articles et relevés de l'inventaire de l'école (SPEC_PHASE_9 §8.4). Rien ne se supprime : un
//! article se retire (`retire_le`) ; une ligne de relevé se vide.

use gesco_core::commun::Id;
use gesco_core::core::{
    AnneeScolaire, ArticleInventaire, FamilleInventaire, QuantitesInventaire, ReleveInventaire,
};
use gesco_core::scolarite::Cycle;
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::depots::{booleen, horodatage_optionnel};
use crate::erreur::{ErreurBase, Resultat};
use crate::id::{lire_id, nouvel_id};

const COLONNES_ARTICLE: &str = "id, etablissement_id, code, famille, designation, matiere, \
    niveau_id, livre, ordre, retire_le";

#[derive(FromRow)]
struct LigneArticle {
    id: String,
    etablissement_id: String,
    code: Option<String>,
    famille: String,
    designation: String,
    matiere: String,
    niveau_id: Option<String>,
    livre: i64,
    ordre: i64,
    retire_le: Option<i64>,
}

impl LigneArticle {
    fn vers_entite(self) -> Resultat<ArticleInventaire> {
        let t = "article_inventaire";
        Ok(ArticleInventaire {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            code: self.code,
            famille: FamilleInventaire::depuis_nom(&self.famille),
            designation: self.designation,
            matiere: self.matiere,
            niveau_id: self
                .niveau_id
                .as_deref()
                .map(|n| lire_id(t, "niveau_id", n))
                .transpose()?,
            livre: booleen(self.livre),
            ordre: self.ordre,
            retire_le: horodatage_optionnel(self.retire_le),
        })
    }
}

/// Articles de l'établissement, retirés compris, par famille puis dans leur ordre.
pub async fn articles(
    cnx: &mut Connexion,
    contexte: &Contexte,
) -> Resultat<Vec<ArticleInventaire>> {
    sqlx::query_as::<_, LigneArticle>(sqlx::AssertSqlSafe(format!(
        "SELECT {COLONNES_ARTICLE} FROM article_inventaire WHERE etablissement_id = ?1
         ORDER BY famille, ordre, designation"
    )))
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneArticle::vers_entite)
    .collect()
}

/// Un article de l'établissement.
pub async fn article(
    cnx: &mut Connexion,
    id: Id<ArticleInventaire>,
    contexte: &Contexte,
) -> Resultat<ArticleInventaire> {
    sqlx::query_as::<_, LigneArticle>(sqlx::AssertSqlSafe(format!(
        "SELECT {COLONNES_ARTICLE} FROM article_inventaire
         WHERE id = ?1 AND etablissement_id = ?2"
    )))
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .ok_or_else(|| ErreurBase::Introuvable {
        entite: "article_inventaire",
        id: id.vers_chaine(),
    })?
    .vers_entite()
}

/// Enregistre un article nouveau.
pub async fn inserer_article(
    cnx: &mut Connexion,
    a: &ArticleInventaire,
    contexte: &Contexte,
) -> Resultat<()> {
    if a.etablissement_id != contexte.etablissement_id {
        return Err(ErreurBase::Cloisonnement {
            entite: "article_inventaire",
            id: a.id.vers_chaine(),
        });
    }
    sqlx::query(
        "INSERT INTO article_inventaire (id, etablissement_id, code, famille, designation,
            matiere, niveau_id, livre, ordre, retire_le, cree_le, cree_par, mis_a_jour_le,
            mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?11, ?12, ?13)",
    )
    .bind(a.id.vers_chaine())
    .bind(a.etablissement_id.vers_chaine())
    .bind(a.code.as_deref())
    .bind(a.famille.nom())
    .bind(&a.designation)
    .bind(&a.matiere)
    .bind(a.niveau_id.map(|n| n.vers_chaine()))
    .bind(i64::from(a.livre))
    .bind(a.ordre)
    .bind(a.retire_le.map(|h| h.millis_epoque()))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Récrit un article : désignation, matière, niveau, ordre, retrait.
pub async fn ecrire_article(
    cnx: &mut Connexion,
    a: &ArticleInventaire,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE article_inventaire SET designation = ?1, matiere = ?2, niveau_id = ?3, ordre = ?4,
            retire_le = ?5, mis_a_jour_le = ?6, mis_a_jour_par = ?7, identifiant_poste = ?8
         WHERE id = ?9 AND etablissement_id = ?10",
    )
    .bind(&a.designation)
    .bind(&a.matiere)
    .bind(a.niveau_id.map(|n| n.vers_chaine()))
    .bind(a.ordre)
    .bind(a.retire_le.map(|h| h.millis_epoque()))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(a.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

const COLONNES_QUANTITES: &str = "existant, bon_etat, besoins, dur, semi_dur, banco, \
    autre_construction, neufs, en_location, murs_bons, murs_mauvais, toits_bons, toits_mauvais, \
    cabines_filles, cabines_garcons, cabines_enseignants";

#[derive(FromRow)]
struct LigneReleve {
    article_id: String,
    cycle: Option<String>,
    existant: Option<i64>,
    bon_etat: Option<i64>,
    besoins: Option<i64>,
    dur: Option<i64>,
    semi_dur: Option<i64>,
    banco: Option<i64>,
    autre_construction: Option<i64>,
    neufs: Option<i64>,
    en_location: Option<i64>,
    murs_bons: Option<i64>,
    murs_mauvais: Option<i64>,
    toits_bons: Option<i64>,
    toits_mauvais: Option<i64>,
    cabines_filles: Option<i64>,
    cabines_garcons: Option<i64>,
    cabines_enseignants: Option<i64>,
    observation: String,
}

fn quantite(q: Option<i64>) -> Option<u32> {
    q.and_then(|q| u32::try_from(q).ok())
}

impl LigneReleve {
    fn vers_entite(self) -> Resultat<ReleveInventaire> {
        Ok(ReleveInventaire {
            article_id: lire_id("releve_inventaire", "article_id", &self.article_id)?,
            cycle: self.cycle.as_deref().and_then(Cycle::depuis_nom),
            quantites: QuantitesInventaire::depuis_valeurs([
                quantite(self.existant),
                quantite(self.bon_etat),
                quantite(self.besoins),
                quantite(self.dur),
                quantite(self.semi_dur),
                quantite(self.banco),
                quantite(self.autre_construction),
                quantite(self.neufs),
                quantite(self.en_location),
                quantite(self.murs_bons),
                quantite(self.murs_mauvais),
                quantite(self.toits_bons),
                quantite(self.toits_mauvais),
                quantite(self.cabines_filles),
                quantite(self.cabines_garcons),
                quantite(self.cabines_enseignants),
            ]),
            observation: self.observation,
        })
    }
}

/// Lignes de relevé d'une année, tous cycles.
pub async fn releves(
    cnx: &mut Connexion,
    annee_id: Id<AnneeScolaire>,
    contexte: &Contexte,
) -> Resultat<Vec<ReleveInventaire>> {
    sqlx::query_as::<_, LigneReleve>(sqlx::AssertSqlSafe(format!(
        "SELECT article_id, cycle, {COLONNES_QUANTITES}, observation FROM releve_inventaire
         WHERE annee_id = ?1 AND etablissement_id = ?2"
    )))
    .bind(annee_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneReleve::vers_entite)
    .collect()
}

/// Écrit une ligne de relevé : la ligne existante (même année, même article, même cycle) est
/// récrite, sinon une ligne est créée. Rend la ligne d'avant, pour l'audit.
pub async fn ecrire_releve(
    cnx: &mut Connexion,
    annee_id: Id<AnneeScolaire>,
    r: &ReleveInventaire,
    contexte: &Contexte,
) -> Resultat<Option<ReleveInventaire>> {
    let cycle = r.cycle.map(|c| c.nom());
    let avant = sqlx::query_as::<_, LigneReleve>(sqlx::AssertSqlSafe(format!(
        "SELECT article_id, cycle, {COLONNES_QUANTITES}, observation FROM releve_inventaire
         WHERE annee_id = ?1 AND article_id = ?2 AND ifnull(cycle, '') = ifnull(?3, '')
           AND etablissement_id = ?4"
    )))
    .bind(annee_id.vers_chaine())
    .bind(r.article_id.vers_chaine())
    .bind(cycle)
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(&mut *cnx)
    .await?
    .map(LigneReleve::vers_entite)
    .transpose()?;
    let q = r.quantites.valeurs().map(|v| v.map(i64::from));
    if avant.is_some() {
        sqlx::query(
            "UPDATE releve_inventaire SET existant = ?1, bon_etat = ?2, besoins = ?3, dur = ?4,
                semi_dur = ?5, banco = ?6, autre_construction = ?7, neufs = ?8, en_location = ?9,
                murs_bons = ?10, murs_mauvais = ?11, toits_bons = ?12, toits_mauvais = ?13,
                cabines_filles = ?14, cabines_garcons = ?15, cabines_enseignants = ?16,
                observation = ?17, mis_a_jour_le = ?18, mis_a_jour_par = ?19,
                identifiant_poste = ?20
             WHERE annee_id = ?21 AND article_id = ?22 AND ifnull(cycle, '') = ifnull(?23, '')
               AND etablissement_id = ?24",
        )
        .bind(q[0])
        .bind(q[1])
        .bind(q[2])
        .bind(q[3])
        .bind(q[4])
        .bind(q[5])
        .bind(q[6])
        .bind(q[7])
        .bind(q[8])
        .bind(q[9])
        .bind(q[10])
        .bind(q[11])
        .bind(q[12])
        .bind(q[13])
        .bind(q[14])
        .bind(q[15])
        .bind(&r.observation)
        .bind(contexte.maintenant.millis_epoque())
        .bind(contexte.auteur())
        .bind(&contexte.poste.0)
        .bind(annee_id.vers_chaine())
        .bind(r.article_id.vers_chaine())
        .bind(cycle)
        .bind(contexte.etablissement_id.vers_chaine())
        .execute(cnx)
        .await?;
    } else {
        let id: Id<ReleveInventaire> = nouvel_id();
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "INSERT INTO releve_inventaire (id, etablissement_id, annee_id, article_id, cycle,
                {COLONNES_QUANTITES}, observation, cree_le, cree_par, mis_a_jour_le,
                mis_a_jour_par, identifiant_poste)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17,
                ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?23, ?24, ?25)"
        )))
        .bind(id.vers_chaine())
        .bind(contexte.etablissement_id.vers_chaine())
        .bind(annee_id.vers_chaine())
        .bind(r.article_id.vers_chaine())
        .bind(cycle)
        .bind(q[0])
        .bind(q[1])
        .bind(q[2])
        .bind(q[3])
        .bind(q[4])
        .bind(q[5])
        .bind(q[6])
        .bind(q[7])
        .bind(q[8])
        .bind(q[9])
        .bind(q[10])
        .bind(q[11])
        .bind(q[12])
        .bind(q[13])
        .bind(q[14])
        .bind(q[15])
        .bind(&r.observation)
        .bind(contexte.maintenant.millis_epoque())
        .bind(contexte.auteur())
        .bind(&contexte.poste.0)
        .execute(cnx)
        .await?;
    }
    Ok(avant)
}
