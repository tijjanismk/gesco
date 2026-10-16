//! Factures, lignes et échéances (SPEC_PHASE_3 §4). Jamais de suppression (R-14).

use gesco_core::commun::{DateCivile, Id, Montant};
use gesco_core::core::{AnneeScolaire, MachineAEtats};
use gesco_core::finance::{
    Echeance, Facture, LigneFacture, StatutEcheance, StatutFacture, TypeFrais,
};
use gesco_core::scolarite::Inscription;
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

fn date(table: &'static str, colonne: &'static str, texte: &str) -> Resultat<DateCivile> {
    DateCivile::analyser(texte).ok_or_else(|| ErreurBase::illisible(table, colonne, texte))
}

#[derive(FromRow)]
struct LigneFactureBrute {
    id: String,
    etablissement_id: String,
    inscription_id: String,
    annee_id: String,
    grille_id: String,
    plan_paiement_id: String,
    numero: String,
    date_emission: String,
    statut: String,
    total: i64,
    motif_annulation: String,
}

impl LigneFactureBrute {
    fn vers_entite(self) -> Resultat<Facture> {
        let t = "facture";
        Ok(Facture {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            inscription_id: lire_id(t, "inscription_id", &self.inscription_id)?,
            annee_id: lire_id(t, "annee_id", &self.annee_id)?,
            grille_id: lire_id(t, "grille_id", &self.grille_id)?,
            plan_paiement_id: lire_id(t, "plan_paiement_id", &self.plan_paiement_id)?,
            date_emission: date(t, "date_emission", &self.date_emission)?,
            statut: StatutFacture::depuis_nom(&self.statut)
                .ok_or_else(|| ErreurBase::illisible(t, "statut", &self.statut))?,
            numero: self.numero,
            total: Montant::depuis_francs(self.total),
            motif_annulation: self.motif_annulation,
        })
    }
}

macro_rules! selectionner_facture {
    ($suite:literal) => {
        concat!(
            "SELECT id, etablissement_id, inscription_id, annee_id, grille_id, plan_paiement_id, numero,
                date_emission, statut, total, motif_annulation FROM facture ",
            $suite
        )
    };
}

/// Enregistre une facture.
pub async fn inserer_facture(
    cnx: &mut Connexion,
    facture: &Facture,
    contexte: &Contexte,
) -> Resultat<()> {
    if facture.etablissement_id != contexte.etablissement_id {
        return Err(ErreurBase::Cloisonnement {
            entite: "facture",
            id: facture.id.vers_chaine(),
        });
    }
    sqlx::query(
        "INSERT INTO facture (id, etablissement_id, inscription_id, annee_id, grille_id, plan_paiement_id,
            numero, date_emission, statut, total, motif_annulation,
            cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?12, ?13, ?14)",
    )
    .bind(facture.id.vers_chaine())
    .bind(facture.etablissement_id.vers_chaine())
    .bind(facture.inscription_id.vers_chaine())
    .bind(facture.annee_id.vers_chaine())
    .bind(facture.grille_id.vers_chaine())
    .bind(facture.plan_paiement_id.vers_chaine())
    .bind(&facture.numero)
    .bind(facture.date_emission.to_string())
    .bind(facture.statut.nom())
    .bind(facture.total.francs())
    .bind(&facture.motif_annulation)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Récrit l'état, le total, le plan et le motif d'annulation d'une facture.
pub async fn ecrire_facture(
    cnx: &mut Connexion,
    facture: &Facture,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE facture SET statut = ?1, total = ?2, plan_paiement_id = ?3, motif_annulation = ?4,
            mis_a_jour_le = ?5, mis_a_jour_par = ?6, identifiant_poste = ?7
         WHERE id = ?8 AND etablissement_id = ?9",
    )
    .bind(facture.statut.nom())
    .bind(facture.total.francs())
    .bind(facture.plan_paiement_id.vers_chaine())
    .bind(&facture.motif_annulation)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(facture.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Facture non annulée d'une inscription, s'il y en a une.
pub async fn facture_vivante(
    cnx: &mut Connexion,
    inscription_id: Id<Inscription>,
    contexte: &Contexte,
) -> Resultat<Option<Facture>> {
    sqlx::query_as::<_, LigneFactureBrute>(selectionner_facture!(
        "WHERE inscription_id = ?1 AND etablissement_id = ?2 AND statut <> 'annulee'"
    ))
    .bind(inscription_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .map(LigneFactureBrute::vers_entite)
    .transpose()
}

/// Factures d'une inscription, annulées comprises, de la plus ancienne à la plus récente.
pub async fn factures_de_l_inscription(
    cnx: &mut Connexion,
    inscription_id: Id<Inscription>,
    contexte: &Contexte,
) -> Resultat<Vec<Facture>> {
    sqlx::query_as::<_, LigneFactureBrute>(selectionner_facture!(
        "WHERE inscription_id = ?1 AND etablissement_id = ?2 ORDER BY cree_le, numero"
    ))
    .bind(inscription_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneFactureBrute::vers_entite)
    .collect()
}

/// Factures d'une année qui ne sont ni soldées ni annulées : les candidates aux impayés.
pub async fn factures_ouvertes(
    cnx: &mut Connexion,
    annee_id: Id<AnneeScolaire>,
    contexte: &Contexte,
) -> Resultat<Vec<Facture>> {
    sqlx::query_as::<_, LigneFactureBrute>(selectionner_facture!(
        "WHERE annee_id = ?1 AND etablissement_id = ?2 AND statut IN ('emise', 'partiellement_reglee')"
    ))
    .bind(annee_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneFactureBrute::vers_entite)
    .collect()
}

/// Inscriptions d'une année qui ont une facture non annulée.
pub async fn inscriptions_facturees(
    cnx: &mut Connexion,
    annee_id: Id<AnneeScolaire>,
    contexte: &Contexte,
) -> Resultat<Vec<Id<Inscription>>> {
    let ids: Vec<String> = sqlx::query_scalar(
        "SELECT inscription_id FROM facture WHERE annee_id = ?1 AND etablissement_id = ?2 AND statut <> 'annulee'",
    )
    .bind(annee_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?;
    ids.iter()
        .map(|i| lire_id("facture", "inscription_id", i))
        .collect()
}

/// Enregistre une ligne de facture.
pub async fn inserer_ligne(
    cnx: &mut Connexion,
    facture_id: Id<Facture>,
    ordre: usize,
    ligne: &LigneFacture,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "INSERT INTO ligne_facture (id, etablissement_id, facture_id, ordre, ligne_tarifaire_id, type_frais, libelle, montant,
            cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?9, ?10, ?11)",
    )
    .bind(ligne.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(facture_id.vers_chaine())
    .bind(i64::try_from(ordre).unwrap_or(i64::MAX))
    .bind(ligne.ligne_tarifaire_id.vers_chaine())
    .bind(ligne.type_frais.nom())
    .bind(&ligne.libelle)
    .bind(ligne.montant.francs())
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Récrit le montant d'une ligne : somme de ses échéances dues.
pub async fn ecrire_montant_ligne(
    cnx: &mut Connexion,
    ligne_id: Id<LigneFacture>,
    montant: Montant,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE ligne_facture SET montant = ?1, mis_a_jour_le = ?2, mis_a_jour_par = ?3, identifiant_poste = ?4
         WHERE id = ?5 AND etablissement_id = ?6",
    )
    .bind(montant.francs())
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(ligne_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

#[derive(FromRow)]
struct LigneLigneFacture {
    id: String,
    ligne_tarifaire_id: String,
    type_frais: String,
    libelle: String,
    montant: i64,
}

/// Lignes d'une facture, dans leur ordre.
pub async fn lignes(
    cnx: &mut Connexion,
    facture_id: Id<Facture>,
    contexte: &Contexte,
) -> Resultat<Vec<LigneFacture>> {
    sqlx::query_as::<_, LigneLigneFacture>(
        "SELECT id, ligne_tarifaire_id, type_frais, libelle, montant FROM ligne_facture
         WHERE facture_id = ?1 AND etablissement_id = ?2 ORDER BY ordre",
    )
    .bind(facture_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(|l| {
        let t = "ligne_facture";
        Ok(LigneFacture {
            id: lire_id(t, "id", &l.id)?,
            ligne_tarifaire_id: lire_id(t, "ligne_tarifaire_id", &l.ligne_tarifaire_id)?,
            type_frais: TypeFrais::depuis_nom(&l.type_frais)
                .ok_or_else(|| ErreurBase::illisible(t, "type_frais", &l.type_frais))?,
            libelle: l.libelle,
            montant: Montant::depuis_francs(l.montant),
        })
    })
    .collect()
}

/// Enregistre une échéance.
pub async fn inserer_echeance(
    cnx: &mut Connexion,
    facture_id: Id<Facture>,
    rang: usize,
    echeance: &Echeance,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "INSERT INTO echeance (id, etablissement_id, facture_id, ligne_facture_id, rang, libelle, date_echeance, montant, statut,
            cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?10, ?11, ?12)",
    )
    .bind(echeance.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(facture_id.vers_chaine())
    .bind(echeance.ligne_facture_id.vers_chaine())
    .bind(i64::try_from(rang).unwrap_or(i64::MAX))
    .bind(&echeance.libelle)
    .bind(echeance.date.to_string())
    .bind(echeance.montant.francs())
    .bind(echeance.statut.nom())
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Annule une échéance par contre-passation : elle reste, marquée annulée.
pub async fn annuler_echeance(
    cnx: &mut Connexion,
    id: Id<Echeance>,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE echeance SET statut = 'annulee', mis_a_jour_le = ?1, mis_a_jour_par = ?2, identifiant_poste = ?3
         WHERE id = ?4 AND etablissement_id = ?5",
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

#[derive(FromRow)]
struct LigneEcheance {
    id: String,
    ligne_facture_id: String,
    libelle: String,
    date_echeance: String,
    montant: i64,
    statut: String,
}

/// Échéances d'une facture, annulées comprises, par date puis par rang.
pub async fn echeances(
    cnx: &mut Connexion,
    facture_id: Id<Facture>,
    contexte: &Contexte,
) -> Resultat<Vec<Echeance>> {
    sqlx::query_as::<_, LigneEcheance>(
        "SELECT id, ligne_facture_id, libelle, date_echeance, montant, statut FROM echeance
         WHERE facture_id = ?1 AND etablissement_id = ?2 ORDER BY date_echeance, rang",
    )
    .bind(facture_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(|e| {
        let t = "echeance";
        Ok(Echeance {
            id: lire_id(t, "id", &e.id)?,
            ligne_facture_id: lire_id(t, "ligne_facture_id", &e.ligne_facture_id)?,
            date: date(t, "date_echeance", &e.date_echeance)?,
            statut: StatutEcheance::depuis_nom(&e.statut)
                .ok_or_else(|| ErreurBase::illisible(t, "statut", &e.statut))?,
            libelle: e.libelle,
            montant: Montant::depuis_francs(e.montant),
        })
    })
    .collect()
}

/// Nombre d'échéances d'une facture, annulées comprises : le rang de la suivante.
pub async fn nombre_d_echeances(
    cnx: &mut Connexion,
    facture_id: Id<Facture>,
    contexte: &Contexte,
) -> Resultat<usize> {
    let n: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM echeance WHERE facture_id = ?1 AND etablissement_id = ?2",
    )
    .bind(facture_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_one(cnx)
    .await?;
    Ok(usize::try_from(n).unwrap_or(0))
}
