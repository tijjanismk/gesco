//! Référentiel pédagogique (SPEC_PHASE_4 §2) : matières, programmes, périodes, règles
//! d'évaluation versionnées.

use gesco_core::commun::{DateCivile, Horodatage, Id};
use gesco_core::core::AnneeScolaire;
use gesco_core::pedagogie::{
    ContenuRegle, GroupeMatiere, LigneProgramme, Matiere, Periode, RegleEvaluation, TypePeriode,
};
use gesco_core::scolarite::{Cycle, Niveau, Serie};
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::depots::booleen;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

const JOUR: i64 = 86_400_000;

fn cloisonner(entite: &'static str, id: String, contexte: &Contexte, de: String) -> Resultat<()> {
    if de != contexte.etablissement_id.vers_chaine() {
        return Err(ErreurBase::Cloisonnement { entite, id });
    }
    Ok(())
}

// --- Matières -------------------------------------------------------------------------------

#[derive(FromRow)]
struct LigneMatiere {
    id: String,
    etablissement_id: String,
    code: String,
    libelle: String,
    libelle_court: String,
    groupe: String,
    ordre: i64,
    active: i64,
}

impl LigneMatiere {
    fn vers_entite(self) -> Resultat<Matiere> {
        let t = "matiere";
        Ok(Matiere {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            groupe: GroupeMatiere::depuis_nom(&self.groupe)
                .ok_or_else(|| ErreurBase::illisible(t, "groupe", &self.groupe))?,
            code: self.code,
            libelle: self.libelle,
            libelle_court: self.libelle_court,
            ordre: self.ordre,
            active: booleen(self.active),
        })
    }
}

/// Enregistre une matière.
pub async fn inserer_matiere(
    cnx: &mut Connexion,
    m: &Matiere,
    contexte: &Contexte,
) -> Resultat<()> {
    cloisonner(
        "matiere",
        m.id.vers_chaine(),
        contexte,
        m.etablissement_id.vers_chaine(),
    )?;
    sqlx::query(
        "INSERT INTO matiere (id, etablissement_id, code, libelle, libelle_court, groupe, ordre, active,
            cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?9, ?10, ?11)",
    )
    .bind(m.id.vers_chaine())
    .bind(m.etablissement_id.vers_chaine())
    .bind(&m.code)
    .bind(&m.libelle)
    .bind(&m.libelle_court)
    .bind(m.groupe.nom())
    .bind(m.ordre)
    .bind(i64::from(m.active))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Récrit une matière.
pub async fn ecrire_matiere(cnx: &mut Connexion, m: &Matiere, contexte: &Contexte) -> Resultat<()> {
    sqlx::query(
        "UPDATE matiere SET code = ?1, libelle = ?2, libelle_court = ?3, groupe = ?4, ordre = ?5, active = ?6,
            mis_a_jour_le = ?7, mis_a_jour_par = ?8, identifiant_poste = ?9
         WHERE id = ?10 AND etablissement_id = ?11",
    )
    .bind(&m.code)
    .bind(&m.libelle)
    .bind(&m.libelle_court)
    .bind(m.groupe.nom())
    .bind(m.ordre)
    .bind(i64::from(m.active))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(m.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Matières de l'établissement, dans l'ordre d'affichage.
pub async fn matieres(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<Matiere>> {
    sqlx::query_as::<_, LigneMatiere>(
        "SELECT id, etablissement_id, code, libelle, libelle_court, groupe, ordre, active FROM matiere
         WHERE etablissement_id = ?1 ORDER BY ordre, code",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneMatiere::vers_entite)
    .collect()
}

// --- Programme ------------------------------------------------------------------------------

#[derive(FromRow)]
struct LigneProgrammeBrute {
    id: String,
    etablissement_id: String,
    annee_id: String,
    niveau_id: String,
    serie_id: Option<String>,
    matiere_id: String,
    coefficient: Option<i64>,
    minutes_hebdo: Option<i64>,
    ordre: i64,
}

impl LigneProgrammeBrute {
    fn vers_entite(self) -> Resultat<LigneProgramme> {
        let t = "programme_matiere";
        Ok(LigneProgramme {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            annee_id: lire_id(t, "annee_id", &self.annee_id)?,
            niveau_id: lire_id(t, "niveau_id", &self.niveau_id)?,
            serie_id: self
                .serie_id
                .as_deref()
                .map(|s| lire_id::<Serie>(t, "serie_id", s))
                .transpose()?,
            matiere_id: lire_id(t, "matiere_id", &self.matiere_id)?,
            coefficient: self.coefficient,
            minutes_hebdo: self.minutes_hebdo,
            ordre: self.ordre,
        })
    }
}

/// Programme d'une année, tous niveaux.
pub async fn programme(
    cnx: &mut Connexion,
    annee_id: Id<AnneeScolaire>,
    contexte: &Contexte,
) -> Resultat<Vec<LigneProgramme>> {
    sqlx::query_as::<_, LigneProgrammeBrute>(
        "SELECT id, etablissement_id, annee_id, niveau_id, serie_id, matiere_id, coefficient, minutes_hebdo, ordre
         FROM programme_matiere WHERE annee_id = ?1 AND etablissement_id = ?2 ORDER BY niveau_id, serie_id, ordre",
    )
    .bind(annee_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneProgrammeBrute::vers_entite)
    .collect()
}

/// Retire le programme d'un niveau (et d'une série) pour une année, avant de le récrire.
pub async fn retirer_programme(
    cnx: &mut Connexion,
    annee_id: Id<AnneeScolaire>,
    niveau_id: Id<Niveau>,
    serie_id: Option<Id<Serie>>,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "DELETE FROM programme_matiere
         WHERE annee_id = ?1 AND niveau_id = ?2 AND serie_id IS ?3 AND etablissement_id = ?4",
    )
    .bind(annee_id.vers_chaine())
    .bind(niveau_id.vers_chaine())
    .bind(serie_id.map(|s| s.vers_chaine()))
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Enregistre une ligne de programme.
pub async fn inserer_ligne_programme(
    cnx: &mut Connexion,
    l: &LigneProgramme,
    contexte: &Contexte,
) -> Resultat<()> {
    cloisonner(
        "programme_matiere",
        l.id.vers_chaine(),
        contexte,
        l.etablissement_id.vers_chaine(),
    )?;
    sqlx::query(
        "INSERT INTO programme_matiere (id, etablissement_id, annee_id, niveau_id, serie_id, matiere_id,
            coefficient, minutes_hebdo, ordre, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?10, ?11, ?12)",
    )
    .bind(l.id.vers_chaine())
    .bind(l.etablissement_id.vers_chaine())
    .bind(l.annee_id.vers_chaine())
    .bind(l.niveau_id.vers_chaine())
    .bind(l.serie_id.map(|s| s.vers_chaine()))
    .bind(l.matiere_id.vers_chaine())
    .bind(l.coefficient)
    .bind(l.minutes_hebdo)
    .bind(l.ordre)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

// --- Périodes -------------------------------------------------------------------------------

#[derive(FromRow)]
struct LignePeriode {
    id: String,
    etablissement_id: String,
    annee_id: String,
    niveau_id: String,
    rang: i64,
    #[sqlx(rename = "type")]
    type_periode: String,
    debut: i64,
    fin: i64,
    ponderation: i64,
    verrouillee: i64,
}

fn jour_de(table: &'static str, colonne: &'static str, millis: i64) -> Resultat<DateCivile> {
    DateCivile::du_jour(Horodatage::depuis_millis_epoque(millis))
        .ok_or_else(|| ErreurBase::illisible(table, colonne, millis.to_string()))
}

impl LignePeriode {
    fn vers_entite(self) -> Resultat<Periode> {
        let t = "periode_academique";
        Ok(Periode {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            annee_id: lire_id(t, "annee_id", &self.annee_id)?,
            niveau_id: lire_id(t, "niveau_id", &self.niveau_id)?,
            rang: self.rang,
            type_periode: TypePeriode::depuis_nom(&self.type_periode)
                .ok_or_else(|| ErreurBase::illisible(t, "type", &self.type_periode))?,
            debut: jour_de(t, "debut", self.debut)?,
            fin: jour_de(t, "fin", self.fin)?,
            ponderation: self.ponderation,
            verrouillee: booleen(self.verrouillee),
        })
    }
}

/// Bornes persistées d'une période : minuit du premier jour, dernière milliseconde du dernier.
fn bornes(p: &Periode) -> (i64, i64) {
    (
        p.debut.jours_depuis_epoque() * JOUR,
        (p.fin.jours_depuis_epoque() + 1) * JOUR - 1,
    )
}

/// Périodes en vigueur d'une année, par niveau puis par rang.
pub async fn periodes(
    cnx: &mut Connexion,
    annee_id: Id<AnneeScolaire>,
    contexte: &Contexte,
) -> Resultat<Vec<Periode>> {
    sqlx::query_as::<_, LignePeriode>(
        "SELECT id, etablissement_id, annee_id, niveau_id, rang, type, debut, fin, ponderation, verrouillee
         FROM periode_academique
         WHERE annee_id = ?1 AND etablissement_id = ?2 AND niveau_id IS NOT NULL AND supprime_le IS NULL
         ORDER BY niveau_id, rang",
    )
    .bind(annee_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LignePeriode::vers_entite)
    .collect()
}

/// Enregistre une période.
pub async fn inserer_periode(
    cnx: &mut Connexion,
    p: &Periode,
    contexte: &Contexte,
) -> Resultat<()> {
    cloisonner(
        "periode_academique",
        p.id.vers_chaine(),
        contexte,
        p.etablissement_id.vers_chaine(),
    )?;
    let (debut, fin) = bornes(p);
    sqlx::query(
        "INSERT INTO periode_academique (id, etablissement_id, annee_id, niveau_id, rang, type, debut, fin,
            ponderation, verrouillee, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?11, ?12, ?13)",
    )
    .bind(p.id.vers_chaine())
    .bind(p.etablissement_id.vers_chaine())
    .bind(p.annee_id.vers_chaine())
    .bind(p.niveau_id.vers_chaine())
    .bind(p.rang)
    .bind(p.type_periode.nom())
    .bind(debut)
    .bind(fin)
    .bind(p.ponderation)
    .bind(i64::from(p.verrouillee))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Récrit les bornes, le poids et le verrou d'une période.
pub async fn ecrire_periode(cnx: &mut Connexion, p: &Periode, contexte: &Contexte) -> Resultat<()> {
    let (debut, fin) = bornes(p);
    sqlx::query(
        "UPDATE periode_academique SET debut = ?1, fin = ?2, ponderation = ?3, verrouillee = ?4,
            mis_a_jour_le = ?5, mis_a_jour_par = ?6, identifiant_poste = ?7
         WHERE id = ?8 AND etablissement_id = ?9",
    )
    .bind(debut)
    .bind(fin)
    .bind(p.ponderation)
    .bind(i64::from(p.verrouillee))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(p.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Retire les périodes d'un niveau pour une année (suppression logique), avant d'en définir
/// d'autres.
pub async fn retirer_periodes(
    cnx: &mut Connexion,
    annee_id: Id<AnneeScolaire>,
    niveau_id: Id<Niveau>,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE periode_academique SET supprime_le = ?1, supprime_par = ?2
         WHERE annee_id = ?3 AND niveau_id = ?4 AND etablissement_id = ?5 AND supprime_le IS NULL",
    )
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(annee_id.vers_chaine())
    .bind(niveau_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

// --- Règles d'évaluation --------------------------------------------------------------------

#[derive(FromRow)]
struct LigneRegle {
    id: String,
    etablissement_id: String,
    annee_id: String,
    cycle: String,
    version: i64,
    contenu: String,
    le: i64,
}

impl LigneRegle {
    fn vers_entite(self) -> Resultat<RegleEvaluation> {
        let t = "regle_evaluation";
        Ok(RegleEvaluation {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            annee_id: lire_id(t, "annee_id", &self.annee_id)?,
            cycle: Cycle::depuis_nom(&self.cycle)
                .ok_or_else(|| ErreurBase::illisible(t, "cycle", &self.cycle))?,
            version: self.version,
            contenu: serde_json::from_str::<ContenuRegle>(&self.contenu)?,
            le: Horodatage::depuis_millis_epoque(self.le),
        })
    }
}

/// Dernière version de la règle de chaque cycle pour une année.
pub async fn regles_en_vigueur(
    cnx: &mut Connexion,
    annee_id: Id<AnneeScolaire>,
    contexte: &Contexte,
) -> Resultat<Vec<RegleEvaluation>> {
    sqlx::query_as::<_, LigneRegle>(
        "SELECT r.id, r.etablissement_id, r.annee_id, r.cycle, r.version, r.contenu, r.le
         FROM regle_evaluation r
         WHERE r.annee_id = ?1 AND r.etablissement_id = ?2
           AND r.version = (SELECT max(version) FROM regle_evaluation x
                            WHERE x.annee_id = r.annee_id AND x.cycle = r.cycle)
         ORDER BY r.cycle",
    )
    .bind(annee_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneRegle::vers_entite)
    .collect()
}

/// Enregistre une version de règle ; une version existante ne change jamais.
pub async fn inserer_regle(
    cnx: &mut Connexion,
    r: &RegleEvaluation,
    contexte: &Contexte,
) -> Resultat<()> {
    cloisonner(
        "regle_evaluation",
        r.id.vers_chaine(),
        contexte,
        r.etablissement_id.vers_chaine(),
    )?;
    sqlx::query(
        "INSERT INTO regle_evaluation (id, etablissement_id, annee_id, cycle, version, contenu, le,
            cree_le, cree_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
    )
    .bind(r.id.vers_chaine())
    .bind(r.etablissement_id.vers_chaine())
    .bind(r.annee_id.vers_chaine())
    .bind(r.cycle.nom())
    .bind(r.version)
    .bind(serde_json::to_string(&r.contenu)?)
    .bind(r.le.millis_epoque())
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}
