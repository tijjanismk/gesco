//! Inscriptions (SPEC_PHASE_2 §5). Aucune suppression : l'état seul évolue (R-14).

use gesco_core::commun::{DateCivile, Id};
use gesco_core::core::{AnneeScolaire, MachineAEtats};
use gesco_core::scolarite::{
    Classe, DecisionFinAnnee, Eleve, Inscription, StatutInscription, TypeInscription,
};
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::depots::booleen;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

#[derive(FromRow)]
struct Ligne {
    id: String,
    etablissement_id: String,
    annee_id: String,
    eleve_id: String,
    classe_id: String,
    date_inscription: String,
    #[sqlx(rename = "type")]
    type_: String,
    statut: String,
    redoublant: i64,
    decision_fin_annee: String,
    classe_precedente_id: Option<String>,
    etablissement_origine: String,
    motif_abandon: String,
    date_depart: Option<String>,
    etablissement_destination: String,
    motif_depart: String,
}

fn date(colonne: &'static str, texte: &str) -> Resultat<DateCivile> {
    DateCivile::analyser(texte).ok_or_else(|| ErreurBase::illisible("inscription", colonne, texte))
}

impl Ligne {
    fn vers_entite(self) -> Resultat<Inscription> {
        Ok(Inscription {
            id: lire_id("inscription", "id", &self.id)?,
            etablissement_id: lire_id("inscription", "etablissement_id", &self.etablissement_id)?,
            annee_id: lire_id("inscription", "annee_id", &self.annee_id)?,
            eleve_id: lire_id("inscription", "eleve_id", &self.eleve_id)?,
            classe_id: lire_id("inscription", "classe_id", &self.classe_id)?,
            date_inscription: date("date_inscription", &self.date_inscription)?,
            type_inscription: TypeInscription::depuis_nom(&self.type_)
                .ok_or_else(|| ErreurBase::illisible("inscription", "type", &self.type_))?,
            statut: StatutInscription::depuis_nom(&self.statut)
                .ok_or_else(|| ErreurBase::illisible("inscription", "statut", &self.statut))?,
            redoublant: booleen(self.redoublant),
            decision_fin_annee: DecisionFinAnnee::depuis_nom(&self.decision_fin_annee).ok_or_else(
                || {
                    ErreurBase::illisible(
                        "inscription",
                        "decision_fin_annee",
                        &self.decision_fin_annee,
                    )
                },
            )?,
            classe_precedente_id: self
                .classe_precedente_id
                .as_deref()
                .map(|c| lire_id("inscription", "classe_precedente_id", c))
                .transpose()?,
            etablissement_origine: self.etablissement_origine,
            motif_abandon: self.motif_abandon,
            date_depart: self
                .date_depart
                .as_deref()
                .map(|d| date("date_depart", d))
                .transpose()?,
            etablissement_destination: self.etablissement_destination,
            motif_depart: self.motif_depart,
        })
    }
}

macro_rules! selectionner {
    ($suite:literal) => {
        concat!(
            "SELECT id, etablissement_id, annee_id, eleve_id, classe_id, date_inscription, type, statut,
                redoublant, decision_fin_annee, classe_precedente_id, etablissement_origine, motif_abandon,
                date_depart, etablissement_destination, motif_depart FROM inscription ",
            $suite
        )
    };
}

fn introuvable(id: Id<Inscription>) -> ErreurBase {
    ErreurBase::Introuvable {
        entite: "inscription",
        id: id.vers_chaine(),
    }
}

/// Crée une inscription.
pub async fn inserer(
    cnx: &mut Connexion,
    inscription: &Inscription,
    contexte: &Contexte,
) -> Resultat<()> {
    if inscription.etablissement_id != contexte.etablissement_id {
        return Err(ErreurBase::Cloisonnement {
            entite: "inscription",
            id: inscription.id.vers_chaine(),
        });
    }
    sqlx::query(
        "INSERT INTO inscription (id, etablissement_id, annee_id, eleve_id, classe_id, date_inscription, type, statut,
            redoublant, decision_fin_annee, classe_precedente_id, etablissement_origine, motif_abandon,
            date_depart, etablissement_destination, motif_depart,
            cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?17, ?18, ?19)",
    )
    .bind(inscription.id.vers_chaine())
    .bind(inscription.etablissement_id.vers_chaine())
    .bind(inscription.annee_id.vers_chaine())
    .bind(inscription.eleve_id.vers_chaine())
    .bind(inscription.classe_id.vers_chaine())
    .bind(inscription.date_inscription.to_string())
    .bind(inscription.type_inscription.nom())
    .bind(inscription.statut.nom())
    .bind(i64::from(inscription.redoublant))
    .bind(inscription.decision_fin_annee.nom())
    .bind(inscription.classe_precedente_id.map(|c| c.vers_chaine()))
    .bind(&inscription.etablissement_origine)
    .bind(&inscription.motif_abandon)
    .bind(inscription.date_depart.map(|d| d.to_string()))
    .bind(&inscription.etablissement_destination)
    .bind(&inscription.motif_depart)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Lit une inscription.
pub async fn lire(
    cnx: &mut Connexion,
    id: Id<Inscription>,
    contexte: &Contexte,
) -> Resultat<Inscription> {
    sqlx::query_as::<_, Ligne>(selectionner!("WHERE id = ?1 AND etablissement_id = ?2"))
        .bind(id.vers_chaine())
        .bind(contexte.etablissement_id.vers_chaine())
        .fetch_optional(cnx)
        .await?
        .ok_or_else(|| introuvable(id))?
        .vers_entite()
}

/// Récrit les champs d'un brouillon : classe, date, redoublement, provenance, nature.
pub async fn modifier_brouillon(
    cnx: &mut Connexion,
    inscription: &Inscription,
    contexte: &Contexte,
) -> Resultat<()> {
    let modifiees = sqlx::query(
        "UPDATE inscription SET classe_id = ?1, date_inscription = ?2, redoublant = ?3, etablissement_origine = ?4,
            type = ?5, mis_a_jour_le = ?6, mis_a_jour_par = ?7, identifiant_poste = ?8
         WHERE id = ?9 AND etablissement_id = ?10 AND statut = 'brouillon'",
    )
    .bind(inscription.classe_id.vers_chaine())
    .bind(inscription.date_inscription.to_string())
    .bind(i64::from(inscription.redoublant))
    .bind(&inscription.etablissement_origine)
    .bind(inscription.type_inscription.nom())
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(inscription.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?
    .rows_affected();
    if modifiees == 0 {
        return Err(introuvable(inscription.id));
    }
    Ok(())
}

/// Applique une transition déjà vérifiée par le noyau (R-30), avec le motif d'un abandon.
pub async fn ecrire_statut(
    cnx: &mut Connexion,
    id: Id<Inscription>,
    statut: StatutInscription,
    motif_abandon: &str,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE inscription SET statut = ?1, motif_abandon = ?2, mis_a_jour_le = ?3, mis_a_jour_par = ?4, identifiant_poste = ?5
         WHERE id = ?6 AND etablissement_id = ?7",
    )
    .bind(statut.nom())
    .bind(motif_abandon)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Écrit la décision de fin d'année d'une inscription.
pub async fn ecrire_decision(
    cnx: &mut Connexion,
    id: Id<Inscription>,
    decision: DecisionFinAnnee,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE inscription SET decision_fin_annee = ?1, mis_a_jour_le = ?2, mis_a_jour_par = ?3, identifiant_poste = ?4
         WHERE id = ?5 AND etablissement_id = ?6",
    )
    .bind(decision.nom())
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Place l'inscription dans une autre classe et garde la précédente.
pub async fn changer_classe(
    cnx: &mut Connexion,
    id: Id<Inscription>,
    nouvelle: Id<Classe>,
    precedente: Id<Classe>,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE inscription SET classe_id = ?1, classe_precedente_id = ?2, mis_a_jour_le = ?3, mis_a_jour_par = ?4, identifiant_poste = ?5
         WHERE id = ?6 AND etablissement_id = ?7",
    )
    .bind(nouvelle.vers_chaine())
    .bind(precedente.vers_chaine())
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Inscriptions d'un élève, toutes années, la plus récente d'abord.
pub async fn de_l_eleve(
    cnx: &mut Connexion,
    eleve_id: Id<Eleve>,
    contexte: &Contexte,
) -> Resultat<Vec<Inscription>> {
    sqlx::query_as::<_, Ligne>(selectionner!(
        "WHERE eleve_id = ?1 AND etablissement_id = ?2 ORDER BY rowid DESC"
    ))
    .bind(eleve_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(Ligne::vers_entite)
    .collect()
}

/// Fait passer toutes les inscriptions d'un élève sur un autre : la fusion de deux fiches.
pub async fn reattribuer(
    cnx: &mut Connexion,
    de: Id<Eleve>,
    vers: Id<Eleve>,
    contexte: &Contexte,
) -> Resultat<u64> {
    Ok(sqlx::query(
        "UPDATE inscription SET eleve_id = ?1, mis_a_jour_le = ?2, mis_a_jour_par = ?3, identifiant_poste = ?4
         WHERE eleve_id = ?5 AND etablissement_id = ?6",
    )
    .bind(vers.vers_chaine())
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(de.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?
    .rows_affected())
}

/// Inscriptions d'une année.
pub async fn de_l_annee(
    cnx: &mut Connexion,
    annee_id: Id<AnneeScolaire>,
    contexte: &Contexte,
) -> Resultat<Vec<Inscription>> {
    sqlx::query_as::<_, Ligne>(selectionner!(
        "WHERE annee_id = ?1 AND etablissement_id = ?2 ORDER BY rowid"
    ))
    .bind(annee_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(Ligne::vers_entite)
    .collect()
}

/// Nombre d'inscriptions, de tout état, dans une classe : une classe qui en a reçu une ne se
/// supprime plus.
pub async fn compter_dans_classe(
    cnx: &mut Connexion,
    classe_id: Id<Classe>,
    contexte: &Contexte,
) -> Resultat<i64> {
    Ok(sqlx::query_scalar(
        "SELECT COUNT(*) FROM inscription WHERE classe_id = ?1 AND etablissement_id = ?2",
    )
    .bind(classe_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_one(cnx)
    .await?)
}
