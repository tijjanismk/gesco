//! Enseignement technique et professionnel par la frontière (SPEC_PHASE_6 §9, §10) : le cycle
//! s'ouvre quand l'école l'enseigne ; CAP et BT comptent en deux semestres, ou en trimestres au
//! choix ; l'école crée ses filières, qui reçoivent leur programme ; règles et bulletins sont ceux
//! du lycée.

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

#[tokio::test]
async fn le_cycle_technique_s_ouvre_avec_ses_semestres_et_ses_filieres() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;

    // Livrées avec GESCO, les années du CAP et du BT restent fermées tant que l'école n'ouvre pas
    // le cycle.
    let referentiel = p.ok("niveaux_lister", Some(&admin), Value::Null).await;
    let techniques: Vec<&Value> = referentiel["niveaux"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|n| n["cycle"] == "technique")
        .collect();
    let codes: Vec<&str> = techniques
        .iter()
        .map(|n| n["code"].as_str().unwrap())
        .collect();
    assert_eq!(codes, ["CAP1", "CAP2", "BT1", "BT2", "BT3", "BT4"]);
    assert!(techniques.iter().all(|n| n["actif"] == false));
    assert_eq!(techniques[1]["diplome_terminal"], "cap");
    assert_eq!(techniques[5]["diplome_terminal"], "bt");
    let referentiel = p
        .ok(
            "cycles_definir",
            Some(&admin),
            json!({ "cycles": ["second_cycle", "lycee", "technique"] }),
        )
        .await;
    let niveaux = referentiel["niveaux"].clone();
    let cap1 = id_par(&niveaux, "code", "CAP1");
    let cap2 = id_par(&niveaux, "code", "CAP2");
    let bt1 = id_par(&niveaux, "code", "BT1");
    let onzieme = id_par(&niveaux, "code", "11");
    assert!(
        niveaux
            .as_array()
            .unwrap()
            .iter()
            .filter(|n| n["cycle"] == "technique")
            .all(|n| n["actif"] == true)
    );

    // L'année : deux semestres par défaut, octobre-janvier et février-juin.
    let annee = p
        .ok(
            "annee_creer",
            Some(&admin),
            json!({ "libelle": "2026-2027", "debut": RENTREE_2026, "fin": FIN_2027 }),
        )
        .await;
    let lire = || p.ok("pedagogie_lire", Some(&admin), json!({ "id": annee["id"] }));
    let ped = lire().await;
    let periodes_de = |ped: &Value, niveau: &Value| -> Vec<Value> {
        ped["periodes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|x| x["periode"]["niveau_id"] == *niveau)
            .cloned()
            .collect()
    };
    let semestres = periodes_de(&ped, &cap1);
    assert_eq!(semestres.len(), 2);
    assert_eq!(semestres[0]["libelle"], "1er semestre");
    assert_eq!(semestres[0]["periode"]["fin"], "2027-01-31");
    assert_eq!(semestres[1]["libelle"], "2e semestre");
    assert_eq!(semestres[1]["periode"]["debut"], "2027-02-01");
    // La règle du cycle : celle du lycée, sur 20, devoirs et composition.
    let regle = ped["regles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["cycle"] == "technique")
        .unwrap()
        .clone();
    let lycee = ped["regles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["cycle"] == "lycee")
        .unwrap()
        .clone();
    assert_eq!(regle["contenu"], lycee["contenu"]);
    // Les matières de la pratique et du stage sont livrées.
    for code in ["TECH", "DTECH", "TPA", "STAGE", "LEGIS", "GEST", "HSE"] {
        id_par(&ped["matieres"], "code", code);
    }

    // Une école du BT qui compte en trimestres : trois trimestres en BT 1re année, puis retour
    // aux semestres ; au lycée, jamais de semestre.
    let definir = |niveau: &Value, nature: &str, nombre: u8| {
        json!({ "annee_id": annee["id"], "niveau_id": niveau, "nature": nature,
            "nombre": nombre })
    };
    let trimestres = p
        .ok(
            "periodes_definir",
            Some(&admin),
            definir(&bt1, "trimestre", 3),
        )
        .await;
    assert_eq!(trimestres.as_array().unwrap().len(), 3);
    assert_eq!(trimestres[0]["type_periode"], "trimestre");
    let semestres = p
        .ok(
            "periodes_definir",
            Some(&admin),
            definir(&bt1, "semestre", 2),
        )
        .await;
    assert_eq!(semestres[1]["type_periode"], "semestre");
    for (niveau, nature, nombre, cle) in [
        (&onzieme, "semestre", 2, "periode.semestre_technique"),
        (&bt1, "semestre", 3, "periode.nombre_invalide"),
    ] {
        assert_eq!(
            p.refus(
                "periodes_definir",
                Some(&admin),
                definir(niveau, nature, nombre)
            )
            .await
            .cle,
            cle
        );
    }

    // L'école crée sa filière Électricité sur les deux années du CAP.
    let filiere = |code: &str, niveaux: Value| json!({ "code": code, "libelle": "Électricité", "niveaux": niveaux });
    let referentiel = p
        .ok(
            "filiere_creer",
            Some(&admin),
            filiere(" elec ", json!([cap1, cap2])),
        )
        .await;
    let elec: Vec<&Value> = referentiel["series"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["code"] == "ELEC")
        .collect();
    assert_eq!(elec.len(), 2);
    assert!(elec.iter().all(|s| s["libelle"] == "Électricité"));
    let elec_cap1 = elec.iter().find(|s| s["niveau_id"] == cap1).unwrap()["id"].clone();
    for (demande, cle) in [
        (filiere("ELEC", json!([cap1])), "filiere.code_pris"),
        (filiere("ELEC", json!([onzieme])), "filiere.hors_technique"),
        (filiere("ELEC", json!([])), "filiere.aucun_niveau"),
        (filiere("Élec", json!([bt1])), "filiere.code_invalide"),
    ] {
        assert_eq!(
            p.refus("filiere_creer", Some(&admin), demande).await.cle,
            cle
        );
    }

    // Son programme est posé dans l'année ouverte : les matières générales, sans coefficient ;
    // l'école y ajoute la pratique et le stage, avec leur coefficient.
    let ped = lire().await;
    let programme: Vec<Value> = ped["programme"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|l| l["niveau_id"] == cap1 && l["serie_id"] == elec_cap1)
        .cloned()
        .collect();
    let codes_programme: Vec<Value> = programme.iter().map(|l| l["matiere_id"].clone()).collect();
    let matieres = ped["matieres"].clone();
    assert_eq!(
        codes_programme,
        ["FR", "ANG", "MATH", "PC"]
            .iter()
            .map(|c| id_par(&matieres, "code", c))
            .collect::<Vec<_>>()
    );
    let ligne = |code: &str, coefficient: i64| {
        json!({ "matiere_id": id_par(&matieres, "code", code), "coefficient": coefficient,
            "minutes_hebdo": null })
    };
    let enregistre = p
        .ok(
            "programme_enregistrer",
            Some(&admin),
            json!({ "annee_id": annee["id"], "niveau_id": cap1, "serie_id": elec_cap1,
                "lignes": [ligne("FR", 2), ligne("MATH", 3), ligne("TECH", 4), ligne("TPA", 5),
                    ligne("STAGE", 3)] }),
        )
        .await;
    assert_eq!(enregistre.as_array().unwrap().len(), 5);

    // Une classe porte son année et sa filière ; une année qui a des filières l'exige.
    let classe = p
        .ok(
            "classe_creer",
            Some(&admin),
            json!({ "annee_id": annee["id"], "niveau_id": cap1, "serie_id": elec_cap1,
                "libelle": "CAP 1 Électricité" }),
        )
        .await;
    assert_eq!(classe["serie_id"], elec_cap1);
    assert_eq!(
        p.refus(
            "classe_creer",
            Some(&admin),
            json!({ "annee_id": annee["id"], "niveau_id": cap1, "serie_id": null,
                "libelle": "CAP 1 B" })
        )
        .await
        .cle,
        "classe.serie_exigee"
    );

    // Le bulletin du cycle a ses réglages d'impression, comme les autres cycles.
    let parametres = p.ok("parametres_lister", Some(&admin), Value::Null).await;
    assert!(
        parametres
            .as_array()
            .unwrap()
            .iter()
            .any(|x| x["cle"] == "bulletin.technique.afficher.rang")
    );

    let verification = p.ok("audit_verifier", Some(&admin), Value::Null).await;
    assert!(verification["lignes_verifiees"].as_u64().unwrap() > 0);
}

/// Élève sans matricule national, avec sa mère responsable légale joignable.
async fn eleve(p: &Poste_, admin: &gesco_services::session::Jeton, prenoms: &str) -> Value {
    let fiche = p
        .ok(
            "eleve_creer",
            Some(admin),
            json!({ "identite": { "nom": "Diallo", "prenoms": prenoms, "sexe": "masculin",
                "date_naissance": "2009-05-14", "date_naissance_estimee": false,
                "lieu_naissance": "Ségou", "nationalite": "Malienne", "adresse": "", "telephones": [] },
                "matricule_interne": "", "matricule_national": "", "acte_nature": "acte",
                "acte_numero": "", "acte_date": "", "ecole_origine": "" }),
        )
        .await;
    p.ok(
        "responsable_rattacher",
        Some(admin),
        json!({ "eleve_id": fiche["eleve"]["id"], "responsable_id": null,
            "saisie": { "identite": { "nom": "Diallo", "prenoms": format!("Mère de {prenoms}"), "sexe": "feminin",
                "date_naissance": "", "date_naissance_estimee": false, "lieu_naissance": "", "nationalite": "",
                "adresse": "", "telephones": ["76 22 33 44"] }, "profession": "", "employeur": "" },
            "qualites": { "lien": "mere", "est_responsable_legal": true, "est_payeur": true,
                "recoit_notifications": true, "vient_chercher": false, "ordre_priorite": 1 } }),
    )
    .await;
    fiche
}

/// Dépose un document produit dans le dossier `GESCO_ECRIRE_DOCUMENTS`, pour le relire à l'œil.
fn deposer(document: &Value, nom: &str) {
    if let Some(dossier) = std::env::var_os("GESCO_ECRIRE_DOCUMENTS") {
        use base64::Engine;
        let pdf = base64::engine::general_purpose::STANDARD
            .decode(document["pdf"].as_str().unwrap())
            .unwrap();
        std::fs::write(std::path::Path::new(&dossier).join(nom), pdf).unwrap();
    }
}

#[tokio::test]
async fn un_cap_s_inscrit_se_note_et_publie_les_bulletins_du_premier_semestre() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    p.ok(
        "cycles_definir",
        Some(&admin),
        json!({ "cycles": ["lycee", "technique"] }),
    )
    .await;
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
    let cap1 = id_par(&ped["niveaux"], "code", "CAP1");
    let referentiel = p
        .ok(
            "filiere_creer",
            Some(&admin),
            json!({ "code": "ELEC", "libelle": "Électricité", "niveaux": [cap1] }),
        )
        .await;
    let elec = id_par(&referentiel["series"], "code", "ELEC");
    let matieres = ped["matieres"].clone();
    let matiere = |code: &str| id_par(&matieres, "code", code);
    let ligne = |code: &str, coefficient: i64| json!({ "matiere_id": matiere(code), "coefficient": coefficient, "minutes_hebdo": null });
    p.ok(
        "programme_enregistrer",
        Some(&admin),
        json!({ "annee_id": annee["id"], "niveau_id": cap1, "serie_id": elec,
            "lignes": [ligne("FR", 2), ligne("MATH", 3), ligne("TECH", 4), ligne("TPA", 5)] }),
    )
    .await;
    let classe = p
        .ok(
            "classe_creer",
            Some(&admin),
            json!({ "annee_id": annee["id"], "niveau_id": cap1, "serie_id": elec,
                "libelle": "CAP 1 Électricité" }),
        )
        .await["id"]
        .clone();

    // Deux apprentis, sans matricule national : facultatif dans ce cycle (Q-70).
    for prenom in ["Moussa", "Sékou"] {
        let fiche = eleve(&p, &admin, prenom).await;
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

    // Le 1er semestre : un devoir et une composition d'atelier, une composition de maths.
    let semestre = ped["periodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["periode"]["niveau_id"] == cap1 && x["periode"]["rang"] == 1)
        .unwrap()["periode"]
        .clone();
    assert_eq!(semestre["type_periode"], "semestre");
    let mut evaluations = Vec::new();
    for (code, type_evaluation) in [
        ("TPA", "devoir"),
        ("TPA", "composition"),
        ("MATH", "composition"),
    ] {
        evaluations.push(
            p.ok(
                "evaluation_creer",
                Some(&admin),
                json!({ "classe_id": classe, "periode_id": semestre["id"], "matiere_id": matiere(code),
                    "type_evaluation": type_evaluation, "libelle": type_evaluation,
                    "date": "2026-10-04", "bareme": null }),
            )
            .await["id"]
                .clone(),
        );
    }
    let grille = p
        .ok(
            "notes_grille",
            Some(&admin),
            json!({ "id": evaluations[0] }),
        )
        .await;
    let apprentis: Vec<Value> = grille["lignes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["inscription_id"].clone())
        .collect();
    // Moussa : 14 et 16 à l'atelier, 9 en maths ; Sékou : 10, 12 et 15.
    for (i, notes) in [["14", "16", "9"], ["10", "12", "15"]].iter().enumerate() {
        for (e, texte) in notes.iter().enumerate() {
            p.ok(
                "note_saisir",
                Some(&admin),
                json!({ "evaluation_id": evaluations[e], "inscription_id": apprentis[i],
                    "saisie": texte }),
            )
            .await;
        }
    }
    for e in &evaluations {
        p.ok("evaluation_valider", Some(&admin), json!({ "id": e }))
            .await;
    }

    // Le bulletin du 1er semestre, sur 20, porte l'année et la filière ; la moyenne pondère
    // l'atelier (5) et les maths (3), les matières sans note n'y entrent pas.
    let demande = json!({ "classe_id": classe, "periode_id": semestre["id"] });
    let calcul = p
        .ok("resultats_calculer", Some(&admin), demande.clone())
        .await;
    let moussa = &calcul["bulletins"][0]["bulletin"]["contenu"];
    assert_eq!(moussa["periode"], "1er semestre");
    assert_eq!(moussa["niveau"], "CAP 1re année, Électricité");
    assert_eq!(moussa["classe"], "CAP 1 Électricité");
    assert_eq!(moussa["cycle"], "technique");
    assert_eq!(moussa["bareme"], 2000);
    assert_eq!(moussa["lignes"].as_array().unwrap().len(), 2);
    // Atelier : (14 + 2 × 16) ÷ 3 = 15,33 ; moyenne (15,33 × 5 + 9 × 3) ÷ 8 = 12,95.
    let atelier = moussa["lignes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["matiere_id"] == matiere("TPA"))
        .unwrap()
        .clone();
    assert_eq!(atelier["moyenne"], 1533);
    assert_eq!(moussa["moyenne"], (1533 * 5 + 900 * 3) / 8);
    assert_eq!(
        p.ok("bulletins_valider", Some(&admin), demande.clone())
            .await,
        2
    );
    let publie = p
        .ok("bulletins_publier", Some(&admin), demande.clone())
        .await;
    assert!(publie["pdf"].as_str().unwrap().len() > 1000);
    deposer(&publie, "bulletins-cap1-semestre1.pdf");

    // La fiche d'inscription, le certificat de transfert et le relevé des moyennes.
    let fiche = p
        .ok(
            "inscription_fiche_produire",
            Some(&admin),
            json!({ "id": apprentis[0] }),
        )
        .await;
    assert_eq!(fiche["document"]["modele_id"], "fiche_inscription");
    deposer(&fiche, "fiche-cap1.pdf");
    p.ok(
        "inscription_transferer",
        Some(&admin),
        json!({ "inscription_id": apprentis[1], "date_depart": "2026-10-04",
            "etablissement_destination": "Centre de formation de Kati", "motif": "Déménagement",
            "passer_outre": "La famille part à Kati, le solde se règle à part" }),
    )
    .await;
    for (commande, nom) in [
        ("certificat_transfert_produire", "certificat-cap1.pdf"),
        ("releve_moyennes_produire", "releve-cap1.pdf"),
    ] {
        let document = p
            .ok(
                commande,
                Some(&admin),
                json!({ "inscription_id": apprentis[1] }),
            )
            .await;
        assert!(document["pdf"].as_str().unwrap().len() > 1000);
        deposer(&document, nom);
    }

    let verification = p.ok("audit_verifier", Some(&admin), Value::Null).await;
    assert!(verification["lignes_verifiees"].as_u64().unwrap() > 0);
}
