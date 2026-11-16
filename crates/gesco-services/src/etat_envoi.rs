//! État d'envoi d'une copie hors de l'école, tel que Windows le connaît (ADR du 15 novembre 2026,
//! copie hors de l'école). GESCO ne parle pas au fournisseur : il lit ce que Windows tient pour
//! un fichier synchronisé. La lecture réelle, par PowerShell, vit dans le binaire ; ici, le trait
//! qu'il remplit et une lecture simulée pour les essais, qui ne lancent jamais PowerShell.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Ce que Windows dit d'un fichier synchronisé.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EtatSignale {
    /// Le fichier est à jour sur le compte.
    AJour,
    /// Le fichier n'est pas encore envoyé.
    EnAttente,
    /// Le logiciel du fournisseur n'a pas pu l'envoyer.
    EnEchec,
}

/// Lecture de l'état d'envoi d'un fichier. Rien quand Windows ne dit rien, ou que la lecture
/// échoue : l'état reste alors « déposée dans le dossier ». Une lecture ne doit jamais faire
/// télécharger le fichier.
pub trait LecteurEtatEnvoi: Send + Sync {
    /// État d'envoi du fichier, si Windows le connaît.
    fn lire(&self, fichier: &Path) -> Option<EtatSignale>;
}

/// Lecture simulée : les essais disent ce que Windows répondrait pour chaque fichier ; sans
/// réponse posée, Windows ne dit rien. Elle compte les lectures.
#[derive(Debug, Default)]
pub struct LecteurEtatSimule {
    etats: Mutex<HashMap<PathBuf, EtatSignale>>,
    defaut: Mutex<Option<EtatSignale>>,
    lectures: AtomicUsize,
}

impl LecteurEtatSimule {
    /// Pose la réponse de Windows pour un fichier ; rien l'efface.
    pub fn signaler(&self, fichier: &Path, etat: Option<EtatSignale>) {
        if let Ok(mut etats) = self.etats.lock() {
            match etat {
                Some(e) => {
                    etats.insert(fichier.to_path_buf(), e);
                }
                None => {
                    etats.remove(fichier);
                }
            }
        }
    }

    /// Pose la réponse de Windows pour tout fichier sans réponse propre.
    pub fn signaler_partout(&self, etat: Option<EtatSignale>) {
        if let Ok(mut defaut) = self.defaut.lock() {
            *defaut = etat;
        }
    }

    /// Nombre de lectures faites depuis le démarrage.
    pub fn lectures(&self) -> usize {
        self.lectures.load(Ordering::Relaxed)
    }
}

impl LecteurEtatEnvoi for LecteurEtatSimule {
    fn lire(&self, fichier: &Path) -> Option<EtatSignale> {
        self.lectures.fetch_add(1, Ordering::Relaxed);
        let propre = self.etats.lock().ok()?.get(fichier).copied();
        propre.or_else(|| self.defaut.lock().ok().and_then(|d| *d))
    }
}

/// Traduit la valeur de la propriété du Shell `System.StorageProviderState`, lue par PowerShell :
/// un entier, vide quand le fournisseur ne la renseigne pas. Ce qui n'est pas sûr ne dit rien.
///
/// Valeurs de Windows : 0 aucun état, 1 disponible en ligne seulement, 2 synchronisé, 3 toujours
/// gardé sur l'appareil, 4 envoi en attente, 5 téléchargement en attente, 6 transfert en cours,
/// 7 erreur, 8 avertissement, 9 exclu de la synchronisation, 10 en attente.
pub fn etat_depuis_windows(valeur: &str) -> Option<EtatSignale> {
    match valeur.trim().parse::<u32>().ok()? {
        // Un fichier libéré du disque, ou à télécharger, est déjà sur le compte.
        1 | 2 | 3 | 5 => Some(EtatSignale::AJour),
        4 | 6 | 10 => Some(EtatSignale::EnAttente),
        7 | 9 => Some(EtatSignale::EnEchec),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_propriete_de_windows_se_traduit_sans_rien_inventer() {
        assert_eq!(etat_depuis_windows("2\r\n"), Some(EtatSignale::AJour));
        assert_eq!(etat_depuis_windows("1"), Some(EtatSignale::AJour));
        assert_eq!(etat_depuis_windows("4"), Some(EtatSignale::EnAttente));
        assert_eq!(etat_depuis_windows("6"), Some(EtatSignale::EnAttente));
        assert_eq!(etat_depuis_windows("7"), Some(EtatSignale::EnEchec));
        assert_eq!(etat_depuis_windows("0"), None);
        assert_eq!(etat_depuis_windows("8"), None);
        assert_eq!(etat_depuis_windows(""), None);
        assert_eq!(etat_depuis_windows("Synchronisé"), None);
    }

    #[test]
    fn la_lecture_simulee_ne_dit_rien_sans_reponse_posee() {
        let lecteur = LecteurEtatSimule::default();
        let fichier = Path::new("copie.gsv");
        assert_eq!(lecteur.lire(fichier), None);
        lecteur.signaler(fichier, Some(EtatSignale::EnAttente));
        assert_eq!(lecteur.lire(fichier), Some(EtatSignale::EnAttente));
        lecteur.signaler(fichier, None);
        lecteur.signaler_partout(Some(EtatSignale::AJour));
        assert_eq!(lecteur.lire(fichier), Some(EtatSignale::AJour));
        assert_eq!(lecteur.lectures(), 3);
    }
}
