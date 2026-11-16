//! État d'envoi d'une copie hors de l'école, lu dans Windows (ADR du 15 novembre 2026, copie hors
//! de l'école) : la propriété du Shell `System.StorageProviderState`, que le logiciel du
//! fournisseur tient à jour, lue par une commande PowerShell lancée sans fenêtre. Pas de code
//! `unsafe`, interdit dans l'espace de travail ; pas de lecture du contenu, qui ferait télécharger
//! une copie libérée du disque. Un échec, un délai dépassé, une valeur inconnue ne disent rien :
//! l'état reste « déposée dans le dossier ».

use std::path::Path;

use gesco_services::etat_envoi::{EtatSignale, LecteurEtatEnvoi, etat_depuis_windows};

/// Lecture par Windows PowerShell 5.1, livré avec Windows 10 et 11.
pub struct LecteurPowerShell;

/// Le chemin passe par l'environnement, jamais dans le texte de la commande.
const SCRIPT: &str = "$ErrorActionPreference = 'Stop'; \
    $dossier = (New-Object -ComObject Shell.Application).Namespace($env:GESCO_DOSSIER); \
    $fichier = $dossier.ParseName($env:GESCO_FICHIER); \
    [Console]::Out.Write([string]$fichier.ExtendedProperty('System.StorageProviderState'))";

/// Au-delà, la lecture est abandonnée.
const DELAI: std::time::Duration = std::time::Duration::from_secs(15);

impl LecteurEtatEnvoi for LecteurPowerShell {
    fn lire(&self, fichier: &Path) -> Option<EtatSignale> {
        let dossier = fichier.parent()?;
        let nom = fichier.file_name()?;
        let mut commande = std::process::Command::new("powershell.exe");
        commande
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                SCRIPT,
            ])
            .env("GESCO_DOSSIER", dossier)
            .env("GESCO_FICHIER", nom)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null());
        sans_fenetre(&mut commande);
        let mut enfant = commande.spawn().ok()?;
        let debut = std::time::Instant::now();
        loop {
            match enfant.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if debut.elapsed() < DELAI => {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                _ => {
                    let _ = enfant.kill();
                    let _ = enfant.wait();
                    return None;
                }
            }
        }
        let sortie = enfant.wait_with_output().ok()?;
        if !sortie.status.success() {
            return None;
        }
        etat_depuis_windows(&String::from_utf8_lossy(&sortie.stdout))
    }
}

#[cfg(windows)]
fn sans_fenetre(commande: &mut std::process::Command) {
    use std::os::windows::process::CommandExt;
    // CREATE_NO_WINDOW : pas de console qui clignote.
    const SANS_FENETRE: u32 = 0x0800_0000;
    commande.creation_flags(SANS_FENETRE);
}

#[cfg(not(windows))]
fn sans_fenetre(_commande: &mut std::process::Command) {}
