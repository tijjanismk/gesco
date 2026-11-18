//! Prise en charge par l'État en fin d'année et avec la reprise avant GESCO
//! (SPEC_PRISE_EN_CHARGE_ETAT §9.4, §11, §12 essai 7, partie report) : l'écran « Payé avant
//! GESCO » et l'import des reprises comparent au dû du parent ; à la réinscription, la case suit
//! l'élève qui reste au lycée, proposée cochée avec le montant attendu de sa nouvelle classe ;
//! le report des impayés ne porte jamais la part de l'État, qui reste attachée à l'année close.
//! Écrits avec le code, passés à la fin du codage de la 2.0.0.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use crate::commun::*;
use crate::prise_en_charge_service::{
    Lycee, brouillon, cibles, compter, inscrit, ligne, lycee, n, ok,
};
use serde_json::{Value, json};

const JOUR: i64 = 86_400_000;

/// Ligne de l'écran « Payé avant GESCO » d'une inscription.
fn ligne_reprise<'a>(liste: &'a Value, inscription: &Value) -> &'a Value {
    liste["lignes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["inscription_id"] == inscription["id"])
        .unwrap()
}

/// Inscription d'un élève dans une année, d'après son nom affiché.
async fn inscription_de(e: &Lycee, annee: &Value, eleve: &str) -> Value {
    let liste = ok(
        e,
        "inscriptions_lister",
        &e.admin,
        json!({ "annee_id": annee["id"] }),
    )
    .await;
    liste
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["eleve"] == eleve)
        .unwrap_or_else(|| panic!("{eleve} absent"))["inscription"]
        .clone()
}

/// L'écran « Payé avant GESCO » montre le dû du parent, avec la mention de l'État pour un élève
/// coché ; l'import des reprises compare au même dû : ce qu'un parent d'élève coché a payé
/// avant GESCO au-delà de sa cantine est à lui rembourser.
#[tokio::test]
async fn paye_avant_gesco_compare_au_du_du_parent() {
    let dossier = tempfile::tempdir().unwrap();
    let e = lycee(dossier.path()).await;
    let (_, awa) = brouillon(&e, "Awa", &e.dixieme).await;
    ok(
        &e,
        "inscription_valider",
        &e.admin,
        json!({ "id": awa["id"], "choix": { "frais_optionnels": [e.cantine] } }),
    )
    .await;
    let (_, binta) = inscrit(&e, "Binta", &e.dixieme).await;
    let (_, kadi) = inscrit(&e, "Kadi", &e.dixieme).await;
    ok(
        &e,
        "prise_en_charge_regler",
        &e.admin,
        json!({ "cibles": cibles(&[&awa, &binta]), "geste": "cocher", "montant_attendu": 120_000 }),
    )
    .await;

    let liste = ok(&e, "reprises_lister", &e.admin, json!({})).await;
    // Awa : la cantine seule ; Binta : rien ; Kadi : inscription et scolarité.
    let l_awa = ligne_reprise(&liste, &awa);
    assert_eq!(l_awa["du"], 30_000);
    assert_eq!(l_awa["pris_en_charge"], true);
    assert_eq!(l_awa["reste"], 30_000);
    let l_binta = ligne_reprise(&liste, &binta);
    assert_eq!(l_binta["du"], 0);
    assert_eq!(l_binta["pris_en_charge"], true);
    let l_kadi = ligne_reprise(&liste, &kadi);
    assert_eq!(l_kadi["du"], 135_000);
    assert_eq!(l_kadi["pris_en_charge"], false);

    // Le classeur rempli : 50 000 F pour Awa (20 000 F de plus que sa cantine), 50 000 F pour
    // Kadi (moins que son dû), rien pour Binta.
    let lignes = [
        format!(
            "{};Coulibaly;Awa;10e A;30000;50000;",
            l_awa["matricule"].as_str().unwrap()
        ),
        format!(
            "{};Coulibaly;Binta;10e A;0;;",
            l_binta["matricule"].as_str().unwrap()
        ),
        format!(
            "{};Coulibaly;Kadi;10e A;135000;50000;",
            l_kadi["matricule"].as_str().unwrap()
        ),
    ];
    let contenu = format!(
        "Matricule;Nom;Prénoms;Classe;Dû de l'année;Payé avant GESCO;Note\n{}\n",
        lignes.join("\n")
    );
    let chemin = dossier.path().join("paye-avant-gesco.csv");
    std::fs::write(&chemin, contenu).unwrap();
    let apercu = ok(
        &e,
        "reprises_analyser_import",
        &e.admin,
        json!({ "chemin": chemin.to_str().unwrap() }),
    )
    .await;
    let numero = |n: u64| -> Value {
        apercu["lignes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|l| l["numero"] == n)
            .unwrap()
            .clone()
    };
    assert_eq!(numero(2)["sort"], "avertissement");
    assert_eq!(
        numero(2)["messages"][0]["cle"],
        "import_reprises.a_rembourser"
    );
    assert_eq!(numero(2)["messages"][0]["montant"], 20_000);
    assert_eq!(numero(3)["sort"], "ignoree");
    assert_eq!(numero(4)["sort"], "valide");
    assert!(numero(4)["messages"].as_array().unwrap().is_empty());

    // La même saisie à l'écran : le reste d'Awa tombe à zéro, 20 000 F à rembourser.
    ok(
        &e,
        "reprise_enregistrer",
        &e.admin,
        json!({ "inscription_id": awa["id"], "montant": 50_000, "note": "cahier p. 3" }),
    )
    .await;
    let liste = ok(&e, "reprises_lister", &e.admin, json!({})).await;
    let l_awa = ligne_reprise(&liste, &awa);
    assert_eq!(l_awa["du"], 30_000);
    assert_eq!(l_awa["reste"], 0);
    assert_eq!(l_awa["a_rembourser"], 20_000);
}

/// À la réinscription, l'élève coché qui reste au lycée reçoit la case cochée dans sa nouvelle
/// inscription, avec le montant attendu de sa nouvelle classe et sa référence ; l'élève décoché
/// ou jamais coché ne la reçoit pas ; la direction la décoche ensuite. Essai 7, partie report :
/// à la rentrée suivante, les arriérés de l'année close ne portent que la part du parent ; ce
/// que doit l'État reste attaché à l'année close.
#[tokio::test]
async fn la_case_suit_l_eleve_et_la_part_de_l_etat_ne_se_reporte_pas() {
    let dossier = tempfile::tempdir().unwrap();
    let mut e = lycee(dossier.path()).await;
    let (_, awa) = brouillon(&e, "Awa", &e.dixieme).await;
    ok(
        &e,
        "inscription_valider",
        &e.admin,
        json!({ "id": awa["id"], "choix": { "frais_optionnels": [e.cantine] } }),
    )
    .await;
    let (_, binta) = inscrit(&e, "Binta", &e.dixieme).await;
    let (_, kadi) = inscrit(&e, "Kadi", &e.dixieme).await;
    let (_, dado) = inscrit(&e, "Dado", &e.dixieme).await;
    ok(
        &e,
        "prise_en_charge_regler",
        &e.admin,
        json!({ "cibles": cibles(&[&awa]), "geste": "cocher", "montant_attendu": null,
            "reference": "DEF-2026-0042" }),
    )
    .await;
    ok(
        &e,
        "prise_en_charge_regler",
        &e.admin,
        json!({ "cibles": cibles(&[&binta, &dado]), "geste": "cocher", "montant_attendu": 120_000 }),
    )
    .await;
    ok(
        &e,
        "prise_en_charge_regler",
        &e.admin,
        json!({ "cibles": cibles(&[&dado]), "geste": "decocher", "motif": "Pas orienté par l'État" }),
    )
    .await;

    // Fin de l'année : aucun parent n'a payé, l'État non plus.
    e.p.horloge.avancer(300 * JOUR);
    e.admin = e.p.connecter("admin", MOT_DE_PASSE).await;
    for (inscription, decision) in [
        (&awa, "admis"),
        (&binta, "redouble"),
        (&kadi, "admis"),
        (&dado, "admis"),
    ] {
        ok(
            &e,
            "decision_poser",
            &e.admin,
            json!({ "inscription_id": inscription["id"], "decision": decision }),
        )
        .await;
    }
    ok(
        &e,
        "annee_cloturer",
        &e.admin,
        json!({ "id": e.annee["id"] }),
    )
    .await;

    // L'année suivante : ses classes et 126 000 F attendus de l'État pour la 11e S ; pas encore
    // de grille.
    let suivante = ok(
        &e,
        "annee_creer",
        &e.admin,
        json!({ "libelle": "2027-2028", "debut": RENTREE_2027, "fin": FIN_2028 }),
    )
    .await;
    let id_suivante = json!({ "id": suivante["id"] });
    ok(&e, "classes_reprendre", &e.admin, id_suivante.clone()).await;
    let tableau = ok(
        &e,
        "montants_etat_lire",
        &e.admin,
        json!({ "annee_id": suivante["id"] }),
    )
    .await;
    ok(
        &e,
        "montants_etat_regler",
        &e.admin,
        json!({ "annee_id": suivante["id"], "lue_le": tableau["lue_le"], "lignes": [
            { "niveau_id": e.niveau_11, "serie_id": e.serie_s, "montant": 126_000 },
        ] }),
    )
    .await;
    let versions_avant = compter(&e, "SELECT COUNT(*) FROM prise_en_charge_etat").await;
    let audits = "SELECT COUNT(*) FROM journal_audit
        WHERE entite = 'prise_en_charge_etat' AND contexte = 'prise_en_charge.cocher'";
    let audits_avant = compter(&e, audits).await;

    // La réinscription : quatre élèves préparés, deux avec la case cochée.
    let bilan = ok(&e, "reinscriptions_preparer", &e.admin, id_suivante.clone()).await;
    assert_eq!(bilan["preparees"], 4);
    assert_eq!(bilan["prises_en_charge"], 2);
    assert_eq!(
        compter(&e, "SELECT COUNT(*) FROM prise_en_charge_etat").await,
        versions_avant + 2
    );
    let awa_2 = inscription_de(&e, &suivante, "Awa Coulibaly").await;
    let binta_2 = inscription_de(&e, &suivante, "Binta Coulibaly").await;
    let kadi_2 = inscription_de(&e, &suivante, "Kadi Coulibaly").await;
    let dado_2 = inscription_de(&e, &suivante, "Dado Coulibaly").await;
    assert_eq!(awa_2["statut"], "brouillon");
    // La case des rapports suit la version en cours.
    assert_eq!(
        compter(
            &e,
            &format!(
                "SELECT prise_en_charge_etat FROM inscription WHERE id = '{}'",
                awa_2["id"].as_str().unwrap()
            )
        )
        .await,
        1
    );
    let liste = ok(
        &e,
        "prises_en_charge_lister",
        &e.admin,
        json!({ "annee_id": suivante["id"] }),
    )
    .await;
    // Awa monte en 11e S : le montant réglé pour la 11e S, sa référence reprise.
    let l = ligne(&liste, &awa_2);
    assert_eq!(l["pris_en_charge"], true);
    assert_eq!(l["montant_attendu"], 126_000);
    assert_eq!(l["reference"], "DEF-2026-0042");
    assert_eq!(l["sans_facture"], true);
    // Binta redouble en 10e A : sans montant réglé ni grille encore, l'attendu de l'année qui
    // s'achève plutôt que zéro.
    let l = ligne(&liste, &binta_2);
    assert_eq!(l["pris_en_charge"], true);
    assert_eq!(l["montant_attendu"], 120_000);
    assert_eq!(l["reference"], "");
    // Kadi n'a jamais été coché ; Dado a été décoché : la case ne les suit pas.
    assert_eq!(ligne(&liste, &kadi_2)["pris_en_charge"], false);
    assert_eq!(ligne(&liste, &kadi_2)["statut"], Value::Null);
    assert_eq!(ligne(&liste, &dado_2)["pris_en_charge"], false);
    assert_eq!(ligne(&liste, &dado_2)["statut"], Value::Null);
    // Le geste est tracé comme un cochage ordinaire, un audit par élève.
    assert_eq!(compter(&e, audits).await, audits_avant + 2);
    // Préparer de nouveau ne coche rien de plus.
    let bilan = ok(&e, "reinscriptions_preparer", &e.admin, id_suivante.clone()).await;
    assert_eq!(bilan["preparees"], 0);
    assert_eq!(bilan["prises_en_charge"], 0);

    // La grille de l'année suivante (scolarité 100 000 F partout) : le montant proposé pour
    // Binta devient la scolarité de la grille, que « Appliquer » porterait ; son attendu ne
    // change pas de lui-même.
    poser_grille(&e.p, &e.admin, &suivante["id"]).await;
    let liste = ok(
        &e,
        "prises_en_charge_lister",
        &e.admin,
        json!({ "annee_id": suivante["id"] }),
    )
    .await;
    assert_eq!(ligne(&liste, &binta_2)["montant_attendu"], 120_000);
    assert_eq!(ligne(&liste, &binta_2)["montant_propose"], 100_000);

    // L'État n'a pas reconduit Binta : la direction décoche, sur l'année en préparation.
    ok(
        &e,
        "prise_en_charge_regler",
        &e.admin,
        json!({ "cibles": cibles(&[&binta_2]), "geste": "decocher", "motif": "Non reconduit par l'État" }),
    )
    .await;
    // Au guichet, la facture d'Awa naît partagée : inscription non due, scolarité à l'État,
    // 26 000 F de complément ; rien de l'année close n'y est reporté.
    ok(
        &e,
        "inscription_valider",
        &e.admin,
        json!({ "id": awa_2["id"] }),
    )
    .await;
    ok(
        &e,
        "inscription_valider",
        &e.admin,
        json!({ "id": kadi_2["id"] }),
    )
    .await;
    let liste = ok(
        &e,
        "prises_en_charge_lister",
        &e.admin,
        json!({ "annee_id": suivante["id"] }),
    )
    .await;
    let l = ligne(&liste, &awa_2);
    assert_eq!(l["reste_parent"], 0);
    assert_eq!(l["du_etat"], 126_000);
    assert_eq!(l["reste_etat"], 126_000);
    assert_eq!(ligne(&liste, &binta_2)["pris_en_charge"], false);
    // Kadi doit l'année nouvelle seule : 10 000 F d'inscription et 100 000 F de scolarité.
    assert_eq!(ligne(&liste, &kadi_2)["reste_parent"], 110_000);

    // Essai 7, partie report : la rentrée ouvre l'année suivante.
    ok(&e, "annee_activer", &e.admin, id_suivante).await;
    let impayes = ok(&e, "impayes_lister", &e.admin, Value::Null).await;
    let arriere = |inscription: &Value| -> Option<Value> {
        impayes["lignes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|l| l["inscription_id"] == inscription["id"] && l["arriere"] == "2026-2027")
            .cloned()
    };
    // Awa : sa cantine seule ; la scolarité, due par l'État, et l'inscription non due n'y sont
    // pas.
    assert_eq!(n(&arriere(&awa).unwrap()["impaye"]["total"]), 30_000);
    // Binta : l'État doit tout, le parent rien : aucun arriéré.
    assert!(arriere(&binta).is_none());
    // Kadi et Dado (décoché) doivent toute leur année.
    assert_eq!(n(&arriere(&kadi).unwrap()["impaye"]["total"]), 135_000);
    assert_eq!(n(&arriere(&dado).unwrap()["impaye"]["total"]), 135_000);

    // Ce que doit l'État reste attaché à l'année close : il se lit, et l'école peut encore y
    // renoncer.
    let close = ok(
        &e,
        "prises_en_charge_lister",
        &e.admin,
        json!({ "annee_id": e.annee["id"] }),
    )
    .await;
    assert_eq!(close["annee_close"], true);
    assert_eq!(ligne(&close, &awa)["reste_etat"], 120_000);
    assert_eq!(ligne(&close, &binta)["reste_etat"], 120_000);
    let du = ok(
        &e,
        "du_etat_lister",
        &e.admin,
        json!({ "annee_id": e.annee["id"], "classe_id": null, "serie_id": null, "seulement_dus": true }),
    )
    .await;
    assert_eq!(du["annee_close"], true);
    assert_eq!(du["totaux"]["reste"], 240_000);
    ok(
        &e,
        "prise_en_charge_abandonner",
        &e.admin,
        json!({ "cibles": cibles(&[&binta]), "montant": 120_000, "motif": "Règlement définitif de l'année" }),
    )
    .await;
    let close = ok(
        &e,
        "prises_en_charge_lister",
        &e.admin,
        json!({ "annee_id": e.annee["id"] }),
    )
    .await;
    assert_eq!(ligne(&close, &binta)["reste_etat"], 0);
    // Le parent de Binta ne doit toujours rien.
    assert_eq!(ligne(&close, &binta)["reste_parent"], 0);
}
