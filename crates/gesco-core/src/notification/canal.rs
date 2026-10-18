//! État du canal d'envoi (SPEC_PHASE_5 §4, C-5) : un prépayé vide ne produit pas d'erreur
//! explicite, les messages échouent et personne ne le sait. Le crédit se lit donc chaque heure ;
//! sous le seuil de l'école, ou après cinq échecs de crédit d'affilée, la file s'arrête et la
//! direction est avertie ; elle repart quand le crédit revient, ou à la main.

use serde::{Deserialize, Serialize};

use crate::commun::Horodatage;

/// Échecs de crédit d'affilée qui arrêtent la file.
pub const ECHECS_CREDIT_MAX: u8 = 5;

/// Intervalle de lecture du crédit pendant que la file tourne.
pub const LECTURE_CREDIT_MS: i64 = 3_600_000;

/// Intervalle de lecture du crédit quand la file est arrêtée : on guette la recharge.
pub const LECTURE_CREDIT_ARRET_MS: i64 = 600_000;

/// Raison d'un arrêt de la file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum RaisonArret {
    /// Le crédit lu est sous le seuil.
    CreditBas,
    /// Cinq envois d'affilée ont échoué faute de crédit.
    EchecsCredit,
}

impl RaisonArret {
    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::CreditBas => "credit_bas",
            Self::EchecsCredit => "echecs_credit",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        [Self::CreditBas, Self::EchecsCredit]
            .into_iter()
            .find(|r| r.nom() == nom)
    }
}

/// État du canal d'un établissement.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct EtatCanal {
    /// Dernier crédit lu, en francs.
    pub credit: Option<i64>,
    /// Réponse de l'opérateur, telle quelle.
    pub credit_texte: String,
    /// Dernière lecture du crédit.
    pub credit_lu_le: Option<Horodatage>,
    /// Vrai si la file est arrêtée.
    pub arretee: bool,
    /// Raison de l'arrêt.
    pub raison: Option<RaisonArret>,
    /// Échecs de crédit d'affilée.
    pub echecs_credit: u8,
    /// Vrai si la clé ne répondait pas au dernier passage.
    pub cle_absente: bool,
}

impl EtatCanal {
    /// Vrai si le crédit est à relire : jamais lu, ou lu depuis plus d'une heure (dix minutes si
    /// la file est arrêtée).
    pub fn lecture_due(&self, maintenant: Horodatage) -> bool {
        let intervalle = if self.arretee {
            LECTURE_CREDIT_ARRET_MS
        } else {
            LECTURE_CREDIT_MS
        };
        self.credit_lu_le
            .is_none_or(|lu| maintenant.millis_epoque() - lu.millis_epoque() >= intervalle)
    }

    /// Après une lecture du crédit : sous le seuil, la file s'arrête (rend vrai s'il faut avertir) ;
    /// arrêtée et de nouveau au-dessus du seuil, elle repart. Un seuil nul n'arrête jamais.
    #[must_use]
    pub fn apres_lecture(
        &self,
        montant: Option<i64>,
        texte: &str,
        seuil: i64,
        maintenant: Horodatage,
    ) -> (Self, bool) {
        let mut etat = Self {
            credit: montant.or(self.credit),
            credit_texte: texte.to_owned(),
            credit_lu_le: Some(maintenant),
            ..self.clone()
        };
        let Some(montant) = montant else {
            return (etat, false);
        };
        if seuil > 0 && montant < seuil {
            if etat.arretee {
                return (etat, false);
            }
            etat.arretee = true;
            etat.raison = Some(RaisonArret::CreditBas);
            return (etat, true);
        }
        if etat.arretee {
            etat = etat.reprise();
        }
        (etat, false)
    }

    /// Après un envoi refusé faute de crédit : au cinquième d'affilée, la file s'arrête (rend vrai
    /// s'il faut avertir).
    #[must_use]
    pub fn apres_erreur_credit(&self) -> (Self, bool) {
        let mut etat = Self {
            echecs_credit: self.echecs_credit.saturating_add(1),
            ..self.clone()
        };
        if etat.echecs_credit >= ECHECS_CREDIT_MAX && !etat.arretee {
            etat.arretee = true;
            etat.raison = Some(RaisonArret::EchecsCredit);
            return (etat, true);
        }
        (etat, false)
    }

    /// Après un envoi réussi : les échecs de crédit sont oubliés.
    #[must_use]
    pub fn apres_envoi(&self) -> Self {
        Self {
            echecs_credit: 0,
            ..self.clone()
        }
    }

    /// La file repart.
    #[must_use]
    pub fn reprise(&self) -> Self {
        Self {
            arretee: false,
            raison: None,
            echecs_credit: 0,
            ..self.clone()
        }
    }
}

/// Texte de l'alerte envoyée au numéro de la direction quand la file s'arrête, en GSM-7.
pub fn texte_alerte(ecole: &str, raison: RaisonArret, credit: Option<i64>) -> String {
    let cause = match (raison, credit) {
        (RaisonArret::CreditBas, Some(c)) => format!("credit SMS bas ({c} F)"),
        (RaisonArret::CreditBas, None) | (RaisonArret::EchecsCredit, _) => {
            "plus de credit SMS".to_owned()
        }
    };
    super::gsm7::translitterer(&format!(
        "{ecole} : {cause}. Les SMS de l'ecole sont arretes. Rechargez la puce de la cle."
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(minutes: i64) -> Horodatage {
        Horodatage::depuis_millis_epoque(minutes * 60_000)
    }

    #[test]
    fn le_credit_se_relit_chaque_heure_ou_dix_minutes_a_l_arret() {
        let neuf = EtatCanal::default();
        assert!(neuf.lecture_due(t(0)));
        let (lu, _) = neuf.apres_lecture(Some(1_000), "", 200, t(0));
        assert!(!lu.lecture_due(t(59)));
        assert!(lu.lecture_due(t(60)));
        let (arrete, _) = lu.apres_lecture(Some(100), "", 200, t(60));
        assert!(arrete.lecture_due(t(70)));
    }

    #[test]
    fn sous_le_seuil_la_file_s_arrete_une_fois_puis_repart() {
        let (arrete, alerter) = EtatCanal::default().apres_lecture(Some(150), "", 200, t(0));
        assert!(arrete.arretee && alerter);
        assert_eq!(arrete.raison, Some(RaisonArret::CreditBas));
        let (encore, alerter) = arrete.apres_lecture(Some(100), "", 200, t(10));
        assert!(encore.arretee && !alerter, "une seule alerte");
        let (reparti, alerter) = encore.apres_lecture(Some(2_000), "", 200, t(20));
        assert!(!reparti.arretee && !alerter);
        assert_eq!(reparti.raison, None);
        let (jamais, _) = EtatCanal::default().apres_lecture(Some(0), "", 0, t(0));
        assert!(!jamais.arretee, "seuil nul");
    }

    #[test]
    fn cinq_echecs_de_credit_arretent_la_file() {
        let mut etat = EtatCanal::default();
        for _ in 0..4 {
            let (suivant, alerter) = etat.apres_erreur_credit();
            assert!(!suivant.arretee && !alerter);
            etat = suivant;
        }
        assert_eq!(etat.apres_envoi().echecs_credit, 0);
        let (arrete, alerter) = etat.apres_erreur_credit();
        assert!(arrete.arretee && alerter);
        assert_eq!(arrete.raison, Some(RaisonArret::EchecsCredit));
        assert_eq!(arrete.reprise(), EtatCanal::default());
    }

    #[test]
    fn alerte_sans_accent() {
        let texte = texte_alerte("École Askia", RaisonArret::CreditBas, Some(150));
        assert_eq!(
            texte,
            "Ecole Askia : credit SMS bas (150 F). Les SMS de l'ecole sont arretes. Rechargez la puce de la cle."
        );
        assert!(super::super::gsm7::nombre_sms(&texte) == 1);
    }
}
