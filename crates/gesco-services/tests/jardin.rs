//! Compétences du jardin par la frontière (SPEC_PHASE_11_B, étape 1) : liste livrée pour les
//! sections ouvertes, trois trimestres au jardin, la direction ajoute, renomme, réordonne,
//! retire puis remet, recopie d'une section à l'autre ; les droits et l'échelle.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use crate::commun::*;
use serde_json::{Value, json};

fn id_par(liste: &Value, champ: &str, valeur: &str) -> Value {
    liste
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x[champ] == valeur)
        .unwrap_or_else(|| panic!("{champ} = {valeur} absent"))["id"]
        .clone()
}

/// Compétences qui servent dans une section et un domaine, dans leur ordre.
fn libelles(j: &Value, section: &Value, domaine: &Value) -> Vec<String> {
    let mut c: Vec<&Value> = j["competences"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| {
            c["niveau_id"] == *section && c["domaine_id"] == *domaine && c["retiree_le"].is_null()
        })
        .collect();
    c.sort_by_key(|c| c["ordre"].as_i64().unwrap());
    c.iter()
        .map(|c| c["libelle"].as_str().unwrap().to_owned())
        .collect()
}

fn actives(j: &Value, section: &Value) -> usize {
    j["competences"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["niveau_id"] == *section && c["retiree_le"].is_null())
        .count()
}

#[tokio::test]
async fn la_liste_livree_se_pose_puis_l_ecole_la_tient() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let annee = p
        .ok(
            "annee_creer",
            Some(&admin),
            json!({ "libelle": "2026-2027", "debut": RENTREE_2026, "fin": FIN_2027 }),
        )
        .await;

    // Critère 1 : la liste livrée pour chaque section ouverte, une vingtaine de compétences.
    let j = p.ok("jardin_lire", Some(&admin), Value::Null).await;
    let sections = j["sections"].clone();
    assert_eq!(sections.as_array().unwrap().len(), 3);
    let ps = id_par(&sections, "code", "PS");
    let ms = id_par(&sections, "code", "MS");
    let gs = id_par(&sections, "code", "GS");
    let domaines: Vec<&str> = j["domaines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["libelle"].as_str().unwrap())
        .collect();
    assert_eq!(
        domaines,
        [
            "Langage",
            "Motricité",
            "Vivre ensemble",
            "Découverte du monde",
            "Arts"
        ]
    );
    for s in [&ps, &ms, &gs] {
        assert_eq!(actives(&j, s), 20);
    }
    assert_eq!(j["echelle"], json!(["Acquis", "En cours", "Non acquis"]));
    assert_eq!(j["commentaire_domaine"], true);
    let langage = id_par(&j["domaines"], "libelle", "Langage");
    let arts = id_par(&j["domaines"], "libelle", "Arts");
    assert_eq!(
        libelles(&j, &gs, &langage)[0],
        "Raconte dans l'ordre une histoire entendue"
    );

    // Critère 1 : trois trimestres au jardin pour l'année ouverte, aux dates du 2e cycle.
    let ped = p
        .ok("pedagogie_lire", Some(&admin), json!({ "id": annee["id"] }))
        .await;
    let du_jardin: Vec<&Value> = ped["periodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|x| x["periode"]["niveau_id"] == ms)
        .collect();
    assert_eq!(du_jardin.len(), 3);
    assert_eq!(du_jardin[0]["libelle"], "1er trimestre");
    assert_eq!(du_jardin[0]["periode"]["debut"], "2026-10-01");
    assert_eq!(du_jardin[2]["periode"]["fin"], "2027-06-30");
    // Le jardin n'a pas de programme de matières.
    assert!(
        !ped["programme"]
            .as_array()
            .unwrap()
            .iter()
            .any(|l| l["niveau_id"] == ms)
    );

    // Critère 3 : deux trimestres acceptés ; un semestre, un autre nombre refusés.
    let deux = p
        .ok(
            "periodes_definir",
            Some(&admin),
            json!({ "annee_id": annee["id"], "niveau_id": ms, "nature": "trimestre", "nombre": 2 }),
        )
        .await;
    assert_eq!(deux.as_array().unwrap().len(), 2);
    assert_eq!(deux[1]["debut"], "2027-02-01");
    for (nature, nombre, cle) in [
        ("semestre", 2, "periode.semestre_technique"),
        ("trimestre", 4, "periode.nombre_invalide"),
    ] {
        assert_eq!(
            p.refus(
                "periodes_definir",
                Some(&admin),
                json!({ "annee_id": annee["id"], "niveau_id": ms, "nature": nature,
                    "nombre": nombre }),
            )
            .await
            .cle,
            cle
        );
    }

    // Critère 2 : ajouter, renommer, réordonner, retirer puis remettre.
    let ajoutee = p
        .ok(
            "competence_enregistrer",
            Some(&admin),
            json!({ "id": null, "domaine_id": langage, "niveau_id": ps,
                "libelle": "  Dit son   prénom et celui de sa maîtresse ", "retiree": false }),
        )
        .await;
    assert_eq!(
        ajoutee["libelle"],
        "Dit son prénom et celui de sa maîtresse"
    );
    assert_eq!(ajoutee["livree"], false);
    assert_eq!(ajoutee["ordre"], 5);
    let doublon = p
        .refus(
            "competence_enregistrer",
            Some(&admin),
            json!({ "id": null, "domaine_id": langage, "niveau_id": ps,
                "libelle": "dit son prénom et celui de sa maîtresse", "retiree": false }),
        )
        .await;
    assert_eq!(doublon.cle, "jardin.competence_existe");
    let renommee = p
        .ok(
            "competence_enregistrer",
            Some(&admin),
            json!({ "id": ajoutee["id"], "domaine_id": langage, "niveau_id": ps,
                "libelle": "Dit son prénom", "retiree": false }),
        )
        .await;
    assert_eq!(renommee["libelle"], "Dit son prénom");
    let apres = p
        .ok(
            "competence_deplacer",
            Some(&admin),
            json!({ "id": ajoutee["id"], "monter": true }),
        )
        .await;
    let j = json!({ "competences": apres });
    let liste = libelles(&j, &ps, &langage);
    assert_eq!(liste.len(), 5);
    assert_eq!(liste[3], "Dit son prénom");
    // Au bord, rien ne bouge.
    let premiere = liste[0].clone();
    let id_premiere = j["competences"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["libelle"] == premiere.as_str() && c["niveau_id"] == ps)
        .unwrap()["id"]
        .clone();
    let inchangee = p
        .ok(
            "competence_deplacer",
            Some(&admin),
            json!({ "id": id_premiere, "monter": true }),
        )
        .await;
    assert_eq!(
        libelles(&json!({ "competences": inchangee }), &ps, &langage),
        liste
    );
    let retiree = p
        .ok(
            "competence_enregistrer",
            Some(&admin),
            json!({ "id": ajoutee["id"], "domaine_id": langage, "niveau_id": ps,
                "libelle": "Dit son prénom", "retiree": true }),
        )
        .await;
    assert!(retiree["retiree_le"].is_i64());
    let j = p.ok("jardin_lire", Some(&admin), Value::Null).await;
    assert_eq!(actives(&j, &ps), 20);
    // Retirée, elle reste en base et se remet telle quelle, à sa place.
    assert!(
        j["competences"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["id"] == ajoutee["id"])
    );
    let remise = p
        .ok(
            "competence_enregistrer",
            Some(&admin),
            json!({ "id": ajoutee["id"], "domaine_id": langage, "niveau_id": ps,
                "libelle": "Dit son prénom", "retiree": false }),
        )
        .await;
    assert!(remise["retiree_le"].is_null());
    assert_eq!(remise["ordre"], retiree["ordre"]);

    // Un domaine se crée, se monte, se retire ; le nom d'un domaine qui sert ne se double pas.
    let eveil = p
        .ok(
            "domaine_competence_enregistrer",
            Some(&admin),
            json!({ "id": null, "libelle": "Éveil religieux", "retire": false }),
        )
        .await;
    assert_eq!(eveil["ordre"], 6);
    assert_eq!(eveil["livre"], false);
    assert_eq!(
        p.refus(
            "domaine_competence_enregistrer",
            Some(&admin),
            json!({ "id": null, "libelle": "ARTS", "retire": false }),
        )
        .await
        .cle,
        "jardin.domaine_existe"
    );
    let ordre = p
        .ok(
            "domaine_competence_deplacer",
            Some(&admin),
            json!({ "id": eveil["id"], "monter": true }),
        )
        .await;
    let noms: Vec<&str> = ordre
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["libelle"].as_str().unwrap())
        .collect();
    assert_eq!(noms[4], "Éveil religieux");
    assert_eq!(noms[5], "Arts");
    p.ok(
        "domaine_competence_enregistrer",
        Some(&admin),
        json!({ "id": eveil["id"], "libelle": "Éveil religieux", "retire": true }),
    )
    .await;
    // Une compétence ne s'ajoute pas dans un domaine retiré.
    assert_eq!(
        p.refus(
            "competence_enregistrer",
            Some(&admin),
            json!({ "id": null, "domaine_id": eveil["id"], "niveau_id": gs,
                "libelle": "Récite une courte sourate", "retiree": false }),
        )
        .await
        .cle,
        "jardin.domaine_retire"
    );
    // Ni dans un niveau qui n'est pas du jardin.
    let premiere_annee = id_par(&ped["niveaux"], "code", "1");
    assert_eq!(
        p.refus(
            "competence_enregistrer",
            Some(&admin),
            json!({ "id": null, "domaine_id": arts, "niveau_id": premiere_annee,
                "libelle": "Chante", "retiree": false }),
        )
        .await
        .cle,
        "jardin.section_invalide"
    );
    assert_eq!(
        p.refus(
            "competence_enregistrer",
            Some(&admin),
            json!({ "id": null, "domaine_id": arts, "niveau_id": gs, "libelle": "   ",
                "retiree": false }),
        )
        .await
        .cle,
        "jardin.competence_libelle_invalide"
    );

    // Recopier les compétences de la PS dans la MS : seules celles que la MS n'a pas encore.
    let ajoutees = p
        .ok(
            "competences_recopier",
            Some(&admin),
            json!({ "source_id": ps, "cible_id": ms }),
        )
        .await;
    assert_eq!(ajoutees, 21);
    let encore = p
        .ok(
            "competences_recopier",
            Some(&admin),
            json!({ "source_id": ps, "cible_id": ms }),
        )
        .await;
    assert_eq!(encore, 0);
    assert_eq!(
        p.refus(
            "competences_recopier",
            Some(&admin),
            json!({ "source_id": ps, "cible_id": ps }),
        )
        .await
        .cle,
        "jardin.recopie_meme_section"
    );

    // L'échelle : trois libellés, réglables, jamais vides.
    p.ok(
        "parametre_definir",
        Some(&admin),
        json!({ "cle": "jardin.echelle_en_cours", "valeur": { "type": "texte",
            "valeur": "En voie d'acquisition" } }),
    )
    .await;
    assert_eq!(
        p.refus(
            "parametre_definir",
            Some(&admin),
            json!({ "cle": "jardin.echelle_acquis", "valeur": { "type": "texte", "valeur": " " } }),
        )
        .await
        .cle,
        "parametre.hors_bornes"
    );
    let j = p.ok("jardin_lire", Some(&admin), Value::Null).await;
    assert_eq!(
        j["echelle"],
        json!(["Acquis", "En voie d'acquisition", "Non acquis"])
    );

    // Chaque écriture est au journal.
    let journal = p
        .ok(
            "audit_lister",
            Some(&admin),
            json!({ "utilisateur_id": null, "entite": "competence", "operation": null,
                "depuis": null, "jusqua": null, "page": 0 }),
        )
        .await;
    assert!(journal["lignes"].as_array().unwrap().len() >= 5);

    // Droits : une secrétaire lit la liste, ne la change pas.
    p.ok(
        "utilisateur_creer",
        Some(&admin),
        json!({ "identifiant": "fanta", "nom_affiche": "Fanta Coulibaly",
            "mot_de_passe_initial": "secretariat-2026", "roles": ["secretaire"] }),
    )
    .await;
    let secretaire = p.connecter("fanta", "secretariat-2026").await;
    p.ok("jardin_lire", Some(&secretaire), Value::Null).await;
    assert_eq!(
        p.refus(
            "competence_enregistrer",
            Some(&secretaire),
            json!({ "id": null, "domaine_id": arts, "niveau_id": gs, "libelle": "Chante",
                "retiree": false }),
        )
        .await
        .cle,
        "erreur.permission_refusee"
    );
}

#[tokio::test]
async fn une_ecole_sans_jardin_ne_recoit_rien_puis_le_recoit_en_l_ouvrant() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    // Une école fondamentale, sans jardin (critère 14).
    p.ok(
        "premier_demarrage",
        None,
        json!({
            "organisation": "Groupe scolaire du Fleuve",
            "etablissement_nom": "École fondamentale du Fleuve",
            "statut": "prive",
            "code_academie": "AE-BKO-RD",
            "code_cap": "CAP-01",
            "adresse": "Bamako",
            "telephones": ["+223 20 22 00 00"],
            "administrateur_identifiant": "admin",
            "administrateur_nom": "Direction",
            "administrateur_mot_de_passe": MOT_DE_PASSE,
            "cycles": ["premier_cycle", "second_cycle"],
        }),
    )
    .await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let j = p.ok("jardin_lire", Some(&admin), Value::Null).await;
    assert!(j["sections"].as_array().unwrap().is_empty());
    assert!(j["domaines"].as_array().unwrap().is_empty());
    assert!(j["competences"].as_array().unwrap().is_empty());
    // L'école ouvre son jardin : la liste livrée arrive pour ses trois sections.
    p.ok(
        "cycles_definir",
        Some(&admin),
        json!({ "cycles": ["jardin", "premier_cycle", "second_cycle"] }),
    )
    .await;
    let j = p.ok("jardin_lire", Some(&admin), Value::Null).await;
    assert_eq!(j["sections"].as_array().unwrap().len(), 3);
    assert_eq!(j["domaines"].as_array().unwrap().len(), 5);
    assert_eq!(j["competences"].as_array().unwrap().len(), 60);
    // Fermé puis rouvert, il ne reçoit pas une seconde liste.
    p.ok(
        "cycles_definir",
        Some(&admin),
        json!({ "cycles": ["premier_cycle", "second_cycle"] }),
    )
    .await;
    p.ok(
        "cycles_definir",
        Some(&admin),
        json!({ "cycles": ["jardin", "premier_cycle", "second_cycle"] }),
    )
    .await;
    let j = p.ok("jardin_lire", Some(&admin), Value::Null).await;
    assert_eq!(j["competences"].as_array().unwrap().len(), 60);
    assert_eq!(j["domaines"].as_array().unwrap().len(), 5);
}
