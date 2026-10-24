//! Modèle `bulletin` (SPEC_PHASE_4 §7.3) : les deux mises en page, trimestrielle et mensuelle,
//! se composent avec toutes les colonnes ou avec celles que l'école a gardées. Relecture à
//! l'œil : `cargo test -p gesco-docs --features apercu-svg --test bulletin` avec
//! `GESCO_BULLETIN_SVG=<dossier>` y dépose chaque page en SVG.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use gesco_docs::{Catalogue, Moteur, Piece, Variante, qr};
use serde_json::{Value, json};

fn variante() -> Variante {
    Variante {
        nom: "École fondamentale Askia".to_owned(),
        adresse: "Hamdallaye ACI 2000, Bamako".to_owned(),
        telephones: vec!["+223 20 22 00 00".to_owned()],
        mentions: "Autorisation d'ouverture n° 2026-0042".to_owned(),
        logo: None,
        papier: gesco_docs::Papier::A4,
        largeurs: Default::default(),
    }
}

fn ligne(
    matiere: &str,
    classe: &str,
    compo: &str,
    moyenne: &str,
    coef: &str,
    points: &str,
) -> Value {
    json!({ "matiere": matiere, "enseignant": "Sékou Traoré", "note_de_classe": classe,
        "composition": compo, "moyenne": moyenne, "coefficient": coef, "points": points,
        "moyenne_classe": "11,50", "appreciation": "Bon travail" })
}

fn bulletin(premier_cycle: bool, qr: &Piece) -> Value {
    let (bareme, lignes) = if premier_cycle {
        (
            "10",
            vec![
                ligne("Langue et communication", "", "7,50", "7,50", "1", "7,50"),
                ligne(
                    "Sciences, maths et technologie",
                    "",
                    "6,00",
                    "6,00",
                    "1",
                    "6,00",
                ),
                ligne("Arts", "", "9,00", "9,00", "1", "9,00"),
            ],
        )
    } else {
        (
            "20",
            vec![
                ligne("Mathématiques", "13,50", "11,00", "11,83", "4", "47,32"),
                ligne("Français", "", "14,00", "14,00", "4", "56,00"),
                ligne("Anglais", "", "", "", "2", ""),
            ],
        )
    };
    json!({
        "premier_cycle": premier_cycle, "eleve": "Awa Traoré", "matricule": "2026-00001",
        "classe": if premier_cycle { "3e A" } else { "7e A" }, "niveau": "7ème année",
        "annee": "2026-2027", "periode": if premier_cycle { "Octobre" } else { "1er trimestre" },
        "bareme": bareme, "lignes": lignes, "total_points": "103,32", "total_coefficients": "8",
        "moyenne": "12,91", "rang": "2e ex", "mention": "Assez bien", "effectif": "42",
        "ont_compose": "41", "premier": "16,25", "dernier": "6,10", "moyenne_classe": "11,02",
        "absences": { "demi_journees": 3, "justifiees": 2, "retards": 1 },
        "appreciation_generale": "Élève sérieuse ; doit soigner les maths.",
        "moyenne_annuelle": if premier_cycle { Value::Null } else { json!("12,42") },
        "rang_annuel": "3e", "arrive_le": if premier_cycle { Value::Null } else { json!("15 janvier 2027") }, "etat": if premier_cycle { "Projet — bulletin non publié" } else { "" },
        "qr": qr.chemin(), "reference": "01J9ZQ3W5N6K8T0V2X4Y6Z8A0B",
        "conduite": if premier_cycle { Value::Null } else {
            json!({ "note": "16,00", "coefficient": "1", "points": "16,00", "comptee": true })
        },
    })
}

fn afficher(tout: bool) -> Value {
    json!({
        "colonne_note_de_classe": tout, "colonne_composition": tout, "colonne_coefficient": tout,
        "colonne_points": tout, "colonne_moyenne_classe": tout,
        "colonne_appreciation": tout, "moyenne_generale": tout, "ont_compose": tout, "rang": true, "mention": true,
        "premier_dernier": true, "moyenne_classe": tout, "absences": true,
        "appreciation_generale": tout,
    })
}

#[test]
fn les_deux_mises_en_page_se_composent_avec_les_colonnes_choisies() {
    let moteur = Moteur::nouveau(Catalogue::integre().unwrap());
    let qr = qr::piece(&qr::reference("01J9ZQ3W5N6K8T0V2X4Y6Z8A0B")).unwrap();
    for (tout, papier) in [
        (true, gesco_docs::Papier::A4),
        (false, gesco_docs::Papier::A4),
        (true, gesco_docs::Papier::A5),
    ] {
        let variante = Variante {
            papier,
            ..variante()
        };
        let donnees = json!({
            "bulletins": [bulletin(false, &qr), bulletin(true, &qr)],
            "afficher": afficher(tout),
        });
        let document = moteur
            .composer_avec("bulletin", &variante, &donnees, std::slice::from_ref(&qr))
            .unwrap();
        assert!(document.pdf.starts_with(b"%PDF"));
        assert_eq!(document.modele_version, 4);
        #[cfg(feature = "apercu-svg")]
        if let Some(dossier) = std::env::var_os("GESCO_BULLETIN_SVG") {
            let pages = moteur
                .composer_svg("bulletin", &variante, &donnees, std::slice::from_ref(&qr))
                .unwrap();
            assert_eq!(pages.len(), 2);
            for (i, page) in pages.iter().enumerate() {
                let nom = format!(
                    "bulletin-{}-{}-{}.svg",
                    papier.nom(),
                    if tout { "complet" } else { "reduit" },
                    i + 1
                );
                std::fs::write(std::path::Path::new(&dossier).join(nom), page).unwrap();
            }
        }
    }
}

/// Cible du §7.3 : 400 bulletins en moins de 2 minutes en mode optimisé, ici 10 classes de 40,
/// un PDF par classe comme à l'écran. Long en mode de mise au point : lancé à la demande,
/// `cargo test --release -p gesco-docs --test bulletin -- --ignored --nocapture`.
#[test]
#[ignore = "mesure de performance, à lancer à la demande"]
#[allow(clippy::print_stdout, reason = "la mesure s'affiche à qui la lance")]
fn quatre_cents_bulletins_en_moins_de_deux_minutes() {
    let moteur = Moteur::nouveau(Catalogue::integre().unwrap());
    let debut = std::time::Instant::now();
    for classe in 0..10 {
        let mut pieces = Vec::new();
        let mut bulletins = Vec::new();
        for eleve in 0..40 {
            let mut qr = qr::piece(&qr::reference(&format!(
                "01J9ZQ3W5N6K8T0V2X4Y6Z{classe:02}{eleve:02}"
            )))
            .unwrap();
            qr.nom = format!("qr-{eleve}.svg");
            bulletins.push(bulletin(classe % 2 == 1, &qr));
            pieces.push(qr);
        }
        let donnees = json!({ "bulletins": bulletins, "afficher": afficher(true) });
        let document = moteur
            .composer_avec("bulletin", &variante(), &donnees, &pieces)
            .unwrap();
        assert!(document.pdf.starts_with(b"%PDF"));
    }
    let duree = debut.elapsed();
    println!("400 bulletins composés en {} ms", duree.as_millis());
    assert!(duree.as_secs() < 120, "{duree:?}");
}

/// Les largeurs réglées par l'école s'appliquent ; une colonne non réglée garde la sienne.
#[test]
fn les_largeurs_reglees_par_l_ecole_s_appliquent() {
    let moteur = Moteur::nouveau(Catalogue::integre().unwrap());
    let qr = qr::piece(&qr::reference("01J9ZQ3W5N6K8T0V2X4Y6Z8A0B")).unwrap();
    let variante = Variante {
        largeurs: [("matiere", 60), ("moyenne", 25), ("appreciation", 40)]
            .into_iter()
            .map(|(c, mm)| (c.to_owned(), mm))
            .collect(),
        ..variante()
    };
    let donnees = json!({
        "bulletins": [bulletin(true, &qr)],
        "afficher": afficher(true),
    });
    let document = moteur
        .composer_avec("bulletin", &variante, &donnees, std::slice::from_ref(&qr))
        .unwrap();
    assert!(document.pdf.starts_with(b"%PDF"));
    #[cfg(feature = "apercu-svg")]
    if let Some(dossier) = std::env::var_os("GESCO_BULLETIN_SVG") {
        let pages = moteur
            .composer_svg("bulletin", &variante, &donnees, std::slice::from_ref(&qr))
            .unwrap();
        std::fs::write(
            std::path::Path::new(&dossier).join("bulletin-largeurs.svg"),
            &pages[0],
        )
        .unwrap();
    }
}
