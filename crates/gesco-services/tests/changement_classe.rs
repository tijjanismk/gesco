//! Changement de classe en cours d'année (décisions du porteur du 2026-11-13), par la frontière :
//! la facture reste telle quelle ou reçoit les frais de la nouvelle classe, sans que les chiffres
//! (facturé, encaissé, impayés, état des frais) se faussent ; les notes restent dans l'historique
//! de l'élève et le bulletin de la nouvelle classe ne compte que les siennes.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use crate::commun::*;
use gesco_services::session::Jeton;
use serde_json::{Value, json};

const JOUR: i64 = 86_400_000;

struct Ecole {
    p: Poste_,
    admin: Jeton,
    annee: Value,
    septa: Value,
    septb: Value,
    sixa: Value,
}

/// Année 2026-2027 : 7e A, 7e B et 6e A ; 10 000 F d'inscription partout, 150 000 F de
/// scolarité en 7e, 120 000 F en 6e, payés en trois tranches (40 %, 30 %, 30 %).
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
    for (code, libelle) in [("7", "7e A"), ("7", "7e B"), ("6", "6e A")] {
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
    let mut lignes = Vec::new();
    for (code, scolarite) in [("7", 150_000), ("6", 120_000)] {
        lignes.push(json!({ "niveau_id": niveau(code), "serie_id": null, "type_frais": "inscription",
            "libelle": "Frais d'inscription", "montant": 10_000, "obligatoire": true, "exigibilite": "une_fois" }));
        lignes.push(json!({ "niveau_id": niveau(code), "serie_id": null, "type_frais": "scolarite",
            "libelle": "Scolarité", "montant": scolarite, "obligatoire": true, "exigibilite": "selon_plan" }));
    }
    p.ok(
        "grille_modifier",
        Some(&admin),
        json!({ "id": grille["grille"]["id"], "lue_le": grille["modifiee_le"], "libelle": "Frais 2026-2027",
            "plans": plans, "lignes": lignes }),
    )
    .await;
    p.ok(
        "grille_mettre_en_vigueur",
        Some(&admin),
        json!({ "id": grille["grille"]["id"] }),
    )
    .await;
    let sixa = classes.pop().unwrap();
    let septb = classes.pop().unwrap();
    Ecole {
        septa: classes.pop().unwrap(),
        septb,
        sixa,
        p,
        admin,
        annee,
    }
}

/// Élève inscrit et validé dans une classe, avec sa mère payeuse ; rend (fiche, inscription).
async fn inscrit(e: &Ecole, prenoms: &str, classe: &Value) -> (Value, Value) {
    let fiche = e
        .p
        .ok(
            "eleve_creer",
            Some(&e.admin),
            json!({ "identite": { "nom": "Traoré", "prenoms": prenoms, "sexe": "feminin",
                "date_naissance": "2014-03-02", "date_naissance_estimee": false, "lieu_naissance": "Bamako",
                "nationalite": "Malienne", "adresse": "", "telephones": [] },
                "matricule_interne": "", "matricule_national": "", "acte_nature": "acte",
                "acte_numero": "", "acte_date": "", "ecole_origine": "" }),
        )
        .await;
    e.p.ok(
        "responsable_rattacher",
        Some(&e.admin),
        json!({ "eleve_id": fiche["eleve"]["id"], "responsable_id": null,
            "saisie": { "identite": { "nom": "Coulibaly", "prenoms": format!("Mère de {prenoms}"), "sexe": "feminin",
                "date_naissance": "", "date_naissance_estimee": false, "lieu_naissance": "", "nationalite": "",
                "adresse": "", "telephones": ["76 12 34 56"] }, "profession": "", "employeur": "" },
            "qualites": { "lien": "mere", "est_responsable_legal": true, "est_payeur": true,
                "recoit_notifications": true, "vient_chercher": false, "ordre_priorite": 1 } }),
    )
    .await;
    let inscription = e
        .p
        .ok(
            "inscription_creer",
            Some(&e.admin),
            json!({ "eleve_id": fiche["eleve"]["id"], "classe_id": classe["id"], "date_inscription": "",
                "redoublant": false, "etablissement_origine": "" }),
        )
        .await["inscription"]
        .clone();
    e.p.ok(
        "inscription_valider",
        Some(&e.admin),
        json!({ "id": inscription["id"] }),
    )
    .await;
    (fiche, inscription)
}

async fn payer(e: &Ecole, inscription: &Value, cle: &str, montant: i64) {
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

async fn situation(e: &Ecole, fiche: &Value) -> Value {
    e.p.ok(
        "situation_financiere",
        Some(&e.admin),
        json!({ "id": fiche["eleve"]["id"] }),
    )
    .await[0]
        .clone()
}

/// Facturé net d'un élève : le total de sa facture, moins les remises, les baisses portées par
/// les échéances et les avoirs nés de ses ajustements.
fn facture_nette(s: &Value) -> i64 {
    let f = &s["facture"];
    let avoirs: i64 = s["avoirs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["montant"].as_i64().unwrap())
        .sum();
    f["facture"]["total"].as_i64().unwrap()
        - f["remise"].as_i64().unwrap()
        - f["ajustement"].as_i64().unwrap()
        - avoirs
}

/// Encaissé de l'école, par catégorie de recette, sur un mois.
async fn recettes(e: &Ecole, mois: &str) -> Value {
    e.p.ok(
        "journal_tresorerie_lire",
        Some(&e.admin),
        json!({ "mois": mois }),
    )
    .await["recettes"]
        .clone()
}

async fn impayes(e: &Ecole) -> Value {
    e.p.ok("impayes_lister", Some(&e.admin), Value::Null).await
}

fn changement(inscription: &Value, classe: &Value, appliquer: bool) -> Value {
    json!({ "id": inscription["id"], "classe_id": classe["id"], "motif": "Niveau mieux adapté",
        "appliquer_frais": appliquer })
}

#[tokio::test]
async fn les_frais_au_changement_de_classe_ne_faussent_aucun_chiffre() {
    let dossier = tempfile::tempdir().unwrap();
    let mut e = ecole(dossier.path()).await;
    e.p.ok(
        "caisse_ouvrir",
        Some(&e.admin),
        json!({ "fonds_initial": 0 }),
    )
    .await;
    // Awa a payé l'inscription et deux tranches d'avance ; Binta rien ; Chaka l'inscription.
    let (awa, awa_i) = inscrit(&e, "Awa", &e.septa).await;
    let (binta, binta_i) = inscrit(&e, "Binta", &e.septa).await;
    let (chaka, chaka_i) = inscrit(&e, "Chaka", &e.septa).await;
    let (dado, dado_i) = inscrit(&e, "Dado", &e.sixa).await;
    payer(&e, &awa_i, "cle-awa-0001", 115_000).await;
    payer(&e, &chaka_i, "cle-chaka-0001", 10_000).await;
    let encaisse_octobre = recettes(&e, "2026-10").await;
    assert_eq!(
        encaisse_octobre,
        json!([
            { "categorie": "Frais d'inscription", "montant": 20_000 },
            { "categorie": "Scolarité", "montant": 105_000 },
        ])
    );

    // Mi-novembre, au milieu du premier trimestre.
    e.p.horloge.avancer(42 * JOUR);
    e.admin = e.p.connecter("admin", MOT_DE_PASSE).await;
    let impayes_avant = impayes(&e).await;

    // 7e A → 7e B, frais gardés (le choix par défaut) : aucun chiffre ne bouge.
    let chaka_avant = situation(&e, &chaka).await;
    let apercu =
        e.p.ok(
            "facture_ajustement_apercu",
            Some(&e.admin),
            json!({ "inscription_id": chaka_i["id"], "classe_id": e.septb["id"] }),
        )
        .await;
    assert_eq!(apercu["ecarts"], json!([]));
    e.p.ok(
        "inscription_changer_classe",
        Some(&e.admin),
        json!({ "id": chaka_i["id"], "classe_id": e.septb["id"], "motif": "" }),
    )
    .await;
    let chaka_apres = situation(&e, &chaka).await;
    assert_eq!(chaka_apres["facture"], chaka_avant["facture"]);
    assert_eq!(chaka_apres["classe"], "7e B");
    // Mêmes montants, élève par élève ; seule la classe affichée de Chaka change.
    let montants = |liste: &Value| {
        let lignes: Vec<(Value, Value)> = liste["lignes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| (l["eleve"].clone(), l["impaye"].clone()))
            .collect();
        (liste["echu"].clone(), liste["total"].clone(), lignes)
    };
    assert_eq!(montants(&impayes(&e).await), montants(&impayes_avant));

    // 7e A → 6e A pour Awa : l'écart se montre avant, sur les tranches à venir seulement.
    let apercu =
        e.p.ok(
            "facture_ajustement_apercu",
            Some(&e.admin),
            json!({ "inscription_id": awa_i["id"], "classe_id": e.sixa["id"] }),
        )
        .await;
    assert_eq!(apercu["classe_avant"], "7e A");
    assert_eq!(apercu["classe_apres"], "6e A");
    let ecarts: Vec<(Value, i64, i64)> = apercu["ecarts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| {
            (
                x["date"].clone(),
                x["avant"].as_i64().unwrap(),
                x["apres"].as_i64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        ecarts,
        vec![
            (json!("2027-01-05"), 45_000, 36_000),
            (json!("2027-04-05"), 45_000, 36_000)
        ]
    );
    assert_eq!(
        (
            apercu["hausse"].clone(),
            apercu["baisse"].clone(),
            apercu["avoir"].clone()
        ),
        (json!(0), json!(18_000), json!(9_000))
    );
    // Rien n'est écrit par l'aperçu.
    assert_eq!(situation(&e, &awa).await["facture"]["ajustement"], 0);

    e.p.ok(
        "inscription_changer_classe",
        Some(&e.admin),
        changement(&awa_i, &e.sixa, true),
    )
    .await;
    let s = situation(&e, &awa).await;
    let f = &s["facture"];
    // La facture n'est pas réécrite : même numéro, même total, mêmes échéances ; la baisse est
    // portée par la tranche d'avril, le trop-perçu de janvier devient un avoir.
    assert_eq!(f["facture"]["numero"], "FAC-2026-000001");
    assert_eq!(f["facture"]["total"], 160_000);
    assert_eq!(f["ajustement"], 9_000);
    assert_eq!(f["paye"], 115_000);
    assert_eq!(f["reste"], 36_000);
    assert_eq!(f["ajustements"].as_array().unwrap().len(), 1);
    assert_eq!(f["ajustements"][0]["baisse"], 18_000);
    assert_eq!(f["ajustements"][0]["avoir"], 9_000);
    assert_eq!(s["avoirs"].as_array().unwrap().len(), 1);
    assert_eq!(s["avoirs"][0]["montant"], 9_000);
    assert_eq!(s["avoirs"][0]["utilise"], 0);
    assert_eq!(facture_nette(&s), 142_000);
    // L'encaissé ne bouge pas : les paiements restent comptés tels quels.
    assert_eq!(recettes(&e, "2026-10").await, encaisse_octobre);

    // Binta, qui n'a rien payé : la baisse se porte en entier sur les tranches à venir.
    e.p.ok(
        "inscription_changer_classe",
        Some(&e.admin),
        changement(&binta_i, &e.sixa, true),
    )
    .await;
    let s = situation(&e, &binta).await;
    assert_eq!(s["facture"]["ajustement"], 18_000);
    assert_eq!(s["facture"]["reste"], 142_000);
    assert_eq!(s["avoirs"], json!([]));
    assert_eq!(facture_nette(&s), 142_000);

    // Dado, de la 6e A à la 7e A : la hausse est une ligne de facture en plus, datée et tracée.
    e.p.ok(
        "inscription_changer_classe",
        Some(&e.admin),
        changement(&dado_i, &e.septa, true),
    )
    .await;
    let s = situation(&e, &dado).await;
    let f = &s["facture"];
    assert_eq!(f["facture"]["total"], 148_000);
    let lignes = f["lignes"].as_array().unwrap();
    assert_eq!(lignes.len(), 3);
    assert_eq!(lignes[2]["libelle"], "Scolarité · ajustement 6e A → 7e A");
    assert_eq!(lignes[2]["montant"], 18_000);
    assert_eq!(f["ajustements"][0]["hausse"], 18_000);
    assert_eq!(facture_nette(&s), 148_000);

    // Le plan ne se change plus sur une facture ajustée ; sur une facture gardée, il se calcule
    // sur les frais de la facture, pas sur ceux de la classe actuelle.
    let mensuel = |s: &Value| {
        s["plans"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["libelle"] == "Mensuel")
            .unwrap()["id"]
            .clone()
    };
    assert_eq!(
        e.p.refus(
            "echeancier_changer_plan",
            Some(&e.admin),
            json!({ "inscription_id": dado_i["id"], "plan_paiement_id": mensuel(&s) }),
        )
        .await
        .cle,
        "echeancier.plan_ajuste"
    );
    e.p.ok(
        "inscription_changer_classe",
        Some(&e.admin),
        changement(&chaka_i, &e.sixa, false),
    )
    .await;
    let s = situation(&e, &chaka).await;
    let plan =
        e.p.ok(
            "echeancier_changer_plan",
            Some(&e.admin),
            json!({ "inscription_id": chaka_i["id"], "plan_paiement_id": mensuel(&s) }),
        )
        .await;
    assert_eq!(plan["facture"]["total"], 160_000);

    // L'avoir d'Awa règle sa tranche d'avril sans nouvel encaissement.
    let utilise =
        e.p.ok(
            "avoirs_utiliser",
            Some(&e.admin),
            json!({ "inscription_id": awa_i["id"] }),
        )
        .await;
    assert_eq!(utilise, 9_000);
    let s = situation(&e, &awa).await;
    assert_eq!(s["facture"]["paye"], 124_000);
    assert_eq!(s["facture"]["reste"], 27_000);
    assert_eq!(s["avoirs"][0]["utilise"], 9_000);
    assert_eq!(facture_nette(&s), 142_000);
    assert_eq!(recettes(&e, "2026-10").await, encaisse_octobre);
    assert_eq!(recettes(&e, "2026-11").await, json!([]));
    // Un avoir épuisé ne règle plus rien.
    assert_eq!(
        e.p.ok(
            "avoirs_utiliser",
            Some(&e.admin),
            json!({ "inscription_id": awa_i["id"] }),
        )
        .await,
        0
    );

    // Mi-janvier, les impayés comptent les échéances après ajustement : Binta doit l'inscription,
    // la première tranche et la tranche de janvier baissée ; Awa ne doit rien d'échu.
    e.p.horloge.avancer(60 * JOUR);
    e.admin = e.p.connecter("admin", MOT_DE_PASSE).await;
    let liste = impayes(&e).await;
    let ligne = |prenoms: &str| {
        liste["lignes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|l| l["eleve"].as_str().unwrap().starts_with(prenoms))
            .cloned()
    };
    assert!(ligne("Awa").is_none());
    let binta_l = ligne("Binta").unwrap();
    assert_eq!(binta_l["impaye"]["echu"], 10_000 + 60_000 + 36_000);
    assert_eq!(binta_l["impaye"]["total"], 142_000);
    assert_eq!(binta_l["classe"], "6e A");
    let total: i64 = liste["lignes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["impaye"]["total"].as_i64().unwrap())
        .sum();
    assert_eq!(liste["total"], total);

    // Plus tard, la direction aligne la facture de Chaka (frais gardés au changement) sur sa
    // classe actuelle : la tranche d'avril baisse, celles de janvier et d'avant restent.
    let apercu =
        e.p.ok(
            "facture_ajustement_apercu",
            Some(&e.admin),
            json!({ "inscription_id": chaka_i["id"], "classe_id": null }),
        )
        .await;
    assert_eq!(apercu["classe_apres"], "6e A");
    let applique =
        e.p.ok(
            "facture_ajuster",
            Some(&e.admin),
            json!({ "inscription_id": chaka_i["id"] }),
        )
        .await;
    assert_eq!(applique["baisse"], apercu["baisse"]);
    let s = situation(&e, &chaka).await;
    assert_eq!(s["facture"]["ajustement"], apercu["baisse"]);
    assert_eq!(
        facture_nette(&s),
        160_000 - apercu["baisse"].as_i64().unwrap()
    );
}

#[tokio::test]
async fn les_notes_restent_et_le_bulletin_ne_compte_que_celles_de_la_classe() {
    let dossier = tempfile::tempdir().unwrap();
    let mut e = ecole(dossier.path()).await;
    let (awa, awa_i) = inscrit(&e, "Awa", &e.septa).await;
    let (_, binta_i) = inscrit(&e, "Binta", &e.septa).await;
    let (_, coumba_i) = inscrit(&e, "Coumba", &e.septb).await;
    let ped =
        e.p.ok(
            "pedagogie_lire",
            Some(&e.admin),
            json!({ "id": e.annee["id"] }),
        )
        .await;
    let septieme = e.septa["niveau_id"].clone();
    let trimestre = ped["periodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["periode"]["niveau_id"] == septieme && x["periode"]["rang"] == 1)
        .unwrap()["periode"]["id"]
        .clone();
    let math = ped["matieres"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["code"] == "MATH")
        .unwrap()["id"]
        .clone();
    let evaluation = |classe: &Value, libelle: &str, date: &str| {
        json!({ "classe_id": classe["id"], "periode_id": trimestre, "matiere_id": math,
            "type_evaluation": "devoir", "libelle": libelle, "date": date, "bareme": null })
    };
    let saisir = |evaluation: &Value, inscription: &Value, texte: &str| json!({ "evaluation_id": evaluation["id"], "inscription_id": inscription["id"], "saisie": texte });
    // Un devoir en 7e A et un en 7e B, avant le changement.
    let devoir_a =
        e.p.ok(
            "evaluation_creer",
            Some(&e.admin),
            evaluation(&e.septa, "Devoir 1", "2026-10-04"),
        )
        .await;
    let devoir_b =
        e.p.ok(
            "evaluation_creer",
            Some(&e.admin),
            evaluation(&e.septb, "Devoir 1", "2026-10-04"),
        )
        .await;
    e.p.ok(
        "note_saisir",
        Some(&e.admin),
        saisir(&devoir_a, &awa_i, "14"),
    )
    .await;
    e.p.ok(
        "note_saisir",
        Some(&e.admin),
        saisir(&devoir_a, &binta_i, "10"),
    )
    .await;
    e.p.ok(
        "note_saisir",
        Some(&e.admin),
        saisir(&devoir_b, &coumba_i, "11"),
    )
    .await;
    // Le bulletin de 7e A est calculé avant le départ d'Awa.
    let demande_a = json!({ "classe_id": e.septa["id"], "periode_id": trimestre });
    let demande_b = json!({ "classe_id": e.septb["id"], "periode_id": trimestre });
    let calcul =
        e.p.ok("resultats_calculer", Some(&e.admin), demande_a.clone())
            .await;
    assert_eq!(calcul["bulletins"].as_array().unwrap().len(), 2);

    // Deux semaines plus tard, Awa passe en 7e B.
    e.p.horloge.avancer(16 * JOUR);
    e.admin = e.p.connecter("admin", MOT_DE_PASSE).await;
    e.p.ok(
        "inscription_changer_classe",
        Some(&e.admin),
        json!({ "id": awa_i["id"], "classe_id": e.septb["id"], "motif": "" }),
    )
    .await;

    // Le devoir de 7e B fait avant son arrivée ne l'attend pas : il se valide sans sa note.
    let grille =
        e.p.ok(
            "notes_grille",
            Some(&e.admin),
            json!({ "id": devoir_b["id"] }),
        )
        .await;
    assert_eq!(grille["lignes"].as_array().unwrap().len(), 1);
    e.p.ok(
        "evaluation_valider",
        Some(&e.admin),
        json!({ "id": devoir_b["id"] }),
    )
    .await;
    // Un devoir de 7e B après son arrivée la compte.
    let devoir_b2 =
        e.p.ok(
            "evaluation_creer",
            Some(&e.admin),
            evaluation(&e.septb, "Devoir 2", "2026-10-20"),
        )
        .await;
    e.p.ok(
        "note_saisir",
        Some(&e.admin),
        saisir(&devoir_b2, &awa_i, "12"),
    )
    .await;
    e.p.ok(
        "note_saisir",
        Some(&e.admin),
        saisir(&devoir_b2, &coumba_i, "13"),
    )
    .await;

    // Le bulletin de 7e B ne compte que la note prise en 7e B.
    let calcul =
        e.p.ok("resultats_calculer", Some(&e.admin), demande_b.clone())
            .await;
    let bulletin_awa = calcul["bulletins"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["bulletin"]["inscription_id"] == awa_i["id"])
        .unwrap();
    let ligne_math = bulletin_awa["bulletin"]["contenu"]["lignes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["matiere_id"] == math)
        .unwrap();
    assert_eq!(ligne_math["note_de_classe"], 1200);

    // En 7e A, son bulletin non publié n'est plus montré ni validé avec la classe.
    e.p.ok(
        "evaluation_valider",
        Some(&e.admin),
        json!({ "id": devoir_a["id"] }),
    )
    .await;
    let lus =
        e.p.ok("resultats_calculer", Some(&e.admin), demande_a.clone())
            .await;
    assert_eq!(lus["bulletins"].as_array().unwrap().len(), 1);
    let valides =
        e.p.ok("bulletins_valider", Some(&e.admin), demande_a.clone())
            .await;
    assert_eq!(valides, 1);

    // Ses notes restent dans son historique, celles des deux classes.
    let notes =
        e.p.ok(
            "eleve_notes",
            Some(&e.admin),
            json!({ "id": awa["eleve"]["id"] }),
        )
        .await;
    let resume: Vec<(Value, Value)> = notes
        .as_array()
        .unwrap()
        .iter()
        .map(|n| (n["classe"].clone(), n["note"]["valeur"].clone()))
        .collect();
    assert_eq!(
        resume,
        vec![(json!("7e A"), json!(1400)), (json!("7e B"), json!(1200))]
    );
    assert_eq!(notes[0]["periode"], "1er trimestre");
}
