//! Écran guichet (SPEC_PHASE_5 §8, étape 6) : une recherche, et la famille entière — la fratrie
//! et ce que doit chaque enfant, les convocations en cours, les absences récentes non
//! justifiées, les messages envoyés.

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

fn identite(prenoms: &str, sexe: &str, telephones: Value) -> Value {
    json!({ "nom": "Traoré", "prenoms": prenoms, "sexe": sexe, "date_naissance": "2014-03-02",
        "date_naissance_estimee": false, "lieu_naissance": "Bamako", "nationalite": "Malienne",
        "adresse": "", "telephones": telephones })
}

#[tokio::test]
async fn le_guichet_montre_toute_la_famille() {
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
    p.ok(
        "parametre_definir",
        Some(&admin),
        json!({ "cle": "sms.canal", "valeur": { "type": "texte", "valeur": "simule" } }),
    )
    .await;

    // Awa et Moussa ont la même mère ; Binta est d'une autre famille.
    let mut fiches = Vec::new();
    for (prenom, sexe) in [
        ("Awa", "feminin"),
        ("Moussa", "masculin"),
        ("Binta", "feminin"),
    ] {
        fiches.push(
            p.ok(
                "eleve_creer",
                Some(&admin),
                json!({ "identite": identite(prenom, sexe, json!([])), "matricule_interne": "",
                    "matricule_national": "", "acte_nature": "acte", "acte_numero": "", "acte_date": "",
                    "ecole_origine": "" }),
            )
            .await,
        );
    }
    let qualites = json!({ "lien": "mere", "est_responsable_legal": true, "est_payeur": true,
        "recoit_notifications": true, "vient_chercher": false, "ordre_priorite": 1 });
    let awa = p
        .ok(
            "responsable_rattacher",
            Some(&admin),
            json!({ "eleve_id": fiches[0]["eleve"]["id"], "responsable_id": null,
                "saisie": { "identite": identite("Mariam", "feminin", json!(["76 00 00 01"])), "profession": "", "employeur": "" },
                "qualites": qualites }),
        )
        .await;
    let mere = awa["responsables"][0]["responsable"]["id"].clone();
    p.ok(
        "responsable_rattacher",
        Some(&admin),
        json!({ "eleve_id": fiches[1]["eleve"]["id"], "responsable_id": mere, "saisie": null, "qualites": qualites }),
    )
    .await;
    p.ok(
        "responsable_rattacher",
        Some(&admin),
        json!({ "eleve_id": fiches[2]["eleve"]["id"], "responsable_id": null,
            "saisie": { "identite": identite("Fanta", "feminin", json!(["76 00 00 02"])), "profession": "", "employeur": "" },
            "qualites": qualites }),
    )
    .await;
    let mut inscriptions = Vec::new();
    for fiche in &fiches {
        inscriptions.push(
            p.ok(
                "inscription_creer",
                Some(&admin),
                json!({ "eleve_id": fiche["eleve"]["id"], "classe_id": classe, "date_inscription": "",
                    "redoublant": false, "etablissement_origine": "" }),
            )
            .await["inscription"]["id"]
                .clone(),
        );
    }
    p.ok(
        "inscriptions_valider_classe",
        Some(&admin),
        json!({ "id": classe }),
    )
    .await;

    // Awa absente ce matin ; Moussa convoqué.
    p.ok(
        "absences_releve_enregistrer",
        Some(&admin),
        json!({ "classe_id": classe, "date": "2026-10-04", "lignes": [
            { "inscription_id": inscriptions[0], "matin": "a", "apres_midi": "" }] }),
    )
    .await;
    p.ok(
        "convocation_creer",
        Some(&admin),
        json!({ "inscription_id": inscriptions[1], "responsable_id": mere, "motif": "Bavardages",
            "date": "2026-10-09", "heure": "10:00", "canal": "eleve", "sanction_id": null }),
    )
    .await;

    let guichet = p
        .ok(
            "guichet_lire",
            Some(&admin),
            json!({ "eleve_id": fiches[0]["eleve"]["id"] }),
        )
        .await;
    let enfants = guichet["enfants"].as_array().unwrap();
    assert_eq!(enfants.len(), 2, "Awa et Moussa, pas Binta");
    assert!(enfants[0]["nom"].as_str().unwrap().starts_with("Awa"));
    assert!(enfants[1]["nom"].as_str().unwrap().starts_with("Moussa"));
    // Les frais d'inscription de chacun sont échus le jour de l'inscription.
    assert_eq!(enfants[0]["echu"], 10_000);
    assert_eq!(guichet["echu_famille"], 20_000);
    assert_eq!(guichet["responsables"].as_array().unwrap().len(), 1);
    assert_eq!(guichet["absences"].as_array().unwrap().len(), 1);
    assert_eq!(guichet["absences"][0]["demi_journee"], "matin");
    assert_eq!(guichet["convocations"].as_array().unwrap().len(), 1);
    assert_eq!(
        guichet["convocations"][0]["convocation"]["accuse"],
        "transmis_non_confirme"
    );
    assert_eq!(
        guichet["messages"].as_array().unwrap().len(),
        1,
        "l'avis d'absence d'Awa"
    );
    assert_eq!(guichet["droits"]["encaisser"], true);

    // Justifiée au guichet, l'absence disparaît ; la convocation honorée aussi.
    p.ok(
        "absences_justifier",
        Some(&admin),
        json!({ "absences": [{ "inscription_id": inscriptions[0], "date": "2026-10-04", "demi_journee": "matin" }],
            "motif": "Déclaré au guichet", "justificatif": "Parent présent" }),
    )
    .await;
    p.ok(
        "convocation_clore",
        Some(&admin),
        json!({ "id": guichet["convocations"][0]["convocation"]["id"], "honoree": true }),
    )
    .await;
    let apres = p
        .ok(
            "guichet_lire",
            Some(&admin),
            json!({ "eleve_id": fiches[1]["eleve"]["id"] }),
        )
        .await;
    assert!(
        apres["enfants"][0]["nom"]
            .as_str()
            .unwrap()
            .starts_with("Moussa")
    );
    assert!(apres["absences"].as_array().unwrap().is_empty());
    assert!(apres["convocations"].as_array().unwrap().is_empty());
}
