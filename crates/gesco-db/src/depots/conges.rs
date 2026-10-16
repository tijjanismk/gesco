//! Absences, retards et congés du personnel (SPEC_PHASE_6 §5 et §6) : types de congé, congés,
//! absences.

use gesco_core::commun::{DateCivile, Id};
use gesco_core::core::Utilisateur;
use gesco_core::core::transition::MachineAEtats;
use gesco_core::rh::{AbsencePersonnel, Conge, NatureAbsencePersonnel, StatutConge, TypeConge};
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::depots::booleen;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

fn cloisonne<T>(entite: &'static str, id: Id<T>, etablissement: bool) -> Resultat<()> {
    if etablissement {
        Ok(())
    } else {
        Err(ErreurBase::Cloisonnement {
            entite,
            id: id.vers_chaine(),
        })
    }
}

fn date(table: &'static str, colonne: &'static str, texte: &str) -> Resultat<DateCivile> {
    DateCivile::analyser(texte).ok_or_else(|| ErreurBase::illisible(table, colonne, texte))
}

#[derive(FromRow)]
struct LigneType {
    id: String,
    etablissement_id: String,
    code: Option<String>,
    libelle: String,
    ordre: i64,
    actif: i64,
    deduit_du_solde: i64,
}

impl LigneType {
    fn vers_entite(self) -> Resultat<TypeConge> {
        let t = "type_conge";
        Ok(TypeConge {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            code: self.code,
            libelle: self.libelle,
            ordre: self.ordre,
            actif: booleen(self.actif),
            deduit_du_solde: booleen(self.deduit_du_solde),
        })
    }
}

/// Types de congé de l'établissement, dans l'ordre.
pub async fn types(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<TypeConge>> {
    sqlx::query_as::<_, LigneType>(
        "SELECT id, etablissement_id, code, libelle, ordre, actif, deduit_du_solde FROM type_conge
         WHERE etablissement_id = ?1 ORDER BY ordre, libelle",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneType::vers_entite)
    .collect()
}

/// Enregistre un type de congé.
pub async fn inserer_type(cnx: &mut Connexion, t: &TypeConge, contexte: &Contexte) -> Resultat<()> {
    cloisonne(
        "type_conge",
        t.id,
        t.etablissement_id == contexte.etablissement_id,
    )?;
    sqlx::query(
        "INSERT INTO type_conge (id, etablissement_id, code, libelle, ordre, actif,
            deduit_du_solde, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?8, ?9, ?10)",
    )
    .bind(t.id.vers_chaine())
    .bind(t.etablissement_id.vers_chaine())
    .bind(t.code.as_deref())
    .bind(&t.libelle)
    .bind(t.ordre)
    .bind(i64::from(t.actif))
    .bind(i64::from(t.deduit_du_solde))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Récrit le libellé, l'état et la déduction d'un type de congé.
pub async fn ecrire_type(cnx: &mut Connexion, t: &TypeConge, contexte: &Contexte) -> Resultat<()> {
    sqlx::query(
        "UPDATE type_conge SET libelle = ?1, ordre = ?2, actif = ?3, deduit_du_solde = ?4,
            mis_a_jour_le = ?5, mis_a_jour_par = ?6, identifiant_poste = ?7
         WHERE id = ?8 AND etablissement_id = ?9",
    )
    .bind(&t.libelle)
    .bind(t.ordre)
    .bind(i64::from(t.actif))
    .bind(i64::from(t.deduit_du_solde))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(t.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

#[derive(FromRow)]
struct LigneConge {
    id: String,
    etablissement_id: String,
    membre_id: String,
    type_id: String,
    debut: String,
    fin: String,
    jours: i64,
    motif: String,
    statut: String,
    decide_le: Option<i64>,
    decide_par: Option<String>,
    motif_refus: String,
    motif_annulation: String,
}

impl LigneConge {
    fn vers_entite(self) -> Resultat<Conge> {
        let t = "conge";
        Ok(Conge {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            membre_id: lire_id(t, "membre_id", &self.membre_id)?,
            type_id: lire_id(t, "type_id", &self.type_id)?,
            debut: date(t, "debut", &self.debut)?,
            fin: date(t, "fin", &self.fin)?,
            jours: self.jours,
            motif: self.motif,
            statut: StatutConge::depuis_nom(&self.statut)
                .ok_or_else(|| ErreurBase::illisible(t, "statut", &self.statut))?,
            decide_le: crate::depots::horodatage_optionnel(self.decide_le),
            decide_par: self
                .decide_par
                .as_deref()
                .map(|u| lire_id::<Utilisateur>(t, "decide_par", u))
                .transpose()?,
            motif_refus: self.motif_refus,
            motif_annulation: self.motif_annulation,
        })
    }
}

macro_rules! selectionner_conge {
    ($suite:literal) => {
        concat!(
            "SELECT id, etablissement_id, membre_id, type_id, debut, fin, jours, motif, statut,
                decide_le, decide_par, motif_refus, motif_annulation
             FROM conge ",
            $suite
        )
    };
}

/// Congés de l'établissement, du plus ancien au plus récent.
pub async fn conges(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<Conge>> {
    sqlx::query_as::<_, LigneConge>(selectionner_conge!(
        "WHERE etablissement_id = ?1 ORDER BY debut, cree_le, id"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneConge::vers_entite)
    .collect()
}

/// Un congé.
pub async fn conge(cnx: &mut Connexion, id: Id<Conge>, contexte: &Contexte) -> Resultat<Conge> {
    sqlx::query_as::<_, LigneConge>(selectionner_conge!(
        "WHERE id = ?1 AND etablissement_id = ?2"
    ))
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .ok_or_else(|| ErreurBase::Introuvable {
        entite: "conge",
        id: id.vers_chaine(),
    })?
    .vers_entite()
}

/// Enregistre un congé demandé.
pub async fn inserer_conge(cnx: &mut Connexion, c: &Conge, contexte: &Contexte) -> Resultat<()> {
    cloisonne(
        "conge",
        c.id,
        c.etablissement_id == contexte.etablissement_id,
    )?;
    sqlx::query(
        "INSERT INTO conge (id, etablissement_id, membre_id, type_id, debut, fin, jours, motif,
            statut, decide_le, decide_par, motif_refus, motif_annulation, cree_le, cree_par,
            mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?14, ?15, ?16)",
    )
    .bind(c.id.vers_chaine())
    .bind(c.etablissement_id.vers_chaine())
    .bind(c.membre_id.vers_chaine())
    .bind(c.type_id.vers_chaine())
    .bind(c.debut.to_string())
    .bind(c.fin.to_string())
    .bind(c.jours)
    .bind(&c.motif)
    .bind(c.statut.nom())
    .bind(c.decide_le.map(|v| v.millis_epoque()))
    .bind(c.decide_par.map(|u| u.vers_chaine()))
    .bind(&c.motif_refus)
    .bind(&c.motif_annulation)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Récrit l'état d'un congé : décision, refus, annulation.
pub async fn ecrire_statut_conge(
    cnx: &mut Connexion,
    c: &Conge,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE conge SET statut = ?1, decide_le = ?2, decide_par = ?3, motif_refus = ?4,
            motif_annulation = ?5, mis_a_jour_le = ?6, mis_a_jour_par = ?7,
            identifiant_poste = ?8
         WHERE id = ?9 AND etablissement_id = ?10",
    )
    .bind(c.statut.nom())
    .bind(c.decide_le.map(|v| v.millis_epoque()))
    .bind(c.decide_par.map(|u| u.vers_chaine()))
    .bind(&c.motif_refus)
    .bind(&c.motif_annulation)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(c.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

#[derive(FromRow)]
struct LigneAbsence {
    id: String,
    etablissement_id: String,
    membre_id: String,
    date: String,
    nature: String,
    minutes: Option<i64>,
    motif: String,
    justifiee: i64,
    annulee: i64,
    motif_annulation: String,
}

impl LigneAbsence {
    fn vers_entite(self) -> Resultat<AbsencePersonnel> {
        let t = "absence_personnel";
        Ok(AbsencePersonnel {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            membre_id: lire_id(t, "membre_id", &self.membre_id)?,
            date: date(t, "date", &self.date)?,
            nature: NatureAbsencePersonnel::depuis_nom(&self.nature)
                .ok_or_else(|| ErreurBase::illisible(t, "nature", &self.nature))?,
            minutes: self.minutes,
            motif: self.motif,
            justifiee: booleen(self.justifiee),
            annulee: booleen(self.annulee),
            motif_annulation: self.motif_annulation,
        })
    }
}

macro_rules! selectionner_absence {
    ($suite:literal) => {
        concat!(
            "SELECT id, etablissement_id, membre_id, date, nature, minutes, motif, justifiee,
                annulee, motif_annulation
             FROM absence_personnel ",
            $suite
        )
    };
}

/// Absences du personnel entre deux jours inclus, par jour.
pub async fn absences_entre(
    cnx: &mut Connexion,
    debut: DateCivile,
    fin: DateCivile,
    contexte: &Contexte,
) -> Resultat<Vec<AbsencePersonnel>> {
    sqlx::query_as::<_, LigneAbsence>(selectionner_absence!(
        "WHERE etablissement_id = ?1 AND date BETWEEN ?2 AND ?3 ORDER BY date, cree_le, id"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(debut.to_string())
    .bind(fin.to_string())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneAbsence::vers_entite)
    .collect()
}

/// Une absence.
pub async fn absence(
    cnx: &mut Connexion,
    id: Id<AbsencePersonnel>,
    contexte: &Contexte,
) -> Resultat<AbsencePersonnel> {
    sqlx::query_as::<_, LigneAbsence>(selectionner_absence!(
        "WHERE id = ?1 AND etablissement_id = ?2"
    ))
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .ok_or_else(|| ErreurBase::Introuvable {
        entite: "absence_personnel",
        id: id.vers_chaine(),
    })?
    .vers_entite()
}

/// Enregistre une absence.
pub async fn inserer_absence(
    cnx: &mut Connexion,
    a: &AbsencePersonnel,
    contexte: &Contexte,
) -> Resultat<()> {
    cloisonne(
        "absence_personnel",
        a.id,
        a.etablissement_id == contexte.etablissement_id,
    )?;
    sqlx::query(
        "INSERT INTO absence_personnel (id, etablissement_id, membre_id, date, nature, minutes,
            motif, justifiee, annulee, motif_annulation, cree_le, cree_par, mis_a_jour_le,
            mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?11, ?12, ?13)",
    )
    .bind(a.id.vers_chaine())
    .bind(a.etablissement_id.vers_chaine())
    .bind(a.membre_id.vers_chaine())
    .bind(a.date.to_string())
    .bind(a.nature.nom())
    .bind(a.minutes)
    .bind(&a.motif)
    .bind(i64::from(a.justifiee))
    .bind(i64::from(a.annulee))
    .bind(&a.motif_annulation)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Récrit le motif, la justification et l'annulation d'une absence.
pub async fn ecrire_absence(
    cnx: &mut Connexion,
    a: &AbsencePersonnel,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE absence_personnel SET motif = ?1, justifiee = ?2, annulee = ?3,
            motif_annulation = ?4, mis_a_jour_le = ?5, mis_a_jour_par = ?6,
            identifiant_poste = ?7
         WHERE id = ?8 AND etablissement_id = ?9",
    )
    .bind(&a.motif)
    .bind(i64::from(a.justifiee))
    .bind(i64::from(a.annulee))
    .bind(&a.motif_annulation)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(a.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}
