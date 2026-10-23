//! Import des élèves par la frontière (SPEC_PHASE_2 §10, assertion 11) : un classeur de quinze
//! élèves avec deux erreurs et un doublon, analysé, validé, appliqué après sauvegarde.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod commun;

use commun::*;
use gesco_docs::tableur::{Liste, classeur};
use gesco_services::session::Jeton;
use serde_json::{Value, json};

/// Poste installé, administrateur connecté, année 2026-2027 ouverte, classes 7e A et 8e A.
async fn ecole(dossier: &std::path::Path) -> (Poste_, Jeton) {
    let p = poste_vierge(dossier).await;
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
    let referentiel = p.ok("niveaux_lister", Some(&admin), Value::Null).await;
    let niveau = |code: &str| {
        referentiel["niveaux"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["code"] == code)
            .unwrap()["id"]
            .clone()
    };
    for (code, libelle) in [("7", "7e A"), ("8", "8e A")] {
        p.ok(
            "classe_creer",
            Some(&admin),
            json!({ "annee_id": annee["id"], "niveau_id": niveau(code), "serie_id": null, "libelle": libelle }),
        )
        .await;
    }
    (p, admin)
}

fn ligne(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| (*s).to_owned()).collect()
}

/// Quinze lignes : treize bonnes, une sans date (erreur), une dans une classe inconnue
/// (erreur), une en doublon d'une autre ligne du fichier (avertissement), une avec une année
/// seule (avertissement).
fn fichier() -> Vec<Vec<String>> {
    let mut lignes = vec![ligne(&[
        "N°",
        "Nom et prénoms",
        "Sexe",
        "Né(e) le",
        "Lieu de naissance",
        "Classe",
        "Téléphone du parent",
    ])];
    let eleves = [
        (
            "TRAORE Awa",
            "F",
            "10/05/2014",
            "Kati",
            "7e A",
            "76 12 34 56",
        ),
        (
            "DIALLO Moussa",
            "M",
            "2013-11-02",
            "Bamako",
            "7e A",
            "66 00 11 22",
        ),
        ("KEITA Fatoumata", "F", "01/01/2014", "Ségou", "7e A", ""),
        (
            "COULIBALY Seydou",
            "M",
            "vers 2013",
            "",
            "7e A",
            "76 12 34 56",
        ),
        (
            "SANGARE Oumar",
            "M",
            "20/01/2013",
            "Ségou",
            "8e A",
            "70 00 00 01",
        ),
        (
            "DEMBELE Mariam",
            "F",
            "03/03/2013",
            "Koutiala",
            "8e A",
            "70 00 00 02",
        ),
        ("KONE Adama", "M", "", "Sikasso", "8e A", "70 00 00 03"),
        (
            "TOURE Aminata",
            "F",
            "14/07/2013",
            "Mopti",
            "9e A",
            "70 00 00 04",
        ),
        (
            "SIDIBE Ibrahim",
            "M",
            "05/05/2014",
            "Bamako",
            "7e A",
            "70 00 00 05",
        ),
        (
            "CISSE Kadiatou",
            "F",
            "06/06/2014",
            "Bamako",
            "7e A",
            "70 00 00 06",
        ),
        (
            "TRAORE Awa",
            "F",
            "10/05/2014",
            "Kati",
            "7e A",
            "76 12 34 56",
        ),
        (
            "MAIGA Hamidou",
            "M",
            "07/07/2013",
            "Gao",
            "8e A",
            "70 00 00 07",
        ),
        (
            "DIARRA Rokia",
            "F",
            "08/08/2013",
            "Bamako",
            "8e A",
            "70 00 00 08",
        ),
        (
            "SAMAKE Bakary",
            "M",
            "09/09/2014",
            "Bamako",
            "7e A",
            "70 00 00 09",
        ),
        (
            "BAGAYOKO Oumou",
            "F",
            "10/10/2014",
            "Bamako",
            "7e A",
            "70 00 00 10",
        ),
    ];
    for (i, (nom, sexe, date, lieu, classe, tel)) in eleves.iter().enumerate() {
        lignes.push(ligne(&[
            &(i + 1).to_string(),
            nom,
            sexe,
            date,
            lieu,
            classe,
            tel,
        ]));
    }
    lignes
}

#[tokio::test]
async fn un_classeur_de_quinze_eleves_s_importe_apres_sauvegarde() {
    let dossier = tempfile::tempdir().unwrap();
    let (p, admin) = ecole(dossier.path()).await;
    let lignes = fichier();
    let entetes = lignes[0].clone();
    let octets = classeur(&Liste {
        titre: "Élèves",
        entetes: &entetes,
        lignes: &lignes[1..],
    })
    .unwrap();
    let chemin = dossier.path().join("eleves.xlsx");
    std::fs::write(&chemin, octets).unwrap();

    // 1. Analyse : colonnes reconnues, disposition, classes de l'année.
    let analyse = p
        .ok(
            "import_eleves_analyser",
            Some(&admin),
            json!({ "chemin": chemin.to_string_lossy() }),
        )
        .await;
    assert_eq!(
        analyse["champs"],
        json!([
            "ignorer",
            "nom_et_prenoms",
            "sexe",
            "date_naissance",
            "lieu_naissance",
            "classe",
            "responsable_telephone"
        ])
    );
    assert_eq!(analyse["disposition"], "colonne_classe");
    assert_eq!(analyse["feuilles"][0]["lignes"], 15);
    assert_eq!(analyse["classes"], json!(["7e A", "8e A"]));
    assert_eq!(analyse["matricule_manuel"], false);
    let reglages = json!({
        "fichier_id": analyse["fichier_id"],
        "champs": analyse["champs"],
        "disposition": "colonne_classe",
        "decoupe": "nom_d_abord",
    });

    // 2. Aperçu : deux erreurs, deux avertissements, onze lignes valides.
    let apercu = p
        .ok("import_eleves_valider", Some(&admin), reglages.clone())
        .await;
    assert_eq!(apercu["valides"], 11);
    assert_eq!(apercu["avertissements"], 2);
    assert_eq!(apercu["erreurs"], 2);
    assert_eq!(apercu["classes_absentes"], json!(["9e A"]));
    let lignes_apercu = apercu["lignes"].as_array().unwrap();
    assert_eq!(lignes_apercu.len(), 15);
    let par_nom = |nom: &str| {
        lignes_apercu
            .iter()
            .filter(|l| l["nom"] == nom)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        par_nom("KONE")[0]["motifs"],
        json!(["import.date_manquante"])
    );
    assert_eq!(
        par_nom("TOURE")[0]["motifs"],
        json!(["import.classe_absente"])
    );
    let coulibaly = par_nom("COULIBALY");
    assert_eq!(coulibaly[0]["etat"], "avertissement");
    assert_eq!(
        coulibaly[0]["motifs"],
        json!(["import.date_estimee", "import.lieu_manquant"])
    );
    let awa = par_nom("TRAORE");
    assert_eq!(awa[0]["etat"], "valide");
    assert_eq!(awa[1]["motifs"], json!(["import.doublon_fichier"]));
    assert_eq!(awa[1]["doublon_de"], "Élèves · 2");
    assert_eq!(awa[0]["date_naissance"], "2014-05-10");

    // Un fichier qui n'est pas le sien, ou une correspondance bancale, sont refusés.
    let mut autre = reglages.clone();
    autre["fichier_id"] = json!("inconnu");
    assert_eq!(
        p.refus("import_eleves_valider", Some(&admin), autre)
            .await
            .cle,
        "import.fichier_inconnu"
    );
    let mut sans_nom = reglages.clone();
    sans_nom["champs"][1] = json!("ignorer");
    assert_eq!(
        p.refus("import_eleves_valider", Some(&admin), sans_nom)
            .await
            .cle,
        "import.nom_absent"
    );

    // 3. Application : sauvegarde d'abord, puis treize élèves, leurs brouillons, leurs
    // responsables (un même téléphone donne un seul responsable : Awa, son doublon et Seydou).
    let avant = p.ok("sauvegardes_lister", Some(&admin), Value::Null).await;
    let rapport = p
        .ok("import_eleves_appliquer", Some(&admin), reglages.clone())
        .await;
    assert_eq!(rapport["importes"], 13);
    assert_eq!(rapport["ignorees"], 2);
    assert_eq!(rapport["responsables_crees"], 10);
    assert_eq!(rapport["responsables_retrouves"], 2);
    let apres = p.ok("sauvegardes_lister", Some(&admin), Value::Null).await;
    assert_eq!(
        apres.as_array().unwrap().len(),
        avant.as_array().unwrap().len() + 1
    );

    let eleves = p
        .ok("eleves_rechercher", Some(&admin), json!({ "texte": "" }))
        .await;
    assert_eq!(eleves.as_array().unwrap().len(), 13);
    let seydou = eleves
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["nom"] == "COULIBALY")
        .unwrap();
    let fiche = p
        .ok("eleve_lire", Some(&admin), json!({ "id": seydou["id"] }))
        .await;
    assert_eq!(fiche["identite"]["date_naissance_estimee"], true);
    assert_eq!(fiche["inscriptions"].as_array().unwrap().len(), 1);
    assert_eq!(
        fiche["inscriptions"][0]["inscription"]["statut"],
        "brouillon"
    );
    assert_eq!(fiche["inscriptions"][0]["classe"], "7e A");
    // Seydou et Awa partagent le téléphone 76 12 34 56 : un seul responsable, lié aux deux.
    assert_eq!(fiche["responsables"].as_array().unwrap().len(), 1);
    assert_eq!(fiche["responsables"][0]["identite"]["nom"], "TRAORE");
    assert_eq!(
        fiche["responsables"][0]["identite"]["telephones"],
        json!(["+22376123456"])
    );
    assert!(
        fiche["eleve"]["matricule_interne"]
            .as_str()
            .unwrap()
            .starts_with("2026/")
    );

    // Le fichier appliqué est oublié : une seconde application est refusée.
    assert_eq!(
        p.refus("import_eleves_appliquer", Some(&admin), reglages)
            .await
            .cle,
        "import.fichier_inconnu"
    );

    // Un CSV au point-virgule se lit aussi, et un format inconnu est refusé.
    let csv = dossier.path().join("eleves.csv");
    std::fs::write(
        &csv,
        "Nom;Prénoms;Sexe;Date de naissance;Classe\r\nTOURE;Aminata;F;14/07/2013;8e A\r\n",
    )
    .unwrap();
    let analyse_csv = p
        .ok(
            "import_eleves_analyser",
            Some(&admin),
            json!({ "chemin": csv.to_string_lossy() }),
        )
        .await;
    assert_eq!(
        analyse_csv["champs"],
        json!(["nom", "prenoms", "sexe", "date_naissance", "classe"])
    );
    let pdf = dossier.path().join("eleves.pdf");
    std::fs::write(&pdf, b"%PDF-").unwrap();
    assert_eq!(
        p.refus(
            "import_eleves_analyser",
            Some(&admin),
            json!({ "chemin": pdf.to_string_lossy() })
        )
        .await
        .cle,
        "import.format_inconnu"
    );
}
