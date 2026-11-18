//! Service de la prise en charge par l'État (SPEC_PRISE_EN_CHARGE_ETAT §3, §5.3, §6.1, §8, §12) :
//! cocher une sélection en une transaction, montant proposé, facture née d'une inscription
//! cochée, année close (essai 7, partie case), droits (essai 9), trace (essai 10, partie audit),
//! motif de l'abandon recopié, changement de classe et tableau des montants. Les commandes
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
use gesco_services::frontiere::ErreurCommande;
use gesco_services::session::Jeton;
use serde_json::{Value, json};

struct Lycee {
    p: Poste_,
    admin: Jeton,
    annee: Value,
    dixieme: Value,
    onzieme_s: Value,
    septieme: Value,
    cantine: Value,
    niveau_10: Value,
    niveau_11: Value,
    serie_s: Value,
}

/// Année 2026-2027 ouverte ; 10e A : inscription 15 000 F, scolarité 120 000 F selon le plan,
/// cantine facultative 30 000 F ; 11e S : inscription 15 000 F, scolarité 110 000 F ; 7e A
/// (second cycle) : inscription 15 000 F, scolarité 100 000 F. Une caisse ouverte.
async fn lycee(dossier: &std::path::Path) -> Lycee {
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
    let (dix, onze, sept) = (niveau("10"), niveau("11"), niveau("7"));
    let serie_s = referentiel["series"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["code"] == "S" && s["niveau_id"] == onze)
        .unwrap()["id"]
        .clone();
    let classe = |niveau: &Value, serie: &Value, libelle: &str| json!({ "annee_id": annee["id"], "niveau_id": niveau, "serie_id": serie, "libelle": libelle });
    let dixieme = p
        .ok(
            "classe_creer",
            Some(&admin),
            classe(&dix, &Value::Null, "10e A"),
        )
        .await;
    let onzieme_s = p
        .ok(
            "classe_creer",
            Some(&admin),
            classe(&onze, &serie_s, "11e S"),
        )
        .await;
    let septieme = p
        .ok(
            "classe_creer",
            Some(&admin),
            classe(&sept, &Value::Null, "7e A"),
        )
        .await;
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
    p.ok(
        "grille_modifier",
        Some(&admin),
        json!({ "id": grille["grille"]["id"], "lue_le": grille["modifiee_le"], "libelle": "Frais 2026-2027", "plans": plans, "lignes": [
            { "niveau_id": dix, "serie_id": null, "type_frais": "inscription", "libelle": "Frais d'inscription",
              "montant": 15_000, "obligatoire": true, "exigibilite": "une_fois" },
            { "niveau_id": dix, "serie_id": null, "type_frais": "scolarite", "libelle": "Scolarité",
              "montant": 120_000, "obligatoire": true, "exigibilite": "selon_plan" },
            { "niveau_id": dix, "serie_id": null, "type_frais": "cantine", "libelle": "Cantine",
              "montant": 30_000, "obligatoire": false, "exigibilite": "une_fois" },
            { "niveau_id": onze, "serie_id": null, "type_frais": "inscription", "libelle": "Frais d'inscription",
              "montant": 15_000, "obligatoire": true, "exigibilite": "une_fois" },
            { "niveau_id": onze, "serie_id": serie_s, "type_frais": "scolarite", "libelle": "Scolarité",
              "montant": 110_000, "obligatoire": true, "exigibilite": "selon_plan" },
            { "niveau_id": sept, "serie_id": null, "type_frais": "inscription", "libelle": "Frais d'inscription",
              "montant": 15_000, "obligatoire": true, "exigibilite": "une_fois" },
            { "niveau_id": sept, "serie_id": null, "type_frais": "scolarite", "libelle": "Scolarité",
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
    let cantine = vigueur["grille"]["contenu"]["lignes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["type_frais"] == "cantine")
        .unwrap()["id"]
        .clone();
    p.ok("caisse_ouvrir", Some(&admin), json!({ "fonds_initial": 0 }))
        .await;
    Lycee {
        p,
        admin,
        annee,
        dixieme,
        onzieme_s,
        septieme,
        cantine,
        niveau_10: dix,
        niveau_11: onze,
        serie_s,
    }
}

/// Élève avec un responsable payeur, en brouillon dans la classe.
async fn brouillon(e: &Lycee, prenoms: &str, classe: &Value) -> (Value, Value) {
    let fiche = e
        .p
        .ok(
            "eleve_creer",
            Some(&e.admin),
            json!({ "identite": { "nom": "Coulibaly", "prenoms": prenoms, "sexe": "feminin",
                "date_naissance": "2010-05-14", "date_naissance_estimee": false, "lieu_naissance": "Ségou",
                "nationalite": "Malienne", "adresse": "", "telephones": [] },
                "matricule_interne": "", "matricule_national": matricule_national(), "acte_nature": "acte",
                "acte_numero": "", "acte_date": "", "ecole_origine": "" }),
        )
        .await;
    e.p.ok(
        "responsable_rattacher",
        Some(&e.admin),
        json!({ "eleve_id": fiche["eleve"]["id"], "responsable_id": null,
            "saisie": { "identite": { "nom": "Keïta", "prenoms": "Mariam", "sexe": "feminin",
                "date_naissance": "", "date_naissance_estimee": false, "lieu_naissance": "", "nationalite": "",
                "adresse": "", "telephones": [format!("66 00 00 {:02}", prenoms.len())] }, "profession": "", "employeur": "" },
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

/// Élève inscrit et validé dans la classe (plan par défaut, sans cantine).
async fn inscrit(e: &Lycee, prenoms: &str, classe: &Value) -> (Value, Value) {
    let (fiche, inscription) = brouillon(e, prenoms, classe).await;
    e.p.ok(
        "inscription_valider",
        Some(&e.admin),
        json!({ "id": inscription["id"] }),
    )
    .await;
    (fiche, inscription)
}

async fn situation(e: &Lycee, fiche: &Value) -> Value {
    e.p.ok(
        "situation_financiere",
        Some(&e.admin),
        json!({ "id": fiche["eleve"]["id"] }),
    )
    .await[0]
        .clone()
}

/// Appel du service, comme la commande du même nom.
async fn appeler(
    e: &Lycee,
    nom: &str,
    jeton: &Jeton,
    entree: Value,
) -> Result<Value, ErreurCommande> {
    e.p.application
        .prise_en_charge_essai(nom, jeton, entree)
        .await
}

async fn ok(e: &Lycee, nom: &str, jeton: &Jeton, entree: Value) -> Value {
    appeler(e, nom, jeton, entree)
        .await
        .unwrap_or_else(|err| panic!("{nom} : {err:?}"))
}

async fn refus(e: &Lycee, nom: &str, jeton: &Jeton, entree: Value) -> String {
    appeler(e, nom, jeton, entree).await.expect_err(nom).cle
}

fn cibles(inscriptions: &[&Value]) -> Value {
    Value::Array(
        inscriptions
            .iter()
            .map(|i| json!({ "inscription_id": i["id"] }))
            .collect(),
    )
}

fn cocher(inscriptions: &[&Value], attendu: Option<i64>) -> Value {
    json!({ "cibles": cibles(inscriptions), "geste": "cocher", "montant_attendu": attendu })
}

fn n(v: &Value) -> i64 {
    v.as_i64().unwrap()
}

/// Ligne de la liste d'une inscription.
fn ligne<'a>(liste: &'a Value, inscription: &Value) -> &'a Value {
    liste["lignes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["inscription_id"] == inscription["id"])
        .unwrap()
}

async fn lister(e: &Lycee) -> Value {
    ok(
        e,
        "prises_en_charge_lister",
        &e.admin,
        json!({ "annee_id": e.annee["id"] }),
    )
    .await
}

async fn compter(e: &Lycee, requete: &str) -> i64 {
    let mut cnx = base_brute(&e.p.racine, e.p.trousseau.clone()).await;
    sqlx::query_scalar(sqlx::AssertSqlSafe(requete.to_owned()))
        .fetch_one(&mut cnx)
        .await
        .unwrap()
}

async fn compte(e: &Lycee, identifiant: &str, role: &str) -> Jeton {
    let mot_de_passe = format!("lycee-2026-{}", identifiant.len() * 7919);
    e.p.ok(
        "utilisateur_creer",
        Some(&e.admin),
        json!({ "identifiant": identifiant, "nom_affiche": identifiant,
            "mot_de_passe_initial": mot_de_passe, "roles": [role] }),
    )
    .await;
    e.p.connecter(identifiant, &mot_de_passe).await
}

/// Sélection : montant proposé élève par élève (tableau, facture, grille), aperçu groupé, une
/// transaction, un audit par élève et un pour le lot ; la facture née d'une inscription cochée
/// porte son statut dès sa naissance.
#[tokio::test]
async fn cocher_une_selection_en_une_transaction() {
    let dossier = tempfile::tempdir().unwrap();
    let e = lycee(dossier.path()).await;
    let (awa, awa_i) = inscrit(&e, "Awa", &e.dixieme).await;
    let (chaka, chaka_i) = brouillon(&e, "Chaka", &e.dixieme).await;
    let (_, dado_i) = inscrit(&e, "Dado", &e.onzieme_s).await;

    // Le tableau des montants : 126 000 F pour la 11e S, vide pour la 10e.
    let tableau = ok(
        &e,
        "montants_etat_lire",
        &e.admin,
        json!({ "annee_id": e.annee["id"] }),
    )
    .await;
    assert_eq!(tableau["lignes"].as_array().unwrap().len(), 2);
    let tableau = ok(
        &e,
        "montants_etat_regler",
        &e.admin,
        json!({ "annee_id": e.annee["id"], "lue_le": tableau["lue_le"], "lignes": [
            { "niveau_id": e.niveau_11, "serie_id": e.serie_s, "montant": 126_000 },
        ] }),
    )
    .await;
    let ligne_tableau = |niveau: &Value| {
        tableau["lignes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|l| l["niveau_id"] == *niveau)
            .unwrap()
            .clone()
    };
    assert_eq!(ligne_tableau(&e.niveau_11)["montant"], 126_000);
    assert_eq!(ligne_tableau(&e.niveau_11)["frais_ecole"], 110_000);
    assert!(ligne_tableau(&e.niveau_10)["montant"].is_null());
    assert_eq!(ligne_tableau(&e.niveau_10)["frais_ecole"], 120_000);

    // Aperçu groupé : rien n'est écrit.
    let selection = [&awa_i, &chaka_i, &dado_i];
    let apercu = ok(
        &e,
        "prise_en_charge_apercu",
        &e.admin,
        json!({ "inscriptions": [awa_i["id"], chaka_i["id"], dado_i["id"]], "geste": "cocher" }),
    )
    .await;
    let eleves = apercu["eleves"].as_array().unwrap();
    // Montant proposé : la facture (Awa), la grille sans facture (Chaka), le tableau (Dado).
    assert_eq!(eleves[0]["montant_propose"], 120_000);
    assert_eq!(eleves[1]["montant_propose"], 120_000);
    assert_eq!(eleves[1]["sans_facture"], true);
    assert_eq!(eleves[2]["montant_propose"], 126_000);
    assert_eq!(apercu["nombre"], 3);
    assert_eq!(apercu["refuses"], 0);
    // Awa ne doit plus 135 000 F, Dado 125 000 F ; Chaka n'a pas encore de facture.
    assert_eq!(apercu["parents_ne_doivent_plus"], 260_000);
    assert_eq!(apercu["doit_encore_etat_avant"], 0);
    assert_eq!(apercu["doit_encore_etat_apres"], 366_000);
    assert_eq!(
        compter(&e, "SELECT COUNT(*) FROM prise_en_charge_etat").await,
        0
    );

    // Le geste, en une transaction.
    let lignes = ok(
        &e,
        "prise_en_charge_regler",
        &e.admin,
        cocher(&selection, None),
    )
    .await;
    assert_eq!(lignes.as_array().unwrap().len(), 3);
    let liste = lister(&e).await;
    assert_eq!(liste["nombre"], 3);
    assert_eq!(liste["total_attendu"], 366_000);
    assert_eq!(liste["total_du_etat"], 366_000);
    assert_eq!(ligne(&liste, &awa_i)["montant_attendu"], 120_000);
    assert_eq!(ligne(&liste, &awa_i)["reste_parent"], 0);
    assert_eq!(ligne(&liste, &dado_i)["montant_attendu"], 126_000);
    // Dado : 110 000 F sur la facture, 16 000 F en complément hors facture.
    assert_eq!(ligne(&liste, &dado_i)["du_etat"], 126_000);
    assert_eq!(ligne(&liste, &chaka_i)["sans_facture"], true);
    assert_eq!(
        compter(
            &e,
            "SELECT COUNT(*) FROM inscription WHERE prise_en_charge_etat = 1"
        )
        .await,
        3
    );
    assert_eq!(
        compter(
            &e,
            "SELECT COUNT(*) FROM journal_audit WHERE entite = 'prise_en_charge_etat'"
        )
        .await,
        3
    );
    assert_eq!(
        compter(
            &e,
            "SELECT COUNT(*) FROM journal_audit WHERE entite = 'prise_en_charge_lot'"
        )
        .await,
        1
    );
    assert_eq!(
        compter(
            &e,
            "SELECT COUNT(*) FROM outbox WHERE type_evenement = 'PriseEnChargeEtatCochee'"
        )
        .await,
        3
    );
    let s = situation(&e, &awa).await;
    assert_eq!(s["facture"]["reste"], 0);
    assert_eq!(s["facture"]["part_etat"], 120_000);
    assert_eq!(s["facture"]["facture"]["total"], 135_000);

    // Cocher de nouveau les mêmes : rien ne change, rien n'est écrit.
    ok(
        &e,
        "prise_en_charge_regler",
        &e.admin,
        cocher(&selection, None),
    )
    .await;
    assert_eq!(
        compter(&e, "SELECT COUNT(*) FROM prise_en_charge_etat").await,
        3
    );

    // Chaka : l'État a payé avant la validation ; la facture naît soldée.
    e.p.application
        .versement_etat_pour_essai(chaka_i["id"].as_str().unwrap(), 120_000)
        .await
        .unwrap();
    e.p.ok(
        "inscription_valider",
        Some(&e.admin),
        json!({ "id": chaka_i["id"] }),
    )
    .await;
    let s = situation(&e, &chaka).await;
    assert_eq!(s["facture"]["reste"], 0);
    assert_eq!(s["facture"]["reste_etat"], 0);
    assert_eq!(s["facture"]["renonce"], 15_000);
    assert_eq!(s["facture"]["facture"]["statut"], "soldee");

    // Une classe du second cycle n'a pas de case : refusé, et rien n'est écrit pour la sélection.
    let (_, eva_i) = inscrit(&e, "Eva", &e.septieme).await;
    let (_, fanta_i) = inscrit(&e, "Fanta", &e.dixieme).await;
    assert_eq!(
        refus(
            &e,
            "prise_en_charge_regler",
            &e.admin,
            cocher(&[&fanta_i, &eva_i], None),
        )
        .await,
        "prise_en_charge.classe_non_concernee"
    );
    assert_eq!(
        compter(&e, "SELECT COUNT(*) FROM prise_en_charge_etat").await,
        3
    );
    let apercu = ok(
        &e,
        "prise_en_charge_apercu",
        &e.admin,
        json!({ "inscriptions": [fanta_i["id"], eva_i["id"]], "geste": "cocher" }),
    )
    .await;
    assert_eq!(apercu["refuses"], 1);
    assert_eq!(
        apercu["eleves"][1]["refus"],
        "prise_en_charge.classe_non_concernee"
    );
}

/// Un élève avec la cantine, coché avec un montant donné : le parent ne doit que la cantine ;
/// le décochage montre ce qu'il devra.
#[tokio::test]
async fn l_apercu_dit_ce_que_le_parent_devra() {
    let dossier = tempfile::tempdir().unwrap();
    let e = lycee(dossier.path()).await;
    let (_, ins) = brouillon(&e, "Awa", &e.dixieme).await;
    e.p.ok(
        "inscription_valider",
        Some(&e.admin),
        json!({ "id": ins["id"], "choix": { "frais_optionnels": [e.cantine] } }),
    )
    .await;
    ok(
        &e,
        "prise_en_charge_regler",
        &e.admin,
        json!({ "cibles": cibles(&[&ins]), "geste": "cocher", "montant_attendu": 100_000,
            "reference": "DEF-2026-0042" }),
    )
    .await;
    let liste = lister(&e).await;
    let l = ligne(&liste, &ins);
    assert_eq!(l["reste_parent"], 30_000);
    assert_eq!(l["reference"], "DEF-2026-0042");
    // L'écart de 20 000 F est à la charge de l'école : l'État doit 100 000 F.
    assert_eq!(l["reste_etat"], 100_000);

    let apercu = ok(
        &e,
        "prise_en_charge_apercu",
        &e.admin,
        json!({ "inscriptions": [ins["id"]], "geste": "decocher" }),
    )
    .await;
    assert_eq!(apercu["eleves"][0]["reste_parent_avant"], 30_000);
    assert_eq!(apercu["eleves"][0]["reste_parent_apres"], 165_000);
    assert_eq!(apercu["parents_devront_en_plus"], 135_000);
    assert_eq!(apercu["dont_deja_rembourse"], 0);
    assert_eq!(apercu["doit_encore_etat_apres"], 0);

    // L'effet d'un autre attendu pour l'État.
    let apercu = ok(
        &e,
        "prise_en_charge_apercu",
        &e.admin,
        json!({ "inscriptions": [ins["id"]], "geste": "modifier", "montant_attendu": 126_000 }),
    )
    .await;
    assert_eq!(apercu["doit_encore_etat_avant"], 100_000);
    assert_eq!(apercu["doit_encore_etat_apres"], 126_000);
    assert_eq!(apercu["eleves"][0]["reste_parent_apres"], 30_000);

    // Décocher demande un motif.
    assert_eq!(
        refus(
            &e,
            "prise_en_charge_regler",
            &e.admin,
            json!({ "cibles": cibles(&[&ins]), "geste": "decocher", "motif": " " }),
        )
        .await,
        "prise_en_charge.motif_obligatoire"
    );
    ok(
        &e,
        "prise_en_charge_regler",
        &e.admin,
        json!({ "cibles": cibles(&[&ins]), "geste": "decocher", "motif": "Pas orienté par l'État" }),
    )
    .await;
    let liste = lister(&e).await;
    assert_eq!(ligne(&liste, &ins)["reste_parent"], 165_000);
    assert_eq!(ligne(&liste, &ins)["pris_en_charge"], false);
    assert_eq!(ligne(&liste, &ins)["statut"], "terminee");
    assert_eq!(
        compter(
            &e,
            "SELECT COUNT(*) FROM inscription WHERE prise_en_charge_etat = 1"
        )
        .await,
        0
    );
}

/// Essai 7, partie case : sur une année close, cocher et décocher sont refusés ; corriger
/// l'attendu et abandonner restent permis.
#[tokio::test]
async fn l_annee_close_fige_la_case_pas_l_attendu() {
    let dossier = tempfile::tempdir().unwrap();
    let e = lycee(dossier.path()).await;
    let (_, awa_i) = inscrit(&e, "Awa", &e.dixieme).await;
    let (_, binta_i) = inscrit(&e, "Binta", &e.dixieme).await;
    ok(
        &e,
        "prise_en_charge_regler",
        &e.admin,
        cocher(&[&awa_i], Some(120_000)),
    )
    .await;
    {
        let mut cnx = base_brute(&e.p.racine, e.p.trousseau.clone()).await;
        sqlx::query("UPDATE annee_scolaire SET statut = 'cloturee'")
            .execute(&mut cnx)
            .await
            .unwrap();
    }
    assert_eq!(
        refus(
            &e,
            "prise_en_charge_regler",
            &e.admin,
            cocher(&[&binta_i], None),
        )
        .await,
        "prise_en_charge.annee_close"
    );
    assert_eq!(
        refus(
            &e,
            "prise_en_charge_regler",
            &e.admin,
            json!({ "cibles": cibles(&[&awa_i]), "geste": "decocher", "motif": "Pas orienté par l'État" }),
        )
        .await,
        "prise_en_charge.annee_close"
    );
    ok(
        &e,
        "prise_en_charge_regler",
        &e.admin,
        json!({ "cibles": cibles(&[&awa_i]), "geste": "modifier", "montant_attendu": 100_000 }),
    )
    .await;
    ok(
        &e,
        "prise_en_charge_abandonner",
        &e.admin,
        json!({ "cibles": cibles(&[&awa_i]), "montant": 20_000, "motif": "Règlement définitif de l'année" }),
    )
    .await;
    let liste = lister(&e).await;
    assert_eq!(liste["annee_close"], true);
    let l = ligne(&liste, &awa_i);
    assert_eq!(l["montant_attendu"], 100_000);
    assert_eq!(l["abandonne"], 20_000);
    assert_eq!(l["reste_etat"], 80_000);
    assert_eq!(l["pris_en_charge"], true);
    assert_eq!(ligne(&liste, &binta_i)["pris_en_charge"], false);
}

/// Essai 9 : le Secrétaire et le Caissier sont refusés sur chaque écriture de la prise en
/// charge, la grille « Pour les rapports » comprise ; le Directeur et le Comptable sont acceptés.
#[tokio::test]
async fn seuls_la_direction_et_la_comptabilite_reglent_la_prise_en_charge() {
    let dossier = tempfile::tempdir().unwrap();
    let e = lycee(dossier.path()).await;
    let (_, awa_i) = inscrit(&e, "Awa", &e.dixieme).await;
    let (_, binta_i) = inscrit(&e, "Binta", &e.dixieme).await;
    let secretaire = compte(&e, "secretaire", "secretaire").await;
    let caissier = compte(&e, "caissier", "caissier").await;
    let directeur = compte(&e, "directeur", "directeur").await;
    let comptable = compte(&e, "comptable", "comptable").await;
    let ecritures = |ins: &Value| {
        vec![
            ("prise_en_charge_regler", cocher(&[ins], None)),
            (
                "prise_en_charge_regler",
                json!({ "cibles": cibles(&[ins]), "geste": "modifier", "montant_attendu": 90_000 }),
            ),
            (
                "prise_en_charge_regler",
                json!({ "cibles": cibles(&[ins]), "geste": "decocher", "motif": "Pas orienté par l'État" }),
            ),
            (
                "prise_en_charge_abandonner",
                json!({ "cibles": cibles(&[ins]), "motif": "Règlement définitif" }),
            ),
            (
                "prise_en_charge_apercu",
                json!({ "inscriptions": [ins["id"]], "geste": "cocher" }),
            ),
            (
                "montants_etat_regler",
                json!({ "annee_id": e.annee["id"], "lignes": [
                    { "niveau_id": e.niveau_10, "serie_id": null, "montant": 126_000 } ] }),
            ),
        ]
    };
    for jeton in [&secretaire, &caissier] {
        for (nom, entree) in ecritures(&awa_i) {
            assert_eq!(
                refus(&e, nom, jeton, entree).await,
                "erreur.permission_refusee",
                "{nom}"
            );
        }
    }
    // La grille « Pour les rapports » : le Secrétaire y lit la case sans la changer.
    assert_eq!(
        e.p.refus(
            "complement_eleve_regler",
            Some(&secretaire),
            json!({ "inscription_id": awa_i["id"], "annee_recrutement": null,
                "prise_en_charge_etat": true }),
        )
        .await
        .cle,
        "erreur.permission_refusee"
    );
    assert_eq!(
        compter(&e, "SELECT COUNT(*) FROM prise_en_charge_etat").await,
        0
    );
    assert_eq!(compter(&e, "SELECT COUNT(*) FROM montant_etat").await, 0);

    // Le Directeur et le Comptable, chacun sur un élève.
    for (jeton, ins) in [(&directeur, &awa_i), (&comptable, &binta_i)] {
        for (nom, entree) in ecritures(ins) {
            if nom == "prise_en_charge_abandonner" {
                // Décoché à l'étape d'avant : on recoche pour abandonner.
                ok(&e, "prise_en_charge_regler", jeton, cocher(&[ins], None)).await;
            }
            ok(&e, nom, jeton, entree).await;
        }
    }
    assert_eq!(
        compter(
            &e,
            "SELECT COUNT(*) FROM prise_en_charge_etat WHERE statut <> 'remplacee'"
        )
        .await,
        2
    );
    // Par la grille, l'administrateur coche avec le montant proposé.
    let (_, chaka_i) = inscrit(&e, "Chaka", &e.dixieme).await;
    e.p.ok(
        "complement_eleve_regler",
        Some(&e.admin),
        json!({ "inscription_id": chaka_i["id"], "annee_recrutement": null,
            "prise_en_charge_etat": true }),
    )
    .await;
    let liste = lister(&e).await;
    assert_eq!(ligne(&liste, &chaka_i)["pris_en_charge"], true);
    assert_eq!(ligne(&liste, &chaka_i)["montant_attendu"], 126_000);
}

/// Essai 10, partie audit : chaque geste laisse la version ancienne `remplacee`, liée, et un
/// audit avant et après ; corriger l'attendu ou la référence d'un élève dont l'abandon reste
/// recopie le motif de l'abandon ; la fraîcheur refuse une ligne lue avant un autre geste.
#[tokio::test]
async fn chaque_geste_garde_sa_trace() {
    let dossier = tempfile::tempdir().unwrap();
    let e = lycee(dossier.path()).await;
    let (_, ins) = inscrit(&e, "Awa", &e.dixieme).await;
    let geste = |entree: Value| ok(&e, "prise_en_charge_regler", &e.admin, entree);
    geste(cocher(&[&ins], Some(120_000))).await;
    let lue = ligne(&lister(&e).await, &ins)["lue_le"].clone();
    e.p.horloge.avancer(60_000);
    geste(json!({ "cibles": cibles(&[&ins]), "geste": "modifier", "montant_attendu": 100_000 }))
        .await;
    // Ligne lue avant la modification : refusée.
    assert_eq!(
        refus(
            &e,
            "prise_en_charge_regler",
            &e.admin,
            json!({ "cibles": [{ "inscription_id": ins["id"], "lue_le": lue }], "geste": "modifier",
                "montant_attendu": 90_000 }),
        )
        .await,
        "fiche.modifiee_entre_temps"
    );
    // Ne plus attendre : jamais plus que ce que l'État doit encore.
    assert_eq!(
        refus(
            &e,
            "prise_en_charge_abandonner",
            &e.admin,
            json!({ "cibles": cibles(&[&ins]), "montant": 100_001, "motif": "Règlement définitif" }),
        )
        .await,
        "prise_en_charge.abandon_superieur"
    );
    assert_eq!(
        refus(
            &e,
            "prise_en_charge_abandonner",
            &e.admin,
            json!({ "cibles": cibles(&[&ins]), "montant": 20_000, "motif": "" }),
        )
        .await,
        "prise_en_charge.motif_obligatoire"
    );
    e.p.horloge.avancer(60_000);
    ok(
        &e,
        "prise_en_charge_abandonner",
        &e.admin,
        json!({ "cibles": cibles(&[&ins]), "montant": 20_000, "motif": "Règlement définitif de l'année" }),
    )
    .await;
    e.p.horloge.avancer(60_000);
    geste(json!({ "cibles": cibles(&[&ins]), "geste": "modifier", "reference": "DEF-2026-0042" }))
        .await;

    let historique = ok(
        &e,
        "prise_en_charge_historique",
        &e.admin,
        json!({ "inscription_id": ins["id"] }),
    )
    .await;
    let h = historique.as_array().unwrap();
    assert_eq!(h.len(), 4);
    assert_eq!(h[0]["statut"], "en_cours");
    assert_eq!(h[0]["montant_attendu"], 100_000);
    assert_eq!(h[0]["abandonne"], 20_000);
    assert_eq!(h[0]["reference"], "DEF-2026-0042");
    // Le motif de l'abandon est recopié : la base l'exige tant qu'un abandon reste.
    assert_eq!(h[0]["motif"], "Règlement définitif de l'année");
    for i in 1..4 {
        assert_eq!(h[i]["statut"], "remplacee");
        assert_eq!(h[i - 1]["remplace_id"], h[i]["id"]);
    }
    assert!(h[3]["remplace_id"].is_null());

    // Défaire l'abandon, puis décocher : deux versions de plus.
    e.p.horloge.avancer(60_000);
    ok(
        &e,
        "prise_en_charge_abandonner",
        &e.admin,
        json!({ "cibles": cibles(&[&ins]), "defaire": true, "motif": "L'État a repris le paiement" }),
    )
    .await;
    e.p.horloge.avancer(60_000);
    geste(json!({ "cibles": cibles(&[&ins]), "geste": "decocher", "motif": "Pas orienté par l'État" })).await;
    let historique = ok(
        &e,
        "prise_en_charge_historique",
        &e.admin,
        json!({ "inscription_id": ins["id"] }),
    )
    .await;
    let h = historique.as_array().unwrap();
    assert_eq!(h.len(), 6);
    assert_eq!(h[0]["statut"], "terminee");
    assert_eq!(h[1]["abandonne"], 0);

    // Un audit par geste, avant et après.
    let mut cnx = base_brute(&e.p.racine, e.p.trousseau.clone()).await;
    let audits: Vec<(String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT operation, avant, apres FROM journal_audit WHERE entite = 'prise_en_charge_etat'
         ORDER BY horodatage, rowid",
    )
    .fetch_all(&mut cnx)
    .await
    .unwrap();
    assert_eq!(audits.len(), 6);
    assert!(audits[0].1.is_none());
    for (i, (_, avant, apres)) in audits.iter().enumerate().skip(1) {
        let avant: Value = serde_json::from_str(avant.as_deref().unwrap()).unwrap();
        let apres: Value = serde_json::from_str(apres.as_deref().unwrap()).unwrap();
        assert_eq!(apres["remplace_id"], avant["id"], "geste {i}");
        assert_eq!(avant["statut"], "en_cours", "geste {i}");
    }
    let dernier: Value = serde_json::from_str(audits[5].2.as_deref().unwrap()).unwrap();
    assert_eq!(dernier["statut"], "terminee");
    let modifiees: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM outbox WHERE type_evenement = 'PriseEnChargeEtatModifiee'",
    )
    .fetch_one(&mut cnx)
    .await
    .unwrap();
    assert_eq!(modifiees, 4);
    let decochees: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM outbox WHERE type_evenement = 'PriseEnChargeEtatDecochee'",
    )
    .fetch_one(&mut cnx)
    .await
    .unwrap();
    assert_eq!(decochees, 1);
    assert!(
        sqlx::query("DELETE FROM prise_en_charge_etat")
            .execute(&mut cnx)
            .await
            .is_err()
    );
}

/// Changement de classe (§5.3) : l'aperçu propose de décocher pour une classe hors du lycée et
/// du technique, et le montant réglé de la nouvelle classe ; rien ne se fait tout seul. Le
/// tableau des montants dit quels élèves cochés changer, puis « Appliquer » les porte au montant
/// proposé.
#[tokio::test]
async fn le_changement_de_classe_et_le_tableau_proposent_sans_rien_faire() {
    let dossier = tempfile::tempdir().unwrap();
    let e = lycee(dossier.path()).await;
    let (_, ins) = inscrit(&e, "Awa", &e.dixieme).await;
    ok(
        &e,
        "prise_en_charge_regler",
        &e.admin,
        cocher(&[&ins], None),
    )
    .await;
    ok(
        &e,
        "montants_etat_regler",
        &e.admin,
        json!({ "annee_id": e.annee["id"], "lignes": [
            { "niveau_id": e.niveau_11, "serie_id": e.serie_s, "montant": 126_000 } ] }),
    )
    .await;
    let vers = |classe: &Value| {
        e.p.ok(
            "facture_ajustement_apercu",
            Some(&e.admin),
            json!({ "inscription_id": ins["id"], "classe_id": classe["id"] }),
        )
    };
    let apercu = vers(&e.septieme).await;
    assert_eq!(apercu["proposer_decocher"], true);
    assert!(apercu["montant_etat_propose"].is_null());
    let apercu = vers(&e.onzieme_s).await;
    assert_eq!(apercu["proposer_decocher"], false);
    assert_eq!(apercu["montant_etat_propose"], 126_000);
    assert_eq!(
        compter(&e, "SELECT COUNT(*) FROM prise_en_charge_etat").await,
        1
    );

    // Le tableau : 130 000 F pour la 10e ; Awa (120 000 F) est à changer.
    let tableau = ok(
        &e,
        "montants_etat_regler",
        &e.admin,
        json!({ "annee_id": e.annee["id"], "lignes": [
            { "niveau_id": e.niveau_10, "serie_id": null, "montant": 130_000 } ] }),
    )
    .await;
    assert_eq!(tableau["inscriptions_a_changer"], json!([ins["id"]]));
    let dixieme = tableau["lignes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["niveau_id"] == e.niveau_10)
        .unwrap();
    assert_eq!(dixieme["eleves_coches"], 1);
    assert_eq!(dixieme["eleves_a_changer"], 1);
    assert_eq!(
        n(&ligne(&lister(&e).await, &ins)["montant_attendu"]),
        120_000
    );
    let apercu = ok(
        &e,
        "prise_en_charge_apercu",
        &e.admin,
        json!({ "inscriptions": tableau["inscriptions_a_changer"], "geste": "appliquer_montant_propose" }),
    )
    .await;
    assert_eq!(apercu["doit_encore_etat_avant"], 120_000);
    assert_eq!(apercu["doit_encore_etat_apres"], 130_000);
    ok(
        &e,
        "prise_en_charge_regler",
        &e.admin,
        json!({ "cibles": cibles(&[&ins]), "geste": "appliquer_montant_propose" }),
    )
    .await;
    let l = ligne(&lister(&e).await, &ins).clone();
    assert_eq!(l["montant_attendu"], 130_000);
    // 120 000 F sur la facture, 10 000 F en complément hors facture.
    assert_eq!(l["du_etat"], 130_000);
    assert_eq!(l["reste_parent"], 0);
    let tableau = ok(
        &e,
        "montants_etat_lire",
        &e.admin,
        json!({ "annee_id": e.annee["id"] }),
    )
    .await;
    assert_eq!(tableau["inscriptions_a_changer"], json!([]));
}
