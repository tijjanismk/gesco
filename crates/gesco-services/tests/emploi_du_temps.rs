//! Emploi du temps par la frontière (SPEC_PHASE_8 §8, T-38, T-64) : la journée proposée du 1er
//! cycle, une séance placée dans la journée de sa classe, une autre hors de la journée refusée ;
//! un maître à deux endroits à la fois et une séance sur la récréation signalés, jamais refusés ;
//! l'horaire de canicule ; l'impression par classe et par enseignant.

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

fn natures(e: &Value) -> Vec<String> {
    let mut n: Vec<String> = e["conflits"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["nature"].as_str().unwrap().to_owned())
        .collect();
    n.sort();
    n
}

#[tokio::test]
async fn les_seances_se_placent_dans_la_journee_et_les_conflits_se_signalent() {
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
    let ped = p
        .ok("pedagogie_lire", Some(&admin), json!({ "id": annee["id"] }))
        .await;
    let mut classes = Vec::new();
    for (code, libelle) in [("1", "1re A"), ("2", "2e A")] {
        classes.push(
            p.ok(
                "classe_creer",
                Some(&admin),
                json!({ "annee_id": annee["id"], "niveau_id": id_par(&ped["niveaux"], "code", code),
                    "serie_id": null, "libelle": libelle }),
            )
            .await["id"]
                .clone(),
        );
    }
    let moussa = p
        .ok(
            "enseignant_enregistrer",
            Some(&admin),
            json!({ "id": null, "nom": "Diarra", "prenoms": "Moussa", "telephones": [],
                "statut": "titulaire", "utilisateur_id": null, "actif": true }),
        )
        .await["enseignant"]["id"]
        .clone();
    p.ok(
        "maitre_affecter",
        Some(&admin),
        json!({ "classe_id": classes[0], "enseignant_id": moussa }),
    )
    .await;

    // La journée proposée du 1er cycle ; les matières de la 1re A, confiées à son maître.
    let e = p
        .ok("emploi_du_temps_lire", Some(&admin), Value::Null)
        .await;
    assert_eq!(e["gere"], true);
    assert_eq!(e["journees"].as_array().unwrap().len(), 1);
    assert_eq!(e["journees"][0]["cycle"], "premier_cycle");
    let premiere = e["classes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["libelle"] == "1re A")
        .unwrap()
        .clone();
    let (m1, m2) = (
        premiere["matieres"][0]["matiere_id"].clone(),
        premiere["matieres"][1]["matiere_id"].clone(),
    );
    assert_eq!(premiere["matieres"][0]["enseignant_id"], moussa);
    let seance = |classe: &Value, jour: u8, debut: u16, fin: u16, matiere: &Value| {
        json!({ "id": null, "classe_id": classe, "jour": jour, "debut": debut, "fin": fin,
            "matiere_id": matiere, "enseignant_id": moussa, "salle": "" })
    };
    let e = p
        .ok(
            "seance_placer",
            Some(&admin),
            seance(&classes[0], 1, 8 * 60, 9 * 60, &m1),
        )
        .await;
    assert!(natures(&e).is_empty());
    // Le mercredi après-midi, il n'y a pas classe.
    assert_eq!(
        p.refus(
            "seance_placer",
            Some(&admin),
            seance(&classes[0], 3, 15 * 60, 16 * 60, &m1)
        )
        .await
        .cle,
        "seance.hors_journee"
    );
    // Le maître placé en 2e A à la même heure, une séance sur la récréation : signalés.
    let matiere_2e = e["classes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["libelle"] == "2e A")
        .unwrap()["matieres"][0]["matiere_id"]
        .clone();
    let e = p
        .ok(
            "seance_placer",
            Some(&admin),
            seance(&classes[1], 1, 8 * 60 + 30, 9 * 60 + 30, &matiere_2e),
        )
        .await;
    assert_eq!(natures(&e), vec!["enseignant", "enseignant"]);
    let e = p
        .ok(
            "seance_placer",
            Some(&admin),
            seance(&classes[0], 1, 9 * 60 + 45, 10 * 60 + 30, &m2),
        )
        .await;
    assert_eq!(natures(&e), vec!["enseignant", "enseignant", "recreation"]);
    let en_2e = e["seances"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["classe"] == "2e A")
        .unwrap()["seance"]["id"]
        .clone();
    let e = p
        .ok("seance_retirer", Some(&admin), json!({ "id": en_2e }))
        .await;
    assert_eq!(natures(&e), vec!["recreation"]);
    assert_eq!(e["seances"].as_array().unwrap().len(), 2);

    // L'horaire de canicule du 1er cycle, d'avril à fin mai, sans après-midi.
    let mut journee = e["journees"][0].clone();
    journee["canicule"] = json!({ "du": "2027-04-01", "au": "2027-05-31",
        "matin": { "debut": 450, "fin": 720 } });
    let e = p.ok("journee_regler", Some(&admin), journee.clone()).await;
    assert_eq!(e["journees"][0]["canicule"]["matin"]["debut"], 450);
    journee["jours"] = json!([]);
    assert_eq!(
        p.refus("journee_regler", Some(&admin), journee).await.cle,
        "journee.jours_invalides"
    );

    // Vacataire, Moussa reçoit ses heures prévues d'octobre d'après ses séances : les lundis 5,
    // 12, 19 et 26, 1 h 45 chaque fois (8 h–9 h et 9 h 45–10 h 30).
    let registre = p.ok("personnel_lister", Some(&admin), Value::Null).await;
    let membre = registre
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["nom"] == "Diarra")
        .unwrap()["membre"]["id"]
        .clone();
    p.ok(
        "engagement_conclure",
        Some(&admin),
        json!({ "membre_id": membre, "nature": "vacation", "debut": "2026-10-01", "fin": null,
            "fonction": "", "remuneration": { "type": "horaire", "taux": 2_500,
            "minutes_par_semaine": 360 }, "note": "", "remplace": null, "renouvelle": null,
            "paye_douze_mois": null }),
    )
    .await;
    let heures = p
        .ok("heures_lire", Some(&admin), json!({ "mois": "2026-10" }))
        .await;
    assert_eq!(heures["lignes"][0]["minutes_prevues"], 4 * 105);
    assert_eq!(heures["lignes"][0]["prevues_emploi_du_temps"], true);

    // L'emploi du temps imprimé de la classe et celui du maître.
    let doc = p
        .ok(
            "emploi_classe_produire",
            Some(&admin),
            json!({ "classe_id": classes[0] }),
        )
        .await;
    assert!(doc["pdf"].as_str().unwrap().len() > 1_000);
    deposer(&doc, "emploi-classe.pdf");
    let doc = p
        .ok(
            "emploi_enseignant_produire",
            Some(&admin),
            json!({ "enseignant_id": moussa }),
        )
        .await;
    assert!(doc["pdf"].as_str().unwrap().len() > 1_000);
    deposer(&doc, "emploi-enseignant.pdf");
}

/// Dépose un document produit dans le dossier `GESCO_ECRIRE_DOCUMENTS`, pour le relire à l'œil.
fn deposer(document: &Value, nom: &str) {
    if let Some(dossier) = std::env::var_os("GESCO_ECRIRE_DOCUMENTS") {
        use base64::Engine;
        let pdf = base64::engine::general_purpose::STANDARD
            .decode(document["pdf"].as_str().unwrap())
            .unwrap();
        std::fs::write(std::path::Path::new(&dossier).join(nom), pdf).unwrap();
    }
}
