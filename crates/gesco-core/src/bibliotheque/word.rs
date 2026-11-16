//! Import d'une leçon depuis Word (ADR « importer une leçon Word », SPEC_PHASE_12_LOT_2 §4) :
//! un `.docx` lu en mémoire devient le texte d'une leçon, ses images à ranger et les notes de
//! l'auteur. Aucune E/S (R-03) : le service lit le fichier et range les images.
//!
//! Passent : titres (styles « Titre 1 » à « Titre 3 », « Heading 1 »… et « Title »), paragraphes,
//! gras, italique, listes à puces et numérotées, tableaux simples, images PNG et JPEG de 5 Mo au
//! plus. Une formule de Word devient une note pour le maître « Formule à retaper ici », jamais vue
//! des élèves ni imprimée. Zones de texte, en-têtes et pieds de page, commentaires, révisions
//! supprimées, champs et macros sont ignorés. Rien du fichier n'est lu comme du HTML : la
//! ponctuation de Markdown est échappée, le filtre de la lecture fait le reste.

use std::collections::HashMap;
use std::io::{Cursor, Read};

use quick_xml::events::Event;
use serde::{Deserialize, Serialize};

use super::fichiers::{SorteFichier, TAILLE_IMAGE_MAX, reconnaitre};
use super::paquet::empreinte;

/// Taille d'un `.docx` au-delà de laquelle il est refusé.
pub const TAILLE_WORD_MAX: u64 = 20 * 1024 * 1024;

/// Taille décompressée d'une partie XML au-delà de laquelle le fichier est jugé abîmé (une
/// archive fabriquée pour gonfler à la lecture).
const TAILLE_XML_MAX: u64 = 32 * 1024 * 1024;

/// Profondeur d'imbrication au-delà de laquelle le fichier est jugé abîmé.
const PROFONDEUR_MAX: usize = 256;

/// Début d'un ancien fichier Word (`.doc`, conteneur OLE).
const ENTETE_OLE: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];

/// Note pour le maître qui remplace une formule de Word.
const NOTE_FORMULE: &str = "> **Note pour le maître** : Formule à retaper ici.";

/// Note pour le maître qui remplace une formule d'un tableau.
const NOTE_FORMULE_TABLEAU: &str =
    "> **Note pour le maître** : Formule à retaper ici (dans le tableau ci-dessus).";

/// Pourquoi un fichier Word est refusé.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ErreurWord {
    /// Ancien format `.doc`.
    #[error("Enregistrez ce fichier au format .docx.")]
    AncienFormat,
    /// Ni un `.docx` ni un `.doc`, ou un `.docx` abîmé.
    #[error("Ce fichier n'est pas un document Word lisible.")]
    Illisible,
    /// Plus de 20 Mo.
    #[error("Ce fichier Word dépasse 20 Mo.")]
    TropLourd,
}

/// Ce que l'auteur lit sous l'aperçu après un import : ce qui n'est pas passé.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NoteWord {
    /// Formules remplacées par « Formule à retaper ici ».
    Formules {
        /// Nombre de formules.
        nombre: u32,
    },
    /// Images de plus de 5 Mo laissées de côté.
    ImagesTropLourdes {
        /// Nombre d'images.
        nombre: u32,
    },
    /// Images dans un format que la bibliothèque ne prend pas (EMF, WMF, GIF…) ou liées à un
    /// fichier extérieur.
    ImagesNonReprises {
        /// Nombre d'images.
        nombre: u32,
    },
    /// Zones de texte ignorées.
    ZonesDeTexte,
    /// En-têtes, pieds de page, commentaires ou révisions supprimées ignorés.
    ElementsIgnores,
}

/// Une image du fichier, à ranger comme celles du bouton Image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageWord {
    /// PNG ou JPEG.
    pub sorte: SorteFichier,
    /// Empreinte SHA-256, en hexadécimal minuscule ; le texte l'appelle par `image:<empreinte>`.
    pub empreinte: String,
    /// Nom du fichier dans le document (`image1.png`).
    pub nom: String,
    /// Octets.
    pub octets: Vec<u8>,
}

/// Une leçon tirée d'un fichier Word.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LeconWord {
    /// Texte de la leçon.
    pub texte: String,
    /// Images appelées par le texte, une fois chacune.
    pub images: Vec<ImageWord>,
    /// Ce qui n'est pas passé, pour l'auteur.
    pub notes: Vec<NoteWord>,
}

/// Convertit un `.docx` en texte de leçon.
pub fn convertir_docx(octets: &[u8]) -> Result<LeconWord, ErreurWord> {
    if octets.starts_with(&ENTETE_OLE) {
        return Err(ErreurWord::AncienFormat);
    }
    if octets.len() as u64 > TAILLE_WORD_MAX {
        return Err(ErreurWord::TropLourd);
    }
    if !octets.starts_with(b"PK") {
        return Err(ErreurWord::Illisible);
    }
    let mut archive =
        zip::ZipArchive::new(Cursor::new(octets)).map_err(|_| ErreurWord::Illisible)?;

    let principal = lire_xml(&mut archive, "_rels/.rels")
        .ok()
        .and_then(|racine| {
            liens(&racine, "")
                .into_iter()
                .find(|l| l.sorte.ends_with("/officeDocument") && !l.externe)
                .map(|l| l.cible)
        })
        .unwrap_or_else(|| "word/document.xml".to_owned());
    let document = lire_xml(&mut archive, &principal)?;
    let dossier = principal
        .rsplit_once('/')
        .map_or(String::new(), |(d, _)| format!("{d}/"));
    let nom_seul = principal.rsplit('/').next().unwrap_or_default();
    let liens_document = lire_xml(&mut archive, &format!("{dossier}_rels/{nom_seul}.rels"))
        .map(|r| liens(&r, &dossier))
        .unwrap_or_default();

    let partie = |fin: &str| {
        liens_document
            .iter()
            .find(|l| l.sorte.ends_with(fin) && !l.externe)
            .map(|l| l.cible.clone())
    };
    let styles = partie("/styles")
        .and_then(|c| lire_xml(&mut archive, &c).ok())
        .map(|r| Styles::lire(&r))
        .unwrap_or_default();
    let numerotation = partie("/numbering")
        .and_then(|c| lire_xml(&mut archive, &c).ok())
        .map(|r| Numerotation::lire(&r))
        .unwrap_or_default();
    // Les notes de bas de page ont toujours leur partie, même vide : elles ne comptent pas.
    let elements_ignores = liens_document.iter().any(|l| {
        ["/header", "/footer", "/comments"]
            .iter()
            .any(|f| l.sorte.ends_with(f))
    });

    let mut convertisseur = Convertisseur {
        archive: &mut archive,
        styles,
        numerotation,
        liens: liens_document
            .iter()
            .map(|l| (l.id.clone(), (l.cible.clone(), l.externe)))
            .collect(),
        blocs: Vec::new(),
        images: Vec::new(),
        vues: HashMap::new(),
        formules: 0,
        trop_lourdes: 0,
        non_reprises: 0,
        zones_de_texte: false,
        elements_ignores,
    };
    if let Some(corps) = document.enfant("body") {
        convertisseur.blocs_de(corps);
    }
    let texte = convertisseur.texte();
    let mut notes = Vec::new();
    if convertisseur.formules > 0 {
        notes.push(NoteWord::Formules {
            nombre: convertisseur.formules,
        });
    }
    if convertisseur.trop_lourdes > 0 {
        notes.push(NoteWord::ImagesTropLourdes {
            nombre: convertisseur.trop_lourdes,
        });
    }
    if convertisseur.non_reprises > 0 {
        notes.push(NoteWord::ImagesNonReprises {
            nombre: convertisseur.non_reprises,
        });
    }
    if convertisseur.zones_de_texte {
        notes.push(NoteWord::ZonesDeTexte);
    }
    if convertisseur.elements_ignores {
        notes.push(NoteWord::ElementsIgnores);
    }
    Ok(LeconWord {
        texte,
        images: convertisseur.images,
        notes,
    })
}

// ---------------------------------------------------------------------------------------------
// Lecture du XML en un petit arbre.

/// Un élément XML, désigné par son nom local (`p` pour `w:p`).
#[derive(Debug, Default)]
struct Noeud {
    nom: String,
    attributs: Vec<(String, String)>,
    enfants: Vec<Enfant>,
}

#[derive(Debug)]
enum Enfant {
    Noeud(Noeud),
    Texte(String),
}

impl Noeud {
    /// Valeur d'un attribut, par son nom local.
    fn attribut(&self, nom: &str) -> Option<&str> {
        self.attributs
            .iter()
            .find(|(n, _)| n == nom)
            .map(|(_, v)| v.as_str())
    }

    fn noeuds(&self) -> impl Iterator<Item = &Noeud> {
        self.enfants.iter().filter_map(|e| match e {
            Enfant::Noeud(n) => Some(n),
            Enfant::Texte(_) => None,
        })
    }

    fn enfant(&self, nom: &str) -> Option<&Noeud> {
        self.noeuds().find(|n| n.nom == nom)
    }

    /// `w:val` d'un enfant (`<w:pStyle w:val="Titre1"/>`).
    fn valeur(&self, nom: &str) -> Option<&str> {
        self.enfant(nom).and_then(|n| n.attribut("val"))
    }

    /// Texte des enfants texte.
    fn texte(&self) -> String {
        self.enfants
            .iter()
            .filter_map(|e| match e {
                Enfant::Texte(t) => Some(t.as_str()),
                Enfant::Noeud(_) => None,
            })
            .collect()
    }
}

/// Nom sans préfixe.
fn local(nom: &str) -> &str {
    nom.rsplit_once(':').map_or(nom, |(_, l)| l)
}

/// Lit une partie XML de l'archive, bornée.
fn lire_xml<R: Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
    nom: &str,
) -> Result<Noeud, ErreurWord> {
    let mut fichier = archive.by_name(nom).map_err(|_| ErreurWord::Illisible)?;
    if fichier.size() > TAILLE_XML_MAX {
        return Err(ErreurWord::Illisible);
    }
    let mut octets = Vec::new();
    (&mut fichier)
        .take(TAILLE_XML_MAX + 1)
        .read_to_end(&mut octets)
        .map_err(|_| ErreurWord::Illisible)?;
    if octets.len() as u64 > TAILLE_XML_MAX {
        return Err(ErreurWord::Illisible);
    }
    let texte = String::from_utf8(octets).map_err(|_| ErreurWord::Illisible)?;
    arbre(texte.trim_start_matches('\u{feff}'))
}

fn ouvrir(debut: &quick_xml::events::BytesStart<'_>) -> Noeud {
    let attributs = debut
        .attributes()
        .with_checks(false)
        .filter_map(Result::ok)
        .map(|a| {
            let valeur = a
                .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                .map(|v| v.into_owned())
                .unwrap_or_default();
            (local(a.key.as_ref()).to_owned(), valeur)
        })
        .collect();
    Noeud {
        nom: local(debut.name().as_ref()).to_owned(),
        attributs,
        enfants: Vec::new(),
    }
}

/// Ajoute du texte au nœud ouvert, s'il est un nœud de texte (`w:t`, `w:delText`,
/// `w:instrText`, `m:t`) : les blancs entre les éléments ne comptent pas.
fn ajouter_texte(pile: &mut [Noeud], texte: &str) {
    if let Some(haut) = pile.last_mut()
        && matches!(haut.nom.as_str(), "t" | "delText" | "instrText")
    {
        match haut.enfants.last_mut() {
            Some(Enfant::Texte(t)) => t.push_str(texte),
            _ => haut.enfants.push(Enfant::Texte(texte.to_owned())),
        }
    }
}

/// Lit un texte XML en arbre.
fn arbre(texte: &str) -> Result<Noeud, ErreurWord> {
    let mut lecteur = quick_xml::Reader::from_str(texte);
    let mut pile: Vec<Noeud> = vec![Noeud::default()];
    loop {
        match lecteur.read_event().map_err(|_| ErreurWord::Illisible)? {
            Event::Start(debut) => {
                if pile.len() > PROFONDEUR_MAX {
                    return Err(ErreurWord::Illisible);
                }
                pile.push(ouvrir(&debut));
            }
            Event::Empty(debut) => {
                let noeud = ouvrir(&debut);
                if let Some(haut) = pile.last_mut() {
                    haut.enfants.push(Enfant::Noeud(noeud));
                }
            }
            Event::End(_) => {
                if pile.len() < 2 {
                    return Err(ErreurWord::Illisible);
                }
                if let Some(noeud) = pile.pop()
                    && let Some(haut) = pile.last_mut()
                {
                    haut.enfants.push(Enfant::Noeud(noeud));
                }
            }
            Event::Text(t) => ajouter_texte(&mut pile, &t.xml10_content()),
            Event::CData(t) => ajouter_texte(&mut pile, &t.xml10_content()),
            Event::GeneralRef(r) => {
                let caractere = match r.resolve_char_ref() {
                    Ok(Some(c)) => Some(c.to_string()),
                    _ => quick_xml::escape::resolve_predefined_entity(&r.xml10_content())
                        .map(str::to_owned),
                };
                if let Some(c) = caractere {
                    ajouter_texte(&mut pile, &c);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if pile.len() != 1 {
        return Err(ErreurWord::Illisible);
    }
    let racine = pile.pop().unwrap_or_default();
    racine
        .enfants
        .into_iter()
        .find_map(|e| match e {
            Enfant::Noeud(n) => Some(n),
            Enfant::Texte(_) => None,
        })
        .ok_or(ErreurWord::Illisible)
}

// ---------------------------------------------------------------------------------------------
// Liens, styles et numérotation.

/// Un lien d'une partie vers une autre (`Relationship`).
struct Lien {
    id: String,
    sorte: String,
    /// Nom de la partie dans l'archive, ou adresse extérieure.
    cible: String,
    externe: bool,
}

/// Les liens d'un fichier `.rels` ; `dossier` est celui de la partie qui les porte.
fn liens(racine: &Noeud, dossier: &str) -> Vec<Lien> {
    racine
        .noeuds()
        .filter(|n| n.nom == "Relationship")
        .filter_map(|n| {
            let externe = n
                .attribut("TargetMode")
                .is_some_and(|m| m.eq_ignore_ascii_case("external"));
            let cible = n.attribut("Target")?;
            Some(Lien {
                id: n.attribut("Id")?.to_owned(),
                sorte: n.attribut("Type").unwrap_or_default().to_owned(),
                cible: if externe {
                    cible.to_owned()
                } else {
                    chemin_dans_archive(dossier, cible)
                },
                externe,
            })
        })
        .collect()
}

/// Nom dans l'archive d'une cible relative à `dossier` (`media/image1.png`, `../media/x.png`,
/// `/word/media/x.png`), adresse décodée (`%20`).
fn chemin_dans_archive(dossier: &str, cible: &str) -> String {
    let cible = decoder_adresse(cible);
    let complet = match cible.strip_prefix('/') {
        Some(absolu) => absolu.to_owned(),
        None => format!("{dossier}{cible}"),
    };
    let mut morceaux: Vec<&str> = Vec::new();
    for m in complet.split('/') {
        match m {
            "" | "." => {}
            ".." => {
                morceaux.pop();
            }
            m => morceaux.push(m),
        }
    }
    morceaux.join("/")
}

/// Décode les `%xx` d'une adresse.
fn decoder_adresse(texte: &str) -> String {
    let octets = texte.as_bytes();
    let mut sortie = Vec::with_capacity(octets.len());
    let mut i = 0;
    while let Some(&o) = octets.get(i) {
        let chiffres = octets
            .get(i + 1..i + 3)
            .and_then(|h| std::str::from_utf8(h).ok())
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match (o, chiffres) {
            (b'%', Some(v)) => {
                sortie.push(v);
                i += 3;
            }
            _ => {
                sortie.push(o);
                i += 1;
            }
        }
    }
    String::from_utf8(sortie).unwrap_or_else(|_| texte.to_owned())
}

/// Un style de Word.
#[derive(Debug, Default, Clone)]
struct Style {
    nom: String,
    parent: Option<String>,
    niveau_plan: Option<u8>,
    numerotation: Option<(String, u32)>,
}

/// Styles de `styles.xml`, par identifiant.
#[derive(Debug, Default)]
struct Styles(HashMap<String, Style>);

impl Styles {
    fn lire(racine: &Noeud) -> Self {
        let mut styles = HashMap::new();
        for s in racine.noeuds().filter(|n| n.nom == "style") {
            let Some(id) = s.attribut("styleId") else {
                continue;
            };
            let ppr = s.enfant("pPr");
            styles.insert(
                id.to_owned(),
                Style {
                    nom: s.valeur("name").unwrap_or_default().to_owned(),
                    parent: s.valeur("basedOn").map(str::to_owned),
                    niveau_plan: ppr
                        .and_then(|p| p.valeur("outlineLvl"))
                        .and_then(|v| v.parse().ok()),
                    numerotation: ppr.and_then(numerotation_de),
                },
            );
        }
        Self(styles)
    }

    /// Le style et ses parents, du plus proche au plus lointain.
    fn chaine(&self, id: &str) -> Vec<(&str, &Style)> {
        let mut chaine = Vec::new();
        let mut courant = Some(id);
        while let Some(id) = courant {
            if chaine.len() > 16 || chaine.iter().any(|(i, _)| *i == id) {
                break;
            }
            match self.0.get_key_value(id) {
                Some((cle, style)) => {
                    chaine.push((cle.as_str(), style));
                    courant = style.parent.as_deref();
                }
                None => break,
            }
        }
        chaine
    }

    /// Niveau de titre (1 à 3) d'un style de paragraphe.
    fn niveau_titre(&self, id: &str) -> Option<u8> {
        let chaine = self.chaine(id);
        let noms = std::iter::once(id).chain(chaine.iter().flat_map(|(i, s)| [*i, &s.nom]));
        for nom in noms {
            if let Some(n) = niveau_d_apres_le_nom(nom) {
                return Some(n);
            }
        }
        chaine
            .iter()
            .find_map(|(_, s)| s.niveau_plan)
            .filter(|n| *n < 9)
            .map(|n| (n + 1).min(3))
    }

    fn numerotation(&self, id: &str) -> Option<(String, u32)> {
        self.chaine(id)
            .iter()
            .find_map(|(_, s)| s.numerotation.clone())
    }

    /// Nom d'un style (pour « Strong », « Emphasis »).
    fn nom(&self, id: &str) -> String {
        self.0
            .get(id)
            .map_or_else(|| id.to_owned(), |s| s.nom.clone())
            .to_lowercase()
    }
}

/// « heading 2 », « Titre 2 », « Heading2 », « Title » → niveau de titre.
fn niveau_d_apres_le_nom(nom: &str) -> Option<u8> {
    let bas = nom.trim().to_lowercase();
    if bas == "title" || bas == "titre" {
        return Some(1);
    }
    for debut in ["heading", "titre"] {
        if let Some(reste) = bas.strip_prefix(debut) {
            let reste = reste.trim();
            if let Ok(n) = reste.parse::<u8>()
                && (1..=9).contains(&n)
            {
                return Some(n.min(3));
            }
        }
    }
    None
}

/// `numPr` d'un `pPr` : (numéro de liste, niveau).
fn numerotation_de(ppr: &Noeud) -> Option<(String, u32)> {
    let num = ppr.enfant("numPr")?;
    let id = num.valeur("numId")?;
    let niveau = num.valeur("ilvl").and_then(|v| v.parse().ok()).unwrap_or(0);
    Some((id.to_owned(), niveau))
}

/// Formats des listes de `numbering.xml` : (numéro de liste, niveau) → format.
#[derive(Debug, Default)]
struct Numerotation {
    abstraits: HashMap<String, HashMap<u32, String>>,
    listes: HashMap<String, String>,
}

impl Numerotation {
    fn lire(racine: &Noeud) -> Self {
        let mut abstraits = HashMap::new();
        let mut listes = HashMap::new();
        for n in racine.noeuds() {
            match n.nom.as_str() {
                "abstractNum" => {
                    let Some(id) = n.attribut("abstractNumId") else {
                        continue;
                    };
                    let niveaux = n
                        .noeuds()
                        .filter(|l| l.nom == "lvl")
                        .filter_map(|l| {
                            Some((
                                l.attribut("ilvl")?.parse().ok()?,
                                l.valeur("numFmt").unwrap_or("decimal").to_owned(),
                            ))
                        })
                        .collect();
                    abstraits.insert(id.to_owned(), niveaux);
                }
                "num" => {
                    if let (Some(id), Some(abstrait)) =
                        (n.attribut("numId"), n.valeur("abstractNumId"))
                    {
                        listes.insert(id.to_owned(), abstrait.to_owned());
                    }
                }
                _ => {}
            }
        }
        Self { abstraits, listes }
    }

    /// `Some(vrai)` pour une liste numérotée, `Some(faux)` pour une liste à puces, `None` si le
    /// paragraphe n'est pas dans une liste.
    fn numerotee(&self, id: &str, niveau: u32) -> Option<bool> {
        if id == "0" {
            return None;
        }
        let format = self
            .listes
            .get(id)
            .and_then(|a| self.abstraits.get(a))
            .and_then(|niveaux| niveaux.get(&niveau));
        match format.map(String::as_str) {
            None | Some("bullet") => Some(false),
            Some("none") => None,
            Some(_) => Some(true),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Conversion.

/// Un morceau du texte d'un paragraphe.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Morceau {
    Texte {
        texte: String,
        gras: bool,
        italique: bool,
    },
    Saut,
    /// Appel d'une image, déjà écrit.
    Image(String),
}

/// Un bloc du texte produit.
struct BlocSortie {
    texte: String,
    /// Élément de liste : se colle au précédent élément de liste.
    liste: bool,
}

struct Convertisseur<'a, R: Read + std::io::Seek> {
    archive: &'a mut zip::ZipArchive<R>,
    styles: Styles,
    numerotation: Numerotation,
    /// Lien → (partie, extérieure).
    liens: HashMap<String, (String, bool)>,
    blocs: Vec<BlocSortie>,
    images: Vec<ImageWord>,
    /// Partie d'une image → son empreinte si elle a été reprise.
    vues: HashMap<String, Option<String>>,
    formules: u32,
    trop_lourdes: u32,
    non_reprises: u32,
    zones_de_texte: bool,
    elements_ignores: bool,
}

/// Vrai si une propriété de mise en forme (`<w:b/>`, `<w:b w:val="0"/>`) est mise.
fn mise(rpr: Option<&Noeud>, nom: &str) -> bool {
    rpr.and_then(|r| r.enfant(nom))
        .is_some_and(|n| !matches!(n.attribut("val"), Some("0" | "false" | "off" | "none")))
}

impl<R: Read + std::io::Seek> Convertisseur<'_, R> {
    fn pousser(&mut self, texte: String, liste: bool) {
        if !texte.trim().is_empty() {
            self.blocs.push(BlocSortie { texte, liste });
        }
    }

    /// Texte final : blocs séparés d'une ligne vide, éléments de liste collés.
    fn texte(&self) -> String {
        let mut texte = String::new();
        let mut precedent_liste = false;
        for (i, b) in self.blocs.iter().enumerate() {
            if i > 0 {
                texte.push_str(if b.liste && precedent_liste {
                    "\n"
                } else {
                    "\n\n"
                });
            }
            texte.push_str(&b.texte);
            precedent_liste = b.liste;
        }
        texte.push('\n');
        texte
    }

    /// Blocs d'un conteneur (corps, cellule) : paragraphes et tableaux.
    fn blocs_de(&mut self, conteneur: &Noeud) {
        for n in conteneur.noeuds() {
            match n.nom.as_str() {
                "p" => self.paragraphe(n),
                "tbl" => self.tableau(n),
                "sdt" => {
                    if let Some(c) = n.enfant("sdtContent") {
                        self.blocs_de(c);
                    }
                }
                "customXml" | "ins" | "moveTo" => self.blocs_de(n),
                "AlternateContent" => {
                    if let Some(c) = n.enfant("Choice") {
                        self.blocs_de(c);
                    }
                }
                "del" | "moveFrom" => self.elements_ignores = true,
                "txbxContent" => self.zones_de_texte = true,
                _ => {}
            }
        }
    }

    /// Morceaux d'un paragraphe, avec le nombre de formules rencontrées.
    fn morceaux(&mut self, noeud: &Noeud, morceaux: &mut Vec<Morceau>, formules: &mut u32) {
        for n in noeud.noeuds() {
            match n.nom.as_str() {
                "r" => self.course(n, morceaux, formules),
                "hyperlink" | "ins" | "moveTo" | "smartTag" | "fldSimple" | "customXml" | "bdo"
                | "dir" => self.morceaux(n, morceaux, formules),
                "sdt" => {
                    if let Some(c) = n.enfant("sdtContent") {
                        self.morceaux(c, morceaux, formules);
                    }
                }
                "AlternateContent" => {
                    if let Some(c) = n.enfant("Choice") {
                        self.morceaux(c, morceaux, formules);
                    }
                }
                "oMath" | "oMathPara" => *formules += 1,
                "del" | "moveFrom" => self.elements_ignores = true,
                _ => {}
            }
        }
    }

    /// Morceaux d'une course de texte (`w:r`).
    fn course(&mut self, course: &Noeud, morceaux: &mut Vec<Morceau>, formules: &mut u32) {
        let rpr = course.enfant("rPr");
        let style = rpr
            .and_then(|r| r.valeur("rStyle"))
            .map(|s| self.styles.nom(s))
            .unwrap_or_default();
        let gras = mise(rpr, "b") || style == "strong";
        let italique = mise(rpr, "i") || style == "emphasis";
        let texte = |t: &str| Morceau::Texte {
            texte: t.to_owned(),
            gras,
            italique,
        };
        for n in course.noeuds() {
            match n.nom.as_str() {
                "t" => morceaux.push(texte(&n.texte())),
                "tab" | "ptab" => morceaux.push(texte(" ")),
                "noBreakHyphen" => morceaux.push(texte("-")),
                "br" => match n.attribut("type") {
                    Some("page") => {}
                    Some("column") => morceaux.push(texte(" ")),
                    _ => morceaux.push(Morceau::Saut),
                },
                "cr" => morceaux.push(Morceau::Saut),
                "drawing" | "pict" => self.images_de(n, morceaux),
                "object" => {
                    if objet_formule(n) {
                        *formules += 1;
                    } else {
                        self.images_de(n, morceaux);
                    }
                }
                "AlternateContent" => {
                    if let Some(c) = n.enfant("Choice") {
                        // Le choix porte les mêmes éléments qu'une course.
                        let mut copie = Vec::new();
                        self.course(c, &mut copie, formules);
                        morceaux.extend(copie);
                    }
                }
                "delText" => self.elements_ignores = true,
                _ => {}
            }
        }
    }

    /// Images d'un dessin (`a:blip r:embed`) ou d'une image ancienne (`v:imagedata r:id`), hors
    /// zones de texte.
    fn images_de(&mut self, noeud: &Noeud, morceaux: &mut Vec<Morceau>) {
        let mut trouvees = Vec::new();
        let mut zones = false;
        chercher_images(noeud, None, &mut trouvees, &mut zones, 0);
        if zones {
            self.zones_de_texte = true;
        }
        for (lien, description) in trouvees {
            if let Some(appel) = self.image(&lien, &description) {
                morceaux.push(Morceau::Image(appel));
            }
        }
    }

    /// Range une image par son lien ; rend son appel, ou `None` si elle n'est pas reprise.
    fn image(&mut self, lien: &str, description: &str) -> Option<String> {
        let Some((partie, externe)) = self.liens.get(lien).cloned() else {
            self.non_reprises += 1;
            return None;
        };
        if externe {
            self.non_reprises += 1;
            return None;
        }
        let deja = self.vues.get(&partie).cloned();
        let cle = match deja {
            Some(cle) => cle,
            None => {
                let cle = self.lire_image(&partie);
                self.vues.insert(partie, cle.clone());
                cle
            }
        }?;
        let description = description.trim();
        let description = if description.is_empty() {
            "Image"
        } else {
            description
        };
        Some(format!(
            "![{}](image:{cle})",
            echapper(&description.replace(['\r', '\n'], " "))
        ))
    }

    /// Lit une image de l'archive : PNG ou JPEG de 5 Mo au plus.
    fn lire_image(&mut self, partie: &str) -> Option<String> {
        let Ok(mut fichier) = self.archive.by_name(partie) else {
            self.non_reprises += 1;
            return None;
        };
        if fichier.size() > TAILLE_IMAGE_MAX {
            self.trop_lourdes += 1;
            return None;
        }
        let mut octets = Vec::new();
        if (&mut fichier)
            .take(TAILLE_IMAGE_MAX + 1)
            .read_to_end(&mut octets)
            .is_err()
        {
            self.non_reprises += 1;
            return None;
        }
        if octets.len() as u64 > TAILLE_IMAGE_MAX {
            self.trop_lourdes += 1;
            return None;
        }
        let sorte =
            reconnaitre(&octets).filter(|s| matches!(s, SorteFichier::Png | SorteFichier::Jpeg));
        let Some(sorte) = sorte else {
            self.non_reprises += 1;
            return None;
        };
        let cle = empreinte(&octets);
        if !self.images.iter().any(|i| i.empreinte == cle) {
            self.images.push(ImageWord {
                sorte,
                empreinte: cle.clone(),
                nom: partie.rsplit('/').next().unwrap_or(partie).to_owned(),
                octets,
            });
        }
        Some(cle)
    }

    fn paragraphe(&mut self, p: &Noeud) {
        let ppr = p.enfant("pPr");
        let style = ppr.and_then(|p| p.valeur("pStyle")).unwrap_or_default();
        let niveau = if style.is_empty() {
            None
        } else {
            self.styles.niveau_titre(style)
        }
        .or_else(|| {
            ppr.and_then(|p| p.valeur("outlineLvl"))
                .and_then(|v| v.parse::<u8>().ok())
                .filter(|n| *n < 9)
                .map(|n| (n + 1).min(3))
        });
        let numerotation = ppr
            .and_then(numerotation_de)
            .or_else(|| {
                (!style.is_empty())
                    .then(|| self.styles.numerotation(style))
                    .flatten()
            })
            .and_then(|(id, n)| self.numerotation.numerotee(&id, n).map(|num| (num, n)));
        let mut morceaux = Vec::new();
        let mut formules = 0;
        self.morceaux(p, &mut morceaux, &mut formules);
        self.formules += formules;
        let texte = ecrire_morceaux(&morceaux, niveau.is_some());
        match (niveau, numerotation) {
            (Some(n), _) => {
                let texte = texte.replace('\n', " ");
                if !texte.trim().is_empty() {
                    let diese = "#".repeat(usize::from(n));
                    self.pousser(format!("{diese} {}", texte.trim()), false);
                }
            }
            (None, Some((numerotee, n))) => {
                if !texte.trim().is_empty() {
                    let retrait = " ".repeat(4 * (n.min(8) as usize));
                    let marque = if numerotee { "1." } else { "-" };
                    // Les lignes suivantes d'un élément se rangent sous son texte.
                    let suite = format!("\n{retrait}{}", " ".repeat(marque.len() + 1));
                    let texte = texte.trim().replace('\n', &suite);
                    self.pousser(format!("{retrait}{marque} {texte}"), true);
                }
            }
            (None, None) => self.pousser(texte.trim().to_owned(), false),
        }
        for _ in 0..formules {
            self.pousser(NOTE_FORMULE.to_owned(), false);
        }
    }

    /// Un tableau simple : la première ligne est l'en-tête ; une cellule fusionnée laisse des
    /// cellules vides ; le texte de chaque cellule tient sur une ligne.
    fn tableau(&mut self, tableau: &Noeud) {
        let mut lignes: Vec<Vec<String>> = Vec::new();
        let mut formules = 0;
        for tr in lignes_du_tableau(tableau) {
            let mut ligne = Vec::new();
            for tc in cellules_de_la_ligne(tr) {
                let mut morceaux = Vec::new();
                self.morceaux_de_cellule(tc, &mut morceaux, &mut formules);
                let texte = ecrire_morceaux(&morceaux, true)
                    .replace('\n', " ")
                    .trim()
                    .to_owned();
                ligne.push(texte);
                let fusion = tc
                    .enfant("tcPr")
                    .and_then(|p| p.valeur("gridSpan"))
                    .and_then(|v| v.parse::<usize>().ok())
                    .unwrap_or(1)
                    .clamp(1, 64);
                ligne.extend(std::iter::repeat_n(String::new(), fusion - 1));
            }
            lignes.push(ligne);
        }
        self.formules += formules;
        let colonnes = lignes.iter().map(Vec::len).max().unwrap_or(0);
        if colonnes > 0 && lignes.iter().any(|l| l.iter().any(|c| !c.is_empty())) {
            let mut texte = String::new();
            for (i, ligne) in lignes.iter().enumerate() {
                let mut cellules = ligne.clone();
                cellules.resize(colonnes, String::new());
                texte.push_str("| ");
                texte.push_str(&cellules.join(" | "));
                texte.push_str(" |\n");
                if i == 0 {
                    texte.push('|');
                    texte.push_str(&"---|".repeat(colonnes));
                    texte.push('\n');
                }
            }
            self.pousser(texte.trim_end().to_owned(), false);
        }
        for _ in 0..formules {
            self.pousser(NOTE_FORMULE_TABLEAU.to_owned(), false);
        }
    }

    /// Morceaux de tous les paragraphes d'une cellule, tableaux imbriqués compris, séparés
    /// d'une espace.
    fn morceaux_de_cellule(
        &mut self,
        cellule: &Noeud,
        morceaux: &mut Vec<Morceau>,
        formules: &mut u32,
    ) {
        for n in cellule.noeuds() {
            match n.nom.as_str() {
                "p" => {
                    if !morceaux.is_empty() {
                        morceaux.push(Morceau::Texte {
                            texte: " ".to_owned(),
                            gras: false,
                            italique: false,
                        });
                    }
                    self.morceaux(n, morceaux, formules);
                }
                "tbl" => {
                    for tr in lignes_du_tableau(n) {
                        for tc in cellules_de_la_ligne(tr) {
                            self.morceaux_de_cellule(tc, morceaux, formules);
                        }
                    }
                }
                "sdt" => {
                    if let Some(c) = n.enfant("sdtContent") {
                        self.morceaux_de_cellule(c, morceaux, formules);
                    }
                }
                "customXml" => self.morceaux_de_cellule(n, morceaux, formules),
                _ => {}
            }
        }
    }
}

/// Lignes d'un tableau, à travers les contrôles de contenu.
fn lignes_du_tableau(tableau: &Noeud) -> Vec<&Noeud> {
    let mut lignes = Vec::new();
    for n in tableau.noeuds() {
        match n.nom.as_str() {
            "tr" => lignes.push(n),
            "sdt" | "customXml" => {
                let dedans = n.enfant("sdtContent").unwrap_or(n);
                lignes.extend(dedans.noeuds().filter(|r| r.nom == "tr"));
            }
            _ => {}
        }
    }
    lignes
}

/// Cellules d'une ligne, à travers les contrôles de contenu.
fn cellules_de_la_ligne(ligne: &Noeud) -> Vec<&Noeud> {
    let mut cellules = Vec::new();
    for n in ligne.noeuds() {
        match n.nom.as_str() {
            "tc" => cellules.push(n),
            "sdt" | "customXml" => {
                let dedans = n.enfant("sdtContent").unwrap_or(n);
                cellules.extend(dedans.noeuds().filter(|c| c.nom == "tc"));
            }
            _ => {}
        }
    }
    cellules
}

/// Vrai pour un objet de l'éditeur d'équations de Word (`Equation.3`, `Equation.DSMT4`…).
fn objet_formule(objet: &Noeud) -> bool {
    fn cherche(n: &Noeud, profondeur: usize) -> bool {
        if profondeur > 16 {
            return false;
        }
        n.attribut("ProgID")
            .is_some_and(|p| p.to_lowercase().starts_with("equation"))
            || n.noeuds().any(|e| cherche(e, profondeur + 1))
    }
    cherche(objet, 0)
}

/// Cherche les images d'un dessin : (lien, description). Ne descend pas dans les zones de texte.
fn chercher_images(
    noeud: &Noeud,
    description: Option<&str>,
    trouvees: &mut Vec<(String, String)>,
    zones: &mut bool,
    profondeur: usize,
) {
    if profondeur > 64 {
        return;
    }
    // `wp:docPr` porte la description d'un dessin ; elle vaut pour l'image qui suit.
    let description = noeud
        .noeuds()
        .find(|n| n.nom == "docPr")
        .and_then(|d| {
            d.attribut("descr")
                .filter(|t| !t.trim().is_empty())
                .or_else(|| d.attribut("title"))
        })
        .or(description);
    for n in noeud.noeuds() {
        match n.nom.as_str() {
            "txbxContent" => *zones = true,
            "Fallback" => {}
            "blip" => {
                if let Some(lien) = n.attribut("embed") {
                    trouvees.push((lien.to_owned(), description.unwrap_or_default().to_owned()));
                } else if n.attribut("link").is_some() {
                    trouvees.push((String::new(), String::new()));
                }
            }
            "imagedata" => {
                if let Some(lien) = n.attribut("id") {
                    let titre = n.attribut("title").or(description).unwrap_or_default();
                    trouvees.push((lien.to_owned(), titre.to_owned()));
                }
            }
            _ => chercher_images(n, description, trouvees, zones, profondeur + 1),
        }
    }
}

/// Écrit les morceaux d'un paragraphe en Markdown : texte échappé, gras et italique regroupés,
/// retours à la ligne forcés. `sans_saut` remplace les retours à la ligne par une espace (titre,
/// cellule).
fn ecrire_morceaux(morceaux: &[Morceau], sans_saut: bool) -> String {
    // Regroupe les textes voisins de même mise en forme.
    let mut groupes: Vec<Morceau> = Vec::new();
    for m in morceaux {
        match (groupes.last_mut(), m) {
            (
                Some(Morceau::Texte {
                    texte,
                    gras,
                    italique,
                }),
                Morceau::Texte {
                    texte: t,
                    gras: g,
                    italique: i,
                },
            ) if (*gras == *g && *italique == *i) || t.trim().is_empty() => texte.push_str(t),
            _ => groupes.push(m.clone()),
        }
    }
    let mut sortie = String::new();
    for m in &groupes {
        match m {
            Morceau::Texte {
                texte,
                gras,
                italique,
            } => {
                let texte = texte.replace(['\r', '\n'], " ");
                let coeur = texte.trim();
                if coeur.is_empty() {
                    sortie.push_str(&texte);
                    continue;
                }
                let avant = texte
                    .get(..texte.len() - texte.trim_start().len())
                    .unwrap_or_default();
                let apres = texte.get(texte.trim_end().len()..).unwrap_or_default();
                let marque = match (gras, italique) {
                    (true, true) => "***",
                    (true, false) => "**",
                    (false, true) => "*",
                    (false, false) => "",
                };
                sortie.push_str(avant);
                sortie.push_str(marque);
                sortie.push_str(&echapper(coeur));
                sortie.push_str(marque);
                sortie.push_str(apres);
            }
            Morceau::Saut => {
                if sans_saut {
                    sortie.push(' ');
                } else {
                    sortie.push_str("\\\n");
                }
            }
            Morceau::Image(appel) => sortie.push_str(appel),
        }
    }
    // Un retour à la ligne forcé en fin de paragraphe ne sert à rien.
    let mut sortie = sortie.trim_end().to_owned();
    while let Some(debut) = sortie.strip_suffix('\\') {
        sortie = debut.trim_end().to_owned();
    }
    // Chaque ligne : sans espaces en tête (ils feraient un bloc de code), et sans marque de
    // Markdown en tête (titre, citation, liste).
    sortie
        .split('\n')
        .map(|l| echapper_debut_de_ligne(l.trim_start()))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Échappe la ponctuation de Markdown et de LaTeX d'un texte venu de Word.
fn echapper(texte: &str) -> String {
    let mut sortie = String::with_capacity(texte.len());
    for c in texte.chars() {
        if matches!(
            c,
            '\\' | '`' | '*' | '_' | '[' | ']' | '$' | '|' | '<' | '>' | '~' | '&' | '#'
        ) {
            sortie.push('\\');
        }
        sortie.push(c);
    }
    sortie
}

/// Échappe ce qui, en tête de ligne, ferait une liste, une citation ou un titre (déjà échappés
/// partout : `#`, `>`).
fn echapper_debut_de_ligne(ligne: &str) -> String {
    let mut caracteres = ligne.chars();
    match caracteres.next() {
        Some(c @ ('-' | '+' | '=')) => format!("\\{c}{}", caracteres.as_str()),
        Some(c) if c.is_ascii_digit() => {
            let chiffres: String = ligne.chars().take_while(char::is_ascii_digit).collect();
            let reste = ligne.get(chiffres.len()..).unwrap_or_default();
            match reste.chars().next() {
                Some(p @ ('.' | ')')) => {
                    format!("{chiffres}\\{p}{}", reste.get(1..).unwrap_or_default())
                }
                _ => ligne.to_owned(),
            }
        }
        _ => ligne.to_owned(),
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use std::io::Write;

    use super::*;
    use crate::bibliotheque::{Bloc, SourceImage, analyser};

    /// Convertit, ou fait échouer l'essai.
    fn convertir(octets: &[u8]) -> LeconWord {
        let resultat = convertir_docx(octets);
        assert!(resultat.is_ok(), "{resultat:?}");
        resultat.unwrap_or_default()
    }

    const W: &str = "xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\" \
        xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" \
        xmlns:m=\"http://schemas.openxmlformats.org/officeDocument/2006/math\" \
        xmlns:wp=\"http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing\" \
        xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" \
        xmlns:pic=\"http://schemas.openxmlformats.org/drawingml/2006/picture\" \
        xmlns:mc=\"http://schemas.openxmlformats.org/markup-compatibility/2006\" \
        xmlns:wps=\"http://schemas.microsoft.com/office/word/2010/wordprocessingShape\"";

    const STYLES: &str = r#"<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
        <w:style w:type="paragraph" w:styleId="Titre1"><w:name w:val="heading 1"/></w:style>
        <w:style w:type="paragraph" w:styleId="Titre2"><w:name w:val="heading 2"/></w:style>
        <w:style w:type="paragraph" w:styleId="Monchapitre"><w:name w:val="Mon chapitre"/><w:basedOn w:val="Titre2"/></w:style>
        <w:style w:type="paragraph" w:styleId="Title"><w:name w:val="Title"/></w:style>
        <w:style w:type="character" w:styleId="lev"><w:name w:val="Strong"/></w:style>
        </w:styles>"#;

    const NUMEROTATION: &str = r#"<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
        <w:abstractNum w:abstractNumId="0"><w:lvl w:ilvl="0"><w:numFmt w:val="bullet"/></w:lvl><w:lvl w:ilvl="1"><w:numFmt w:val="bullet"/></w:lvl></w:abstractNum>
        <w:abstractNum w:abstractNumId="1"><w:lvl w:ilvl="0"><w:numFmt w:val="decimal"/></w:lvl></w:abstractNum>
        <w:num w:numId="1"><w:abstractNumId w:val="0"/></w:num>
        <w:num w:numId="2"><w:abstractNumId w:val="1"/></w:num>
        </w:numbering>"#;

    fn png(graine: u8, taille: usize) -> Vec<u8> {
        let mut o = b"\x89PNG\r\n\x1a\n".to_vec();
        o.resize(taille.max(16), graine);
        o
    }

    fn dessin(lien: &str, description: &str) -> String {
        format!(
            r#"<w:r><w:drawing><wp:inline><wp:docPr id="1" name="Image" descr="{description}"/><a:graphic><a:graphicData><pic:pic><pic:blipFill><a:blip r:embed="{lien}"/></pic:blipFill></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r>"#
        )
    }

    /// Fabrique un `.docx` : corps du document, liens d'images et fichiers en plus.
    fn docx(corps: &str, images: &[(&str, &str, Vec<u8>)], en_plus: &[(&str, &str)]) -> Vec<u8> {
        let mut liens = String::from(
            r#"<Relationship Id="rIdS" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rIdN" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering" Target="numbering.xml"/>"#,
        );
        for (id, nom, _) in images {
            liens.push_str(&format!(
                r#"<Relationship Id="{id}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/{nom}"/>"#
            ));
        }
        for (_, rel) in en_plus.iter().filter(|(n, _)| n.is_empty()) {
            liens.push_str(rel);
        }
        let mut sortie = Vec::new();
        {
            let mut z = zip::ZipWriter::new(Cursor::new(&mut sortie));
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            let mut ecrire = |nom: &str, octets: &[u8]| {
                z.start_file(nom, options).ok();
                z.write_all(octets).ok();
            };
            ecrire(
                "_rels/.rels",
                br#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#,
            );
            ecrire(
                "word/document.xml",
                format!(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document {W}><w:body>{corps}<w:sectPr/></w:body></w:document>"#).as_bytes(),
            );
            ecrire(
                "word/_rels/document.xml.rels",
                format!(r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">{liens}</Relationships>"#).as_bytes(),
            );
            ecrire("word/styles.xml", STYLES.as_bytes());
            ecrire("word/numbering.xml", NUMEROTATION.as_bytes());
            for (_, nom, octets) in images {
                ecrire(&format!("word/media/{nom}"), octets);
            }
            for (nom, contenu) in en_plus.iter().filter(|(n, _)| !n.is_empty()) {
                ecrire(nom, contenu.as_bytes());
            }
            z.finish().ok();
        }
        sortie
    }

    fn p(style: &str, corps: &str) -> String {
        if style.is_empty() {
            format!("<w:p>{corps}</w:p>")
        } else {
            format!(r#"<w:p><w:pPr><w:pStyle w:val="{style}"/></w:pPr>{corps}</w:p>"#)
        }
    }

    fn r(texte: &str) -> String {
        format!(r#"<w:r><w:t xml:space="preserve">{texte}</w:t></w:r>"#)
    }

    fn element(numero: &str, niveau: u32, corps: &str) -> String {
        format!(
            r#"<w:p><w:pPr><w:numPr><w:ilvl w:val="{niveau}"/><w:numId w:val="{numero}"/></w:numPr></w:pPr>{corps}</w:p>"#
        )
    }

    /// Le document d'essai du critère 3 : titres, gras, liste, tableau, deux images, une formule.
    fn document_d_essai() -> Vec<u8> {
        let corps = [
            p("Title", &r("Les fractions")),
            p("Titre1", &r("L'essentiel")),
            p(
                "",
                &format!(
                    r#"{}<w:r><w:rPr><w:b/></w:rPr><w:t>numérateur</w:t></w:r>{}<w:r><w:rPr><w:i/></w:rPr><w:t>dénominateur</w:t></w:r>"#,
                    r("Une fraction a un "),
                    r(" et un "),
                ),
            ),
            element("1", 0, &r("un quart")),
            element("1", 1, &r("un demi")),
            element("2", 0, &r("première étape")),
            element("2", 0, &r("deuxième étape")),
            "<w:tbl><w:tblGrid/><w:tr><w:tc><w:p><w:r><w:t>Fraction</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>Valeur</w:t></w:r></w:p></w:tc></w:tr><w:tr><w:tc><w:p><w:r><w:t>1/2</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>0,5 | moitié</w:t></w:r></w:p></w:tc></w:tr></w:tbl>".to_owned(),
            p("", &dessin("rIdI1", "Un gâteau coupé")),
            p("Monchapitre", &r("Exemples")),
            p("", &format!("{}<m:oMathPara><m:oMath><m:r><m:t>x</m:t></m:r></m:oMath></m:oMathPara>", r("On calcule :"))),
            p("", &dessin("rIdI2", "")),
            p("", &r("Prix : 5 $ ; * étoile ; [crochets] ; &lt;b&gt;gras&lt;/b&gt; &#233;")),
        ]
        .concat();
        docx(
            &corps,
            &[
                ("rIdI1", "image1.png", png(1, 64)),
                ("rIdI2", "image2.jpeg", {
                    let mut o = vec![0xFF, 0xD8, 0xFF, 0xE0];
                    o.resize(64, 2);
                    o
                }),
            ],
            &[],
        )
    }

    #[test]
    fn un_document_d_essai_devient_une_lecon() {
        let lecon = convertir(&document_d_essai());
        let texte = &lecon.texte;
        assert!(
            texte.starts_with("# Les fractions\n\n# L'essentiel\n\n"),
            "{texte}"
        );
        assert!(
            texte.contains("Une fraction a un **numérateur** et un *dénominateur*"),
            "{texte}"
        );
        assert!(
            texte.contains("- un quart\n    - un demi\n1. première étape\n1. deuxième étape"),
            "{texte}"
        );
        assert!(
            texte.contains("| Fraction | Valeur |\n|---|---|\n| 1/2 | 0,5 \\| moitié |"),
            "{texte}"
        );
        assert!(texte.contains("## Exemples"), "{texte}");
        assert!(
            texte.contains("Prix : 5 \\$ ; \\* étoile ; \\[crochets\\] ; \\<b\\>gras\\</b\\> é"),
            "{texte}"
        );
        assert_eq!(lecon.images.len(), 2);
        assert_eq!(lecon.images[0].sorte, SorteFichier::Png);
        assert_eq!(lecon.images[1].sorte, SorteFichier::Jpeg);
        assert!(texte.contains(&format!(
            "![Un gâteau coupé](image:{})",
            lecon.images[0].empreinte
        )));
        assert!(texte.contains(&format!("![Image](image:{})", lecon.images[1].empreinte)));
        assert_eq!(lecon.notes, vec![NoteWord::Formules { nombre: 1 }]);

        // L'arbre de la leçon : titres, liste, tableau, images, note pour le maître.
        let arbre = analyser(texte);
        let titres = arbre
            .blocs
            .iter()
            .filter(|b| matches!(b, Bloc::Titre { .. }))
            .count();
        assert_eq!(titres, 3);
        assert!(arbre.blocs.iter().any(|b| matches!(
            b,
            Bloc::Liste {
                numerotee: false,
                ..
            }
        )));
        assert!(arbre.blocs.iter().any(|b| matches!(
            b,
            Bloc::Liste {
                numerotee: true,
                ..
            }
        )));
        assert!(
            arbre
                .blocs
                .iter()
                .any(|b| matches!(b, Bloc::Tableau { lignes, .. } if lignes.len() == 1))
        );
        assert_eq!(arbre.images().len(), 2);
        assert!(
            arbre
                .images()
                .iter()
                .all(|s| matches!(s, SourceImage::Ecole { .. }))
        );
        assert!(
            arbre.formules().is_empty(),
            "le $ échappé n'ouvre pas de formule"
        );
        let note = arbre
            .blocs
            .iter()
            .find(|b| matches!(b, Bloc::NoteMaitre { .. }));
        assert!(note.is_some(), "la formule devient une note pour le maître");
        // Les élèves ne la voient pas.
        let eleves = arbre.pour_les_eleves();
        assert!(
            !eleves
                .blocs
                .iter()
                .any(|b| matches!(b, Bloc::NoteMaitre { .. }))
        );
        assert!(!eleves.texte_brut().contains("Formule à retaper"));
        // Le HTML reste du texte.
        assert!(arbre.texte_brut().contains("<b>gras</b> é"));
    }

    #[test]
    fn un_ancien_doc_un_fichier_illisible_et_un_fichier_trop_lourd_sont_refuses() {
        let mut doc = ENTETE_OLE.to_vec();
        doc.extend_from_slice(&[0; 512]);
        assert_eq!(convertir_docx(&doc), Err(ErreurWord::AncienFormat));
        assert_eq!(
            ErreurWord::AncienFormat.to_string(),
            "Enregistrez ce fichier au format .docx."
        );
        assert_eq!(convertir_docx(b"bonjour"), Err(ErreurWord::Illisible));
        assert_eq!(
            convertir_docx(b"PK\x03\x04abime"),
            Err(ErreurWord::Illisible)
        );
        let mut lourd = b"PK\x03\x04".to_vec();
        lourd.resize(usize::try_from(TAILLE_WORD_MAX).unwrap_or(0) + 1, 0);
        assert_eq!(convertir_docx(&lourd), Err(ErreurWord::TropLourd));
        // Une archive sans document Word.
        let mut zip_vide = Vec::new();
        {
            let mut z = zip::ZipWriter::new(Cursor::new(&mut zip_vide));
            z.start_file("autre.txt", zip::write::SimpleFileOptions::default())
                .ok();
            z.write_all(b"rien").ok();
            z.finish().ok();
        }
        assert_eq!(convertir_docx(&zip_vide), Err(ErreurWord::Illisible));
    }

    #[test]
    fn une_image_trop_lourde_ou_d_un_autre_format_est_laissee_de_cote() {
        let lourde = png(3, usize::try_from(TAILLE_IMAGE_MAX).unwrap_or(0) + 1);
        let corps = [
            p("", &dessin("rIdL", "lourde")),
            p("", &dessin("rIdE", "schéma")),
            p("", &dessin("rIdP", "petite")),
            p("", &dessin("rIdP", "la même")),
            p("", &r("Suite du texte.")),
        ]
        .concat();
        let lecon = convertir(&docx(
            &corps,
            &[
                ("rIdL", "lourde.png", lourde),
                ("rIdE", "schema.emf", vec![1, 0, 0, 0, 0x6C, 0, 0, 0]),
                ("rIdP", "petite.png", png(4, 32)),
            ],
            &[],
        ));
        assert_eq!(
            lecon.images.len(),
            1,
            "la même image n'est rangée qu'une fois"
        );
        assert_eq!(lecon.images[0].nom, "petite.png");
        assert!(lecon.texte.contains("Suite du texte."));
        assert_eq!(
            lecon.notes,
            vec![
                NoteWord::ImagesTropLourdes { nombre: 1 },
                NoteWord::ImagesNonReprises { nombre: 1 },
            ]
        );
    }

    #[test]
    fn zones_de_texte_revisions_champs_et_en_tetes_sont_ignores() {
        let corps = [
            p(
                "",
                &format!(
                    r#"{}<w:del><w:r><w:delText>supprimé</w:delText></w:r></w:del><w:ins><w:r><w:t>ajouté</w:t></w:r></w:ins>"#,
                    r("Texte ")
                ),
            ),
            p(
                "",
                r#"<w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText>PAGE</w:instrText></w:r><w:r><w:fldChar w:fldCharType="separate"/></w:r><w:r><w:t>3</w:t></w:r><w:r><w:fldChar w:fldCharType="end"/></w:r>"#,
            ),
            p(
                "",
                r#"<w:r><mc:AlternateContent><mc:Choice Requires="wps"><w:drawing><wp:anchor><a:graphic><a:graphicData><wps:wsp><wps:txbx><w:txbxContent><w:p><w:r><w:t>dans la zone</w:t></w:r></w:p></w:txbxContent></wps:txbx></wps:wsp></a:graphicData></a:graphic></wp:anchor></w:drawing></mc:Choice><mc:Fallback><w:pict><w:t>secours</w:t></w:pict></mc:Fallback></mc:AlternateContent></w:r>"#,
            ),
        ]
        .concat();
        let en_tete = r#"<Relationship Id="rIdH" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header1.xml"/>"#;
        let lecon = convertir(&docx(&corps, &[], &[("", en_tete)]));
        assert_eq!(lecon.texte, "Texte ajouté\n\n3\n");
        assert_eq!(
            lecon.notes,
            vec![NoteWord::ZonesDeTexte, NoteWord::ElementsIgnores]
        );
    }

    #[test]
    fn retours_a_la_ligne_et_debuts_de_ligne_restent_du_texte() {
        let corps = [
            p(
                "",
                &format!("{}<w:r><w:br/></w:r>{}", r("Ligne un"), r("# pas un titre")),
            ),
            p("", &r("- pas une liste")),
            p("", &r("1. pas un numéro")),
            p("", &format!("{}<w:r><w:br/></w:r>", r("Fin"))),
            p(
                "Titre1",
                &format!("{}<w:r><w:br/></w:r>{}", r("Titre sur"), r("deux lignes")),
            ),
        ]
        .concat();
        let lecon = convertir(&docx(&corps, &[], &[]));
        assert_eq!(
            lecon.texte,
            "Ligne un\\\n\\# pas un titre\n\n\\- pas une liste\n\n1\\. pas un numéro\n\nFin\n\n# Titre sur deux lignes\n"
        );
        let arbre = analyser(&lecon.texte);
        assert_eq!(
            arbre
                .blocs
                .iter()
                .filter(|b| matches!(b, Bloc::Paragraphe { .. }))
                .count(),
            4
        );
        assert_eq!(
            arbre
                .blocs
                .iter()
                .filter(|b| matches!(b, Bloc::Titre { .. }))
                .count(),
            1
        );
    }

    #[test]
    fn les_noms_de_styles_donnent_les_niveaux_de_titre() {
        assert_eq!(niveau_d_apres_le_nom("heading 1"), Some(1));
        assert_eq!(niveau_d_apres_le_nom("Titre 2"), Some(2));
        assert_eq!(niveau_d_apres_le_nom("Heading4"), Some(3));
        assert_eq!(niveau_d_apres_le_nom("Title"), Some(1));
        assert_eq!(niveau_d_apres_le_nom("Normal"), None);
        assert_eq!(niveau_d_apres_le_nom("Titre de la leçon"), None);
        assert_eq!(
            chemin_dans_archive("word/", "../media/a%20b.png"),
            "media/a b.png"
        );
        assert_eq!(
            chemin_dans_archive("word/", "/word/media/x.png"),
            "word/media/x.png"
        );
    }
}
