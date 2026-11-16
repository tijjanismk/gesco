//! Copie hors de l'école par un dossier synchronisé : ce qui y part, quand, ce qui n'y part
//! jamais, un dossier absent qui ne bloque rien, un nettoyage qui ne touche qu'à ses propres
//! copies, et les réglages de la 1.2 qui ne changent pas. Le dossier synchronisé est ici un
//! simple dossier : GESCO ne voit rien d'autre du logiciel du fournisseur.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use gesco_services::session::Jeton;
use serde_json::{Value, json};

use crate::commun::{MOT_DE_PASSE, Poste_, poste_vierge};

const HEURE: i64 = 3_600_000;
const JOUR: i64 = 24 * HEURE;

/// Fichiers d'un dossier, par nom ; vide si le dossier n'existe pas.
fn fichiers(dossier: &Path) -> Vec<String> {
    let mut noms: Vec<String> = std::fs::read_dir(dossier)
        .map(|contenu| {
            contenu
                .flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    noms.sort();
    noms
}

fn archives(dossier: &Path) -> Vec<String> {
    fichiers(dossier)
        .into_iter()
        .filter(|n| n.ends_with(".gsv"))
        .collect()
}

/// Un poste installé, son administrateur, un dossier synchronisé (le sous-dossier `GESCO` d'un
/// faux OneDrive) et une clé USB, réglés comme destinations.
async fn poste_avec_dossier(racine: &Path) -> (Poste_, Jeton, String, PathBuf, PathBuf) {
    let p = poste_vierge(&racine.join("poste")).await;
    let phrase = p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let onedrive = racine.join("OneDrive");
    std::fs::create_dir_all(&onedrive).unwrap();
    let synchronise = onedrive.join("GESCO");
    let usb = racine.join("usb");
    p.ok(
        "sauvegarde_destinations_definir",
        Some(&admin),
        json!([
            { "chemin": usb, "sorte": "amovible", "amovible": true },
            { "chemin": synchronise, "sorte": "synchronise", "amovible": false },
        ]),
    )
    .await;
    (p, admin, phrase, synchronise, usb)
}

fn ligne_vers<'a>(lignes: &'a Value, dossier: &Path) -> &'a Value {
    lignes
        .as_array()
        .unwrap()
        .iter()
        .find(|l| Path::new(l["destination"].as_str().unwrap()).starts_with(dossier))
        .unwrap_or_else(|| panic!("aucune ligne vers {}", dossier.display()))
}

#[tokio::test]
async fn la_copie_a_la_demande_part_dans_le_dossier_avec_la_meme_empreinte() {
    let dossier = tempfile::tempdir().unwrap();
    let (p, admin, _, synchronise, usb) = poste_avec_dossier(dossier.path()).await;
    // Le sous-dossier GESCO n'existe pas encore : il se crée dans le dossier du fournisseur.
    assert!(!synchronise.exists());
    p.horloge.avancer(60_000);
    let lignes = p
        .ok("sauvegarde_hors_ecole_envoyer", Some(&admin), Value::Null)
        .await;
    assert_eq!(lignes.as_array().unwrap().len(), 3);
    let locale = &lignes[0];
    let copie = ligne_vers(&lignes, &synchronise);
    assert_eq!(copie["resultat"], "reussie");
    assert_eq!(copie["sorte"], "synchronise");
    assert_eq!(copie["etat_envoi"], "deposee");
    assert_eq!(copie["empreinte_sha256"], locale["empreinte_sha256"]);
    assert_eq!(locale["sorte"], "dossier");
    assert_eq!(locale["etat_envoi"], Value::Null);
    assert_eq!(ligne_vers(&lignes, &usb)["sorte"], "amovible");
    assert_eq!(archives(&synchronise).len(), 1);
    assert_eq!(fichiers(&synchronise), archives(&synchronise));
}

/// Critère 3 : les autres sauvegardes vont partout ailleurs, jamais dans le dossier synchronisé.
#[tokio::test]
async fn les_autres_sauvegardes_ne_vont_pas_hors_de_l_ecole() {
    let dossier = tempfile::tempdir().unwrap();
    let (p, admin, _, synchronise, usb) = poste_avec_dossier(dossier.path()).await;
    // « Sauvegarder maintenant » garde son sens d'avant.
    p.horloge.avancer(60_000);
    let lignes = p.ok("sauvegarde_creer", Some(&admin), Value::Null).await;
    assert_eq!(lignes.as_array().unwrap().len(), 2);
    assert_eq!(ligne_vers(&lignes, &usb)["resultat"], "reussie");
    // Sauvegarde à l'intervalle réglé, au rythme « à la demande » : rien ne part.
    for _ in 0..3 {
        p.horloge.avancer(25 * HEURE);
        p.application.taches_periodiques().await;
    }
    // Sauvegarde de fermeture.
    p.horloge.avancer(25 * HEURE);
    p.application.fermer().await.unwrap();
    assert!(archives(&synchronise).is_empty());
    assert!(!synchronise.exists());
    assert!(archives(&usb).len() >= 4, "{:?}", archives(&usb));
}

/// Critère 2 : trente jours au rythme « chaque semaine » donnent quatre ou cinq copies, et
/// seulement des fichiers `.gsv` ; au rythme « à la demande », aucune.
#[tokio::test]
async fn trente_jours_chaque_semaine_donnent_quatre_ou_cinq_copies() {
    let dossier = tempfile::tempdir().unwrap();
    let (p, admin, _, synchronise, _) = poste_avec_dossier(dossier.path()).await;
    p.horloge.avancer(60_000);
    p.application.taches_periodiques().await;
    assert!(archives(&synchronise).is_empty());

    let rythme = p
        .ok(
            "sauvegarde_hors_ecole_regler",
            Some(&admin),
            json!({ "rythme": "semaine" }),
        )
        .await;
    assert_eq!(rythme, "semaine");
    // Trente jours, un passage de la tâche toutes les six heures.
    for _ in 0..(30 * 4) {
        p.horloge.avancer(6 * HEURE);
        p.application.taches_periodiques().await;
    }
    let copies = archives(&synchronise);
    assert!((4..=5).contains(&copies.len()), "{copies:?}");
    assert_eq!(fichiers(&synchronise), copies);
}

/// Une copie à la demande compte comme une copie du rythme : elle repousse la suivante.
#[tokio::test]
async fn la_copie_a_la_demande_repousse_celle_du_rythme() {
    let dossier = tempfile::tempdir().unwrap();
    let (p, admin, _, synchronise, _) = poste_avec_dossier(dossier.path()).await;
    p.ok(
        "sauvegarde_hors_ecole_regler",
        Some(&admin),
        json!({ "rythme": "semaine" }),
    )
    .await;
    p.horloge.avancer(60_000);
    p.ok("sauvegarde_hors_ecole_envoyer", Some(&admin), Value::Null)
        .await;
    for _ in 0..6 {
        p.horloge.avancer(JOUR);
        p.application.taches_periodiques().await;
    }
    assert_eq!(archives(&synchronise).len(), 1);
    p.horloge.avancer(JOUR);
    p.application.taches_periodiques().await;
    assert_eq!(archives(&synchronise).len(), 2);
}

/// Critère 4 : ni l'en-tête d'une base SQLite, ni la clé de l'école, ni les douze mots dans le
/// dossier hors de l'école.
#[tokio::test]
async fn ni_la_base_ni_la_cle_ni_les_douze_mots_ne_partent() {
    let dossier = tempfile::tempdir().unwrap();
    let (p, admin, phrase, synchronise, _) = poste_avec_dossier(dossier.path()).await;
    p.horloge.avancer(60_000);
    p.ok("sauvegarde_hors_ecole_envoyer", Some(&admin), Value::Null)
        .await;
    let espace = gesco_services::espace::EspaceDonnees::nouveau(dossier.path().join("poste"));
    let cles = gesco_services::cles::EspaceCles::nouveau(espace.cles(), Arc::clone(&p.trousseau));
    let gesco_services::cles::EtatCles::Ouvertes(cle) = cles.preparer().unwrap() else {
        panic!("clés de données verrouillées");
    };
    let cle_hex = cle.en_hexadecimal().as_str().to_owned();
    let cle_octets = hex::decode(&cle_hex).unwrap();
    let interdits: Vec<Vec<u8>> = vec![
        b"SQLite format 3\0".to_vec(),
        cle_octets,
        cle_hex.as_bytes().to_vec(),
        cle_hex.to_uppercase().into_bytes(),
        phrase.as_bytes().to_vec(),
    ];
    let noms = fichiers(&synchronise);
    assert_eq!(noms, archives(&synchronise));
    assert!(!noms.is_empty());
    for nom in noms {
        let contenu = std::fs::read(synchronise.join(&nom)).unwrap();
        for interdit in &interdits {
            assert!(
                !contenu
                    .windows(interdit.len())
                    .any(|w| w == interdit.as_slice()),
                "{nom} contient un motif interdit"
            );
        }
    }
}

/// Critère 5 : dossier absent (lecteur disparu, logiciel désinstallé) : la sauvegarde du poste et
/// celle de la clé réussissent, rien ne s'ouvre, l'historique dit « n'a pas pu être déposée ».
#[tokio::test]
async fn un_dossier_absent_ne_bloque_rien() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(&dossier.path().join("poste")).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let usb = dossier.path().join("usb");
    // Le lecteur de Google Drive a disparu : ni lui ni son dossier n'existent.
    let disparu = dossier
        .path()
        .join("lecteur-G")
        .join("Mon Drive")
        .join("GESCO");
    p.ok(
        "sauvegarde_destinations_definir",
        Some(&admin),
        json!([
            { "chemin": usb, "sorte": "amovible", "amovible": true },
            { "chemin": disparu, "sorte": "synchronise", "amovible": false },
        ]),
    )
    .await;
    p.ok(
        "sauvegarde_hors_ecole_regler",
        Some(&admin),
        json!({ "rythme": "semaine" }),
    )
    .await;
    p.horloge.avancer(60_000);
    let lignes = p
        .ok("sauvegarde_hors_ecole_envoyer", Some(&admin), Value::Null)
        .await;
    assert_eq!(lignes[0]["resultat"], "reussie");
    assert_eq!(ligne_vers(&lignes, &usb)["resultat"], "reussie");
    let copie = ligne_vers(&lignes, &disparu);
    assert_eq!(copie["resultat"], "echouee");
    assert_eq!(copie["etat_envoi"], "non_deposee");
    // GESCO ne recrée pas le dossier d'un fournisseur disparu.
    assert!(!dossier.path().join("lecteur-G").exists());

    // La tâche périodique ne retente pas chaque minute ; la fermeture ne l'attend pas.
    let avant = p
        .ok("sauvegardes_lister", Some(&admin), Value::Null)
        .await
        .as_array()
        .unwrap()
        .len();
    for _ in 0..5 {
        p.horloge.avancer(60_000);
        p.application.taches_periodiques().await;
    }
    let apres = p
        .ok("sauvegardes_lister", Some(&admin), Value::Null)
        .await
        .as_array()
        .unwrap()
        .len();
    assert_eq!(apres, avant);
    p.horloge.avancer(60_000);
    p.application.fermer().await.unwrap();
}

/// Disque plein ou copie interrompue : rien d'autre que des archives entières dans le dossier.
#[tokio::test]
async fn une_copie_refusee_ne_laisse_rien_dans_le_dossier() {
    let dossier = tempfile::tempdir().unwrap();
    let (p, admin, _, synchronise, _) = poste_avec_dossier(dossier.path()).await;
    // Un fichier du même nom que la copie occupe la place du sous-dossier : le dépôt échoue.
    std::fs::write(&synchronise, b"pas un dossier").unwrap();
    p.horloge.avancer(60_000);
    let lignes = p
        .ok("sauvegarde_hors_ecole_envoyer", Some(&admin), Value::Null)
        .await;
    assert_eq!(lignes[0]["resultat"], "reussie");
    let copie = ligne_vers(&lignes, &synchronise);
    assert_eq!(copie["etat_envoi"], "non_deposee");
    assert_eq!(
        std::fs::read(&synchronise).unwrap(),
        b"pas un dossier".to_vec()
    );
}

/// Critère 9 : le nettoyage garde 4 semaines et 12 mois, ne supprime que les copies que GESCO a
/// déposées pour cette école, jamais un autre fichier ni la copie d'une autre école.
#[tokio::test]
async fn le_nettoyage_ne_touche_qu_a_ses_propres_copies() {
    let dossier = tempfile::tempdir().unwrap();
    let (p, admin, _, synchronise, _) = poste_avec_dossier(dossier.path()).await;

    // Une autre école dépose dans le même dossier.
    let autre = poste_vierge(&dossier.path().join("autre-ecole")).await;
    autre.installer().await;
    let admin_autre = autre.connecter("admin", MOT_DE_PASSE).await;
    autre
        .ok(
            "sauvegarde_destinations_definir",
            Some(&admin_autre),
            json!([{ "chemin": synchronise, "sorte": "synchronise", "amovible": false }]),
        )
        .await;
    autre.horloge.avancer(60_000);
    autre
        .ok(
            "sauvegarde_hors_ecole_envoyer",
            Some(&admin_autre),
            Value::Null,
        )
        .await;
    let de_l_autre = archives(&synchronise);
    assert_eq!(de_l_autre.len(), 1);
    // Un fichier rangé là par quelqu'un, et une archive d'un nom semblable, inconnue de GESCO.
    std::fs::write(synchronise.join("notes.txt"), b"a garder").unwrap();
    std::fs::write(
        synchronise.join("gesco-20200101-000000-AAAAAA.gsv"),
        b"inconnue",
    )
    .unwrap();

    // Une copie par semaine pendant trente semaines.
    let mut derniere = String::new();
    for _ in 0..30 {
        p.horloge.avancer(7 * JOUR);
        let lignes = p
            .ok("sauvegarde_hors_ecole_envoyer", Some(&admin), Value::Null)
            .await;
        derniere = ligne_vers(&lignes, &synchronise)["destination"]
            .as_str()
            .unwrap()
            .to_owned();
    }
    let restantes = fichiers(&synchronise);
    assert!(restantes.contains(&"notes.txt".to_owned()));
    assert!(restantes.contains(&"gesco-20200101-000000-AAAAAA.gsv".to_owned()));
    assert!(restantes.contains(&de_l_autre[0]));
    assert!(Path::new(&derniere).is_file());
    // Les nôtres : 4 semaines et un par mois, 16 au plus.
    let notres = restantes.len() - 3;
    assert!((5..=16).contains(&notres), "{restantes:?}");

    // Les copies retirées disent qu'elles ne sont plus dans le dossier ; les autres y sont.
    let historique = p.ok("sauvegardes_lister", Some(&admin), Value::Null).await;
    for l in historique.as_array().unwrap() {
        if l["sorte"] != "synchronise" {
            continue;
        }
        let present = Path::new(l["destination"].as_str().unwrap()).is_file();
        assert_eq!(l["etat_envoi"] == "absente", !present, "{l}");
    }
}

/// Critère 11 : un dossier ajouté en 1.2 comme destination ordinaire garde son comportement ; il
/// reçoit les mêmes sauvegardes qu'avant, et GESCO n'en fait pas un dossier synchronisé.
#[tokio::test]
async fn un_reglage_de_la_1_2_ne_change_pas() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(&dossier.path().join("poste")).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let drive = dossier.path().join("Google Drive").join("GESCO");
    let usb = dossier.path().join("usb");
    // Les réglages de la 1.2 n'ont que la case « amovible ».
    let destinations = p
        .ok(
            "sauvegarde_destinations_definir",
            Some(&admin),
            json!([
                { "chemin": drive, "amovible": false },
                { "chemin": usb, "amovible": true },
            ]),
        )
        .await;
    assert_eq!(destinations[1]["sorte"], "dossier");
    assert_eq!(destinations[2]["sorte"], "amovible");
    assert_eq!(destinations[2]["amovible"], true);
    let relues = p
        .ok("sauvegarde_destinations_lister", Some(&admin), Value::Null)
        .await;
    assert_eq!(relues, destinations);

    p.horloge.avancer(60_000);
    let lignes = p.ok("sauvegarde_creer", Some(&admin), Value::Null).await;
    assert_eq!(ligne_vers(&lignes, &drive)["resultat"], "reussie");
    assert_eq!(ligne_vers(&lignes, &drive)["sorte"], "dossier");
    assert_eq!(ligne_vers(&lignes, &drive)["etat_envoi"], Value::Null);
    p.horloge.avancer(25 * HEURE);
    p.application.taches_periodiques().await;
    assert_eq!(archives(&drive).len(), 2);
}
