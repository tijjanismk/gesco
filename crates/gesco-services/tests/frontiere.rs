//! Tests de bout en bout par la frontière, comme le frontend l'appelle : premier démarrage,
//! garde de permission (assertion 4), document d'essai (assertion 8), sauvegarde et restauration
//! sur un poste vierge avec la seule phrase de secours (assertion 9, R-57), phrase fausse,
//! régénération, test de restauration, destinations.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use base64::Engine;
use gesco_core::commun::Horodatage;
use gesco_core::core::licence::{self, Edition, Licence};
use gesco_services::Application;
use gesco_services::archive::lire_manifeste;
use gesco_services::cles::TrousseauMemoire;
use gesco_services::commandes::COMMANDES;
use gesco_services::frontiere::COMMANDES_IMPLEMENTEES;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

mod commun;

use commun::*;

#[test]
fn le_contrat_couvre_exactement_le_catalogue_des_acces() {
    let contrat: HashSet<&str> = COMMANDES_IMPLEMENTEES.iter().copied().collect();
    let catalogue: HashSet<&str> = COMMANDES.iter().map(|c| c.nom).collect();
    assert_eq!(contrat, catalogue);
    assert_eq!(contrat.len(), COMMANDES_IMPLEMENTEES.len());
}

#[tokio::test]
async fn premier_demarrage_puis_connexion() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    assert_eq!(p.phase().await, "premier_demarrage");
    assert_eq!(
        p.refus(
            "connexion",
            None,
            json!({ "identifiant": "admin", "mot_de_passe": "x" })
        )
        .await
        .cle,
        "premier_demarrage.requis"
    );
    p.installer().await;
    assert_eq!(p.phase().await, "pret");
    assert_eq!(
        p.refus(
            "premier_demarrage",
            None,
            json!({
                "organisation": "", "etablissement_nom": "Autre", "statut": "public",
                "code_academie": "", "code_cap": "", "adresse": "", "telephones": [],
                "administrateur_identifiant": "x", "administrateur_nom": "x",
                "administrateur_mot_de_passe": "mot-de-passe-long",
                "cycles": ["lycee"],
            })
        )
        .await
        .cle,
        "premier_demarrage.deja_effectue"
    );
    let jeton = p.connecter("admin", MOT_DE_PASSE).await;
    let courante = p.ok("session_courante", Some(&jeton), Value::Null).await;
    assert_eq!(courante["session"]["nom_affiche"], "Awa Traoré");
    assert_eq!(courante["doit_changer_mot_de_passe"], true);
    let permissions: Vec<String> = serde_json::from_value(courante["permissions"].clone()).unwrap();
    assert!(permissions.contains(&"core:backup:restore".to_owned()));
    let alertes = avertissements(&p.ok("avertissements", Some(&jeton), Value::Null).await);
    assert!(alertes.contains("avertissement.certificat_non_confirme"));
    assert!(alertes.contains("avertissement.aucune_destination_amovible"));
    p.ok("certificat_confirmer", Some(&jeton), Value::Null)
        .await;
    let alertes = avertissements(&p.ok("avertissements", Some(&jeton), Value::Null).await);
    assert!(!alertes.contains("avertissement.certificat_non_confirme"));
}

#[tokio::test]
async fn une_commande_sans_permission_est_refusee_par_la_garde() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    p.ok(
        "utilisateur_creer",
        Some(&admin),
        json!({
            "identifiant": "fanta",
            "nom_affiche": "Fanta Coulibaly",
            "mot_de_passe_initial": "secretariat-2026",
            "roles": ["secretaire"],
        }),
    )
    .await;
    let secretaire = p.connecter("fanta", "secretariat-2026").await;
    for commande in ["sauvegarde_creer", "audit_lister", "utilisateurs_lister"] {
        let refus = p
            .refus(commande, Some(&secretaire), json!({ "limite": 10 }))
            .await;
        assert_eq!(refus.cle, "erreur.permission_refusee", "{commande}");
    }
    assert_eq!(
        p.refus("sauvegarde_creer", None, Value::Null).await.cle,
        "session.invalide"
    );
    assert_eq!(
        p.refus("commande_inexistante", Some(&admin), Value::Null)
            .await
            .cle,
        "commande.inconnue"
    );
    let fiches = p.ok("utilisateurs_lister", Some(&admin), Value::Null).await;
    assert_eq!(fiches.as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn le_document_d_essai_est_enregistre_avec_son_empreinte_et_sa_version() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let produit = p
        .ok("document_essai_produire", Some(&admin), Value::Null)
        .await;
    let pdf = base64::engine::general_purpose::STANDARD
        .decode(produit["pdf"].as_str().unwrap())
        .unwrap();
    let document = &produit["document"];
    assert_eq!(
        document["empreinte_sha256"],
        hex::encode(Sha256::digest(&pdf))
    );
    assert_eq!(document["modele_id"], "essai");
    assert_eq!(document["modele_version"], 1);
    let sur_disque = std::fs::read(document["chemin"].as_str().unwrap()).unwrap();
    assert_eq!(sur_disque, pdf);
    let journal = p
        .ok("audit_lister", Some(&admin), json!({ "page": 0 }))
        .await;
    assert_eq!(journal["lignes"][0]["entite"], "document");
    assert!(
        p.ok("audit_verifier", Some(&admin), Value::Null).await["lignes_verifiees"]
            .as_u64()
            .unwrap()
            >= 5
    );
}

#[tokio::test]
async fn restauration_sur_un_poste_vierge_avec_la_seule_phrase_de_secours() {
    let dossier = tempfile::tempdir().unwrap();
    let origine = poste_vierge(&dossier.path().join("origine")).await;
    let phrase = origine.installer().await;
    let admin = origine.connecter("admin", MOT_DE_PASSE).await;
    origine
        .ok("document_essai_produire", Some(&admin), Value::Null)
        .await;
    let archive = origine
        .sauvegarder(&admin, &dossier.path().join("cle-usb"))
        .await;
    let manifeste = lire_manifeste(&archive).unwrap();
    origine.application.fermer().await.unwrap();
    drop(origine);

    // Poste vierge : autre dossier, autre trousseau. Seuls intrants : l'archive et la phrase.
    let neuf = poste_vierge(&dossier.path().join("neuf")).await;
    assert_eq!(neuf.phase().await, "premier_demarrage");
    let apercu = neuf
        .ok("sauvegarde_apercu", None, json!({ "chemin": archive }))
        .await;
    assert_eq!(apercu["etablissement_nom"], "Lycée Askia Mohamed");
    assert_eq!(
        neuf.refus(
            "sauvegarde_restaurer_initiale",
            None,
            json!({ "chemin": archive, "phrase": null })
        )
        .await
        .cle,
        "sauvegarde.phrase_requise"
    );
    let fausse = ["abaisser"; 12].join(" ");
    assert_eq!(
        neuf.refus(
            "sauvegarde_restaurer_initiale",
            None,
            json!({ "chemin": archive, "phrase": fausse })
        )
        .await
        .cle,
        "cles.phrase_invalide"
    );
    let saisie = phrase.to_uppercase().replace(['É', 'È'], "E");
    let rapport = neuf
        .ok(
            "sauvegarde_restaurer_initiale",
            None,
            json!({ "chemin": archive, "phrase": saisie }),
        )
        .await;
    assert_eq!(rapport["par_phrase"], true);
    assert_eq!(rapport["empreinte_contenu"], manifeste.empreinte_contenu);
    assert_eq!(neuf.phase().await, "pret");
    let jeton = neuf.connecter("admin", MOT_DE_PASSE).await;
    neuf.ok("audit_verifier", Some(&jeton), Value::Null).await;
    let journal = neuf
        .ok("audit_lister", Some(&jeton), json!({ "page": 0 }))
        .await;
    assert!(
        journal["lignes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|l| l["contexte"] == "sauvegarde.restaurer"),
        "la restauration laisse sa trace dans la base restaurée"
    );

    // Le poste a reçu sa propre enveloppe machine : le redémarrage ne demande plus rien.
    let (racine, trousseau, horloge) = (
        neuf.racine.clone(),
        Arc::clone(&neuf.trousseau),
        neuf.horloge.clone(),
    );
    neuf.application.fermer().await.unwrap();
    drop(neuf);
    let relance = demarrer(&racine, trousseau, horloge).await;
    assert_eq!(relance.phase().await, "pret");
}

#[tokio::test]
async fn trousseau_perdu_puis_phrase_fausse_avec_attente_progressive() {
    let dossier = tempfile::tempdir().unwrap();
    let racine = dossier.path().join("poste");
    let p = poste_vierge(&racine).await;
    let phrase = p.installer().await;
    let horloge = p.horloge.clone();
    p.application.fermer().await.unwrap();
    drop(p);

    // Réinstallation du système : le trousseau est vide, la clé attend la phrase.
    let p = demarrer(
        &racine,
        Arc::new(TrousseauMemoire::default()),
        horloge.clone(),
    )
    .await;
    assert_eq!(p.phase().await, "cles_verrouillees");
    assert_eq!(
        p.refus(
            "connexion",
            None,
            json!({ "identifiant": "admin", "mot_de_passe": MOT_DE_PASSE })
        )
        .await
        .cle,
        "application.cles_verrouillees"
    );
    for _ in 0..3 {
        let refus = p
            .refus(
                "cles_deverrouiller_phrase",
                None,
                json!({ "phrase": "un deux trois" }),
            )
            .await;
        assert_eq!(refus.cle, "cles.phrase_mal_formee");
    }
    let attente = p
        .refus(
            "cles_deverrouiller_phrase",
            None,
            json!({ "phrase": phrase }),
        )
        .await;
    assert_eq!(attente.cle, "cles.phrase_verrouillee");
    assert_eq!(attente.jusqua, Some(DEBUT + 30_000));
    horloge.avancer(30_000);
    let etat = p
        .ok(
            "cles_deverrouiller_phrase",
            None,
            json!({ "phrase": phrase }),
        )
        .await;
    assert_eq!(etat["phase"], "pret");
    p.connecter("admin", MOT_DE_PASSE).await;
}

#[tokio::test]
async fn la_phrase_regeneree_remplace_l_ancienne() {
    let dossier = tempfile::tempdir().unwrap();
    let origine = poste_vierge(&dossier.path().join("origine")).await;
    let ancienne = origine.installer().await;
    let admin = origine.connecter("admin", MOT_DE_PASSE).await;
    origine
        .ok("certificat_confirmer", Some(&admin), Value::Null)
        .await;
    assert_eq!(
        origine
            .refus(
                "phrase_secours_regenerer",
                Some(&admin),
                json!({ "mot_de_passe": "faux-mot-de-passe" })
            )
            .await
            .cle,
        "connexion.identifiants_invalides"
    );
    let nouvelle = origine
        .ok(
            "phrase_secours_regenerer",
            Some(&admin),
            json!({ "mot_de_passe": MOT_DE_PASSE }),
        )
        .await;
    let nouvelle: Vec<String> = serde_json::from_value(nouvelle["mots"].clone()).unwrap();
    let nouvelle = nouvelle.join(" ");
    assert_ne!(nouvelle, ancienne);
    let alertes = avertissements(
        &origine
            .ok("avertissements", Some(&admin), Value::Null)
            .await,
    );
    assert!(alertes.contains("avertissement.certificat_non_confirme"));
    let archive = origine
        .sauvegarder(&admin, &dossier.path().join("cle-usb"))
        .await;
    origine.application.fermer().await.unwrap();

    let neuf = poste_vierge(&dossier.path().join("neuf")).await;
    assert_eq!(
        neuf.refus(
            "sauvegarde_restaurer_initiale",
            None,
            json!({ "chemin": archive, "phrase": ancienne })
        )
        .await
        .cle,
        "cles.phrase_invalide"
    );
    neuf.ok(
        "sauvegarde_restaurer_initiale",
        None,
        json!({ "chemin": archive, "phrase": nouvelle }),
    )
    .await;
}

#[tokio::test]
async fn test_de_restauration_et_archive_alteree() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    p.horloge.avancer(60_000);
    let lignes = p.ok("sauvegarde_creer", Some(&admin), Value::Null).await;
    let archive = PathBuf::from(lignes[0]["destination"].as_str().unwrap());
    let rapport = p.ok("sauvegarde_tester", Some(&admin), Value::Null).await;
    assert_eq!(rapport["reussi"], true, "{rapport}");
    let liste = p.ok("sauvegardes_lister", Some(&admin), Value::Null).await;
    assert!(liste[0]["dernier_test_restauration"].is_number());

    let mut octets = std::fs::read(&archive).unwrap();
    let milieu = octets.len() - 40;
    octets[milieu] ^= 0x55;
    std::fs::write(&archive, &octets).unwrap();
    let rapport = p.ok("sauvegarde_tester", Some(&admin), Value::Null).await;
    assert_eq!(rapport["reussi"], false);
    assert_eq!(rapport["motif_echec"], "sauvegarde.archive_alteree");
    let alertes = avertissements(&p.ok("avertissements", Some(&admin), Value::Null).await);
    assert!(alertes.contains("avertissement.test_restauration_echoue"));

    // Une archive tronquée est refusée, et la base en service reste intacte.
    octets.truncate(octets.len() / 2);
    let tronquee = dossier.path().join("tronquee.gsv");
    std::fs::write(&tronquee, &octets).unwrap();
    let refus = p
        .refus(
            "sauvegarde_restaurer",
            Some(&admin),
            json!({ "chemin": tronquee, "phrase": null }),
        )
        .await;
    assert_eq!(refus.cle, "sauvegarde.archive_alteree");
    let jeton = p.connecter("admin", MOT_DE_PASSE).await;
    p.ok("audit_verifier", Some(&jeton), Value::Null).await;
}

#[tokio::test]
async fn restaurer_une_archive_locale_du_jour_sans_phrase() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    p.horloge.avancer(60_000);
    let lignes = p.ok("sauvegarde_creer", Some(&admin), Value::Null).await;
    let archive = PathBuf::from(lignes[0]["destination"].as_str().unwrap());
    p.horloge.avancer(60_000);

    // La sauvegarde de sécurité qui précède la restauration ne doit pas emporter, par la
    // rotation quotidienne, l'archive choisie dans le dossier local.
    let rapport = p
        .ok(
            "sauvegarde_restaurer",
            Some(&admin),
            json!({ "chemin": archive, "phrase": null }),
        )
        .await;
    assert_eq!(rapport["par_phrase"], false);
    assert!(archive.exists());
    let jeton = p.connecter("admin", MOT_DE_PASSE).await;
    p.ok("audit_verifier", Some(&jeton), Value::Null).await;
}

#[tokio::test]
async fn une_destination_amovible_recoit_la_meme_archive() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(&dossier.path().join("poste")).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let usb = dossier.path().join("usb");
    let destinations = p
        .ok(
            "sauvegarde_destinations_definir",
            Some(&admin),
            json!([{ "chemin": usb, "amovible": true }]),
        )
        .await;
    assert_eq!(destinations.as_array().unwrap().len(), 2);
    let alertes = avertissements(&p.ok("avertissements", Some(&admin), Value::Null).await);
    assert!(!alertes.contains("avertissement.aucune_destination_amovible"));
    p.horloge.avancer(60_000);
    let lignes = p.ok("sauvegarde_creer", Some(&admin), Value::Null).await;
    let lignes = lignes.as_array().unwrap();
    assert_eq!(lignes.len(), 2);
    assert!(lignes.iter().all(|l| l["resultat"] == "reussie"));
    assert_eq!(lignes[0]["empreinte_sha256"], lignes[1]["empreinte_sha256"]);
    assert!(Path::new(lignes[1]["destination"].as_str().unwrap()).starts_with(&usb));
}

/// Le binaire Tauri exécute les commandes sur un fil quelconque et partage l'application entre
/// fils : ses futures doivent être `Send`, elle-même `Send + Sync`. Vérifié à la compilation.
#[test]
fn la_frontiere_se_partage_entre_fils() {
    fn exige_send<T: Send>(_: T) {}
    fn exige_partage<T: Send + Sync>() {}
    exige_partage::<Application>();
    let _verification = |application: &'static Application| {
        exige_send(application.executer("etat_demarrage", None, Value::Null));
        exige_send(application.taches_periodiques());
        exige_send(application.fermer());
    };
}

fn licence_pour(empreinte: &str, graine: &[u8; 32]) -> String {
    licence::signer(
        &Licence {
            licence_id: "LIC-TEST-0001".to_owned(),
            etablissement: "Lycée Askia Mohamed".to_owned(),
            edition: Edition::Essentielle,
            modules: vec![],
            empreinte_machine: empreinte.to_owned(),
            emise_le: Horodatage::depuis_millis_epoque(DEBUT),
            echeance: None,
        },
        graine,
    )
    .unwrap()
}

#[tokio::test]
async fn la_licence_s_installe_sans_jamais_bloquer_le_poste() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;

    // Sans licence : tout fonctionne, le bandeau le signale.
    let etat = p.ok("licence_etat", Some(&admin), Value::Null).await;
    assert_eq!(etat["etat"]["statut"], "absente");
    assert_eq!(etat["empreinte_poste"], "machine-test");
    let alertes = avertissements(&p.ok("avertissements", Some(&admin), Value::Null).await);
    assert!(alertes.contains("avertissement.licence_absente"));

    // Signature d'une autre clé, fichier quelconque : refusés.
    let contrefaite = licence_pour("machine-test", &[1; 32]);
    let refus = p
        .refus(
            "licence_installer",
            Some(&admin),
            json!({ "contenu": contrefaite }),
        )
        .await;
    assert_eq!(refus.cle, "licence.signature_invalide");
    let refus = p
        .refus(
            "licence_installer",
            Some(&admin),
            json!({ "contenu": "bonjour" }),
        )
        .await;
    assert_eq!(refus.cle, "licence.illisible");

    // Licence émise pour ce poste.
    let etat = p
        .ok(
            "licence_installer",
            Some(&admin),
            json!({ "contenu": licence_pour("machine-test", &GRAINE_EDITEUR) }),
        )
        .await;
    assert_eq!(etat["etat"]["statut"], "valide");
    assert_eq!(etat["licence"]["licence_id"], "LIC-TEST-0001");
    let alertes = avertissements(&p.ok("avertissements", Some(&admin), Value::Null).await);
    assert!(
        !alertes
            .iter()
            .any(|a| a.starts_with("avertissement.licence"))
    );
    let journal = p
        .ok("audit_lister", Some(&admin), json!({ "page": 0 }))
        .await;
    assert!(
        journal["lignes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|l| l["entite"] == "licence")
    );

    // Licence d'une autre machine (disque changé) : à réémettre, rien n'est bloqué.
    p.ok(
        "licence_installer",
        Some(&admin),
        json!({ "contenu": licence_pour("autre-machine", &GRAINE_EDITEUR) }),
    )
    .await;
    let etat = p.ok("licence_etat", Some(&admin), Value::Null).await;
    assert_eq!(etat["etat"]["statut"], "a_reemettre");
    let alertes = avertissements(&p.ok("avertissements", Some(&admin), Value::Null).await);
    assert!(alertes.contains("avertissement.licence_a_reemettre"));
    p.ok("utilisateurs_lister", Some(&admin), Value::Null).await;

    // Licence libre numérotée : elle s'attache au poste qui l'installe.
    let etat = p
        .ok(
            "licence_installer",
            Some(&admin),
            json!({ "contenu": licence_pour(licence::EMPREINTE_LIBRE, &GRAINE_EDITEUR) }),
        )
        .await;
    assert_eq!(etat["etat"]["statut"], "valide");
    assert_eq!(etat["empreinte_attachee"], "machine-test");
    let alertes = avertissements(&p.ok("avertissements", Some(&admin), Value::Null).await);
    assert!(
        !alertes
            .iter()
            .any(|a| a.starts_with("avertissement.licence"))
    );
}

fn permissions_de(fiche: &Value) -> Vec<String> {
    serde_json::from_value(fiche["permissions"].clone()).unwrap()
}

#[tokio::test]
async fn un_role_personnalise_se_cree_se_recompose_et_se_supprime() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;

    let catalogue = p
        .ok("permissions_catalogue", Some(&admin), Value::Null)
        .await;
    assert!(catalogue.as_array().unwrap().len() > 70);

    let cree = p
        .ok(
            "role_creer",
            Some(&admin),
            json!({ "libelle": "Agent d'accueil", "permissions": ["documents:reprint", "students:read"] }),
        )
        .await;
    assert_eq!(cree["role"]["code"], "perso_agent_d_accueil");
    assert_eq!(cree["role"]["systeme"], false);
    assert_eq!(
        permissions_de(&cree),
        ["students:read", "documents:reprint"]
    );

    let modifie = p
        .ok(
            "role_modifier",
            Some(&admin),
            json!({
                "code": "perso_agent_d_accueil",
                "libelle": "Accueil",
                "permissions": ["students:read", "attendance:create"],
            }),
        )
        .await;
    assert_eq!(modifie["role"]["libelle"], "Accueil");
    assert_eq!(
        permissions_de(&modifie),
        ["students:read", "attendance:create"]
    );

    // Q-43 : notes et caisse ne se cumulent pas, même dans un rôle personnalisé.
    let refus = p
        .refus(
            "role_creer",
            Some(&admin),
            json!({ "libelle": "Tout-en-un", "permissions": ["grades:create", "payments:create"] }),
        )
        .await;
    assert_eq!(refus.cle, "role.cumul_notes_caisse");
    for (commande, entree, cle) in [
        (
            "role_creer",
            json!({ "libelle": "accueil", "permissions": [] }),
            "role.libelle_existant",
        ),
        (
            "role_creer",
            json!({ "libelle": "x", "permissions": ["grades:fly"] }),
            "role.permission_inconnue",
        ),
        (
            "role_modifier",
            json!({ "code": "administrateur", "libelle": "", "permissions": [] }),
            "role.administrateur_intouchable",
        ),
        (
            "role_supprimer",
            json!({ "code": "secretaire" }),
            "role.livre_non_supprimable",
        ),
        (
            "role_retablir",
            json!({ "code": "perso_agent_d_accueil" }),
            "role.non_retablissable",
        ),
    ] {
        assert_eq!(
            p.refus(commande, Some(&admin), entree).await.cle,
            cle,
            "{commande}"
        );
    }

    p.ok(
        "utilisateur_creer",
        Some(&admin),
        json!({
            "identifiant": "awa",
            "nom_affiche": "Awa Diarra",
            "mot_de_passe_initial": "accueil-2026-bureau",
            "roles": ["perso_agent_d_accueil"],
        }),
    )
    .await;
    let refus = p
        .refus(
            "role_supprimer",
            Some(&admin),
            json!({ "code": "perso_agent_d_accueil" }),
        )
        .await;
    assert_eq!(refus.cle, "role.encore_attribue");
    let fiches = p.ok("utilisateurs_lister", Some(&admin), Value::Null).await;
    let awa = fiches
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["utilisateur"]["identifiant"] == "awa")
        .unwrap()["utilisateur"]["id"]
        .clone();
    p.ok(
        "role_attribuer",
        Some(&admin),
        json!({ "utilisateur_id": awa, "role": "perso_agent_d_accueil", "attribue": false }),
    )
    .await;
    p.ok(
        "role_supprimer",
        Some(&admin),
        json!({ "code": "perso_agent_d_accueil" }),
    )
    .await;
    let roles = p.ok("roles_lister", Some(&admin), Value::Null).await;
    assert_eq!(roles.as_array().unwrap().len(), 10);
    // Un code n'est jamais réemployé : un nouveau rôle de même nom reçoit un suffixe.
    let recree = p
        .ok(
            "role_creer",
            Some(&admin),
            json!({ "libelle": "Agent d'accueil", "permissions": [] }),
        )
        .await;
    assert_eq!(recree["role"]["code"], "perso_agent_d_accueil_2");
    p.ok("audit_verifier", Some(&admin), Value::Null).await;
}

#[tokio::test]
async fn un_role_livre_se_recompose_puis_se_retablit() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let roles = p.ok("roles_lister", Some(&admin), Value::Null).await;
    let secretaire = roles
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["role"]["code"] == "secretaire")
        .unwrap();
    let origine = permissions_de(secretaire);
    let reduite: Vec<&String> = origine.iter().filter(|c| *c != "students:merge").collect();
    let modifie = p
        .ok(
            "role_modifier",
            Some(&admin),
            json!({ "code": "secretaire", "libelle": "Autre nom", "permissions": reduite }),
        )
        .await;
    assert_eq!(
        modifie["role"]["libelle"], "Secrétaire",
        "un rôle livré garde son libellé"
    );
    assert_eq!(permissions_de(&modifie).len(), origine.len() - 1);
    let retabli = p
        .ok(
            "role_retablir",
            Some(&admin),
            json!({ "code": "secretaire" }),
        )
        .await;
    assert_eq!(permissions_de(&retabli), origine);
}

#[tokio::test]
async fn on_n_accorde_pas_une_permission_que_l_on_ne_detient_pas() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    p.ok(
        "role_creer",
        Some(&admin),
        json!({ "libelle": "Gestion des rôles", "permissions": ["core:roles:assign", "core:users:read"] }),
    )
    .await;
    p.ok(
        "utilisateur_creer",
        Some(&admin),
        json!({
            "identifiant": "moussa",
            "nom_affiche": "Moussa Keïta",
            "mot_de_passe_initial": "roles-2026-gestion",
            "roles": ["perso_gestion_des_roles"],
        }),
    )
    .await;
    let moussa = p.connecter("moussa", "roles-2026-gestion").await;
    let refus = p
        .refus(
            "role_modifier",
            Some(&moussa),
            json!({
                "code": "perso_gestion_des_roles",
                "libelle": "Gestion des rôles",
                "permissions": ["core:roles:assign", "core:users:read", "core:backup:restore"],
            }),
        )
        .await;
    assert_eq!(refus.cle, "role.permission_non_detenue");
    p.ok(
        "role_creer",
        Some(&moussa),
        json!({ "libelle": "Lecture des comptes", "permissions": ["core:users:read"] }),
    )
    .await;
}

fn id_de(fiches: &Value, identifiant: &str) -> Value {
    fiches
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["utilisateur"]["identifiant"] == identifiant)
        .unwrap()["utilisateur"]["id"]
        .clone()
}

#[tokio::test]
async fn cycle_de_vie_d_un_compte_et_garde_fous_du_dernier_administrateur() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    p.ok(
        "utilisateur_creer",
        Some(&admin),
        json!({
            "identifiant": "fanta",
            "nom_affiche": "Fanta Coulibaly",
            "mot_de_passe_initial": "secretariat-2026",
            "roles": ["secretaire"],
        }),
    )
    .await;
    let fiches = p.ok("utilisateurs_lister", Some(&admin), Value::Null).await;
    let fanta = id_de(&fiches, "fanta");
    let moi = id_de(&fiches, "admin");

    // Désactiver ferme la session ouverte, et le compte ne se connecte plus.
    let session_fanta = p.connecter("fanta", "secretariat-2026").await;
    p.ok(
        "utilisateur_modifier",
        Some(&admin),
        json!({ "id": fanta, "nom_affiche": "Fanta Coulibaly", "actif": false }),
    )
    .await;
    assert_eq!(
        p.refus("session_courante", Some(&session_fanta), Value::Null)
            .await
            .cle,
        "session.invalide"
    );
    let refus = p
        .refus(
            "connexion",
            None,
            json!({ "identifiant": "fanta", "mot_de_passe": "secretariat-2026" }),
        )
        .await;
    assert_eq!(refus.cle, "connexion.compte_inactif");
    p.ok(
        "utilisateur_modifier",
        Some(&admin),
        json!({ "id": fanta, "nom_affiche": "Fanta Coulibaly", "actif": true }),
    )
    .await;

    // Trois échecs verrouillent le compte ; l'administrateur le déverrouille.
    for _ in 0..3 {
        p.refus(
            "connexion",
            None,
            json!({ "identifiant": "fanta", "mot_de_passe": "mauvais-mot-de-passe" }),
        )
        .await;
    }
    let refus = p
        .refus(
            "connexion",
            None,
            json!({ "identifiant": "fanta", "mot_de_passe": "secretariat-2026" }),
        )
        .await;
    assert_eq!(refus.cle, "connexion.verrouille");
    p.ok(
        "utilisateur_deverrouiller",
        Some(&admin),
        json!({ "id": fanta }),
    )
    .await;

    // Réinitialisation : nouveau mot de passe initial, changement exigé, ancienne session fermée.
    let session_fanta = p.connecter("fanta", "secretariat-2026").await;
    let reinitialise = p
        .ok(
            "utilisateur_reinitialiser_mot_de_passe",
            Some(&admin),
            json!({ "id": fanta, "mot_de_passe_initial": "nouveau-depart-2026" }),
        )
        .await;
    assert_eq!(reinitialise["doit_changer_mot_de_passe"], true);
    assert_eq!(
        p.refus("session_courante", Some(&session_fanta), Value::Null)
            .await
            .cle,
        "session.invalide"
    );
    p.connecter("fanta", "nouveau-depart-2026").await;

    // Garde-fous.
    for (commande, entree, cle) in [
        (
            "utilisateur_modifier",
            json!({ "id": moi, "nom_affiche": "Administrateur", "actif": false }),
            "utilisateur.desactivation_de_soi",
        ),
        (
            "role_attribuer",
            json!({ "utilisateur_id": moi, "role": "administrateur", "attribue": false }),
            "role.modification_de_soi",
        ),
        (
            "utilisateur_reinitialiser_mot_de_passe",
            json!({ "id": moi, "mot_de_passe_initial": "autre-mot-de-passe" }),
            "utilisateur.reinitialisation_de_soi",
        ),
    ] {
        assert_eq!(
            p.refus(commande, Some(&admin), entree).await.cle,
            cle,
            "{commande}"
        );
    }

    // Un second administrateur ne peut ni désactiver ni dégrader le premier s'il est le dernier
    // actif ; une fois deux administrateurs actifs, le retrait redevient possible.
    p.ok(
        "role_attribuer",
        Some(&admin),
        json!({ "utilisateur_id": fanta, "role": "administrateur", "attribue": true }),
    )
    .await;
    let fanta_admin = p.connecter("fanta", "nouveau-depart-2026").await;
    p.ok(
        "role_attribuer",
        Some(&fanta_admin),
        json!({ "utilisateur_id": moi, "role": "administrateur", "attribue": false }),
    )
    .await;
    let refus = p
        .refus(
            "role_attribuer",
            Some(&admin),
            json!({ "utilisateur_id": fanta, "role": "administrateur", "attribue": false }),
        )
        .await;
    // L'ancien administrateur a perdu ses permissions : la garde le refuse avant le garde-fou.
    assert_eq!(refus.cle, "erreur.permission_refusee");
    p.ok(
        "role_attribuer",
        Some(&fanta_admin),
        json!({ "utilisateur_id": moi, "role": "administrateur", "attribue": true }),
    )
    .await;
    p.ok(
        "utilisateur_modifier",
        Some(&admin),
        json!({ "id": fanta, "nom_affiche": "Fanta Coulibaly", "actif": false }),
    )
    .await;
    let seul = p
        .refus(
            "role_attribuer",
            Some(&admin),
            json!({ "utilisateur_id": moi, "role": "administrateur", "attribue": false }),
        )
        .await;
    assert_eq!(seul.cle, "role.modification_de_soi");
    p.ok("audit_verifier", Some(&admin), Value::Null).await;
}

#[tokio::test]
async fn on_n_attribue_pas_un_role_plus_puissant_que_le_sien() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    p.ok(
        "role_creer",
        Some(&admin),
        json!({ "libelle": "Gestion des comptes", "permissions": ["core:roles:assign", "core:users:read", "core:users:create"] }),
    )
    .await;
    p.ok(
        "utilisateur_creer",
        Some(&admin),
        json!({
            "identifiant": "moussa",
            "nom_affiche": "Moussa Keïta",
            "mot_de_passe_initial": "comptes-2026-gestion",
            "roles": ["perso_gestion_des_comptes"],
        }),
    )
    .await;
    p.ok(
        "utilisateur_creer",
        Some(&admin),
        json!({
            "identifiant": "awa",
            "nom_affiche": "Awa Diarra",
            "mot_de_passe_initial": "accueil-2026-bureau",
            "roles": [],
        }),
    )
    .await;
    let fiches = p.ok("utilisateurs_lister", Some(&admin), Value::Null).await;
    let awa = id_de(&fiches, "awa");
    let moussa = p.connecter("moussa", "comptes-2026-gestion").await;
    let refus = p
        .refus(
            "role_attribuer",
            Some(&moussa),
            json!({ "utilisateur_id": awa, "role": "administrateur", "attribue": true }),
        )
        .await;
    assert_eq!(refus.cle, "role.permission_non_detenue");
    p.ok(
        "role_attribuer",
        Some(&moussa),
        json!({ "utilisateur_id": awa, "role": "perso_gestion_des_comptes", "attribue": true }),
    )
    .await;
}

#[tokio::test]
async fn la_politique_de_mot_de_passe_s_applique_partout() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let creer = |mot_de_passe: &str| {
        json!({
            "identifiant": "fanta",
            "nom_affiche": "Fanta Coulibaly",
            "mot_de_passe_initial": mot_de_passe,
            "roles": [],
        })
    };
    for (mot_de_passe, cle) in [
        ("court", "mot_de_passe.trop_court"),
        ("fanta-2026-ecole", "mot_de_passe.contient_identifiant"),
    ] {
        assert_eq!(
            p.refus("utilisateur_creer", Some(&admin), creer(mot_de_passe))
                .await
                .cle,
            cle
        );
    }
    for (cle_parametre, valeur, cle) in [
        ("mot_de_passe.longueur_min", 5, "parametre.hors_bornes"),
        (
            "connexion.echecs_avant_verrouillage",
            1,
            "parametre.hors_bornes",
        ),
        // SPEC_PHASE_3 §2 : arrondi au franc ou aux coupures en usage seulement.
        ("montant.multiple_arrondi", 7, "parametre.hors_bornes"),
    ] {
        let refus = p
            .refus(
                "parametre_definir",
                Some(&admin),
                json!({ "cle": cle_parametre, "valeur": { "type": "entier", "valeur": valeur } }),
            )
            .await;
        assert_eq!(refus.cle, cle, "{cle_parametre}");
    }
    p.ok(
        "parametre_definir",
        Some(&admin),
        json!({ "cle": "mot_de_passe.longueur_min", "valeur": { "type": "entier", "valeur": 12 } }),
    )
    .await;
    assert_eq!(
        p.refus("utilisateur_creer", Some(&admin), creer("onze-signes"))
            .await
            .cle,
        "mot_de_passe.trop_court"
    );
    p.ok(
        "utilisateur_creer",
        Some(&admin),
        creer("cour-du-lycee-2026"),
    )
    .await;
    let fiches = p.ok("utilisateurs_lister", Some(&admin), Value::Null).await;
    let fanta = id_de(&fiches, "fanta");
    assert_eq!(
        p.refus(
            "utilisateur_reinitialiser_mot_de_passe",
            Some(&admin),
            json!({ "id": fanta, "mot_de_passe_initial": "FANTA-nouvelle-annee" }),
        )
        .await
        .cle,
        "mot_de_passe.contient_identifiant"
    );
    assert_eq!(
        p.refus(
            "changer_mot_de_passe",
            Some(&admin),
            json!({ "ancien": MOT_DE_PASSE, "nouveau": "admin-de-l-ecole-2026" }),
        )
        .await
        .cle,
        "mot_de_passe.contient_identifiant"
    );

    // Par défaut, six chiffres simples suffisent ; l'école peut refuser les plus courants.
    p.ok(
        "parametre_definir",
        Some(&admin),
        json!({ "cle": "mot_de_passe.longueur_min", "valeur": { "type": "entier", "valeur": 6 } }),
    )
    .await;
    let simple = |identifiant: &str, mot_de_passe: &str| {
        json!({ "identifiant": identifiant, "nom_affiche": identifiant,
            "mot_de_passe_initial": mot_de_passe, "roles": [] })
    };
    p.ok("utilisateur_creer", Some(&admin), simple("kadi", "000000"))
        .await;
    p.ok(
        "parametre_definir",
        Some(&admin),
        json!({ "cle": "mot_de_passe.refuser_courants", "valeur": { "type": "booleen", "valeur": true } }),
    )
    .await;
    assert_eq!(
        p.refus(
            "utilisateur_creer",
            Some(&admin),
            simple("mariam", "Bamako2026")
        )
        .await
        .cle,
        "mot_de_passe.trop_courant"
    );
}

#[tokio::test]
async fn les_durees_de_session_de_l_ecole_s_appliquent() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    // Quinze minutes par défaut : à dix, la session est encore ouverte.
    p.horloge.avancer(10 * 60_000);
    p.ok("session_courante", Some(&admin), Value::Null).await;
    // L'école règle une minute : au-delà, la session se verrouille.
    p.ok(
        "parametre_definir",
        Some(&admin),
        json!({ "cle": "session.verrouillage_inactivite_s", "valeur": { "type": "entier", "valeur": 60 } }),
    )
    .await;
    assert_eq!(
        p.refus(
            "parametre_definir",
            Some(&admin),
            json!({ "cle": "session.verrouillage_inactivite_s", "valeur": { "type": "entier", "valeur": 5 } }),
        )
        .await
        .cle,
        "parametre.hors_bornes"
    );
    p.horloge.avancer(61_000);
    assert_eq!(
        p.refus("session_courante", Some(&admin), Value::Null)
            .await
            .cle,
        "session.verrouillee"
    );
}

#[tokio::test]
async fn ecran_verrouille_pin_epuise_et_deverrouillage_par_mot_de_passe() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let courante = p.ok("session_courante", Some(&admin), Value::Null).await;
    assert_eq!(courante["pin_defini"], false);

    // Sans PIN, la session verrouillée se rouvre par le mot de passe ; un mot de passe faux
    // est refusé.
    p.ok("verrouiller", Some(&admin), Value::Null).await;
    let sessions = p.ok("sessions_du_poste", None, Value::Null).await;
    assert_eq!(sessions[0]["pin_defini"], false);
    let faux = p
        .refus(
            "deverrouiller_mot_de_passe",
            None,
            json!({ "jeton": admin, "mot_de_passe": "pas-le-bon-mot-de-passe" }),
        )
        .await;
    assert_eq!(faux.cle, "connexion.identifiants_invalides");
    p.ok(
        "deverrouiller_mot_de_passe",
        None,
        json!({ "jeton": admin, "mot_de_passe": MOT_DE_PASSE }),
    )
    .await;

    // Avec un PIN : la session le sait aussitôt ; trois PIN faux ferment la session.
    p.ok(
        "definir_pin",
        Some(&admin),
        json!({ "mot_de_passe": MOT_DE_PASSE, "pin": "2468" }),
    )
    .await;
    let courante = p.ok("session_courante", Some(&admin), Value::Null).await;
    assert_eq!(courante["pin_defini"], true);
    p.ok("verrouiller", Some(&admin), Value::Null).await;
    for attendu in [
        "session.pin_invalide",
        "session.pin_invalide",
        "session.pin_epuise",
    ] {
        let refus = p
            .refus(
                "deverrouiller_pin",
                None,
                json!({ "jeton": admin, "pin": "0000" }),
            )
            .await;
        assert_eq!(refus.cle, attendu);
    }
    let refus = p
        .refus(
            "deverrouiller_pin",
            None,
            json!({ "jeton": admin, "pin": "2468" }),
        )
        .await;
    assert_eq!(
        refus.cle, "session.invalide",
        "la session épuisée est fermée"
    );
    let nouvelle = p.connecter("admin", MOT_DE_PASSE).await;
    p.ok("verrouiller", Some(&nouvelle), Value::Null).await;
    p.ok(
        "deverrouiller_pin",
        None,
        json!({ "jeton": nouvelle, "pin": "2468" }),
    )
    .await;
}

/// PNG valide de 4 × 4 pixels bleus.

#[tokio::test]
async fn la_fiche_et_le_logo_de_l_etablissement_passent_dans_les_documents() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let fiche = p.ok("etablissement_lire", Some(&admin), Value::Null).await;
    assert_eq!(fiche["logo"], Value::Null);

    let modifiee = p
        .ok(
            "etablissement_modifier",
            Some(&admin),
            json!({
                "nom": "  Lycée Askia Mohamed de Bamako ",
                "statut": "prive",
                "code_academie": "AE-BKO-RD",
                "code_cap": "CAP-02",
                "adresse": "Hamdallaye ACI 2000",
                "telephones": ["+223 20 22 00 00", " "],
                "mentions_legales": "Autorisation n° 2026-0042",
            }),
        )
        .await;
    assert_eq!(
        modifiee["etablissement"]["nom"],
        "Lycée Askia Mohamed de Bamako"
    );
    assert_eq!(
        modifiee["etablissement"]["telephones"],
        json!(["+223 20 22 00 00"])
    );
    let etat = p.ok("etat_demarrage", None, Value::Null).await;
    assert_eq!(
        etat["etablissement"]["nom"],
        "Lycée Askia Mohamed de Bamako"
    );

    for (contenu, cle) in [
        ("R0lGODlhAQABAAAAACw=", "etablissement.logo_format"),
        ("pas du base64 !", "etablissement.logo_illisible"),
    ] {
        let refus = p
            .refus(
                "etablissement_logo_definir",
                Some(&admin),
                json!({ "contenu_base64": contenu }),
            )
            .await;
        assert_eq!(refus.cle, cle);
    }
    let avec_logo = p
        .ok(
            "etablissement_logo_definir",
            Some(&admin),
            json!({ "contenu_base64": PNG_ESSAI }),
        )
        .await;
    assert!(
        avec_logo["logo"]
            .as_str()
            .unwrap()
            .starts_with("data:image/png;base64,")
    );

    // Le document suivant se compose avec le logo et la nouvelle fiche.
    let document = p
        .ok("document_essai_produire", Some(&admin), Value::Null)
        .await;
    let pdf = base64::engine::general_purpose::STANDARD
        .decode(document["pdf"].as_str().unwrap())
        .unwrap();
    assert!(pdf.starts_with(b"%PDF-"));

    let sans_logo = p
        .ok(
            "etablissement_logo_definir",
            Some(&admin),
            json!({ "contenu_base64": null }),
        )
        .await;
    assert_eq!(sans_logo["logo"], Value::Null);
    let refus = p
        .refus(
            "etablissement_modifier",
            Some(&admin),
            json!({
                "nom": " ", "statut": "prive", "code_academie": "", "code_cap": "",
                "adresse": "", "telephones": [], "mentions_legales": "",
            }),
        )
        .await;
    assert_eq!(refus.cle, "etablissement.nom_obligatoire");
    p.ok("audit_verifier", Some(&admin), Value::Null).await;
}

#[tokio::test]
async fn une_annee_scolaire_se_prepare_puis_s_ouvre() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let alertes = avertissements(&p.ok("avertissements", Some(&admin), Value::Null).await);
    assert!(alertes.contains("avertissement.annee_absente"));

    for (entree, cle) in [
        (
            json!({ "libelle": "2026-2028", "debut": RENTREE_2026, "fin": FIN_2027 }),
            "annee.libelle_invalide",
        ),
        (
            json!({ "libelle": "2026-2027", "debut": FIN_2027, "fin": RENTREE_2026 }),
            "annee.bornes_invalides",
        ),
        (
            json!({ "libelle": "2026-2027", "debut": RENTREE_2026, "fin": FIN_2028 }),
            "annee.bornes_invalides",
        ),
    ] {
        assert_eq!(p.refus("annee_creer", Some(&admin), entree).await.cle, cle);
    }
    let annee = p
        .ok(
            "annee_creer",
            Some(&admin),
            json!({ "libelle": "2026-2027", "debut": RENTREE_2026, "fin": FIN_2027 }),
        )
        .await;
    assert_eq!(annee["statut"], "preparation");
    let chevauche = p
        .refus(
            "annee_creer",
            Some(&admin),
            json!({ "libelle": "2027-2028", "debut": 1_811_808_000_000_i64, "fin": FIN_2028 }),
        )
        .await;
    assert_eq!(chevauche.cle, "annee.chevauchement");
    let corrigee = p
        .ok(
            "annee_modifier",
            Some(&admin),
            json!({ "id": annee["id"], "libelle": "2026-2027", "debut": RENTREE_2026, "fin": 1_811_808_000_000_i64 }),
        )
        .await;
    assert_eq!(corrigee["fin"], 1_811_808_000_000_i64);

    let sauvegardes_avant = p
        .ok("sauvegardes_lister", Some(&admin), Value::Null)
        .await
        .as_array()
        .unwrap()
        .len();
    let ouverte = p
        .ok("annee_activer", Some(&admin), json!({ "id": annee["id"] }))
        .await;
    assert_eq!(ouverte["statut"], "active");
    assert_eq!(ouverte["est_courante"], true);
    let sauvegardes = p.ok("sauvegardes_lister", Some(&admin), Value::Null).await;
    assert_eq!(sauvegardes.as_array().unwrap().len(), sauvegardes_avant + 1);
    let courante = p.ok("session_courante", Some(&admin), Value::Null).await;
    assert_eq!(courante["annee_courante"], "2026-2027");
    let alertes = avertissements(&p.ok("avertissements", Some(&admin), Value::Null).await);
    assert!(!alertes.contains("avertissement.annee_absente"));

    // La rentrée suivante se prépare, mais ne s'ouvre qu'après la clôture (phase 10).
    let suivante = p
        .ok(
            "annee_creer",
            Some(&admin),
            json!({ "libelle": "2027-2028", "debut": RENTREE_2027, "fin": FIN_2028 }),
        )
        .await;
    for (commande, entree, cle) in [
        (
            "annee_activer",
            json!({ "id": suivante["id"] }),
            "annee.deja_active",
        ),
        (
            "annee_activer",
            json!({ "id": annee["id"] }),
            "erreur.transition_interdite",
        ),
        (
            "annee_modifier",
            json!({ "id": annee["id"], "libelle": "2026-2027", "debut": RENTREE_2026, "fin": FIN_2027 }),
            "annee.non_modifiable",
        ),
        (
            "annee_supprimer",
            json!({ "id": annee["id"] }),
            "annee.non_supprimable",
        ),
    ] {
        assert_eq!(
            p.refus(commande, Some(&admin), entree).await.cle,
            cle,
            "{commande}"
        );
    }
    p.ok(
        "annee_supprimer",
        Some(&admin),
        json!({ "id": suivante["id"] }),
    )
    .await;
    let annees = p.ok("annees_lister", Some(&admin), Value::Null).await;
    assert_eq!(annees.as_array().unwrap().len(), 1);
    p.ok("audit_verifier", Some(&admin), Value::Null).await;
}

#[tokio::test]
async fn le_journal_se_filtre_et_nomme_ses_auteurs() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    p.ok(
        "role_creer",
        Some(&admin),
        json!({ "libelle": "Accueil", "permissions": ["students:read"] }),
    )
    .await;
    p.ok(
        "utilisateur_creer",
        Some(&admin),
        json!({
            "identifiant": "fanta",
            "nom_affiche": "Fanta Coulibaly",
            "mot_de_passe_initial": "secretariat-2026",
            "roles": ["secretaire"],
        }),
    )
    .await;

    let tout = p
        .ok("audit_lister", Some(&admin), json!({ "page": 0 }))
        .await;
    assert!(
        tout["entites"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e == "role")
    );
    assert!(
        tout["auteurs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["nom"] == "Fanta Coulibaly")
    );
    assert_eq!(tout["suivante"], false);

    let roles = p
        .ok(
            "audit_lister",
            Some(&admin),
            json!({ "page": 0, "entite": "role" }),
        )
        .await;
    let lignes = roles["lignes"].as_array().unwrap();
    assert!(!lignes.is_empty());
    assert!(lignes.iter().all(|l| l["entite"] == "role"));

    let creations = p
        .ok(
            "audit_lister",
            Some(&admin),
            json!({ "page": 0, "operation": "creation", "entite": "utilisateur" }),
        )
        .await;
    assert!(
        creations["lignes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|l| l["operation"] == "creation")
    );

    let courante = p.ok("session_courante", Some(&admin), Value::Null).await;
    let miennes = p
        .ok(
            "audit_lister",
            Some(&admin),
            json!({ "page": 0, "utilisateur_id": courante["session"]["utilisateur_id"] }),
        )
        .await;
    assert!(
        miennes["lignes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|l| l["utilisateur_id"] == courante["session"]["utilisateur_id"])
    );

    let futur = p
        .ok(
            "audit_lister",
            Some(&admin),
            json!({ "page": 0, "depuis": 4_102_444_800_000_i64 }),
        )
        .await;
    assert!(futur["lignes"].as_array().unwrap().is_empty());
}
