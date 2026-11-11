//! Fichiers d'un poste relié (SPEC_PHASE_11_A §3.3). Un chemin venu d'un poste relié désigne
//! son disque, pas celui du poste principal : c'est le contenu du fichier qui voyage à sa place.
//!
//! - Commande qui **lit** : le champ du chemin porte `{ "nom": …, "contenu": … }` (contenu en
//!   base64) ; il est déposé dans un dossier de travail du poste principal, et le gestionnaire
//!   reçoit ce chemin.
//! - Commande qui **écrit** : le champ porte le nom du fichier ; le gestionnaire écrit dans le
//!   dossier de travail, et la réponse devient `{ "sortie": …, "contenu": … }`.
//!
//! Le dossier de travail s'efface dès la réponse rendue, quoi qu'il arrive. Aucun gestionnaire
//! ne reçoit jamais un chemin choisi par un poste relié.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};

use crate::commandes::Fichier;
use crate::espace::EspaceDonnees;
use crate::{ErreurService, Resultat};

/// Champ du nom d'un fichier qui voyage.
pub const CHAMP_NOM: &str = "nom";
/// Champ du contenu d'un fichier qui voyage, en base64.
pub const CHAMP_CONTENU: &str = "contenu";
/// Champ de la sortie de la commande, dans la réponse d'une commande qui écrit un fichier.
pub const CHAMP_SORTIE: &str = "sortie";

/// Numéro du prochain dossier de travail de ce lancement.
static SUIVANT: AtomicU64 = AtomicU64::new(0);

const ENTREE_INVALIDE: ErreurService = ErreurService::Regle {
    cle: "commande.entree_invalide",
};

/// Dossier où se déposent les fichiers des postes reliés, le temps d'une commande.
pub(crate) fn dossier_de_travail(espace: &EspaceDonnees) -> PathBuf {
    espace.temporaire().join("reseau")
}

/// Code le contenu d'un fichier pour le voyage.
pub fn coder(octets: &[u8]) -> String {
    STANDARD.encode(octets)
}

/// Décode le contenu d'un fichier reçu ; `None` s'il n'est pas lisible.
pub fn decoder(texte: &str) -> Option<Vec<u8>> {
    STANDARD.decode(texte).ok()
}

/// Nom de fichier sans dossier : ce qu'un poste relié envoie ne sort jamais du dossier de
/// travail.
pub fn nom_seul(nom: &str) -> String {
    let dernier = nom.rsplit(['/', '\\']).next().unwrap_or_default().trim();
    let propre: String = dernier
        .chars()
        .filter(|c| !c.is_control() && !matches!(c, ':' | '*' | '?' | '"' | '<' | '>' | '|'))
        .collect();
    if propre.is_empty() || propre.chars().all(|c| c == '.') {
        "fichier".to_owned()
    } else {
        propre
    }
}

/// Dépôt du fichier d'une commande venue d'un poste relié.
pub(crate) struct Depot {
    dossier: PathBuf,
    /// Fichier que le gestionnaire écrit, et dont le contenu repart avec la réponse.
    ecrit: Option<PathBuf>,
}

impl Depot {
    /// Remplace, dans l'entrée, ce qu'envoie le poste relié par un chemin du dossier de travail.
    /// `None` quand l'entrée ne désigne aucun fichier.
    pub(crate) fn preparer(
        espace: &EspaceDonnees,
        fichier: Fichier,
        entree: &mut Value,
    ) -> Resultat<Option<Self>> {
        let (champ, lu) = match fichier {
            Fichier::Lu(champ) => (champ, true),
            Fichier::Ecrit(champ) => (champ, false),
        };
        let Some(valeur) = entree.get_mut(champ) else {
            return Ok(None);
        };
        if valeur.is_null() {
            return Ok(None);
        }
        let (nom, contenu) = if lu {
            let nom = valeur.get(CHAMP_NOM).and_then(Value::as_str);
            let contenu = valeur
                .get(CHAMP_CONTENU)
                .and_then(Value::as_str)
                .and_then(decoder);
            match (nom, contenu) {
                (Some(nom), Some(contenu)) => (nom_seul(nom), Some(contenu)),
                _ => return Err(ENTREE_INVALIDE),
            }
        } else {
            match valeur.as_str() {
                Some(nom) => (nom_seul(nom), None),
                None => return Err(ENTREE_INVALIDE),
            }
        };
        let dossier = dossier_de_travail(espace).join(format!(
            "{}-{}",
            std::process::id(),
            SUIVANT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dossier)?;
        let depot = Self {
            ecrit: (!lu).then(|| dossier.join(&nom)),
            dossier,
        };
        let chemin = depot.dossier.join(&nom);
        if let Some(contenu) = contenu {
            std::fs::write(&chemin, contenu)?;
        }
        *valeur = Value::String(chemin.to_string_lossy().into_owned());
        Ok(Some(depot))
    }

    /// Réponse à rendre au poste relié : la sortie telle quelle pour une commande qui lit ; la
    /// sortie et le contenu du fichier écrit pour une commande qui écrit.
    pub(crate) fn conclure(self, sortie: Value) -> Resultat<Value> {
        match &self.ecrit {
            Some(chemin) => {
                let contenu = lire_si_present(chemin)?;
                Ok(json!({ CHAMP_SORTIE: sortie, CHAMP_CONTENU: contenu.map(|c| coder(&c)) }))
            }
            None => Ok(sortie),
        }
    }
}

fn lire_si_present(chemin: &Path) -> Resultat<Option<Vec<u8>>> {
    match std::fs::read(chemin) {
        Ok(octets) => Ok(Some(octets)),
        Err(erreur) if erreur.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(erreur) => Err(erreur.into()),
    }
}

impl Drop for Depot {
    fn drop(&mut self) {
        // Rien ne reste sur le poste principal, que la commande ait réussi ou non.
        let _ = std::fs::remove_dir_all(&self.dossier);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn espace() -> (tempfile::TempDir, EspaceDonnees) {
        let dossier = tempfile::tempdir().unwrap();
        let espace = EspaceDonnees::nouveau(dossier.path());
        (dossier, espace)
    }

    #[test]
    fn un_nom_venu_d_un_poste_relie_ne_sort_pas_du_dossier() {
        assert_eq!(nom_seul("C:\\Users\\Awa\\eleves.xlsx"), "eleves.xlsx");
        assert_eq!(nom_seul("/home/awa/reçu n° 12.pdf"), "reçu n° 12.pdf");
        assert_eq!(nom_seul("..\\..\\gesco.db"), "gesco.db");
        assert_eq!(nom_seul(".."), "fichier");
        assert_eq!(nom_seul("dossier/"), "fichier");
        assert_eq!(nom_seul("a:b*c?.txt"), "abc.txt");
    }

    #[test]
    fn un_fichier_lu_est_depose_puis_efface() {
        let (_garde, espace) = espace();
        let mut entree = json!({
            "chemin": { "nom": "..\\eleves.csv", "contenu": coder(b"nom;prenom") },
            "annee": 2026,
        });
        let depot = Depot::preparer(&espace, Fichier::Lu("chemin"), &mut entree)
            .ok()
            .flatten();
        let chemin = PathBuf::from(entree["chemin"].as_str().unwrap_or_default());
        assert!(chemin.starts_with(dossier_de_travail(&espace)));
        assert_eq!(
            chemin.file_name().and_then(|n| n.to_str()),
            Some("eleves.csv")
        );
        assert_eq!(std::fs::read(&chemin).ok(), Some(b"nom;prenom".to_vec()));
        assert_eq!(entree["annee"], 2026);
        let rendu = depot.map(|d| d.conclure(json!({ "lignes": 1 })));
        assert!(matches!(rendu, Some(Ok(ref v)) if v == &json!({ "lignes": 1 })));
        assert!(!chemin.exists(), "le dépôt s'efface après la réponse");
    }

    #[test]
    fn un_fichier_ecrit_repart_avec_la_reponse() {
        let (_garde, espace) = espace();
        let mut entree = json!({ "chemin": "D:\\Bureau\\liste.xlsx" });
        let depot = Depot::preparer(&espace, Fichier::Ecrit("chemin"), &mut entree)
            .ok()
            .flatten();
        let chemin = PathBuf::from(entree["chemin"].as_str().unwrap_or_default());
        assert!(chemin.starts_with(dossier_de_travail(&espace)));
        assert!(std::fs::write(&chemin, b"classeur").is_ok());
        let rendu = depot.and_then(|d| d.conclure(json!({ "lignes": 3 })).ok());
        assert_eq!(
            rendu,
            Some(json!({ "sortie": { "lignes": 3 }, "contenu": coder(b"classeur") }))
        );
        assert!(!chemin.exists());
    }

    #[test]
    fn un_chemin_brut_ou_un_contenu_illisible_est_refuse() {
        let (_garde, espace) = espace();
        for entree in [
            json!({ "chemin": "C:\\Windows\\system.ini" }),
            json!({ "chemin": { "nom": "a.csv", "contenu": "pas du base64 !" } }),
            json!({ "chemin": { "contenu": "" } }),
        ] {
            let mut entree = entree;
            assert!(Depot::preparer(&espace, Fichier::Lu("chemin"), &mut entree).is_err());
        }
        let mut objet = json!({ "chemin": { "nom": "a.xlsx" } });
        assert!(Depot::preparer(&espace, Fichier::Ecrit("chemin"), &mut objet).is_err());
        // Sans fichier désigné, rien n'est déposé.
        let mut vide = json!({ "chemin": null });
        assert!(matches!(
            Depot::preparer(&espace, Fichier::Lu("chemin"), &mut vide),
            Ok(None)
        ));
        let mut absent = json!({});
        assert!(matches!(
            Depot::preparer(&espace, Fichier::Ecrit("chemin"), &mut absent),
            Ok(None)
        ));
    }
}
