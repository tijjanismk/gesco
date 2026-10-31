//! Référentiel pédagogique par la frontière (SPEC_PHASE_4 §2) : matières livrées, programme et
//! coefficients, périodes, règles d'évaluation versionnées.

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

fn periodes_du(p: &Value, niveau: &Value) -> Vec<Value> {
    p["periodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|x| x["periode"]["niveau_id"] == *niveau)
        .cloned()
        .collect()
}

fn programme_du(p: &Value, niveau: &Value, serie: &Value) -> Vec<Value> {
    p["programme"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|l| l["niveau_id"] == *niveau && l["serie_id"] == *serie)
        .cloned()
        .collect()
}

#[tokio::test]
async fn le_referentiel_pedagogique_se_pose_puis_se_corrige() {
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
    let lire = |id: Value| {
        let p = &p;
        let admin = &admin;
        async move {
            p.ok("pedagogie_lire", Some(admin), json!({ "id": id }))
                .await
        }
    };
    let ped = lire(annee["id"].clone()).await;

    // Défauts posés à la création de l'année.
    assert_eq!(ped["matieres"].as_array().unwrap().len(), 44);
    let niveaux = ped["niveaux"].clone();
    let premiere = id_par(&niveaux, "code", "1");
    let septieme = id_par(&niveaux, "code", "7");
    let neuvieme = id_par(&niveaux, "code", "9");
    let onzieme = id_par(&niveaux, "code", "11");
    let ps = id_par(&niveaux, "code", "PS");
    let mois = periodes_du(&ped, &premiere);
    assert_eq!(mois.len(), 9);
    assert_eq!(mois[0]["libelle"], "Octobre");
    assert_eq!(mois[0]["periode"]["debut"], "2026-10-01");
    assert_eq!(mois[8]["periode"]["fin"], "2027-06-30");
    let trimestres = periodes_du(&ped, &septieme);
    assert_eq!(trimestres.len(), 3);
    assert_eq!(trimestres[0]["libelle"], "1er trimestre");
    assert!(periodes_du(&ped, &ps).is_empty());
    // 1re année : les douze matières de son emploi du temps, dans son ordre, sans coefficient
    // (réponse du 13 octobre 2026).
    let matieres_1re = programme_du(&ped, &premiere, &Value::Null);
    assert_eq!(matieres_1re.len(), 12);
    assert!(matieres_1re.iter().all(|l| l["coefficient"].is_null()));
    assert_eq!(
        matieres_1re[0]["matiere_id"],
        id_par(&ped["matieres"], "code", "LANG")
    );
    assert_eq!(
        programme_du(&ped, &id_par(&niveaux, "code", "4"), &Value::Null).len(),
        22
    );
    assert_eq!(programme_du(&ped, &septieme, &Value::Null).len(), 8);
    let tse = id_par(&ped["series"], "code", "TSE");
    let terminale = programme_du(&ped, &id_par(&niveaux, "code", "12"), &tse);
    let matieres = ped["matieres"].clone();
    let philo = id_par(&matieres, "code", "PHILO");
    assert!(terminale.iter().any(|l| l["matiere_id"] == philo));
    let regles = ped["regles"].as_array().unwrap();
    assert_eq!(regles.len(), 4);
    assert!(regles.iter().all(|r| r["version"] == 1));
    let premier = regles
        .iter()
        .find(|r| r["cycle"] == "premier_cycle")
        .unwrap();
    assert_eq!(
        premier["contenu"]["types"],
        json!([{ "type_evaluation": "composition", "bareme": 1000 }])
    );

    // Coefficients du 2e cycle saisis par l'école ; refusés au 1er cycle.
    let fr = id_par(&matieres, "code", "FR");
    let math = id_par(&matieres, "code", "MATH");
    let ligne =
        |m: &Value, c: Value| json!({ "matiere_id": m, "coefficient": c, "minutes_hebdo": 300 });
    let enregistre = p
        .ok(
            "programme_enregistrer",
            Some(&admin),
            json!({ "annee_id": annee["id"], "niveau_id": septieme, "serie_id": null,
                "lignes": [ligne(&math, json!(4)), ligne(&fr, json!(3))] }),
        )
        .await;
    assert_eq!(enregistre.as_array().unwrap().len(), 2);
    assert_eq!(enregistre[0]["matiere_id"], math);
    let refus = |demande: Value| {
        let p = &p;
        let admin = &admin;
        async move {
            p.refus("programme_enregistrer", Some(admin), demande)
                .await
                .cle
        }
    };
    assert_eq!(
        refus(
            json!({ "annee_id": annee["id"], "niveau_id": premiere, "serie_id": null,
            "lignes": [ligne(&id_par(&matieres, "code", "LC"), json!(2))] })
        )
        .await,
        "programme.coefficient_premier_cycle"
    );
    assert_eq!(
        refus(
            json!({ "annee_id": annee["id"], "niveau_id": septieme, "serie_id": null,
            "lignes": [ligne(&fr, json!(0))] })
        )
        .await,
        "programme.coefficient_invalide"
    );
    assert_eq!(
        refus(
            json!({ "annee_id": annee["id"], "niveau_id": onzieme, "serie_id": null, "lignes": [] })
        )
        .await,
        "programme.serie_invalide"
    );

    // Deux trimestres en 9e, jamais de semestre ; le 1er cycle garde ses neuf mois.
    let deux = p
        .ok(
            "periodes_definir",
            Some(&admin),
            json!({ "annee_id": annee["id"], "niveau_id": neuvieme, "nature": "trimestre",
                "nombre": 2 }),
        )
        .await;
    assert_eq!(deux.as_array().unwrap().len(), 2);
    assert_eq!(deux[1]["type_periode"], "trimestre");
    assert_eq!(deux[1]["debut"], "2027-02-01");
    assert_eq!(
        p.refus(
            "periodes_definir",
            Some(&admin),
            json!({ "annee_id": annee["id"], "niveau_id": premiere, "nature": "trimestre",
                "nombre": 2 })
        )
        .await
        .cle,
        "periode.nombre_fixe"
    );
    assert_eq!(
        p.refus(
            "periodes_definir",
            Some(&admin),
            json!({ "annee_id": annee["id"], "niveau_id": neuvieme, "nature": "semestre",
                "nombre": 2 })
        )
        .await
        .cle,
        "periode.semestre_technique"
    );

    // Dates corrigées : vacances de Noël ; un chevauchement est refusé.
    let ids: Vec<Value> = trimestres
        .iter()
        .map(|t| t["periode"]["id"].clone())
        .collect();
    let calendrier = |fin_premier: &str| {
        json!({ "annee_id": annee["id"], "niveau_id": septieme, "periodes": [
            { "id": ids[0], "debut": "2026-10-05", "fin": fin_premier, "ponderation": 1 },
            { "id": ids[1], "debut": "2027-01-04", "fin": "2027-03-26", "ponderation": 1 },
            { "id": ids[2], "debut": "2027-04-06", "fin": "2027-06-25", "ponderation": 2 },
        ] })
    };
    let corrige = p
        .ok(
            "calendrier_modifier",
            Some(&admin),
            calendrier("2026-12-18"),
        )
        .await;
    assert_eq!(corrige[2]["ponderation"], 2);
    assert_eq!(
        p.refus(
            "calendrier_modifier",
            Some(&admin),
            calendrier("2027-01-10")
        )
        .await
        .cle,
        "periode.chevauchement"
    );

    // Règle du 2e cycle : l'absent compte zéro, nouvelle version.
    let mut contenu = regles
        .iter()
        .find(|r| r["cycle"] == "second_cycle")
        .unwrap()["contenu"]
        .clone();
    contenu["absent_non_justifie"] = json!("zero");
    let v2 = p
        .ok(
            "regle_enregistrer",
            Some(&admin),
            json!({ "annee_id": annee["id"], "cycle": "second_cycle", "contenu": contenu }),
        )
        .await;
    assert_eq!(v2["version"], 2);
    contenu["mentions"][6]["seuil"] = json!(900);
    assert_eq!(
        p.refus(
            "regle_enregistrer",
            Some(&admin),
            json!({ "annee_id": annee["id"], "cycle": "second_cycle", "contenu": contenu })
        )
        .await
        .cle,
        "regle.mentions_non_decroissantes"
    );
    contenu["mentions"][0]["libelle"] = json!("Parfait");
    assert_eq!(
        p.refus(
            "regle_enregistrer",
            Some(&admin),
            json!({ "annee_id": annee["id"], "cycle": "second_cycle", "contenu": contenu })
        )
        .await
        .cle,
        "regle.mentions_hautes_fixes"
    );

    // Une matière ajoutée ; un code déjà pris est refusé.
    let info = json!({ "id": null, "code": "INFO", "libelle": "Informatique", "libelle_court": "Info",
        "groupe": "scientifique", "active": true });
    let creee = p
        .ok("matiere_enregistrer", Some(&admin), info.clone())
        .await;
    assert_eq!(creee["ordre"], 45);
    assert_eq!(
        p.refus("matiere_enregistrer", Some(&admin), info).await.cle,
        "matiere.code_pris"
    );

    // Un secrétaire lit le référentiel mais ne le modifie pas.
    p.ok(
        "utilisateur_creer",
        Some(&admin),
        json!({ "identifiant": "fanta", "nom_affiche": "Fanta Keïta",
            "mot_de_passe_initial": "secretariat-2026", "roles": ["secretaire"] }),
    )
    .await;
    let secretaire = p.connecter("fanta", "secretariat-2026").await;
    p.ok(
        "pedagogie_lire",
        Some(&secretaire),
        json!({ "id": annee["id"] }),
    )
    .await;
    assert_eq!(
        p.refus(
            "programme_enregistrer",
            Some(&secretaire),
            json!({ "annee_id": annee["id"], "niveau_id": septieme, "serie_id": null, "lignes": [] })
        )
        .await
        .cle,
        "erreur.permission_refusee"
    );

    // L'année suivante reprend le programme, coefficients compris.
    let suivante = p
        .ok(
            "annee_creer",
            Some(&admin),
            json!({ "libelle": "2027-2028", "debut": RENTREE_2027, "fin": FIN_2028 }),
        )
        .await;
    let repris = p
        .ok(
            "programme_reprendre",
            Some(&admin),
            json!({ "id": suivante["id"] }),
        )
        .await;
    assert!(repris.as_u64().unwrap() > 50);
    let ped2 = lire(suivante["id"].clone()).await;
    let septieme_2 = programme_du(&ped2, &septieme, &Value::Null);
    assert_eq!(septieme_2.len(), 2);
    assert_eq!(septieme_2[0]["coefficient"], 4);
    assert_eq!(
        periodes_du(&ped2, &premiere)[0]["periode"]["debut"],
        "2027-10-01"
    );
}
