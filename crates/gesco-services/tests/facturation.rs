//! Facture et échéancier par la frontière (SPEC_PHASE_3 §4, assertions 3, 4 et 5).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod commun;

use commun::*;
use gesco_services::session::Jeton;
use serde_json::{Value, json};
use sqlx::Connection;

struct Ecole {
    p: Poste_,
    admin: Jeton,
    annee: Value,
    septieme: Value,
    huitieme: Value,
    cantine: Value,
    plans: Vec<Value>,
}

/// Année 2026-2027 ouverte, classes 7e A et 8e A ; grille : la 7e seulement, avec une cantine
/// facultative ; la 8e n'a aucun frais.
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
    let sept = niveau("7");
    p.ok(
        "grille_modifier",
        Some(&admin),
        json!({ "id": grille["grille"]["id"], "libelle": "Frais 2026-2027", "plans": plans, "lignes": [
            { "niveau_id": sept, "serie_id": null, "type_frais": "inscription", "libelle": "Frais d'inscription",
              "montant": 10_000, "obligatoire": true, "exigibilite": "une_fois" },
            { "niveau_id": sept, "serie_id": null, "type_frais": "scolarite", "libelle": "Scolarité",
              "montant": 150_000, "obligatoire": true, "exigibilite": "selon_plan" },
            { "niveau_id": sept, "serie_id": null, "type_frais": "cantine", "libelle": "Cantine",
              "montant": 45_000, "obligatoire": false, "exigibilite": "une_fois" },
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
    Ecole {
        cantine: contenu["lignes"][2]["id"].clone(),
        plans: contenu["plans"].as_array().unwrap().clone(),
        p,
        admin,
        annee,
        huitieme: classes[1].clone(),
        septieme: classes.remove(0),
    }
}

/// Élève complet avec sa mère, responsable légale joignable : prêt à être validé.
async fn eleve(e: &Ecole, prenoms: &str) -> Value {
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
            "saisie": { "identite": { "nom": "Coulibaly", "prenoms": "Mariam", "sexe": "feminin",
                "date_naissance": "", "date_naissance_estimee": false, "lieu_naissance": "", "nationalite": "",
                "adresse": "", "telephones": ["76 12 34 56"] }, "profession": "", "employeur": "" },
            "qualites": { "lien": "mere", "est_responsable_legal": true, "est_payeur": true,
                "recoit_notifications": true, "vient_chercher": false, "ordre_priorite": 1 } }),
    )
    .await;
    fiche
}

async fn brouillon(e: &Ecole, fiche: &Value, classe: &Value, date: &str) -> Value {
    e.p.ok(
        "inscription_creer",
        Some(&e.admin),
        json!({ "eleve_id": fiche["eleve"]["id"], "classe_id": classe["id"], "date_inscription": date,
            "redoublant": false, "etablissement_origine": "" }),
    )
    .await["inscription"]
        .clone()
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

fn montants(facture: &Value) -> Vec<i64> {
    facture["echeances"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["echeance"]["montant"].as_i64().unwrap())
        .collect()
}

fn nets(facture: &Value) -> Vec<i64> {
    facture["echeances"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["net"].as_i64().unwrap())
        .collect()
}

#[tokio::test]
async fn la_validation_cree_la_facture_et_son_echeancier() {
    let dossier = tempfile::tempdir().unwrap();
    let e = ecole(dossier.path()).await;

    // Validation avec la cantine cochée et le plan par défaut (trois tranches).
    let awa = eleve(&e, "Awa").await;
    let inscription = brouillon(&e, &awa, &e.septieme, "").await;
    e.p.ok(
        "inscription_valider",
        Some(&e.admin),
        json!({ "id": inscription["id"], "choix": { "frais_optionnels": [e.cantine] } }),
    )
    .await;
    let s = situation(&e, &awa).await;
    let facture = &s["facture"];
    assert_eq!(facture["facture"]["numero"], "FAC-2026-000001");
    assert_eq!(facture["facture"]["statut"], "emise");
    assert_eq!(facture["facture"]["total"], 205_000);
    assert_eq!(facture["reste"], 205_000);
    assert_eq!(facture["plan"], "Trois tranches");
    assert_eq!(facture["lignes"].as_array().unwrap().len(), 3);
    assert_eq!(
        montants(facture),
        vec![10_000, 45_000, 60_000, 45_000, 45_000]
    );
    assert_eq!(s["plans"].as_array().unwrap().len(), 3);

    // Le parent passe au comptant : la scolarité devient une seule échéance, le reste ne bouge pas.
    let comptant = e.plans.iter().find(|p| p["libelle"] == "Comptant").unwrap();
    let change =
        e.p.ok(
            "echeancier_changer_plan",
            Some(&e.admin),
            json!({ "inscription_id": inscription["id"], "plan_paiement_id": comptant["id"] }),
        )
        .await;
    assert_eq!(change["plan"], "Comptant");
    assert_eq!(change["facture"]["total"], 205_000);
    assert_eq!(montants(&change), vec![10_000, 45_000, 150_000]);

    // Une classe dont le niveau n'a aucun frais : refus, et le brouillon reste un brouillon.
    let binta = eleve(&e, "Binta").await;
    let huitieme = brouillon(&e, &binta, &e.huitieme, "").await;
    assert_eq!(
        e.p.refus(
            "inscription_valider",
            Some(&e.admin),
            json!({ "id": huitieme["id"] })
        )
        .await
        .cle,
        "facture.grille_absente"
    );
    let s = situation(&e, &binta).await;
    assert_eq!(s["inscription"]["statut"], "brouillon");
    assert_eq!(s["facture"], Value::Null);

    // Un frais obligatoire ne se « coche » pas.
    let fanta = eleve(&e, "Fanta").await;
    let ins_fanta = brouillon(&e, &fanta, &e.septieme, "").await;
    let scolarite_id =
        e.p.ok(
            "grilles_lister",
            Some(&e.admin),
            json!({ "annee_id": e.annee["id"] }),
        )
        .await[0]["contenu"]["lignes"][1]["id"]
            .clone();
    assert_eq!(
        e.p.refus(
            "inscription_valider",
            Some(&e.admin),
            json!({ "id": ins_fanta["id"], "choix": { "frais_optionnels": [scolarite_id] } })
        )
        .await
        .cle,
        "facture.option_inconnue"
    );

    // Abandon d'une inscription validée : la facture est annulée, ses échéances contre-passées.
    e.p.ok(
        "inscription_abandonner",
        Some(&e.admin),
        json!({ "id": inscription["id"], "motif": "Départ de la famille" }),
    )
    .await;
    assert_eq!(situation(&e, &awa).await["facture"], Value::Null);
}

#[tokio::test]
async fn un_eleve_arrive_en_cours_d_annee_selon_le_reglage_de_l_ecole() {
    let dossier = tempfile::tempdir().unwrap();
    let e = ecole(dossier.path()).await;

    // Défaut : frais d'inscription entiers, tranches non échues seulement (janvier et avril).
    let awa = eleve(&e, "Awa").await;
    let ins = brouillon(&e, &awa, &e.septieme, "2026-12-01").await;
    e.p.ok(
        "inscription_valider",
        Some(&e.admin),
        json!({ "id": ins["id"] }),
    )
    .await;
    let s = situation(&e, &awa).await;
    assert_eq!(s["facture"]["facture"]["total"], 100_000);
    assert_eq!(montants(&s["facture"]), vec![10_000, 45_000, 45_000]);

    // Au prorata des mois restants de l'année, réparti sur les tranches à venir.
    e.p.ok(
        "parametre_definir",
        Some(&e.admin),
        json!({ "cle": "facturation.arrivee_en_cours", "valeur": { "type": "texte", "valeur": "prorata_mensuel" } }),
    )
    .await;
    let binta = eleve(&e, "Binta").await;
    let ins = brouillon(&e, &binta, &e.septieme, "2026-12-01").await;
    e.p.ok(
        "inscription_valider",
        Some(&e.admin),
        json!({ "id": ins["id"] }),
    )
    .await;
    let s = situation(&e, &binta).await;
    let scolarite: i64 = montants(&s["facture"])[1..].iter().sum();
    assert!(scolarite < 150_000 && scolarite > 90_000, "{scolarite}");
    assert_eq!(scolarite % 5, 0, "arrondi au multiple de l'école");
    assert_eq!(
        e.p.refus(
            "parametre_definir",
            Some(&e.admin),
            json!({ "cle": "facturation.arrivee_en_cours", "valeur": { "type": "texte", "valeur": "gratuit" } }),
        )
        .await
        .cle,
        "parametre.hors_bornes"
    );
}

#[tokio::test]
async fn les_inscriptions_validees_sans_facture_se_rattrapent_apres_sauvegarde() {
    let dossier = tempfile::tempdir().unwrap();
    let e = ecole(dossier.path()).await;
    let awa = eleve(&e, "Awa").await;
    let ins = brouillon(&e, &awa, &e.septieme, "").await;
    e.p.ok(
        "inscription_valider",
        Some(&e.admin),
        json!({ "id": ins["id"] }),
    )
    .await;
    assert!(
        e.p.ok("factures_rattraper_apercu", Some(&e.admin), Value::Null)
            .await
            .as_array()
            .unwrap()
            .is_empty()
    );

    // Base de la phase 2 : l'inscription est validée et n'a pas de facture. On l'imite en
    // annulant la facture directement dans la base.
    let mut cnx = base_brute(dossier.path(), e.p.trousseau.clone()).await;
    sqlx::query("UPDATE facture SET statut = 'annulee'")
        .execute(&mut cnx)
        .await
        .unwrap();
    cnx.close().await.unwrap();

    let avertissements = e.p.ok("avertissements", Some(&e.admin), Value::Null).await;
    assert!(
        avertissements
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["cle"] == "avertissement.inscriptions_sans_facture")
    );
    let apercu =
        e.p.ok("factures_rattraper_apercu", Some(&e.admin), Value::Null)
            .await;
    assert_eq!(apercu.as_array().unwrap().len(), 1);
    assert_eq!(apercu[0]["total"], 160_000);
    assert_eq!(apercu[0]["classe"], "7e A");

    let sauvegardes =
        e.p.ok("sauvegardes_lister", Some(&e.admin), Value::Null)
            .await;
    let rapport =
        e.p.ok("factures_rattraper", Some(&e.admin), Value::Null)
            .await;
    assert_eq!(rapport["facturees"], 1);
    assert_eq!(rapport["ignorees"], 0);
    assert_eq!(
        e.p.ok("sauvegardes_lister", Some(&e.admin), Value::Null)
            .await
            .as_array()
            .unwrap()
            .len(),
        sauvegardes.as_array().unwrap().len() + 1
    );
    assert_eq!(
        situation(&e, &awa).await["facture"]["facture"]["numero"],
        "FAC-2026-000002"
    );
    // Une seule fois : plus rien à rattraper, plus d'avertissement.
    assert!(
        e.p.ok("factures_rattraper_apercu", Some(&e.admin), Value::Null)
            .await
            .as_array()
            .unwrap()
            .is_empty()
    );
    let avertissements = e.p.ok("avertissements", Some(&e.admin), Value::Null).await;
    assert!(
        !avertissements
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["cle"] == "avertissement.inscriptions_sans_facture")
    );
}

#[tokio::test]
async fn une_remise_proposee_puis_accordee_reduit_les_dernieres_tranches() {
    let dossier = tempfile::tempdir().unwrap();
    let e = ecole(dossier.path()).await;
    let awa = eleve(&e, "Awa").await;
    let ins = brouillon(&e, &awa, &e.septieme, "").await;
    e.p.ok(
        "inscription_valider",
        Some(&e.admin),
        json!({ "id": ins["id"] }),
    )
    .await;

    // La secrétaire propose ; elle ne peut pas accorder.
    e.p.ok(
        "utilisateur_creer",
        Some(&e.admin),
        json!({ "identifiant": "fanta", "nom_affiche": "Fanta Coulibaly",
            "mot_de_passe_initial": "secretariat-2026", "roles": ["secretaire"] }),
    )
    .await;
    let secretaire = e.p.connecter("fanta", "secretariat-2026").await;
    let proposer = |valeur: Value, motif: &str| {
        json!({ "inscription_id": ins["id"], "type_remise": "fratrie", "valeur": valeur,
            "type_frais": "scolarite", "motif": motif })
    };
    assert_eq!(
        e.p.refus(
            "remise_proposer",
            Some(&secretaire),
            proposer(json!({ "type": "taux", "pour_cent": 50 }), " ")
        )
        .await
        .cle,
        "remise.motif_obligatoire"
    );
    let remise =
        e.p.ok(
            "remise_proposer",
            Some(&secretaire),
            proposer(
                json!({ "type": "taux", "pour_cent": 50 }),
                "Trois enfants inscrits",
            ),
        )
        .await;
    assert_eq!(remise["statut"], "proposee");
    assert_eq!(
        e.p.refus(
            "remise_accorder",
            Some(&secretaire),
            json!({ "id": remise["id"] })
        )
        .await
        .cle,
        "erreur.permission_refusee"
    );
    let attente =
        e.p.ok("remises_en_attente", Some(&e.admin), Value::Null)
            .await;
    assert_eq!(attente[0]["eleve"], "Awa Traoré");
    assert_eq!(attente[0]["classe"], "7e A");

    // La direction accorde : 50 % de 150 000 = 75 000, pris sur les deux dernières tranches.
    let accordee =
        e.p.ok(
            "remise_accorder",
            Some(&e.admin),
            json!({ "id": remise["id"] }),
        )
        .await;
    assert_eq!(accordee["statut"], "accordee");
    assert_eq!(accordee["montant_applique"], 75_000);
    let s = situation(&e, &awa).await;
    assert_eq!(nets(&s["facture"]), vec![10_000, 60_000, 15_000, 0]);
    assert_eq!(s["facture"]["remise"], 75_000);
    assert_eq!(s["facture"]["reste"], 85_000);
    assert_eq!(s["remises"].as_array().unwrap().len(), 1);

    // Le parent passe au mensuel : la remise se répartit à nouveau, depuis juin.
    let mensuel = e.plans.iter().find(|p| p["libelle"] == "Mensuel").unwrap();
    let change =
        e.p.ok(
            "echeancier_changer_plan",
            Some(&e.admin),
            json!({ "inscription_id": ins["id"], "plan_paiement_id": mensuel["id"] }),
        )
        .await;
    assert_eq!(change["remise"], 75_000);
    assert_eq!(change["reste"], 85_000);
    let n = nets(&change);
    assert_eq!(n.last(), Some(&0));
    assert_eq!(n.iter().sum::<i64>(), 85_000);

    // Une remise supérieure au reste est refusée ; un refus et une annulation sont motivés.
    let trop =
        e.p.ok(
            "remise_proposer",
            Some(&e.admin),
            proposer(
                json!({ "type": "montant", "montant": 90_000 }),
                "Erreur de saisie",
            ),
        )
        .await;
    assert_eq!(
        e.p.refus(
            "remise_accorder",
            Some(&e.admin),
            json!({ "id": trop["id"] })
        )
        .await
        .cle,
        "remise.superieure_au_reste"
    );
    let refusee =
        e.p.ok(
            "remise_refuser",
            Some(&e.admin),
            json!({ "id": trop["id"], "motif": "Montant trop élevé" }),
        )
        .await;
    assert_eq!(refusee["statut"], "refusee");
    e.p.ok(
        "remise_annuler",
        Some(&e.admin),
        json!({ "id": remise["id"], "motif": "La fratrie a quitté l'école" }),
    )
    .await;
    let s = situation(&e, &awa).await;
    assert_eq!(s["facture"]["remise"], 0);
    assert_eq!(s["facture"]["reste"], 160_000);
}

#[tokio::test]
async fn une_remise_accordee_avant_la_validation_s_applique_a_la_facture() {
    let dossier = tempfile::tempdir().unwrap();
    let e = ecole(dossier.path()).await;
    let awa = eleve(&e, "Awa").await;
    let ins = brouillon(&e, &awa, &e.septieme, "").await;
    let remise =
        e.p.ok(
            "remise_proposer",
            Some(&e.admin),
            json!({ "inscription_id": ins["id"], "type_remise": "enfant_personnel",
                "valeur": { "type": "montant", "montant": 50_000 }, "type_frais": "scolarite",
                "motif": "Fille de l'économe" }),
        )
        .await;
    let accordee =
        e.p.ok(
            "remise_accorder",
            Some(&e.admin),
            json!({ "id": remise["id"] }),
        )
        .await;
    assert_eq!(accordee["montant_applique"], 0, "pas encore de facture");
    e.p.ok(
        "inscription_valider",
        Some(&e.admin),
        json!({ "id": ins["id"] }),
    )
    .await;
    let s = situation(&e, &awa).await;
    assert_eq!(s["facture"]["remise"], 50_000);
    assert_eq!(nets(&s["facture"]), vec![10_000, 60_000, 40_000, 0]);
    assert_eq!(s["remises"][0]["montant_applique"], 50_000);
}

fn ids_echeances(facture: &Value) -> Vec<Value> {
    facture["echeances"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["echeance"]["id"].clone())
        .collect()
}

fn demande_encaissement(
    ins: &Value,
    cle: &str,
    montant: i64,
    mode: &str,
    reference: &str,
    payeur: &Value,
) -> Value {
    json!({ "cle_idempotence": cle, "inscription_id": ins["id"], "montant": montant, "mode": mode,
        "reference": reference, "payeur_id": payeur, "payeur_nom": "", "repartition": null })
}

#[tokio::test]
async fn un_versement_s_impute_se_numerote_et_ne_se_double_pas() {
    let dossier = tempfile::tempdir().unwrap();
    let e = ecole(dossier.path()).await;
    let awa = eleve(&e, "Awa").await;
    let ins = brouillon(&e, &awa, &e.septieme, "").await;
    e.p.ok(
        "inscription_valider",
        Some(&e.admin),
        json!({ "id": ins["id"] }),
    )
    .await;

    // Sans caisse ouverte, rien ne s'encaisse.
    let preparation =
        e.p.ok(
            "encaissement_preparer",
            Some(&e.admin),
            json!({ "inscription_id": ins["id"], "montant": 40_000 }),
        )
        .await;
    assert_eq!(preparation["caisse_ouverte"], false);
    assert_eq!(preparation["reste"], 160_000);
    assert_eq!(preparation["eleve"], "Awa Traoré");
    let mere = preparation["payeurs"][0]["responsable_id"].clone();
    assert_eq!(preparation["payeurs"][0]["nom"], "Mariam Coulibaly");
    let proposees: Vec<i64> = preparation["imputation"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["montant"].as_i64().unwrap())
        .collect();
    assert_eq!(proposees, vec![10_000, 30_000], "la plus ancienne d'abord");
    assert_eq!(
        e.p.refus(
            "encaisser",
            Some(&e.admin),
            demande_encaissement(&ins, "cle-awa-0001", 40_000, "especes", "", &mere)
        )
        .await
        .cle,
        "caisse.session_fermee"
    );
    e.p.ok(
        "caisse_ouvrir",
        Some(&e.admin),
        json!({ "fonds_initial": 10_000 }),
    )
    .await;

    // Premier versement en espèces : reçu numéroté, PDF, imputations.
    let recu =
        e.p.ok(
            "encaisser",
            Some(&e.admin),
            demande_encaissement(&ins, "cle-awa-0001", 40_000, "especes", "", &mere),
        )
        .await;
    assert_eq!(recu["versement"]["numero_recu"], "REC-2026-000001");
    assert_eq!(recu["versement"]["payeur_nom"], "Mariam Coulibaly");
    assert_eq!(recu["recu"]["document"]["modele_id"], "recu");
    // Relecture à l'œil : `GESCO_ECRIRE_RECU=chemin.pdf` dépose le reçu composé.
    if let Some(chemin) = std::env::var_os("GESCO_ECRIRE_RECU") {
        use base64::Engine;
        let pdf = base64::engine::general_purpose::STANDARD
            .decode(recu["recu"]["pdf"].as_str().unwrap())
            .unwrap();
        std::fs::write(chemin, pdf).unwrap();
    }
    // Rejoué (double clic, coupure) : même versement, pas de seconde recette.
    let rejoue =
        e.p.ok(
            "encaisser",
            Some(&e.admin),
            demande_encaissement(&ins, "cle-awa-0001", 40_000, "especes", "", &mere),
        )
        .await;
    assert_eq!(rejoue["versement"]["id"], recu["versement"]["id"]);
    let s = situation(&e, &awa).await;
    assert_eq!(s["facture"]["paye"], 40_000);
    assert_eq!(s["facture"]["reste"], 120_000);
    assert_eq!(s["facture"]["facture"]["statut"], "partiellement_reglee");
    assert_eq!(s["versements"].as_array().unwrap().len(), 1);

    // Orange Money : la référence est exigée ; répartition à la main sur la dernière tranche.
    assert_eq!(
        e.p.refus(
            "encaisser",
            Some(&e.admin),
            demande_encaissement(&ins, "cle-awa-0002", 25_000, "orange_money", " ", &mere)
        )
        .await
        .cle,
        "versement.reference_obligatoire"
    );
    let derniere = ids_echeances(&s["facture"]).last().unwrap().clone();
    let mut manuel = demande_encaissement(
        &ins,
        "cle-awa-0002",
        25_000,
        "orange_money",
        "OM-7781-22",
        &mere,
    );
    manuel["repartition"] = json!([{ "echeance_id": derniere, "montant": 25_000 }]);
    let recu2 = e.p.ok("encaisser", Some(&e.admin), manuel).await;
    assert_eq!(recu2["versement"]["numero_recu"], "REC-2026-000002");
    let s = situation(&e, &awa).await;
    assert_eq!(s["facture"]["echeances"][3]["paye"], 25_000);
    assert_eq!(s["facture"]["reste"], 95_000);

    // Plus que le reste : refusé (T-18). Une tranche payée verrouille le plan.
    assert_eq!(
        e.p.refus(
            "encaisser",
            Some(&e.admin),
            demande_encaissement(&ins, "cle-awa-0003", 95_001, "especes", "", &mere)
        )
        .await
        .cle,
        "versement.superieur_au_reste"
    );
    let comptant = e.plans.iter().find(|p| p["libelle"] == "Comptant").unwrap();
    assert_eq!(
        e.p.refus(
            "echeancier_changer_plan",
            Some(&e.admin),
            json!({ "inscription_id": ins["id"], "plan_paiement_id": comptant["id"] })
        )
        .await
        .cle,
        "echeancier.plan_verrouille"
    );

    // Totaux de caisse : espèces au tiroir, Orange Money à part.
    let caisse =
        e.p.ok("caisse_session_courante", Some(&e.admin), Value::Null)
            .await;
    assert_eq!(caisse["totaux"]["entrees_especes"], 40_000);
    assert_eq!(caisse["totaux"]["theorique"], 50_000);
    assert_eq!(caisse["totaux"]["nombre_recus"], 2);
    assert!(
        caisse["totaux"]["par_mode"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["mode"] == "orange_money" && m["montant"] == 25_000)
    );

    // Le solde : la facture est soldée. Duplicata et relevé s'impriment.
    e.p.ok(
        "encaisser",
        Some(&e.admin),
        demande_encaissement(&ins, "cle-awa-0004", 95_000, "especes", "", &mere),
    )
    .await;
    let s = situation(&e, &awa).await;
    assert_eq!(s["facture"]["facture"]["statut"], "soldee");
    assert_eq!(s["facture"]["reste"], 0);
    let duplicata =
        e.p.ok(
            "recu_produire",
            Some(&e.admin),
            json!({ "id": recu["versement"]["id"] }),
        )
        .await;
    assert_eq!(duplicata["document"]["modele_id"], "recu");
    let releve =
        e.p.ok(
            "releve_compte_produire",
            Some(&e.admin),
            json!({ "id": awa["eleve"]["id"] }),
        )
        .await;
    assert_eq!(releve["document"]["modele_id"], "releve_compte");
}

#[tokio::test]
async fn un_parent_paie_pour_deux_enfants_un_recu_chacun() {
    let dossier = tempfile::tempdir().unwrap();
    let e = ecole(dossier.path()).await;
    let mut inscriptions = Vec::new();
    for prenom in ["Awa", "Binta"] {
        let fiche = eleve(&e, prenom).await;
        let ins = brouillon(&e, &fiche, &e.septieme, "").await;
        e.p.ok(
            "inscription_valider",
            Some(&e.admin),
            json!({ "id": ins["id"] }),
        )
        .await;
        inscriptions.push(ins);
    }
    e.p.ok(
        "caisse_ouvrir",
        Some(&e.admin),
        json!({ "fonds_initial": 0 }),
    )
    .await;
    let famille = json!({ "cle_idempotence": "famille-0001", "mode": "especes", "reference": "",
        "payeur_id": null, "payeur_nom": "Moussa Traoré",
        "parts": [
            { "inscription_id": inscriptions[0]["id"], "montant": 70_000 },
            { "inscription_id": inscriptions[1]["id"], "montant": 10_000 },
        ] });
    let recus =
        e.p.ok("encaisser_famille", Some(&e.admin), famille.clone())
            .await;
    assert_eq!(recus.as_array().unwrap().len(), 2);
    assert_eq!(recus[0]["versement"]["numero_recu"], "REC-2026-000001");
    assert_eq!(recus[1]["versement"]["numero_recu"], "REC-2026-000002");
    assert_eq!(recus[1]["versement"]["payeur_nom"], "Moussa Traoré");
    let rejoue = e.p.ok("encaisser_famille", Some(&e.admin), famille).await;
    assert_eq!(rejoue[1]["versement"]["id"], recus[1]["versement"]["id"]);
    let caisse =
        e.p.ok("caisse_session_courante", Some(&e.admin), Value::Null)
            .await;
    assert_eq!(caisse["totaux"]["entrees_especes"], 80_000);
    // Un tiers sans nom est refusé.
    assert_eq!(
        e.p.refus(
            "encaisser_famille",
            Some(&e.admin),
            json!({ "cle_idempotence": "famille-0002", "mode": "especes", "reference": "", "payeur_id": null,
                "payeur_nom": " ", "parts": [{ "inscription_id": inscriptions[0]["id"], "montant": 1 }] })
        )
        .await
        .cle,
        "versement.payeur_obligatoire"
    );
}

#[tokio::test]
async fn l_abandon_garde_le_paye_et_annule_les_tranches_a_venir() {
    let dossier = tempfile::tempdir().unwrap();
    let e = ecole(dossier.path()).await;
    let awa = eleve(&e, "Awa").await;
    let ins = brouillon(&e, &awa, &e.septieme, "").await;
    e.p.ok(
        "inscription_valider",
        Some(&e.admin),
        json!({ "id": ins["id"] }),
    )
    .await;
    e.p.ok(
        "caisse_ouvrir",
        Some(&e.admin),
        json!({ "fonds_initial": 0 }),
    )
    .await;
    let mut versement =
        demande_encaissement(&ins, "cle-abandon-1", 70_000, "especes", "", &Value::Null);
    versement["payeur_nom"] = json!("Mariam Coulibaly");
    e.p.ok("encaisser", Some(&e.admin), versement).await;
    // Une remise ne peut plus dépasser ce qui reste après versement.
    let remise =
        e.p.ok(
            "remise_proposer",
            Some(&e.admin),
            json!({ "inscription_id": ins["id"], "type_remise": "bourse",
                "valeur": { "type": "montant", "montant": 90_001 },
                "type_frais": "scolarite", "motif": "Bourse de l'État" }),
        )
        .await;
    assert_eq!(
        e.p.refus(
            "remise_accorder",
            Some(&e.admin),
            json!({ "id": remise["id"] })
        )
        .await
        .cle,
        "remise.superieure_au_reste"
    );
    e.p.ok(
        "inscription_abandonner",
        Some(&e.admin),
        json!({ "id": ins["id"], "motif": "Déménagement à Kayes" }),
    )
    .await;
    let s = situation(&e, &awa).await;
    // La facture reste : 70 000 payés sur l'inscription et la 1re tranche ; janvier et avril
    // sont annulées.
    assert_eq!(s["facture"]["facture"]["total"], 70_000);
    assert_eq!(s["facture"]["paye"], 70_000);
    assert_eq!(s["facture"]["reste"], 0);
    assert_eq!(s["facture"]["facture"]["statut"], "soldee");
    assert_eq!(montants(&s["facture"]), vec![10_000, 60_000]);
}

fn a_l_avertissement(avertissements: &Value, cle: &str) -> bool {
    avertissements
        .as_array()
        .unwrap()
        .iter()
        .any(|a| a["cle"] == cle)
}

#[tokio::test]
async fn un_versement_annule_se_contre_passe_ou_se_regularise_dans_la_caisse_du_jour() {
    let dossier = tempfile::tempdir().unwrap();
    let e = ecole(dossier.path()).await;
    let awa = eleve(&e, "Awa").await;
    let ins = brouillon(&e, &awa, &e.septieme, "").await;
    e.p.ok(
        "inscription_valider",
        Some(&e.admin),
        json!({ "id": ins["id"] }),
    )
    .await;
    e.p.ok(
        "utilisateur_creer",
        Some(&e.admin),
        json!({ "identifiant": "oumou", "nom_affiche": "Oumou Diarra",
            "mot_de_passe_initial": "caisse-ecole-2026", "roles": ["caissier"] }),
    )
    .await;
    let caissiere = e.p.connecter("oumou", "caisse-ecole-2026").await;
    e.p.ok(
        "caisse_ouvrir",
        Some(&caissiere),
        json!({ "fonds_initial": 10_000 }),
    )
    .await;
    let mut premier =
        demande_encaissement(&ins, "cle-annul-0001", 40_000, "especes", "", &Value::Null);
    premier["payeur_nom"] = json!("Mariam Coulibaly");
    let recu = e.p.ok("encaisser", Some(&caissiere), premier).await;
    let id = recu["versement"]["id"].clone();

    // Le caissier n'annule pas (§11.5) ; le motif est obligatoire.
    assert_eq!(
        e.p.refus(
            "versement_annuler",
            Some(&caissiere),
            json!({ "id": id, "motif": "Erreur de saisie" })
        )
        .await
        .cle,
        "erreur.permission_refusee"
    );
    assert_eq!(
        e.p.refus(
            "versement_annuler",
            Some(&e.admin),
            json!({ "id": id, "motif": "  " })
        )
        .await
        .cle,
        "versement.motif_obligatoire"
    );

    // Session encore ouverte : simple contre-passation, sans sortie.
    let annulation =
        e.p.ok(
            "versement_annuler",
            Some(&e.admin),
            json!({ "id": id, "motif": "Erreur de saisie" }),
        )
        .await;
    assert_eq!(annulation["versement"]["statut"], "annule");
    assert_eq!(
        annulation["versement"]["motif_annulation"],
        "Erreur de saisie"
    );
    assert_eq!(annulation["regularisation"], Value::Null);
    assert_eq!(annulation["recu"]["document"]["modele_id"], "recu");
    let s = situation(&e, &awa).await;
    assert_eq!(s["facture"]["paye"], 0);
    assert_eq!(s["facture"]["reste"], 160_000);
    assert_eq!(s["facture"]["facture"]["statut"], "emise");
    assert_eq!(s["versements"][0]["statut"], "annule");
    let caisse =
        e.p.ok("caisse_session_courante", Some(&caissiere), Value::Null)
            .await;
    assert_eq!(caisse["totaux"]["entrees_especes"], 0);
    assert_eq!(caisse["totaux"]["theorique"], 10_000);
    assert_eq!(caisse["totaux"]["nombre_recus"], 0);
    assert_eq!(
        e.p.refus(
            "versement_annuler",
            Some(&e.admin),
            json!({ "id": id, "motif": "Deux fois" })
        )
        .await
        .cle,
        "versement.deja_annule"
    );
    // Le reçu annulé se réimprime, marqué annulé.
    e.p.ok("recu_produire", Some(&e.admin), json!({ "id": id }))
        .await;

    // Un second versement, puis la caisse se clôture : son annulation exige une caisse ouverte
    // chez qui annule, avec assez d'espèces pour la sortie.
    let mut second =
        demande_encaissement(&ins, "cle-annul-0002", 50_000, "especes", "", &Value::Null);
    second["payeur_nom"] = json!("Mariam Coulibaly");
    let second = e.p.ok("encaisser", Some(&caissiere), second).await;
    let id2 = second["versement"]["id"].clone();
    e.p.ok(
        "caisse_cloturer",
        Some(&caissiere),
        json!({ "total_compte": 60_000, "motif_ecart": "" }),
    )
    .await;
    assert_eq!(
        e.p.refus(
            "versement_annuler",
            Some(&e.admin),
            json!({ "id": id2, "motif": "Chèque sans provision" })
        )
        .await
        .cle,
        "versement.caisse_requise"
    );
    e.p.ok(
        "caisse_ouvrir",
        Some(&e.admin),
        json!({ "fonds_initial": 20_000 }),
    )
    .await;
    assert_eq!(
        e.p.refus(
            "versement_annuler",
            Some(&e.admin),
            json!({ "id": id2, "motif": "Payé deux fois" })
        )
        .await
        .cle,
        "versement.especes_insuffisantes"
    );
    let binta = eleve(&e, "Binta").await;
    let ins_b = brouillon(&e, &binta, &e.septieme, "").await;
    e.p.ok(
        "inscription_valider",
        Some(&e.admin),
        json!({ "id": ins_b["id"] }),
    )
    .await;
    let mut tiers = demande_encaissement(
        &ins_b,
        "cle-annul-0003",
        30_000,
        "especes",
        "",
        &Value::Null,
    );
    tiers["payeur_nom"] = json!("Moussa Traoré");
    e.p.ok("encaisser", Some(&e.admin), tiers).await;
    let annulation =
        e.p.ok(
            "versement_annuler",
            Some(&e.admin),
            json!({ "id": id2, "motif": "Payé deux fois" }),
        )
        .await;
    let sortie = &annulation["regularisation"];
    assert_eq!(sortie["montant"], 50_000);
    assert_eq!(sortie["mode"], "especes");
    assert_eq!(sortie["versement_id"], id2);
    assert!(
        sortie["motif"]
            .as_str()
            .unwrap()
            .starts_with("Annulation du reçu REC-2026-000002")
    );

    // La sortie diminue le tiroir de qui annule ; la session close garde son versement.
    let ma_caisse =
        e.p.ok("caisse_session_courante", Some(&e.admin), Value::Null)
            .await;
    assert_eq!(ma_caisse["totaux"]["entrees_especes"], 30_000);
    assert_eq!(ma_caisse["totaux"]["sorties_especes"], 50_000);
    assert_eq!(ma_caisse["totaux"]["theorique"], 0);
    assert_eq!(
        ma_caisse["totaux"]["regularisations"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let sessions =
        e.p.ok("caisse_sessions_lister", Some(&e.admin), Value::Null)
            .await;
    let close = sessions
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["caissier"] == "Oumou Diarra")
        .unwrap();
    assert_eq!(close["totaux"]["entrees_especes"], 50_000);
    assert_eq!(close["totaux"]["theorique"], 60_000);
    assert_eq!(close["totaux"]["nombre_recus"], 1);
    let s = situation(&e, &awa).await;
    assert_eq!(s["facture"]["paye"], 0);

    // La direction est avertie jusqu'au visa de la session où passe la sortie.
    let avertissements = e.p.ok("avertissements", Some(&e.admin), Value::Null).await;
    assert!(a_l_avertissement(
        &avertissements,
        "avertissement.regularisations_a_viser"
    ));
    let avertissements =
        e.p.ok("avertissements", Some(&caissiere), Value::Null)
            .await;
    assert!(!a_l_avertissement(
        &avertissements,
        "avertissement.regularisations_a_viser"
    ));
    let etat =
        e.p.ok(
            "etat_caisse_produire",
            Some(&e.admin),
            json!({ "id": ma_caisse["session"]["id"] }),
        )
        .await;
    assert_eq!(etat["document"]["modele_id"], "etat_caisse");
    let fermee =
        e.p.ok(
            "caisse_cloturer",
            Some(&e.admin),
            json!({ "total_compte": 0, "motif_ecart": "" }),
        )
        .await;
    e.p.ok(
        "caisse_rapprocher",
        Some(&e.admin),
        json!({ "id": fermee["session"]["id"], "commentaire": "Remboursement remis au parent" }),
    )
    .await;
    let avertissements = e.p.ok("avertissements", Some(&e.admin), Value::Null).await;
    assert!(!a_l_avertissement(
        &avertissements,
        "avertissement.regularisations_a_viser"
    ));
}

#[tokio::test]
async fn les_impayes_se_classent_par_anciennete_et_le_journal_totalise_les_recus() {
    let dossier = tempfile::tempdir().unwrap();
    let e = ecole(dossier.path()).await;
    let mut inscriptions = Vec::new();
    for prenom in ["Awa", "Binta"] {
        let fiche = eleve(&e, prenom).await;
        let ins = brouillon(&e, &fiche, &e.septieme, "").await;
        e.p.ok(
            "inscription_valider",
            Some(&e.admin),
            json!({ "id": ins["id"] }),
        )
        .await;
        inscriptions.push(ins);
    }
    e.p.ok(
        "caisse_ouvrir",
        Some(&e.admin),
        json!({ "fonds_initial": 0 }),
    )
    .await;
    // Binta règle l'inscription et la 1re tranche ; Awa paie 5 000 F par Orange Money, versement
    // annulé ensuite.
    let mut binta = demande_encaissement(
        &inscriptions[1],
        "cle-journal-1",
        70_000,
        "especes",
        "",
        &Value::Null,
    );
    binta["payeur_nom"] = json!("Moussa Traoré");
    e.p.ok("encaisser", Some(&e.admin), binta).await;
    let mut awa = demande_encaissement(
        &inscriptions[0],
        "cle-journal-2",
        5_000,
        "orange_money",
        "OM-1",
        &Value::Null,
    );
    awa["payeur_nom"] = json!("Mariam Coulibaly");
    let awa = e.p.ok("encaisser", Some(&e.admin), awa).await;
    e.p.ok(
        "versement_annuler",
        Some(&e.admin),
        json!({ "id": awa["versement"]["id"], "motif": "Mauvais élève" }),
    )
    .await;

    // Le jour même : seule l'inscription d'Awa est échue, depuis 0 jour.
    let liste = e.p.ok("impayes_lister", Some(&e.admin), Value::Null).await;
    assert_eq!(liste["annee"], "2026-2027");
    assert_eq!(liste["lignes"].as_array().unwrap().len(), 1);
    let ligne = &liste["lignes"][0];
    assert_eq!(ligne["eleve"], "Awa Traoré");
    assert_eq!(ligne["classe"], "7e A");
    assert_eq!(ligne["impaye"]["echu"], 10_000);
    assert_eq!(ligne["impaye"]["total"], 160_000);
    assert_eq!(ligne["impaye"]["jours"], 0);
    assert_eq!(ligne["impaye"]["tranche"], "de_0_a_30");
    assert_eq!(ligne["payeurs"][0]["nom"], "Mariam Coulibaly");
    assert_eq!(
        ligne["payeurs"][0]["telephones"].as_array().unwrap().len(),
        1
    );

    // Le journal du jour : deux reçus, dont un annulé ; seul le valide compte.
    let jour = liste["date"].clone();
    let journal =
        e.p.ok(
            "encaissements_lister",
            Some(&e.admin),
            json!({ "du": jour, "au": jour, "caissier_id": null, "mode": null }),
        )
        .await;
    assert_eq!(journal["lignes"].as_array().unwrap().len(), 2);
    assert_eq!(journal["nombre"], 1);
    assert_eq!(journal["total"], 70_000);
    assert_eq!(journal["annules"], 1);
    assert_eq!(journal["montant_annule"], 5_000);
    assert_eq!(journal["par_mode"].as_array().unwrap().len(), 1);
    assert_eq!(journal["par_mode"][0]["mode"], "especes");
    assert_eq!(journal["par_jour"][0]["montant"], 70_000);
    assert_eq!(journal["caissiers"].as_array().unwrap().len(), 1);
    assert_eq!(journal["lignes"][0]["eleve"], "Binta Traoré");
    let filtre = e
        .p
        .ok(
            "encaissements_lister",
            Some(&e.admin),
            json!({ "du": jour, "au": jour, "caissier_id": journal["caissiers"][0]["id"], "mode": "orange_money" }),
        )
        .await;
    assert_eq!(filtre["lignes"].as_array().unwrap().len(), 1);
    assert_eq!(filtre["total"], 0);
    assert_eq!(
        e.p.refus(
            "encaissements_lister",
            Some(&e.admin),
            json!({ "du": "2026-10-05", "au": "2026-10-01", "caissier_id": null, "mode": null })
        )
        .await
        .cle,
        "journal.periode_invalide"
    );

    // Le 20 novembre : Awa doit l'inscription et la 1re tranche depuis 47 jours.
    e.p.horloge.avancer(4_031_069_750);
    let admin = e.p.connecter("admin", MOT_DE_PASSE).await;
    let liste = e.p.ok("impayes_lister", Some(&admin), Value::Null).await;
    assert_eq!(liste["lignes"].as_array().unwrap().len(), 1);
    assert_eq!(liste["lignes"][0]["impaye"]["echu"], 70_000);
    assert_eq!(liste["lignes"][0]["impaye"]["jours"], 47);
    assert_eq!(liste["lignes"][0]["impaye"]["tranche"], "de_31_a_60");

    // Le 20 janvier : Awa a plus de 90 jours de retard ; Binta doit la 2e tranche depuis 5 jours.
    e.p.horloge.avancer(9_301_469_750 - 4_031_069_750);
    let admin = e.p.connecter("admin", MOT_DE_PASSE).await;
    let liste = e.p.ok("impayes_lister", Some(&admin), Value::Null).await;
    let lignes = liste["lignes"].as_array().unwrap();
    assert_eq!(lignes.len(), 2);
    let tranche = |prenom: &str| {
        lignes
            .iter()
            .find(|l| l["eleve"].as_str().unwrap().starts_with(prenom))
            .map(|l| (l["impaye"]["tranche"].clone(), l["impaye"]["echu"].clone()))
            .unwrap()
    };
    assert_eq!(tranche("Awa"), (json!("plus_de_90"), json!(115_000)));
    assert_eq!(tranche("Binta"), (json!("de_0_a_30"), json!(45_000)));
    assert_eq!(liste["par_classe"][0]["eleves"], 2);
    assert_eq!(liste["par_classe"][0]["echu"], 160_000);
    assert_eq!(liste["echu"], 160_000);
    let par_tranche: Vec<(Value, Value)> = liste["par_tranche"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| (t["tranche"].clone(), t["eleves"].clone()))
        .collect();
    assert_eq!(
        par_tranche,
        vec![
            (json!("de_0_a_30"), json!(1)),
            (json!("de_31_a_60"), json!(0)),
            (json!("de_61_a_90"), json!(0)),
            (json!("plus_de_90"), json!(1)),
        ]
    );
}

/// SPEC_PHASE_8 §7 : le journal de trésorerie range un versement par type de frais d'après ses
/// imputations, les dépenses par catégorie, et donne le solde de chaque compte ; le budget compare
/// le prévu au réalisé et avertit d'un dépassement sans rien bloquer.
#[tokio::test]
async fn le_journal_range_les_versements_par_type_de_frais_et_le_budget_avertit() {
    let dossier = tempfile::tempdir().unwrap();
    let e = ecole(dossier.path()).await;
    let awa = eleve(&e, "Awa").await;
    let ins = brouillon(&e, &awa, &e.septieme, "").await;
    e.p.ok(
        "inscription_valider",
        Some(&e.admin),
        json!({ "id": ins["id"] }),
    )
    .await;
    e.p.ok(
        "caisse_ouvrir",
        Some(&e.admin),
        json!({ "fonds_initial": 10_000 }),
    )
    .await;
    let mere =
        e.p.ok(
            "encaissement_preparer",
            Some(&e.admin),
            json!({ "inscription_id": ins["id"], "montant": 40_000 }),
        )
        .await["payeurs"][0]["responsable_id"]
            .clone();
    e.p.ok(
        "encaisser",
        Some(&e.admin),
        demande_encaissement(&ins, "cle-journal-1", 40_000, "especes", "", &mere),
    )
    .await;
    let d =
        e.p.ok(
            "depenses_lire",
            Some(&e.admin),
            json!({ "mois": "2026-10" }),
        )
        .await;
    let categorie = |libelle: &str| {
        d["categories"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["libelle"] == libelle)
            .unwrap()["id"]
            .clone()
    };
    let caisse = d["comptes"][0]["id"].clone();
    e.p.ok(
        "depense_saisir",
        Some(&e.admin),
        json!({ "date": "", "categorie_id": categorie("Fournitures"), "libelle": "Craies",
            "beneficiaire": "Librairie Kanté", "montant": 7_500,
            "paiement": { "compte_id": caisse, "mode": "especes", "reference": "" } }),
    )
    .await;

    let j =
        e.p.ok(
            "journal_tresorerie_lire",
            Some(&e.admin),
            json!({ "mois": "2026-10" }),
        )
        .await;
    assert_eq!(
        j["recettes"],
        json!([
            { "categorie": "Frais d'inscription", "montant": 10_000 },
            { "categorie": "Scolarité", "montant": 30_000 },
        ])
    );
    assert_eq!(
        j["depenses"],
        json!([{ "categorie": "Fournitures", "montant": 7_500 }])
    );
    assert_eq!(
        j["soldes"],
        json!([{ "compte": "Caisse", "debut": 0, "fin": 32_500 }])
    );

    // Le budget de l'année : 5 000 F prévus pour les fournitures, déjà dépassés.
    let b =
        e.p.ok("budget_lire", Some(&e.admin), json!({ "annee_id": null }))
            .await;
    assert_eq!(b["annee"], "2026-2027");
    assert_eq!(b["depassements"], 0);
    let b =
        e.p.ok(
            "budget_prevoir",
            Some(&e.admin),
            json!({ "annee_id": b["annee_id"], "categorie_id": categorie("Fournitures"),
                "montant": 5_000 }),
        )
        .await;
    let fournitures = b["depenses"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["categorie"]["libelle"] == "Fournitures")
        .unwrap();
    assert_eq!(
        (
            fournitures["realise"].clone(),
            fournitures["ecart"].clone(),
            fournitures["depasse"].clone()
        ),
        (json!(7_500), json!(-2_500), json!(true))
    );
    assert_eq!(b["depassements"], 1);
}

/// SPEC_PHASE_8 §9 : la cantine que l'école active montre ses abonnés d'après leurs factures ;
/// le repas se pointe quand l'école les compte, et le bilan du mois les additionne.
#[tokio::test]
async fn la_cantine_activee_liste_ses_abonnes_et_pointe_les_repas() {
    let dossier = tempfile::tempdir().unwrap();
    let e = ecole(dossier.path()).await;
    for (prenoms, cantine) in [("Awa", true), ("Bina", false)] {
        let fiche = eleve(&e, prenoms).await;
        let ins = brouillon(&e, &fiche, &e.septieme, "").await;
        let choix = if cantine {
            json!([e.cantine])
        } else {
            json!([])
        };
        e.p.ok(
            "inscription_valider",
            Some(&e.admin),
            json!({ "id": ins["id"], "choix": { "frais_optionnels": choix } }),
        )
        .await;
    }
    // Inactive, la cantine ne montre rien.
    let c =
        e.p.ok(
            "cantine_lire",
            Some(&e.admin),
            json!({ "date": "2026-10-05" }),
        )
        .await;
    assert_eq!(c["active"], false);
    assert!(c["abonnes"].as_array().unwrap().is_empty());
    let regler = |repas: bool| json!({ "seuil_accord": 50_000, "cantine_active": true, "repas_comptes": repas });
    e.p.ok(
        "tresorerie_reglages_enregistrer",
        Some(&e.admin),
        regler(false),
    )
    .await;
    let c =
        e.p.ok(
            "cantine_lire",
            Some(&e.admin),
            json!({ "date": "2026-10-05" }),
        )
        .await;
    let abonnes = c["abonnes"].as_array().unwrap();
    assert_eq!(abonnes.len(), 1);
    assert_eq!(abonnes[0]["eleve"], "Awa Traoré");
    assert_eq!(abonnes[0]["classe"], "7e A");
    let awa = abonnes[0]["inscription_id"].clone();
    let repas = json!({ "date": "2026-10-05", "inscription_id": awa, "pris": true });
    assert_eq!(
        e.p.refus("repas_pointer", Some(&e.admin), repas.clone())
            .await
            .cle,
        "cantine.repas_non_comptes"
    );
    e.p.ok(
        "tresorerie_reglages_enregistrer",
        Some(&e.admin),
        regler(true),
    )
    .await;
    let c = e.p.ok("repas_pointer", Some(&e.admin), repas).await;
    assert_eq!(c["abonnes"][0]["repas_pris"], true);
    assert_eq!(c["repas_du_jour"], 1);
    assert_eq!(c["bilan"]["repas"], 1);
    let liste =
        e.p.ok(
            "liste_cantine_produire",
            Some(&e.admin),
            json!({ "date": "2026-10-05" }),
        )
        .await;
    assert!(liste["pdf"].as_str().unwrap().len() > 1_000);
    // Relecture à l'œil : `GESCO_ECRIRE_CANTINE=chemin.pdf` dépose la liste du jour.
    if let Some(chemin) = std::env::var_os("GESCO_ECRIRE_CANTINE") {
        use base64::Engine;
        let pdf = base64::engine::general_purpose::STANDARD
            .decode(liste["pdf"].as_str().unwrap())
            .unwrap();
        std::fs::write(chemin, pdf).unwrap();
    }
}
