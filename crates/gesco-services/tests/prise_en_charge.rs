//! Prise en charge par l'État (SPEC_PRISE_EN_CHARGE_ETAT §12) : calcul de la facture (essais 2,
//! 3, 3 bis, 4, 4 bis) et lecteurs du reste (essai 8). Les gestes de la direction (cocher,
//! décocher, abandonner) et le versement de l'État passent ici par les aides d'essai de
//! l'application, qui écrivent par les dépôts et recalculent la facture ; le partage ne dépend pas
//! du cycle de la classe. Écrits avec le code, passés à la fin du codage de la 2.0.0.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use crate::commun::*;
use crate::reprises::{Ecole, brouillon, inscrit, situation};
use gesco_services::session::Jeton;
use serde_json::{Value, json};

const JOUR: i64 = 86_400_000;

/// Année 2026-2027 ouverte ; 7e : inscription 15 000 F, scolarité 120 000 F selon le plan (trois
/// tranches par défaut : 48 000, 36 000, 36 000), cantine facultative 30 000 F ; 8e : inscription
/// 15 000 F, scolarité 100 000 F selon le plan.
async fn ecole(dossier: &std::path::Path) -> Ecole {
    let p = poste_vierge(dossier).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let annee = p
        .ok(
            "annee_creer",
            Some(&admin),
            json!({ "libelle": "2026-2027", "debut": RENTREE_2026, "fin": FIN_2027 }),
        )
        .await;
    p.ok("annee_activer", Some(&admin), json!({ "id": annee["id"] }))
        .await;
    let referentiel = p.ok("niveaux_lister", Some(&admin), Value::Null).await;
    let niveau = |code: &str| {
        referentiel["niveaux"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["code"] == code)
            .unwrap()["id"]
            .clone()
    };
    let mut classes = Vec::new();
    for (code, libelle) in [("7", "7e A"), ("8", "8e A")] {
        classes.push(
            p.ok(
                "classe_creer",
                Some(&admin),
                json!({ "annee_id": annee["id"], "niveau_id": niveau(code), "serie_id": null, "libelle": libelle }),
            )
            .await,
        );
    }
    let grille = p
        .ok(
            "grille_creer",
            Some(&admin),
            json!({ "annee_id": annee["id"], "copie_de": null }),
        )
        .await;
    let plans: Vec<Value> = grille["contenu"]["plans"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| json!({ "libelle": p["libelle"], "par_defaut": p["par_defaut"], "tranches": p["tranches"] }))
        .collect();
    let (sept, huit) = (niveau("7"), niveau("8"));
    p.ok(
        "grille_modifier",
        Some(&admin),
        json!({ "id": grille["grille"]["id"], "lue_le": grille["modifiee_le"], "libelle": "Frais 2026-2027", "plans": plans, "lignes": [
            { "niveau_id": sept, "serie_id": null, "type_frais": "inscription", "libelle": "Frais d'inscription",
              "montant": 15_000, "obligatoire": true, "exigibilite": "une_fois" },
            { "niveau_id": sept, "serie_id": null, "type_frais": "scolarite", "libelle": "Scolarité",
              "montant": 120_000, "obligatoire": true, "exigibilite": "selon_plan" },
            { "niveau_id": sept, "serie_id": null, "type_frais": "cantine", "libelle": "Cantine",
              "montant": 30_000, "obligatoire": false, "exigibilite": "une_fois" },
            { "niveau_id": huit, "serie_id": null, "type_frais": "inscription", "libelle": "Frais d'inscription",
              "montant": 15_000, "obligatoire": true, "exigibilite": "une_fois" },
            { "niveau_id": huit, "serie_id": null, "type_frais": "scolarite", "libelle": "Scolarité",
              "montant": 100_000, "obligatoire": true, "exigibilite": "selon_plan" },
        ] }),
    )
    .await;
    let vigueur = p
        .ok(
            "grille_mettre_en_vigueur",
            Some(&admin),
            json!({ "id": grille["grille"]["id"] }),
        )
        .await;
    let contenu = &vigueur["grille"]["contenu"];
    let cantine = contenu["lignes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["type_frais"] == "cantine")
        .unwrap()["id"]
        .clone();
    // Cantine active (liste des abonnés), remboursements sans accord préalable.
    p.ok(
        "tresorerie_reglages_enregistrer",
        Some(&admin),
        json!({ "seuil_accord": 200_000, "cantine_active": true, "repas_comptes": false }),
    )
    .await;
    p.ok("caisse_ouvrir", Some(&admin), json!({ "fonds_initial": 0 }))
        .await;
    Ecole {
        cantine,
        plans: contenu["plans"].as_array().unwrap().clone(),
        p,
        admin,
        annee,
        huitieme: classes[1].clone(),
        septieme: classes.remove(0),
    }
}

/// Élève de 7e validée avec la cantine cochée.
async fn inscrit_avec_cantine(e: &Ecole, prenoms: &str) -> (Value, Value) {
    let (fiche, inscription) = brouillon(e, prenoms, &e.septieme).await;
    e.p.ok(
        "inscription_valider",
        Some(&e.admin),
        json!({ "id": inscription["id"], "choix": { "frais_optionnels": [e.cantine] } }),
    )
    .await;
    (fiche, inscription)
}

fn id(inscription: &Value) -> &str {
    inscription["id"].as_str().unwrap()
}

/// Case cochée, avec le montant attendu et ce que l'école a abandonné.
async fn cocher(e: &Ecole, inscription: &Value, attendu: i64, abandonne: i64) {
    let motif = if abandonne == 0 {
        ""
    } else {
        "Règlement définitif de l'année"
    };
    e.p.application
        .prise_en_charge_pour_essai(id(inscription), true, attendu, abandonne, motif)
        .await
        .unwrap();
}

async fn decocher(e: &Ecole, inscription: &Value) {
    e.p.application
        .prise_en_charge_pour_essai(id(inscription), false, 0, 0, "Pas orienté par l'État")
        .await
        .unwrap();
}

async fn verser_etat(e: &Ecole, inscription: &Value, montant: i64) {
    e.p.application
        .versement_etat_pour_essai(id(inscription), montant)
        .await
        .unwrap();
}

/// Versement en espèces du parent, imputé exactement comme dit.
async fn payer_sur(
    e: &Ecole,
    jeton: &Jeton,
    inscription: &Value,
    cle: &str,
    parts: &[(Value, i64)],
) -> Value {
    let montant: i64 = parts.iter().map(|(_, m)| m).sum();
    let preparation =
        e.p.ok(
            "encaissement_preparer",
            Some(jeton),
            json!({ "inscription_id": inscription["id"], "montant": montant }),
        )
        .await;
    let repartition: Vec<Value> = parts
        .iter()
        .map(|(echeance, m)| json!({ "echeance_id": echeance, "montant": m }))
        .collect();
    e.p.ok(
        "encaisser",
        Some(jeton),
        json!({ "cle_idempotence": format!("essai-{cle}"), "inscription_id": inscription["id"], "montant": montant,
            "mode": "especes", "reference": "", "payeur_id": preparation["payeurs"][0]["responsable_id"],
            "payeur_nom": "", "repartition": repartition }),
    )
    .await
}

fn n(v: &Value) -> i64 {
    v.as_i64().unwrap()
}

/// Échéances de la facture d'une situation avec le type de frais de leur ligne, par date puis
/// rang.
fn echeances(s: &Value) -> Vec<(String, Value)> {
    let lignes = s["facture"]["lignes"].as_array().unwrap();
    s["facture"]["echeances"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            let ligne = lignes
                .iter()
                .find(|l| l["id"] == e["echeance"]["ligne_facture_id"])
                .unwrap();
            (ligne["type_frais"].as_str().unwrap().to_owned(), e.clone())
        })
        .collect()
}

/// La `rang`-ième échéance (0 d'abord) d'un type de frais.
fn echeance(s: &Value, type_frais: &str, rang: usize) -> Value {
    echeances(s)
        .into_iter()
        .filter(|(t, _)| t == type_frais)
        .nth(rang)
        .unwrap()
        .1
}

fn echeance_id(s: &Value, type_frais: &str, rang: usize) -> Value {
    echeance(s, type_frais, rang)["echeance"]["id"].clone()
}

fn facture(s: &Value, champ: &str) -> i64 {
    n(&s["facture"][champ])
}

/// Chaque échéance : `net = part de l'État + renoncé + part du parent`, `reste = part du parent
/// − payé`, aucun montant négatif.
fn verifier_partage(s: &Value) {
    for (_, e) in echeances(s) {
        assert_eq!(
            n(&e["net"]),
            n(&e["part_etat"]) + n(&e["renonce"]) + n(&e["part_parent"]),
            "{e}"
        );
        assert_eq!(n(&e["reste"]), n(&e["part_parent"]) - n(&e["paye"]), "{e}");
        assert_eq!(n(&e["reste_etat"]), n(&e["part_etat"]) - n(&e["paye_etat"]));
        for champ in [
            "part_etat",
            "renonce",
            "part_parent",
            "paye_etat",
            "reste_etat",
            "paye",
            "reste",
        ] {
            assert!(n(&e[champ]) >= 0, "{champ} : {e}");
        }
    }
}

/// Compte de la caisse, pour payer un remboursement en espèces.
async fn compte_caisse(e: &Ecole, jeton: &Jeton) -> Value {
    e.p.ok("tresorerie_lire", Some(jeton), Value::Null).await["comptes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["nature"] == "caisse")
        .unwrap()["id"]
        .clone()
}

/// Demande de remboursement au parent, payée en espèces.
async fn remboursement(e: &Ecole, jeton: &Jeton, inscription: &Value, montant: i64) -> Value {
    let depenses =
        e.p.ok("depenses_lire", Some(jeton), json!({ "mois": "2026-10" }))
            .await;
    let categorie = depenses["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["code"] == "remboursement_parent")
        .unwrap()["id"]
        .clone();
    json!({ "date": "", "categorie_id": categorie, "libelle": "Trop-payé — scolarité prise en charge par l'État",
        "beneficiaire": "Fanta Sangaré", "montant": montant, "inscription_id": inscription["id"],
        "paiement": { "compte_id": compte_caisse(e, jeton).await, "mode": "especes", "reference": "" } })
}

/// L'exemple : le parent a payé l'inscription (15 000 F) et la 1re tranche (48 000 F).
async fn exemple(e: &Ecole, prenoms: &str, avec_cantine: bool) -> (Value, Value) {
    let (fiche, ins) = if avec_cantine {
        inscrit_avec_cantine(e, prenoms).await
    } else {
        inscrit(e, prenoms, &e.septieme).await
    };
    let s = situation(e, &fiche).await;
    let cle = |i: u8| format!("{prenoms}-{i}");
    payer_sur(
        e,
        &e.admin,
        &ins,
        &cle(1),
        &[(echeance_id(&s, "inscription", 0), 15_000)],
    )
    .await;
    payer_sur(
        e,
        &e.admin,
        &ins,
        &cle(2),
        &[(echeance_id(&s, "scolarite", 0), 48_000)],
    )
    .await;
    (fiche, ins)
}

/// Essai 2 : la part de l'État et l'inscription sortent du reste du parent ; son trop-payé se
/// reporte sur la cantine, le surplus est à rembourser ; rien n'est réécrit ; le versement de
/// l'État solde la facture. Coché avant la facture, le premier versement proposé est la cantine.
#[tokio::test]
async fn le_parent_ne_doit_plus_que_le_reste() {
    let dossier = tempfile::tempdir().unwrap();
    let e = ecole(dossier.path()).await;
    let (awa, ins) = exemple(&e, "Awa", true).await;
    let avant = situation(&e, &awa).await;
    assert_eq!(facture(&avant, "reste"), 102_000);
    assert_eq!(facture(&avant, "part_etat"), 0);
    assert!(avant["facture"]["prise_en_charge"].is_null());

    cocher(&e, &ins, 120_000, 0).await;
    let s = situation(&e, &awa).await;
    verifier_partage(&s);
    // Rien n'est réécrit : total, lignes, échéances, versements.
    assert_eq!(s["facture"]["facture"]["total"], 165_000);
    assert_eq!(s["facture"]["lignes"], avant["facture"]["lignes"]);
    assert_eq!(s["versements"], avant["versements"]);
    let ids = |s: &Value| -> Vec<Value> {
        echeances(s)
            .into_iter()
            .map(|(_, e)| e["echeance"].clone())
            .collect()
    };
    assert_eq!(ids(&s), ids(&avant));
    assert_eq!(facture(&s, "part_etat"), 120_000);
    assert_eq!(facture(&s, "reste_etat"), 120_000);
    assert_eq!(facture(&s, "renonce"), 15_000);
    assert_eq!(facture(&s, "complement_etat"), 0);
    assert_eq!(facture(&s, "trop_paye"), 63_000);
    assert_eq!(facture(&s, "trop_paye_reporte"), 30_000);
    assert_eq!(facture(&s, "a_rembourser"), 33_000);
    assert_eq!(facture(&s, "reste"), 0);
    assert_eq!(s["facture"]["prise_en_charge"]["statut"], "en_cours");
    assert_eq!(s["facture"]["prise_en_charge"]["montant_attendu"], 120_000);
    // Le parent ne doit rien, l'État doit encore : la facture n'est pas soldée.
    assert_eq!(s["facture"]["facture"]["statut"], "partiellement_reglee");
    let inscription = echeance(&s, "inscription", 0);
    assert_eq!(inscription["renonce"], 15_000);
    assert_eq!(inscription["verse_parent"], 15_000);
    assert_eq!(inscription["reste"], 0);
    let tranche = echeance(&s, "scolarite", 0);
    assert_eq!(tranche["part_etat"], 48_000);
    assert_eq!(tranche["verse_parent"], 48_000);
    assert_eq!(tranche["paye"], 0);
    let cantine = echeance(&s, "cantine", 0);
    assert_eq!(cantine["part_parent"], 30_000);
    assert_eq!(cantine["paye"], 30_000);
    assert_eq!(cantine["reste"], 0);

    // Le versement de l'État solde la facture.
    verser_etat(&e, &ins, 120_000).await;
    let s = situation(&e, &awa).await;
    verifier_partage(&s);
    assert_eq!(facture(&s, "paye_etat"), 120_000);
    assert_eq!(facture(&s, "reste_etat"), 0);
    assert_eq!(facture(&s, "recu_en_plus_etat"), 0);
    assert_eq!(facture(&s, "reste"), 0);
    assert_eq!(facture(&s, "a_rembourser"), 33_000);
    assert_eq!(s["facture"]["facture"]["statut"], "soldee");

    // Cochée avant la validation : la facture naît partagée, le parent ne doit que la cantine.
    let (binta, brouillon_b) = brouillon(&e, "Binta", &e.septieme).await;
    cocher(&e, &brouillon_b, 120_000, 0).await;
    e.p.ok(
        "inscription_valider",
        Some(&e.admin),
        json!({ "id": brouillon_b["id"], "choix": { "frais_optionnels": [e.cantine] } }),
    )
    .await;
    let s = situation(&e, &binta).await;
    verifier_partage(&s);
    assert_eq!(facture(&s, "reste"), 30_000);
    let preparation =
        e.p.ok(
            "encaissement_preparer",
            Some(&e.admin),
            json!({ "inscription_id": brouillon_b["id"], "montant": null }),
        )
        .await;
    assert_eq!(preparation["reste"], 30_000);
    let proposee =
        e.p.ok(
            "encaissement_preparer",
            Some(&e.admin),
            json!({ "inscription_id": brouillon_b["id"], "montant": 30_000 }),
        )
        .await;
    assert_eq!(
        proposee["imputation"],
        json!([{ "echeance_id": echeance_id(&s, "cantine", 0), "montant": 30_000 }])
    );

    // Plus tard, l'élève n'est pas aux impayés.
    e.p.horloge.avancer(200 * JOUR);
    let admin = e.p.connecter("admin", MOT_DE_PASSE).await;
    let impayes = e.p.ok("impayes_lister", Some(&admin), Value::Null).await;
    assert!(
        impayes["lignes"]
            .as_array()
            .is_none_or(|l| l.iter().all(|l| l["inscription_id"] != ins["id"]))
    );
}

/// Essai 3 : un attendu plus bas que la scolarité laisse un écart renoncé, à la charge de
/// l'école, sur la tranche d'avril ; l'abandon s'y ajoute ; le parent ne doit jamais plus.
#[tokio::test]
async fn l_ecart_et_l_abandon_sont_a_la_charge_de_l_ecole() {
    let dossier = tempfile::tempdir().unwrap();
    let e = ecole(dossier.path()).await;
    let (awa, ins) = inscrit(&e, "Awa", &e.septieme).await;
    cocher(&e, &ins, 100_000, 0).await;
    let s = situation(&e, &awa).await;
    verifier_partage(&s);
    assert_eq!(facture(&s, "part_etat"), 100_000);
    // 20 000 F d'écart et l'inscription (15 000 F), à la charge de l'école.
    assert_eq!(facture(&s, "renonce"), 35_000);
    assert_eq!(echeance(&s, "scolarite", 2)["renonce"], 20_000);
    assert_eq!(echeance(&s, "scolarite", 2)["part_etat"], 16_000);
    assert_eq!(facture(&s, "reste"), 0);
    assert_eq!(facture(&s, "complement_etat"), 0);

    // L'État ne paiera pas 36 000 F : part de l'État 64 000 F, renoncé 56 000 F sur la scolarité.
    cocher(&e, &ins, 100_000, 36_000).await;
    let s = situation(&e, &awa).await;
    verifier_partage(&s);
    assert_eq!(facture(&s, "part_etat"), 64_000);
    assert_eq!(facture(&s, "renonce"), 56_000 + 15_000);
    assert_eq!(echeance(&s, "scolarite", 0)["part_etat"], 48_000);
    assert_eq!(echeance(&s, "scolarite", 1)["part_etat"], 16_000);
    assert_eq!(echeance(&s, "scolarite", 2)["part_etat"], 0);
    assert_eq!(facture(&s, "reste"), 0);
    assert_eq!(facture(&s, "a_rembourser"), 0);
    assert_eq!(s["facture"]["prise_en_charge"]["abandonne"], 36_000);
}

/// Essai 3 bis : un attendu plus haut que la scolarité laisse un complément hors facture, suivi
/// avec ce que l'État doit ; la facture ne change pas.
#[tokio::test]
async fn le_complement_attendu_reste_hors_facture() {
    let dossier = tempfile::tempdir().unwrap();
    let e = ecole(dossier.path()).await;
    let (awa, ins) = inscrit(&e, "Awa", &e.huitieme).await;
    let (binta, ins_b) = inscrit(&e, "Binta", &e.huitieme).await;
    cocher(&e, &ins, 126_000, 0).await;
    let s = situation(&e, &awa).await;
    verifier_partage(&s);
    assert_eq!(s["facture"]["facture"]["total"], 115_000);
    assert_eq!(facture(&s, "part_etat"), 100_000);
    assert_eq!(facture(&s, "complement_etat"), 26_000);
    assert_eq!(facture(&s, "reste"), 0);

    // 126 000 F reçus : l'État ne doit plus rien, rien de reçu en plus, facture soldée.
    verser_etat(&e, &ins, 126_000).await;
    let s = situation(&e, &awa).await;
    assert_eq!(facture(&s, "reste_etat"), 0);
    assert_eq!(facture(&s, "paye_complement"), 26_000);
    assert_eq!(facture(&s, "recu_en_plus_etat"), 0);
    assert_eq!(s["facture"]["facture"]["statut"], "soldee");

    // 110 000 F reçus : facture soldée côté État, 16 000 F du complément encore dus.
    cocher(&e, &ins_b, 126_000, 0).await;
    verser_etat(&e, &ins_b, 110_000).await;
    let s = situation(&e, &binta).await;
    assert_eq!(facture(&s, "reste_etat"), 0);
    assert_eq!(facture(&s, "paye_complement"), 10_000);
    assert_eq!(
        facture(&s, "complement_etat") - facture(&s, "paye_complement"),
        16_000
    );
    assert_eq!(s["facture"]["facture"]["statut"], "soldee");
    // Abandon des 16 000 F : l'État ne doit plus rien.
    cocher(&e, &ins_b, 126_000, 16_000).await;
    let s = situation(&e, &binta).await;
    assert_eq!(facture(&s, "reste_etat"), 0);
    assert_eq!(
        facture(&s, "complement_etat") - facture(&s, "paye_complement"),
        0
    );
    assert_eq!(facture(&s, "recu_en_plus_etat"), 0);
}

/// Essai 4 : après le remboursement de 33 000 F, la case décochée rend au parent l'inscription,
/// la scolarité et ce qui lui a été rendu ; un versement de l'État de 60 000 F réduit sa part ;
/// l'annulation d'un versement qui laisserait le remboursé au-delà de son argent est refusée.
#[tokio::test]
async fn decocher_apres_un_remboursement() {
    let dossier = tempfile::tempdir().unwrap();
    let e = ecole(dossier.path()).await;
    let (awa, ins) = exemple(&e, "Awa", true).await;
    cocher(&e, &ins, 120_000, 0).await;

    // Remboursé 33 000 F ; un franc de plus est refusé.
    e.p.ok(
        "depense_saisir",
        Some(&e.admin),
        remboursement(&e, &e.admin, &ins, 33_000).await,
    )
    .await;
    let s = situation(&e, &awa).await;
    verifier_partage(&s);
    assert_eq!(facture(&s, "rembourse"), 33_000);
    assert_eq!(facture(&s, "a_rembourser"), 0);
    assert_eq!(facture(&s, "reste"), 0);
    assert_eq!(
        e.p.refus(
            "depense_saisir",
            Some(&e.admin),
            remboursement(&e, &e.admin, &ins, 1).await
        )
        .await
        .cle,
        "remboursement.superieur_a_l_excedent"
    );

    // Décochée, sans versement de l'État : 135 000 F, dont 33 000 F sur la tranche d'octobre.
    decocher(&e, &ins).await;
    let s = situation(&e, &awa).await;
    verifier_partage(&s);
    assert_eq!(s["facture"]["prise_en_charge"]["statut"], "terminee");
    assert_eq!(facture(&s, "part_etat"), 0);
    assert_eq!(facture(&s, "renonce"), 0);
    assert_eq!(facture(&s, "reste"), 135_000);
    assert_eq!(echeance(&s, "inscription", 0)["reste"], 0);
    assert_eq!(echeance(&s, "scolarite", 0)["reste"], 33_000);
    assert_eq!(facture(&s, "a_rembourser"), 0);

    // L'État a payé 60 000 F : la 1re tranche entière et 12 000 F de la 2e.
    verser_etat(&e, &ins, 60_000).await;
    let s = situation(&e, &awa).await;
    verifier_partage(&s);
    assert_eq!(facture(&s, "part_etat"), 60_000);
    assert_eq!(facture(&s, "reste_etat"), 0);
    assert_eq!(echeance(&s, "scolarite", 0)["part_etat"], 48_000);
    assert_eq!(echeance(&s, "scolarite", 1)["part_etat"], 12_000);
    assert_eq!(facture(&s, "reste"), 75_000);
    assert_eq!(echeance(&s, "cantine", 0)["reste"], 15_000);
    assert_eq!(echeance(&s, "scolarite", 1)["reste"], 24_000);
    assert_eq!(echeance(&s, "scolarite", 2)["reste"], 36_000);

    // Annuler les 48 000 F du parent laisserait 33 000 F rendus pour 15 000 F payés : refusé.
    let premiere = s["versements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["montant"] == 48_000)
        .unwrap()["id"]
        .clone();
    assert_eq!(
        e.p.refus(
            "versement_annuler",
            Some(&e.admin),
            json!({ "id": premiere, "motif": "Erreur de saisie" }),
        )
        .await
        .cle,
        "versement.deja_rembourse"
    );
    assert_eq!(facture(&situation(&e, &awa).await, "reste"), 75_000);
}

/// Essai 4 bis : sans cantine, rien à reporter : tout le trop-payé est à rembourser.
#[tokio::test]
async fn sans_cantine_tout_le_trop_paye_est_a_rembourser() {
    let dossier = tempfile::tempdir().unwrap();
    let e = ecole(dossier.path()).await;
    let (awa, ins) = exemple(&e, "Awa", false).await;
    cocher(&e, &ins, 120_000, 0).await;
    let s = situation(&e, &awa).await;
    verifier_partage(&s);
    assert_eq!(facture(&s, "trop_paye"), 63_000);
    assert_eq!(facture(&s, "trop_paye_reporte"), 0);
    assert_eq!(facture(&s, "a_rembourser"), 63_000);
    assert_eq!(facture(&s, "reste"), 0);
    // Une seule ligne « à rembourser » : celle de la reprise porte aussi le trop-payé.
    assert_eq!(s["reprise"]["chiffres"]["a_rembourser"], 63_000);
}

/// Essai 8 : les lecteurs du reste ne voient que la part du parent : impayés, relances, guichet,
/// encaissement (proposition et refus), reçu, départ, abonnés de la cantine.
#[tokio::test]
async fn les_lecteurs_ne_voient_que_la_part_du_parent() {
    let dossier = tempfile::tempdir().unwrap();
    let e = ecole(dossier.path()).await;
    for (cle, valeur) in [
        ("sms.actif", json!({ "type": "booleen", "valeur": true })),
        ("sms.canal", json!({ "type": "texte", "valeur": "simule" })),
        (
            "sms.fenetre_envoi",
            json!({ "type": "texte", "valeur": "0-24" }),
        ),
    ] {
        e.p.ok(
            "parametre_definir",
            Some(&e.admin),
            json!({ "cle": cle, "valeur": valeur }),
        )
        .await;
    }
    // Awa, cochée après avoir payé l'inscription : son trop-payé (15 000 F) couvre la moitié de
    // la cantine. Binta, témoin, n'est pas cochée et n'a rien payé.
    let (awa, ins) = inscrit_avec_cantine(&e, "Awa").await;
    let (_, ins_b) = inscrit_avec_cantine(&e, "Binta").await;
    let s = situation(&e, &awa).await;
    payer_sur(
        &e,
        &e.admin,
        &ins,
        "awa-1",
        &[(echeance_id(&s, "inscription", 0), 15_000)],
    )
    .await;
    cocher(&e, &ins, 120_000, 0).await;
    let s = situation(&e, &awa).await;
    assert_eq!(facture(&s, "reste"), 15_000);

    // Abonnés de la cantine : le report du trop-payé, que la base n'écrit pas, est compté.
    let cantine =
        e.p.ok(
            "cantine_lire",
            Some(&e.admin),
            json!({ "date": "2026-10-05" }),
        )
        .await;
    let abonne = |c: &Value, ins: &Value| -> Value {
        c["abonnes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["inscription_id"] == ins["id"])
            .unwrap()
            .clone()
    };
    assert_eq!(abonne(&cantine, &ins)["reste_echu"], 15_000);
    assert_eq!(abonne(&cantine, &ins_b)["reste_echu"], 30_000);

    // Encaissement : le reste du parent seulement ; au-delà, refusé.
    let preparation =
        e.p.ok(
            "encaissement_preparer",
            Some(&e.admin),
            json!({ "inscription_id": ins["id"], "montant": null }),
        )
        .await;
    assert_eq!(preparation["reste"], 15_000);
    assert_eq!(
        e.p.refus(
            "encaisser",
            Some(&e.admin),
            json!({ "cle_idempotence": "awa-trop", "inscription_id": ins["id"], "montant": 15_005,
                "mode": "especes", "reference": "", "payeur_id": preparation["payeurs"][0]["responsable_id"],
                "payeur_nom": "", "repartition": null }),
        )
        .await
        .cle,
        "versement.superieur_au_reste"
    );
    let recu = payer_sur(
        &e,
        &e.admin,
        &ins,
        "awa-2",
        &[(echeance_id(&s, "cantine", 0), 15_000)],
    )
    .await;
    assert_eq!(facture(&situation(&e, &awa).await, "reste"), 0);
    // Le reçu se produit sur le reste du parent.
    let duplicata =
        e.p.ok(
            "recu_produire",
            Some(&e.admin),
            json!({ "id": recu["versement"]["id"] }),
        )
        .await;
    assert_eq!(duplicata["document"]["modele_id"], "recu");

    // Huit jours plus tard : Binta est relancée et aux impayés ; Awa, dont seul l'État est en
    // retard, ni l'un ni l'autre.
    e.p.horloge.avancer(8 * JOUR);
    let admin = e.p.connecter("admin", MOT_DE_PASSE).await;
    e.p.application.traiter_file_sms().await.unwrap();
    let messages =
        e.p.ok("messages_lire", Some(&admin), json!({ "etat": null }))
            .await;
    let relances = |numero: &str| -> usize {
        messages["messages"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|m| m["message"]["numero"] == numero && m["message"]["nature"] == "impaye")
            .count()
    };
    assert_eq!(relances("+22376000003"), 0, "Awa");
    assert_eq!(relances("+22376000005"), 1, "Binta");
    let impayes = e.p.ok("impayes_lister", Some(&admin), Value::Null).await;
    let lignes = impayes["lignes"].as_array().unwrap();
    assert!(lignes.iter().all(|l| l["inscription_id"] != ins["id"]));
    assert!(lignes.iter().any(|l| l["inscription_id"] == ins_b["id"]));

    // Guichet : rien d'échu pour Awa.
    let guichet =
        e.p.ok(
            "guichet_lire",
            Some(&admin),
            json!({ "eleve_id": awa["eleve"]["id"] }),
        )
        .await;
    let enfant = guichet["enfants"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["inscription_id"] == ins["id"])
        .unwrap();
    assert_eq!(enfant["echu"], 0);
    assert_eq!(enfant["reste"], 0);

    // Départ : aucun solde exigé du parent, alors que l'État doit encore.
    let depart =
        e.p.ok(
            "inscription_transferer",
            Some(&admin),
            json!({ "inscription_id": ins["id"], "date_depart": "2026-10-12",
                "etablissement_destination": "Lycée de la ville", "motif": "Déménagement de la famille",
                "passer_outre": "" }),
        )
        .await;
    assert_eq!(depart["reste_du"], 0);
    assert_eq!(depart["avertissements"], json!([]));
}
