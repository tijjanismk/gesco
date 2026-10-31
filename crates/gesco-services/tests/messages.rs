//! Numéros et consentement des responsables (SPEC_PHASE_5 §2, étape 1) : l'état de chaque numéro,
//! le refus d'un type de message sans perdre les autres, la remise à l'essai d'un numéro mort.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use crate::commun::*;
use serde_json::{Value, json};
use sqlx::Connection;

fn identite(nom: &str, prenoms: &str, sexe: &str, telephones: Value) -> Value {
    json!({
        "nom": nom, "prenoms": prenoms, "sexe": sexe, "date_naissance": "2014-03-02",
        "date_naissance_estimee": false, "lieu_naissance": "Bamako", "nationalite": "Malienne",
        "adresse": "", "telephones": telephones,
    })
}

fn consentement(messages: &Value, type_message: &str) -> bool {
    messages["consentements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["type_message"] == type_message)
        .unwrap_or_else(|| panic!("{type_message} absent"))["accepte"]
        .as_bool()
        .unwrap()
}

async fn evenements(cnx: &mut sqlx::SqliteConnection, type_evenement: &str) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM outbox WHERE type_evenement = ?1")
        .bind(type_evenement)
        .fetch_one(cnx)
        .await
        .unwrap()
}

#[tokio::test]
async fn numeros_et_consentement_d_un_responsable() {
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
    let awa = p
        .ok(
            "eleve_creer",
            Some(&admin),
            json!({
                "identite": identite("Traoré", "Awa", "feminin", json!([])),
                "matricule_interne": "", "matricule_national": "", "acte_nature": "acte",
                "acte_numero": "", "acte_date": "", "ecole_origine": "",
            }),
        )
        .await;
    let fiche = p
        .ok(
            "responsable_rattacher",
            Some(&admin),
            json!({
                "eleve_id": awa["eleve"]["id"], "responsable_id": null,
                "saisie": {
                    "identite": identite("Coulibaly", "Mariam", "feminin", json!(["76 12 34 56", "66 00 00 02"])),
                    "profession": "", "employeur": "",
                },
                "qualites": {
                    "lien": "mere", "est_responsable_legal": true, "est_payeur": true,
                    "recoit_notifications": true, "vient_chercher": false, "ordre_priorite": 1,
                },
            }),
        )
        .await;
    let responsable_id = fiche["responsables"][0]["responsable"]["id"].clone();

    // Deux numéros non vérifiés, dans l'ordre de la fiche ; tous les types acceptés.
    let lus = p
        .ok(
            "responsable_messages_lire",
            Some(&admin),
            json!({ "responsable_id": responsable_id }),
        )
        .await;
    assert_eq!(lus["modifiable"], true);
    let numeros: Vec<(&str, &str)> = lus["numeros"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| (n["numero"].as_str().unwrap(), n["etat"].as_str().unwrap()))
        .collect();
    assert_eq!(
        numeros,
        [
            ("+22376123456", "non_verifie"),
            ("+22366000002", "non_verifie")
        ]
    );
    assert_eq!(lus["consentements"].as_array().unwrap().len(), 4);
    for t in ["bulletin", "impaye", "absence", "convocation"] {
        assert!(consentement(&lus, t), "{t}");
    }

    // Retirée des relances, elle garde l'avis de bulletin ; redire la même chose n'écrit rien.
    let sans_relance =
        json!({ "responsable_id": responsable_id, "type_message": "impaye", "accepte": false });
    let apres = p
        .ok(
            "responsable_consentement_definir",
            Some(&admin),
            sans_relance.clone(),
        )
        .await;
    assert!(!consentement(&apres, "impaye"));
    assert!(consentement(&apres, "bulletin"));
    p.ok(
        "responsable_consentement_definir",
        Some(&admin),
        sans_relance,
    )
    .await;
    let mut cnx = base_brute(dossier.path(), p.trousseau.clone()).await;
    assert_eq!(evenements(&mut cnx, "ConsentementModifie").await, 1);

    // Un numéro mort se remet à l'essai ; un numéro qui n'est pas le sien est refusé.
    sqlx::query(
        "INSERT INTO joignabilite_numero (etablissement_id, numero, etat, echecs_consecutifs,
            mis_a_jour_le, identifiant_poste)
         SELECT id, '+22376123456', 'injoignable', 3, 0, 'secretariat' FROM etablissement",
    )
    .execute(&mut cnx)
    .await
    .unwrap();
    let mort = p
        .ok(
            "responsable_messages_lire",
            Some(&admin),
            json!({ "responsable_id": responsable_id }),
        )
        .await;
    assert_eq!(mort["numeros"][0]["etat"], "injoignable");
    assert_eq!(mort["numeros"][1]["etat"], "non_verifie");
    assert_eq!(
        p.refus(
            "numero_remettre_a_l_essai",
            Some(&admin),
            json!({ "responsable_id": responsable_id, "numero": "+22370000000" }),
        )
        .await
        .cle,
        "messages.numero_inconnu"
    );
    let remis = p
        .ok(
            "numero_remettre_a_l_essai",
            Some(&admin),
            json!({ "responsable_id": responsable_id, "numero": "+22376123456" }),
        )
        .await;
    assert_eq!(remis["numeros"][0]["etat"], "non_verifie");
    assert_eq!(remis["numeros"][0]["echecs_consecutifs"], 0);
    assert_eq!(evenements(&mut cnx, "JoignabiliteReinitialisee").await, 1);
    cnx.close().await.unwrap();

    // La consultation seule lit, sans modifier.
    p.ok(
        "utilisateur_creer",
        Some(&admin),
        json!({
            "identifiant": "lecteur", "nom_affiche": "Lecteur",
            "mot_de_passe_initial": "consultation-2026", "roles": ["consultation_seule"],
        }),
    )
    .await;
    let lecteur = p.connecter("lecteur", "consultation-2026").await;
    let vu = p
        .ok(
            "responsable_messages_lire",
            Some(&lecteur),
            json!({ "responsable_id": responsable_id }),
        )
        .await;
    assert_eq!(vu["modifiable"], false);
    assert!(!consentement(&vu, "impaye"));
    p.refus(
        "responsable_consentement_definir",
        Some(&lecteur),
        json!({ "responsable_id": responsable_id, "type_message": "absence", "accepte": false }),
    )
    .await;
}

#[tokio::test]
async fn reglages_des_sms_livres_et_bornes() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let parametres = p.ok("parametres_lister", Some(&admin), Value::Null).await;
    let valeur = |cle: &str| {
        parametres
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["cle"] == cle)
            .unwrap_or_else(|| panic!("{cle} absent"))["valeur"]["valeur"]
            .clone()
    };
    assert_eq!(valeur("sms.fenetre_envoi"), "7-20");
    assert_eq!(valeur("sms.debit_par_minute"), 6);
    assert_eq!(valeur("sms.auto_bulletin"), true);
    assert_eq!(valeur("sms.auto_impaye"), true);
    assert_eq!(valeur("sms.auto_absence"), true);
    assert_eq!(valeur("sms.sanction_preparer"), false);
    p.ok(
        "parametre_definir",
        Some(&admin),
        json!({ "cle": "sms.fenetre_envoi", "valeur": { "type": "texte", "valeur": "8-18" } }),
    )
    .await;
    p.refus(
        "parametre_definir",
        Some(&admin),
        json!({ "cle": "sms.fenetre_envoi", "valeur": { "type": "texte", "valeur": "20-7" } }),
    )
    .await;
    p.refus(
        "parametre_definir",
        Some(&admin),
        json!({ "cle": "sms.debit_par_minute", "valeur": { "type": "entier", "valeur": 0 } }),
    )
    .await;
}
