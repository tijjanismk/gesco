//! `gesco-paquet-bibliotheque` — outil de l'éditeur (ADR « paquet de contenus »).
//!
//! ```text
//! gesco-paquet-bibliotheque cles <fichier-cle-privee>
//! gesco-paquet-bibliotheque fabriquer --contenus <dossier> --cle <fichier-cle-privee>
//!                           --version <2026.11.1> --date <AAAA-MM-JJ> --sortie <fichier.gbib>
//! gesco-paquet-bibliotheque verifier --paquet <fichier.gbib> --cle-publique <hexadécimal>
//! ```
//!
//! `fabriquer` lit le dossier `contenus` de la bibliothèque : leçons (`lecon-*.md`) et sujets
//! (`sujets/def/…/def-*.md`, `sujets/bac/…/bac-*.md`). Il tire des en-têtes le titre, le
//! classement et la nature, retire du texte affiché les lignes d'état et de source, change les
//! lignes « Image à chercher » et la section « Figures qui manquent » en notes pour le maître,
//! joint à chaque leçon sa liste de faits, puis refuse de fabriquer si un contenu n'a pas de
//! chapitre, si une image appelée manque, si une formule ne se convertit pas pour l'impression ou
//! si un texte affiché porte une marque interdite. Le tableau de couverture est écrit à côté du
//! paquet (`…-couverture.csv`).
//!
//! La clé privée des paquets reste sur le poste de l'éditeur, hors du dépôt ; elle est distincte
//! de celle des licences. Seule la clé publique est livrée, dans
//! `crates/gesco-services/cle-publique-bibliotheque.txt`.

use std::collections::{BTreeMap, HashMap};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use gesco_core::bibliotheque::paquet::{
    self, ChapitrePaquet, ContenuPaquet, DisciplinePaquet, ExamenSujet, ManifestePaquet,
    NatureContenu,
};
use gesco_core::bibliotheque::{SourceImage, analyser, marques_interdites};
use gesco_docs::formules::latex_vers_typst;
use zeroize::Zeroizing;

const USAGE: &str = "\
Usage :
  gesco-paquet-bibliotheque cles <fichier-cle-privee>
  gesco-paquet-bibliotheque fabriquer --contenus <dossier> --cle <fichier> --version <version>
                            --date <AAAA-MM-JJ> --sortie <fichier.gbib>
  gesco-paquet-bibliotheque verifier --paquet <fichier.gbib> --cle-publique <hexadécimal>";

/// Code du seul paquet livré aujourd'hui.
const CODE_PAQUET: &str = "bibliotheque-mali";
/// Version minimale de GESCO qui lit ce format.
const GESCO_MINIMUM: &str = "2.0.0";
/// Programme que suit le découpage, nommé pour l'éditeur (jamais affiché).
const PROGRAMME: &str = "Découpage de la bibliothèque GESCO, novembre 2026";
/// Début d'une note pour le maître (reconnue par le noyau).
const NOTE: &str = "> **Note pour le maître** : ";

/// Dossier de matière → (code de référence, libellé de référence).
const DISCIPLINES: [(&str, &str, &str); 8] = [
    ("mathematiques", "MATH", "Mathématiques"),
    ("mathematiques-tse", "MATH", "Mathématiques"),
    ("physique-chimie", "PC", "Physique-chimie"),
    ("biologie", "SVT", "Biologie"),
    ("francais", "FR", "Français"),
    ("histoire-geographie", "HG", "Histoire-géographie"),
    ("anglais", "ANG", "Anglais"),
    ("ecm", "ECM", "Éducation civique et morale"),
];

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let issue = match arguments.split_first() {
        Some((commande, reste)) if commande == "cles" => generer_cles(reste),
        Some((commande, reste)) if commande == "fabriquer" => fabriquer(reste),
        Some((commande, reste)) if commande == "verifier" => verifier(reste),
        _ => Err(USAGE.to_owned()),
    };
    match issue {
        Ok(message) => {
            let _ = writeln!(std::io::stdout(), "{message}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            let _ = writeln!(std::io::stderr(), "{message}");
            ExitCode::FAILURE
        }
    }
}

fn options(arguments: &[String]) -> Result<HashMap<String, String>, String> {
    let mut options = HashMap::new();
    let mut suite = arguments.iter();
    while let Some(nom) = suite.next() {
        let nom = nom
            .strip_prefix("--")
            .ok_or_else(|| format!("argument inattendu : {nom}\n{USAGE}"))?;
        let valeur = suite
            .next()
            .ok_or_else(|| format!("--{nom} attend une valeur"))?;
        options.insert(nom.to_owned(), valeur.clone());
    }
    Ok(options)
}

fn exiger(options: &HashMap<String, String>, nom: &str) -> Result<String, String> {
    options
        .get(nom)
        .cloned()
        .ok_or_else(|| format!("--{nom} manque\n{USAGE}"))
}

/// Tire une graine de clé privée, l'écrit dans un fichier neuf et rend la clé publique.
fn generer_cles(arguments: &[String]) -> Result<String, String> {
    let [chemin] = arguments else {
        return Err(USAGE.to_owned());
    };
    let chemin = Path::new(chemin);
    if chemin.exists() {
        return Err(format!(
            "{} existe déjà : une clé de l'éditeur ne s'écrase jamais.",
            chemin.display()
        ));
    }
    let mut graine = Zeroizing::new([0u8; 32]);
    getrandom::fill(graine.as_mut()).map_err(|e| format!("aléa indisponible : {e}"))?;
    if let Some(dossier) = chemin.parent() {
        std::fs::create_dir_all(dossier).map_err(|e| e.to_string())?;
    }
    std::fs::write(chemin, hex::encode(graine.as_slice())).map_err(|e| e.to_string())?;
    Ok(format!(
        "Clé privée des paquets écrite dans {} — à sauvegarder hors ligne, jamais dans le dépôt.\n\
         Clé publique : {}",
        chemin.display(),
        hex::encode(paquet::cle_publique(&graine))
    ))
}

fn lire_graine(chemin: &Path) -> Result<Zeroizing<[u8; 32]>, String> {
    let texte = Zeroizing::new(
        std::fs::read_to_string(chemin).map_err(|e| format!("{} : {e}", chemin.display()))?,
    );
    let octets = Zeroizing::new(hex::decode(texte.trim()).map_err(|e| e.to_string())?);
    let graine: [u8; 32] = octets
        .as_slice()
        .try_into()
        .map_err(|_| "clé privée de 32 octets attendue".to_owned())?;
    Ok(Zeroizing::new(graine))
}

/// Vérifie un paquet fabriqué contre une clé publique.
fn verifier(arguments: &[String]) -> Result<String, String> {
    let options = options(arguments)?;
    let octets = std::fs::read(exiger(&options, "paquet")?).map_err(|e| e.to_string())?;
    let cle: [u8; 32] = hex::decode(exiger(&options, "cle-publique")?.trim())
        .ok()
        .and_then(|o| o.try_into().ok())
        .ok_or("clé publique de 32 octets attendue")?;
    let lu = paquet::lire_paquet(&octets, &cle).map_err(|e| e.to_string())?;
    Ok(format!(
        "Paquet {} {} vérifié : {} chapitres, {} contenus, {} fichiers.",
        lu.manifeste.code,
        lu.manifeste.version,
        lu.manifeste.chapitres.len(),
        lu.manifeste.contenus.len(),
        lu.fichiers.len()
    ))
}

/// Un contenu lu dans la bibliothèque, avant d'être rangé.
struct Lu {
    cle: String,
    nature: NatureContenu,
    niveau: String,
    series: Vec<String>,
    discipline: String,
    partie: String,
    chapitre: String,
    a_confirmer: bool,
    titre: String,
    examen: Option<ExamenSujet>,
    session: Option<u16>,
    texte: String,
    faits: Option<String>,
    images: Vec<(String, PathBuf)>,
    /// Rang de tri dans le chapitre.
    tri: String,
}

fn fabriquer(arguments: &[String]) -> Result<String, String> {
    let options = options(arguments)?;
    let racine = PathBuf::from(exiger(&options, "contenus")?);
    let graine = lire_graine(Path::new(&exiger(&options, "cle")?))?;
    let version = exiger(&options, "version")?;
    let date = exiger(&options, "date")?;
    let sortie = PathBuf::from(exiger(&options, "sortie")?);

    let mut fichiers = Vec::new();
    trouver(&racine, &racine, &mut fichiers)?;
    fichiers.sort();

    let mut refus = Vec::new();
    let mut lus = Vec::new();
    for relatif in &fichiers {
        match lire_contenu(&racine, relatif) {
            Ok(lu) => {
                refus.extend(controler(&lu));
                lus.push(lu);
            }
            Err(e) => refus.push(format!("{relatif} : {e}")),
        }
    }
    let (manifeste, paquet_fichiers) = ranger(&lus, &version, &date, &mut refus)?;
    if !refus.is_empty() {
        return Err(format!(
            "Paquet non fabriqué, {} refus :\n{}",
            refus.len(),
            refus.join("\n")
        ));
    }
    let octets = paquet::fabriquer_paquet(&manifeste, &paquet_fichiers, &graine)
        .map_err(|e| e.to_string())?;
    // Relecture du paquet écrit, comme le fera GESCO.
    paquet::lire_paquet(&octets, &paquet::cle_publique(&graine)).map_err(|e| e.to_string())?;
    if let Some(dossier) = sortie.parent() {
        std::fs::create_dir_all(dossier).map_err(|e| e.to_string())?;
    }
    std::fs::write(&sortie, &octets).map_err(|e| e.to_string())?;
    let couverture = sortie.with_file_name(format!(
        "{}-couverture.csv",
        sortie
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default()
    ));
    let (tableau, a_reprendre) = tableau_de_couverture(&manifeste, &lus);
    std::fs::write(&couverture, tableau).map_err(|e| e.to_string())?;
    let quiz_a_reprendre = couverture.with_file_name(
        couverture
            .file_name()
            .map(|n| {
                n.to_string_lossy()
                    .replace("-couverture.csv", "-quiz-a-reprendre.txt")
            })
            .unwrap_or_default(),
    );
    std::fs::write(&quiz_a_reprendre, a_reprendre).map_err(|e| e.to_string())?;
    let compter = |n: NatureContenu| manifeste.contenus.iter().filter(|c| c.nature == n).count();
    Ok(format!(
        "Paquet {} {} écrit dans {} ({} octets) : {} chapitres, {} leçons, {} sujets, {} corrigés.\n\
         Couverture : {}",
        CODE_PAQUET,
        version,
        sortie.display(),
        octets.len(),
        manifeste.chapitres.len(),
        compter(NatureContenu::Lecon),
        compter(NatureContenu::Sujet),
        compter(NatureContenu::Corrige),
        couverture.display()
    ))
}

/// Chemins relatifs (séparés par `/`) des leçons et des sujets.
fn trouver(racine: &Path, dossier: &Path, trouves: &mut Vec<String>) -> Result<(), String> {
    for entree in std::fs::read_dir(dossier).map_err(|e| format!("{} : {e}", dossier.display()))? {
        let chemin = entree.map_err(|e| e.to_string())?.path();
        let nom = chemin
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if chemin.is_dir() {
            if !nom.starts_with('_') && !nom.starts_with('.') {
                trouver(racine, &chemin, trouves)?;
            }
            continue;
        }
        let contenu = nom.ends_with(".md")
            && !nom.ends_with(".faits.md")
            && ["lecon-", "def-", "bac-"]
                .iter()
                .any(|p| nom.starts_with(p));
        if contenu {
            let relatif = chemin
                .strip_prefix(racine)
                .map_err(|e| e.to_string())?
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            trouves.push(relatif);
        }
    }
    Ok(())
}

fn discipline(dossier: &str) -> Option<(&'static str, &'static str)> {
    DISCIPLINES
        .iter()
        .find(|(d, _, _)| *d == dossier)
        .map(|(_, code, libelle)| (*code, *libelle))
}

fn niveau(dossier: &str) -> Option<&'static str> {
    Some(match dossier {
        "7e-annee" => "7",
        "8e-annee" => "8",
        "9e-annee" => "9",
        "terminale" => "12",
        _ => return None,
    })
}

/// Séries nommées dans un texte (« Terminale TSE et TSExp »).
fn series(texte: &str) -> Vec<String> {
    let mut series: Vec<String> = texte
        .split(|c: char| !c.is_alphanumeric())
        .filter(|m| matches!(*m, "TSE" | "TSExp" | "TSECO" | "TLL" | "TAL"))
        .map(str::to_owned)
        .collect();
    series.sort();
    series.dedup();
    series
}

/// Identifiant ASCII en minuscules (`Chapitre I — Nombres réels` → `chapitre-i-nombres-reels`).
fn slug(texte: &str) -> String {
    let mut sortie = String::new();
    for c in texte.chars() {
        let c = match c {
            'à' | 'â' | 'ä' | 'À' | 'Â' => 'a',
            'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' => 'e',
            'î' | 'ï' | 'Î' => 'i',
            'ô' | 'ö' | 'Ô' => 'o',
            'ù' | 'û' | 'ü' | 'Ù' | 'Û' => 'u',
            'ç' | 'Ç' => 'c',
            autre => autre,
        };
        if c.is_ascii_alphanumeric() {
            sortie.push(c.to_ascii_lowercase());
        } else if !sortie.ends_with('-') && !sortie.is_empty() {
            sortie.push('-');
        }
    }
    sortie.trim_end_matches('-').to_owned()
}

/// En-tête (titre, lignes jusqu'au premier filet) et corps d'un fichier.
fn decouper(texte: &str) -> Result<(String, Vec<String>, Vec<String>), String> {
    let lignes: Vec<String> = texte.lines().map(str::to_owned).collect();
    let titre = lignes
        .iter()
        .find_map(|l| l.strip_prefix("# "))
        .map(|t| t.trim().to_owned())
        .ok_or("pas de titre")?;
    let filet = lignes
        .iter()
        .position(|l| l.trim() == "---")
        .ok_or("pas de filet sous l'en-tête")?;
    let entete = lignes.get(..filet).unwrap_or_default().to_vec();
    let corps = lignes.get(filet + 1..).unwrap_or_default().to_vec();
    Ok((titre, entete, corps))
}

/// Ligne de classement en gras (`**7e année · Biologie · Chapitre A — … · Leçon 1**`), sur une
/// ou plusieurs lignes.
fn classement(entete: &[String]) -> Option<String> {
    let debut = entete
        .iter()
        .position(|l| l.trim_start().starts_with("**"))?;
    let mut texte = String::new();
    for ligne in entete.get(debut..)? {
        texte.push(' ');
        texte.push_str(ligne.trim());
        if ligne.trim_end().ends_with("**") && texte.trim().len() > 2 {
            break;
        }
    }
    Some(texte.trim().trim_matches('*').trim().to_owned())
}

fn lire_contenu(racine: &Path, relatif: &str) -> Result<Lu, String> {
    let chemin = racine.join(relatif);
    let texte = std::fs::read_to_string(&chemin).map_err(|e| e.to_string())?;
    let (titre, entete, corps) = decouper(&texte)?;
    let morceaux: Vec<&str> = relatif.split('/').collect();
    let nom = morceaux.last().copied().unwrap_or_default();
    let tige = nom.trim_end_matches(".md");
    let dossier = chemin.parent().map(Path::to_path_buf).unwrap_or_default();
    let a_confirmer = entete.iter().any(|l| l.contains("à confirmer"));

    let mut lu = if morceaux.first() == Some(&"sujets") {
        // sujets/def/<matière>/def-2024-….md, sujets/bac/<série>/<matière>/bac-2025-….md
        let examen = match morceaux.get(1) {
            Some(&"def") => ExamenSujet::Def,
            Some(&"bac") => ExamenSujet::Bac,
            _ => return Err("examen inconnu".into()),
        };
        let (serie, matiere) = match examen {
            ExamenSujet::Def => (None, morceaux.get(2)),
            ExamenSujet::Bac => (morceaux.get(2), morceaux.get(3)),
        };
        let (code, _) = matiere
            .and_then(|m| discipline(m))
            .ok_or("matière inconnue")?;
        let session = tige
            .split('-')
            .find_map(|m| m.parse::<u16>().ok().filter(|a| *a > 1900))
            .ok_or("session introuvable")?;
        let corrige = tige.ends_with("-corrige");
        let serie_code = serie.map(|s| match *s {
            "tse" => "TSE".to_owned(),
            "tsexp" => "TSExp".to_owned(),
            autre => autre.to_uppercase(),
        });
        let titre_chapitre = match examen {
            ExamenSujet::Def => "Sujets du DEF",
            ExamenSujet::Bac => "Sujets du BAC",
        };
        Lu {
            cle: relatif.trim_end_matches(".md").to_owned(),
            nature: if corrige {
                NatureContenu::Corrige
            } else {
                NatureContenu::Sujet
            },
            niveau: if examen == ExamenSujet::Def {
                "9"
            } else {
                "12"
            }
            .to_owned(),
            series: serie_code.into_iter().collect(),
            discipline: code.to_owned(),
            partie: "Examens".to_owned(),
            chapitre: titre_chapitre.to_owned(),
            a_confirmer: false,
            titre,
            examen: Some(examen),
            session: Some(session),
            texte: String::new(),
            faits: None,
            images: Vec::new(),
            tri: format!(
                "{session}-{}-{}",
                tige.trim_end_matches("-corrige"),
                u8::from(corrige)
            ),
        }
    } else {
        let niveau = morceaux
            .first()
            .and_then(|n| niveau(n))
            .ok_or("niveau inconnu")?;
        let (code, libelle) = morceaux
            .get(1)
            .and_then(|m| discipline(m))
            .ok_or("matière inconnue")?;
        let classement = classement(&entete).ok_or("pas de ligne de classement")?;
        let elements: Vec<&str> = classement.split(" · ").map(str::trim).collect();
        let premier = elements.first().copied().unwrap_or_default();
        let sous = elements.get(1).copied().unwrap_or_default();
        let milieu = elements
            .get(2..elements.len().saturating_sub(1))
            .unwrap_or_default();
        let sous_discipline = !sous.is_empty() && sous != libelle;
        let (partie, chapitre) = match milieu {
            [partie, .., chapitre] => ((*partie).to_owned(), (*chapitre).to_owned()),
            [chapitre] if sous_discipline => (sous.to_owned(), (*chapitre).to_owned()),
            [chapitre] => (String::new(), (*chapitre).to_owned()),
            [] if sous_discipline => (sous.to_owned(), sous.to_owned()),
            [] => return Err("contenu sans chapitre".into()),
        };
        Lu {
            cle: relatif.trim_end_matches(".md").to_owned(),
            nature: NatureContenu::Lecon,
            niveau: niveau.to_owned(),
            series: if niveau == "12" {
                series(premier)
            } else {
                Vec::new()
            },
            discipline: code.to_owned(),
            partie,
            chapitre,
            a_confirmer,
            titre,
            examen: None,
            session: None,
            texte: String::new(),
            faits: None,
            images: Vec::new(),
            tri: relatif.to_owned(),
        }
    };

    // Liste de faits et figures qui manquent, à côté de la leçon.
    let mut notes_figures = Vec::new();
    let faits = dossier.join(format!("{tige}.faits.md"));
    if faits.is_file() {
        let texte = std::fs::read_to_string(&faits).map_err(|e| e.to_string())?;
        let (liste, figures) = lire_faits(&texte);
        if !liste.trim().is_empty() {
            lu.faits = Some(liste);
        }
        notes_figures = figures;
    }
    // Le point du programme reste sous le titre ; le reste de l'en-tête part au manifeste.
    let point = point_du_programme(&entete);
    let mut texte = nettoyer(&corps);
    if let Some(point) = point {
        texte = format!("{point}\n\n{texte}");
    }
    if !notes_figures.is_empty() {
        texte.push_str(&format!("\n\n{NOTE}Figures à ajouter :\n>\n"));
        for figure in &notes_figures {
            texte.push_str(&format!("> - {figure}\n"));
        }
    }
    let analyse = analyser(&texte);
    for image in analyse.images() {
        if let SourceImage::Paquet { nom } = image {
            lu.images.push((nom.clone(), dossier.join(nom)));
        }
    }
    lu.texte = texte;
    Ok(lu)
}

/// Paragraphe « *Point du programme : …* » de l'en-tête, s'il y en a un.
fn point_du_programme(entete: &[String]) -> Option<String> {
    let debut = entete
        .iter()
        .position(|l| l.trim_start().starts_with("*Point du programme"))?;
    let lignes: Vec<&str> = entete
        .get(debut..)?
        .iter()
        .map(|l| l.trim())
        .take_while(|l| !l.is_empty())
        .collect();
    Some(lignes.join("\n"))
}

/// Lignes du corps sans les mentions d'état, de source et de relecture ; notes de travail
/// changées en notes pour le maître.
fn nettoyer(corps: &[String]) -> String {
    // Paragraphes séparés par une ligne vide.
    let mut paragraphes: Vec<Vec<String>> = vec![Vec::new()];
    for ligne in corps {
        if ligne.trim().is_empty() {
            paragraphes.push(Vec::new());
        } else if let Some(dernier) = paragraphes.last_mut() {
            dernier.push(ligne.clone());
        }
    }
    let mut sortie: Vec<String> = Vec::new();
    let mut dans_figures = false;
    for paragraphe in paragraphes.into_iter().filter(|p| !p.is_empty()) {
        let texte = paragraphe.join(" ");
        let premiere = paragraphe.first().map(|l| l.trim()).unwrap_or_default();
        // Section « Figures qui manquent » : ses lignes deviennent une note.
        if premiere.starts_with('#') {
            dans_figures = premiere.to_lowercase().contains("figures qui manquent");
            if dans_figures {
                sortie.push(format!("{NOTE}Figures à ajouter :\n>"));
                continue;
            }
        } else if dans_figures {
            // Dans la même citation que « Note pour le maître », sinon les élèves la verraient.
            let lignes: Vec<String> = paragraphe.iter().map(|l| format!("> {l}")).collect();
            if let Some(note) = sortie.last_mut() {
                note.push('\n');
                note.push_str(&lignes.join("\n"));
                note.push_str("\n>");
            }
            continue;
        }
        // Paragraphe d'état ou de source, en italique : retiré.
        let italique = premiere.starts_with('*') && !premiere.starts_with("**");
        let mention = [
            "État :",
            "Etat :",
            "Source :",
            "Sources :",
            ".verif.py",
            "relu",
            "rédigé pour GESCO",
            "rédigée pour GESCO",
        ]
        .iter()
        .any(|m| texte.contains(m));
        if italique && mention {
            continue;
        }
        let mut lignes = Vec::new();
        for ligne in paragraphe {
            if ligne.contains(".verif.py") {
                continue;
            }
            let ligne = sans_renvoi_image(&ligne);
            let sans_citation = ligne.trim_start().trim_start_matches('>').trim_start();
            if let Some(reste) = sans_citation
                .strip_prefix("Image à chercher")
                .or_else(|| sans_citation.strip_prefix("*Image à chercher"))
            {
                let reste = reste.trim_start_matches([' ', ':']).trim_end_matches('*');
                lignes.push(format!("{NOTE}Image à ajouter ici : {reste}"));
            } else {
                lignes.push(ligne);
            }
        }
        if !lignes.is_empty() {
            sortie.push(lignes.join("\n"));
        }
    }
    sortie.join("\n\n") + "\n"
}

/// Ligne sans ses renvois entre parenthèses à une image à chercher (« Schéma — Une dune (une
/// image à chercher, voir plus haut). »).
fn sans_renvoi_image(ligne: &str) -> String {
    let mut sortie = ligne.to_owned();
    while let Some(i) = sortie.to_lowercase().find("image à chercher") {
        let avant = sortie.get(..i).unwrap_or_default();
        let (Some(debut), Some(fin)) = (
            avant.rfind('('),
            sortie.get(i..).and_then(|s| s.find(')')).map(|f| i + f),
        ) else {
            break;
        };
        let debut = sortie.get(..debut).map_or(debut, |a| a.trim_end().len());
        sortie.replace_range(debut..=fin, "");
    }
    sortie
}

/// Liste de faits (les seules lignes numérotées, sans le préambule) et figures qui manquent.
fn lire_faits(texte: &str) -> (String, Vec<String>) {
    let mut liste = Vec::new();
    let mut figures: Vec<String> = Vec::new();
    let mut section_figures = false;
    let mut dans_liste = false;
    for ligne in texte.lines() {
        let nette = ligne.trim();
        if nette.starts_with('#') {
            section_figures = nette.to_lowercase().contains("figures qui manquent");
            dans_liste = false;
            continue;
        }
        if section_figures {
            if let Some(f) = nette.strip_prefix("- ") {
                figures.push(f.to_owned());
            } else if !nette.is_empty()
                && let Some(derniere) = figures.last_mut()
            {
                derniere.push(' ');
                derniere.push_str(nette);
            }
            continue;
        }
        let numerotee = nette
            .split_once(". ")
            .is_some_and(|(n, _)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
        if numerotee {
            dans_liste = true;
            liste.push(ligne.to_owned());
        } else if dans_liste && !nette.is_empty() && ligne.starts_with(' ') {
            liste.push(ligne.to_owned());
        } else if nette.is_empty() {
            dans_liste = false;
        }
    }
    (liste.join("\n"), figures)
}

/// Refus d'un contenu : images absentes, formules non convertibles, marques interdites.
fn controler(lu: &Lu) -> Vec<String> {
    let mut refus = Vec::new();
    for (nom, chemin) in &lu.images {
        if !chemin.is_file() {
            refus.push(format!("{} : image absente {nom}", lu.cle));
        }
    }
    let analyse = analyser(&lu.texte);
    for formule in analyse.formules() {
        if let Err(e) = latex_vers_typst(&formule.latex) {
            refus.push(format!("{} : formule {} ({e})", lu.cle, formule.latex));
        }
    }
    let mut affiche = format!("{}\n{}\n{}\n", lu.titre, lu.chapitre, lu.partie);
    affiche.push_str(&analyse.texte_brut());
    if let Some(faits) = &lu.faits {
        affiche.push_str(&analyser(faits).texte_brut());
    }
    let marques = marques_interdites(&affiche);
    if !marques.is_empty() {
        refus.push(format!(
            "{} : texte affiché avec {}",
            lu.cle,
            marques.join(", ")
        ));
    }
    refus
}

type Rangement = (ManifestePaquet, Vec<(String, Vec<u8>)>);

/// Range les contenus en chapitres et construit le manifeste et la liste des fichiers.
fn ranger(
    lus: &[Lu],
    version: &str,
    date: &str,
    refus: &mut Vec<String>,
) -> Result<Rangement, String> {
    // Chapitres : clé → (chapitre, rang de première apparition).
    let mut chapitres: BTreeMap<String, ChapitrePaquet> = BTreeMap::new();
    let mut apparition: Vec<String> = Vec::new();
    let cle_chapitre = |lu: &Lu| {
        let serie = if lu.nature == NatureContenu::Lecon {
            String::new()
        } else {
            lu.series.join("-").to_lowercase()
        };
        [
            lu.niveau.clone(),
            lu.discipline.to_lowercase(),
            slug(&lu.partie),
            slug(&lu.chapitre),
            serie,
        ]
        .iter()
        .filter(|m| !m.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join("-")
    };
    for lu in lus {
        let cle = cle_chapitre(lu);
        let chapitre = chapitres.entry(cle.clone()).or_insert_with(|| {
            apparition.push(cle.clone());
            ChapitrePaquet {
                cle: cle.clone(),
                niveau: lu.niveau.clone(),
                series: Vec::new(),
                discipline: lu.discipline.clone(),
                partie: lu.partie.clone(),
                titre: lu.chapitre.clone(),
                ordre: 0,
                a_confirmer: false,
            }
        });
        chapitre.a_confirmer |= lu.a_confirmer;
        for s in &lu.series {
            if !chapitre.series.contains(s) {
                chapitre.series.push(s.clone());
            }
        }
        chapitre.series.sort();
    }
    // Rang des chapitres dans chaque matière d'un niveau : leçons dans l'ordre des dossiers,
    // examens à la fin.
    let mut rangs: HashMap<(String, String), u32> = HashMap::new();
    let examens_a_la_fin = |cle: &String| chapitres.get(cle).is_some_and(|c| c.partie == "Examens");
    let mut ordre: Vec<String> = apparition.clone();
    ordre.sort_by_key(|c| examens_a_la_fin(c));
    for cle in &ordre {
        if let Some(c) = chapitres.get_mut(cle) {
            let rang = rangs
                .entry((c.niveau.clone(), c.discipline.clone()))
                .or_insert(0);
            *rang += 1;
            c.ordre = *rang;
        }
    }

    // Contenus, rangés dans leur chapitre.
    let mut par_chapitre: BTreeMap<String, Vec<&Lu>> = BTreeMap::new();
    for lu in lus {
        par_chapitre.entry(cle_chapitre(lu)).or_default().push(lu);
    }
    let cles: std::collections::BTreeSet<&str> = lus.iter().map(|l| l.cle.as_str()).collect();
    let mut contenus = Vec::new();
    let mut fichiers: Vec<(String, Vec<u8>)> = Vec::new();
    let mut images: BTreeMap<String, String> = BTreeMap::new();
    for (chapitre, mut membres) in par_chapitre {
        membres.sort_by(|a, b| a.tri.cmp(&b.tri));
        for (rang, lu) in membres.into_iter().enumerate() {
            let lie = match lu.nature {
                NatureContenu::Sujet => Some(format!("{}-corrige", lu.cle)),
                NatureContenu::Corrige => Some(lu.cle.trim_end_matches("-corrige").to_owned()),
                _ => None,
            }
            .filter(|l| cles.contains(l.as_str()));
            let fichier = format!("contenus/{}.md", lu.cle);
            fichiers.push((fichier.clone(), lu.texte.clone().into_bytes()));
            let faits = lu.faits.as_ref().map(|f| {
                let nom = format!("faits/{}.md", lu.cle);
                fichiers.push((nom.clone(), f.clone().into_bytes()));
                nom
            });
            let mut noms = Vec::new();
            for (nom, chemin) in &lu.images {
                let Ok(octets) = std::fs::read(chemin) else {
                    continue;
                };
                let empreinte = paquet::empreinte(&octets);
                let nom_paquet = paquet::nom_fichier_image(&lu.cle, nom);
                match images.get(&nom_paquet) {
                    Some(deja) if *deja != empreinte => refus.push(format!(
                        "{} : deux images différentes portent le nom {nom_paquet}",
                        lu.cle
                    )),
                    Some(_) => {}
                    None => {
                        images.insert(nom_paquet.clone(), empreinte);
                        fichiers.push((nom_paquet, octets));
                    }
                }
                if !noms.contains(nom) {
                    noms.push(nom.clone());
                }
            }
            contenus.push(ContenuPaquet {
                cle: lu.cle.clone(),
                nature: lu.nature,
                chapitre: chapitre.clone(),
                titre: lu.titre.clone(),
                ordre: u32::try_from(rang + 1).map_err(|e| e.to_string())?,
                lie,
                examen: lu.examen,
                session: lu.session,
                fichier,
                faits,
                images: noms,
            });
        }
    }
    let mut disciplines: Vec<DisciplinePaquet> = Vec::new();
    for (_, code, libelle) in DISCIPLINES {
        if !disciplines.iter().any(|d| d.code == code) {
            disciplines.push(DisciplinePaquet {
                code: code.to_owned(),
                libelle: libelle.to_owned(),
            });
        }
    }
    let mut chapitres: Vec<ChapitrePaquet> = chapitres.into_values().collect();
    chapitres.sort_by(|a, b| {
        (&a.niveau, &a.discipline, a.ordre).cmp(&(&b.niveau, &b.discipline, b.ordre))
    });
    Ok((
        ManifestePaquet {
            code: CODE_PAQUET.to_owned(),
            version: version.to_owned(),
            date: date.to_owned(),
            gesco_minimum: GESCO_MINIMUM.to_owned(),
            programme: PROGRAMME.to_owned(),
            disciplines,
            chapitres,
            contenus,
            fichiers: Vec::new(),
        },
        fichiers,
    ))
}

/// Tableau de couverture (E-14), pour l'éditeur : par niveau, séries et matière ; avec les
/// questions de quiz reconnues et écartées (lot 3). Rend aussi la liste des leçons dont une
/// question n'est pas reconnue, pour l'intégration des cours.
fn tableau_de_couverture(manifeste: &ManifestePaquet, lus: &[Lu]) -> (String, String) {
    let mut lignes: BTreeMap<(String, String, String), [usize; 6]> = BTreeMap::new();
    let textes: HashMap<&str, &str> = lus
        .iter()
        .map(|l| (l.cle.as_str(), l.texte.as_str()))
        .collect();
    let mut a_reprendre = String::new();
    let chapitres: HashMap<&str, &ChapitrePaquet> = manifeste
        .chapitres
        .iter()
        .map(|c| (c.cle.as_str(), c))
        .collect();
    for c in &manifeste.chapitres {
        let ligne = lignes
            .entry((c.niveau.clone(), c.series.join(" "), c.discipline.clone()))
            .or_default();
        if let Some(n) = ligne.get_mut(0) {
            *n += 1;
        }
    }
    for contenu in &manifeste.contenus {
        let Some(c) = chapitres.get(contenu.chapitre.as_str()) else {
            continue;
        };
        let ligne = lignes
            .entry((c.niveau.clone(), c.series.join(" "), c.discipline.clone()))
            .or_default();
        let colonne = match contenu.nature {
            NatureContenu::Lecon => 1,
            NatureContenu::Sujet => 2,
            NatureContenu::Corrige | NatureContenu::Document => 3,
        };
        if let Some(n) = ligne.get_mut(colonne) {
            *n += 1;
        }
        if contenu.nature == NatureContenu::Lecon
            && let Some(texte) = textes.get(contenu.cle.as_str())
        {
            let quiz = gesco_core::bibliotheque::analyser_quiz(texte, false);
            if let Some(n) = ligne.get_mut(4) {
                *n += quiz.questions.len();
            }
            if let Some(n) = ligne.get_mut(5) {
                *n += quiz.ecartees.len();
            }
            for e in &quiz.ecartees {
                a_reprendre.push_str(&format!(
                    "{} : question {} : {}\n",
                    contenu.cle, e.numero, e.raison
                ));
            }
        }
    }
    let mut csv = String::from(
        "niveau;series;discipline;chapitres;lecons;sujets;corriges;questions_reconnues;questions_ecartees\n",
    );
    for ((niveau, series, discipline), [ch, le, su, co, qr, qe]) in lignes {
        csv.push_str(&format!(
            "{niveau};{series};{discipline};{ch};{le};{su};{co};{qr};{qe}\n"
        ));
    }
    (csv, a_reprendre)
}
