//! Joignabilité des numéros (§25.6) : sans elle, une campagne brûle du crédit sur des numéros
//! morts. L'état se tient par numéro, pas par personne : deux responsables qui partagent un
//! téléphone partagent sa joignabilité.

use serde::{Deserialize, Serialize};

/// Échecs de remise d'affilée après lesquels un numéro ne reçoit plus rien.
pub const ECHECS_AVANT_INJOIGNABLE: u8 = 3;

/// État d'un numéro.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum EtatJoignabilite {
    /// Aucun SMS n'y a encore été remis.
    NonVerifie,
    /// Un accusé de remise est revenu.
    Joignable,
    /// Plusieurs remises ont échoué d'affilée.
    Injoignable,
    /// Le réseau a refusé le numéro.
    Invalide,
}

impl EtatJoignabilite {
    /// Tous les états.
    pub const TOUS: [Self; 4] = [
        Self::NonVerifie,
        Self::Joignable,
        Self::Injoignable,
        Self::Invalide,
    ];

    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::NonVerifie => "non_verifie",
            Self::Joignable => "joignable",
            Self::Injoignable => "injoignable",
            Self::Invalide => "invalide",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::TOUS.into_iter().find(|e| e.nom() == nom)
    }
}

/// Joignabilité d'un numéro canonique.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Joignabilite {
    /// Numéro, sous forme canonique `+223…`.
    pub numero: String,
    /// État.
    pub etat: EtatJoignabilite,
    /// Échecs de remise d'affilée depuis le dernier succès.
    pub echecs_consecutifs: u8,
}

impl Joignabilite {
    /// Un numéro qui n'a encore rien reçu.
    pub fn nouvelle(numero: &str) -> Self {
        Self {
            numero: numero.to_owned(),
            etat: EtatJoignabilite::NonVerifie,
            echecs_consecutifs: 0,
        }
    }

    /// Vrai si un message peut lui être envoyé.
    pub const fn peut_recevoir(&self) -> bool {
        matches!(
            self.etat,
            EtatJoignabilite::NonVerifie | EtatJoignabilite::Joignable
        )
    }

    /// Après un accusé de remise : joignable, les échecs oubliés.
    pub fn apres_remise(&self) -> Self {
        Self {
            numero: self.numero.clone(),
            etat: EtatJoignabilite::Joignable,
            echecs_consecutifs: 0,
        }
    }

    /// Après un échec de remise : injoignable au troisième d'affilée. Un numéro invalide le reste.
    pub fn apres_echec(&self) -> Self {
        let echecs = self.echecs_consecutifs.saturating_add(1);
        let etat = match self.etat {
            EtatJoignabilite::Invalide => EtatJoignabilite::Invalide,
            _ if echecs >= ECHECS_AVANT_INJOIGNABLE => EtatJoignabilite::Injoignable,
            etat => etat,
        };
        Self {
            numero: self.numero.clone(),
            etat,
            echecs_consecutifs: echecs,
        }
    }

    /// Après un refus du réseau sur le format du numéro.
    pub fn apres_refus(&self) -> Self {
        Self {
            numero: self.numero.clone(),
            etat: EtatJoignabilite::Invalide,
            echecs_consecutifs: self.echecs_consecutifs,
        }
    }

    /// Remis à l'essai depuis la fiche du responsable : il reçoit de nouveau.
    pub fn remise_a_l_essai(&self) -> Self {
        Self::nouvelle(&self.numero)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trois_echecs_d_affilee_rendent_injoignable() {
        let n = Joignabilite::nouvelle("+22376000001");
        assert!(n.peut_recevoir());
        let deux = n.apres_echec().apres_echec();
        assert_eq!(deux.etat, EtatJoignabilite::NonVerifie);
        assert!(deux.peut_recevoir());
        let trois = deux.apres_echec();
        assert_eq!(trois.etat, EtatJoignabilite::Injoignable);
        assert!(!trois.peut_recevoir());
    }

    #[test]
    fn une_remise_efface_les_echecs() {
        let n = Joignabilite::nouvelle("+22376000001")
            .apres_echec()
            .apres_echec()
            .apres_remise();
        assert_eq!(n.etat, EtatJoignabilite::Joignable);
        assert_eq!(n.echecs_consecutifs, 0);
        assert_eq!(n.apres_echec().etat, EtatJoignabilite::Joignable);
    }

    #[test]
    fn invalide_jusqu_a_la_remise_a_l_essai() {
        let n = Joignabilite::nouvelle("+22376000001").apres_refus();
        assert_eq!(n.etat, EtatJoignabilite::Invalide);
        assert_eq!(n.apres_echec().etat, EtatJoignabilite::Invalide);
        let essai = n.remise_a_l_essai();
        assert_eq!(essai, Joignabilite::nouvelle("+22376000001"));
    }

    #[test]
    fn noms_persistes() {
        for e in EtatJoignabilite::TOUS {
            assert_eq!(EtatJoignabilite::depuis_nom(e.nom()), Some(e));
        }
    }
}
