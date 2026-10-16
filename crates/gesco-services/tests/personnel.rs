//! Registre du personnel par la frontière (SPEC_PHASE_6 §2) : catégories et fonctions de l'école
//! sans liste fermée (T-39), matricules, fonctionnaires sans service d'origine (T-40), enseignants
//! rattachés (Q-63), départ et retour, pièces du dossier.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod commun;

use commun::*;
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
