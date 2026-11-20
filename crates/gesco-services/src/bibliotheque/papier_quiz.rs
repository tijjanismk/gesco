//! Le quiz sur papier (SPEC_PHASE_12_LOT_3 §6) : « Quiz seul » (les questions et leurs choix, une
//! ligne pour la réponse courte, aucune réponse, « Nom : ……… ») et « Réponses du quiz » (pour le
//! maître : la bonne réponse et l'explication de chaque question). Les deux formes deviennent un
//! arbre de blocs ordinaire, imprimé par le modèle des leçons. Une question non reconnue
//! s'imprime telle qu'elle est écrite.

use gesco_core::bibliotheque::{Bloc, ChoixQuestion, Enligne, Lecon, QuestionQuiz, SorteQuestion};

/// Ligne laissée pour écrire.
const LIGNE: &str = "……………………………………………………";

fn texte(t: &str) -> Enligne {
    Enligne::Texte {
        texte: t.to_owned(),
    }
}

fn gras(t: &str) -> Enligne {
    Enligne::Gras {
        contenu: vec![texte(t)],
    }
}

/// Texte simple d'éléments de ligne.
fn texte_de(elements: &[Enligne]) -> String {
    let mut s = String::new();
    for e in elements {
        match e {
            Enligne::Texte { texte } | Enligne::Code { texte } => s.push_str(texte),
            Enligne::Gras { contenu } | Enligne::Italique { contenu } => {
                s.push_str(&texte_de(contenu));
            }
            _ => {}
        }
    }
    s
}

/// Sans accents, en minuscules, sans numéro en tête.
fn simple(t: &str) -> String {
    t.trim()
        .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ')')
        .trim()
        .chars()
        .map(|c| match c {
            'é' | 'è' | 'ê' | 'É' | 'È' | 'Ê' => 'e',
            'à' | 'â' | 'À' | 'Â' => 'a',
            autre => autre.to_lowercase().next().unwrap_or(autre),
        })
        .collect()
}

/// Numéro d'un paragraphe « **Question n.** … ».
fn numero_question(bloc: &Bloc) -> Option<u32> {
    let Bloc::Paragraphe { contenu } = bloc else {
        return None;
    };
    let t = texte_de(contenu);
    let reste = t.trim_start().strip_prefix("Question")?.trim_start();
    let chiffres: String = reste.chars().take_while(char::is_ascii_digit).collect();
    chiffres.parse().ok()
}

/// Blocs de la section « Quiz » telle qu'elle est écrite, sans « Réponses » ni corrigés ; tout le
/// texte pour un contenu de nature « quiz ».
fn section_ecrite(lecon: &Lecon, tout_le_texte: bool) -> Vec<Bloc> {
    let blocs = lecon.sans_les_corriges().blocs;
    if tout_le_texte {
        return blocs;
    }
    let mut dedans: Option<u8> = None;
    let mut section = Vec::new();
    for b in blocs {
        if let Bloc::Titre { niveau, contenu } = &b {
            match dedans {
                None if simple(&texte_de(contenu)).starts_with("quiz") => {
                    dedans = Some(*niveau);
                    continue;
                }
                Some(n) if *niveau <= n => break,
                _ => {}
            }
        }
        if dedans.is_some() {
            section.push(b);
        }
    }
    section
}

/// Questions écrites dans la section, avec leurs blocs, dans l'ordre du texte.
fn questions_ecrites(section: Vec<Bloc>) -> Vec<(u32, Vec<Bloc>)> {
    let mut questions: Vec<(u32, Vec<Bloc>)> = Vec::new();
    for b in section {
        if let Some(n) = numero_question(&b) {
            questions.push((n, vec![b]));
        } else if let Some((_, blocs)) = questions.last_mut() {
            blocs.push(b);
        }
    }
    questions
}

/// Une liste de choix « A) … ».
fn liste(choix: &[ChoixQuestion]) -> Bloc {
    Bloc::Liste {
        numerotee: false,
        debut: None,
        elements: choix
            .iter()
            .map(|c| {
                let mut ligne = vec![texte(&format!("{}) ", c.repere))];
                ligne.extend(c.texte.iter().cloned());
                vec![Bloc::Paragraphe { contenu: ligne }]
            })
            .collect(),
    }
}

/// « Question n. » suivi de l'énoncé : le premier paragraphe de l'énoncé reçoit le titre.
fn tete(q: &QuestionQuiz) -> Vec<Bloc> {
    let mut blocs = q.enonce.clone();
    let titre = gras(&format!("Question {}.", q.numero));
    match blocs.first_mut() {
        Some(Bloc::Paragraphe { contenu }) => {
            contenu.insert(0, texte(" "));
            contenu.insert(0, titre);
        }
        _ => blocs.insert(
            0,
            Bloc::Paragraphe {
                contenu: vec![titre],
            },
        ),
    }
    blocs
}

/// La question telle que l'élève la lit, sans réponse.
fn question_seule(q: &QuestionQuiz) -> Vec<Bloc> {
    let mut blocs = tete(q);
    match q.sorte {
        SorteQuestion::ReponseCourte => blocs.push(Bloc::Paragraphe {
            contenu: vec![texte(&format!("Réponse : {LIGNE}"))],
        }),
        SorteQuestion::Appariement => {
            blocs.push(liste(&q.choix));
            blocs.push(liste(&q.elements));
            blocs.push(Bloc::Paragraphe {
                contenu: vec![texte(&format!("Réponse : {LIGNE}"))],
            });
        }
        _ => blocs.push(liste(&q.choix)),
    }
    blocs
}

/// La bonne réponse écrite pour le maître.
fn bonne_reponse(q: &QuestionQuiz) -> Vec<Enligne> {
    let trouver = |r: &str, l: &[ChoixQuestion]| l.iter().find(|c| c.repere == r).cloned();
    let mut ligne = vec![gras(&format!("Question {}.", q.numero)), texte(" ")];
    match q.sorte {
        SorteQuestion::ReponseCourte => {
            let admises: Vec<String> = q.bonnes.iter().map(|b| format!("« {b} »")).collect();
            ligne.push(texte(&admises.join(" ou ")));
        }
        SorteQuestion::Appariement => {
            for (i, b) in q.bonnes.iter().enumerate() {
                if i > 0 {
                    ligne.push(texte(" ; "));
                }
                let (l, n) = b.split_once('-').unwrap_or((b.as_str(), ""));
                ligne.push(texte(&format!("{l}) ")));
                if let Some(c) = trouver(l, &q.choix) {
                    ligne.extend(c.texte);
                }
                ligne.push(texte(&format!(" → {n}) ")));
                if let Some(e) = trouver(n, &q.elements) {
                    ligne.extend(e.texte);
                }
            }
        }
        _ => {
            for (i, b) in q.bonnes.iter().enumerate() {
                if i > 0 {
                    ligne.push(texte(" ; "));
                }
                ligne.push(texte(&format!("{b}) ")));
                if let Some(c) = trouver(b, &q.choix) {
                    ligne.extend(c.texte);
                }
            }
        }
    }
    ligne
}

/// L'arbre de blocs d'une des deux formes. `reponses` : « Réponses du quiz » ; sinon « Quiz
/// seul ». Rend `None` si le contenu n'a aucune question écrite.
pub(crate) fn lecon_du_quiz(
    lecon: &Lecon,
    tout_le_texte: bool,
    questions: &[QuestionQuiz],
    reponses: bool,
) -> Option<Lecon> {
    let ecrites = questions_ecrites(section_ecrite(lecon, tout_le_texte));
    if ecrites.is_empty() && questions.is_empty() {
        return None;
    }
    let mut blocs = Vec::new();
    if !reponses {
        blocs.push(Bloc::Paragraphe {
            contenu: vec![texte(&format!("Nom : {LIGNE}"))],
        });
    }
    let mut faites: Vec<u32> = Vec::new();
    let ajouter = |blocs: &mut Vec<Bloc>, q: &QuestionQuiz| {
        if reponses {
            blocs.push(Bloc::Paragraphe {
                contenu: bonne_reponse(q),
            });
            blocs.extend(q.explication.iter().cloned());
        } else {
            blocs.extend(question_seule(q));
        }
    };
    for (n, ecrits) in ecrites {
        match questions.iter().find(|q| q.numero == n) {
            Some(q) => ajouter(&mut blocs, q),
            // Pas reconnue : telle qu'elle est écrite, sans réponse.
            None => blocs.extend(ecrits),
        }
        faites.push(n);
    }
    for q in questions.iter().filter(|q| !faites.contains(&q.numero)) {
        ajouter(&mut blocs, q);
    }
    Some(Lecon { blocs })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use gesco_core::bibliotheque::{analyser, analyser_quiz};

    use super::*;

    const TEXTE: &str = "# Racine\n\nCours.\n\n## Quiz\n\n**Question 1.** √25 est égal à :\nA) 5  B) 25\n\n**Question 2.** Combien font 7 / 2 ?\n\n**Question 3.** Sans réponse ?\nA) un  B) deux\n\n### Réponses\n\n1. **A.** Car 5 × 5 = 25.\n2. **« 3,5 ».** On partage.\n\n## Exercices\n\nFaire.\n";

    fn tout(l: &Lecon) -> String {
        format!("{:?}", l.blocs)
    }

    #[test]
    fn le_quiz_seul_n_a_aucune_reponse_et_garde_les_questions_ecrites() {
        let quiz = analyser_quiz(TEXTE, false);
        let seul = lecon_du_quiz(&analyser(TEXTE), false, &quiz.questions, false).expect("un quiz");
        let s = tout(&seul);
        assert!(s.contains("Nom : "));
        assert!(s.contains("Question 1."));
        assert!(s.contains("Réponse : "), "une ligne pour la réponse courte");
        assert!(
            s.contains("Sans réponse"),
            "la question non reconnue telle qu'elle est écrite"
        );
        assert!(!s.contains("Car 5"), "aucune explication");
        assert!(!s.contains("3,5"), "aucune réponse");
        assert!(!s.contains("Faire."), "rien hors de la section");
        assert!(!s.contains("Cours."));
    }

    #[test]
    fn les_reponses_du_quiz_donnent_la_bonne_reponse_et_l_explication() {
        let quiz = analyser_quiz(TEXTE, false);
        let r = lecon_du_quiz(&analyser(TEXTE), false, &quiz.questions, true).expect("un quiz");
        let s = tout(&r);
        assert!(s.contains("A) "));
        assert!(s.contains("Car 5"));
        assert!(s.contains("« 3,5 »"));
        assert!(!s.contains("Nom : "));
    }

    #[test]
    fn une_lecon_sans_quiz_ne_donne_rien() {
        let t = "# Leçon\n\nTexte.\n";
        assert_eq!(lecon_du_quiz(&analyser(t), false, &[], false), None);
    }
}
