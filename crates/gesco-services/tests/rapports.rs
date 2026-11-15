//! Rapports de l'école par la frontière (SPEC_PHASE_9 §3, §4, §5, §8.6, §9.3, §10) : une école sans
//! année les liste sans refus ; un rapport se prépare une fois, avec la date d'arrêt proposée ;
//! ses tableaux comptent les élèves à cette date ; un texte s'enregistre et refuse une
//! modification croisée ; l'aperçu sort ; arrêté, il est figé et refuse les modifications ;
//! rouvert, il passe à sa version suivante et l'ancien document reste ; le droit est tenu.

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

fn tableau<'a>(rapport: &'a Value, code: &str) -> &'a Value {
    rapport["contenu"]["tableaux"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["code"] == code)
        .unwrap_or_else(|| panic!("tableau {code} absent"))
}

/// Case d'un tableau de rapport, par le libellé de sa ligne et le titre de sa colonne.
fn case(t: &Value, ligne: &str, colonne: &str) -> String {
    let i = t["colonnes"]
        .as_array()
        .unwrap()
        .iter()
        .position(|c| c == colonne)
        .unwrap_or_else(|| panic!("colonne {colonne} absente"));
    t["lignes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l[0] == ligne)
        .unwrap_or_else(|| panic!("ligne {ligne} absente"))[i]
        .as_str()
        .unwrap()
        .to_owned()
}

async fn inscrire(
    p: &Poste_,
    admin: &gesco_services::session::Jeton,
    classe: &Value,
    prenoms: &str,
    sexe: &str,
    date: &str,
    redoublant: bool,
) -> (Value, Value) {
    let fiche = p
        .ok(
            "eleve_creer",
            Some(admin),
            json!({ "identite": { "nom": "Coulibaly", "prenoms": prenoms, "sexe": sexe,
                "date_naissance": "2019-03-02", "date_naissance_estimee": false,
                "lieu_naissance": "Kati", "nationalite": "Malienne", "adresse": "",
                "telephones": [] },
                "matricule_interne": "", "matricule_national": "", "acte_nature": "acte",
                "acte_numero": "", "acte_date": "", "ecole_origine": "" }),
        )
        .await;
    let inscription = p
        .ok(
            "inscription_creer",
            Some(admin),
            json!({ "eleve_id": fiche["eleve"]["id"], "classe_id": classe,
                "date_inscription": date, "redoublant": redoublant,
                "etablissement_origine": "" }),
        )
        .await["inscription"]["id"]
        .clone();
    p.ok(
        "inscription_valider",
        Some(admin),
        json!({ "id": inscription, "choix": {} }),
    )
    .await;
    (fiche["eleve"]["id"].clone(), inscription)
}

#[tokio::test]
async fn les_rapports_de_rentree_se_preparent_s_arretent_et_se_rouvrent() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;

    // Une école sans année : les trois rapports sont listés, aucun n'est préparé.
    let vide = p.ok("rapports_lister", Some(&admin), json!({})).await;
    assert_eq!(vide["rapports"].as_array().unwrap().len(), 3);
    assert!(vide["rapports"][0]["id"].is_null());

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
    for (code, libelle) in [("1", "1re A"), ("7", "7e A")] {
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
    inscrire(
        &p,
        &admin,
        &classes[0],
        "Awa",
        "feminin",
        "2026-10-01",
        false,
    )
    .await;
    inscrire(
        &p,
        &admin,
        &classes[0],
        "Issa",
        "masculin",
        "2026-10-01",
        true,
    )
    .await;
    inscrire(
        &p,
        &admin,
        &classes[1],
        "Mariam",
        "feminin",
        "2026-10-02",
        false,
    )
    .await;
    // Inscrite après la date d'arrêt proposée : elle ne compte pas.
    inscrire(
        &p,
        &admin,
        &classes[1],
        "Kadia",
        "feminin",
        "2026-10-20",
        false,
    )
    .await;

    // Le rapport de rentrée : six sections, l'introduction aidée, date proposée.
    let rentree = p
        .ok(
            "rapport_preparer",
            Some(&admin),
            json!({ "annee_id": annee["id"], "type_rapport": "rentree" }),
        )
        .await;
    assert_eq!(rentree["etat"], "preparation");
    assert_eq!(rentree["version"], 1);
    assert_eq!(rentree["date_arret"], "2026-10-16");
    assert_eq!(rentree["qualite_signataire"], "Le Directeur");
    assert_eq!(rentree["sections"].as_array().unwrap().len(), 6);
    let intro = &rentree["sections"][0];
    assert_eq!(intro["code"], "introduction");
    assert!(
        intro["propose"]
            .as_str()
            .unwrap()
            .contains("3 élèves, dont 2 filles"),
        "{intro}"
    );
    assert!(rentree["sections"][1]["propose"].is_null());
    assert!(intro["an_dernier"].is_null());
    let id = rentree["id"].clone();

    // Préparer de nouveau rend le même rapport.
    let encore = p
        .ok(
            "rapport_preparer",
            Some(&admin),
            json!({ "annee_id": annee["id"], "type_rapport": "rentree" }),
        )
        .await;
    assert_eq!(encore["id"], id);

    // Tableaux : élèves par année d'études à la date d'arrêt.
    let t = tableau(&rentree, "eleves_par_annee");
    assert_eq!(case(t, "1re année", "Passants T"), "1");
    assert_eq!(case(t, "1re année", "Redoublants G"), "1");
    assert_eq!(case(t, "Total premier cycle", "Total T"), "2");
    assert_eq!(case(t, "Total second cycle", "Total F"), "1");
    assert_eq!(case(t, "Total général", "Total T"), "3");
    // Sans relevé d'inventaire, les salles restent vides, jamais à zéro.
    let salles = tableau(&rentree, "salles_de_classe");
    assert_eq!(case(salles, "Ensemble", "Dur"), "");
    let classes_t = tableau(&rentree, "classes_par_cours");
    assert_eq!(case(classes_t, "1re année", "Simple"), "1");
    assert_eq!(case(classes_t, "Total", "Classique"), "2");

    // Un texte s'enregistre ; une écriture sur un texte lu avant elle est refusée.
    let ecrit = p
        .ok(
            "rapport_texte_enregistrer",
            Some(&admin),
            json!({ "id": id, "section": "conclusion", "texte": "Bonne rentrée.", "lue_le": 0 }),
        )
        .await;
    let conclusion = ecrit["sections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["code"] == "conclusion")
        .unwrap()
        .clone();
    assert_eq!(conclusion["texte"], "Bonne rentrée.");
    assert_ne!(conclusion["lue_le"], 0);
    assert_eq!(
        p.refus(
            "rapport_texte_enregistrer",
            Some(&admin),
            json!({ "id": id, "section": "conclusion", "texte": "Autre.", "lue_le": 0 }),
        )
        .await
        .cle,
        "fiche.modifiee_entre_temps"
    );
    assert_eq!(
        p.refus(
            "rapport_texte_enregistrer",
            Some(&admin),
            json!({ "id": id, "section": "inconnue", "texte": "x", "lue_le": 0 }),
        )
        .await
        .cle,
        "rapports.section_inconnue"
    );

    // Réglages : lieu et date de signature passent dans « Fait à ».
    let regle = p
        .ok(
            "rapport_regler",
            Some(&admin),
            json!({ "id": id, "date_arret": "2026-10-16", "lieu": "Kati",
                "date_signature": "2026-10-17", "qualite_signataire": "La Directrice" }),
        )
        .await;
    assert_eq!(
        regle["contenu"]["fait_a"],
        "Fait à Kati, le 17 octobre 2026"
    );
    assert_eq!(regle["contenu"]["qualite"], "La Directrice");

    // Le rapport succinct reprend le lieu et la qualité du dernier rapport réglé.
    let succinct = p
        .ok(
            "rapport_preparer",
            Some(&admin),
            json!({ "annee_id": annee["id"], "type_rapport": "succinct_rentree" }),
        )
        .await;
    assert_eq!(succinct["lieu"], "Kati");
    assert_eq!(succinct["qualite_signataire"], "La Directrice");
    assert!(succinct["sections"].as_array().unwrap().is_empty());
    let t2 = tableau(&succinct, "groupes_eleves_redoublants");
    assert_eq!(case(t2, "1re année", "Groupes pédagogiques"), "1");
    assert_eq!(case(t2, "1re année", "Élèves T"), "2");
    assert_eq!(case(t2, "Total premier cycle", "Redoublants T"), "1");
    let entete = succinct["contenu"]["entete"].as_array().unwrap();
    assert!(
        entete
            .iter()
            .any(|l| l["libelle"] == "Cycles" && l["valeur"] == "Cycle complet")
    );

    // Aperçu d'un rapport en préparation : un PDF, sans document enregistré.
    let apercu = p
        .ok("rapport_apercu", Some(&admin), json!({ "id": id }))
        .await;
    assert!(!apercu["pdf"].as_str().unwrap().is_empty());
    let relu = p
        .ok("rapport_lire", Some(&admin), json!({ "id": id }))
        .await;
    assert!(relu["documents"].as_array().unwrap().is_empty());

    // Arrêter : le document est enregistré, le contenu figé, les modifications refusées.
    let arrete = p
        .ok("rapport_arreter", Some(&admin), json!({ "id": id }))
        .await;
    assert_eq!(arrete["etat"], "arrete");
    assert_eq!(arrete["documents"].as_array().unwrap().len(), 1);
    assert_eq!(
        p.refus("rapport_arreter", Some(&admin), json!({ "id": id }))
            .await
            .cle,
        "rapports.deja_arrete"
    );
    assert_eq!(
        p.refus(
            "rapport_texte_enregistrer",
            Some(&admin),
            json!({ "id": id, "section": "conclusion", "texte": "x",
                "lue_le": conclusion["lue_le"] }),
        )
        .await
        .cle,
        "rapports.arrete"
    );
    assert_eq!(
        p.refus(
            "rapport_regler",
            Some(&admin),
            json!({ "id": id, "date_arret": "2026-10-30", "lieu": "Kati",
                "date_signature": null, "qualite_signataire": "" }),
        )
        .await
        .cle,
        "rapports.arrete"
    );
    // Une inscription de plus, avant la date d'arrêt : le rapport arrêté ne bouge pas.
    inscrire(
        &p,
        &admin,
        &classes[0],
        "Seydou",
        "masculin",
        "2026-10-03",
        false,
    )
    .await;
    let fige = p
        .ok("rapport_lire", Some(&admin), json!({ "id": id }))
        .await;
    assert_eq!(
        case(
            tableau(&fige, "eleves_par_annee"),
            "Total général",
            "Total T"
        ),
        "3"
    );
    let reimpression = p
        .ok("rapport_apercu", Some(&admin), json!({ "id": id }))
        .await;
    assert!(!reimpression["pdf"].as_str().unwrap().is_empty());

    // Rouvrir : version suivante, l'ancien document reste, les chiffres suivent les données.
    let rouvert = p
        .ok("rapport_rouvrir", Some(&admin), json!({ "id": id }))
        .await;
    assert_eq!(rouvert["etat"], "preparation");
    assert_eq!(rouvert["version"], 2);
    assert_eq!(rouvert["documents"].as_array().unwrap().len(), 1);
    assert_eq!(
        case(
            tableau(&rouvert, "eleves_par_annee"),
            "Total général",
            "Total T"
        ),
        "4"
    );
    assert_eq!(
        p.refus("rapport_rouvrir", Some(&admin), json!({ "id": id }))
            .await
            .cle,
        "rapports.pas_arrete"
    );

    let liste = p.ok("rapports_lister", Some(&admin), json!({})).await;
    assert!(
        liste["rapports"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["type_rapport"] != "fermeture")
            .all(|r| !r["id"].is_null())
    );
}

#[tokio::test]
async fn les_rapports_sont_reserves_a_la_direction() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    p.ok(
        "utilisateur_creer",
        Some(&admin),
        json!({ "identifiant": "secretaire", "nom_affiche": "secretaire",
            "mot_de_passe_initial": "mot-de-passe-2026", "roles": ["secretaire"] }),
    )
    .await;
    let secretaire = p.connecter("secretaire", "mot-de-passe-2026").await;
    assert_eq!(
        p.refus("rapports_lister", Some(&secretaire), json!({}))
            .await
            .cle,
        "erreur.permission_refusee"
    );
    assert_eq!(
        p.refus("rapport_rouvrir", Some(&secretaire), json!({ "id": "x" }))
            .await
            .cle,
        "erreur.permission_refusee"
    );
    assert_eq!(
        p.refus(
            "rapport_effectif_regler",
            Some(&secretaire),
            json!({ "id": "x", "niveau_id": "y", "garcons": 1, "filles": 1 }),
        )
        .await
        .cle,
        "erreur.permission_refusee"
    );
    // La fiche scolaire est un document de l'élève : le secrétariat l'imprime.
    assert_eq!(
        p.refus(
            "fiche_scolaire_produire",
            Some(&secretaire),
            json!({ "eleve_id": null, "classe_id": null }),
        )
        .await
        .cle,
        "commande.entree_invalide"
    );
}

/// Ligne `k` d'un tableau de rapport, case par titre de colonne.
fn case_ligne(t: &Value, k: usize, colonne: &str) -> String {
    let i = t["colonnes"]
        .as_array()
        .unwrap()
        .iter()
        .position(|c| c == colonne)
        .unwrap_or_else(|| panic!("colonne {colonne} absente"));
    t["lignes"][k][i].as_str().unwrap().to_owned()
}

#[tokio::test]
async fn le_rapport_de_fermeture_compte_la_fin_d_annee_et_garde_les_effectifs_saisis() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let vide = p.ok("rapports_lister", Some(&admin), json!({})).await;
    assert_eq!(vide["rapports"].as_array().unwrap().len(), 3);

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
    let libelle = |code: &str| -> String {
        ped["niveaux"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["code"] == code)
            .unwrap()["libelle"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    let mut classes = Vec::new();
    for (code, nom) in [("1", "1re A"), ("2", "2e A"), ("9", "9e A"), ("3", "3e A")] {
        classes.push(
            p.ok(
                "classe_creer",
                Some(&admin),
                json!({ "annee_id": annee["id"], "niveau_id": id_par(&ped["niveaux"], "code", code),
                    "serie_id": null, "libelle": nom }),
            )
            .await["id"]
                .clone(),
        );
    }
    let (_, awa) = inscrire(
        &p,
        &admin,
        &classes[0],
        "Awa",
        "feminin",
        "2026-10-01",
        false,
    )
    .await;
    let (_, moussa) = inscrire(
        &p,
        &admin,
        &classes[0],
        "Moussa",
        "masculin",
        "2026-10-01",
        false,
    )
    .await;
    let (_, fanta) = inscrire(
        &p,
        &admin,
        &classes[1],
        "Fanta",
        "feminin",
        "2026-10-01",
        false,
    )
    .await;
    let (issa_eleve, issa) = inscrire(
        &p,
        &admin,
        &classes[2],
        "Issa",
        "masculin",
        "2026-10-01",
        false,
    )
    .await;
    for (inscription, decision) in [
        (&awa, "admis"),
        (&moussa, "redouble"),
        (&fanta, "exclu"),
        (&issa, "admis"),
    ] {
        p.ok(
            "decision_poser",
            Some(&admin),
            json!({ "inscription_id": inscription, "decision": decision }),
        )
        .await;
    }
    p.ok(
        "resultat_examen_regler",
        Some(&admin),
        json!({ "inscription_id": issa, "inscrit": null, "present": null, "admis": true,
            "mention": "Bien" }),
    )
    .await;

    // Préparé une fois, arrêté à la fin de l'année.
    let r = p
        .ok(
            "rapport_preparer",
            Some(&admin),
            json!({ "annee_id": annee["id"], "type_rapport": "fermeture" }),
        )
        .await;
    let id = r["id"].clone();
    assert!(r["date_arret"].as_str().unwrap().starts_with("2027-07"));
    assert_eq!(r["sections"].as_array().unwrap().len(), 10);
    assert!(
        r["contenu"]["entete"]
            .as_array()
            .unwrap()
            .iter()
            .any(|l| l["libelle"] == "Cercle")
    );

    // Effectifs de fin d'année du premier cycle : fermeture, passages, redoublements, exclusions.
    let premier = tableau(&r, "effectifs_fin_premier_cycle");
    let (n1, n2) = (libelle("1"), libelle("2"));
    assert_eq!(case_ligne(premier, 0, &n1), "1", "garçons de 1re année");
    assert_eq!(case_ligne(premier, 2, &n1), "2");
    assert_eq!(case_ligne(premier, 2, "Total"), "3");
    assert_eq!(case_ligne(premier, 4, &n1), "1", "une fille passe");
    assert_eq!(case_ligne(premier, 6, &n1), "1", "un garçon redouble");
    assert_eq!(case_ligne(premier, 10, &n2), "1", "une fille exclue");
    assert_eq!(case_ligne(premier, 9, &n2), "0");
    // Une année sans classe laisse ses cases vides.
    assert_eq!(case_ligne(premier, 2, &libelle("4")), "");
    // Probables : redoublants de l'année, plus passants de l'année d'en dessous.
    assert_eq!(case_ligne(premier, 12, &n1), "1");
    assert_eq!(case_ligne(premier, 13, &n2), "1");
    assert_eq!(case_ligne(premier, 14, &n2), "1");

    // DEF : détail par élève.
    let def = tableau(&r, "examen_def");
    assert_eq!(case(def, "École", "Admis G"), "1");
    assert_eq!(case(def, "École", "Admis F"), "0");
    assert!(case(def, "École", "% de réussite G").starts_with("100"));
    // Entrée en 7e : rien de saisi, des cases vides.
    assert_eq!(
        case(tableau(&r, "examen_entree_7e"), "École", "Inscrits T"),
        ""
    );
    // Locaux : les rubriques du modèle, vides sans inventaire.
    assert_eq!(
        tableau(&r, "infrastructures")["lignes"]
            .as_array()
            .unwrap()
            .len(),
        7
    );
    assert_eq!(
        case(tableau(&r, "enseignants_par_statut"), "Total", "Total H"),
        "0"
    );

    // Phrases proposées : état des lieux et résultats ; rien pour les besoins sans inventaire.
    let section = |r: &Value, code: &str| -> Value {
        r["sections"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["code"] == code)
            .unwrap()
            .clone()
    };
    let etat = section(&r, "etat_des_lieux")["propose"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(
        etat.contains("3 élèves au premier cycle, dont 2 filles et 1 garçon"),
        "{etat}"
    );
    assert!(
        section(&r, "resultats")["propose"]
            .as_str()
            .unwrap()
            .contains("au premier cycle")
    );
    assert!(section(&r, "besoins")["propose"].is_null());
    assert!(section(&r, "introduction")["propose"].is_null());

    // La direction corrige les effectifs probables de la 1re année ; une case vide garde le
    // nombre proposé.
    let grille = r["effectifs_probables"].as_array().unwrap();
    let premiere = grille.iter().find(|n| n["niveau"] == n1.as_str()).unwrap();
    assert_eq!(premiere["garcons_proposes"], 1);
    assert!(premiere["garcons"].is_null());
    let r = p
        .ok(
            "rapport_effectif_regler",
            Some(&admin),
            json!({ "id": id, "niveau_id": premiere["niveau_id"], "garcons": 30, "filles": null }),
        )
        .await;
    let premier = tableau(&r, "effectifs_fin_premier_cycle");
    assert_eq!(case_ligne(premier, 12, &n1), "30");
    assert_eq!(case_ligne(premier, 13, &n1), "0");
    assert_eq!(case_ligne(premier, 14, &n1), "30");
    assert_eq!(
        p.refus(
            "rapport_effectif_regler",
            Some(&admin),
            json!({ "id": id, "niveau_id": premiere["niveau_id"], "garcons": 200_000,
                "filles": null }),
        )
        .await
        .cle,
        "rapports.effectif_trop_grand"
    );

    // Aperçu, arrêt : les effectifs ne se saisissent plus.
    let apercu = p
        .ok("rapport_apercu", Some(&admin), json!({ "id": id }))
        .await;
    assert!(!apercu["pdf"].as_str().unwrap().is_empty());
    let arrete = p
        .ok("rapport_arreter", Some(&admin), json!({ "id": id }))
        .await;
    assert_eq!(arrete["etat"], "arrete");
    assert!(arrete["effectifs_probables"].as_array().unwrap().is_empty());
    assert_eq!(
        case_ligne(tableau(&arrete, "effectifs_fin_premier_cycle"), 12, &n1),
        "30"
    );
    assert_eq!(
        p.refus(
            "rapport_effectif_regler",
            Some(&admin),
            json!({ "id": id, "niveau_id": premiere["niveau_id"], "garcons": 1, "filles": 1 }),
        )
        .await
        .cle,
        "rapports.arrete"
    );

    // Fiche scolaire : d'un élève, d'une classe, d'une classe vide ; jamais sans cible.
    for demande in [
        json!({ "eleve_id": issa_eleve, "classe_id": null }),
        json!({ "eleve_id": null, "classe_id": classes[0] }),
        json!({ "eleve_id": null, "classe_id": classes[3] }),
    ] {
        let fiche = p.ok("fiche_scolaire_produire", Some(&admin), demande).await;
        assert!(!fiche["pdf"].as_str().unwrap().is_empty());
    }
    assert_eq!(
        p.refus(
            "fiche_scolaire_produire",
            Some(&admin),
            json!({ "eleve_id": null, "classe_id": null }),
        )
        .await
        .cle,
        "commande.entree_invalide"
    );
}
