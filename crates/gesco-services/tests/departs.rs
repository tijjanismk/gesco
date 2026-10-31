//! Départs et exclusions par la frontière (SPEC_PHASE_4 §9 ; réponse du 13 octobre 2026) :
//! transfert vers une autre école avec certificat numéroté et relevé, impayé qui avertit ou qui
//! bloque selon l'école ; exclusion définitive puis réintégration, échéances rétablies ;
//! exclusion temporaire qui tient l'élève hors du relevé des absences.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use crate::commun::*;
use serde_json::{Value, json};

fn id_par(liste: &Value, champ: &str, valeur: &str) -> Value {
    liste
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x[champ] == valeur)
        .unwrap_or_else(|| panic!("{champ} = {valeur} absent"))["id"]
        .clone()
}

/// Élève complet inscrit en 7e A : (fiche, inscription).
async fn inscrire(
    p: &Poste_,
    admin: &gesco_services::session::Jeton,
    classe: &Value,
    prenoms: &str,
) -> (Value, Value) {
    let fiche = p
        .ok(
            "eleve_creer",
            Some(admin),
            json!({ "identite": { "nom": "Traoré", "prenoms": prenoms, "sexe": "feminin",
                "date_naissance": "2014-03-02", "date_naissance_estimee": false,
                "lieu_naissance": "Bamako", "nationalite": "Malienne", "adresse": "", "telephones": [] },
                "matricule_interne": "", "matricule_national": "", "acte_nature": "acte",
                "acte_numero": "", "acte_date": "", "ecole_origine": "" }),
        )
        .await;
    p.ok(
        "responsable_rattacher",
        Some(admin),
        json!({ "eleve_id": fiche["eleve"]["id"], "responsable_id": null,
            "saisie": { "identite": { "nom": "Coulibaly", "prenoms": format!("Mère de {prenoms}"), "sexe": "feminin",
                "date_naissance": "", "date_naissance_estimee": false, "lieu_naissance": "", "nationalite": "",
                "adresse": "", "telephones": ["76 12 34 56"] }, "profession": "", "employeur": "" },
            "qualites": { "lien": "mere", "est_responsable_legal": true, "est_payeur": true,
                "recoit_notifications": true, "vient_chercher": false, "ordre_priorite": 1 } }),
    )
    .await;
    let inscription = p
        .ok(
            "inscription_creer",
            Some(admin),
            json!({ "eleve_id": fiche["eleve"]["id"], "classe_id": classe, "date_inscription": "",
                "redoublant": false, "etablissement_origine": "" }),
        )
        .await["inscription"]["id"]
        .clone();
    p.ok(
        "inscription_valider",
        Some(admin),
        json!({ "id": inscription, "choix": {} }),
    )
    .await;
    (fiche, inscription)
}

async fn echeances_dues(
    p: &Poste_,
    admin: &gesco_services::session::Jeton,
    fiche: &Value,
) -> usize {
    p.ok(
        "situation_financiere",
        Some(admin),
        json!({ "id": fiche["eleve"]["id"] }),
    )
    .await[0]["facture"]["echeances"]
        .as_array()
        .unwrap()
        .len()
}

async fn statut(
    p: &Poste_,
    admin: &gesco_services::session::Jeton,
    annee: &Value,
    inscription: &Value,
) -> Value {
    p.ok(
        "inscriptions_lister",
        Some(admin),
        json!({ "annee_id": annee }),
    )
    .await
    .as_array()
    .unwrap()
    .iter()
    .find(|l| l["inscription"]["id"] == *inscription)
    .unwrap()["inscription"]["statut"]
        .clone()
}

#[tokio::test]
async fn departs_et_exclusions_d_une_classe() {
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
    poser_grille(&p, &admin, &annee["id"]).await;
    let ped = p
        .ok("pedagogie_lire", Some(&admin), json!({ "id": annee["id"] }))
        .await;
    let septieme = id_par(&ped["niveaux"], "code", "7");
    let classe = p
        .ok(
            "classe_creer",
            Some(&admin),
            json!({ "annee_id": annee["id"], "niveau_id": septieme, "serie_id": null, "libelle": "7e A" }),
        )
        .await["id"]
        .clone();
    let (_, awa) = inscrire(&p, &admin, &classe, "Awa").await;
    let (binta_fiche, binta) = inscrire(&p, &admin, &classe, "Binta").await;
    let (_, coumba) = inscrire(&p, &admin, &classe, "Coumba").await;
    let (_, djeneba) = inscrire(&p, &admin, &classe, "Djeneba").await;
    // Nous sommes le 10 janvier 2027 ; rien n'a été payé.
    p.horloge.avancer(98 * 86_400_000);
    let admin = p.connecter("admin", MOT_DE_PASSE).await;

    // Awa part pour le lycée de Kati : certificat numéroté, échéances à venir annulées,
    // l'impayé échu avertit sans bloquer (Q-33).
    let transfert = |inscription: &Value, passer_outre: &str| {
        json!({ "inscription_id": inscription, "date_depart": "2027-01-10",
            "etablissement_destination": "Lycée de Kati", "motif": "Déménagement de la famille",
            "passer_outre": passer_outre })
    };
    let t = p
        .ok("inscription_transferer", Some(&admin), transfert(&awa, ""))
        .await;
    assert_eq!(t["inscription"]["statut"], "transferee");
    assert_eq!(
        t["inscription"]["etablissement_destination"],
        "Lycée de Kati"
    );
    assert_eq!(t["numero_certificat"], "CTR-2026-000001");
    assert!(t["reste_du"].as_i64().unwrap() > 0);
    assert_eq!(t["avertissements"], json!(["transfert.impaye"]));
    assert!(t["echeances_annulees"].as_u64().unwrap() >= 1);
    p.refus("inscription_transferer", Some(&admin), transfert(&awa, ""))
        .await;
    let certificat = p
        .ok(
            "certificat_transfert_produire",
            Some(&admin),
            json!({ "inscription_id": awa }),
        )
        .await;
    assert_eq!(certificat["document"]["modele_id"], "certificat_transfert");
    assert!(certificat["pdf"].as_str().unwrap().len() > 1000);
    let releve = p
        .ok(
            "releve_moyennes_produire",
            Some(&admin),
            json!({ "inscription_id": awa }),
        )
        .await;
    assert_eq!(releve["document"]["modele_id"], "releve_moyennes");

    // L'école exige un solde nul : le transfert de Djeneba est refusé, sauf décision motivée.
    p.ok(
        "parametre_definir",
        Some(&admin),
        json!({ "cle": "transfert.solde_nul_exige", "valeur": { "type": "booleen", "valeur": true } }),
    )
    .await;
    assert_eq!(
        p.refus(
            "inscription_transferer",
            Some(&admin),
            transfert(&djeneba, "")
        )
        .await
        .cle,
        "transfert.solde_non_nul"
    );
    let t = p
        .ok(
            "inscription_transferer",
            Some(&admin),
            transfert(&djeneba, "Famille partie en urgence, dette reconnue"),
        )
        .await;
    assert_eq!(t["numero_certificat"], "CTR-2026-000002");

    // Binta est exclue définitivement, puis réintégrée sur recours : ses échéances reviennent.
    let dues_avant = echeances_dues(&p, &admin, &binta_fiche).await;
    let exclusion = p
        .ok(
            "inscription_exclure",
            Some(&admin),
            json!({ "inscription_id": binta, "nature": "definitive", "du": "2027-01-10", "au": null,
                "motif": "Violence envers un camarade", "decision": "Conseil de discipline du 8 janvier 2027" }),
        )
        .await;
    assert_eq!(exclusion["statut"], "en_cours");
    assert_eq!(statut(&p, &admin, &annee["id"], &binta).await, "radiee");
    let annulees = exclusion["echeances_annulees"].as_array().unwrap().len();
    assert!(annulees >= 1);
    assert_eq!(
        echeances_dues(&p, &admin, &binta_fiche).await,
        dues_avant - annulees
    );
    let revoquee = p
        .ok(
            "exclusion_lever",
            Some(&admin),
            json!({ "id": exclusion["id"], "motif": "Recours de la famille accepté" }),
        )
        .await;
    assert_eq!(revoquee["statut"], "revoquee");
    assert_eq!(statut(&p, &admin, &annee["id"], &binta).await, "validee");
    assert_eq!(echeances_dues(&p, &admin, &binta_fiche).await, dues_avant);
    let historique = p
        .ok(
            "exclusions_lister",
            Some(&admin),
            json!({ "inscription_id": binta }),
        )
        .await;
    assert_eq!(historique.as_array().unwrap().len(), 1);
    assert_eq!(
        historique[0]["motif_levee"],
        "Recours de la famille accepté"
    );

    // Coumba est exclue trois jours : elle n'est pas comptée absente ces jours-là.
    for (demande, cle) in [
        (
            json!({ "inscription_id": coumba, "nature": "temporaire", "du": "2027-01-07", "au": null,
                "motif": "Insolence", "decision": "" }),
            "exclusion.retour_obligatoire",
        ),
        (
            json!({ "inscription_id": coumba, "nature": "temporaire", "du": "2027-01-07", "au": "2027-03-01",
                "motif": "Insolence", "decision": "" }),
            "exclusion.trop_longue",
        ),
    ] {
        assert_eq!(
            p.refus("inscription_exclure", Some(&admin), demande)
                .await
                .cle,
            cle
        );
    }
    let temporaire = p
        .ok(
            "inscription_exclure",
            Some(&admin),
            json!({ "inscription_id": coumba, "nature": "temporaire", "du": "2027-01-07", "au": "2027-01-11",
                "motif": "Insolence", "decision": "Décision du directeur" }),
        )
        .await;
    assert_eq!(statut(&p, &admin, &annee["id"], &coumba).await, "validee");
    let releve = p
        .ok(
            "absences_releve_lire",
            Some(&admin),
            json!({ "classe_id": classe, "date": "2027-01-08" }),
        )
        .await;
    let ligne = releve["lignes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["inscription_id"] == coumba)
        .unwrap()
        .clone();
    assert_eq!(ligne["exclu"], true);
    // Les élèves partis ne sont plus sur le relevé ; Binta, réintégrée, y revient.
    assert!(
        !releve["lignes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|l| l["inscription_id"] == awa)
    );
    assert!(
        releve["lignes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|l| l["inscription_id"] == binta)
    );
    assert_eq!(
        p.refus(
            "absences_releve_enregistrer",
            Some(&admin),
            json!({ "classe_id": classe, "date": "2027-01-08",
                "lignes": [{ "inscription_id": coumba, "matin": "a", "apres_midi": "" }] })
        )
        .await
        .cle,
        "absence.eleve_exclu"
    );
    // Levée le 10 : elle revient en classe dès ce jour.
    let levee = p
        .ok(
            "exclusion_lever",
            Some(&admin),
            json!({ "id": temporaire["id"], "motif": "Excuses présentées" }),
        )
        .await;
    assert_eq!(levee["statut"], "levee");
    assert_eq!(levee["leve_le"], "2027-01-10");

    // Le tableau de bord compte les élèves restés : Binta, réintégrée, et Coumba.
    let tableau = p
        .ok("tableau_de_bord_lire", Some(&admin), json!(null))
        .await;
    assert_eq!(tableau["annee"], "2026-2027");
    assert_eq!(tableau["effectifs"]["total"], 2);
    assert_eq!(tableau["effectifs"]["filles"], 2);
    assert_eq!(tableau["effectifs"]["par_classe"][0]["classe"], "7e A");
    assert_eq!(tableau["scolarite"]["exclusions_en_cours"], 0);
    assert!(tableau["finances"]["reste_echu"].as_i64().unwrap() > 0);
}
