//! Postes de l'école (SPEC_PHASE_11_A §4, critères 3, 5 et 14) : activer le réseau, relier un
//! poste par un code qui ne sert qu'une fois, expire et s'annule après cinq essais faux ; le
//! reconnaître à son secret ; le révoquer, ce qui ferme ses sessions ; renouveler la clé du
//! réseau ; nombre de postes que la licence couvre.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use gesco_core::commun::Horodatage;
use gesco_core::core::Poste;
use gesco_core::core::licence::{self, Edition, Licence};
use gesco_core::core::reseau::{ESSAIS_CODE_MAX, VALIDITE_CODE_MILLIS};
use gesco_services::Origine;
use gesco_services::session::Jeton;
use serde_json::{Value, json};

use crate::commun::*;

/// Une école installée, son administrateur connecté.
async fn ecole(racine: &std::path::Path) -> (Poste_, Jeton) {
    let p = poste_vierge(racine).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    (p, admin)
}

/// Le code affiché, tel qu'on le recopie sans son espace.
fn code_affiche(etat: &Value) -> String {
    etat["appairage"]["code"].as_str().unwrap().replace(' ', "")
}

async fn ouvrir(p: &Poste_, admin: &Jeton) -> String {
    code_affiche(
        &p.ok("poste_appairage_ouvrir", Some(admin), Value::Null)
            .await,
    )
}

fn licence_avec(modules: &[&str]) -> String {
    licence::signer(
        &Licence {
            licence_id: "LIC-TEST-POSTES".to_owned(),
            etablissement: "Lycée Askia Mohamed".to_owned(),
            edition: Edition::Essentielle,
            modules: modules.iter().map(|m| (*m).to_owned()).collect(),
            empreinte_machine: "machine-test".to_owned(),
            emise_le: Horodatage::depuis_millis_epoque(DEBUT),
            echeance: None,
        },
        &GRAINE_EDITEUR,
    )
    .unwrap()
}

#[tokio::test]
async fn relier_un_poste_puis_le_reconnaitre_a_son_secret() {
    let dossier = tempfile::tempdir().unwrap();
    let (p, admin) = ecole(dossier.path()).await;

    // À la livraison : réseau éteint, aucun poste, un poste à l'essai sans licence.
    let etat = p.ok("reseau_etat_lire", Some(&admin), Value::Null).await;
    assert_eq!(etat["actif"], false);
    assert_eq!(etat["port"], 47_800);
    assert_eq!(etat["en_service"], 0);
    assert_eq!(etat["couverts"], json!({ "type": "nombre", "nombre": 1 }));
    assert_eq!(etat["appairage"], Value::Null);
    assert_eq!(etat["nom_poste_principal"], "secretariat");

    // Réseau éteint : pas d'appairage.
    assert_eq!(
        p.refus("poste_appairage_ouvrir", Some(&admin), Value::Null)
            .await
            .cle,
        "reseau.eteint"
    );
    assert_eq!(
        p.application
            .conclure_appairage("ABCDEFGH", "Caisse")
            .await
            .unwrap_err()
            .cle,
        "reseau.aucun_appairage"
    );

    let etat = p.ok("reseau_activer", Some(&admin), Value::Null).await;
    assert_eq!(etat["actif"], true);
    let code = ouvrir(&p, &admin).await;
    assert_eq!(code.len(), 8);

    // Le poste recopie le code, avec ou sans espace, en minuscules : il est relié.
    let saisie = format!("{} {}", code[..4].to_lowercase(), &code[4..]);
    let conclu = p
        .application
        .conclure_appairage(&saisie, "  Caisse ")
        .await
        .unwrap();
    assert_eq!(conclu.nom, "Caisse");
    assert_eq!(conclu.secret.len(), 64);

    let etat = p.ok("reseau_etat_lire", Some(&admin), Value::Null).await;
    assert_eq!(etat["en_service"], 1);
    assert_eq!(etat["appairage"], Value::Null, "le code est consommé");
    let poste = &etat["postes"][0];
    assert_eq!(poste["nom"], "Caisse");
    assert_eq!(poste["id"], conclu.poste_id);
    assert_eq!(poste["revoque_le"], Value::Null);
    assert!(
        poste.get("empreinte_secret").is_none(),
        "jamais rendue à l'écran"
    );

    // Le même code ne sert pas deux fois.
    assert_eq!(
        p.application
            .conclure_appairage(&code, "Direction")
            .await
            .unwrap_err()
            .cle,
        "reseau.aucun_appairage"
    );

    // Reconnu à son identifiant et à son secret ; rien d'autre ne passe.
    assert_eq!(
        p.application
            .reconnaitre_poste(&conclu.poste_id, &conclu.secret)
            .await,
        Some(Poste("Caisse".to_owned()))
    );
    assert_eq!(
        p.application
            .reconnaitre_poste(&conclu.poste_id, &"0".repeat(64))
            .await,
        None
    );
    assert_eq!(
        p.application
            .reconnaitre_poste("pas-un-identifiant", &conclu.secret)
            .await,
        None
    );

    // Réseau éteint : le poste n'est plus reconnu, sa liaison reste ; rallumé, il l'est de nouveau.
    p.ok("reseau_eteindre", Some(&admin), Value::Null).await;
    assert_eq!(
        p.application
            .reconnaitre_poste(&conclu.poste_id, &conclu.secret)
            .await,
        None
    );
    let etat = p.ok("reseau_activer", Some(&admin), Value::Null).await;
    assert_eq!(etat["en_service"], 1);
    assert!(
        p.application
            .reconnaitre_poste(&conclu.poste_id, &conclu.secret)
            .await
            .is_some()
    );

    // Le journal garde la liaison, sans le secret ni son empreinte.
    let journal = p
        .ok(
            "audit_lister",
            Some(&admin),
            json!({ "entite": "poste_relie", "page": 0 }),
        )
        .await;
    let ligne = &journal["lignes"][0];
    assert_eq!(ligne["contexte"], "postes.relier");
    assert_eq!(ligne["apres"]["nom"], "Caisse");
    let texte = journal.to_string();
    assert!(!texte.contains(&conclu.secret));
    assert!(!texte.contains("empreinte_secret"));
}

#[tokio::test]
async fn un_code_faux_expire_ou_use_ne_relie_rien() {
    let dossier = tempfile::tempdir().unwrap();
    let (p, admin) = ecole(dossier.path()).await;
    p.ok("reseau_activer", Some(&admin), Value::Null).await;

    // Cinq essais faux annulent le code : le bon code lui-même ne sert plus.
    let code = ouvrir(&p, &admin).await;
    for essai in 1..=ESSAIS_CODE_MAX {
        let cle = p
            .application
            .conclure_appairage("ZZZZZZZZ", "Caisse")
            .await
            .unwrap_err()
            .cle;
        let attendue = if essai < ESSAIS_CODE_MAX {
            "reseau.code_faux"
        } else {
            "reseau.code_annule"
        };
        assert_eq!(cle, attendue, "essai {essai}");
    }
    assert_eq!(
        p.application
            .conclure_appairage(&code, "Caisse")
            .await
            .unwrap_err()
            .cle,
        "reseau.aucun_appairage"
    );

    // Un code expire au bout de dix minutes.
    let code = ouvrir(&p, &admin).await;
    p.horloge.avancer(VALIDITE_CODE_MILLIS);
    assert_eq!(
        p.application
            .conclure_appairage(&code, "Caisse")
            .await
            .unwrap_err()
            .cle,
        "reseau.code_expire"
    );
    let etat = p.ok("reseau_etat_lire", Some(&admin), Value::Null).await;
    assert_eq!(etat["appairage"], Value::Null);
    assert_eq!(etat["en_service"], 0);

    // Un nom vide ou trop long est refusé avant même de regarder le code, qui reste valable.
    let code = ouvrir(&p, &admin).await;
    assert_eq!(
        p.application
            .conclure_appairage(&code, "   ")
            .await
            .unwrap_err()
            .cle,
        "reseau.nom_poste_vide"
    );
    // Le nom du poste principal est pris ; le code reste valable pour un autre nom.
    assert_eq!(
        p.application
            .conclure_appairage(&code, "SECRETARIAT")
            .await
            .unwrap_err()
            .cle,
        "reseau.nom_poste_pris"
    );
    assert!(
        p.application
            .conclure_appairage(&code, "Caisse")
            .await
            .is_ok()
    );

    // Les refus sont au journal, sans le code présenté.
    let journal = p
        .ok(
            "audit_lister",
            Some(&admin),
            json!({ "entite": "reseau_local", "page": 0 }),
        )
        .await;
    let refus: Vec<&str> = journal["lignes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|l| l["apres"]["evenement"] == "appairage_refuse")
        .map(|l| l["apres"]["motif"].as_str().unwrap())
        .collect();
    assert_eq!(refus.len(), 6);
    assert_eq!(refus.iter().filter(|m| **m == "faux").count(), 4);
    assert!(refus.contains(&"annule"));
    assert!(refus.contains(&"expire"));
    assert!(!journal.to_string().contains("ZZZZZZZZ"));
}

#[tokio::test]
async fn un_poste_revoque_est_refuse_et_ses_sessions_sont_fermees() {
    let dossier = tempfile::tempdir().unwrap();
    let (p, admin) = ecole(dossier.path()).await;
    p.ok("reseau_activer", Some(&admin), Value::Null).await;
    let code = ouvrir(&p, &admin).await;
    let conclu = p
        .application
        .conclure_appairage(&code, "Caisse")
        .await
        .unwrap();

    // Une session ouverte depuis le poste relié.
    let caisse = Origine::Relie(Poste("Caisse".to_owned()));
    let session = p
        .application
        .executer_depuis(
            caisse.clone(),
            "connexion",
            None,
            json!({ "identifiant": "admin", "mot_de_passe": MOT_DE_PASSE }),
        )
        .await
        .unwrap();
    let jeton: Jeton = serde_json::from_value(session["jeton"].clone()).unwrap();
    assert!(
        p.application
            .executer_depuis(
                caisse.clone(),
                "session_courante",
                Some(jeton.clone()),
                Value::Null
            )
            .await
            .is_ok()
    );

    let etat = p
        .ok(
            "poste_revoquer",
            Some(&admin),
            json!({ "poste_id": conclu.poste_id }),
        )
        .await;
    assert_eq!(etat["en_service"], 0);
    assert_ne!(etat["postes"][0]["revoque_le"], Value::Null);

    // Refusé à l'appel suivant ; sa session n'existe plus ; celle du poste principal, si.
    assert_eq!(
        p.application
            .reconnaitre_poste(&conclu.poste_id, &conclu.secret)
            .await,
        None
    );
    assert_eq!(
        p.application
            .executer_depuis(caisse, "session_courante", Some(jeton), Value::Null)
            .await
            .unwrap_err()
            .cle,
        "session.invalide"
    );
    p.ok("session_courante", Some(&admin), Value::Null).await;

    // Révoquer deux fois, ou un poste inconnu, se refuse.
    assert_eq!(
        p.refus(
            "poste_revoquer",
            Some(&admin),
            json!({ "poste_id": conclu.poste_id })
        )
        .await
        .cle,
        "reseau.poste_deja_revoque"
    );
    assert_eq!(
        p.refus(
            "poste_revoquer",
            Some(&admin),
            json!({ "poste_id": "01ARZ3NDEKTSV4RRFFQ69G5FAV" })
        )
        .await
        .cle,
        "reseau.poste_inconnu"
    );

    // Son nom redevient libre : un autre poste peut le prendre.
    let code = ouvrir(&p, &admin).await;
    assert!(
        p.application
            .conclure_appairage(&code, "Caisse")
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn la_licence_dit_combien_de_postes_se_relient() {
    let dossier = tempfile::tempdir().unwrap();
    let (p, admin) = ecole(dossier.path()).await;
    p.ok("reseau_activer", Some(&admin), Value::Null).await;

    // Sans licence : un poste à l'essai, pas deux.
    let code = ouvrir(&p, &admin).await;
    let premier = p
        .application
        .conclure_appairage(&code, "Caisse")
        .await
        .unwrap();
    assert_eq!(
        p.refus("poste_appairage_ouvrir", Some(&admin), Value::Null)
            .await
            .cle,
        "reseau.postes_tous_pris"
    );

    // Licence pour deux postes : un de plus, pas trois.
    p.ok(
        "licence_installer",
        Some(&admin),
        json!({ "contenu": licence_avec(&["postes:2"]) }),
    )
    .await;
    let etat = p.ok("reseau_etat_lire", Some(&admin), Value::Null).await;
    assert_eq!(etat["couverts"], json!({ "type": "nombre", "nombre": 2 }));
    let code = ouvrir(&p, &admin).await;
    p.application
        .conclure_appairage(&code, "Direction")
        .await
        .unwrap();
    assert_eq!(
        p.refus("poste_appairage_ouvrir", Some(&admin), Value::Null)
            .await
            .cle,
        "reseau.postes_tous_pris"
    );

    // Un poste révoqué rend sa place.
    p.ok(
        "poste_revoquer",
        Some(&admin),
        json!({ "poste_id": premier.poste_id }),
    )
    .await;
    let code = ouvrir(&p, &admin).await;
    p.application
        .conclure_appairage(&code, "Surveillance")
        .await
        .unwrap();

    // Formule « postes compris » : autant qu'on veut.
    p.ok(
        "licence_installer",
        Some(&admin),
        json!({ "contenu": licence_avec(&["postes:illimites"]) }),
    )
    .await;
    let etat = p.ok("reseau_etat_lire", Some(&admin), Value::Null).await;
    assert_eq!(etat["couverts"], json!({ "type": "illimites" }));
    for nom in ["Comptable", "Etudes", "Accueil"] {
        let code = ouvrir(&p, &admin).await;
        p.application.conclure_appairage(&code, nom).await.unwrap();
    }

    // La licence retirée de la portée (une licence sans mention) : rien n'est coupé, seul l'ajout
    // d'un poste est refusé.
    p.ok(
        "licence_installer",
        Some(&admin),
        json!({ "contenu": licence_avec(&[]) }),
    )
    .await;
    let etat = p.ok("reseau_etat_lire", Some(&admin), Value::Null).await;
    assert_eq!(etat["en_service"], 5);
    assert_eq!(
        p.refus("poste_appairage_ouvrir", Some(&admin), Value::Null)
            .await
            .cle,
        "reseau.postes_tous_pris"
    );
}

#[tokio::test]
async fn renouveler_la_cle_du_reseau_oblige_a_relier_chaque_poste() {
    let dossier = tempfile::tempdir().unwrap();
    let (p, admin) = ecole(dossier.path()).await;

    // Réseau éteint : le serveur n'a rien à servir.
    assert_eq!(p.application.reseau_a_servir().await.unwrap(), None);
    let mut suivi = p.application.suivre_reseau();
    suivi.borrow_and_update();

    p.ok("reseau_activer", Some(&admin), Value::Null).await;
    assert!(suivi.has_changed().unwrap(), "le serveur est prévenu");
    let a_servir = p.application.reseau_a_servir().await.unwrap().unwrap();
    assert_eq!(a_servir.port, 47_800);
    assert_eq!(a_servir.certificat, None);

    // Le serveur dépose le certificat qu'il a fabriqué ; un second dépôt ne le remplace pas.
    p.application
        .enregistrer_certificat_reseau(b"certificat".to_vec(), b"cle".to_vec())
        .await
        .unwrap();
    p.application
        .enregistrer_certificat_reseau(b"autre".to_vec(), b"autre-cle".to_vec())
        .await
        .unwrap();
    let a_servir = p.application.reseau_a_servir().await.unwrap().unwrap();
    assert_eq!(
        a_servir.certificat,
        Some((b"certificat".to_vec(), b"cle".to_vec()))
    );
    let etat = p.ok("reseau_etat_lire", Some(&admin), Value::Null).await;
    let empreinte = etat["empreinte_courte"].as_str().unwrap().to_owned();
    assert_eq!(empreinte.len(), 19);

    let code = ouvrir(&p, &admin).await;
    let conclu = p
        .application
        .conclure_appairage(&code, "Caisse")
        .await
        .unwrap();

    // Clé renouvelée : plus de certificat, le poste est révoqué, le serveur est prévenu.
    suivi.borrow_and_update();
    let etat = p
        .ok("reseau_cle_renouveler", Some(&admin), Value::Null)
        .await;
    assert_eq!(etat["empreinte_courte"], Value::Null);
    assert_eq!(etat["en_service"], 0);
    assert!(suivi.has_changed().unwrap());
    assert_eq!(
        p.application
            .reconnaitre_poste(&conclu.poste_id, &conclu.secret)
            .await,
        None
    );
    assert_eq!(
        p.application
            .reseau_a_servir()
            .await
            .unwrap()
            .unwrap()
            .certificat,
        None
    );
}

#[tokio::test]
async fn les_postes_se_gerent_du_poste_principal_avec_le_droit() {
    let dossier = tempfile::tempdir().unwrap();
    let (p, admin) = ecole(dossier.path()).await;
    // Depuis un poste relié, même l'administrateur ne touche pas au réseau.
    let caisse = Origine::Relie(Poste("Caisse".to_owned()));
    let session = p
        .application
        .executer_depuis(
            caisse.clone(),
            "connexion",
            None,
            json!({ "identifiant": "admin", "mot_de_passe": MOT_DE_PASSE }),
        )
        .await
        .unwrap();
    let jeton: Jeton = serde_json::from_value(session["jeton"].clone()).unwrap();
    for commande in [
        "reseau_etat_lire",
        "reseau_activer",
        "reseau_eteindre",
        "poste_appairage_ouvrir",
        "poste_appairage_fermer",
        "poste_revoquer",
        "reseau_cle_renouveler",
    ] {
        assert_eq!(
            p.application
                .executer_depuis(caisse.clone(), commande, Some(jeton.clone()), Value::Null)
                .await
                .unwrap_err()
                .cle,
            "commande.poste_principal_seulement",
            "{commande}"
        );
    }
    // Le droit est celui du Directeur et de l'Administrateur.
    let courante = p.ok("session_courante", Some(&admin), Value::Null).await;
    let permissions: Vec<String> = serde_json::from_value(courante["permissions"].clone()).unwrap();
    assert!(permissions.contains(&"workstations:manage".to_owned()));
}
