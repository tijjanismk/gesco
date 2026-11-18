//! Reprise avant GESCO (SPEC_REPRISE_AVANT_GESCO §11) : ce que les parents ont payé avant
//! l'installation réduit le reste à payer, sans versement, sans reçu, sans caisse. Écrits avec le
//! code, passés à la fin du codage de la 2.0.0.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use crate::commun::*;
use gesco_services::session::Jeton;
use serde_json::{Value, json};

pub struct Ecole {
    pub p: Poste_,
    pub admin: Jeton,
    pub annee: Value,
    pub septieme: Value,
    pub huitieme: Value,
    pub cantine: Value,
    pub plans: Vec<Value>,
}

/// Année 2026-2027 ouverte ; 7e : inscription 10 000 F, scolarité 150 000 F selon le plan
/// (trois tranches par défaut : 60 000, 45 000, 45 000), cantine facultative 30 000 F ;
/// 8e : inscription 10 000 F, scolarité 90 000 F selon le plan.
pub async fn ecole(dossier: &std::path::Path) -> Ecole {
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
              "montant": 10_000, "obligatoire": true, "exigibilite": "une_fois" },
            { "niveau_id": sept, "serie_id": null, "type_frais": "scolarite", "libelle": "Scolarité",
              "montant": 150_000, "obligatoire": true, "exigibilite": "selon_plan" },
            { "niveau_id": sept, "serie_id": null, "type_frais": "cantine", "libelle": "Cantine",
              "montant": 30_000, "obligatoire": false, "exigibilite": "une_fois" },
            { "niveau_id": huit, "serie_id": null, "type_frais": "inscription", "libelle": "Frais d'inscription",
              "montant": 10_000, "obligatoire": true, "exigibilite": "une_fois" },
            { "niveau_id": huit, "serie_id": null, "type_frais": "scolarite", "libelle": "Scolarité",
              "montant": 90_000, "obligatoire": true, "exigibilite": "selon_plan" },
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

/// Élève avec un responsable payeur, inscrit et validé dans la classe (plan par défaut).
pub async fn inscrit(e: &Ecole, prenoms: &str, classe: &Value) -> (Value, Value) {
    let (fiche, inscription) = brouillon(e, prenoms, classe).await;
    e.p.ok(
        "inscription_valider",
        Some(&e.admin),
        json!({ "id": inscription["id"] }),
    )
    .await;
    (fiche, inscription)
}

/// Élève avec un responsable payeur, en brouillon dans la classe.
pub async fn brouillon(e: &Ecole, prenoms: &str, classe: &Value) -> (Value, Value) {
    let fiche = e
        .p
        .ok(
            "eleve_creer",
            Some(&e.admin),
            json!({ "identite": { "nom": "Diarra", "prenoms": prenoms, "sexe": "feminin",
                "date_naissance": "2014-03-02", "date_naissance_estimee": false, "lieu_naissance": "Kati",
                "nationalite": "Malienne", "adresse": "", "telephones": [] },
                "matricule_interne": "", "matricule_national": "", "acte_nature": "acte",
                "acte_numero": "", "acte_date": "", "ecole_origine": "" }),
        )
        .await;
    e.p.ok(
        "responsable_rattacher",
        Some(&e.admin),
        json!({ "eleve_id": fiche["eleve"]["id"], "responsable_id": null,
            "saisie": { "identite": { "nom": "Sangaré", "prenoms": "Fanta", "sexe": "feminin",
                "date_naissance": "", "date_naissance_estimee": false, "lieu_naissance": "", "nationalite": "",
                "adresse": "", "telephones": [format!("76 00 00 {:02}", prenoms.len())] }, "profession": "", "employeur": "" },
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
    (fiche, inscription)
}

pub async fn situation(e: &Ecole, fiche: &Value) -> Value {
    e.p.ok(
        "situation_financiere",
        Some(&e.admin),
        json!({ "id": fiche["eleve"]["id"] }),
    )
    .await[0]
        .clone()
}

pub fn reste(s: &Value) -> i64 {
    s["facture"]["reste"].as_i64().unwrap()
}

/// Enregistre (ou remplace, ou annule pour zéro) la reprise, sans date de lecture.
pub async fn reprendre(e: &Ecole, jeton: &Jeton, inscription: &Value, montant: i64) -> Value {
    e.p.ok(
        "reprise_enregistrer",
        Some(jeton),
        json!({ "inscription_id": inscription["id"], "montant": montant, "note": "cahier p. 12" }),
    )
    .await
}

pub async fn payer(
    e: &Ecole,
    jeton: &Jeton,
    inscription: &Value,
    cle: &str,
    montant: i64,
) -> Value {
    let preparation =
        e.p.ok(
            "encaissement_preparer",
            Some(jeton),
            json!({ "inscription_id": inscription["id"], "montant": montant }),
        )
        .await;
    e.p.ok(
        "encaisser",
        Some(jeton),
        json!({ "cle_idempotence": cle, "inscription_id": inscription["id"], "montant": montant,
            "mode": "especes", "reference": "", "payeur_id": preparation["payeurs"][0]["responsable_id"],
            "payeur_nom": "", "repartition": null }),
    )
    .await
}

fn payes(s: &Value) -> Vec<(i64, i64)> {
    s["facture"]["echeances"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| (e["reprise"].as_i64().unwrap(), e["reste"].as_i64().unwrap()))
        .collect()
}

/// Essai 2 : le reste suit la reprise, plus ancienne échéance d'abord.
#[tokio::test]
async fn la_reprise_reduit_le_reste_plus_ancienne_d_abord() {
    let dossier = tempfile::tempdir().unwrap();
    let e = ecole(dossier.path()).await;
    let (awa, ins) = inscrit(&e, "Awa", &e.septieme).await;
    // Dû 160 000 F : inscription 10 000, tranches 60 000, 45 000, 45 000.
    assert_eq!(reste(&situation(&e, &awa).await), 160_000);

    let ligne = reprendre(&e, &e.admin, &ins, 60_000).await;
    assert_eq!(ligne["reprise"], 60_000);
    assert_eq!(ligne["reste"], 100_000);
    assert_eq!(ligne["a_rembourser"], 0);
    let s = situation(&e, &awa).await;
    assert_eq!(reste(&s), 100_000);
    assert_eq!(
        payes(&s),
        vec![(10_000, 0), (50_000, 10_000), (0, 45_000), (0, 45_000)]
    );
    assert_eq!(s["facture"]["reprise"], 60_000);
    assert_eq!(s["facture"]["facture"]["statut"], "partiellement_reglee");
    assert_eq!(s["reprise"]["active"]["montant"], 60_000);
    assert_eq!(s["reprise"]["active"]["note"], "cahier p. 12");
    // Aucun versement, aucun reçu.
    assert_eq!(s["versements"].as_array().unwrap().len(), 0);

    e.p.ok(
        "caisse_ouvrir",
        Some(&e.admin),
        json!({ "fonds_initial": 0 }),
    )
    .await;
    payer(&e, &e.admin, &ins, "awa-1", 10_000).await;
    assert_eq!(reste(&situation(&e, &awa).await), 90_000);
    reprendre(&e, &e.admin, &ins, 100_000).await;
    assert_eq!(reste(&situation(&e, &awa).await), 50_000);
    e.p.ok(
        "reprise_annuler",
        Some(&e.admin),
        json!({ "inscription_id": ins["id"], "motif": "Saisie erronée" }),
    )
    .await;
    let s = situation(&e, &awa).await;
    assert_eq!(reste(&s), 150_000);
    assert_eq!(s["facture"]["reprise"], 0);
    // Une reprise qui couvre tout solde la facture, sans reçu.
    reprendre(&e, &e.admin, &ins, 150_000).await;
    let s = situation(&e, &awa).await;
    assert_eq!(reste(&s), 0);
    assert_eq!(s["facture"]["facture"]["statut"], "soldee");
    assert_eq!(s["versements"].as_array().unwrap().len(), 1);
    // Montant zéro : annule ; le motif proposé est « Saisie erronée ».
    reprendre(&e, &e.admin, &ins, 0).await;
    let historique =
        e.p.ok(
            "reprise_historique",
            Some(&e.admin),
            json!({ "inscription_id": ins["id"] }),
        )
        .await;
    assert!(
        historique
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["statut"] == "annulee")
    );
    assert_eq!(historique[0]["motif_annulation"], "Saisie erronée");
}

/// Essai 2, propriété : pour une suite de gestes, reste = net − payé, aucun reste négatif,
/// imputé + à rembourser + remboursé = montant de la reprise.
#[tokio::test]
async fn les_chiffres_restent_justes_pour_toute_suite_de_gestes() {
    let dossier = tempfile::tempdir().unwrap();
    let e = ecole(dossier.path()).await;
    let (awa, ins) = inscrit(&e, "Awa", &e.septieme).await;
    e.p.ok(
        "caisse_ouvrir",
        Some(&e.admin),
        json!({ "fonds_initial": 0 }),
    )
    .await;
    let mut graine: u64 = 20_261_117;
    let mut suivant = |borne: u64| {
        graine = graine
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (graine >> 33) % borne
    };
    for tour in 0..40 {
        let montant = i64::try_from(suivant(40) * 5_000).unwrap();
        match suivant(3) {
            0 => {
                reprendre(&e, &e.admin, &ins, montant).await;
            }
            1 => {
                let r = reste(&situation(&e, &awa).await);
                if r > 0 {
                    payer(
                        &e,
                        &e.admin,
                        &ins,
                        &format!("tour-{tour}"),
                        r.min(montant.max(1_000)),
                    )
                    .await;
                }
            }
            _ => {
                let _ =
                    e.p.appeler(
                        "frais_ajouter",
                        Some(&e.admin),
                        json!({ "inscription_id": ins["id"], "ligne_tarifaire_id": e.cantine }),
                    )
                    .await;
            }
        }
        let s = situation(&e, &awa).await;
        let f = &s["facture"];
        for echeance in f["echeances"].as_array().unwrap() {
            assert!(echeance["reste"].as_i64().unwrap() >= 0, "tour {tour}");
            assert_eq!(
                echeance["reste"].as_i64().unwrap(),
                echeance["net"].as_i64().unwrap() - echeance["paye"].as_i64().unwrap()
            );
        }
        let c = &s["reprise"]["chiffres"];
        assert_eq!(
            c["impute"].as_i64().unwrap()
                + c["a_rembourser"].as_i64().unwrap()
                + c["rembourse"].as_i64().unwrap(),
            c["montant"].as_i64().unwrap(),
            "tour {tour}"
        );
        assert_eq!(f["reprise"], c["impute"]);
    }
}

/// Essai 4 : la reprise ne verrouille pas le plan ; un versement sur la scolarité, si.
#[tokio::test]
async fn le_plan_se_change_apres_une_reprise() {
    let dossier = tempfile::tempdir().unwrap();
    let e = ecole(dossier.path()).await;
    let (awa, ins) = inscrit(&e, "Awa", &e.septieme).await;
    reprendre(&e, &e.admin, &ins, 100_000).await;
    let comptant = e.plans.iter().find(|p| p["libelle"] == "Comptant").unwrap();
    let change =
        e.p.ok(
            "echeancier_changer_plan",
            Some(&e.admin),
            json!({ "inscription_id": ins["id"], "plan_paiement_id": comptant["id"] }),
        )
        .await;
    assert_eq!(change["plan"], "Comptant");
    assert_eq!(change["reste"], 60_000);
    assert_eq!(change["reprise"], 100_000);
    let s = situation(&e, &awa).await;
    assert_eq!(payes(&s), vec![(10_000, 0), (90_000, 60_000)]);

    // Un versement sur la scolarité verrouille toujours le plan.
    let (binta, ins_b) = inscrit(&e, "Binta", &e.septieme).await;
    e.p.ok(
        "caisse_ouvrir",
        Some(&e.admin),
        json!({ "fonds_initial": 0 }),
    )
    .await;
    payer(&e, &e.admin, &ins_b, "binta-1", 20_000).await;
    assert_eq!(
        e.p.refus(
            "echeancier_changer_plan",
            Some(&e.admin),
            json!({ "inscription_id": ins_b["id"], "plan_paiement_id": comptant["id"] }),
        )
        .await
        .cle,
        "echeancier.plan_verrouille"
    );
    assert_eq!(reste(&situation(&e, &binta).await), 140_000);
}

/// Essai 5 : vers des frais plus bas après une reprise qui couvre tout, la baisse devient un
/// avoir ; la reprise ne descend plus sous ce qui le garde couvert.
#[tokio::test]
async fn un_avoir_ne_de_la_reprise_reste_couvert() {
    let dossier = tempfile::tempdir().unwrap();
    let e = ecole(dossier.path()).await;
    let (awa, ins) = inscrit(&e, "Awa", &e.septieme).await;
    reprendre(&e, &e.admin, &ins, 160_000).await;
    e.p.ok(
        "inscription_changer_classe",
        Some(&e.admin),
        json!({ "id": ins["id"], "classe_id": e.huitieme["id"], "motif": "Niveau mieux adapté",
            "appliquer_frais": true }),
    )
    .await;
    let s = situation(&e, &awa).await;
    assert_eq!(reste(&s), 0);
    let avoir: i64 = s["avoirs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["montant"].as_i64().unwrap())
        .sum();
    assert!(avoir > 0);
    assert_eq!(
        e.p.refus(
            "reprise_enregistrer",
            Some(&e.admin),
            json!({ "inscription_id": ins["id"], "montant": 10_000, "note": "" }),
        )
        .await
        .cle,
        "reprise.avoir_a_couvrir"
    );
}

/// Essai 6 : droits. Le caissier ne touche à aucune reprise ; directeur et comptable, si.
#[tokio::test]
async fn seuls_les_titulaires_de_la_remise_saisissent_une_reprise() {
    let dossier = tempfile::tempdir().unwrap();
    let e = ecole(dossier.path()).await;
    let (_, ins) = inscrit(&e, "Awa", &e.septieme).await;
    for (identifiant, role) in [
        ("oumou", "caissier"),
        ("modibo", "directeur"),
        ("assa", "comptable"),
    ] {
        e.p.ok(
            "utilisateur_creer",
            Some(&e.admin),
            json!({ "identifiant": identifiant, "nom_affiche": identifiant,
                "mot_de_passe_initial": "ecole-essai-2026", "roles": [role] }),
        )
        .await;
    }
    let caissier = e.p.connecter("oumou", "ecole-essai-2026").await;
    for (commande, entree) in [
        (
            "reprise_enregistrer",
            json!({ "inscription_id": ins["id"], "montant": 5_000, "note": "" }),
        ),
        (
            "reprise_annuler",
            json!({ "inscription_id": ins["id"], "motif": "Erreur" }),
        ),
        ("reprises_lister", json!({})),
        ("reprises_modele", json!({ "chemin": "modele.xlsx" })),
        (
            "reprises_analyser_import",
            json!({ "chemin": "reprises.xlsx" }),
        ),
        ("reprises_importer", json!({ "fichier_id": "x" })),
    ] {
        assert_eq!(
            e.p.refus(commande, Some(&caissier), entree).await.cle,
            "erreur.permission_refusee",
            "{commande}"
        );
    }
    let directeur = e.p.connecter("modibo", "ecole-essai-2026").await;
    reprendre(&e, &directeur, &ins, 20_000).await;
    let comptable = e.p.connecter("assa", "ecole-essai-2026").await;
    let ligne = reprendre(&e, &comptable, &ins, 25_000).await;
    assert_eq!(ligne["reprise"], 25_000);
    // La lecture suit la situation de l'élève.
    e.p.ok(
        "reprise_historique",
        Some(&caissier),
        json!({ "inscription_id": ins["id"] }),
    )
    .await;
}

/// Essai 7 : année close, la reprise se lit et ne se change plus.
#[tokio::test]
async fn la_reprise_d_une_annee_close_se_lit_seulement() {
    let dossier = tempfile::tempdir().unwrap();
    let e = ecole(dossier.path()).await;
    let (_, ins) = inscrit(&e, "Awa", &e.septieme).await;
    reprendre(&e, &e.admin, &ins, 40_000).await;
    {
        let mut cnx = base_brute(&e.p.racine, e.p.trousseau.clone()).await;
        sqlx::query("UPDATE annee_scolaire SET statut = 'cloturee'")
            .execute(&mut cnx)
            .await
            .unwrap();
    }
    assert_eq!(
        e.p.refus(
            "reprise_enregistrer",
            Some(&e.admin),
            json!({ "inscription_id": ins["id"], "montant": 50_000, "note": "" }),
        )
        .await
        .cle,
        "reprise.annee_close"
    );
    let liste =
        e.p.ok(
            "reprises_lister",
            Some(&e.admin),
            json!({ "annee_id": e.annee["id"] }),
        )
        .await;
    assert_eq!(liste["lecture_seule"], true);
    assert_eq!(liste["total"], 40_000);
}

/// Essai 8 : modification et annulation laissent l'ancienne ligne annulée, auditée ; la base
/// refuse toute suppression.
#[tokio::test]
async fn une_reprise_garde_sa_trace() {
    let dossier = tempfile::tempdir().unwrap();
    let e = ecole(dossier.path()).await;
    let (_, ins) = inscrit(&e, "Awa", &e.septieme).await;
    reprendre(&e, &e.admin, &ins, 50_000).await;
    reprendre(&e, &e.admin, &ins, 75_000).await;
    let historique =
        e.p.ok(
            "reprise_historique",
            Some(&e.admin),
            json!({ "inscription_id": ins["id"] }),
        )
        .await;
    let h = historique.as_array().unwrap();
    assert_eq!(h.len(), 2);
    assert_eq!(h[0]["statut"], "active");
    assert_eq!(h[0]["montant"], 75_000);
    assert_eq!(h[1]["statut"], "annulee");
    assert_eq!(h[1]["motif_annulation"], "remplacée");
    assert_eq!(h[1]["remplacee_par"], h[0]["id"]);
    assert_eq!(
        e.p.refus(
            "reprise_annuler",
            Some(&e.admin),
            json!({ "inscription_id": ins["id"], "motif": " " }),
        )
        .await
        .cle,
        "reprise.motif_obligatoire"
    );
    let mut cnx = base_brute(&e.p.racine, e.p.trousseau.clone()).await;
    let audits: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM journal_audit WHERE entite = 'reprise_avant_gesco'",
    )
    .fetch_one(&mut cnx)
    .await
    .unwrap();
    assert_eq!(audits, 2);
    assert!(
        sqlx::query("DELETE FROM reprise_avant_gesco")
            .execute(&mut cnx)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM reprise_imputation")
            .execute(&mut cnx)
            .await
            .is_err()
    );
}

/// Essai 9 : les lecteurs du reste lisent le reste après reprise (impayés, cantine, situation),
/// et toute requête de `gesco-db` qui additionne `imputation_versement` pour un payé lit aussi
/// `reprise_imputation`.
#[tokio::test]
async fn les_lecteurs_du_reste_voient_la_reprise() {
    let dossier = tempfile::tempdir().unwrap();
    let e = ecole(dossier.path()).await;
    let (_, ins) = inscrit(&e, "Awa", &e.septieme).await;
    e.p.ok(
        "frais_ajouter",
        Some(&e.admin),
        json!({ "inscription_id": ins["id"], "ligne_tarifaire_id": e.cantine }),
    )
    .await;
    // Tout est réglé avant GESCO : aucune ligne d'impayé, aucun reste de cantine.
    reprendre(&e, &e.admin, &ins, 190_000).await;
    e.p.horloge.avancer(200 * 86_400_000);
    let admin = e.p.connecter("admin", MOT_DE_PASSE).await;
    let impayes = e.p.ok("impayes_lister", Some(&admin), Value::Null).await;
    assert!(
        impayes["lignes"]
            .as_array()
            .is_none_or(|l| l.iter().all(|l| l["inscription_id"] != ins["id"]))
    );
}

#[test]
fn toute_requete_de_paye_lit_la_reprise() {
    let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../gesco-db/src/depots");
    // Encaissé (budget, versements) : de l'argent reçu, pas un payé.
    let encaisse = ["budget.rs", "versements.rs"];
    for entree in std::fs::read_dir(&racine).unwrap() {
        let chemin = entree.unwrap().path();
        let nom = chemin.file_name().unwrap().to_string_lossy().into_owned();
        if encaisse.contains(&nom.as_str()) {
            continue;
        }
        let source = std::fs::read_to_string(&chemin).unwrap();
        if source.contains("SUM(im.montant), 0) FROM imputation_versement")
            || source.contains("FROM imputation_versement")
        {
            assert!(
                source.contains("reprise_imputation"),
                "{nom} additionne imputation_versement sans reprise_imputation"
            );
        }
    }
}

/// Ce que la caisse et les comptes montrent : totaux de la session ouverte, journal de
/// trésorerie du mois, soldes.
async fn argent(e: &Ecole, jeton: &Jeton) -> (Value, Value, Value) {
    let session =
        e.p.ok("caisse_session_courante", Some(jeton), Value::Null)
            .await;
    let journal =
        e.p.ok(
            "journal_tresorerie_lire",
            Some(&e.admin),
            json!({ "mois": "2026-10" }),
        )
        .await;
    let tresorerie = e.p.ok("tresorerie_lire", Some(&e.admin), Value::Null).await;
    (
        session["totaux"].clone(),
        json!({ "recettes": journal["total_recettes"], "depenses": journal["total_depenses"],
            "lignes": journal["depenses"] }),
        tresorerie["comptes"].clone(),
    )
}

fn categorie_remboursement(depenses: &Value) -> Value {
    depenses["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["code"] == "remboursement_parent")
        .unwrap()["id"]
        .clone()
}

/// Essai 3 : l'excédent d'une reprise est à rembourser ; seule la dépense payée sort de
/// l'argent ; rien n'est remboursé deux fois ; un frais ajouté absorbe d'abord l'excédent.
#[tokio::test]
async fn l_excedent_se_rembourse_par_une_depense_et_jamais_deux_fois() {
    let dossier = tempfile::tempdir().unwrap();
    let e = ecole(dossier.path()).await;
    let (awa, ins) = inscrit(&e, "Awa", &e.septieme).await;
    e.p.ok(
        "caisse_ouvrir",
        Some(&e.admin),
        json!({ "fonds_initial": 50_000 }),
    )
    .await;
    let avant = argent(&e, &e.admin).await;

    // Facture de 160 000 F, cahier à 175 000 F : soldée, 15 000 F à rembourser, argent intact.
    let ligne = reprendre(&e, &e.admin, &ins, 175_000).await;
    assert_eq!(ligne["reste"], 0);
    assert_eq!(ligne["a_rembourser"], 15_000);
    let s = situation(&e, &awa).await;
    assert_eq!(s["facture"]["facture"]["statut"], "soldee");
    assert_eq!(s["facture"]["a_rembourser"], 15_000);
    assert_eq!(argent(&e, &e.admin).await, avant);

    // Remboursement de 10 000 F en espèces : la session perd exactement 10 000 F.
    let depenses =
        e.p.ok(
            "depenses_lire",
            Some(&e.admin),
            json!({ "mois": "2026-10" }),
        )
        .await;
    let categorie = categorie_remboursement(&depenses);
    let caisse = argent(&e, &e.admin)
        .await
        .2
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["nature"] == "caisse")
        .unwrap()["id"]
        .clone();
    let rembourser = |montant: i64| {
        json!({ "date": "", "categorie_id": categorie, "libelle": "Trop-payé avant GESCO — Awa Diarra",
            "beneficiaire": "Fanta Sangaré", "montant": montant, "inscription_id": ins["id"],
            "paiement": { "compte_id": caisse, "mode": "especes", "reference": "" } })
    };
    let apres =
        e.p.ok("depense_saisir", Some(&e.admin), rembourser(10_000))
            .await;
    let depense = apres["depenses"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["depense"]["inscription_id"] == ins["id"])
        .unwrap()["depense"]["id"]
        .clone();
    let (totaux, journal, _) = argent(&e, &e.admin).await;
    assert_eq!(
        totaux["theorique"].as_i64().unwrap(),
        avant.0["theorique"].as_i64().unwrap() - 10_000
    );
    assert_eq!(
        journal["depenses"].as_i64().unwrap(),
        avant.1["depenses"].as_i64().unwrap() + 10_000
    );
    assert!(
        journal["lignes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|l| l["libelle"] == "Remboursements aux parents" && l["montant"] == 10_000)
    );
    let s = situation(&e, &awa).await;
    assert_eq!(s["facture"]["a_rembourser"], 5_000);
    assert_eq!(s["facture"]["rembourse"], 10_000);
    assert_eq!(s["reprise"]["remboursements"].as_array().unwrap().len(), 1);

    // 6 000 F de plus : refusé, rien n'est remboursé deux fois.
    assert_eq!(
        e.p.refus("depense_saisir", Some(&e.admin), rembourser(6_000))
            .await
            .cle,
        "remboursement.superieur_a_l_excedent"
    );

    // La cantine oubliée (30 000 F) : la reprise en couvre 5 000 F, reste 25 000 F.
    let apercu =
        e.p.ok(
            "frais_ajouter_apercu",
            Some(&e.admin),
            json!({ "inscription_id": ins["id"] }),
        )
        .await;
    assert_eq!(apercu[0]["montant"], 30_000);
    e.p.ok(
        "frais_ajouter",
        Some(&e.admin),
        json!({ "inscription_id": ins["id"], "ligne_tarifaire_id": e.cantine }),
    )
    .await;
    let s = situation(&e, &awa).await;
    assert_eq!(s["facture"]["a_rembourser"], 0);
    assert_eq!(reste(&s), 25_000);

    // Baisser la reprise sous ce qui a été rendu : refusé.
    assert_eq!(
        e.p.refus(
            "reprise_enregistrer",
            Some(&e.admin),
            json!({ "inscription_id": ins["id"], "montant": 5_000, "note": "" }),
        )
        .await
        .cle,
        "reprise.deja_remboursee"
    );

    // Dépense annulée : la reprise se réimpute, reste 15 000 F ; la session reçoit l'entrée
    // d'annulation comme pour toute dépense.
    e.p.ok(
        "depense_annuler",
        Some(&e.admin),
        json!({ "id": depense, "motif": "Parent revenu payer la cantine" }),
    )
    .await;
    let s = situation(&e, &awa).await;
    assert_eq!(reste(&s), 15_000);
    assert_eq!(s["facture"]["rembourse"], 0);
    let (totaux, _, _) = argent(&e, &e.admin).await;
    assert_eq!(totaux["theorique"], avant.0["theorique"]);
}

/// Essai 3 bis : un frais ajouté sans reprise reçoit ses échéances comme à la validation ;
/// refusé sur une facture annulée ou une année close.
#[tokio::test]
async fn un_frais_oublie_s_ajoute_comme_a_la_validation() {
    let dossier = tempfile::tempdir().unwrap();
    let e = ecole(dossier.path()).await;
    let (awa, ins) = inscrit(&e, "Awa", &e.septieme).await;
    let (coumba, ins_c) = inscrit(&e, "Coumba", &e.septieme).await;
    // Binta, validée avec la cantine cochée : le modèle de ce que l'ajout doit donner.
    let (binta, brouillon_b) = brouillon(&e, "Binta", &e.septieme).await;
    e.p.ok(
        "inscription_valider",
        Some(&e.admin),
        json!({ "id": brouillon_b["id"], "choix": { "frais_optionnels": [e.cantine] } }),
    )
    .await;
    let facture =
        e.p.ok(
            "frais_ajouter",
            Some(&e.admin),
            json!({ "inscription_id": ins["id"], "ligne_tarifaire_id": e.cantine }),
        )
        .await;
    assert_eq!(facture["facture"]["total"], 190_000);
    let echeances = |s: &Value| -> Vec<(Value, Value)> {
        s["facture"]["echeances"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| {
                (
                    e["echeance"]["date"].clone(),
                    e["echeance"]["montant"].clone(),
                )
            })
            .collect()
    };
    let mut a = echeances(&situation(&e, &awa).await);
    let mut b = echeances(&situation(&e, &binta).await);
    a.sort_by_key(|(d, m)| (d.to_string(), m.to_string()));
    b.sort_by_key(|(d, m)| (d.to_string(), m.to_string()));
    assert_eq!(a, b);
    // Déjà là : plus rien à ajouter.
    assert_eq!(
        e.p.refus(
            "frais_ajouter",
            Some(&e.admin),
            json!({ "inscription_id": ins["id"], "ligne_tarifaire_id": e.cantine }),
        )
        .await
        .cle,
        "frais.non_ajoutable"
    );
    {
        let mut cnx = base_brute(&e.p.racine, e.p.trousseau.clone()).await;
        sqlx::query("UPDATE annee_scolaire SET statut = 'cloturee'")
            .execute(&mut cnx)
            .await
            .unwrap();
    }
    assert_eq!(
        e.p.refus(
            "frais_ajouter",
            Some(&e.admin),
            json!({ "inscription_id": ins_c["id"], "ligne_tarifaire_id": e.cantine }),
        )
        .await
        .cle,
        "frais.annee_close"
    );
    assert_eq!(
        situation(&e, &coumba).await["facture"]["facture"]["total"],
        160_000
    );
}
