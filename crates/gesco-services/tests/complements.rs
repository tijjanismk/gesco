//! Compléments pour les rapports par la frontière (SPEC_PHASE_9 §8.2, §8.3, critère 10) : année
//! de recrutement proposée puis saisie, fonctionnement de la classe et sa valeur par défaut tirée
//! de la fiche, difficultés notées avec leur droit propre et comptées sans nom, tableaux des
//! statistiques correspondants.

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

fn tableau<'a>(onglet: &'a Value, code: &str) -> Option<&'a Value> {
    onglet["tableaux"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["code"] == code)
}

#[tokio::test]
async fn les_complements_se_saisissent_et_se_comptent() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;

    // Une école sans année : grilles vides, aucun refus.
    let vide = p
        .ok(
            "complements_eleves_lire",
            Some(&admin),
            json!({ "annee_id": null, "classe_id": null }),
        )
        .await;
    assert!(vide["classes"].as_array().unwrap().is_empty());
    assert_eq!(vide["fonctionnement_defaut"], "simple");
    let personnel = p
        .ok("complements_personnel_lire", Some(&admin), Value::Null)
        .await;
    assert!(personnel.as_array().unwrap().is_empty());

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
    let classe = p
        .ok(
            "classe_creer",
            Some(&admin),
            json!({ "annee_id": annee["id"], "niveau_id": id_par(&ped["niveaux"], "code", "1"),
                "serie_id": null, "libelle": "1re A" }),
        )
        .await["id"]
        .clone();
    let mut eleves = Vec::new();
    for (prenoms, sexe) in [("Awa", "feminin"), ("Moussa", "masculin")] {
        let fiche = p
            .ok(
                "eleve_creer",
                Some(&admin),
                json!({ "identite": { "nom": "Diarra", "prenoms": prenoms, "sexe": sexe,
                    "date_naissance": "2020-03-02", "date_naissance_estimee": false,
                    "lieu_naissance": "Bamako", "nationalite": "Malienne", "adresse": "",
                    "telephones": [] },
                    "matricule_interne": "", "matricule_national": "", "acte_nature": "acte",
                    "acte_numero": "", "acte_date": "", "ecole_origine": "" }),
            )
            .await;
        let inscription = p
            .ok(
                "inscription_creer",
                Some(&admin),
                json!({ "eleve_id": fiche["eleve"]["id"], "classe_id": classe,
                    "date_inscription": "2026-10-01", "redoublant": false,
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
        eleves.push((fiche["eleve"]["id"].clone(), inscription));
    }

    // Entrés en 1re année dans l'école : l'année de recrutement est proposée, pas écrite.
    let grille = p
        .ok(
            "complements_eleves_lire",
            Some(&admin),
            json!({ "annee_id": null, "classe_id": classe }),
        )
        .await;
    assert_eq!(grille["eleves"].as_array().unwrap().len(), 2);
    for e in grille["eleves"].as_array().unwrap() {
        assert_eq!(e["annee_recrutement"], Value::Null);
        assert_eq!(e["annee_proposee"], 2026);
    }
    let grille = p
        .ok(
            "complement_eleve_regler",
            Some(&admin),
            json!({ "inscription_id": eleves[0].1, "annee_recrutement": 2025,
                "prise_en_charge_etat": false }),
        )
        .await;
    let awa = grille["eleves"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["inscription_id"] == eleves[0].1)
        .expect("Awa absente");
    assert_eq!(awa["annee_recrutement"], 2025);
    assert_eq!(awa["annee_proposee"], Value::Null);
    assert_eq!(
        p.refus(
            "complement_eleve_regler",
            Some(&admin),
            json!({ "inscription_id": eleves[0].1, "annee_recrutement": 1800,
                "prise_en_charge_etat": false }),
        )
        .await
        .cle,
        "complements.annee_recrutement_invalide"
    );

    // Fonctionnement : la valeur par défaut vient de la fiche ; la classe a la sienne.
    let fiche = p.ok("fiche_ecole_lire", Some(&admin), json!({})).await;
    let rubrique = fiche["rubriques"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["code"] == "fonctionnement_classes")
        .unwrap()["id"]
        .clone();
    p.ok(
        "fiche_ecole_repondre",
        Some(&admin),
        json!({ "rubrique_id": rubrique, "annee_id": null, "valeur": "Double division" }),
    )
    .await;
    let grille = p
        .ok(
            "classe_rapports_regler",
            Some(&admin),
            json!({ "classe_id": classe, "fonctionnement": "double_vacation", "pedagogie": null }),
        )
        .await;
    assert_eq!(grille["fonctionnement_defaut"], "double_division");
    assert_eq!(grille["pedagogie_defaut"], "classique");
    assert_eq!(grille["classes"][0]["fonctionnement"], "double_vacation");
    assert_eq!(grille["classes"][0]["pedagogie"], Value::Null);

    // Une difficulté notée : comptée sans nom, pour qui en a le droit.
    let liste = p
        .ok(
            "difficulte_eleve_regler",
            Some(&admin),
            json!({ "eleve_id": eleves[1].0, "type_difficulte": "vue", "degre": "beaucoup",
                "note": "Lunettes" }),
        )
        .await;
    assert_eq!(liste.as_array().unwrap().len(), 1);
    let stats = p
        .ok(
            "statistiques_lire",
            Some(&admin),
            json!({ "date": "2026-11-02" }),
        )
        .await;
    let recrutement = tableau(&stats["eleves"], "eleves_par_annee_recrutement").unwrap();
    let lignes: Vec<(String, u64)> = recrutement["lignes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| {
            (
                l["libelle"].as_str().unwrap().to_owned(),
                l["cellules"][2]["nombre"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(lignes, vec![("2025".to_owned(), 1), ("2026".to_owned(), 1)]);
    let handicap = tableau(&stats["eleves"], "eleves_handicapes").unwrap();
    assert_eq!(handicap["lignes"].as_array().unwrap().len(), 1);
    assert_eq!(handicap["total"]["cellules"][2]["nombre"], 1);
    for c in handicap["lignes"][0]["cellules"].as_array().unwrap() {
        assert!(c["membres"].as_array().unwrap().is_empty());
    }

    // Le secrétariat lit les statistiques sans le tableau du handicap, et ne lit pas les
    // difficultés.
    p.ok(
        "utilisateur_creer",
        Some(&admin),
        json!({ "identifiant": "secretaire", "nom_affiche": "secretaire",
            "mot_de_passe_initial": "mot-de-passe-2026", "roles": ["secretaire"] }),
    )
    .await;
    let secretaire = p.connecter("secretaire", "mot-de-passe-2026").await;
    let stats = p
        .ok(
            "statistiques_lire",
            Some(&secretaire),
            json!({ "date": "2026-11-02" }),
        )
        .await;
    assert!(tableau(&stats["eleves"], "eleves_handicapes").is_none());
    assert!(tableau(&stats["eleves"], "eleves_par_annee_recrutement").is_some());
    assert_eq!(
        p.refus(
            "difficultes_eleve_lire",
            Some(&secretaire),
            json!({ "eleve_id": eleves[1].0 }),
        )
        .await
        .cle,
        "erreur.permission_refusee"
    );
}
