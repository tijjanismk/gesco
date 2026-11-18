//! Portail des élèves sur le Wi-Fi de l'école (SPEC_PHASE_12_LOT_4) : ses réglages, leurs valeurs
//! à la livraison et leurs bornes. Règles pures, sans E/S (R-03).

/// « Les élèves lisent les cours sur leurs appareils » : éteint à la livraison ; la direction
/// l'allume quand le Wi-Fi et les billets sont prêts.
pub const ELEVES: &str = "portail.eleves";

/// Ce que le maître voit du suivi : `eleve` (qui a lu quoi) ou `classe` (nombres sans noms).
pub const SUIVI_GRAIN: &str = "portail.suivi_grain";

/// Minutes sans activité au bout desquelles un élève entré avec son code doit entrer de nouveau.
pub const DUREE_SESSION_MIN: &str = "portail.duree_session_min";

/// Les vidéos de l'école se regardent aussi sur les appareils des élèves.
pub const VIDEOS: &str = "portail.videos";

/// Le PDF d'une leçon s'enregistre depuis le portail.
pub const PDF: &str = "portail.pdf";

/// Durée d'une session à la livraison : un cours dure une à deux heures.
pub const DUREE_SESSION_DEFAUT: i64 = 120;

/// Bornes de la durée d'une session : de quinze minutes à une journée.
pub const DUREE_SESSION_BORNES: std::ops::RangeInclusive<i64> = 15..=1_440;

/// Port du portail quand il est libre : l'adresse à taper reste courte.
pub const PORT_PREFERE: u16 = 80;

/// Port du portail quand le 80 est pris.
pub const PORT_SECOURS: u16 = 47_801;

/// Codes faux dans la journée qui bloquent un matricule jusqu'au lendemain ; « Lire sans code »
/// reste possible.
pub const CODES_FAUX_MAX: i64 = 5;

/// Vérifications de code menées en même temps, au plus : le reste attend son tour.
pub const VERIFICATIONS_SIMULTANEES: usize = 4;

/// Billets par page A4.
pub const BILLETS_PAR_PAGE: usize = 10;

// Charge (ADR « charge du portail des élèves ») : la gestion d'abord. Valeurs de départ, à régler
// d'après la mesure de l'outil `soixante_eleves_lisent_pendant_que_deux_postes_travaillent`.

/// Requêtes du portail servies en même temps, au plus (pages, fichiers et API).
pub const REQUETES_SIMULTANEES: usize = 48;

/// Morceaux de vidéo lus et envoyés en même temps, au plus.
pub const VIDEOS_SIMULTANEES: usize = 4;

/// PDF fabriqués en même temps, au plus ; un PDF déjà fabriqué se sert sans attendre.
pub const PDF_SIMULTANES: usize = 2;

/// Attente d'une place, en millisecondes, pour l'API et la page d'entrée : au-delà, l'élève
/// lit « Beaucoup d'élèves sont connectés » et la page se relance seule.
pub const ATTENTE_PLACE_MS: u64 = 2_000;

/// Attente d'une place pour un fichier des pages ou un morceau de vidéo : ce sont des lectures
/// courtes, et un fichier des pages refusé laisserait l'écran blanc.
pub const ATTENTE_PLACE_FICHIER_MS: u64 = 10_000;

/// Attente d'une place pour fabriquer un PDF : une fabrication dure quelques secondes, et le
/// téléphone attend son téléchargement.
pub const ATTENTE_PLACE_PDF_MS: u64 = 30_000;

/// Secondes après lesquelles la page se relance seule quand le poste est occupé
/// (`Retry-After`) ; les écrans de l'élève réessaient au même rythme.
pub const RELANCE_S: u64 = 5;

/// Jour d'un instant, `AAAA-MM-JJ` (le Mali vit à l'heure universelle).
pub fn jour_de(instant: crate::commun::Horodatage) -> String {
    instant.vers_rfc3339().chars().take(10).collect()
}

/// Matricule saisi sous sa forme comparable : sans espace autour, en majuscules.
pub fn normaliser_matricule(saisie: &str) -> String {
    saisie.trim().to_uppercase()
}

/// Vrai pour un matricule qu'on peut chercher : ni vide ni démesuré.
pub fn matricule_plausible(matricule: &str) -> bool {
    (1..=64).contains(&matricule.chars().count())
}

/// Ce que le maître voit du suivi de ses élèves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GrainSuivi {
    /// Par élève : qui a ouvert et fini quelles leçons.
    #[default]
    Eleve,
    /// Par classe seulement : des nombres, sans noms.
    Classe,
}

impl GrainSuivi {
    /// Nom du grain, tel qu'il est rangé dans le réglage.
    pub fn nom(self) -> &'static str {
        match self {
            Self::Eleve => "eleve",
            Self::Classe => "classe",
        }
    }

    /// Grain d'après son nom ; `None` pour un nom inconnu.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        match nom {
            "eleve" => Some(Self::Eleve),
            "classe" => Some(Self::Classe),
            _ => None,
        }
    }
}

/// Adresse à donner aux élèves : le port n'y paraît que s'il n'est pas le 80.
pub fn adresse_a_donner(hote: &str, port: u16) -> String {
    if port == PORT_PREFERE {
        format!("http://{hote}")
    } else {
        format!("http://{hote}:{port}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_duree_livree_est_dans_ses_bornes() {
        assert!(DUREE_SESSION_BORNES.contains(&DUREE_SESSION_DEFAUT));
    }

    #[test]
    fn le_grain_se_relit_par_son_nom() {
        for grain in [GrainSuivi::Eleve, GrainSuivi::Classe] {
            assert_eq!(GrainSuivi::depuis_nom(grain.nom()), Some(grain));
        }
        assert_eq!(GrainSuivi::depuis_nom("ecole"), None);
        assert_eq!(GrainSuivi::default(), GrainSuivi::Eleve);
    }

    #[test]
    fn le_jour_et_le_matricule() {
        let h = crate::commun::Horodatage::depuis_millis_epoque(1_794_960_000_000);
        assert_eq!(jour_de(h).len(), 10);
        assert_eq!(normaliser_matricule("  ab-12 "), "AB-12");
        assert!(matricule_plausible("A1"));
        assert!(!matricule_plausible(""));
        assert!(!matricule_plausible(&"9".repeat(65)));
    }

    #[test]
    fn le_port_80_ne_se_tape_pas() {
        assert_eq!(adresse_a_donner("192.168.1.10", 80), "http://192.168.1.10");
        assert_eq!(
            adresse_a_donner("192.168.1.10", PORT_SECOURS),
            "http://192.168.1.10:47801"
        );
    }
}
