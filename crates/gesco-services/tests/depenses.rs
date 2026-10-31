//! Dépenses par la frontière (SPEC_PHASE_8 §3, T-61) : sous le seuil, saisie et paiement d'un
//! geste en espèces de la caisse ; au-dessus, l'accord de la direction, puis le paiement par
//! virement ; le mode qui suit le compte ; l'annulation qui remet l'argent dans la caisse ; le
//! justificatif joint ; un compte sans droit sur les dépenses ne les lit pas.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use crate::commun::*;
use serde_json::{Value, json};

fn par_libelle<'a>(e: &'a Value, cle: &str, libelle: &str) -> &'a Value {
    e[cle]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["depense"]["libelle"] == libelle)
        .unwrap_or_else(|| panic!("{libelle} absente de {cle}"))
}

#[tokio::test]
async fn une_depense_se_saisit_s_accorde_se_paie_et_s_annule() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    p.ok(
        "caisse_ouvrir",
        Some(&admin),
        json!({ "fonds_initial": 20_000 }),
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
                "numero": "", "solde_ouverture": 250_000, "ouvert_le": "2026-10-01", "actif": true }),
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
        .ok("depenses_lire", Some(&admin), json!({ "mois": "2026-10" }))
        .await;
    assert_eq!(
        e["droits"],
        json!({ "saisir": true, "approuver": true, "payer": true })
    );
    assert_eq!(e["seuil"], 50_000);
    let categorie = |libelle: &str| {
        e["categories"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["libelle"] == libelle)
            .unwrap()["id"]
            .clone()
    };
    let saisir = |libelle: &str, beneficiaire: &str, montant: i64, cat: &str, paiement: Value| {
        json!({ "date": "2026-10-03", "categorie_id": categorie(cat), "libelle": libelle,
            "beneficiaire": beneficiaire, "montant": montant, "paiement": paiement })
    };

    // Sous le seuil : saisie et payée d'un geste, en espèces de la caisse.
    let e = p
        .ok(
            "depense_saisir",
            Some(&admin),
            saisir(
                "Craies et feutres",
                "Librairie Kanté",
                7_500,
                "Fournitures",
                json!({ "compte_id": caisse, "mode": "especes", "reference": "" }),
            ),
        )
        .await;
    let craies = par_libelle(&e, "depenses", "Craies et feutres");
    assert_eq!(craies["depense"]["statut"], "payee");
    assert_eq!(craies["compte"], "Caisse");
    assert_eq!(e["total_paye"], 7_500);
    assert_eq!(tiroir().await, 12_500);

    // Au-dessus : elle attend l'accord, et ne se paie pas avant.
    let e = p
        .ok(
            "depense_saisir",
            Some(&admin),
            saisir(
                "Réparation de la toiture",
                "Entreprise Diakité",
                150_000,
                "Entretien et réparations",
                json!({ "compte_id": banque, "mode": "virement", "reference": "VIR-1" }),
            ),
        )
        .await;
    let toiture = par_libelle(&e, "en_attente", "Réparation de la toiture")["depense"].clone();
    assert_eq!(toiture["statut"], "a_approuver");
    // La direction en est avertie, et le tableau de bord le compte.
    assert!(
        avertissements(&p.ok("avertissements", Some(&admin), Value::Null).await)
            .contains("avertissement.depenses_a_approuver")
    );
    let tableau = p
        .ok("tableau_de_bord_lire", Some(&admin), Value::Null)
        .await;
    assert_eq!(tableau["tresorerie"]["a_approuver"], 1);
    let payer = |mode: &str, compte: &Value, reference: &str| {
        json!({ "id": toiture["id"], "paiement": { "compte_id": compte, "mode": mode,
            "reference": reference } })
    };
    p.refus(
        "depense_payer",
        Some(&admin),
        payer("virement", &banque, "VIR-1"),
    )
    .await;
    assert_eq!(
        p.refus(
            "depense_decider",
            Some(&admin),
            json!({ "id": toiture["id"], "accorder": false, "motif": " " })
        )
        .await
        .cle,
        "depense.motif_obligatoire"
    );
    let e = p
        .ok(
            "depense_decider",
            Some(&admin),
            json!({ "id": toiture["id"], "accorder": true, "motif": "" }),
        )
        .await;
    assert_eq!(
        par_libelle(&e, "en_attente", "Réparation de la toiture")["depense"]["statut"],
        "approuvee"
    );
    // Le mode suit le compte ; hors espèces, la référence est exigée.
    for (demande, cle) in [
        (payer("especes", &banque, ""), "depense.mode_du_compte"),
        (
            payer("virement", &banque, " "),
            "depense.reference_obligatoire",
        ),
    ] {
        assert_eq!(
            p.refus("depense_payer", Some(&admin), demande).await.cle,
            cle
        );
    }
    let e = p
        .ok(
            "depense_payer",
            Some(&admin),
            payer("virement", &banque, "VIR-118"),
        )
        .await;
    assert!(e["en_attente"].as_array().unwrap().is_empty());
    assert_eq!(e["total_paye"], 157_500);

    // Les craies, payées deux fois par erreur : l'annulation remet l'argent dans le tiroir.
    let e = p
        .ok(
            "depense_annuler",
            Some(&admin),
            json!({ "id": craies["depense"]["id"], "motif": "Saisie en double" }),
        )
        .await;
    assert_eq!(
        par_libelle(&e, "depenses", "Craies et feutres")["depense"]["statut"],
        "annulee"
    );
    assert_eq!(e["total_paye"], 150_000);
    assert_eq!(tiroir().await, 20_000);

    // Le justificatif de la toiture : un PDF joint, puis réécrit où l'on veut.
    let facture = dossier.path().join("facture-toiture.pdf");
    std::fs::write(&facture, b"%PDF-1.4\n% facture\n").unwrap();
    let e = p
        .ok(
            "depense_piece_joindre",
            Some(&admin),
            json!({ "id": toiture["id"], "chemin": facture.to_string_lossy() }),
        )
        .await;
    let pieces = par_libelle(&e, "depenses", "Réparation de la toiture")["pieces"].clone();
    assert_eq!(pieces[0]["nom_fichier"], "facture-toiture.pdf");
    let copie = dossier.path().join("copie.pdf");
    p.ok(
        "depense_piece_enregistrer_sous",
        Some(&admin),
        json!({ "piece_id": pieces[0]["id"], "chemin": copie.to_string_lossy() }),
    )
    .await;
    assert_eq!(
        std::fs::read(&copie).unwrap(),
        b"%PDF-1.4\n% facture\n".to_vec()
    );
    let texte = dossier.path().join("note.txt");
    std::fs::write(&texte, b"note").unwrap();
    assert_eq!(
        p.refus(
            "depense_piece_joindre",
            Some(&admin),
            json!({ "id": toiture["id"], "chemin": texte.to_string_lossy() })
        )
        .await
        .cle,
        "depense.piece_format"
    );

    // Un enseignant n'a aucun droit sur les dépenses : il ne les lit pas.
    let mot_de_passe = "ecole-2026-47514";
    p.ok(
        "utilisateur_creer",
        Some(&admin),
        json!({ "identifiant": "maitre", "nom_affiche": "Maître de la 3e",
            "mot_de_passe_initial": mot_de_passe, "roles": ["enseignant"] }),
    )
    .await;
    let maitre = p.connecter("maitre", mot_de_passe).await;
    assert_eq!(
        p.refus("depenses_lire", Some(&maitre), json!({ "mois": "2026-10" }))
            .await
            .cle,
        "depense.sans_droit"
    );
}
