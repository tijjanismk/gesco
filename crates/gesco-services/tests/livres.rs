//! Comptes et soldes par la frontière (SPEC_PHASE_8 §5, §6) : les avances payées deviennent des
//! dépenses « Salaires » sur le compte de leur mode, une seule fois, et s'annulent avec leur
//! paiement ; un dépôt de la caisse à la banque sort de l'une et entre dans l'autre ; le livre du
//! mois se coche contre le relevé.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod commun;

use commun::*;
use serde_json::{Value, json};

fn solde(e: &Value, compte: &Value) -> (i64, i64) {
    let s = e["soldes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["compte_id"] == *compte)
        .unwrap();
    (
        s["soldes"]["livre"].as_i64().unwrap(),
        s["soldes"]["rapproche"].as_i64().unwrap(),
    )
}

#[tokio::test]
async fn les_salaires_deviennent_des_depenses_et_un_depot_passe_de_la_caisse_a_la_banque() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    p.ok(
        "caisse_ouvrir",
        Some(&admin),
        json!({ "fonds_initial": 100_000 }),
    )
    .await;
    let t = p.ok("tresorerie_lire", Some(&admin), Value::Null).await;
    let caisse = t["comptes"][0].clone();
    assert_eq!(caisse["nature"], "caisse");
    let compte = |id: Value, nature: &str, libelle: &str, organisme: &str, solde: i64| {
        json!({ "id": id, "nature": nature, "libelle": libelle, "organisme": organisme,
            "numero": "", "solde_ouverture": solde, "ouvert_le": "2026-10-01", "actif": true })
    };
    p.ok(
        "compte_tresorerie_enregistrer",
        Some(&admin),
        compte(caisse["id"].clone(), "caisse", "Caisse", "", 100_000),
    )
    .await;
    p.ok(
        "compte_tresorerie_enregistrer",
        Some(&admin),
        compte(Value::Null, "banque", "Compte courant", "BDM", 250_000),
    )
    .await;
    let t = p
        .ok(
            "compte_tresorerie_enregistrer",
            Some(&admin),
            compte(
                Value::Null,
                "mobile_money",
                "Orange Money",
                "Orange",
                50_000,
            ),
        )
        .await;
    let id = |libelle: &str| {
        t["comptes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["libelle"] == libelle)
            .unwrap()["id"]
            .clone()
    };
    let (caisse, bdm, orange) = (id("Caisse"), id("Compte courant"), id("Orange Money"));

    // Deux avances à Aminata : l'une en Orange Money hors caisse, l'autre en espèces de la caisse.
    let categorie = p
        .ok("personnel_referentiel_lire", Some(&admin), Value::Null)
        .await["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["code"] == "administration")
        .unwrap()["id"]
        .clone();
    let aminata = p
        .ok(
            "membre_enregistrer",
            Some(&admin),
            json!({
                "id": null, "nom": "Kanté", "prenoms": "Aminata", "sexe": "feminin",
                "date_naissance": null, "telephones": [], "adresse": "", "matricule": "",
                "categorie_id": categorie, "fonction": "Secrétaire", "arrive_le": "2025-10-01",
                "fonctionnaire": false, "numero_fonctionnaire": "", "numero_inps": "",
                "prevenir_nom": "", "prevenir_telephone": "", "paiement": { "mode": "especes" },
                "diplomes": [], "utilisateur_id": null, "situation_famille": null,
                "enfants_a_charge": null,
            }),
        )
        .await["membre"]
        .clone();
    p.ok(
        "engagement_conclure",
        Some(&admin),
        json!({ "membre_id": aminata["id"], "nature": "duree_indeterminee",
            "debut": "2026-10-01", "fin": null, "fonction": "",
            "remuneration": { "type": "mensuelle", "montant": 90_000 }, "note": "",
            "remplace": null, "renouvelle": null, "paye_douze_mois": null }),
    )
    .await;
    let avance = |montant: i64, mode: &str, reference: &str, source: &str| {
        json!({ "membre_id": aminata["id"], "montant": montant, "mensualites": 2,
            "premier_mois": "2026-10", "motif": "Rentrée des enfants", "mode": mode,
            "reference": reference, "source": source })
    };
    let fait = p
        .ok(
            "avance_accorder",
            Some(&admin),
            avance(30_000, "orange_money", "OM-77", "hors_caisse"),
        )
        .await;
    let avance_om = fait["etat"]["avances"][0]["avance"]["id"].clone();
    p.ok(
        "avance_accorder",
        Some(&admin),
        avance(10_000, "especes", "", "caisse"),
    )
    .await;
    let d = p
        .ok("depenses_lire", Some(&admin), json!({ "mois": "2026-10" }))
        .await;
    let salaires: Vec<&Value> = d["depenses"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|x| x["categorie"] == "Salaires")
        .collect();
    assert_eq!(salaires.len(), 2);
    assert!(salaires.iter().all(|x| x["depense"]["statut"] == "payee"));
    let du_paiement = salaires[0]["depense"]["id"].clone();
    assert_eq!(
        p.refus(
            "depense_annuler",
            Some(&admin),
            json!({ "id": du_paiement, "motif": "Erreur" })
        )
        .await
        .cle,
        "depense.nee_d_un_paiement"
    );
    let e = p.ok("comptes_soldes_lire", Some(&admin), Value::Null).await;
    assert_eq!(solde(&e, &orange), (20_000, 50_000));
    assert_eq!(solde(&e, &caisse), (90_000, 100_000));
    assert_eq!(e["salaires_sans_compte"], 0);

    // Le dépôt de la recette : il sort de la caisse et entre à la BDM.
    let depot = |source: &Value, destination: &Value| {
        json!({ "date": "2026-10-04", "source_id": source, "destination_id": destination,
            "montant": 60_000, "reference": "BV-12", "motif": "Dépôt de la recette" })
    };
    assert_eq!(
        p.refus("transfert_faire", Some(&admin), depot(&bdm, &bdm))
            .await
            .cle,
        "transfert_compte.meme_compte"
    );
    let e = p
        .ok("transfert_faire", Some(&admin), depot(&caisse, &bdm))
        .await;
    assert_eq!(solde(&e, &caisse), (30_000, 100_000));
    assert_eq!(solde(&e, &bdm), (310_000, 250_000));
    let tiroir = p
        .ok("caisse_session_courante", Some(&admin), Value::Null)
        .await["totaux"]["theorique"]
        .as_i64()
        .unwrap();
    assert_eq!(tiroir, 30_000);

    // Le livre d'octobre de la BDM : le report, le dépôt, coché contre le relevé.
    let livre = p
        .ok(
            "livre_compte_lire",
            Some(&admin),
            json!({ "compte_id": bdm, "mois": "2026-10" }),
        )
        .await;
    assert_eq!(livre["report"], 250_000);
    assert_eq!(livre["fin"], 310_000);
    let ligne = livre["lignes"][0].clone();
    assert_eq!(
        (ligne["origine"].clone(), ligne["entree"].clone()),
        (json!("transfert"), json!(60_000))
    );
    let livre = p
        .ok(
            "ligne_tresorerie_pointer",
            Some(&admin),
            json!({ "compte_id": bdm, "mois": "2026-10", "origine": "transfert",
                "origine_id": ligne["origine_id"], "pointee": true }),
        )
        .await;
    assert_eq!(livre["soldes"]["rapproche"], 310_000);
    assert_eq!(livre["soldes"]["a_pointer"], 0);

    // Le relevé de la BDM, en CSV : le dépôt s'y retrouve par sa référence et se coche ; des
    // frais bancaires inconnus du livre restent à part. Les colonnes se devinent d'après l'en-tête
    // et se gardent pour le prochain relevé.
    let releve = dossier.path().join("releve-bdm.csv");
    std::fs::write(
        &releve,
        "Date;Libellé;Débit;Crédit;Référence\n04/10/2026;Versement espèces;;60 000;BV12\n\
         05/10/2026;Frais de tenue de compte;2 500;;FR-10\n",
    )
    .unwrap();
    let apercu = p
        .ok(
            "releve_apercu",
            Some(&admin),
            json!({ "compte_id": bdm, "chemin": releve.to_string_lossy() }),
        )
        .await;
    assert_eq!(
        apercu["format"],
        json!({ "date": 0, "montant": null, "debit": 2, "credit": 3, "reference": 4 })
    );
    // Le dépôt est déjà coché : décoché, le relevé le recoche.
    p.ok(
        "ligne_tresorerie_pointer",
        Some(&admin),
        json!({ "compte_id": bdm, "mois": "2026-10", "origine": "transfert",
            "origine_id": ligne["origine_id"], "pointee": false }),
    )
    .await;
    let resultat = p
        .ok(
            "releve_importer",
            Some(&admin),
            json!({ "compte_id": bdm, "chemin": releve.to_string_lossy(), "format": apercu["format"] }),
        )
        .await;
    assert_eq!(resultat["lues"], 2);
    assert_eq!(resultat["cochees"], 1);
    assert_eq!(resultat["non_retrouvees"][0]["montant"], -2_500);
    assert_eq!(resultat["a_pointer"], 0);
    let apercu = p
        .ok(
            "releve_apercu",
            Some(&admin),
            json!({ "compte_id": bdm, "chemin": releve.to_string_lossy() }),
        )
        .await;
    assert_eq!(apercu["format"]["credit"], 3);

    // L'avance en Orange Money annulée : sa dépense l'est aussi, le compte retrouve son solde.
    p.ok(
        "avance_annuler",
        Some(&admin),
        json!({ "id": avance_om, "motif": "Saisie en double" }),
    )
    .await;
    let e = p.ok("comptes_soldes_lire", Some(&admin), Value::Null).await;
    assert_eq!(solde(&e, &orange), (50_000, 50_000));
    assert_eq!(
        p.refus(
            "compte_du_mode_choisir",
            Some(&admin),
            json!({ "mode": "wave", "compte_id": bdm })
        )
        .await
        .cle,
        "tresorerie.compte_du_mode_invalide"
    );
}
