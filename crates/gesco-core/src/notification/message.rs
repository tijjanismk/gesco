//! Message de la file (SPEC_PHASE_5 §3) : un fait et son destinataire, formulés au moment de la
//! mise en file. États : à décider → en attente → envoyé → remis ; en attente → échoué après les
//! essais ; à décider ou en attente → annulé (motif) ; en attente → expiré. Un message parti reste
//! au journal (R-14).

use serde::{Deserialize, Serialize};

use super::TypeMessage;
use crate::commun::{Horodatage, Id};
use crate::core::Etablissement;
use crate::scolarite::{Eleve, Responsable};

/// Essais d'envoi avant qu'un message soit échoué.
pub const ESSAIS_MAX: u8 = 3;

/// Attente avant un nouvel essai, multipliée par le nombre d'essais déjà faits.
pub const ATTENTE_ENTRE_ESSAIS_MS: i64 = 5 * 60 * 1_000;

/// Nature d'un message : ce dont il parle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum NatureMessage {
    /// Bulletin à retirer.
    Bulletin,
    /// Relance d'impayé.
    Impaye,
    /// Absence du jour.
    Absence,
    /// Convocation, préparée puis décidée.
    Convocation,
    /// Avis de sanction, préparé puis décidé.
    Sanction,
    /// Message d'essai de la clé, vers un numéro saisi.
    Essai,
}

impl NatureMessage {
    /// Toutes les natures.
    pub const TOUTES: [Self; 6] = [
        Self::Bulletin,
        Self::Impaye,
        Self::Absence,
        Self::Convocation,
        Self::Sanction,
        Self::Essai,
    ];

    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Bulletin => "bulletin",
            Self::Impaye => "impaye",
            Self::Absence => "absence",
            Self::Convocation => "convocation",
            Self::Sanction => "sanction",
            Self::Essai => "essai",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::TOUTES.into_iter().find(|n| n.nom() == nom)
    }

    /// Type de message dont le consentement décide ; aucun pour un essai.
    pub const fn consentement(&self) -> Option<TypeMessage> {
        match self {
            Self::Bulletin => Some(TypeMessage::Bulletin),
            Self::Impaye => Some(TypeMessage::Impaye),
            Self::Absence => Some(TypeMessage::Absence),
            Self::Convocation | Self::Sanction => Some(TypeMessage::Convocation),
            Self::Essai => None,
        }
    }

    /// Vrai si le message naît « à décider » : une personne le relit avant qu'il parte (§25.6).
    pub const fn a_decider(&self) -> bool {
        matches!(self, Self::Convocation | Self::Sanction)
    }

    /// Durée de vie d'un message en attente, en millisecondes : passé ce délai, il n'a plus de sens.
    /// L'avis d'absence vit jusqu'à la fin du jour, calculée par l'appelant.
    pub const fn duree_de_vie_ms(&self) -> Option<i64> {
        const JOUR: i64 = 24 * 3_600 * 1_000;
        match self {
            Self::Bulletin => Some(15 * JOUR),
            Self::Impaye => Some(7 * JOUR),
            Self::Convocation | Self::Sanction => Some(7 * JOUR),
            Self::Essai => Some(JOUR),
            Self::Absence => None,
        }
    }
}

/// État d'un message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum EtatMessage {
    /// Préparé, il attend qu'une personne le relise et décide de l'envoyer.
    ADecider,
    /// Dans la file, il partira à son tour.
    EnAttente,
    /// Parti ; l'accusé de remise n'est pas revenu.
    Envoye,
    /// L'accusé de remise est revenu.
    Remis,
    /// Les essais ont échoué, ou la remise.
    Echoue,
    /// Annulé avant de partir, avec un motif.
    Annule,
    /// Resté trop longtemps en attente.
    Expire,
}

impl EtatMessage {
    /// Tous les états, dans l'ordre d'affichage.
    pub const TOUS: [Self; 7] = [
        Self::ADecider,
        Self::EnAttente,
        Self::Envoye,
        Self::Remis,
        Self::Echoue,
        Self::Annule,
        Self::Expire,
    ];

    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::ADecider => "a_decider",
            Self::EnAttente => "en_attente",
            Self::Envoye => "envoye",
            Self::Remis => "remis",
            Self::Echoue => "echoue",
            Self::Annule => "annule",
            Self::Expire => "expire",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::TOUS.into_iter().find(|e| e.nom() == nom)
    }

    /// Vrai si le message peut encore être annulé : il n'est pas parti.
    pub const fn annulable(&self) -> bool {
        matches!(self, Self::ADecider | Self::EnAttente)
    }

    /// Vrai si le message peut être renvoyé : il a échoué ou expiré.
    pub const fn renvoyable(&self) -> bool {
        matches!(self, Self::Echoue | Self::Expire)
    }
}

/// Raison d'un échec, clé du message affiché.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ErreurEnvoi {
    /// Pas de réseau, ou la clé ne répond pas.
    Reseau,
    /// Plus de crédit sur la puce.
    Credit,
    /// Le réseau refuse le numéro.
    NumeroRefuse,
    /// Le numéro ne reçoit plus : injoignable ou invalide.
    NumeroInjoignable,
    /// L'accusé dit que le SMS n'a pas été remis.
    NonRemis,
}

impl ErreurEnvoi {
    /// Toutes les erreurs.
    pub const TOUTES: [Self; 5] = [
        Self::Reseau,
        Self::Credit,
        Self::NumeroRefuse,
        Self::NumeroInjoignable,
        Self::NonRemis,
    ];

    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Reseau => "reseau",
            Self::Credit => "credit",
            Self::NumeroRefuse => "numero_refuse",
            Self::NumeroInjoignable => "numero_injoignable",
            Self::NonRemis => "non_remis",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::TOUTES.into_iter().find(|e| e.nom() == nom)
    }

    /// Vrai si l'erreur peut passer en réessayant plus tard.
    pub const fn passagere(&self) -> bool {
        matches!(self, Self::Reseau | Self::Credit)
    }
}

/// Message de la file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Message {
    /// Identifiant technique.
    pub id: Id<Message>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Nature.
    pub nature: NatureMessage,
    /// Clé du fait : un même fait ne met en file qu'un message par destinataire.
    pub cle_fait: String,
    /// Responsable destinataire ; aucun pour un essai.
    pub responsable_id: Option<Id<Responsable>>,
    /// Élève dont il parle.
    pub eleve_id: Option<Id<Eleve>>,
    /// Numéro, sous forme canonique.
    pub numero: String,
    /// Texte en GSM-7, tel qu'il partira.
    pub texte: String,
    /// Nombre de SMS.
    pub sms: u8,
    /// État.
    pub etat: EtatMessage,
    /// Essais d'envoi faits.
    pub essais: u8,
    /// Pas d'envoi avant cet instant (délai de grâce, attente entre deux essais).
    pub pas_avant: Horodatage,
    /// Expire à cet instant s'il n'est pas parti.
    pub expire_le: Option<Horodatage>,
    /// Référence donnée par le canal à l'envoi, pour retrouver l'accusé.
    pub reference: Option<String>,
    /// Envoi.
    pub envoye_le: Option<Horodatage>,
    /// Remise.
    pub remis_le: Option<Horodatage>,
    /// Dernière erreur.
    pub erreur: Option<ErreurEnvoi>,
    /// Motif d'une annulation.
    pub motif_annulation: String,
    /// Mise en file.
    pub cree_le: Horodatage,
}

/// Nouvel état d'un message après une erreur d'envoi : réessayé plus tard tant que l'erreur est
/// passagère et que les essais ne sont pas épuisés, échoué sinon.
pub fn apres_erreur(
    essais_faits: u8,
    erreur: ErreurEnvoi,
    maintenant: Horodatage,
) -> (EtatMessage, Horodatage) {
    if erreur.passagere() && essais_faits < ESSAIS_MAX {
        let attente = ATTENTE_ENTRE_ESSAIS_MS * i64::from(essais_faits.max(1));
        (
            EtatMessage::EnAttente,
            Horodatage::depuis_millis_epoque(maintenant.millis_epoque() + attente),
        )
    } else {
        (EtatMessage::Echoue, maintenant)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noms_persistes() {
        for n in NatureMessage::TOUTES {
            assert_eq!(NatureMessage::depuis_nom(n.nom()), Some(n));
        }
        for e in EtatMessage::TOUS {
            assert_eq!(EtatMessage::depuis_nom(e.nom()), Some(e));
        }
        for e in ErreurEnvoi::TOUTES {
            assert_eq!(ErreurEnvoi::depuis_nom(e.nom()), Some(e));
        }
    }

    #[test]
    fn seules_convocation_et_sanction_attendent_une_decision() {
        let a_decider: Vec<_> = NatureMessage::TOUTES
            .into_iter()
            .filter(NatureMessage::a_decider)
            .collect();
        assert_eq!(
            a_decider,
            [NatureMessage::Convocation, NatureMessage::Sanction]
        );
        assert_eq!(NatureMessage::Essai.consentement(), None);
        assert_eq!(
            NatureMessage::Sanction.consentement(),
            Some(TypeMessage::Convocation)
        );
    }

    #[test]
    fn trois_essais_puis_echec() {
        let t = Horodatage::depuis_millis_epoque(1_000_000);
        let (etat, pas_avant) = apres_erreur(1, ErreurEnvoi::Reseau, t);
        assert_eq!(etat, EtatMessage::EnAttente);
        assert_eq!(
            pas_avant.millis_epoque(),
            1_000_000 + ATTENTE_ENTRE_ESSAIS_MS
        );
        let (etat, pas_avant) = apres_erreur(2, ErreurEnvoi::Credit, t);
        assert_eq!(etat, EtatMessage::EnAttente);
        assert_eq!(
            pas_avant.millis_epoque(),
            1_000_000 + 2 * ATTENTE_ENTRE_ESSAIS_MS
        );
        assert_eq!(
            apres_erreur(3, ErreurEnvoi::Reseau, t).0,
            EtatMessage::Echoue
        );
        assert_eq!(
            apres_erreur(1, ErreurEnvoi::NumeroRefuse, t).0,
            EtatMessage::Echoue
        );
    }

    #[test]
    fn annuler_et_renvoyer() {
        assert!(EtatMessage::ADecider.annulable());
        assert!(EtatMessage::EnAttente.annulable());
        assert!(!EtatMessage::Envoye.annulable());
        assert!(EtatMessage::Echoue.renvoyable());
        assert!(EtatMessage::Expire.renvoyable());
        assert!(!EtatMessage::Remis.renvoyable());
    }
}
