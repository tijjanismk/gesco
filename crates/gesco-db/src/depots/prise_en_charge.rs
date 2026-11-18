//! Prise en charge par l'État (migration 0066) : versions de la prise en charge d'une
//! inscription, montant attendu par niveau, versements de l'État et leurs parts. Rien ne se
//! supprime ni ne se réécrit (R-14) : une version passe `remplacee`, un versement `annule`, une
//! part `annulee` ; la base refuse toute suppression. Le partage des factures ne s'écrit nulle
//! part : il se calcule (`gesco_core::finance::prise_en_charge::partager`).

use gesco_core::commun::{DateCivile, Horodatage, Id, Montant};
use gesco_core::core::{AnneeScolaire, Utilisateur};
use gesco_core::finance::prise_en_charge::{
    MontantEtat, OriginePartEtat, PartVersementEtat, PriseEnChargeEtat, StatutPartEtat,
    StatutPriseEnCharge, StatutVersementEtat, VersementEtat,
};
use gesco_core::scolarite::{Inscription, Niveau, Serie};
use gesco_core::tresorerie::recette::Recette;
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::depots::booleen;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

fn utilisateur(
    table: &'static str,
    colonne: &'static str,
    valeur: Option<&str>,
) -> Resultat<Option<Id<Utilisateur>>> {
    valeur.map(|u| lire_id(table, colonne, u)).transpose()
}

// ---------------------------------------------------------------------------------------------
// Versions de la prise en charge
// ---------------------------------------------------------------------------------------------

#[derive(FromRow)]
struct LignePriseEnCharge {
    id: String,
    etablissement_id: String,
    inscription_id: String,
    statut: String,
    montant_attendu: i64,
    abandonne: i64,
    reference: String,
    motif: String,
    remplace_id: Option<String>,
    cree_le: i64,
    cree_par: Option<String>,
    mis_a_jour_le: i64,
    mis_a_jour_par: Option<String>,
}

impl LignePriseEnCharge {
    fn vers_entite(self) -> Resultat<PriseEnChargeEtat> {
        let t = "prise_en_charge_etat";
        Ok(PriseEnChargeEtat {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            inscription_id: lire_id(t, "inscription_id", &self.inscription_id)?,
            statut: StatutPriseEnCharge::depuis_nom(&self.statut)
                .ok_or_else(|| ErreurBase::illisible(t, "statut", &self.statut))?,
            montant_attendu: Montant::depuis_francs(self.montant_attendu),
            abandonne: Montant::depuis_francs(self.abandonne),
            reference: self.reference,
            motif: self.motif,
            remplace_id: self
                .remplace_id
                .as_deref()
                .map(|r| lire_id(t, "remplace_id", r))
                .transpose()?,
            cree_le: Horodatage::depuis_millis_epoque(self.cree_le),
            cree_par: utilisateur(t, "cree_par", self.cree_par.as_deref())?,
            mis_a_jour_le: Horodatage::depuis_millis_epoque(self.mis_a_jour_le),
            mis_a_jour_par: utilisateur(t, "mis_a_jour_par", self.mis_a_jour_par.as_deref())?,
        })
    }
}

macro_rules! selectionner_prise {
    ($suite:literal) => {
        concat!(
            "SELECT p.id, p.etablissement_id, p.inscription_id, p.statut, p.montant_attendu,
                p.abandonne, p.reference, p.motif, p.remplace_id, p.cree_le, p.cree_par,
                p.mis_a_jour_le, p.mis_a_jour_par
             FROM prise_en_charge_etat p ",
            $suite
        )
    };
}

/// Version courante (en cours ou terminée) de la prise en charge d'une inscription.
pub async fn courante(
    cnx: &mut Connexion,
    inscription_id: Id<Inscription>,
    contexte: &Contexte,
) -> Resultat<Option<PriseEnChargeEtat>> {
    sqlx::query_as::<_, LignePriseEnCharge>(selectionner_prise!(
        "WHERE p.inscription_id = ?1 AND p.etablissement_id = ?2 AND p.statut <> 'remplacee'"
    ))
    .bind(inscription_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .map(LignePriseEnCharge::vers_entite)
    .transpose()
}

/// Versions courantes des inscriptions d'une année.
pub async fn courantes_de_l_annee(
    cnx: &mut Connexion,
    annee_id: Id<AnneeScolaire>,
    contexte: &Contexte,
) -> Resultat<Vec<PriseEnChargeEtat>> {
    sqlx::query_as::<_, LignePriseEnCharge>(selectionner_prise!(
        "JOIN inscription i ON i.id = p.inscription_id
         WHERE i.annee_id = ?1 AND p.etablissement_id = ?2 AND p.statut <> 'remplacee'"
    ))
    .bind(annee_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LignePriseEnCharge::vers_entite)
    .collect()
}

/// Versions d'une inscription, de la plus récente à la plus ancienne.
pub async fn historique(
    cnx: &mut Connexion,
    inscription_id: Id<Inscription>,
    contexte: &Contexte,
) -> Resultat<Vec<PriseEnChargeEtat>> {
    sqlx::query_as::<_, LignePriseEnCharge>(selectionner_prise!(
        "WHERE p.inscription_id = ?1 AND p.etablissement_id = ?2
         ORDER BY p.cree_le DESC, p.rowid DESC"
    ))
    .bind(inscription_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LignePriseEnCharge::vers_entite)
    .collect()
}

/// Enregistre une nouvelle version courante (`en_cours` ou `terminee`) : la version courante
/// d'avant, s'il y en a une, passe `remplacee` et la nouvelle la désigne (`remplace_id`). Rend la
/// version écrite. À appeler dans la transaction du geste.
pub async fn inserer_version(
    cnx: &mut Connexion,
    version: &PriseEnChargeEtat,
    contexte: &Contexte,
) -> Resultat<PriseEnChargeEtat> {
    if version.etablissement_id != contexte.etablissement_id {
        return Err(ErreurBase::Cloisonnement {
            entite: "prise_en_charge_etat",
            id: version.id.vers_chaine(),
        });
    }
    let avant = courante(&mut *cnx, version.inscription_id, contexte).await?;
    if let Some(avant) = &avant {
        sqlx::query(
            "UPDATE prise_en_charge_etat SET statut = 'remplacee', mis_a_jour_le = ?1,
                mis_a_jour_par = ?2, identifiant_poste = ?3
             WHERE id = ?4 AND etablissement_id = ?5 AND statut <> 'remplacee'",
        )
        .bind(contexte.maintenant.millis_epoque())
        .bind(contexte.auteur())
        .bind(&contexte.poste.0)
        .bind(avant.id.vers_chaine())
        .bind(contexte.etablissement_id.vers_chaine())
        .execute(&mut *cnx)
        .await?;
    }
    let ecrite = PriseEnChargeEtat {
        remplace_id: avant.map(|a| a.id),
        cree_le: contexte.maintenant,
        cree_par: contexte.utilisateur_id,
        mis_a_jour_le: contexte.maintenant,
        mis_a_jour_par: contexte.utilisateur_id,
        ..version.clone()
    };
    sqlx::query(
        "INSERT INTO prise_en_charge_etat (id, etablissement_id, inscription_id, statut,
            montant_attendu, abandonne, reference, motif, remplace_id, cree_le, cree_par,
            mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?10, ?11, ?12)",
    )
    .bind(ecrite.id.vers_chaine())
    .bind(ecrite.etablissement_id.vers_chaine())
    .bind(ecrite.inscription_id.vers_chaine())
    .bind(ecrite.statut.nom())
    .bind(ecrite.montant_attendu.francs())
    .bind(ecrite.abandonne.francs())
    .bind(&ecrite.reference)
    .bind(&ecrite.motif)
    .bind(ecrite.remplace_id.map(|r| r.vers_chaine()))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(ecrite)
}

/// Écrit la case `inscription.prise_en_charge_etat`, lue par les rapports officiels : vraie
/// exactement quand la version courante est `en_cours`. Même requête que la grille « Pour les
/// rapports ».
pub async fn ecrire_case(
    cnx: &mut Connexion,
    inscription_id: Id<Inscription>,
    prise: bool,
    contexte: &Contexte,
) -> Resultat<()> {
    crate::depots::complements::ecrire_prise_en_charge(cnx, inscription_id, prise, contexte).await
}

// ---------------------------------------------------------------------------------------------
// Montant attendu par niveau et série
// ---------------------------------------------------------------------------------------------

#[derive(FromRow)]
struct LigneMontantEtat {
    id: String,
    etablissement_id: String,
    annee_id: String,
    niveau_id: String,
    serie_id: Option<String>,
    montant: Option<i64>,
}

impl LigneMontantEtat {
    fn vers_entite(self) -> Resultat<MontantEtat> {
        let t = "montant_etat";
        Ok(MontantEtat {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            annee_id: lire_id(t, "annee_id", &self.annee_id)?,
            niveau_id: lire_id(t, "niveau_id", &self.niveau_id)?,
            serie_id: self
                .serie_id
                .as_deref()
                .map(|s| lire_id(t, "serie_id", s))
                .transpose()?,
            montant: self.montant.map(Montant::depuis_francs),
        })
    }
}

/// Montants attendus de l'État d'une année (lignes vides comprises).
pub async fn montants_etat(
    cnx: &mut Connexion,
    annee_id: Id<AnneeScolaire>,
    contexte: &Contexte,
) -> Resultat<Vec<MontantEtat>> {
    sqlx::query_as::<_, LigneMontantEtat>(
        "SELECT id, etablissement_id, annee_id, niveau_id, serie_id, montant FROM montant_etat
         WHERE annee_id = ?1 AND etablissement_id = ?2 ORDER BY niveau_id, serie_id",
    )
    .bind(annee_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneMontantEtat::vers_entite)
    .collect()
}

/// Montant attendu réglé pour un niveau et une série : la ligne de la série, à défaut celle du
/// niveau sans série ; `None` si rien n'est réglé (les frais de l'école).
pub async fn montant_etat(
    cnx: &mut Connexion,
    annee_id: Id<AnneeScolaire>,
    niveau_id: Id<Niveau>,
    serie_id: Option<Id<Serie>>,
    contexte: &Contexte,
) -> Resultat<Option<Montant>> {
    let montant = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT montant FROM montant_etat
         WHERE annee_id = ?1 AND niveau_id = ?2 AND etablissement_id = ?3
           AND montant IS NOT NULL AND (serie_id = ?4 OR serie_id IS NULL)
         ORDER BY serie_id IS NULL LIMIT 1",
    )
    .bind(annee_id.vers_chaine())
    .bind(niveau_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(serie_id.map(|s| s.vers_chaine()))
    .fetch_optional(cnx)
    .await?
    .flatten();
    Ok(montant.map(Montant::depuis_francs))
}

/// Règle le montant attendu d'une ligne (année, niveau, série) : met à jour la ligne qui existe,
/// sinon la crée. Un montant vide garde la ligne, vide.
pub async fn regler_montant_etat(
    cnx: &mut Connexion,
    m: &MontantEtat,
    contexte: &Contexte,
) -> Resultat<()> {
    if m.etablissement_id != contexte.etablissement_id {
        return Err(ErreurBase::Cloisonnement {
            entite: "montant_etat",
            id: m.id.vers_chaine(),
        });
    }
    let modifiees = sqlx::query(
        "UPDATE montant_etat SET montant = ?1, mis_a_jour_le = ?2, mis_a_jour_par = ?3,
            identifiant_poste = ?4
         WHERE etablissement_id = ?5 AND annee_id = ?6 AND niveau_id = ?7
           AND COALESCE(serie_id, '') = COALESCE(?8, '')",
    )
    .bind(m.montant.map(|v| v.francs()))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(m.annee_id.vers_chaine())
    .bind(m.niveau_id.vers_chaine())
    .bind(m.serie_id.map(|s| s.vers_chaine()))
    .execute(&mut *cnx)
    .await?
    .rows_affected();
    if modifiees > 0 {
        return Ok(());
    }
    sqlx::query(
        "INSERT INTO montant_etat (id, etablissement_id, annee_id, niveau_id, serie_id, montant,
            cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?7, ?8, ?9)",
    )
    .bind(m.id.vers_chaine())
    .bind(m.etablissement_id.vers_chaine())
    .bind(m.annee_id.vers_chaine())
    .bind(m.niveau_id.vers_chaine())
    .bind(m.serie_id.map(|s| s.vers_chaine()))
    .bind(m.montant.map(|v| v.francs()))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Versements de l'État
// ---------------------------------------------------------------------------------------------

#[derive(FromRow)]
struct LigneVersementEtat {
    id: String,
    etablissement_id: String,
    annee_id: String,
    date: String,
    montant: i64,
    reference: String,
    libelle: String,
    payeur: String,
    recette_id: Option<String>,
    avant_gesco: i64,
    statut: String,
    motif_annulation: String,
    cree_le: i64,
    cree_par: Option<String>,
    mis_a_jour_le: i64,
    mis_a_jour_par: Option<String>,
}

impl LigneVersementEtat {
    fn vers_entite(self) -> Resultat<VersementEtat> {
        let t = "versement_etat";
        Ok(VersementEtat {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            annee_id: lire_id(t, "annee_id", &self.annee_id)?,
            date: DateCivile::analyser(&self.date)
                .ok_or_else(|| ErreurBase::illisible(t, "date", &self.date))?,
            montant: Montant::depuis_francs(self.montant),
            reference: self.reference,
            libelle: self.libelle,
            payeur: self.payeur,
            recette_id: self
                .recette_id
                .as_deref()
                .map(|r| lire_id(t, "recette_id", r))
                .transpose()?,
            avant_gesco: booleen(self.avant_gesco),
            statut: StatutVersementEtat::depuis_nom(&self.statut)
                .ok_or_else(|| ErreurBase::illisible(t, "statut", &self.statut))?,
            motif_annulation: self.motif_annulation,
            cree_le: Horodatage::depuis_millis_epoque(self.cree_le),
            cree_par: utilisateur(t, "cree_par", self.cree_par.as_deref())?,
            mis_a_jour_le: Horodatage::depuis_millis_epoque(self.mis_a_jour_le),
            mis_a_jour_par: utilisateur(t, "mis_a_jour_par", self.mis_a_jour_par.as_deref())?,
        })
    }
}

macro_rules! selectionner_versement {
    ($suite:literal) => {
        concat!(
            "SELECT v.id, v.etablissement_id, v.annee_id, v.date, v.montant, v.reference,
                v.libelle, v.payeur, v.recette_id, v.avant_gesco, v.statut, v.motif_annulation,
                v.cree_le, v.cree_par, v.mis_a_jour_le, v.mis_a_jour_par
             FROM versement_etat v ",
            $suite
        )
    };
}

/// Enregistre un versement de l'État (valide).
pub async fn inserer_versement(
    cnx: &mut Connexion,
    v: &VersementEtat,
    contexte: &Contexte,
) -> Resultat<()> {
    if v.etablissement_id != contexte.etablissement_id {
        return Err(ErreurBase::Cloisonnement {
            entite: "versement_etat",
            id: v.id.vers_chaine(),
        });
    }
    sqlx::query(
        "INSERT INTO versement_etat (id, etablissement_id, annee_id, date, montant, reference,
            libelle, payeur, recette_id, avant_gesco, statut, motif_annulation, cree_le, cree_par,
            mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?13, ?14, ?15)",
    )
    .bind(v.id.vers_chaine())
    .bind(v.etablissement_id.vers_chaine())
    .bind(v.annee_id.vers_chaine())
    .bind(v.date.to_string())
    .bind(v.montant.francs())
    .bind(&v.reference)
    .bind(&v.libelle)
    .bind(&v.payeur)
    .bind(v.recette_id.map(|r| r.vers_chaine()))
    .bind(i64::from(v.avant_gesco))
    .bind(v.statut.nom())
    .bind(&v.motif_annulation)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Un versement de l'État par son identifiant.
pub async fn versement(
    cnx: &mut Connexion,
    id: Id<VersementEtat>,
    contexte: &Contexte,
) -> Resultat<Option<VersementEtat>> {
    sqlx::query_as::<_, LigneVersementEtat>(selectionner_versement!(
        "WHERE v.id = ?1 AND v.etablissement_id = ?2"
    ))
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .map(LigneVersementEtat::vers_entite)
    .transpose()
}

/// Le versement de l'État que porte une recette, s'il y en a un (annulation de la recette
/// depuis la trésorerie).
pub async fn versement_de_la_recette(
    cnx: &mut Connexion,
    recette_id: Id<Recette>,
    contexte: &Contexte,
) -> Resultat<Option<VersementEtat>> {
    sqlx::query_as::<_, LigneVersementEtat>(selectionner_versement!(
        "WHERE v.recette_id = ?1 AND v.etablissement_id = ?2"
    ))
    .bind(recette_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .map(LigneVersementEtat::vers_entite)
    .transpose()
}

/// Versements de l'État d'une année, valides et annulés, par date de réception.
pub async fn versements_de_l_annee(
    cnx: &mut Connexion,
    annee_id: Id<AnneeScolaire>,
    contexte: &Contexte,
) -> Resultat<Vec<VersementEtat>> {
    sqlx::query_as::<_, LigneVersementEtat>(selectionner_versement!(
        "WHERE v.annee_id = ?1 AND v.etablissement_id = ?2 ORDER BY v.date, v.cree_le"
    ))
    .bind(annee_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneVersementEtat::vers_entite)
    .collect()
}

/// Annule un versement valide, avec son motif ; ses parts actives passent `annulee`. La recette
/// s'annule par le geste des recettes, dans la même transaction.
pub async fn annuler_versement(
    cnx: &mut Connexion,
    id: Id<VersementEtat>,
    motif: &str,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE versement_etat SET statut = 'annule', motif_annulation = ?1, mis_a_jour_le = ?2,
            mis_a_jour_par = ?3, identifiant_poste = ?4
         WHERE id = ?5 AND etablissement_id = ?6 AND statut = 'valide'",
    )
    .bind(motif)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(&mut *cnx)
    .await?;
    annuler_parts(cnx, id, contexte).await
}

// ---------------------------------------------------------------------------------------------
// Parts des versements de l'État
// ---------------------------------------------------------------------------------------------

#[derive(FromRow)]
struct LignePart {
    id: String,
    versement_etat_id: String,
    inscription_id: String,
    montant: i64,
    origine: String,
    statut: String,
}

impl LignePart {
    fn vers_entite(self) -> Resultat<PartVersementEtat> {
        let t = "versement_etat_part";
        Ok(PartVersementEtat {
            id: lire_id(t, "id", &self.id)?,
            versement_etat_id: lire_id(t, "versement_etat_id", &self.versement_etat_id)?,
            inscription_id: lire_id(t, "inscription_id", &self.inscription_id)?,
            montant: Montant::depuis_francs(self.montant),
            origine: OriginePartEtat::depuis_nom(&self.origine)
                .ok_or_else(|| ErreurBase::illisible(t, "origine", &self.origine))?,
            statut: StatutPartEtat::depuis_nom(&self.statut)
                .ok_or_else(|| ErreurBase::illisible(t, "statut", &self.statut))?,
        })
    }
}

/// Enregistre une part active d'un versement sur une inscription.
pub async fn inserer_part(
    cnx: &mut Connexion,
    versement_etat_id: Id<VersementEtat>,
    inscription_id: Id<Inscription>,
    montant: Montant,
    origine: OriginePartEtat,
    contexte: &Contexte,
) -> Resultat<Id<PartVersementEtat>> {
    let id = crate::nouvel_id::<PartVersementEtat>();
    sqlx::query(
        "INSERT INTO versement_etat_part (id, etablissement_id, versement_etat_id, inscription_id,
            montant, origine, statut, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par,
            identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'active', ?7, ?8, ?7, ?8, ?9)",
    )
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(versement_etat_id.vers_chaine())
    .bind(inscription_id.vers_chaine())
    .bind(montant.francs())
    .bind(origine.nom())
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(id)
}

/// Parts actives d'un versement, dans l'ordre de leur saisie.
pub async fn parts_du_versement(
    cnx: &mut Connexion,
    versement_etat_id: Id<VersementEtat>,
    contexte: &Contexte,
) -> Resultat<Vec<PartVersementEtat>> {
    sqlx::query_as::<_, LignePart>(
        "SELECT id, versement_etat_id, inscription_id, montant, origine, statut
         FROM versement_etat_part
         WHERE versement_etat_id = ?1 AND etablissement_id = ?2 AND statut = 'active'
         ORDER BY cree_le, rowid",
    )
    .bind(versement_etat_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LignePart::vers_entite)
    .collect()
}

/// Annule les parts actives d'un versement (avant une nouvelle répartition, ou avec lui).
pub async fn annuler_parts(
    cnx: &mut Connexion,
    versement_etat_id: Id<VersementEtat>,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE versement_etat_part SET statut = 'annulee', mis_a_jour_le = ?1,
            mis_a_jour_par = ?2, identifiant_poste = ?3
         WHERE versement_etat_id = ?4 AND etablissement_id = ?5 AND statut = 'active'",
    )
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(versement_etat_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Ce que l'État a versé pour une inscription, versement par versement (fiche de l'élève).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartRecue {
    /// Versement.
    pub versement_etat_id: Id<VersementEtat>,
    /// Jour de réception du versement.
    pub date: DateCivile,
    /// Référence du versement.
    pub reference: String,
    /// Part attribuée à l'inscription.
    pub montant: Montant,
}

/// Parts actives des versements valides d'une inscription, par date de réception.
pub async fn parts_recues(
    cnx: &mut Connexion,
    inscription_id: Id<Inscription>,
    contexte: &Contexte,
) -> Resultat<Vec<PartRecue>> {
    let lignes: Vec<(String, String, String, i64)> = sqlx::query_as(
        "SELECT v.id, v.date, v.reference, p.montant FROM versement_etat_part p
         JOIN versement_etat v ON v.id = p.versement_etat_id
         WHERE p.inscription_id = ?1 AND p.etablissement_id = ?2 AND p.statut = 'active'
           AND v.statut = 'valide'
         ORDER BY v.date, p.cree_le",
    )
    .bind(inscription_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?;
    lignes
        .into_iter()
        .map(|(id, date, reference, montant)| {
            Ok(PartRecue {
                versement_etat_id: lire_id("versement_etat", "id", &id)?,
                date: DateCivile::analyser(&date)
                    .ok_or_else(|| ErreurBase::illisible("versement_etat", "date", &date))?,
                reference,
                montant: Montant::depuis_francs(montant),
            })
        })
        .collect()
}

/// `S` : ce que l'État a payé pour une inscription (parts actives des versements valides).
pub async fn paye_etat(
    cnx: &mut Connexion,
    inscription_id: Id<Inscription>,
    contexte: &Contexte,
) -> Resultat<Montant> {
    let total: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(p.montant), 0) FROM versement_etat_part p
         JOIN versement_etat v ON v.id = p.versement_etat_id
         WHERE p.inscription_id = ?1 AND p.etablissement_id = ?2 AND p.statut = 'active'
           AND v.statut = 'valide'",
    )
    .bind(inscription_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_one(cnx)
    .await?;
    Ok(Montant::depuis_francs(total))
}

/// `S` de chaque inscription d'une année qui a reçu de l'État (listes et états).
pub async fn paye_etat_de_l_annee(
    cnx: &mut Connexion,
    annee_id: Id<AnneeScolaire>,
    contexte: &Contexte,
) -> Resultat<Vec<(Id<Inscription>, Montant)>> {
    let lignes: Vec<(String, i64)> = sqlx::query_as(
        "SELECT p.inscription_id, SUM(p.montant) FROM versement_etat_part p
         JOIN versement_etat v ON v.id = p.versement_etat_id
         JOIN inscription i ON i.id = p.inscription_id
         WHERE i.annee_id = ?1 AND p.etablissement_id = ?2 AND p.statut = 'active'
           AND v.statut = 'valide'
         GROUP BY p.inscription_id",
    )
    .bind(annee_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?;
    lignes
        .into_iter()
        .map(|(i, m)| {
            Ok((
                lire_id("versement_etat_part", "inscription_id", &i)?,
                Montant::depuis_francs(m),
            ))
        })
        .collect()
}
