//! Ce que doit l'État et rapports (SPEC_PRISE_EN_CHARGE_ETAT §6.3, §6.4, §9, §10, §12, essai 12) :
//! les totaux de l'état égalent la somme des restes de l'État de la liste des élèves ; filtres
//! (classe, série, seulement ce qui est dû) ; versements de l'année et non réparti ; budget :
//! « À recevoir de l'État » hors du total réalisé, la recette « Versements de l'État » dans le
//! réalisé ; tuile du tableau de bord ; PDF produit pour une classe, toutes et une année close ;
//! Excel ; relevé de compte remis au parent sans la part de l'État. Écrits avec le code, passés à
//! la fin du codage de la 2.0.0.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use crate::commun::*;
use crate::prise_en_charge_service::{
    Lycee, cocher, compte, inscrit, ligne, lister, lycee, n, ok, refus, situation,
};
use serde_json::{Value, json};

/// Un compte de banque, pour que le virement de l'État ait où entrer.
async fn banque(e: &Lycee) {
    e.p.ok(
        "compte_tresorerie_enregistrer",
        Some(&e.admin),
        json!({ "id": null, "nature": "banque", "libelle": "Compte courant", "organisme": "Banque",
            "numero": "", "solde_ouverture": 0, "ouvert_le": "2026-10-01", "actif": true }),
    )
    .await;
}

/// Un versement du parent en espèces, dans la caisse ouverte de l'administrateur.
async fn payer(e: &Lycee, inscription: &Value, cle: &str, montant: i64) {
    let preparation =
        e.p.ok(
            "encaissement_preparer",
            Some(&e.admin),
            json!({ "inscription_id": inscription["id"], "montant": montant }),
        )
        .await;
    e.p.ok(
        "encaisser",
        Some(&e.admin),
        json!({ "cle_idempotence": cle, "inscription_id": inscription["id"], "montant": montant,
            "mode": "especes", "reference": "", "payeur_id": preparation["payeurs"][0]["responsable_id"],
            "payeur_nom": "", "repartition": null }),
    )
    .await;
}

async fn du_etat(e: &Lycee, filtres: Value) -> Value {
    let mut entree = json!({ "annee_id": e.annee["id"] });
    for (cle, valeur) in filtres.as_object().unwrap() {
        entree[cle] = valeur.clone();
    }
    ok(e, "du_etat_lister", &e.admin, entree).await
}

/// Toutes les lignes de l'état, toutes classes confondues.
fn lignes_de(etat: &Value) -> Vec<Value> {
    etat["classes"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|c| c["lignes"].as_array().unwrap().clone())
        .collect()
}

fn somme(lignes: &[Value], cle: &str) -> i64 {
    lignes.iter().map(|l| n(&l[cle])).sum()
}

/// Essai 12 : l'état de ce que doit l'État, ses filtres, ses documents, le budget et la tuile.
#[tokio::test]
async fn l_etat_de_ce_que_doit_l_etat_egale_la_somme_des_restes() {
    let dossier = tempfile::tempdir().unwrap();
    let e = lycee(dossier.path()).await;
    banque(&e).await;
    let (_, awa) = inscrit(&e, "Awa", &e.dixieme).await;
    let (_, binta) = inscrit(&e, "Binta", &e.dixieme).await;
    let (_, coumba) = inscrit(&e, "Coumba", &e.dixieme).await;
    let (_, dado) = inscrit(&e, "Dado", &e.onzieme_s).await;
    // Awa, Binta et Dado sont cochés (scolarité de l'école : 120 000 F en 10e, 110 000 F en
    // 11e S) ; Coumba ne l'est pas.
    ok(
        &e,
        "prise_en_charge_regler",
        &e.admin,
        cocher(&[&awa, &binta, &dado], None),
    )
    .await;
    // Rien encore de l'État : la tuile et le budget disent tout l'attendu.
    let etat = du_etat(&e, json!({})).await;
    assert_eq!(etat["totaux"]["nombre"], 3);
    assert_eq!(etat["totaux"]["reste"], 350_000);
    assert_eq!(etat["non_reparti"], 0);
    assert!(etat["versements"].as_array().unwrap().is_empty());

    // Un virement de 150 000 F : 120 000 F pour Awa, 20 000 F pour Binta, 10 000 F non répartis.
    ok(
        &e,
        "versement_etat_enregistrer",
        &e.admin,
        json!({ "montant": 150_000, "mode": "virement", "reference": "OV-2026-201",
            "libelle": "Scolarité des élèves orientés — 1er versement",
            "parts": [ { "inscription_id": awa["id"], "montant": 120_000 },
                       { "inscription_id": binta["id"], "montant": 20_000 } ] }),
    )
    .await;

    // Les totaux de l'état égalent la somme des restes de l'État de la liste des élèves, ligne
    // par ligne et en tout.
    let liste = lister(&e).await;
    let etat = du_etat(&e, json!({})).await;
    let lignes = lignes_de(&etat);
    assert_eq!(lignes.len(), 3);
    assert!(
        lignes.iter().all(|l| l["inscription_id"] != coumba["id"]),
        "Coumba n'est pas cochée : elle n'est pas dans l'état"
    );
    for l in &lignes {
        let de_la_liste = liste["lignes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["inscription_id"] == l["inscription_id"])
            .unwrap();
        assert_eq!(l["reste"], de_la_liste["reste_etat"]);
        assert_eq!(l["echu"], de_la_liste["reste_etat_echu"]);
        assert_eq!(l["recu"], de_la_liste["recu_etat"]);
        assert_eq!(l["du"], de_la_liste["du_etat"]);
        assert!(n(&l["echu"]) <= n(&l["reste"]));
    }
    let restes_de_la_liste: i64 = liste["lignes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| n(&l["reste_etat"]))
        .sum();
    assert_eq!(n(&etat["totaux"]["reste"]), restes_de_la_liste);
    assert_eq!(etat["totaux"]["reste"], liste["total_du_etat"]);
    assert_eq!(etat["totaux"]["echu"], liste["total_du_etat_echu"]);
    assert_eq!(etat["totaux"]["reste"], 210_000);
    assert_eq!(etat["totaux"]["recu"], 140_000);
    assert_eq!(etat["totaux"]["attendu"], 350_000);
    assert_eq!(n(&etat["totaux"]["reste"]), somme(&lignes, "reste"));
    assert_eq!(ligne(&liste, &awa)["reste_etat"], 0);
    // Regroupé par classe, avec des sous-totaux qui font le total.
    let classes = etat["classes"].as_array().unwrap();
    assert_eq!(classes.len(), 2);
    assert_eq!(classes[0]["classe"], "10e A");
    assert_eq!(classes[0]["totaux"]["nombre"], 2);
    assert_eq!(classes[0]["totaux"]["reste"], 100_000);
    assert_eq!(classes[1]["classe"], "11e S");
    assert_eq!(classes[1]["totaux"]["reste"], 110_000);
    let par_classes: i64 = classes.iter().map(|c| n(&c["totaux"]["reste"])).sum();
    assert_eq!(n(&etat["totaux"]["reste"]), par_classes);
    // Les versements de l'année et le non réparti.
    assert_eq!(etat["versements"].as_array().unwrap().len(), 1);
    assert_eq!(etat["versements"][0]["reference"], "OV-2026-201");
    assert_eq!(etat["total_verse"], 150_000);
    assert_eq!(etat["non_reparti"], 10_000);
    // Filtres : les classes et séries qui ont une prise en charge.
    assert_eq!(etat["classes_filtre"].as_array().unwrap().len(), 2);
    assert_eq!(etat["series_filtre"].as_array().unwrap().len(), 1);

    // Une classe, une série, seulement ce qui est dû.
    let onze = du_etat(&e, json!({ "classe_id": e.onzieme_s["id"] })).await;
    assert_eq!(onze["classes"].as_array().unwrap().len(), 1);
    assert_eq!(onze["totaux"]["reste"], 110_000);
    assert_eq!(
        onze["non_reparti"], 10_000,
        "le non réparti est celui de l'année"
    );
    let serie = du_etat(&e, json!({ "serie_id": e.serie_s })).await;
    assert_eq!(serie["totaux"]["nombre"], 1);
    let dus = du_etat(&e, json!({ "seulement_dus": true })).await;
    assert_eq!(
        dus["totaux"]["nombre"], 2,
        "Awa ne doit plus rien de l'État"
    );
    assert_eq!(dus["totaux"]["reste"], 210_000);
    assert!(
        lignes_de(&dus)
            .iter()
            .all(|l| l["inscription_id"] != awa["id"])
    );

    // Budget : la recette « Versements de l'État » dans le réalisé ; « À recevoir de l'État »
    // à part, hors du total réalisé.
    let budget =
        e.p.ok("budget_lire", Some(&e.admin), json!({ "annee_id": null }))
            .await;
    let recettes = budget["recettes"].as_array().unwrap();
    let realise_etat = recettes
        .iter()
        .find(|l| l["categorie"]["code"] == "versement_etat")
        .map_or(0, |l| n(&l["realise"]));
    assert_eq!(realise_etat, 150_000);
    let realise: i64 =
        recettes.iter().map(|l| n(&l["realise"])).sum::<i64>() + n(&budget["avant_gesco"]);
    assert_eq!(n(&budget["recettes_realisees"]), realise);
    assert_eq!(budget["a_recevoir_etat"], 210_000);
    assert_eq!(budget["a_recevoir_etat"], etat["totaux"]["reste"]);

    // Tuile du tableau de bord : à recevoir de l'État, dont échu ; « Reste échu » des parents
    // ne compte pas l'État.
    let tableau =
        e.p.ok("tableau_de_bord_lire", Some(&e.admin), Value::Null)
            .await;
    assert_eq!(tableau["finances"]["a_recevoir_etat"]["reste"], 210_000);
    assert_eq!(
        tableau["finances"]["a_recevoir_etat"]["echu"],
        etat["totaux"]["echu"]
    );
    let impayes = e.p.ok("impayes_lister", Some(&e.admin), Value::Null).await;
    assert_eq!(tableau["finances"]["reste_echu"], impayes["echu"]);

    // PDF : toutes les classes, une classe, seulement ce qui est dû.
    for filtres in [
        json!({ "annee_id": e.annee["id"] }),
        json!({ "annee_id": e.annee["id"], "classe_id": e.dixieme["id"] }),
        json!({ "annee_id": e.annee["id"], "seulement_dus": true }),
    ] {
        let produit = ok(&e, "du_etat_produire", &e.admin, filtres).await;
        assert_eq!(
            produit["document"]["document"]["modele_id"],
            "sommes_dues_etat"
        );
        assert!(!produit["document"]["pdf"].as_str().unwrap().is_empty());
        assert!(produit["fichier"].is_null());
    }
    // Excel : les mêmes colonnes ; trois élèves, deux sous-totaux, le total général.
    let chemin = dossier.path().join("sommes-dues-etat.xlsx");
    let excel = ok(
        &e,
        "du_etat_produire",
        &e.admin,
        json!({ "annee_id": e.annee["id"], "chemin": chemin.to_string_lossy() }),
    )
    .await;
    assert!(excel["document"].is_null());
    assert_eq!(excel["fichier"]["lignes"], 6);
    assert!(chemin.exists());
    assert_eq!(
        refus(
            &e,
            "du_etat_produire",
            &e.admin,
            json!({ "annee_id": e.annee["id"], "chemin": dossier.path().join("etat.txt").to_string_lossy() }),
        )
        .await,
        "export.extension"
    );

    // Une année close : l'état se lit et s'imprime encore.
    {
        let mut cnx = base_brute(&e.p.racine, e.p.trousseau.clone()).await;
        sqlx::query("UPDATE annee_scolaire SET statut = 'cloturee'")
            .execute(&mut cnx)
            .await
            .unwrap();
    }
    let close = du_etat(&e, json!({})).await;
    assert_eq!(close["annee_close"], true);
    assert_eq!(close["totaux"]["reste"], 210_000);
    let produit = ok(
        &e,
        "du_etat_produire",
        &e.admin,
        json!({ "annee_id": e.annee["id"] }),
    )
    .await;
    assert_eq!(
        produit["document"]["document"]["modele_id"],
        "sommes_dues_etat"
    );
}

/// Droits : l'état se lit avec la facturation (Caissier, Comptable) ou la remise (Directeur) ; le
/// Secrétaire ne le lit pas. Sans prise en charge, ni tuile ni ligne du budget.
#[tokio::test]
async fn l_etat_se_lit_avec_la_facturation_ou_la_remise() {
    let dossier = tempfile::tempdir().unwrap();
    let e = lycee(dossier.path()).await;
    let (_, awa) = inscrit(&e, "Awa", &e.dixieme).await;
    let tableau =
        e.p.ok("tableau_de_bord_lire", Some(&e.admin), Value::Null)
            .await;
    assert!(tableau["finances"]["a_recevoir_etat"].is_null());
    let budget =
        e.p.ok("budget_lire", Some(&e.admin), json!({ "annee_id": null }))
            .await;
    assert!(budget["a_recevoir_etat"].is_null());
    ok(
        &e,
        "prise_en_charge_regler",
        &e.admin,
        cocher(&[&awa], None),
    )
    .await;
    let entree = json!({ "annee_id": e.annee["id"] });
    for role in ["comptable", "directeur", "caissier"] {
        let jeton = compte(&e, &format!("{role}-etat"), role).await;
        let etat = ok(&e, "du_etat_lister", &jeton, entree.clone()).await;
        assert_eq!(etat["totaux"]["reste"], 120_000);
    }
    let secretaire = compte(&e, "secretaire-etat", "secretaire").await;
    assert_eq!(
        refus(&e, "du_etat_lister", &secretaire, entree.clone()).await,
        "erreur.permission_refusee"
    );
    assert_eq!(
        refus(&e, "du_etat_produire", &secretaire, entree).await,
        "erreur.permission_refusee"
    );
}

/// Relevé de compte remis au parent (§6.4) : les échéances ne portent que la part du parent ;
/// une mention sans montant ni retard de l'État ; le trop-payé, ce qui en est reporté et ce qui
/// reste à rembourser.
#[tokio::test]
async fn le_releve_du_parent_ne_porte_pas_la_part_de_l_etat() {
    let dossier = tempfile::tempdir().unwrap();
    let e = lycee(dossier.path()).await;
    let (awa_f, awa) = inscrit(&e, "Awa", &e.dixieme).await;
    // Le parent paie l'inscription et une partie de la scolarité avant que la case soit cochée.
    payer(&e, &awa, "awa-1", 63_000).await;
    let avant =
        e.p.application
            .releve_compte_donnees_pour_essai(&e.admin, awa_f["eleve"]["id"].as_str().unwrap())
            .await
            .unwrap();
    let facture_avant = &avant["inscriptions"][0]["facture"];
    assert_eq!(facture_avant["prise_en_charge"], false);
    let echeances_avant = facture_avant["echeances"].as_array().unwrap().len();
    ok(
        &e,
        "prise_en_charge_regler",
        &e.admin,
        cocher(&[&awa], None),
    )
    .await;
    let s = situation(&e, &awa_f).await;
    assert_eq!(s["facture"]["reste"], 0);
    let releve =
        e.p.application
            .releve_compte_donnees_pour_essai(&e.admin, awa_f["eleve"]["id"].as_str().unwrap())
            .await
            .unwrap();
    let facture = &releve["inscriptions"][0]["facture"];
    assert_eq!(facture["prise_en_charge"], true);
    // Plus aucune échéance de la scolarité ni de l'inscription : tout est à l'État ou à l'école.
    let echeances = facture["echeances"].as_array().unwrap();
    assert!(echeances.len() < echeances_avant);
    assert!(echeances.is_empty(), "sans cantine, le parent ne doit rien");
    assert_eq!(facture["reste"], "0\u{202F}F");
    assert_eq!(facture["total"], "0\u{202F}F");
    // Le trop-payé : rien à reporter, tout à rembourser.
    assert_eq!(facture["trop_paye"], "63\u{202F}000\u{202F}F");
    assert!(facture["trop_paye_reporte"].is_null());
    assert_eq!(
        releve["inscriptions"][0]["reprise"]["a_rembourser"],
        "63\u{202F}000\u{202F}F"
    );
    // Aucun montant de l'État dans le relevé.
    let texte = releve.to_string();
    assert!(
        !texte.contains("120\u{202F}000"),
        "la part de l'État dans le relevé"
    );
    // Le document se produit.
    let document =
        e.p.ok(
            "releve_compte_produire",
            Some(&e.admin),
            json!({ "id": awa_f["eleve"]["id"] }),
        )
        .await;
    assert_eq!(document["document"]["modele_id"], "releve_compte");
}
