//! Clé modem GSM (SPEC_PHASE_5 §4, étape 3) sur le modem simulé : la clé s'essaie, les SMS partent
//! en trames PDU et leurs accusés reviennent ; une clé débranchée met la file en pause ; le crédit
//! lu par USSD sous le seuil arrête la file et prévient la direction, la recharge la fait repartir ;
//! cinq échecs de crédit l'arrêtent aussi, et elle repart à la main.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use crate::commun::*;
use gesco_services::file_sms::BilanFile;
use gesco_services::session::Jeton;
use serde_json::{Value, json};

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

async fn traiter(p: &Poste_) -> BilanFile {
    p.application.traiter_file_sms().await.unwrap()
}

async fn essai(p: &Poste_, admin: &Jeton, numero: &str) {
    p.ok(
        "message_essai_envoyer",
        Some(admin),
        json!({ "numero": numero }),
    )
    .await;
}

async fn file(p: &Poste_, admin: &Jeton) -> Value {
    p.ok("messages_lire", Some(admin), json!({ "etat": null }))
        .await
}

async fn dernier(p: &Poste_, admin: &Jeton, numero: &str) -> Value {
    file(p, admin).await["messages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["message"]["numero"] == numero)
        .unwrap_or_else(|| panic!("aucun message vers {numero}"))["message"]
        .clone()
}

async fn avertissements(p: &Poste_, admin: &Jeton) -> Vec<String> {
    p.ok("avertissements", Some(admin), Value::Null)
        .await
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["cle"].as_str().unwrap().to_owned())
        .collect()
}

#[tokio::test]
async fn la_cle_modem_envoie_et_surveille_son_credit() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let mut admin = p.connecter("admin", MOT_DE_PASSE).await;
    let modem = p.application.modem_simule();
    regler(&p, &admin, "sms.fenetre_envoi", json!("0-24")).await;

    // Canal « clé » sans port : la clé est absente, la file attend, le tableau de bord le dit.
    regler(&p, &admin, "sms.actif", json!(true)).await;
    regler(&p, &admin, "sms.canal", json!("modem")).await;
    essai(&p, &admin, "76 00 00 01").await;
    let bilan = traiter(&p).await;
    assert!(bilan.cle_absente);
    assert_eq!(bilan.envoyes, 0);
    assert!(
        avertissements(&p, &admin)
            .await
            .contains(&"avertissement.sms_cle_absente".to_owned())
    );

    // La clé simulée figure parmi les ports ; elle s'essaie, puis, avec le code USSD, dit son
    // crédit.
    let ports = p.ok("sms_ports_lister", Some(&admin), Value::Null).await;
    assert!(
        ports
            .as_array()
            .unwrap()
            .iter()
            .any(|port| port["nom"] == "simule")
    );
    let info = p
        .ok("sms_cle_essayer", Some(&admin), json!({ "port": "simule" }))
        .await;
    assert_eq!(info["modele"], "Cle GSM simulee");
    assert_eq!(info["operateur"], "Reseau simule");
    assert_eq!(info["signal"], 64);
    assert_eq!(info["credit"], Value::Null);
    regler(&p, &admin, "sms.ussd_solde", json!("#111#")).await;
    let info = p
        .ok("sms_cle_essayer", Some(&admin), json!({ "port": "simule" }))
        .await;
    assert_eq!(info["credit"], 5_000);
    assert_eq!(info["credit_texte"], "Votre solde est de 5 000 FCFA.");

    // Le port choisi, l'essai part en trame PDU ; l'accusé revient par la clé.
    p.ok(
        "sms_port_choisir",
        Some(&admin),
        json!({ "port": "simule" }),
    )
    .await;
    let bilan = traiter(&p).await;
    assert_eq!(
        (bilan.envoyes, bilan.remis, bilan.cle_absente),
        (1, 1, false)
    );
    assert_eq!(dernier(&p, &admin, "+22376000001").await["etat"], "remis");
    assert!(
        modem
            .envoyes()
            .iter()
            .any(|(n, t)| n == "+22376000001" && t.contains("message d'essai de GESCO"))
    );
    let lue = file(&p, &admin).await;
    assert_eq!(lue["port"], "simule");
    assert_eq!(lue["etat_canal"]["credit"], 5_000);
    assert_eq!(lue["etat_canal"]["cle_absente"], false);
    assert!(
        !avertissements(&p, &admin)
            .await
            .iter()
            .any(|a| a.starts_with("avertissement.sms"))
    );

    // Débranchée, la clé met la file en pause sans rien perdre ; rebranchée, tout repart.
    modem.debrancher(true);
    essai(&p, &admin, "76 00 00 02").await;
    assert!(traiter(&p).await.cle_absente);
    assert_eq!(
        dernier(&p, &admin, "+22376000002").await["etat"],
        "en_attente"
    );
    modem.debrancher(false);
    assert_eq!(traiter(&p).await.envoyes, 1);
    assert_eq!(dernier(&p, &admin, "+22376000002").await["etat"], "remis");

    // Le réseau refuse un numéro ; un autre ne reçoit pas.
    modem.refuser("+22370000003");
    modem.ne_pas_remettre("+22366000004");
    essai(&p, &admin, "70 00 00 03").await;
    essai(&p, &admin, "66 00 00 04").await;
    traiter(&p).await;
    let refuse = dernier(&p, &admin, "+22370000003").await;
    assert_eq!(
        (refuse["etat"].as_str(), refuse["erreur"].as_str()),
        (Some("echoue"), Some("numero_refuse"))
    );
    let perdu = dernier(&p, &admin, "+22366000004").await;
    assert_eq!(
        (perdu["etat"].as_str(), perdu["erreur"].as_str()),
        (Some("echoue"), Some("non_remis"))
    );

    // Crédit sous le seuil à la lecture suivante : la file s'arrête, la direction est prévenue.
    regler(&p, &admin, "sms.seuil_credit", json!(1_000)).await;
    regler(&p, &admin, "sms.numero_alerte", json!("76 99 99 99")).await;
    modem.fixer_credit(900);
    p.horloge.avancer(61 * 60_000);
    admin = p.connecter("admin", MOT_DE_PASSE).await;
    essai(&p, &admin, "76 00 00 05").await;
    let bilan = traiter(&p).await;
    assert!(bilan.arretee);
    assert_eq!(bilan.envoyes, 0);
    let (numero, alerte) = modem.envoyes().last().cloned().unwrap();
    assert_eq!(numero, "+22376999999");
    assert!(alerte.contains("credit SMS bas (900 F)"), "{alerte}");
    let lue = file(&p, &admin).await;
    assert_eq!(lue["etat_canal"]["arretee"], true);
    assert_eq!(lue["etat_canal"]["raison"], "credit_bas");
    assert_eq!(
        dernier(&p, &admin, "+22376000005").await["etat"],
        "en_attente"
    );
    assert!(
        avertissements(&p, &admin)
            .await
            .contains(&"avertissement.sms_arretes".to_owned())
    );

    // Rechargée, la puce est relue dix minutes plus tard : la file repart seule.
    modem.recharger(5_000);
    p.horloge.avancer(10 * 60_000);
    admin = p.connecter("admin", MOT_DE_PASSE).await;
    let bilan = traiter(&p).await;
    assert!(!bilan.arretee);
    assert_eq!(bilan.envoyes, 1);
    assert_eq!(dernier(&p, &admin, "+22376000005").await["etat"], "remis");

    // Lecture du crédit à la demande.
    let lue = p.ok("sms_credit_lire", Some(&admin), Value::Null).await;
    assert_eq!(lue["etat_canal"]["credit"], modem.credit());

    // Plus de crédit et un seuil nul : cinq refus d'affilée arrêtent la file.
    regler(&p, &admin, "sms.seuil_credit", json!(0)).await;
    modem.fixer_credit(0);
    for n in 11..=15 {
        essai(&p, &admin, &format!("76 00 00 {n}")).await;
    }
    let bilan = traiter(&p).await;
    assert!(bilan.arretee);
    let lue = file(&p, &admin).await;
    assert_eq!(lue["etat_canal"]["raison"], "echecs_credit");
    assert_eq!(
        dernier(&p, &admin, "+22376000015").await["erreur"],
        "credit"
    );

    // Rechargée sans que l'USSD le voie, la file repart à la main ; les messages en attente
    // partent à leur nouvel essai.
    modem.recharger(1_000);
    let lue = p.ok("sms_reprendre", Some(&admin), Value::Null).await;
    assert_eq!(lue["etat_canal"]["arretee"], false);
    assert_eq!(
        p.refus("sms_reprendre", Some(&admin), Value::Null)
            .await
            .cle,
        "sms.file_non_arretee"
    );
    p.horloge.avancer(15 * 60_000);
    admin = p.connecter("admin", MOT_DE_PASSE).await;
    assert_eq!(traiter(&p).await.envoyes, 5);
    assert_eq!(dernier(&p, &admin, "+22376000015").await["etat"], "remis");
}
