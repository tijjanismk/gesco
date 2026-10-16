//! Export Excel d'une liste affichée (R-41, SPEC_PHASE_2 §11). Le classeur est composé par
//! `gesco-docs` et écrit à l'emplacement que l'utilisateur a choisi dans le dialogue du système.
//! La commande ne lit rien en base : elle met en forme ce que l'écran montre déjà.

use std::path::Path;

use gesco_docs::tableur::{Liste, classeur};
use serde::{Deserialize, Serialize};

use crate::erreur::{ErreurService, Resultat};

/// Liste telle que l'écran la montre, et fichier où l'écrire.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeExport {
    /// Chemin du classeur à écrire, extension `.xlsx` ; rendu par le dialogue du système.
    pub chemin: String,
    /// Titre de la liste : nom de la feuille et titre du classeur.
    pub titre: String,
    /// En-têtes des colonnes, dans l'ordre affiché.
    pub entetes: Vec<String>,
    /// Lignes affichées, chacune aussi large que les en-têtes.
    pub lignes: Vec<Vec<String>>,
}

/// Classeur écrit.
#[derive(Debug, Serialize, specta::Type)]
pub struct FichierEcrit {
    /// Chemin du fichier écrit.
    pub chemin: String,
    /// Lignes exportées, en-têtes non comptés.
    pub lignes: u32,
}

/// Compose le classeur et l'écrit. Refuse un chemin qui ne porte pas l'extension `.xlsx` : on
/// n'écrit pas un classeur sous un nom qu'Excel n'ouvrirait pas.
pub async fn exporter_xlsx(demande: DemandeExport) -> Resultat<FichierEcrit> {
    let extension_xlsx = Path::new(&demande.chemin)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("xlsx"));
    if !extension_xlsx {
        return Err(ErreurService::Regle {
            cle: "export.extension",
        });
    }
    let lignes = u32::try_from(demande.lignes.len()).map_err(|_| ErreurService::Regle {
        cle: "export.liste_trop_grande",
    })?;
    // Composition hors de l'exécuteur asynchrone : une grande liste prend une fraction de
    // seconde, pendant laquelle les autres commandes doivent rester servies.
    let octets = tokio::task::spawn_blocking(move || {
        let liste = Liste {
            titre: &demande.titre,
            entetes: &demande.entetes,
            lignes: &demande.lignes,
        };
        classeur(&liste).map(|octets| (demande.chemin, octets))
    })
    .await
    .map_err(|e| ErreurService::Fichier(std::io::Error::other(e)))?;
    let (chemin, octets) = octets?;
    tokio::fs::write(&chemin, octets).await?;
    Ok(FichierEcrit { chemin, lignes })
}
