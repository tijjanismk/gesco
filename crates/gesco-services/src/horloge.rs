//! Le temps entre dans les services par une horloge injectée : le noyau ne lit jamais l'heure
//! (R-03) et les tests fixent l'instant.

use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use gesco_core::commun::Horodatage;

/// Source de l'instant courant.
pub trait Horloge: Send + Sync {
    /// Instant courant.
    fn maintenant(&self) -> Horodatage;
}

/// Horloge du système d'exploitation.
#[derive(Debug, Default, Clone, Copy)]
pub struct HorlogeSysteme;

impl Horloge for HorlogeSysteme {
    fn maintenant(&self) -> Horodatage {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
            .unwrap_or(0);
        Horodatage::depuis_millis_epoque(millis)
    }
}

/// Horloge pilotée par les tests.
#[derive(Debug)]
pub struct HorlogeFixe(Mutex<i64>);

impl HorlogeFixe {
    /// Horloge arrêtée sur l'instant donné.
    pub fn a(instant: Horodatage) -> Self {
        Self(Mutex::new(instant.millis_epoque()))
    }

    /// Avance l'horloge du nombre de millisecondes donné.
    pub fn avancer(&self, millis: i64) {
        if let Ok(mut v) = self.0.lock() {
            *v = v.saturating_add(millis);
        }
    }
}

impl Horloge for HorlogeFixe {
    fn maintenant(&self) -> Horodatage {
        let millis = self.0.lock().map(|v| *v).unwrap_or(0);
        Horodatage::depuis_millis_epoque(millis)
    }
}

/// Horloge du système ramenée à un autre jour, l'heure avançant normalement : les essais de bout
/// en bout s'exécutent ainsi toujours au même jour du calendrier scolaire, quel que soit le jour
/// réel.
#[derive(Debug, Clone, Copy)]
pub struct HorlogeDecalee {
    decalage_millis: i64,
}

impl HorlogeDecalee {
    /// Horloge qui vit le jour donné, à l'heure du système.
    pub fn au_jour(jour: gesco_core::commun::DateCivile) -> Self {
        let reel = HorlogeSysteme.maintenant();
        let jours = gesco_core::commun::DateCivile::du_jour(reel)
            .map_or(0, |aujourd_hui| aujourd_hui.jours_jusqua(jour));
        Self {
            decalage_millis: jours.saturating_mul(86_400_000),
        }
    }
}

impl Horloge for HorlogeDecalee {
    fn maintenant(&self) -> Horodatage {
        Horodatage::depuis_millis_epoque(
            HorlogeSysteme
                .maintenant()
                .millis_epoque()
                .saturating_add(self.decalage_millis),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gesco_core::commun::DateCivile;

    #[test]
    fn l_horloge_decalee_vit_le_jour_demande() {
        let jour = DateCivile::nouvelle(2026, 10, 14).unwrap_or_else(|| unreachable!());
        let horloge = HorlogeDecalee::au_jour(jour);
        assert_eq!(DateCivile::du_jour(horloge.maintenant()), Some(jour));
    }
}
