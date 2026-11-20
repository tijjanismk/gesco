//! Quiz des leçons (ADR « format des quiz », SPEC_PHASE_12_LOT_3 §3) : une convention écrite dans
//! le texte, reconnue ici. Une question commence par « **Question n.** », ses choix sont des
//! morceaux de ligne « A) … B) … », la sous-section « Réponses » donne « n. **B.** » puis
//! l'explication. Ce qui n'est pas reconnu reste du texte de la leçon : la question n'entre pas
//! dans le quiz, et l'auteur seul lit pourquoi. L'analyse ne refuse jamais un texte.

use serde::{Deserialize, Serialize};

use super::lecon::{Bloc, Enligne, analyser};

/// Choix d'une question, au plus (A à F).
pub const CHOIX_MAX: usize = 6;

/// Questions d'une révision de chapitre, au plus.
pub const QUESTIONS_REVISION: usize = 10;

/// Lettres des choix.
const LETTRES: [char; CHOIX_MAX] = ['A', 'B', 'C', 'D', 'E', 'F'];

/// Sorte d'une question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum SorteQuestion {
    /// Une seule bonne réponse parmi les choix.
    ChoixUnique,
    /// Plusieurs bonnes réponses ; le point seulement si toutes, et elles seules.
    ChoixMultiples,
    /// Deux choix « Vrai » et « Faux ».
    VraiFaux,
    /// Pas de choix : une réponse écrite.
    ReponseCourte,
    /// Choix « A) … » à relier à des éléments « 1) … ».
    Appariement,
}

/// Un choix d'une question (« A) … » ou, pour un appariement, « 1) … »).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ChoixQuestion {
    /// Repère : « A » à « F », ou « 1 », « 2 »…
    pub repere: String,
    /// Texte du choix.
    pub texte: Vec<Enligne>,
}

/// Une question reconnue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct QuestionQuiz {
    /// Numéro écrit dans le texte.
    pub numero: u32,
    /// Sorte.
    pub sorte: SorteQuestion,
    /// Énoncé (formules et images comprises).
    pub enonce: Vec<Bloc>,
    /// Choix « A) … ».
    pub choix: Vec<ChoixQuestion>,
    /// Éléments « 1) … » d'un appariement.
    pub elements: Vec<ChoixQuestion>,
    /// Bonnes réponses : lettres (« B »), paires (« A-2 ») ou réponses écrites admises.
    pub bonnes: Vec<String>,
    /// Explication de la réponse.
    pub explication: Vec<Bloc>,
}

/// Une question qui ne sera pas proposée en quiz, et pourquoi (pour l'auteur seul).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct QuestionEcartee {
    /// Numéro écrit dans le texte.
    pub numero: u32,
    /// Raison, en une phrase.
    pub raison: String,
}

/// Le quiz d'un contenu.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, specta::Type)]
pub struct Quiz {
    /// Questions reconnues, dans l'ordre du texte.
    pub questions: Vec<QuestionQuiz>,
    /// Questions écrites mais pas reconnues.
    pub ecartees: Vec<QuestionEcartee>,
}

impl Quiz {
    /// Vrai si aucune question n'est écrite.
    pub fn est_vide(&self) -> bool {
        self.questions.is_empty() && self.ecartees.is_empty()
    }
}

/// Lit le quiz d'une leçon : la section dont le titre est « Quiz » (numéro et accents ignorés).
/// `tout_le_texte` : contenu de nature « quiz », dont tout le texte suit la convention.
pub fn analyser_quiz(texte: &str, tout_le_texte: bool) -> Quiz {
    let lignes: Vec<&str> = texte.lines().collect();
    let section: &[&str] = if tout_le_texte {
        &lignes
    } else {
        match section_quiz(&lignes) {
            Some(s) => s,
            None => return Quiz::default(),
        }
    };
    let (questions_brutes, reponses_brutes) = separer(section);
    let reponses = lire_reponses(&reponses_brutes);
    let mut quiz = Quiz::default();
    for (numero, lignes_question) in questions_brutes {
        match reconnaitre(
            numero,
            &lignes_question,
            reponses.iter().find(|r| r.0 == numero),
        ) {
            Ok(q) => quiz.questions.push(q),
            Err(raison) => quiz.ecartees.push(QuestionEcartee {
                numero,
                raison: raison.to_owned(),
            }),
        }
    }
    quiz
}

/// Niveau d'un titre Markdown (« ## … » : 2), s'il en est un.
fn niveau_titre(ligne: &str) -> Option<(usize, &str)> {
    let l = ligne.trim_start();
    let dieses = l.bytes().take_while(|o| *o == b'#').count();
    if !(1..=6).contains(&dieses) {
        return None;
    }
    let reste = l.get(dieses..)?;
    if !reste.is_empty() && !reste.starts_with(' ') {
        return None;
    }
    Some((dieses, reste.trim()))
}

/// Titre ramené à sa forme simple : sans numéro en tête, sans accents, en minuscules.
fn titre_simple(titre: &str) -> String {
    simplifier(
        titre
            .trim()
            .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ')')
            .trim(),
    )
}

/// Sans accents et en minuscules.
fn simplifier(texte: &str) -> String {
    texte
        .chars()
        .map(|c| match c {
            'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => 'e',
            'à' | 'â' | 'ä' | 'À' | 'Â' | 'Ä' => 'a',
            'î' | 'ï' | 'Î' | 'Ï' => 'i',
            'ô' | 'ö' | 'Ô' | 'Ö' => 'o',
            'û' | 'ù' | 'ü' | 'Û' | 'Ù' | 'Ü' => 'u',
            'ç' | 'Ç' => 'c',
            'œ' | 'Œ' => 'o',
            autre => autre.to_lowercase().next().unwrap_or(autre),
        })
        .collect()
}

/// Lignes de la section « Quiz », titre exclu, jusqu'au titre suivant de même niveau ou plus haut.
fn section_quiz<'a>(lignes: &'a [&'a str]) -> Option<&'a [&'a str]> {
    let mut debut = None;
    let mut niveau_quiz = 0;
    let mut dans_code = false;
    for (i, ligne) in lignes.iter().enumerate() {
        if ligne.trim_start().starts_with("```") {
            dans_code = !dans_code;
            continue;
        }
        if dans_code {
            continue;
        }
        let Some((niveau, titre)) = niveau_titre(ligne) else {
            continue;
        };
        match debut {
            None => {
                if titre_simple(titre).starts_with("quiz") {
                    debut = Some(i.saturating_add(1));
                    niveau_quiz = niveau;
                }
            }
            Some(d) => {
                if niveau <= niveau_quiz {
                    return lignes.get(d..i);
                }
            }
        }
    }
    debut.and_then(|d| lignes.get(d..))
}

/// Numéro d'une ligne « **Question n.** … », et le reste de la ligne.
fn tete_de_question(ligne: &str) -> Option<(u32, &str)> {
    let reste = ligne.trim_start().strip_prefix("**")?;
    let reste = reste
        .strip_prefix("Question")
        .or_else(|| reste.strip_prefix("question"))?;
    let reste = reste.trim_start();
    let chiffres = reste.bytes().take_while(u8::is_ascii_digit).count();
    let numero: u32 = reste.get(..chiffres)?.parse().ok()?;
    let reste = reste.get(chiffres..)?.trim_start();
    // « 1.** », « 1 :** », « 1** ».
    let reste = reste.trim_start_matches(['.', ':', ')']).trim_start();
    let reste = reste.strip_prefix("**")?;
    Some((numero, reste.trim()))
}

type Brutes<'a> = Vec<(u32, Vec<&'a str>)>;

/// Sépare les questions (lignes sous chaque « **Question n.** ») de la sous-section « Réponses ».
fn separer<'a>(section: &[&'a str]) -> (Brutes<'a>, Vec<&'a str>) {
    let mut questions: Brutes<'a> = Vec::new();
    let mut reponses = Vec::new();
    let mut dans_reponses = false;
    for ligne in section {
        if let Some((_, titre)) = niveau_titre(ligne) {
            dans_reponses = titre_simple(titre).starts_with("reponse");
            continue;
        }
        if dans_reponses {
            reponses.push(*ligne);
            continue;
        }
        if let Some((numero, reste)) = tete_de_question(ligne) {
            questions.push((numero, vec![reste]));
        } else if let Some((_, lignes)) = questions.last_mut() {
            lignes.push(ligne);
        }
    }
    (questions, reponses)
}

/// Une réponse lue : numéro, ce qui est en gras, explication.
type ReponseLue = (u32, String, String);

/// Lit « n. **B.** explication » ; une explication peut continuer sur les lignes suivantes.
fn lire_reponses(lignes: &[&str]) -> Vec<ReponseLue> {
    let mut reponses: Vec<ReponseLue> = Vec::new();
    for ligne in lignes {
        let l = ligne.trim();
        if let Some(r) = ligne_de_reponse(l) {
            reponses.push(r);
        } else if !l.is_empty()
            && let Some((_, _, explication)) = reponses.last_mut()
        {
            explication.push(' ');
            explication.push_str(l);
        }
    }
    reponses
}

fn ligne_de_reponse(l: &str) -> Option<ReponseLue> {
    let chiffres = l.bytes().take_while(u8::is_ascii_digit).count();
    let numero: u32 = l.get(..chiffres)?.parse().ok()?;
    let reste = l.get(chiffres..)?;
    let reste = reste
        .strip_prefix('.')
        .or_else(|| reste.strip_prefix(')'))?;
    let reste = reste.trim_start().strip_prefix("**")?;
    let fin = reste.find("**")?;
    let gras = reste.get(..fin)?.trim().to_owned();
    let explication = reste.get(fin.saturating_add(2)..)?.trim().to_owned();
    Some((numero, gras, explication))
}

/// Découpe une ligne en choix « X) … » dont les repères se suivent (A, B, C… ou 1, 2, 3…), en
/// commençant par `premier`. Le premier repère est en tête de ligne ; les suivants après une
/// espace. Rend le texte avant le premier repère et les choix.
fn decouper_choix(texte: &str, reperes: &[String]) -> Option<Vec<(String, String)>> {
    let premier = reperes.first()?;
    let debut = format!("{premier}) ");
    let mut reste = texte.trim_start().strip_prefix(&debut)?;
    let mut choix = Vec::new();
    let mut courant = premier.clone();
    for suivant in reperes.iter().skip(1) {
        let marque = format!(" {suivant}) ");
        match reste.find(&marque) {
            Some(i) => {
                let (avant, apres) = reste.split_at(i);
                choix.push((courant, avant.trim().to_owned()));
                reste = apres.get(marque.len()..).unwrap_or("");
                courant = suivant.clone();
            }
            None => break,
        }
    }
    choix.push((courant, reste.trim().to_owned()));
    Some(choix)
}

fn reperes_lettres() -> Vec<String> {
    LETTRES.iter().map(char::to_string).collect()
}

fn reperes_chiffres() -> Vec<String> {
    (1..=CHOIX_MAX).map(|n| n.to_string()).collect()
}

/// Texte d'un choix en éléments de ligne.
fn en_ligne(texte: &str) -> Vec<Enligne> {
    let lecon = analyser(texte);
    match lecon.blocs.into_iter().next() {
        Some(Bloc::Paragraphe { contenu }) => contenu,
        Some(Bloc::Formule { latex }) => vec![Enligne::Formule { latex, seule: true }],
        Some(_) | None => vec![Enligne::Texte {
            texte: texte.to_owned(),
        }],
    }
}

/// Reconnaît une question ; sinon la raison, pour l'auteur.
fn reconnaitre(
    numero: u32,
    lignes: &[&str],
    reponse: Option<&ReponseLue>,
) -> Result<QuestionQuiz, &'static str> {
    // L'énoncé : les lignes jusqu'à la première ligne de choix « A) ». Les choix : cette ligne
    // et les suivantes jusqu'à une ligne vide ; une ligne qui ne commence pas par un repère
    // continue le choix précédent.
    let mut enonce: Vec<&str> = Vec::new();
    let mut zone_choix: Vec<String> = Vec::new();
    let mut zone_elements: Vec<String> = Vec::new();
    let mut dans_choix = false;
    let mut dans_elements = false;
    for ligne in lignes {
        let l = ligne.trim();
        let commence_lettre = LETTRES.iter().any(|c| l.starts_with(&format!("{c}) ")));
        let commence_chiffre = (1..=CHOIX_MAX).any(|n| l.starts_with(&format!("{n}) ")));
        if commence_lettre && !dans_elements {
            dans_choix = true;
            zone_choix.push(l.to_owned());
        } else if commence_chiffre && (dans_choix || dans_elements) {
            dans_choix = false;
            dans_elements = true;
            zone_elements.push(l.to_owned());
        } else if l.is_empty() {
            if dans_choix || dans_elements {
                // Une ligne vide après les choix : la suite n'est plus la question.
                dans_choix = false;
                dans_elements = false;
            } else {
                enonce.push(ligne);
            }
        } else if dans_elements {
            if let Some(dernier) = zone_elements.last_mut() {
                dernier.push(' ');
                dernier.push_str(l);
            }
        } else if dans_choix {
            if let Some(dernier) = zone_choix.last_mut() {
                dernier.push(' ');
                dernier.push_str(l);
            }
        } else if zone_choix.is_empty() {
            enonce.push(ligne);
        }
    }
    let texte_enonce = enonce.join("\n");
    if texte_enonce.trim().is_empty() {
        return Err("Cette question ne sera pas proposée en quiz : l'énoncé manque.");
    }
    let Some((_, gras, explication)) = reponse else {
        return Err("Cette question ne sera pas proposée en quiz : la réponse manque.");
    };
    let choix = lire_zone(&zone_choix, &reperes_lettres())?;
    let elements = lire_zone(&zone_elements, &reperes_chiffres())?;
    let gras = gras.trim().trim_end_matches('.').trim();
    let (sorte, bonnes) = if choix.is_empty() {
        let admises = reponses_ecrites(gras);
        if admises.is_empty() {
            return Err("Cette question ne sera pas proposée en quiz : la réponse manque.");
        }
        (SorteQuestion::ReponseCourte, admises)
    } else if !elements.is_empty() {
        let mut paires = Vec::new();
        for morceau in gras.split(',') {
            let morceau = morceau.trim();
            let Some((l, n)) = morceau.split_once('-') else {
                return Err(
                    "Cette question ne sera pas proposée en quiz : la réponse n'est pas de la forme « A-2, B-1 ».",
                );
            };
            let (l, n) = (l.trim().to_uppercase(), n.trim().to_owned());
            if !choix.iter().any(|c| c.repere == l) || !elements.iter().any(|e| e.repere == n) {
                return Err(
                    "Cette question ne sera pas proposée en quiz : la réponse n'est pas parmi les choix.",
                );
            }
            paires.push(format!("{l}-{n}"));
        }
        if paires.len() != choix.len() {
            return Err(
                "Cette question ne sera pas proposée en quiz : chaque choix doit être relié à un élément.",
            );
        }
        paires.sort();
        (SorteQuestion::Appariement, paires)
    } else {
        let mut lettres = Vec::new();
        for morceau in gras.split(',') {
            let l = morceau.trim().to_uppercase();
            if !choix.iter().any(|c| c.repere == l) {
                return Err(
                    "Cette question ne sera pas proposée en quiz : la réponse n'est pas parmi les choix.",
                );
            }
            if !lettres.contains(&l) {
                lettres.push(l);
            }
        }
        lettres.sort();
        let vrai_faux = choix.len() == 2
            && choix
                .iter()
                .map(|c| simplifier(&texte_simple(&c.texte)))
                .eq(["vrai", "faux"]);
        let sorte = if vrai_faux {
            SorteQuestion::VraiFaux
        } else if lettres.len() > 1 {
            SorteQuestion::ChoixMultiples
        } else {
            SorteQuestion::ChoixUnique
        };
        (sorte, lettres)
    };
    if choix.len() == 1 {
        return Err("Cette question ne sera pas proposée en quiz : il faut au moins deux choix.");
    }
    Ok(QuestionQuiz {
        numero,
        sorte,
        enonce: analyser(&texte_enonce).pour_les_eleves().blocs,
        choix,
        elements,
        bonnes,
        explication: analyser(explication).pour_les_eleves().blocs,
    })
}

/// Choix d'une zone (lignes de choix déjà jointes).
fn lire_zone(zone: &[String], reperes: &[String]) -> Result<Vec<ChoixQuestion>, &'static str> {
    if zone.is_empty() {
        return Ok(Vec::new());
    }
    let texte = zone.join(" ");
    let Some(lus) = decouper_choix(&texte, reperes) else {
        return Ok(Vec::new());
    };
    if lus.iter().any(|(_, t)| t.is_empty()) {
        return Err("Cette question ne sera pas proposée en quiz : un choix est vide.");
    }
    Ok(lus
        .into_iter()
        .map(|(repere, texte)| ChoixQuestion {
            repere,
            texte: en_ligne(&texte),
        })
        .collect())
}

/// Réponses écrites admises : chaque texte entre « » ; sans guillemets, tout le texte.
fn reponses_ecrites(gras: &str) -> Vec<String> {
    let mut admises = Vec::new();
    let mut reste = gras;
    while let Some(i) = reste.find('«') {
        let apres = reste.get(i.saturating_add('«'.len_utf8())..).unwrap_or("");
        let Some(j) = apres.find('»') else {
            break;
        };
        let r = apres.get(..j).unwrap_or("").trim();
        if !r.is_empty() {
            admises.push(r.to_owned());
        }
        reste = apres.get(j.saturating_add('»'.len_utf8())..).unwrap_or("");
    }
    if admises.is_empty() && !gras.trim().is_empty() {
        admises.push(gras.trim().to_owned());
    }
    admises
}

/// Texte simple d'éléments de ligne.
fn texte_simple(enlignes: &[Enligne]) -> String {
    let mut t = String::new();
    for e in enlignes {
        match e {
            Enligne::Texte { texte } | Enligne::Code { texte } => t.push_str(texte),
            Enligne::Gras { contenu } | Enligne::Italique { contenu } => {
                t.push_str(&texte_simple(contenu));
            }
            Enligne::Formule { latex, .. } => t.push_str(latex),
            _ => {}
        }
    }
    t.trim().to_owned()
}

// ---------------------------------------------------------------------------------------------
// Corriger
// ---------------------------------------------------------------------------------------------

/// Réponse ramenée à sa forme de comparaison : sans accents, en minuscules, sans espaces, sans
/// point final ; « 3,5 » et « 3.5 » deviennent le même nombre.
pub fn normaliser_reponse(texte: &str) -> String {
    let simple: String = simplifier(texte.trim())
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '\u{a0}' && *c != '\u{202f}')
        .collect();
    let simple = simple.trim_end_matches('.').to_owned();
    let nombre = simple.replace(',', ".");
    match nombre.parse::<f64>() {
        Ok(n) if n.is_finite() => {
            // Écriture stable d'un nombre : « 3.50 » et « 3.5 » se valent.
            let mut s = format!("{n}");
            if s.contains('.') {
                s = s.trim_end_matches('0').trim_end_matches('.').to_owned();
            }
            s
        }
        _ => simple,
    }
}

/// Lettres ou paires d'une réponse donnée, rangées : « c, a » → [« A », « C »].
fn morceaux(reponse: &str) -> Vec<String> {
    let mut m: Vec<String> = reponse
        .split([',', ';', ' '])
        .map(|x| x.trim().to_uppercase())
        .filter(|x| !x.is_empty())
        .collect();
    m.sort();
    m.dedup();
    m
}

/// Vrai si la réponse donnée est juste. Aucune réponse : faux. Plusieurs bonnes réponses ou un
/// appariement : juste seulement si tout est juste, et rien de plus.
pub fn corriger(question: &QuestionQuiz, reponse: &str) -> bool {
    if reponse.trim().is_empty() {
        return false;
    }
    match question.sorte {
        SorteQuestion::ReponseCourte => {
            let donnee = normaliser_reponse(reponse);
            question
                .bonnes
                .iter()
                .any(|b| normaliser_reponse(b) == donnee)
        }
        SorteQuestion::ChoixUnique
        | SorteQuestion::VraiFaux
        | SorteQuestion::ChoixMultiples
        | SorteQuestion::Appariement => morceaux(reponse) == question.bonnes,
    }
}

/// Score d'une suite de réponses : (justes, sur).
pub fn score<'a>(questions: impl IntoIterator<Item = (&'a QuestionQuiz, &'a str)>) -> (u32, u32) {
    let mut justes = 0_u32;
    let mut sur = 0_u32;
    for (q, r) in questions {
        sur = sur.saturating_add(1);
        if corriger(q, r) {
            justes = justes.saturating_add(1);
        }
    }
    (justes, sur)
}

// ---------------------------------------------------------------------------------------------
// Écrire
// ---------------------------------------------------------------------------------------------

/// Plus grand numéro de question écrit dans le texte.
fn dernier_numero(texte: &str) -> u32 {
    texte
        .lines()
        .filter_map(|l| tete_de_question(l).map(|(n, _)| n))
        .max()
        .unwrap_or(0)
}

/// Gabarit d'une question, à insérer à l'endroit du curseur, et la ligne de réponse à ajouter
/// sous « Réponses ». Le numéro suit le dernier du texte.
pub fn gabarit_question(texte: &str) -> (String, String) {
    let n = dernier_numero(texte).saturating_add(1);
    (
        format!(
            "**Question {n}.** Énoncé de la question :\nA) premier choix  B) deuxième choix  C) troisième choix  D) quatrième choix\n"
        ),
        format!("{n}. **A.** Explication : d'où vient l'erreur de qui a choisi une autre réponse."),
    )
}

/// Ajoute une question au texte : le gabarit avant la sous-section « Réponses » du quiz, la ligne
/// de réponse à la fin de celle-ci ; crée la section « Quiz » ou la sous-section « Réponses » si
/// elles manquent. `tout_le_texte` : contenu de nature « quiz ».
pub fn ajouter_question(texte: &str, tout_le_texte: bool, question: &str, reponse: &str) -> String {
    let mut lignes: Vec<String> = texte.lines().map(str::to_owned).collect();
    // Bornes de la section du quiz.
    let (debut, fin, niveau) = if tout_le_texte {
        (0, lignes.len(), 1)
    } else {
        let mut debut = None;
        let mut niveau = 2;
        let mut fin = lignes.len();
        for (i, l) in lignes.iter().enumerate() {
            if let Some((n, titre)) = niveau_titre(l) {
                match debut {
                    None if titre_simple(titre).starts_with("quiz") => {
                        debut = Some(i.saturating_add(1));
                        niveau = n;
                    }
                    Some(_) if n <= niveau => {
                        fin = i;
                        break;
                    }
                    _ => {}
                }
            }
        }
        match debut {
            Some(d) => (d, fin, niveau),
            None => {
                if lignes.last().is_some_and(|l| !l.trim().is_empty()) {
                    lignes.push(String::new());
                }
                lignes.push("## Quiz".to_owned());
                lignes.push(String::new());
                (lignes.len(), lignes.len(), 2)
            }
        }
    };
    let titre_reponses = (debut..fin).find(|i| {
        lignes
            .get(*i)
            .and_then(|l| niveau_titre(l))
            .is_some_and(|(_, t)| titre_simple(t).starts_with("reponse"))
    });
    let mut q: Vec<String> = question.lines().map(str::to_owned).collect();
    match titre_reponses {
        Some(r) => {
            // Réponse à la fin de la sous-section, question juste avant son titre.
            let mut fin_reponses = fin;
            while fin_reponses > r.saturating_add(1)
                && lignes
                    .get(fin_reponses.saturating_sub(1))
                    .is_some_and(|l| l.trim().is_empty())
            {
                fin_reponses = fin_reponses.saturating_sub(1);
            }
            lignes.insert(fin_reponses, reponse.to_owned());
            if fin_reponses == r.saturating_add(1) {
                lignes.insert(fin_reponses, String::new());
            }
            let mut place = r;
            while place > debut
                && lignes
                    .get(place.saturating_sub(1))
                    .is_some_and(|l| l.trim().is_empty())
            {
                place = place.saturating_sub(1);
            }
            q.insert(0, String::new());
            q.push(String::new());
            for (k, l) in q.into_iter().enumerate() {
                lignes.insert(place.saturating_add(k), l);
            }
        }
        None => {
            let mut place = fin;
            while place > debut
                && lignes
                    .get(place.saturating_sub(1))
                    .is_some_and(|l| l.trim().is_empty())
            {
                place = place.saturating_sub(1);
            }
            let titre = format!("{} Réponses", "#".repeat(niveau.saturating_add(1).min(6)));
            let mut ajout = vec![String::new()];
            ajout.append(&mut q);
            ajout.extend([
                String::new(),
                titre,
                String::new(),
                reponse.to_owned(),
                String::new(),
            ]);
            for (k, l) in ajout.into_iter().enumerate() {
                lignes.insert(place.saturating_add(k), l);
            }
        }
    }
    let mut sortie = lignes.join("\n");
    if texte.ends_with('\n') || sortie.is_empty() {
        sortie.push('\n');
    }
    sortie
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    const RACINE: &str = "## 6. Corrigés\n\nRien.\n\n## 7. Quiz\n\n\
        **Question 1.** `√(16 + 9)` est égal à :\nA) 7  B) 5  C) 25  D) 12\n\n\
        **Question 2.** Pour tout nombre a, $\\sqrt{a^2} = a$.\nA) Vrai  B) Faux\n\n\
        **Question 3.** Combien font 7 / 2 ?\n\n\
        **Question 4.** Quels nombres sont pairs ?\nA) 2  B) 3  C) 4\n\n\
        **Question 5.** Relie chaque nombre à son carré.\nA) 2  B) 3\n1) 9  2) 4\n\n\
        ### Réponses\n\n\
        1. **B.** √25 = 5. La réponse 7 vient de √16 + √9.\n\
        2. **B.** Pour a = −3, √9 = 3.\n\
        3. **« 3,5 » ou « 7/2 ».** On partage 7 en deux.\n\
        4. **A, C.** 2 et 4 se divisent par 2.\n\
        5. **A-2, B-1.** 2 × 2 = 4 ;\n   3 × 3 = 9.\n\n\
        ## 8. Pour aller plus loin\n\nSuite.\n";

    #[test]
    fn les_cinq_sortes_sont_reconnues() {
        let quiz = analyser_quiz(RACINE, false);
        assert!(quiz.ecartees.is_empty(), "{:?}", quiz.ecartees);
        let sortes: Vec<SorteQuestion> = quiz.questions.iter().map(|q| q.sorte).collect();
        assert_eq!(
            sortes,
            vec![
                SorteQuestion::ChoixUnique,
                SorteQuestion::VraiFaux,
                SorteQuestion::ReponseCourte,
                SorteQuestion::ChoixMultiples,
                SorteQuestion::Appariement,
            ]
        );
        let q1 = quiz.questions.first().expect("question 1");
        assert_eq!(q1.choix.len(), 4);
        assert_eq!(q1.bonnes, vec!["B".to_owned()]);
        let q5 = quiz.questions.get(4).expect("question 5");
        assert_eq!(q5.elements.len(), 2);
        assert_eq!(q5.bonnes, vec!["A-2".to_owned(), "B-1".to_owned()]);
        let explication = texte_des_blocs(&q5.explication);
        assert!(explication.contains("3 × 3 = 9"), "{explication}");
    }

    fn texte_des_blocs(blocs: &[Bloc]) -> String {
        super::super::lecon::Lecon {
            blocs: blocs.to_vec(),
        }
        .texte_brut()
    }

    #[test]
    fn le_score_est_juste_pour_chaque_sorte_et_les_cas_limites() {
        let quiz = analyser_quiz(RACINE, false);
        let q = |n: usize| quiz.questions.get(n).expect("question");
        assert!(corriger(q(0), "B"));
        assert!(corriger(q(0), " b "));
        assert!(!corriger(q(0), "A"));
        assert!(!corriger(q(0), ""));
        assert!(corriger(q(1), "B"));
        assert!(corriger(q(2), "3.5"));
        assert!(corriger(q(2), "3,5"));
        assert!(corriger(q(2), " 3,50 "));
        assert!(corriger(q(2), "7/2"));
        assert!(corriger(q(2), "7 / 2"));
        assert!(!corriger(q(2), "3"));
        assert!(!corriger(q(2), "   "));
        assert!(corriger(q(3), "C, A"));
        assert!(!corriger(q(3), "A"), "choix multiples incomplets");
        assert!(!corriger(q(3), "A, B, C"));
        assert!(corriger(q(4), "B-1, A-2"));
        assert!(!corriger(q(4), "A-2"), "appariement partiel");
        assert!(!corriger(q(4), "A-1, B-2"));
        let reponses = ["B", "A", "3,5", "A,C", "A-2"];
        let (justes, sur) = score(quiz.questions.iter().zip(reponses));
        assert_eq!((justes, sur), (3, 5));
    }

    #[test]
    fn majuscules_accents_et_espaces_d_une_reponse_courte() {
        let quiz = analyser_quiz(
            "## Quiz\n\n**Question 1.** Capitale du Mali ?\n\n### Réponses\n\n1. **« Bamako ».** Sur le fleuve.\n\n**Question 2.** Le contraire de « léger » ?\n\n### Réponses\n\n2. **« Lourd ».** Rien.\n",
            false,
        );
        let q = quiz.questions.first().expect("question");
        assert!(corriger(q, "  BAMAKO "));
        assert!(!corriger(q, "Bamak"));
        assert_eq!(normaliser_reponse("Élève  Été"), "eleveete");
    }

    #[test]
    fn une_question_incomplete_reste_du_texte_avec_sa_raison() {
        let quiz = analyser_quiz(
            "## Quiz\n\n**Question 1.** Sans réponse :\nA) un  B) deux\n\n\
             **Question 2.** Réponse absente des choix :\nA) un  B) deux\n\n\
             **Question 3.** Bonne :\nA) un  B) deux\n\n\
             ### Réponses\n\n2. **D.** Rien.\n3. **A.** Oui.\n",
            false,
        );
        assert_eq!(quiz.questions.len(), 1);
        assert_eq!(quiz.ecartees.len(), 2);
        let raisons: Vec<&str> = quiz.ecartees.iter().map(|e| e.raison.as_str()).collect();
        assert!(raisons.iter().any(|r| r.contains("la réponse manque")));
        assert!(raisons.iter().any(|r| r.contains("pas parmi les choix")));
    }

    #[test]
    fn un_texte_sans_quiz_ne_donne_aucune_question() {
        assert!(analyser_quiz("## L'essentiel\n\nTexte.\n", false).est_vide());
        assert!(analyser_quiz("", true).est_vide());
    }

    #[test]
    fn une_ligne_de_choix_coupee_et_trois_choix_sont_reconnus() {
        let quiz = analyser_quiz(
            "## 7. Quiz\n\n**Question 1.** En saison sèche, beaucoup de grenouilles :\n\
             A) meurent toutes  B) s'enfouissent dans la boue  C) deviennent des\n\
             crapauds  D) partent vivre dans les arbres\n\n\
             **Question 2.** Le lapin :\nA) pond  B) allaite  C) vole\n\n\
             ### Réponses\n\n1. **B.** Elles ressortent aux pluies.\n2. **B.** C'est un mammifère.\n",
            false,
        );
        assert!(quiz.ecartees.is_empty(), "{:?}", quiz.ecartees);
        let q1 = quiz.questions.first().expect("q1");
        assert_eq!(q1.choix.len(), 4);
        assert_eq!(
            texte_simple(&q1.choix.get(2).expect("C").texte),
            "deviennent des crapauds"
        );
        assert_eq!(quiz.questions.get(1).expect("q2").choix.len(), 3);
    }

    #[test]
    fn un_contenu_quiz_se_lit_en_entier() {
        let quiz = analyser_quiz(
            "**Question 1.** 2 + 2 ?\nA) 3  B) 4\n\n### Réponses\n\n1. **B.** On compte.\n",
            true,
        );
        assert_eq!(quiz.questions.len(), 1);
    }

    #[test]
    fn le_bouton_question_ecrit_le_gabarit_et_les_reponses() {
        let (q, r) = gabarit_question("");
        let texte = ajouter_question("## L'essentiel\n\nTexte.\n", false, &q, &r);
        let quiz = analyser_quiz(&texte, false);
        assert_eq!(quiz.questions.len(), 1, "{texte}");
        let (q2, r2) = gabarit_question(&texte);
        assert!(q2.starts_with("**Question 2.**"));
        let texte2 = ajouter_question(&texte, false, &q2, &r2);
        let quiz2 = analyser_quiz(&texte2, false);
        assert_eq!(quiz2.questions.len(), 2, "{texte2}");
        assert!(texte2.contains("L'essentiel"));
        // Dans un contenu de nature « quiz ».
        let (q, r) = gabarit_question("");
        let seul = ajouter_question("", true, &q, &r);
        assert_eq!(analyser_quiz(&seul, true).questions.len(), 1, "{seul}");
    }
}
