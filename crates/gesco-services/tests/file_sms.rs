//! File des SMS (SPEC_PHASE_5 §3, étape 2) avec le canal simulé : rien ne part sans canal ; un
//! message part à son tour, dans les heures d'envoi, au débit et sous le plafond réglés ; les
//! accusés disent si le numéro est joignable ; une erreur passagère se réessaie, puis échoue ; une
//! personne décide, annule, renvoie.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod commun;

use commun::*;
use gesco_core::notification::ErreurEnvoi;
use gesco_services::file_sms::BilanFile;
use gesco_services::session::Jeton;
use serde_json::{Value, json};
use sqlx::Connection;

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

async fn essai(p: &Poste_, admin: &Jeton, numero: &str) -> Value {
    p.ok(
        "message_essai_envoyer",
        Some(admin),
        json!({ "numero": numero }),
    )
    .await
}

/// Le dernier message mis en file vers ce numéro.
async fn dernier(p: &Poste_, admin: &Jeton, numero: &str) -> Value {
    let file = p
        .ok("messages_lire", Some(admin), json!({ "etat": null }))
        .await;
    file["messages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["message"]["numero"] == numero)
        .unwrap_or_else(|| panic!("aucun message vers {numero}"))["message"]
        .clone()
}

async fn joignabilite(p: &Poste_, numero: &str) -> Option<String> {
    let mut cnx = base_brute(&p.racine, p.trousseau.clone()).await;
    let etat = sqlx::query_scalar("SELECT etat FROM joignabilite_numero WHERE numero = ?1")
        .bind(numero)
        .fetch_optional(&mut cnx)
        .await
        .unwrap();
    cnx.close().await.unwrap();
    etat
}

#[tokio::test]
async fn la_file_envoie_a_son_tour_par_le_canal_simule() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let mut admin = p.connecter("admin", MOT_DE_PASSE).await;
    let canal = p.application.canal_simule();

    // Les modèles livrés tiennent en un SMS ; sans canal, rien ne part.
    let file = p
        .ok("messages_lire", Some(&admin), json!({ "etat": null }))
        .await;
    assert_eq!(file["canal"], "aucun");
    assert_eq!(file["modeles"].as_array().unwrap().len(), 5);
    assert!(
        file["modeles"]
            .as_array()
            .unwrap()
            .iter()
            .all(|m| m["sms"] == 1)
    );
    essai(&p, &admin, "76 00 00 01").await;
    assert_eq!(traiter(&p).await.envoyes, 0);
    assert_eq!(
        dernier(&p, &admin, "+22376000001").await["etat"],
        "en_attente"
    );

    // Canal simulé : l'essai part, l'accusé revient, le numéro est joignable.
    regler(&p, &admin, "sms.canal", json!("simule")).await;
    let bilan = traiter(&p).await;
    assert_eq!((bilan.envoyes, bilan.remis), (1, 1));
    let parti = dernier(&p, &admin, "+22376000001").await;
    assert_eq!(parti["etat"], "remis");
    assert_eq!(parti["sms"], 1);
    assert!(
        canal
            .envoyes()
            .iter()
            .any(|(n, t)| n == "+22376000001" && t.contains("message d'essai de GESCO"))
    );
    assert_eq!(
        joignabilite(&p, "+22376000001").await.as_deref(),
        Some("joignable")
    );

    // Débit : deux par minute ; le troisième attend la minute suivante.
    regler(&p, &admin, "sms.debit_par_minute", json!(2)).await;
    p.horloge.avancer(61_000);
    for n in ["76 00 00 11", "76 00 00 12", "76 00 00 13"] {
        essai(&p, &admin, n).await;
    }
    assert_eq!(traiter(&p).await.envoyes, 2);
    assert_eq!(
        dernier(&p, &admin, "+22376000013").await["etat"],
        "en_attente"
    );
    p.horloge.avancer(61_000);
    assert_eq!(traiter(&p).await.envoyes, 1);
    regler(&p, &admin, "sms.debit_par_minute", json!(6)).await;

    // Hors des heures d'envoi, rien ne part ; le message attend, puis expire au bout d'un jour.
    regler(&p, &admin, "sms.fenetre_envoi", json!("20-21")).await;
    essai(&p, &admin, "76 00 00 21").await;
    assert_eq!(traiter(&p).await.envoyes, 0);
    p.horloge.avancer(25 * 3_600_000);
    admin = p.connecter("admin", MOT_DE_PASSE).await;
    assert_eq!(traiter(&p).await.expires, 1);
    assert_eq!(dernier(&p, &admin, "+22376000021").await["etat"], "expire");
    regler(&p, &admin, "sms.fenetre_envoi", json!("0-24")).await;

    // Pas de réseau : deux nouveaux essais espacés, puis l'échec ; renvoyé, il part.
    canal.faire_echouer("+22376000031", ErreurEnvoi::Reseau);
    essai(&p, &admin, "76 00 00 31").await;
    traiter(&p).await;
    let attente = dernier(&p, &admin, "+22376000031").await;
    assert_eq!(
        (
            attente["etat"].as_str(),
            attente["essais"].as_u64(),
            attente["erreur"].as_str()
        ),
        (Some("en_attente"), Some(1), Some("reseau"))
    );
    assert_eq!(
        traiter(&p).await.envoyes,
        0,
        "l'essai suivant attend cinq minutes"
    );
    p.horloge.avancer(5 * 60_000);
    traiter(&p).await;
    p.horloge.avancer(10 * 60_000);
    admin = p.connecter("admin", MOT_DE_PASSE).await;
    assert_eq!(traiter(&p).await.echoues, 1);
    let echoue = dernier(&p, &admin, "+22376000031").await;
    assert_eq!(
        (echoue["etat"].as_str(), echoue["essais"].as_u64()),
        (Some("echoue"), Some(3))
    );
    canal.retablir("+22376000031");
    p.ok(
        "message_renvoyer",
        Some(&admin),
        json!({ "id": echoue["id"] }),
    )
    .await;
    traiter(&p).await;
    assert_eq!(dernier(&p, &admin, "+22376000031").await["etat"], "remis");

    // Trois remises manquées d'affilée : le numéro devient injoignable et ne coûte plus rien.
    canal.ne_pas_remettre("+22376000041");
    for _ in 0..3 {
        essai(&p, &admin, "76 00 00 41").await;
        p.horloge.avancer(61_000);
        traiter(&p).await;
    }
    assert_eq!(
        joignabilite(&p, "+22376000041").await.as_deref(),
        Some("injoignable")
    );
    let partis = canal.envoyes().len();
    essai(&p, &admin, "76 00 00 41").await;
    traiter(&p).await;
    let mort = dernier(&p, &admin, "+22376000041").await;
    assert_eq!(
        (mort["etat"].as_str(), mort["erreur"].as_str()),
        (Some("echoue"), Some("numero_injoignable"))
    );
    assert_eq!(canal.envoyes().len(), partis);

    // Un numéro refusé par le réseau est invalide.
    canal.faire_echouer("+22376000051", ErreurEnvoi::NumeroRefuse);
    essai(&p, &admin, "76 00 00 51").await;
    traiter(&p).await;
    assert_eq!(dernier(&p, &admin, "+22376000051").await["etat"], "echoue");
    assert_eq!(
        joignabilite(&p, "+22376000051").await.as_deref(),
        Some("invalide")
    );

    // Plafond du mois atteint : rien ne part plus.
    let file = p
        .ok("messages_lire", Some(&admin), json!({ "etat": null }))
        .await;
    let consommes = file["sms_du_mois"].as_i64().unwrap();
    assert!(consommes >= 8, "{consommes}");
    regler(&p, &admin, "sms.plafond_mensuel", json!(consommes)).await;
    essai(&p, &admin, "76 00 00 61").await;
    p.horloge.avancer(61_000);
    assert_eq!(traiter(&p).await.envoyes, 0);
    assert_eq!(
        dernier(&p, &admin, "+22376000061").await["etat"],
        "en_attente"
    );

    // Annuler : avec un motif, et seulement ce qui n'est pas parti.
    let en_attente = dernier(&p, &admin, "+22376000061").await;
    assert_eq!(
        p.refus(
            "message_annuler",
            Some(&admin),
            json!({ "id": en_attente["id"], "motif": " " })
        )
        .await
        .cle,
        "sms.motif_obligatoire"
    );
    p.ok(
        "message_annuler",
        Some(&admin),
        json!({ "id": en_attente["id"], "motif": "Numéro saisi par erreur" }),
    )
    .await;
    assert_eq!(dernier(&p, &admin, "+22376000061").await["etat"], "annule");
    assert_eq!(
        p.refus(
            "message_annuler",
            Some(&admin),
            json!({ "id": parti["id"], "motif": "Trop tard" })
        )
        .await
        .cle,
        "sms.message_parti"
    );

    // Les compteurs de la file.
    let file = p
        .ok("messages_lire", Some(&admin), json!({ "etat": "echoue" }))
        .await;
    assert!(
        file["messages"]
            .as_array()
            .unwrap()
            .iter()
            .all(|m| m["message"]["etat"] == "echoue")
    );
    let compte = |etat: &str| {
        file["compteurs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["etat"] == etat)
            .map_or(0, |c| c["nombre"].as_u64().unwrap())
    };
    assert_eq!(compte("annule"), 1);
    assert_eq!(compte("expire"), 1);
    assert!(compte("remis") >= 5);

    let mut cnx = base_brute(dossier.path(), p.trousseau.clone()).await;
    let mis: i64 =
        sqlx::query_scalar("SELECT count(*) FROM outbox WHERE type_evenement = 'MessageMisEnFile'")
            .fetch_one(&mut cnx)
            .await
            .unwrap();
    assert_eq!(mis, 12);
    cnx.close().await.unwrap();
}

#[tokio::test]
async fn un_message_prepare_attend_la_decision_d_une_personne() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    regler(&p, &admin, "sms.canal", json!("simule")).await;

    // Un message de convocation préparé : il ne part pas seul.
    let mut cnx = base_brute(dossier.path(), p.trousseau.clone()).await;
    sqlx::query(
        "INSERT INTO message_sms (id, etablissement_id, nature, cle_fait, numero, texte, sms, etat,
            pas_avant, cree_le, mis_a_jour_le, identifiant_poste)
         SELECT '01JAAAAAAAAAAAAAAAAAAAAAAA', id, 'convocation', 'convocation:essai',
            '+22376000071', 'Vous etes convoque.', 1, 'a_decider', 0, 0, 0, 'secretariat'
         FROM etablissement",
    )
    .execute(&mut cnx)
    .await
    .unwrap();
    cnx.close().await.unwrap();
    assert_eq!(traiter(&p).await.envoyes, 0);
    let prepare = dernier(&p, &admin, "+22376000071").await;
    assert_eq!(prepare["etat"], "a_decider");

    // Relu et corrigé, il entre dans la file et part.
    p.ok(
        "message_decider",
        Some(&admin),
        json!({ "id": prepare["id"], "texte": "Vous êtes convoqué(e) lundi à 10 h." }),
    )
    .await;
    let decide = dernier(&p, &admin, "+22376000071").await;
    assert_eq!(decide["etat"], "en_attente");
    assert_eq!(decide["texte"], "Vous etes convoque(e) lundi a 10 h.");
    assert_eq!(
        p.refus(
            "message_decider",
            Some(&admin),
            json!({ "id": prepare["id"], "texte": "Encore" })
        )
        .await
        .cle,
        "sms.message_deja_decide"
    );
    assert_eq!(traiter(&p).await.envoyes, 1);

    // Un modèle reformulé : variables connues, trois SMS au plus ; vide, le modèle livré revient.
    assert_eq!(
        p.refus(
            "message_modele_definir",
            Some(&admin),
            json!({ "nature": "bulletin", "texte": "{eleve} doit {montant}" })
        )
        .await
        .cle,
        "sms.modele_variable_inconnue"
    );
    let file = p
        .ok(
            "message_modele_definir",
            Some(&admin),
            json!({ "nature": "bulletin", "texte": "{ecole} : bulletin de {eleve} prêt." }),
        )
        .await;
    let bulletin = file["modeles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["nature"] == "bulletin")
        .unwrap()
        .clone();
    assert_eq!(bulletin["texte"], "{ecole} : bulletin de {eleve} prêt.");
    assert!(
        bulletin["exemple"]
            .as_str()
            .unwrap()
            .ends_with(": bulletin de Aminata Kadiatou Coulibaly pret.")
    );
    let file = p
        .ok(
            "message_modele_definir",
            Some(&admin),
            json!({ "nature": "bulletin", "texte": "" }),
        )
        .await;
    assert!(
        file["modeles"][0]["texte"]
            .as_str()
            .unwrap()
            .contains("Passez le retirer")
    );

    // Aperçu d'un texte tel qu'il partirait.
    let apercu = p
        .ok(
            "message_apercu",
            Some(&admin),
            json!({ "texte": "Élève absent" }),
        )
        .await;
    assert_eq!(apercu["texte"], "Eleve absent");
    assert_eq!(apercu["sms"], 1);
    assert_eq!(
        p.refus(
            "message_essai_envoyer",
            Some(&admin),
            json!({ "numero": "12" })
        )
        .await
        .cle,
        "telephone.invalide"
    );
}
