//! Inventaire de l'école par la frontière (SPEC_PHASE_9 §8.4, §9.2) : les articles livrés sont
//! posés à l'installation ; une ligne se saisit par cycle ou pour toute l'école, se vide sans
//! disparaître ; l'an dernier est montré puis repris à la demande ; un manuel s'ajoute, se
//! renomme, se retire ; un article livré ne se renomme pas ; l'onglet Locaux et équipements des
//! statistiques additionne les lignes ; le droit est à part et se donne à n'importe quel rôle.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use crate::commun::*;
use serde_json::{Value, json};

fn article<'a>(inventaire: &'a Value, code: &str) -> &'a Value {
    inventaire["articles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["code"] == code)
        .unwrap_or_else(|| panic!("article {code} absent"))
}

fn ligne<'a>(lignes: &'a Value, article_id: &Value) -> Option<&'a Value> {
    lignes
        .as_array()
        .unwrap()
        .iter()
        .find(|l| &l["article_id"] == article_id)
}

fn quantites(existant: Option<u32>, bon_etat: Option<u32>, besoins: Option<u32>) -> Value {
    json!({
        "existant": existant, "bon_etat": bon_etat, "besoins": besoins, "dur": null,
        "semi_dur": null, "banco": null, "autre_construction": null, "neufs": null,
        "en_location": null, "murs_bons": null, "murs_mauvais": null, "toits_bons": null,
        "toits_mauvais": null, "cabines_filles": null, "cabines_garcons": null,
        "cabines_enseignants": null,
    })
}

fn tableau<'a>(onglet: &'a Value, code: &str) -> &'a Value {
    onglet["tableaux"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["code"] == code)
        .unwrap_or_else(|| panic!("tableau {code} absent"))
}

#[tokio::test]
async fn l_inventaire_se_saisit_se_reprend_et_se_compte() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;

    // Une école sans année : le catalogue livré est là, sans ligne ; aucun refus.
    let inv = p.ok("inventaire_lire", Some(&admin), json!({})).await;
    assert_eq!(inv["annee_id"], Value::Null);
    assert!(inv["articles"].as_array().unwrap().len() > 30);
    assert!(inv["lignes"].as_array().unwrap().is_empty());
    let salles = article(&inv, "salle_classe")["id"].clone();
    let tables = article(&inv, "table_banc_2")["id"].clone();
    let latrines = article(&inv, "latrines")["id"].clone();
    assert_eq!(article(&inv, "salle_classe")["livre"], true);

    let a1 = p
        .ok(
            "annee_creer",
            Some(&admin),
            json!({ "libelle": "2026-2027", "debut": RENTREE_2026, "fin": FIN_2027 }),
        )
        .await;
    p.ok("annee_activer", Some(&admin), json!({ "id": a1["id"] }))
        .await;

    // Une ligne pour le premier cycle, une pour toute l'école : chacune reste à sa place.
    let mut salles_premier = quantites(Some(6), Some(5), Some(2));
    salles_premier["dur"] = json!(4);
    salles_premier["banco"] = json!(2);
    let inv = p
        .ok(
            "inventaire_releve_enregistrer",
            Some(&admin),
            json!({ "annee_id": a1["id"], "releve": { "article_id": salles,
                "cycle": "premier_cycle", "quantites": salles_premier,
                "observation": "  Deux salles en banco à refaire  " } }),
        )
        .await;
    assert_eq!(inv["cycle"], "premier_cycle");
    let l = ligne(&inv["lignes"], &salles).unwrap();
    assert_eq!(l["quantites"]["existant"], 6);
    assert_eq!(l["quantites"]["semi_dur"], Value::Null);
    assert_eq!(l["observation"], "Deux salles en banco à refaire");
    p.ok(
        "inventaire_releve_enregistrer",
        Some(&admin),
        json!({ "annee_id": a1["id"], "releve": { "article_id": salles, "cycle": "second_cycle",
            "quantites": quantites(Some(3), None, None), "observation": "" } }),
    )
    .await;
    let mut cabines = quantites(Some(2), None, Some(1));
    cabines["cabines_filles"] = json!(4);
    cabines["cabines_garcons"] = json!(4);
    let inv = p
        .ok(
            "inventaire_releve_enregistrer",
            Some(&admin),
            json!({ "annee_id": a1["id"], "releve": { "article_id": latrines, "cycle": null,
                "quantites": cabines, "observation": "" } }),
        )
        .await;
    assert_eq!(inv["cycle"], Value::Null);
    assert!(ligne(&inv["lignes"], &salles).is_none());
    assert!(ligne(&inv["lignes"], &latrines).is_some());
    p.ok(
        "inventaire_releve_enregistrer",
        Some(&admin),
        json!({ "annee_id": a1["id"], "releve": { "article_id": tables, "cycle": null,
            "quantites": quantites(Some(120), Some(100), Some(30)), "observation": "" } }),
    )
    .await;

    // Une quantité trop grande est refusée ; vider une ligne la garde, vide.
    assert_eq!(
        p.refus(
            "inventaire_releve_enregistrer",
            Some(&admin),
            json!({ "annee_id": a1["id"], "releve": { "article_id": tables, "cycle": null,
                "quantites": quantites(Some(2_000_000), None, None), "observation": "" } }),
        )
        .await
        .cle,
        "inventaire.quantite_trop_grande"
    );

    // Un manuel s'ajoute, se renomme ; un article livré ne se renomme pas ; un doublon non plus.
    let manuel = p
        .ok(
            "article_inventaire_enregistrer",
            Some(&admin),
            json!({ "id": null, "famille": "manuel", "designation": "Lecture 1re année",
                "matiere": "Lecture", "niveau_id": null }),
        )
        .await;
    assert_eq!(manuel["livre"], false);
    assert_eq!(manuel["famille"], "manuel");
    let manuel = p
        .ok(
            "article_inventaire_enregistrer",
            Some(&admin),
            json!({ "id": manuel["id"], "famille": "manuel", "designation": "Je lis, 1re année",
                "matiere": "Lecture", "niveau_id": null }),
        )
        .await;
    assert_eq!(manuel["designation"], "Je lis, 1re année");
    assert_eq!(
        p.refus(
            "article_inventaire_enregistrer",
            Some(&admin),
            json!({ "id": salles, "famille": "local", "designation": "Classes",
                "matiere": "", "niveau_id": null }),
        )
        .await
        .cle,
        "inventaire.article_livre"
    );
    assert_eq!(
        p.refus(
            "article_inventaire_enregistrer",
            Some(&admin),
            json!({ "id": null, "famille": "manuel", "designation": "je lis, 1re ANNÉE",
                "matiere": "", "niveau_id": null }),
        )
        .await
        .cle,
        "inventaire.designation_existante"
    );
    p.ok(
        "inventaire_releve_enregistrer",
        Some(&admin),
        json!({ "annee_id": a1["id"], "releve": { "article_id": manuel["id"], "cycle": null,
            "quantites": quantites(Some(40), None, Some(10)), "observation": "" } }),
    )
    .await;

    // L'onglet Locaux et équipements : toute l'école additionne les cycles ; un cycle n'a que
    // ses lignes ; une case jamais saisie reste vide.
    let stats = p
        .ok(
            "statistiques_lire",
            Some(&admin),
            json!({ "annee_id": a1["id"], "date": null, "cycle": null }),
        )
        .await;
    let locaux = tableau(&stats["locaux"], "inventaire_locaux");
    let colonnes: Vec<&str> = locaux["colonnes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap())
        .collect();
    let col = |nom: &str| colonnes.iter().position(|c| *c == nom).unwrap();
    let salle = locaux["lignes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["libelle"] == "Salles de classe utilisées")
        .unwrap();
    assert_eq!(salle["cellules"][col("existant")]["nombre"], 9);
    assert_eq!(salle["cellules"][col("bon_etat")]["nombre"], 5);
    assert_eq!(salle["cellules"][col("semi_dur")]["nombre"], Value::Null);
    assert_eq!(
        salle["cellules"][col("observation")]["texte"],
        "Deux salles en banco à refaire"
    );
    let latrines_stat = tableau(&stats["locaux"], "inventaire_latrines");
    assert_eq!(latrines_stat["lignes"][0]["cellules"][0]["nombre"], 4);
    assert_eq!(
        latrines_stat["lignes"][0]["cellules"][2]["nombre"],
        Value::Null
    );
    let mobilier = tableau(&stats["locaux"], "inventaire_mobilier");
    assert_eq!(mobilier["total"]["cellules"][0]["nombre"], 120);
    let manuels = tableau(&stats["locaux"], "inventaire_manuels");
    assert_eq!(
        manuels["lignes"][0]["libelle"],
        "Je lis, 1re année (Lecture)"
    );
    let stats_second = p
        .ok(
            "statistiques_lire",
            Some(&admin),
            json!({ "annee_id": a1["id"], "date": null, "cycle": "second_cycle" }),
        )
        .await;
    let locaux_second = tableau(&stats_second["locaux"], "inventaire_locaux");
    assert_eq!(locaux_second["lignes"].as_array().unwrap().len(), 1);
    assert_eq!(locaux_second["lignes"][0]["cellules"][0]["nombre"], 3);
    assert!(
        tableau(&stats_second["locaux"], "inventaire_mobilier")["lignes"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    // L'année suivante : l'an dernier est montré, rien n'est recopié tant qu'on ne le demande
    // pas ; « Reprendre » ne touche pas une ligne déjà saisie ni un article retiré.
    let a2 = p
        .ok(
            "annee_creer",
            Some(&admin),
            json!({ "libelle": "2027-2028", "debut": RENTREE_2027, "fin": FIN_2028 }),
        )
        .await;
    let inv = p
        .ok(
            "inventaire_lire",
            Some(&admin),
            json!({ "annee_id": a2["id"], "cycle": null }),
        )
        .await;
    assert!(inv["lignes"].as_array().unwrap().is_empty());
    assert_eq!(
        ligne(&inv["an_dernier"], &tables).unwrap()["quantites"]["existant"],
        120
    );
    p.ok(
        "inventaire_releve_enregistrer",
        Some(&admin),
        json!({ "annee_id": a2["id"], "releve": { "article_id": tables, "cycle": null,
            "quantites": quantites(Some(150), None, None), "observation": "" } }),
    )
    .await;
    p.ok(
        "article_inventaire_activer",
        Some(&admin),
        json!({ "id": manuel["id"], "actif": false }),
    )
    .await;
    let inv = p
        .ok(
            "inventaire_reprendre",
            Some(&admin),
            json!({ "annee_id": a2["id"], "cycle": null }),
        )
        .await;
    assert_eq!(
        ligne(&inv["lignes"], &tables).unwrap()["quantites"]["existant"],
        150
    );
    assert_eq!(
        ligne(&inv["lignes"], &latrines).unwrap()["quantites"]["cabines_filles"],
        4
    );
    assert!(ligne(&inv["lignes"], &manuel["id"]).is_none());
    // Le premier cycle a sa propre reprise.
    let inv = p
        .ok(
            "inventaire_lire",
            Some(&admin),
            json!({ "annee_id": a2["id"], "cycle": "premier_cycle" }),
        )
        .await;
    assert!(ligne(&inv["lignes"], &salles).is_none());
    assert!(ligne(&inv["an_dernier"], &salles).is_some());

    // Un manuel retiré revient avec ses lignes.
    let remis = p
        .ok(
            "article_inventaire_activer",
            Some(&admin),
            json!({ "id": manuel["id"], "actif": true }),
        )
        .await;
    assert_eq!(remis["retire_le"], Value::Null);
}

#[tokio::test]
async fn le_droit_de_l_inventaire_est_a_part() {
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
    let secretaire = p.connecter("secretaire", "mot-de-passe-2026").await;
    p.ok("inventaire_lire", Some(&secretaire), json!({})).await;
    let caissier = p.connecter("caissier", "mot-de-passe-2026").await;
    assert_eq!(
        p.refus("inventaire_lire", Some(&caissier), json!({}))
            .await
            .cle,
        "erreur.permission_refusee"
    );
    assert_eq!(
        p.refus(
            "article_inventaire_enregistrer",
            Some(&caissier),
            json!({ "id": null, "famille": "manuel", "designation": "Calcul",
                "matiere": "", "niveau_id": null }),
        )
        .await
        .cle,
        "erreur.permission_refusee"
    );
}
