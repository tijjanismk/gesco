//! Manifeste et rotation des sauvegardes (blueprint §18, spécification §11).

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::cle::Enveloppe;
use super::etablissement::Etablissement;
use crate::commun::{Horodatage, Id};

/// Format courant de l'archive de sauvegarde.
pub const FORMAT_ARCHIVE: u16 = 1;

/// Rappel imprimé en clair dans chaque manifeste (spécification §11.2).
pub const RAPPEL_PHRASE: &str = "La restauration de cette sauvegarde sur un autre poste demande la phrase de secours imprimée sur le certificat de clé de secours.";

/// Ce qui a déclenché une sauvegarde (§11.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum MotifSauvegarde {
    /// Demandée par un utilisateur.
    Manuelle,
    /// Intervalle configuré.
    Planifiee,
    /// Fermeture de l'application.
    Fermeture,
    /// Avant une migration de schéma (R-48).
    AvantMigration,
    /// Avant une restauration (R-48).
    AvantRestauration,
    /// Avant l'ouverture d'une année scolaire (SPEC_PHASE_1 §3.3).
    AvantOuvertureAnnee,
    /// Avant la clôture d'une année scolaire (SPEC_PHASE_10 §4).
    AvantClotureAnnee,
    /// Avant l'application d'un fichier d'élèves importé (R-48, SPEC_PHASE_2 §10).
    AvantImport,
    /// Avant la facturation en masse des inscriptions déjà validées (R-48, SPEC_PHASE_3 §0.3).
    AvantFacturation,
}

/// Manifeste d'une archive, lisible sans la clé. Il est lié au contenu chiffré : le modifier
/// rend l'archive illisible.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifeste {
    /// Format de l'archive.
    pub format: u16,
    /// Version de l'application qui l'a produite.
    pub version_applicative: String,
    /// Version du schéma de la base sauvegardée.
    pub version_schema: i64,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Nom de l'établissement, pour l'assistant de restauration.
    pub etablissement_nom: String,
    /// Instant de la sauvegarde.
    pub horodatage: Horodatage,
    /// Déclencheur.
    pub motif: MotifSauvegarde,
    /// Identifiant de la clé de données qui chiffre le contenu.
    pub cle_id: String,
    /// Enveloppe de secours de cette clé : la phrase du certificat l'ouvre (R-57). Jamais
    /// l'enveloppe machine.
    pub enveloppe_secours: Enveloppe,
    /// Empreinte SHA-256 du fichier de base sauvegardé.
    pub empreinte_base: String,
    /// Empreinte du contenu logique de la base, comparée après restauration.
    pub empreinte_contenu: String,
    /// Préfixe des nonces des blocs chiffrés.
    #[serde(with = "hex::serde")]
    pub prefixe_nonce: [u8; 16],
    /// Rappel en clair.
    pub rappel: String,
}

/// Politique de rotation : 7 quotidiennes, 4 hebdomadaires, 12 mensuelles par défaut (§11.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PolitiqueRotation {
    /// Jours distincts conservés.
    pub quotidiennes: u32,
    /// Semaines distinctes conservées.
    pub hebdomadaires: u32,
    /// Mois distincts conservés.
    pub mensuelles: u32,
}

impl Default for PolitiqueRotation {
    fn default() -> Self {
        Self {
            quotidiennes: 7,
            hebdomadaires: 4,
            mensuelles: 12,
        }
    }
}

const MILLIS_PAR_JOUR: i64 = 86_400_000;

/// Regroupe les sauvegardes par jour, semaine ou mois.
type CleDePalier = fn(Horodatage) -> i64;

fn jour(h: Horodatage) -> i64 {
    h.millis_epoque().div_euclid(MILLIS_PAR_JOUR)
}

// Le 1er janvier 1970 est un jeudi : décaler de trois jours aligne les semaines sur le lundi.
fn semaine(h: Horodatage) -> i64 {
    (jour(h) + 3).div_euclid(7)
}

fn mois(h: Horodatage) -> i64 {
    let civil = h.civil_utc();
    civil.annee * 12 + i64::from(civil.mois)
}

/// Sauvegardes à conserver : la plus récente de chacun des derniers jours, semaines et mois, dans
/// la limite de la politique. La plus récente de toutes est toujours conservée. Le Mali vit à
/// l'heure UTC : le jour civil UTC est le jour de l'école.
pub fn a_conserver<T: Clone + Eq + std::hash::Hash>(
    sauvegardes: &[(T, Horodatage)],
    politique: PolitiqueRotation,
) -> Vec<T> {
    let mut triees: Vec<&(T, Horodatage)> = sauvegardes.iter().collect();
    triees.sort_by_key(|(_, h)| std::cmp::Reverse(*h));
    let mut gardees: Vec<T> = Vec::new();
    let mut garder = |id: &T| {
        if !gardees.contains(id) {
            gardees.push(id.clone());
        }
    };
    if let Some((id, _)) = triees.first() {
        garder(id);
    }
    let paliers: [(u32, CleDePalier); 3] = [
        (politique.quotidiennes, jour),
        (politique.hebdomadaires, semaine),
        (politique.mensuelles, mois),
    ];
    for (limite, cle) in paliers {
        let mut vues = HashSet::new();
        for (id, h) in &triees {
            if vues.len() >= limite as usize {
                break;
            }
            if vues.insert(cle(*h)) {
                garder(id);
            }
        }
    }
    gardees
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEURE: i64 = 3_600_000;

    fn h(jours: i64, heures: i64) -> Horodatage {
        // 2026-10-05 est un lundi : jour 20_731 depuis l'époque.
        Horodatage::depuis_millis_epoque((20_731 + jours) * MILLIS_PAR_JOUR + heures * HEURE)
    }

    #[test]
    fn une_par_jour_sur_sept_jours() {
        let mut sauvegardes = Vec::new();
        for j in 0..10 {
            sauvegardes.push((format!("{j}-matin"), h(j, 8)));
            sauvegardes.push((format!("{j}-soir"), h(j, 18)));
        }
        let politique = PolitiqueRotation {
            quotidiennes: 7,
            hebdomadaires: 0,
            mensuelles: 0,
        };
        let mut gardees = a_conserver(&sauvegardes, politique);
        gardees.sort();
        let attendues: Vec<String> = (3..10).map(|j| format!("{j}-soir")).collect();
        assert_eq!(gardees, attendues);
    }

    #[test]
    fn semaines_et_mois_s_ajoutent_aux_jours() {
        let sauvegardes: Vec<(i64, Horodatage)> = (0..400).map(|j| (j, h(j, 12))).collect();
        let mut gardees = a_conserver(&sauvegardes, PolitiqueRotation::default());
        gardees.sort_unstable();
        // Sept derniers jours (393 à 399) ; deux semaines de plus (391, 384), la semaine du
        // jour 399 et la précédente étant couvertes ; puis le dernier jour de chacun des dix mois
        // précédents.
        assert_eq!(
            gardees,
            vec![
                87, 118, 146, 177, 207, 238, 268, 299, 330, 360, 384, 391, 393, 394, 395, 396, 397,
                398, 399
            ]
        );
    }

    #[test]
    fn la_plus_recente_est_toujours_gardee() {
        let sauvegardes = vec![("a", h(0, 1)), ("b", h(0, 2))];
        let politique = PolitiqueRotation {
            quotidiennes: 0,
            hebdomadaires: 0,
            mensuelles: 0,
        };
        assert_eq!(a_conserver(&sauvegardes, politique), vec!["b"]);
        assert!(a_conserver::<&str>(&[], PolitiqueRotation::default()).is_empty());
    }

    #[test]
    fn semaine_alignee_sur_le_lundi() {
        assert_eq!(semaine(h(0, 0)), semaine(h(6, 23)));
        assert_ne!(semaine(h(0, 0)), semaine(h(-1, 23)));
    }
}
