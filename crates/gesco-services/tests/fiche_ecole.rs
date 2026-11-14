//! Fiche de l'établissement et rubriques par la frontière (SPEC_PHASE_9 §8.1, critères 6 et 9) :
//! les rubriques livrées sont posées à l'installation ; une réponse se saisit, se vide ; une
//! réponse annuelle de l'an dernier est proposée sans être recopiée, puis reprise à la demande ;
//! une rubrique de l'école s'ajoute, se modifie, se désactive ; une rubrique livrée ne se
//! renomme pas ; le droit est tenu.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use crate::commun::*;
use serde_json::{Value, json};

fn rubrique<'a>(fiche: &'a Value, code: &str) -> &'a Value {
    fiche["rubriques"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["code"] == code)
        .unwrap_or_else(|| panic!("rubrique {code} absente"))
}

fn reponse<'a>(fiche: &'a Value, rubrique_id: &Value) -> Option<&'a Value> {
    fiche["reponses"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| &r["rubrique_id"] == rubrique_id)
}

#[tokio::test]
async fn la_fiche_se_remplit_se_reprend_et_s_enrichit() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;

    // Une école neuve, sans année : les rubriques livrées sont là, sans réponse.
    let fiche = p.ok("fiche_ecole_lire", Some(&admin), json!({})).await;
    assert_eq!(fiche["annee_id"], Value::Null);
    assert!(fiche["rubriques"].as_array().unwrap().len() > 50);
    assert!(fiche["reponses"].as_array().unwrap().is_empty());
    let cercle = rubrique(&fiche, "cercle")["id"].clone();
    let point_eau = rubrique(&fiche, "point_eau")["id"].clone();
    assert_eq!(rubrique(&fiche, "point_eau")["annuelle"], true);
    assert_eq!(rubrique(&fiche, "cercle")["annuelle"], false);

    // Une rubrique permanente se saisit sans année ; une annuelle demande une année.
    let fiche = p
        .ok(
            "fiche_ecole_repondre",
            Some(&admin),
            json!({ "rubrique_id": cercle, "annee_id": null, "valeur": "  Kati  " }),
        )
        .await;
    assert_eq!(reponse(&fiche, &cercle).unwrap()["valeur"], "Kati");
    assert_eq!(
        p.refus(
            "fiche_ecole_repondre",
            Some(&admin),
            json!({ "rubrique_id": point_eau, "annee_id": null, "valeur": "oui" }),
        )
        .await
        .cle,
        "fiche.annee_requise"
    );

    // Deux années : la réponse de la première est proposée dans la seconde, pas recopiée.
    let a1 = p
        .ok(
            "annee_creer",
            Some(&admin),
            json!({ "libelle": "2026-2027", "debut": RENTREE_2026, "fin": FIN_2027 }),
        )
        .await;
    let a2 = p
        .ok(
            "annee_creer",
            Some(&admin),
            json!({ "libelle": "2027-2028", "debut": RENTREE_2027, "fin": FIN_2028 }),
        )
        .await;
    let fiche = p
        .ok(
            "fiche_ecole_repondre",
            Some(&admin),
            json!({ "rubrique_id": point_eau, "annee_id": a1["id"], "valeur": "oui" }),
        )
        .await;
    assert_eq!(fiche["annee_id"], a1["id"]);
    assert_eq!(reponse(&fiche, &point_eau).unwrap()["valeur"], "oui");
    let fiche = p
        .ok(
            "fiche_ecole_lire",
            Some(&admin),
            json!({ "annee_id": a2["id"] }),
        )
        .await;
    let r = reponse(&fiche, &point_eau).unwrap();
    assert_eq!(r["valeur"], Value::Null);
    assert_eq!(r["proposee"], "oui");
    // La réponse permanente vaut pour toutes les années.
    assert_eq!(reponse(&fiche, &cercle).unwrap()["valeur"], "Kati");

    // Reprise : la proposition devient la réponse de l'année ; on la modifie ensuite à part.
    let fiche = p
        .ok(
            "fiche_ecole_reprendre",
            Some(&admin),
            json!({ "annee_id": a2["id"], "rubriques": null }),
        )
        .await;
    let r = reponse(&fiche, &point_eau).unwrap();
    assert_eq!(r["valeur"], "oui");
    assert_eq!(r["proposee"], Value::Null);
    p.ok(
        "fiche_ecole_repondre",
        Some(&admin),
        json!({ "rubrique_id": point_eau, "annee_id": a2["id"], "valeur": "non" }),
    )
    .await;
    let fiche = p
        .ok(
            "fiche_ecole_lire",
            Some(&admin),
            json!({ "annee_id": a1["id"] }),
        )
        .await;
    assert_eq!(reponse(&fiche, &point_eau).unwrap()["valeur"], "oui");

    // Une réponse se vide ; une réponse hors des choix est refusée.
    let fiche = p
        .ok(
            "fiche_ecole_repondre",
            Some(&admin),
            json!({ "rubrique_id": cercle, "annee_id": null, "valeur": "" }),
        )
        .await;
    assert!(reponse(&fiche, &cercle).is_none());
    let milieu = rubrique(&fiche, "milieu")["id"].clone();
    assert_eq!(
        p.refus(
            "fiche_ecole_repondre",
            Some(&admin),
            json!({ "rubrique_id": milieu, "annee_id": null, "valeur": "Lunaire" }),
        )
        .await
        .cle,
        "fiche.reponse_invalide"
    );

    // Une rubrique de l'école : elle s'ajoute, se saisit, se modifie, se désactive, revient.
    let ajoutee = p
        .ok(
            "rubrique_fiche_enregistrer",
            Some(&admin),
            json!({ "id": null, "theme": "autres", "libelle": "Jardin potager",
                "aide": "", "type_reponse": "choix", "choix": ["Oui", "Non", "Oui"],
                "annuelle": false }),
        )
        .await;
    assert_eq!(ajoutee["livree"], false);
    assert_eq!(ajoutee["choix"], json!(["Oui", "Non"]));
    let fiche = p
        .ok(
            "fiche_ecole_repondre",
            Some(&admin),
            json!({ "rubrique_id": ajoutee["id"], "annee_id": null, "valeur": "Oui" }),
        )
        .await;
    assert_eq!(reponse(&fiche, &ajoutee["id"]).unwrap()["valeur"], "Oui");
    let modifiee = p
        .ok(
            "rubrique_fiche_enregistrer",
            Some(&admin),
            json!({ "id": ajoutee["id"], "theme": "equipements", "libelle": "Jardin scolaire",
                "aide": "Potager ou verger", "type_reponse": "choix", "choix": ["Oui", "Non"],
                "annuelle": false }),
        )
        .await;
    assert_eq!(modifiee["libelle"], "Jardin scolaire");
    assert_eq!(modifiee["theme"], "equipements");
    let desactivee = p
        .ok(
            "rubrique_fiche_activer",
            Some(&admin),
            json!({ "id": ajoutee["id"], "active": false }),
        )
        .await;
    assert!(!desactivee["retiree_le"].is_null());
    let remise = p
        .ok(
            "rubrique_fiche_activer",
            Some(&admin),
            json!({ "id": ajoutee["id"], "active": true }),
        )
        .await;
    assert!(remise["retiree_le"].is_null());
    // Sa réponse est restée.
    let fiche = p.ok("fiche_ecole_lire", Some(&admin), json!({})).await;
    assert_eq!(reponse(&fiche, &ajoutee["id"]).unwrap()["valeur"], "Oui");

    // Une rubrique livrée ne se renomme pas ; elle se désactive.
    assert_eq!(
        p.refus(
            "rubrique_fiche_enregistrer",
            Some(&admin),
            json!({ "id": cercle, "theme": "localisation", "libelle": "Arrondissement",
                "aide": "", "type_reponse": "texte", "choix": [], "annuelle": false }),
        )
        .await
        .cle,
        "fiche.rubrique_livree"
    );
    p.ok(
        "rubrique_fiche_activer",
        Some(&admin),
        json!({ "id": cercle, "active": false }),
    )
    .await;
    // Deux rubriques ne portent pas le même libellé.
    assert_eq!(
        p.refus(
            "rubrique_fiche_enregistrer",
            Some(&admin),
            json!({ "id": null, "theme": "autres", "libelle": "jardin SCOLAIRE",
                "aide": "", "type_reponse": "texte", "choix": [], "annuelle": true }),
        )
        .await
        .cle,
        "fiche.libelle_existant"
    );
}

#[tokio::test]
async fn le_droit_de_la_fiche_est_tenu() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    for (identifiant, role) in [("caissier", "caissier"), ("secretaire", "secretaire")] {
        p.ok(
            "utilisateur_creer",
            Some(&admin),
            json!({ "identifiant": identifiant, "nom_affiche": identifiant,
                "mot_de_passe_initial": "mot-de-passe-2026", "roles": [role] }),
        )
        .await;
    }
    let caissier = p.connecter("caissier", "mot-de-passe-2026").await;
    assert_eq!(
        p.refus("fiche_ecole_lire", Some(&caissier), json!({}))
            .await
            .cle,
        "erreur.permission_refusee"
    );
    let secretaire = p.connecter("secretaire", "mot-de-passe-2026").await;
    let fiche = p.ok("fiche_ecole_lire", Some(&secretaire), json!({})).await;
    let sigle = rubrique(&fiche, "sigle")["id"].clone();
    p.ok(
        "fiche_ecole_repondre",
        Some(&secretaire),
        json!({ "rubrique_id": sigle, "annee_id": null, "valeur": "GSE" }),
    )
    .await;
    assert_eq!(
        p.refus(
            "fiche_ecole_repondre",
            Some(&caissier),
            json!({ "rubrique_id": sigle, "annee_id": null, "valeur": "X" }),
        )
        .await
        .cle,
        "erreur.permission_refusee"
    );
}
