//! Essai de la bibliothèque réelle (SPEC_PHASE_12_LOT_1, étape 1) : toutes les leçons et tous les
//! sujets s'analysent, et chaque image appelée par un nom de fichier est reconnue.
//!
//! La bibliothèque est hors du dépôt : son dossier `contenus` est lu dans `GESCO_BIBLIOTHEQUE`,
//! sinon à côté du dossier du dépôt. Absente (intégration continue), l'essai ne vérifie rien.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::print_stderr
)]

use std::path::{Path, PathBuf};

use gesco_core::bibliotheque::{Bloc, analyser};

fn dossier_bibliotheque() -> Option<PathBuf> {
    let dossier = std::env::var_os("GESCO_BIBLIOTHEQUE").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../gesco-bibliotheque/contenus"),
        PathBuf::from,
    );
    dossier.is_dir().then_some(dossier)
}

/// Leçons (`lecon-*.md`) et sujets (`def-*.md`, `bac-*.md`), hors listes de faits et brouillons.
fn contenus(dossier: &Path, trouves: &mut Vec<PathBuf>) {
    for entree in std::fs::read_dir(dossier).unwrap() {
        let chemin = entree.unwrap().path();
        let nom = chemin.file_name().unwrap().to_string_lossy().into_owned();
        if chemin.is_dir() {
            if !nom.starts_with('_') {
                contenus(&chemin, trouves);
            }
        } else if nom.ends_with(".md")
            && !nom.ends_with(".faits.md")
            && ["lecon-", "def-", "bac-"]
                .iter()
                .any(|p| nom.starts_with(p))
        {
            trouves.push(chemin);
        }
    }
}

/// Nombre d'appels d'image `](nom.svg)` écrits dans le texte, hors blocs de code.
fn appels_d_image(texte: &str) -> usize {
    let mut nombre = 0;
    let mut reste = texte;
    while let Some(i) = reste.find("](") {
        reste = reste.get(i + 2..).unwrap_or("");
        let cible = reste.split(')').next().unwrap_or("");
        let minuscule = cible.to_ascii_lowercase();
        let image = [".svg", ".png", ".jpg", ".jpeg"]
            .iter()
            .any(|e| minuscule.ends_with(e));
        if image && !cible.contains('/') && !cible.contains('\\') {
            nombre += 1;
        }
    }
    nombre
}

#[test]
fn toutes_les_lecons_et_tous_les_sujets_s_analysent() {
    let Some(dossier) = dossier_bibliotheque() else {
        return;
    };
    let mut fichiers = Vec::new();
    contenus(&dossier, &mut fichiers);
    assert!(
        fichiers.len() > 400,
        "{} contenus seulement",
        fichiers.len()
    );

    let mut formules = 0;
    let mut ecarts = Vec::new();
    for fichier in &fichiers {
        let texte = std::fs::read_to_string(fichier).unwrap();
        let lecon = analyser(&texte);
        assert!(
            lecon
                .blocs
                .iter()
                .any(|b| matches!(b, Bloc::Titre { .. } | Bloc::Paragraphe { .. })),
            "{} : aucun texte",
            fichier.display()
        );
        formules += lecon.formules().len();
        let attendues = appels_d_image(&texte);
        if lecon.images().len() != attendues {
            ecarts.push(format!(
                "{} : {} images reconnues sur {attendues}",
                fichier.display(),
                lecon.images().len()
            ));
        }
        // Le texte pour la recherche ne garde aucun balisage d'image.
        assert!(!lecon.texte_brut().contains("]("), "{}", fichier.display());
    }
    assert!(ecarts.is_empty(), "{}", ecarts.join("\n"));
    assert!(formules > 10_000, "{formules} formules seulement");
}

/// Décompte indépendant de l'analyse : lignes « **Question n.** » de la section du quiz.
fn questions_ecrites(texte: &str) -> usize {
    let mut dans_quiz = false;
    let mut nombre = 0;
    for ligne in texte.lines() {
        if ligne.starts_with("## ") {
            dans_quiz = ligne.to_lowercase().contains("quiz");
        } else if dans_quiz && ligne.starts_with("**Question ") {
            nombre += 1;
        }
    }
    nombre
}

/// Lot 3, étape 1 : les quiz livrés sont reconnus (§0.3, critère 1). Au moins 1 950 questions
/// sur les 1 990 écrites ; le décompte de l'analyse (reconnues et écartées) égale le décompte
/// indépendant ; les leçons dont une question n'est pas reconnue sont listées pour l'intégration
/// des cours. Aucun sujet ne donne de quiz.
#[test]
fn les_quiz_livres_sont_reconnus() {
    use gesco_core::bibliotheque::{analyser_quiz, corriger};
    let Some(dossier) = dossier_bibliotheque() else {
        return;
    };
    let mut fichiers = Vec::new();
    contenus(&dossier, &mut fichiers);
    let mut reconnues = 0;
    let mut ecrites = 0;
    let mut a_reprendre = Vec::new();
    for fichier in &fichiers {
        let texte = std::fs::read_to_string(fichier).unwrap();
        let quiz = analyser_quiz(&texte, false);
        let nom = fichier.file_name().unwrap().to_string_lossy().into_owned();
        if !nom.starts_with("lecon-") {
            continue;
        }
        let attendues = questions_ecrites(&texte);
        assert_eq!(
            quiz.questions.len() + quiz.ecartees.len(),
            attendues,
            "{}",
            fichier.display()
        );
        ecrites += attendues;
        reconnues += quiz.questions.len();
        for q in &quiz.questions {
            // La bonne réponse est juste, et une autre lettre est fausse.
            let bonne = q.bonnes.join(",");
            assert!(corriger(q, &bonne), "{} : {}", fichier.display(), q.numero);
            assert!(!corriger(q, ""), "{}", fichier.display());
            assert!(
                !q.explication.is_empty(),
                "{} : {}",
                fichier.display(),
                q.numero
            );
        }
        if !quiz.ecartees.is_empty() {
            a_reprendre.push(format!(
                "{} : {}",
                fichier.display(),
                quiz.ecartees
                    .iter()
                    .map(|e| format!("question {} ({})", e.numero, e.raison))
                    .collect::<Vec<_>>()
                    .join(" ; ")
            ));
        }
    }
    eprintln!(
        "Quiz : {reconnues} questions reconnues sur {ecrites}.\nÀ reprendre à l'intégration des cours :\n{}",
        a_reprendre.join("\n")
    );
    assert!(ecrites >= 1_990, "{ecrites} questions écrites seulement");
    assert!(
        reconnues >= 1_950,
        "{reconnues} questions reconnues seulement"
    );
}
