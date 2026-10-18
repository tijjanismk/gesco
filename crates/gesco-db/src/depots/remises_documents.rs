//! Remise des documents et convocations (SPEC_PHASE_5 §6 et §7).

use gesco_core::commun::{DateCivile, Horodatage, Id};
use gesco_core::pedagogie::Periode;
use gesco_core::scolarite::Inscription;
use gesco_core::scolarite::convocation::{
    AccuseConvocation, CanalConvocation, Convocation, SuiteConvocation,
};
use gesco_core::scolarite::remise_document::{RemisA, RemiseDocument, StatutRemiseDocument};
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::depots::booleen;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

#[derive(FromRow)]
struct LigneRemise {
    id: String,
    etablissement_id: String,
    bulletin_id: String,
    inscription_id: String,
    periode_id: String,
    statut: String,
    remis_a: Option<String>,
    responsable_id: Option<String>,
    nom_retrait: String,
    signataire: String,
    remis_le: Option<i64>,
    signe_le: Option<i64>,
    remis_par: Option<String>,
    publie_le: i64,
}

impl LigneRemise {
    fn vers_entite(self) -> Resultat<RemiseDocument> {
        let t = "remise_document";
        Ok(RemiseDocument {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            bulletin_id: lire_id(t, "bulletin_id", &self.bulletin_id)?,
            inscription_id: lire_id(t, "inscription_id", &self.inscription_id)?,
            periode_id: lire_id(t, "periode_id", &self.periode_id)?,
            statut: StatutRemiseDocument::depuis_nom(&self.statut)
                .ok_or_else(|| ErreurBase::illisible(t, "statut", &self.statut))?,
            remis_a: self
                .remis_a
                .as_deref()
                .map(|r| {
                    RemisA::depuis_nom(r).ok_or_else(|| ErreurBase::illisible(t, "remis_a", r))
                })
                .transpose()?,
            responsable_id: self
                .responsable_id
                .as_deref()
                .map(|r| lire_id(t, "responsable_id", r))
                .transpose()?,
            nom_retrait: self.nom_retrait,
            signataire: self.signataire,
            remis_le: self.remis_le.map(Horodatage::depuis_millis_epoque),
            signe_le: self.signe_le.map(Horodatage::depuis_millis_epoque),
            remis_par: self
                .remis_par
                .as_deref()
                .map(|r| lire_id(t, "remis_par", r))
                .transpose()?,
            publie_le: Horodatage::depuis_millis_epoque(self.publie_le),
        })
    }
}

macro_rules! selectionner_remise {
    ($suite:literal) => {
        concat!(
            "SELECT id, etablissement_id, bulletin_id, inscription_id, periode_id, statut, remis_a,
                responsable_id, nom_retrait, signataire, remis_le, signe_le, remis_par, publie_le
             FROM remise_document ",
            $suite
        )
    };
}

fn remises(lignes: Vec<LigneRemise>) -> Resultat<Vec<RemiseDocument>> {
    lignes.into_iter().map(LigneRemise::vers_entite).collect()
}

/// Une remise.
pub async fn remise(
    cnx: &mut Connexion,
    id: Id<RemiseDocument>,
    contexte: &Contexte,
) -> Resultat<RemiseDocument> {
    sqlx::query_as::<_, LigneRemise>(selectionner_remise!(
        "WHERE id = ?1 AND etablissement_id = ?2"
    ))
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .ok_or_else(|| ErreurBase::Introuvable {
        entite: "remise_document",
        id: id.vers_chaine(),
    })?
    .vers_entite()
}

/// La remise du bulletin d'une inscription pour une période, s'il y en a une.
pub async fn de_la_periode_de(
    cnx: &mut Connexion,
    inscription_id: Id<Inscription>,
    periode_id: Id<Periode>,
    contexte: &Contexte,
) -> Resultat<Option<RemiseDocument>> {
    sqlx::query_as::<_, LigneRemise>(selectionner_remise!(
        "WHERE inscription_id = ?1 AND periode_id = ?2 AND etablissement_id = ?3"
    ))
    .bind(inscription_id.vers_chaine())
    .bind(periode_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .map(LigneRemise::vers_entite)
    .transpose()
}

/// Remises d'une période.
pub async fn de_la_periode(
    cnx: &mut Connexion,
    periode_id: Id<Periode>,
    contexte: &Contexte,
) -> Resultat<Vec<RemiseDocument>> {
    remises(
        sqlx::query_as::<_, LigneRemise>(selectionner_remise!(
            "WHERE periode_id = ?1 AND etablissement_id = ?2 ORDER BY publie_le"
        ))
        .bind(periode_id.vers_chaine())
        .bind(contexte.etablissement_id.vers_chaine())
        .fetch_all(cnx)
        .await?,
    )
}

/// Remises d'une inscription, par publication.
pub async fn de_l_inscription(
    cnx: &mut Connexion,
    inscription_id: Id<Inscription>,
    contexte: &Contexte,
) -> Resultat<Vec<RemiseDocument>> {
    remises(
        sqlx::query_as::<_, LigneRemise>(selectionner_remise!(
            "WHERE inscription_id = ?1 AND etablissement_id = ?2 ORDER BY publie_le"
        ))
        .bind(inscription_id.vers_chaine())
        .bind(contexte.etablissement_id.vers_chaine())
        .fetch_all(cnx)
        .await?,
    )
}

/// Remises qui attendent encore : à remettre ou non retirées, les plus anciennes d'abord.
pub async fn en_attente(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<RemiseDocument>> {
    remises(
        sqlx::query_as::<_, LigneRemise>(selectionner_remise!(
            "WHERE etablissement_id = ?1 AND statut IN ('a_remettre', 'non_retire')
             ORDER BY publie_le"
        ))
        .bind(contexte.etablissement_id.vers_chaine())
        .fetch_all(cnx)
        .await?,
    )
}

/// Enregistre une remise, nouvelle ou modifiée.
pub async fn enregistrer(
    cnx: &mut Connexion,
    r: &RemiseDocument,
    contexte: &Contexte,
) -> Resultat<()> {
    if r.etablissement_id != contexte.etablissement_id {
        return Err(ErreurBase::Cloisonnement {
            entite: "remise_document",
            id: r.id.vers_chaine(),
        });
    }
    sqlx::query(
        "INSERT INTO remise_document (id, etablissement_id, bulletin_id, inscription_id,
            periode_id, statut, remis_a, responsable_id, nom_retrait, signataire, remis_le,
            signe_le, remis_par, publie_le, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par,
            identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?15, ?16,
            ?17)
         ON CONFLICT (id) DO UPDATE SET bulletin_id = excluded.bulletin_id,
            statut = excluded.statut, remis_a = excluded.remis_a,
            responsable_id = excluded.responsable_id, nom_retrait = excluded.nom_retrait,
            signataire = excluded.signataire, remis_le = excluded.remis_le,
            signe_le = excluded.signe_le, remis_par = excluded.remis_par,
            publie_le = excluded.publie_le, mis_a_jour_le = excluded.mis_a_jour_le,
            mis_a_jour_par = excluded.mis_a_jour_par,
            identifiant_poste = excluded.identifiant_poste",
    )
    .bind(r.id.vers_chaine())
    .bind(r.etablissement_id.vers_chaine())
    .bind(r.bulletin_id.vers_chaine())
    .bind(r.inscription_id.vers_chaine())
    .bind(r.periode_id.vers_chaine())
    .bind(r.statut.nom())
    .bind(r.remis_a.map(|x| x.nom()))
    .bind(r.responsable_id.map(|x| x.vers_chaine()))
    .bind(&r.nom_retrait)
    .bind(&r.signataire)
    .bind(r.remis_le.map(|h| h.millis_epoque()))
    .bind(r.signe_le.map(|h| h.millis_epoque()))
    .bind(r.remis_par.map(|x| x.vers_chaine()))
    .bind(r.publie_le.millis_epoque())
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

#[derive(FromRow)]
struct LigneConvocation {
    id: String,
    etablissement_id: String,
    inscription_id: String,
    responsable_id: Option<String>,
    sanction_id: Option<String>,
    motif: String,
    date: String,
    heure: String,
    canal: String,
    accuse: String,
    suite: String,
    emise_le: i64,
    emise_par: String,
    annulee: i64,
    motif_annulation: String,
}

impl LigneConvocation {
    fn vers_entite(self) -> Resultat<Convocation> {
        let t = "convocation";
        Ok(Convocation {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            inscription_id: lire_id(t, "inscription_id", &self.inscription_id)?,
            responsable_id: self
                .responsable_id
                .as_deref()
                .map(|r| lire_id(t, "responsable_id", r))
                .transpose()?,
            sanction_id: self
                .sanction_id
                .as_deref()
                .map(|r| lire_id(t, "sanction_id", r))
                .transpose()?,
            motif: self.motif,
            date: DateCivile::analyser(&self.date)
                .ok_or_else(|| ErreurBase::illisible(t, "date", &self.date))?,
            heure: self.heure,
            canal: CanalConvocation::depuis_nom(&self.canal)
                .ok_or_else(|| ErreurBase::illisible(t, "canal", &self.canal))?,
            accuse: AccuseConvocation::depuis_nom(&self.accuse)
                .ok_or_else(|| ErreurBase::illisible(t, "accuse", &self.accuse))?,
            suite: SuiteConvocation::depuis_nom(&self.suite)
                .ok_or_else(|| ErreurBase::illisible(t, "suite", &self.suite))?,
            emise_le: Horodatage::depuis_millis_epoque(self.emise_le),
            emise_par: lire_id(t, "emise_par", &self.emise_par)?,
            annulee: booleen(self.annulee),
            motif_annulation: self.motif_annulation,
        })
    }
}

macro_rules! selectionner_convocation {
    ($suite:literal) => {
        concat!(
            "SELECT id, etablissement_id, inscription_id, responsable_id, sanction_id, motif, date,
                heure, canal, accuse, suite, emise_le, emise_par, annulee, motif_annulation
             FROM convocation ",
            $suite
        )
    };
}

fn convocations(lignes: Vec<LigneConvocation>) -> Resultat<Vec<Convocation>> {
    lignes
        .into_iter()
        .map(LigneConvocation::vers_entite)
        .collect()
}

/// Une convocation.
pub async fn convocation(
    cnx: &mut Connexion,
    id: Id<Convocation>,
    contexte: &Contexte,
) -> Resultat<Convocation> {
    sqlx::query_as::<_, LigneConvocation>(selectionner_convocation!(
        "WHERE id = ?1 AND etablissement_id = ?2"
    ))
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .ok_or_else(|| ErreurBase::Introuvable {
        entite: "convocation",
        id: id.vers_chaine(),
    })?
    .vers_entite()
}

/// Convocations de l'établissement, les rendez-vous les plus proches d'abord.
pub async fn toutes(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<Convocation>> {
    convocations(
        sqlx::query_as::<_, LigneConvocation>(selectionner_convocation!(
            "WHERE etablissement_id = ?1 ORDER BY date, heure, emise_le"
        ))
        .bind(contexte.etablissement_id.vers_chaine())
        .fetch_all(cnx)
        .await?,
    )
}

/// Convocations d'une inscription.
pub async fn de_l_inscription_convoquee(
    cnx: &mut Connexion,
    inscription_id: Id<Inscription>,
    contexte: &Contexte,
) -> Resultat<Vec<Convocation>> {
    convocations(
        sqlx::query_as::<_, LigneConvocation>(selectionner_convocation!(
            "WHERE inscription_id = ?1 AND etablissement_id = ?2 ORDER BY date, heure"
        ))
        .bind(inscription_id.vers_chaine())
        .bind(contexte.etablissement_id.vers_chaine())
        .fetch_all(cnx)
        .await?,
    )
}

/// Enregistre une convocation, nouvelle ou modifiée.
pub async fn enregistrer_convocation(
    cnx: &mut Connexion,
    c: &Convocation,
    contexte: &Contexte,
) -> Resultat<()> {
    if c.etablissement_id != contexte.etablissement_id {
        return Err(ErreurBase::Cloisonnement {
            entite: "convocation",
            id: c.id.vers_chaine(),
        });
    }
    sqlx::query(
        "INSERT INTO convocation (id, etablissement_id, inscription_id, responsable_id,
            sanction_id, motif, date, heure, canal, accuse, suite, emise_le, emise_par, annulee,
            motif_annulation, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?16,
            ?17, ?18)
         ON CONFLICT (id) DO UPDATE SET date = excluded.date, heure = excluded.heure,
            accuse = excluded.accuse, suite = excluded.suite, annulee = excluded.annulee,
            motif_annulation = excluded.motif_annulation, mis_a_jour_le = excluded.mis_a_jour_le,
            mis_a_jour_par = excluded.mis_a_jour_par,
            identifiant_poste = excluded.identifiant_poste",
    )
    .bind(c.id.vers_chaine())
    .bind(c.etablissement_id.vers_chaine())
    .bind(c.inscription_id.vers_chaine())
    .bind(c.responsable_id.map(|x| x.vers_chaine()))
    .bind(c.sanction_id.map(|x| x.vers_chaine()))
    .bind(&c.motif)
    .bind(c.date.to_string())
    .bind(&c.heure)
    .bind(c.canal.nom())
    .bind(c.accuse.nom())
    .bind(c.suite.nom())
    .bind(c.emise_le.millis_epoque())
    .bind(c.emise_par.vers_chaine())
    .bind(i64::from(c.annulee))
    .bind(&c.motif_annulation)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}
