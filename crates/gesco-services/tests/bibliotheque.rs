//! Bibliothèque de cours, installation du paquet (phase 12, lot 1, étape 4) : le paquet déposé
//! par l'installeur s'installe avec l'école ; une mise à jour plus ancienne, déjà installée ou
//! abîmée est refusée et la bibliothèque en place reste (critère 3) ; une mise à jour réécrit le
//! livré par sa clé ; après une restauration sur un poste qui n'a qu'un paquet plus ancien,
//! l'index se refait depuis celui-ci et l'écran dit qu'une mise à jour existe.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::path::{Path, PathBuf};

use gesco_core::bibliotheque::paquet::{
    ChapitrePaquet, ContenuPaquet, DisciplinePaquet, ManifestePaquet, fabriquer_paquet,
};
use gesco_core::bibliotheque::NatureContenu;
use serde_json::{Value, json};

use crate::commun::{GRAINE_BIBLIOTHEQUE, MOT_DE_PASSE, poste_vierge};

/// Une leçon d'essai : (clé, titre, texte).
type LeconEssai<'a> = (&'a str, &'a str, &'a str);

fn paquet(version: &str, lecons: &[LeconEssai<'_>]) -> Vec<u8> {
    let chapitre = ChapitrePaquet {
        cle: "9/MATH/Algèbre/Racines".to_owned(),
        niveau: "9".to_owned(),
        series: vec![],
        discipline: "MATH".to_owned(),
        partie: "Algèbre".to_owned(),
        titre: "Racines carrées".to_owned(),
        ordre: 1,
        a_confirmer: false,
    };
    let mut fichiers = Vec::new();
    let mut contenus = Vec::new();
    for (i, (cle, titre, texte)) in lecons.iter().enumerate() {
        let fichier = format!("{cle}.md");
        fichiers.push((fichier.clone(), texte.as_bytes().to_vec()));
        contenus.push(ContenuPaquet {
            cle: (*cle).to_owned(),
            nature: NatureContenu::Lecon,
            chapitre: chapitre.cle.clone(),
            titre: (*titre).to_owned(),
            ordre: u32::try_from(i + 1).unwrap(),
            lie: None,
            examen: None,
            session: None,
            fichier,
            faits: None,
            images: vec![],
        });
    }
    let manifeste = ManifestePaquet {
        code: "bibliotheque-mali".to_owned(),
        version: version.to_owned(),
        date: "2026-11-15".to_owned(),
        gesco_minimum: "0.1.0".to_owned(),
        programme: String::new(),
        disciplines: vec![DisciplinePaquet {
            code: "MATH".to_owned(),
            libelle: "Mathématiques".to_owned(),
        }],
        chapitres: vec![chapitre],
        contenus,
        fichiers: vec![],
    };
    fabriquer_paquet(&manifeste, &fichiers, &GRAINE_BIBLIOTHEQUE).unwrap()
}

const RACINE: LeconEssai<'static> = (
    "9e-annee/mathematiques/racine-carree",
    "La racine carrée",
    "# La racine carrée\n\nLa racine carrée de $16$ est $4$.\n",
);
const PUISSANCES: LeconEssai<'static> = (
    "9e-annee/mathematiques/puissances",
    "Les puissances",
    "# Les puissances\n\n$2^3 = 8$.\n",
);
const RACINE_REVUE: LeconEssai<'static> = (
    "9e-annee/mathematiques/racine-carree",
    "La racine carrée",
    "# La racine carrée\n\nLa racine carrée de $25$ est $5$.\n",
);
const PYTHAGORE: LeconEssai<'static> = (
    "9e-annee/mathematiques/pythagore",
    "Le théorème de Pythagore",
    "# Le théorème de Pythagore\n\n$a^2 + b^2 = c^2$.\n",
);

fn deposer(racine: &Path, nom: &str, octets: &[u8]) -> PathBuf {
    let dossier = racine.join("bibliotheque");
    std::fs::create_dir_all(&dossier).unwrap();
    let chemin = dossier.join(nom);
    std::fs::write(&chemin, octets).unwrap();
    chemin
}

fn ecrire(dossier: &Path, nom: &str, octets: &[u8]) -> PathBuf {
    let chemin = dossier.join(nom);
    std::fs::write(&chemin, octets).unwrap();
    chemin
}

#[tokio::test]
async fn le_paquet_depose_s_installe_avec_l_ecole() {
    let dossier = tempfile::tempdir().unwrap();
    deposer(
        dossier.path(),
        "bibliotheque-mali-1.gbib",
        &paquet("2026.11.1", &[RACINE, PUISSANCES]),
    );
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let etat = p.ok("bibliotheque_etat", Some(&admin), Value::Null).await;
    assert_eq!(etat["paquets"][0]["version"], "2026.11.1");
    assert_eq!(etat["paquets"][0]["lecons"], 2);
    assert_eq!(etat["paquets"][0]["present"], true);
    assert_eq!(etat["paquets"][0]["version_plus_recente"], Value::Null);
}

#[tokio::test]
async fn une_mise_a_jour_plus_ancienne_deja_installee_ou_abimee_est_refusee() {
    let dossier = tempfile::tempdir().unwrap();
    let cle = tempfile::tempdir().unwrap();
    deposer(
        dossier.path(),
        "bibliotheque-mali-1.gbib",
        &paquet("2026.11.1", &[RACINE, PUISSANCES]),
    );
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;

    let v2 = paquet("2026.12.1", &[RACINE_REVUE, PYTHAGORE]);
    let chemin = ecrire(cle.path(), "mise-a-jour.gbib", &v2);
    let installe = p
        .ok(
            "bibliotheque_installer",
            Some(&admin),
            json!({ "chemin": chemin }),
        )
        .await;
    assert_eq!(installe["version"], "2026.12.1");
    // Par sa clé : la racine carrée change de texte, Pythagore arrive, les puissances ne sont
    // plus livrées.
    assert_eq!(installe["bilan"]["contenus_ajoutes"], 1);
    assert_eq!(installe["bilan"]["contenus_changes"], 1);
    assert_eq!(installe["bilan"]["contenus_plus_livres"], 1);
    assert_eq!(installe["etat"]["paquets"][0]["lecons"], 2);

    for (octets, cle_attendue) in [
        (paquet("2026.11.1", &[RACINE]), "bibliotheque.plus_ancienne"),
        (v2.clone(), "bibliotheque.deja_installee"),
    ] {
        let chemin = ecrire(cle.path(), "autre.gbib", &octets);
        let refus = p
            .refus(
                "bibliotheque_installer",
                Some(&admin),
                json!({ "chemin": chemin }),
            )
            .await;
        assert_eq!(refus.cle, cle_attendue);
    }
    // Un octet changé n'importe où : le paquet est refusé entier.
    let mut abime = paquet("2027.1.1", &[RACINE, PYTHAGORE]);
    let milieu = abime.len() / 2;
    abime[milieu] ^= 0x01;
    let chemin = ecrire(cle.path(), "abime.gbib", &abime);
    let refus = p
        .refus(
            "bibliotheque_installer",
            Some(&admin),
            json!({ "chemin": chemin }),
        )
        .await;
    assert_eq!(refus.cle, "bibliotheque.paquet_abime");
    let etat = p.ok("bibliotheque_etat", Some(&admin), Value::Null).await;
    assert_eq!(etat["paquets"][0]["version"], "2026.12.1");
    assert_eq!(etat["paquets"][0]["present"], true);
    // Le fichier refusé n'a pas été copié dans le dossier de la bibliothèque.
    let copies = std::fs::read_dir(dossier.path().join("bibliotheque"))
        .unwrap()
        .count();
    assert_eq!(copies, 2);
}

#[tokio::test]
async fn un_maitre_n_installe_pas_de_mise_a_jour() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    // Sans paquet déposé, la bibliothèque est vide et rien ne bloque.
    let etat = p.ok("bibliotheque_etat", Some(&admin), Value::Null).await;
    assert_eq!(etat["paquets"], json!([]));
    let commande = gesco_services::commandes::Commande::par_nom("bibliotheque_installer").unwrap();
    assert_eq!(
        commande.acces,
        gesco_services::commandes::Acces::Permission(
            gesco_core::core::Permission::LIBRARY_MANAGE
        )
    );
}

#[tokio::test]
async fn apres_une_restauration_l_index_se_refait_depuis_le_paquet_present() {
    let dossier = tempfile::tempdir().unwrap();
    let cle = tempfile::tempdir().unwrap();
    deposer(
        dossier.path(),
        "bibliotheque-mali-1.gbib",
        &paquet("2026.11.1", &[RACINE, PUISSANCES]),
    );
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let chemin = ecrire(
        cle.path(),
        "mise-a-jour.gbib",
        &paquet("2026.12.1", &[RACINE_REVUE, PYTHAGORE]),
    );
    p.ok(
        "bibliotheque_installer",
        Some(&admin),
        json!({ "chemin": chemin }),
    )
    .await;
    let archive = p.sauvegarder(&admin, &cle.path().join("copies")).await;
    // L'autre ordinateur n'a que le paquet de l'installeur.
    std::fs::remove_file(
        dossier
            .path()
            .join("bibliotheque")
            .join("bibliotheque-mali-2026.12.1.gbib"),
    )
    .unwrap();
    p.horloge.avancer(60_000);
    p.ok(
        "sauvegarde_restaurer",
        Some(&admin),
        json!({ "chemin": archive, "phrase": null }),
    )
    .await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let etat = p.ok("bibliotheque_etat", Some(&admin), Value::Null).await;
    assert_eq!(etat["paquets"][0]["version"], "2026.11.1");
    assert_eq!(etat["paquets"][0]["present"], true);
    assert_eq!(etat["paquets"][0]["lecons"], 2);
    assert_eq!(etat["paquets"][0]["version_plus_recente"], "2026.12.1");
}
