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
//! Publier (étape 8) : soumettre, renvoyer avec un commentaire, publier (la précédente passe à
//! « remplacée »), revenir à la version livrée, file d'attente de qui valide, droit de publier
//! donné nommément à un maître puis retiré, faits à vérifier pour le seul personnel qui écrit.
//! Images, documents, vidéos et impression (étape 9) : bornes, nettoyage du SVG, fichiers d'un
//! document, vidéo copiée, lue par morceaux, absente, copiée vers une clé et reprise ; PDF d'une
//! leçon dans ses deux formes, d'un chapitre et des faits à vérifier.

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
    paquet_avec_faits(version, lecons, None)
}

/// Un paquet dont la première leçon a une liste de faits à vérifier.
fn paquet_avec_faits(version: &str, lecons: &[LeconEssai<'_>], faits: Option<&str>) -> Vec<u8> {
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
        let liste = match faits {
            Some(f) if i == 0 => {
                let nom = format!("{cle}.faits.md");
                fichiers.push((nom.clone(), f.as_bytes().to_vec()));
                Some(nom)
            }
            _ => None,
        };
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
            faits: liste,
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
/// La racine carrée, avec son quiz : un choix unique et une réponse courte.
const RACINE_QUIZ: LeconEssai<'static> = (
    "9e-annee/mathematiques/racine-carree",
    "La racine carrée",
    "# La racine carrée\n\nLa racine carrée de $16$ est $4$.\n\n## Quiz\n\n**Question 1.** √25 est égal à :\nA) 5  B) 25  C) 12\n\n**Question 2.** Combien font 7 / 2 ?\n\n### Réponses\n\n1. **A.** 5 × 5 = 25.\n2. **« 3,5 » ou « 7/2 ».** On partage 7 en deux.\n",
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
    "# Le théorème de Thalès\n\nDeux droites parallèles coupent deux sécantes au cœur de la figure.\n\n\
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
    // « coeur » tapé au clavier trouve « cœur ».
    let coeur = p
        .ok(
            "bibliotheque_rechercher",
            Some(&admin),
            json!({ "texte": "coeur figure" }),
        )
        .await;
    assert_eq!(coeur.as_array().unwrap().len(), 1, "{coeur}");
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

    // Un quiz de chapitre naît brouillon, avec une première question reconnue ; le bouton
    // Question en ajoute une deuxième, numérotée, sa réponse sous « Réponses » ; l'aperçu
    // rend le quiz tel que l'élève le passera et l'auteur seul lit les questions écartées.
    let quiz = p
        .ok(
            "bibliotheque_contenu_ajouter",
            Some(&admin),
            json!({ "nature": "quiz", "chapitre_id": chapitre["id"], "niveau_id": null,
                "matiere_id": null, "titre": "Quiz des équations", "presentation": "",
                "examen": null, "serie_code": null, "session": null, "sujet_id": null,
                "apres_cle": null }),
        )
        .await;
    assert_eq!(quiz["nature"], "quiz");
    let texte_quiz = quiz["brouillon"]["texte"].as_str().unwrap().to_owned();
    assert!(texte_quiz.contains("**Question 1.**"));
    let deux = p
        .ok(
            "bibliotheque_quiz_question",
            Some(&admin),
            json!({ "contenu_id": quiz["contenu_id"], "texte": texte_quiz }),
        )
        .await;
    let deux = deux["texte"].as_str().unwrap();
    assert!(deux.contains("**Question 2.**"));
    assert_eq!(deux.matches("Réponses").count(), 1);
    let apercu_quiz = p
        .ok(
            "bibliotheque_apercu",
            Some(&admin),
            json!({ "contenu_id": quiz["contenu_id"], "texte": format!("{deux}\n**Question 3.** Sans réponse ?\nA) un  B) deux\n") }),
        )
        .await;
    assert_eq!(
        apercu_quiz["quiz"]["questions"].as_array().unwrap().len(),
        2
    );
    assert_eq!(apercu_quiz["quiz"]["ecartees"][0]["numero"], 3);
    // Essayer le quiz : la correction ne demande rien d'autre que la question et la réponse.
    let premiere = apercu_quiz["quiz"]["questions"][0].clone();
    let juste = p
        .ok(
            "bibliotheque_quiz_verifier",
            Some(&admin),
            json!({ "question": premiere, "reponse": "A" }),
        )
        .await;
    assert_eq!(juste, json!(true));
    let faux = p
        .ok(
            "bibliotheque_quiz_verifier",
            Some(&admin),
            json!({ "question": premiere, "reponse": "" }),
        )
        .await;
    assert_eq!(faux, json!(false));
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

// --- Publier (étape 8) ----------------------------------------------------------------------

/// Une année ouverte, une classe de 9e, et des maîtres de mathématiques avec leur compte :
/// (identifiant, nom, prénoms). Rend le jeton de chaque maître, dans l'ordre.
async fn maitres_de_math(
    p: &crate::commun::Poste_,
    admin: &gesco_services::session::Jeton,
    maitres: &[(&str, &str, &str)],
) -> Vec<gesco_services::session::Jeton> {
    let annee = p
        .ok(
            "annee_creer",
            Some(admin),
            json!({ "libelle": "2026-2027", "debut": crate::commun::RENTREE_2026,
                "fin": crate::commun::FIN_2027 }),
        )
        .await;
    p.ok("annee_activer", Some(admin), json!({ "id": annee["id"] }))
        .await;
    let ped = p
        .ok("pedagogie_lire", Some(admin), json!({ "id": annee["id"] }))
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
    let neuvieme = trouver(&ped["niveaux"], "9");
    let classe = p
        .ok(
            "classe_creer",
            Some(admin),
            json!({ "annee_id": annee["id"], "niveau_id": neuvieme, "serie_id": null,
                "libelle": "9e A" }),
        )
        .await;
    for (identifiant, nom, prenoms) in maitres {
        let compte = p
            .ok(
                "utilisateur_creer",
                Some(admin),
                json!({ "identifiant": identifiant, "nom_affiche": format!("{prenoms} {nom}"),
                    "mot_de_passe_initial": "enseignant-2026", "roles": ["enseignant"] }),
            )
            .await;
        let enseignant = p
            .ok(
                "enseignant_enregistrer",
                Some(admin),
                json!({ "id": null, "nom": nom, "prenoms": prenoms, "telephones": [],
                    "statut": "titulaire", "utilisateur_id": compte["id"], "actif": true }),
            )
            .await;
        p.ok(
            "affectation_ajouter",
            Some(admin),
            json!({ "classe_id": classe["id"], "matiere_id": math,
                "enseignant_id": enseignant["enseignant"]["id"] }),
        )
        .await;
    }
    p.admettre_les_maitres(admin).await;
    let mut jetons = Vec::new();
    for (identifiant, _, _) in maitres {
        jetons.push(p.connecter(identifiant, "enseignant-2026").await);
    }
    jetons
}

fn version(contenu: &Value, version_id: &Value) -> Value {
    json!({ "contenu_id": contenu, "version_id": version_id })
}

/// Titres trouvés par la recherche.
async fn trouves(
    p: &crate::commun::Poste_,
    jeton: &gesco_services::session::Jeton,
    texte: &str,
) -> Vec<String> {
    p.ok(
        "bibliotheque_rechercher",
        Some(jeton),
        json!({ "texte": texte }),
    )
    .await
    .as_array()
    .unwrap()
    .iter()
    .map(|r| r["titre"].as_str().unwrap().to_owned())
    .collect()
}

#[tokio::test]
async fn un_maitre_soumet_la_direction_renvoie_puis_publie() {
    let dossier = tempfile::tempdir().unwrap();
    deposer(
        dossier.path(),
        "bibliotheque-mali-1.gbib",
        &paquet("2026.11.1", &[RACINE]),
    );
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let jetons = maitres_de_math(&p, &admin, &[("sekou", "Traoré", "Sékou")]).await;
    let sekou = &jetons[0];
    let id = premier_contenu(&p, sekou).await;

    // Le maître écrit, ne publie pas lui-même : il soumet.
    let ouverte = p
        .ok(
            "bibliotheque_brouillon_commencer",
            Some(sekou),
            commencer(&id, json!({ "type": "visible" })),
        )
        .await;
    assert_eq!(ouverte["peut_publier"], false);
    let premier = "# La racine carrée\n\nUne pirogue de $16$ places.\n";
    let ouverte = p
        .ok(
            "bibliotheque_brouillon_enregistrer",
            Some(sekou),
            enregistrer(&id, &ouverte, premier),
        )
        .await;
    let v1 = ouverte["brouillon"]["version_id"].clone();
    let refus = p
        .refus("bibliotheque_publier", Some(sekou), version(&id, &v1))
        .await;
    assert_eq!(refus.cle, "bibliotheque.sans_droit_de_publier");
    let soumise = p
        .ok("bibliotheque_soumettre", Some(sekou), version(&id, &v1))
        .await;
    assert_eq!(soumise["brouillon"], Value::Null);
    assert_eq!(soumise["soumise"]["mien"], true);
    assert_eq!(soumise["soumise"]["texte"], premier);
    // Une version soumise ne change plus, et aucun brouillon ne commence en attendant.
    let refus = p
        .refus(
            "bibliotheque_brouillon_enregistrer",
            Some(sekou),
            enregistrer(&id, &ouverte, "autre texte"),
        )
        .await;
    assert_eq!(refus.cle, "bibliotheque.modifiee_ailleurs");
    let refus = p
        .refus(
            "bibliotheque_brouillon_commencer",
            Some(sekou),
            commencer(&id, json!({ "type": "visible" })),
        )
        .await;
    assert_eq!(refus.cle, "bibliotheque.en_attente_de_validation");
    // Rien n'est visible avant la publication.
    assert!(trouves(&p, &admin, "pirogue").await.is_empty());

    // La direction voit la leçon en attente, l'ouvre dans la vue « soumise ».
    let attente = p
        .ok("bibliotheque_en_attente", Some(&admin), Value::Null)
        .await;
    assert_eq!(attente.as_array().unwrap().len(), 1);
    assert_eq!(attente[0]["titre"], "La racine carrée");
    assert_eq!(attente[0]["auteur"], "Sékou Traoré");
    assert_eq!(attente[0]["chapitre"], "Racines carrées");
    let refus = p
        .refus("bibliotheque_en_attente", Some(sekou), Value::Null)
        .await;
    assert!(
        ["erreur.permission_refusee", "maitre.commande_refusee"].contains(&refus.cle.as_str()),
        "{}",
        refus.cle
    );
    let lecture = p
        .ok(
            "bibliotheque_lire",
            Some(&admin),
            json!({ "contenu_id": id, "vue": "soumise" }),
        )
        .await;
    assert_eq!(lecture["vue"], "soumise");
    assert_eq!(lecture["version_ecole"], false);

    // Renvoyée avec un commentaire : le texte revient au maître, dans un brouillon nouveau.
    let renvoyee = p
        .ok(
            "bibliotheque_renvoyer",
            Some(&admin),
            json!({ "contenu_id": id, "version_id": v1, "commentaire": " Ajoutez un exemple. " }),
        )
        .await;
    assert_eq!(renvoyee["soumise"], Value::Null);
    assert_eq!(renvoyee["brouillon"]["mien"], false);
    let chez_sekou = p
        .ok(
            "bibliotheque_ecriture_lire",
            Some(sekou),
            json!({ "contenu_id": id }),
        )
        .await;
    assert_eq!(chez_sekou["brouillon"]["mien"], true);
    assert_eq!(chez_sekou["brouillon"]["texte"], premier);
    assert_eq!(
        chez_sekou["brouillon"]["renvoi"]["commentaire"],
        "Ajoutez un exemple."
    );
    assert_eq!(chez_sekou["brouillon"]["renvoi"]["par"], "Awa Traoré");
    let second = "# La racine carrée\n\nUne pirogue de $16$ places : $\\sqrt{16} = 4$.\n";
    let chez_sekou = p
        .ok(
            "bibliotheque_brouillon_enregistrer",
            Some(sekou),
            enregistrer(&id, &chez_sekou, second),
        )
        .await;
    let v2 = chez_sekou["brouillon"]["version_id"].clone();
    p.ok("bibliotheque_soumettre", Some(sekou), version(&id, &v2))
        .await;

    // Publiée : visible aussitôt, trouvée par la recherche ; la file se vide.
    let publiee = p
        .ok("bibliotheque_publier", Some(&admin), version(&id, &v2))
        .await;
    assert_eq!(publiee["soumise"], Value::Null);
    assert_eq!(publiee["version_ecole"], true);
    let visible = p
        .ok(
            "bibliotheque_lire",
            Some(sekou),
            json!({ "contenu_id": id, "vue": null }),
        )
        .await;
    assert_eq!(visible["version_ecole"], true);
    assert_eq!(
        trouves(&p, &admin, "pirogue").await,
        vec!["La racine carrée".to_owned()]
    );
    let attente = p
        .ok("bibliotheque_en_attente", Some(&admin), Value::Null)
        .await;
    assert_eq!(attente, json!([]));
    let historique = p
        .ok(
            "bibliotheque_historique",
            Some(&admin),
            json!({ "contenu_id": id }),
        )
        .await;
    assert_eq!(historique["versions"][0]["etat"], "publiee");
    assert_eq!(historique["versions"][0]["decidee_par"], "Awa Traoré");
    assert_eq!(historique["versions"][1]["etat"], "renvoyee");
    assert_eq!(
        historique["versions"][1]["commentaire"],
        "Ajoutez un exemple."
    );

    // La direction publie sans soumettre ; la version d'avant passe à « remplacée ».
    let ouverte = p
        .ok(
            "bibliotheque_brouillon_commencer",
            Some(&admin),
            commencer(&id, json!({ "type": "visible" })),
        )
        .await;
    assert_eq!(ouverte["peut_publier"], true);
    let troisieme = "# La racine carrée\n\nUne case de $9$ mètres carrés.\n";
    let ouverte = p
        .ok(
            "bibliotheque_brouillon_enregistrer",
            Some(&admin),
            enregistrer(&id, &ouverte, troisieme),
        )
        .await;
    p.ok(
        "bibliotheque_publier",
        Some(&admin),
        version(&id, &ouverte["brouillon"]["version_id"]),
    )
    .await;
    let historique = p
        .ok(
            "bibliotheque_historique",
            Some(&admin),
            json!({ "contenu_id": id }),
        )
        .await;
    assert_eq!(historique["versions"][0]["etat"], "publiee");
    assert_eq!(historique["versions"][1]["etat"], "remplacee");
    assert!(trouves(&p, &admin, "pirogue").await.is_empty());
    assert_eq!(trouves(&p, &admin, "case").await.len(), 1);

    // Revenir à la version livrée : publiée, elle rend visible le texte livré.
    let retour = p
        .ok(
            "bibliotheque_retour_livree",
            Some(&admin),
            json!({ "contenu_id": id }),
        )
        .await;
    assert_eq!(retour["brouillon"]["retour_livre"], true);
    p.ok(
        "bibliotheque_publier",
        Some(&admin),
        version(&id, &retour["brouillon"]["version_id"]),
    )
    .await;
    let visible = p
        .ok(
            "bibliotheque_lire",
            Some(&admin),
            json!({ "contenu_id": id, "vue": null }),
        )
        .await;
    assert_eq!(visible["version_ecole"], false);
    assert!(trouves(&p, &admin, "case").await.is_empty());
    assert_eq!(trouves(&p, &admin, "racine").await.len(), 1);
}

#[tokio::test]
async fn le_droit_de_publier_donne_a_un_maitre_vaut_aussitot_et_se_retire() {
    let dossier = tempfile::tempdir().unwrap();
    deposer(
        dossier.path(),
        "bibliotheque-mali-1.gbib",
        &paquet("2026.11.1", &[RACINE, PUISSANCES]),
    );
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let jetons = maitres_de_math(
        &p,
        &admin,
        &[("sekou", "Traoré", "Sékou"), ("fanta", "Keïta", "Fanta")],
    )
    .await;
    let (sekou, fanta) = (&jetons[0], &jetons[1]);
    let catalogue = p
        .ok("bibliotheque_catalogue", Some(sekou), Value::Null)
        .await;
    let ids: Vec<Value> = contenus_de(&catalogue)
        .iter()
        .map(|c| c["id"].clone())
        .collect();

    // Les deux maîtres sont proposés ; aucun n'a le droit.
    let droits = p
        .ok(
            "bibliotheque_droits_publication_lire",
            Some(&admin),
            Value::Null,
        )
        .await;
    assert_eq!(droits["droits"], json!([]));
    let sekou_id = droits["maitres"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["nom"] == "Sékou Traoré")
        .unwrap()["utilisateur_id"]
        .clone();
    assert_eq!(droits["maitres"].as_array().unwrap().len(), 2);
    let refus = p
        .refus(
            "bibliotheque_droit_publication_donner",
            Some(sekou),
            json!({ "utilisateur_id": sekou_id }),
        )
        .await;
    assert!(
        ["erreur.permission_refusee", "maitre.commande_refusee"].contains(&refus.cle.as_str()),
        "{}",
        refus.cle
    );

    // Donné à Sékou (une seconde fois ne double rien) : il publie son brouillon lui-même.
    p.ok(
        "bibliotheque_droit_publication_donner",
        Some(&admin),
        json!({ "utilisateur_id": sekou_id }),
    )
    .await;
    let droits = p
        .ok(
            "bibliotheque_droit_publication_donner",
            Some(&admin),
            json!({ "utilisateur_id": sekou_id }),
        )
        .await;
    assert_eq!(droits["droits"].as_array().unwrap().len(), 1);
    assert_eq!(droits["droits"][0]["nom"], "Sékou Traoré");
    assert_eq!(droits["droits"][0]["donne_par"], "Awa Traoré");
    assert_eq!(droits["maitres"].as_array().unwrap().len(), 1);
    let ouverte = p
        .ok(
            "bibliotheque_brouillon_commencer",
            Some(sekou),
            commencer(&ids[0], json!({ "type": "visible" })),
        )
        .await;
    assert_eq!(ouverte["peut_publier"], true);
    let ouverte = p
        .ok(
            "bibliotheque_brouillon_enregistrer",
            Some(sekou),
            enregistrer(&ids[0], &ouverte, "# La racine carrée\n\nUn baobab.\n"),
        )
        .await;
    let publiee = p
        .ok(
            "bibliotheque_publier",
            Some(sekou),
            version(&ids[0], &ouverte["brouillon"]["version_id"]),
        )
        .await;
    assert_eq!(publiee["version_ecole"], true);
    assert_eq!(trouves(&p, &admin, "baobab").await.len(), 1);

    // Fanta, sans le droit, soumet ; Sékou ne valide pas sa version.
    let chez_fanta = p
        .ok(
            "bibliotheque_brouillon_commencer",
            Some(fanta),
            commencer(&ids[1], json!({ "type": "visible" })),
        )
        .await;
    assert_eq!(chez_fanta["peut_publier"], false);
    let v_fanta = chez_fanta["brouillon"]["version_id"].clone();
    let refus = p
        .refus(
            "bibliotheque_publier",
            Some(fanta),
            version(&ids[1], &v_fanta),
        )
        .await;
    assert_eq!(refus.cle, "bibliotheque.sans_droit_de_publier");
    p.ok(
        "bibliotheque_soumettre",
        Some(fanta),
        version(&ids[1], &v_fanta),
    )
    .await;
    let refus = p
        .refus(
            "bibliotheque_publier",
            Some(sekou),
            version(&ids[1], &v_fanta),
        )
        .await;
    assert_eq!(refus.cle, "bibliotheque.sans_droit_de_publier");
    // Un autre maître de la matière voit la version soumise de sa collègue.
    let chez_sekou = p
        .ok(
            "bibliotheque_ecriture_lire",
            Some(sekou),
            json!({ "contenu_id": ids[1] }),
        )
        .await;
    assert_eq!(chez_sekou["soumise"]["auteur"], "Fanta Keïta");
    assert_eq!(chez_sekou["soumise"]["mien"], false);

    // Retiré : le droit ne vaut plus, aussitôt.
    let droits = p
        .ok(
            "bibliotheque_droit_publication_retirer",
            Some(&admin),
            json!({ "utilisateur_id": sekou_id }),
        )
        .await;
    assert_eq!(droits["droits"], json!([]));
    assert_eq!(droits["maitres"].as_array().unwrap().len(), 2);
    let ouverte = p
        .ok(
            "bibliotheque_brouillon_commencer",
            Some(sekou),
            commencer(&ids[0], json!({ "type": "visible" })),
        )
        .await;
    assert_eq!(ouverte["peut_publier"], false);
    let refus = p
        .refus(
            "bibliotheque_publier",
            Some(sekou),
            version(&ids[0], &ouverte["brouillon"]["version_id"]),
        )
        .await;
    assert_eq!(refus.cle, "bibliotheque.sans_droit_de_publier");
}

#[tokio::test]
async fn les_faits_a_verifier_ne_se_montrent_qu_au_personnel_qui_ecrit() {
    let dossier = tempfile::tempdir().unwrap();
    deposer(
        dossier.path(),
        "bibliotheque-mali-1.gbib",
        &paquet_avec_faits(
            "2026.11.1",
            &[RACINE],
            Some("1. La racine carrée de $16$ est $4$ (calcul mental).\n"),
        ),
    );
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let id = premier_contenu(&p, &admin).await;
    let lecture = p
        .ok(
            "bibliotheque_lire",
            Some(&admin),
            json!({ "contenu_id": id, "vue": null }),
        )
        .await;
    assert_eq!(lecture["faits"]["blocs"][0]["type"], "liste");
    p.ok(
        "utilisateur_creer",
        Some(&admin),
        json!({ "identifiant": "awa", "nom_affiche": "Awa Diarra",
            "mot_de_passe_initial": "secretaire-2026", "roles": ["secretaire"] }),
    )
    .await;
    let secretaire = p.connecter("awa", "secretaire-2026").await;
    let lecture = p
        .ok(
            "bibliotheque_lire",
            Some(&secretaire),
            json!({ "contenu_id": id, "vue": null }),
        )
        .await;
    assert_eq!(lecture["faits"], Value::Null);
    // Rien de la liste ne passe dans la recherche.
    assert!(trouves(&p, &admin, "mental").await.is_empty());
}

// --- Étape 9 : images, documents, vidéos, impression -----------------------------------------

/// Un PNG d'essai de la taille voulue : la signature, puis des octets quelconques.
fn png(taille: usize) -> Vec<u8> {
    let mut octets = b"\x89PNG\r\n\x1a\n".to_vec();
    octets.resize(taille, 7);
    octets
}

/// Une vidéo MP4 d'essai de la taille voulue.
fn mp4(taille: usize, graine: u8) -> Vec<u8> {
    let mut octets = b"\0\0\0\x18ftypmp42".to_vec();
    octets.resize(taille, graine);
    octets
}

const MO: usize = 1024 * 1024;

fn base64_vers_octets(texte: &Value) -> Vec<u8> {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD
        .decode(texte.as_str().unwrap())
        .unwrap()
}

#[tokio::test]
async fn une_image_se_range_par_son_empreinte_et_une_image_trop_lourde_est_refusee() {
    let dossier = tempfile::tempdir().unwrap();
    deposer(
        dossier.path(),
        "bibliotheque-mali-1.gbib",
        &paquet("2026.11.1", &[RACINE]),
    );
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let fichiers = tempfile::tempdir().unwrap();

    // Critère 13 : une image de 6 Mo est refusée, une image de 4 Mo s'ajoute.
    let lourde = ecrire(fichiers.path(), "lourde.png", &png(6 * MO));
    let refus = p
        .refus(
            "bibliotheque_image_ajouter",
            Some(&admin),
            json!({ "chemin": lourde }),
        )
        .await;
    assert_eq!(refus.cle, "bibliotheque.image_trop_lourde");
    let bonne = ecrire(fichiers.path(), "triangle.png", &png(4 * MO));
    let image = p
        .ok(
            "bibliotheque_image_ajouter",
            Some(&admin),
            json!({ "chemin": bonne }),
        )
        .await;
    let reference = image["reference"].as_str().unwrap().to_owned();
    assert!(reference.starts_with("image:"));
    assert_eq!(image["nom"], "triangle");
    // La même image ajoutée deux fois n'est gardée qu'une fois.
    let encore = p
        .ok(
            "bibliotheque_image_ajouter",
            Some(&admin),
            json!({ "chemin": bonne }),
        )
        .await;
    assert_eq!(encore["reference"], reference.as_str());

    // Un fichier qui n'est pas une image est refusé ; un SVG est nettoyé, ou refusé.
    let texte = ecrire(fichiers.path(), "note.png", b"pas une image");
    let refus = p
        .refus(
            "bibliotheque_image_ajouter",
            Some(&admin),
            json!({ "chemin": texte }),
        )
        .await;
    assert_eq!(refus.cle, "bibliotheque.image_inconnue");
    let svg = ecrire(
        fichiers.path(),
        "figure.svg",
        b"<svg xmlns=\"http://www.w3.org/2000/svg\" onload=\"x()\"><script>x()</script><rect width=\"4\" height=\"4\"/></svg>",
    );
    let figure = p
        .ok(
            "bibliotheque_image_ajouter",
            Some(&admin),
            json!({ "chemin": svg }),
        )
        .await;
    let malforme = ecrire(fichiers.path(), "casse.svg", b"<svg><rect width=4/></svg>");
    let refus = p
        .refus(
            "bibliotheque_image_ajouter",
            Some(&admin),
            json!({ "chemin": malforme }),
        )
        .await;
    assert_eq!(refus.cle, "bibliotheque.svg_refuse");

    // L'aperçu d'un texte qui les appelle les montre ; le SVG n'a plus de script.
    let id = premier_contenu(&p, &admin).await;
    let texte = format!(
        "![Triangle]({reference})\n\n![Figure]({})",
        figure["reference"].as_str().unwrap()
    );
    let apercu = p
        .ok(
            "bibliotheque_apercu",
            Some(&admin),
            json!({ "contenu_id": id, "texte": texte }),
        )
        .await;
    let images = apercu["images"].as_array().unwrap();
    assert_eq!(images.len(), 2);
    let svg_lu = images
        .iter()
        .find(|i| i["type_mime"] == "image/svg+xml")
        .unwrap();
    let propre = String::from_utf8(base64_vers_octets(&svg_lu["donnees"])).unwrap();
    assert!(!propre.contains("script") && !propre.contains("onload"));
    assert!(propre.contains("<rect"));
}

/// Critères 13 et 14 : une image de 4 Mo et un PDF de 15 Mo sont retrouvés après sauvegarde et
/// restauration sur un ordinateur neuf ; la vidéo, elle, n'est pas dans la sauvegarde.
#[tokio::test]
async fn image_et_pdf_reviennent_avec_la_sauvegarde_mais_pas_la_video() {
    let dossier = tempfile::tempdir().unwrap();
    let livre = paquet("2026.11.1", &[RACINE]);
    deposer(
        &dossier.path().join("origine"),
        "bibliotheque-mali-1.gbib",
        &livre,
    );
    let origine = poste_vierge(&dossier.path().join("origine")).await;
    let phrase = origine.installer().await;
    let admin = origine.connecter("admin", MOT_DE_PASSE).await;
    let fichiers = tempfile::tempdir().unwrap();
    let image_octets = png(4 * MO);
    let image = origine
        .ok(
            "bibliotheque_image_ajouter",
            Some(&admin),
            json!({ "chemin": ecrire(fichiers.path(), "carte.png", &image_octets) }),
        )
        .await;
    let catalogue = origine
        .ok("bibliotheque_catalogue", Some(&admin), Value::Null)
        .await;
    let chapitre = catalogue["niveaux"][0]["series"][0]["matieres"][0]["parties"][0]["chapitres"]
        [0]["id"]
        .clone();
    let document = origine
        .ok(
            "bibliotheque_contenu_ajouter",
            Some(&admin),
            json!({ "nature": "document", "chapitre_id": chapitre, "niveau_id": null,
                "matiere_id": null, "titre": "Sujet photographié", "presentation": "",
                "examen": null, "serie_code": null, "session": null, "sujet_id": null,
                "apres_cle": null }),
        )
        .await;
    let mut pdf = b"%PDF-1.7\n".to_vec();
    pdf.resize(15 * MO, b'y');
    let joints = origine
        .ok(
            "bibliotheque_document_fichier_ajouter",
            Some(&admin),
            json!({ "contenu_id": document["contenu_id"],
                "chemin": ecrire(fichiers.path(), "sujet.pdf", &pdf) }),
        )
        .await;
    let empreinte_pdf = joints[0]["empreinte"].clone();
    let video = origine
        .ok(
            "bibliotheque_video_ajouter",
            Some(&admin),
            json!({ "chemin": ecrire(fichiers.path(), "classe.mp4", &mp4(MO, 3)), "confirme": false }),
        )
        .await;
    assert_eq!(video["ajoutee"], true);
    let empreinte_video = video["reference"]
        .as_str()
        .unwrap()
        .trim_start_matches("video:")
        .to_owned();
    origine
        .ok(
            "bibliotheque_video_morceau",
            Some(&admin),
            json!({ "empreinte": empreinte_video, "position": 0 }),
        )
        .await;
    let archive = origine
        .sauvegarder(&admin, &dossier.path().join("cle-usb"))
        .await;
    origine.application.fermer().await.unwrap();
    drop(origine);

    // Un ordinateur neuf, qui n'a que le paquet de l'installeur.
    let racine_neuf = dossier.path().join("neuf");
    deposer(&racine_neuf, "bibliotheque-mali-1.gbib", &livre);
    let neuf = poste_vierge(&racine_neuf).await;
    neuf.ok(
        "sauvegarde_restaurer_initiale",
        None,
        json!({ "chemin": archive, "phrase": phrase }),
    )
    .await;
    let admin = neuf.connecter("admin", MOT_DE_PASSE).await;
    let lecon = premier_contenu(&neuf, &admin).await;
    let apercu = neuf
        .ok(
            "bibliotheque_apercu",
            Some(&admin),
            json!({ "contenu_id": lecon,
                "texte": format!("![Carte]({})", image["reference"].as_str().unwrap()) }),
        )
        .await;
    assert_eq!(
        base64_vers_octets(&apercu["images"][0]["donnees"]),
        image_octets
    );
    let copie = fichiers.path().join("copie.pdf");
    neuf.ok(
        "bibliotheque_fichier_enregistrer_sous",
        Some(&admin),
        json!({ "empreinte": empreinte_pdf, "chemin": copie }),
    )
    .await;
    assert_eq!(std::fs::read(&copie).unwrap(), pdf);
    // La vidéo n'a pas voyagé avec la sauvegarde : elle se reprend d'une clé.
    neuf.refus(
        "bibliotheque_video_morceau",
        Some(&admin),
        json!({ "empreinte": empreinte_video, "position": 0 }),
    )
    .await;
    assert!(
        !racine_neuf
            .join("videos")
            .join(format!("{empreinte_video}.mp4"))
            .exists()
    );
}

#[tokio::test]
async fn un_document_de_l_ecole_recoit_ses_fichiers_et_les_rend() {
    let dossier = tempfile::tempdir().unwrap();
    deposer(
        dossier.path(),
        "bibliotheque-mali-1.gbib",
        &paquet("2026.11.1", &[RACINE]),
    );
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let catalogue = p
        .ok("bibliotheque_catalogue", Some(&admin), Value::Null)
        .await;
    let chapitre = catalogue["niveaux"][0]["series"][0]["matieres"][0]["parties"][0]["chapitres"]
        [0]["id"]
        .clone();
    // La leçon livrée, lue avant que le document ne prenne la tête du chapitre.
    let lecon = premier_contenu(&p, &admin).await;
    let document = p
        .ok(
            "bibliotheque_contenu_ajouter",
            Some(&admin),
            json!({ "nature": "document", "chapitre_id": chapitre, "niveau_id": null,
                "matiere_id": null, "titre": "Sujet photographié",
                "presentation": "La photo du sujet.", "examen": null, "serie_code": null,
                "session": null, "sujet_id": null, "apres_cle": null }),
        )
        .await;
    let id = document["contenu_id"].clone();
    let fichiers = tempfile::tempdir().unwrap();
    // Critère 13 : un PDF de 15 Mo s'ajoute ; un PDF de plus de 20 Mo est refusé.
    let mut pdf = b"%PDF-1.7\n".to_vec();
    pdf.resize(15 * MO, b'x');
    let chemin = ecrire(fichiers.path(), "sujet.pdf", &pdf);
    let liste = p
        .ok(
            "bibliotheque_document_fichier_ajouter",
            Some(&admin),
            json!({ "contenu_id": id, "chemin": chemin }),
        )
        .await;
    assert_eq!(liste.as_array().unwrap().len(), 1);
    assert_eq!(liste[0]["nom"], "sujet.pdf");
    assert_eq!(liste[0]["type_mime"], "application/pdf");
    let mut trop = b"%PDF-1.7\n".to_vec();
    trop.resize(21 * MO, b'x');
    let chemin_trop = ecrire(fichiers.path(), "gros.pdf", &trop);
    let refus = p
        .refus(
            "bibliotheque_document_fichier_ajouter",
            Some(&admin),
            json!({ "contenu_id": id, "chemin": chemin_trop }),
        )
        .await;
    assert_eq!(refus.cle, "bibliotheque.document_trop_lourd");
    // Un fichier ne se joint pas à une leçon.
    let refus = p
        .refus(
            "bibliotheque_document_fichier_ajouter",
            Some(&admin),
            json!({ "contenu_id": lecon, "chemin": chemin }),
        )
        .await;
    assert_eq!(refus.cle, "bibliotheque.pas_un_document");

    // La lecture rend la liste ; le fichier s'enregistre tel quel ailleurs.
    let lecture = p
        .ok(
            "bibliotheque_lire",
            Some(&admin),
            json!({ "contenu_id": id, "vue": null }),
        )
        .await;
    assert_eq!(lecture["fichiers"][0]["empreinte"], liste[0]["empreinte"]);
    let copie = fichiers.path().join("copie.pdf");
    p.ok(
        "bibliotheque_fichier_enregistrer_sous",
        Some(&admin),
        json!({ "empreinte": liste[0]["empreinte"], "chemin": copie }),
    )
    .await;
    assert_eq!(std::fs::read(&copie).unwrap(), pdf);
    // Retiré du document, il ne paraît plus ; il reste dans la base.
    let liste = p
        .ok(
            "bibliotheque_document_fichier_retirer",
            Some(&admin),
            json!({ "contenu_id": id, "lien_id": liste[0]["lien_id"] }),
        )
        .await;
    assert!(liste.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn une_video_se_copie_se_lit_par_morceaux_et_se_reprend_depuis_une_cle() {
    let dossier = tempfile::tempdir().unwrap();
    deposer(
        dossier.path(),
        "bibliotheque-mali-1.gbib",
        &paquet("2026.11.1", &[RACINE]),
    );
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let fichiers = tempfile::tempdir().unwrap();

    // Un fichier qui n'est pas une vidéo MP4 est refusé.
    let faux = ecrire(fichiers.path(), "film.mp4", b"pas une video");
    let refus = p
        .refus(
            "bibliotheque_video_ajouter",
            Some(&admin),
            json!({ "chemin": faux, "confirme": false }),
        )
        .await;
    assert_eq!(refus.cle, "bibliotheque.video_inconnue");

    // La première vidéo le dit ; elle se range sous son empreinte.
    let courte = mp4(5 * MO, 1);
    let chemin = ecrire(fichiers.path(), "lecon.mp4", &courte);
    let video = p
        .ok(
            "bibliotheque_video_ajouter",
            Some(&admin),
            json!({ "chemin": chemin, "confirme": false }),
        )
        .await;
    assert_eq!(video["ajoutee"], true);
    assert_eq!(video["premiere"], true);
    let reference = video["reference"].as_str().unwrap().to_owned();
    let empreinte = reference.strip_prefix("video:").unwrap().to_owned();
    let range = dossier
        .path()
        .join("videos")
        .join(format!("{empreinte}.mp4"));
    assert_eq!(std::fs::read(&range).unwrap(), courte);

    // Au-delà de la taille conseillée : une confirmation, jamais un refus.
    p.ok(
        "parametre_definir",
        Some(&admin),
        json!({ "cle": "bibliotheque.taille_video_conseillee",
            "valeur": { "type": "entier", "valeur": 10 } }),
    )
    .await;
    let lourde = ecrire(fichiers.path(), "longue.mp4", &mp4(11 * MO, 2));
    let attente = p
        .ok(
            "bibliotheque_video_ajouter",
            Some(&admin),
            json!({ "chemin": lourde, "confirme": false }),
        )
        .await;
    assert_eq!(attente["ajoutee"], false);
    assert_eq!(attente["lourde"], true);
    let ajoutee = p
        .ok(
            "bibliotheque_video_ajouter",
            Some(&admin),
            json!({ "chemin": lourde, "confirme": true }),
        )
        .await;
    assert_eq!(ajoutee["ajoutee"], true);
    assert_eq!(ajoutee["premiere"], false);

    // Elle se lit par morceaux.
    let mut lu: Vec<u8> = Vec::new();
    loop {
        let morceau = p
            .ok(
                "bibliotheque_video_morceau",
                Some(&admin),
                json!({ "empreinte": empreinte, "position": lu.len() }),
            )
            .await;
        let octets = base64_vers_octets(&morceau["contenu"]);
        if octets.is_empty() {
            break;
        }
        lu.extend(octets);
        if lu.len() as i64 >= morceau["taille"].as_i64().unwrap() {
            break;
        }
    }
    assert_eq!(lu, courte);

    // Une leçon qui l'appelle s'ouvre ; absente du dossier, elle est « non disponible ».
    let id = premier_contenu(&p, &admin).await;
    let ouverte = p
        .ok(
            "bibliotheque_brouillon_commencer",
            Some(&admin),
            commencer(&id, json!({ "type": "visible" })),
        )
        .await;
    let texte = format!("# Leçon\n\n![Au tableau]({reference})\n");
    p.ok(
        "bibliotheque_brouillon_enregistrer",
        Some(&admin),
        enregistrer(&id, &ouverte, &texte),
    )
    .await;
    let lecture = p
        .ok(
            "bibliotheque_lire",
            Some(&admin),
            json!({ "contenu_id": id, "vue": "brouillon" }),
        )
        .await;
    assert_eq!(lecture["videos"][0]["disponible"], true);

    // Copiée vers une clé, puis retirée du dossier : la leçon s'ouvre sans elle.
    let cle = tempfile::tempdir().unwrap();
    let bilan = p
        .ok(
            "bibliotheque_videos_copier",
            Some(&admin),
            json!({ "dossier": cle.path() }),
        )
        .await;
    assert_eq!(bilan["copiees"], 2);
    std::fs::remove_file(&range).unwrap();
    let lecture = p
        .ok(
            "bibliotheque_lire",
            Some(&admin),
            json!({ "contenu_id": id, "vue": "brouillon" }),
        )
        .await;
    assert_eq!(lecture["videos"][0]["disponible"], false);
    let refus = p
        .refus(
            "bibliotheque_video_morceau",
            Some(&admin),
            json!({ "empreinte": empreinte, "position": 0 }),
        )
        .await;
    assert_eq!(refus.cle, "bibliotheque.video_absente");
    // Reprise depuis la clé : seule la vidéo absente revient, vérifiée par son empreinte.
    let bilan = p
        .ok(
            "bibliotheque_videos_reprendre",
            Some(&admin),
            json!({ "dossier": cle.path() }),
        )
        .await;
    assert_eq!(bilan["copiees"], 1);
    assert_eq!(bilan["deja_la"], 1);
    assert_eq!(std::fs::read(&range).unwrap(), courte);
}

const LECON_CORRIGEE: LeconEssai<'static> = (
    "9e-annee/mathematiques/racine-carree",
    "La racine carrée",
    "# La racine carrée\n\n## 1. L'essentiel\n\n$\\sqrt{16} = 4$ et $\\frac{1}{2}$.\n\n## 5. Exercices\n\nCalculer $\\sqrt{25}$.\n\n## 6. Corrigés\n\n$\\sqrt{25} = 5$.\n\n## 7. Quiz\n\n1. Combien fait $\\sqrt{9}$ ?\n\n### Réponses\n\n1. $3$\n\n> Note pour le maître : insister sur le signe.\n",
);

#[tokio::test]
async fn une_lecon_un_chapitre_et_les_faits_s_impriment_en_pdf() {
    let dossier = tempfile::tempdir().unwrap();
    deposer(
        dossier.path(),
        "bibliotheque-mali-1.gbib",
        &paquet_avec_faits(
            "2026.11.1",
            &[LECON_CORRIGEE, PUISSANCES],
            Some("1. La racine carrée de $16$ est $4$.\n"),
        ),
    );
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let id = premier_contenu(&p, &admin).await;
    let sortie = tempfile::tempdir().unwrap();
    let complete = sortie.path().join("complete.pdf");
    p.ok(
        "bibliotheque_imprimer",
        Some(&admin),
        json!({ "contenu_id": id, "brouillon": false, "forme": "complete", "chemin": complete }),
    )
    .await;
    let sans = sortie.path().join("sans.pdf");
    p.ok(
        "bibliotheque_imprimer",
        Some(&admin),
        json!({ "contenu_id": id, "brouillon": false, "forme": "sans_corriges", "chemin": sans }),
    )
    .await;
    let complete = std::fs::read(&complete).unwrap();
    let sans = std::fs::read(&sans).unwrap();
    assert!(complete.starts_with(b"%PDF-"));
    assert!(sans.starts_with(b"%PDF-"));
    assert_ne!(complete, sans);

    // Le chapitre entier ; les faits à vérifier pour qui écrit.
    let catalogue = p
        .ok("bibliotheque_catalogue", Some(&admin), Value::Null)
        .await;
    let chapitre = catalogue["niveaux"][0]["series"][0]["matieres"][0]["parties"][0]["chapitres"]
        [0]["id"]
        .clone();
    let chemin = sortie.path().join("chapitre.pdf");
    p.ok(
        "bibliotheque_chapitre_imprimer",
        Some(&admin),
        json!({ "chapitre_id": chapitre, "forme": "complete", "chemin": chemin }),
    )
    .await;
    assert!(std::fs::read(&chemin).unwrap().starts_with(b"%PDF-"));
    let faits = sortie.path().join("faits.pdf");
    p.ok(
        "bibliotheque_faits_imprimer",
        Some(&admin),
        json!({ "contenu_id": id, "brouillon": false, "forme": "complete", "chemin": faits }),
    )
    .await;
    assert!(std::fs::read(&faits).unwrap().starts_with(b"%PDF-"));
    // Aucune composition n'a dû repartir sans images ni sans formules converties.
    let journal = std::fs::read_to_string(
        dossier
            .path()
            .join(gesco_services::journal_technique::FICHIER),
    )
    .unwrap_or_default();
    assert!(!journal.contains("impression reprise"), "{journal}");

    // Les faits à vérifier ne s'impriment qu'avec le droit d'écrire ; la leçon, avec celui de
    // lire.
    for (nom, permission) in [
        (
            "bibliotheque_faits_imprimer",
            gesco_core::core::Permission::LIBRARY_EDIT,
        ),
        (
            "bibliotheque_imprimer",
            gesco_core::core::Permission::LIBRARY_READ,
        ),
    ] {
        let commande = gesco_services::commandes::Commande::par_nom(nom).unwrap();
        assert_eq!(
            commande.acces,
            gesco_services::commandes::Acces::Permission(permission)
        );
    }
}

/// Un son d'essai de la taille voulue, avec l'en-tête de sa sorte.
fn son(entete: &[u8], taille: usize, graine: u8) -> Vec<u8> {
    let mut octets = entete.to_vec();
    octets.resize(taille, graine);
    octets
}

/// Lit un son ou une vidéo par morceaux, comme l'écran.
async fn lire_par_morceaux(
    p: &crate::commun::Poste_,
    session: &gesco_services::session::Jeton,
    empreinte: &str,
) -> Vec<u8> {
    let mut lu: Vec<u8> = Vec::new();
    loop {
        let morceau = p
            .ok(
                "bibliotheque_video_morceau",
                Some(session),
                json!({ "empreinte": empreinte, "position": lu.len() }),
            )
            .await;
        let octets = base64_vers_octets(&morceau["contenu"]);
        if octets.is_empty() {
            break;
        }
        lu.extend(octets);
        if lu.len() as u64 >= morceau["taille"].as_u64().unwrap() {
            break;
        }
    }
    lu
}

/// Lot 2, critères 1 et 2 : un MP3 de 3 Mo et un M4A s'ajoutent et se lisent par morceaux ; un son
/// de 12 Mo et un faux `.mp3` sont refusés ; après sauvegarde et restauration sur un ordinateur
/// neuf, les sons sont là ; un son absent ne bloque pas la leçon.
#[tokio::test]
async fn un_son_s_ajoute_se_lit_et_revient_avec_la_sauvegarde() {
    let dossier = tempfile::tempdir().unwrap();
    let livre = paquet("2026.11.1", &[RACINE]);
    deposer(
        &dossier.path().join("origine"),
        "bibliotheque-mali-1.gbib",
        &livre,
    );
    let origine = poste_vierge(&dossier.path().join("origine")).await;
    let phrase = origine.installer().await;
    let admin = origine.connecter("admin", MOT_DE_PASSE).await;
    let fichiers = tempfile::tempdir().unwrap();

    let mp3 = son(b"ID3\x04\0\0\0\0\0\0", 3 * MO, 5);
    let ajoute = origine
        .ok(
            "bibliotheque_son_ajouter",
            Some(&admin),
            json!({ "chemin": ecrire(fichiers.path(), "Hello.mp3", &mp3) }),
        )
        .await;
    let reference_mp3 = ajoute["reference"].as_str().unwrap().to_owned();
    assert!(reference_mp3.starts_with("son:"));
    assert_eq!(ajoute["nom"], "Hello");
    let m4a = son(b"\0\0\0\x20ftypM4A \0\0\0\0", MO / 2, 6);
    let ajoute_m4a = origine
        .ok(
            "bibliotheque_son_ajouter",
            Some(&admin),
            json!({ "chemin": ecrire(fichiers.path(), "consigne.m4a", &m4a) }),
        )
        .await;
    let reference_m4a = ajoute_m4a["reference"].as_str().unwrap().to_owned();

    // Un son de 12 Mo, un PNG renommé « .mp3 » : refusés, chacun avec son message.
    let refus = origine
        .refus(
            "bibliotheque_son_ajouter",
            Some(&admin),
            json!({ "chemin": ecrire(fichiers.path(), "long.mp3", &son(b"ID3\x03", 12 * MO, 1)) }),
        )
        .await;
    assert_eq!(refus.cle, "bibliotheque.son_trop_lourd");
    let refus = origine
        .refus(
            "bibliotheque_son_ajouter",
            Some(&admin),
            json!({ "chemin": ecrire(fichiers.path(), "faux.mp3", &png(1000)) }),
        )
        .await;
    assert_eq!(refus.cle, "bibliotheque.son_inconnu");

    // L'aperçu les connaît ; un son appelé mais absent est dit absent, sans erreur.
    let lecon = premier_contenu(&origine, &admin).await;
    let absent = "c".repeat(64);
    let texte = format!(
        "![Hello]({reference_mp3})\n\n![Consigne]({reference_m4a})\n\n![Perdu](son:{absent})"
    );
    let apercu = origine
        .ok(
            "bibliotheque_apercu",
            Some(&admin),
            json!({ "contenu_id": lecon, "texte": texte }),
        )
        .await;
    let sons = apercu["sons"].as_array().unwrap();
    assert_eq!(sons.len(), 3);
    let empreinte_mp3 = reference_mp3.trim_start_matches("son:");
    let mp3_lu = sons
        .iter()
        .find(|s| s["empreinte"] == empreinte_mp3)
        .unwrap();
    assert_eq!(mp3_lu["disponible"], true);
    assert_eq!(mp3_lu["type_mime"], "audio/mpeg");
    let m4a_lu = sons
        .iter()
        .find(|s| s["empreinte"] == reference_m4a.trim_start_matches("son:"))
        .unwrap();
    assert_eq!(m4a_lu["type_mime"], "audio/mp4");
    assert_eq!(
        sons.iter()
            .find(|s| s["empreinte"] == absent.as_str())
            .unwrap()["disponible"],
        false
    );

    // Il se lit par morceaux, par la route des vidéos (celle des postes reliés).
    assert_eq!(
        lire_par_morceaux(&origine, &admin, empreinte_mp3).await,
        mp3
    );

    let archive = origine
        .sauvegarder(&admin, &dossier.path().join("cle-usb"))
        .await;
    origine.application.fermer().await.unwrap();
    drop(origine);

    let racine_neuf = dossier.path().join("neuf");
    deposer(&racine_neuf, "bibliotheque-mali-1.gbib", &livre);
    let neuf = poste_vierge(&racine_neuf).await;
    neuf.ok(
        "sauvegarde_restaurer_initiale",
        None,
        json!({ "chemin": archive, "phrase": phrase }),
    )
    .await;
    let admin = neuf.connecter("admin", MOT_DE_PASSE).await;
    assert_eq!(lire_par_morceaux(&neuf, &admin, empreinte_mp3).await, mp3);
    assert_eq!(
        lire_par_morceaux(&neuf, &admin, reference_m4a.trim_start_matches("son:")).await,
        m4a
    );
}

/// Lot 2, critère 4 : une leçon de l'école réservée à la 9e A porte la marque « 9e A
/// seulement » au catalogue ; un maître ne coche que ses classes ; le livré reste pour tout le
/// niveau. (La règle « visible pour une classe » est essayée dans le noyau.)
#[tokio::test]
async fn une_lecon_de_l_ecole_se_reserve_a_la_9e_a() {
    let dossier = tempfile::tempdir().unwrap();
    deposer(
        dossier.path(),
        "bibliotheque-mali-1.gbib",
        &paquet("2026.11.1", &[RACINE]),
    );
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let jetons = maitres_de_math(&p, &admin, &[("sekou", "Traoré", "Sékou")]).await;
    let sekou = &jetons[0];
    // Une seconde classe de 9e, où Sékou n'enseigne pas.
    let annees = p.ok("annees_lister", Some(&admin), Value::Null).await;
    let annee = annees
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["libelle"] == "2026-2027")
        .unwrap()["id"]
        .clone();
    let ped = p
        .ok("pedagogie_lire", Some(&admin), json!({ "id": annee }))
        .await;
    let neuvieme = ped["niveaux"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["code"] == "9")
        .unwrap()["id"]
        .clone();
    let neuf_b = p
        .ok(
            "classe_creer",
            Some(&admin),
            json!({ "annee_id": annee, "niveau_id": neuvieme, "serie_id": null,
                "libelle": "9e B" }),
        )
        .await;

    let catalogue = p
        .ok("bibliotheque_catalogue", Some(sekou), Value::Null)
        .await;
    let chapitre = catalogue["niveaux"][0]["series"][0]["matieres"][0]["parties"][0]["chapitres"]
        [0]["id"]
        .clone();
    let ajout = p
        .ok(
            "bibliotheque_contenu_ajouter",
            Some(sekou),
            json!({ "nature": "lecon", "chapitre_id": chapitre, "niveau_id": null,
                "matiere_id": null, "titre": "Exercices de la 9e A", "presentation": "",
                "examen": null, "serie_code": null, "session": null, "sujet_id": null,
                "apres_cle": null }),
        )
        .await;
    let lecon = ajout["contenu_id"].clone();
    let cible = json!({ "contenu_id": lecon, "chapitre_id": null });

    // Sékou voit les deux classes du niveau ; il ne peut cocher que la sienne.
    let pour_qui = p
        .ok("bibliotheque_classes_lire", Some(sekou), cible.clone())
        .await;
    assert_eq!(pour_qui["modifiable"], true);
    let classes = pour_qui["classes"].as_array().unwrap();
    assert_eq!(classes.len(), 2);
    let neuf_a = classes.iter().find(|c| c["libelle"] == "9e A").unwrap();
    assert_eq!(neuf_a["permise"], true);
    assert_eq!(neuf_a["choisie"], false);
    let b = classes.iter().find(|c| c["libelle"] == "9e B").unwrap();
    assert_eq!(b["permise"], false);
    let neuf_a = neuf_a["id"].clone();

    let regle = p
        .ok(
            "bibliotheque_classes_regler",
            Some(sekou),
            json!({ "contenu_id": lecon, "chapitre_id": null, "classes": [neuf_a] }),
        )
        .await;
    assert!(
        regle["classes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["id"] == neuf_a && c["choisie"] == true)
    );
    // La marque au catalogue du personnel, et à la lecture.
    let catalogue = p
        .ok("bibliotheque_catalogue", Some(&admin), Value::Null)
        .await;
    let marque = catalogue["niveaux"][0]["series"][0]["matieres"][0]["parties"][0]["chapitres"][0]
        ["contenus"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == lecon)
        .unwrap()["classes_seulement"]
        .clone();
    assert_eq!(marque, json!(["9e A"]));
    let lu = p
        .ok(
            "bibliotheque_lire",
            Some(&admin),
            json!({ "contenu_id": lecon, "vue": null }),
        )
        .await;
    assert_eq!(lu["classes_seulement"], json!(["9e A"]));

    // La direction ajoute la 9e B ; Sékou retire tout : la 9e B, qui n'est pas à lui, reste.
    p.ok(
        "bibliotheque_classes_regler",
        Some(&admin),
        json!({ "contenu_id": lecon, "chapitre_id": null, "classes": [neuf_a, neuf_b["id"]] }),
    )
    .await;
    let apres = p
        .ok(
            "bibliotheque_classes_regler",
            Some(sekou),
            json!({ "contenu_id": lecon, "chapitre_id": null, "classes": [] }),
        )
        .await;
    let choisies: Vec<&str> = apres["classes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["choisie"] == true)
        .map(|c| c["libelle"].as_str().unwrap())
        .collect();
    assert_eq!(choisies, vec!["9e B"]);

    // Une classe d'un autre niveau est refusée ; le livré reste pour tout le niveau.
    let refus = p
        .refus(
            "bibliotheque_classes_regler",
            Some(&admin),
            json!({ "contenu_id": lecon, "chapitre_id": null, "classes": ["0".repeat(26)] }),
        )
        .await;
    assert_eq!(refus.cle, "bibliotheque.pour_qui_classe_inconnue");
    // La leçon de l'école est en tête du chapitre : on prend une leçon livrée.
    let catalogue = p
        .ok("bibliotheque_catalogue", Some(&admin), Value::Null)
        .await;
    let livre = contenus_de(&catalogue)
        .into_iter()
        .find(|c| c["origine"] == "livre")
        .unwrap()["id"]
        .clone();
    let refus = p
        .refus(
            "bibliotheque_classes_regler",
            Some(&admin),
            json!({ "contenu_id": livre, "chapitre_id": null, "classes": [neuf_a] }),
        )
        .await;
    assert_eq!(refus.cle, "bibliotheque.pour_qui_livre");
}

// --- Lot 2, étape 4 : importer une leçon depuis Word --------------------------------------------

/// Un `.docx` d'essai (critère 3) : titres, gras, liste, tableau, deux images, une formule.
fn docx_d_essai(titre: &str) -> Vec<u8> {
    use std::io::Write;
    let w = "xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\" \
        xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" \
        xmlns:m=\"http://schemas.openxmlformats.org/officeDocument/2006/math\" \
        xmlns:wp=\"http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing\" \
        xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\"";
    let image = |lien: &str| {
        format!(
            "<w:p><w:r><w:drawing><wp:inline><wp:docPr id=\"1\" name=\"x\" descr=\"Figure\"/>\
             <a:graphic><a:graphicData><a:blip r:embed=\"{lien}\"/></a:graphicData></a:graphic>\
             </wp:inline></w:drawing></w:r></w:p>"
        )
    };
    let corps = format!(
        "<w:p><w:pPr><w:pStyle w:val=\"Titre1\"/></w:pPr><w:r><w:t>{titre}</w:t></w:r></w:p>\
         <w:p><w:r><w:t xml:space=\"preserve\">Un </w:t></w:r><w:r><w:rPr><w:b/></w:rPr><w:t>mot gras</w:t></w:r></w:p>\
         <w:p><w:pPr><w:numPr><w:ilvl w:val=\"0\"/><w:numId w:val=\"1\"/></w:numPr></w:pPr><w:r><w:t>premier point</w:t></w:r></w:p>\
         <w:tbl><w:tr><w:tc><w:p><w:r><w:t>a</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>b</w:t></w:r></w:p></w:tc></w:tr>\
         <w:tr><w:tc><w:p><w:r><w:t>1</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>2</w:t></w:r></w:p></w:tc></w:tr></w:tbl>\
         {}{}<w:p><m:oMathPara><m:oMath><m:r><m:t>x</m:t></m:r></m:oMath></m:oMathPara></w:p>",
        image("rIdA"),
        image("rIdB"),
    );
    let mut sortie = Vec::new();
    {
        let mut z = zip::ZipWriter::new(std::io::Cursor::new(&mut sortie));
        let options = zip::write::SimpleFileOptions::default();
        let mut mettre = |nom: &str, octets: &[u8]| {
            z.start_file(nom, options).unwrap();
            z.write_all(octets).unwrap();
        };
        mettre(
            "_rels/.rels",
            b"<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"word/document.xml\"/></Relationships>",
        );
        mettre(
            "word/document.xml",
            format!("<w:document {w}><w:body>{corps}</w:body></w:document>").as_bytes(),
        );
        mettre(
            "word/_rels/document.xml.rels",
            b"<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
              <Relationship Id=\"rIdS\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/>\
              <Relationship Id=\"rIdA\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/image\" Target=\"media/image1.png\"/>\
              <Relationship Id=\"rIdB\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/image\" Target=\"media/image2.png\"/>\
              </Relationships>",
        );
        mettre(
            "word/styles.xml",
            b"<w:styles xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:style w:type=\"paragraph\" w:styleId=\"Titre1\"><w:name w:val=\"heading 1\"/></w:style></w:styles>",
        );
        mettre("word/media/image1.png", &png(300));
        let mut autre = png(400);
        autre[300] = 9;
        mettre("word/media/image2.png", &autre);
        z.finish().unwrap();
    }
    sortie
}

/// Lot 2, critère 3 : un `.docx` devient un brouillon dont l'aperçu montre titres, liste,
/// tableau et images ; la formule devient une note pour l'auteur ; un `.doc` est refusé.
#[tokio::test]
async fn une_fiche_word_devient_un_brouillon_et_un_doc_est_refuse() {
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
    let fichiers = tempfile::tempdir().unwrap();

    // Un cours sans brouillon : l'import en commence un, parti de la version livrée.
    let chemin = ecrire(
        fichiers.path(),
        "Fiche racines.docx",
        &docx_d_essai("Racines"),
    );
    let import = p
        .ok(
            "bibliotheque_word_importer",
            Some(&admin),
            json!({ "contenu_id": id, "chemin": chemin, "version_id": null, "revision": null }),
        )
        .await;
    assert_eq!(
        import["notes"],
        json!([{ "type": "formules", "nombre": 1 }])
    );
    let brouillon = &import["ecriture"]["brouillon"];
    assert_eq!(brouillon["numero"], 1);
    let texte = brouillon["texte"].as_str().unwrap().to_owned();
    assert!(
        texte.starts_with(
            "# Racines\n\nUn **mot gras**\n\n- premier point\n\n| a | b |\n|---|---|\n| 1 | 2 |"
        ),
        "{texte}"
    );
    assert!(texte.contains("Formule à retaper ici"));

    // L'aperçu montre les titres, la liste, le tableau et les deux images rangées.
    let apercu = p
        .ok(
            "bibliotheque_apercu",
            Some(&admin),
            json!({ "contenu_id": id, "texte": texte }),
        )
        .await;
    let types: Vec<&str> = apercu["lecon"]["blocs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["type"].as_str().unwrap())
        .collect();
    for attendu in ["titre", "liste", "tableau", "note_maitre"] {
        assert!(types.contains(&attendu), "{types:?}");
    }
    assert_eq!(apercu["images"].as_array().unwrap().len(), 2);
    // La version visible reste la livrée tant que rien n'est publié.
    let visible = p
        .ok(
            "bibliotheque_lire",
            Some(&admin),
            json!({ "contenu_id": id, "vue": null }),
        )
        .await;
    assert_eq!(visible["version_ecole"], false);

    // Un second import remplace le texte du brouillon ouvert, à la révision lue.
    let chemin2 = ecrire(
        fichiers.path(),
        "Fiche 2.docx",
        &docx_d_essai("Racines, suite"),
    );
    let refus = p
        .refus(
            "bibliotheque_word_importer",
            Some(&admin),
            json!({ "contenu_id": id, "chemin": chemin2, "version_id": null, "revision": null }),
        )
        .await;
    assert_eq!(refus.cle, "bibliotheque.modifiee_ailleurs");
    let import2 = p
        .ok(
            "bibliotheque_word_importer",
            Some(&admin),
            json!({ "contenu_id": id, "chemin": chemin2,
                "version_id": brouillon["version_id"], "revision": brouillon["revision"] }),
        )
        .await;
    assert_eq!(import2["ecriture"]["brouillon"]["numero"], 1);
    assert_eq!(
        import2["ecriture"]["brouillon"]["revision"],
        brouillon["revision"].as_u64().unwrap() + 1
    );
    assert!(
        import2["ecriture"]["brouillon"]["texte"]
            .as_str()
            .unwrap()
            .starts_with("# Racines, suite")
    );

    // Un ancien fichier Word et un fichier qui n'en est pas un sont refusés.
    let mut doc = vec![0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
    doc.resize(2048, 0);
    let refus = p
        .refus(
            "bibliotheque_word_importer",
            Some(&admin),
            json!({ "contenu_id": id, "chemin": ecrire(fichiers.path(), "ancienne.doc", &doc),
                "version_id": null, "revision": null }),
        )
        .await;
    assert_eq!(refus.cle, "bibliotheque.word_ancien_format");
    let refus = p
        .refus(
            "bibliotheque_word_importer",
            Some(&admin),
            json!({ "contenu_id": id, "chemin": ecrire(fichiers.path(), "image.docx", &png(100)),
                "version_id": null, "revision": null }),
        )
        .await;
    assert_eq!(refus.cle, "bibliotheque.word_illisible");
}

// --- Échange de leçons entre écoles (lot 2, étape 5) ----------------------------------------

/// Le contenu du catalogue qui porte ce titre.
fn contenu_titre(catalogue: &Value, titre: &str) -> Value {
    contenus_de(catalogue)
        .into_iter()
        .find(|c| c["titre"] == titre)
        .unwrap_or_else(|| panic!("{titre} absent du catalogue"))
}

/// Ajoute une leçon de l'école dans un chapitre, écrit son texte et la publie.
async fn lecon_publiee(
    p: &crate::commun::Poste_,
    jeton: &gesco_services::session::Jeton,
    chapitre_id: &Value,
    titre: &str,
    texte: &str,
) -> Value {
    let ouverte = p
        .ok(
            "bibliotheque_contenu_ajouter",
            Some(jeton),
            json!({ "nature": "lecon", "chapitre_id": chapitre_id, "niveau_id": null,
                "matiere_id": null, "titre": titre, "presentation": "", "examen": null,
                "serie_code": null, "session": null, "sujet_id": null, "apres_cle": null }),
        )
        .await;
    let id = ouverte["contenu_id"].clone();
    let ecrite = p
        .ok(
            "bibliotheque_brouillon_enregistrer",
            Some(jeton),
            enregistrer(&id, &ouverte, texte),
        )
        .await;
    p.ok(
        "bibliotheque_publier",
        Some(jeton),
        version(&id, &ecrite["brouillon"]["version_id"]),
    )
    .await;
    id
}

/// Écrit et publie une version de l'école d'une leçon (livrée ou non).
async fn version_publiee(
    p: &crate::commun::Poste_,
    jeton: &gesco_services::session::Jeton,
    contenu: &Value,
    texte: &str,
) {
    let ouverte = p
        .ok(
            "bibliotheque_brouillon_commencer",
            Some(jeton),
            commencer(contenu, json!({ "type": "visible" })),
        )
        .await;
    let ecrite = p
        .ok(
            "bibliotheque_brouillon_enregistrer",
            Some(jeton),
            enregistrer(contenu, &ouverte, texte),
        )
        .await;
    p.ok(
        "bibliotheque_publier",
        Some(jeton),
        version(contenu, &ecrite["brouillon"]["version_id"]),
    )
    .await;
}

/// L'école B : même paquet, mathématiques rangées sous sa matière « Maths » (code MAT).
async fn ecole_b(
    dossier: &Path,
    livre: &[u8],
) -> (crate::commun::Poste_, gesco_services::session::Jeton) {
    deposer(dossier, "bibliotheque-mali-1.gbib", livre);
    let b = poste_vierge(dossier).await;
    b.installer().await;
    let admin = b.connecter("admin", MOT_DE_PASSE).await;
    let maths = b
        .ok(
            "matiere_enregistrer",
            Some(&admin),
            json!({ "id": null, "code": "MAT", "libelle": "Maths", "libelle_court": "Maths",
                "groupe": "scientifique", "active": true }),
        )
        .await;
    b.ok(
        "bibliotheque_correspondance_regler",
        Some(&admin),
        json!({ "discipline_code": "MATH", "niveau_code": "9", "matiere_id": maths["id"] }),
    )
    .await;
    (b, admin)
}

#[tokio::test]
async fn un_chapitre_passe_d_une_ecole_a_l_autre_en_brouillons() {
    let dossier = tempfile::tempdir().unwrap();
    let fichiers = tempfile::tempdir().unwrap();
    let livre = paquet("2026.11.1", &[RACINE, PUISSANCES]);

    // École A : une version de l'école de la racine carrée, deux leçons de l'école dans le
    // même chapitre livré, l'une avec une image et un son.
    deposer(
        &dossier.path().join("a"),
        "bibliotheque-mali-1.gbib",
        &livre,
    );
    let a = poste_vierge(&dossier.path().join("a")).await;
    a.installer().await;
    let admin_a = a.connecter("admin", MOT_DE_PASSE).await;
    let image = a
        .ok(
            "bibliotheque_image_ajouter",
            Some(&admin_a),
            json!({ "chemin": ecrire(fichiers.path(), "figure.png", &png(2000)) }),
        )
        .await;
    let voix = a
        .ok(
            "bibliotheque_son_ajouter",
            Some(&admin_a),
            json!({ "chemin": ecrire(fichiers.path(), "voix.mp3", &son(b"ID3\x04\0\0\0\0\0\0", 4000, 3)) }),
        )
        .await;
    let catalogue = a
        .ok("bibliotheque_catalogue", Some(&admin_a), Value::Null)
        .await;
    let chapitre =
        catalogue["niveaux"][0]["series"][0]["matieres"][0]["parties"][0]["chapitres"][0].clone();
    assert_eq!(chapitre["titre"], "Racines carrées");
    let racine = contenu_titre(&catalogue, "La racine carrée")["id"].clone();
    version_publiee(
        &a,
        &admin_a,
        &racine,
        "# La racine carrée\n\nVersion de l'école A.\n",
    )
    .await;
    lecon_publiee(
        &a,
        &admin_a,
        &chapitre["id"],
        "Encadrer une racine",
        &format!(
            "# Encadrer\n\n![Figure]({}) ![Écouter]({})\n",
            image["reference"].as_str().unwrap(),
            voix["reference"].as_str().unwrap()
        ),
    )
    .await;
    lecon_publiee(
        &a,
        &admin_a,
        &chapitre["id"],
        "Racines et carrés",
        "# Carrés\n\n$3^2 = 9$.\n",
    )
    .await;

    // Ce qui partira : les trois leçons de l'école, pas la leçon livrée intacte.
    let selection = json!({ "type": "chapitre", "chapitre_cle": chapitre["cle"] });
    let apercu = a
        .ok(
            "bibliotheque_export_apercu",
            Some(&admin_a),
            json!({ "selection": selection }),
        )
        .await;
    let titres: Vec<&str> = apercu["lecons"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["titre"].as_str().unwrap())
        .collect();
    assert_eq!(titres.len(), 3, "{titres:?}");
    assert!(!titres.contains(&"Les puissances"));
    assert!(apercu["nom_propose"].as_str().unwrap().ends_with(".gles"));
    let chemin = fichiers.path().join("lecons.gles");
    let fait = a
        .ok(
            "bibliotheque_exporter",
            Some(&admin_a),
            json!({ "selection": selection, "chemin": chemin }),
        )
        .await;
    assert_eq!(fait["lecons"], 3);
    assert_eq!(fait["fichiers"], 2);
    let octets = std::fs::read(&chemin).unwrap();
    let texte = String::from_utf8_lossy(&octets);
    assert!(
        !texte.contains("Administrateur") && !texte.contains("admin"),
        "aucun nom ne part"
    );

    // École B : l'aperçu dit où chaque leçon se range, puis l'import.
    let (b, admin_b) = ecole_b(&dossier.path().join("b"), &livre).await;
    let arrivee = b
        .ok(
            "bibliotheque_import_apercu",
            Some(&admin_b),
            json!({ "chemin": chemin }),
        )
        .await;
    assert!(arrivee["deja_importe_le"].is_null());
    let sortes: Vec<&str> = arrivee["lecons"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["arrivee"].as_str().unwrap())
        .collect();
    assert_eq!(
        sortes.iter().filter(|s| **s == "nouvelle").count(),
        2,
        "{sortes:?}"
    );
    assert_eq!(
        sortes.iter().filter(|s| **s == "brouillon").count(),
        1,
        "{sortes:?}"
    );
    assert!(
        arrivee["lecons"]
            .as_array()
            .unwrap()
            .iter()
            .all(|l| l["matiere"] == "Maths")
    );
    let resume = b
        .ok(
            "bibliotheque_importer",
            Some(&admin_b),
            json!({ "chemin": chemin }),
        )
        .await;
    assert_eq!(resume["brouillons"], 3);
    assert_eq!(resume["ignorees"], 0);
    let import_id = resume["import_id"].clone();

    // Rangées sous la matière de B, en brouillons ; l'image et le son sont arrivés.
    let catalogue_b = b
        .ok("bibliotheque_catalogue", Some(&admin_b), Value::Null)
        .await;
    let matiere = &catalogue_b["niveaux"][0]["series"][0]["matieres"][0];
    assert_eq!(matiere["libelle"], "Maths");
    for titre in [
        "Encadrer une racine",
        "Racines et carrés",
        "La racine carrée",
    ] {
        let c = contenu_titre(&catalogue_b, titre);
        assert_eq!(c["brouillon"], true, "{titre}");
    }
    let encadrer = contenu_titre(&catalogue_b, "Encadrer une racine")["id"].clone();
    let ecriture = b
        .ok(
            "bibliotheque_ecriture_lire",
            Some(&admin_b),
            json!({ "contenu_id": encadrer }),
        )
        .await;
    assert_eq!(ecriture["brouillon"]["mien"], true, "au nom de qui importe");
    let apercu_b = b
        .ok(
            "bibliotheque_apercu",
            Some(&admin_b),
            json!({ "contenu_id": encadrer, "texte": ecriture["brouillon"]["texte"] }),
        )
        .await;
    assert_eq!(apercu_b["images"].as_array().unwrap().len(), 1);
    assert_eq!(apercu_b["sons"][0]["disponible"], true);

    // Rien n'est visible des élèves avant publication.
    b.ok(
        "utilisateur_creer",
        Some(&admin_b),
        json!({ "identifiant": "awa", "nom_affiche": "Awa Diarra",
            "mot_de_passe_initial": "secretaire-2026", "roles": ["secretaire"] }),
    )
    .await;
    let lecteur = b.connecter("awa", "secretaire-2026").await;
    let vu = b
        .ok("bibliotheque_catalogue", Some(&lecteur), Value::Null)
        .await;
    assert!(
        !contenus_de(&vu)
            .iter()
            .any(|c| c["titre"] == "Encadrer une racine")
    );
    let racine_b = contenu_titre(&vu, "La racine carrée")["id"].clone();
    let lue = b
        .ok(
            "bibliotheque_lire",
            Some(&lecteur),
            json!({ "contenu_id": racine_b, "vue": null }),
        )
        .await;
    assert!(!lue["lecon"].to_string().contains("École A"));

    // Le même fichier une seconde fois : rien n'est ajouté, et le résumé le dit.
    let encore = b
        .ok(
            "bibliotheque_import_apercu",
            Some(&admin_b),
            json!({ "chemin": chemin }),
        )
        .await;
    assert!(encore["deja_importe_le"].is_number());
    let resume = b
        .ok(
            "bibliotheque_importer",
            Some(&admin_b),
            json!({ "chemin": chemin }),
        )
        .await;
    assert_eq!(resume["brouillons"], 0);
    assert_eq!(resume["versions_recues"], 0);
    assert_eq!(resume["ignorees"], 3);
    let catalogue_b = b
        .ok("bibliotheque_catalogue", Some(&admin_b), Value::Null)
        .await;
    assert_eq!(
        contenus_de(&catalogue_b)
            .iter()
            .filter(|c| c["titre"] == "Encadrer une racine")
            .count(),
        1
    );

    // A récrit la racine carrée : chez B, un brouillon existe déjà, elle arrive en version reçue.
    version_publiee(
        &a,
        &admin_a,
        &racine,
        "# La racine carrée\n\nSeconde version de A.\n",
    )
    .await;
    let chemin_2 = fichiers.path().join("lecons-2.gles");
    a.ok(
        "bibliotheque_exporter",
        Some(&admin_a),
        json!({ "selection": { "type": "contenus", "contenu_ids": [racine] }, "chemin": chemin_2 }),
    )
    .await;
    let resume = b
        .ok(
            "bibliotheque_importer",
            Some(&admin_b),
            json!({ "chemin": chemin_2 }),
        )
        .await;
    assert_eq!(resume["versions_recues"], 1);
    let historique = b
        .ok(
            "bibliotheque_historique",
            Some(&admin_b),
            json!({ "contenu_id": racine_b }),
        )
        .await;
    let recue = historique["recues"][0]["id"].clone();
    assert!(recue.is_string());
    let comparaison = b
        .ok(
            "bibliotheque_comparer",
            Some(&admin_b),
            json!({ "contenu_id": racine_b, "gauche": { "type": "visible" },
                "droite": { "type": "recue", "version_recue_id": recue } }),
        )
        .await;
    assert!(comparaison["ajoutees"].as_u64().unwrap() > 0);

    // « Publier les leçons reçues de cet import » : les trois brouillons du premier import.
    let imports = b
        .ok("bibliotheque_imports_lister", Some(&admin_b), Value::Null)
        .await;
    assert_eq!(imports.as_array().unwrap().len(), 3);
    let publication = b
        .ok(
            "bibliotheque_import_publier",
            Some(&admin_b),
            json!({ "import_id": import_id }),
        )
        .await;
    assert_eq!(publication["publiees"], 3);
    let vu = b
        .ok("bibliotheque_catalogue", Some(&lecteur), Value::Null)
        .await;
    assert!(
        contenus_de(&vu)
            .iter()
            .any(|c| c["titre"] == "Encadrer une racine")
    );

    // Repartir de la version reçue.
    let ouverte = b
        .ok(
            "bibliotheque_brouillon_commencer",
            Some(&admin_b),
            commencer(
                &racine_b,
                json!({ "type": "recue", "version_recue_id": recue }),
            ),
        )
        .await;
    assert!(
        ouverte["brouillon"]["texte"]
            .as_str()
            .unwrap()
            .contains("Seconde version")
    );

    // Sans la gestion de la bibliothèque, aucun import.
    let refus = b
        .refus("bibliotheque_imports_lister", Some(&lecteur), Value::Null)
        .await;
    assert_eq!(refus.cle, "erreur.permission_refusee");
}

#[tokio::test]
async fn un_chapitre_de_l_ecole_se_cree_sous_la_matiere_de_l_autre_ecole() {
    let dossier = tempfile::tempdir().unwrap();
    let fichiers = tempfile::tempdir().unwrap();
    let livre = paquet("2026.11.1", &[RACINE]);
    deposer(
        &dossier.path().join("a"),
        "bibliotheque-mali-1.gbib",
        &livre,
    );
    let a = poste_vierge(&dossier.path().join("a")).await;
    a.installer().await;
    let admin_a = a.connecter("admin", MOT_DE_PASSE).await;
    let choix = a
        .ok("bibliotheque_ecriture_choix", Some(&admin_a), Value::Null)
        .await;
    let neuvieme = choix["niveaux"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["code"] == "9")
        .unwrap()["id"]
        .clone();
    let catalogue = a
        .ok("bibliotheque_catalogue", Some(&admin_a), Value::Null)
        .await;
    let math = catalogue["niveaux"][0]["series"][0]["matieres"][0]["matiere_id"].clone();
    let chapitre = a
        .ok(
            "bibliotheque_chapitre_ajouter",
            Some(&admin_a),
            json!({ "niveau_id": neuvieme, "matiere_id": math, "series": [],
                "partie": "Algèbre", "titre": "Les équations", "apres_cle": null }),
        )
        .await;
    lecon_publiee(
        &a,
        &admin_a,
        &chapitre["id"],
        "Résoudre une équation",
        "# Équations\n\n$x + 1 = 2$.\n",
    )
    .await;
    // Un brouillon ne part jamais.
    a.ok(
        "bibliotheque_contenu_ajouter",
        Some(&admin_a),
        json!({ "nature": "lecon", "chapitre_id": chapitre["id"], "niveau_id": null,
            "matiere_id": null, "titre": "Brouillon de A", "presentation": "", "examen": null,
            "serie_code": null, "session": null, "sujet_id": null, "apres_cle": null }),
    )
    .await;
    let matiere_cle = catalogue["niveaux"][0]["series"][0]["matieres"][0]["cle"].clone();
    let chemin = fichiers.path().join("matiere.gles");
    let fait = a
        .ok(
            "bibliotheque_exporter",
            Some(&admin_a),
            json!({ "selection": { "type": "matiere", "niveau_code": "9", "matiere_cle": matiere_cle },
                "chemin": chemin }),
        )
        .await;
    assert_eq!(fait["lecons"], 1);

    let (b, admin_b) = ecole_b(&dossier.path().join("b"), &livre).await;
    let resume = b
        .ok(
            "bibliotheque_importer",
            Some(&admin_b),
            json!({ "chemin": chemin }),
        )
        .await;
    assert_eq!(resume["brouillons"], 1);
    assert_eq!(resume["chapitres"], 1);
    let catalogue_b = b
        .ok("bibliotheque_catalogue", Some(&admin_b), Value::Null)
        .await;
    let matiere = &catalogue_b["niveaux"][0]["series"][0]["matieres"][0];
    assert_eq!(matiere["libelle"], "Maths");
    let chapitres: Vec<&str> = matiere["parties"][0]["chapitres"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["titre"].as_str().unwrap())
        .collect();
    assert_eq!(
        chapitres,
        ["Racines carrées", "Les équations"],
        "à la fin de sa partie"
    );
    assert!(
        !contenus_de(&catalogue_b)
            .iter()
            .any(|c| c["titre"] == "Brouillon de A")
    );
}

#[tokio::test]
async fn un_fichier_abime_est_refuse_et_un_fichier_piege_arrive_nettoye() {
    use gesco_core::bibliotheque::paquet::empreinte;
    use gesco_core::bibliotheque::{
        ChapitreEchange, LeconEchange, ManifesteEchange, fabriquer_echange, nom_fichier_echange,
        nom_texte_echange,
    };

    let dossier = tempfile::tempdir().unwrap();
    let fichiers = tempfile::tempdir().unwrap();
    let livre = paquet("2026.11.1", &[RACINE]);
    let (b, admin_b) = ecole_b(&dossier.path().join("b"), &livre).await;

    let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><script>alert(1)</script>\
        <circle r=\"4\" onload=\"vol()\"/></svg>"
        .to_vec();
    let empreinte_svg = empreinte(&svg);
    let texte = format!(
        "# Piège\n\n<script>alert(1)</script>\n\nVoir [ici](https://exemple.org).\n\n![Figure](image:{empreinte_svg})\n"
    );
    let manifeste = ManifesteEchange {
        date: "2026-11-16".to_owned(),
        gesco: "2.0.0".to_owned(),
        lecons: vec![LeconEchange {
            cle: "ecole/01JPIEGE000000000000000000".to_owned(),
            livre: false,
            nature: NatureContenu::Lecon,
            titre: "Leçon piégée".to_owned(),
            presentation: String::new(),
            examen: None,
            serie: None,
            session: None,
            lie: None,
            chapitre: ChapitreEchange {
                cle: "ecole/01JCHAPITREPIEGE0000000000".to_owned(),
                livre: false,
                niveau: "9".to_owned(),
                discipline: "MATH".to_owned(),
                discipline_libelle: "Mathématiques".to_owned(),
                series: vec![],
                partie: String::new(),
                titre: "Pièges".to_owned(),
            },
            texte: nom_texte_echange(0),
            joints: vec![],
        }],
        noms: Default::default(),
        fichiers: vec![],
    };
    let octets = fabriquer_echange(
        &manifeste,
        &[
            (nom_texte_echange(0), texte.into_bytes()),
            (nom_fichier_echange(&empreinte_svg), svg),
        ],
    )
    .unwrap();

    // Un octet changé : refusé entier.
    let mut abime = octets.clone();
    let milieu = abime.len() / 2;
    abime[milieu] ^= 0x01;
    let refus = b
        .refus(
            "bibliotheque_import_apercu",
            Some(&admin_b),
            json!({ "chemin": ecrire(fichiers.path(), "abime.gles", &abime) }),
        )
        .await;
    assert_eq!(refus.cle, "bibliotheque.echange_abime");
    let refus = b
        .refus(
            "bibliotheque_importer",
            Some(&admin_b),
            json!({ "chemin": ecrire(fichiers.path(), "abime.gles", &abime) }),
        )
        .await;
    assert_eq!(refus.cle, "bibliotheque.echange_abime");

    // Le fichier piégé arrive filtré : le HTML reste du texte, le lien perd sa cible, le SVG est
    // nettoyé et le texte appelle l'image nettoyée.
    let resume = b
        .ok(
            "bibliotheque_importer",
            Some(&admin_b),
            json!({ "chemin": ecrire(fichiers.path(), "piege.gles", &octets) }),
        )
        .await;
    assert_eq!(resume["brouillons"], 1);
    let catalogue = b
        .ok("bibliotheque_catalogue", Some(&admin_b), Value::Null)
        .await;
    let piege = contenu_titre(&catalogue, "Leçon piégée")["id"].clone();
    let ecriture = b
        .ok(
            "bibliotheque_ecriture_lire",
            Some(&admin_b),
            json!({ "contenu_id": piege }),
        )
        .await;
    let texte_recu = ecriture["brouillon"]["texte"].as_str().unwrap().to_owned();
    assert!(
        !texte_recu.contains(&empreinte_svg),
        "l'appel suit l'image nettoyée"
    );
    let apercu = b
        .ok(
            "bibliotheque_apercu",
            Some(&admin_b),
            json!({ "contenu_id": piege, "texte": texte_recu }),
        )
        .await;
    assert!(!apercu["lecon"].to_string().contains("exemple.org"));
    let image = base64_vers_octets(&apercu["images"][0]["donnees"]);
    let image = String::from_utf8(image).unwrap();
    assert!(!image.contains("script") && !image.contains("onload"));
    assert!(image.contains("circle"));
}

// ---------------------------------------------------------------------------------------------
// Portail des élèves (phase 12, lot 4, étape 3) : ce que lit l'élève.
// ---------------------------------------------------------------------------------------------

/// Inscrit un élève dans une classe et rend son identifiant.
async fn eleve_dans(
    p: &crate::commun::Poste_,
    admin: &gesco_services::session::Jeton,
    classe: &Value,
    prenom: &str,
    matricule: &str,
) -> String {
    // Les essais donnent leurs propres matricules : l'école les saisit elle-même.
    p.ok(
        "parametre_definir",
        Some(admin),
        json!({ "cle": "matricule.saisie_manuelle",
            "valeur": { "type": "booleen", "valeur": true } }),
    )
    .await;
    let fiche = p
        .ok(
            "eleve_creer",
            Some(admin),
            json!({ "identite": { "nom": "Diarra", "prenoms": prenom, "sexe": "feminin",
                "date_naissance": "2011-03-02", "date_naissance_estimee": false,
                "lieu_naissance": "Kati", "nationalite": "Malienne", "adresse": "",
                "telephones": [] },
                "matricule_interne": matricule, "matricule_national": format!("N{matricule}"),
                "acte_nature": "acte", "acte_numero": "", "acte_date": "", "ecole_origine": "" }),
        )
        .await;
    // Un parent joignable et le matricule national : l'inscription se valide même en 9e.
    p.ok(
        "responsable_rattacher",
        Some(admin),
        json!({ "eleve_id": fiche["eleve"]["id"], "responsable_id": null,
            "saisie": { "identite": { "nom": "Coulibaly", "prenoms": "Mariam", "sexe": "feminin",
                "date_naissance": "", "date_naissance_estimee": false, "lieu_naissance": "",
                "nationalite": "", "adresse": "", "telephones": ["76 12 34 56"] },
                "profession": "", "employeur": "" },
            "qualites": { "lien": "mere", "est_responsable_legal": true, "est_payeur": true,
                "recoit_notifications": true, "vient_chercher": false, "ordre_priorite": 1 } }),
    )
    .await;
    p.ok(
        "inscription_creer",
        Some(admin),
        json!({ "eleve_id": fiche["eleve"]["id"], "classe_id": classe, "date_inscription": "",
            "redoublant": false, "etablissement_origine": "" }),
    )
    .await;
    let validation = p
        .ok(
            "inscriptions_valider_classe",
            Some(admin),
            json!({ "id": classe }),
        )
        .await;
    assert!(
        validation["refusees"].as_array().unwrap().is_empty(),
        "{validation}"
    );
    fiche["eleve"]["id"].as_str().unwrap().to_owned()
}

/// L'élève entre avec son code ; rend son jeton.
async fn entrer_au_portail(p: &crate::commun::Poste_, eleve_id: &str, matricule: &str) -> String {
    use gesco_services::portail::entree::{DemandeEntrer, Entree};
    let code = p
        .application
        .portail_code_pour_essai(eleve_id)
        .await
        .unwrap();
    match p
        .application
        .portail_entrer(&DemandeEntrer {
            matricule: matricule.to_owned(),
            code,
            pas_mon_telephone: false,
        })
        .await
        .unwrap()
    {
        Some(Entree::Entre { jeton, .. }) => jeton,
        _ => panic!("{matricule} devait entrer"),
    }
}

/// Titres des contenus du catalogue de l'élève.
fn titres_eleve(catalogue: &gesco_services::portail::lecture::CatalogueEleve) -> Vec<String> {
    catalogue
        .matieres
        .iter()
        .flat_map(|m| m.parties.iter())
        .flat_map(|p| p.chapitres.iter())
        .flat_map(|c| c.contenus.iter())
        .map(|c| c.titre.clone())
        .collect()
}

/// Publie un texte de l'école par la direction : brouillon ouvert, enregistré, soumis, publié.
async fn publier_par_la_direction(
    p: &crate::commun::Poste_,
    admin: &gesco_services::session::Jeton,
    id: &Value,
    ouverte: &Value,
    texte: &str,
) {
    let apres = p
        .ok(
            "bibliotheque_brouillon_enregistrer",
            Some(admin),
            enregistrer(id, ouverte, texte),
        )
        .await;
    let v = apres["brouillon"]["version_id"].clone();
    p.ok("bibliotheque_soumettre", Some(admin), version(id, &v))
        .await;
    p.ok("bibliotheque_publier", Some(admin), version(id, &v))
        .await;
}

#[tokio::test]
async fn l_eleve_ne_lit_que_les_versions_visibles_de_sa_classe() {
    use gesco_services::portail::entree::DemandeLireSansCode;
    use gesco_services::portail::lecture::{FichierLuPortail, ReponseLecture};

    let dossier = tempfile::tempdir().unwrap();
    deposer(
        dossier.path(),
        "bibliotheque-mali-1.gbib",
        &paquet_avec_faits(
            "2026.11.1",
            &[RACINE, PUISSANCES],
            Some("- Fait à vérifier : la racine de 16."),
        ),
    );
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let _maitres = maitres_de_math(&p, &admin, &[("sekou", "Traoré", "Sékou")]).await;
    let annees = p.ok("annees_lister", Some(&admin), Value::Null).await;
    let annee = annees
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["libelle"] == "2026-2027")
        .unwrap()["id"]
        .clone();
    // Une grille de frais en vigueur : sans elle, aucune inscription ne se valide.
    crate::commun::poser_grille(&p, &admin, &annee).await;
    let ped = p
        .ok("pedagogie_lire", Some(&admin), json!({ "id": annee }))
        .await;
    let neuvieme = ped["niveaux"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["code"] == "9")
        .unwrap()["id"]
        .clone();
    let classes = p
        .ok("classes_lister", Some(&admin), json!({ "annee_id": annee }))
        .await;
    let neuf_a = classes
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["classe"]["libelle"] == "9e A")
        .unwrap()["classe"]["id"]
        .clone();
    let neuf_b = p
        .ok(
            "classe_creer",
            Some(&admin),
            json!({ "annee_id": annee, "niveau_id": neuvieme, "serie_id": null,
                "libelle": "9e B" }),
        )
        .await["id"]
        .clone();
    let adama = eleve_dans(&p, &admin, &neuf_a, "Adama", "E-911").await;
    let binta = eleve_dans(&p, &admin, &neuf_b, "Binta", "E-912").await;

    // Une leçon de l'école, publiée, réservée à la 9e A.
    let catalogue = p
        .ok("bibliotheque_catalogue", Some(&admin), Value::Null)
        .await;
    let chapitre = catalogue["niveaux"][0]["series"][0]["matieres"][0]["parties"][0]["chapitres"]
        [0]["id"]
        .clone();
    let ajout = p
        .ok(
            "bibliotheque_contenu_ajouter",
            Some(&admin),
            json!({ "nature": "lecon", "chapitre_id": chapitre, "niveau_id": null,
                "matiere_id": null, "titre": "Exercices de la 9e A", "presentation": "",
                "examen": null, "serie_code": null, "session": null, "sujet_id": null,
                "apres_cle": null }),
        )
        .await;
    let exercices = ajout["contenu_id"].clone();
    publier_par_la_direction(
        &p,
        &admin,
        &exercices,
        &ajout,
        "# Exercices de la 9e A\n\nCalcule $\\sqrt{49}$.\n",
    )
    .await;
    p.ok(
        "bibliotheque_classes_regler",
        Some(&admin),
        json!({ "contenu_id": exercices, "chapitre_id": null, "classes": [neuf_a] }),
    )
    .await;
    // Un brouillon en cours sur la leçon livrée : jamais au portail.
    let racine = contenus_de(&catalogue)
        .into_iter()
        .find(|c| c["titre"] == "La racine carrée")
        .unwrap()["id"]
        .clone();
    let brouillon = p
        .ok(
            "bibliotheque_brouillon_commencer",
            Some(&admin),
            commencer(&racine, json!({ "type": "livree" })),
        )
        .await;
    p.ok(
        "bibliotheque_brouillon_enregistrer",
        Some(&admin),
        enregistrer(
            &racine,
            &brouillon,
            "# La racine carrée\n\nBROUILLON PAS ENCORE PUBLIÉ.\n",
        ),
    )
    .await;

    let a = entrer_au_portail(&p, &adama, "E-911").await;
    let b = entrer_au_portail(&p, &binta, "E-912").await;
    let sans_code = match p
        .application
        .portail_lire_sans_code(&DemandeLireSansCode {
            niveau_id: neuvieme.as_str().unwrap().to_owned(),
            serie_id: None,
        })
        .await
        .unwrap()
    {
        Some((jeton, _)) => jeton,
        None => panic!("la 9e se lit sans code"),
    };

    let lire = |jeton: String| {
        let p = &p;
        async move {
            match p.application.portail_catalogue(Some(&jeton)).await.unwrap() {
                ReponseLecture::Lu(c) => c,
                autre => panic!("catalogue : {autre:?}"),
            }
        }
    };
    // Sa classe seulement.
    let cat_a = lire(a.clone()).await;
    let titres_a = titres_eleve(&cat_a);
    assert!(titres_a.contains(&"Exercices de la 9e A".to_owned()));
    assert!(titres_a.contains(&"La racine carrée".to_owned()));
    assert!(titres_a.contains(&"Les puissances".to_owned()));
    assert_eq!(cat_a.matieres[0].lecons, 3);
    let titres_b = titres_eleve(&lire(b.clone()).await);
    assert!(!titres_b.contains(&"Exercices de la 9e A".to_owned()));
    assert!(titres_b.contains(&"La racine carrée".to_owned()));
    let titres_sans = titres_eleve(&lire(sans_code.clone()).await);
    assert!(!titres_sans.contains(&"Exercices de la 9e A".to_owned()));
    assert!(titres_sans.contains(&"Les puissances".to_owned()));

    // Le contenu réservé n'est pas lu par la 9e B, même par son adresse.
    let ex_id = exercices.as_str().unwrap();
    assert!(matches!(
        p.application
            .portail_contenu(Some(&b), ex_id)
            .await
            .unwrap(),
        ReponseLecture::Introuvable
    ));
    let ReponseLecture::Lu(ex) = p
        .application
        .portail_contenu(Some(&a), ex_id)
        .await
        .unwrap()
    else {
        panic!("la 9e A lit ses exercices");
    };
    assert_eq!(ex.titre, "Exercices de la 9e A");

    // La version visible, sans brouillon, sans faits, sans note pour le maître.
    let racine_id = racine.as_str().unwrap();
    let ReponseLecture::Lu(lu) = p
        .application
        .portail_contenu(Some(&a), racine_id)
        .await
        .unwrap()
    else {
        panic!("la racine se lit");
    };
    let texte = serde_json::to_string(&lu).unwrap();
    assert!(texte.contains("16"), "{texte}");
    for interdit in [
        "BROUILLON",
        "Fait à vérifier",
        "brouillon",
        "soumise",
        "faits",
    ] {
        assert!(!texte.contains(interdit), "{interdit} dans {texte}");
    }
    assert_eq!(lu.version.len(), 64);

    // Sans session : l'élève doit entrer de nouveau.
    assert!(matches!(
        p.application.portail_catalogue(None).await.unwrap(),
        ReponseLecture::SansSession
    ));
    assert!(matches!(
        p.application
            .portail_catalogue(Some(&"f".repeat(64)))
            .await
            .unwrap(),
        ReponseLecture::SansSession
    ));
    // Un fichier qu'aucun contenu visible n'appelle n'est pas servi.
    assert!(matches!(
        p.application
            .portail_fichier(Some(&a), &"a".repeat(64), None)
            .await
            .unwrap(),
        ReponseLecture::Introuvable
    ));
    assert!(matches!(
        p.application
            .portail_fichier(Some(&a), "../../gesco.db", None)
            .await
            .unwrap(),
        ReponseLecture::Introuvable
    ));
    let _ = FichierLuPortail::Absent;

    // Visible tout de suite : la direction publie une version, l'élève qui recharge la voit.
    let ecriture = p
        .ok(
            "bibliotheque_ecriture_lire",
            Some(&admin),
            json!({ "contenu_id": racine }),
        )
        .await;
    publier_par_la_direction(
        &p,
        &admin,
        &racine,
        &ecriture,
        "# La racine carrée\n\nLa racine carrée de $81$ est $9$.\n",
    )
    .await;
    let ReponseLecture::Lu(relu) = p
        .application
        .portail_contenu(Some(&a), racine_id)
        .await
        .unwrap()
    else {
        panic!("la racine se relit");
    };
    let texte = serde_json::to_string(&relu).unwrap();
    assert!(texte.contains("81"), "{texte}");
    assert_ne!(relu.version, lu.version);

    // Un contenu retiré disparaît au rechargement.
    p.ok(
        "bibliotheque_retirer",
        Some(&admin),
        retrait("contenu", PUISSANCES.0, None),
    )
    .await;
    let titres_a = titres_eleve(&lire(a.clone()).await);
    assert!(!titres_a.contains(&"Les puissances".to_owned()));

    // Le PDF de la leçon complète : fabriqué une fois, gardé hors de la sauvegarde.
    let ReponseLecture::Lu((nom, pdf)) = p
        .application
        .portail_pdf(Some(&a), racine_id)
        .await
        .unwrap()
    else {
        panic!("le PDF se fabrique");
    };
    assert_eq!(nom, "La racine carrée.pdf");
    assert!(pdf.starts_with(b"%PDF"));
    let gardes = std::fs::read_dir(dossier.path().join("portail-pdf"))
        .unwrap()
        .count();
    assert_eq!(gardes, 1);
    let ReponseLecture::Lu((_, encore)) = p
        .application
        .portail_pdf(Some(&a), racine_id)
        .await
        .unwrap()
    else {
        panic!("le PDF se relit");
    };
    assert_eq!(encore, pdf);
    // La direction coupe le PDF : il n'est plus servi.
    let etat = p.ok("portail_etat", Some(&admin), Value::Null).await;
    let mut reglages = etat["reglages"].clone();
    reglages["pdf"] = json!(false);
    p.ok("portail_regler", Some(&admin), reglages).await;
    assert!(matches!(
        p.application
            .portail_pdf(Some(&a), racine_id)
            .await
            .unwrap(),
        ReponseLecture::Introuvable
    ));
}

/// Règle un paramètre par la direction.
async fn regler(
    p: &crate::commun::Poste_,
    admin: &gesco_services::session::Jeton,
    cle: &str,
    valeur: Value,
) {
    p.ok(
        "parametre_definir",
        Some(admin),
        json!({ "cle": cle, "valeur": valeur }),
    )
    .await;
}

/// Phase 12, lot 4, étape 5 (SPEC_PHASE_12_LOT_4 §0.3 critère 6, §6, §10) : un élève entré avec
/// son code ouvre une leçon et la finit ; le maître de la classe le voit au suivi dès le passage
/// de la file (cinq secondes) ; un élève sans code n'est jamais compté ; suivi éteint, rien ne
/// s'écrit et rien ne se montre ; « par classe seulement » cache les noms ; le maître n'a que ses
/// classes et ses matières ; « Les maîtres ont leur compte » sans la saisie lui ouvre le suivi
/// mais pas les notes.
#[tokio::test]
async fn le_maitre_voit_au_suivi_ce_que_ses_eleves_ont_ouvert_et_fini() {
    use gesco_services::portail::entree::DemandeLireSansCode;
    use gesco_services::portail::lecture::ReponseLecture;
    use gesco_services::portail::suivi::{DemandeAvancement, GesteAvancement};

    let dossier = tempfile::tempdir().unwrap();
    deposer(
        dossier.path(),
        "bibliotheque-mali-1.gbib",
        &paquet("2026.11.1", &[RACINE, PUISSANCES]),
    );
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let _maitres = maitres_de_math(&p, &admin, &[("sekou", "Traoré", "Sékou")]).await;
    let annees = p.ok("annees_lister", Some(&admin), Value::Null).await;
    let annee = annees
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["libelle"] == "2026-2027")
        .unwrap()["id"]
        .clone();
    // Une grille de frais en vigueur : sans elle, aucune inscription ne se valide.
    crate::commun::poser_grille(&p, &admin, &annee).await;
    let ped = p
        .ok("pedagogie_lire", Some(&admin), json!({ "id": annee }))
        .await;
    let neuvieme = ped["niveaux"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["code"] == "9")
        .unwrap()["id"]
        .clone();
    let classes = p
        .ok("classes_lister", Some(&admin), json!({ "annee_id": annee }))
        .await;
    let neuf_a = classes
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["classe"]["libelle"] == "9e A")
        .unwrap()["classe"]["id"]
        .clone();
    let neuf_b = p
        .ok(
            "classe_creer",
            Some(&admin),
            json!({ "annee_id": annee, "niveau_id": neuvieme, "serie_id": null,
                "libelle": "9e B" }),
        )
        .await["id"]
        .clone();
    let adama = eleve_dans(&p, &admin, &neuf_a, "Adama", "E-921").await;
    let _awa = eleve_dans(&p, &admin, &neuf_a, "Awa", "E-922").await;
    let _binta = eleve_dans(&p, &admin, &neuf_b, "Binta", "E-923").await;
    let catalogue = p
        .ok("bibliotheque_catalogue", Some(&admin), Value::Null)
        .await;
    let racine = contenus_de(&catalogue)
        .into_iter()
        .find(|c| c["titre"] == "La racine carrée")
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let a = entrer_au_portail(&p, &adama, "E-921").await;
    let sans_code = match p
        .application
        .portail_lire_sans_code(&DemandeLireSansCode {
            niveau_id: neuvieme.as_str().unwrap().to_owned(),
            serie_id: None,
        })
        .await
        .unwrap()
    {
        Some((jeton, _)) => jeton,
        None => panic!("la 9e se lit sans code"),
    };
    let ReponseLecture::Lu(lu) = p
        .application
        .portail_contenu(Some(&a), &racine)
        .await
        .unwrap()
    else {
        panic!("la racine se lit");
    };
    let geste = |g: GesteAvancement| DemandeAvancement {
        contenu_id: racine.clone(),
        version: lu.version.clone(),
        geste: g,
    };
    for (jeton, g) in [
        (&a, GesteAvancement::Ouvert),
        (&a, GesteAvancement::Fini),
        (&sans_code, GesteAvancement::Ouvert),
        (&sans_code, GesteAvancement::Fini),
    ] {
        assert!(matches!(
            p.application
                .portail_avancement(Some(jeton), &geste(g))
                .await
                .unwrap(),
            ReponseLecture::Lu(())
        ));
    }
    // Sans session, rien.
    assert!(matches!(
        p.application
            .portail_avancement(None, &geste(GesteAvancement::Ouvert))
            .await
            .unwrap(),
        ReponseLecture::SansSession
    ));
    // Avant même l'écriture, « Mon avancement » le compte.
    let ReponseLecture::Lu(mon) = p
        .application
        .portail_mon_avancement(Some(&a))
        .await
        .unwrap()
    else {
        panic!("mon avancement se lit");
    };
    assert_eq!(mon.finies, vec![racine.clone()]);
    assert_eq!(
        mon.recentes.first().map(|r| r.id.as_str()),
        Some(racine.as_str())
    );
    let math = mon.matieres.first().unwrap();
    assert_eq!((math.finies, math.visibles), (1, 2));
    // Un élève sans code n'a pas d'avancement.
    assert!(matches!(
        p.application
            .portail_mon_avancement(Some(&sans_code))
            .await
            .unwrap(),
        ReponseLecture::Introuvable
    ));
    // Le passage de la file écrit une seule ligne : celle d'Adama.
    assert_eq!(p.application.portail_avancement_ecrire().await, 1);
    assert_eq!(p.application.portail_avancement_ecrire().await, 0);

    // La direction voit toutes les classes, par leçon et par élève.
    let suivi = p
        .ok(
            "bibliotheque_suivi_lire",
            Some(&admin),
            json!({ "classe_id": neuf_a }),
        )
        .await;
    assert_eq!(suivi["actif"], true);
    assert_eq!(suivi["classes"].as_array().unwrap().len(), 2);
    assert_eq!(suivi["eleves"], 2);
    let ligne = suivi["lecons"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["titre"] == "La racine carrée")
        .unwrap()
        .clone();
    assert_eq!(
        (ligne["ouvertes"].clone(), ligne["finies"].clone()),
        (json!(1), json!(1))
    );
    let par_eleve = suivi["par_eleve"].as_array().unwrap();
    assert_eq!(par_eleve.len(), 2);
    let d_adama = par_eleve
        .iter()
        .find(|e| e["nom"].as_str().unwrap().contains("Adama"))
        .unwrap();
    assert_eq!(d_adama["finies"], 1);
    assert!(d_adama["derniere_visite"].is_i64());
    let d_awa = par_eleve
        .iter()
        .find(|e| e["nom"].as_str().unwrap().contains("Awa"))
        .unwrap();
    assert_eq!(d_awa["finies"], 0);
    assert!(d_awa["derniere_visite"].is_null());
    // Imprimable.
    let imprime = p
        .ok(
            "bibliotheque_suivi_imprimer",
            Some(&admin),
            json!({ "classe_id": neuf_a }),
        )
        .await;
    assert!(!imprime["pdf"].as_str().unwrap().is_empty());

    // Le compte du maître sans la saisie des notes.
    let non = json!({ "type": "booleen", "valeur": false });
    let oui = json!({ "type": "booleen", "valeur": true });
    regler(&p, &admin, "notes.saisie_maitres", non.clone()).await;
    regler(&p, &admin, "maitres.comptes", oui.clone()).await;
    let sekou = p.connecter("sekou", "enseignant-2026").await;
    let du_maitre = p
        .ok("bibliotheque_suivi_lire", Some(&sekou), json!({}))
        .await;
    let siennes: Vec<&str> = du_maitre["classes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["libelle"].as_str().unwrap())
        .collect();
    assert_eq!(siennes, vec!["9e A"]);
    assert_eq!(du_maitre["matieres"].as_array().unwrap().len(), 1);
    assert_eq!(
        p.refus(
            "bibliotheque_suivi_lire",
            Some(&sekou),
            json!({ "classe_id": neuf_b }),
        )
        .await
        .cle,
        "erreur.portee_refusee"
    );
    assert_eq!(
        p.refus(
            "evaluations_lister",
            Some(&sekou),
            json!({ "classe_id": neuf_a }),
        )
        .await
        .cle,
        "maitre.saisie_fermee"
    );
    p.ok("bibliotheque_catalogue", Some(&sekou), Value::Null)
        .await;

    // Par classe seulement : des nombres, sans noms.
    regler(
        &p,
        &admin,
        "portail.suivi_grain",
        json!({ "type": "texte", "valeur": "classe" }),
    )
    .await;
    let par_classe = p
        .ok("bibliotheque_suivi_lire", Some(&sekou), json!({}))
        .await;
    assert!(par_classe["par_eleve"].as_array().unwrap().is_empty());
    assert!(!par_classe["lecons"].as_array().unwrap().is_empty());

    // Suivi éteint : rien ne s'écrit, rien ne se montre ; ce qui est écrit reste.
    regler(&p, &admin, "bibliotheque.suivi_eleves", non.clone()).await;
    let eteint = p
        .ok("bibliotheque_suivi_lire", Some(&sekou), json!({}))
        .await;
    assert_eq!(eteint["actif"], false);
    assert!(eteint["lecons"].as_array().unwrap().is_empty());
    p.application
        .portail_avancement(Some(&a), &geste(GesteAvancement::Ouvert))
        .await
        .unwrap();
    assert_eq!(p.application.portail_avancement_ecrire().await, 0);
    assert!(matches!(
        p.application
            .portail_mon_avancement(Some(&a))
            .await
            .unwrap(),
        ReponseLecture::Introuvable
    ));
    regler(&p, &admin, "bibliotheque.suivi_eleves", oui).await;
    let rallume = p
        .ok(
            "bibliotheque_suivi_lire",
            Some(&admin),
            json!({ "classe_id": neuf_a }),
        )
        .await;
    assert!(
        rallume["lecons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|l| l["titre"] == "La racine carrée" && l["finies"] == 1)
    );

    // Ni compte ni saisie : le maître n'entre plus.
    regler(&p, &admin, "maitres.comptes", non).await;
    let refus = p
        .refus(
            "connexion",
            None,
            json!({ "identifiant": "sekou", "mot_de_passe": "enseignant-2026" }),
        )
        .await;
    assert_eq!(refus.cle, "connexion.maitres_non_admis");
}

/// Phase 12, lot 3, étape 4 (SPEC_PHASE_12_LOT_3 §0.3 critères 4 et 6, §5, §7) : un élève entré
/// avec son code passe le quiz de « La racine carrée » ; le téléphone reçoit les bonnes réponses
/// et les explications ; le poste principal corrige de nouveau et n'accepte jamais le score du
/// téléphone ; la tentative s'écrit au passage de la file ; un élève sans code passe le quiz et
/// rien ne s'écrit ; une version qui ne correspond plus fait oublier la tentative ; réviser le
/// chapitre donne toutes ses questions quand il en a moins de dix ; un quiz abandonné n'envoie
/// rien.
#[tokio::test]
async fn l_eleve_passe_le_quiz_et_seule_sa_tentative_corrigee_s_ecrit() {
    use gesco_services::portail::entree::DemandeLireSansCode;
    use gesco_services::portail::lecture::ReponseLecture;
    use gesco_services::portail::quiz::{DemandeTentative, ReponseDonnee};
    use sqlx::Connection as _;

    let dossier = tempfile::tempdir().unwrap();
    deposer(
        dossier.path(),
        "bibliotheque-mali-1.gbib",
        &paquet("2026.11.1", &[RACINE_QUIZ, PUISSANCES]),
    );
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let annees = p.ok("annees_lister", Some(&admin), Value::Null).await;
    let annee = annees
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["libelle"] == "2026-2027")
        .unwrap()["id"]
        .clone();
    crate::commun::poser_grille(&p, &admin, &annee).await;
    let ped = p
        .ok("pedagogie_lire", Some(&admin), json!({ "id": annee }))
        .await;
    let neuvieme = ped["niveaux"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["code"] == "9")
        .unwrap()["id"]
        .clone();
    let classes = p
        .ok("classes_lister", Some(&admin), json!({ "annee_id": annee }))
        .await;
    let neuf_a = classes
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["classe"]["libelle"] == "9e A")
        .unwrap()["classe"]["id"]
        .clone();
    let adama = eleve_dans(&p, &admin, &neuf_a, "Adama", "E-931").await;
    let a = entrer_au_portail(&p, &adama, "E-931").await;
    let sans_code = match p
        .application
        .portail_lire_sans_code(&DemandeLireSansCode {
            niveau_id: neuvieme.as_str().unwrap().to_owned(),
            serie_id: None,
        })
        .await
        .unwrap()
    {
        Some((jeton, _)) => jeton,
        None => panic!("la 9e se lit sans code"),
    };
    let ReponseLecture::Lu(catalogue) = p.application.portail_catalogue(Some(&a)).await.unwrap()
    else {
        panic!("le catalogue se lit");
    };
    let chapitre = catalogue
        .matieres
        .iter()
        .flat_map(|m| m.parties.iter())
        .flat_map(|pa| pa.chapitres.iter())
        .find(|c| c.contenus.iter().any(|x| x.titre == "La racine carrée"))
        .unwrap();
    let racine = chapitre
        .contenus
        .iter()
        .find(|x| x.titre == "La racine carrée")
        .unwrap()
        .id
        .clone();

    // La leçon dit qu'elle a un quiz ; la page de la leçon ne montre pas les réponses.
    let ReponseLecture::Lu(lecon) = p
        .application
        .portail_contenu(Some(&a), &racine)
        .await
        .unwrap()
    else {
        panic!("la racine se lit");
    };
    assert_eq!(lecon.questions, 2);
    // Le quiz, avec bonnes réponses et explications ; sans code aussi.
    let ReponseLecture::Lu(quiz) = p.application.portail_quiz(Some(&a), &racine).await.unwrap()
    else {
        panic!("le quiz se lit");
    };
    assert_eq!(quiz.questions.len(), 2);
    assert_eq!(quiz.questions[0].question.bonnes, vec!["A"]);
    assert!(!quiz.questions[0].question.explication.is_empty());
    assert!(matches!(
        p.application
            .portail_quiz(Some(&sans_code), &racine)
            .await
            .unwrap(),
        ReponseLecture::Lu(_)
    ));
    // Une leçon sans quiz n'en a pas.
    let puissances = chapitre
        .contenus
        .iter()
        .find(|x| x.titre == "Les puissances")
        .map(|x| x.id.clone());
    if let Some(id) = puissances {
        assert!(matches!(
            p.application.portail_quiz(Some(&a), &id).await.unwrap(),
            ReponseLecture::Introuvable
        ));
    }
    // Réviser le chapitre : moins de dix questions, toutes viennent.
    let ReponseLecture::Lu(revision) = p
        .application
        .portail_revision(Some(&a), &chapitre.id)
        .await
        .unwrap()
    else {
        panic!("la révision se lit");
    };
    assert_eq!(revision.questions.len(), 2);
    assert!(revision.questions.iter().all(|q| q.contenu_id == racine));

    // Le téléphone envoie ses réponses : une juste (« 3.5 » pour « 3,5 »), une fausse.
    let version = quiz.questions[0].version.clone();
    let tentative = |reponses: [&str; 2], version: &str| DemandeTentative {
        contenu_id: Some(racine.clone()),
        chapitre_id: None,
        commencee_le: Some(0),
        reponses: reponses
            .iter()
            .zip([1u32, 2])
            .map(|(r, n)| ReponseDonnee {
                contenu_id: racine.clone(),
                version: version.to_owned(),
                numero: n,
                reponse: (*r).to_owned(),
            })
            .collect(),
    };
    for (jeton, reponses, v) in [
        (&a, ["B", "3.5"], version.as_str()),
        (&sans_code, ["A", "3,5"], version.as_str()),
        (&a, ["A", "3,5"], "une-autre-version"),
    ] {
        assert!(matches!(
            p.application
                .portail_tentative(Some(jeton), &tentative(reponses, v))
                .await
                .unwrap(),
            ReponseLecture::Lu(())
        ));
    }
    // Une seule tentative s'écrit : celle d'Adama dans la bonne version.
    assert_eq!(p.application.portail_avancement_ecrire().await, 1);
    let mut cnx = crate::commun::base_brute(dossier.path(), p.trousseau.clone()).await;
    let lignes: Vec<(i64, i64, String, String)> =
        sqlx::query_as("SELECT score, sur, eleve_id, empreinte_version FROM tentative_quiz")
            .fetch_all(&mut cnx)
            .await
            .unwrap();
    assert_eq!(
        lignes,
        vec![(1, 2, adama.clone(), version.clone())],
        "le score vient du poste principal"
    );
    let reponses: Vec<(i64, String, i64)> =
        sqlx::query_as("SELECT numero, reponse, juste FROM reponse_quiz ORDER BY numero")
            .fetch_all(&mut cnx)
            .await
            .unwrap();
    assert_eq!(
        reponses,
        vec![(1, "B".to_owned(), 0), (2, "3.5".to_owned(), 1)]
    );
    let notes_avant: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM note")
        .fetch_one(&mut cnx)
        .await
        .unwrap();
    cnx.close().await.unwrap();

    // Étape 6 : le suivi du maître montre le quiz ; « Mon avancement », le dernier score.
    let quiz_de_la_racine = |suivi: &Value| {
        suivi["quiz"]
            .as_array()
            .unwrap()
            .iter()
            .find(|q| q["titre"] == "La racine carrée")
            .cloned()
    };
    let suivi = p
        .ok(
            "bibliotheque_suivi_lire",
            Some(&admin),
            json!({ "classe_id": neuf_a }),
        )
        .await;
    let q = quiz_de_la_racine(&suivi).expect("le quiz paraît au suivi");
    assert_eq!(q["eleves"], 1);
    assert_eq!(q["questions"], 2);
    assert_eq!(q["moyenne_premier"], 50);
    let manquee = &q["manquees"][0];
    assert_eq!(
        (manquee["numero"].clone(), manquee["ancienne"].clone()),
        (json!(1), json!(false))
    );
    assert!(!manquee["enonce"].as_str().unwrap().is_empty());
    let d_adama = q["par_eleve"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["eleve_id"] == json!(adama))
        .unwrap()
        .clone();
    assert_eq!(d_adama["premier"], json!({ "score": 1, "sur": 2 }));
    assert_eq!(d_adama["essais"], 1);
    let ReponseLecture::Lu(mon) = p
        .application
        .portail_mon_avancement(Some(&a))
        .await
        .unwrap()
    else {
        panic!("mon avancement se lit");
    };
    assert_eq!(
        mon.scores
            .iter()
            .map(|s| (s.titre.as_str(), s.score, s.sur))
            .collect::<Vec<_>>(),
        vec![("La racine carrée", 1, 2)]
    );
    // Un deuxième essai, encore dans la file : déjà compté, le meilleur monte.
    p.application
        .portail_tentative(Some(&a), &tentative(["A", "3,5"], &version))
        .await
        .unwrap();
    let suivi = p
        .ok(
            "bibliotheque_suivi_lire",
            Some(&admin),
            json!({ "classe_id": neuf_a }),
        )
        .await;
    let q = quiz_de_la_racine(&suivi).unwrap();
    let d_adama = q["par_eleve"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["eleve_id"] == json!(adama))
        .unwrap()
        .clone();
    assert_eq!(d_adama["premier"], json!({ "score": 1, "sur": 2 }));
    assert_eq!(d_adama["meilleur"], json!({ "score": 2, "sur": 2 }));
    assert_eq!(q["moyenne_premier"], 50, "seul le premier essai compte");
    assert_eq!(p.application.portail_avancement_ecrire().await, 1);
    let imprime = p
        .ok(
            "bibliotheque_suivi_imprimer",
            Some(&admin),
            json!({ "classe_id": neuf_a }),
        )
        .await;
    assert!(!imprime["pdf"].as_str().unwrap().is_empty());
    // Rien au bulletin : aucune note n'est née d'un quiz.
    let mut cnx = crate::commun::base_brute(dossier.path(), p.trousseau.clone()).await;
    let notes: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM note")
        .fetch_one(&mut cnx)
        .await
        .unwrap();
    assert_eq!(notes, notes_avant, "un quiz ne donne aucune note");
    cnx.close().await.unwrap();

    // Le module éteint coupe les quiz ; les tentatives restent et se relisent rallumé.
    let non = json!({ "type": "booleen", "valeur": false });
    let oui = json!({ "type": "booleen", "valeur": true });
    regler(&p, &admin, "module.bibliotheque", non).await;
    let eteint = p
        .ok(
            "bibliotheque_suivi_lire",
            Some(&admin),
            json!({ "classe_id": neuf_a }),
        )
        .await;
    assert_eq!(eteint["actif"], false);
    assert!(eteint["quiz"].as_array().unwrap().is_empty());
    let mut cnx = crate::commun::base_brute(dossier.path(), p.trousseau.clone()).await;
    let gardees: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tentative_quiz")
        .fetch_one(&mut cnx)
        .await
        .unwrap();
    assert_eq!(gardees, 2, "module éteint, les tentatives sont gardées");
    cnx.close().await.unwrap();
    regler(&p, &admin, "module.bibliotheque", oui).await;
    let rallume = p
        .ok(
            "bibliotheque_suivi_lire",
            Some(&admin),
            json!({ "classe_id": neuf_a }),
        )
        .await;
    let q = quiz_de_la_racine(&rallume).expect("rallumé, le quiz revient au suivi");
    assert_eq!(q["eleves"], 1);
    assert!(!q["par_eleve"].as_array().unwrap().is_empty());

    // Étape 5 : le quiz sur papier, pour la leçon et pour le chapitre.
    let sortie = tempfile::tempdir().unwrap();
    for forme in ["quiz_seul", "reponses_quiz"] {
        let lecon = sortie.path().join(format!("{forme}.pdf"));
        p.ok(
            "bibliotheque_imprimer",
            Some(&admin),
            json!({ "contenu_id": racine, "brouillon": false, "forme": forme, "chemin": lecon }),
        )
        .await;
        assert!(std::fs::read(&lecon).unwrap().starts_with(b"%PDF-"));
        let tout = sortie.path().join(format!("chapitre-{forme}.pdf"));
        p.ok(
            "bibliotheque_chapitre_imprimer",
            Some(&admin),
            json!({ "chapitre_id": chapitre.id, "forme": forme, "chemin": tout }),
        )
        .await;
        assert!(std::fs::read(&tout).unwrap().starts_with(b"%PDF-"));
    }
}

/// Identifiant de la classe de ce libellé dans l'année.
async fn classe_nommee(
    p: &crate::commun::Poste_,
    admin: &gesco_services::session::Jeton,
    annee: &Value,
    libelle: &str,
) -> Value {
    p.ok("classes_lister", Some(admin), json!({ "annee_id": annee }))
        .await
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["classe"]["libelle"] == libelle)
        .unwrap()["classe"]["id"]
        .clone()
}

/// Un devoir de mathématiques à écrire.
fn devoir_ecrit(classe: &Value, matiere: &Value, titre: &str) -> Value {
    json!({ "classe_id": classe, "matiere_id": matiere, "titre": titre,
        "consignes": "Calcule $\\sqrt{49}$ et explique.", "contenu_id": null,
        "date_limite": 1_789_000_000_000_i64, "bareme": 2000, "forme_rendu": "texte_ou_photos" })
}

/// L'année 2026-2027 de `maitres_de_math`, ses mathématiques, une autre matière, la 9e A et
/// une 9e B sans maître.
async fn annee_des_devoirs(
    p: &crate::commun::Poste_,
    admin: &gesco_services::session::Jeton,
) -> (Value, Value, Value, Value, Value) {
    let annees = p.ok("annees_lister", Some(admin), Value::Null).await;
    let annee = annees
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["libelle"] == "2026-2027")
        .unwrap()["id"]
        .clone();
    let ped = p
        .ok("pedagogie_lire", Some(admin), json!({ "id": annee }))
        .await;
    let matieres = ped["matieres"].as_array().unwrap();
    let math = matieres.iter().find(|m| m["code"] == "MATH").unwrap()["id"].clone();
    let autre = matieres.iter().find(|m| m["code"] != "MATH").unwrap()["id"].clone();
    let neuvieme = ped["niveaux"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["code"] == "9")
        .unwrap()["id"]
        .clone();
    let neuf_a = classe_nommee(p, admin, &annee, "9e A").await;
    let neuf_b = p
        .ok(
            "classe_creer",
            Some(admin),
            json!({ "annee_id": annee, "niveau_id": neuvieme, "serie_id": null,
                "libelle": "9e B" }),
        )
        .await["id"]
        .clone();
    (annee, math, autre, neuf_a, neuf_b)
}

#[tokio::test]
async fn le_maitre_donne_un_devoir_dans_ses_seules_classes_et_matieres() {
    use sqlx::Connection as _;

    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let maitres = maitres_de_math(&p, &admin, &[("sekou", "Traoré", "Sékou")]).await;
    let sekou = &maitres[0];
    let (_, math, autre, neuf_a, neuf_b) = annee_des_devoirs(&p, &admin).await;

    // Le maître ne se voit proposer que la 9e A, en mathématiques.
    let liste = p.ok("devoirs_lister", Some(sekou), json!({})).await;
    assert_eq!(liste["actif"], true);
    assert_eq!(liste["peut_donner"], true);
    let classes = liste["classes"].as_array().unwrap();
    assert_eq!(classes.len(), 1);
    assert_eq!(classes[0]["id"], neuf_a);
    let matieres = classes[0]["matieres"].as_array().unwrap();
    assert_eq!(matieres.len(), 1);
    assert_eq!(matieres[0]["id"], math);

    // Il donne un devoir à la 9e A ; pas dans une autre matière, pas à la 9e B.
    let devoir = p
        .ok(
            "devoir_creer",
            Some(sekou),
            devoir_ecrit(&neuf_a, &math, "Racines carrées"),
        )
        .await;
    assert_eq!(devoir["etat"], "brouillon");
    assert_eq!(devoir["classe"], "9e A");
    assert!(!devoir["lecon"]["blocs"].as_array().unwrap().is_empty());
    let id = devoir["id"].clone();
    for (classe, m) in [(&neuf_a, &autre), (&neuf_b, &math)] {
        let refus = p
            .refus(
                "devoir_creer",
                Some(sekou),
                devoir_ecrit(classe, m, "Hors de sa portée"),
            )
            .await;
        assert_eq!(refus.cle, "erreur.portee_refusee");
    }
    let mut sans_titre = devoir_ecrit(&neuf_a, &math, "  ");
    sans_titre["forme_rendu"] = json!("texte");
    let refus = p.refus("devoir_creer", Some(sekou), sans_titre).await;
    assert_eq!(refus.cle, "devoir.titre_vide");

    // Modifier : l'instant lu protège ce qu'un autre a écrit entre-temps.
    let mut modifie = devoir_ecrit(&neuf_a, &math, "Racines carrées (exercices)");
    modifie["bareme"] = json!(1000);
    let refus = p
        .refus(
            "devoir_enregistrer",
            Some(sekou),
            json!({ "id": id, "lue_le": 1, "devoir": modifie }),
        )
        .await;
    assert_eq!(refus.cle, "fiche.modifiee_entre_temps");
    let devoir = p
        .ok(
            "devoir_enregistrer",
            Some(sekou),
            json!({ "id": id, "lue_le": devoir["modifie_le"], "devoir": modifie }),
        )
        .await;
    assert_eq!(devoir["titre"], "Racines carrées (exercices)");
    assert_eq!(devoir["bareme"], 1000);

    // Brouillon → donné → clos → rouvert ; la feuille s'imprime.
    let pdf = p
        .ok("devoir_feuille", Some(sekou), json!({ "id": id }))
        .await;
    assert!(!pdf["pdf"].as_str().unwrap().is_empty());
    let donne = p
        .ok("devoir_donner", Some(sekou), json!({ "id": id }))
        .await;
    assert_eq!(donne["etat"], "donne");
    assert!(donne["donne_le"].is_i64());
    // Donné, il ne change plus de classe.
    let refus = p
        .refus(
            "devoir_enregistrer",
            Some(sekou),
            json!({ "id": id, "lue_le": donne["modifie_le"],
                "devoir": devoir_ecrit(&neuf_b, &math, "Ailleurs") }),
        )
        .await;
    assert_eq!(refus.cle, "devoir.classe_fixee");
    let clos = p.ok("devoir_clore", Some(sekou), json!({ "id": id })).await;
    assert_eq!(clos["etat"], "clos");
    assert!(clos["clos_le"].is_i64());
    let refus = p
        .refus("devoir_donner", Some(sekou), json!({ "id": id }))
        .await;
    assert_eq!(refus.cle, "devoir.etat_invalide");
    let rouvert = p
        .ok("devoir_rouvrir", Some(sekou), json!({ "id": id }))
        .await;
    assert_eq!(rouvert["etat"], "donne");
    assert!(rouvert["clos_le"].is_null());
    let pdf = p
        .ok("devoir_feuille", Some(sekou), json!({ "id": id }))
        .await;
    assert!(!pdf["pdf"].as_str().unwrap().is_empty());

    // La direction le donne aussi à la 9e B : une copie, donnée, que le maître ne voit pas.
    let copie = p
        .ok(
            "devoir_copier",
            Some(&admin),
            json!({ "id": id, "classe_id": neuf_b }),
        )
        .await;
    assert_eq!(copie["etat"], "donne");
    assert_eq!(copie["classe"], "9e B");
    assert_eq!(copie["copie_de"], id);
    let refus = p
        .refus(
            "devoir_copier",
            Some(sekou),
            json!({ "id": id, "classe_id": neuf_b }),
        )
        .await;
    assert_eq!(refus.cle, "erreur.portee_refusee");
    let refus = p
        .refus("devoir_lire", Some(sekou), json!({ "id": copie["id"] }))
        .await;
    assert_eq!(refus.cle, "erreur.portee_refusee");
    let du_maitre = p.ok("devoirs_lister", Some(sekou), json!({})).await;
    assert_eq!(du_maitre["devoirs"].as_array().unwrap().len(), 1);
    let de_la_direction = p.ok("devoirs_lister", Some(&admin), json!({})).await;
    assert_eq!(de_la_direction["devoirs"].as_array().unwrap().len(), 2);
    let de_la_b = p
        .ok(
            "devoirs_lister",
            Some(&admin),
            json!({ "classe_id": neuf_b, "matiere_id": null }),
        )
        .await;
    assert_eq!(de_la_b["devoirs"].as_array().unwrap().len(), 1);

    // L'interrupteur de la bibliothèque coupe les devoirs ; ils sont gardés et reviennent.
    let non = json!({ "type": "booleen", "valeur": false });
    let oui = json!({ "type": "booleen", "valeur": true });
    regler(&p, &admin, "module.bibliotheque", non).await;
    let eteint = p.ok("devoirs_lister", Some(&admin), json!({})).await;
    assert_eq!(eteint["actif"], false);
    assert!(eteint["devoirs"].as_array().unwrap().is_empty());
    let refus = p
        .refus(
            "devoir_creer",
            Some(&admin),
            devoir_ecrit(&neuf_a, &math, "Module éteint"),
        )
        .await;
    assert_eq!(refus.cle, "devoir.module_eteint");
    let refus = p
        .refus("devoir_clore", Some(&admin), json!({ "id": id }))
        .await;
    assert_eq!(refus.cle, "devoir.module_eteint");
    let mut cnx = crate::commun::base_brute(dossier.path(), p.trousseau.clone()).await;
    let gardes: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM devoir")
        .fetch_one(&mut cnx)
        .await
        .unwrap();
    assert_eq!(gardes, 2, "module éteint, les devoirs sont gardés");
    cnx.close().await.unwrap();
    regler(&p, &admin, "module.bibliotheque", oui).await;
    let rallume = p.ok("devoirs_lister", Some(&admin), json!({})).await;
    assert_eq!(rallume["actif"], true);
    assert_eq!(rallume["devoirs"].as_array().unwrap().len(), 2);

    // Chaque geste est au journal d'audit : création, modification, donné, clos, rouvert, copie.
    let mut cnx = crate::commun::base_brute(dossier.path(), p.trousseau.clone()).await;
    let audits: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM journal_audit WHERE entite = 'devoir'")
            .fetch_one(&mut cnx)
            .await
            .unwrap();
    assert_eq!(audits, 6);
    cnx.close().await.unwrap();
}

/// Un JPEG de forme juste avec un EXIF qui porte une position GPS ; `taille` le grossit.
fn jpeg_avec_gps(taille: usize) -> Vec<u8> {
    fn segment(marqueur: u8, donnees: &[u8]) -> Vec<u8> {
        let mut s = vec![0xFF, marqueur];
        s.extend_from_slice(&u16::try_from(donnees.len() + 2).unwrap().to_be_bytes());
        s.extend_from_slice(donnees);
        s
    }
    let mut o = vec![0xFF, 0xD8];
    o.extend(segment(0xE1, b"Exif\0\0MM\0*GPSLatitude 12.6392 N"));
    o.extend(segment(0xDB, &[0u8; 65]));
    o.extend(segment(0xC0, &[8, 0, 16, 0, 16, 1, 1, 0x11, 0]));
    o.extend(segment(0xC4, &[0u8; 20]));
    o.extend(segment(0xDA, &[1, 1, 0, 0, 63, 0]));
    let donnees = taille.saturating_sub(o.len() + 2).max(4);
    o.extend((0..donnees).map(|i| u8::try_from(i % 200).unwrap()));
    o.extend_from_slice(&[0xFF, 0xD9]);
    o
}

/// Lit une réponse du portail qui doit être lue.
fn lu<T: std::fmt::Debug>(r: gesco_services::portail::lecture::ReponseLecture<T>) -> T {
    match r {
        gesco_services::portail::lecture::ReponseLecture::Lu(t) => t,
        autre => panic!("{autre:?}"),
    }
}

/// Phase 12, lot 5, étape 3 (SPEC_PHASE_12_LOT_5 §0.3 critères 2 à 4, §4 ; ADR « copies
/// photographiées ») : les élèves de 9e A voient le devoir dans « Mes devoirs », ceux de 9e B
/// non ; sans code, rien à rendre ; trois photos arrivent sans position GPS ; un fichier qui
/// n'est pas une photo, une photo trop grosse, une sixième photo sont refusés ; rendre de nouveau
/// garde l'ancien rendu ; après la date limite le rendu est marqué en retard ; clos, on ne rend
/// plus ; un élève ne voit jamais la photo d'un autre ; bibliothèque éteinte, le portail ne sert
/// plus les devoirs, qui sont gardés.
#[tokio::test]
async fn un_eleve_rend_son_devoir_par_le_portail() {
    use base64::Engine as _;
    use gesco_services::portail::devoirs::{DemandeRendu, EtatPourEleve, RefusRendu};
    use gesco_services::portail::entree::DemandeLireSansCode;
    use gesco_services::portail::lecture::ReponseLecture;
    use sqlx::Connection as _;

    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let maitres = maitres_de_math(&p, &admin, &[("sekou", "Traoré", "Sékou")]).await;
    let sekou = &maitres[0];
    let (annee, math, _, neuf_a, neuf_b) = annee_des_devoirs(&p, &admin).await;
    crate::commun::poser_grille(&p, &admin, &annee).await;
    let adama = eleve_dans(&p, &admin, &neuf_a, "Adama", "E-931").await;
    let awa = eleve_dans(&p, &admin, &neuf_a, "Awa", "E-932").await;
    let binta = eleve_dans(&p, &admin, &neuf_b, "Binta", "E-933").await;

    // Un devoir donné à la 9e A, à rendre dans une heure ; un brouillon qui ne se voit pas.
    let mut ecrit = devoir_ecrit(&neuf_a, &math, "Racines carrées");
    ecrit["date_limite"] = json!(crate::commun::DEBUT + 3_600_000);
    let devoir = p.ok("devoir_creer", Some(sekou), ecrit).await;
    let id = devoir["id"].as_str().unwrap().to_owned();
    p.ok("devoir_donner", Some(sekou), json!({ "id": id }))
        .await;
    p.ok(
        "devoir_creer",
        Some(sekou),
        devoir_ecrit(&neuf_a, &math, "Brouillon caché"),
    )
    .await;

    let a = entrer_au_portail(&p, &adama, "E-931").await;
    let w = entrer_au_portail(&p, &awa, "E-932").await;
    let b = entrer_au_portail(&p, &binta, "E-933").await;
    let de_a = lu(p.application.portail_mes_devoirs(Some(&a)).await.unwrap());
    assert!(de_a.avec_code);
    assert_eq!(de_a.devoirs.len(), 1, "le brouillon ne se voit pas");
    assert_eq!(de_a.devoirs[0].titre, "Racines carrées");
    assert_eq!(de_a.devoirs[0].etat, EtatPourEleve::ARendre);
    let de_b = lu(p.application.portail_mes_devoirs(Some(&b)).await.unwrap());
    assert!(de_b.devoirs.is_empty(), "la 9e B ne le voit pas");
    assert!(matches!(
        p.application.portail_devoir(Some(&b), &id).await.unwrap(),
        ReponseLecture::Introuvable
    ));

    // Sans code : la page dit d'entrer avec son code ; rien ne se rend.
    let ped = p
        .ok("pedagogie_lire", Some(&admin), json!({ "id": annee }))
        .await;
    let neuvieme = ped["niveaux"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["code"] == "9")
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (sans_code, _) = p
        .application
        .portail_lire_sans_code(&DemandeLireSansCode {
            niveau_id: neuvieme,
            serie_id: None,
        })
        .await
        .unwrap()
        .expect("la 9e se lit sans code");
    let sans = lu(p
        .application
        .portail_mes_devoirs(Some(&sans_code))
        .await
        .unwrap());
    assert!(!sans.avec_code && sans.devoirs.is_empty());
    let demande = |devoir: &str, texte: &str, photos: &[Vec<u8>]| DemandeRendu {
        devoir_id: devoir.to_owned(),
        texte: texte.to_owned(),
        photos: photos
            .iter()
            .map(|o| base64::engine::general_purpose::STANDARD.encode(o))
            .collect(),
    };
    let refus = lu(p
        .application
        .portail_rendre(Some(&sans_code), &demande(&id, "x", &[]))
        .await
        .unwrap());
    assert_eq!(refus.err(), Some(RefusRendu::SansCode));

    // Refus : pas une photo, trop grosse (4 Mo non réduite), six photos, rien du tout.
    let cas: [(Vec<Vec<u8>>, &str, RefusRendu); 4] = [
        (vec![b"%PDF-1.7".to_vec()], "", RefusRendu::PasUnePhoto),
        (
            vec![jpeg_avec_gps(4 * 1024 * 1024)],
            "",
            RefusRendu::PhotoTropGrosse,
        ),
        (vec![jpeg_avec_gps(1000); 6], "", RefusRendu::TropDePhotos),
        (Vec::new(), "  ", RefusRendu::Vide),
    ];
    for (photos, texte, attendu) in cas {
        let r = lu(p
            .application
            .portail_rendre(Some(&a), &demande(&id, texte, &photos))
            .await
            .unwrap());
        assert_eq!(r.err(), Some(attendu));
    }

    // Trois photos réduites : elles arrivent sans position GPS.
    let photos = [
        jpeg_avec_gps(300_000),
        jpeg_avec_gps(310_000),
        jpeg_avec_gps(320_000),
    ];
    let fait = lu(p
        .application
        .portail_rendre(Some(&a), &demande(&id, "Voici ma copie.", &photos))
        .await
        .unwrap())
    .unwrap();
    assert_eq!(fait.numero, 1);
    assert!(!fait.apres_limite);
    let ouvert = lu(p.application.portail_devoir(Some(&a), &id).await.unwrap());
    assert_eq!(ouvert.etat, EtatPourEleve::Rendu);
    assert_eq!(ouvert.rendus.len(), 1);
    assert_eq!(ouvert.rendus[0].photos.len(), 3);
    assert!(ouvert.correction.is_none());
    let rendu_a = ouvert.rendus[0].id.clone();
    let photo = lu(p
        .application
        .portail_photo_rendu(Some(&a), &rendu_a, 2)
        .await
        .unwrap());
    assert_eq!(photo.type_mime, "image/jpeg");
    let octets = photo.octets.unwrap();
    assert!(!octets.windows(3).any(|f| f == b"GPS"), "sans position");
    assert!(octets.starts_with(&[0xFF, 0xD8]) && octets.ends_with(&[0xFF, 0xD9]));
    // Un autre élève, même de la classe, ne voit jamais la photo ni le rendu d'Adama.
    for autre in [&w, &b] {
        assert!(matches!(
            p.application
                .portail_photo_rendu(Some(autre), &rendu_a, 2)
                .await
                .unwrap(),
            ReponseLecture::Introuvable
        ));
    }
    let de_w = lu(p.application.portail_devoir(Some(&w), &id).await.unwrap());
    assert!(de_w.rendus.is_empty());
    assert_eq!(de_w.etat, EtatPourEleve::ARendre);
    let mut cnx = crate::commun::base_brute(dossier.path(), p.trousseau.clone()).await;
    let fichiers: Vec<Vec<u8>> = sqlx::query_scalar("SELECT contenu FROM fichier_rendu")
        .fetch_all(&mut cnx)
        .await
        .unwrap();
    assert_eq!(fichiers.len(), 3);
    assert!(
        fichiers
            .iter()
            .all(|f| !f.windows(3).any(|x| x == b"GPS") && f.len() < 330_000)
    );
    cnx.close().await.unwrap();

    // Après la date limite : rendre de nouveau est accepté, marqué en retard ; l'ancien reste.
    p.horloge.avancer(2 * 3_600_000);
    let de_w = lu(p.application.portail_mes_devoirs(Some(&w)).await.unwrap());
    assert_eq!(de_w.devoirs[0].etat, EtatPourEleve::EnRetard);
    let fait = lu(p
        .application
        .portail_rendre(Some(&a), &demande(&id, "Copie revue.", &[]))
        .await
        .unwrap())
    .unwrap();
    assert_eq!(fait.numero, 2);
    assert!(fait.apres_limite);
    let de_a = lu(p.application.portail_mes_devoirs(Some(&a)).await.unwrap());
    assert_eq!(de_a.devoirs[0].etat, EtatPourEleve::Rendu);
    assert!(de_a.devoirs[0].apres_limite);
    let ouvert = lu(p.application.portail_devoir(Some(&a), &id).await.unwrap());
    assert_eq!(ouvert.rendus.len(), 2);
    assert_eq!(ouvert.rendus[1].texte, "Copie revue.");

    // Clos : on ne rend plus ; « Clos » pour qui n'a pas rendu.
    p.ok("devoir_clore", Some(sekou), json!({ "id": id })).await;
    let refus = lu(p
        .application
        .portail_rendre(Some(&w), &demande(&id, "Trop tard", &[]))
        .await
        .unwrap());
    assert_eq!(refus.err(), Some(RefusRendu::Clos));
    let de_w = lu(p.application.portail_mes_devoirs(Some(&w)).await.unwrap());
    assert_eq!(de_w.devoirs[0].etat, EtatPourEleve::Clos);

    // Un devoir sur papier seulement : consignes lisibles, rien ne se rend par le portail.
    let mut papier = devoir_ecrit(&neuf_a, &math, "Sur la feuille");
    papier["forme_rendu"] = json!("papier");
    let papier = p.ok("devoir_creer", Some(sekou), papier).await;
    let papier_id = papier["id"].as_str().unwrap().to_owned();
    p.ok("devoir_donner", Some(sekou), json!({ "id": papier_id }))
        .await;
    let de_w = lu(p.application.portail_mes_devoirs(Some(&w)).await.unwrap());
    let ligne = de_w
        .devoirs
        .iter()
        .find(|d| d.titre == "Sur la feuille")
        .unwrap();
    assert_eq!(ligne.etat, EtatPourEleve::Papier);
    let refus = lu(p
        .application
        .portail_rendre(Some(&w), &demande(&papier_id, "x", &[]))
        .await
        .unwrap());
    assert_eq!(refus.err(), Some(RefusRendu::Papier));

    // Bibliothèque éteinte : le portail ne sert plus les devoirs ; tout est gardé.
    regler(
        &p,
        &admin,
        "module.bibliotheque",
        json!({ "type": "booleen", "valeur": false }),
    )
    .await;
    assert!(matches!(
        p.application.portail_mes_devoirs(Some(&a)).await.unwrap(),
        ReponseLecture::Ferme
    ));
    assert!(matches!(
        p.application
            .portail_photo_rendu(Some(&a), &rendu_a, 1)
            .await
            .unwrap(),
        ReponseLecture::Ferme
    ));
    let mut cnx = crate::commun::base_brute(dossier.path(), p.trousseau.clone()).await;
    let rendus: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rendu_devoir")
        .fetch_one(&mut cnx)
        .await
        .unwrap();
    assert_eq!(rendus, 2);
    cnx.close().await.unwrap();
    regler(
        &p,
        &admin,
        "module.bibliotheque",
        json!({ "type": "booleen", "valeur": true }),
    )
    .await;
    let de_a = lu(p.application.portail_mes_devoirs(Some(&a)).await.unwrap());
    assert_eq!(de_a.devoirs.len(), 2);
}
