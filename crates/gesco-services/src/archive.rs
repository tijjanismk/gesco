//! Format des archives de sauvegarde (spécification §11.2).
//!
//! ```text
//! "GESCOSAV" | format (u16) | longueur du manifeste (u32) | manifeste JSON, en clair
//! puis des blocs : dernier (u8) | longueur (u32) | bloc chiffré
//! ```
//!
//! Le contenu (base, modèles sur mesure) est mis bout à bout, compressé, puis découpé en blocs
//! d'un mébioctet chiffrés par la clé de données (`gesco_core::core::cle::chiffrer_bloc`). Le
//! manifeste reste lisible sans clé, pour l'assistant de restauration ; son empreinte est liée
//! à chaque bloc. Lecture et écriture se font en flux : une base de plusieurs centaines de
//! mébioctets ne tient jamais entière en mémoire.

use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Component, Path, PathBuf};

use flate2::Compression;
use flate2::read::DeflateDecoder;
use flate2::write::DeflateEncoder;
use gesco_core::core::cle::{chiffrer_bloc, dechiffrer_bloc};
use gesco_core::core::sauvegarde::FORMAT_ARCHIVE;
use gesco_core::core::{CleDonnees, Manifeste};
use sha2::{Digest, Sha256};

use crate::erreur::{ErreurService, Resultat};

const MAGIQUE: &[u8; 8] = b"GESCOSAV";
const TAILLE_BLOC: usize = 1 << 20;
const MANIFESTE_MAX: u32 = 1 << 20;
/// Étiquette Poly1305.
const SURCOUT_BLOC: usize = 16;

/// Extension des archives.
pub const EXTENSION: &str = "gsv";

/// Fichier à placer dans l'archive, sous un nom relatif.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EntreeArchive {
    pub(crate) nom: String,
    pub(crate) chemin: PathBuf,
}

fn alteree() -> ErreurService {
    ErreurService::Regle {
        cle: "sauvegarde.archive_alteree",
    }
}

fn empreinte_manifeste(octets: &[u8]) -> [u8; 32] {
    Sha256::digest(octets).into()
}

/// Empreinte SHA-256 d'un fichier, lu en flux.
pub(crate) fn empreinte_fichier(chemin: &Path) -> Resultat<String> {
    let mut lecteur = BufReader::new(File::open(chemin)?);
    let mut empreinte = Sha256::new();
    let mut tampon = vec![0u8; 64 * 1024];
    loop {
        let lus = lecteur.read(&mut tampon)?;
        if lus == 0 {
            break;
        }
        empreinte.update(tampon.get(..lus).unwrap_or_default());
    }
    Ok(hex::encode(empreinte.finalize()))
}

struct EcrivainChiffre<W: Write> {
    sortie: W,
    cle: CleDonnees,
    prefixe: [u8; 16],
    manifeste: [u8; 32],
    tampon: Vec<u8>,
    rang: u64,
}

impl<W: Write> EcrivainChiffre<W> {
    fn emettre(&mut self, longueur: usize, dernier: bool) -> std::io::Result<()> {
        let clair: Vec<u8> = self.tampon.drain(..longueur).collect();
        let chiffre = chiffrer_bloc(
            &self.cle,
            &self.prefixe,
            self.rang,
            dernier,
            &self.manifeste,
            &clair,
        )
        .map_err(std::io::Error::other)?;
        let longueur = u32::try_from(chiffre.len()).map_err(std::io::Error::other)?;
        self.sortie.write_all(&[u8::from(dernier)])?;
        self.sortie.write_all(&longueur.to_be_bytes())?;
        self.sortie.write_all(&chiffre)?;
        self.rang += 1;
        Ok(())
    }

    fn terminer(mut self) -> std::io::Result<W> {
        let reste = self.tampon.len();
        self.emettre(reste, true)?;
        self.sortie.flush()?;
        Ok(self.sortie)
    }
}

impl<W: Write> Write for EcrivainChiffre<W> {
    fn write(&mut self, octets: &[u8]) -> std::io::Result<usize> {
        self.tampon.extend_from_slice(octets);
        // On garde toujours au moins un octet : le dernier bloc porte le marqueur de fin.
        while self.tampon.len() > TAILLE_BLOC {
            self.emettre(TAILLE_BLOC, false)?;
        }
        Ok(octets.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.sortie.flush()
    }
}

struct LecteurChiffre<R: Read> {
    entree: R,
    cle: CleDonnees,
    prefixe: [u8; 16],
    manifeste: [u8; 32],
    tampon: Vec<u8>,
    position: usize,
    rang: u64,
    fini: bool,
}

impl<R: Read> LecteurChiffre<R> {
    fn bloc_suivant(&mut self) -> std::io::Result<()> {
        let mut entete = [0u8; 5];
        self.entree.read_exact(&mut entete).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "archive tronquée")
        })?;
        let [marqueur, l0, l1, l2, l3] = entete;
        let dernier = marqueur == 1;
        let longueur = u32::from_be_bytes([l0, l1, l2, l3]) as usize;
        if marqueur > 1 || longueur > TAILLE_BLOC + SURCOUT_BLOC {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "en-tête de bloc invalide",
            ));
        }
        let mut chiffre = vec![0u8; longueur];
        self.entree.read_exact(&mut chiffre)?;
        self.tampon = dechiffrer_bloc(
            &self.cle,
            &self.prefixe,
            self.rang,
            dernier,
            &self.manifeste,
            &chiffre,
        )
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        self.position = 0;
        self.rang += 1;
        self.fini = dernier;
        Ok(())
    }
}

impl<R: Read> Read for LecteurChiffre<R> {
    fn read(&mut self, sortie: &mut [u8]) -> std::io::Result<usize> {
        while self.position >= self.tampon.len() {
            if self.fini {
                return Ok(0);
            }
            self.bloc_suivant()?;
        }
        let disponibles = self.tampon.get(self.position..).unwrap_or_default();
        let n = disponibles.len().min(sortie.len());
        sortie
            .get_mut(..n)
            .unwrap_or_default()
            .copy_from_slice(disponibles.get(..n).unwrap_or_default());
        self.position += n;
        Ok(n)
    }
}

fn copier_entree(sortie: &mut impl Write, entree: &EntreeArchive) -> Resultat<()> {
    let nom = entree.nom.as_bytes();
    let longueur_nom = u16::try_from(nom.len()).map_err(|_| alteree())?;
    let taille = std::fs::metadata(&entree.chemin)?.len();
    sortie.write_all(&longueur_nom.to_be_bytes())?;
    sortie.write_all(nom)?;
    sortie.write_all(&taille.to_be_bytes())?;
    let copies = std::io::copy(&mut File::open(&entree.chemin)?, sortie)?;
    if copies != taille {
        return Err(alteree());
    }
    Ok(())
}

/// Écrit une archive. Rend sa taille. Le fichier n'apparaît sous son nom qu'une fois complet et
/// synchronisé sur disque.
pub(crate) fn ecrire(
    destination: &Path,
    manifeste: &Manifeste,
    cle: &CleDonnees,
    entrees: &[EntreeArchive],
) -> Resultat<u64> {
    let octets_manifeste = serde_json::to_vec_pretty(manifeste)?;
    let longueur = u32::try_from(octets_manifeste.len()).map_err(|_| alteree())?;
    let partiel = destination.with_extension("partiel");
    let mut fichier = BufWriter::new(File::create(&partiel)?);
    fichier.write_all(MAGIQUE)?;
    fichier.write_all(&FORMAT_ARCHIVE.to_be_bytes())?;
    fichier.write_all(&longueur.to_be_bytes())?;
    fichier.write_all(&octets_manifeste)?;
    let chiffreur = EcrivainChiffre {
        sortie: fichier,
        cle: cle.clone(),
        prefixe: manifeste.prefixe_nonce,
        manifeste: empreinte_manifeste(&octets_manifeste),
        tampon: Vec::with_capacity(TAILLE_BLOC + 64 * 1024),
        rang: 0,
    };
    let mut compresseur = DeflateEncoder::new(chiffreur, Compression::default());
    for entree in entrees {
        copier_entree(&mut compresseur, entree)?;
    }
    let fichier = compresseur.finish()?.terminer()?;
    let fichier = fichier.into_inner().map_err(|e| e.into_error())?;
    fichier.sync_all()?;
    drop(fichier);
    std::fs::rename(&partiel, destination)?;
    Ok(std::fs::metadata(destination)?.len())
}

fn ouvrir_manifeste(chemin: &Path) -> Resultat<(BufReader<File>, Manifeste, [u8; 32])> {
    let mut lecteur = BufReader::new(File::open(chemin)?);
    let mut entete = [0u8; 14];
    lecteur
        .read_exact(&mut entete)
        .map_err(|_| ErreurService::Regle {
            cle: "sauvegarde.fichier_inconnu",
        })?;
    let [m0, m1, m2, m3, m4, m5, m6, m7, f0, f1, l0, l1, l2, l3] = entete;
    if [m0, m1, m2, m3, m4, m5, m6, m7] != *MAGIQUE {
        return Err(ErreurService::Regle {
            cle: "sauvegarde.fichier_inconnu",
        });
    }
    if u16::from_be_bytes([f0, f1]) != FORMAT_ARCHIVE {
        return Err(ErreurService::Regle {
            cle: "sauvegarde.format_incompatible",
        });
    }
    let longueur = u32::from_be_bytes([l0, l1, l2, l3]);
    if longueur > MANIFESTE_MAX {
        return Err(alteree());
    }
    let mut octets = vec![0u8; longueur as usize];
    lecteur.read_exact(&mut octets).map_err(|_| alteree())?;
    let manifeste: Manifeste = serde_json::from_slice(&octets).map_err(|_| alteree())?;
    Ok((lecteur, manifeste, empreinte_manifeste(&octets)))
}

/// Lit le manifeste sans clé.
pub fn lire_manifeste(chemin: &Path) -> Resultat<Manifeste> {
    Ok(ouvrir_manifeste(chemin)?.1)
}

fn chemin_sur(dossier: &Path, nom: &str) -> Resultat<PathBuf> {
    let relatif = Path::new(nom);
    if nom.is_empty()
        || !relatif
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
    {
        return Err(alteree());
    }
    Ok(dossier.join(relatif))
}

/// Déchiffre et extrait une archive dans `dossier`. Toute altération (bloc, manifeste,
/// troncature, mauvaise clé) est une erreur ; rien n'est garanti sur les fichiers déjà écrits,
/// d'où l'extraction dans un dossier de travail.
pub(crate) fn extraire(chemin: &Path, cle: &CleDonnees, dossier: &Path) -> Resultat<Manifeste> {
    let (lecteur, manifeste, empreinte) = ouvrir_manifeste(chemin)?;
    let dechiffreur = LecteurChiffre {
        entree: lecteur,
        cle: cle.clone(),
        prefixe: manifeste.prefixe_nonce,
        manifeste: empreinte,
        tampon: Vec::new(),
        position: 0,
        rang: 0,
        fini: false,
    };
    let mut flux = DeflateDecoder::new(dechiffreur);
    let lire = |flux: &mut DeflateDecoder<_>, tampon: &mut [u8]| -> Resultat<bool> {
        match flux.read_exact(tampon) {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => Ok(false),
            Err(e) if e.kind() == std::io::ErrorKind::InvalidData => Err(alteree()),
            Err(e) => Err(e.into()),
        }
    };
    loop {
        let mut longueur_nom = [0u8; 2];
        if !lire(&mut flux, &mut longueur_nom)? {
            break;
        }
        let mut nom = vec![0u8; usize::from(u16::from_be_bytes(longueur_nom))];
        let mut taille = [0u8; 8];
        if !(lire(&mut flux, &mut nom)? && lire(&mut flux, &mut taille)?) {
            return Err(alteree());
        }
        let nom = String::from_utf8(nom).map_err(|_| alteree())?;
        let cible = chemin_sur(dossier, &nom)?;
        if let Some(parent) = cible.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let taille = u64::from_be_bytes(taille);
        let mut fichier = File::create(&cible)?;
        let copies =
            std::io::copy(&mut (&mut flux).take(taille), &mut fichier).map_err(|e| {
                match e.kind() {
                    std::io::ErrorKind::InvalidData | std::io::ErrorKind::UnexpectedEof => {
                        alteree()
                    }
                    _ => e.into(),
                }
            })?;
        if copies != taille {
            return Err(alteree());
        }
        fichier.sync_all()?;
    }
    // Le flux doit s'achever sur le dernier bloc authentifié : une archive tronquée échoue ici.
    if !flux.get_ref().fini {
        return Err(alteree());
    }
    Ok(manifeste)
}
