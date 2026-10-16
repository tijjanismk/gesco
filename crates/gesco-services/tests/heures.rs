//! Heures des vacataires par la frontière (SPEC_PHASE_6 §4, T-41) : heures prévues proposées
//! d'après l'engagement, heures faites saisies au quart d'heure, visa du mois, déverrouillage
//! motivé, état imprimable.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod commun;

use commun::*;
use gesco_services::session::Jeton;
use serde_json::{Value, json};

async fn vacataire(p: &Poste_, admin: &Jeton, nom: &str, minutes: i64) -> Value {
    let referentiel = p
        .ok("personnel_referentiel_lire", Some(admin), Value::Null)
        .await;
    let enseignement = referentiel["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["code"] == "enseignement")
        .unwrap()["id"]
        .clone();
    let membre = p
        .ok(
            "membre_enregistrer",
            Some(admin),
            json!({
                "id": null, "nom": nom, "prenoms": "Bakary", "sexe": "masculin",
                "date_naissance": null, "telephones": [], "adresse": "", "matricule": "",
                "categorie_id": enseignement, "fonction": "Professeur", "arrive_le": null,
                "fonctionnaire": true, "numero_fonctionnaire": "", "numero_inps": "",
                "prevenir_nom": "", "prevenir_telephone": "", "paiement": { "mode": "especes" },
                "diplomes": [], "utilisateur_id": null,
            }),
        )
        .await["membre"]
        .clone();
    p.ok(
        "engagement_conclure",
        Some(admin),
        json!({
            "membre_id": membre["id"], "nature": "vacation", "debut": "2026-10-01", "fin": null,
            "fonction": "", "remuneration": { "type": "horaire", "taux": 2_500, "minutes_par_semaine": minutes },
            "note": "", "remplace": null, "renouvelle": null,
        }),
    )
    .await
}

fn saisie(engagement: &Value, mois: &str, prevues: &str, faites: &str, observation: &str) -> Value {
    json!({ "engagement_id": engagement["id"], "mois": mois, "prevues": prevues,
        "faites": faites, "observation": observation })
}

#[tokio::test]
async fn les_heures_des_vacataires_se_saisissent_et_se_visent_chaque_mois() {
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
    let sangare = vacataire(&p, &admin, "Sangaré", 6 * 60 + 30).await;
    let traore = vacataire(&p, &admin, "Traoré", 4 * 60).await;

    // Octobre : deux vacations, heures prévues proposées (heures par semaine × 4), rien de saisi.
    let etat = p
        .ok("heures_lire", Some(&admin), json!({ "mois": "2026-10" }))
        .await;
    let lignes = etat["lignes"].as_array().unwrap();
    assert_eq!(lignes.len(), 2);
    assert_eq!(lignes[0]["membre"], "Bakary Sangaré");
    assert_eq!(lignes[0]["id"], Value::Null);
    assert_eq!(lignes[0]["minutes_prevues"], 26 * 60);
    assert_eq!(lignes[0]["minutes_faites"], Value::Null);
    // Septembre précède les vacations : rien à saisir.
    let septembre = p
        .ok("heures_lire", Some(&admin), json!({ "mois": "2026-09" }))
        .await;
    assert!(septembre["lignes"].as_array().unwrap().is_empty());
    assert_eq!(
        p.refus(
            "heures_saisir",
            Some(&admin),
            saisie(&sangare, "2026-09", "26", "26", "")
        )
        .await
        .cle,
        "heures.hors_vacation"
    );

    // Heures faites au quart d'heure, avec l'explication de l'écart ; le montant suit.
    assert_eq!(
        p.refus(
            "heures_saisir",
            Some(&admin),
            saisie(&sangare, "2026-10", "26", "24,3", "")
        )
        .await
        .cle,
        "heures.quart_d_heure"
    );
    let etat = p
        .ok(
            "heures_saisir",
            Some(&admin),
            saisie(
                &sangare,
                "2026-10",
                "26",
                "24,5",
                "Deux heures manquées le 21",
            ),
        )
        .await;
    let ligne = &etat["lignes"][0];
    assert_ne!(ligne["id"], Value::Null);
    assert_eq!(ligne["minutes_faites"], 24 * 60 + 30);
    assert_eq!(ligne["montant"], 61_250);
    assert_eq!(etat["montant"], 61_250);

    // Le visa du mois exige les heures faites de chaque ligne saisie.
    p.ok(
        "heures_saisir",
        Some(&admin),
        saisie(&traore, "2026-10", "16", "", ""),
    )
    .await;
    assert_eq!(
        p.refus(
            "heures_viser",
            Some(&admin),
            json!({ "mois": "2026-10", "engagement_id": null })
        )
        .await
        .cle,
        "heures.faites_manquantes"
    );
    // Une vacation seule se vise.
    let etat = p
        .ok(
            "heures_viser",
            Some(&admin),
            json!({ "mois": "2026-10", "engagement_id": sangare["id"] }),
        )
        .await;
    assert_eq!(etat["lignes"][0]["statut"], "visee");
    assert_eq!(etat["lignes"][1]["statut"], "en_saisie");
    // Visée, elle ne change plus.
    assert_eq!(
        p.refus(
            "heures_saisir",
            Some(&admin),
            saisie(&sangare, "2026-10", "26", "26", "")
        )
        .await
        .cle,
        "heures.deja_visees"
    );
    // Sauf déverrouillage motivé.
    let id = etat["lignes"][0]["id"].clone();
    assert_eq!(
        p.refus(
            "heures_deverrouiller",
            Some(&admin),
            json!({ "id": id, "motif": " " })
        )
        .await
        .cle,
        "heures.motif_obligatoire"
    );
    let etat = p
        .ok(
            "heures_deverrouiller",
            Some(&admin),
            json!({ "id": id, "motif": "Rattrapage oublié le 28" }),
        )
        .await;
    assert_eq!(etat["lignes"][0]["statut"], "en_saisie");
    assert_eq!(
        etat["lignes"][0]["motif_deverrouillage"],
        "Rattrapage oublié le 28"
    );
    p.ok(
        "heures_saisir",
        Some(&admin),
        saisie(&sangare, "2026-10", "26", "26,5", "Rattrapage le 28"),
    )
    .await;
    p.ok(
        "heures_saisir",
        Some(&admin),
        saisie(&traore, "2026-10", "16", "16", ""),
    )
    .await;
    let etat = p
        .ok(
            "heures_viser",
            Some(&admin),
            json!({ "mois": "2026-10", "engagement_id": null }),
        )
        .await;
    assert!(
        etat["lignes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|l| l["statut"] == "visee")
    );
    assert_eq!(etat["minutes_faites"], (26 * 60 + 30) + 16 * 60);
    assert_eq!(etat["montant"], 66_250 + 40_000);
    assert_eq!(
        p.refus(
            "heures_viser",
            Some(&admin),
            json!({ "mois": "2026-10", "engagement_id": null })
        )
        .await
        .cle,
        "heures.rien_a_viser"
    );

    // L'état du mois s'imprime.
    let document = p
        .ok(
            "etat_heures_produire",
            Some(&admin),
            json!({ "mois": "2026-10" }),
        )
        .await;
    assert!(document["pdf"].as_str().unwrap().starts_with("JVBER"));

    let verification = p.ok("audit_verifier", Some(&admin), Value::Null).await;
    assert!(verification["lignes_verifiees"].as_u64().unwrap() > 0);
}
