//! Bulletins par la frontière (SPEC_PHASE_4 §7, UC-04) : calcul d'une classe à partir des notes,
//! appréciations, validation sur des évaluations validées, publication en un PDF, immuabilité,
//! correction en nouvelle version, l'ancienne remplacée et consultable.

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

/// Élève complet, avec sa mère responsable légale joignable.
async fn eleve(p: &Poste_, admin: &gesco_services::session::Jeton, prenoms: &str) -> Value {
    let fiche = p
        .ok(
            "eleve_creer",
            Some(admin),
            json!({ "identite": { "nom": "Traoré", "prenoms": prenoms, "sexe": "feminin",
                "date_naissance": "2014-03-02", "date_naissance_estimee": false,
                "lieu_naissance": "Bamako", "nationalite": "Malienne", "adresse": "", "telephones": [] },
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

/// Moyenne pondérée tronquée au centième, comme au crayon.
fn generale(lignes: &[(i64, i64)]) -> i64 {
    let points: i64 = lignes.iter().map(|(m, c)| m * c).sum();
    let coefficients: i64 = lignes.iter().map(|(_, c)| c).sum();
    points / coefficients
}

#[tokio::test]
async fn une_classe_se_calcule_se_valide_se_publie_puis_se_corrige() {
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
    let trimestre = ped["periodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["periode"]["niveau_id"] == septieme && x["periode"]["rang"] == 1)
        .unwrap()["periode"]["id"]
        .clone();
    let math = id_par(&ped["matieres"], "code", "MATH");
    let francais = id_par(&ped["matieres"], "code", "FR");
    let coefficient = |m: &Value| {
        ped["programme"]
            .as_array()
            .unwrap()
            .iter()
            .find(|l| l["niveau_id"] == septieme && l["matiere_id"] == *m)
            .unwrap()["coefficient"]
            .as_i64()
            .unwrap_or(1)
    };
    let (c_math, c_fr) = (coefficient(&math), coefficient(&francais));

    // Deux devoirs et une composition de maths, une composition de français.
    let evaluation = |matiere: &Value, type_evaluation: &str, libelle: &str| {
        json!({ "classe_id": classe, "periode_id": trimestre, "matiere_id": matiere,
            "type_evaluation": type_evaluation, "libelle": libelle, "date": "2026-10-04", "bareme": null })
    };
    let mut evaluations = Vec::new();
    for (matiere, type_evaluation, libelle) in [
        (&math, "devoir", "Devoir 1"),
        (&math, "devoir", "Devoir 2"),
        (&math, "composition", "Composition"),
        (&francais, "composition", "Composition"),
    ] {
        evaluations.push(
            p.ok(
                "evaluation_creer",
                Some(&admin),
                evaluation(matiere, type_evaluation, libelle),
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
    let eleves: Vec<Value> = grille["lignes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["inscription_id"].clone())
        .collect();
    // Awa : 12, 15, 11 et 14 (cas 2) ; Binta absente aux devoirs, 16 et 10 ; Coumba 13 partout.
    let notes = [
        ["12", "15", "11", "14"],
        ["a", "a", "16", "10"],
        ["13", "13", "13", "13"],
    ];
    for (i, ligne) in notes.iter().enumerate() {
        for (e, texte) in ligne.iter().enumerate() {
            p.ok(
                "note_saisir",
                Some(&admin),
                json!({ "evaluation_id": evaluations[e], "inscription_id": eleves[i], "saisie": texte }),
            )
            .await;
        }
    }
    // Awa absente un matin.
    p.ok(
        "absences_releve_enregistrer",
        Some(&admin),
        json!({ "classe_id": classe, "date": "2026-10-04",
            "lignes": [{ "inscription_id": eleves[0], "matin": "a", "apres_midi": "" }] }),
    )
    .await;

    // Discipline (T-70) : le conseil de classe blâme Binta pour le trimestre ; une retenue reste
    // au dossier sans aller au bulletin, et ne se rattache pas à une période.
    let sanction = |nature: &str, motif: &str, periode: Value| {
        json!({ "inscription_id": eleves[1], "nature": nature, "date": "2026-10-04",
            "motif": motif, "decidee_par": "Conseil de classe", "periode_id": periode })
    };
    assert_eq!(
        p.refus(
            "sanction_noter",
            Some(&admin),
            sanction("retenue", "Devoir non fait", trimestre.clone())
        )
        .await
        .cle,
        "sanction.periode_reservee_au_conseil"
    );
    p.ok(
        "sanction_noter",
        Some(&admin),
        sanction("blame", "Bavardages répétés", trimestre.clone()),
    )
    .await;
    let dossier = p
        .ok(
            "sanction_noter",
            Some(&admin),
            sanction("retenue", "Devoir non fait", Value::Null),
        )
        .await;
    assert_eq!(dossier["sanctions"].as_array().unwrap().len(), 2);
    assert_eq!(dossier["sanctions"][0]["periode"], "1er trimestre");
    let retenue = dossier["sanctions"][1]["sanction"]["id"].clone();
    let dossier = p
        .ok(
            "sanction_annuler",
            Some(&admin),
            json!({ "id": retenue, "motif": "Devoir retrouvé" }),
        )
        .await;
    assert_eq!(dossier["sanctions"][1]["sanction"]["annulee"], true);
    let registre = p
        .ok("discipline_registre_lire", Some(&admin), Value::Null)
        .await;
    assert_eq!(registre["sanctions"].as_array().unwrap().len(), 2);
    assert_eq!(registre["sanctions"][0]["classe"], "7e A");

    let demande = json!({ "classe_id": classe, "periode_id": trimestre });
    let calcul = p
        .ok("resultats_calculer", Some(&admin), demande.clone())
        .await;
    assert_eq!(calcul["evaluations_en_saisie"], 4);
    assert_eq!(
        calcul["bulletins"][1]["bulletin"]["contenu"]["sanctions_conseil"],
        json!(["Blâme : Bavardages répétés"])
    );
    assert_eq!(
        calcul["bulletins"][0]["bulletin"]["contenu"]["sanctions_conseil"],
        json!([])
    );
    assert_eq!(calcul["bulletins"].as_array().unwrap().len(), 3);
    let awa = &calcul["bulletins"][0]["bulletin"];
    assert_eq!(awa["statut"], "calcule");
    assert_eq!(awa["version"], 1);
    let contenu = &awa["contenu"];
    assert_eq!(contenu["eleve"], "Awa Traoré");
    assert_eq!(contenu["periode"], "1er trimestre");
    let ligne_math = contenu["lignes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["matiere_id"] == math)
        .unwrap();
    assert_eq!(ligne_math["note_de_classe"], 1350);
    assert_eq!(ligne_math["composition"], 1100);
    assert_eq!(ligne_math["moyenne"], 1183);
    assert_eq!(ligne_math["points"], 1183 * c_math);
    // Les matières sans évaluation ne sont pas notées et sortent de la moyenne (Q-59).
    assert_eq!(contenu["total_coefficients"], c_math + c_fr);
    // Elles ne s'impriment pas non plus : seules les deux matières notées restent.
    assert_eq!(contenu["lignes"].as_array().unwrap().len(), 2);
    let m_awa = generale(&[(1183, c_math), (1400, c_fr)]);
    let m_binta = generale(&[(1600, c_math), (1000, c_fr)]);
    assert_eq!(contenu["moyenne"], m_awa);
    assert_eq!(
        calcul["bulletins"][1]["bulletin"]["contenu"]["moyenne"],
        m_binta
    );
    assert_eq!(
        calcul["bulletins"][2]["bulletin"]["contenu"]["moyenne"],
        1300
    );
    assert_eq!(contenu["effectif"], 3);
    assert_eq!(contenu["ont_compose"], 3);
    assert_eq!(contenu["premier"], m_awa.max(m_binta).max(1300));
    assert_eq!(contenu["dernier"], m_awa.min(m_binta).min(1300));
    assert_eq!(contenu["absences"]["demi_journees"], 1);

    // Appréciations : celle de la matière, puis la générale de la direction.
    p.ok(
        "appreciation_saisir",
        Some(&admin),
        json!({ "inscription_id": eleves[0], "periode_id": trimestre, "matiere_id": math, "texte": "Bon travail" }),
    )
    .await;
    let apprecie = p
        .ok(
            "appreciation_saisir",
            Some(&admin),
            json!({ "inscription_id": eleves[0], "periode_id": trimestre, "matiere_id": null,
                "texte": "Élève sérieuse, peut mieux faire en maths." }),
        )
        .await;
    assert_eq!(
        apprecie["contenu"]["appreciation_generale"],
        "Élève sérieuse, peut mieux faire en maths."
    );
    // Un recalcul garde les appréciations.
    let recalcul = p
        .ok("resultats_calculer", Some(&admin), demande.clone())
        .await;
    let ligne_math = recalcul["bulletins"][0]["bulletin"]["contenu"]["lignes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["matiere_id"] == math)
        .unwrap()
        .clone();
    assert_eq!(ligne_math["appreciation"], "Bon travail");

    // La validation attend les évaluations validées.
    assert_eq!(
        p.refus("bulletins_valider", Some(&admin), demande.clone())
            .await
            .cle,
        "bulletin.evaluations_en_saisie"
    );
    for e in &evaluations {
        p.ok("evaluation_valider", Some(&admin), json!({ "id": e }))
            .await;
    }
    assert_eq!(
        p.ok("bulletins_valider", Some(&admin), demande.clone())
            .await,
        3
    );
    // L'école envoie ses SMS : la publication met en file l'avis « bulletin à retirer ».
    p.ok(
        "parametre_definir",
        Some(&admin),
        json!({ "cle": "sms.canal", "valeur": { "type": "texte", "valeur": "simule" } }),
    )
    .await;
    let publie = p
        .ok("bulletins_publier", Some(&admin), demande.clone())
        .await;
    assert!(publie["pdf"].as_str().unwrap().len() > 1000);
    let avis = |file: &Value| -> Vec<String> {
        file["messages"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|m| m["message"]["nature"] == "bulletin")
            .map(|m| m["message"]["texte"].as_str().unwrap().to_owned())
            .collect()
    };
    let file = p
        .ok("messages_lire", Some(&admin), json!({ "etat": null }))
        .await;
    let textes = avis(&file);
    assert_eq!(textes.len(), 3, "un avis par élève");
    assert!(
        textes
            .iter()
            .any(|t| t.contains("le bulletin de 1er trimestre de Awa Traore (7e A) est disponible")),
        "{textes:?}"
    );
    assert_eq!(publie["document"]["type_document"], "bulletin");
    // Relecture à l'œil : `GESCO_ECRIRE_BULLETINS=chemin.pdf` dépose les bulletins publiés.
    if let Some(chemin) = std::env::var_os("GESCO_ECRIRE_BULLETINS") {
        use base64::Engine;
        let pdf = base64::engine::general_purpose::STANDARD
            .decode(publie["pdf"].as_str().unwrap())
            .unwrap();
        std::fs::write(chemin, pdf).unwrap();
    }
    let lu = p.ok("bulletins_lire", Some(&admin), demande.clone()).await;
    assert_eq!(lu["bulletins"][0]["bulletin"]["statut"], "publie");
    let v1 = lu["bulletins"][0]["bulletin"]["id"].clone();

    // Publié, il ne change plus : ni appréciation, ni recalcul.
    assert_eq!(
        p.refus(
            "appreciation_saisir",
            Some(&admin),
            json!({ "inscription_id": eleves[0], "periode_id": trimestre, "matiere_id": null, "texte": "Autre" })
        )
        .await
        .cle,
        "bulletin.publie"
    );
    let encore = p
        .ok("resultats_calculer", Some(&admin), demande.clone())
        .await;
    assert_eq!(
        encore["bulletins"][0]["bulletin"]["contenu"]["moyenne"],
        m_awa
    );

    // Une erreur de report en maths : la composition d'Awa était 14. La note se corrige après
    // déverrouillage, puis le bulletin publié se corrige en version 2.
    p.ok(
        "evaluation_deverrouiller",
        Some(&admin),
        json!({ "id": evaluations[2], "motif": "Erreur de report" }),
    )
    .await;
    p.ok(
        "note_saisir",
        Some(&admin),
        json!({ "evaluation_id": evaluations[2], "inscription_id": eleves[0], "saisie": "14" }),
    )
    .await;
    p.ok(
        "evaluation_valider",
        Some(&admin),
        json!({ "id": evaluations[2] }),
    )
    .await;
    assert_eq!(
        p.refus(
            "bulletin_corriger",
            Some(&admin),
            json!({ "id": v1, "motif": " " })
        )
        .await
        .cle,
        "bulletin.motif_obligatoire"
    );
    let v2 = p
        .ok(
            "bulletin_corriger",
            Some(&admin),
            json!({ "id": v1, "motif": "Erreur de report en maths" }),
        )
        .await;
    assert_eq!(v2["version"], 2);
    assert_eq!(v2["statut"], "calcule");
    // (13,50 + 2 × 14) ÷ 3 = 13,83.
    let math_v2 = v2["contenu"]["lignes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["matiere_id"] == math)
        .unwrap()
        .clone();
    assert_eq!(math_v2["moyenne"], 1383);
    assert_eq!(math_v2["appreciation"], "Bon travail");
    assert_eq!(
        p.ok("bulletins_valider", Some(&admin), demande.clone())
            .await,
        1
    );
    p.ok("bulletins_publier", Some(&admin), demande.clone())
        .await;
    // Republié après correction, le bulletin ne renvoie pas d'avis.
    let file = p
        .ok("messages_lire", Some(&admin), json!({ "etat": null }))
        .await;
    assert_eq!(avis(&file).len(), 3);
    let lu = p.ok("bulletins_lire", Some(&admin), demande.clone()).await;
    let awa = &lu["bulletins"][0];
    assert_eq!(awa["bulletin"]["version"], 2);
    assert_eq!(awa["bulletin"]["statut"], "publie");
    assert_eq!(
        awa["bulletin"]["motif_correction"],
        "Erreur de report en maths"
    );
    assert_eq!(awa["anterieures"][0]["statut"], "remplace");
    assert_eq!(
        awa["bulletin"]["contenu"]["moyenne"],
        generale(&[(1383, c_math), (1400, c_fr)])
    );
    // L'ancienne version reste consultable et s'imprime marquée remplacée.
    let ancienne = p
        .ok("bulletin_produire", Some(&admin), json!({ "id": v1 }))
        .await;
    assert!(ancienne["pdf"].as_str().unwrap().len() > 1000);
    // L'école choisit les colonnes imprimées : tout masqué, puis tout affiché, le bulletin se
    // compose toujours ; la matière et sa moyenne restent.
    let parametres = p.ok("parametres_lister", Some(&admin), json!(null)).await;
    let reglages: Vec<String> = parametres
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|x| x["cle"].as_str())
        .filter(|c| c.starts_with("bulletin."))
        .map(str::to_owned)
        .collect();
    // 2e cycle, lycée et enseignement technique : 14 réglages chacun ; 1er cycle : 10, sans les
    // colonnes trimestrielles.
    assert_eq!(reglages.len(), 52);
    for affiche in [false, true] {
        for cle in &reglages {
            p.ok(
                "parametre_definir",
                Some(&admin),
                json!({ "cle": cle, "valeur": { "type": "booleen", "valeur": affiche } }),
            )
            .await;
        }
        let classe_pdf = p
            .ok("bulletins_produire", Some(&admin), demande.clone())
            .await;
        assert!(classe_pdf["pdf"].as_str().unwrap().len() > 1000);
        if let Some(chemin) = std::env::var_os(format!("GESCO_ECRIRE_BULLETINS_{affiche}")) {
            use base64::Engine;
            let pdf = base64::engine::general_purpose::STANDARD
                .decode(classe_pdf["pdf"].as_str().unwrap())
                .unwrap();
            std::fs::write(chemin, pdf).unwrap();
        }
    }
    // Les notes et les moyennes n'ont pas bougé.
    let relu = p.ok("bulletins_lire", Some(&admin), demande.clone()).await;
    assert_eq!(
        relu["bulletins"][0]["bulletin"]["contenu"]["moyenne"],
        generale(&[(1383, c_math), (1400, c_fr)])
    );
    // Une version remplacée ne se corrige plus.
    assert_eq!(
        p.refus(
            "bulletin_corriger",
            Some(&admin),
            json!({ "id": v1, "motif": "encore" })
        )
        .await
        .cle,
        "bulletin.non_corrigeable"
    );
}
