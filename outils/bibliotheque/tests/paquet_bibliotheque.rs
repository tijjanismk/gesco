//! Essai de l'outil sur la bibliothèque réelle (SPEC_PHASE_12_LOT_1, critères 1 et 2 du §0.3) :
//! le paquet fabriqué compte autant de leçons, de sujets et de corrigés qu'un décompte
//! indépendant des fichiers, aucun texte affichable ne porte de marque interdite, et toutes les
//! formules se convertissent.
//!
//! La bibliothèque est hors du dépôt : son dossier `contenus` est lu dans `GESCO_BIBLIOTHEQUE`,
//! sinon à côté du dossier du dépôt. Absente (intégration continue), l'essai ne vérifie rien.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;

use gesco_core::bibliotheque::paquet::{NatureContenu, lire_paquet};
use gesco_core::bibliotheque::{analyser, marques_interdites};
use gesco_docs::formules::latex_vers_typst;

fn dossier_bibliotheque() -> Option<PathBuf> {
    let dossier = std::env::var_os("GESCO_BIBLIOTHEQUE").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../gesco-bibliotheque/contenus"),
        PathBuf::from,
    );
    dossier.is_dir().then_some(dossier)
}

/// Décompte indépendant : (leçons, sujets, corrigés), d'après les seuls noms de fichiers.
fn compter(dossier: &Path, n: &mut (usize, usize, usize)) {
    for entree in std::fs::read_dir(dossier).unwrap() {
        let chemin = entree.unwrap().path();
        let nom = chemin.file_name().unwrap().to_string_lossy().into_owned();
        if chemin.is_dir() {
            if !nom.starts_with('_') {
                compter(&chemin, n);
            }
        } else if nom.ends_with(".md") && !nom.ends_with(".faits.md") {
            if nom.starts_with("lecon-") {
                n.0 += 1;
            } else if nom.starts_with("def-") || nom.starts_with("bac-") {
                if nom.ends_with("-corrige.md") {
                    n.2 += 1;
                } else {
                    n.1 += 1;
                }
            }
        }
    }
}

fn outil(arguments: &[&str]) -> String {
    let sortie = Command::new(env!("CARGO_BIN_EXE_gesco-paquet-bibliotheque"))
        .args(arguments)
        .output()
        .unwrap();
    assert!(
        sortie.status.success(),
        "{}",
        String::from_utf8_lossy(&sortie.stderr)
    );
    String::from_utf8_lossy(&sortie.stdout).into_owned()
}

#[test]
fn le_paquet_de_la_bibliotheque_est_complet_et_propre() {
    let Some(contenus) = dossier_bibliotheque() else {
        return;
    };
    let temporaire = tempfile::tempdir().unwrap();
    let cle = temporaire.path().join("cle.txt");
    let sortie = temporaire.path().join("paquet.gbib");
    let message = outil(&["cles", cle.to_str().unwrap()]);
    let publique = message
        .lines()
        .find_map(|l| l.strip_prefix("Clé publique : "))
        .unwrap()
        .trim()
        .to_owned();
    outil(&[
        "fabriquer",
        "--contenus",
        contenus.to_str().unwrap(),
        "--cle",
        cle.to_str().unwrap(),
        "--version",
        "2026.11.1",
        "--date",
        "2026-11-15",
        "--sortie",
        sortie.to_str().unwrap(),
    ]);
    let cle_publique: [u8; 32] = hex::decode(publique).unwrap().try_into().unwrap();
    let paquet = lire_paquet(&std::fs::read(&sortie).unwrap(), &cle_publique).unwrap();

    // Critère 1 : décompte indépendant.
    let mut attendu = (0, 0, 0);
    compter(&contenus, &mut attendu);
    let nombre = |n: NatureContenu| {
        paquet
            .manifeste
            .contenus
            .iter()
            .filter(|c| c.nature == n)
            .count()
    };
    assert_eq!(
        (
            nombre(NatureContenu::Lecon),
            nombre(NatureContenu::Sujet),
            nombre(NatureContenu::Corrige)
        ),
        attendu
    );
    // Aucun chapitre vide.
    for chapitre in &paquet.manifeste.chapitres {
        assert!(
            paquet
                .manifeste
                .contenus
                .iter()
                .any(|c| c.chapitre == chapitre.cle),
            "chapitre vide {}",
            chapitre.cle
        );
    }

    // Critère 2 : aucun texte affichable ne porte de marque interdite ; critère 5 : toutes les
    // formules se convertissent.
    let mut fautes = Vec::new();
    for contenu in &paquet.manifeste.contenus {
        let mut affiche = format!("{}\n", contenu.titre);
        for nom in std::iter::once(&contenu.fichier).chain(contenu.faits.iter()) {
            let lecon = analyser(paquet.texte(nom).unwrap());
            affiche.push_str(&lecon.texte_brut());
            for formule in lecon.formules() {
                if latex_vers_typst(&formule.latex).is_err() {
                    fautes.push(format!("{} : formule {}", contenu.cle, formule.latex));
                }
            }
        }
        let marques = marques_interdites(&affiche);
        if !marques.is_empty() {
            fautes.push(format!("{} : {}", contenu.cle, marques.join(", ")));
        }
    }
    for chapitre in &paquet.manifeste.chapitres {
        let marques = marques_interdites(&format!("{} {}", chapitre.partie, chapitre.titre));
        if !marques.is_empty() {
            fautes.push(format!("{} : {}", chapitre.cle, marques.join(", ")));
        }
    }
    assert!(fautes.is_empty(), "{}", fautes.join("\n"));
}
