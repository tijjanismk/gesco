//! Moyenne annuelle, arrivées en cours d'année et moyennes reprises par la frontière
//! (SPEC_PHASE_4 §5, T-33, Q-27 à Q-29) : un élève arrivé au 2e trimestre n'a ni note ni
//! bulletin au 1er, sa moyenne du 1er est reprise de son relevé, il est classé dès le 2e, et la
//! moyenne annuelle de chacun se calcule au 3e sur les périodes suivies.

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

/// Élève complet, avec sa mère responsable légale joignable.
async fn eleve(p: &Poste_, admin: &gesco_services::session::Jeton, prenoms: &str) -> Value {
    let fiche = p
        .ok(
            "eleve_creer",
            Some(admin),
            json!({ "identite": { "nom": "Traoré", "prenoms": prenoms, "sexe": "feminin",
                "date_naissance": "2014-03-02", "date_naissance_estimee": false,
                "lieu_naissance": "Bamako", "nationalite": "Malienne", "adresse": "", "telephones": [] },
                "matricule_interne": "", "matricule_national": "", "acte_nature": "acte",
                "acte_numero": "", "acte_date": "", "ecole_origine": "" }),
        )
        .await;
    p.ok(
        "responsable_rattacher",
        Some(admin),
        json!({ "eleve_id": fiche["eleve"]["id"], "responsable_id": null,
            "saisie": { "identite": { "nom": "Coulibaly", "prenoms": format!("Mère de {prenoms}"), "sexe": "feminin",
                "date_naissance": "", "date_naissance_estimee": false, "lieu_naissance": "", "nationalite": "",
                "adresse": "", "telephones": ["76 12 34 56"] }, "profession": "", "employeur": "" },
            "qualites": { "lien": "mere", "est_responsable_legal": true, "est_payeur": true,
                "recoit_notifications": true, "vient_chercher": false, "ordre_priorite": 1 } }),
    )
    .await;
    fiche
}

async fn inscrire(
    p: &Poste_,
    admin: &gesco_services::session::Jeton,
    classe: &Value,
    prenom: &str,
) -> Value {
    let fiche = eleve(p, admin, prenom).await;
    p.ok(
        "inscription_creer",
        Some(admin),
        json!({ "eleve_id": fiche["eleve"]["id"], "classe_id": classe, "date_inscription": "",
            "redoublant": false, "etablissement_origine": "" }),
    )
    .await["inscription"]["id"]
        .clone()
}

#[tokio::test]
async fn un_eleve_arrive_au_deuxieme_trimestre_a_sa_moyenne_annuelle() {
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
    p.ok("annee_activer", Some(&admin), json!({ "id": annee["id"] }))
        .await;
    poser_grille(&p, &admin, &annee["id"]).await;
    let ped = p
        .ok("pedagogie_lire", Some(&admin), json!({ "id": annee["id"] }))
        .await;
    let septieme = id_par(&ped["niveaux"], "code", "7");
    let classe = p
        .ok(
            "classe_creer",
            Some(&admin),
            json!({ "annee_id": annee["id"], "niveau_id": septieme, "serie_id": null, "libelle": "7e A" }),
        )
        .await["id"]
        .clone();
    // Awa et Binta à la rentrée (4 octobre) ; Coumba arrive le 15 janvier, au 2e trimestre.
    let awa = inscrire(&p, &admin, &classe, "Awa").await;
    let binta = inscrire(&p, &admin, &classe, "Binta").await;
    p.ok(
        "inscriptions_valider_classe",
        Some(&admin),
        json!({ "id": classe }),
    )
    .await;
    p.horloge.avancer(103 * 86_400_000);
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let coumba = inscrire(&p, &admin, &classe, "Coumba").await;
    p.ok(
        "inscriptions_valider_classe",
        Some(&admin),
        json!({ "id": classe }),
    )
    .await;

    let trimestre = |rang: i64| {
        ped["periodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["periode"]["niveau_id"] == septieme && x["periode"]["rang"] == rang)
            .unwrap()["periode"]["id"]
            .clone()
    };
    let math = id_par(&ped["matieres"], "code", "MATH");
    // Une composition de maths par trimestre ; la grille ne montre que les élèves inscrits le jour
    // de l'évaluation.
    let composer = |rang: i64, date: &'static str| {
        json!({ "classe_id": classe, "periode_id": trimestre(rang), "matiere_id": math,
            "type_evaluation": "composition", "libelle": "Composition", "date": date, "bareme": null })
    };
    let publier = async |rang: i64, date: &'static str, notes: &[(&Value, &str)]| {
        let e = p
            .ok("evaluation_creer", Some(&admin), composer(rang, date))
            .await;
        let grille = p
            .ok("notes_grille", Some(&admin), json!({ "id": e["id"] }))
            .await;
        assert_eq!(grille["lignes"].as_array().unwrap().len(), notes.len());
        for (inscription, texte) in notes {
            p.ok(
                "note_saisir",
                Some(&admin),
                json!({ "evaluation_id": e["id"], "inscription_id": inscription, "saisie": texte }),
            )
            .await;
        }
        p.ok("evaluation_valider", Some(&admin), json!({ "id": e["id"] }))
            .await;
        let demande = json!({ "classe_id": classe, "periode_id": trimestre(rang) });
        let calcul = p
            .ok("resultats_calculer", Some(&admin), demande.clone())
            .await;
        p.ok("bulletins_valider", Some(&admin), demande.clone())
            .await;
        p.ok("bulletins_publier", Some(&admin), demande).await;
        calcul
    };

    // 1er trimestre : Coumba n'était pas là, ni note ni bulletin (Q-27).
    let t1 = publier(1, "2026-11-20", &[(&awa, "12"), (&binta, "10")]).await;
    assert_eq!(t1["bulletins"].as_array().unwrap().len(), 2);
    assert_eq!(t1["bulletins"][0]["bulletin"]["contenu"]["effectif"], 2);
    assert_eq!(
        t1["bulletins"][0]["bulletin"]["contenu"]["moyenne_annuelle"],
        Value::Null
    );

    // Sa moyenne du 1er trimestre est reprise du relevé de son ancienne école (Q-28).
    let demande_t1 = json!({ "classe_id": classe, "periode_id": trimestre(1) });
    let a_reprendre = p
        .ok("moyennes_reprises_lire", Some(&admin), demande_t1.clone())
        .await;
    assert_eq!(a_reprendre["lignes"].as_array().unwrap().len(), 1);
    assert_eq!(a_reprendre["lignes"][0]["eleve"], "Coumba Traoré");
    let reprise = |inscription: &Value, rang: i64, moyenne: &str, origine: &str| {
        json!({ "classe_id": classe, "inscription_id": inscription, "periode_id": trimestre(rang),
            "moyenne": moyenne, "etablissement_origine": origine, "piece": "Relevé n° 42" })
    };
    let r = p
        .ok(
            "moyenne_reprise_saisir",
            Some(&admin),
            reprise(&coumba, 1, "12,50", "École Mamadou Konaté"),
        )
        .await;
    assert_eq!(r["moyenne"], 1250);
    for (demande, cle) in [
        (
            reprise(&awa, 1, "13", "Ailleurs"),
            "moyenne_reprise.periode_suivie",
        ),
        (
            reprise(&coumba, 2, "13", "Ailleurs"),
            "moyenne_reprise.periode_suivie",
        ),
        (reprise(&coumba, 1, "21", "Ailleurs"), "note.hors_bareme"),
        (
            reprise(&coumba, 1, "12", " "),
            "moyenne_reprise.origine_obligatoire",
        ),
    ] {
        assert_eq!(
            p.refus("moyenne_reprise_saisir", Some(&admin), demande)
                .await
                .cle,
            cle
        );
    }

    // 2e trimestre : Coumba arrive pendant la période, porte la date et est classée (Q-29).
    let t2 = publier(
        2,
        "2027-03-10",
        &[(&awa, "13"), (&binta, "11"), (&coumba, "15")],
    )
    .await;
    let coumba_t2 = t2["bulletins"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["bulletin"]["inscription_id"] == coumba)
        .unwrap()["bulletin"]["contenu"]
        .clone();
    assert_eq!(coumba_t2["arrive_le"], "2027-01-15");
    assert_eq!(coumba_t2["rang"]["rang"], 1);
    assert_eq!(
        t2["bulletins"][0]["bulletin"]["contenu"]["arrive_le"],
        Value::Null
    );

    // 3e trimestre, le dernier : la moyenne annuelle de chacun, sur les périodes suivies (T-33).
    let t3 = publier(
        3,
        "2027-05-20",
        &[(&awa, "14"), (&binta, "12"), (&coumba, "16")],
    )
    .await;
    let annuelle = |inscription: &Value| {
        t3["bulletins"]
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["bulletin"]["inscription_id"] == *inscription)
            .unwrap()["bulletin"]["contenu"]
            .clone()
    };
    // Awa (12 + 13 + 14) ÷ 3 = 13,00 ; Binta 11,00 ; Coumba (12,50 + 15 + 16) ÷ 3 = 14,50.
    assert_eq!(annuelle(&awa)["moyenne_annuelle"], 1300);
    assert_eq!(annuelle(&binta)["moyenne_annuelle"], 1100);
    assert_eq!(annuelle(&coumba)["moyenne_annuelle"], 1450);
    assert_eq!(annuelle(&coumba)["rang_annuel"]["rang"], 1);
    assert_eq!(annuelle(&awa)["rang_annuel"]["rang"], 2);
    // Enregistrée partout, imprimée dans les seules classes d'examen par défaut : pas en 7e.
    assert_eq!(annuelle(&awa)["moyenne_annuelle_imprimee"], false);
    // Une reprise se retire ; les bulletins publiés gardent ce qu'ils portaient.
    p.ok(
        "moyenne_reprise_saisir",
        Some(&admin),
        reprise(&coumba, 1, "", "École Mamadou Konaté"),
    )
    .await;
    let demande_t3 = json!({ "classe_id": classe, "periode_id": trimestre(3) });
    let apercu = p.ok("bulletins_produire", Some(&admin), demande_t3).await;
    assert!(apercu["pdf"].as_str().unwrap().len() > 1000);
}
