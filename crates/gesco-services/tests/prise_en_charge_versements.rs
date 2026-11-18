//! Versement de l'État (SPEC_PRISE_EN_CHARGE_ETAT §7, §8, §10, §12) : caisse intacte (essai 1),
//! versement groupé (essai 6), année close (essai 7, partie versement), annulation depuis la
//! trésorerie, droits du versement (essai 9, partie versement), trace et totaux. Les commandes
//! viennent avec les écrans : le service s'appelle ici par l'aide d'essai de l'application, avec la
//! session du jeton, comme le feront les commandes du même nom. Écrits avec le code, passés à la
//! fin du codage de la 2.0.0.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use crate::commun::*;
use crate::prise_en_charge_service::{
    Lycee, cibles, cocher, compte, compter, inscrit, ligne, lister, lycee, n, ok, refus, situation,
};
use crate::reprises::photo_de_l_argent_de;
use gesco_services::session::Jeton;
use serde_json::{Value, json};

/// Un compte de banque avec son solde d'ouverture ; rend son identifiant.
async fn banque(e: &Lycee) -> Value {
    e.p.ok(
        "compte_tresorerie_enregistrer",
        Some(&e.admin),
        json!({ "id": null, "nature": "banque", "libelle": "Compte courant", "organisme": "Banque",
            "numero": "", "solde_ouverture": 250_000, "ouvert_le": "2026-10-01", "actif": true }),
    )
    .await;
    let comptes = e.p.ok("tresorerie_lire", Some(&e.admin), Value::Null).await["comptes"].clone();
    comptes
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["nature"] == "banque")
        .unwrap()["id"]
        .clone()
}

/// Un versement du parent en espèces, dans la caisse ouverte de l'administrateur.
async fn payer(e: &Lycee, inscription: &Value, cle: &str, montant: i64) -> Value {
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
    .await
}

fn parts(lignes: &[(&Value, i64)]) -> Value {
    Value::Array(
        lignes
            .iter()
            .map(|(i, m)| json!({ "inscription_id": i["id"], "montant": m }))
            .collect(),
    )
}

/// Un virement de l'État au compte du mode (la première banque active), daté d'aujourd'hui.
fn virement(montant: i64, lignes: &[(&Value, i64)]) -> Value {
    json!({ "montant": montant, "mode": "virement", "reference": "OV-2026-118",
        "libelle": "Scolarité des élèves orientés — 1er versement", "parts": parts(lignes) })
}

async fn photo(e: &Lycee) -> Value {
    photo_de_l_argent_de(&e.p, &e.admin).await
}

/// La photographie sans les clés données.
fn sans(photo: &Value, cles: &[&str]) -> Value {
    let mut p = photo.clone();
    for cle in cles {
        p.as_object_mut().unwrap().remove(*cle);
    }
    p
}

/// Livre du mois d'un compte de la photographie, par sa nature.
fn livre<'a>(photo: &'a Value, nature: &str) -> &'a Value {
    photo["livres"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["compte"]["nature"] == nature)
        .unwrap()
}

/// Ce que le journal de trésorerie du mois porte pour une catégorie de recette.
fn recette_du_journal(photo: &Value, categorie: &str) -> i64 {
    photo["tresorerie"]["recettes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["categorie"] == categorie)
        .map_or(0, |l| n(&l["montant"]))
}

/// Réalisé du budget de l'année pour la catégorie « Versements de l'État ».
async fn realise_etat(e: &Lycee) -> i64 {
    let budget =
        e.p.ok("budget_lire", Some(&e.admin), json!({ "annee_id": null }))
            .await;
    budget["recettes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["categorie"]["code"] == "versement_etat")
        .map_or(0, |l| n(&l["realise"]))
}

async fn versements(e: &Lycee, jeton: &Jeton) -> Value {
    ok(
        e,
        "versements_etat_lister",
        jeton,
        json!({ "annee_id": e.annee["id"] }),
    )
    .await
}

/// Essai 1 : pour une école avec des sessions ouvertes et clôturées, des versements, une
/// annulation, une sortie et des comptes avec leurs soldes d'ouverture, cocher, décocher,
/// modifier, abandonner, répartir et modifier une répartition ne changent rien à l'argent ; un
/// versement de l'État reçu avant GESCO non plus ; par virement, il ne change que le livre et le
/// solde de son compte, le journal de trésorerie du mois (catégorie « Versements de l'État ») et
/// le budget ; en espèces, il ajoute une entrée à la session.
#[tokio::test]
async fn la_caisse_ne_voit_que_l_argent_recu_de_l_etat() {
    let dossier = tempfile::tempdir().unwrap();
    let e = lycee(dossier.path()).await;
    let (_, awa) = inscrit(&e, "Awa", &e.dixieme).await;
    let (_, binta) = inscrit(&e, "Binta", &e.dixieme).await;
    let (_, coumba) = inscrit(&e, "Coumba", &e.dixieme).await;
    let compte_banque = banque(&e).await;
    // Une première session (ouverte par l'école d'essai) : deux versements, l'un annulé, puis la
    // clôture.
    payer(&e, &awa, "awa-1", 10_000).await;
    let recu = payer(&e, &binta, "binta-1", 5_000).await;
    e.p.ok(
        "versement_annuler",
        Some(&e.admin),
        json!({ "id": recu["versement"]["id"], "motif": "Erreur de montant" }),
    )
    .await;
    let courante =
        e.p.ok("caisse_session_courante", Some(&e.admin), Value::Null)
            .await;
    e.p.ok(
        "caisse_cloturer",
        Some(&e.admin),
        json!({ "total_compte": courante["totaux"]["theorique"], "motif_ecart": "" }),
    )
    .await;
    // Une seconde session, ouverte : un versement et une sortie.
    e.p.ok(
        "caisse_ouvrir",
        Some(&e.admin),
        json!({ "fonds_initial": 30_000 }),
    )
    .await;
    payer(&e, &coumba, "coumba-1", 25_000).await;
    let depenses =
        e.p.ok(
            "depenses_lire",
            Some(&e.admin),
            json!({ "mois": "2026-10" }),
        )
        .await;
    let ordinaire = depenses["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["code"] != "remboursement_parent")
        .unwrap()["id"]
        .clone();
    let caisse = depenses["comptes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["nature"] == "caisse")
        .unwrap()["id"]
        .clone();
    e.p.ok(
        "depense_saisir",
        Some(&e.admin),
        json!({ "date": "", "categorie_id": ordinaire, "libelle": "Craie", "beneficiaire": "Librairie",
            "montant": 3_000, "paiement": { "compte_id": caisse, "mode": "especes", "reference": "" } }),
    )
    .await;

    let avant = photo(&e).await;
    let budget_avant = realise_etat(&e).await;
    // Cocher, modifier, abandonner, décocher, recocher.
    ok(
        &e,
        "prise_en_charge_regler",
        &e.admin,
        cocher(&[&awa, &coumba], None),
    )
    .await;
    assert_eq!(photo(&e).await, avant, "après le cochage");
    ok(
        &e,
        "prise_en_charge_regler",
        &e.admin,
        json!({ "cibles": cibles(&[&awa]), "geste": "modifier", "montant_attendu": 100_000 }),
    )
    .await;
    assert_eq!(photo(&e).await, avant, "après la modification");
    ok(
        &e,
        "prise_en_charge_abandonner",
        &e.admin,
        json!({ "cibles": cibles(&[&awa]), "montant": 10_000, "motif": "Règlement définitif" }),
    )
    .await;
    assert_eq!(photo(&e).await, avant, "après l'abandon");
    ok(
        &e,
        "prise_en_charge_regler",
        &e.admin,
        json!({ "cibles": cibles(&[&coumba]), "geste": "decocher", "motif": "Pas orienté par l'État" }),
    )
    .await;
    assert_eq!(photo(&e).await, avant, "après le décochage");
    ok(
        &e,
        "prise_en_charge_regler",
        &e.admin,
        cocher(&[&coumba], None),
    )
    .await;
    assert_eq!(photo(&e).await, avant, "après le nouveau cochage");

    // Reçu avant GESCO : aucune recette, rien ne bouge ; sa répartition non plus.
    let avant_gesco = ok(
        &e,
        "versement_etat_enregistrer",
        &e.admin,
        json!({ "montant": 50_000, "avant_gesco": true, "parts": parts(&[(&awa, 50_000)]) }),
    )
    .await;
    assert!(avant_gesco["detail"]["versement"]["versement"]["recette_id"].is_null());
    assert_eq!(avant_gesco["detail"]["versement"]["compte"], "");
    assert!(avant_gesco["detail"]["versement"]["mode"].is_null());
    assert_eq!(
        avant_gesco["detail"]["versement"]["date_affichee"],
        avant_gesco["detail"]["versement"]["versement"]["date"]
    );
    assert_eq!(
        photo(&e).await,
        avant,
        "après le versement reçu avant GESCO"
    );
    ok(
        &e,
        "versement_etat_repartir",
        &e.admin,
        json!({ "versement_id": avant_gesco["detail"]["versement"]["versement"]["id"],
            "lue_le": avant_gesco["detail"]["versement"]["lue_le"],
            "parts": parts(&[(&awa, 30_000), (&coumba, 20_000)]) }),
    )
    .await;
    assert_eq!(photo(&e).await, avant, "après sa répartition");
    assert_eq!(realise_etat(&e).await, budget_avant);
    assert_eq!(
        compter(&e, "SELECT COUNT(*) FROM recette_tresorerie").await,
        0
    );

    // Par virement : le compte de banque, le journal du mois et le budget, rien d'autre.
    let vir = ok(
        &e,
        "versement_etat_enregistrer",
        &e.admin,
        virement(200_000, &[(&awa, 60_000), (&coumba, 60_000)]),
    )
    .await;
    assert_eq!(vir["detail"]["versement"]["compte_id"], compte_banque);
    assert_eq!(vir["detail"]["versement"]["mode"], "virement");
    let apres_virement = photo(&e).await;
    let intact = ["tresorerie", "comptes", "livres"];
    assert_eq!(
        sans(&apres_virement, &intact),
        sans(&avant, &intact),
        "le virement ne touche ni la caisse, ni les encaissements, ni les reçus"
    );
    assert_eq!(livre(&apres_virement, "caisse"), livre(&avant, "caisse"));
    assert_eq!(
        n(&livre(&apres_virement, "banque")["soldes"]["livre"]),
        n(&livre(&avant, "banque")["soldes"]["livre"]) + 200_000
    );
    assert_eq!(
        recette_du_journal(&apres_virement, "Versements de l'État"),
        recette_du_journal(&avant, "Versements de l'État") + 200_000
    );
    assert_eq!(
        n(&apres_virement["tresorerie"]["total_recettes"]),
        n(&avant["tresorerie"]["total_recettes"]) + 200_000
    );
    assert_eq!(
        apres_virement["tresorerie"]["total_depenses"],
        avant["tresorerie"]["total_depenses"]
    );
    assert_eq!(realise_etat(&e).await, budget_avant + 200_000);
    assert_eq!(
        compter(
            &e,
            "SELECT COUNT(*) FROM recette_tresorerie WHERE session_id IS NULL"
        )
        .await,
        1
    );
    // La liste des recettes porte la ligne, avec le versement de l'État qu'elle ouvre.
    let recettes =
        e.p.ok(
            "recettes_lire",
            Some(&e.admin),
            json!({ "mois": "2026-10" }),
        )
        .await;
    let ligne_recette = recettes["recettes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["recette"]["id"] == vir["detail"]["versement"]["versement"]["recette_id"])
        .unwrap()
        .clone();
    assert_eq!(
        ligne_recette["versement_etat_id"],
        vir["detail"]["versement"]["versement"]["id"]
    );
    assert_eq!(ligne_recette["categorie"], "Versements de l'État");

    // Modifier la répartition du virement : rien de l'argent ne bouge.
    ok(
        &e,
        "versement_etat_repartir",
        &e.admin,
        json!({ "versement_id": vir["detail"]["versement"]["versement"]["id"],
            "parts": parts(&[(&awa, 100_000), (&coumba, 50_000)]) }),
    )
    .await;
    assert_eq!(
        photo(&e).await,
        apres_virement,
        "après la nouvelle répartition"
    );

    // En espèces : une entrée dans la session ouverte, aucun versement de parent, aucun reçu.
    let especes = ok(
        &e,
        "versement_etat_enregistrer",
        &e.admin,
        json!({ "montant": 40_000, "mode": "especes", "parts": [] }),
    )
    .await;
    assert_eq!(especes["detail"]["versement"]["non_reparti"], 40_000);
    let apres_especes = photo(&e).await;
    assert_eq!(
        apres_especes["encaissements"],
        apres_virement["encaissements"]
    );
    assert_eq!(
        apres_especes["dernier_recu"],
        apres_virement["dernier_recu"]
    );
    assert_eq!(
        apres_especes["lignes"][0], apres_virement["lignes"][0],
        "versements"
    );
    assert_eq!(
        apres_especes["lignes"][1], apres_virement["lignes"][1],
        "imputations"
    );
    assert_eq!(
        n(&apres_especes["lignes"][2]),
        n(&apres_virement["lignes"][2]) + 1,
        "une entrée de caisse"
    );
    assert_eq!(
        n(&apres_especes["session"]["theorique"]),
        n(&apres_virement["session"]["theorique"]) + 40_000
    );
    // Annulé, il ressort de la caisse ouverte de qui annule.
    ok(
        &e,
        "versement_etat_annuler",
        &e.admin,
        json!({ "versement_id": especes["detail"]["versement"]["versement"]["id"],
            "motif": "Saisi deux fois" }),
    )
    .await;
    let apres_annulation = photo(&e).await;
    assert_eq!(
        n(&apres_annulation["session"]["theorique"]),
        n(&apres_virement["session"]["theorique"])
    );
    assert_eq!(
        n(&apres_annulation["lignes"][2]),
        n(&apres_virement["lignes"][2]) + 2
    );
}

/// Essai 6 : un virement de 2 400 000 F pour 20 élèves donne 20 parts, une recette et ne touche
/// aucune session ; une répartition de 2 500 000 F est refusée ; 1 800 000 F répartis laissent
/// 600 000 F non répartis, visibles ; une part de 130 000 F pour un élève qui en doit 120 000 F
/// laisse 10 000 F reçus en plus, avec l'avertissement et l'attendu porté à la somme reçue ;
/// l'annulation annule la recette et rend aux restes de l'État ce qu'ils étaient.
#[tokio::test]
async fn un_versement_groupe_se_repartit_puis_s_annule() {
    let dossier = tempfile::tempdir().unwrap();
    let e = lycee(dossier.path()).await;
    banque(&e).await;
    let mut eleves = Vec::new();
    for prenoms in [
        "Aminata",
        "Bakary",
        "Coumba",
        "Daouda",
        "Fatoumata",
        "Hawa",
        "Ibrahim",
        "Kadiatou",
        "Lassana",
        "Mariam",
        "Nafissatou",
        "Oumar",
        "Rokia",
        "Salif",
        "Tenin",
        "Youssouf",
        "Assétou",
        "Boubacar",
        "Djeneba",
        "Moussa",
    ] {
        eleves.push(inscrit(&e, prenoms, &e.dixieme).await.1);
    }
    let tous: Vec<&Value> = eleves.iter().collect();
    ok(&e, "prise_en_charge_regler", &e.admin, cocher(&tous, None)).await;
    let liste = lister(&e).await;
    assert_eq!(liste["nombre"], 20);
    assert_eq!(liste["total_du_etat"], 2_400_000);
    let mouvements = compter(&e, "SELECT COUNT(*) FROM mouvement_caisse").await;
    let session =
        e.p.ok("caisse_session_courante", Some(&e.admin), Value::Null)
            .await["totaux"]
            .clone();

    // La liste de répartition proposée : les vingt élèves, 120 000 F prérempli chacun.
    let proposee = ok(
        &e,
        "versement_etat_repartition_proposee",
        &e.admin,
        json!({ "annee_id": e.annee["id"] }),
    )
    .await;
    assert_eq!(proposee["eleves"].as_array().unwrap().len(), 20);
    assert_eq!(proposee["total_reste_du_etat"], 2_400_000);
    assert!(
        proposee["eleves"]
            .as_array()
            .unwrap()
            .iter()
            .all(|l| l["propose"] == 120_000 && l["deja_recu"] == 0)
    );

    // Le virement, réparti sur les vingt.
    let chacun: Vec<(&Value, i64)> = tous.iter().map(|i| (*i, 120_000)).collect();
    let apercu = ok(
        &e,
        "versement_etat_apercu",
        &e.admin,
        json!({ "montant": 2_400_000, "parts": parts(&chacun) }),
    )
    .await;
    assert_eq!(apercu["reparti"], 2_400_000);
    assert_eq!(apercu["non_reparti"], 0);
    assert!(apercu["refus"].is_null());
    assert_eq!(apercu["avertissements"].as_array().unwrap().len(), 0);
    let r = ok(
        &e,
        "versement_etat_enregistrer",
        &e.admin,
        virement(2_400_000, &chacun),
    )
    .await;
    let versement_id = r["detail"]["versement"]["versement"]["id"].clone();
    assert_eq!(r["detail"]["parts"].as_array().unwrap().len(), 20);
    assert_eq!(r["detail"]["versement"]["reparti"], 2_400_000);
    assert_eq!(r["detail"]["versement"]["non_reparti"], 0);
    assert_eq!(r["detail"]["versement"]["nombre_eleves"], 20);
    assert_eq!(r["avertissements"].as_array().unwrap().len(), 0);
    assert_eq!(
        compter(
            &e,
            "SELECT COUNT(*) FROM versement_etat_part WHERE statut = 'active'"
        )
        .await,
        20
    );
    assert_eq!(
        compter(&e, "SELECT COUNT(*) FROM recette_tresorerie").await,
        1
    );
    assert_eq!(
        compter(
            &e,
            "SELECT COUNT(*) FROM recette_tresorerie WHERE session_id IS NULL"
        )
        .await,
        1
    );
    assert_eq!(
        compter(&e, "SELECT COUNT(*) FROM mouvement_caisse").await,
        mouvements
    );
    assert_eq!(
        e.p.ok("caisse_session_courante", Some(&e.admin), Value::Null)
            .await["totaux"],
        session
    );
    let liste = lister(&e).await;
    assert_eq!(liste["total_du_etat"], 0);
    assert_eq!(liste["total_recu"], 2_400_000);
    let s = situation(
        &e,
        &json!({ "eleve": { "id": ligne(&liste, &eleves[0])["eleve_id"] } }),
    )
    .await;
    assert_eq!(s["facture"]["reste"], 0);
    assert_eq!(s["facture"]["reste_etat"], 0);
    assert_eq!(s["facture"]["facture"]["statut"], "soldee");

    // 2 500 000 F répartis : refusé, rien n'est écrit.
    let trop: Vec<(&Value, i64)> = tous.iter().map(|i| (*i, 125_000)).collect();
    let apercu = ok(
        &e,
        "versement_etat_apercu",
        &e.admin,
        json!({ "versement_id": versement_id, "parts": parts(&trop) }),
    )
    .await;
    assert_eq!(apercu["refus"], "versement_etat.reparti_superieur");
    assert_eq!(apercu["depassement"], 100_000);
    assert_eq!(
        refus(
            &e,
            "versement_etat_repartir",
            &e.admin,
            json!({ "versement_id": versement_id, "parts": parts(&trop) }),
        )
        .await,
        "versement_etat.reparti_superieur"
    );
    assert_eq!(
        compter(
            &e,
            "SELECT COUNT(*) FROM versement_etat_part WHERE statut = 'active'"
        )
        .await,
        20
    );

    // 1 800 000 F répartis : 600 000 F non répartis, visibles ; cinq élèves attendent l'État.
    let quinze: Vec<(&Value, i64)> = tous[..15].iter().map(|i| (*i, 120_000)).collect();
    let r = ok(
        &e,
        "versement_etat_repartir",
        &e.admin,
        json!({ "versement_id": versement_id, "lue_le": r["detail"]["versement"]["lue_le"],
            "parts": parts(&quinze) }),
    )
    .await;
    assert_eq!(r["detail"]["versement"]["non_reparti"], 600_000);
    let v = versements(&e, &e.admin).await;
    assert_eq!(v["total_recu"], 2_400_000);
    assert_eq!(v["total_reparti"], 1_800_000);
    assert_eq!(v["total_non_reparti"], 600_000);
    assert_eq!(v["versements"][0]["nombre_eleves"], 15);
    assert_eq!(v["versements"][0]["compte"], "Compte courant");
    let liste = lister(&e).await;
    assert_eq!(liste["total_du_etat"], 600_000);
    assert_eq!(ligne(&liste, &eleves[19])["reste_etat"], 120_000);
    assert_eq!(ligne(&liste, &eleves[0])["reste_etat"], 0);
    // Une ligne lue avant ce geste est refusée.
    assert_eq!(
        refus(
            &e,
            "versement_etat_repartir",
            &e.admin,
            json!({ "versement_id": versement_id, "lue_le": 1, "parts": parts(&quinze) }),
        )
        .await,
        "fiche.modifiee_entre_temps"
    );

    // 130 000 F pour un élève qui en doit 120 000 F : accepté, 10 000 F reçus en plus.
    let mut avec_plus = quinze.clone();
    avec_plus[14].1 = 130_000;
    let apercu = ok(
        &e,
        "versement_etat_apercu",
        &e.admin,
        json!({ "versement_id": versement_id, "parts": parts(&avec_plus) }),
    )
    .await;
    assert!(apercu["refus"].is_null());
    assert_eq!(apercu["non_reparti"], 590_000);
    let alerte = &apercu["avertissements"][0];
    assert_eq!(alerte["inscription_id"], eleves[14]["id"]);
    assert_eq!(alerte["en_plus"], 10_000);
    assert_eq!(alerte["attendu"], 120_000);
    assert_eq!(alerte["attendu_porte"], 130_000);
    let r = ok(
        &e,
        "versement_etat_repartir",
        &e.admin,
        json!({ "versement_id": versement_id, "parts": parts(&avec_plus) }),
    )
    .await;
    assert_eq!(r["avertissements"][0]["en_plus"], 10_000);
    let liste = lister(&e).await;
    assert_eq!(ligne(&liste, &eleves[14])["recu_en_plus"], 10_000);
    assert_eq!(ligne(&liste, &eleves[14])["reste_etat"], 0);
    // « Corriger l'attendu » : porté à la somme reçue, plus rien de reçu en plus.
    ok(
        &e,
        "prise_en_charge_regler",
        &e.admin,
        json!({ "cibles": cibles(&[&eleves[14]]), "geste": "modifier",
            "montant_attendu": r["avertissements"][0]["attendu_porte"] }),
    )
    .await;
    let liste = lister(&e).await;
    assert_eq!(ligne(&liste, &eleves[14])["recu_en_plus"], 0);
    assert_eq!(ligne(&liste, &eleves[14])["reste_etat"], 0);
    assert_eq!(liste["total_du_etat"], 600_000);

    // La même répartition encore : rien n'est écrit.
    let audits = compter(
        &e,
        "SELECT COUNT(*) FROM journal_audit WHERE entite = 'versement_etat'",
    )
    .await;
    ok(
        &e,
        "versement_etat_repartir",
        &e.admin,
        json!({ "versement_id": versement_id, "parts": parts(&avec_plus) }),
    )
    .await;
    assert_eq!(
        compter(
            &e,
            "SELECT COUNT(*) FROM journal_audit WHERE entite = 'versement_etat'"
        )
        .await,
        audits
    );

    // Annulation : la recette est annulée, les restes de l'État reviennent.
    assert_eq!(
        refus(
            &e,
            "versement_etat_annuler",
            &e.admin,
            json!({ "versement_id": versement_id, "motif": "  " }),
        )
        .await,
        "versement_etat.motif_obligatoire"
    );
    let annule = ok(
        &e,
        "versement_etat_annuler",
        &e.admin,
        json!({ "versement_id": versement_id, "motif": "Virement rejeté par la banque" }),
    )
    .await;
    assert_eq!(annule["versement"]["versement"]["statut"], "annule");
    assert_eq!(annule["versement"]["non_reparti"], 0);
    assert_eq!(annule["parts"].as_array().unwrap().len(), 0);
    assert_eq!(
        compter(
            &e,
            "SELECT COUNT(*) FROM recette_tresorerie WHERE annulee = 1"
        )
        .await,
        1
    );
    assert_eq!(
        compter(
            &e,
            "SELECT COUNT(*) FROM versement_etat_part WHERE statut = 'active'"
        )
        .await,
        0
    );
    let liste = lister(&e).await;
    assert_eq!(liste["total_recu"], 0);
    assert_eq!(liste["total_du_etat"], 2_410_000);
    assert_eq!(ligne(&liste, &eleves[0])["reste_etat"], 120_000);
    let v = versements(&e, &e.admin).await;
    assert_eq!(v["nombre"], 0);
    assert_eq!(v["total_recu"], 0);
    assert_eq!(
        refus(
            &e,
            "versement_etat_repartir",
            &e.admin,
            json!({ "versement_id": versement_id, "parts": parts(&quinze) }),
        )
        .await,
        "versement_etat.annule"
    );
    assert_eq!(
        compter(&e, "SELECT COUNT(*) FROM mouvement_caisse").await,
        mouvements
    );

    // Trace : un audit et un événement par geste.
    assert_eq!(
        compter(
            &e,
            "SELECT COUNT(*) FROM journal_audit WHERE entite = 'versement_etat'"
        )
        .await,
        4
    );
    for (evenement, nombre) in [
        ("VersementEtatEnregistre", 1),
        ("VersementEtatReparti", 2),
        ("VersementEtatAnnule", 1),
    ] {
        assert_eq!(
            compter(
                &e,
                &format!("SELECT COUNT(*) FROM outbox WHERE type_evenement = '{evenement}'")
            )
            .await,
            nombre,
            "{evenement}"
        );
    }

    // Sans aucun élève : tout est non réparti.
    let r = ok(
        &e,
        "versement_etat_enregistrer",
        &e.admin,
        virement(300_000, &[]),
    )
    .await;
    assert_eq!(r["detail"]["versement"]["non_reparti"], 300_000);
    assert_eq!(r["detail"]["versement"]["nombre_eleves"], 0);
}

/// Les refus de la répartition : élève jamais coché, élève deux fois, élève d'une autre année,
/// montant négatif, référence exigée hors espèces, montant reçu nul.
#[tokio::test]
async fn la_repartition_ne_va_qu_aux_eleves_pris_en_charge() {
    let dossier = tempfile::tempdir().unwrap();
    let e = lycee(dossier.path()).await;
    banque(&e).await;
    let (_, awa) = inscrit(&e, "Awa", &e.dixieme).await;
    let (_, binta) = inscrit(&e, "Binta", &e.dixieme).await;
    ok(
        &e,
        "prise_en_charge_regler",
        &e.admin,
        cocher(&[&awa], None),
    )
    .await;
    for (entree, cle) in [
        (
            virement(100_000, &[(&binta, 50_000)]),
            "versement_etat.eleve_non_pris_en_charge",
        ),
        (
            virement(100_000, &[(&awa, 20_000), (&awa, 30_000)]),
            "versement_etat.eleve_en_double",
        ),
        (
            virement(100_000, &[(&awa, -5_000)]),
            "versement_etat.part_invalide",
        ),
        (virement(0, &[]), "versement_etat.montant_invalide"),
        (
            json!({ "montant": 100_000, "mode": "virement", "reference": "", "parts": [] }),
            "depense.reference_obligatoire",
        ),
    ] {
        assert_eq!(
            refus(&e, "versement_etat_enregistrer", &e.admin, entree).await,
            cle
        );
    }
    assert_eq!(compter(&e, "SELECT COUNT(*) FROM versement_etat").await, 0);
    assert_eq!(
        compter(&e, "SELECT COUNT(*) FROM recette_tresorerie").await,
        0
    );
    // Un montant nul dans la liste est ignoré.
    let r = ok(
        &e,
        "versement_etat_enregistrer",
        &e.admin,
        virement(100_000, &[(&awa, 100_000), (&binta, 0)]),
    )
    .await;
    assert_eq!(r["detail"]["parts"].as_array().unwrap().len(), 1);
    // Un élève décoché après un versement reste dans la répartition de ce versement.
    ok(
        &e,
        "prise_en_charge_regler",
        &e.admin,
        json!({ "cibles": cibles(&[&awa]), "geste": "decocher", "motif": "Pas orienté par l'État" }),
    )
    .await;
    let proposee = ok(
        &e,
        "versement_etat_repartition_proposee",
        &e.admin,
        json!({ "versement_id": r["detail"]["versement"]["versement"]["id"] }),
    )
    .await;
    let eleves = proposee["eleves"].as_array().unwrap();
    assert_eq!(eleves.len(), 1);
    assert_eq!(eleves[0]["pris_en_charge"], false);
    assert_eq!(eleves[0]["part"], 100_000);
    assert_eq!(eleves[0]["propose"], 100_000);
    ok(
        &e,
        "versement_etat_repartir",
        &e.admin,
        json!({ "versement_id": r["detail"]["versement"]["versement"]["id"],
            "parts": parts(&[(&awa, 90_000)]) }),
    )
    .await;
}

/// Essai 7, partie versement : sur une année close, enregistrer et répartir un versement de
/// l'État restent permis ; seul le côté de l'État des factures change.
#[tokio::test]
async fn l_annee_close_recoit_encore_les_versements_de_l_etat() {
    let dossier = tempfile::tempdir().unwrap();
    let e = lycee(dossier.path()).await;
    banque(&e).await;
    let (awa_f, awa) = inscrit(&e, "Awa", &e.dixieme).await;
    let (_, binta) = inscrit(&e, "Binta", &e.dixieme).await;
    ok(
        &e,
        "prise_en_charge_regler",
        &e.admin,
        cocher(&[&awa, &binta], None),
    )
    .await;
    {
        let mut cnx = base_brute(&e.p.racine, e.p.trousseau.clone()).await;
        sqlx::query("UPDATE annee_scolaire SET statut = 'cloturee'")
            .execute(&mut cnx)
            .await
            .unwrap();
    }
    let reste_parent = situation(&e, &awa_f).await["facture"]["reste"].clone();
    let r = ok(
        &e,
        "versement_etat_enregistrer",
        &e.admin,
        json!({ "annee_id": e.annee["id"], "montant": 120_000, "mode": "virement",
            "reference": "OV-2027-004", "parts": parts(&[(&awa, 120_000)]) }),
    )
    .await;
    assert_eq!(r["detail"]["annee_close"], true);
    let liste = lister(&e).await;
    assert_eq!(liste["annee_close"], true);
    assert_eq!(ligne(&liste, &awa)["reste_etat"], 0);
    assert_eq!(ligne(&liste, &binta)["reste_etat"], 120_000);
    ok(
        &e,
        "versement_etat_repartir",
        &e.admin,
        json!({ "versement_id": r["detail"]["versement"]["versement"]["id"],
            "parts": parts(&[(&awa, 60_000), (&binta, 60_000)]) }),
    )
    .await;
    let liste = lister(&e).await;
    assert_eq!(ligne(&liste, &awa)["reste_etat"], 60_000);
    assert_eq!(ligne(&liste, &binta)["reste_etat"], 60_000);
    // Le parent d'une case cochée ne voit rien changer.
    assert_eq!(
        situation(&e, &awa_f).await["facture"]["reste"],
        reste_parent
    );
    let v = versements(&e, &e.admin).await;
    assert_eq!(v["annee_close"], true);
    assert_eq!(v["total_reparti"], 120_000);
}

/// Annuler la recette depuis la trésorerie annule le versement de l'État de la même façon : ses
/// parts tombent, les restes de l'État reviennent ; un versement en espèces ressort de la caisse.
#[tokio::test]
async fn annuler_la_recette_annule_le_versement_de_l_etat() {
    let dossier = tempfile::tempdir().unwrap();
    let e = lycee(dossier.path()).await;
    banque(&e).await;
    let (_, awa) = inscrit(&e, "Awa", &e.dixieme).await;
    ok(
        &e,
        "prise_en_charge_regler",
        &e.admin,
        cocher(&[&awa], None),
    )
    .await;
    let r = ok(
        &e,
        "versement_etat_enregistrer",
        &e.admin,
        virement(120_000, &[(&awa, 120_000)]),
    )
    .await;
    assert_eq!(ligne(&lister(&e).await, &awa)["reste_etat"], 0);
    e.p.ok(
        "recette_annuler",
        Some(&e.admin),
        json!({ "id": r["detail"]["versement"]["versement"]["recette_id"],
            "motif": "Avis de crédit erroné" }),
    )
    .await;
    let detail = ok(
        &e,
        "versement_etat_lire",
        &e.admin,
        json!({ "versement_id": r["detail"]["versement"]["versement"]["id"] }),
    )
    .await;
    assert_eq!(detail["versement"]["versement"]["statut"], "annule");
    assert_eq!(
        detail["versement"]["versement"]["motif_annulation"],
        "Avis de crédit erroné"
    );
    assert_eq!(detail["parts"].as_array().unwrap().len(), 0);
    assert_eq!(ligne(&lister(&e).await, &awa)["reste_etat"], 120_000);
    assert_eq!(
        compter(
            &e,
            "SELECT COUNT(*) FROM outbox WHERE type_evenement = 'VersementEtatAnnule'"
        )
        .await,
        1
    );
    // Le versement annulé ne s'annule pas deux fois.
    assert_eq!(
        refus(
            &e,
            "versement_etat_annuler",
            &e.admin,
            json!({ "versement_id": r["detail"]["versement"]["versement"]["id"],
                "motif": "Encore" }),
        )
        .await,
        "versement_etat.annule"
    );

    // En espèces : la recette annulée depuis la trésorerie ressort de la caisse ouverte.
    let mouvements = compter(&e, "SELECT COUNT(*) FROM mouvement_caisse").await;
    let especes = ok(
        &e,
        "versement_etat_enregistrer",
        &e.admin,
        json!({ "montant": 120_000, "mode": "especes", "parts": parts(&[(&awa, 120_000)]) }),
    )
    .await;
    assert_eq!(
        compter(&e, "SELECT COUNT(*) FROM mouvement_caisse").await,
        mouvements + 1
    );
    e.p.ok(
        "recette_annuler",
        Some(&e.admin),
        json!({ "id": especes["detail"]["versement"]["versement"]["recette_id"],
            "motif": "Billets refusés" }),
    )
    .await;
    assert_eq!(
        compter(&e, "SELECT COUNT(*) FROM mouvement_caisse").await,
        mouvements + 2
    );
    assert_eq!(ligne(&lister(&e).await, &awa)["reste_etat"], 120_000);
    assert_eq!(
        compter(
            &e,
            "SELECT COUNT(*) FROM versement_etat WHERE statut = 'annule'"
        )
        .await,
        2
    );
}

/// Essai 9, partie versement : sans `treasury:manage`, enregistrer, répartir et annuler sont
/// refusés ; « reçu avant GESCO » est refusé sans `invoices:discount` ; la lecture demande
/// `treasury:read` ; le Directeur et le Comptable enregistrent les deux.
#[tokio::test]
async fn le_versement_de_l_etat_demande_la_tresorerie() {
    let dossier = tempfile::tempdir().unwrap();
    let e = lycee(dossier.path()).await;
    banque(&e).await;
    let (_, awa) = inscrit(&e, "Awa", &e.dixieme).await;
    ok(
        &e,
        "prise_en_charge_regler",
        &e.admin,
        cocher(&[&awa], None),
    )
    .await;
    let secretaire = compte(&e, "secretaire", "secretaire").await;
    let caissier = compte(&e, "caissier", "caissier").await;
    let consultation = compte(&e, "consultation", "consultation_seule").await;
    let directeur = compte(&e, "directeur", "directeur").await;
    let comptable = compte(&e, "comptable", "comptable").await;
    // Un rôle de l'école : la trésorerie sans la remise.
    let role =
        e.p.ok(
            "role_creer",
            Some(&e.admin),
            json!({ "libelle": "Gestion des comptes",
                "permissions": ["treasury:read", "treasury:manage"] }),
        )
        .await;
    let tresorier = compte(
        &e,
        "tresorier-ecole",
        role["role"]["code"].as_str().unwrap(),
    )
    .await;
    let existant = ok(
        &e,
        "versement_etat_enregistrer",
        &e.admin,
        virement(50_000, &[(&awa, 50_000)]),
    )
    .await;
    let id = existant["detail"]["versement"]["versement"]["id"].clone();
    let ecritures = || {
        vec![
            (
                "versement_etat_enregistrer",
                virement(10_000, &[(&awa, 10_000)]),
            ),
            (
                "versement_etat_enregistrer",
                json!({ "montant": 10_000, "avant_gesco": true, "parts": [] }),
            ),
            (
                "versement_etat_repartir",
                json!({ "versement_id": id, "parts": parts(&[(&awa, 40_000)]) }),
            ),
            (
                "versement_etat_apercu",
                json!({ "versement_id": id, "parts": [] }),
            ),
            (
                "versement_etat_annuler",
                json!({ "versement_id": id, "motif": "Essai" }),
            ),
        ]
    };
    for jeton in [&secretaire, &caissier, &consultation] {
        for (nom, entree) in ecritures() {
            assert_eq!(
                refus(&e, nom, jeton, entree).await,
                "erreur.permission_refusee",
                "{nom}"
            );
        }
    }
    // Lire demande `treasury:read` : la consultation lit, le Secrétaire non.
    versements(&e, &consultation).await;
    assert_eq!(
        refus(
            &e,
            "versements_etat_lister",
            &secretaire,
            json!({ "annee_id": e.annee["id"] }),
        )
        .await,
        "erreur.permission_refusee"
    );
    assert_eq!(
        refus(
            &e,
            "versement_etat_lire",
            &caissier,
            json!({ "versement_id": id }),
        )
        .await,
        "erreur.permission_refusee"
    );
    // La trésorerie sans la remise : le virement oui, « reçu avant GESCO » non.
    ok(
        &e,
        "versement_etat_enregistrer",
        &tresorier,
        virement(10_000, &[(&awa, 10_000)]),
    )
    .await;
    assert_eq!(
        refus(
            &e,
            "versement_etat_enregistrer",
            &tresorier,
            json!({ "montant": 10_000, "avant_gesco": true, "parts": [] }),
        )
        .await,
        "erreur.permission_refusee"
    );
    ok(
        &e,
        "versement_etat_repartir",
        &tresorier,
        json!({ "versement_id": id, "parts": parts(&[(&awa, 40_000)]) }),
    )
    .await;
    assert_eq!(
        compter(
            &e,
            "SELECT COUNT(*) FROM versement_etat WHERE avant_gesco = 1"
        )
        .await,
        0
    );
    // Le Directeur et le Comptable : tout, « reçu avant GESCO » compris.
    for jeton in [&directeur, &comptable] {
        ok(
            &e,
            "versement_etat_enregistrer",
            jeton,
            json!({ "montant": 5_000, "avant_gesco": true, "parts": parts(&[(&awa, 5_000)]) }),
        )
        .await;
    }
    assert_eq!(
        compter(
            &e,
            "SELECT COUNT(*) FROM versement_etat WHERE avant_gesco = 1"
        )
        .await,
        2
    );
    ok(
        &e,
        "versement_etat_annuler",
        &directeur,
        json!({ "versement_id": id, "motif": "Doublon" }),
    )
    .await;
}
