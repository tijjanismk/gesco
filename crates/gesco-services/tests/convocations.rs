//! Convocations (SPEC_PHASE_5 §7, étape 5) : confiée à l'élève, elle est transmise non
//! confirmée ; par SMS, le message attend une décision ; la réception, le report et la suite se
//! notent ; annulée, son SMS préparé tombe ; le billet s'imprime.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use crate::commun::*;
use gesco_services::session::Jeton;
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

async fn regler(p: &Poste_, admin: &Jeton, cle: &str, valeur: Value) {
    let type_valeur = match &valeur {
        Value::Bool(_) => "booleen",
        Value::Number(_) => "entier",
        _ => "texte",
    };
    p.ok(
        "parametre_definir",
        Some(admin),
        json!({ "cle": cle, "valeur": { "type": type_valeur, "valeur": valeur } }),
    )
    .await;
}

#[tokio::test]
async fn une_convocation_part_se_suit_et_s_imprime() {
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
    let fiche = p
        .ok(
            "eleve_creer",
            Some(&admin),
            json!({ "identite": { "nom": "Traoré", "prenoms": "Awa", "sexe": "feminin",
                "date_naissance": "2014-03-02", "date_naissance_estimee": false,
                "lieu_naissance": "Bamako", "nationalite": "Malienne", "adresse": "", "telephones": [] },
                "matricule_interne": "", "matricule_national": "", "acte_nature": "acte",
                "acte_numero": "", "acte_date": "", "ecole_origine": "" }),
        )
        .await;
    let fiche = p
        .ok(
            "responsable_rattacher",
            Some(&admin),
            json!({ "eleve_id": fiche["eleve"]["id"], "responsable_id": null,
                "saisie": { "identite": { "nom": "Coulibaly", "prenoms": "Mariam", "sexe": "feminin",
                    "date_naissance": "", "date_naissance_estimee": false, "lieu_naissance": "", "nationalite": "",
                    "adresse": "", "telephones": ["76 00 00 01"] }, "profession": "", "employeur": "" },
                "qualites": { "lien": "mere", "est_responsable_legal": true, "est_payeur": true,
                    "recoit_notifications": true, "vient_chercher": false, "ordre_priorite": 1 } }),
        )
        .await;
    let mere = fiche["responsables"][0]["responsable"]["id"].clone();
    let inscription = p
        .ok(
            "inscription_creer",
            Some(&admin),
            json!({ "eleve_id": fiche["eleve"]["id"], "classe_id": classe, "date_inscription": "",
                "redoublant": false, "etablissement_origine": "" }),
        )
        .await;
    p.ok(
        "inscriptions_valider_classe",
        Some(&admin),
        json!({ "id": classe }),
    )
    .await;
    let convoquer = |canal: &str, date: &str, heure: &str| {
        json!({ "inscription_id": inscription["inscription"]["id"], "responsable_id": mere, "motif": "Retards répétés",
            "date": date, "heure": heure, "canal": canal, "sanction_id": null })
    };

    // Rendez-vous passé, heure illisible : refusés.
    assert_eq!(
        p.refus(
            "convocation_creer",
            Some(&admin),
            convoquer("eleve", "2026-10-01", "10:30")
        )
        .await
        .cle,
        "convocation.date_passee"
    );
    assert_eq!(
        p.refus(
            "convocation_creer",
            Some(&admin),
            convoquer("eleve", "2026-10-12", "10h30")
        )
        .await
        .cle,
        "convocation.heure_invalide"
    );

    // Confiée à l'élève : transmise, non confirmée.
    let par_eleve = p
        .ok(
            "convocation_creer",
            Some(&admin),
            convoquer("eleve", "2026-10-12", "10:30"),
        )
        .await;
    assert_eq!(par_eleve["convocation"]["accuse"], "transmis_non_confirme");
    assert_eq!(par_eleve["convocation"]["suite"], "en_attente");
    assert_eq!(par_eleve["responsable"], "Mariam Coulibaly");

    // Par SMS sans canal : refusé ; avec le canal, le message attend une décision.
    assert_eq!(
        p.refus(
            "convocation_creer",
            Some(&admin),
            convoquer("sms", "2026-10-13", "08:00")
        )
        .await
        .cle,
        "convocation.sms_sans_canal"
    );
    regler(&p, &admin, "sms.actif", json!(true)).await;
    regler(&p, &admin, "sms.canal", json!("simule")).await;
    let par_sms = p
        .ok(
            "convocation_creer",
            Some(&admin),
            convoquer("sms", "2026-10-13", "08:00"),
        )
        .await;
    let file = p
        .ok(
            "messages_lire",
            Some(&admin),
            json!({ "etat": "a_decider" }),
        )
        .await;
    let prepare = &file["messages"][0]["message"];
    assert_eq!(prepare["nature"], "convocation");
    assert_eq!(prepare["numero"], "+22376000001");
    assert!(prepare["texte"].as_str().unwrap().contains(
        "vous etes convoque(e) le 13/10/2026 a 8 h au sujet de Awa Traore. Motif : Retards repetes."
    ));

    // Réception confirmée, rendez-vous reporté, puis honoré.
    let id = par_eleve["convocation"]["id"].clone();
    let confirmee = p
        .ok(
            "convocation_accuser",
            Some(&admin),
            json!({ "id": id, "accuse": "confirme" }),
        )
        .await;
    assert_eq!(confirmee["convocation"]["accuse"], "confirme");
    let reportee = p
        .ok(
            "convocation_reporter",
            Some(&admin),
            json!({ "id": id, "date": "2026-10-15", "heure": "09:00" }),
        )
        .await;
    assert_eq!(reportee["convocation"]["suite"], "reportee");
    assert_eq!(reportee["convocation"]["date"], "2026-10-15");
    let honoree = p
        .ok(
            "convocation_clore",
            Some(&admin),
            json!({ "id": id, "honoree": true }),
        )
        .await;
    assert_eq!(honoree["convocation"]["suite"], "honoree");
    assert_eq!(
        p.refus(
            "convocation_clore",
            Some(&admin),
            json!({ "id": id, "honoree": false })
        )
        .await
        .cle,
        "convocation.close"
    );
    let en_cours = p
        .ok(
            "convocations_lire",
            Some(&admin),
            json!({ "en_cours": true }),
        )
        .await;
    assert_eq!(en_cours["liste"].as_array().unwrap().len(), 1);
    let toutes = p
        .ok(
            "convocations_lire",
            Some(&admin),
            json!({ "en_cours": false }),
        )
        .await;
    assert_eq!(toutes["liste"].as_array().unwrap().len(), 2);

    // Annulée, la convocation par SMS emporte son message préparé.
    p.ok(
        "convocation_annuler",
        Some(&admin),
        json!({ "id": par_sms["convocation"]["id"], "motif": "Le père est passé ce matin" }),
    )
    .await;
    let annules = p
        .ok("messages_lire", Some(&admin), json!({ "etat": "annule" }))
        .await;
    assert_eq!(
        annules["messages"][0]["message"]["motif_annulation"],
        "Convocation annulée"
    );

    // Le billet s'imprime.
    let billet = p
        .ok(
            "convocation_billet_produire",
            Some(&admin),
            json!({ "id": id }),
        )
        .await;
    assert!(billet["pdf"].as_str().unwrap().len() > 1000);
    assert_eq!(billet["document"]["type_document"], "convocation");
}
