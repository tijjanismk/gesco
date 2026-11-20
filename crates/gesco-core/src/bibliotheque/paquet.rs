//! Paquet de la bibliothèque (ADR « paquet de contenus ») : un fichier `.gbib` signé par
//! l'éditeur, qui porte les chapitres, les contenus et leurs fichiers.
//!
//! ```text
//! "GBIB" | format (u16, petit-boutiste) | longueur du manifeste (u32) | manifeste JSON
//!        | signature Ed25519 du manifeste (64 octets) | fichiers compressés, bout à bout
//! ```
//!
//! Le manifeste porte l'empreinte SHA-256 et la taille de chaque fichier : la signature les couvre
//! tous. Un paquet dont un seul octet a changé est refusé entier. Lecture, vérification et
//! fabrication travaillent en mémoire, sans E/S (R-03).

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Marque de début d'un paquet.
const MARQUE: &[u8; 4] = b"GBIB";
/// Version du format de fichier.
pub const FORMAT_PAQUET: u16 = 1;
/// Taille d'une signature Ed25519.
const TAILLE_SIGNATURE: usize = 64;
/// Taille d'un fichier décompressé au-delà de laquelle le paquet est jugé abîmé.
const TAILLE_FICHIER_MAX: u64 = 64 * 1024 * 1024;

/// Pourquoi un paquet est refusé.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ErreurPaquet {
    /// Fichier tronqué, modifié, ou signé par une autre clé.
    #[error("Ce paquet de la bibliothèque est abîmé ou ne vient pas de GESCO.")]
    Abime,
    /// Format plus récent que celui que ce GESCO sait lire.
    #[error("Ce paquet de la bibliothèque demande une version plus récente de GESCO.")]
    FormatInconnu,
    /// Fabrication impossible (outil de l'éditeur).
    #[error("Fabrication du paquet impossible : {0}")]
    Fabrication(String),
}

/// Nature d'un contenu.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, specta::Type,
)]
#[serde(rename_all = "snake_case")]
pub enum NatureContenu {
    /// Leçon.
    Lecon,
    /// Énoncé d'un sujet d'examen.
    Sujet,
    /// Corrigé d'un sujet.
    Corrige,
    /// Document (PDF, images) ajouté par l'école.
    Document,
    /// Quiz d'un chapitre écrit par l'école : tout son texte suit la convention des quiz.
    Quiz,
}

impl NatureContenu {
    /// Nom persisté.
    pub fn nom(self) -> &'static str {
        match self {
            Self::Lecon => "lecon",
            Self::Sujet => "sujet",
            Self::Corrige => "corrige",
            Self::Document => "document",
            Self::Quiz => "quiz",
        }
    }

    /// Nature d'après son nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Some(match nom {
            "lecon" => Self::Lecon,
            "sujet" => Self::Sujet,
            "corrige" => Self::Corrige,
            "document" => Self::Document,
            "quiz" => Self::Quiz,
            _ => return None,
        })
    }
}

/// Examen d'un sujet.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, specta::Type,
)]
pub enum ExamenSujet {
    /// Diplôme d'études fondamentales.
    #[serde(rename = "DEF")]
    Def,
    /// Baccalauréat.
    #[serde(rename = "BAC")]
    Bac,
}

impl ExamenSujet {
    /// Sigle affiché et persisté.
    pub fn sigle(self) -> &'static str {
        match self {
            Self::Def => "DEF",
            Self::Bac => "BAC",
        }
    }
}

/// Discipline de référence (repère commun) : code de la matière livrée et son libellé.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisciplinePaquet {
    /// Code de référence (`MATH`, `PC`, `SVT`…).
    pub code: String,
    /// Libellé de référence.
    pub libelle: String,
}

/// Un chapitre livré.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChapitrePaquet {
    /// Clé stable d'une version du paquet à l'autre.
    pub cle: String,
    /// Code national du niveau (`7`, `8`, `9`, `12`…).
    pub niveau: String,
    /// Codes des séries ; vide pour toutes les séries du niveau.
    pub series: Vec<String>,
    /// Code de la discipline de référence.
    pub discipline: String,
    /// Partie du programme (« Algèbre », « Physique », « Examens »…) ; vide si la matière n'en a
    /// pas.
    pub partie: String,
    /// Titre du chapitre.
    pub titre: String,
    /// Rang dans la matière du niveau.
    pub ordre: u32,
    /// Chapitre bâti sans programme officiel : vu du seul éditeur.
    pub a_confirmer: bool,
}

/// Un contenu livré.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContenuPaquet {
    /// Clé stable d'une version du paquet à l'autre.
    pub cle: String,
    /// Nature.
    pub nature: NatureContenu,
    /// Clé du chapitre.
    pub chapitre: String,
    /// Titre affiché.
    pub titre: String,
    /// Rang dans le chapitre.
    pub ordre: u32,
    /// Contenu lié : le corrigé d'un sujet, le sujet d'un corrigé.
    pub lie: Option<String>,
    /// Examen d'un sujet ou d'un corrigé.
    pub examen: Option<ExamenSujet>,
    /// Session d'un sujet ou d'un corrigé.
    pub session: Option<u16>,
    /// Nom du fichier du texte, dans le paquet.
    pub fichier: String,
    /// Liste de faits jointe, vue du seul personnel qui écrit.
    pub faits: Option<String>,
    /// Images que le texte appelle, par leur nom.
    pub images: Vec<String>,
}

/// Un fichier du paquet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FichierPaquet {
    /// Nom, unique dans le paquet (`contenus/…`, `faits/…`, `images/…`).
    pub nom: String,
    /// Empreinte SHA-256 du fichier décompressé.
    pub empreinte: String,
    /// Taille décompressée, en octets.
    pub taille: u64,
    /// Début des données compressées, depuis le début de la partie des fichiers.
    pub debut: u64,
    /// Longueur des données compressées.
    pub longueur: u64,
}

/// Manifeste d'un paquet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestePaquet {
    /// Code du paquet (`bibliotheque-mali`).
    pub code: String,
    /// Version (`2026.11.1`).
    pub version: String,
    /// Date de fabrication (AAAA-MM-JJ).
    pub date: String,
    /// Version minimale de GESCO.
    pub gesco_minimum: String,
    /// Programme suivi par le découpage, pour l'éditeur ; jamais affiché.
    pub programme: String,
    /// Disciplines de référence.
    pub disciplines: Vec<DisciplinePaquet>,
    /// Chapitres.
    pub chapitres: Vec<ChapitrePaquet>,
    /// Contenus.
    pub contenus: Vec<ContenuPaquet>,
    /// Fichiers ; rempli à la fabrication.
    pub fichiers: Vec<FichierPaquet>,
}

/// Un paquet lu et vérifié : son manifeste et ses fichiers décompressés.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaquetLu {
    /// Manifeste.
    pub manifeste: ManifestePaquet,
    /// Fichiers par nom.
    pub fichiers: BTreeMap<String, Vec<u8>>,
}

impl PaquetLu {
    /// Texte d'un fichier.
    pub fn texte(&self, nom: &str) -> Option<&str> {
        self.fichiers
            .get(nom)
            .and_then(|o| std::str::from_utf8(o).ok())
    }
}

/// Empreinte SHA-256 en hexadécimal minuscule.
pub fn empreinte(octets: &[u8]) -> String {
    hex::encode(Sha256::digest(octets))
}

/// Compare deux versions pointées (`2026.11.1`, `2.0.0`) nombre par nombre ; un morceau qui
/// n'est pas un nombre vaut zéro.
pub fn comparer_versions(a: &str, b: &str) -> Ordering {
    let nombres = |v: &str| -> Vec<u64> {
        v.trim()
            .split('.')
            .map(|n| n.parse::<u64>().unwrap_or(0))
            .collect()
    };
    let (mut a, mut b) = (nombres(a), nombres(b));
    let n = a.len().max(b.len());
    a.resize(n, 0);
    b.resize(n, 0);
    a.cmp(&b)
}

/// Clé publique correspondant à une graine de clé privée.
pub fn cle_publique(graine: &[u8; 32]) -> [u8; 32] {
    SigningKey::from_bytes(graine).verifying_key().to_bytes()
}

/// Lit un paquet, vérifie sa signature et chaque fichier, et rend son contenu.
pub fn lire_paquet(octets: &[u8], cle_editeur: &[u8; 32]) -> Result<PaquetLu, ErreurPaquet> {
    let (manifeste_octets, signature, donnees) = decouper(octets)?;
    let cle = VerifyingKey::from_bytes(cle_editeur).map_err(|_| ErreurPaquet::Abime)?;
    cle.verify_strict(manifeste_octets, &Signature::from_bytes(&signature))
        .map_err(|_| ErreurPaquet::Abime)?;
    let manifeste: ManifestePaquet =
        serde_json::from_slice(manifeste_octets).map_err(|_| ErreurPaquet::Abime)?;

    let mut fichiers = BTreeMap::new();
    let mut fin_attendue = 0_u64;
    for f in &manifeste.fichiers {
        // Les fichiers se suivent sans trou ni chevauchement : aucun octet n'échappe à la
        // vérification.
        if f.debut != fin_attendue || f.taille > TAILLE_FICHIER_MAX {
            return Err(ErreurPaquet::Abime);
        }
        fin_attendue = f.debut.checked_add(f.longueur).ok_or(ErreurPaquet::Abime)?;
        let debut = usize::try_from(f.debut).map_err(|_| ErreurPaquet::Abime)?;
        let fin = usize::try_from(fin_attendue).map_err(|_| ErreurPaquet::Abime)?;
        let compresse = donnees.get(debut..fin).ok_or(ErreurPaquet::Abime)?;
        let mut decompresse = Vec::new();
        flate2::read::DeflateDecoder::new(compresse)
            .take(f.taille.saturating_add(1))
            .read_to_end(&mut decompresse)
            .map_err(|_| ErreurPaquet::Abime)?;
        if decompresse.len() as u64 != f.taille || empreinte(&decompresse) != f.empreinte {
            return Err(ErreurPaquet::Abime);
        }
        if fichiers.insert(f.nom.clone(), decompresse).is_some() {
            return Err(ErreurPaquet::Abime);
        }
    }
    if fin_attendue != donnees.len() as u64 {
        return Err(ErreurPaquet::Abime);
    }
    verifier_references(&manifeste, &fichiers)?;
    Ok(PaquetLu {
        manifeste,
        fichiers,
    })
}

/// Lit le seul manifeste d'un paquet, sans vérifier la signature (pour dire sa version avant de
/// l'installer). Le paquet n'est installé qu'après `lire_paquet`.
pub fn lire_manifeste_sans_verifier(octets: &[u8]) -> Result<ManifestePaquet, ErreurPaquet> {
    let (manifeste, _, _) = decouper(octets)?;
    serde_json::from_slice(manifeste).map_err(|_| ErreurPaquet::Abime)
}

type Morceaux<'a> = (&'a [u8], [u8; TAILLE_SIGNATURE], &'a [u8]);

fn decouper(octets: &[u8]) -> Result<Morceaux<'_>, ErreurPaquet> {
    let marque = octets.get(..4).ok_or(ErreurPaquet::Abime)?;
    if marque != MARQUE {
        return Err(ErreurPaquet::Abime);
    }
    let format = octets
        .get(4..6)
        .and_then(|o| <[u8; 2]>::try_from(o).ok())
        .map(u16::from_le_bytes)
        .ok_or(ErreurPaquet::Abime)?;
    if format != FORMAT_PAQUET {
        return Err(ErreurPaquet::FormatInconnu);
    }
    let longueur = octets
        .get(6..10)
        .and_then(|o| <[u8; 4]>::try_from(o).ok())
        .map(u32::from_le_bytes)
        .ok_or(ErreurPaquet::Abime)?;
    let longueur = usize::try_from(longueur).map_err(|_| ErreurPaquet::Abime)?;
    let fin_manifeste = 10_usize.checked_add(longueur).ok_or(ErreurPaquet::Abime)?;
    let manifeste = octets.get(10..fin_manifeste).ok_or(ErreurPaquet::Abime)?;
    let fin_signature = fin_manifeste
        .checked_add(TAILLE_SIGNATURE)
        .ok_or(ErreurPaquet::Abime)?;
    let signature: [u8; TAILLE_SIGNATURE] = octets
        .get(fin_manifeste..fin_signature)
        .and_then(|o| o.try_into().ok())
        .ok_or(ErreurPaquet::Abime)?;
    let donnees = octets.get(fin_signature..).ok_or(ErreurPaquet::Abime)?;
    Ok((manifeste, signature, donnees))
}

/// Chaque chapitre, contenu lié, texte, liste de faits et image cités existent ; clés uniques.
fn verifier_references(
    manifeste: &ManifestePaquet,
    fichiers: &BTreeMap<String, Vec<u8>>,
) -> Result<(), ErreurPaquet> {
    let mut chapitres = BTreeSet::new();
    for c in &manifeste.chapitres {
        if !chapitres.insert(c.cle.as_str()) {
            return Err(ErreurPaquet::Abime);
        }
    }
    let mut contenus = BTreeSet::new();
    for c in &manifeste.contenus {
        if !contenus.insert(c.cle.as_str()) {
            return Err(ErreurPaquet::Abime);
        }
    }
    for c in &manifeste.contenus {
        let present = |nom: &str| fichiers.contains_key(nom);
        let images_presentes = c
            .images
            .iter()
            .all(|i| present(&nom_fichier_image(&c.cle, i)));
        if !chapitres.contains(c.chapitre.as_str())
            || !present(&c.fichier)
            || c.faits.as_deref().is_some_and(|f| !present(f))
            || c.lie.as_deref().is_some_and(|l| !contenus.contains(l))
            || !images_presentes
        {
            return Err(ErreurPaquet::Abime);
        }
    }
    Ok(())
}

/// Nom dans le paquet de l'image que le contenu `cle` appelle par `nom` : les images sont rangées
/// par dossier de contenu, deux leçons pouvant appeler deux figures du même nom.
pub fn nom_fichier_image(cle: &str, nom: &str) -> String {
    match cle.rsplit_once('/') {
        Some((dossier, _)) => format!("images/{dossier}/{nom}"),
        None => format!("images/{nom}"),
    }
}

/// Fabrique un paquet signé (outil de l'éditeur). Les fichiers sont rangés dans l'ordre donné ;
/// le champ `fichiers` du manifeste est rempli ici.
pub fn fabriquer_paquet(
    manifeste: &ManifestePaquet,
    fichiers: &[(String, Vec<u8>)],
    graine: &[u8; 32],
) -> Result<Vec<u8>, ErreurPaquet> {
    let erreur = |e: &dyn std::fmt::Display| ErreurPaquet::Fabrication(e.to_string());
    let mut manifeste = manifeste.clone();
    manifeste.fichiers.clear();
    let mut donnees = Vec::new();
    for (nom, octets) in fichiers {
        let mut compresseur =
            flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::best());
        compresseur.write_all(octets).map_err(|e| erreur(&e))?;
        let compresse = compresseur.finish().map_err(|e| erreur(&e))?;
        manifeste.fichiers.push(FichierPaquet {
            nom: nom.clone(),
            empreinte: empreinte(octets),
            taille: octets.len() as u64,
            debut: donnees.len() as u64,
            longueur: compresse.len() as u64,
        });
        donnees.extend_from_slice(&compresse);
    }
    let presents: BTreeMap<String, Vec<u8>> = fichiers.iter().cloned().collect();
    if presents.len() != fichiers.len() {
        return Err(ErreurPaquet::Fabrication(
            "deux fichiers portent le même nom".into(),
        ));
    }
    verifier_references(&manifeste, &presents)
        .map_err(|_| ErreurPaquet::Fabrication("référence sans fichier ni chapitre".into()))?;
    let texte = serde_json::to_vec(&manifeste).map_err(|e| erreur(&e))?;
    let longueur = u32::try_from(texte.len()).map_err(|e| erreur(&e))?;
    let signature = SigningKey::from_bytes(graine).sign(&texte);
    let mut paquet = Vec::with_capacity(10 + texte.len() + TAILLE_SIGNATURE + donnees.len());
    paquet.extend_from_slice(MARQUE);
    paquet.extend_from_slice(&FORMAT_PAQUET.to_le_bytes());
    paquet.extend_from_slice(&longueur.to_le_bytes());
    paquet.extend_from_slice(&texte);
    paquet.extend_from_slice(&signature.to_bytes());
    paquet.extend_from_slice(&donnees);
    Ok(paquet)
}

/// Marques qu'aucun texte affiché ne doit porter (règle 6 de la spécification) : sources, notes de
/// fabrication, numéros de question, chemins de disque, adresses web, et toute mention d'une
/// relecture de la leçon par un enseignant ou par la direction. Rend celles qui sont trouvées.
pub fn marques_interdites(texte: &str) -> Vec<String> {
    let mut trouvees = Vec::new();
    // « Source : … » en tête de ligne cite une origine ; « le Niger prend sa source : » non.
    let source = texte.lines().any(|l| {
        let l = l.trim_start_matches(|c: char| c.is_whitespace() || matches!(c, '*' | '_' | '>'));
        ["Source :", "Source:", "Sources :", "Sources:"]
            .iter()
            .any(|m| l.starts_with(m))
    });
    if source {
        trouvees.push("source :".to_owned());
    }
    let minuscule = texte.to_lowercase();
    for marque in [
        ".verif.py",
        ".fig.py",
        "image à chercher",
        "http://",
        "https://",
        "www.",
        "pas encore relu",
        "relu par un enseignant",
        "relue par un enseignant",
        "relu par un professeur",
        "relue par un professeur",
        "relu par la direction",
        "relue par la direction",
        "rédigé pour gesco",
        "rédigée pour gesco",
        "recalculés par un programme",
    ] {
        if contient_mot(&minuscule, marque) {
            trouvees.push(marque.to_owned());
        }
    }
    if texte
        .split(|c: char| !c.is_alphanumeric())
        .any(|m| m == "SPEC")
    {
        trouvees.push("SPEC".to_owned());
    }
    // « T-116 », « Q-03 » : une lettre seule, un tiret, un chiffre ; « C:\ » : un chemin.
    let caracteres: Vec<char> = std::iter::once(' ').chain(texte.chars()).collect();
    for fenetre in caracteres.windows(4) {
        if let [avant, lettre, '-', chiffre] = fenetre
            && !avant.is_alphanumeric()
            && matches!(lettre, 'T' | 'Q')
            && chiffre.is_ascii_digit()
        {
            let marque = format!("{lettre}-{chiffre}");
            if !trouvees.contains(&marque) {
                trouvees.push(marque);
            }
        }
        if let [avant, lettre, ':', '\\'] = fenetre
            && !avant.is_alphanumeric()
            && lettre.is_ascii_alphabetic()
        {
            let marque = "chemin de disque".to_owned();
            if !trouvees.contains(&marque) {
                trouvees.push(marque);
            }
        }
    }
    trouvees
}

/// Vrai si `texte` contient `marque` sans lettre juste avant (« ressource : » n'est pas
/// « source : »).
fn contient_mot(texte: &str, marque: &str) -> bool {
    texte.match_indices(marque).any(|(i, _)| {
        marque.starts_with('.')
            || texte
                .get(..i)
                .and_then(|avant| avant.chars().last())
                .is_none_or(|c| !c.is_alphanumeric())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const GRAINE: [u8; 32] = [7; 32];

    fn manifeste() -> ManifestePaquet {
        ManifestePaquet {
            code: "bibliotheque-mali".to_owned(),
            version: "2026.11.1".to_owned(),
            date: "2026-11-15".to_owned(),
            gesco_minimum: "2.0.0".to_owned(),
            programme: "Programme d'essai".to_owned(),
            disciplines: vec![DisciplinePaquet {
                code: "MATH".to_owned(),
                libelle: "Mathématiques".to_owned(),
            }],
            chapitres: vec![ChapitrePaquet {
                cle: "9-math-reels".to_owned(),
                niveau: "9".to_owned(),
                series: Vec::new(),
                discipline: "MATH".to_owned(),
                partie: "Algèbre".to_owned(),
                titre: "Chapitre I — Nombres réels".to_owned(),
                ordre: 1,
                a_confirmer: false,
            }],
            contenus: vec![ContenuPaquet {
                cle: "9-math-racine".to_owned(),
                nature: NatureContenu::Lecon,
                chapitre: "9-math-reels".to_owned(),
                titre: "La racine carrée".to_owned(),
                ordre: 1,
                lie: None,
                examen: None,
                session: None,
                fichier: "contenus/9-math-racine.md".to_owned(),
                faits: None,
                images: vec!["carre.svg".to_owned()],
            }],
            fichiers: Vec::new(),
        }
    }

    fn fichiers() -> Vec<(String, Vec<u8>)> {
        vec![
            (
                "contenus/9-math-racine.md".to_owned(),
                "## L'essentiel\n\n$\\sqrt{4} = 2$ ![Un carré](carre.svg)\n"
                    .repeat(50)
                    .into_bytes(),
            ),
            ("images/carre.svg".to_owned(), b"<svg/>".to_vec()),
        ]
    }

    #[test]
    fn un_paquet_fabrique_se_relit() {
        let paquet = fabriquer_paquet(&manifeste(), &fichiers(), &GRAINE).unwrap_or_default();
        let lu = match lire_paquet(&paquet, &cle_publique(&GRAINE)) {
            Ok(lu) => lu,
            Err(e) => unreachable!("paquet refusé : {e}"),
        };
        assert_eq!(lu.manifeste.version, "2026.11.1");
        assert_eq!(lu.manifeste.fichiers.len(), 2);
        assert!(
            lu.texte("contenus/9-math-racine.md")
                .is_some_and(|t| t.contains("sqrt"))
        );
        assert_eq!(
            lire_manifeste_sans_verifier(&paquet).map(|m| m.code),
            Ok("bibliotheque-mali".to_owned())
        );
    }

    #[test]
    fn un_octet_change_n_importe_ou_fait_refuser_le_paquet() {
        let paquet = fabriquer_paquet(&manifeste(), &fichiers(), &GRAINE).unwrap_or_default();
        let cle = cle_publique(&GRAINE);
        for position in (0..paquet.len()).step_by(7) {
            let mut abime = paquet.clone();
            if let Some(o) = abime.get_mut(position) {
                *o ^= 0x01;
            }
            assert!(
                lire_paquet(&abime, &cle).is_err(),
                "octet {position} changé, paquet accepté"
            );
        }
        let mut allonge = paquet.clone();
        allonge.push(0);
        assert_eq!(lire_paquet(&allonge, &cle), Err(ErreurPaquet::Abime));
        let tronque = paquet.get(..paquet.len() - 1).unwrap_or_default();
        assert_eq!(lire_paquet(tronque, &cle), Err(ErreurPaquet::Abime));
        assert_eq!(
            lire_paquet(&paquet, &cle_publique(&[8; 32])),
            Err(ErreurPaquet::Abime)
        );
    }

    #[test]
    fn une_image_absente_empeche_la_fabrication() {
        let sans_image: Vec<_> = fichiers().into_iter().take(1).collect();
        assert!(matches!(
            fabriquer_paquet(&manifeste(), &sans_image, &GRAINE),
            Err(ErreurPaquet::Fabrication(_))
        ));
    }

    #[test]
    fn versions_comparees_nombre_par_nombre() {
        assert_eq!(comparer_versions("2026.11.1", "2026.11.1"), Ordering::Equal);
        assert_eq!(comparer_versions("2026.11.2", "2026.11.10"), Ordering::Less);
        assert_eq!(comparer_versions("2026.12", "2026.11.9"), Ordering::Greater);
        assert_eq!(comparer_versions("2.0", "2.0.0"), Ordering::Equal);
    }

    #[test]
    fn marques_interdites_trouvees() {
        assert!(marques_interdites("Le carré de 3 vaut 9.").is_empty());
        assert!(
            marques_interdites(
                "Il a relu sa lettre ; le T-shirt ; le Niger prend sa source : ici ; PT-100."
            )
            .is_empty()
        );
        let m = marques_interdites(
            "Source : manuel. Voir T-116, SPEC, C:\\Users, https://x.org. Pas encore relu par un enseignant.",
        );
        for attendue in [
            "source :",
            "T-1",
            "SPEC",
            "chemin de disque",
            "https://",
            "pas encore relu",
            "relu par un enseignant",
        ] {
            assert!(
                m.iter().any(|x| x == attendue),
                "{attendue} manque dans {m:?}"
            );
        }
    }
}
