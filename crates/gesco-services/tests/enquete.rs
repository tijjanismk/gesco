//! Données pour l'enquête et tableau libre par la frontière (SPEC_PHASE_9 §6, §12 étape 8,
//! critères 3, 8 et 9) : le classeur a une feuille par partie du questionnaire, dans son ordre,
//! sur une école vide comme sur une école remplie ; une rubrique ajoutée par l'école se retrouve
//! dans le classeur et dans le tableau libre ; le total du tableau libre compte aussi les fiches
//! qui n'entrent dans aucune colonne.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use calamine::{Data, Reader, Xlsx};
use serde_json::{Value, json};
use sqlx::Connection;

use crate::commun::*;

const FEUILLES: [&str; 14] = [
    "I. Identification",
    "II. Informations générales",
    "III. Locaux",
    "IV. Mobilier et équipements",
    "V. Données financières",
    "VI.1 Personnel",
    "VI.2 Disciplines",
    "VII. Nouveaux inscrits",
    "VIII. Élèves",
    "IX. Élèves handicapés",
    "X. Accessibilité",
    "XI. Handicap, personnel",
    "XII. Résultats scolaires",
    "XIII. Observations",
];

fn id_par(liste: &Value, champ: &str, valeur: &str) -> Value {
    liste
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x[champ] == valeur)
        .unwrap_or_else(|| panic!("{champ} = {valeur} introuvable"))["id"]
        .clone()
}

/// Les lignes d'une feuille du classeur, en texte.
fn feuille(chemin: &std::path::Path, nom: &str) -> Vec<Vec<String>> {
    let mut classeur = Xlsx::new(std::io::Cursor::new(std::fs::read(chemin).unwrap())).unwrap();
    let plage = classeur.worksheet_range(nom).unwrap();
    plage
        .rows()
        .map(|l| {
            l.iter()
                .map(|c| match c {
                    Data::String(s) => s.clone(),
                    Data::Empty => String::new(),
                    autre => autre.to_string(),
                })
                .collect()
        })
        .collect()
}

fn noms_feuilles(chemin: &std::path::Path) -> Vec<String> {
    let classeur = Xlsx::new(std::io::Cursor::new(std::fs::read(chemin).unwrap())).unwrap();
    classeur.sheet_names()
}

fn ligne<'a>(lignes: &'a [Vec<String>], premiere: &str) -> &'a [String] {
    lignes
        .iter()
        .find(|l| l.first().map(String::as_str) == Some(premiere))
        .unwrap_or_else(|| panic!("ligne « {premiere} » introuvable"))
}

#[tokio::test]
async fn le_classeur_de_l_enquete_et_le_tableau_libre() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;

    // Une école vide : le classeur sort, une feuille par partie, sans refus.
    let vide = dossier.path().join("vide.xlsx");
    p.ok(
        "statistiques_enquete",
        Some(&admin),
        json!({ "annee_id": null, "date": null, "chemin": vide.to_string_lossy() }),
    )
    .await;
    assert_eq!(noms_feuilles(&vide), FEUILLES);
    let locaux = feuille(&vide, "III. Locaux");
    assert_eq!(locaux[0][0], "Type de local");
    let observations = feuille(&vide, "XIII. Observations");
    assert_eq!(
        ligne(&observations, "Observations")[1],
        "À remplir à la main"
    );
    let libre_vide = p
        .ok(
            "statistiques_tableau_libre",
            Some(&admin),
            json!({ "annee_id": null, "date": null, "cycle": null, "population": "eleves",
                "lignes": "classe", "colonnes": "sexe" }),
        )
        .await;
    assert!(
        libre_vide["tableau"]["lignes"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(libre_vide["tableau"]["total"], Value::Null);

    // Une année, une classe de 10e, trois élèves dont un redoublant et une fiche sans sexe.
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
    let classe = p
        .ok(
            "classe_creer",
            Some(&admin),
            json!({ "annee_id": annee["id"], "niveau_id": id_par(&ped["niveaux"], "code", "10"),
                "serie_id": null, "libelle": "10e A" }),
        )
        .await["id"]
        .clone();
    for (prenoms, sexe, naissance, redoublant) in [
        ("Awa", "feminin", "2011-03-02", false),
        ("Bakary", "masculin", "2010-06-15", true),
        ("Issa", "masculin", "2011-01-20", false),
    ] {
        let fiche = p
            .ok(
                "eleve_creer",
                Some(&admin),
                json!({ "identite": { "nom": "Traoré", "prenoms": prenoms, "sexe": sexe,
                    "date_naissance": naissance, "date_naissance_estimee": false,
                    "lieu_naissance": "Bamako", "nationalite": "Malienne", "adresse": "",
                    "telephones": [] },
                    "matricule_interne": "", "matricule_national": matricule_national(), "acte_nature": "acte",
                    "acte_numero": "", "acte_date": "", "ecole_origine": "" }),
            )
            .await;
        p.ok(
            "responsable_rattacher",
            Some(&admin),
            json!({ "eleve_id": fiche["eleve"]["id"], "responsable_id": null,
                "saisie": { "identite": { "nom": "Coulibaly", "prenoms": "Mariam",
                    "sexe": "feminin", "date_naissance": "", "date_naissance_estimee": false,
                    "lieu_naissance": "", "nationalite": "", "adresse": "", "telephones": ["76 12 34 56"] },
                    "profession": "", "employeur": "" },
                "qualites": { "lien": "mere", "est_responsable_legal": true, "est_payeur": true,
                    "recoit_notifications": true, "vient_chercher": false, "ordre_priorite": 1 } }),
        )
        .await;
        let inscription = p
            .ok(
                "inscription_creer",
                Some(&admin),
                json!({ "eleve_id": fiche["eleve"]["id"], "classe_id": classe,
                    "date_inscription": "2026-10-01", "redoublant": redoublant,
                    "etablissement_origine": "" }),
            )
            .await["inscription"]["id"]
            .clone();
        p.ok(
            "inscription_valider",
            Some(&admin),
            json!({ "id": inscription, "choix": {} }),
        )
        .await;
    }
    let mut cnx = base_brute(dossier.path(), p.trousseau.clone()).await;
    sqlx::query("UPDATE personne SET sexe = NULL WHERE prenoms = 'Issa'")
        .execute(&mut cnx)
        .await
        .unwrap();
    cnx.close().await.unwrap();

    // La fiche : une réponse livrée, une rubrique ajoutée par l'école.
    let fiche = p
        .ok(
            "fiche_ecole_lire",
            Some(&admin),
            json!({ "annee_id": annee["id"] }),
        )
        .await;
    let cercle = id_par(&fiche["rubriques"], "code", "cercle");
    p.ok(
        "fiche_ecole_repondre",
        Some(&admin),
        json!({ "rubrique_id": cercle, "annee_id": null, "valeur": "Kati" }),
    )
    .await;
    let ajoutee = p
        .ok(
            "rubrique_fiche_enregistrer",
            Some(&admin),
            json!({ "id": null, "theme": "autres", "libelle": "Jardin potager", "aide": "",
                "type_reponse": "oui_non", "choix": [], "annuelle": false }),
        )
        .await;
    let potager = ajoutee["id"].clone();
    p.ok(
        "fiche_ecole_repondre",
        Some(&admin),
        json!({ "rubrique_id": potager, "annee_id": null, "valeur": "oui" }),
    )
    .await;

    let chemin = dossier.path().join("enquete.xlsx");
    p.ok(
        "statistiques_enquete",
        Some(&admin),
        json!({ "annee_id": annee["id"], "date": "2026-11-15", "chemin": chemin.to_string_lossy() }),
    )
    .await;
    assert_eq!(noms_feuilles(&chemin), FEUILLES);
    let identification = feuille(&chemin, "I. Identification");
    assert_eq!(ligne(&identification, "Cercle")[1], "Kati");
    let generales = feuille(&chemin, "II. Informations générales");
    assert_eq!(ligne(&generales, "Jardin potager")[1], "oui");
    // Le total compte la fiche sans sexe ; garçons et filles non.
    let eleves = feuille(&chemin, "VIII. Élèves");
    let dixieme = ligne(&eleves, "10e année");
    assert_eq!(dixieme[2..9], ["1", "1", "1", "3", "1", "0", "1"]);
    let nouveaux = feuille(&chemin, "VII. Nouveaux inscrits");
    assert_eq!(ligne(&nouveaux, "Total")[1..4], ["0", "1", "2"]);
    assert_eq!(ligne(&nouveaux, "15 ans")[1..4], ["0", "1", "2"]);

    // Le tableau libre : classe par sexe ; le total compte la fiche sans sexe, et l'écran la
    // signale d'une ligne neutre.
    let libre = p
        .ok(
            "statistiques_tableau_libre",
            Some(&admin),
            json!({ "annee_id": annee["id"], "date": "2026-11-15", "cycle": null,
                "population": "eleves", "lignes": "classe", "colonnes": "sexe" }),
        )
        .await;
    let t = &libre["tableau"];
    assert_eq!(
        t["colonnes"],
        json!([
            "cle:statistiques.colonne.garcons",
            "cle:statistiques.colonne.filles",
            "total"
        ])
    );
    assert_eq!(t["lignes"][0]["libelle"], "10e A");
    let nombres = |l: &Value| -> Vec<u64> {
        l["cellules"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["nombre"].as_u64().unwrap())
            .collect()
    };
    assert_eq!(nombres(&t["lignes"][0]), [1, 1, 3]);
    assert_eq!(nombres(&t["total"]), [1, 1, 3]);
    assert_eq!(t["hors"][0]["cle"], "statistiques.hors.libre_eleves");
    assert_eq!(t["hors"][0]["membres"].as_array().unwrap().len(), 1);
    assert_eq!(libre["fiches"].as_array().unwrap().len(), 3);

    // Par âge, sans colonnes : une seule colonne, le total.
    let ages = p
        .ok(
            "statistiques_tableau_libre",
            Some(&admin),
            json!({ "annee_id": annee["id"], "date": "2026-11-15", "cycle": null,
                "population": "eleves", "lignes": "age", "colonnes": null }),
        )
        .await;
    assert_eq!(ages["tableau"]["colonnes"], json!(["total"]));
    let par_age: Vec<(String, u64)> = ages["tableau"]["lignes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| {
            (
                l["libelle"].as_str().unwrap().to_owned(),
                l["cellules"][0]["nombre"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(par_age, [("15".to_owned(), 2), ("16".to_owned(), 1)]);

    // La fiche : la rubrique ajoutée et sa réponse.
    let rubriques = p
        .ok(
            "statistiques_tableau_libre",
            Some(&admin),
            json!({ "annee_id": annee["id"], "date": null, "cycle": null,
                "population": "fiche", "lignes": "classe", "colonnes": null }),
        )
        .await;
    let potager = rubriques["tableau"]["lignes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["libelle"] == "Jardin potager")
        .unwrap();
    assert_eq!(potager["cellules"][0]["texte"], "oui");

    // Sans session, rien.
    let sans_session = p
        .refus(
            "statistiques_tableau_libre",
            None,
            json!({ "annee_id": null, "date": null, "cycle": null, "population": "eleves",
                "lignes": "classe", "colonnes": null }),
        )
        .await;
    assert!(!sans_session.cle.is_empty());
}
