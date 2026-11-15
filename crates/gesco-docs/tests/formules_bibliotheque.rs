//! Essai de la bibliothèque réelle (SPEC_PHASE_12_LOT_1, étape 1, critère 5 du §0.3) : toutes les
//! formules des leçons et des sujets se convertissent vers Typst, et Typst les compose toutes.
//!
//! La bibliothèque est hors du dépôt : son dossier `contenus` est lu dans `GESCO_BIBLIOTHEQUE`,
//! sinon à côté du dossier du dépôt. Absente (intégration continue), l'essai ne vérifie rien.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use gesco_core::bibliotheque::analyser;
use gesco_docs::formules::latex_vers_typst;
use gesco_docs::{Catalogue, Moteur, Papier, Variante};
use serde_json::json;

/// Formules composées par document d'essai.
const PAR_DOCUMENT: usize = 1500;

fn dossier_bibliotheque() -> Option<PathBuf> {
    let dossier = std::env::var_os("GESCO_BIBLIOTHEQUE").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../gesco-bibliotheque/contenus"),
        PathBuf::from,
    );
    dossier.is_dir().then_some(dossier)
}

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

#[test]
fn toutes_les_formules_se_convertissent_et_se_composent() {
    let Some(dossier) = dossier_bibliotheque() else {
        return;
    };
    let mut fichiers = Vec::new();
    contenus(&dossier, &mut fichiers);

    let mut converties = BTreeSet::new();
    let mut refusees = Vec::new();
    for fichier in &fichiers {
        let lecon = analyser(&std::fs::read_to_string(fichier).unwrap());
        for formule in lecon.formules() {
            match latex_vers_typst(&formule.latex) {
                Ok(typst) => {
                    converties.insert(typst);
                }
                Err(e) => refusees.push(format!("{} : {} ({e})", fichier.display(), formule.latex)),
            }
        }
    }
    assert!(
        refusees.is_empty(),
        "{} formules non converties :\n{}",
        refusees.len(),
        refusees.join("\n")
    );

    // Typst compose chaque formule convertie : un document par lot, chaque formule évaluée en
    // mode mathématique, comme le fera le modèle de la leçon.
    let racine = tempfile::tempdir().unwrap();
    let modele = racine.path().join("formules_essai");
    std::fs::create_dir(&modele).unwrap();
    std::fs::write(
        modele.join("modele.json"),
        r#"{"id":"formules_essai","version":1,"libelle":"Formules","variables":[{"nom":"formules","requise":true}]}"#,
    )
    .unwrap();
    std::fs::write(
        modele.join("modele.typ"),
        "#let d = json(\"/donnees.json\")\n\
         #set text(font: \"Charis SIL\", size: 10pt, lang: \"fr\")\n\
         #show math.equation: set text(font: \"New Computer Modern Math\")\n\
         #for f in d.formules [\n  #math.equation(block: true, eval(f, mode: \"math\"))\n]\n",
    )
    .unwrap();
    let mut catalogue = Catalogue::integre().unwrap();
    assert_eq!(catalogue.surcharger_depuis(racine.path()).unwrap(), 1);
    let moteur = Moteur::nouveau(catalogue);
    let variante = Variante {
        nom: "École d'essai".to_owned(),
        adresse: String::new(),
        telephones: Vec::new(),
        mentions: String::new(),
        logo: None,
        papier: Papier::A4,
        largeurs: Default::default(),
    };
    let formules: Vec<String> = converties.into_iter().collect();
    for lot in formules.chunks(PAR_DOCUMENT) {
        if let Err(e) = moteur.composer("formules_essai", &variante, &json!({ "formules": lot })) {
            // Le lot ne passe pas : on cherche la formule fautive pour la nommer.
            let fautives: Vec<&String> = lot
                .iter()
                .filter(|f| {
                    moteur
                        .composer("formules_essai", &variante, &json!({ "formules": [f] }))
                        .is_err()
                })
                .take(20)
                .collect();
            panic!("{e} ; formules refusées par Typst : {fautives:#?}");
        }
    }
}
