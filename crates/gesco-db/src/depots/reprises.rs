//! Reprise avant GESCO (migration 0065) : ce que les parents ont payé avant l'installation, et ses
//! parts portées par les échéances. Rien ne se supprime ni ne se réécrit (R-14) : une reprise et
//! ses parts passent `annulee`, la base refuse toute suppression.

use gesco_core::commun::{DateCivile, Horodatage, Id, Montant};
use gesco_core::core::{AnneeScolaire, Utilisateur};
use gesco_core::finance::reprise::{OrigineReprise, PartReprise, RepriseAvantGesco, StatutReprise};
use gesco_core::finance::{Echeance, Facture};
use gesco_core::scolarite::Inscription;
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

#[derive(FromRow)]
struct LigneReprise {
    id: String,
    etablissement_id: String,
    inscription_id: String,
    montant: i64,
    note: String,
    origine: String,
    statut: String,
    motif_annulation: String,
    remplacee_par: Option<String>,
    cree_le: i64,
    cree_par: Option<String>,
    mis_a_jour_le: i64,
    mis_a_jour_par: Option<String>,
}

impl LigneReprise {
    fn vers_entite(self) -> Resultat<RepriseAvantGesco> {
        let t = "reprise_avant_gesco";
        let utilisateur = |colonne, valeur: Option<String>| {
            valeur
                .as_deref()
                .map(|u| lire_id::<Utilisateur>(t, colonne, u))
                .transpose()
        };
        Ok(RepriseAvantGesco {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            inscription_id: lire_id(t, "inscription_id", &self.inscription_id)?,
            montant: Montant::depuis_francs(self.montant),
            note: self.note,
            origine: OrigineReprise::depuis_nom(&self.origine)
                .ok_or_else(|| ErreurBase::illisible(t, "origine", &self.origine))?,
            statut: StatutReprise::depuis_nom(&self.statut)
                .ok_or_else(|| ErreurBase::illisible(t, "statut", &self.statut))?,
            motif_annulation: self.motif_annulation,
            remplacee_par: self
                .remplacee_par
                .as_deref()
                .map(|r| lire_id(t, "remplacee_par", r))
                .transpose()?,
            cree_le: Horodatage::depuis_millis_epoque(self.cree_le),
            cree_par: utilisateur("cree_par", self.cree_par)?,
            mis_a_jour_le: Horodatage::depuis_millis_epoque(self.mis_a_jour_le),
            mis_a_jour_par: utilisateur("mis_a_jour_par", self.mis_a_jour_par)?,
        })
    }
}

macro_rules! selectionner_reprise {
    ($suite:literal) => {
        concat!(
            "SELECT r.id, r.etablissement_id, r.inscription_id, r.montant, r.note, r.origine,
                r.statut, r.motif_annulation, r.remplacee_par, r.cree_le, r.cree_par,
                r.mis_a_jour_le, r.mis_a_jour_par
             FROM reprise_avant_gesco r ",
            $suite
        )
    };
}

/// Enregistre une reprise active.
pub async fn inserer(
    cnx: &mut Connexion,
    r: &RepriseAvantGesco,
    contexte: &Contexte,
) -> Resultat<()> {
    if r.etablissement_id != contexte.etablissement_id {
        return Err(ErreurBase::Cloisonnement {
            entite: "reprise_avant_gesco",
            id: r.id.vers_chaine(),
        });
    }
    sqlx::query(
        "INSERT INTO reprise_avant_gesco (id, etablissement_id, inscription_id, montant, note,
            origine, statut, motif_annulation, remplacee_par, cree_le, cree_par, mis_a_jour_le,
            mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, ?9, ?10, ?9, ?10, ?11)",
    )
    .bind(r.id.vers_chaine())
    .bind(r.etablissement_id.vers_chaine())
    .bind(r.inscription_id.vers_chaine())
    .bind(r.montant.francs())
    .bind(&r.note)
    .bind(r.origine.nom())
    .bind(r.statut.nom())
    .bind(&r.motif_annulation)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Annule une reprise active, avec son motif et, pour une modification, la reprise qui la
/// remplace ; ses parts actives passent `annulee` aussi.
pub async fn annuler(
    cnx: &mut Connexion,
    id: Id<RepriseAvantGesco>,
    motif: &str,
    remplacee_par: Option<Id<RepriseAvantGesco>>,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE reprise_avant_gesco SET statut = 'annulee', motif_annulation = ?1,
            remplacee_par = ?2, mis_a_jour_le = ?3, mis_a_jour_par = ?4, identifiant_poste = ?5
         WHERE id = ?6 AND etablissement_id = ?7 AND statut = 'active'",
    )
    .bind(motif)
    .bind(remplacee_par.map(|r| r.vers_chaine()))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(&mut *cnx)
    .await?;
    sqlx::query(
        "UPDATE reprise_imputation SET statut = 'annulee', mis_a_jour_le = ?1, mis_a_jour_par = ?2,
            identifiant_poste = ?3
         WHERE reprise_id = ?4 AND etablissement_id = ?5 AND statut = 'active'",
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

/// Reprise active d'une inscription, s'il y en a une.
pub async fn active(
    cnx: &mut Connexion,
    inscription_id: Id<Inscription>,
    contexte: &Contexte,
) -> Resultat<Option<RepriseAvantGesco>> {
    sqlx::query_as::<_, LigneReprise>(selectionner_reprise!(
        "WHERE r.inscription_id = ?1 AND r.etablissement_id = ?2 AND r.statut = 'active'"
    ))
    .bind(inscription_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .map(LigneReprise::vers_entite)
    .transpose()
}

/// Reprises d'une inscription, actives et annulées, de la plus récente à la plus ancienne.
pub async fn historique(
    cnx: &mut Connexion,
    inscription_id: Id<Inscription>,
    contexte: &Contexte,
) -> Resultat<Vec<RepriseAvantGesco>> {
    sqlx::query_as::<_, LigneReprise>(selectionner_reprise!(
        "WHERE r.inscription_id = ?1 AND r.etablissement_id = ?2
         ORDER BY r.cree_le DESC, r.rowid DESC"
    ))
    .bind(inscription_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneReprise::vers_entite)
    .collect()
}

/// Reprises actives des inscriptions d'une année.
pub async fn actives_de_l_annee(
    cnx: &mut Connexion,
    annee_id: Id<AnneeScolaire>,
    contexte: &Contexte,
) -> Resultat<Vec<RepriseAvantGesco>> {
    sqlx::query_as::<_, LigneReprise>(selectionner_reprise!(
        "JOIN inscription i ON i.id = r.inscription_id
         WHERE i.annee_id = ?1 AND r.etablissement_id = ?2 AND r.statut = 'active'"
    ))
    .bind(annee_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneReprise::vers_entite)
    .collect()
}

/// Parts actives de la reprise sur les échéances d'une facture.
pub async fn imputations_de_la_facture(
    cnx: &mut Connexion,
    facture_id: Id<Facture>,
    contexte: &Contexte,
) -> Resultat<Vec<PartReprise>> {
    let lignes: Vec<(String, i64)> = sqlx::query_as(
        "SELECT p.echeance_id, p.montant FROM reprise_imputation p
         JOIN echeance e ON e.id = p.echeance_id
         WHERE e.facture_id = ?1 AND p.etablissement_id = ?2 AND p.statut = 'active'
            AND e.statut = 'due'
         ORDER BY e.date_echeance, e.rang",
    )
    .bind(facture_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?;
    lignes
        .into_iter()
        .map(|(e, m)| {
            Ok(PartReprise {
                echeance_id: lire_id("reprise_imputation", "echeance_id", &e)?,
                montant: Montant::depuis_francs(m),
            })
        })
        .collect()
}

/// Parts actives d'une reprise, toutes échéances (même contre-passées), par date puis rang.
pub async fn parts_actives(
    cnx: &mut Connexion,
    reprise_id: Id<RepriseAvantGesco>,
    contexte: &Contexte,
) -> Resultat<Vec<PartReprise>> {
    let lignes: Vec<(String, i64)> = sqlx::query_as(
        "SELECT p.echeance_id, p.montant FROM reprise_imputation p
         JOIN echeance e ON e.id = p.echeance_id
         WHERE p.reprise_id = ?1 AND p.etablissement_id = ?2 AND p.statut = 'active'
         ORDER BY e.date_echeance, e.rang",
    )
    .bind(reprise_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?;
    lignes
        .into_iter()
        .map(|(e, m)| {
            Ok(PartReprise {
                echeance_id: lire_id("reprise_imputation", "echeance_id", &e)?,
                montant: Montant::depuis_francs(m),
            })
        })
        .collect()
}

/// Enregistre une part active de la reprise sur une échéance.
pub async fn inserer_imputation(
    cnx: &mut Connexion,
    reprise_id: Id<RepriseAvantGesco>,
    echeance_id: Id<Echeance>,
    montant: Montant,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "INSERT INTO reprise_imputation (id, etablissement_id, reprise_id, echeance_id, montant,
            statut, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, 'active', ?6, ?7, ?6, ?7, ?8)",
    )
    .bind(crate::nouvel_id::<()>().vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(reprise_id.vers_chaine())
    .bind(echeance_id.vers_chaine())
    .bind(montant.francs())
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Annule les parts actives d'une reprise (avant de les recalculer).
pub async fn annuler_imputations(
    cnx: &mut Connexion,
    reprise_id: Id<RepriseAvantGesco>,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE reprise_imputation SET statut = 'annulee', mis_a_jour_le = ?1, mis_a_jour_par = ?2,
            identifiant_poste = ?3
         WHERE reprise_id = ?4 AND etablissement_id = ?5 AND statut = 'active'",
    )
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(reprise_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Remboursé au parent : somme des dépenses payées liées à l'inscription (une dépense annulée
/// n'est plus `payee`).
pub async fn rembourse(
    cnx: &mut Connexion,
    inscription_id: Id<Inscription>,
    contexte: &Contexte,
) -> Resultat<Montant> {
    let total: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(montant), 0) FROM depense
         WHERE inscription_id = ?1 AND etablissement_id = ?2 AND statut = 'payee'",
    )
    .bind(inscription_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_one(cnx)
    .await?;
    Ok(Montant::depuis_francs(total))
}

/// Remboursements engagés : dépenses liées non annulées ni refusées (en attente d'accord,
/// accordées, payées). Sert à refuser une dépense qui rembourserait deux fois.
pub async fn remboursements_engages(
    cnx: &mut Connexion,
    inscription_id: Id<Inscription>,
    contexte: &Contexte,
) -> Resultat<Montant> {
    let total: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(montant), 0) FROM depense
         WHERE inscription_id = ?1 AND etablissement_id = ?2
           AND statut IN ('a_approuver', 'approuvee', 'payee')",
    )
    .bind(inscription_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_one(cnx)
    .await?;
    Ok(Montant::depuis_francs(total))
}

/// Jour d'ouverture de la première session de caisse ouverte entre deux instants (l'année
/// scolaire) ; `None` sans caisse.
pub async fn premiere_caisse(
    cnx: &mut Connexion,
    debut: Horodatage,
    fin: Horodatage,
    contexte: &Contexte,
) -> Resultat<Option<Horodatage>> {
    let le: Option<i64> = sqlx::query_scalar(
        "SELECT MIN(ouverte_le) FROM session_caisse
         WHERE etablissement_id = ?1 AND ouverte_le >= ?2 AND ouverte_le <= ?3",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(debut.millis_epoque())
    .bind(fin.millis_epoque())
    .fetch_one(cnx)
    .await?;
    Ok(le.map(Horodatage::depuis_millis_epoque))
}

/// Dépenses de remboursement d'une inscription, la plus ancienne d'abord : (jour, montant,
/// statut, mode).
pub async fn remboursements(
    cnx: &mut Connexion,
    inscription_id: Id<Inscription>,
    contexte: &Contexte,
) -> Resultat<Vec<(DateCivile, Montant, String, Option<String>)>> {
    let lignes: Vec<(String, i64, String, Option<String>)> = sqlx::query_as(
        "SELECT date, montant, statut, mode FROM depense
         WHERE inscription_id = ?1 AND etablissement_id = ?2
           AND statut IN ('a_approuver', 'approuvee', 'payee')
         ORDER BY date, cree_le",
    )
    .bind(inscription_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?;
    lignes
        .into_iter()
        .map(|(date, montant, statut, mode)| {
            Ok((
                DateCivile::analyser(&date)
                    .ok_or_else(|| ErreurBase::illisible("depense", "date", &date))?,
                Montant::depuis_francs(montant),
                statut,
                mode,
            ))
        })
        .collect()
}
