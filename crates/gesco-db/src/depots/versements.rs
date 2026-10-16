//! Versements et imputations (SPEC_PHASE_3 §7). Jamais de suppression (R-14).

use gesco_core::commun::{Horodatage, Id, Montant};
use gesco_core::core::{MachineAEtats, Utilisateur};
use gesco_core::finance::{
    Echeance, Facture, Imputation, ModePaiement, SessionCaisse, StatutVersement, Versement,
};
use gesco_core::scolarite::{Inscription, Responsable};
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

#[derive(FromRow)]
struct LigneVersement {
    id: String,
    etablissement_id: String,
    inscription_id: String,
    facture_id: String,
    session_id: String,
    numero_recu: String,
    encaisse_le: i64,
    montant: i64,
    mode: String,
    reference: String,
    payeur_id: Option<String>,
    payeur_nom: String,
    encaisse_par: String,
    cle_idempotence: String,
    statut: String,
    motif_annulation: String,
}

impl LigneVersement {
    fn vers_entite(self) -> Resultat<Versement> {
        let t = "versement";
        Ok(Versement {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            inscription_id: lire_id(t, "inscription_id", &self.inscription_id)?,
            facture_id: lire_id(t, "facture_id", &self.facture_id)?,
            session_id: lire_id(t, "session_id", &self.session_id)?,
            encaisse_le: Horodatage::depuis_millis_epoque(self.encaisse_le),
            montant: Montant::depuis_francs(self.montant),
            mode: ModePaiement::depuis_nom(&self.mode)
                .ok_or_else(|| ErreurBase::illisible(t, "mode", &self.mode))?,
            payeur_id: self
                .payeur_id
                .as_deref()
                .map(|p| lire_id::<Responsable>(t, "payeur_id", p))
                .transpose()?,
            encaisse_par: lire_id::<Utilisateur>(t, "encaisse_par", &self.encaisse_par)?,
            statut: StatutVersement::depuis_nom(&self.statut)
                .ok_or_else(|| ErreurBase::illisible(t, "statut", &self.statut))?,
            numero_recu: self.numero_recu,
            reference: self.reference,
            payeur_nom: self.payeur_nom,
            cle_idempotence: self.cle_idempotence,
            motif_annulation: self.motif_annulation,
        })
    }
}

macro_rules! selectionner_versement {
    ($suite:literal) => {
        concat!(
            "SELECT id, etablissement_id, inscription_id, facture_id, session_id, numero_recu, encaisse_le, montant,
                mode, reference, payeur_id, payeur_nom, encaisse_par, cle_idempotence, statut, motif_annulation
             FROM versement ",
            $suite
        )
    };
}

/// Enregistre un versement.
pub async fn inserer(
    cnx: &mut Connexion,
    versement: &Versement,
    contexte: &Contexte,
) -> Resultat<()> {
    if versement.etablissement_id != contexte.etablissement_id {
        return Err(ErreurBase::Cloisonnement {
            entite: "versement",
            id: versement.id.vers_chaine(),
        });
    }
    sqlx::query(
        "INSERT INTO versement (id, etablissement_id, inscription_id, facture_id, session_id, numero_recu, encaisse_le,
            montant, mode, reference, payeur_id, payeur_nom, encaisse_par, cle_idempotence, statut, motif_annulation,
            cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?17, ?18, ?19)",
    )
    .bind(versement.id.vers_chaine())
    .bind(versement.etablissement_id.vers_chaine())
    .bind(versement.inscription_id.vers_chaine())
    .bind(versement.facture_id.vers_chaine())
    .bind(versement.session_id.vers_chaine())
    .bind(&versement.numero_recu)
    .bind(versement.encaisse_le.millis_epoque())
    .bind(versement.montant.francs())
    .bind(versement.mode.nom())
    .bind(&versement.reference)
    .bind(versement.payeur_id.map(|p| p.vers_chaine()))
    .bind(&versement.payeur_nom)
    .bind(versement.encaisse_par.vers_chaine())
    .bind(&versement.cle_idempotence)
    .bind(versement.statut.nom())
    .bind(&versement.motif_annulation)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Enregistre l'imputation d'un versement sur une échéance.
pub async fn inserer_imputation(
    cnx: &mut Connexion,
    versement_id: Id<Versement>,
    imputation: &Imputation,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "INSERT INTO imputation_versement (id, etablissement_id, versement_id, echeance_id, montant, statut,
            cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, 'active', ?6, ?7, ?6, ?7, ?8)",
    )
    .bind(crate::nouvel_id::<()>().vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(versement_id.vers_chaine())
    .bind(imputation.echeance_id.vers_chaine())
    .bind(imputation.montant.francs())
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Versement déjà enregistré sous cette clé d'idempotence, s'il y en a un.
pub async fn par_cle(
    cnx: &mut Connexion,
    cle: &str,
    contexte: &Contexte,
) -> Resultat<Option<Versement>> {
    sqlx::query_as::<_, LigneVersement>(selectionner_versement!(
        "WHERE cle_idempotence = ?1 AND etablissement_id = ?2"
    ))
    .bind(cle)
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .map(LigneVersement::vers_entite)
    .transpose()
}

/// Lit un versement.
pub async fn lire(
    cnx: &mut Connexion,
    id: Id<Versement>,
    contexte: &Contexte,
) -> Resultat<Versement> {
    sqlx::query_as::<_, LigneVersement>(selectionner_versement!(
        "WHERE id = ?1 AND etablissement_id = ?2"
    ))
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .ok_or_else(|| ErreurBase::Introuvable {
        entite: "versement",
        id: id.vers_chaine(),
    })?
    .vers_entite()
}

/// Versements d'une inscription, du plus ancien au plus récent.
pub async fn de_l_inscription(
    cnx: &mut Connexion,
    inscription_id: Id<Inscription>,
    contexte: &Contexte,
) -> Resultat<Vec<Versement>> {
    sqlx::query_as::<_, LigneVersement>(selectionner_versement!(
        "WHERE inscription_id = ?1 AND etablissement_id = ?2 ORDER BY encaisse_le, numero_recu"
    ))
    .bind(inscription_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneVersement::vers_entite)
    .collect()
}

/// Versements d'une session de caisse, dans l'ordre des reçus.
pub async fn de_la_session(
    cnx: &mut Connexion,
    session_id: Id<SessionCaisse>,
    contexte: &Contexte,
) -> Resultat<Vec<Versement>> {
    sqlx::query_as::<_, LigneVersement>(selectionner_versement!(
        "WHERE session_id = ?1 AND etablissement_id = ?2 ORDER BY encaisse_le, numero_recu"
    ))
    .bind(session_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneVersement::vers_entite)
    .collect()
}

#[derive(FromRow)]
struct LigneImputation {
    versement_id: String,
    echeance_id: String,
    montant: i64,
}

/// Imputations actives sur les échéances d'une facture : (versement, échéance, montant).
pub async fn imputations_de_la_facture(
    cnx: &mut Connexion,
    facture_id: Id<Facture>,
    contexte: &Contexte,
) -> Resultat<Vec<(Id<Versement>, Id<Echeance>, Montant)>> {
    sqlx::query_as::<_, LigneImputation>(
        "SELECT i.versement_id, i.echeance_id, i.montant FROM imputation_versement i
         JOIN echeance e ON e.id = i.echeance_id
         WHERE e.facture_id = ?1 AND i.etablissement_id = ?2 AND i.statut = 'active'",
    )
    .bind(facture_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(|l| {
        Ok((
            lire_id("imputation_versement", "versement_id", &l.versement_id)?,
            lire_id("imputation_versement", "echeance_id", &l.echeance_id)?,
            Montant::depuis_francs(l.montant),
        ))
    })
    .collect()
}

/// Annule un versement encore valide : statut, motif, auteur, instant. Rend faux s'il ne l'était
/// plus (deux annulations concurrentes).
pub async fn annuler(
    cnx: &mut Connexion,
    id: Id<Versement>,
    motif: &str,
    contexte: &Contexte,
) -> Resultat<bool> {
    let fait = sqlx::query(
        "UPDATE versement SET statut = 'annule', motif_annulation = ?1, annule_par = ?2, annule_le = ?3,
            mis_a_jour_le = ?3, mis_a_jour_par = ?2, identifiant_poste = ?4
         WHERE id = ?5 AND etablissement_id = ?6 AND statut = 'valide'",
    )
    .bind(motif)
    .bind(contexte.auteur())
    .bind(contexte.maintenant.millis_epoque())
    .bind(&contexte.poste.0)
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(fait.rows_affected() == 1)
}

/// Contre-passe les imputations d'un versement : elles passent à `annulee` et ne comptent plus
/// dans le payé des échéances.
pub async fn annuler_imputations(
    cnx: &mut Connexion,
    versement_id: Id<Versement>,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE imputation_versement SET statut = 'annulee', mis_a_jour_le = ?1, mis_a_jour_par = ?2,
            identifiant_poste = ?3
         WHERE versement_id = ?4 AND etablissement_id = ?5 AND statut = 'active'",
    )
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(versement_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Imputations d'origine d'un versement, actives ou contre-passées : le reçu, même annulé,
/// montre ce que le versement avait réglé.
pub async fn imputations_du_versement(
    cnx: &mut Connexion,
    versement_id: Id<Versement>,
    contexte: &Contexte,
) -> Resultat<Vec<Imputation>> {
    sqlx::query_as::<_, LigneImputation>(
        "SELECT versement_id, echeance_id, montant FROM imputation_versement
         WHERE versement_id = ?1 AND etablissement_id = ?2 ORDER BY cree_le",
    )
    .bind(versement_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(|l| {
        Ok(Imputation {
            echeance_id: lire_id("imputation_versement", "echeance_id", &l.echeance_id)?,
            montant: Montant::depuis_francs(l.montant),
        })
    })
    .collect()
}

/// Versement de la période avec ce qu'en montre le journal (SPEC_PHASE_3 §9.2).
#[derive(Debug, Clone)]
pub struct VersementNomme {
    /// Versement.
    pub versement: Versement,
    /// Élève : prénoms et nom.
    pub eleve: String,
    /// Matricule interne.
    pub matricule: String,
    /// Classe de l'inscription.
    pub classe: String,
    /// Nom du caissier.
    pub caissier: String,
}

#[derive(FromRow)]
struct LigneVersementNommee {
    #[sqlx(flatten)]
    versement: LigneVersement,
    eleve_prenoms: String,
    eleve_nom: String,
    matricule: String,
    classe: String,
    caissier: String,
}

/// Versements encaissés entre deux instants (début compris, fin exclue), annulés compris, dans
/// l'ordre des reçus.
pub async fn de_la_periode(
    cnx: &mut Connexion,
    du: Horodatage,
    au: Horodatage,
    contexte: &Contexte,
) -> Resultat<Vec<VersementNomme>> {
    sqlx::query_as::<_, LigneVersementNommee>(
        "SELECT v.id, v.etablissement_id, v.inscription_id, v.facture_id, v.session_id, v.numero_recu,
            v.encaisse_le, v.montant, v.mode, v.reference, v.payeur_id, v.payeur_nom, v.encaisse_par,
            v.cle_idempotence, v.statut, v.motif_annulation,
            p.prenoms AS eleve_prenoms, p.nom AS eleve_nom, e.matricule_interne AS matricule,
            c.libelle AS classe, u.nom_affiche AS caissier
         FROM versement v
         JOIN inscription i ON i.id = v.inscription_id
         JOIN eleve e ON e.id = i.eleve_id
         JOIN personne p ON p.id = e.personne_id
         JOIN classe c ON c.id = i.classe_id
         JOIN utilisateur u ON u.id = v.encaisse_par
         WHERE v.etablissement_id = ?1 AND v.encaisse_le >= ?2 AND v.encaisse_le < ?3
         ORDER BY v.encaisse_le, v.numero_recu",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(du.millis_epoque())
    .bind(au.millis_epoque())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(|l| {
        Ok(VersementNomme {
            versement: l.versement.vers_entite()?,
            eleve: format!("{} {}", l.eleve_prenoms, l.eleve_nom),
            matricule: l.matricule,
            classe: l.classe,
            caissier: l.caissier,
        })
    })
    .collect()
}
