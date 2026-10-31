//! Absences et congés du personnel par la frontière (SPEC_PHASE_6 §5 et §6, T-43, T-44) : un
//! congé se demande, s'accorde ou se refuse avec un motif, s'annule avant son début ; une absence
//! pendant un congé accordé est refusée ; le décompte, quand l'école l'active, signale un congé
//! annuel au-delà du solde sans le bloquer.

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

async fn secretaire(p: &Poste_, admin: &Jeton, arrive_le: &str) -> Value {
    let referentiel = p
        .ok("personnel_referentiel_lire", Some(admin), Value::Null)
        .await;
    let administration = referentiel["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["code"] == "administration")
        .unwrap()["id"]
        .clone();
    p.ok(
        "membre_enregistrer",
        Some(admin),
        json!({
            "id": null, "nom": "Kanté", "prenoms": "Aminata", "sexe": "feminin",
            "date_naissance": null, "telephones": [], "adresse": "", "matricule": "",
            "categorie_id": administration, "fonction": "Secrétaire", "arrive_le": arrive_le,
            "fonctionnaire": false, "numero_fonctionnaire": "", "numero_inps": "",
            "prevenir_nom": "", "prevenir_telephone": "", "paiement": { "mode": "especes" },
            "diplomes": [], "utilisateur_id": null,
        }),
    )
    .await["membre"]
        .clone()
}

fn type_de(etat: &Value, code: &str) -> Value {
    etat["types"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["code"] == code)
        .unwrap()["id"]
        .clone()
}

fn conge(membre: &Value, type_id: &Value, debut: &str, fin: &str) -> Value {
    json!({ "membre_id": membre["id"], "type_id": type_id, "debut": debut, "fin": fin,
        "motif": "Mariage de sa sœur à Kayes" })
}

fn absence(membre: &Value, date: &str, nature: &str, minutes: Option<i64>) -> Value {
    json!({ "membre_id": membre["id"], "date": date, "nature": nature, "minutes": minutes,
        "motif": "", "justifiee": false })
}

#[tokio::test]
async fn les_conges_se_demandent_s_accordent_et_couvrent_les_absences() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let aminata = secretaire(&p, &admin, "2026-06-01").await;

    // Types livrés : le congé annuel se déduit du solde, la permission non.
    let etat = p
        .ok(
            "absences_conges_lire",
            Some(&admin),
            json!({ "mois": "2026-10" }),
        )
        .await;
    assert_eq!(etat["types"].as_array().unwrap().len(), 5);
    assert_eq!(etat["decompte_actif"], false);
    let permission = type_de(&etat, "permission");
    let annuel = type_de(&etat, "annuel");

    // Une permission du lundi 19 au vendredi 23 octobre : cinq jours.
    let demande = p
        .ok(
            "conge_demander",
            Some(&admin),
            conge(&aminata, &permission, "2026-10-19", "2026-10-25"),
        )
        .await;
    assert_eq!(demande["jours"], 5);
    assert_eq!(demande["statut"], "demande");
    assert_eq!(
        p.refus(
            "conge_demander",
            Some(&admin),
            conge(&aminata, &annuel, "2026-10-23", "2026-10-27")
        )
        .await
        .cle,
        "conge.chevauchement"
    );
    assert_eq!(
        p.refus(
            "conge_demander",
            Some(&admin),
            conge(&aminata, &annuel, "2026-10-31", "2026-11-01")
        )
        .await
        .cle,
        "conge.aucun_jour"
    );
    assert_eq!(
        p.refus(
            "conge_refuser",
            Some(&admin),
            json!({ "id": demande["id"], "motif": " " })
        )
        .await
        .cle,
        "conge.motif_obligatoire"
    );
    let accorde = p
        .ok(
            "conge_accorder",
            Some(&admin),
            json!({ "id": demande["id"] }),
        )
        .await;
    assert_eq!(accorde["statut"], "accorde");
    assert_ne!(accorde["decide_par"], Value::Null);

    // Le 22 octobre : une absence pendant le congé est refusée, le congé suffit.
    p.horloge.avancer(18 * JOUR);
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    assert_eq!(
        p.refus(
            "absence_personnel_noter",
            Some(&admin),
            absence(&aminata, "2026-10-21", "journee", None)
        )
        .await
        .cle,
        "absence_personnel.en_conge"
    );
    assert_eq!(
        p.refus(
            "absence_personnel_noter",
            Some(&admin),
            absence(&aminata, "2026-10-19", "journee", None)
        )
        .await
        .cle,
        "absence_personnel.en_conge"
    );
    assert_eq!(
        p.refus(
            "absence_personnel_noter",
            Some(&admin),
            absence(&aminata, "2026-10-26", "journee", None)
        )
        .await
        .cle,
        "absence_personnel.date_future"
    );
    // Un retard, noté puis annulé avec un motif ; le jour se note de nouveau.
    let retard = p
        .ok(
            "absence_personnel_noter",
            Some(&admin),
            absence(&aminata, "2026-10-15", "retard", Some(20)),
        )
        .await;
    assert_eq!(
        p.refus(
            "absence_personnel_noter",
            Some(&admin),
            absence(&aminata, "2026-10-15", "demi_journee", None)
        )
        .await
        .cle,
        "absence_personnel.deja_notee"
    );
    assert_eq!(
        p.refus(
            "absence_personnel_annuler",
            Some(&admin),
            json!({ "id": retard["id"], "motif": "" })
        )
        .await
        .cle,
        "absence_personnel.motif_obligatoire"
    );
    let annule = p
        .ok(
            "absence_personnel_annuler",
            Some(&admin),
            json!({ "id": retard["id"], "motif": "Notée sur la mauvaise fiche" }),
        )
        .await;
    assert_eq!(annule["annulee"], true);
    let demi = p
        .ok(
            "absence_personnel_noter",
            Some(&admin),
            absence(&aminata, "2026-10-15", "demi_journee", None),
        )
        .await;
    // Justifiée après coup.
    let justifiee = p
        .ok(
            "absence_personnel_justifier",
            Some(&admin),
            json!({ "id": demi["id"], "motif": "Enfant malade" }),
        )
        .await;
    assert_eq!(justifiee["justifiee"], true);
    assert_eq!(justifiee["motif"], "Enfant malade");
    let etat = p
        .ok(
            "absences_conges_lire",
            Some(&admin),
            json!({ "mois": "2026-10" }),
        )
        .await;
    assert_eq!(etat["absences"].as_array().unwrap().len(), 2);
    assert_eq!(etat["absences"][0]["membre"], "Aminata Kanté");
    assert_eq!(etat["conges"][0]["type_libelle"], "Permission d'absence");

    // Un congé commencé ne s'annule plus ; un congé à venir, si.
    assert_eq!(
        p.refus(
            "conge_annuler",
            Some(&admin),
            json!({ "id": demande["id"], "motif": "Retour anticipé" })
        )
        .await
        .cle,
        "conge.deja_commence"
    );
    let novembre = p
        .ok(
            "conge_demander",
            Some(&admin),
            conge(&aminata, &annuel, "2026-11-02", "2026-11-06"),
        )
        .await;
    p.ok(
        "conge_accorder",
        Some(&admin),
        json!({ "id": novembre["id"] }),
    )
    .await;
    let annule = p
        .ok(
            "conge_annuler",
            Some(&admin),
            json!({ "id": novembre["id"], "motif": "Examens reportés" }),
        )
        .await;
    assert_eq!(annule["statut"], "annule");
    assert_eq!(
        p.refus(
            "conge_accorder",
            Some(&admin),
            json!({ "id": novembre["id"] })
        )
        .await
        .cle,
        "erreur.transition_interdite"
    );

    // Le décompte activé : arrivée le 1er juin, six mois entiers au 7 décembre, quinze jours ;
    // dix-neuf jours demandés, le dépassement est signalé sans bloquer.
    p.ok(
        "parametre_definir",
        Some(&admin),
        json!({ "cle": "conge.decompte_actif", "valeur": { "type": "booleen", "valeur": true } }),
    )
    .await;
    let decembre = p
        .ok(
            "conge_demander",
            Some(&admin),
            conge(&aminata, &annuel, "2026-12-07", "2026-12-31"),
        )
        .await;
    assert_eq!(decembre["jours"], 19);
    let etat = p
        .ok(
            "absences_conges_lire",
            Some(&admin),
            json!({ "mois": "2026-12" }),
        )
        .await;
    assert_eq!(etat["decompte_actif"], true);
    let ligne = &etat["conges"][0];
    assert_eq!(ligne["conge"]["id"], decembre["id"]);
    assert_eq!(ligne["solde"], 30);
    assert_eq!(ligne["depasse_solde"], true);
    p.ok(
        "conge_accorder",
        Some(&admin),
        json!({ "id": decembre["id"] }),
    )
    .await;

    // Le samedi compté, par réglage de l'école.
    p.ok(
        "parametre_definir",
        Some(&admin),
        json!({ "cle": "conge.samedi_compte", "valeur": { "type": "booleen", "valeur": true } }),
    )
    .await;
    let samedi = p
        .ok(
            "conge_demander",
            Some(&admin),
            conge(&aminata, &permission, "2027-01-04", "2027-01-10"),
        )
        .await;
    assert_eq!(samedi["jours"], 6);

    // Un type ajouté, puis retiré : il ne se demande plus.
    let examen = p
        .ok(
            "type_conge_enregistrer",
            Some(&admin),
            json!({ "id": null, "libelle": "Congé d'examen", "actif": true, "deduit_du_solde": false }),
        )
        .await;
    p.ok(
        "type_conge_enregistrer",
        Some(&admin),
        json!({ "id": examen["id"], "libelle": "Congé d'examen", "actif": false, "deduit_du_solde": false }),
    )
    .await;
    assert_eq!(
        p.refus(
            "conge_demander",
            Some(&admin),
            conge(&aminata, &examen["id"], "2027-02-01", "2027-02-05")
        )
        .await
        .cle,
        "conge.type_retire"
    );

    let verification = p.ok("audit_verifier", Some(&admin), Value::Null).await;
    assert!(verification["lignes_verifiees"].as_u64().unwrap() > 0);
}
