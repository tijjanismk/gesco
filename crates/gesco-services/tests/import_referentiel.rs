//! Import des classes et des matières seules, sans le programme : le modèle part de ce que
//! l'école a et se réimporte sans rien créer ; un fichier corrigé crée les classes de l'année et
//! les matières qui manquent, laisse ce qui existe, et montre les lignes à corriger.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use crate::commun::*;
use gesco_services::session::Jeton;
use serde_json::{Value, json};

async fn ecole(p: &Poste_) -> (Jeton, Value) {
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
    (admin, annee)
}

#[tokio::test]
async fn les_classes_et_les_matieres_s_importent_sans_le_programme() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    let (admin, annee) = ecole(&p).await;
    let ped = p
        .ok("pedagogie_lire", Some(&admin), json!({ "id": annee["id"] }))
        .await;
    let programme_avant = ped["programme"].as_array().unwrap().len();

    // Le modèle, à deux feuilles, se réimporte sans rien créer : tout y est déjà.
    let modele = dossier.path().join("classes-et-matieres.xlsx");
    p.ok(
        "referentiel_import_modele",
        Some(&admin),
        json!({ "chemin": modele.to_string_lossy() }),
    )
    .await;
    let relu = p
        .ok(
            "referentiel_import_analyser",
            Some(&admin),
            json!({ "chemin": modele.to_string_lossy() }),
        )
        .await;
    assert_eq!(relu["matieres_a_creer"], 0, "{relu}");
    assert!(
        relu["lignes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|l| l["deja"] == true)
    );

    // Des classes.
    let classes = dossier.path().join("classes.csv");
    std::fs::write(
        &classes,
        "Niveau;Série;Classe\n7e;;7e A\n7e année;;7e B\n9e;;9e A\n7e;;7e A\nCAP 1re année;;CAP 1\n11e;;11e A\n",
    )
    .unwrap();
    let analyse = p
        .ok(
            "referentiel_import_analyser",
            Some(&admin),
            json!({ "chemin": classes.to_string_lossy() }),
        )
        .await;
    let lignes = analyse["lignes"].as_array().unwrap();
    assert_eq!(analyse["classes_a_creer"], 3, "{analyse}");
    assert_eq!(
        lignes[3]["erreurs"],
        json!(["import_referentiel.en_double"])
    );
    // L'enseignement technique n'est pas ouvert ; au lycée, la série manque.
    assert!(
        lignes[4]["erreurs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e == "classe.niveau_inactif")
    );
    assert_eq!(
        lignes[5]["erreurs"],
        json!(["import_programme.serie_inconnue"])
    );
    let bilan = p
        .ok(
            "referentiel_import_appliquer",
            Some(&admin),
            json!({ "chemin": classes.to_string_lossy() }),
        )
        .await;
    assert_eq!(bilan["classes"], 3, "{bilan}");
    let liste = p
        .ok(
            "classes_lister",
            Some(&admin),
            json!({ "annee_id": annee["id"] }),
        )
        .await;
    let noms: Vec<String> = liste
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["classe"]["libelle"].as_str().unwrap().to_owned())
        .collect();
    for nom in ["7e A", "7e B", "9e A"] {
        assert!(noms.iter().any(|n| n == nom), "{nom} absente de {noms:?}");
    }
    // Réimporté, le même fichier ne crée plus rien.
    let encore = p
        .ok(
            "referentiel_import_analyser",
            Some(&admin),
            json!({ "chemin": classes.to_string_lossy() }),
        )
        .await;
    assert_eq!(encore["classes_a_creer"], 0);

    // Des matières : une nouvelle avec son code, une sans code, une existante, une fausse.
    let matieres = dossier.path().join("matieres.csv");
    std::fs::write(
        &matieres,
        "Matière;Code;Nom court;Groupe\nBambara;BAM;Bambara;Littéraire\nInformatique;;;Scientifique\nFrançais;;;\nCouture;COUT;;Ménager\n",
    )
    .unwrap();
    let analyse = p
        .ok(
            "referentiel_import_analyser",
            Some(&admin),
            json!({ "chemin": matieres.to_string_lossy() }),
        )
        .await;
    let lignes = analyse["lignes"].as_array().unwrap();
    assert_eq!(analyse["matieres_a_creer"], 2, "{analyse}");
    assert_eq!(lignes[2]["deja"], true);
    assert_eq!(
        lignes[3]["erreurs"],
        json!(["import_referentiel.groupe_inconnu"])
    );
    let bilan = p
        .ok(
            "referentiel_import_appliquer",
            Some(&admin),
            json!({ "chemin": matieres.to_string_lossy() }),
        )
        .await;
    assert_eq!(
        (bilan["matieres"].clone(), bilan["ignorees"].clone()),
        (json!(2), json!(2))
    );
    let ped = p
        .ok("pedagogie_lire", Some(&admin), json!({ "id": annee["id"] }))
        .await;
    let codes: Vec<&str> = ped["matieres"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["code"].as_str().unwrap())
        .collect();
    assert!(
        codes.contains(&"BAM") && codes.contains(&"INFOR"),
        "{codes:?}"
    );
    // Le programme n'a pas bougé.
    assert_eq!(ped["programme"].as_array().unwrap().len(), programme_avant);
}

/// Le classeur d'exemple livré dans `docs/exemples` s'importe sans ligne à corriger.
#[tokio::test]
async fn l_exemple_des_classes_et_matieres_s_importe() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    let (admin, _) = ecole(&p).await;
    let exemple = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/exemples/classes-et-matieres.xlsx");
    let chemin = json!({ "chemin": exemple.to_string_lossy() });
    let analyse = p
        .ok("referentiel_import_analyser", Some(&admin), chemin.clone())
        .await;
    assert!(
        analyse["lignes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|l| l["erreurs"].as_array().unwrap().is_empty()),
        "{analyse}"
    );
    let bilan = p
        .ok("referentiel_import_appliquer", Some(&admin), chemin)
        .await;
    assert!(bilan["classes"].as_u64().unwrap() >= 9, "{bilan}");
    assert!(bilan["matieres"].as_u64().unwrap() >= 1);
}
