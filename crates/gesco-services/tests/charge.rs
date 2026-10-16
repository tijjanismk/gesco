//! Charge d'une école de taille réelle (SPEC_PHASE_10 §7) : 1 520 élèves en 40 classes, importés
//! d'un classeur, validés et facturés classe par classe, notés dans une matière, leurs bulletins
//! calculés, la liste des impayés et la fin d'année lues. Chaque étape se mesure et reste sous
//! son plafond.
//!
//! Long : `cargo test -p gesco-services --release --test charge -- --ignored --nocapture`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::print_stdout
)]

mod commun;

use std::time::{Duration, Instant};

use commun::*;
use gesco_docs::tableur::{Liste, classeur};
use serde_json::{Value, json};

const CLASSES_PAR_NIVEAU: usize = 4;
const ELEVES_PAR_CLASSE: usize = 38;
const NOMS: [&str; 8] = [
    "TRAORE",
    "DIALLO",
    "KEITA",
    "COULIBALY",
    "SANGARE",
    "DEMBELE",
    "KONE",
    "TOURE",
];
const PRENOMS: [&str; 8] = [
    "Awa",
    "Moussa",
    "Fatoumata",
    "Seydou",
    "Oumar",
    "Mariam",
    "Adama",
    "Aminata",
];

fn mesurer(etape: &str, debut: Instant, plafond: Duration) {
    let duree = debut.elapsed();
    println!("{etape} : {:.1} s", duree.as_secs_f64());
    assert!(
        duree <= plafond,
        "{etape} : {duree:?} au-delà de {plafond:?}"
    );
}

#[tokio::test]
#[ignore = "test de charge, long : à lancer en --release avec --ignored"]
async fn une_ecole_de_1500_eleves_tient_dans_des_temps_bornes() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let annee = p
        .ok(
            "annee_creer",
            Some(&admin),
            json!({ "libelle": "2026-2027", "debut": RENTREE_2026, "fin": FIN_2027 }),
        )
        .await;
    p.ok("annee_activer", Some(&admin), json!({ "id": annee["id"] }))
        .await;
    poser_grille(&p, &admin, &annee["id"]).await;
    let ped = p
        .ok("pedagogie_lire", Some(&admin), json!({ "id": annee["id"] }))
        .await;
    // Dix niveaux du 1er cycle au lycée, quatre classes chacun : 40 classes.
    let mut classes: Vec<(String, Value, Value)> = Vec::new();
    for code in ["1", "2", "3", "4", "5", "6", "7", "8", "9", "10"] {
        let niveau = ped["niveaux"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["code"] == code)
            .unwrap()["id"]
            .clone();
        for lettre in ["A", "B", "C", "D"].iter().take(CLASSES_PAR_NIVEAU) {
            let libelle = format!("{code}e {lettre}");
            let id = p
                .ok(
                    "classe_creer",
                    Some(&admin),
                    json!({ "annee_id": annee["id"], "niveau_id": niveau, "serie_id": null, "libelle": libelle }),
                )
                .await["id"]
                .clone();
            classes.push((libelle, id, niveau.clone()));
        }
    }

    // Le classeur : 38 élèves par classe, chacun avec le téléphone de son parent.
    let entetes = [
        "N°",
        "Nom et prénoms",
        "Sexe",
        "Né(e) le",
        "Lieu de naissance",
        "Classe",
        "Téléphone du parent",
    ]
    .map(str::to_owned)
    .to_vec();
    let mut lignes = Vec::new();
    for (c, (libelle, _, _)) in classes.iter().enumerate() {
        for e in 0..ELEVES_PAR_CLASSE {
            let n = c * ELEVES_PAR_CLASSE + e;
            lignes.push(vec![
                (n + 1).to_string(),
                format!("{} {}", NOMS[n % 8], PRENOMS[(n / 8) % 8]),
                if n.is_multiple_of(2) { "F" } else { "M" }.to_owned(),
                format!("{:02}/{:02}/{}", 1 + n % 28, 1 + n % 12, 2008 + c / 4),
                "Bamako".to_owned(),
                libelle.clone(),
                format!(
                    "7{} {:02} {:02} {:02}",
                    n % 10,
                    (n / 100) % 100,
                    (n / 10) % 100,
                    n % 100
                ),
            ]);
        }
    }
    let total = lignes.len();
    let octets = classeur(&Liste {
        titre: "Élèves",
        entetes: &entetes,
        lignes: &lignes,
    })
    .unwrap();
    let chemin = dossier.path().join("eleves.xlsx");
    std::fs::write(&chemin, octets).unwrap();

    let debut = Instant::now();
    let analyse = p
        .ok(
            "import_eleves_analyser",
            Some(&admin),
            json!({ "chemin": chemin.to_string_lossy() }),
        )
        .await;
    let reglages = json!({
        "fichier_id": analyse["fichier_id"],
        "champs": analyse["champs"],
        "disposition": "colonne_classe",
        "decoupe": "nom_d_abord",
    });
    let rapport = p
        .ok("import_eleves_appliquer", Some(&admin), reglages)
        .await;
    assert_eq!(rapport["importes"], total);
    mesurer(
        &format!("Import de {total} élèves"),
        debut,
        Duration::from_secs(300),
    );

    // Validation et facturation, classe par classe.
    let debut = Instant::now();
    let mut validees = 0;
    let mut refus = std::collections::BTreeMap::<String, u32>::new();
    for (_, id, _) in &classes {
        let bilan = p
            .ok(
                "inscriptions_valider_classe",
                Some(&admin),
                json!({ "id": id }),
            )
            .await;
        validees += bilan["validees"].as_u64().unwrap_or_default();
        for r in bilan["refusees"].as_array().unwrap() {
            *refus
                .entry(r["cle"].as_str().unwrap().to_owned())
                .or_default() += 1;
        }
    }
    println!("{validees} inscriptions validées et facturées ; refusées : {refus:?}");
    assert!(validees > 0);
    mesurer(
        "Validation et facturation des 40 classes",
        debut,
        Duration::from_secs(300),
    );

    // Une composition dans la première matière au programme du niveau, à la 1re période de
    // chaque classe, puis les bulletins.
    let mut duree_bulletins = Duration::ZERO;
    let mut plus_long = Duration::ZERO;
    for (_, classe, niveau) in &classes {
        let periode = ped["periodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["periode"]["niveau_id"] == *niveau && x["periode"]["rang"] == 1)
            .unwrap()["periode"]
            .clone();
        let (date, periode) = (periode["fin"].clone(), periode["id"].clone());
        let matiere = ped["programme"]
            .as_array()
            .unwrap()
            .iter()
            .find(|l| l["niveau_id"] == *niveau)
            .unwrap()["matiere_id"]
            .clone();
        let e = p
            .ok(
                "evaluation_creer",
                Some(&admin),
                json!({ "classe_id": classe, "periode_id": periode, "matiere_id": matiere,
                    "type_evaluation": "composition", "libelle": "Composition", "date": date, "bareme": null }),
            )
            .await;
        let grille = p
            .ok("notes_grille", Some(&admin), json!({ "id": e["id"] }))
            .await;
        for (i, l) in grille["lignes"].as_array().unwrap().iter().enumerate() {
            p.ok(
                "note_saisir",
                Some(&admin),
                json!({ "evaluation_id": e["id"], "inscription_id": l["inscription_id"],
                    "saisie": (3 + i % 8).to_string() }),
            )
            .await;
        }
        p.ok("evaluation_valider", Some(&admin), json!({ "id": e["id"] }))
            .await;
        let debut = Instant::now();
        p.ok(
            "resultats_calculer",
            Some(&admin),
            json!({ "classe_id": classe, "periode_id": periode }),
        )
        .await;
        let duree = debut.elapsed();
        duree_bulletins += duree;
        plus_long = plus_long.max(duree);
    }
    println!(
        "Bulletins des 40 classes : {:.1} s en tout, {:.2} s pour la plus longue",
        duree_bulletins.as_secs_f64(),
        plus_long.as_secs_f64()
    );
    assert!(plus_long <= Duration::from_secs(10));

    let debut = Instant::now();
    let impayes = p.ok("impayes_lister", Some(&admin), Value::Null).await;
    println!(
        "{} élèves en retard de paiement",
        impayes["lignes"].as_array().unwrap().len()
    );
    mesurer("Liste des impayés", debut, Duration::from_secs(30));

    let debut = Instant::now();
    let fin = p.ok("fin_annee_lire", Some(&admin), Value::Null).await;
    assert_eq!(fin["classes"].as_array().unwrap().len(), 40);
    mesurer("Fin d'année", debut, Duration::from_secs(30));
}
