//! Espace de données du poste : où vivent la base, les clés, les documents et les sauvegardes
//! locales. Tous les chemins dérivent d'une racine, ce qui permet de restaurer dans un espace
//! temporaire (test de restauration, §11.5) avec le même code.

use std::io::Write;
use std::path::{Path, PathBuf};

/// Arborescence des données d'une installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EspaceDonnees {
    racine: PathBuf,
}

impl EspaceDonnees {
    /// Espace sous une racine.
    pub fn nouveau(racine: impl Into<PathBuf>) -> Self {
        Self {
            racine: racine.into(),
        }
    }

    /// Racine.
    pub fn racine(&self) -> &Path {
        &self.racine
    }

    /// Fichier de base.
    pub fn base(&self) -> PathBuf {
        self.racine.join("gesco.db")
    }

    /// Fichier des enveloppes de la clé de données (§6.3), hors de la base.
    pub fn cles(&self) -> PathBuf {
        self.racine.join("cles.json")
    }

    /// Documents produits.
    pub fn documents(&self) -> PathBuf {
        self.racine.join("documents")
    }

    /// Modèles de documents sur mesure (blueprint §13.2, niveau 3).
    pub fn modeles(&self) -> PathBuf {
        self.racine.join("modeles")
    }

    /// Sauvegardes conservées sur le poste.
    pub fn sauvegardes_locales(&self) -> PathBuf {
        self.racine.join("sauvegardes")
    }

    /// Fichiers de travail : instantanés, restaurations en cours.
    pub fn temporaire(&self) -> PathBuf {
        self.racine.join("temp")
    }

    /// Crée les dossiers.
    pub fn preparer(&self) -> std::io::Result<()> {
        for dossier in [
            self.racine.clone(),
            self.documents(),
            self.modeles(),
            self.sauvegardes_locales(),
            self.temporaire(),
        ] {
            std::fs::create_dir_all(dossier)?;
        }
        Ok(())
    }
}

/// Écrit un fichier sans jamais laisser de version partielle : écriture sous un nom temporaire,
/// synchronisation sur disque, puis renommage. Une coupure de courant laisse l'ancien fichier
/// ou le nouveau, jamais un mélange (blueprint §3.1).
pub(crate) fn ecrire_atomiquement(chemin: &Path, octets: &[u8]) -> std::io::Result<()> {
    let partiel = chemin.with_extension("partiel");
    {
        let mut fichier = std::fs::File::create(&partiel)?;
        fichier.write_all(octets)?;
        fichier.sync_all()?;
    }
    std::fs::rename(&partiel, chemin)
}
