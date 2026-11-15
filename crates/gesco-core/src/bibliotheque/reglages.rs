//! Réglages de la bibliothèque de cours : le module lui-même (`module.bibliotheque`, porté par
//! [`crate::core::ModuleEcole::Bibliotheque`]), le suivi des élèves et la taille conseillée
//! d'une vidéo.

/// Suivi des élèves par leurs maîtres : allumé à la livraison, il ne commande rien avant que
/// les élèves lisent les cours sur leurs appareils, et ne bloque jamais.
pub const SUIVI_ELEVES: &str = "bibliotheque.suivi_eleves";

/// Taille conseillée d'une vidéo, en mégaoctets : au-delà, un avertissement, jamais un refus.
pub const TAILLE_VIDEO_CONSEILLEE: &str = "bibliotheque.taille_video_conseillee";

/// Taille conseillée à la livraison : une vingtaine de minutes filmées par un téléphone.
pub const TAILLE_VIDEO_CONSEILLEE_DEFAUT: i64 = 200;

/// Bornes de la taille conseillée : de 10 Mo à 10 Go.
pub const TAILLE_VIDEO_CONSEILLEE_BORNES: std::ops::RangeInclusive<i64> = 10..=10_000;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_taille_livree_est_dans_ses_bornes() {
        assert!(TAILLE_VIDEO_CONSEILLEE_BORNES.contains(&TAILLE_VIDEO_CONSEILLEE_DEFAUT));
    }
}
