//! Fournisseurs par la frontière (SPEC_PHASE_8 §4) : une facture payée en plusieurs fois, sous le
//! seuil d'un geste, au-dessus après l'accord ; ce qui reste dû ; un paiement au-delà du reste
//! refusé ; un refus qui libère le reste ; une facture réglée qui ne s'annule plus ; l'échéance
//! passée signalée.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use crate::commun::*;
use serde_json::{Value, json};

fn facture<'a>(e: &'a Value, numero: &str) -> &'a Value {
    e["factures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["facture"]["numero"] == numero)
        .unwrap_or_else(|| panic!("facture {numero} absente"))
}

#[tokio::test]
async fn une_facture_se_paie_en_plusieurs_fois_et_le_reste_du_se_lit() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    p.ok(
        "caisse_ouvrir",
        Some(&admin),
        json!({ "fonds_initial": 50_000 }),
    )
    .await;
    let t = p
        .ok(
            "compte_tresorerie_enregistrer",
            Some(&admin),
            json!({ "id": null, "nature": "banque", "libelle": "Compte courant", "organisme": "BDM",
                "numero": "", "solde_ouverture": 500_000, "ouvert_le": "2026-10-01", "actif": true }),
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

    let e = p
        .ok(
            "fournisseur_enregistrer",
            Some(&admin),
            json!({ "id": null, "nom": "Quincaillerie Coulibaly", "telephone": "76 12 34 56",
                "fourniture": "Peinture et matériaux", "actif": true }),
        )
        .await;
    let quincaillerie = e["fournisseurs"][0]["fournisseur"]["id"].clone();
    assert_eq!(
        p.refus(
            "fournisseur_enregistrer",
            Some(&admin),
            json!({ "id": null, "nom": "quincaillerie coulibaly", "telephone": "",
                "fourniture": "", "actif": true })
        )
        .await
        .cle,
        "fournisseur.nom_en_double"
    );
    let entretien = e["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["libelle"] == "Entretien et réparations")
        .unwrap()["id"]
        .clone();
    let nouvelle = |numero: &str, date: &str, echeance: &str, montant: i64| {
        json!({ "fournisseur_id": quincaillerie, "numero": numero, "date": date,
            "echeance": echeance, "libelle": "Peinture des classes", "categorie_id": entretien,
            "montant": montant })
    };
    assert_eq!(
        p.refus(
            "facture_fournisseur_enregistrer",
            Some(&admin),
            nouvelle("F-118", "2026-10-02", "2026-10-01", 120_000)
        )
        .await
        .cle,
        "fournisseur.echeance_avant_date"
    );
    let e = p
        .ok(
            "facture_fournisseur_enregistrer",
            Some(&admin),
            nouvelle("F-118", "2026-10-02", "2026-10-31", 120_000),
        )
        .await;
    assert_eq!(facture(&e, "F-118")["reste"], 120_000);
    assert_eq!(e["total_du"], 120_000);
    assert_eq!(
        p.refus(
            "facture_fournisseur_enregistrer",
            Some(&admin),
            nouvelle("F-118", "2026-10-03", "", 5_000)
        )
        .await
        .cle,
        "fournisseur.facture_en_double"
    );
    let f118 = facture(&e, "F-118")["facture"]["id"].clone();
    let payer = |montant: i64, paiement: Value| {
        json!({ "date": "2026-10-04", "categorie_id": entretien, "libelle": "Facture F-118",
            "beneficiaire": "Quincaillerie Coulibaly", "montant": montant, "paiement": paiement,
            "facture_id": f118 })
    };

    // Un acompte en espèces, sous le seuil : payé d'un geste.
    p.ok(
        "depense_saisir",
        Some(&admin),
        payer(
            40_000,
            json!({ "compte_id": caisse, "mode": "especes", "reference": "" }),
        ),
    )
    .await;
    let e = p.ok("fournisseurs_lire", Some(&admin), Value::Null).await;
    let f = facture(&e, "F-118");
    assert_eq!(
        (f["paye"].clone(), f["reste"].clone(), f["a_payer"].clone()),
        (json!(40_000), json!(80_000), json!(80_000))
    );
    assert_eq!(
        p.refus("depense_saisir", Some(&admin), payer(90_000, Value::Null))
            .await
            .cle,
        "fournisseur.au_dela_du_reste"
    );

    // Le solde, au-dessus du seuil : il attend l'accord et engage le reste.
    let d = p
        .ok("depense_saisir", Some(&admin), payer(80_000, Value::Null))
        .await;
    let solde = d["en_attente"][0]["depense"]["id"].clone();
    let e = p.ok("fournisseurs_lire", Some(&admin), Value::Null).await;
    let f = facture(&e, "F-118");
    assert_eq!(
        (
            f["en_cours"].clone(),
            f["reste"].clone(),
            f["a_payer"].clone()
        ),
        (json!(80_000), json!(80_000), json!(0))
    );
    assert_eq!(
        p.refus("depense_saisir", Some(&admin), payer(1_000, Value::Null))
            .await
            .cle,
        "fournisseur.au_dela_du_reste"
    );
    // Refusé, il libère le reste ; saisi de nouveau, accordé et payé par virement, il solde.
    p.ok(
        "depense_decider",
        Some(&admin),
        json!({ "id": solde, "accorder": false, "motif": "Attendre la fin des travaux" }),
    )
    .await;
    let e = p.ok("fournisseurs_lire", Some(&admin), Value::Null).await;
    assert_eq!(facture(&e, "F-118")["a_payer"], 80_000);
    let d = p
        .ok("depense_saisir", Some(&admin), payer(80_000, Value::Null))
        .await;
    let solde = d["en_attente"][0]["depense"]["id"].clone();
    p.ok(
        "depense_decider",
        Some(&admin),
        json!({ "id": solde, "accorder": true, "motif": "" }),
    )
    .await;
    p.ok(
        "depense_payer",
        Some(&admin),
        json!({ "id": solde, "paiement": { "compte_id": banque, "mode": "virement",
            "reference": "VIR-204" } }),
    )
    .await;
    let e = p.ok("fournisseurs_lire", Some(&admin), Value::Null).await;
    let f = facture(&e, "F-118");
    assert_eq!(
        (f["paye"].clone(), f["reste"].clone()),
        (json!(120_000), json!(0))
    );
    assert_eq!(f["reglements"].as_array().unwrap().len(), 3);
    assert_eq!(e["fournisseurs"][0]["factures_ouvertes"], 0);
    assert_eq!(e["total_du"], 0);
    assert_eq!(
        p.refus(
            "facture_fournisseur_annuler",
            Some(&admin),
            json!({ "id": f118, "motif": "Erreur" })
        )
        .await
        .cle,
        "fournisseur.facture_reglee"
    );

    // Une facture échue et impayée est signalée ; saisie par erreur, elle s'annule.
    let e = p
        .ok(
            "facture_fournisseur_enregistrer",
            Some(&admin),
            nouvelle("F-097", "2026-09-15", "2026-09-30", 35_000),
        )
        .await;
    assert_eq!(facture(&e, "F-097")["en_retard"], true);
    assert_eq!(e["fournisseurs"][0]["en_retard"], true);
    assert_eq!(e["total_du"], 35_000);
    let f097 = facture(&e, "F-097")["facture"]["id"].clone();
    let e = p
        .ok(
            "facture_fournisseur_annuler",
            Some(&admin),
            json!({ "id": f097, "motif": "Facture d'un autre client" }),
        )
        .await;
    let f = facture(&e, "F-097");
    assert_eq!(
        (f["facture"]["annulee"].clone(), f["reste"].clone()),
        (json!(true), json!(0))
    );
    assert_eq!(e["total_du"], 0);
    assert_eq!(
        p.refus(
            "depense_saisir",
            Some(&admin),
            json!({ "date": "2026-10-04", "categorie_id": entretien, "libelle": "Facture F-097",
                "beneficiaire": "Quincaillerie Coulibaly", "montant": 1_000, "paiement": null,
                "facture_id": f097 })
        )
        .await
        .cle,
        "fournisseur.facture_annulee"
    );
}
