//! Enseignants et affectations (SPEC_PHASE_4 §3).

use gesco_core::commun::Id;
use gesco_core::core::{AnneeScolaire, Utilisateur};
use gesco_core::pedagogie::{AffectationEnseignement, Enseignant, Matiere, StatutEnseignant};
use gesco_core::scolarite::Classe;
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::depots::booleen;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

#[derive(FromRow)]
struct LigneEnseignant {
    id: String,
    etablissement_id: String,
    personne_id: String,
    statut: String,
    utilisateur_id: Option<String>,
    actif: i64,
}

impl LigneEnseignant {
    fn vers_entite(self) -> Resultat<Enseignant> {
        let t = "enseignant";
        Ok(Enseignant {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            personne_id: lire_id(t, "personne_id", &self.personne_id)?,
            statut: StatutEnseignant::depuis_nom(&self.statut)
                .ok_or_else(|| ErreurBase::illisible(t, "statut", &self.statut))?,
            utilisateur_id: self
                .utilisateur_id
                .as_deref()
                .map(|u| lire_id::<Utilisateur>(t, "utilisateur_id", u))
                .transpose()?,
            actif: booleen(self.actif),
        })
    }
}

macro_rules! selectionner_enseignant {
    ($suite:literal) => {
        concat!(
            "SELECT id, etablissement_id, personne_id, statut, utilisateur_id, actif FROM enseignant ",
            $suite
        )
    };
}

/// Enregistre un enseignant.
pub async fn inserer_enseignant(
    cnx: &mut Connexion,
    e: &Enseignant,
    contexte: &Contexte,
) -> Resultat<()> {
    if e.etablissement_id != contexte.etablissement_id {
        return Err(ErreurBase::Cloisonnement {
            entite: "enseignant",
            id: e.id.vers_chaine(),
        });
    }
    sqlx::query(
        "INSERT INTO enseignant (id, etablissement_id, personne_id, statut, utilisateur_id, actif,
            cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?7, ?8, ?9)",
    )
    .bind(e.id.vers_chaine())
    .bind(e.etablissement_id.vers_chaine())
    .bind(e.personne_id.vers_chaine())
    .bind(e.statut.nom())
    .bind(e.utilisateur_id.map(|u| u.vers_chaine()))
    .bind(i64::from(e.actif))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Récrit le statut, le compte et l'état d'un enseignant.
pub async fn ecrire_enseignant(
    cnx: &mut Connexion,
    e: &Enseignant,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE enseignant SET statut = ?1, utilisateur_id = ?2, actif = ?3,
            mis_a_jour_le = ?4, mis_a_jour_par = ?5, identifiant_poste = ?6
         WHERE id = ?7 AND etablissement_id = ?8",
    )
    .bind(e.statut.nom())
    .bind(e.utilisateur_id.map(|u| u.vers_chaine()))
    .bind(i64::from(e.actif))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(e.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Enseignants de l'établissement.
pub async fn enseignants(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<Enseignant>> {
    sqlx::query_as::<_, LigneEnseignant>(selectionner_enseignant!(
        "WHERE etablissement_id = ?1 ORDER BY cree_le"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneEnseignant::vers_entite)
    .collect()
}

/// Enseignant lié à un compte, s'il y en a un.
pub async fn enseignant_du_compte(
    cnx: &mut Connexion,
    utilisateur_id: Id<Utilisateur>,
    contexte: &Contexte,
) -> Resultat<Option<Enseignant>> {
    sqlx::query_as::<_, LigneEnseignant>(selectionner_enseignant!(
        "WHERE utilisateur_id = ?1 AND etablissement_id = ?2 AND actif = 1"
    ))
    .bind(utilisateur_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .map(LigneEnseignant::vers_entite)
    .transpose()
}

#[derive(FromRow)]
struct LigneAffectation {
    id: String,
    etablissement_id: String,
    annee_id: String,
    classe_id: String,
    matiere_id: String,
    enseignant_id: String,
}

impl LigneAffectation {
    fn vers_entite(self) -> Resultat<AffectationEnseignement> {
        let t = "affectation_enseignement";
        Ok(AffectationEnseignement {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            annee_id: lire_id(t, "annee_id", &self.annee_id)?,
            classe_id: lire_id(t, "classe_id", &self.classe_id)?,
            matiere_id: lire_id(t, "matiere_id", &self.matiere_id)?,
            enseignant_id: lire_id(t, "enseignant_id", &self.enseignant_id)?,
        })
    }
}

/// Affectations d'une année.
pub async fn affectations(
    cnx: &mut Connexion,
    annee_id: Id<AnneeScolaire>,
    contexte: &Contexte,
) -> Resultat<Vec<AffectationEnseignement>> {
    sqlx::query_as::<_, LigneAffectation>(
        "SELECT id, etablissement_id, annee_id, classe_id, matiere_id, enseignant_id FROM affectation_enseignement
         WHERE annee_id = ?1 AND etablissement_id = ?2 ORDER BY cree_le",
    )
    .bind(annee_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneAffectation::vers_entite)
    .collect()
}

/// Affectations d'un enseignant, toutes années.
pub async fn affectations_de(
    cnx: &mut Connexion,
    enseignant_id: Id<Enseignant>,
    contexte: &Contexte,
) -> Resultat<Vec<AffectationEnseignement>> {
    sqlx::query_as::<_, LigneAffectation>(
        "SELECT id, etablissement_id, annee_id, classe_id, matiere_id, enseignant_id FROM affectation_enseignement
         WHERE enseignant_id = ?1 AND etablissement_id = ?2 ORDER BY cree_le",
    )
    .bind(enseignant_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneAffectation::vers_entite)
    .collect()
}

/// Enregistre une affectation.
pub async fn inserer_affectation(
    cnx: &mut Connexion,
    a: &AffectationEnseignement,
    contexte: &Contexte,
) -> Resultat<()> {
    if a.etablissement_id != contexte.etablissement_id {
        return Err(ErreurBase::Cloisonnement {
            entite: "affectation_enseignement",
            id: a.id.vers_chaine(),
        });
    }
    sqlx::query(
        "INSERT INTO affectation_enseignement (id, etablissement_id, annee_id, classe_id, matiere_id, enseignant_id,
            cree_le, cree_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
    )
    .bind(a.id.vers_chaine())
    .bind(a.etablissement_id.vers_chaine())
    .bind(a.annee_id.vers_chaine())
    .bind(a.classe_id.vers_chaine())
    .bind(a.matiere_id.vers_chaine())
    .bind(a.enseignant_id.vers_chaine())
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Retire une affectation.
pub async fn retirer_affectation(
    cnx: &mut Connexion,
    id: Id<AffectationEnseignement>,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query("DELETE FROM affectation_enseignement WHERE id = ?1 AND etablissement_id = ?2")
        .bind(id.vers_chaine())
        .bind(contexte.etablissement_id.vers_chaine())
        .execute(cnx)
        .await?;
    Ok(())
}

/// Retire toutes les affectations d'une matière d'une classe, avant de la confier à un autre.
pub async fn retirer_affectations_de(
    cnx: &mut Connexion,
    classe_id: Id<Classe>,
    matiere_id: Id<Matiere>,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "DELETE FROM affectation_enseignement WHERE classe_id = ?1 AND matiere_id = ?2 AND etablissement_id = ?3",
    )
    .bind(classe_id.vers_chaine())
    .bind(matiere_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}
