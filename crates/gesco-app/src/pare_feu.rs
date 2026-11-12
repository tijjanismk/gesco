//! Pare-feu de Windows du poste principal (SPEC_PHASE_11_A §10). GESCO s'installe sans droits
//! d'administrateur : la règle qui laisse entrer les postes reliés se pose à l'activation du
//! réseau, avec l'accord que Windows demande alors à la personne. Les commandes et la lecture de
//! leurs réponses sont dans `gesco_reseau::pare_feu` ; ici, elles sont lancées.
//!
//! En mode d'essai, rien ne touche à Windows : un pare-feu simulé refuse la première demande
//! (comme une personne qui répond « Non » à Windows) et accepte les suivantes, et un fichier
//! `essai-reseau-public` dans le dossier d'essai fait passer le réseau pour public.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use gesco_reseau::pare_feu;
use serde::Serialize;

/// Fichier du dossier d'essai dont la présence fait passer le réseau pour public.
const TEMOIN_RESEAU_PUBLIC: &str = "essai-reseau-public";

/// Ce que l'écran « Postes de l'école » dit du pare-feu de Windows.
#[derive(Debug, Clone, Copy, Serialize)]
pub(crate) struct EtatPareFeu {
    /// La règle laisse entrer les postes reliés vers ce GESCO, sur ce port.
    pub(crate) ouvert: bool,
    /// Windows range un réseau de l'ordinateur parmi les réseaux publics : la règle n'y vaut pas.
    pub(crate) reseau_public: bool,
}

/// Pare-feu du poste : celui de Windows, ou sa doublure des essais.
pub(crate) enum PareFeu {
    /// Le pare-feu de Windows, pour ce programme.
    Windows {
        /// Chemin de GESCO sur ce poste ; vide s'il n'a pas pu être lu.
        programme: String,
    },
    /// Doublure du mode d'essai.
    Essai {
        racine: PathBuf,
        ouvert: AtomicBool,
        demandes: AtomicU32,
    },
}

/// Lance un programme de Windows sans fenêtre et rend ce qu'il a écrit, s'il a réussi.
fn lancer(programme: &str, arguments: &[String]) -> Option<String> {
    let mut commande = Command::new(programme);
    commande.args(arguments);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        /// `CREATE_NO_WINDOW` : pas de console noire devant GESCO.
        const SANS_FENETRE: u32 = 0x0800_0000;
        commande.creation_flags(SANS_FENETRE);
    }
    let sortie = commande.output().ok()?;
    sortie
        .status
        .success()
        .then(|| String::from_utf8_lossy(&sortie.stdout).into_owned())
}

impl PareFeu {
    /// Pare-feu du poste : simulé dans le dossier d'essai, celui de Windows sinon.
    pub(crate) fn du_poste(essai: Option<PathBuf>) -> Self {
        match essai {
            Some(racine) => Self::Essai {
                racine,
                ouvert: AtomicBool::new(false),
                demandes: AtomicU32::new(0),
            },
            None => Self::Windows {
                programme: std::env::current_exe()
                    .map(|chemin| chemin.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            },
        }
    }

    /// Lit l'état du pare-feu pour ce port. Ne demande aucun droit.
    pub(crate) fn lire(&self, port: u16) -> EtatPareFeu {
        match self {
            Self::Windows { programme } => EtatPareFeu {
                ouvert: lancer(pare_feu::NETSH, &pare_feu::lecture())
                    .is_some_and(|reponse| pare_feu::regle_posee(&reponse, programme, port)),
                reseau_public: lancer(pare_feu::POWERSHELL, &pare_feu::lecture_des_reseaux())
                    .is_some_and(|reponse| pare_feu::reseau_public(&reponse)),
            },
            Self::Essai { racine, ouvert, .. } => EtatPareFeu {
                ouvert: ouvert.load(Ordering::Relaxed),
                reseau_public: racine.join(TEMOIN_RESEAU_PUBLIC).exists(),
            },
        }
    }

    /// Pose la règle si elle manque : Windows demande alors son accord à la personne. Rend
    /// l'état relu ensuite, que la personne ait accepté ou non.
    pub(crate) fn ouvrir(&self, port: u16) -> EtatPareFeu {
        match self {
            Self::Windows { programme } => {
                if !programme.is_empty() && !self.lire(port).ouvert {
                    let _ = lancer(pare_feu::POWERSHELL, &pare_feu::ouverture(programme, port));
                }
            }
            Self::Essai {
                ouvert, demandes, ..
            } => {
                if demandes.fetch_add(1, Ordering::Relaxed) > 0 {
                    ouvert.store(true, Ordering::Relaxed);
                }
            }
        }
        self.lire(port)
    }
}
