//! File des SMS (SPEC_PHASE_5 §3) : les messages, leur état, ce qui part et ce qui est revenu.

use gesco_core::commun::{Horodatage, Id};
use gesco_core::notification::{
    ErreurEnvoi, EtatCanal, EtatMessage, Message, NatureMessage, RaisonArret,
};
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

#[derive(FromRow)]
struct LigneMessage {
    id: String,
    etablissement_id: String,
    nature: String,
    cle_fait: String,
    responsable_id: Option<String>,
    eleve_id: Option<String>,
    numero: String,
    texte: String,
    sms: i64,
    etat: String,
    essais: i64,
    pas_avant: i64,
    expire_le: Option<i64>,
    reference: Option<String>,
    envoye_le: Option<i64>,
    remis_le: Option<i64>,
    erreur: Option<String>,
    motif_annulation: String,
    cree_le: i64,
}

impl LigneMessage {
    fn vers_entite(self) -> Resultat<Message> {
        let t = "message_sms";
        let h = Horodatage::depuis_millis_epoque;
        let petit = |colonne: &'static str, n: i64| {
            u8::try_from(n).map_err(|_| ErreurBase::illisible(t, colonne, n.to_string()))
        };
        Ok(Message {
            id: lire_id(t, "id", &self.id)?,
            etablissement_id: lire_id(t, "etablissement_id", &self.etablissement_id)?,
            nature: NatureMessage::depuis_nom(&self.nature)
                .ok_or_else(|| ErreurBase::illisible(t, "nature", &self.nature))?,
            cle_fait: self.cle_fait,
            responsable_id: self
                .responsable_id
                .as_deref()
                .map(|r| lire_id(t, "responsable_id", r))
                .transpose()?,
            eleve_id: self
                .eleve_id
                .as_deref()
                .map(|e| lire_id(t, "eleve_id", e))
                .transpose()?,
            numero: self.numero,
            texte: self.texte,
            sms: petit("sms", self.sms)?,
            etat: EtatMessage::depuis_nom(&self.etat)
                .ok_or_else(|| ErreurBase::illisible(t, "etat", &self.etat))?,
            essais: petit("essais", self.essais)?,
            pas_avant: h(self.pas_avant),
            expire_le: self.expire_le.map(h),
            reference: self.reference,
            envoye_le: self.envoye_le.map(h),
            remis_le: self.remis_le.map(h),
            erreur: self
                .erreur
                .as_deref()
                .map(|e| {
                    ErreurEnvoi::depuis_nom(e).ok_or_else(|| ErreurBase::illisible(t, "erreur", e))
                })
                .transpose()?,
            motif_annulation: self.motif_annulation,
            cree_le: h(self.cree_le),
        })
    }
}

macro_rules! selectionner_message {
    ($suite:literal) => {
        concat!(
            "SELECT id, etablissement_id, nature, cle_fait, responsable_id, eleve_id, numero,
                texte, sms, etat, essais, pas_avant, expire_le, reference, envoye_le, remis_le,
                erreur, motif_annulation, cree_le
             FROM message_sms ",
            $suite
        )
    };
}

/// Met un message en file ; faux s'il y était déjà pour ce fait et ce numéro.
pub async fn inserer(cnx: &mut Connexion, m: &Message, contexte: &Contexte) -> Resultat<bool> {
    if m.etablissement_id != contexte.etablissement_id {
        return Err(ErreurBase::Cloisonnement {
            entite: "message_sms",
            id: m.id.vers_chaine(),
        });
    }
    let fait = sqlx::query(
        "INSERT INTO message_sms (id, etablissement_id, nature, cle_fait, responsable_id,
            eleve_id, numero, texte, sms, etat, essais, pas_avant, expire_le, reference,
            envoye_le, remis_le, erreur, motif_annulation, cree_le, cree_par, mis_a_jour_le,
            mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18,
            ?19, ?20, ?19, ?20, ?21)
         ON CONFLICT (etablissement_id, cle_fait, numero) DO NOTHING",
    )
    .bind(m.id.vers_chaine())
    .bind(m.etablissement_id.vers_chaine())
    .bind(m.nature.nom())
    .bind(&m.cle_fait)
    .bind(m.responsable_id.map(|r| r.vers_chaine()))
    .bind(m.eleve_id.map(|e| e.vers_chaine()))
    .bind(&m.numero)
    .bind(&m.texte)
    .bind(i64::from(m.sms))
    .bind(m.etat.nom())
    .bind(i64::from(m.essais))
    .bind(m.pas_avant.millis_epoque())
    .bind(m.expire_le.map(|h| h.millis_epoque()))
    .bind(&m.reference)
    .bind(m.envoye_le.map(|h| h.millis_epoque()))
    .bind(m.remis_le.map(|h| h.millis_epoque()))
    .bind(m.erreur.map(|e| e.nom()))
    .bind(&m.motif_annulation)
    .bind(m.cree_le.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(fait.rows_affected() == 1)
}

/// Enregistre l'état d'un message : texte, état, essais, envoi, remise, erreur, annulation.
pub async fn mettre_a_jour(cnx: &mut Connexion, m: &Message, contexte: &Contexte) -> Resultat<()> {
    sqlx::query(
        "UPDATE message_sms SET texte = ?1, sms = ?2, etat = ?3, essais = ?4, pas_avant = ?5,
            expire_le = ?6, reference = ?7, envoye_le = ?8, remis_le = ?9, erreur = ?10,
            motif_annulation = ?11, mis_a_jour_le = ?12, mis_a_jour_par = ?13,
            identifiant_poste = ?14
         WHERE id = ?15 AND etablissement_id = ?16",
    )
    .bind(&m.texte)
    .bind(i64::from(m.sms))
    .bind(m.etat.nom())
    .bind(i64::from(m.essais))
    .bind(m.pas_avant.millis_epoque())
    .bind(m.expire_le.map(|h| h.millis_epoque()))
    .bind(&m.reference)
    .bind(m.envoye_le.map(|h| h.millis_epoque()))
    .bind(m.remis_le.map(|h| h.millis_epoque()))
    .bind(m.erreur.map(|e| e.nom()))
    .bind(&m.motif_annulation)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(m.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Un message.
pub async fn message(
    cnx: &mut Connexion,
    id: Id<Message>,
    contexte: &Contexte,
) -> Resultat<Message> {
    sqlx::query_as::<_, LigneMessage>(selectionner_message!(
        "WHERE id = ?1 AND etablissement_id = ?2"
    ))
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .ok_or_else(|| ErreurBase::Introuvable {
        entite: "message_sms",
        id: id.vers_chaine(),
    })?
    .vers_entite()
}

/// Les messages d'un état, ou de tous, les plus récents d'abord, au plus `limite`.
pub async fn lister(
    cnx: &mut Connexion,
    etat: Option<EtatMessage>,
    limite: i64,
    contexte: &Contexte,
) -> Resultat<Vec<Message>> {
    sqlx::query_as::<_, LigneMessage>(selectionner_message!(
        "WHERE etablissement_id = ?1 AND (?2 IS NULL OR etat = ?2)
         ORDER BY cree_le DESC, id DESC LIMIT ?3"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(etat.map(|e| e.nom()))
    .bind(limite)
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneMessage::vers_entite)
    .collect()
}

/// Nombre de messages de chaque état qui en a.
pub async fn compter_par_etat(
    cnx: &mut Connexion,
    contexte: &Contexte,
) -> Resultat<Vec<(EtatMessage, i64)>> {
    let lignes: Vec<(String, i64)> = sqlx::query_as(
        "SELECT etat, count(*) FROM message_sms WHERE etablissement_id = ?1 GROUP BY etat",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?;
    lignes
        .into_iter()
        .map(|(etat, n)| {
            EtatMessage::depuis_nom(&etat)
                .map(|e| (e, n))
                .ok_or_else(|| ErreurBase::illisible("message_sms", "etat", &etat))
        })
        .collect()
}

/// Messages en attente dont le tour est venu, les plus anciens d'abord, au plus `limite`.
pub async fn a_envoyer(
    cnx: &mut Connexion,
    maintenant: Horodatage,
    limite: i64,
    contexte: &Contexte,
) -> Resultat<Vec<Message>> {
    sqlx::query_as::<_, LigneMessage>(selectionner_message!(
        "WHERE etablissement_id = ?1 AND etat = 'en_attente' AND pas_avant <= ?2
         ORDER BY pas_avant, cree_le, id LIMIT ?3"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(maintenant.millis_epoque())
    .bind(limite)
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneMessage::vers_entite)
    .collect()
}

/// Comme [`a_envoyer`], pour une seule nature : hors des heures d'envoi, les réponses aux
/// consultations partent seules.
pub async fn a_envoyer_de_nature(
    cnx: &mut Connexion,
    maintenant: Horodatage,
    nature: NatureMessage,
    limite: i64,
    contexte: &Contexte,
) -> Resultat<Vec<Message>> {
    sqlx::query_as::<_, LigneMessage>(selectionner_message!(
        "WHERE etablissement_id = ?1 AND etat = 'en_attente' AND pas_avant <= ?2 AND nature = ?4
         ORDER BY pas_avant, cree_le, id LIMIT ?3"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(maintenant.millis_epoque())
    .bind(limite)
    .bind(nature.nom())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneMessage::vers_entite)
    .collect()
}

/// Passe en « expiré » les messages en attente dont l'heure est passée ; rend leur nombre.
pub async fn expirer(
    cnx: &mut Connexion,
    maintenant: Horodatage,
    contexte: &Contexte,
) -> Resultat<u64> {
    Ok(sqlx::query(
        "UPDATE message_sms SET etat = 'expire', mis_a_jour_le = ?1, mis_a_jour_par = ?2,
            identifiant_poste = ?3
         WHERE etablissement_id = ?4 AND etat = 'en_attente' AND expire_le IS NOT NULL
            AND expire_le <= ?1",
    )
    .bind(maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?
    .rows_affected())
}

/// Le message parti le plus récemment sous cette référence, s'il attend encore son accusé : la
/// clé recommence ses références après 255.
pub async fn par_reference(
    cnx: &mut Connexion,
    reference: &str,
    contexte: &Contexte,
) -> Resultat<Option<Message>> {
    sqlx::query_as::<_, LigneMessage>(selectionner_message!(
        "WHERE etablissement_id = ?1 AND reference = ?2 AND etat = 'envoye'
         ORDER BY envoye_le DESC LIMIT 1"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(reference)
    .fetch_optional(cnx)
    .await?
    .map(LigneMessage::vers_entite)
    .transpose()
}

/// Messages partis depuis cet instant (débit) et SMS qu'ils ont coûtés (plafond).
pub async fn partis_depuis(
    cnx: &mut Connexion,
    depuis: Horodatage,
    contexte: &Contexte,
) -> Resultat<(i64, i64)> {
    Ok(sqlx::query_as(
        "SELECT count(*), coalesce(sum(sms), 0) FROM message_sms
         WHERE etablissement_id = ?1 AND envoye_le IS NOT NULL AND envoye_le >= ?2",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(depuis.millis_epoque())
    .fetch_one(cnx)
    .await?)
}

#[derive(FromRow)]
struct LigneEtatCanal {
    credit: Option<i64>,
    credit_texte: String,
    credit_lu_le: Option<i64>,
    arretee: i64,
    raison: Option<String>,
    echecs_credit: i64,
    cle_absente: i64,
    relances_le: Option<String>,
}

/// État du canal des SMS ; l'état neuf s'il n'a jamais été enregistré.
pub async fn etat_canal(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<EtatCanal> {
    let t = "etat_canal_sms";
    let Some(l) = sqlx::query_as::<_, LigneEtatCanal>(
        "SELECT credit, credit_texte, credit_lu_le, arretee, raison, echecs_credit, cle_absente,
            relances_le
         FROM etat_canal_sms WHERE etablissement_id = ?1",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    else {
        return Ok(EtatCanal::default());
    };
    Ok(EtatCanal {
        credit: l.credit,
        credit_texte: l.credit_texte,
        credit_lu_le: l.credit_lu_le.map(Horodatage::depuis_millis_epoque),
        arretee: crate::depots::booleen(l.arretee),
        raison: l
            .raison
            .as_deref()
            .map(|r| {
                RaisonArret::depuis_nom(r).ok_or_else(|| ErreurBase::illisible(t, "raison", r))
            })
            .transpose()?,
        echecs_credit: u8::try_from(l.echecs_credit)
            .map_err(|_| ErreurBase::illisible(t, "echecs_credit", l.echecs_credit))?,
        cle_absente: crate::depots::booleen(l.cle_absente),
        relances_le: l
            .relances_le
            .as_deref()
            .map(|d| {
                gesco_core::commun::DateCivile::analyser(d)
                    .ok_or_else(|| ErreurBase::illisible(t, "relances_le", d))
            })
            .transpose()?,
    })
}

/// Enregistre l'état du canal des SMS.
pub async fn enregistrer_etat_canal(
    cnx: &mut Connexion,
    etat: &EtatCanal,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "INSERT INTO etat_canal_sms (etablissement_id, credit, credit_texte, credit_lu_le,
            arretee, raison, echecs_credit, cle_absente, relances_le, mis_a_jour_le,
            mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?12, ?9, ?10, ?11)
         ON CONFLICT (etablissement_id) DO UPDATE SET credit = excluded.credit,
            credit_texte = excluded.credit_texte, credit_lu_le = excluded.credit_lu_le,
            arretee = excluded.arretee, raison = excluded.raison,
            echecs_credit = excluded.echecs_credit, cle_absente = excluded.cle_absente,
            relances_le = excluded.relances_le,
            mis_a_jour_le = excluded.mis_a_jour_le, mis_a_jour_par = excluded.mis_a_jour_par,
            identifiant_poste = excluded.identifiant_poste",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(etat.credit)
    .bind(&etat.credit_texte)
    .bind(etat.credit_lu_le.map(|h| h.millis_epoque()))
    .bind(i64::from(etat.arretee))
    .bind(etat.raison.map(|r| r.nom()))
    .bind(i64::from(etat.echecs_credit))
    .bind(i64::from(etat.cle_absente))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(etat.relances_le.map(|d| d.to_string()))
    .execute(cnx)
    .await?;
    Ok(())
}

/// Messages d'un fait, de tout état.
pub async fn par_fait(
    cnx: &mut Connexion,
    cle_fait: &str,
    contexte: &Contexte,
) -> Resultat<Vec<Message>> {
    sqlx::query_as::<_, LigneMessage>(selectionner_message!(
        "WHERE etablissement_id = ?1 AND cle_fait = ?2 ORDER BY cree_le, id"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(cle_fait)
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneMessage::vers_entite)
    .collect()
}

/// Messages dont la clé de fait commence ainsi (les relances successives d'une échéance).
pub async fn par_prefixe(
    cnx: &mut Connexion,
    prefixe: &str,
    contexte: &Contexte,
) -> Resultat<Vec<Message>> {
    sqlx::query_as::<_, LigneMessage>(selectionner_message!(
        "WHERE etablissement_id = ?1 AND substr(cle_fait, 1, length(?2)) = ?2 ORDER BY cree_le, id"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(prefixe)
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(LigneMessage::vers_entite)
    .collect()
}
