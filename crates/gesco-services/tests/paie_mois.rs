//! Paie du mois par la frontière (SPEC_PHASE_7 §5) : calcul de tout le personnel payé, prime du
//! mois saisie puis retirée, absence non justifiée retenue sur confirmation, validation qui
//! numérote les bulletins et marque les heures payées, heures visées en retard payées en rappel
//! le mois suivant, cumuls de l'année.

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

async fn membre(
    p: &Poste_,
    admin: &Jeton,
    nom: &str,
    prenoms: &str,
    enseignant: bool,
    utilisateur: Value,
) -> Value {
    let referentiel = p
        .ok("personnel_referentiel_lire", Some(admin), Value::Null)
        .await;
    let code = if enseignant {
        "enseignement"
    } else {
        "administration"
    };
    let categorie = referentiel["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["code"] == code)
        .unwrap()["id"]
        .clone();
    p.ok(
        "membre_enregistrer",
        Some(admin),
        json!({
            "id": null, "nom": nom, "prenoms": prenoms, "sexe": "feminin",
            "date_naissance": null, "telephones": [], "adresse": "", "matricule": "",
            "categorie_id": categorie, "fonction": if enseignant { "Professeur" } else { "Secrétaire" },
            "arrive_le": "2025-10-01", "fonctionnaire": enseignant, "numero_fonctionnaire": "",
            "numero_inps": "", "prevenir_nom": "", "prevenir_telephone": "",
            "paiement": { "mode": "especes" }, "diplomes": [], "utilisateur_id": utilisateur,
            "situation_famille": null, "enfants_a_charge": null,
        }),
    )
    .await["membre"]
        .clone()
}

async fn vacation(p: &Poste_, admin: &Jeton, m: &Value) -> Value {
    p.ok(
        "engagement_conclure",
        Some(admin),
        json!({
            "membre_id": m["id"], "nature": "vacation", "debut": "2026-10-01", "fin": null,
            "fonction": "", "remuneration": { "type": "horaire", "taux": 2_500, "minutes_par_semaine": 360 },
            "note": "", "remplace": null, "renouvelle": null, "paye_douze_mois": null,
        }),
    )
    .await
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

fn bulletin<'a>(paie: &'a Value, membre: &str) -> &'a Value {
    &paie["bulletins"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["membre"] == membre)
        .unwrap_or_else(|| panic!("{membre} absent"))["bulletin"]
}

fn element_id(etat: &Value, code: &str) -> Value {
    etat["elements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["element"]["code"] == code)
        .unwrap()["element"]["id"]
        .clone()
}

#[tokio::test]
async fn la_paie_du_mois_se_calcule_se_revoit_et_se_valide() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;

    // La retraite INPS activée ; une secrétaire à 90 000 F, deux vacataires fonctionnaires.
    let etat = p.ok("elements_paie_lire", Some(&admin), Value::Null).await;
    let retraite = element_id(&etat, "INPS_RET");
    p.ok(
        "element_paie_enregistrer",
        Some(&admin),
        json!({ "id": retraite, "libelle": "Retraite INPS", "nature": "retenue",
            "natures": ["duree_indeterminee", "duree_determinee", "vacation", "stage"],
            "fonctionnaires": false, "actif": true, "calcul": null }),
    )
    .await;
    let prime = element_id(&etat, "PRIME");
    let aminata = membre(&p, &admin, "Kanté", "Aminata", false, Value::Null).await;
    p.ok(
        "engagement_conclure",
        Some(&admin),
        json!({ "membre_id": aminata["id"], "nature": "duree_indeterminee",
            "debut": "2026-10-01", "fin": null, "fonction": "",
            "remuneration": { "type": "mensuelle", "montant": 90_000 }, "note": "",
            "remplace": null, "renouvelle": null, "paye_douze_mois": null }),
    )
    .await;
    let sangare = membre(&p, &admin, "Sangaré", "Bakary", true, Value::Null).await;
    let vacation_sangare = vacation(&p, &admin, &sangare).await;
    let traore = membre(&p, &admin, "Traoré", "Moussa", true, Value::Null).await;
    let vacation_traore = vacation(&p, &admin, &traore).await;
    let heures = |engagement: &Value, mois: &str, faites: &str| {
        json!({ "engagement_id": engagement["id"], "mois": mois, "prevues": "24",
            "faites": faites, "observation": "" })
    };
    p.ok(
        "heures_saisir",
        Some(&admin),
        heures(&vacation_sangare, "2026-10", "22,5"),
    )
    .await;
    p.ok(
        "heures_viser",
        Some(&admin),
        json!({ "mois": "2026-10", "engagement_id": vacation_sangare["id"] }),
    )
    .await;

    // Octobre se calcule ; novembre, pas encore commencé, non.
    let octobre = json!({ "mois": "2026-10" });
    let paie = p.ok("paie_calculer", Some(&admin), octobre.clone()).await;
    assert_eq!(paie["paie"]["statut"], "calculee");
    assert_eq!(paie["bulletins"].as_array().unwrap().len(), 2);
    // 90 000 − 3,6 % de retraite ; Bakary, fonctionnaire : 22 h 30 × 2 500, sans retraite.
    assert_eq!(bulletin(&paie, "Aminata Kanté")["resultat"]["net"], 86_760);
    assert_eq!(bulletin(&paie, "Bakary Sangaré")["resultat"]["net"], 56_250);
    // Les heures de Moussa attendent leur visa : il n'est pas payé ce mois-ci.
    assert_eq!(paie["heures_en_saisie"], 0);
    assert_eq!(
        p.refus("paie_calculer", Some(&admin), json!({ "mois": "2026-11" }))
            .await
            .cle,
        "paie.mois_a_venir"
    );

    // Une absence non justifiée : proposée, retenue seulement sur confirmation (T-56).
    p.ok(
        "absence_personnel_noter",
        Some(&admin),
        json!({ "membre_id": aminata["id"], "date": "2026-10-02", "nature": "journee",
            "minutes": null, "motif": "", "justifiee": false }),
    )
    .await;
    let paie = p.ok("paie_mois_lire", Some(&admin), octobre.clone()).await;
    assert_eq!(paie["absences"].as_array().unwrap().len(), 1);
    assert_eq!(paie["absences"][0]["retenue"], Value::Null);
    assert_eq!(bulletin(&paie, "Aminata Kanté")["resultat"]["net"], 86_760);
    let absence = paie["absences"][0]["absence"]["id"].clone();
    let paie = p
        .ok(
            "paie_absence_confirmer",
            Some(&admin),
            json!({ "mois": "2026-10", "absence_id": absence, "retenue": true }),
        )
        .await;
    // Une journée au trentième : 3 000 F.
    assert_eq!(bulletin(&paie, "Aminata Kanté")["resultat"]["net"], 83_760);

    // Une prime d'examen du mois, puis retirée, puis saisie de nouveau.
    let saisir = |element: &Value, montant: i64, libelle: &str| {
        json!({ "mois": "2026-10", "membre_id": aminata["id"], "element_id": element,
            "montant": montant, "libelle": libelle })
    };
    let paie = p
        .ok(
            "paie_saisir",
            Some(&admin),
            saisir(&prime, 10_000, "Prime d'examen"),
        )
        .await;
    // Brut 100 000, retraite 3 600, absence 3 000.
    assert_eq!(
        bulletin(&paie, "Aminata Kanté")["resultat"]["brut"],
        100_000
    );
    assert_eq!(bulletin(&paie, "Aminata Kanté")["resultat"]["net"], 93_400);
    let saisie = paie["saisies"][0]["saisie"]["id"].clone();
    let paie = p
        .ok("paie_saisie_retirer", Some(&admin), json!({ "id": saisie }))
        .await;
    assert_eq!(bulletin(&paie, "Aminata Kanté")["resultat"]["net"], 83_760);
    assert!(paie["saisies"].as_array().unwrap().is_empty());
    for (element, montant, libelle, cle) in [
        (&retraite, 1_000, "Retenue", "paie.element_non_saisi"),
        (&prime, 0, "Prime", "paie.montant_nul"),
        (&prime, 5_000, " ", "paie.libelle_saisie"),
    ] {
        assert_eq!(
            p.refus(
                "paie_saisir",
                Some(&admin),
                saisir(element, montant, libelle)
            )
            .await
            .cle,
            cle
        );
    }
    p.ok(
        "paie_saisir",
        Some(&admin),
        saisir(&prime, 10_000, "Prime d'examen"),
    )
    .await;

    // Validation : numéros, heures payées ; la paie ne change plus.
    let paie = p.ok("paie_valider", Some(&admin), octobre.clone()).await;
    assert_eq!(paie["paie"]["statut"], "validee");
    assert_eq!(
        bulletin(&paie, "Aminata Kanté")["numero"],
        "PAI-2026-000001"
    );
    assert_eq!(
        bulletin(&paie, "Bakary Sangaré")["numero"],
        "PAI-2026-000002"
    );
    assert_eq!(paie["net"], 93_400 + 56_250);
    for (commande, entree) in [
        ("paie_calculer", octobre.clone()),
        ("paie_saisir", saisir(&prime, 5_000, "Oubli")),
        ("paie_valider", octobre.clone()),
    ] {
        assert_eq!(
            p.refus(commande, Some(&admin), entree).await.cle,
            "paie.validee",
            "{commande}"
        );
    }
    let etat = p
        .ok("heures_lire", Some(&admin), json!({ "mois": "2026-10" }))
        .await;
    let ligne_sangare = etat["lignes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["membre"] == "Bakary Sangaré")
        .unwrap()["id"]
        .clone();
    assert_eq!(
        p.refus(
            "heures_deverrouiller",
            Some(&admin),
            json!({ "id": ligne_sangare, "motif": "Erreur" })
        )
        .await
        .cle,
        "heures.payees"
    );

    // Les documents : bulletins du mois, celui d'Aminata seul, état de paie, état des retenues.
    for (commande, entree, nom) in [
        (
            "bulletins_paie_produire",
            octobre.clone(),
            "bulletins-paie.pdf",
        ),
        (
            "bulletin_paie_produire",
            json!({ "id": bulletin(&paie, "Aminata Kanté")["id"] }),
            "bulletin-paie.pdf",
        ),
        ("etat_paie_produire", octobre.clone(), "etat-paie.pdf"),
        (
            "etat_retenues_produire",
            octobre.clone(),
            "etat-retenues.pdf",
        ),
    ] {
        let document = p.ok(commande, Some(&admin), entree).await;
        assert!(document["pdf"].as_str().unwrap().len() > 1000, "{commande}");
        deposer(&document, nom);
    }
    // L'administrateur n'est pas du personnel : il n'a pas de bulletin à lui, et celui d'Aminata
    // ne s'imprime pas comme le sien.
    assert_eq!(
        p.ok("mes_bulletins_paie_lire", Some(&admin), Value::Null)
            .await,
        json!([])
    );
    assert_eq!(
        p.refus(
            "mon_bulletin_paie_produire",
            Some(&admin),
            json!({ "id": bulletin(&paie, "Aminata Kanté")["id"] })
        )
        .await
        .cle,
        "paie.bulletin_d_un_autre"
    );

    // Les heures d'octobre de Moussa, visées après la paie, sont payées en novembre, en rappel.
    p.ok(
        "heures_saisir",
        Some(&admin),
        heures(&vacation_traore, "2026-10", "10"),
    )
    .await;
    p.ok(
        "heures_viser",
        Some(&admin),
        json!({ "mois": "2026-10", "engagement_id": vacation_traore["id"] }),
    )
    .await;
    p.horloge.avancer(35 * JOUR);
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let paie = p
        .ok("paie_calculer", Some(&admin), json!({ "mois": "2026-11" }))
        .await;
    let moussa = bulletin(&paie, "Moussa Traoré");
    let rappel = moussa["resultat"]["lignes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["sorte"] == "heures")
        .unwrap();
    assert_eq!(rappel["mois_rappel"], "2026-10");
    assert_eq!(rappel["montant"], 25_000);
    // Cumuls de l'année : octobre et novembre.
    let aminata_novembre = bulletin(&paie, "Aminata Kanté");
    assert_eq!(aminata_novembre["resultat"]["brut"], 90_000);
    assert_eq!(
        aminata_novembre["resultat"]["cumuls"]["brut"],
        100_000 + 90_000
    );
    // Les heures d'octobre de Bakary, déjà payées, ne reviennent pas : rien à lui payer.
    assert!(
        paie["bulletins"]
            .as_array()
            .unwrap()
            .iter()
            .all(|b| b["membre"] != "Bakary Sangaré")
    );
}

#[tokio::test]
async fn une_avance_se_verse_se_rembourse_et_un_salaire_se_paie_en_plusieurs_fois() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    // Aminata a son compte : elle relira ses bulletins dans « Mon compte ».
    let mot_de_passe = "ecole-2026-55433";
    let compte = p
        .ok(
            "utilisateur_creer",
            Some(&admin),
            json!({ "identifiant": "kante", "nom_affiche": "Aminata Kanté",
                "mot_de_passe_initial": mot_de_passe, "roles": ["secretaire"] }),
        )
        .await;
    let aminata = membre(&p, &admin, "Kanté", "Aminata", false, compte["id"].clone()).await;
    p.ok(
        "engagement_conclure",
        Some(&admin),
        json!({ "membre_id": aminata["id"], "nature": "duree_indeterminee",
            "debut": "2026-10-01", "fin": null, "fonction": "",
            "remuneration": { "type": "mensuelle", "montant": 90_000 }, "note": "",
            "remplace": null, "renouvelle": null, "paye_douze_mois": null }),
    )
    .await;
    p.ok(
        "caisse_ouvrir",
        Some(&admin),
        json!({ "fonds_initial": 50_000 }),
    )
    .await;
    let tiroir = async || {
        p.ok("caisse_session_courante", Some(&admin), Value::Null)
            .await["totaux"]["theorique"]
            .as_i64()
            .unwrap()
    };

    // Une avance de 30 000 F en trois mois, versée en espèces de la caisse.
    let avance = |montant: i64, motif: &str| {
        json!({ "membre_id": aminata["id"], "montant": montant, "mensualites": 3,
            "premier_mois": "2026-10", "motif": motif, "mode": "especes", "reference": "",
            "source": "caisse" })
    };
    assert_eq!(
        p.refus("avance_accorder", Some(&admin), avance(30_000, " "))
            .await
            .cle,
        "avance.motif_obligatoire"
    );
    let fait = p
        .ok(
            "avance_accorder",
            Some(&admin),
            avance(30_000, "Rentrée des enfants"),
        )
        .await;
    assert_eq!(fait["etat"]["avances"][0]["mensualite"], 10_000);
    assert_eq!(fait["etat"]["avances"][0]["reste"], 30_000);
    assert_eq!(tiroir().await, 20_000);

    // Octobre retient la mensualité.
    let paie = p
        .ok("paie_calculer", Some(&admin), json!({ "mois": "2026-10" }))
        .await;
    let b = bulletin(&paie, "Aminata Kanté");
    assert_eq!(b["resultat"]["net"], 80_000);
    let ligne = b["resultat"]["lignes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["sorte"] == "avance")
        .unwrap();
    assert_eq!(ligne["libelle"], "Avance du 04/10/2026");
    assert_eq!(ligne["reste"], 20_000);
    p.ok("paie_valider", Some(&admin), json!({ "mois": "2026-10" }))
        .await;
    let etat = p
        .ok("paiements_personnel_lire", Some(&admin), Value::Null)
        .await;
    assert_eq!(etat["avances"][0]["reste"], 20_000);
    assert_eq!(etat["a_payer"].as_array().unwrap().len(), 1);
    assert_eq!(etat["total_du"], 80_000);
    let id_bulletin = etat["a_payer"][0]["bulletin_id"].clone();
    // Remboursée en partie, l'avance ne s'annule plus.
    assert_eq!(
        p.refus(
            "avance_annuler",
            Some(&admin),
            json!({ "id": etat["avances"][0]["avance"]["id"], "motif": "Erreur" })
        )
        .await
        .cle,
        "avance.deja_remboursee"
    );

    // Le salaire en plusieurs fois : 50 000 F de la caisse, qui n'en a que 20 000 (avertissement,
    // jamais un blocage), puis le reste par Orange Money, avec sa référence.
    let payer = |montant: i64, mode: &str, reference: &str, source: &str| {
        json!({ "bulletin_id": id_bulletin, "montant": montant, "mode": mode,
            "reference": reference, "source": source })
    };
    let fait = p
        .ok(
            "salaire_payer",
            Some(&admin),
            payer(50_000, "especes", "", "caisse"),
        )
        .await;
    assert_eq!(
        fait["avertissements"],
        json!(["paiement.tiroir_insuffisant"])
    );
    assert_eq!(fait["etat"]["a_payer"][0]["reste"], 30_000);
    assert_eq!(tiroir().await, -30_000);
    for (demande, cle) in [
        (
            payer(40_000, "especes", "", "caisse"),
            "paiement.superieur_au_reste",
        ),
        (
            payer(30_000, "orange_money", " ", "hors_caisse"),
            "paiement.reference_obligatoire",
        ),
        (
            payer(30_000, "orange_money", "OM-1", "caisse"),
            "paiement.caisse_especes",
        ),
    ] {
        assert_eq!(
            p.refus("salaire_payer", Some(&admin), demande).await.cle,
            cle
        );
    }
    let fait = p
        .ok(
            "salaire_payer",
            Some(&admin),
            payer(30_000, "orange_money", "OM-77812", "hors_caisse"),
        )
        .await;
    assert!(fait["etat"]["a_payer"].as_array().unwrap().is_empty());

    // Le paiement de la caisse, saisi par erreur, s'annule : l'argent revient dans le tiroir.
    let especes = fait["etat"]["paiements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["paiement"]["source"] == "caisse" && x["paiement"]["objet"] == "salaire")
        .unwrap()["paiement"]["id"]
        .clone();
    let fait = p
        .ok(
            "paiement_personnel_annuler",
            Some(&admin),
            json!({ "id": especes, "motif": "Saisi deux fois" }),
        )
        .await;
    assert_eq!(fait["etat"]["a_payer"][0]["reste"], 50_000);
    assert_eq!(tiroir().await, 20_000);
    // Le reste du mois se paie d'un geste, en espèces selon la fiche.
    let fait = p
        .ok(
            "paie_payer_reste",
            Some(&admin),
            json!({ "mois": "2026-10" }),
        )
        .await;
    assert!(fait["etat"]["a_payer"].as_array().unwrap().is_empty());
    // Le tiroir n'a que 20 000 F : les 50 000 F se notent payés hors caisse, ce qui est signalé.
    assert_eq!(
        fait["avertissements"],
        json!(["paiement.reste_hors_caisse"])
    );
    assert_eq!(tiroir().await, 20_000);

    // Novembre et décembre : la dernière mensualité solde l'avance.
    for (jours, mois) in [(35, "2026-11"), (31, "2026-12")] {
        p.horloge.avancer(jours * JOUR);
        let admin = p.connecter("admin", MOT_DE_PASSE).await;
        p.ok("paie_calculer", Some(&admin), json!({ "mois": mois }))
            .await;
        p.ok("paie_valider", Some(&admin), json!({ "mois": mois }))
            .await;
    }
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let etat = p
        .ok("paiements_personnel_lire", Some(&admin), Value::Null)
        .await;
    assert_eq!(etat["avances"][0]["avance"]["statut"], "soldee");
    assert_eq!(etat["avances"][0]["reste"], 0);
    // Novembre et décembre restent à payer, chacun 90 000 F moins sa mensualité.
    assert_eq!(etat["total_du"], 160_000);

    // Au tableau de bord : la paie de novembre validée, son bulletin pas encore payé, ce qui
    // reste dû ; aucun avertissement, puisque le mois écoulé est validé.
    let tableau = p
        .ok("tableau_de_bord_lire", Some(&admin), Value::Null)
        .await;
    assert_eq!(
        tableau["paie"],
        json!({ "mois": "2026-11", "statut": "validee", "bulletins": 1, "payes": 0,
            "reste_du": 160_000, "a_valider": false })
    );
    let alertes = avertissements(&p.ok("avertissements", Some(&admin), Value::Null).await);
    assert!(!alertes.contains("avertissement.paie_a_valider"));

    // Aminata relit ses trois bulletins validés, le plus récent d'abord, et imprime le sien.
    let elle = p.connecter("kante", mot_de_passe).await;
    let siens = p
        .ok("mes_bulletins_paie_lire", Some(&elle), Value::Null)
        .await;
    let siens = siens.as_array().unwrap();
    assert_eq!(siens.len(), 3);
    assert_eq!(siens[0]["mois"], "2026-12");
    assert_eq!(siens[2]["net"], 80_000);
    let document = p
        .ok(
            "mon_bulletin_paie_produire",
            Some(&elle),
            json!({ "id": siens[0]["id"] }),
        )
        .await;
    assert!(document["pdf"].as_str().unwrap().len() > 1000);
    deposer(&document, "mon-bulletin-paie.pdf");
}
