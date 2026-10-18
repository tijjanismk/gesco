//! Canal SMS (blueprint §25.3, SPEC_PHASE_5 §4) : l'interface `Canal`, que la file appelle sans
//! savoir ce qu'il y a derrière. Implémentation livrée : le canal simulé, qui écrit chaque SMS
//! dans un journal du poste sans rien envoyer (décision du 17 octobre 2026 : le matériel est
//! différé). Le pilotage de la clé modem en commandes AT viendra derrière la même interface.

use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use gesco_core::notification::ErreurEnvoi;

/// Accusé de remise d'un SMS parti.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Accuse {
    /// Référence rendue par `envoyer`.
    pub reference: String,
    /// Vrai si le SMS a été remis au téléphone.
    pub remis: bool,
}

/// Canal d'envoi des SMS.
pub trait Canal: Send + Sync {
    /// Envoie un texte déjà en GSM-7 à un numéro canonique ; rend la référence de l'envoi.
    ///
    /// # Errors
    /// La raison de l'échec : réseau, crédit, numéro refusé.
    fn envoyer(&self, numero: &str, texte: &str) -> Result<String, ErreurEnvoi>;

    /// Accusés de remise revenus depuis le dernier appel.
    fn accuses(&self) -> Vec<Accuse>;
}

/// Canal simulé : rien ne part ; chaque SMS s'écrit dans un journal, et son accusé de remise
/// revient aussitôt. Les essais lui font jouer un échec d'envoi ou de remise.
#[derive(Debug, Default)]
pub struct CanalSimule {
    journal: Option<PathBuf>,
    compteur: AtomicU64,
    accuses: Mutex<Vec<Accuse>>,
    envoyes: Mutex<Vec<(String, String)>>,
    echecs: Mutex<HashMap<String, ErreurEnvoi>>,
    non_remis: Mutex<HashSet<String>>,
}

impl CanalSimule {
    /// Canal simulé qui écrit ses SMS dans ce fichier, s'il y en a un.
    pub fn nouveau(journal: Option<PathBuf>) -> Self {
        Self {
            journal,
            ..Self::default()
        }
    }

    /// Les envois à ce numéro échoueront avec cette erreur.
    pub fn faire_echouer(&self, numero: &str, erreur: ErreurEnvoi) {
        if let Ok(mut echecs) = self.echecs.lock() {
            echecs.insert(numero.to_owned(), erreur);
        }
    }

    /// Les SMS à ce numéro partiront, mais leur accusé dira « non remis ».
    pub fn ne_pas_remettre(&self, numero: &str) {
        if let Ok(mut non_remis) = self.non_remis.lock() {
            non_remis.insert(numero.to_owned());
        }
    }

    /// Les envois à ce numéro réussissent de nouveau.
    pub fn retablir(&self, numero: &str) {
        if let Ok(mut echecs) = self.echecs.lock() {
            echecs.remove(numero);
        }
        if let Ok(mut non_remis) = self.non_remis.lock() {
            non_remis.remove(numero);
        }
    }

    /// SMS partis, numéro et texte, dans l'ordre.
    pub fn envoyes(&self) -> Vec<(String, String)> {
        self.envoyes.lock().map(|e| e.clone()).unwrap_or_default()
    }
}

impl Canal for CanalSimule {
    fn envoyer(&self, numero: &str, texte: &str) -> Result<String, ErreurEnvoi> {
        if let Some(erreur) = self.echecs.lock().ok().and_then(|e| e.get(numero).copied()) {
            return Err(erreur);
        }
        let reference = format!(
            "simule-{}",
            self.compteur.fetch_add(1, Ordering::Relaxed) + 1
        );
        if let Some(chemin) = &self.journal
            && let Ok(mut fichier) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(chemin)
        {
            // Le journal n'est qu'un confort de lecture : son échec n'empêche pas l'envoi simulé.
            let _ = writeln!(fichier, "{numero} : {texte}");
        }
        if let Ok(mut envoyes) = self.envoyes.lock() {
            envoyes.push((numero.to_owned(), texte.to_owned()));
        }
        let remis = self
            .non_remis
            .lock()
            .map(|n| !n.contains(numero))
            .unwrap_or(true);
        if let Ok(mut accuses) = self.accuses.lock() {
            accuses.push(Accuse {
                reference: reference.clone(),
                remis,
            });
        }
        Ok(reference)
    }

    fn accuses(&self) -> Vec<Accuse> {
        self.accuses
            .lock()
            .map(|mut a| std::mem::take(&mut *a))
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_canal_simule_envoie_et_accuse() {
        let canal = CanalSimule::nouveau(None);
        let r = canal.envoyer("+22376000001", "Bonjour");
        assert_eq!(r, Ok("simule-1".to_owned()));
        assert_eq!(
            canal.accuses(),
            [Accuse {
                reference: "simule-1".to_owned(),
                remis: true
            }]
        );
        assert!(canal.accuses().is_empty());
        assert_eq!(canal.envoyes().len(), 1);
    }

    #[test]
    fn echecs_joues_par_les_essais() {
        let canal = CanalSimule::nouveau(None);
        canal.faire_echouer("+22376000001", ErreurEnvoi::Reseau);
        assert_eq!(canal.envoyer("+22376000001", "x"), Err(ErreurEnvoi::Reseau));
        canal.retablir("+22376000001");
        canal.ne_pas_remettre("+22366000002");
        let r = canal.envoyer("+22366000002", "x");
        assert!(r.is_ok());
        assert!(canal.accuses().iter().all(|a| !a.remis));
    }

    #[test]
    fn le_journal_garde_chaque_sms() {
        let dossier = std::env::temp_dir().join(format!("gesco-sms-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dossier);
        let chemin = dossier.join("sms-simules.txt");
        let _ = std::fs::remove_file(&chemin);
        let canal = CanalSimule::nouveau(Some(chemin.clone()));
        let _ = canal.envoyer("+22376000001", "Premier");
        let _ = canal.envoyer("+22376000001", "Second");
        let contenu = std::fs::read_to_string(&chemin).unwrap_or_default();
        assert_eq!(contenu, "+22376000001 : Premier\n+22376000001 : Second\n");
        let _ = std::fs::remove_dir_all(&dossier);
    }
}
