//! NOTIFICATION (SPEC_PHASE_5, blueprint §25.3 à §25.6) : les messages aux familles. Un numéro sait
//! s'il est joignable ; un responsable refuse un type de message sans perdre les autres ; un
//! message va au premier numéro joignable de sa liste.

pub mod gsm7;
pub mod message;
pub mod modele;
pub mod numero;
pub mod reglages;

use serde::{Deserialize, Serialize};

use crate::commun::Id;
use crate::scolarite::Responsable;

pub use message::{ErreurEnvoi, EtatMessage, Message, NatureMessage};
pub use numero::{EtatJoignabilite, Joignabilite};

/// Type de message aux familles, objet du consentement (§25.6) : un parent qui ne veut plus de
/// relances garde l'avis de bulletin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum TypeMessage {
    /// Bulletin à retirer.
    Bulletin,
    /// Relance d'impayé.
    Impaye,
    /// Absence du jour.
    Absence,
    /// Convocation.
    Convocation,
}

impl TypeMessage {
    /// Tous les types, dans l'ordre d'affichage.
    pub const TOUS: [Self; 4] = [
        Self::Bulletin,
        Self::Impaye,
        Self::Absence,
        Self::Convocation,
    ];

    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Bulletin => "bulletin",
            Self::Impaye => "impaye",
            Self::Absence => "absence",
            Self::Convocation => "convocation",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::TOUS.into_iter().find(|t| t.nom() == nom)
    }
}

/// Consentement d'un responsable à un type de message. Sans ligne enregistrée, il accepte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Consentement {
    /// Responsable.
    pub responsable_id: Id<Responsable>,
    /// Type de message.
    pub type_message: TypeMessage,
    /// Faux : il ne reçoit pas ce type de message.
    pub accepte: bool,
}

/// Consentement de chaque type, d'après les lignes enregistrées d'un responsable ; un type sans
/// ligne est accepté.
pub fn consentements_complets(
    responsable_id: Id<Responsable>,
    enregistres: &[Consentement],
) -> Vec<Consentement> {
    TypeMessage::TOUS
        .into_iter()
        .map(|type_message| Consentement {
            responsable_id,
            type_message,
            accepte: enregistres
                .iter()
                .find(|c| c.responsable_id == responsable_id && c.type_message == type_message)
                .is_none_or(|c| c.accepte),
        })
        .collect()
}

/// Vrai si un responsable reçoit un type de message : la case « reçoit les notifications » de son
/// lien avec l'élève, puis son consentement à ce type.
pub fn recoit(
    type_message: TypeMessage,
    recoit_notifications: bool,
    responsable_id: Id<Responsable>,
    consentements: &[Consentement],
) -> bool {
    recoit_notifications
        && consentements
            .iter()
            .find(|c| c.responsable_id == responsable_id && c.type_message == type_message)
            .is_none_or(|c| c.accepte)
}

/// Numéro où envoyer un message : le premier de la liste du responsable qui peut recevoir. Un
/// numéro sans état connu n'est pas encore vérifié, il peut recevoir.
pub fn numero_destinataire<'a>(
    telephones: &'a [String],
    joignabilites: &[Joignabilite],
) -> Option<&'a String> {
    telephones.iter().find(|numero| {
        joignabilites
            .iter()
            .find(|j| &j.numero == *numero)
            .is_none_or(Joignabilite::peut_recevoir)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u8) -> Id<Responsable> {
        let mut octets = [0; 16];
        octets[15] = n;
        Id::depuis_octets(octets)
    }

    #[test]
    fn un_type_sans_ligne_est_accepte() {
        let r = id(1);
        let refus = [Consentement {
            responsable_id: r,
            type_message: TypeMessage::Impaye,
            accepte: false,
        }];
        let complets = consentements_complets(r, &refus);
        assert_eq!(complets.len(), 4);
        assert!(
            complets
                .iter()
                .all(|c| c.accepte == (c.type_message != TypeMessage::Impaye))
        );
        assert!(recoit(TypeMessage::Bulletin, true, r, &refus));
        assert!(!recoit(TypeMessage::Impaye, true, r, &refus));
    }

    #[test]
    fn la_case_des_notifications_passe_avant_le_consentement() {
        let r = id(2);
        assert!(!recoit(TypeMessage::Bulletin, false, r, &[]));
        assert!(recoit(TypeMessage::Absence, true, r, &[]));
    }

    #[test]
    fn le_premier_numero_qui_peut_recevoir() {
        let telephones = vec!["+22376000001".to_owned(), "+22366000002".to_owned()];
        assert_eq!(numero_destinataire(&telephones, &[]), telephones.first());
        let mort = Joignabilite {
            numero: "+22376000001".to_owned(),
            etat: EtatJoignabilite::Injoignable,
            echecs_consecutifs: 3,
        };
        assert_eq!(
            numero_destinataire(&telephones, std::slice::from_ref(&mort)),
            telephones.get(1)
        );
        let invalide = Joignabilite {
            numero: "+22366000002".to_owned(),
            etat: EtatJoignabilite::Invalide,
            echecs_consecutifs: 0,
        };
        assert_eq!(numero_destinataire(&telephones, &[mort, invalide]), None);
    }

    #[test]
    fn noms_persistes() {
        for t in TypeMessage::TOUS {
            assert_eq!(TypeMessage::depuis_nom(t.nom()), Some(t));
        }
        assert_eq!(TypeMessage::depuis_nom("sanction"), None);
    }
}
