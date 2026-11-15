//! Bibliothèque de cours, installation du paquet (phase 12, lot 1, étape 4) et lecture (étape 5) :
//! le paquet déposé
//! par l'installeur s'installe avec l'école ; une mise à jour plus ancienne, déjà installée ou
//! abîmée est refusée et la bibliothèque en place reste (critère 3) ; une mise à jour réécrit le
//! livré par sa clé ; après une restauration sur un poste qui n'a qu'un paquet plus ancien,
//! l'index se refait depuis celui-ci et l'écran dit qu'une mise à jour existe. Le catalogue range
//! les leçons sous la matière de l'école ; une leçon s'ouvre en arbre de blocs ; la recherche ignore
//! accents et majuscules. Écrire (étape 7) : brouillon unique et compteur de révision, aperçu,
//! abandon, comparaison, repartir d'une version, ligne « version plus récente », chapitres,
//! leçons, sujets et corrigés de l'école, portée d'un maître et reprise d'un brouillon.

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

// --- Retirer (étape 6) ----------------------------------------------------------------------

fn retrait(grain: &str, cible: &str, niveau: Option<&str>) -> Value {
    json!({ "grain": grain, "cible": cible, "niveau_code": niveau, "serie_code": null })
}

#[tokio::test]
async fn une_matiere_retiree_disparait_du_catalogue_et_de_la_recherche_puis_revient() {
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
    let catalogue = p
        .ok("bibliotheque_catalogue", Some(&admin), Value::Null)
        .await;
    assert_eq!(catalogue["peut_gerer"], true);
    let matiere = catalogue["niveaux"][0]["series"][0]["matieres"][0]["cle"]
        .as_str()
        .unwrap()
        .to_owned();

    // La confirmation dit ce qui sera caché.
    let demande = retrait("matiere", &matiere, Some("9"));
    let apercu = p
        .ok("bibliotheque_retrait_apercu", Some(&admin), demande.clone())
        .await;
    assert_eq!(
        apercu,
        json!({ "lecons": 2, "sujets": 0, "corriges": 0, "documents": 0 })
    );
    let retraits = p
        .ok("bibliotheque_retirer", Some(&admin), demande.clone())
        .await;
    assert_eq!(retraits.as_array().unwrap().len(), 1);
    assert_eq!(retraits[0]["grain"], "matiere");
    assert_eq!(retraits[0]["retire_par"], "Awa Traoré");
    // Retirer deux fois la même chose ne double pas le retrait.
    let retraits = p.ok("bibliotheque_retirer", Some(&admin), demande).await;
    assert_eq!(retraits.as_array().unwrap().len(), 1);

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

    // Une mise à jour qui ajoute une leçon à la matière ne la fait pas paraître.
    let fichier = ecrire(
        cle.path(),
        "maj.gbib",
        &paquet("2026.12.1", &[RACINE, PUISSANCES, PYTHAGORE]),
    );
    p.ok(
        "bibliotheque_installer",
        Some(&admin),
        json!({ "chemin": fichier.to_string_lossy() }),
    )
    .await;
    let catalogue = p
        .ok("bibliotheque_catalogue", Some(&admin), Value::Null)
        .await;
    assert_eq!(catalogue["niveaux"], json!([]));

    // Remise : tout revient, la leçon nouvelle comprise.
    let id = retraits[0]["id"].clone();
    let restants = p
        .ok(
            "bibliotheque_remettre",
            Some(&admin),
            json!({ "retrait_id": id }),
        )
        .await;
    assert_eq!(restants, json!([]));
    let catalogue = p
        .ok("bibliotheque_catalogue", Some(&admin), Value::Null)
        .await;
    assert_eq!(contenus_de(&catalogue).len(), 3);
    // Une remise faite ne se refait pas.
    let refus = p
        .refus(
            "bibliotheque_remettre",
            Some(&admin),
            json!({ "retrait_id": id }),
        )
        .await;
    assert_eq!(refus.cle, "bibliotheque.retrait_inconnu");
}

#[tokio::test]
async fn un_contenu_cache_par_deux_retraits_le_dit() {
    let dossier = tempfile::tempdir().unwrap();
    deposer(
        dossier.path(),
        "bibliotheque-mali-1.gbib",
        &paquet("2026.11.1", &[RACINE, PUISSANCES]),
    );
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    p.ok(
        "bibliotheque_retirer",
        Some(&admin),
        retrait("contenu", RACINE.0, None),
    )
    .await;
    let catalogue = p
        .ok("bibliotheque_catalogue", Some(&admin), Value::Null)
        .await;
    assert_eq!(contenus_de(&catalogue).len(), 1);
    let retraits = p
        .ok(
            "bibliotheque_retirer",
            Some(&admin),
            retrait("chapitre", "9/MATH/Algèbre/Racines", None),
        )
        .await;
    let chapitre = retraits
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["grain"] == "chapitre")
        .unwrap();
    let contenu = retraits
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["grain"] == "contenu")
        .unwrap();
    assert_eq!(contenu["titre"], "La racine carrée");
    assert_eq!(chapitre["titre"], "Racines carrées");
    assert_eq!(contenu["aussi"], json!([chapitre["id"]]));
    assert_eq!(chapitre["aussi"], json!([]));
}

#[tokio::test]
async fn seule_la_direction_retire_et_regle_la_correspondance() {
    let dossier = tempfile::tempdir().unwrap();
    deposer(
        dossier.path(),
        "bibliotheque-mali-1.gbib",
        &paquet("2026.11.1", &[RACINE]),
    );
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    p.ok(
        "utilisateur_creer",
        Some(&admin),
        json!({ "identifiant": "awa", "nom_affiche": "Awa Diarra",
            "mot_de_passe_initial": "secretaire-2026", "roles": ["secretaire"] }),
    )
    .await;
    let secretaire = p.connecter("awa", "secretaire-2026").await;
    let catalogue = p
        .ok("bibliotheque_catalogue", Some(&secretaire), Value::Null)
        .await;
    assert_eq!(catalogue["peut_gerer"], false);
    for (commande, entree) in [
        ("bibliotheque_retirer", retrait("niveau", "9", None)),
        ("bibliotheque_retrait_apercu", retrait("niveau", "9", None)),
        ("bibliotheque_retraits_lister", Value::Null),
        ("bibliotheque_correspondances_lire", Value::Null),
    ] {
        let refus = p.refus(commande, Some(&secretaire), entree).await;
        assert_eq!(refus.cle, "erreur.permission_refusee", "{commande}");
    }
    // Une matière sans niveau ne se retire pas.
    let refus = p
        .refus(
            "bibliotheque_retirer",
            Some(&admin),
            retrait("matiere", "d:MATH", None),
        )
        .await;
    assert_eq!(refus.cle, "bibliotheque.retrait_incomplet");
}

#[tokio::test]
async fn la_correspondance_range_la_discipline_sous_la_matiere_choisie() {
    let dossier = tempfile::tempdir().unwrap();
    deposer(
        dossier.path(),
        "bibliotheque-mali-1.gbib",
        &paquet("2026.11.1", &[RACINE]),
    );
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let lues = p
        .ok(
            "bibliotheque_correspondances_lire",
            Some(&admin),
            Value::Null,
        )
        .await;
    let ligne = lues["correspondances"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["discipline_code"] == "MATH" && c["niveau_code"] == "9")
        .unwrap()
        .clone();
    assert!(ligne["matiere_id"].is_string(), "posée par code exact");
    assert_eq!(ligne["discipline"], "Mathématiques");

    // Sans matière : les leçons gardent le nom de la bibliothèque.
    p.ok(
        "bibliotheque_correspondance_regler",
        Some(&admin),
        json!({ "discipline_code": "MATH", "niveau_code": "9", "matiere_id": null }),
    )
    .await;
    let catalogue = p
        .ok("bibliotheque_catalogue", Some(&admin), Value::Null)
        .await;
    let matiere = &catalogue["niveaux"][0]["series"][0]["matieres"][0];
    assert_eq!(matiere["cle"], "d:MATH");
    assert_eq!(matiere["libelle"], "Mathématiques");

    // Une matière inconnue est refusée.
    let refus = p
        .refus(
            "bibliotheque_correspondance_regler",
            Some(&admin),
            json!({ "discipline_code": "MATH", "niveau_code": "9",
                "matiere_id": "01JZZZZZZZZZZZZZZZZZZZZZZZ" }),
        )
        .await;
    assert_eq!(refus.cle, "bibliotheque.matiere_inconnue");

    // De retour sous la matière de l'école ; une mise à jour ne change plus ce choix.
    p.ok(
        "bibliotheque_correspondance_regler",
        Some(&admin),
        json!({ "discipline_code": "MATH", "niveau_code": "9",
            "matiere_id": ligne["matiere_id"] }),
    )
    .await;
    let catalogue = p
        .ok("bibliotheque_catalogue", Some(&admin), Value::Null)
        .await;
    assert!(
        catalogue["niveaux"][0]["series"][0]["matieres"][0]["cle"]
            .as_str()
            .unwrap()
            .starts_with("m:")
    );
}

// --- Écrire (étape 7) -----------------------------------------------------------------------

/// Le premier contenu du catalogue de l'administrateur.
async fn premier_contenu(
    p: &crate::commun::Poste_,
    jeton: &gesco_services::session::Jeton,
) -> Value {
    let catalogue = p
        .ok("bibliotheque_catalogue", Some(jeton), Value::Null)
        .await;
    contenus_de(&catalogue)[0]["id"].clone()
}

fn commencer(contenu: &Value, depart: Value) -> Value {
    json!({ "contenu_id": contenu, "depart": depart })
}

fn enregistrer(contenu: &Value, ouverte: &Value, texte: &str) -> Value {
    json!({
        "contenu_id": contenu,
        "version_id": ouverte["brouillon"]["version_id"],
        "revision": ouverte["brouillon"]["revision"],
        "texte": texte,
    })
}

#[tokio::test]
async fn un_brouillon_s_ecrit_et_refuse_un_enregistrement_depasse() {
    let dossier = tempfile::tempdir().unwrap();
    deposer(
        dossier.path(),
        "bibliotheque-mali-1.gbib",
        &paquet("2026.11.1", &[RACINE]),
    );
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let id = premier_contenu(&p, &admin).await;

    let ouverte = p
        .ok(
            "bibliotheque_ecriture_lire",
            Some(&admin),
            json!({ "contenu_id": id }),
        )
        .await;
    assert_eq!(ouverte["brouillon"], Value::Null);
    assert_eq!(ouverte["peut_ecrire"], true);
    assert_eq!(ouverte["a_une_version_livree"], true);
    assert_eq!(ouverte["texte_depart"], RACINE.2);

    // Le brouillon part de la version livrée, qui ne change pas.
    let ouverte = p
        .ok(
            "bibliotheque_brouillon_commencer",
            Some(&admin),
            commencer(&id, json!({ "type": "livree" })),
        )
        .await;
    assert_eq!(ouverte["brouillon"]["numero"], 1);
    assert_eq!(ouverte["brouillon"]["revision"], 1);
    assert_eq!(ouverte["brouillon"]["mien"], true);
    assert_eq!(ouverte["brouillon"]["texte"], RACINE.2);
    // Un seul brouillon à la fois.
    let refus = p
        .refus(
            "bibliotheque_brouillon_commencer",
            Some(&admin),
            commencer(&id, json!({ "type": "visible" })),
        )
        .await;
    assert_eq!(refus.cle, "bibliotheque.brouillon_existe");

    let texte = "# La racine carrée\n\nLa racine carrée de $9$ est $3$.\n";
    let apres = p
        .ok(
            "bibliotheque_brouillon_enregistrer",
            Some(&admin),
            enregistrer(&id, &ouverte, texte),
        )
        .await;
    assert_eq!(apres["brouillon"]["revision"], 2);
    assert_eq!(apres["brouillon"]["texte"], texte);
    // Un enregistrement fait sur la révision lue avant est refusé ; le brouillon ne bouge pas.
    let refus = p
        .refus(
            "bibliotheque_brouillon_enregistrer",
            Some(&admin),
            enregistrer(&id, &ouverte, "autre texte"),
        )
        .await;
    assert_eq!(refus.cle, "bibliotheque.modifiee_ailleurs");

    // Le brouillon se lit dans la vue « Brouillon » ; la version visible reste la livrée.
    let lecture = p
        .ok(
            "bibliotheque_lire",
            Some(&admin),
            json!({ "contenu_id": id, "vue": "brouillon" }),
        )
        .await;
    assert_eq!(lecture["vue"], "brouillon");
    assert_eq!(lecture["peut_modifier"], true);
    let visible = p
        .ok(
            "bibliotheque_lire",
            Some(&admin),
            json!({ "contenu_id": id, "vue": null }),
        )
        .await;
    assert_eq!(visible["version_ecole"], false);

    // L'aperçu rend l'arbre de blocs et dit ce qui s'imprimera tel quel.
    let apercu = p
        .ok(
            "bibliotheque_apercu",
            Some(&admin),
            json!({ "contenu_id": id, "texte": "Texte $\\commandeinconnue{x}$.\n\n> **Note pour le maître** : Image à ajouter ici : une pirogue.\n" }),
        )
        .await;
    assert_eq!(apercu["formules_brutes"], json!(["\\commandeinconnue{x}"]));
    assert_eq!(apercu["images_a_ajouter"], json!(["une pirogue."]));

    // Abandonner : le brouillon reste dans l'historique.
    let abandonne = p
        .ok(
            "bibliotheque_brouillon_abandonner",
            Some(&admin),
            json!({ "contenu_id": id, "version_id": ouverte["brouillon"]["version_id"] }),
        )
        .await;
    assert_eq!(abandonne["brouillon"], Value::Null);
    let historique = p
        .ok(
            "bibliotheque_historique",
            Some(&admin),
            json!({ "contenu_id": id }),
        )
        .await;
    assert_eq!(historique["versions"][0]["etat"], "abandonnee");
    assert_eq!(historique["versions"][0]["depart_livree"], true);
    assert_eq!(historique["livree_plus_recente"], false);
    // Rien à rendre visible : la version visible est déjà la livrée.
    let refus = p
        .refus(
            "bibliotheque_retour_livree",
            Some(&admin),
            json!({ "contenu_id": id }),
        )
        .await;
    assert_eq!(refus.cle, "bibliotheque.deja_livree");
}

#[tokio::test]
async fn comparer_deux_versions_et_repartir_d_une_ancienne() {
    let dossier = tempfile::tempdir().unwrap();
    deposer(
        dossier.path(),
        "bibliotheque-mali-1.gbib",
        &paquet("2026.11.1", &[RACINE]),
    );
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let id = premier_contenu(&p, &admin).await;
    let ouverte = p
        .ok(
            "bibliotheque_brouillon_commencer",
            Some(&admin),
            commencer(&id, json!({ "type": "visible" })),
        )
        .await;
    let texte = "# La racine carrée\n\nLa racine carrée de $81$ est $9$.\n";
    p.ok(
        "bibliotheque_brouillon_enregistrer",
        Some(&admin),
        enregistrer(&id, &ouverte, texte),
    )
    .await;
    let version = ouverte["brouillon"]["version_id"].clone();
    let comparaison = p
        .ok(
            "bibliotheque_comparer",
            Some(&admin),
            json!({ "contenu_id": id, "gauche": { "type": "livree" },
                "droite": { "type": "version", "version_id": version } }),
        )
        .await;
    assert_eq!(comparaison["retirees"], 1);
    assert_eq!(comparaison["ajoutees"], 1);
    let changee = comparaison["rangees"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["change"] == true)
        .unwrap();
    assert_eq!(changee["gauche"]["numero"], 3);
    assert_eq!(
        changee["droite"]["texte"],
        "La racine carrée de $81$ est $9$."
    );

    // Repartir de la version abandonnée : un brouillon nouveau, numéro 2, avec son texte.
    p.ok(
        "bibliotheque_brouillon_abandonner",
        Some(&admin),
        json!({ "contenu_id": id, "version_id": version }),
    )
    .await;
    let reparti = p
        .ok(
            "bibliotheque_brouillon_commencer",
            Some(&admin),
            commencer(&id, json!({ "type": "version", "version_id": version })),
        )
        .await;
    assert_eq!(reparti["brouillon"]["numero"], 2);
    assert_eq!(reparti["brouillon"]["texte"], texte);
    let historique = p
        .ok(
            "bibliotheque_historique",
            Some(&admin),
            json!({ "contenu_id": id }),
        )
        .await;
    assert_eq!(historique["versions"][0]["depart_numero"], 1);
    assert_eq!(historique["versions"][1]["etat"], "abandonnee");
}

#[tokio::test]
async fn une_mise_a_jour_du_paquet_dit_qu_une_version_livree_plus_recente_existe() {
    let dossier = tempfile::tempdir().unwrap();
    let cle = tempfile::tempdir().unwrap();
    deposer(
        dossier.path(),
        "bibliotheque-mali-1.gbib",
        &paquet("2026.11.1", &[RACINE]),
    );
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let id = premier_contenu(&p, &admin).await;
    let ouverte = p
        .ok(
            "bibliotheque_brouillon_commencer",
            Some(&admin),
            commencer(&id, json!({ "type": "livree" })),
        )
        .await;
    let chemin = ecrire(
        cle.path(),
        "mise-a-jour.gbib",
        &paquet("2026.12.1", &[RACINE_REVUE]),
    );
    p.ok(
        "bibliotheque_installer",
        Some(&admin),
        json!({ "chemin": chemin }),
    )
    .await;
    let apres = p
        .ok(
            "bibliotheque_ecriture_lire",
            Some(&admin),
            json!({ "contenu_id": id }),
        )
        .await;
    // Le brouillon de l'école ne change pas ; la ligne « version plus récente » paraît.
    assert_eq!(apres["brouillon"]["texte"], ouverte["brouillon"]["texte"]);
    assert_eq!(apres["livree_plus_recente"], true);
    let historique = p
        .ok(
            "bibliotheque_historique",
            Some(&admin),
            json!({ "contenu_id": id }),
        )
        .await;
    assert_eq!(historique["livree_plus_recente"], true);
}

#[tokio::test]
async fn l_ecole_ajoute_un_chapitre_une_lecon_un_sujet_et_son_corrige() {
    let dossier = tempfile::tempdir().unwrap();
    deposer(
        dossier.path(),
        "bibliotheque-mali-1.gbib",
        &paquet("2026.11.1", &[RACINE]),
    );
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let choix = p
        .ok("bibliotheque_ecriture_choix", Some(&admin), Value::Null)
        .await;
    assert_eq!(choix["partout"], true);
    let neuvieme = choix["niveaux"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["code"] == "9")
        .unwrap()["id"]
        .clone();
    // La matière de l'école qui reçoit les mathématiques de la bibliothèque.
    let catalogue = p
        .ok("bibliotheque_catalogue", Some(&admin), Value::Null)
        .await;
    let math = catalogue["niveaux"][0]["series"][0]["matieres"][0]["matiere_id"].clone();
    assert!(math.is_string());

    let chapitre = p
        .ok(
            "bibliotheque_chapitre_ajouter",
            Some(&admin),
            json!({ "niveau_id": neuvieme, "matiere_id": math, "series": [],
                "partie": "Algèbre", "titre": "Les équations",
                "apres_cle": "9/MATH/Algèbre/Racines" }),
        )
        .await;
    assert!(chapitre["cle"].as_str().unwrap().starts_with("ecole/"));
    let refus = p
        .refus(
            "bibliotheque_chapitre_ajouter",
            Some(&admin),
            json!({ "niveau_id": neuvieme, "matiere_id": math, "series": [],
                "partie": "", "titre": "   ", "apres_cle": null }),
        )
        .await;
    assert_eq!(refus.cle, "bibliotheque.titre_invalide");

    // Une leçon naît brouillon, avec le plan des leçons livrées.
    let lecon = p
        .ok(
            "bibliotheque_contenu_ajouter",
            Some(&admin),
            json!({ "nature": "lecon", "chapitre_id": chapitre["id"], "niveau_id": null,
                "matiere_id": null, "titre": "Résoudre une équation", "presentation": "",
                "examen": null, "serie_code": null, "session": null, "sujet_id": null,
                "apres_cle": null }),
        )
        .await;
    assert_eq!(lecon["origine"], "ecole");
    assert!(
        lecon["brouillon"]["texte"]
            .as_str()
            .unwrap()
            .contains("## L'essentiel")
    );
    // Le chapitre de l'école paraît, rangé après le chapitre livré, pour qui écrit.
    let catalogue = p
        .ok("bibliotheque_catalogue", Some(&admin), Value::Null)
        .await;
    let chapitres: Vec<&str> =
        catalogue["niveaux"][0]["series"][0]["matieres"][0]["parties"][0]["chapitres"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["titre"].as_str().unwrap())
            .collect();
    assert_eq!(chapitres, ["Racines carrées", "Les équations"]);
    assert!(
        contenus_de(&catalogue)
            .iter()
            .any(|c| c["titre"] == "Résoudre une équation" && c["brouillon"] == true)
    );
    // La secrétaire ne voit pas une leçon de l'école pas encore publiée.
    p.ok(
        "utilisateur_creer",
        Some(&admin),
        json!({ "identifiant": "awa", "nom_affiche": "Awa Diarra",
            "mot_de_passe_initial": "secretaire-2026", "roles": ["secretaire"] }),
    )
    .await;
    let secretaire = p.connecter("awa", "secretaire-2026").await;
    let vu = p
        .ok("bibliotheque_catalogue", Some(&secretaire), Value::Null)
        .await;
    assert!(
        !contenus_de(&vu)
            .iter()
            .any(|c| c["titre"] == "Résoudre une équation")
    );
    let refus = p
        .refus(
            "bibliotheque_brouillon_commencer",
            Some(&secretaire),
            commencer(&lecon["contenu_id"], json!({ "type": "visible" })),
        )
        .await;
    assert_eq!(refus.cle, "erreur.permission_refusee");

    // Un sujet sans chapitre va sous « Sujets du DEF », créé une fois.
    let sujet = |titre: &str, session: u16| {
        json!({ "nature": "sujet", "chapitre_id": null, "niveau_id": neuvieme,
            "matiere_id": math, "titre": titre, "presentation": "", "examen": "DEF",
            "serie_code": null, "session": session, "sujet_id": null, "apres_cle": null })
    };
    let s2025 = p
        .ok(
            "bibliotheque_contenu_ajouter",
            Some(&admin),
            sujet("Sujet de mathématiques", 2025),
        )
        .await;
    p.ok(
        "bibliotheque_contenu_ajouter",
        Some(&admin),
        sujet("Sujet de mathématiques", 2024),
    )
    .await;
    let catalogue = p
        .ok("bibliotheque_catalogue", Some(&admin), Value::Null)
        .await;
    let mut sujets_du_def = 0;
    for partie in catalogue["niveaux"][0]["series"][0]["matieres"][0]["parties"]
        .as_array()
        .unwrap()
    {
        for c in partie["chapitres"].as_array().unwrap() {
            if c["titre"] == "Sujets du DEF" {
                sujets_du_def += 1;
                assert_eq!(partie["titre"], "Examens");
                assert_eq!(c["contenus"].as_array().unwrap().len(), 2);
            }
        }
    }
    assert_eq!(sujets_du_def, 1);
    let refus = p
        .refus(
            "bibliotheque_contenu_ajouter",
            Some(&admin),
            json!({ "nature": "sujet", "chapitre_id": null, "niveau_id": neuvieme,
                "matiere_id": math, "titre": "Sans examen", "presentation": "",
                "examen": null, "serie_code": null, "session": null, "sujet_id": null,
                "apres_cle": null }),
        )
        .await;
    assert_eq!(refus.cle, "bibliotheque.examen_manquant");

    // Le corrigé se lie à son sujet, dans les deux sens.
    let corrige = p
        .ok(
            "bibliotheque_contenu_ajouter",
            Some(&admin),
            json!({ "nature": "corrige", "chapitre_id": null, "niveau_id": null,
                "matiere_id": null, "titre": "Corrigé", "presentation": "", "examen": null,
                "serie_code": null, "session": null, "sujet_id": s2025["contenu_id"],
                "apres_cle": null }),
        )
        .await;
    assert_eq!(corrige["nature"], "corrige");
    let lu = p
        .ok(
            "bibliotheque_lire",
            Some(&admin),
            json!({ "contenu_id": s2025["contenu_id"], "vue": null }),
        )
        .await;
    assert_eq!(lu["lie"]["id"], corrige["contenu_id"]);
    assert_eq!(lu["session"], 2025);
}

#[tokio::test]
async fn un_maitre_n_ecrit_que_dans_ses_matieres_et_reprend_le_brouillon_d_un_collegue() {
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
    let math = trouver(&ped["matieres"], "MATH");
    let autre = ped["matieres"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["code"] != "MATH")
        .unwrap()["id"]
        .clone();
    let neuvieme = trouver(&ped["niveaux"], "9");
    let classe = p
        .ok(
            "classe_creer",
            Some(&admin),
            json!({ "annee_id": annee["id"], "niveau_id": neuvieme, "serie_id": null,
                "libelle": "9e A" }),
        )
        .await;
    for (identifiant, nom, prenoms) in [("sekou", "Traoré", "Sékou"), ("fanta", "Keïta", "Fanta")]
    {
        let compte = p
            .ok(
                "utilisateur_creer",
                Some(&admin),
                json!({ "identifiant": identifiant, "nom_affiche": format!("{prenoms} {nom}"),
                    "mot_de_passe_initial": "enseignant-2026", "roles": ["enseignant"] }),
            )
            .await;
        let enseignant = p
            .ok(
                "enseignant_enregistrer",
                Some(&admin),
                json!({ "id": null, "nom": nom, "prenoms": prenoms, "telephones": [],
                    "statut": "titulaire", "utilisateur_id": compte["id"], "actif": true }),
            )
            .await;
        p.ok(
            "affectation_ajouter",
            Some(&admin),
            json!({ "classe_id": classe["id"], "matiere_id": math,
                "enseignant_id": enseignant["enseignant"]["id"] }),
        )
        .await;
    }
    p.admettre_les_maitres(&admin).await;
    let sekou = p.connecter("sekou", "enseignant-2026").await;
    let fanta = p.connecter("fanta", "enseignant-2026").await;

    let catalogue = p
        .ok("bibliotheque_catalogue", Some(&sekou), Value::Null)
        .await;
    assert_eq!(catalogue["ecrit_partout"], false);
    let id = contenus_de(&catalogue)[0]["id"].clone();
    let choix = p
        .ok("bibliotheque_ecriture_choix", Some(&sekou), Value::Null)
        .await;
    assert_eq!(choix["partout"], false);
    assert_eq!(
        choix["places"],
        json!([{ "matiere_id": math, "niveau_code": "9" }])
    );
    // Hors de ses matières, rien ne s'écrit.
    let refus = p
        .refus(
            "bibliotheque_chapitre_ajouter",
            Some(&sekou),
            json!({ "niveau_id": neuvieme, "matiere_id": autre, "series": [], "partie": "",
                "titre": "Un chapitre", "apres_cle": null }),
        )
        .await;
    assert_eq!(refus.cle, "erreur.portee_refusee");
    p.ok(
        "bibliotheque_chapitre_ajouter",
        Some(&sekou),
        json!({ "niveau_id": neuvieme, "matiere_id": math, "series": [], "partie": "",
            "titre": "Exercices de la classe", "apres_cle": null }),
    )
    .await;

    // Sékou commence un brouillon ; Fanta, de la même matière, le voit chez lui et le reprend.
    let ouverte = p
        .ok(
            "bibliotheque_brouillon_commencer",
            Some(&sekou),
            commencer(&id, json!({ "type": "visible" })),
        )
        .await;
    let chez_fanta = p
        .ok(
            "bibliotheque_ecriture_lire",
            Some(&fanta),
            json!({ "contenu_id": id }),
        )
        .await;
    assert_eq!(chez_fanta["brouillon"]["mien"], false);
    assert_eq!(chez_fanta["brouillon"]["auteur"], "Sékou Traoré");
    let refus = p
        .refus(
            "bibliotheque_brouillon_enregistrer",
            Some(&fanta),
            enregistrer(&id, &chez_fanta, "texte"),
        )
        .await;
    assert_eq!(refus.cle, "bibliotheque.brouillon_d_un_autre");
    let reprise = p
        .ok(
            "bibliotheque_brouillon_reprendre",
            Some(&fanta),
            json!({ "contenu_id": id, "version_id": ouverte["brouillon"]["version_id"] }),
        )
        .await;
    assert_eq!(reprise["brouillon"]["mien"], true);
    // L'éditeur de Sékou ne peut plus enregistrer par-dessus.
    let refus = p
        .refus(
            "bibliotheque_brouillon_enregistrer",
            Some(&sekou),
            enregistrer(&id, &ouverte, "texte de Sékou"),
        )
        .await;
    assert_eq!(refus.cle, "bibliotheque.modifiee_ailleurs");
    p.ok(
        "bibliotheque_brouillon_enregistrer",
        Some(&fanta),
        enregistrer(&id, &reprise, "texte de Fanta"),
    )
    .await;
    // Revenir à la version livrée demande le droit de valider.
    let refus = p
        .refus(
            "bibliotheque_retour_livree",
            Some(&fanta),
            json!({ "contenu_id": id }),
        )
        .await;
    assert!(
        ["erreur.permission_refusee", "maitre.commande_refusee"].contains(&refus.cle.as_str()),
        "{}",
        refus.cle
    );
}
