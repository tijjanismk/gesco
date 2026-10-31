//! Évaluations et saisie des notes par la frontière (SPEC_PHASE_4 §4, UC-03) : validation en lot
//! des brouillons, grille, codes d'absence, barème, validation, déverrouillage motivé, portée de
//! l'enseignant.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use crate::commun::*;
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

/// Élève avec sa mère, responsable légale joignable ; sans lieu de naissance s'il est incomplet.
async fn eleve(
    p: &Poste_,
    admin: &gesco_services::session::Jeton,
    prenoms: &str,
    complet: bool,
) -> Value {
    let fiche = p
        .ok(
            "eleve_creer",
            Some(admin),
            json!({ "identite": { "nom": "Traoré", "prenoms": prenoms, "sexe": "feminin",
                "date_naissance": "2014-03-02", "date_naissance_estimee": false,
                "lieu_naissance": if complet { "Bamako" } else { "" },
                "nationalite": "Malienne", "adresse": "", "telephones": [] },
                "matricule_interne": "", "matricule_national": "", "acte_nature": "acte",
                "acte_numero": "", "acte_date": "", "ecole_origine": "" }),
        )
        .await;
    p.ok(
        "responsable_rattacher",
        Some(admin),
        json!({ "eleve_id": fiche["eleve"]["id"], "responsable_id": null,
            "saisie": { "identite": { "nom": "Coulibaly", "prenoms": format!("Mère de {prenoms}"), "sexe": "feminin",
                "date_naissance": "", "date_naissance_estimee": false, "lieu_naissance": "", "nationalite": "",
                "adresse": "", "telephones": ["76 12 34 56"] }, "profession": "", "employeur": "" },
            "qualites": { "lien": "mere", "est_responsable_legal": true, "est_payeur": true,
                "recoit_notifications": true, "vient_chercher": false, "ordre_priorite": 1 } }),
    )
    .await;
    fiche
}

#[tokio::test]
async fn une_classe_se_note_au_clavier_puis_se_valide() {
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
    let septieme = id_par(&ped["niveaux"], "code", "7");
    let classe = p
        .ok(
            "classe_creer",
            Some(&admin),
            json!({ "annee_id": annee["id"], "niveau_id": septieme, "serie_id": null, "libelle": "7e A" }),
        )
        .await["id"]
        .clone();

    // Trois brouillons complets et un incomplet ; la validation en lot prend les trois.
    for (prenom, complet) in [
        ("Awa", true),
        ("Binta", true),
        ("Coumba", true),
        ("Djeneba", false),
    ] {
        let fiche = eleve(&p, &admin, prenom, complet).await;
        p.ok(
            "inscription_creer",
            Some(&admin),
            json!({ "eleve_id": fiche["eleve"]["id"], "classe_id": classe, "date_inscription": "",
                "redoublant": false, "etablissement_origine": "" }),
        )
        .await;
    }
    let bilan = p
        .ok(
            "inscriptions_valider_classe",
            Some(&admin),
            json!({ "id": classe }),
        )
        .await;
    assert_eq!(bilan["validees"], 3);
    assert_eq!(bilan["refusees"][0]["eleve"], "Djeneba Traoré");
    assert_eq!(
        bilan["refusees"][0]["cle"],
        "inscription.identite_incomplete"
    );

    // Un devoir de maths au 1er trimestre, barème de la règle (20).
    let trimestre = ped["periodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["periode"]["niveau_id"] == septieme && x["periode"]["rang"] == 1)
        .unwrap()["periode"]["id"]
        .clone();
    let math = id_par(&ped["matieres"], "code", "MATH");
    let demande = |matiere: &Value, date: &str| {
        json!({ "classe_id": classe, "periode_id": trimestre, "matiere_id": matiere,
            "type_evaluation": "devoir", "libelle": "Devoir 1", "date": date, "bareme": null })
    };
    let devoir = p
        .ok(
            "evaluation_creer",
            Some(&admin),
            demande(&math, "2026-10-04"),
        )
        .await;
    assert_eq!(devoir["bareme"], 2000);
    assert_eq!(devoir["statut"], "en_saisie");
    assert_eq!(
        p.refus(
            "evaluation_creer",
            Some(&admin),
            demande(&math, "2027-02-01")
        )
        .await
        .cle,
        "evaluation.hors_periode"
    );

    // La grille ne montre que les inscriptions validées, dans l'ordre de la liste.
    let grille = p
        .ok("notes_grille", Some(&admin), json!({ "id": devoir["id"] }))
        .await;
    let lignes = grille["lignes"].as_array().unwrap().clone();
    assert_eq!(lignes.len(), 3);
    assert_eq!(lignes[0]["eleve"], "Awa Traoré");
    let saisir = |i: usize, texte: &str| json!({ "evaluation_id": devoir["id"], "inscription_id": lignes[i]["inscription_id"], "saisie": texte });
    let note = p.ok("note_saisir", Some(&admin), saisir(0, "12")).await;
    assert_eq!(note["valeur"], 1200);
    p.ok("note_saisir", Some(&admin), saisir(1, "13,5")).await;
    let absente = p.ok("note_saisir", Some(&admin), saisir(2, "a")).await;
    assert_eq!(absente["valeur"], Value::Null);
    assert_eq!(absente["absent"], true);
    assert_eq!(
        p.refus("note_saisir", Some(&admin), saisir(0, "25"))
            .await
            .cle,
        "note.hors_bareme"
    );
    assert_eq!(
        p.refus("note_saisir", Some(&admin), saisir(0, "douze"))
            .await
            .cle,
        "note.illisible"
    );
    let grille = p
        .ok("notes_grille", Some(&admin), json!({ "id": devoir["id"] }))
        .await;
    // Moyenne des notes, sans l'absente : (12 + 13,5) ÷ 2 = 12,75.
    assert_eq!(grille["moyenne"], 1275);

    // Validation : plus aucune note ne change ; le déverrouillage exige un motif.
    let validee = p
        .ok(
            "evaluation_valider",
            Some(&admin),
            json!({ "id": devoir["id"] }),
        )
        .await;
    assert_eq!(validee["statut"], "validee");
    assert_eq!(
        p.refus("note_saisir", Some(&admin), saisir(2, "aj"))
            .await
            .cle,
        "evaluation.validee"
    );
    assert_eq!(
        p.refus(
            "evaluation_deverrouiller",
            Some(&admin),
            json!({ "id": devoir["id"], "motif": " " })
        )
        .await
        .cle,
        "evaluation.motif_obligatoire"
    );
    let rouverte = p
        .ok(
            "evaluation_deverrouiller",
            Some(&admin),
            json!({ "id": devoir["id"], "motif": "Certificat médical de Coumba" }),
        )
        .await;
    assert_eq!(rouverte["statut"], "en_saisie");
    let justifiee = p.ok("note_saisir", Some(&admin), saisir(2, "aj")).await;
    assert_eq!(justifiee["justifie"], true);
    // Vidée, une case retire la note sans la supprimer ; l'évaluation n'est plus complète.
    assert_eq!(
        p.ok("note_saisir", Some(&admin), saisir(1, "")).await,
        Value::Null
    );
    assert_eq!(
        p.refus(
            "evaluation_valider",
            Some(&admin),
            json!({ "id": devoir["id"] })
        )
        .await
        .cle,
        "evaluation.incomplete"
    );

    // L'enseignant de maths saisit en maths, pas en français, et ne valide pas.
    let compte = p
        .ok(
            "utilisateur_creer",
            Some(&admin),
            json!({ "identifiant": "sekou", "nom_affiche": "Sékou Traoré",
                "mot_de_passe_initial": "enseignant-2026", "roles": ["enseignant"] }),
        )
        .await;
    let sekou = p
        .ok(
            "enseignant_enregistrer",
            Some(&admin),
            json!({ "id": null, "nom": "Traoré", "prenoms": "Sékou", "telephones": [],
                "statut": "titulaire", "utilisateur_id": compte["id"], "actif": true }),
        )
        .await;
    p.ok(
        "affectation_ajouter",
        Some(&admin),
        json!({ "classe_id": classe, "matiere_id": math, "enseignant_id": sekou["enseignant"]["id"] }),
    )
    .await;
    let jeton = p.connecter("sekou", "enseignant-2026").await;
    p.ok("note_saisir", Some(&jeton), saisir(1, "14")).await;
    let francais = id_par(&ped["matieres"], "code", "FR");
    assert_eq!(
        p.refus(
            "evaluation_creer",
            Some(&jeton),
            demande(&francais, "2026-10-05")
        )
        .await
        .cle,
        "erreur.portee_refusee"
    );
    p.ok(
        "evaluation_creer",
        Some(&admin),
        demande(&francais, "2026-10-05"),
    )
    .await;
    let siennes = p
        .ok(
            "evaluations_lister",
            Some(&jeton),
            json!({ "classe_id": classe, "periode_id": trimestre }),
        )
        .await;
    assert_eq!(siennes.as_array().unwrap().len(), 1);
    assert_eq!(siennes[0]["saisies"], 3);
    assert_eq!(
        p.refus(
            "evaluation_valider",
            Some(&jeton),
            json!({ "id": devoir["id"] })
        )
        .await
        .cle,
        "erreur.permission_refusee"
    );
    let toutes = p
        .ok(
            "evaluations_lister",
            Some(&admin),
            json!({ "classe_id": classe, "periode_id": trimestre }),
        )
        .await;
    assert_eq!(toutes.as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn une_grille_remplie_s_importe_apres_apercu() {
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
    let septieme = id_par(&ped["niveaux"], "code", "7");
    let classe = p
        .ok(
            "classe_creer",
            Some(&admin),
            json!({ "annee_id": annee["id"], "niveau_id": septieme, "serie_id": null, "libelle": "7e A" }),
        )
        .await["id"]
        .clone();
    for prenom in ["Awa", "Binta", "Coumba"] {
        let fiche = eleve(&p, &admin, prenom, true).await;
        p.ok(
            "inscription_creer",
            Some(&admin),
            json!({ "eleve_id": fiche["eleve"]["id"], "classe_id": classe, "date_inscription": "",
                "redoublant": false, "etablissement_origine": "" }),
        )
        .await;
    }
    p.ok(
        "inscriptions_valider_classe",
        Some(&admin),
        json!({ "id": classe }),
    )
    .await;
    let trimestre = ped["periodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["periode"]["niveau_id"] == septieme && x["periode"]["rang"] == 1)
        .unwrap()["periode"]["id"]
        .clone();
    let devoir = p
        .ok(
            "evaluation_creer",
            Some(&admin),
            json!({ "classe_id": classe, "periode_id": trimestre, "matiere_id": id_par(&ped["matieres"], "code", "MATH"),
                "type_evaluation": "devoir", "libelle": "Devoir 1", "date": "2026-10-04", "bareme": null }),
        )
        .await;
    let grille = p
        .ok("notes_grille", Some(&admin), json!({ "id": devoir["id"] }))
        .await;
    let matricule = |i: usize| {
        grille["lignes"][i]["matricule"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    // Awa a déjà 10 ; le fichier la passe à 12.
    p.ok(
        "note_saisir",
        Some(&admin),
        json!({ "evaluation_id": devoir["id"], "inscription_id": grille["lignes"][0]["inscription_id"], "saisie": "10" }),
    )
    .await;

    // Le classeur exporté par l'écran se relit tel quel : rien ne change.
    let classeur = dossier.path().join("grille.xlsx");
    p.ok(
        "exporter_xlsx",
        Some(&admin),
        json!({ "chemin": classeur.to_str().unwrap(), "titre": "7e A — Maths — Devoir 1",
            "entetes": ["N°", "Matricule", "Élève", "Note sur 20"],
            "lignes": [["1", matricule(0), "Awa Traoré", "10"], ["2", matricule(1), "Binta Traoré", ""], ["3", matricule(2), "Coumba Traoré", ""]] }),
    )
    .await;
    let relu = p
        .ok(
            "notes_import_analyser",
            Some(&admin),
            json!({ "evaluation_id": devoir["id"], "chemin": classeur.to_str().unwrap() }),
        )
        .await;
    assert_eq!(relu["a_ecrire"], 0);
    assert_eq!(relu["lignes"][0]["etat"], "inchangee");

    // La grille remplie par l'enseignant : une note changée, un absent, une note hors barème,
    // un matricule inconnu, une ligne en double.
    let rempli = dossier.path().join("grille-remplie.csv");
    std::fs::write(
        &rempli,
        format!(
            "N°;Matricule;Élève;Note sur 20\r\n1;{};Awa;12\r\n2;{};Binta;a\r\n3;{};Coumba;25\r\n4;X/999;Inconnu;8\r\n5;{};Awa encore;14\r\n",
            matricule(0),
            matricule(1),
            matricule(2),
            matricule(0)
        ),
    )
    .unwrap();
    let demande = json!({ "evaluation_id": devoir["id"], "chemin": rempli.to_str().unwrap() });
    let apercu = p
        .ok("notes_import_analyser", Some(&admin), demande.clone())
        .await;
    let etats: Vec<Value> = apercu["lignes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["etat"].clone())
        .collect();
    assert_eq!(
        etats,
        vec![
            json!("modifiee"),
            json!("nouvelle"),
            json!("erreur"),
            json!("erreur")
        ]
    );
    assert_eq!(apercu["lignes"][0]["avant"], "10");
    assert_eq!(apercu["lignes"][0]["apres"], "12");
    assert_eq!(apercu["lignes"][2]["erreur"], "note.hors_bareme");
    assert_eq!(
        apercu["lignes"][3]["erreur"],
        "import_notes.matricule_en_double"
    );
    assert_eq!(apercu["inconnus"], json!(["X/999"]));
    assert_eq!(apercu["a_ecrire"], 2);
    assert_eq!(apercu["erreurs"], 2);
    // L'aperçu n'a rien écrit.
    let avant = p
        .ok("notes_grille", Some(&admin), json!({ "id": devoir["id"] }))
        .await;
    assert_eq!(avant["lignes"][0]["note"]["valeur"], 1000);

    let bilan = p.ok("notes_importer", Some(&admin), demande.clone()).await;
    assert_eq!(bilan["a_ecrire"], 2);
    let apres = p
        .ok("notes_grille", Some(&admin), json!({ "id": devoir["id"] }))
        .await;
    assert_eq!(apres["lignes"][0]["note"]["valeur"], 1200);
    assert_eq!(apres["lignes"][1]["note"]["absent"], true);
    assert_eq!(apres["lignes"][2]["note"], Value::Null);

    // Une évaluation validée ne s'importe plus.
    p.ok(
        "note_saisir",
        Some(&admin),
        json!({ "evaluation_id": devoir["id"], "inscription_id": grille["lignes"][2]["inscription_id"], "saisie": "15" }),
    )
    .await;
    p.ok(
        "evaluation_valider",
        Some(&admin),
        json!({ "id": devoir["id"] }),
    )
    .await;
    assert_eq!(
        p.refus("notes_importer", Some(&admin), demande).await.cle,
        "evaluation.validee"
    );
}
