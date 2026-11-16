//! Lecture du système pour trouver les dossiers synchronisés (copie hors de l'école) : variables
//! d'environnement du compte Windows, petits fichiers de réglage des fournisseurs, registre lu par
//! `reg.exe` (pas de code `unsafe`, interdit dans l'espace de travail), compte Windows lu par
//! `whoami`, lecteurs présents. Rien n'est jamais écrit.
//!
//! En mode d'essai, seuls les variables et les fichiers sont lus : les essais posent un faux
//! dossier OneDrive par la variable `OneDrive`, sans voir le registre ni les lecteurs du poste.

use std::path::Path;

use gesco_services::dossiers_synchronises::SystemeDossiers;

/// Lecture de Windows.
pub struct SystemeWindows {
    /// Faux en mode d'essai : ni registre, ni compte, ni lecteurs.
    pub complet: bool,
}

impl SystemeDossiers for SystemeWindows {
    fn variable(&self, nom: &str) -> Option<String> {
        std::env::var(nom).ok().filter(|v| !v.trim().is_empty())
    }

    fn est_dossier(&self, chemin: &Path) -> bool {
        chemin.is_dir()
    }

    fn lire_texte(&self, chemin: &Path) -> Option<String> {
        // Un fichier de réglage fait quelques kilo-octets ; au-delà, ce n'est pas lui.
        let meta = std::fs::metadata(chemin).ok()?;
        if meta.len() > 1_000_000 {
            return None;
        }
        std::fs::read_to_string(chemin).ok()
    }

    fn registre(&self, cle: &str) -> Option<String> {
        if !self.complet {
            return None;
        }
        lancer("reg", &["query", cle, "/s"])
    }

    fn compte(&self) -> Option<String> {
        if !self.complet {
            return None;
        }
        // « "poste\nom","S-1-5-21-…" » : le dernier champ est l'identifiant du compte.
        let sortie = lancer("whoami", &["/user", "/fo", "csv", "/nh"])?;
        sortie
            .trim()
            .rsplit(',')
            .next()
            .map(|s| s.trim().trim_matches('"').to_owned())
            .filter(|s| s.starts_with("S-"))
    }

    fn lecteurs(&self) -> Vec<char> {
        if !self.complet {
            return Vec::new();
        }
        // A et B sont les lecteurs de disquette d'autrefois.
        ('C'..='Z')
            .filter(|lettre| Path::new(&format!("{lettre}:\\")).is_dir())
            .collect()
    }
}

/// Lance une commande de Windows sans fenêtre et rend sa sortie, si elle a réussi.
fn lancer(programme: &str, arguments: &[&str]) -> Option<String> {
    let mut commande = std::process::Command::new(programme);
    commande
        .args(arguments)
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    sans_fenetre(&mut commande);
    let sortie = commande.output().ok()?;
    if !sortie.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&sortie.stdout).into_owned())
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
