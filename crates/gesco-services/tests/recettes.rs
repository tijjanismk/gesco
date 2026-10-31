//! Autres recettes par la frontière (SPEC_VERSION_1_1 §1) : un don en espèces entre dans le
//! tiroir, une location de salle par virement dans le livre de la banque ; le journal du mois les
//! range dans leur catégorie ; annulée, une recette ressort du tiroir ou du livre ; une catégorie
//! de frais des parents ne reçoit pas d'autre recette.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use crate::commun::*;
use serde_json::{Value, json};

fn solde(e: &Value, compte: &Value) -> i64 {
    e["soldes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["compte_id"] == *compte)
        .unwrap()["soldes"]["livre"]
        .as_i64()
        .unwrap()
}

#[tokio::test]
async fn un_don_et_une_location_entrent_dans_leur_compte_et_le_journal() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    p.ok(
        "caisse_ouvrir",
        Some(&admin),
        json!({ "fonds_initial": 10_000 }),
    )
    .await;
    let tiroir = async || {
        p.ok("caisse_session_courante", Some(&admin), Value::Null)
            .await["totaux"]["theorique"]
            .as_i64()
            .unwrap()
    };
    let t = p
        .ok(
            "compte_tresorerie_enregistrer",
            Some(&admin),
            json!({ "id": null, "nature": "banque", "libelle": "Compte courant", "organisme": "BDM",
                "numero": "", "solde_ouverture": 0, "ouvert_le": "2026-10-01", "actif": true }),
        )
        .await;
    let compte = |nature: &str| {
        t["comptes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["nature"] == nature)
            .unwrap()["id"]
            .clone()
    };
    let (caisse, banque) = (compte("caisse"), compte("banque"));
    let r = p
        .ok("recettes_lire", Some(&admin), json!({ "mois": "2026-10" }))
        .await;
    // Seule la catégorie des dons et autres recettes les reçoit, pas celles des frais.
    let categories: Vec<&str> = r["categories"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["libelle"].as_str().unwrap())
        .collect();
    assert_eq!(categories, vec!["Dons et autres recettes"]);
    let dons = r["categories"][0]["id"].clone();
    let recette = |libelle: &str,
                   payeur: &str,
                   montant: i64,
                   compte: &Value,
                   mode: &str,
                   reference: &str| {
        json!({ "date": "2026-10-04", "categorie_id": dons, "libelle": libelle, "payeur": payeur,
            "montant": montant, "compte_id": compte, "mode": mode, "reference": reference })
    };

    // Un don en espèces : il entre dans le tiroir.
    p.ok(
        "recette_saisir",
        Some(&admin),
        recette(
            "Don pour la bibliothèque",
            "Association des parents",
            100_000,
            &caisse,
            "especes",
            "",
        ),
    )
    .await;
    assert_eq!(tiroir().await, 110_000);
    // La location de la salle par virement : référence exigée, puis le livre de la banque.
    assert_eq!(
        p.refus(
            "recette_saisir",
            Some(&admin),
            recette(
                "Location de la salle",
                "Mairie de la commune IV",
                25_000,
                &banque,
                "virement",
                " "
            )
        )
        .await
        .cle,
        "depense.reference_obligatoire"
    );
    let r = p
        .ok(
            "recette_saisir",
            Some(&admin),
            recette(
                "Location de la salle",
                "Mairie de la commune IV",
                25_000,
                &banque,
                "virement",
                "VIR-77",
            ),
        )
        .await;
    assert_eq!(r["total"], 125_000);
    let e = p.ok("comptes_soldes_lire", Some(&admin), Value::Null).await;
    assert_eq!(solde(&e, &banque), 25_000);
    let j = p
        .ok(
            "journal_tresorerie_lire",
            Some(&admin),
            json!({ "mois": "2026-10" }),
        )
        .await;
    assert_eq!(
        j["recettes"],
        json!([{ "categorie": "Dons et autres recettes", "montant": 125_000 }])
    );

    // Une catégorie de frais refusée ; une annulation sans motif refusée.
    let scolarite = p.ok("tresorerie_lire", Some(&admin), Value::Null).await["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["libelle"] == "Scolarité")
        .unwrap()["id"]
        .clone();
    let mut mauvaise = recette("Don", "Un parent", 5_000, &caisse, "especes", "");
    mauvaise["categorie_id"] = scolarite;
    assert_eq!(
        p.refus("recette_saisir", Some(&admin), mauvaise).await.cle,
        "recette.categorie_invalide"
    );
    let par_libelle = |r: &Value, libelle: &str| {
        r["recettes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["recette"]["libelle"] == libelle)
            .unwrap()["recette"]["id"]
            .clone()
    };
    let don = par_libelle(&r, "Don pour la bibliothèque");
    assert_eq!(
        p.refus(
            "recette_annuler",
            Some(&admin),
            json!({ "id": don, "motif": " " })
        )
        .await
        .cle,
        "recette.motif_obligatoire"
    );

    // Annulés, le don ressort du tiroir et la location du livre de la banque.
    p.ok(
        "recette_annuler",
        Some(&admin),
        json!({ "id": don, "motif": "Saisi en double" }),
    )
    .await;
    assert_eq!(tiroir().await, 10_000);
    let location = par_libelle(&r, "Location de la salle");
    let r = p
        .ok(
            "recette_annuler",
            Some(&admin),
            json!({ "id": location, "motif": "Location reportée" }),
        )
        .await;
    assert_eq!(r["total"], 0);
    let e = p.ok("comptes_soldes_lire", Some(&admin), Value::Null).await;
    assert_eq!(solde(&e, &banque), 0);
}
