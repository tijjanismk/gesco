//! Bibliothèque de cours, installation du paquet (phase 12, lot 1, étape 4) et lecture (étape 5) :
//! le paquet déposé
//! par l'installeur s'installe avec l'école ; une mise à jour plus ancienne, déjà installée ou
//! abîmée est refusée et la bibliothèque en place reste (critère 3) ; une mise à jour réécrit le
//! livré par sa clé ; après une restauration sur un poste qui n'a qu'un paquet plus ancien,
//! l'index se refait depuis celui-ci et l'écran dit qu'une mise à jour existe. Le catalogue range
//! les leçons sous la matière de l'école ; une leçon s'ouvre en arbre de blocs ; la recherche ignore
//! accents et majuscules.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::path::{Path, PathBuf};

use gesco_core::bibliotheque::NatureContenu;
use gesco_core::bibliotheque::paquet::{
    ChapitrePaquet, ContenuPaquet, DisciplinePaquet, ManifestePaquet, fabriquer_paquet,
};
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
        gesco_services::commandes::Acces::Permission(gesco_core::core::Permission::LIBRARY_MANAGE)
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

// --- Lire (étape 5) -------------------------------------------------------------------------

const NOTE: LeconEssai<'static> = (
    "9e-annee/mathematiques/theoreme",
    "Le théorème de Thalès",
    "# Le théorème de Thalès\n\nDeux droites parallèles coupent deux sécantes.\n\n\
     > Note pour le maître : faire tracer la figure au tableau.\n\n\
     $$\\frac{AM}{AB} = \\frac{AN}{AC}$$\n",
);

fn contenus_de(catalogue: &Value) -> Vec<Value> {
    let mut contenus = Vec::new();
    for n in catalogue["niveaux"].as_array().unwrap() {
        for s in n["series"].as_array().unwrap() {
            for m in s["matieres"].as_array().unwrap() {
                for p in m["parties"].as_array().unwrap() {
                    for c in p["chapitres"].as_array().unwrap() {
                        contenus.extend(c["contenus"].as_array().unwrap().iter().cloned());
                    }
                }
            }
        }
    }
    contenus
}

fn types_des_blocs(lecture: &Value) -> Vec<String> {
    lecture["lecon"]["blocs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["type"].as_str().unwrap().to_owned())
        .collect()
}

#[tokio::test]
async fn le_catalogue_range_les_lecons_du_niveau_sous_la_matiere_de_l_ecole() {
    let dossier = tempfile::tempdir().unwrap();
    deposer(
        dossier.path(),
        "bibliotheque-mali-1.gbib",
        &paquet("2026.11.1", &[RACINE, PUISSANCES, NOTE]),
    );
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let catalogue = p
        .ok("bibliotheque_catalogue", Some(&admin), Value::Null)
        .await;
    assert_eq!(catalogue["peut_ecrire"], true);
    assert_eq!(catalogue["peut_valider"], true);
    assert_eq!(catalogue["maitre"], false);
    let neuvieme = catalogue["niveaux"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["code"] == "9")
        .expect("la 9e paraît");
    let matiere = &neuvieme["series"][0]["matieres"][0];
    assert!(matiere["cle"].as_str().unwrap().starts_with("m:"));
    assert_eq!(matiere["discipline_code"], "MATH");
    assert_eq!(matiere["parties"][0]["titre"], "Algèbre");
    let titres: Vec<&str> = matiere["parties"][0]["chapitres"][0]["contenus"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["titre"].as_str().unwrap())
        .collect();
    assert_eq!(
        titres,
        [
            "La racine carrée",
            "Les puissances",
            "Le théorème de Thalès"
        ]
    );
    // Aucun autre niveau ne montre ces leçons de 9e.
    assert_eq!(contenus_de(&catalogue).len(), 3);
}

#[tokio::test]
async fn une_lecon_s_ouvre_en_arbre_de_blocs_et_la_note_reste_au_personnel_qui_ecrit() {
    let dossier = tempfile::tempdir().unwrap();
    deposer(
        dossier.path(),
        "bibliotheque-mali-1.gbib",
        &paquet("2026.11.1", &[NOTE]),
    );
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let catalogue = p
        .ok("bibliotheque_catalogue", Some(&admin), Value::Null)
        .await;
    let id = contenus_de(&catalogue)[0]["id"].clone();
    let lecture = p
        .ok(
            "bibliotheque_lire",
            Some(&admin),
            json!({ "contenu_id": id, "vue": null }),
        )
        .await;
    assert_eq!(lecture["vue"], "visible");
    assert_eq!(lecture["vues"], json!(["visible"]));
    assert_eq!(lecture["version_ecole"], false);
    assert_eq!(
        types_des_blocs(&lecture),
        ["titre", "paragraphe", "note_maitre", "formule"]
    );
    // Une version demandée qui n'existe pas rend la version visible.
    let livree = p
        .ok(
            "bibliotheque_lire",
            Some(&admin),
            json!({ "contenu_id": id, "vue": "livree" }),
        )
        .await;
    assert_eq!(livree["vue"], "visible");

    // La secrétaire lit, sans la note pour le maître.
    let compte = p
        .ok(
            "utilisateur_creer",
            Some(&admin),
            json!({ "identifiant": "awa", "nom_affiche": "Awa Diarra",
                "mot_de_passe_initial": "secretaire-2026", "roles": ["secretaire"] }),
        )
        .await;
    assert!(compte["id"].is_string());
    let secretaire = p.connecter("awa", "secretaire-2026").await;
    let catalogue = p
        .ok("bibliotheque_catalogue", Some(&secretaire), Value::Null)
        .await;
    assert_eq!(catalogue["peut_ecrire"], false);
    let lecture = p
        .ok(
            "bibliotheque_lire",
            Some(&secretaire),
            json!({ "contenu_id": id, "vue": null }),
        )
        .await;
    assert_eq!(
        types_des_blocs(&lecture),
        ["titre", "paragraphe", "formule"]
    );

    // Un contenu inconnu est introuvable.
    let refus = p
        .refus(
            "bibliotheque_lire",
            Some(&admin),
            json!({ "contenu_id": "01JZZZZZZZZZZZZZZZZZZZZZZZ", "vue": null }),
        )
        .await;
    assert_eq!(refus.cle, "erreur.introuvable");
}

#[tokio::test]
async fn la_recherche_ignore_accents_et_majuscules_et_ne_montre_que_le_catalogue() {
    let dossier = tempfile::tempdir().unwrap();
    deposer(
        dossier.path(),
        "bibliotheque-mali-1.gbib",
        &paquet("2026.11.1", &[RACINE, PYTHAGORE, NOTE]),
    );
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let trouves = p
        .ok(
            "bibliotheque_rechercher",
            Some(&admin),
            json!({ "texte": "THEOREME" }),
        )
        .await;
    let titres: Vec<&str> = trouves
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["titre"].as_str().unwrap())
        .collect();
    assert_eq!(titres.len(), 2);
    assert!(titres.contains(&"Le théorème de Pythagore"));
    assert!(titres.contains(&"Le théorème de Thalès"));
    assert_eq!(trouves[0]["chapitre"], "Racines carrées");
    assert!(trouves[0]["extrait"].as_str().unwrap().contains('\u{2}'));
    // Un préfixe suffit ; la note pour le maître n'est pas dans l'index.
    let racine = p
        .ok(
            "bibliotheque_rechercher",
            Some(&admin),
            json!({ "texte": "racin" }),
        )
        .await;
    assert_eq!(racine.as_array().unwrap().len(), 1);
    let note = p
        .ok(
            "bibliotheque_rechercher",
            Some(&admin),
            json!({ "texte": "tableau" }),
        )
        .await;
    assert_eq!(note, json!([]));
    // Les signes de la syntaxe de recherche sont des mots comme les autres.
    let signes = p
        .ok(
            "bibliotheque_rechercher",
            Some(&admin),
            json!({ "texte": "\"racine\" OR NEAR(" }),
        )
        .await;
    assert_eq!(signes, json!([]));
    let vide = p
        .ok(
            "bibliotheque_rechercher",
            Some(&admin),
            json!({ "texte": " ; " }),
        )
        .await;
    assert_eq!(vide, json!([]));
}

#[tokio::test]
async fn sans_paquet_la_bibliotheque_est_vide_et_rien_ne_bloque() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let catalogue = p
        .ok("bibliotheque_catalogue", Some(&admin), Value::Null)
        .await;
    assert_eq!(catalogue["niveaux"], json!([]));
    let trouves = p
        .ok(
            "bibliotheque_rechercher",
            Some(&admin),
            json!({ "texte": "racine" }),
        )
        .await;
    assert_eq!(trouves, json!([]));
}

#[tokio::test]
async fn un_maitre_lit_la_bibliotheque_et_arrive_sur_ses_matieres() {
    let dossier = tempfile::tempdir().unwrap();
    deposer(
        dossier.path(),
        "bibliotheque-mali-1.gbib",
        &paquet("2026.11.1", &[RACINE]),
    );
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let annee = p
        .ok(
            "annee_creer",
            Some(&admin),
            json!({ "libelle": "2026-2027", "debut": crate::commun::RENTREE_2026,
                "fin": crate::commun::FIN_2027 }),
        )
        .await;
    p.ok("annee_activer", Some(&admin), json!({ "id": annee["id"] }))
        .await;
    let ped = p
        .ok("pedagogie_lire", Some(&admin), json!({ "id": annee["id"] }))
        .await;
    let trouver = |liste: &Value, code: &str| -> Value {
        liste
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["code"] == code)
            .unwrap()["id"]
            .clone()
    };
    let classe = p
        .ok(
            "classe_creer",
            Some(&admin),
            json!({ "annee_id": annee["id"], "niveau_id": trouver(&ped["niveaux"], "9"),
                "serie_id": null, "libelle": "9e A" }),
        )
        .await;
    let compte = p
        .ok(
            "utilisateur_creer",
            Some(&admin),
            json!({ "identifiant": "sekou", "nom_affiche": "Sékou Traoré",
                "mot_de_passe_initial": "enseignant-2026", "roles": ["enseignant"] }),
        )
        .await;
    let sekou = p
        .ok(
            "enseignant_enregistrer",
            Some(&admin),
            json!({ "id": null, "nom": "Traoré", "prenoms": "Sékou", "telephones": [],
                "statut": "titulaire", "utilisateur_id": compte["id"], "actif": true }),
        )
        .await;
    p.ok(
        "affectation_ajouter",
        Some(&admin),
        json!({ "classe_id": classe["id"], "matiere_id": trouver(&ped["matieres"], "MATH"),
            "enseignant_id": sekou["enseignant"]["id"] }),
    )
    .await;
    p.admettre_les_maitres(&admin).await;
    let jeton = p.connecter("sekou", "enseignant-2026").await;
    let catalogue = p
        .ok("bibliotheque_catalogue", Some(&jeton), Value::Null)
        .await;
    assert_eq!(catalogue["maitre"], true);
    assert_eq!(catalogue["peut_ecrire"], true);
    assert_eq!(catalogue["peut_valider"], false);
    assert_eq!(
        catalogue["niveaux"][0]["series"][0]["matieres"][0]["mienne"],
        true
    );
    let id = contenus_de(&catalogue)[0]["id"].clone();
    p.ok(
        "bibliotheque_lire",
        Some(&jeton),
        json!({ "contenu_id": id, "vue": null }),
    )
    .await;
}
