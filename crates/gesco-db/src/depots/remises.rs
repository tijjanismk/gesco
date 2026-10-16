//! Remises et leur part sur chaque échéance (SPEC_PHASE_3 §5). Jamais de suppression (R-14).

use gesco_core::commun::{DateCivile, Id, Montant};
use gesco_core::core::{MachineAEtats, Utilisateur};
use gesco_core::finance::{Echeance, Remise, StatutRemise, TypeFrais, TypeRemise, ValeurRemise};
use gesco_core::scolarite::Inscription;
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

#[derive(FromRow)]
struct LigneRemise {
    id: String,
    etablissement_id: String,
    inscription_id: String,
    type_remise: String,
    nature_valeur: String,
    valeur: i64,
    type_frais: String,
    motif: String,
    statut: String,
    proposee_par: String,
    proposee_le: String,
    decidee_par: Option<String>,
    motif_decision: String,
    montant_applique: i64,
}

impl LigneRemise {
    fn vers_entite(self) -> Resultat<Remise> {
        let t = "reduction";
        let illisible = |c: &'static str, v: &str| ErreurBase::illisible(t, c, v);
        Ok(Remise {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            inscription_id: lire_id(t, "inscription_id", &self.inscription_id)?,
            type_remise: TypeRemise::depuis_nom(&self.type_remise)
                .ok_or_else(|| illisible("type_remise", &self.type_remise))?,
            valeur: match self.nature_valeur.as_str() {
                "taux" => ValeurRemise::Taux {
                    pour_cent: self.valeur,
                },
                "montant" => ValeurRemise::Montant {
                    montant: Montant::depuis_francs(self.valeur),
                },
                autre => return Err(illisible("nature_valeur", autre)),
            },
            type_frais: TypeFrais::depuis_nom(&self.type_frais)
                .ok_or_else(|| illisible("type_frais", &self.type_frais))?,
            statut: StatutRemise::depuis_nom(&self.statut)
                .ok_or_else(|| illisible("statut", &self.statut))?,
            proposee_par: lire_id(t, "proposee_par", &self.proposee_par)?,
            proposee_le: DateCivile::analyser(&self.proposee_le)
                .ok_or_else(|| illisible("proposee_le", &self.proposee_le))?,
            decidee_par: self
                .decidee_par
                .as_deref()
                .map(|d| lire_id::<Utilisateur>(t, "decidee_par", d))
                .transpose()?,
            motif: self.motif,
            motif_decision: self.motif_decision,
            montant_applique: Montant::depuis_francs(self.montant_applique),
        })
    }
}

macro_rules! selectionner_remise {
    ($suite:literal) => {
        concat!(
            "SELECT id, etablissement_id, inscription_id, type_remise, nature_valeur, valeur, type_frais, motif,
                statut, proposee_par, proposee_le, decidee_par, motif_decision, montant_applique FROM reduction ",
            $suite
        )
    };
}

fn valeur(remise: &Remise) -> (&'static str, i64) {
    match remise.valeur {
        ValeurRemise::Taux { pour_cent } => ("taux", pour_cent),
        ValeurRemise::Montant { montant } => ("montant", montant.francs()),
    }
}

/// Enregistre une remise proposée.
pub async fn inserer(cnx: &mut Connexion, remise: &Remise, contexte: &Contexte) -> Resultat<()> {
    if remise.etablissement_id != contexte.etablissement_id {
        return Err(ErreurBase::Cloisonnement {
            entite: "reduction",
            id: remise.id.vers_chaine(),
        });
    }
    let (nature, valeur) = valeur(remise);
    sqlx::query(
        "INSERT INTO reduction (id, etablissement_id, inscription_id, type_remise, nature_valeur, valeur, type_frais, motif,
            statut, proposee_par, proposee_le, decidee_par, motif_decision, montant_applique,
            cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?15, ?16, ?17)",
    )
    .bind(remise.id.vers_chaine())
    .bind(remise.etablissement_id.vers_chaine())
    .bind(remise.inscription_id.vers_chaine())
    .bind(remise.type_remise.nom())
    .bind(nature)
    .bind(valeur)
    .bind(remise.type_frais.nom())
    .bind(&remise.motif)
    .bind(remise.statut.nom())
    .bind(remise.proposee_par.vers_chaine())
    .bind(remise.proposee_le.to_string())
    .bind(remise.decidee_par.map(|d| d.vers_chaine()))
    .bind(&remise.motif_decision)
    .bind(remise.montant_applique.francs())
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Récrit l'état, la décision et le montant appliqué d'une remise.
pub async fn ecrire(cnx: &mut Connexion, remise: &Remise, contexte: &Contexte) -> Resultat<()> {
    sqlx::query(
        "UPDATE reduction SET statut = ?1, decidee_par = ?2, motif_decision = ?3, montant_applique = ?4,
            mis_a_jour_le = ?5, mis_a_jour_par = ?6, identifiant_poste = ?7
         WHERE id = ?8 AND etablissement_id = ?9",
    )
    .bind(remise.statut.nom())
    .bind(remise.decidee_par.map(|d| d.vers_chaine()))
    .bind(&remise.motif_decision)
    .bind(remise.montant_applique.francs())
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(remise.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Lit une remise.
pub async fn lire(cnx: &mut Connexion, id: Id<Remise>, contexte: &Contexte) -> Resultat<Remise> {
    sqlx::query_as::<_, LigneRemise>(selectionner_remise!(
        "WHERE id = ?1 AND etablissement_id = ?2"
    ))
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .ok_or_else(|| ErreurBase::Introuvable {
        entite: "reduction",
        id: id.vers_chaine(),
    })?
    .vers_entite()
}

/// Remises d'une inscription, de la plus ancienne à la plus récente.
pub async fn de_l_inscription(
    cnx: &mut Connexion,
    inscription_id: Id<Inscription>,
    contexte: &Contexte,
) -> Resultat<Vec<Remise>> {
    sqlx::query_as::<_, LigneRemise>(selectionner_remise!(
        "WHERE inscription_id = ?1 AND etablissement_id = ?2 ORDER BY cree_le"
    ))
    .bind(inscription_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneRemise::vers_entite)
    .collect()
}

/// Remises proposées de l'établissement, en attente de la direction.
pub async fn proposees(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<Remise>> {
    sqlx::query_as::<_, LigneRemise>(selectionner_remise!(
        "WHERE etablissement_id = ?1 AND statut = 'proposee' ORDER BY cree_le"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneRemise::vers_entite)
    .collect()
}

/// Enregistre la part d'une remise sur une échéance.
pub async fn inserer_part(
    cnx: &mut Connexion,
    remise_id: Id<Remise>,
    echeance_id: Id<Echeance>,
    montant: Montant,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "INSERT INTO reduction_echeance (id, etablissement_id, reduction_id, echeance_id, montant, statut,
            cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, 'active', ?6, ?7, ?6, ?7, ?8)",
    )
    .bind(crate::nouvel_id::<()>().vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(remise_id.vers_chaine())
    .bind(echeance_id.vers_chaine())
    .bind(montant.francs())
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Contre-passe les parts actives d'une remise, toutes ou seulement celles d'une échéance.
pub async fn annuler_parts(
    cnx: &mut Connexion,
    remise_id: Id<Remise>,
    echeance_id: Option<Id<Echeance>>,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE reduction_echeance SET statut = 'annulee', mis_a_jour_le = ?1, mis_a_jour_par = ?2, identifiant_poste = ?3
         WHERE reduction_id = ?4 AND etablissement_id = ?5 AND statut = 'active'
           AND (?6 IS NULL OR echeance_id = ?6)",
    )
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(remise_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(echeance_id.map(|e| e.vers_chaine()))
    .execute(cnx)
    .await?;
    Ok(())
}

#[derive(FromRow)]
struct LignePart {
    reduction_id: String,
    echeance_id: String,
    montant: i64,
}

/// Parts actives des remises sur les échéances d'une facture : (remise, échéance, montant).
pub async fn parts_de_la_facture(
    cnx: &mut Connexion,
    facture_id: Id<gesco_core::finance::Facture>,
    contexte: &Contexte,
) -> Resultat<Vec<(Id<Remise>, Id<Echeance>, Montant)>> {
    sqlx::query_as::<_, LignePart>(
        "SELECT p.reduction_id, p.echeance_id, p.montant FROM reduction_echeance p
         JOIN echeance e ON e.id = p.echeance_id
         WHERE e.facture_id = ?1 AND p.etablissement_id = ?2 AND p.statut = 'active'",
    )
    .bind(facture_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(|p| {
        Ok((
            lire_id("reduction_echeance", "reduction_id", &p.reduction_id)?,
            lire_id("reduction_echeance", "echeance_id", &p.echeance_id)?,
            Montant::depuis_francs(p.montant),
        ))
    })
    .collect()
}
