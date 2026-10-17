//! Numéros et consentement (SPEC_PHASE_5 §2, blueprint §25.6) : sur la fiche d'un responsable,
//! l'état de chacun de ses numéros et les types de messages qu'il accepte. Un parent qui ne veut
//! plus de relances en est retiré sans perdre l'avis de bulletin ; un numéro injoignable se remet
//! à l'essai une fois corrigé.

use gesco_core::commun::Id;
use gesco_core::core::{Operation, Permission, TypeEvenement};
use gesco_core::notification::{Consentement, Joignabilite, TypeMessage, consentements_complets};
use gesco_core::scolarite::Responsable;
use gesco_db::Contexte;
use gesco_db::depots::{messages, personnes, role};
use gesco_db::unite::Durabilite;
use serde::{Deserialize, Serialize};

use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Ce qu'un responsable reçoit, tel que sa fiche le montre.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct MessagesResponsable {
    /// Responsable.
    pub responsable_id: Id<Responsable>,
    /// Chacun de ses numéros, dans l'ordre de sa fiche, avec son état.
    pub numeros: Vec<Joignabilite>,
    /// Son consentement à chaque type de message.
    pub consentements: Vec<Consentement>,
    /// Vrai si le compte connecté peut les modifier.
    pub modifiable: bool,
}

/// Responsable dont on lit les messages.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeMessagesResponsable {
    /// Responsable.
    pub responsable_id: Id<Responsable>,
}

/// Consentement à enregistrer.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeConsentement {
    /// Responsable.
    pub responsable_id: Id<Responsable>,
    /// Type de message.
    pub type_message: TypeMessage,
    /// Faux : il ne le reçoit plus.
    pub accepte: bool,
}

/// Numéro d'un responsable à remettre à l'essai.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeRemiseALEssai {
    /// Responsable.
    pub responsable_id: Id<Responsable>,
    /// Un de ses numéros, sous forme canonique.
    pub numero: String,
}

/// Service des numéros et du consentement.
pub struct ServiceMessages<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceMessages<'n> {
    /// Service sur le noyau.
    pub fn nouveau(noyau: &'n Noyau) -> Self {
        Self { noyau }
    }

    async fn connexion(&self) -> Resultat<gesco_db::ConnexionEmpruntee> {
        Ok(self
            .noyau
            .base()
            .pool()
            .acquire()
            .await
            .map_err(gesco_db::ErreurBase::from)?)
    }

    async fn lire_avec(
        cnx: &mut gesco_db::Connexion,
        session: &Session,
        responsable_id: Id<Responsable>,
        contexte: &Contexte,
    ) -> Resultat<MessagesResponsable> {
        let (_, personne) = personnes::responsable(cnx, responsable_id, contexte).await?;
        let connues = messages::joignabilites(cnx, &personne.telephones, contexte).await?;
        let numeros = personne
            .telephones
            .iter()
            .map(|n| {
                connues
                    .iter()
                    .find(|j| &j.numero == n)
                    .cloned()
                    .unwrap_or_else(|| Joignabilite::nouvelle(n))
            })
            .collect();
        let enregistres = messages::consentements(cnx, responsable_id, contexte).await?;
        let permissions = role::permissions_de(cnx, session.utilisateur_id, contexte).await?;
        Ok(MessagesResponsable {
            responsable_id,
            numeros,
            consentements: consentements_complets(responsable_id, &enregistres),
            modifiable: permissions.contains(&Permission::STUDENTS_UPDATE),
        })
    }

    /// Les numéros et les consentements d'un responsable.
    pub async fn lire(
        &self,
        session: &Session,
        demande: &DemandeMessagesResponsable,
    ) -> Resultat<MessagesResponsable> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        Self::lire_avec(&mut cnx, session, demande.responsable_id, &contexte).await
    }

    /// Enregistre le consentement d'un responsable à un type de message.
    pub async fn definir_consentement(
        &self,
        session: &Session,
        demande: &DemandeConsentement,
    ) -> Resultat<MessagesResponsable> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        personnes::responsable(unite.connexion(), demande.responsable_id, &contexte).await?;
        let avant = consentements_complets(
            demande.responsable_id,
            &messages::consentements(unite.connexion(), demande.responsable_id, &contexte).await?,
        )
        .into_iter()
        .find(|c| c.type_message == demande.type_message)
        .is_none_or(|c| c.accepte);
        if avant != demande.accepte {
            let consentement = Consentement {
                responsable_id: demande.responsable_id,
                type_message: demande.type_message,
                accepte: demande.accepte,
            };
            messages::definir_consentement(unite.connexion(), &consentement, &contexte).await?;
            let id = demande.responsable_id.vers_chaine();
            unite.auditer(
                "consentement_message",
                &id,
                Operation::Modification,
                Some(serde_json::json!({ "type_message": demande.type_message, "accepte": avant })),
                Some(serde_json::to_value(consentement)?),
                "messages.definir_consentement",
            );
            unite.emettre(
                TypeEvenement::ConsentementModifie,
                "responsable",
                &id,
                serde_json::json!({
                    "type_message": demande.type_message,
                    "accepte": demande.accepte,
                }),
            );
        }
        let lus = Self::lire_avec(
            unite.connexion(),
            session,
            demande.responsable_id,
            &contexte,
        )
        .await?;
        self.noyau.terminer(unite).await?;
        Ok(lus)
    }

    /// Remet à l'essai un numéro injoignable ou invalide d'un responsable : il reçoit de nouveau.
    pub async fn remettre_a_l_essai(
        &self,
        session: &Session,
        demande: &DemandeRemiseALEssai,
    ) -> Resultat<MessagesResponsable> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        let (_, personne) =
            personnes::responsable(unite.connexion(), demande.responsable_id, &contexte).await?;
        if !personne.telephones.contains(&demande.numero) {
            return Err(regle("messages.numero_inconnu"));
        }
        let avant = messages::joignabilite(unite.connexion(), &demande.numero, &contexte)
            .await?
            .unwrap_or_else(|| Joignabilite::nouvelle(&demande.numero));
        if !avant.peut_recevoir() {
            let apres = avant.remise_a_l_essai();
            messages::enregistrer_joignabilite(unite.connexion(), &apres, &contexte).await?;
            unite.auditer(
                "joignabilite_numero",
                &apres.numero,
                Operation::Modification,
                Some(serde_json::to_value(&avant)?),
                Some(serde_json::to_value(&apres)?),
                "messages.remettre_a_l_essai",
            );
            unite.emettre(
                TypeEvenement::JoignabiliteReinitialisee,
                "responsable",
                &demande.responsable_id.vers_chaine(),
                serde_json::json!({ "numero": apres.numero, "etat_precedent": avant.etat }),
            );
        }
        let lus = Self::lire_avec(
            unite.connexion(),
            session,
            demande.responsable_id,
            &contexte,
        )
        .await?;
        self.noyau.terminer(unite).await?;
        Ok(lus)
    }
}
