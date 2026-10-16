//! Sessions et mouvements de caisse (SPEC_PHASE_3 §6, §8). Jamais de suppression (R-14).

use gesco_core::commun::{Horodatage, Id, Montant};
use gesco_core::core::{MachineAEtats, Utilisateur};
use gesco_core::finance::{
    ModePaiement, MouvementCaisse, NatureMouvement, SessionCaisse, StatutSession, Versement,
};
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::depots::horodatage_optionnel;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

#[derive(FromRow)]
struct LigneSession {
    id: String,
    etablissement_id: String,
    caissier_id: String,
    ouverte_le: i64,
    fonds_initial: i64,
    statut: String,
    cloturee_le: Option<i64>,
    total_theorique: Option<i64>,
    total_compte: Option<i64>,
    ecart: Option<i64>,
    motif_ecart: String,
    rapprochee_par: Option<String>,
    rapprochee_le: Option<i64>,
    commentaire_visa: String,
}

impl LigneSession {
    fn vers_entite(self) -> Resultat<SessionCaisse> {
        let t = "session_caisse";
        Ok(SessionCaisse {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            caissier_id: lire_id(t, "caissier_id", &self.caissier_id)?,
            ouverte_le: Horodatage::depuis_millis_epoque(self.ouverte_le),
            fonds_initial: Montant::depuis_francs(self.fonds_initial),
            statut: StatutSession::depuis_nom(&self.statut)
                .ok_or_else(|| ErreurBase::illisible(t, "statut", &self.statut))?,
            cloturee_le: horodatage_optionnel(self.cloturee_le),
            total_theorique: self.total_theorique.map(Montant::depuis_francs),
            total_compte: self.total_compte.map(Montant::depuis_francs),
            ecart: self.ecart.map(Montant::depuis_francs),
            motif_ecart: self.motif_ecart,
            rapprochee_par: self
                .rapprochee_par
                .as_deref()
                .map(|r| lire_id::<Utilisateur>(t, "rapprochee_par", r))
                .transpose()?,
            rapprochee_le: horodatage_optionnel(self.rapprochee_le),
            commentaire_visa: self.commentaire_visa,
        })
    }
}

macro_rules! selectionner_session {
    ($suite:literal) => {
        concat!(
            "SELECT id, etablissement_id, caissier_id, ouverte_le, fonds_initial, statut, cloturee_le, total_theorique,
                total_compte, ecart, motif_ecart, rapprochee_par, rapprochee_le, commentaire_visa FROM session_caisse ",
            $suite
        )
    };
}

/// Ouvre une session.
pub async fn inserer(
    cnx: &mut Connexion,
    session: &SessionCaisse,
    contexte: &Contexte,
) -> Resultat<()> {
    if session.etablissement_id != contexte.etablissement_id {
        return Err(ErreurBase::Cloisonnement {
            entite: "session_caisse",
            id: session.id.vers_chaine(),
        });
    }
    sqlx::query(
        "INSERT INTO session_caisse (id, etablissement_id, caissier_id, ouverte_le, fonds_initial, statut,
            cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?7, ?8, ?9)",
    )
    .bind(session.id.vers_chaine())
    .bind(session.etablissement_id.vers_chaine())
    .bind(session.caissier_id.vers_chaine())
    .bind(session.ouverte_le.millis_epoque())
    .bind(session.fonds_initial.francs())
    .bind(session.statut.nom())
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Récrit la clôture et le visa d'une session.
pub async fn ecrire(
    cnx: &mut Connexion,
    session: &SessionCaisse,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE session_caisse SET statut = ?1, cloturee_le = ?2, total_theorique = ?3, total_compte = ?4, ecart = ?5,
            motif_ecart = ?6, rapprochee_par = ?7, rapprochee_le = ?8, commentaire_visa = ?9,
            mis_a_jour_le = ?10, mis_a_jour_par = ?11, identifiant_poste = ?12
         WHERE id = ?13 AND etablissement_id = ?14",
    )
    .bind(session.statut.nom())
    .bind(session.cloturee_le.map(|h| h.millis_epoque()))
    .bind(session.total_theorique.map(|m| m.francs()))
    .bind(session.total_compte.map(|m| m.francs()))
    .bind(session.ecart.map(|m| m.francs()))
    .bind(&session.motif_ecart)
    .bind(session.rapprochee_par.map(|r| r.vers_chaine()))
    .bind(session.rapprochee_le.map(|h| h.millis_epoque()))
    .bind(&session.commentaire_visa)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(session.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Lit une session.
pub async fn lire(
    cnx: &mut Connexion,
    id: Id<SessionCaisse>,
    contexte: &Contexte,
) -> Resultat<SessionCaisse> {
    sqlx::query_as::<_, LigneSession>(selectionner_session!(
        "WHERE id = ?1 AND etablissement_id = ?2"
    ))
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .ok_or_else(|| ErreurBase::Introuvable {
        entite: "session_caisse",
        id: id.vers_chaine(),
    })?
    .vers_entite()
}

/// Session ouverte d'un caissier, s'il en a une.
pub async fn ouverte_de(
    cnx: &mut Connexion,
    caissier_id: Id<Utilisateur>,
    contexte: &Contexte,
) -> Resultat<Option<SessionCaisse>> {
    sqlx::query_as::<_, LigneSession>(selectionner_session!(
        "WHERE caissier_id = ?1 AND etablissement_id = ?2 AND statut = 'ouverte'"
    ))
    .bind(caissier_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .map(LigneSession::vers_entite)
    .transpose()
}

/// Sessions de l'établissement, de la plus récente à la plus ancienne.
pub async fn lister(
    cnx: &mut Connexion,
    limite: i64,
    contexte: &Contexte,
) -> Resultat<Vec<SessionCaisse>> {
    sqlx::query_as::<_, LigneSession>(selectionner_session!(
        "WHERE etablissement_id = ?1 ORDER BY ouverte_le DESC LIMIT ?2"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(limite)
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneSession::vers_entite)
    .collect()
}

#[derive(FromRow)]
struct LigneMouvement {
    id: String,
    etablissement_id: String,
    session_id: String,
    montant: i64,
    mode: String,
    motif: String,
    versement_id: Option<String>,
    le: i64,
    par: String,
    nature: String,
    paiement_id: Option<String>,
    depense_id: Option<String>,
    transfert_id: Option<String>,
}

impl LigneMouvement {
    fn vers_entite(self) -> Resultat<MouvementCaisse> {
        let t = "mouvement_caisse";
        Ok(MouvementCaisse {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            session_id: lire_id(t, "session_id", &self.session_id)?,
            montant: Montant::depuis_francs(self.montant),
            mode: ModePaiement::depuis_nom(&self.mode)
                .ok_or_else(|| ErreurBase::illisible(t, "mode", &self.mode))?,
            motif: self.motif,
            versement_id: self
                .versement_id
                .as_deref()
                .map(|v| lire_id::<Versement>(t, "versement_id", v))
                .transpose()?,
            le: Horodatage::depuis_millis_epoque(self.le),
            par: lire_id(t, "par", &self.par)?,
            nature: NatureMouvement::depuis_nom(&self.nature)
                .ok_or_else(|| ErreurBase::illisible(t, "nature", &self.nature))?,
            paiement_id: self
                .paiement_id
                .as_deref()
                .map(|p| lire_id(t, "paiement_id", p))
                .transpose()?,
            depense_id: self
                .depense_id
                .as_deref()
                .map(|d| lire_id(t, "depense_id", d))
                .transpose()?,
            transfert_id: self
                .transfert_id
                .as_deref()
                .map(|d| lire_id(t, "transfert_id", d))
                .transpose()?,
        })
    }
}

macro_rules! selectionner_mouvement {
    ($suite:literal) => {
        concat!(
            "SELECT id, etablissement_id, session_id, montant, mode, motif, versement_id, le, par,
                nature, paiement_id, depense_id, transfert_id
             FROM mouvement_caisse ",
            $suite
        )
    };
}

/// Enregistre un mouvement de caisse.
pub async fn inserer_mouvement(
    cnx: &mut Connexion,
    mouvement: &MouvementCaisse,
    contexte: &Contexte,
) -> Resultat<()> {
    if mouvement.etablissement_id != contexte.etablissement_id {
        return Err(ErreurBase::Cloisonnement {
            entite: "mouvement_caisse",
            id: mouvement.id.vers_chaine(),
        });
    }
    sqlx::query(
        "INSERT INTO mouvement_caisse (id, etablissement_id, session_id, nature, montant, mode, motif, versement_id,
            le, par, cree_le, cree_par, identifiant_poste, paiement_id, depense_id, transfert_id)
         VALUES (?1, ?2, ?3, ?13, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?14, ?15, ?16)",
    )
    .bind(mouvement.id.vers_chaine())
    .bind(mouvement.etablissement_id.vers_chaine())
    .bind(mouvement.session_id.vers_chaine())
    .bind(mouvement.montant.francs())
    .bind(mouvement.mode.nom())
    .bind(&mouvement.motif)
    .bind(mouvement.versement_id.map(|v| v.vers_chaine()))
    .bind(mouvement.le.millis_epoque())
    .bind(mouvement.par.vers_chaine())
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(mouvement.nature.nom())
    .bind(mouvement.paiement_id.map(|p| p.vers_chaine()))
    .bind(mouvement.depense_id.map(|d| d.vers_chaine()))
    .bind(mouvement.transfert_id.map(|t| t.vers_chaine()))
    .execute(cnx)
    .await?;
    Ok(())
}

/// Tous les mouvements de caisse de l'école, dans l'ordre.
pub async fn mouvements(
    cnx: &mut Connexion,
    contexte: &Contexte,
) -> Resultat<Vec<MouvementCaisse>> {
    sqlx::query_as::<_, LigneMouvement>(selectionner_mouvement!(
        "WHERE etablissement_id = ?1 ORDER BY le"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneMouvement::vers_entite)
    .collect()
}

/// Mouvements passés dans une session, dans l'ordre.
pub async fn mouvements_de_la_session(
    cnx: &mut Connexion,
    session_id: Id<SessionCaisse>,
    contexte: &Contexte,
) -> Resultat<Vec<MouvementCaisse>> {
    sqlx::query_as::<_, LigneMouvement>(selectionner_mouvement!(
        "WHERE session_id = ?1 AND etablissement_id = ?2 ORDER BY le"
    ))
    .bind(session_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneMouvement::vers_entite)
    .collect()
}

/// Mouvement qui régularise un versement, s'il y en a un.
pub async fn regularisation_du_versement(
    cnx: &mut Connexion,
    versement_id: Id<Versement>,
    contexte: &Contexte,
) -> Resultat<Option<MouvementCaisse>> {
    sqlx::query_as::<_, LigneMouvement>(selectionner_mouvement!(
        "WHERE versement_id = ?1 AND etablissement_id = ?2"
    ))
    .bind(versement_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .map(LigneMouvement::vers_entite)
    .transpose()
}

/// Versements d'une session annulés après sa clôture : ils restent dans ses totaux, leur sortie
/// passe dans une autre session.
pub async fn regularises_ailleurs(
    cnx: &mut Connexion,
    session_id: Id<SessionCaisse>,
    contexte: &Contexte,
) -> Resultat<Vec<Id<Versement>>> {
    let ids: Vec<(String,)> = sqlx::query_as(
        "SELECT v.id FROM versement v JOIN mouvement_caisse m ON m.versement_id = v.id
         WHERE v.session_id = ?1 AND v.etablissement_id = ?2 AND m.session_id <> v.session_id",
    )
    .bind(session_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?;
    ids.into_iter()
        .map(|(id,)| lire_id("versement", "id", &id))
        .collect()
}

/// Nombre de régularisations passées dans des sessions que personne n'a encore visées : la
/// direction en est avertie (SPEC_PHASE_3 §8).
pub async fn regularisations_a_viser(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<i64> {
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM mouvement_caisse m JOIN session_caisse s ON s.id = m.session_id
         WHERE m.etablissement_id = ?1 AND s.statut <> 'rapprochee'
            AND m.nature IN ('sortie_regularisation', 'entree_regularisation')",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_one(cnx)
    .await?;
    Ok(n)
}
