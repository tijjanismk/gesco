//! Modules de l'école : ce que chaque école active ou non, pour tout le monde. Un module éteint
//! disparaît de la barre latérale et du tableau de bord ; ses données restent, et il revient tel
//! quel quand on le rallume. Élèves, inscriptions, notes, bulletins et caisse ne s'éteignent pas.

use serde::{Deserialize, Serialize};

/// Module que l'école active ou non.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ModuleEcole {
    /// Messages aux familles par SMS (SPEC_PHASE_5) : éteint, rien ne part ni n'arrive.
    Sms,
    /// Cantine scolaire (SPEC_PHASE_8).
    Cantine,
    /// Personnel, heures, congés et paie (SPEC_PHASE_6 et 7).
    Personnel,
    /// Dépenses, autres recettes, fournisseurs, comptes et trésorerie (SPEC_PHASE_8).
    Tresorerie,
    /// Emploi du temps (SPEC_PHASE_8).
    EmploiDuTemps,
    /// Discipline (SPEC_VERSION_1_1).
    Discipline,
}

impl ModuleEcole {
    /// Tous les modules, dans l'ordre de l'écran des paramètres.
    pub const TOUS: [Self; 6] = [
        Self::Sms,
        Self::Cantine,
        Self::Personnel,
        Self::Tresorerie,
        Self::EmploiDuTemps,
        Self::Discipline,
    ];

    /// Réglage qui l'active. La cantine et les SMS gardent le leur, antérieur à cet écran.
    pub const fn cle_reglage(&self) -> &'static str {
        match self {
            Self::Sms => "sms.actif",
            Self::Cantine => "cantine.active",
            Self::Personnel => "module.personnel",
            Self::Tresorerie => "module.tresorerie",
            Self::EmploiDuTemps => "module.emploi_du_temps",
            Self::Discipline => "module.discipline",
        }
    }

    /// État à la livraison : les SMS et la cantine attendent que l'école les veuille ; ce qui
    /// existait avant cet écran reste allumé.
    pub const fn actif_par_defaut(&self) -> bool {
        !matches!(self, Self::Sms | Self::Cantine)
    }

    /// Module dont un réglage est l'interrupteur.
    pub fn du_reglage(cle: &str) -> Option<Self> {
        Self::TOUS.into_iter().find(|m| m.cle_reglage() == cle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chaque_module_a_son_reglage() {
        for m in ModuleEcole::TOUS {
            assert_eq!(ModuleEcole::du_reglage(m.cle_reglage()), Some(m));
            // Une clé à un seul point, comme les autres réglages.
            assert_eq!(m.cle_reglage().matches('.').count(), 1);
        }
        assert!(!ModuleEcole::Sms.actif_par_defaut());
        assert!(ModuleEcole::Personnel.actif_par_defaut());
    }
}
