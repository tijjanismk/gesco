//! Génération en lot (blueprint §13.3, spécification §10.2) : parallélisée, avec progression,
//! annulation et reprise. Chaque document est écrit sous un nom temporaire puis renommé : un
//! fichier final présent est complet, ce qui rend la reprise sûre après une coupure.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::erreur::ErreurDocument;
use crate::moteur::{Moteur, Variante, empreinte};

/// Un document du lot : une clé, qui devient le nom du fichier, et ses données.
#[derive(Debug, Clone, PartialEq)]
pub struct ElementLot {
    /// Clé stable (identifiant de l'élève, de l'inscription…), lettres, chiffres, `-` et `_`.
    pub cle: String,
    /// Données du modèle.
    pub donnees: serde_json::Value,
}

/// Avancement d'un lot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progression {
    /// Documents traités, composés ou repris.
    pub traites: usize,
    /// Taille du lot.
    pub total: usize,
}

/// Drapeau d'annulation partagé entre l'interface et les ouvriers du lot.
#[derive(Debug, Default)]
pub struct Annulation(AtomicBool);

impl Annulation {
    /// Demande l'arrêt : les documents en cours se terminent, aucun autre ne commence.
    pub fn annuler(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    /// Vrai si l'arrêt a été demandé.
    pub fn est_annulee(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// Document présent dans le dossier du lot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentLot {
    /// Clé de l'élément.
    pub cle: String,
    /// Fichier PDF.
    pub chemin: PathBuf,
    /// Empreinte SHA-256.
    pub empreinte_sha256: String,
    /// Taille en octets.
    pub taille: u64,
    /// Vrai si le fichier existait déjà (reprise) et n'a pas été recomposé.
    pub repris: bool,
}

/// Bilan d'un lot, complet ou interrompu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BilanLot {
    /// Modèle utilisé.
    pub modele_id: String,
    /// Version du modèle (R-45).
    pub modele_version: u32,
    /// Documents présents, dans l'ordre du lot.
    pub documents: Vec<DocumentLot>,
    /// Vrai si le lot a été annulé avant la fin ; le relancer reprend où il s'est arrêté.
    pub annule: bool,
}

fn cle_valide(cle: &str) -> bool {
    !cle.is_empty()
        && cle.len() <= 100
        && cle
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

impl Moteur {
    /// Compose un lot dans `dossier`. Toutes les données sont vérifiées avant le premier
    /// document : un lot dont un élément est incomplet ne commence pas.
    pub fn composer_lot(
        &self,
        modele_id: &str,
        variante: &Variante,
        elements: &[ElementLot],
        dossier: &Path,
        annulation: &Annulation,
        progression: &(dyn Fn(Progression) + Sync),
    ) -> Result<BilanLot, ErreurDocument> {
        let modele = self.catalogue().modele(modele_id)?;
        for element in elements {
            if !cle_valide(&element.cle) {
                return Err(ErreurDocument::CleLotInvalide(element.cle.clone()));
            }
            modele.verifier(&element.donnees)?;
        }
        std::fs::create_dir_all(dossier)?;

        let total = elements.len();
        let suivant = AtomicUsize::new(0);
        let traites = AtomicUsize::new(0);
        let arret = AtomicBool::new(false);
        let resultats: Mutex<Vec<Option<DocumentLot>>> = Mutex::new(vec![None; total]);
        let premiere_erreur: Mutex<Option<ErreurDocument>> = Mutex::new(None);
        let ouvriers = std::thread::available_parallelism()
            .map_or(1, std::num::NonZero::get)
            .min(total.max(1));

        let traiter = |element: &ElementLot| -> Result<DocumentLot, ErreurDocument> {
            let chemin = dossier.join(format!("{}.pdf", element.cle));
            if chemin.is_file() {
                let octets = std::fs::read(&chemin)?;
                return Ok(DocumentLot {
                    cle: element.cle.clone(),
                    empreinte_sha256: empreinte(&octets),
                    taille: octets.len() as u64,
                    chemin,
                    repris: true,
                });
            }
            let compose = self.composer(modele_id, variante, &element.donnees)?;
            let partiel = dossier.join(format!("{}.pdf.partiel", element.cle));
            std::fs::write(&partiel, &compose.pdf)?;
            std::fs::rename(&partiel, &chemin)?;
            Ok(DocumentLot {
                cle: element.cle.clone(),
                empreinte_sha256: compose.empreinte_sha256,
                taille: compose.pdf.len() as u64,
                chemin,
                repris: false,
            })
        };

        std::thread::scope(|portee| {
            for _ in 0..ouvriers {
                portee.spawn(|| {
                    loop {
                        if annulation.est_annulee() || arret.load(Ordering::SeqCst) {
                            break;
                        }
                        let index = suivant.fetch_add(1, Ordering::SeqCst);
                        let Some(element) = elements.get(index) else {
                            break;
                        };
                        match traiter(element) {
                            Ok(document) => {
                                if let Ok(mut r) = resultats.lock()
                                    && let Some(place) = r.get_mut(index)
                                {
                                    *place = Some(document);
                                }
                                let faits = traites.fetch_add(1, Ordering::SeqCst) + 1;
                                progression(Progression {
                                    traites: faits,
                                    total,
                                });
                            }
                            Err(erreur) => {
                                arret.store(true, Ordering::SeqCst);
                                if let Ok(mut e) = premiere_erreur.lock() {
                                    e.get_or_insert(erreur);
                                }
                            }
                        }
                    }
                });
            }
        });

        if let Some(erreur) = premiere_erreur.into_inner().ok().flatten() {
            return Err(erreur);
        }
        let documents: Vec<DocumentLot> = resultats
            .into_inner()
            .unwrap_or_default()
            .into_iter()
            .flatten()
            .collect();
        Ok(BilanLot {
            modele_id: modele.id.clone(),
            modele_version: modele.version,
            annule: documents.len() < total,
            documents,
        })
    }
}
