//! Absences et retards par la frontière (SPEC_PHASE_4 §6, T-37) : relevé d'une journée par
//! demi-journée, codes de saisie, justification après coup, totaux de la période, portée de
//! l'enseignant.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod commun;

use commun::*;
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
async fn eleve(
    p: &Poste_,
    admin: &gesco_services::session::Jeton,
    prenoms: &str,
    complet: bool,
) -> Value {
    let fiche = p
        .ok(
            "eleve_creer",
            Some(admin),
            json!({ "identite": { "nom": "Traoré", "prenoms": prenoms, "sexe": "feminin",
                "date_naissance": "2014-03-02", "date_naissance_estimee": false,
                "lieu_naissance": if complet { "Bamako" } else { "" },
                "nationalite": "Malienne", "adresse": "", "telephones": [] },
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

#[tokio::test]
async fn une_journee_se_releve_puis_se_justifie() {
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
    let classe = |libelle: &'static str| json!({ "annee_id": annee["id"], "niveau_id": septieme, "serie_id": null, "libelle": libelle });
    let a = p.ok("classe_creer", Some(&admin), classe("7e A")).await["id"].clone();
    let b = p.ok("classe_creer", Some(&admin), classe("7e B")).await["id"].clone();
    for prenom in ["Awa", "Binta", "Coumba"] {
        let fiche = eleve(&p, &admin, prenom, true).await;
        p.ok(
            "inscription_creer",
            Some(&admin),
            json!({ "eleve_id": fiche["eleve"]["id"], "classe_id": a, "date_inscription": "",
                "redoublant": false, "etablissement_origine": "" }),
        )
        .await;
    }
    p.ok(
        "inscriptions_valider_classe",
        Some(&admin),
        json!({ "id": a }),
    )
    .await;
    // Inscrits le 4 octobre ; nous sommes le 6.
    p.horloge.avancer(2 * 86_400_000);
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let jour = |date: &str| json!({ "classe_id": a, "date": date });

    // Avant leur inscription, les élèves ne sont pas sur le relevé.
    let avant = p
        .ok("absences_releve_lire", Some(&admin), jour("2026-10-03"))
        .await;
    assert_eq!(avant["lignes"].as_array().unwrap().len(), 0);

    let releve = p
        .ok("absences_releve_lire", Some(&admin), jour("2026-10-05"))
        .await;
    assert_eq!(releve["lignes"].as_array().unwrap().len(), 3);
    assert_eq!(releve["fait_le"], Value::Null);
    assert_eq!(releve["refus"], Value::Null);
    let eleve = |i: usize| releve["lignes"][i]["inscription_id"].clone();
    let saisie = |lignes: Vec<(usize, &str, &str)>| {
        json!({ "classe_id": a, "date": "2026-10-05", "lignes": lignes.into_iter()
            .map(|(i, matin, apres_midi)| json!({ "inscription_id": eleve(i), "matin": matin, "apres_midi": apres_midi }))
            .collect::<Vec<_>>() })
    };

    // Awa absente la journée, Binta en retard d'un quart d'heure le matin.
    let bilan = p
        .ok(
            "absences_releve_enregistrer",
            Some(&admin),
            saisie(vec![(0, "a", "A"), (1, "r15", ""), (2, "", "")]),
        )
        .await;
    assert_eq!(bilan, json!({ "absences": 2, "retards": 1, "changees": 3 }));
    let relu = p
        .ok("absences_releve_lire", Some(&admin), jour("2026-10-05"))
        .await;
    assert!(relu["fait_le"].is_i64());
    assert_eq!(relu["fait_par"], "Awa Traoré");
    assert_eq!(relu["lignes"][0]["matin"]["nature"], "absence");
    assert_eq!(relu["lignes"][0]["apres_midi"]["nature"], "absence");
    assert_eq!(relu["lignes"][1]["matin"]["minutes"], 15);
    assert_eq!(relu["lignes"][1]["apres_midi"], Value::Null);
    // Réenregistrer le même relevé ne change rien.
    let encore = p
        .ok(
            "absences_releve_enregistrer",
            Some(&admin),
            saisie(vec![(0, "a", "a"), (1, "r15", ""), (2, "", "")]),
        )
        .await;
    assert_eq!(encore["changees"], 0);

    // Codes illisibles, jour à venir, élève d'une autre classe : refusés sans rien écrire.
    assert_eq!(
        p.refus(
            "absences_releve_enregistrer",
            Some(&admin),
            saisie(vec![(2, "x", "")])
        )
        .await
        .cle,
        "absence.illisible"
    );
    assert_eq!(
        p.refus(
            "absences_releve_enregistrer",
            Some(&admin),
            saisie(vec![(2, "r500", "")])
        )
        .await
        .cle,
        "absence.retard_invalide"
    );
    let futur = p
        .ok("absences_releve_lire", Some(&admin), jour("2026-10-07"))
        .await;
    assert_eq!(futur["refus"], "absence.date_future");
    assert_eq!(
        p.refus(
            "absences_releve_enregistrer",
            Some(&admin),
            json!({ "classe_id": a, "date": "2026-10-07", "lignes": [] })
        )
        .await
        .cle,
        "absence.date_future"
    );
    assert_eq!(
        p.refus(
            "absences_releve_enregistrer",
            Some(&admin),
            json!({ "classe_id": b, "date": "2026-10-05",
                "lignes": [{ "inscription_id": eleve(0), "matin": "a", "apres_midi": "" }] })
        )
        .await
        .cle,
        "absence.eleve_hors_classe"
    );

    // Le lendemain, la maman apporte un certificat pour la journée d'Awa.
    let cle = |i: usize, demi: &str| json!({ "inscription_id": eleve(i), "date": "2026-10-05", "demi_journee": demi });
    assert_eq!(
        p.refus(
            "absences_justifier",
            Some(&admin),
            json!({ "absences": [cle(0, "matin")], "motif": "  ", "justificatif": "" })
        )
        .await
        .cle,
        "absence.motif_obligatoire"
    );
    let justifiees = p
        .ok(
            "absences_justifier",
            Some(&admin),
            json!({ "absences": [cle(0, "matin"), cle(0, "apres_midi")], "motif": "Maladie",
                "justificatif": "Certificat médical" }),
        )
        .await;
    assert_eq!(justifiees, 2);

    // Les totaux du trimestre, pour le bulletin.
    let trimestre = ped["periodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["periode"]["niveau_id"] == septieme && x["periode"]["rang"] == 1)
        .unwrap()["periode"]["id"]
        .clone();
    let periode = p
        .ok(
            "absences_periode_lire",
            Some(&admin),
            json!({ "classe_id": a, "periode_id": trimestre }),
        )
        .await;
    assert_eq!(periode["jours_releves"], 1);
    assert_eq!(periode["absences"].as_array().unwrap().len(), 3);
    assert_eq!(periode["absences"][0]["absence"]["motif"], "Maladie");
    assert_eq!(
        periode["totaux"][0]["totaux"],
        json!({ "demi_journees": 2, "justifiees": 2, "retards": 0, "minutes_de_retard": 0 })
    );
    assert_eq!(
        periode["totaux"][1]["totaux"],
        json!({ "demi_journees": 0, "justifiees": 0, "retards": 1, "minutes_de_retard": 15 })
    );

    // L'enseignant de maths de la 7e A relève dans sa classe seulement ; il ne justifie pas, et ne
    // change pas une absence justifiée.
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
        json!({ "classe_id": a, "matiere_id": id_par(&ped["matieres"], "code", "MATH"),
            "enseignant_id": sekou["enseignant"]["id"] }),
    )
    .await;
    let jeton = p.connecter("sekou", "enseignant-2026").await;
    let portee = p
        .ok("portee_absences_lire", Some(&jeton), json!(null))
        .await;
    assert_eq!(portee, json!({ "type": "classes", "classes": [a] }));
    p.ok(
        "absences_releve_enregistrer",
        Some(&jeton),
        saisie(vec![(2, "", "a")]),
    )
    .await;
    assert_eq!(
        p.refus(
            "absences_releve_enregistrer",
            Some(&jeton),
            saisie(vec![(0, "", "a")])
        )
        .await
        .cle,
        "absence.deja_justifiee"
    );
    assert_eq!(
        p.refus(
            "absences_releve_lire",
            Some(&jeton),
            json!({ "classe_id": b, "date": "2026-10-05" })
        )
        .await
        .cle,
        "erreur.portee_refusee"
    );
    assert_eq!(
        p.refus(
            "absences_justifier",
            Some(&jeton),
            json!({ "absences": [cle(2, "apres_midi")], "motif": "Maladie", "justificatif": "" })
        )
        .await
        .cle,
        "erreur.permission_refusee"
    );
}

/// SPEC_VERSION_1_1 §1, T-71 : réglé par l'école, le relevé se fait à chaque séance de l'emploi du
/// temps ; absente à toutes les séances du matin, Awa compte la demi-journée ; absente à une
/// seule, Binta ne la compte pas.
#[tokio::test]
async fn l_absence_a_toutes_les_seances_du_matin_compte_la_demi_journee() {
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
    let mut inscriptions = Vec::new();
    for prenom in ["Awa", "Binta"] {
        let fiche = eleve(&p, &admin, prenom, true).await;
        inscriptions.push(
            p.ok(
                "inscription_creer",
                Some(&admin),
                json!({ "eleve_id": fiche["eleve"]["id"], "classe_id": classe, "date_inscription": "",
                    "redoublant": false, "etablissement_origine": "" }),
            )
            .await["inscription"]["id"]
                .clone(),
        );
    }
    p.ok(
        "inscriptions_valider_classe",
        Some(&admin),
        json!({ "id": classe }),
    )
    .await;
    // Le lundi : 8 h–9 h et 9 h–10 h, puis 15 h–16 h.
    let e = p
        .ok("emploi_du_temps_lire", Some(&admin), Value::Null)
        .await;
    let matieres = e["classes"][0]["matieres"].as_array().unwrap().clone();
    let mut seances = Vec::new();
    for (i, (debut, fin)) in [(480, 540), (540, 600), (900, 960)].into_iter().enumerate() {
        let e = p
            .ok(
                "seance_placer",
                Some(&admin),
                json!({ "id": null, "classe_id": classe, "jour": 1, "debut": debut, "fin": fin,
                    "matiere_id": matieres[i]["matiere_id"], "enseignant_id": null, "salle": "" }),
            )
            .await;
        seances.push(
            e["seances"]
                .as_array()
                .unwrap()
                .iter()
                .find(|s| s["seance"]["plage"]["debut"] == debut)
                .unwrap()["seance"]["id"]
                .clone(),
        );
    }
    // Inscrites le 4 octobre ; nous sommes le mardi 6 : le lundi 5 se relève.
    p.horloge.avancer(2 * 86_400_000);
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let jour = json!({ "classe_id": classe, "date": "2026-10-05" });
    let saisie = json!({ "classe_id": classe, "date": "2026-10-05", "lignes": [
        { "inscription_id": inscriptions[0], "absent_a": [seances[0], seances[1]] },
        { "inscription_id": inscriptions[1], "absent_a": [seances[0]] },
    ] });
    assert_eq!(
        p.refus("absences_seances_enregistrer", Some(&admin), saisie.clone())
            .await
            .cle,
        "absence.par_seance_inactif"
    );
    p.ok(
        "parametre_definir",
        Some(&admin),
        json!({ "cle": "absence.par_seance", "valeur": { "type": "booleen", "valeur": true } }),
    )
    .await;
    let lu = p
        .ok("absences_seances_lire", Some(&admin), jour.clone())
        .await;
    assert_eq!(lu["actif"], true);
    assert_eq!(lu["seances"].as_array().unwrap().len(), 3);
    assert_eq!(lu["seances"][0]["horaire"], "08:00 – 09:00");
    let lu = p
        .ok("absences_seances_enregistrer", Some(&admin), saisie)
        .await;
    assert_eq!(lu["lignes"][0]["absent_a"].as_array().unwrap().len(), 2);
    let releve = p.ok("absences_releve_lire", Some(&admin), jour).await;
    let ligne = |i: usize| {
        releve["lignes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|l| l["inscription_id"] == inscriptions[i])
            .unwrap()
            .clone()
    };
    assert_eq!(ligne(0)["matin"]["nature"], "absence");
    assert_eq!(ligne(0)["apres_midi"], Value::Null);
    assert_eq!(ligne(1)["matin"], Value::Null);
}
