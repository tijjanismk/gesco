//! Code d'accès des responsables et SMS reçus (SPEC_PHASE_5 §9). Un code ne se supprime pas, il
//! se révoque ; un SMS reçu reste au journal (R-14).

use gesco_core::commun::{Horodatage, Id};
use gesco_core::notification::consultation::{
    CodeAcces, CompteDuJour, EtatCodeAcces, IssueConsultation, MotConsultation, SmsRecu,
};
use gesco_core::scolarite::Responsable;
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::depots::horodatage_optionnel;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

#[derive(FromRow)]
struct LigneCode {
    id: String,
    etablissement_id: String,
    responsable_id: String,
    empreinte: String,
    etat: String,
    cree_le: i64,
    revoque_le: Option<i64>,
    derniere_utilisation: Option<i64>,
}

impl LigneCode {
    fn vers_entite(self) -> Resultat<CodeAcces> {
        let t = "code_acces";
        Ok(CodeAcces {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            responsable_id: lire_id(t, "responsable_id", &self.responsable_id)?,
            empreinte: self.empreinte,
            etat: EtatCodeAcces::depuis_nom(&self.etat)
                .ok_or_else(|| ErreurBase::illisible(t, "etat", &self.etat))?,
            cree_le: Horodatage::depuis_millis_epoque(self.cree_le),
            revoque_le: horodatage_optionnel(self.revoque_le),
            derniere_utilisation: horodatage_optionnel(self.derniere_utilisation),
        })
    }
}

/// Le code actif d'un responsable, s'il en a un.
pub async fn code_actif(
    cnx: &mut Connexion,
    responsable_id: Id<Responsable>,
    contexte: &Contexte,
) -> Resultat<Option<CodeAcces>> {
    sqlx::query_as::<_, LigneCode>(
        "SELECT id, etablissement_id, responsable_id, empreinte, etat, cree_le, revoque_le,
            derniere_utilisation
         FROM code_acces WHERE responsable_id = ?1 AND etablissement_id = ?2 AND etat = 'actif'",
    )
    .bind(responsable_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .map(LigneCode::vers_entite)
    .transpose()
}

/// Le dernier code révoqué d'un responsable, s'il en a eu un.
pub async fn dernier_revoque(
    cnx: &mut Connexion,
    responsable_id: Id<Responsable>,
    contexte: &Contexte,
) -> Resultat<Option<CodeAcces>> {
    sqlx::query_as::<_, LigneCode>(
        "SELECT id, etablissement_id, responsable_id, empreinte, etat, cree_le, revoque_le,
            derniere_utilisation
         FROM code_acces WHERE responsable_id = ?1 AND etablissement_id = ?2 AND etat = 'revoque'
         ORDER BY revoque_le DESC, id DESC LIMIT 1",
    )
    .bind(responsable_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .map(LigneCode::vers_entite)
    .transpose()
}

/// Enregistre un code remis.
pub async fn inserer_code(cnx: &mut Connexion, c: &CodeAcces, contexte: &Contexte) -> Resultat<()> {
    sqlx::query(
        "INSERT INTO code_acces (id, etablissement_id, responsable_id, empreinte, etat, cree_le,
            cree_par, revoque_le, derniere_utilisation, mis_a_jour_le, mis_a_jour_par,
            identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?6, ?7, ?10)",
    )
    .bind(c.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(c.responsable_id.vers_chaine())
    .bind(&c.empreinte)
    .bind(c.etat.nom())
    .bind(c.cree_le.millis_epoque())
    .bind(contexte.auteur())
    .bind(c.revoque_le.map(|h| h.millis_epoque()))
    .bind(c.derniere_utilisation.map(|h| h.millis_epoque()))
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Révoque un code.
pub async fn revoquer_code(
    cnx: &mut Connexion,
    id: Id<CodeAcces>,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE code_acces SET etat = 'revoque', revoque_le = ?1, mis_a_jour_le = ?1,
            mis_a_jour_par = ?2, identifiant_poste = ?3
         WHERE id = ?4 AND etablissement_id = ?5 AND etat = 'actif'",
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

/// Note la dernière consultation servie avec un code.
pub async fn noter_utilisation(
    cnx: &mut Connexion,
    id: Id<CodeAcces>,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE code_acces SET derniere_utilisation = ?1, mis_a_jour_le = ?1,
            identifiant_poste = ?2
         WHERE id = ?3 AND etablissement_id = ?4",
    )
    .bind(contexte.maintenant.millis_epoque())
    .bind(&contexte.poste.0)
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

#[derive(FromRow)]
struct LigneSmsRecu {
    id: String,
    etablissement_id: String,
    numero: String,
    texte: String,
    mot: Option<String>,
    issue: String,
    responsable_id: Option<String>,
    reponse_id: Option<String>,
    recu_le: i64,
}

impl LigneSmsRecu {
    fn vers_entite(self) -> Resultat<SmsRecu> {
        let t = "sms_recu";
        Ok(SmsRecu {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            numero: self.numero,
            texte: self.texte,
            mot: self
                .mot
                .map(|m| {
                    MotConsultation::depuis_nom(&m)
                        .ok_or_else(|| ErreurBase::illisible(t, "mot", &m))
                })
                .transpose()?,
            issue: IssueConsultation::depuis_nom(&self.issue)
                .ok_or_else(|| ErreurBase::illisible(t, "issue", &self.issue))?,
            responsable_id: self
                .responsable_id
                .map(|r| lire_id(t, "responsable_id", &r))
                .transpose()?,
            reponse_id: self
                .reponse_id
                .map(|r| lire_id(t, "reponse_id", &r))
                .transpose()?,
            recu_le: Horodatage::depuis_millis_epoque(self.recu_le),
        })
    }
}

/// Enregistre un SMS reçu.
pub async fn inserer_sms_recu(
    cnx: &mut Connexion,
    s: &SmsRecu,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "INSERT INTO sms_recu (id, etablissement_id, numero, texte, mot, issue, responsable_id,
            reponse_id, recu_le, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
    )
    .bind(s.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(&s.numero)
    .bind(&s.texte)
    .bind(s.mot.map(|m| m.nom()))
    .bind(s.issue.nom())
    .bind(s.responsable_id.map(|r| r.vers_chaine()))
    .bind(s.reponse_id.map(|r| r.vers_chaine()))
    .bind(s.recu_le.millis_epoque())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Les derniers SMS reçus, du plus récent au plus ancien.
pub async fn sms_recus(
    cnx: &mut Connexion,
    limite: i64,
    contexte: &Contexte,
) -> Resultat<Vec<SmsRecu>> {
    sqlx::query_as::<_, LigneSmsRecu>(
        "SELECT id, etablissement_id, numero, texte, mot, issue, responsable_id, reponse_id, recu_le
         FROM sms_recu WHERE etablissement_id = ?1 ORDER BY recu_le DESC, id DESC LIMIT ?2",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(limite)
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneSmsRecu::vers_entite)
    .collect()
}

/// Ce qu'un numéro a demandé depuis un instant (le début du jour).
pub async fn compte_depuis(
    cnx: &mut Connexion,
    numero: &str,
    depuis: Horodatage,
    contexte: &Contexte,
) -> Resultat<CompteDuJour> {
    let issues: Vec<String> = sqlx::query_scalar(
        "SELECT issue FROM sms_recu WHERE etablissement_id = ?1 AND numero = ?2 AND recu_le >= ?3",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(numero)
    .bind(depuis.millis_epoque())
    .fetch_all(cnx)
    .await?;
    let mut compte = CompteDuJour::default();
    for issue in issues
        .iter()
        .filter_map(|i| IssueConsultation::depuis_nom(i))
    {
        if issue.compte_dans_le_quota() {
            compte.demandes += 1;
        }
        if issue.code_faux() {
            compte.codes_faux += 1;
        }
    }
    Ok(compte)
}
