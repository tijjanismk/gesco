//! Paie par la frontière (SPEC_PHASE_7) : éléments livrés, retenues du Mali inactives aux taux
//! proposés ; éléments de l'école et versions datées ; attributions ; réglages des mois payés et
//! du prorata ; mois payés d'un engagement ; situation de famille.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use crate::commun::*;
use gesco_services::session::Jeton;
use serde_json::{Value, json};

fn element<'a>(etat: &'a Value, code: &str) -> &'a Value {
    etat["elements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["element"]["code"] == code)
        .unwrap_or_else(|| panic!("{code} absent"))
}

fn par_libelle<'a>(etat: &'a Value, libelle: &str) -> &'a Value {
    etat["elements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["element"]["libelle"] == libelle)
        .unwrap_or_else(|| panic!("{libelle} absent"))
}

async fn secretaire(p: &Poste_, admin: &Jeton, enfants: Value) -> Value {
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
            "categorie_id": administration, "fonction": "Secrétaire", "arrive_le": "2025-10-01",
            "fonctionnaire": false, "numero_fonctionnaire": "", "numero_inps": "",
            "prevenir_nom": "", "prevenir_telephone": "", "paiement": { "mode": "especes" },
            "diplomes": [], "utilisateur_id": null, "situation_famille": "marie",
            "enfants_a_charge": enfants,
        }),
    )
    .await["membre"]
        .clone()
}

#[tokio::test]
async fn les_elements_de_paie_se_reglent_et_se_datent() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;

    // Livrés : les éléments saisis, actifs ; les retenues du Mali, inactives, aux taux proposés.
    let etat = p.ok("elements_paie_lire", Some(&admin), Value::Null).await;
    assert_eq!(etat["mois"], "2026-10");
    assert_eq!(etat["elements"].as_array().unwrap().len(), 12);
    assert_eq!(element(&etat, "PRIME")["element"]["actif"], true);
    let retraite = element(&etat, "INPS_RET");
    assert_eq!(retraite["element"]["actif"], false);
    assert_eq!(retraite["element"]["fonctionnaires"], false);
    assert_eq!(
        retraite["en_vigueur"]["calcul"],
        json!({ "mode": "taux", "taux": 360, "plafond": null })
    );
    let its = element(&etat, "ITS");
    let bareme = &its["en_vigueur"]["calcul"];
    assert_eq!(bareme["mode"], "bareme");
    assert_eq!(bareme["annuel"], true);
    assert_eq!(bareme["tranches"].as_array().unwrap().len(), 7);
    assert_eq!(bareme["reduction_points"], 200);
    assert_eq!(
        bareme["famille"],
        json!({ "marie": 1000, "par_enfant": 250, "plafond": 3500 })
    );
    assert_eq!(
        its["en_vigueur"]["assiette_deduit"],
        json!([retraite["element"]["id"]])
    );
    assert_eq!(
        element(&etat, "INPS_PF")["element"]["nature"],
        "cotisation_patronale"
    );
    assert_eq!(
        etat["reglages"],
        json!({ "douze_mois": true, "mois_vacances": [7, 8, 9], "prorata": "trentieme" })
    );

    // L'école active la retraite, puis lui donne un nouveau taux en janvier ; celui d'octobre
    // reste en vigueur ce mois-ci.
    let id_retraite = retraite["element"]["id"].clone();
    let natures = json!([
        "duree_indeterminee",
        "duree_determinee",
        "vacation",
        "stage"
    ]);
    p.ok(
        "element_paie_enregistrer",
        Some(&admin),
        json!({ "id": id_retraite, "libelle": "Retraite INPS", "nature": "retenue",
            "natures": natures, "fonctionnaires": false, "actif": true, "calcul": null }),
    )
    .await;
    let etat = p
        .ok(
            "element_paie_version_enregistrer",
            Some(&admin),
            json!({ "element_id": id_retraite, "effet": "2027-01",
                "calcul": { "mode": "taux", "taux": 400, "plafond": null }, "assiette_deduit": [] }),
        )
        .await;
    let retraite = element(&etat, "INPS_RET");
    assert_eq!(retraite["element"]["actif"], true);
    assert_eq!(retraite["versions"].as_array().unwrap().len(), 2);
    assert_eq!(retraite["en_vigueur"]["calcul"]["taux"], 360);

    // Un élément de l'école : une prime de transport fixe, aux permanents seulement.
    let etat = p
        .ok(
            "element_paie_enregistrer",
            Some(&admin),
            json!({ "id": null, "libelle": "Prime de transport", "nature": "gain",
                "natures": ["duree_indeterminee", "duree_determinee"], "fonctionnaires": true,
                "actif": true, "calcul": { "mode": "fixe", "montant": 10_000 } }),
        )
        .await;
    let transport = par_libelle(&etat, "Prime de transport");
    assert_eq!(transport["en_vigueur"]["effet"], "2026-10");
    let id_transport = transport["element"]["id"].clone();
    let refus = |demande: Value| {
        let p = &p;
        let admin = &admin;
        async move {
            p.refus("element_paie_enregistrer", Some(admin), demande)
                .await
                .cle
        }
    };
    assert_eq!(
        refus(
            json!({ "id": null, "libelle": "prime de transport", "nature": "gain",
            "natures": natures, "fonctionnaires": true, "actif": true,
            "calcul": { "mode": "fixe", "montant": 5_000 } })
        )
        .await,
        "element.libelle_pris"
    );
    assert_eq!(
        refus(
            json!({ "id": null, "libelle": "Prime progressive", "nature": "gain",
            "natures": natures, "fonctionnaires": true, "actif": true,
            "calcul": { "mode": "bareme", "tranches": [{ "jusqua": null, "taux": 100 }],
                "annuel": false, "reduction_points": 0, "famille": null } })
        )
        .await,
        "element.bareme_retenue"
    );
    assert_eq!(
        refus(
            json!({ "id": null, "libelle": "Sans nature", "nature": "gain", "natures": [],
            "fonctionnaires": true, "actif": true, "calcul": { "mode": "saisi" } })
        )
        .await,
        "element.aucune_nature"
    );
    // L'assiette d'un impôt ne retranche que des retenues.
    assert_eq!(
        p.refus(
            "element_paie_version_enregistrer",
            Some(&admin),
            json!({ "element_id": element(&etat, "ITS")["element"]["id"], "effet": "2026-10",
                "calcul": { "mode": "taux", "taux": 1_000, "plafond": null },
                "assiette_deduit": [id_transport] })
        )
        .await
        .cle,
        "element.assiette_invalide"
    );

    // La situation de famille de la secrétaire, puis sa prime attribuée d'octobre à décembre.
    let aminata = secretaire(&p, &admin, json!(3)).await;
    let fiche = p
        .ok("membre_lire", Some(&admin), json!({ "id": aminata["id"] }))
        .await;
    assert_eq!(fiche["membre"]["situation_famille"], "marie");
    assert_eq!(fiche["membre"]["enfants_a_charge"], 3);
    let attribution = |element: &Value, fin: Value| {
        json!({ "id": null, "membre_id": aminata["id"], "element_id": element,
            "montant": 10_000, "debut": "2026-10", "fin": fin })
    };
    let etat = p
        .ok(
            "attribution_enregistrer",
            Some(&admin),
            attribution(&id_transport, json!("2026-12")),
        )
        .await;
    assert_eq!(etat["attributions"][0]["membre"], "Aminata Kanté");
    assert_eq!(etat["attributions"][0]["element"], "Prime de transport");
    for (demande, cle) in [
        (
            attribution(&element(&etat, "INPS_PF")["element"]["id"], Value::Null),
            "attribution.cotisation",
        ),
        (
            attribution(&id_transport, json!("2026-09")),
            "attribution.fin_avant_debut",
        ),
    ] {
        assert_eq!(
            p.refus("attribution_enregistrer", Some(&admin), demande)
                .await
                .cle,
            cle
        );
    }

    // Une école qui paie neuf mois, au prorata des jours ouvrables.
    let etat = p
        .ok(
            "paie_reglages_enregistrer",
            Some(&admin),
            json!({ "douze_mois": false, "mois_vacances": [8, 7], "prorata": "ouvrables" }),
        )
        .await;
    assert_eq!(
        etat["reglages"],
        json!({ "douze_mois": false, "mois_vacances": [7, 8], "prorata": "ouvrables" })
    );
    assert_eq!(
        p.refus(
            "paie_reglages_enregistrer",
            Some(&admin),
            json!({ "douze_mois": false, "mois_vacances": [13], "prorata": "ouvrables" })
        )
        .await
        .cle,
        "paie.reglage_invalide"
    );

    // Un engagement mensuel prend le réglage de l'école ; il se change ensuite.
    let cdi = p
        .ok(
            "engagement_conclure",
            Some(&admin),
            json!({ "membre_id": aminata["id"], "nature": "duree_indeterminee",
                "debut": "2026-10-01", "fin": null, "fonction": "",
                "remuneration": { "type": "mensuelle", "montant": 90_000 }, "note": "",
                "remplace": null, "renouvelle": null, "paye_douze_mois": null }),
        )
        .await;
    assert_eq!(cdi["paye_douze_mois"], false);
    let change = p
        .ok(
            "engagement_mois_payes",
            Some(&admin),
            json!({ "id": cdi["id"], "paye_douze_mois": true }),
        )
        .await;
    assert_eq!(change["paye_douze_mois"], true);

    // Pas plus de trente enfants à charge.
    let referentiel = p
        .ok("personnel_referentiel_lire", Some(&admin), Value::Null)
        .await;
    assert_eq!(
        p.refus(
            "membre_enregistrer",
            Some(&admin),
            json!({
                "id": null, "nom": "Traoré", "prenoms": "Moussa", "sexe": "masculin",
                "date_naissance": null, "telephones": [], "adresse": "", "matricule": "",
                "categorie_id": referentiel["categories"][0]["id"], "fonction": "Gardien",
                "arrive_le": null, "fonctionnaire": false, "numero_fonctionnaire": "",
                "numero_inps": "", "prevenir_nom": "", "prevenir_telephone": "",
                "paiement": { "mode": "especes" }, "diplomes": [], "utilisateur_id": null,
                "situation_famille": null, "enfants_a_charge": 31,
            })
        )
        .await
        .cle,
        "personnel.enfants_invalides"
    );

    let verification = p.ok("audit_verifier", Some(&admin), Value::Null).await;
    assert!(verification["lignes_verifiees"].as_u64().unwrap() > 0);
}
