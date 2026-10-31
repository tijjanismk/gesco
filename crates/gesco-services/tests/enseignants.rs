//! Enseignants et affectations par la frontière (SPEC_PHASE_4 §3) : maître du 1er cycle, matière
//! confiée ou partagée, portée de saisie des notes, rôles en version 4.

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

#[tokio::test]
async fn les_enseignants_s_affectent_et_ne_saisissent_que_dans_leurs_classes() {
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
    let ped = p
        .ok("pedagogie_lire", Some(&admin), json!({ "id": annee["id"] }))
        .await;
    let niveau = |code: &str| id_par(&ped["niveaux"], "code", code);
    let matiere = |code: &str| id_par(&ped["matieres"], "code", code);
    let mut classes = Vec::new();
    for (code, libelle) in [("1", "1re A"), ("7", "7e A")] {
        classes.push(
            p.ok(
                "classe_creer",
                Some(&admin),
                json!({ "annee_id": annee["id"], "niveau_id": niveau(code), "serie_id": null, "libelle": libelle }),
            )
            .await["id"]
                .clone(),
        );
    }
    let (premiere, septieme) = (classes[0].clone(), classes[1].clone());

    // Un compte enseignant, pour qu'il saisisse lui-même.
    let compte = p
        .ok(
            "utilisateur_creer",
            Some(&admin),
            json!({ "identifiant": "sekou", "nom_affiche": "Sékou Traoré",
                "mot_de_passe_initial": "enseignant-2026", "roles": ["enseignant"] }),
        )
        .await;
    let enseignant = |nom: &str, prenoms: &str, statut: &str, compte: Value| {
        json!({ "id": null, "nom": nom, "prenoms": prenoms, "telephones": ["76 11 22 33"],
            "statut": statut, "utilisateur_id": compte, "actif": true })
    };
    let moussa = p
        .ok(
            "enseignant_enregistrer",
            Some(&admin),
            enseignant("Diarra", "Moussa", "titulaire", Value::Null),
        )
        .await;
    assert_eq!(moussa["telephones"], json!(["+22376112233"]));
    let aminata = p
        .ok(
            "enseignant_enregistrer",
            Some(&admin),
            enseignant("Koné", "Aminata", "vacataire", Value::Null),
        )
        .await;
    let sekou = p
        .ok(
            "enseignant_enregistrer",
            Some(&admin),
            enseignant("Traoré", "Sékou", "titulaire", compte["id"].clone()),
        )
        .await;
    assert_eq!(sekou["compte"], "Sékou Traoré");
    assert_eq!(
        p.refus(
            "enseignant_enregistrer",
            Some(&admin),
            enseignant("Coulibaly", "Fanta", "titulaire", compte["id"].clone())
        )
        .await
        .cle,
        "enseignant.compte_deja_lie"
    );
    let id_ens = |f: &Value| f["enseignant"]["id"].clone();

    // Le maître reçoit toute sa classe du 1er cycle, les douze matières de l'emploi du temps de
    // la 1re année ; le dessin est confié à une autre.
    let recues = p
        .ok(
            "maitre_affecter",
            Some(&admin),
            json!({ "classe_id": premiere, "enseignant_id": id_ens(&moussa) }),
        )
        .await;
    assert_eq!(recues, 12);
    p.ok(
        "affectation_confier",
        Some(&admin),
        json!({ "classe_id": premiere, "matiere_id": matiere("DESS"), "enseignant_id": id_ens(&aminata) }),
    )
    .await;
    let liste = p
        .ok(
            "affectations_lister",
            Some(&admin),
            json!({ "id": annee["id"] }),
        )
        .await;
    let arts: Vec<&Value> = liste
        .as_array()
        .unwrap()
        .iter()
        .filter(|a| a["matiere"] == "Dessin")
        .collect();
    assert_eq!(arts.len(), 1);
    assert_eq!(arts[0]["enseignant"], "Aminata Koné");
    assert_eq!(
        p.refus(
            "maitre_affecter",
            Some(&admin),
            json!({ "classe_id": premiere, "enseignant_id": id_ens(&moussa) })
        )
        .await
        .cle,
        "affectation.rien_a_confier"
    );

    // En 7e : les maths à Sékou ; la physique-chimie partagée (T-22) ; le LC n'est pas au programme.
    let ajouter = |classe: &Value, code: &str, ens: &Value| json!({ "classe_id": classe, "matiere_id": matiere(code), "enseignant_id": ens });
    p.ok(
        "affectation_ajouter",
        Some(&admin),
        ajouter(&septieme, "MATH", &id_ens(&sekou)),
    )
    .await;
    assert_eq!(
        p.refus(
            "affectation_ajouter",
            Some(&admin),
            ajouter(&septieme, "MATH", &id_ens(&sekou))
        )
        .await
        .cle,
        "affectation.deja"
    );
    p.ok(
        "affectation_ajouter",
        Some(&admin),
        ajouter(&septieme, "PC", &id_ens(&aminata)),
    )
    .await;
    let pc_sekou = p
        .ok(
            "affectation_ajouter",
            Some(&admin),
            ajouter(&septieme, "PC", &id_ens(&sekou)),
        )
        .await;
    assert_eq!(
        p.refus(
            "affectation_ajouter",
            Some(&admin),
            ajouter(&septieme, "LC", &id_ens(&sekou))
        )
        .await
        .cle,
        "affectation.matiere_hors_programme"
    );

    // Portée : Sékou dans ses deux couples, la direction partout.
    let sekou_jeton = p.connecter("sekou", "enseignant-2026").await;
    let portee = p
        .ok("portee_notes_lire", Some(&sekou_jeton), Value::Null)
        .await;
    assert_eq!(portee["type"], "couples");
    let couples = portee["couples"].as_array().unwrap();
    assert_eq!(couples.len(), 2);
    assert!(couples.contains(&json!([septieme, matiere("MATH")])));
    assert_eq!(
        p.ok("portee_notes_lire", Some(&admin), Value::Null).await["type"],
        "etablissement"
    );
    p.ok(
        "affectation_retirer",
        Some(&admin),
        json!({ "id": pc_sekou["id"] }),
    )
    .await;
    let portee = p
        .ok("portee_notes_lire", Some(&sekou_jeton), Value::Null)
        .await;
    assert_eq!(portee["couples"].as_array().unwrap().len(), 1);

    // Le Directeur saisit les notes (rôles v4) ; l'enseignant ne gère pas les affectations.
    p.ok(
        "utilisateur_creer",
        Some(&admin),
        json!({ "identifiant": "awa", "nom_affiche": "Awa Diallo",
            "mot_de_passe_initial": "direction-2026", "roles": ["directeur"] }),
    )
    .await;
    let directrice = p.connecter("awa", "direction-2026").await;
    let session = p
        .ok("session_courante", Some(&directrice), Value::Null)
        .await;
    let permissions = session["permissions"].as_array().unwrap();
    assert!(permissions.contains(&json!("grades:create")));
    assert!(
        !permissions
            .iter()
            .any(|x| x.as_str().unwrap().starts_with("payments:"))
    );
    assert_eq!(
        p.ok("portee_notes_lire", Some(&directrice), Value::Null)
            .await["type"],
        "etablissement"
    );
    assert_eq!(
        p.refus(
            "affectation_ajouter",
            Some(&sekou_jeton),
            ajouter(&septieme, "FR", &id_ens(&sekou))
        )
        .await
        .cle,
        "erreur.permission_refusee"
    );
    let fiches = p
        .ok("enseignants_lister", Some(&sekou_jeton), Value::Null)
        .await;
    assert_eq!(fiches.as_array().unwrap().len(), 3);
    assert_eq!(fiches[0]["nom"], "Diarra");
}
