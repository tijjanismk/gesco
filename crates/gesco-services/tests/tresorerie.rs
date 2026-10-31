//! Trésorerie par la frontière (SPEC_PHASE_8 §2 et §5) : catégories livrées que l'école renomme,
//! complète et retire ; la caisse livrée, un compte en banque et un compte mobile money ajoutés ;
//! le seuil d'accord des dépenses et la cantine réglés.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use crate::commun::*;
use serde_json::{Value, json};

fn categorie<'a>(t: &'a Value, libelle: &str) -> &'a Value {
    t["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["libelle"] == libelle)
        .unwrap_or_else(|| panic!("{libelle} absente"))
}

#[tokio::test]
async fn les_categories_et_les_comptes_de_l_ecole_se_reglent() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;

    // Livrés : huit catégories de recettes, dix de dépenses, la caisse, un seuil de 50 000 F.
    let t = p.ok("tresorerie_lire", Some(&admin), Value::Null).await;
    let sens = |s: &str| {
        t["categories"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["sens"] == s)
            .count()
    };
    assert_eq!(sens("recette"), 8);
    assert_eq!(sens("depense"), 10);
    assert_eq!(categorie(&t, "Salaires")["code"], "salaires");
    assert_eq!(t["comptes"].as_array().unwrap().len(), 1);
    assert_eq!(t["comptes"][0]["nature"], "caisse");
    assert_eq!(
        t["reglages"],
        json!({ "seuil_accord": 50_000, "cantine_active": false, "repas_comptes": false })
    );

    // L'école ajoute une catégorie, en renomme une, en retire une ; deux libellés égaux, non.
    let enregistrer = |id: Value, sens: &str, libelle: &str, active: bool| json!({ "id": id, "sens": sens, "libelle": libelle, "active": active });
    let t = p
        .ok(
            "categorie_tresorerie_enregistrer",
            Some(&admin),
            enregistrer(Value::Null, "depense", "Communication", true),
        )
        .await;
    let divers = categorie(&t, "Divers")["id"].clone();
    let t = p
        .ok(
            "categorie_tresorerie_enregistrer",
            Some(&admin),
            enregistrer(divers.clone(), "depense", "Divers et imprévus", true),
        )
        .await;
    assert_eq!(categorie(&t, "Divers et imprévus")["code"], "divers");
    let loyer = categorie(&t, "Loyer")["id"].clone();
    let t = p
        .ok(
            "categorie_tresorerie_enregistrer",
            Some(&admin),
            enregistrer(loyer, "depense", "Loyer", false),
        )
        .await;
    assert_eq!(categorie(&t, "Loyer")["active"], false);
    assert_eq!(
        p.refus(
            "categorie_tresorerie_enregistrer",
            Some(&admin),
            enregistrer(Value::Null, "depense", "Communication", true)
        )
        .await
        .cle,
        "tresorerie.categorie_existante"
    );

    // Un compte à la BDM et l'Orange Money de l'école ; une banque sans nom, une seconde caisse,
    // non.
    let compte = |nature: &str, libelle: &str, organisme: &str, solde: i64| {
        json!({ "id": null, "nature": nature, "libelle": libelle, "organisme": organisme,
            "numero": "", "solde_ouverture": solde, "ouvert_le": "2026-10-01", "actif": true })
    };
    p.ok(
        "compte_tresorerie_enregistrer",
        Some(&admin),
        compte("banque", "Compte courant", "BDM", 250_000),
    )
    .await;
    let t = p
        .ok(
            "compte_tresorerie_enregistrer",
            Some(&admin),
            compte("mobile_money", "Orange Money de l'école", "Orange Money", 0),
        )
        .await;
    assert_eq!(t["comptes"].as_array().unwrap().len(), 3);
    assert_eq!(t["comptes"][0]["nature"], "caisse");
    for (demande, cle) in [
        (
            compte("banque", "Compte", " ", 0),
            "tresorerie.organisme_obligatoire",
        ),
        (
            compte("caisse", "Seconde caisse", "", 0),
            "tresorerie.caisse_unique",
        ),
        (
            compte("banque", "Compte", "BNDA", -5),
            "tresorerie.solde_negatif",
        ),
    ] {
        assert_eq!(
            p.refus("compte_tresorerie_enregistrer", Some(&admin), demande)
                .await
                .cle,
            cle
        );
    }

    // Le seuil d'accord monte à 100 000 F, la cantine s'active.
    let t = p
        .ok(
            "tresorerie_reglages_enregistrer",
            Some(&admin),
            json!({ "seuil_accord": 100_000, "cantine_active": true, "repas_comptes": false }),
        )
        .await;
    assert_eq!(t["reglages"]["seuil_accord"], 100_000);
    assert_eq!(t["reglages"]["cantine_active"], true);
}
