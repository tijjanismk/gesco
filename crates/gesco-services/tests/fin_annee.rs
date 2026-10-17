//! Fin d'année par la frontière (SPEC_PHASE_10 §3 et §4, UC-06) : la décision proposée suit la
//! moyenne annuelle et la moyenne de passage du cycle, le jardin passe d'office, la direction
//! valide ; l'année ne se clôture qu'avec toutes ses décisions, puis ne change plus, sauf
//! réouverture motivée.

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

fn id_par(liste: &Value, champ: &str, valeur: &str) -> Value {
    liste
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x[champ] == valeur)
        .unwrap_or_else(|| panic!("{champ} = {valeur} absent"))["id"]
        .clone()
}

/// Élève inscrit et validé dans la classe ; un enfant du jardin a sa photo et sa mère vient le
/// chercher.
async fn inscrire(p: &Poste_, admin: &Jeton, classe: &Value, prenoms: &str, jardin: bool) -> Value {
    let fiche = p
        .ok(
            "eleve_creer",
            Some(admin),
            json!({ "identite": { "nom": "Diarra", "prenoms": prenoms, "sexe": "feminin",
                "date_naissance": if jardin { "2023-02-11" } else { "2014-03-02" },
                "date_naissance_estimee": false, "lieu_naissance": "Bamako", "nationalite": "Malienne",
                "adresse": "", "telephones": [] },
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
                "recoit_notifications": true, "vient_chercher": jardin, "ordre_priorite": 1 } }),
    )
    .await;
    if jardin {
        p.ok(
            "eleve_photo_definir",
            Some(admin),
            json!({ "id": fiche["eleve"]["id"], "contenu_base64": PNG_ESSAI }),
        )
        .await;
    }
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
        json!({ "id": inscription }),
    )
    .await;
    inscription
}

fn ligne<'a>(decisions: &'a Value, inscription: &Value) -> &'a Value {
    decisions["lignes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["inscription_id"] == *inscription)
        .unwrap()
}

#[tokio::test]
async fn les_decisions_se_proposent_se_valident_puis_l_annee_se_cloture() {
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
    let creer_classe = async |niveau: &Value, libelle: &str| {
        p.ok(
            "classe_creer",
            Some(&admin),
            json!({ "annee_id": annee["id"], "niveau_id": niveau, "serie_id": null, "libelle": libelle }),
        )
        .await["id"]
            .clone()
    };
    let classe = creer_classe(&septieme, "7e A").await;
    let referentiel = p.ok("niveaux_lister", Some(&admin), Value::Null).await;
    let petite = id_par(&referentiel["niveaux"], "code", "PS");
    let jardin = creer_classe(&petite, "Petite section").await;
    let awa = inscrire(&p, &admin, &classe, "Awa", false).await;
    let binta = inscrire(&p, &admin, &classe, "Binta", false).await;
    let coumba = inscrire(&p, &admin, &classe, "Coumba", false).await;
    let bakary = inscrire(&p, &admin, &jardin, "Bakary", true).await;

    // Une composition de maths par trimestre, jusqu'au bulletin du 3e qui porte la moyenne
    // annuelle : Awa 13,00, Binta 9,00, Coumba 10,00 tout juste.
    let math = id_par(&ped["matieres"], "code", "MATH");
    let trimestre = |rang: i64| {
        ped["periodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["periode"]["niveau_id"] == septieme && x["periode"]["rang"] == rang)
            .unwrap()["periode"]["id"]
            .clone()
    };
    for (rang, date, notes) in [
        (1, "2026-11-20", ["12", "8", "10"]),
        (2, "2027-03-10", ["13", "9", "10"]),
        (3, "2027-05-20", ["14", "10", "10"]),
    ] {
        let e = p
            .ok(
                "evaluation_creer",
                Some(&admin),
                json!({ "classe_id": classe, "periode_id": trimestre(rang), "matiere_id": math,
                    "type_evaluation": "composition", "libelle": "Composition", "date": date, "bareme": null }),
            )
            .await;
        for (inscription, note) in [&awa, &binta, &coumba].into_iter().zip(notes) {
            p.ok(
                "note_saisir",
                Some(&admin),
                json!({ "evaluation_id": e["id"], "inscription_id": inscription, "saisie": note }),
            )
            .await;
        }
        p.ok("evaluation_valider", Some(&admin), json!({ "id": e["id"] }))
            .await;
        p.ok(
            "resultats_calculer",
            Some(&admin),
            json!({ "classe_id": classe, "periode_id": trimestre(rang) }),
        )
        .await;
    }

    // Les moyennes de passage des cycles enseignés : le 2e cycle sur 20, pas le jardin.
    let fin = p.ok("fin_annee_lire", Some(&admin), Value::Null).await;
    assert_eq!(fin["passages"].as_array().unwrap().len(), 1);
    assert_eq!(fin["passages"][0]["cycle"], "second_cycle");
    assert_eq!(fin["passages"][0]["passage"], 1000);
    assert_eq!(fin["en_attente"], 4);

    let demande = json!({ "classe_id": classe });
    let d = p
        .ok("decisions_classe_lire", Some(&admin), demande.clone())
        .await;
    assert_eq!(ligne(&d, &awa)["moyenne_annuelle"], 1300);
    assert_eq!(ligne(&d, &awa)["proposee"], "admis");
    assert_eq!(ligne(&d, &binta)["proposee"], "redouble");
    assert_eq!(ligne(&d, &coumba)["proposee"], "admis");
    assert_eq!(ligne(&d, &coumba)["decision"], "non_statue");

    // L'académie relève la barre à 10,50 : Coumba redoublerait. Puis elle revient à 10.
    let regler = async |passage: i64| {
        p.ok(
            "passage_regler",
            Some(&admin),
            json!({ "cycle": "second_cycle", "passage": passage }),
        )
        .await
    };
    regler(1050).await;
    let d = p
        .ok("decisions_classe_lire", Some(&admin), demande.clone())
        .await;
    assert_eq!(ligne(&d, &coumba)["proposee"], "redouble");
    regler(1000).await;
    for (cycle, passage) in [("second_cycle", 2001), ("jardin", 0)] {
        assert_eq!(
            p.refus(
                "passage_regler",
                Some(&admin),
                json!({ "cycle": cycle, "passage": passage })
            )
            .await
            .cle,
            "fin_annee.passage_invalide"
        );
    }

    // Rien n'est clôturé tant qu'une décision manque.
    let id_annee = json!({ "id": annee["id"] });
    assert_eq!(
        p.refus("annee_cloturer", Some(&admin), id_annee.clone())
            .await
            .cle,
        "fin_annee.decisions_en_attente"
    );

    // La direction valide les propositions de la classe, puis change une décision : Binta passe
    // un rattrapage, qui laisse sa décision en attente, puis redouble.
    let d = p
        .ok("decisions_valider_classe", Some(&admin), demande.clone())
        .await;
    assert_eq!(ligne(&d, &awa)["decision"], "admis");
    assert_eq!(ligne(&d, &binta)["decision"], "redouble");
    let poser = async |inscription: &Value, decision: &str| {
        p.ok(
            "decision_poser",
            Some(&admin),
            json!({ "inscription_id": inscription, "decision": decision }),
        )
        .await
    };
    poser(&binta, "rattrapage").await;
    let d = p
        .ok(
            "decisions_valider_classe",
            Some(&admin),
            json!({ "classe_id": jardin }),
        )
        .await;
    // Au jardin, sans moyenne, l'enfant passe à la section suivante.
    assert_eq!(ligne(&d, &bakary)["moyenne_annuelle"], Value::Null);
    assert_eq!(ligne(&d, &bakary)["decision"], "admis");
    let fin = p.ok("fin_annee_lire", Some(&admin), Value::Null).await;
    assert_eq!(fin["en_attente"], 1);
    assert_eq!(
        p.refus("annee_cloturer", Some(&admin), id_annee.clone())
            .await
            .cle,
        "fin_annee.decisions_en_attente"
    );
    poser(&binta, "redouble").await;

    // Clôture : une sauvegarde d'abord, les inscriptions s'achèvent, l'année ne change plus.
    let sauvegardes = async || {
        p.ok("sauvegardes_lister", Some(&admin), Value::Null)
            .await
            .as_array()
            .map_or(0, Vec::len)
    };
    let avant = sauvegardes().await;
    let close = p.ok("annee_cloturer", Some(&admin), id_annee.clone()).await;
    assert_eq!(close["statut"], "cloturee");
    assert_eq!(sauvegardes().await, avant + 1);
    // Le journal de l'année s'archive, signé, dans le dossier des sauvegardes du poste ; un octet
    // changé rompt la signature (§12.4, T-73).
    let archive = p.racine.join("sauvegardes").join("journal-2026-2027.gja");
    let verifiee = p
        .ok(
            "journal_archive_verifier",
            Some(&admin),
            json!({ "chemin": archive.to_string_lossy() }),
        )
        .await;
    assert_eq!(verifiee["valide"], true);
    assert_eq!(verifiee["annee"], "2026-2027");
    assert!(verifiee["lignes"].as_u64().unwrap() > 10);
    let texte = std::fs::read_to_string(&archive).unwrap();
    let altere = dossier.path().join("journal-altere.gja");
    std::fs::write(
        &altere,
        texte.replacen("\"annee\": \"2026-2027\"", "\"annee\": \"2026-2028\"", 1),
    )
    .unwrap();
    let verifiee = p
        .ok(
            "journal_archive_verifier",
            Some(&admin),
            json!({ "chemin": altere.to_string_lossy() }),
        )
        .await;
    assert_eq!(verifiee["valide"], false);
    let inscrits = p
        .ok(
            "inscriptions_lister",
            Some(&admin),
            json!({ "annee_id": annee["id"] }),
        )
        .await;
    assert!(
        inscrits
            .as_array()
            .unwrap()
            .iter()
            .all(|l| l["inscription"]["statut"] == "achevee")
    );
    assert_eq!(
        p.refus(
            "decision_poser",
            Some(&admin),
            json!({ "inscription_id": awa, "decision": "redouble" })
        )
        .await
        .cle,
        "fin_annee.annee_non_active"
    );
    let d = p
        .ok("decisions_classe_lire", Some(&admin), demande.clone())
        .await;
    assert_eq!(d["figee"], true);
    assert_eq!(ligne(&d, &binta)["decision"], "redouble");
    // Ni note, ni bulletin, ni relevé d'absences sur une année clôturée.
    let refus_clos = async |commande: &str, entree: Value| {
        assert_eq!(
            p.refus(commande, Some(&admin), entree).await.cle,
            "annee.cloturee",
            "{commande}"
        );
    };
    refus_clos(
        "evaluation_creer",
        json!({ "classe_id": classe, "periode_id": trimestre(3), "matiere_id": math,
            "type_evaluation": "devoir", "libelle": "Devoir", "date": "2027-06-01", "bareme": null }),
    )
    .await;
    refus_clos(
        "resultats_calculer",
        json!({ "classe_id": classe, "periode_id": trimestre(3) }),
    )
    .await;
    refus_clos(
        "absences_releve_enregistrer",
        json!({ "classe_id": classe, "date": "2027-06-01", "lignes": [] }),
    )
    .await;

    // Réouverture : motivée, les inscriptions redeviennent validées, décisions gardées.
    assert_eq!(
        p.refus(
            "annee_rouvrir",
            Some(&admin),
            json!({ "annee_id": annee["id"], "motif": " " })
        )
        .await
        .cle,
        "fin_annee.motif_requis"
    );
    let rouverte = p
        .ok(
            "annee_rouvrir",
            Some(&admin),
            json!({ "annee_id": annee["id"], "motif": "Note de Binta corrigée après le conseil" }),
        )
        .await;
    assert_eq!(rouverte["statut"], "active");
    let d = p.ok("decisions_classe_lire", Some(&admin), demande).await;
    assert_eq!(d["figee"], false);
    assert_eq!(ligne(&d, &awa)["decision"], "admis");
    poser(&awa, "admis").await;
    let journal = p
        .ok("audit_lister", Some(&admin), json!({ "page": 0 }))
        .await;
    assert!(journal.to_string().contains("fin_annee.rouvrir"));
    p.ok("annee_cloturer", Some(&admin), id_annee).await;

    // L'année suivante se prépare : ses classes reprises de la précédente, puis chaque élève
    // admis ou redoublant préparé dans la classe où il revient.
    let suivante = p
        .ok(
            "annee_creer",
            Some(&admin),
            json!({ "libelle": "2027-2028", "debut": RENTREE_2027, "fin": FIN_2028 }),
        )
        .await;
    let id_suivante = json!({ "id": suivante["id"] });
    let fin = p.ok("fin_annee_lire", Some(&admin), Value::Null).await;
    assert_eq!(fin["suivante"]["annee"]["libelle"], "2027-2028");
    assert_eq!(fin["suivante"]["classes"], 0);
    let fin = p
        .ok("classes_reprendre", Some(&admin), id_suivante.clone())
        .await;
    assert_eq!(fin["suivante"]["classes"], 2);
    // Binta redouble en 7e A ; Awa et Coumba montent en 8e, Bakary en moyenne section, qui
    // n'ont pas encore de classe.
    let bilan = p
        .ok("reinscriptions_preparer", Some(&admin), id_suivante.clone())
        .await;
    assert_eq!(bilan["preparees"], 1);
    assert_eq!(bilan["sans_classe"], json!(["8e année", "Moyenne section"]));
    let huitieme = id_par(&ped["niveaux"], "code", "8");
    let huitieme_a = p
        .ok(
            "classe_creer",
            Some(&admin),
            json!({ "annee_id": suivante["id"], "niveau_id": huitieme, "serie_id": null, "libelle": "8e A" }),
        )
        .await["id"]
        .clone();
    let bilan = p
        .ok("reinscriptions_preparer", Some(&admin), id_suivante)
        .await;
    assert_eq!(bilan["preparees"], 2);
    assert_eq!(bilan["deja_inscrits"], 1);
    assert_eq!(bilan["fin"]["suivante"]["a_valider"], 3);
    let preparees = p
        .ok(
            "inscriptions_lister",
            Some(&admin),
            json!({ "annee_id": suivante["id"] }),
        )
        .await;
    let binta_suivante = preparees
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["eleve"] == "Binta Diarra")
        .unwrap()["inscription"]
        .clone();
    assert_eq!(binta_suivante["redoublant"], true);
    assert_eq!(binta_suivante["type_inscription"], "reinscription");
    assert_eq!(binta_suivante["date_inscription"], "2027-10-01");

    // Au guichet, avant la rentrée : la classe de 8e se valide d'un geste, factures comprises,
    // sur la grille de l'année suivante.
    poser_grille(&p, &admin, &suivante["id"]).await;
    p.ok(
        "inscriptions_valider_classe",
        Some(&admin),
        json!({ "id": huitieme_a }),
    )
    .await;
    let fin = p.ok("fin_annee_lire", Some(&admin), Value::Null).await;
    assert_eq!(fin["suivante"]["validees"], 2);
    assert_eq!(fin["suivante"]["a_valider"], 1);

    // La rentrée : l'année suivante s'ouvre ; ce que chacun doit encore de 2026-2027 le suit,
    // en arriéré, dans la liste des impayés.
    p.ok(
        "annee_activer",
        Some(&admin),
        json!({ "id": suivante["id"] }),
    )
    .await;
    let impayes = p.ok("impayes_lister", Some(&admin), Value::Null).await;
    let arrieres: Vec<&Value> = impayes["lignes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|l| l["arriere"] == "2026-2027")
        .collect();
    assert_eq!(arrieres.len(), 4);
    assert!(
        arrieres
            .iter()
            .all(|l| l["classe"].as_str().unwrap().ends_with("· 2026-2027"))
    );
}
