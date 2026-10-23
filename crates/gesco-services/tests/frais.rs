//! Grilles de frais et plans de paiement par la frontière (SPEC_PHASE_3 §3, assertion 2).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod commun;

use commun::*;
use gesco_services::session::Jeton;
use serde_json::{Value, json};

/// Poste installé, administrateur connecté, année 2026-2027 ouverte, classe 7e A.
async fn ecole(dossier: &std::path::Path) -> (Poste_, Jeton, Value, Value) {
    let p = poste_vierge(dossier).await;
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
    let referentiel = p.ok("niveaux_lister", Some(&admin), Value::Null).await;
    p.ok(
        "classe_creer",
        Some(&admin),
        json!({ "annee_id": annee["id"], "niveau_id": niveau(&referentiel, "7"), "serie_id": null, "libelle": "7e A" }),
    )
    .await;
    p.ok(
        "classe_creer",
        Some(&admin),
        json!({ "annee_id": annee["id"], "niveau_id": niveau(&referentiel, "8"), "serie_id": null, "libelle": "8e A" }),
    )
    .await;
    (p, admin, annee, referentiel)
}

fn niveau(referentiel: &Value, code: &str) -> Value {
    referentiel["niveaux"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["code"] == code)
        .unwrap()["id"]
        .clone()
}

fn ligne(niveau: &Value, type_frais: &str, libelle: &str, montant: i64) -> Value {
    json!({
        "niveau_id": niveau,
        "serie_id": null,
        "type_frais": type_frais,
        "libelle": libelle,
        "montant": montant,
        "obligatoire": type_frais != "cantine",
        "exigibilite": if type_frais == "scolarite" { "selon_plan" } else { "une_fois" },
    })
}

#[tokio::test]
async fn une_grille_se_saisit_se_met_en_vigueur_et_se_remplace() {
    let dossier = tempfile::tempdir().unwrap();
    let (p, admin, annee, referentiel) = ecole(dossier.path()).await;
    let sept = niveau(&referentiel, "7");

    // Grille neuve : aucun frais, les trois plans livrés, « Trois tranches » par défaut.
    let grille = p
        .ok(
            "grille_creer",
            Some(&admin),
            json!({ "annee_id": annee["id"], "copie_de": null }),
        )
        .await;
    assert_eq!(grille["grille"]["libelle"], "Frais 2026-2027");
    assert_eq!(grille["grille"]["etat"], "brouillon");
    assert_eq!(grille["contenu"]["lignes"], json!([]));
    let plans = grille["contenu"]["plans"].as_array().unwrap().clone();
    assert_eq!(plans.len(), 3);
    assert_eq!(plans[1]["libelle"], "Trois tranches");
    assert_eq!(plans[1]["par_defaut"], true);
    assert_eq!(plans[1]["tranches"][0]["echeance"], "2026-10-05");

    // Une grille vide ne se met pas en vigueur.
    let id = json!({ "id": grille["grille"]["id"] });
    assert_eq!(
        p.refus("grille_mettre_en_vigueur", Some(&admin), id.clone())
            .await
            .cle,
        "frais.grille_vide"
    );

    // Saisie : inscription, scolarité, cantine facultative en 7e ; deux plans seulement.
    let saisie_plans = json!([
        { "libelle": "Comptant", "par_defaut": false, "tranches": [
            { "libelle": "Paiement unique", "echeance": "2026-10-15", "part": 100 } ] },
        { "libelle": "Deux fois", "par_defaut": true, "tranches": [
            { "libelle": "Octobre", "echeance": "2026-10-15", "part": 60 },
            { "libelle": "Février", "echeance": "2027-02-15", "part": 40 } ] },
    ]);
    let modification = |lignes: Value, plans: Value| json!({ "id": grille["grille"]["id"], "libelle": "Frais 2026-2027", "lignes": lignes, "plans": plans });
    let lignes = json!([
        ligne(&sept, "inscription", "Frais d'inscription", 10_000),
        ligne(&sept, "scolarite", "Scolarité", 150_000),
        ligne(&sept, "cantine", "Cantine", 45_000),
    ]);
    let modifiee = p
        .ok(
            "grille_modifier",
            Some(&admin),
            modification(lignes.clone(), saisie_plans.clone()),
        )
        .await;
    assert_eq!(modifiee["contenu"]["lignes"].as_array().unwrap().len(), 3);
    assert_eq!(modifiee["contenu"]["lignes"][1]["montant"], 150_000);
    assert_eq!(modifiee["contenu"]["plans"][1]["libelle"], "Deux fois");

    // Les refus du noyau : parts, plan par défaut, montant négatif, série d'un autre niveau.
    let mut parts = saisie_plans.clone();
    parts[1]["tranches"][1]["part"] = json!(30);
    let mut sans_defaut = saisie_plans.clone();
    sans_defaut[1]["par_defaut"] = json!(false);
    let mut negative = lignes.clone();
    negative[0]["montant"] = json!(-5);
    let serie_s = referentiel["series"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["code"] == "S")
        .unwrap()["id"]
        .clone();
    let mut serie = lignes.clone();
    serie[1]["serie_id"] = serie_s;
    for (entree, cle) in [
        (
            modification(lignes.clone(), parts),
            "frais.parts_differentes_de_cent",
        ),
        (
            modification(lignes.clone(), sans_defaut),
            "frais.plan_par_defaut",
        ),
        (
            modification(negative, saisie_plans.clone()),
            "frais.montant_negatif",
        ),
        (
            modification(serie, saisie_plans.clone()),
            "frais.serie_hors_niveau",
        ),
    ] {
        assert_eq!(
            p.refus("grille_modifier", Some(&admin), entree).await.cle,
            cle
        );
    }

    // Mise en vigueur : la 8e a une classe et pas de scolarité, elle est signalée.
    let vigueur = p
        .ok("grille_mettre_en_vigueur", Some(&admin), id.clone())
        .await;
    assert_eq!(vigueur["grille"]["grille"]["etat"], "en_vigueur");
    assert_eq!(vigueur["niveaux_sans_scolarite"], json!(["8e année"]));
    // En vigueur, elle ne se modifie plus.
    assert_eq!(
        p.refus(
            "grille_modifier",
            Some(&admin),
            modification(lignes.clone(), saisie_plans.clone())
        )
        .await
        .cle,
        "frais.grille_figee"
    );

    // Nouvelle version par copie : même contenu, puis en vigueur ; la première est remplacée.
    let copie = p
        .ok(
            "grille_creer",
            Some(&admin),
            json!({ "annee_id": annee["id"], "copie_de": grille["grille"]["id"] }),
        )
        .await;
    assert_eq!(copie["grille"]["version"], 2);
    assert_eq!(copie["grille"]["libelle"], "Frais 2026-2027 (version 2)");
    assert_eq!(copie["contenu"]["lignes"].as_array().unwrap().len(), 3);
    assert_eq!(
        copie["contenu"]["plans"][1]["tranches"][1]["echeance"],
        "2027-02-15"
    );
    assert_ne!(
        copie["contenu"]["lignes"][0]["id"],
        modifiee["contenu"]["lignes"][0]["id"]
    );
    p.ok(
        "grille_mettre_en_vigueur",
        Some(&admin),
        json!({ "id": copie["grille"]["id"] }),
    )
    .await;
    let grilles = p
        .ok(
            "grilles_lister",
            Some(&admin),
            json!({ "annee_id": annee["id"] }),
        )
        .await;
    let etats: Vec<&str> = grilles
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["grille"]["etat"].as_str().unwrap())
        .collect();
    assert_eq!(etats, vec!["en_vigueur", "remplacee"]);

    // L'année suivante recopie la grille, échéances décalées d'un an.
    let suivante = p
        .ok(
            "annee_creer",
            Some(&admin),
            json!({ "libelle": "2027-2028", "debut": RENTREE_2027, "fin": FIN_2028 }),
        )
        .await;
    let recopiee = p
        .ok(
            "grille_creer",
            Some(&admin),
            json!({ "annee_id": suivante["id"], "copie_de": copie["grille"]["id"] }),
        )
        .await;
    assert_eq!(recopiee["grille"]["version"], 1);
    assert_eq!(
        recopiee["contenu"]["plans"][1]["tranches"][1]["echeance"],
        "2028-02-15"
    );

    // Une secrétaire lit les grilles mais ne les saisit pas.
    p.ok(
        "utilisateur_creer",
        Some(&admin),
        json!({
            "identifiant": "fanta",
            "nom_affiche": "Fanta Coulibaly",
            "mot_de_passe_initial": "secretariat-2026",
            "roles": ["secretaire"],
        }),
    )
    .await;
    let secretaire = p.connecter("fanta", "secretariat-2026").await;
    p.ok(
        "grilles_lister",
        Some(&secretaire),
        json!({ "annee_id": annee["id"] }),
    )
    .await;
    assert_eq!(
        p.refus(
            "grille_creer",
            Some(&secretaire),
            json!({ "annee_id": annee["id"], "copie_de": null })
        )
        .await
        .cle,
        "erreur.permission_refusee"
    );
}
