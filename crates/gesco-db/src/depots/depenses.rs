//! Dépenses (SPEC_PHASE_8 §3) et leurs justificatifs.

use gesco_core::commun::{DateCivile, Id, Montant};
use gesco_core::core::{MachineAEtats, Utilisateur};
use gesco_core::finance::ModePaiement;
use gesco_core::tresorerie::{Depense, StatutDepense};
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::depots::horodatage_optionnel;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

#[derive(FromRow)]
struct LigneDepense {
    id: String,
    etablissement_id: String,
    date: String,
    categorie_id: String,
    libelle: String,
    beneficiaire: String,
    montant: i64,
    statut: String,
    saisie_par: String,
    decidee_par: Option<String>,
    compte_id: Option<String>,
    mode: Option<String>,
    reference: String,
    session_id: Option<String>,
    payee_le: Option<i64>,
    motif: String,
    facture_id: Option<String>,
    paiement_id: Option<String>,
}

impl LigneDepense {
    fn vers_entite(self) -> Resultat<Depense> {
        let t = "depense";
        Ok(Depense {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            date: DateCivile::analyser(&self.date)
                .ok_or_else(|| ErreurBase::illisible(t, "date", &self.date))?,
            categorie_id: lire_id(t, "categorie_id", &self.categorie_id)?,
            libelle: self.libelle,
            beneficiaire: self.beneficiaire,
            montant: Montant::depuis_francs(self.montant),
            statut: StatutDepense::depuis_nom(&self.statut)
                .ok_or_else(|| ErreurBase::illisible(t, "statut", &self.statut))?,
            saisie_par: lire_id::<Utilisateur>(t, "saisie_par", &self.saisie_par)?,
            decidee_par: self
                .decidee_par
                .as_deref()
                .map(|u| lire_id::<Utilisateur>(t, "decidee_par", u))
                .transpose()?,
            compte_id: self
                .compte_id
                .as_deref()
                .map(|c| lire_id(t, "compte_id", c))
                .transpose()?,
            mode: self
                .mode
                .as_deref()
                .map(|m| {
                    ModePaiement::depuis_nom(m).ok_or_else(|| ErreurBase::illisible(t, "mode", m))
                })
                .transpose()?,
            reference: self.reference,
            session_id: self
                .session_id
                .as_deref()
                .map(|s| lire_id(t, "session_id", s))
                .transpose()?,
            payee_le: horodatage_optionnel(self.payee_le),
            motif: self.motif,
            facture_id: self
                .facture_id
                .as_deref()
                .map(|f| lire_id(t, "facture_id", f))
                .transpose()?,
            paiement_id: self
                .paiement_id
                .as_deref()
                .map(|p| lire_id(t, "paiement_id", p))
                .transpose()?,
        })
    }
}

macro_rules! selectionner_depense {
    ($suite:literal) => {
        concat!(
            "SELECT id, etablissement_id, date, categorie_id, libelle, beneficiaire, montant, statut,
                saisie_par, decidee_par, compte_id, mode, reference, session_id, payee_le, motif,
                facture_id, paiement_id
             FROM depense ",
            $suite
        )
    };
}

/// Dépenses entre deux jours inclus, la plus récente d'abord.
pub async fn depenses_entre(
    cnx: &mut Connexion,
    debut: DateCivile,
    fin: DateCivile,
    contexte: &Contexte,
) -> Resultat<Vec<Depense>> {
    sqlx::query_as::<_, LigneDepense>(selectionner_depense!(
        "WHERE etablissement_id = ?1 AND date BETWEEN ?2 AND ?3 ORDER BY date DESC, cree_le DESC"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(debut.to_string())
    .bind(fin.to_string())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneDepense::vers_entite)
    .collect()
}

/// Dépenses qui attendent : l'accord de la direction, ou leur paiement.
pub async fn en_attente(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<Depense>> {
    sqlx::query_as::<_, LigneDepense>(selectionner_depense!(
        "WHERE etablissement_id = ?1 AND statut IN ('a_approuver', 'approuvee')
         ORDER BY date, cree_le"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneDepense::vers_entite)
    .collect()
}

/// Une dépense.
pub async fn depense(
    cnx: &mut Connexion,
    id: Id<Depense>,
    contexte: &Contexte,
) -> Resultat<Depense> {
    sqlx::query_as::<_, LigneDepense>(selectionner_depense!(
        "WHERE id = ?1 AND etablissement_id = ?2"
    ))
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .ok_or_else(|| ErreurBase::Introuvable {
        entite: "depense",
        id: id.vers_chaine(),
    })?
    .vers_entite()
}

/// Enregistre une dépense.
pub async fn inserer(cnx: &mut Connexion, d: &Depense, contexte: &Contexte) -> Resultat<()> {
    if d.etablissement_id != contexte.etablissement_id {
        return Err(ErreurBase::Cloisonnement {
            entite: "depense",
            id: d.id.vers_chaine(),
        });
    }
    sqlx::query(
        "INSERT INTO depense (id, etablissement_id, date, categorie_id, libelle, beneficiaire,
            montant, statut, saisie_par, decidee_par, compte_id, mode, reference, session_id,
            payee_le, motif, facture_id, paiement_id, cree_le, cree_par, mis_a_jour_le,
            mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18,
            ?19, ?20, ?19, ?20, ?21)",
    )
    .bind(d.id.vers_chaine())
    .bind(d.etablissement_id.vers_chaine())
    .bind(d.date.to_string())
    .bind(d.categorie_id.vers_chaine())
    .bind(&d.libelle)
    .bind(&d.beneficiaire)
    .bind(d.montant.francs())
    .bind(d.statut.nom())
    .bind(d.saisie_par.vers_chaine())
    .bind(d.decidee_par.map(|u| u.vers_chaine()))
    .bind(d.compte_id.map(|c| c.vers_chaine()))
    .bind(d.mode.map(|m| m.nom()))
    .bind(&d.reference)
    .bind(d.session_id.map(|s| s.vers_chaine()))
    .bind(d.payee_le.map(|h| h.millis_epoque()))
    .bind(&d.motif)
    .bind(d.facture_id.map(|f| f.vers_chaine()))
    .bind(d.paiement_id.map(|p| p.vers_chaine()))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Dépenses payées hors d'une session de caisse : d'une banque, d'un compte mobile money, ou en
/// espèces de l'école hors du tiroir.
pub async fn payees_hors_session(
    cnx: &mut Connexion,
    contexte: &Contexte,
) -> Resultat<Vec<Depense>> {
    sqlx::query_as::<_, LigneDepense>(selectionner_depense!(
        "WHERE etablissement_id = ?1 AND statut = 'payee' AND session_id IS NULL
            AND compte_id IS NOT NULL ORDER BY payee_le"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneDepense::vers_entite)
    .collect()
}

/// Dépenses nées des salaires et des avances payés.
pub async fn de_paiements(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<Depense>> {
    sqlx::query_as::<_, LigneDepense>(selectionner_depense!(
        "WHERE etablissement_id = ?1 AND paiement_id IS NOT NULL ORDER BY cree_le"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneDepense::vers_entite)
    .collect()
}

/// Dépenses qui paient une facture de fournisseur, de la plus ancienne à la plus récente.
pub async fn de_factures(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<Depense>> {
    sqlx::query_as::<_, LigneDepense>(selectionner_depense!(
        "WHERE etablissement_id = ?1 AND facture_id IS NOT NULL ORDER BY date, cree_le"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneDepense::vers_entite)
    .collect()
}

/// Récrit l'état d'une dépense, sa décision, son paiement et son motif.
pub async fn ecrire(cnx: &mut Connexion, d: &Depense, contexte: &Contexte) -> Resultat<()> {
    sqlx::query(
        "UPDATE depense SET statut = ?1, decidee_par = ?2, compte_id = ?3, mode = ?4,
            reference = ?5, session_id = ?6, payee_le = ?7, motif = ?8, mis_a_jour_le = ?9,
            mis_a_jour_par = ?10, identifiant_poste = ?11
         WHERE id = ?12 AND etablissement_id = ?13",
    )
    .bind(d.statut.nom())
    .bind(d.decidee_par.map(|u| u.vers_chaine()))
    .bind(d.compte_id.map(|c| c.vers_chaine()))
    .bind(d.mode.map(|m| m.nom()))
    .bind(&d.reference)
    .bind(d.session_id.map(|s| s.vers_chaine()))
    .bind(d.payee_le.map(|h| h.millis_epoque()))
    .bind(&d.motif)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(d.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Justificatif d'une dépense, sans son contenu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PieceDepense {
    /// Identifiant technique.
    pub id: String,
    /// Dépense.
    pub depense_id: Id<Depense>,
    /// Nom du fichier.
    pub nom_fichier: String,
    /// Taille, en octets.
    pub taille: i64,
}

/// Justificatifs d'une dépense.
pub async fn pieces(
    cnx: &mut Connexion,
    depense_id: Id<Depense>,
    contexte: &Contexte,
) -> Resultat<Vec<PieceDepense>> {
    let lignes: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT id, nom_fichier, taille FROM piece_depense
         WHERE depense_id = ?1 AND etablissement_id = ?2 ORDER BY cree_le",
    )
    .bind(depense_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|(id, nom_fichier, taille)| PieceDepense {
            id,
            depense_id,
            nom_fichier,
            taille,
        })
        .collect())
}

/// Contenu d'un justificatif.
pub async fn contenu_piece(
    cnx: &mut Connexion,
    id: &str,
    contexte: &Contexte,
) -> Resultat<(String, Vec<u8>)> {
    sqlx::query_as::<_, (String, Vec<u8>)>(
        "SELECT nom_fichier, contenu FROM piece_depense WHERE id = ?1 AND etablissement_id = ?2",
    )
    .bind(id)
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .ok_or_else(|| ErreurBase::Introuvable {
        entite: "piece_depense",
        id: id.to_owned(),
    })
}

/// Joint un justificatif à une dépense.
pub async fn inserer_piece(
    cnx: &mut Connexion,
    id: &str,
    depense_id: Id<Depense>,
    nom_fichier: &str,
    contenu: &[u8],
    empreinte: &str,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "INSERT INTO piece_depense (id, etablissement_id, depense_id, nom_fichier, contenu, taille,
            empreinte_sha256, cree_le, cree_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
    )
    .bind(id)
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(depense_id.vers_chaine())
    .bind(nom_fichier)
    .bind(contenu)
    .bind(i64::try_from(contenu.len()).unwrap_or(i64::MAX))
    .bind(empreinte)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}
