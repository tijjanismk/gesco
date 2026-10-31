//! Droits du personnel par la frontière (SPEC_PHASE_6 §7, §8, Q-67) : sans `contracts:read`, le
//! service retire les montants des engagements, des heures et du journal ; chacun lit sa propre
//! fiche, montants compris, sans aucun droit ; le tableau de bord montre le personnel du jour.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use crate::commun::*;
use gesco_services::session::Jeton;
use serde_json::{Value, json};

async fn compte(p: &Poste_, admin: &Jeton, identifiant: &str, role: &str) -> (Value, Jeton) {
    let mot_de_passe = format!("ecole-2026-{}", identifiant.len() * 7919);
    let u = p
        .ok(
            "utilisateur_creer",
            Some(admin),
            json!({ "identifiant": identifiant, "nom_affiche": identifiant,
                "mot_de_passe_initial": mot_de_passe, "roles": [role] }),
        )
        .await;
    (
        u["id"].clone(),
        p.connecter(identifiant, &mot_de_passe).await,
    )
}

async fn membre(
    p: &Poste_,
    admin: &Jeton,
    (prenoms, nom): (&str, &str),
    categorie: &str,
    fonction: &str,
    utilisateur: Value,
) -> Value {
    let referentiel = p
        .ok("personnel_referentiel_lire", Some(admin), Value::Null)
        .await;
    let categorie = referentiel["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["code"] == categorie)
        .unwrap()["id"]
        .clone();
    p.ok(
        "membre_enregistrer",
        Some(admin),
        json!({
            "id": null, "nom": nom, "prenoms": prenoms, "sexe": null, "date_naissance": null,
            "telephones": [], "adresse": "", "matricule": "", "categorie_id": categorie,
            "fonction": fonction, "arrive_le": "2025-10-01", "fonctionnaire": false,
            "numero_fonctionnaire": "", "numero_inps": "", "prevenir_nom": "",
            "prevenir_telephone": "", "paiement": { "mode": "especes" }, "diplomes": [],
            "utilisateur_id": utilisateur,
        }),
    )
    .await["membre"]
        .clone()
}

async fn engager(
    p: &Poste_,
    admin: &Jeton,
    m: &Value,
    nature: &str,
    fin: Value,
    r: Value,
) -> Value {
    p.ok(
        "engagement_conclure",
        Some(admin),
        json!({
            "membre_id": m["id"], "nature": nature, "debut": "2026-09-01", "fin": fin,
            "fonction": "", "remuneration": r, "note": "", "remplace": null, "renouvelle": null,
        }),
    )
    .await
}

fn remuneration(engagements: &Value) -> Value {
    engagements[0]["engagement"]["remuneration"].clone()
}

#[tokio::test]
async fn les_montants_suivent_contracts_read_et_chacun_lit_sa_fiche() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let (id_aminata, aminata) = compte(&p, &admin, "aminata", "secretaire").await;
    let (_, compta) = compte(&p, &admin, "compta", "comptable").await;
    let (id_prof, prof) = compte(&p, &admin, "prof", "enseignant").await;
    let (_, lecture) = compte(&p, &admin, "lecture", "consultation_seule").await;

    let gardien = membre(
        &p,
        &admin,
        ("Moussa", "Coulibaly"),
        "appui",
        "Gardien",
        Value::Null,
    )
    .await;
    let secretaire = membre(
        &p,
        &admin,
        ("Aminata", "Kanté"),
        "administration",
        "Secrétaire",
        id_aminata,
    )
    .await;
    let vacataire = membre(
        &p,
        &admin,
        ("Bakary", "Sangaré"),
        "enseignement",
        "Professeur de mathématiques",
        id_prof,
    )
    .await;
    engager(
        &p,
        &admin,
        &gardien,
        "duree_indeterminee",
        Value::Null,
        json!({ "type": "mensuelle", "montant": 75_000 }),
    )
    .await;
    // Un engagement à durée déterminée qui finit dans moins de soixante jours.
    let cdd = engager(
        &p,
        &admin,
        &secretaire,
        "duree_determinee",
        json!("2026-11-30"),
        json!({ "type": "mensuelle", "montant": 120_000 }),
    )
    .await;
    engager(
        &p,
        &admin,
        &vacataire,
        "vacation",
        json!("2027-06-30"),
        json!({ "type": "horaire", "taux": 2_500, "minutes_par_semaine": 360 }),
    )
    .await;

    // La secrétaire lit les fiches et les engagements, sans aucun montant…
    let du_gardien = p
        .ok(
            "engagements_lister",
            Some(&aminata),
            json!({ "id": gardien["id"] }),
        )
        .await;
    assert_eq!(remuneration(&du_gardien), json!({ "type": "masquee" }));
    p.ok(
        "membre_lire",
        Some(&aminata),
        json!({ "id": gardien["id"] }),
    )
    .await;
    // … sauf les siens.
    let les_siens = p
        .ok(
            "engagements_lister",
            Some(&aminata),
            json!({ "id": secretaire["id"] }),
        )
        .await;
    assert_eq!(remuneration(&les_siens)["montant"], 120_000);
    let heures = p
        .ok("heures_lire", Some(&aminata), json!({ "mois": "2026-10" }))
        .await;
    assert_eq!(heures["montants"], false);
    assert_eq!(heures["montant"], Value::Null);
    assert_eq!(heures["lignes"][0]["membre"], "Bakary Sangaré");
    assert_eq!(heures["lignes"][0]["taux"], Value::Null);
    assert_eq!(heures["lignes"][0]["minutes_prevues"], 24 * 60);
    let echeances = p
        .ok("engagements_a_echeance", Some(&aminata), Value::Null)
        .await;
    assert_eq!(echeances[0]["engagement"]["id"], cdd["id"]);
    assert_eq!(
        echeances[0]["engagement"]["remuneration"],
        json!({ "type": "masquee" })
    );
    // Aucun document qui les imprime, aucun engagement conclu.
    for (commande, entree) in [
        ("lettre_engagement_produire", json!({ "id": cdd["id"] })),
        ("etat_heures_produire", json!({ "mois": "2026-10" })),
        (
            "engagement_terminer",
            json!({ "id": cdd["id"], "le": "2026-10-04", "motif": "Départ" }),
        ),
    ] {
        assert_eq!(
            p.refus(commande, Some(&aminata), entree).await.cle,
            "erreur.permission_refusee",
            "{commande}"
        );
    }

    // Le comptable lit les montants, sans gérer les engagements.
    let du_gardien = p
        .ok(
            "engagements_lister",
            Some(&compta),
            json!({ "id": gardien["id"] }),
        )
        .await;
    assert_eq!(remuneration(&du_gardien)["montant"], 75_000);
    let heures = p
        .ok("heures_lire", Some(&compta), json!({ "mois": "2026-10" }))
        .await;
    assert_eq!(heures["montants"], true);
    assert_eq!(heures["lignes"][0]["taux"], 2_500);
    p.ok(
        "lettre_engagement_produire",
        Some(&compta),
        json!({ "id": cdd["id"] }),
    )
    .await;
    assert_eq!(
        p.refus(
            "heures_viser",
            Some(&compta),
            json!({ "mois": "2026-10", "engagement_id": null })
        )
        .await
        .cle,
        "erreur.permission_refusee"
    );

    // L'enseignant n'a aucun droit sur le personnel, mais lit sa fiche, montants compris.
    assert_eq!(
        p.refus("membre_lire", Some(&prof), json!({ "id": vacataire["id"] }))
            .await
            .cle,
        "erreur.permission_refusee"
    );
    let ma_fiche = p.ok("ma_fiche_lire", Some(&prof), Value::Null).await;
    assert_eq!(ma_fiche["fiche"]["membre"]["id"], vacataire["id"]);
    assert_eq!(ma_fiche["fiche"]["personne"]["nom"], "Sangaré");
    assert_eq!(
        remuneration(&ma_fiche["engagements"]),
        json!({ "type": "horaire", "taux": 2_500, "minutes_par_semaine": 360 })
    );
    // Un compte lié à aucun membre n'a pas de fiche.
    assert_eq!(
        p.ok("ma_fiche_lire", Some(&admin), Value::Null).await,
        Value::Null
    );

    // Le journal d'audit suit la même règle.
    let recherche = json!({ "utilisateur_id": null, "entite": "engagement", "operation": null,
        "depuis": null, "jusqua": null, "page": 0 });
    let lignes = p
        .ok("audit_lister", Some(&lecture), recherche.clone())
        .await["lignes"]
        .clone();
    assert_eq!(lignes.as_array().unwrap().len(), 3);
    for l in lignes.as_array().unwrap() {
        assert_eq!(l["apres"]["remuneration"], json!({ "type": "masquee" }));
    }
    let lignes = p.ok("audit_lister", Some(&admin), recherche).await["lignes"].clone();
    assert_ne!(
        lignes[0]["apres"]["remuneration"],
        json!({ "type": "masquee" })
    );

    // Le personnel du jour au tableau de bord : une permission accordée en cours, une demande en
    // attente, un retard noté ce matin.
    let etat = p
        .ok(
            "absences_conges_lire",
            Some(&admin),
            json!({ "mois": "2026-10" }),
        )
        .await;
    let permission = etat["types"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["code"] == "permission")
        .unwrap()["id"]
        .clone();
    let conge = |m: &Value, debut: &str, fin: &str| {
        json!({ "membre_id": m["id"], "type_id": permission, "debut": debut, "fin": fin,
            "motif": "Baptême" })
    };
    let accordee = p
        .ok(
            "conge_demander",
            Some(&admin),
            conge(&secretaire, "2026-10-02", "2026-10-06"),
        )
        .await;
    p.ok(
        "conge_accorder",
        Some(&admin),
        json!({ "id": accordee["id"] }),
    )
    .await;
    p.ok(
        "conge_demander",
        Some(&aminata),
        conge(&vacataire, "2026-10-12", "2026-10-13"),
    )
    .await;
    p.ok(
        "absence_personnel_noter",
        Some(&aminata),
        json!({ "membre_id": gardien["id"], "date": "2026-10-04", "nature": "retard",
            "minutes": 15, "motif": "", "justifiee": false }),
    )
    .await;
    let tableau = p
        .ok("tableau_de_bord_lire", Some(&aminata), Value::Null)
        .await;
    let personnel = &tableau["personnel"];
    assert_eq!(personnel["presents"], 3);
    let categories: Vec<(String, u64)> = personnel["par_categorie"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            (
                c["categorie"].as_str().unwrap().to_owned(),
                c["membres"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(categories.len(), 3);
    assert!(categories.iter().all(|(_, n)| *n == 1));
    assert_eq!(personnel["mois"], "2026-10");
    assert_eq!(personnel["vacataires_du_mois"], 1);
    assert_eq!(personnel["heures_a_viser"], 1);
    assert_eq!(personnel["mois_precedent"], "2026-09");
    assert_eq!(personnel["heures_a_viser_mois_precedent"], 1);
    assert_eq!(
        personnel["echeances"],
        json!([{ "membre": "Aminata Kanté", "nature": "duree_determinee", "fin": "2026-11-30" }])
    );
    assert_eq!(
        personnel["absents"],
        json!([{ "membre": "Moussa Coulibaly", "nature": "retard", "minutes": 15 }])
    );
    assert_eq!(personnel["conges_en_attente"], 1);
    assert_eq!(
        personnel["en_conge"],
        json!([{ "membre": "Aminata Kanté", "type_libelle": "Permission d'absence",
            "fin": "2026-10-06" }])
    );
    // Sans `employees:read`, pas de bloc.
    let tableau = p.ok("tableau_de_bord_lire", Some(&prof), Value::Null).await;
    assert_eq!(tableau["personnel"], Value::Null);

    let verification = p.ok("audit_verifier", Some(&admin), Value::Null).await;
    assert!(verification["lignes_verifiees"].as_u64().unwrap() > 0);
}
