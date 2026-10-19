//! Import du programme depuis Excel ou CSV : le modèle part du programme en vigueur et se
//! réimporte tel quel ; un fichier corrigé récrit le programme des niveaux qu'il nomme, crée les
//! matières qui manquent, et laisse tel quel un niveau dont une ligne est fausse.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod commun;

use commun::*;
use serde_json::{Value, json};

fn id_par(liste: &Value, champ: &str, valeur: &str) -> Value {
    liste
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x[champ] == valeur)
        .unwrap_or_else(|| panic!("{champ} = {valeur} absent"))["id"]
        .clone()
}

/// Les lignes du programme d'un niveau, dans l'ordre du bulletin.
fn programme_de(ped: &Value, niveau_id: &Value) -> Vec<Value> {
    let mut lignes: Vec<Value> = ped["programme"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|l| l["niveau_id"] == *niveau_id)
        .cloned()
        .collect();
    lignes.sort_by_key(|l| l["ordre"].as_i64());
    lignes
}

#[tokio::test]
async fn le_programme_s_importe_d_un_classeur() {
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
    let ped = p
        .ok("pedagogie_lire", Some(&admin), json!({ "id": annee["id"] }))
        .await;
    let (premiere, septieme, huitieme) = (
        id_par(&ped["niveaux"], "code", "1"),
        id_par(&ped["niveaux"], "code", "7"),
        id_par(&ped["niveaux"], "code", "8"),
    );
    let premiere_avant = programme_de(&ped, &premiere).len();
    assert!(premiere_avant > 0);

    // Le modèle part du programme en vigueur et se réimporte sans erreur.
    let modele = dossier.path().join("programme.xlsx");
    p.ok(
        "programme_import_modele",
        Some(&admin),
        json!({ "chemin": modele.to_string_lossy() }),
    )
    .await;
    let relu = p
        .ok(
            "programme_import_analyser",
            Some(&admin),
            json!({ "chemin": modele.to_string_lossy() }),
        )
        .await;
    assert_eq!(relu["niveaux_ignores"], 0, "{relu}");
    assert!(relu["niveaux_prets"].as_u64().unwrap() >= 9);
    assert!(relu["nouvelles_matieres"].as_array().unwrap().is_empty());

    // Un fichier corrigé par l'école.
    let fichier = dossier.path().join("programme-corrige.csv");
    std::fs::write(
        &fichier,
        "Niveau;Série;Matière;Coefficient;Heures par semaine\n\
         7e;;Français;4;6\n\
         7e année;;Mathématiques;4;5\n\
         7e;;Bambara;1;1h30\n\
         7e;;ANG;;\n\
         8ème;;Français;3;5\n\
         8ème;;Sciences de la vie et de la terre;2;1,5\n\
         1re;;Lecture;2;\n\
         Xe;;Français;2;\n",
    )
    .unwrap();
    let chemin = json!({ "chemin": fichier.to_string_lossy() });
    let analyse = p
        .ok("programme_import_analyser", Some(&admin), chemin.clone())
        .await;
    assert_eq!(analyse["niveaux_prets"], 2, "{analyse}");
    // La 1re a un coefficient (il n'y en a pas au 1er cycle) ; « Xe » n'est aucun niveau.
    assert_eq!(analyse["niveaux_ignores"], 2);
    assert_eq!(analyse["nouvelles_matieres"], json!(["Bambara"]));
    let lignes = analyse["lignes"].as_array().unwrap();
    assert_eq!(lignes[2]["minutes_hebdo"], 90);
    assert_eq!(lignes[3]["matiere"], "Anglais", "reconnue à son code");
    assert_eq!(
        lignes[6]["erreurs"],
        json!(["import_programme.coefficient_premier_cycle"])
    );
    assert_eq!(
        lignes[7]["erreurs"],
        json!(["import_programme.niveau_inconnu"])
    );

    let bilan = p
        .ok("programme_import_appliquer", Some(&admin), chemin)
        .await;
    assert_eq!(
        (
            bilan["niveaux"].clone(),
            bilan["matieres_creees"].clone(),
            bilan["niveaux_ignores"].clone()
        ),
        (json!(2), json!(1), json!(2))
    );
    let ped = p
        .ok("pedagogie_lire", Some(&admin), json!({ "id": annee["id"] }))
        .await;
    let code_de = |id: &Value| {
        ped["matieres"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["id"] == *id)
            .map(|m| m["code"].as_str().unwrap().to_owned())
            .unwrap()
    };
    let septieme_apres = programme_de(&ped, &septieme);
    let codes: Vec<String> = septieme_apres
        .iter()
        .map(|l| code_de(&l["matiere_id"]))
        .collect();
    assert_eq!(codes, ["FR", "MATH", "BAMBA", "ANG"]);
    assert_eq!(septieme_apres[0]["coefficient"], 4);
    assert_eq!(septieme_apres[0]["minutes_hebdo"], 360);
    assert_eq!(septieme_apres[3]["coefficient"], Value::Null);
    let huitieme_apres = programme_de(&ped, &huitieme);
    assert_eq!(huitieme_apres.len(), 2);
    assert_eq!(huitieme_apres[1]["minutes_hebdo"], 90);
    // La 1re, laissée telle quelle.
    assert_eq!(programme_de(&ped, &premiere).len(), premiere_avant);

    // Un fichier sans les colonnes attendues est refusé.
    let mauvais = dossier.path().join("mauvais.csv");
    std::fs::write(&mauvais, "Classe;Cours\n7e;Français\n").unwrap();
    let refus = p
        .refus(
            "programme_import_analyser",
            Some(&admin),
            json!({ "chemin": mauvais.to_string_lossy() }),
        )
        .await;
    assert!(
        format!("{refus:?}").contains("import_programme.colonnes"),
        "{refus:?}"
    );
}
