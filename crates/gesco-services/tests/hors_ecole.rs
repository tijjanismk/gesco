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

use gesco_services::etat_envoi::EtatSignale;
use gesco_services::session::Jeton;
use serde_json::{Value, json};

use crate::commun::{MOT_DE_PASSE, Poste_, avertissements, poste_vierge};

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
    let (p, _, _, synchronise, _) = poste_avec_dossier(dossier.path()).await;

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
        let lignes = envoyer(&p).await;
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
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
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

const ALERTE: &str = "avertissement.copie_hors_ecole_non_arrivee";

/// État de la dernière copie du seul dossier synchronisé, lu par la commande de l'écran. Une
/// session neuve : l'horloge a pu avancer de plusieurs jours depuis la connexion.
async fn etat_derniere(p: &Poste_) -> Value {
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let etat = p
        .ok("sauvegarde_hors_ecole_etat", Some(&admin), Value::Null)
        .await;
    etat["dossiers"][0]["derniere"]["etat_envoi"].clone()
}

async fn alerte(p: &Poste_) -> bool {
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    avertissements(&p.ok("avertissements", Some(&admin), Value::Null).await).contains(ALERTE)
}

/// Copie à la demande, d'une session neuve.
async fn envoyer(p: &Poste_) -> Value {
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    p.ok("sauvegarde_hors_ecole_envoyer", Some(&admin), Value::Null)
        .await
}

/// Critère 6 : une copie effacée du dossier après son dépôt se dit « n'est plus dans le
/// dossier » à la vérification suivante, et le bandeau le dit.
#[tokio::test]
async fn une_copie_effacee_n_est_plus_dans_le_dossier() {
    let dossier = tempfile::tempdir().unwrap();
    let (p, admin, _, synchronise, _) = poste_avec_dossier(dossier.path()).await;
    p.application.taches_periodiques().await;
    p.horloge.avancer(60_000);
    p.ok("sauvegarde_hors_ecole_envoyer", Some(&admin), Value::Null)
        .await;
    assert_eq!(etat_derniere(&p).await, "deposee");
    assert!(!alerte(&p).await);
    for nom in archives(&synchronise) {
        std::fs::remove_file(synchronise.join(nom)).unwrap();
    }
    p.horloge.avancer(5 * 60_000);
    p.application.taches_periodiques().await;
    assert_eq!(etat_derniere(&p).await, "absente");
    assert!(alerte(&p).await);
    // La copie suivante, bien déposée, efface la ligne du bandeau.
    p.horloge.avancer(60_000);
    p.ok("sauvegarde_hors_ecole_envoyer", Some(&admin), Value::Null)
        .await;
    assert!(!alerte(&p).await);
}

/// Critère 6, seconde partie : la vérification ne lit que les métadonnées. Une copie dont le
/// contenu change sans changer de taille reste « déposée » : la vérification ne l'a pas lue,
/// elle n'aurait donc rien fait télécharger.
#[tokio::test]
async fn la_verification_ne_lit_jamais_le_contenu() {
    let dossier = tempfile::tempdir().unwrap();
    let (p, admin, _, synchronise, _) = poste_avec_dossier(dossier.path()).await;
    p.application.taches_periodiques().await;
    p.horloge.avancer(60_000);
    p.ok("sauvegarde_hors_ecole_envoyer", Some(&admin), Value::Null)
        .await;
    let nom = archives(&synchronise).remove(0);
    let taille = std::fs::metadata(synchronise.join(&nom)).unwrap().len();
    std::fs::write(
        synchronise.join(&nom),
        vec![0_u8; usize::try_from(taille).unwrap()],
    )
    .unwrap();
    p.horloge.avancer(5 * 60_000);
    p.application.taches_periodiques().await;
    assert_eq!(etat_derniere(&p).await, "deposee");
}

/// Critère 7, partie simulée : « envoyée » seulement quand Windows le dit ; sinon la copie
/// reste « déposée dans le dossier ». Une fois envoyée, elle ne se vérifie plus.
#[tokio::test]
async fn envoyee_seulement_quand_windows_le_dit() {
    let dossier = tempfile::tempdir().unwrap();
    let (p, admin, _, _, _) = poste_avec_dossier(dossier.path()).await;
    let windows = p.application.lecteur_etat_simule();
    p.application.taches_periodiques().await;
    p.horloge.avancer(60_000);
    p.ok("sauvegarde_hors_ecole_envoyer", Some(&admin), Value::Null)
        .await;
    // Un fournisseur dont Windows ne connaît pas l'état : jamais « envoyée ».
    p.horloge.avancer(5 * 60_000);
    p.application.taches_periodiques().await;
    assert_eq!(etat_derniere(&p).await, "deposee");
    let lectures = windows.lectures();
    assert!(lectures >= 1);
    // Rien entre deux jalons du calendrier.
    p.horloge.avancer(10 * 60_000);
    p.application.taches_periodiques().await;
    assert_eq!(windows.lectures(), lectures);
    // Une heure après le dépôt, Windows dit la copie à jour sur le compte.
    windows.signaler_partout(Some(EtatSignale::AJour));
    p.horloge.avancer(50 * 60_000);
    p.application.taches_periodiques().await;
    assert_eq!(etat_derniere(&p).await, "envoyee");
    let lectures = windows.lectures();
    for _ in 0..3 {
        p.horloge.avancer(25 * 3_600_000);
        p.application.taches_periodiques().await;
    }
    assert_eq!(windows.lectures(), lectures);
    assert!(!alerte(&p).await);
}

/// T-130 : en attente plus de sept jours, ou en échec d'envoi, la copie fait parler le bandeau ;
/// en attente depuis moins longtemps, non.
#[tokio::test]
async fn une_copie_qui_attend_trop_ou_echoue_fait_parler_le_bandeau() {
    let dossier = tempfile::tempdir().unwrap();
    let (p, admin, _, _, _) = poste_avec_dossier(dossier.path()).await;
    let windows = p.application.lecteur_etat_simule();
    windows.signaler_partout(Some(EtatSignale::EnAttente));
    p.application.taches_periodiques().await;
    p.horloge.avancer(60_000);
    p.ok("sauvegarde_hors_ecole_envoyer", Some(&admin), Value::Null)
        .await;
    p.horloge.avancer(5 * 60_000);
    p.application.taches_periodiques().await;
    assert_eq!(etat_derniere(&p).await, "en_attente");
    assert!(!alerte(&p).await);
    for _ in 0..8 {
        p.horloge.avancer(24 * 3_600_000);
        p.application.taches_periodiques().await;
    }
    assert_eq!(etat_derniere(&p).await, "en_attente");
    assert!(alerte(&p).await);

    windows.signaler_partout(Some(EtatSignale::EnEchec));
    p.horloge.avancer(60_000);
    envoyer(&p).await;
    assert!(!alerte(&p).await);
    p.horloge.avancer(5 * 60_000);
    p.application.taches_periodiques().await;
    assert_eq!(etat_derniere(&p).await, "echec_envoi");
    assert!(alerte(&p).await);
}

/// T-130 : rien pour une école sans dossier synchronisé, ni tant qu'aucune copie n'est partie ;
/// l'état dit le rythme, la taille d'une copie et la prochaine copie due.
#[tokio::test]
async fn sans_dossier_ni_copie_rien_ne_se_fait_remarquer() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(&dossier.path().join("poste")).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    p.horloge.avancer(60_000);
    p.ok("sauvegarde_creer", Some(&admin), Value::Null).await;
    let etat = p
        .ok("sauvegarde_hors_ecole_etat", Some(&admin), Value::Null)
        .await;
    assert_eq!(etat["rythme"], "demande");
    assert_eq!(etat["dossiers"], json!([]));
    assert_eq!(etat["prochaine_copie"], Value::Null);
    assert!(etat["taille_copie"].as_i64().unwrap() > 0);
    assert!(!alerte(&p).await);

    let synchronise = dossier.path().join("Dropbox").join("GESCO");
    std::fs::create_dir_all(dossier.path().join("Dropbox")).unwrap();
    p.ok(
        "sauvegarde_destinations_definir",
        Some(&admin),
        json!([{ "chemin": synchronise, "sorte": "synchronise", "amovible": false }]),
    )
    .await;
    let etat = p
        .ok("sauvegarde_hors_ecole_etat", Some(&admin), Value::Null)
        .await;
    assert_eq!(etat["dossiers"][0]["derniere"], Value::Null);
    assert_eq!(etat["prochaine_copie"], Value::Null);
    assert!(!alerte(&p).await);

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
    let depot = ligne_vers(&lignes, &synchronise)["horodatage"]
        .as_i64()
        .unwrap();
    let etat = p
        .ok("sauvegarde_hors_ecole_etat", Some(&admin), Value::Null)
        .await;
    assert_eq!(etat["rythme"], "semaine");
    assert_eq!(etat["prochaine_copie"], depot + 7 * JOUR);
    assert_eq!(etat["dossiers"][0]["derniere"]["etat_envoi"], "deposee");
}

/// Critère 1 : un dossier OneDrive trouvé sur le poste est proposé ; choisi, il reçoit la copie
/// à la demande dans son sous-dossier `GESCO`, octet pour octet l'archive du poste.
#[tokio::test]
async fn le_dossier_onedrive_trouve_recoit_la_copie() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(&dossier.path().join("poste")).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    // Rien sur le poste : rien n'est proposé, sans message.
    let trouves = p
        .ok(
            "sauvegarde_dossiers_synchronises",
            Some(&admin),
            Value::Null,
        )
        .await;
    assert_eq!(trouves, json!([]));

    let onedrive = dossier.path().join("OneDrive");
    std::fs::create_dir_all(&onedrive).unwrap();
    p.application
        .systeme_dossiers_simule()
        .variable_posee("OneDrive", Some(&onedrive.to_string_lossy()));
    let trouves = p
        .ok(
            "sauvegarde_dossiers_synchronises",
            Some(&admin),
            Value::Null,
        )
        .await;
    assert_eq!(trouves.as_array().unwrap().len(), 1);
    assert_eq!(trouves[0]["fournisseur"], "one_drive");
    assert_eq!(trouves[0]["nom"], "OneDrive");
    let propose = PathBuf::from(trouves[0]["propose"].as_str().unwrap());
    assert_eq!(propose, onedrive.join("GESCO"));
    // Trouver ne choisit rien : aucune destination n'a changé.
    let destinations = p
        .ok("sauvegarde_destinations_lister", Some(&admin), Value::Null)
        .await;
    assert_eq!(destinations.as_array().unwrap().len(), 1);

    p.ok(
        "sauvegarde_destinations_definir",
        Some(&admin),
        json!([{ "chemin": propose, "sorte": "synchronise", "amovible": false }]),
    )
    .await;
    p.horloge.avancer(60_000);
    let lignes = p
        .ok("sauvegarde_hors_ecole_envoyer", Some(&admin), Value::Null)
        .await;
    let locale = PathBuf::from(lignes[0]["destination"].as_str().unwrap());
    let copie = ligne_vers(&lignes, &propose);
    assert_eq!(copie["etat_envoi"], "deposee");
    let deposees = archives(&propose);
    assert_eq!(deposees.len(), 1);
    assert_eq!(fichiers(&propose), deposees);
    assert_eq!(
        std::fs::read(propose.join(&deposees[0])).unwrap(),
        std::fs::read(&locale).unwrap()
    );
}

/// Critère 10 : depuis un poste relié, même l'administrateur ne touche pas à la copie hors de
/// l'école ; l'écran des sauvegardes, qui en dépend, n'y apparaît pas.
#[tokio::test]
async fn un_poste_relie_ne_touche_pas_a_la_copie_hors_de_l_ecole() {
    use gesco_core::core::Poste;
    use gesco_services::Origine;

    let dossier = tempfile::tempdir().unwrap();
    let (p, _, _, synchronise, _) = poste_avec_dossier(dossier.path()).await;
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
    for (commande, entree) in [
        ("sauvegarde_dossiers_synchronises", Value::Null),
        ("sauvegarde_hors_ecole_etat", Value::Null),
        ("sauvegarde_hors_ecole_envoyer", Value::Null),
        (
            "sauvegarde_hors_ecole_regler",
            json!({ "rythme": "semaine" }),
        ),
        ("sauvegarde_destinations_definir", json!([])),
    ] {
        assert_eq!(
            p.application
                .executer_depuis(caisse.clone(), commande, Some(jeton.clone()), entree)
                .await
                .unwrap_err()
                .cle,
            "commande.poste_principal_seulement",
            "{commande}"
        );
    }
    // Rien n'est parti, rien n'a changé.
    assert!(!synchronise.exists());
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let etat = p
        .ok("sauvegarde_hors_ecole_etat", Some(&admin), Value::Null)
        .await;
    assert_eq!(etat["rythme"], "demande");
    assert_eq!(etat["dossiers"].as_array().unwrap().len(), 1);
}
