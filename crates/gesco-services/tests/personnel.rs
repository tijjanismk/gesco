//! Registre du personnel par la frontière (SPEC_PHASE_6 §2) : catégories et fonctions de l'école
//! sans liste fermée (T-39), matricules, fonctionnaires sans service d'origine (T-40), enseignants
//! rattachés (Q-63), départ et retour, pièces du dossier.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use crate::commun::*;
use serde_json::{Value, json};

fn par_code(liste: &Value, code: &str) -> Value {
    liste
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["code"] == code)
        .unwrap_or_else(|| panic!("code {code} absent"))["id"]
        .clone()
}

fn membre(nom: &str, prenoms: &str, categorie: &Value, fonction: &str) -> Value {
    json!({
        "id": null, "nom": nom, "prenoms": prenoms, "sexe": "feminin", "date_naissance": null,
        "telephones": ["76 11 22 33"], "adresse": "Hamdallaye", "matricule": "",
        "categorie_id": categorie, "fonction": fonction, "arrive_le": "2024-10-01",
        "fonctionnaire": false, "numero_fonctionnaire": "", "numero_inps": "",
        "prevenir_nom": "", "prevenir_telephone": "", "paiement": { "mode": "especes" },
        "diplomes": [], "utilisateur_id": null,
    })
}

#[tokio::test]
async fn le_registre_tient_tout_le_personnel_avec_les_fonctions_de_l_ecole() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;

    // Catégories et fonctions livrées, que l'école complète (T-39).
    let referentiel = p
        .ok("personnel_referentiel_lire", Some(&admin), Value::Null)
        .await;
    assert_eq!(referentiel["categories"].as_array().unwrap().len(), 3);
    assert_eq!(referentiel["fonctions"].as_array().unwrap().len(), 13);
    let enseignement = par_code(&referentiel["categories"], "enseignement");
    let administration = par_code(&referentiel["categories"], "administration");
    let appui = par_code(&referentiel["categories"], "appui");
    let veilleur = p
        .ok(
            "fonction_personnel_enregistrer",
            Some(&admin),
            json!({ "id": null, "categorie_id": appui, "libelle": "Veilleur de nuit", "active": true }),
        )
        .await;
    assert_eq!(veilleur["code"], Value::Null);
    assert_eq!(
        p.refus(
            "fonction_personnel_enregistrer",
            Some(&admin),
            json!({ "id": null, "categorie_id": appui, "libelle": "veilleur de nuit", "active": true }),
        )
        .await
        .cle,
        "personnel.libelle_existe"
    );

    // Une secrétaire, une cuisinière, et un gardien à la fonction écrite librement.
    let secretaire = p
        .ok(
            "membre_enregistrer",
            Some(&admin),
            membre("Coulibaly", "Awa", &administration, "Secrétaire"),
        )
        .await;
    assert_eq!(secretaire["membre"]["matricule"], "P-0001");
    assert_eq!(secretaire["personne"]["telephones"][0], "+22376112233");
    let cuisiniere = p
        .ok(
            "membre_enregistrer",
            Some(&admin),
            membre("Diarra", "Kadiatou", &appui, "Cuisinière"),
        )
        .await;
    assert_eq!(cuisiniere["membre"]["matricule"], "P-0002");
    let mut gardien = membre("Keïta", "Moussa", &appui, "Gardien et jardinier");
    gardien["sexe"] = json!("masculin");
    gardien["matricule"] = json!("G-01");
    gardien["paiement"] = json!({ "mode": "mobile_money", "numero": "66 00 11 22" });
    gardien["diplomes"] =
        json!([{ "intitule": "CAP maçonnerie", "delivre_par": "CFP de Bamako", "annee": 2009 }]);
    let gardien = p.ok("membre_enregistrer", Some(&admin), gardien).await;
    assert_eq!(gardien["membre"]["matricule"], "G-01");
    assert_eq!(gardien["membre"]["paiement"]["numero"], "+22366001122");
    assert_eq!(
        gardien["membre"]["diplomes"][0]["intitule"],
        "CAP maçonnerie"
    );
    // Un matricule pris, une fonction vide, un compte bancaire incomplet : refusés.
    let mut doublon = membre("Traoré", "Fanta", &appui, "Agent d'entretien");
    doublon["matricule"] = json!("G-01");
    assert_eq!(
        p.refus("membre_enregistrer", Some(&admin), doublon)
            .await
            .cle,
        "personnel.matricule_pris"
    );
    assert_eq!(
        p.refus(
            "membre_enregistrer",
            Some(&admin),
            membre("Traoré", "Fanta", &appui, " ")
        )
        .await
        .cle,
        "personnel.fonction_obligatoire"
    );
    let mut banque = membre("Traoré", "Fanta", &appui, "Agent d'entretien");
    banque["paiement"] = json!({ "mode": "banque", "banque": "BDM", "compte": "" });
    assert_eq!(
        p.refus("membre_enregistrer", Some(&admin), banque)
            .await
            .cle,
        "personnel.compte_bancaire_incomplet"
    );

    // Un fonctionnaire de l'État : son numéro, pas de service d'origine (T-40).
    let mut prof = membre("Sangaré", "Bakary", &enseignement, "Enseignant");
    prof["fonctionnaire"] = json!(true);
    prof["numero_fonctionnaire"] = json!("0123456-A");
    let prof = p.ok("membre_enregistrer", Some(&admin), prof).await;
    assert_eq!(prof["membre"]["fonctionnaire"], true);
    assert_eq!(prof["membre"]["numero_fonctionnaire"], "0123456-A");
    assert_eq!(prof["membre"]["matricule"], "P-0003");
    assert_eq!(prof["enseignant"], Value::Null);
    // Il devient enseignant d'un clic, vacataire ; une seconde fois, refusé.
    let devenu = p
        .ok(
            "membre_enseignant",
            Some(&admin),
            json!({ "id": prof["membre"]["id"], "statut": "vacataire" }),
        )
        .await;
    assert_eq!(devenu["enseignant"]["statut"], "vacataire");
    assert_eq!(
        p.refus(
            "membre_enseignant",
            Some(&admin),
            json!({ "id": prof["membre"]["id"], "statut": "vacataire" }),
        )
        .await
        .cle,
        "personnel.deja_enseignant"
    );
    let enseignants = p.ok("enseignants_lister", Some(&admin), Value::Null).await;
    assert!(
        enseignants
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["nom"] == "Sangaré")
    );

    // Un enseignant créé dans « Enseignants » entre au registre (Q-63).
    p.ok(
        "enseignant_enregistrer",
        Some(&admin),
        json!({ "id": null, "nom": "Touré", "prenoms": "Mariam", "telephones": [],
            "statut": "titulaire", "utilisateur_id": null, "actif": true }),
    )
    .await;
    let registre = p.ok("personnel_lister", Some(&admin), Value::Null).await;
    let registre = registre.as_array().unwrap();
    assert_eq!(registre.len(), 5);
    let mariam = registre.iter().find(|m| m["nom"] == "Touré").unwrap();
    assert_eq!(mariam["categorie"], "Enseignement");
    assert_eq!(mariam["membre"]["fonction"], "Enseignant");
    assert_eq!(mariam["enseignant"], true);
    assert_eq!(mariam["membre"]["matricule"], "P-0004");

    // Renommer une catégorie ou retirer une fonction ne touche pas les fiches.
    p.ok(
        "categorie_personnel_enregistrer",
        Some(&admin),
        json!({ "id": appui, "libelle": "Personnel d'appui", "active": true }),
    )
    .await;
    let cuisinier = par_code(&referentiel["fonctions"], "cuisinier");
    p.ok(
        "fonction_personnel_enregistrer",
        Some(&admin),
        json!({ "id": cuisinier, "categorie_id": appui, "libelle": "Cuisinière", "active": false }),
    )
    .await;
    let relue = p
        .ok(
            "membre_lire",
            Some(&admin),
            json!({ "id": cuisiniere["membre"]["id"] }),
        )
        .await;
    assert_eq!(relue["categorie"], "Personnel d'appui");
    assert_eq!(relue["membre"]["fonction"], "Cuisinière");
    // Une catégorie retirée ne reçoit plus de nouvelles fiches.
    let autre = p
        .ok(
            "categorie_personnel_enregistrer",
            Some(&admin),
            json!({ "id": null, "libelle": "Stagiaires", "active": true }),
        )
        .await;
    p.ok(
        "categorie_personnel_enregistrer",
        Some(&admin),
        json!({ "id": autre["id"], "libelle": "Stagiaires", "active": false }),
    )
    .await;
    assert_eq!(
        p.refus(
            "membre_enregistrer",
            Some(&admin),
            membre("Dembélé", "Ina", &autre["id"], "Stagiaire")
        )
        .await
        .cle,
        "personnel.categorie_retiree"
    );

    // Départ : la fiche reste, l'enseignant cesse d'être actif ; puis retour.
    assert_eq!(
        p.refus(
            "membre_partir",
            Some(&admin),
            json!({ "id": prof["membre"]["id"], "le": "2027-06-30", "motif": " " }),
        )
        .await
        .cle,
        "personnel.motif_depart_obligatoire"
    );
    let parti = p
        .ok(
            "membre_partir",
            Some(&admin),
            json!({ "id": prof["membre"]["id"], "le": "2027-06-30", "motif": "Muté hors de Bamako" }),
        )
        .await;
    assert_eq!(parti["membre"]["parti_le"], "2027-06-30");
    assert_eq!(parti["enseignant"]["actif"], false);
    let revenu = p
        .ok(
            "membre_revenir",
            Some(&admin),
            json!({ "id": prof["membre"]["id"] }),
        )
        .await;
    assert_eq!(revenu["membre"]["parti_le"], Value::Null);

    // Pièces du dossier : un PDF, relu à l'identique ; un format ou un poids refusés.
    let pdf = dossier.path().join("diplome.pdf");
    std::fs::write(&pdf, b"%PDF-1.4 diplome de Bakary").unwrap();
    let piece = p
        .ok(
            "piece_dossier_ajouter",
            Some(&admin),
            json!({ "membre_id": prof["membre"]["id"], "intitule": "Licence de mathématiques",
                "nature": "diplome", "chemin": pdf.to_str().unwrap() }),
        )
        .await;
    assert_eq!(piece["nom_fichier"], "diplome.pdf");
    let copie = dossier.path().join("copie.pdf");
    p.ok(
        "piece_dossier_enregistrer_sous",
        Some(&admin),
        json!({ "id": piece["id"], "chemin": copie.to_str().unwrap() }),
    )
    .await;
    assert_eq!(
        std::fs::read(&copie).unwrap(),
        b"%PDF-1.4 diplome de Bakary"
    );
    let docx = dossier.path().join("lettre.docx");
    std::fs::write(&docx, b"x").unwrap();
    assert_eq!(
        p.refus(
            "piece_dossier_ajouter",
            Some(&admin),
            json!({ "membre_id": prof["membre"]["id"], "intitule": "Lettre",
                "nature": "autre", "chemin": docx.to_str().unwrap() }),
        )
        .await
        .cle,
        "personnel.piece_format"
    );
    let lourd = dossier.path().join("scan.jpg");
    std::fs::write(&lourd, vec![0_u8; 2 * 1024 * 1024 + 1]).unwrap();
    assert_eq!(
        p.refus(
            "piece_dossier_ajouter",
            Some(&admin),
            json!({ "membre_id": prof["membre"]["id"], "intitule": "Scan",
                "nature": "identite", "chemin": lourd.to_str().unwrap() }),
        )
        .await
        .cle,
        "personnel.piece_trop_lourde"
    );
    let retiree = p
        .ok(
            "piece_dossier_retirer",
            Some(&admin),
            json!({ "id": piece["id"] }),
        )
        .await;
    assert_eq!(retiree["active"], false);
    let fiche = p
        .ok(
            "membre_lire",
            Some(&admin),
            json!({ "id": prof["membre"]["id"] }),
        )
        .await;
    assert_eq!(fiche["pieces"][0]["active"], false);

    // Un compte sans les droits du personnel n'y accède pas.
    p.ok(
        "utilisateur_creer",
        Some(&admin),
        json!({ "identifiant": "caisse", "nom_affiche": "Caisse",
            "mot_de_passe_initial": "tiroir-2026-2027", "roles": ["caissier"] }),
    )
    .await;
    let caissier = p.connecter("caisse", "tiroir-2026-2027").await;
    assert_eq!(
        p.refus("personnel_lister", Some(&caissier), Value::Null)
            .await
            .cle,
        "erreur.permission_refusee"
    );

    // Tout est tracé, la chaîne d'audit intacte.
    let verification = p.ok("audit_verifier", Some(&admin), Value::Null).await;
    assert!(verification["lignes_verifiees"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn une_liste_du_personnel_s_importe_apres_verification() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let referentiel = p
        .ok("personnel_referentiel_lire", Some(&admin), Value::Null)
        .await;
    let administration = par_code(&referentiel["categories"], "administration");
    p.ok(
        "membre_enregistrer",
        Some(&admin),
        membre("Coulibaly", "Awa", &administration, "Secrétaire"),
    )
    .await;

    // Le modèle : les colonnes attendues, sans ligne.
    let modele = dossier.path().join("modele-personnel.xlsx");
    let ecrit = p
        .ok(
            "personnel_import_modele",
            Some(&admin),
            json!({ "chemin": modele.to_string_lossy() }),
        )
        .await;
    assert_eq!(ecrit["lignes"], 0);
    assert!(modele.exists());

    // Une liste en CSV, colonnes dans un autre ordre, accents oubliés : deux lignes prêtes, une
    // personne déjà au registre, une catégorie inconnue, une date illisible.
    let liste = dossier.path().join("personnel.csv");
    std::fs::write(
        &liste,
        "Prenoms;Nom;Categorie;Fonction;Sexe;Date de naissance;Telephone;Fonctionnaire\n\
         Moussa;Diarra;Enseignement;Professeur de mathématiques;M;12/03/1985;76 44 55 66;oui\n\
         Fatoumata;Keïta;Appui;Cuisinière;F;;;\n\
         Awa;COULIBALY;Direction et administration;Secrétaire;F;;;\n\
         Bakary;Sanogo;Cantine;Cuisinier;M;;;\n\
         Oumar;Traoré;Appui;Gardien;M;31/02/1990;;\n",
    )
    .unwrap();
    let analyse = p
        .ok(
            "personnel_import_analyser",
            Some(&admin),
            json!({ "chemin": liste.to_string_lossy() }),
        )
        .await;
    assert_eq!(analyse["pretes"], 2);
    let lignes = analyse["lignes"].as_array().unwrap();
    assert_eq!(lignes.len(), 5);
    assert_eq!(lignes[0]["numero"], 2);
    assert_eq!(lignes[2]["doublon"], true);
    assert_eq!(
        lignes[3]["erreurs"],
        json!(["import_personnel.categorie_inconnue"])
    );
    assert_eq!(
        lignes[4]["erreurs"],
        json!(["import_personnel.date_illisible"])
    );
    // L'analyse n'écrit rien.
    let registre = p.ok("personnel_lister", Some(&admin), Value::Null).await;
    assert_eq!(registre.as_array().unwrap().len(), 1);

    let bilan = p
        .ok(
            "personnel_import_appliquer",
            Some(&admin),
            json!({ "chemin": liste.to_string_lossy() }),
        )
        .await;
    assert_eq!(bilan, json!({ "importes": 2, "ignorees": 3 }));
    let registre = p.ok("personnel_lister", Some(&admin), Value::Null).await;
    let registre = registre.as_array().unwrap();
    assert_eq!(registre.len(), 3);
    let diarra = registre.iter().find(|m| m["nom"] == "Diarra").unwrap();
    assert_eq!(diarra["membre"]["matricule"], "P-0002");
    assert_eq!(diarra["membre"]["fonctionnaire"], true);
    assert_eq!(diarra["telephones"], json!(["+22376445566"]));
    assert_eq!(diarra["categorie"], "Enseignement");

    // Relancée, la même liste ne crée personne de plus.
    let bilan = p
        .ok(
            "personnel_import_appliquer",
            Some(&admin),
            json!({ "chemin": liste.to_string_lossy() }),
        )
        .await;
    assert_eq!(bilan["importes"], 0);

    // Une liste sans les colonnes exigées est refusée.
    let mauvaise = dossier.path().join("autre.csv");
    std::fs::write(&mauvaise, "Nom complet;Poste\nMoussa Diarra;Professeur\n").unwrap();
    assert_eq!(
        p.refus(
            "personnel_import_analyser",
            Some(&admin),
            json!({ "chemin": mauvaise.to_string_lossy() }),
        )
        .await
        .cle,
        "import_personnel.colonnes"
    );
}
