//! Format d'une leçon (ADR « format des leçons ») : un texte Markdown restreint, formules LaTeX
//! entre `$…$` et `$$…$$`, lu une seule fois ici en un arbre de blocs. L'écran affiche cet arbre,
//! l'impression le traduit en Typst ; ni l'un ni l'autre n'analyse le texte.
//!
//! Le noyau filtre le texte au passage (règle 10 de la spécification) : le HTML brut reste du
//! texte, un lien garde son texte et perd sa cible, une image n'est appelée que par un nom de
//! fichier du paquet ou par une empreinte, une vidéo que par une empreinte. L'analyse ne refuse
//! jamais un texte : ce qu'elle ne reconnaît pas devient du texte.

use pulldown_cmark::{Alignment, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag};
use serde::{Deserialize, Serialize};

/// Profondeur d'imbrication au-delà de laquelle le reste d'un bloc devient du texte : une leçon
/// réelle ne dépasse pas cinq niveaux ; la borne protège la pile d'un texte fabriqué.
const PROFONDEUR_MAX: usize = 32;

/// Début d'une citation qui est une note pour le maître, jamais montrée aux élèves.
const NOTE_POUR_LE_MAITRE: [&str; 2] = ["note pour le maître", "note pour le maitre"];

/// Une leçon analysée : la suite de ses blocs.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, specta::Type)]
pub struct Lecon {
    /// Blocs dans l'ordre du texte.
    pub blocs: Vec<Bloc>,
}

/// Un bloc d'une leçon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Bloc {
    /// Titre de niveau 1 à 3 (les niveaux plus bas sont ramenés à 3).
    Titre {
        /// 1, 2 ou 3.
        niveau: u8,
        /// Texte du titre.
        contenu: Vec<Enligne>,
    },
    /// Paragraphe.
    Paragraphe {
        /// Texte du paragraphe.
        contenu: Vec<Enligne>,
    },
    /// Formule seule sur sa ligne (`$$…$$`).
    Formule {
        /// Texte LaTeX, tel qu'il est écrit.
        latex: String,
    },
    /// Liste, numérotée ou non.
    Liste {
        /// Vrai pour une liste numérotée.
        numerotee: bool,
        /// Premier numéro d'une liste numérotée.
        debut: Option<u32>,
        /// Éléments, chacun une suite de blocs.
        elements: Vec<Vec<Bloc>>,
    },
    /// Tableau.
    Tableau {
        /// Alignement de chaque colonne.
        alignements: Vec<Alignement>,
        /// Cellules de la ligne d'en-tête.
        entete: Vec<Vec<Enligne>>,
        /// Lignes du corps, chacune une suite de cellules.
        lignes: Vec<Vec<Vec<Enligne>>>,
    },
    /// Citation.
    Citation {
        /// Blocs cités.
        blocs: Vec<Bloc>,
    },
    /// Note pour le maître : une citation qui commence par « Note pour le maître ». Vue du seul
    /// personnel qui écrit ; jamais des élèves ni de l'impression pour les élèves.
    NoteMaitre {
        /// Blocs de la note, « Note pour le maître » compris.
        blocs: Vec<Bloc>,
    },
    /// Texte montré tel qu'il est écrit, en caractères à chasse fixe (bloc de code, HTML brut).
    TexteBrut {
        /// Texte.
        texte: String,
    },
    /// Filet de séparation.
    Filet,
}

/// Alignement d'une colonne de tableau.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum Alignement {
    /// Aucun alignement demandé.
    Aucun,
    /// À gauche.
    Gauche,
    /// Centré.
    Centre,
    /// À droite.
    Droite,
}

/// Un élément du texte d'un bloc.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Enligne {
    /// Texte simple.
    Texte {
        /// Texte.
        texte: String,
    },
    /// Texte à chasse fixe (`` `…` ``).
    Code {
        /// Texte.
        texte: String,
    },
    /// Texte en gras.
    Gras {
        /// Contenu.
        contenu: Vec<Enligne>,
    },
    /// Texte en italique.
    Italique {
        /// Contenu.
        contenu: Vec<Enligne>,
    },
    /// Formule LaTeX.
    Formule {
        /// Texte LaTeX, tel qu'il est écrit.
        latex: String,
        /// Vrai pour `$$…$$` (formule centrée), faux pour `$…$` (dans la ligne).
        seule: bool,
    },
    /// Image.
    Image {
        /// D'où vient l'image.
        source: SourceImage,
        /// Description (texte entre crochets).
        description: String,
    },
    /// Vidéo de l'école (`![titre](video:<empreinte>)`).
    Video {
        /// Empreinte SHA-256 du fichier, en hexadécimal minuscule.
        empreinte: String,
        /// Titre.
        titre: String,
    },
    /// Son de l'école (`![titre](son:<empreinte>)`), rangé dans la base.
    Son {
        /// Empreinte SHA-256 du fichier, en hexadécimal minuscule.
        empreinte: String,
        /// Titre.
        titre: String,
    },
    /// Retour à la ligne forcé.
    Saut,
}

/// D'où vient une image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SourceImage {
    /// Un fichier du paquet, par son nom (`figure-1.svg`).
    Paquet {
        /// Nom du fichier, sans dossier.
        nom: String,
    },
    /// Une image de l'école, par son empreinte (`image:<empreinte>`).
    Ecole {
        /// Empreinte SHA-256, en hexadécimal minuscule.
        empreinte: String,
    },
}

/// Une formule d'une leçon, pour l'auteur (formules que l'impression montrera telles quelles) et
/// pour les essais.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormuleLecon {
    /// Texte LaTeX.
    pub latex: String,
    /// Formule seule sur sa ligne.
    pub seule: bool,
}

/// Lit le texte d'une leçon. Ne refuse jamais : ce qui n'est pas reconnu devient du texte.
pub fn analyser(texte: &str) -> Lecon {
    let options = Options::ENABLE_TABLES | Options::ENABLE_MATH;
    let texte = proteger_descriptions(texte);
    let mut lecteur = Lecteur {
        evenements: Parser::new_ext(&texte, options),
        profondeur: 0,
    };
    Lecon {
        blocs: lecteur.blocs(),
    }
}

impl Lecon {
    /// Toutes les formules, dans l'ordre du texte.
    pub fn formules(&self) -> Vec<FormuleLecon> {
        let mut formules = Vec::new();
        for bloc in &self.blocs {
            formules_bloc(bloc, &mut formules);
        }
        formules
    }

    /// Texte de la leçon sans balisage, formules comprises, pour la recherche.
    pub fn texte_brut(&self) -> String {
        let mut texte = String::new();
        for bloc in &self.blocs {
            texte_bloc(bloc, &mut texte);
        }
        texte.trim().to_owned()
    }

    /// Images de la leçon, dans l'ordre du texte.
    pub fn images(&self) -> Vec<&SourceImage> {
        let mut images = Vec::new();
        for bloc in &self.blocs {
            parcourir_enlignes_bloc(bloc, &mut |e| {
                if let Enligne::Image { source, .. } = e {
                    images.push(source);
                }
            });
        }
        images
    }

    /// Empreintes des vidéos de la leçon, dans l'ordre du texte.
    pub fn videos(&self) -> Vec<&str> {
        let mut videos = Vec::new();
        for bloc in &self.blocs {
            parcourir_enlignes_bloc(bloc, &mut |e| {
                if let Enligne::Video { empreinte, .. } = e {
                    videos.push(empreinte.as_str());
                }
            });
        }
        videos
    }

    /// Empreintes des sons de la leçon, dans l'ordre du texte.
    pub fn sons(&self) -> Vec<&str> {
        let mut sons = Vec::new();
        for bloc in &self.blocs {
            parcourir_enlignes_bloc(bloc, &mut |e| {
                if let Enligne::Son { empreinte, .. } = e {
                    sons.push(empreinte.as_str());
                }
            });
        }
        sons
    }

    /// La leçon sans les notes pour le maître : ce que voient les élèves.
    pub fn pour_les_eleves(&self) -> Lecon {
        Lecon {
            blocs: sans_notes(&self.blocs),
        }
    }

    /// La leçon « sans les corrigés » (forme d'impression) : la section dont le titre est
    /// « Corrigés » et, sous le quiz, la sous-section « Réponses » sont retirées jusqu'au titre
    /// suivant de même niveau ou plus haut. Une leçon qui n'a pas ces titres reste entière.
    pub fn sans_les_corriges(&self) -> Lecon {
        let mut blocs = Vec::new();
        // Niveau du titre de la section qu'on saute, s'il y en a une.
        let mut saute: Option<u8> = None;
        // Niveau du titre de la section du quiz en cours.
        let mut quiz: Option<u8> = None;
        for bloc in &self.blocs {
            if let Bloc::Titre { niveau, contenu } = bloc {
                if saute.is_some_and(|n| *niveau <= n) {
                    saute = None;
                }
                if quiz.is_some_and(|n| *niveau <= n) {
                    quiz = None;
                }
                if saute.is_none() {
                    let titre = titre_simple(&texte_de(contenu));
                    if titre.starts_with("corrige") || (quiz.is_some() && titre == "reponses") {
                        saute = Some(*niveau);
                    } else if titre.starts_with("quiz") {
                        quiz = Some(*niveau);
                    }
                }
            }
            if saute.is_none() {
                blocs.push(bloc.clone());
            }
        }
        Lecon { blocs }
    }
}

/// Titre ramené à sa forme simple : sans numéro en tête (« 6. »), sans accents, en minuscules.
fn titre_simple(titre: &str) -> String {
    let sans_numero = titre
        .trim()
        .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ')')
        .trim();
    sans_numero
        .chars()
        .map(|c| match c {
            'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' => 'e',
            'à' | 'â' | 'À' | 'Â' => 'a',
            'î' | 'ï' | 'Î' => 'i',
            'ô' | 'Ô' => 'o',
            'û' | 'ù' | 'ü' | 'Û' => 'u',
            'ç' | 'Ç' => 'c',
            autre => autre.to_ascii_lowercase(),
        })
        .collect()
}

fn sans_notes(blocs: &[Bloc]) -> Vec<Bloc> {
    blocs
        .iter()
        .filter(|b| !matches!(b, Bloc::NoteMaitre { .. }))
        .map(|b| match b {
            Bloc::Citation { blocs } => Bloc::Citation {
                blocs: sans_notes(blocs),
            },
            Bloc::Liste {
                numerotee,
                debut,
                elements,
            } => Bloc::Liste {
                numerotee: *numerotee,
                debut: *debut,
                elements: elements.iter().map(|e| sans_notes(e)).collect(),
            },
            autre => autre.clone(),
        })
        .collect()
}

fn parcourir_enlignes_bloc<'a>(bloc: &'a Bloc, f: &mut impl FnMut(&'a Enligne)) {
    match bloc {
        Bloc::Titre { contenu, .. } | Bloc::Paragraphe { contenu } => {
            parcourir_enlignes(contenu, f);
        }
        Bloc::Liste { elements, .. } => {
            for element in elements {
                for b in element {
                    parcourir_enlignes_bloc(b, f);
                }
            }
        }
        Bloc::Tableau { entete, lignes, .. } => {
            for cellule in entete.iter().chain(lignes.iter().flatten()) {
                parcourir_enlignes(cellule, f);
            }
        }
        Bloc::Citation { blocs } | Bloc::NoteMaitre { blocs } => {
            for b in blocs {
                parcourir_enlignes_bloc(b, f);
            }
        }
        Bloc::Formule { .. } | Bloc::TexteBrut { .. } | Bloc::Filet => {}
    }
}

fn parcourir_enlignes<'a>(enlignes: &'a [Enligne], f: &mut impl FnMut(&'a Enligne)) {
    for e in enlignes {
        f(e);
        if let Enligne::Gras { contenu } | Enligne::Italique { contenu } = e {
            parcourir_enlignes(contenu, f);
        }
    }
}

fn formules_bloc(bloc: &Bloc, formules: &mut Vec<FormuleLecon>) {
    if let Bloc::Formule { latex } = bloc {
        formules.push(FormuleLecon {
            latex: latex.clone(),
            seule: true,
        });
        return;
    }
    if let Bloc::Liste { elements, .. } = bloc {
        for b in elements.iter().flatten() {
            formules_bloc(b, formules);
        }
        return;
    }
    if let Bloc::Citation { blocs } | Bloc::NoteMaitre { blocs } = bloc {
        for b in blocs {
            formules_bloc(b, formules);
        }
        return;
    }
    parcourir_enlignes_bloc(bloc, &mut |e| {
        if let Enligne::Formule { latex, seule } = e {
            formules.push(FormuleLecon {
                latex: latex.clone(),
                seule: *seule,
            });
        }
    });
}

fn texte_bloc(bloc: &Bloc, texte: &mut String) {
    match bloc {
        Bloc::Titre { contenu, .. } | Bloc::Paragraphe { contenu } => {
            texte_enlignes(contenu, texte);
        }
        Bloc::Formule { latex } => texte.push_str(latex),
        Bloc::Liste { elements, .. } => {
            for b in elements.iter().flatten() {
                texte_bloc(b, texte);
            }
        }
        Bloc::Tableau { entete, lignes, .. } => {
            for cellule in entete.iter().chain(lignes.iter().flatten()) {
                texte_enlignes(cellule, texte);
                texte.push(' ');
            }
        }
        Bloc::Citation { blocs } | Bloc::NoteMaitre { blocs } => {
            for b in blocs {
                texte_bloc(b, texte);
            }
        }
        Bloc::TexteBrut { texte: t } => texte.push_str(t),
        Bloc::Filet => {}
    }
    texte.push('\n');
}

fn texte_enlignes(enlignes: &[Enligne], texte: &mut String) {
    for e in enlignes {
        match e {
            Enligne::Texte { texte: t } | Enligne::Code { texte: t } => texte.push_str(t),
            Enligne::Gras { contenu } | Enligne::Italique { contenu } => {
                texte_enlignes(contenu, texte);
            }
            Enligne::Formule { latex, .. } => {
                texte.push(' ');
                texte.push_str(latex);
                texte.push(' ');
            }
            Enligne::Image { description, .. } => texte.push_str(description),
            Enligne::Video { titre, .. } | Enligne::Son { titre, .. } => texte.push_str(titre),
            Enligne::Saut => texte.push(' '),
        }
    }
}

/// Texte d'une suite d'éléments, sans balisage.
fn texte_de(enlignes: &[Enligne]) -> String {
    let mut texte = String::new();
    texte_enlignes(enlignes, &mut texte);
    texte
}

/// Vrai pour une empreinte SHA-256 en hexadécimal minuscule.
fn est_empreinte(texte: &str) -> bool {
    texte.len() == 64
        && texte
            .bytes()
            .all(|o| o.is_ascii_digit() || (b'a'..=b'f').contains(&o))
}

/// Vrai pour un nom de fichier image du paquet : sans dossier, sans adresse.
fn est_nom_image(nom: &str) -> bool {
    let minuscule = nom.to_ascii_lowercase();
    let extension_connue = [".svg", ".png", ".jpg", ".jpeg"]
        .iter()
        .any(|e| minuscule.ends_with(e));
    extension_connue
        && !nom.starts_with('.')
        && !nom.contains("..")
        && nom
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

/// Protège les crochets de la description d'une image : « la demi-droite [AB) » ne doit pas
/// couper l'appel `![…](figure.svg)`. Seuls les appels reconnus, écrits sur une ligne, dont les
/// crochets ne vont pas par paires, sont touchés ; un crochet déjà protégé le reste.
fn proteger_descriptions(texte: &str) -> std::borrow::Cow<'_, str> {
    if !texte.contains("![") {
        return std::borrow::Cow::Borrowed(texte);
    }
    let mut sortie = String::with_capacity(texte.len().saturating_add(16));
    let mut change = false;
    for ligne in texte.split_inclusive('\n') {
        let mut reste = ligne;
        while let Some((avant, apres)) = reste.split_once("![") {
            sortie.push_str(avant);
            sortie.push_str("![");
            reste = apres;
            let Some(fin) = fin_de_description(apres) else {
                continue;
            };
            let (description, suite) = apres.split_at(fin);
            if crochets_sans_paire(description) {
                let mut precedent = ' ';
                for c in description.chars() {
                    if matches!(c, '[' | ']') && precedent != '\\' {
                        sortie.push('\\');
                        change = true;
                    }
                    sortie.push(c);
                    precedent = c;
                }
            } else {
                sortie.push_str(description);
            }
            reste = suite;
        }
        sortie.push_str(reste);
    }
    if change {
        std::borrow::Cow::Owned(sortie)
    } else {
        std::borrow::Cow::Borrowed(texte)
    }
}

/// Position du `](` qui ferme la description d'un appel d'image ou de vidéo reconnu.
fn fin_de_description(apres: &str) -> Option<usize> {
    let mut depart = 0;
    while let Some(j) = apres.get(depart..)?.find("](") {
        let fin = depart.saturating_add(j);
        let cible = apres.get(fin.saturating_add(2)..)?.split(')').next()?;
        if appel_image(cible, String::new()).is_some() {
            return Some(fin);
        }
        depart = fin.saturating_add(2);
    }
    None
}

/// Vrai si les crochets non protégés d'un texte ne vont pas par paires.
fn crochets_sans_paire(texte: &str) -> bool {
    let mut profondeur: usize = 0;
    let mut precedent = ' ';
    for c in texte.chars() {
        if precedent != '\\' {
            match c {
                '[' => profondeur = profondeur.saturating_add(1),
                ']' => match profondeur.checked_sub(1) {
                    Some(p) => profondeur = p,
                    None => return true,
                },
                _ => {}
            }
        }
        precedent = c;
    }
    profondeur != 0
}

/// Ce qu'appelle `![description](cible)`.
fn appel_image(cible: &str, description: String) -> Option<Enligne> {
    if let Some(empreinte) = cible.strip_prefix("video:") {
        return est_empreinte(empreinte).then(|| Enligne::Video {
            empreinte: empreinte.to_owned(),
            titre: description,
        });
    }
    if let Some(empreinte) = cible.strip_prefix("son:") {
        return est_empreinte(empreinte).then(|| Enligne::Son {
            empreinte: empreinte.to_owned(),
            titre: description,
        });
    }
    if let Some(empreinte) = cible.strip_prefix("image:") {
        return est_empreinte(empreinte).then(|| Enligne::Image {
            source: SourceImage::Ecole {
                empreinte: empreinte.to_owned(),
            },
            description,
        });
    }
    est_nom_image(cible).then(|| Enligne::Image {
        source: SourceImage::Paquet {
            nom: cible.to_owned(),
        },
        description,
    })
}

/// Ajoute un élément, en fusionnant deux textes qui se suivent.
fn pousser(enlignes: &mut Vec<Enligne>, element: Enligne) {
    if let Enligne::Texte { texte: nouveau } = &element {
        if nouveau.is_empty() {
            return;
        }
        if let Some(Enligne::Texte { texte }) = enlignes.last_mut() {
            texte.push_str(nouveau);
            return;
        }
    }
    enlignes.push(element);
}

fn est_note_pour_le_maitre(blocs: &[Bloc]) -> bool {
    let Some(Bloc::Paragraphe { contenu }) = blocs.first() else {
        return false;
    };
    let debut = texte_de(contenu).trim_start().to_lowercase();
    NOTE_POUR_LE_MAITRE.iter().any(|n| debut.starts_with(n))
}

/// Un paragraphe fait d'une seule formule `$$…$$` devient un bloc de formule.
fn paragraphe(contenu: Vec<Enligne>) -> Option<Bloc> {
    let utiles: Vec<&Enligne> = contenu
        .iter()
        .filter(|e| !matches!(e, Enligne::Texte { texte } if texte.trim().is_empty()))
        .collect();
    match utiles.as_slice() {
        [] => None,
        [Enligne::Formule { latex, seule: true }] => Some(Bloc::Formule {
            latex: latex.clone(),
        }),
        _ => Some(Bloc::Paragraphe { contenu }),
    }
}

fn niveau_titre(niveau: HeadingLevel) -> u8 {
    match niveau {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        _ => 3,
    }
}

fn alignement(a: Alignment) -> Alignement {
    match a {
        Alignment::None => Alignement::Aucun,
        Alignment::Left => Alignement::Gauche,
        Alignment::Center => Alignement::Centre,
        Alignment::Right => Alignement::Droite,
    }
}

struct Lecteur<'a> {
    evenements: Parser<'a>,
    profondeur: usize,
}

impl Lecteur<'_> {
    /// Lit des blocs jusqu'à la fin de l'élément ouvert (ou du texte).
    fn blocs(&mut self) -> Vec<Bloc> {
        let mut blocs = Vec::new();
        let mut en_cours: Vec<Enligne> = Vec::new();
        let vider = |en_cours: &mut Vec<Enligne>, blocs: &mut Vec<Bloc>| {
            if let Some(b) = paragraphe(std::mem::take(en_cours)) {
                blocs.push(b);
            }
        };
        if self.profondeur > PROFONDEUR_MAX {
            let texte = self.texte_jusqua_fin();
            if !texte.trim().is_empty() {
                blocs.push(Bloc::TexteBrut { texte });
            }
            return blocs;
        }
        self.profondeur += 1;
        while let Some(evenement) = self.evenements.next() {
            match evenement {
                Event::End(_) => break,
                Event::Start(Tag::Paragraph) => {
                    vider(&mut en_cours, &mut blocs);
                    let contenu = self.enlignes();
                    if let Some(b) = paragraphe(contenu) {
                        blocs.push(b);
                    }
                }
                Event::Start(Tag::Heading { level, .. }) => {
                    vider(&mut en_cours, &mut blocs);
                    let contenu = self.enlignes();
                    blocs.push(Bloc::Titre {
                        niveau: niveau_titre(level),
                        contenu,
                    });
                }
                Event::Start(Tag::BlockQuote(_)) => {
                    vider(&mut en_cours, &mut blocs);
                    let cites = self.blocs();
                    if est_note_pour_le_maitre(&cites) {
                        blocs.push(Bloc::NoteMaitre { blocs: cites });
                    } else {
                        blocs.push(Bloc::Citation { blocs: cites });
                    }
                }
                Event::Start(Tag::CodeBlock(genre)) => {
                    vider(&mut en_cours, &mut blocs);
                    let texte = self.texte_jusqua_fin();
                    let texte = match genre {
                        CodeBlockKind::Fenced(_) | CodeBlockKind::Indented => {
                            texte.trim_end_matches('\n').to_owned()
                        }
                    };
                    if !texte.trim().is_empty() {
                        blocs.push(Bloc::TexteBrut { texte });
                    }
                }
                Event::Start(Tag::HtmlBlock) => {
                    vider(&mut en_cours, &mut blocs);
                    let texte = self.texte_jusqua_fin();
                    let texte = texte.trim().to_owned();
                    if !texte.is_empty() {
                        blocs.push(Bloc::Paragraphe {
                            contenu: vec![Enligne::Texte { texte }],
                        });
                    }
                }
                Event::Start(Tag::List(debut)) => {
                    vider(&mut en_cours, &mut blocs);
                    let elements = self.elements_de_liste();
                    blocs.push(Bloc::Liste {
                        numerotee: debut.is_some(),
                        debut: debut.map(|d| u32::try_from(d).unwrap_or(u32::MAX)),
                        elements,
                    });
                }
                Event::Start(Tag::Table(alignements)) => {
                    vider(&mut en_cours, &mut blocs);
                    let (entete, lignes) = self.tableau();
                    blocs.push(Bloc::Tableau {
                        alignements: alignements.into_iter().map(alignement).collect(),
                        entete,
                        lignes,
                    });
                }
                Event::Start(
                    Tag::Item
                    | Tag::FootnoteDefinition(_)
                    | Tag::DefinitionList
                    | Tag::DefinitionListTitle
                    | Tag::DefinitionListDefinition
                    | Tag::MetadataBlock(_)
                    | Tag::TableHead
                    | Tag::TableRow
                    | Tag::TableCell,
                ) => {
                    vider(&mut en_cours, &mut blocs);
                    blocs.extend(self.blocs());
                }
                Event::Rule => {
                    vider(&mut en_cours, &mut blocs);
                    blocs.push(Bloc::Filet);
                }
                autre => self.enligne(autre, &mut en_cours),
            }
        }
        vider(&mut en_cours, &mut blocs);
        self.profondeur -= 1;
        blocs
    }

    fn elements_de_liste(&mut self) -> Vec<Vec<Bloc>> {
        let mut elements = Vec::new();
        while let Some(evenement) = self.evenements.next() {
            match evenement {
                Event::End(_) => break,
                Event::Start(Tag::Item) => elements.push(self.blocs()),
                _ => {}
            }
        }
        elements
    }

    #[allow(clippy::type_complexity)]
    fn tableau(&mut self) -> (Vec<Vec<Enligne>>, Vec<Vec<Vec<Enligne>>>) {
        let mut entete = Vec::new();
        let mut lignes = Vec::new();
        while let Some(evenement) = self.evenements.next() {
            match evenement {
                Event::End(_) => break,
                Event::Start(Tag::TableHead) => entete = self.cellules(),
                Event::Start(Tag::TableRow) => lignes.push(self.cellules()),
                _ => {}
            }
        }
        (entete, lignes)
    }

    fn cellules(&mut self) -> Vec<Vec<Enligne>> {
        let mut cellules = Vec::new();
        while let Some(evenement) = self.evenements.next() {
            match evenement {
                Event::End(_) => break,
                Event::Start(Tag::TableCell) => cellules.push(self.enlignes()),
                _ => {}
            }
        }
        cellules
    }

    /// Lit des éléments de texte jusqu'à la fin de l'élément ouvert.
    fn enlignes(&mut self) -> Vec<Enligne> {
        let mut enlignes = Vec::new();
        if self.profondeur > PROFONDEUR_MAX {
            pousser(
                &mut enlignes,
                Enligne::Texte {
                    texte: self.texte_jusqua_fin(),
                },
            );
            return enlignes;
        }
        self.profondeur += 1;
        while let Some(evenement) = self.evenements.next() {
            if let Event::End(_) = evenement {
                break;
            }
            self.enligne(evenement, &mut enlignes);
        }
        self.profondeur -= 1;
        enlignes
    }

    /// Traite un événement de texte.
    fn enligne(&mut self, evenement: Event<'_>, enlignes: &mut Vec<Enligne>) {
        match evenement {
            Event::Text(t) | Event::Html(t) | Event::InlineHtml(t) => pousser(
                enlignes,
                Enligne::Texte {
                    texte: t.into_string(),
                },
            ),
            Event::Code(t) => enlignes.push(Enligne::Code {
                texte: t.into_string(),
            }),
            Event::InlineMath(t) => enlignes.push(Enligne::Formule {
                latex: t.into_string(),
                seule: false,
            }),
            Event::DisplayMath(t) => enlignes.push(Enligne::Formule {
                latex: t.into_string(),
                seule: true,
            }),
            Event::SoftBreak => pousser(
                enlignes,
                Enligne::Texte {
                    texte: " ".to_owned(),
                },
            ),
            Event::HardBreak => enlignes.push(Enligne::Saut),
            Event::FootnoteReference(t) => pousser(
                enlignes,
                Enligne::Texte {
                    texte: format!("[{t}]"),
                },
            ),
            Event::TaskListMarker(coche) => pousser(
                enlignes,
                Enligne::Texte {
                    texte: if coche { "☒ " } else { "☐ " }.to_owned(),
                },
            ),
            Event::Start(Tag::Emphasis) => {
                let contenu = self.enlignes();
                if !contenu.is_empty() {
                    enlignes.push(Enligne::Italique { contenu });
                }
            }
            Event::Start(Tag::Strong) => {
                let contenu = self.enlignes();
                if !contenu.is_empty() {
                    enlignes.push(Enligne::Gras { contenu });
                }
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                let description = texte_de(&self.enlignes()).trim().to_owned();
                match appel_image(&dest_url, description.clone()) {
                    Some(element) => enlignes.push(element),
                    None => pousser(enlignes, Enligne::Texte { texte: description }),
                }
            }
            // Un lien garde son texte et perd sa cible ; barré, exposant, indice et tout autre
            // élément gardent leur texte.
            Event::Start(_) => {
                for e in self.enlignes() {
                    pousser(enlignes, e);
                }
            }
            Event::End(_) | Event::Rule => {}
        }
    }

    /// Texte brut jusqu'à la fin de l'élément ouvert (blocs de code, HTML).
    fn texte_jusqua_fin(&mut self) -> String {
        let mut texte = String::new();
        let mut ouverts = 0_usize;
        for evenement in self.evenements.by_ref() {
            match evenement {
                Event::Start(_) => ouverts += 1,
                Event::End(_) => {
                    if ouverts == 0 {
                        break;
                    }
                    ouverts -= 1;
                }
                Event::Text(t)
                | Event::Code(t)
                | Event::Html(t)
                | Event::InlineHtml(t)
                | Event::InlineMath(t)
                | Event::DisplayMath(t) => texte.push_str(&t),
                Event::SoftBreak | Event::HardBreak => texte.push('\n'),
                _ => {}
            }
        }
        texte
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sans_les_corriges_retire_la_section_des_corriges_et_les_reponses_du_quiz() {
        let lecon = analyser(
            "## 5. Exercices\n\nExercice 1.\n\n## 6. Corrigés\n\nRéponse 1.\n\n### Exercice 1\n\nDétail.\n\n## 7. Quiz\n\nQuestion 1.\n\n### Réponses\n\n1. b\n\n## 8. Pour aller plus loin\n\nSuite.",
        );
        let texte = lecon.sans_les_corriges().texte_brut();
        assert!(texte.contains("Exercice 1."));
        assert!(texte.contains("Question 1."));
        assert!(texte.contains("Suite."));
        assert!(!texte.contains("Réponse 1."));
        assert!(!texte.contains("Détail."));
        assert!(!texte.contains("Réponses"));
        assert!(!texte.contains("Corrigés"));
    }

    #[test]
    fn une_lecon_sans_corriges_reste_entiere() {
        let lecon = analyser("## L'essentiel\n\nTexte.\n\n### Réponses\n\nHors du quiz.");
        assert_eq!(lecon.sans_les_corriges(), lecon);
    }

    fn texte(t: &str) -> Enligne {
        Enligne::Texte {
            texte: t.to_owned(),
        }
    }

    #[test]
    fn titres_paragraphes_et_mise_en_valeur() {
        let lecon = analyser("# Titre\n\nUn **mot** et un *autre*.\n\n#### Bas\n");
        assert_eq!(
            lecon.blocs,
            vec![
                Bloc::Titre {
                    niveau: 1,
                    contenu: vec![texte("Titre")]
                },
                Bloc::Paragraphe {
                    contenu: vec![
                        texte("Un "),
                        Enligne::Gras {
                            contenu: vec![texte("mot")]
                        },
                        texte(" et un "),
                        Enligne::Italique {
                            contenu: vec![texte("autre")]
                        },
                        texte("."),
                    ]
                },
                Bloc::Titre {
                    niveau: 3,
                    contenu: vec![texte("Bas")]
                },
            ]
        );
    }

    #[test]
    fn formules_dans_la_ligne_et_seules() {
        let lecon = analyser("Soit $x^2$ un carré.\n\n$$\\frac{a}{b}$$\n");
        assert_eq!(
            lecon.blocs,
            vec![
                Bloc::Paragraphe {
                    contenu: vec![
                        texte("Soit "),
                        Enligne::Formule {
                            latex: "x^2".to_owned(),
                            seule: false
                        },
                        texte(" un carré."),
                    ]
                },
                Bloc::Formule {
                    latex: "\\frac{a}{b}".to_owned()
                },
            ]
        );
        assert_eq!(lecon.formules().len(), 2);
    }

    #[test]
    fn le_html_reste_du_texte_et_les_liens_perdent_leur_cible() {
        let lecon = analyser(
            "<script>alert(1)</script>\n\nVoir [la leçon](https://exemple.org) et <b>gras</b>.\n",
        );
        let brut = lecon.texte_brut();
        assert!(brut.contains("<script>alert(1)</script>"));
        assert!(brut.contains("Voir la leçon et <b>gras</b>."));
        assert!(!brut.contains("exemple.org"));
    }

    #[test]
    fn un_son_de_l_ecole_s_appelle_par_son_empreinte() {
        let empreinte = "b".repeat(64);
        let lecon = analyser(&format!(
            "Écoutez : ![Prononciation](son:{empreinte}) ![Faux](son:abc)\n"
        ));
        assert_eq!(lecon.sons(), vec![empreinte.as_str()]);
        let brut = lecon.texte_brut();
        assert!(brut.contains("Prononciation") && brut.contains("Faux"));
        assert!(!brut.contains("son:"));
    }

    #[test]
    fn images_du_paquet_de_l_ecole_et_videos() {
        let empreinte = "a".repeat(64);
        let lecon = analyser(&format!(
            "![Un triangle](triangle-1.svg) ![Photo](image:{empreinte}) \
             ![Expérience](video:{empreinte}) ![Ailleurs](../../x.svg) \
             ![Web](https://exemple.org/a.png) ![Disque](C:\\a.png)\n"
        ));
        assert_eq!(
            lecon.images(),
            vec![
                &SourceImage::Paquet {
                    nom: "triangle-1.svg".to_owned()
                },
                &SourceImage::Ecole {
                    empreinte: empreinte.clone()
                },
            ]
        );
        assert_eq!(lecon.videos(), vec![empreinte.as_str()]);
        assert!(lecon.sons().is_empty());
        let brut = lecon.texte_brut();
        assert!(brut.contains("Ailleurs") && brut.contains("Web") && brut.contains("Disque"));
        assert!(!brut.contains("exemple.org") && !brut.contains("C:"));
    }

    #[test]
    fn une_demi_droite_dans_la_description_ne_coupe_pas_l_image() {
        let lecon = analyser(
            "![La droite (AB), le segment [AB] et la demi-droite [AB)](figure-1.svg)\n\n\
             ![Les demi-droites [Ox), [Oz) : 60 degrés](figure-2.svg) et [un lien](x)\n\n\
             ![Le milieu de [AB]](figure-3.svg)\n",
        );
        assert_eq!(lecon.images().len(), 3);
        let brut = lecon.texte_brut();
        assert!(brut.contains("la demi-droite [AB)"), "{brut}");
        assert!(brut.contains("[Ox), [Oz)"), "{brut}");
        assert!(!brut.contains('\\'), "{brut}");
        // Un texte sans appel d'image n'est pas touché.
        assert!(matches!(
            proteger_descriptions("Le segment [AB) seul."),
            std::borrow::Cow::Borrowed(_)
        ));
    }

    #[test]
    fn listes_et_tableaux() {
        let lecon = analyser(
            "3. trois\n4. quatre $y$\n\n| A | B |\n|:--|--:|\n| 1 | $2$ |\n\n- un\n- deux\n",
        );
        let [
            Bloc::Liste {
                numerotee: true,
                debut: Some(3),
                elements,
            },
            Bloc::Tableau {
                alignements,
                entete,
                lignes,
            },
            Bloc::Liste {
                numerotee: false,
                elements: puces,
                ..
            },
        ] = lecon.blocs.as_slice()
        else {
            unreachable!("blocs inattendus : {:?}", lecon.blocs);
        };
        assert_eq!(elements.len(), 2);
        assert_eq!(alignements, &vec![Alignement::Gauche, Alignement::Droite]);
        assert_eq!(entete.len(), 2);
        assert_eq!(lignes.len(), 1);
        assert_eq!(puces.len(), 2);
        assert_eq!(lecon.formules().len(), 2);
    }

    #[test]
    fn note_pour_le_maitre_retiree_pour_les_eleves() {
        let lecon = analyser(
            "> **Note pour le maître** : Image à ajouter ici : une pirogue.\n\n> Une citation.\n",
        );
        assert!(matches!(lecon.blocs.first(), Some(Bloc::NoteMaitre { .. })));
        assert!(matches!(lecon.blocs.get(1), Some(Bloc::Citation { .. })));
        let eleves = lecon.pour_les_eleves();
        assert_eq!(eleves.blocs.len(), 1);
        assert!(!eleves.texte_brut().contains("pirogue"));
    }

    #[test]
    fn un_texte_tres_imbrique_ne_fait_pas_deborder_la_pile() {
        let texte = format!("{}fin\n", "> ".repeat(500));
        let lecon = analyser(&texte);
        assert!(lecon.texte_brut().contains("fin"));
    }
}
