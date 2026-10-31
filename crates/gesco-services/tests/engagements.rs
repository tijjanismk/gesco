//! Engagements du personnel par la frontière (SPEC_PHASE_6 §3) : vacation d'un fonctionnaire
//! pour l'année, salaire changé à une date sans toucher au passé (Q-65), fin proche signalée,
//! clôture motivée, renouvellement pour l'année suivante, lettre d'engagement.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use crate::commun::*;
use gesco_services::session::Jeton;
use serde_json::{Value, json};

async fn membre(p: &Poste_, admin: &Jeton, nom: &str, categorie: &str, fonction: &str) -> Value {
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
            "id": null, "nom": nom, "prenoms": "Bakary", "sexe": "masculin", "date_naissance": null,
            "telephones": [], "adresse": "", "matricule": "", "categorie_id": categorie,
            "fonction": fonction, "arrive_le": null, "fonctionnaire": true,
            "numero_fonctionnaire": "0123456-A", "numero_inps": "", "prevenir_nom": "",
            "prevenir_telephone": "", "paiement": { "mode": "especes" }, "diplomes": [],
            "utilisateur_id": null,
        }),
    )
    .await["membre"]
        .clone()
}

fn vacation(membre: &Value, debut: &str, fin: Value, taux: i64, minutes: i64) -> Value {
    json!({
        "membre_id": membre["id"], "nature": "vacation", "debut": debut, "fin": fin,
        "fonction": "", "remuneration": { "type": "horaire", "taux": taux, "minutes_par_semaine": minutes },
        "note": "Mathématiques en 11S et 12TSE", "remplace": null, "renouvelle": null,
    })
}

fn mensuel(membre: &Value, nature: &str, debut: &str, fin: Value, montant: i64) -> Value {
    json!({
        "membre_id": membre["id"], "nature": nature, "debut": debut, "fin": fin, "fonction": "",
        "remuneration": { "type": "mensuelle", "montant": montant }, "note": "",
        "remplace": null, "renouvelle": null,
    })
}

#[tokio::test]
async fn les_engagements_se_concluent_se_remplacent_et_se_renouvellent() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
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

    // Un fonctionnaire engagé en vacation : la fin de l'année scolaire par défaut.
    let prof = membre(
        &p,
        &admin,
        "Sangaré",
        "enseignement",
        "Professeur de mathématiques",
    )
    .await;
    let v = p
        .ok(
            "engagement_conclure",
            Some(&admin),
            vacation(&prof, "2026-10-01", Value::Null, 2_500, 6 * 60),
        )
        .await;
    assert_eq!(v["fin"], "2027-07-31");
    assert_eq!(v["fonction"], "Professeur de mathématiques");
    assert_eq!(v["statut"], "en_cours");
    // Une seconde vacation sur la même période : refusée.
    assert_eq!(
        p.refus(
            "engagement_conclure",
            Some(&admin),
            vacation(&prof, "2026-11-01", Value::Null, 3_000, 4 * 60),
        )
        .await
        .cle,
        "engagement.chevauchement"
    );
    // Des heures qui ne tombent pas au quart d'heure : refusées.
    assert_eq!(
        p.refus(
            "engagement_conclure",
            Some(&admin),
            vacation(&prof, "2027-08-01", json!("2027-09-30"), 2_500, 6 * 60 + 10),
        )
        .await
        .cle,
        "engagement.heures_invalides"
    );

    // Un gardien à durée indéterminée, augmenté au 1er novembre : l'ancien salaire s'arrête la
    // veille, rien n'est récrit.
    let gardien = membre(&p, &admin, "Keïta", "appui", "Gardien").await;
    let cdi = p
        .ok(
            "engagement_conclure",
            Some(&admin),
            mensuel(
                &gardien,
                "duree_indeterminee",
                "2024-10-01",
                Value::Null,
                60_000,
            ),
        )
        .await;
    let mut augmentation = mensuel(
        &gardien,
        "duree_indeterminee",
        "2026-11-01",
        Value::Null,
        65_000,
    );
    augmentation["remplace"] = cdi["id"].clone();
    let nouveau = p
        .ok("engagement_conclure", Some(&admin), augmentation)
        .await;
    assert_eq!(nouveau["precedent_id"], cdi["id"]);
    let liste = p
        .ok(
            "engagements_lister",
            Some(&admin),
            json!({ "id": gardien["id"] }),
        )
        .await;
    let liste = liste.as_array().unwrap();
    assert_eq!(liste.len(), 2);
    assert_eq!(liste[0]["engagement"]["statut"], "termine");
    assert_eq!(liste[0]["engagement"]["arrete_le"], "2026-10-31");
    assert_eq!(liste[0]["engagement"]["remuneration"]["montant"], 60_000);
    assert_eq!(liste[1]["engagement"]["statut"], "en_cours");
    // Un remplacement qui ne commence pas après l'engagement remplacé : refusé.
    let mut trop_tot = mensuel(
        &gardien,
        "duree_indeterminee",
        "2026-11-01",
        Value::Null,
        70_000,
    );
    trop_tot["remplace"] = nouveau["id"].clone();
    assert_eq!(
        p.refus("engagement_conclure", Some(&admin), trop_tot)
            .await
            .cle,
        "engagement.remplacement_trop_tot"
    );

    // Une secrétaire à durée déterminée qui finit dans moins de soixante jours : signalée.
    let secretaire = membre(&p, &admin, "Coulibaly", "administration", "Secrétaire").await;
    assert_eq!(
        p.refus(
            "engagement_conclure",
            Some(&admin),
            mensuel(
                &secretaire,
                "duree_determinee",
                "2026-01-01",
                Value::Null,
                80_000
            ),
        )
        .await
        .cle,
        "engagement.fin_obligatoire"
    );
    let cdd = p
        .ok(
            "engagement_conclure",
            Some(&admin),
            mensuel(
                &secretaire,
                "duree_determinee",
                "2026-01-01",
                json!("2026-11-30"),
                80_000,
            ),
        )
        .await;
    let echeances = p
        .ok("engagements_a_echeance", Some(&admin), Value::Null)
        .await;
    let echeances = echeances.as_array().unwrap();
    assert_eq!(echeances.len(), 1);
    assert_eq!(echeances[0]["engagement"]["id"], cdd["id"]);
    assert_eq!(echeances[0]["membre"], "Bakary Coulibaly");
    // Terminée avant son terme : un motif est exigé.
    assert_eq!(
        p.refus(
            "engagement_terminer",
            Some(&admin),
            json!({ "id": cdd["id"], "le": "2026-10-31", "motif": "" }),
        )
        .await
        .cle,
        "engagement.motif_obligatoire"
    );
    let termine = p
        .ok(
            "engagement_terminer",
            Some(&admin),
            json!({ "id": cdd["id"], "le": "2026-10-31", "motif": "Départ souhaité" }),
        )
        .await;
    assert_eq!(termine["statut"], "termine");
    assert_eq!(
        p.refus(
            "engagement_rompre",
            Some(&admin),
            json!({ "id": cdd["id"], "le": "2026-10-31", "motif": "Faute grave" }),
        )
        .await
        .cle,
        "erreur.transition_interdite"
    );
    assert!(
        p.ok("engagements_a_echeance", Some(&admin), Value::Null)
            .await
            .as_array()
            .unwrap()
            .is_empty()
    );

    // La vacation se renouvelle pour l'année suivante, aux mêmes conditions.
    p.ok(
        "annee_creer",
        Some(&admin),
        json!({ "libelle": "2027-2028", "debut": RENTREE_2027, "fin": FIN_2028 }),
    )
    .await;
    let proposition = p
        .ok(
            "engagement_renouvellement",
            Some(&admin),
            json!({ "id": v["id"] }),
        )
        .await;
    assert_eq!(proposition["debut"], "2027-10-01");
    assert_eq!(proposition["remuneration"]["taux"], 2_500);
    let mut renouvelee = proposition.clone();
    renouvelee["remplace"] = Value::Null;
    let renouvelee = p.ok("engagement_conclure", Some(&admin), renouvelee).await;
    assert_eq!(renouvelee["precedent_id"], v["id"]);
    let liste = p
        .ok(
            "engagements_lister",
            Some(&admin),
            json!({ "id": prof["id"] }),
        )
        .await;
    assert_eq!(liste[0]["renouvele"], true);
    assert_eq!(liste[1]["renouvele"], false);

    // La lettre d'engagement s'imprime.
    let lettre = p
        .ok(
            "lettre_engagement_produire",
            Some(&admin),
            json!({ "id": v["id"] }),
        )
        .await;
    assert!(lettre["pdf"].as_str().unwrap().starts_with("JVBER"));

    let verification = p.ok("audit_verifier", Some(&admin), Value::Null).await;
    assert!(verification["lignes_verifiees"].as_u64().unwrap() > 0);
}
