//! Résultats aux examens et onglet « Examens et fin d'année » par la frontière (SPEC_PHASE_9
//! §8.5, §9.1) : la grille d'une classe d'examen, un résultat rangé sans contradiction, les
//! totaux qui servent quand rien n'est saisi par élève, le réglage de l'entrée en 7e, les
//! décisions par niveau, le taux d'admis par cycle, les effectifs probables, le droit tenu.

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

fn tableau<'a>(onglet: &'a Value, code: &str) -> &'a Value {
    onglet["tableaux"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["code"] == code)
        .unwrap_or_else(|| panic!("tableau {code} absent"))
}

fn case<'a>(t: &'a Value, ligne: &Value, colonne: &str) -> &'a Value {
    let i = t["colonnes"]
        .as_array()
        .unwrap()
        .iter()
        .position(|c| c == colonne)
        .unwrap_or_else(|| panic!("colonne {colonne} absente"));
    let l = t["lignes"]
        .as_array()
        .unwrap()
        .iter()
        .chain(std::iter::once(&t["total"]))
        .find(|l| &l["libelle"] == ligne || &l["cle"] == ligne)
        .unwrap_or_else(|| panic!("ligne {ligne} absente"));
    &l["cellules"][i]
}

async fn inscrire(
    p: &Poste_,
    admin: &gesco_services::session::Jeton,
    classe: &Value,
    prenoms: &str,
    sexe: &str,
) -> Value {
    let fiche = p
        .ok(
            "eleve_creer",
            Some(admin),
            json!({ "identite": { "nom": "Traoré", "prenoms": prenoms, "sexe": sexe,
                "date_naissance": "2012-03-02", "date_naissance_estimee": false,
                "lieu_naissance": "Kati", "nationalite": "Malienne", "adresse": "",
                "telephones": [] },
                "matricule_interne": "", "matricule_national": matricule_national(), "acte_nature": "acte",
                "acte_numero": "", "acte_date": "", "ecole_origine": "" }),
        )
        .await;
    p.ok(
        "responsable_rattacher",
        Some(admin),
        json!({ "eleve_id": fiche["eleve"]["id"], "responsable_id": null,
            "saisie": { "identite": { "nom": "Coulibaly", "prenoms": "Mariam",
                "sexe": "feminin", "date_naissance": "", "date_naissance_estimee": false,
                "lieu_naissance": "", "nationalite": "", "adresse": "", "telephones": ["76 12 34 56"] },
                "profession": "", "employeur": "" },
            "qualites": { "lien": "mere", "est_responsable_legal": true, "est_payeur": true,
                "recoit_notifications": true, "vient_chercher": false, "ordre_priorite": 1 } }),
    )
    .await;
    let inscription = p
        .ok(
            "inscription_creer",
            Some(admin),
            json!({ "eleve_id": fiche["eleve"]["id"], "classe_id": classe,
                "date_inscription": "2026-10-01", "redoublant": false,
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
    inscription
}

#[tokio::test]
async fn les_examens_se_saisissent_et_se_comptent() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;

    // Une école sans année : rien à saisir, aucun refus.
    let vide = p.ok("examens_lire", Some(&admin), json!({})).await;
    assert!(vide["classes"].as_array().unwrap().is_empty());
    assert_eq!(vide["entree_7e"], false);

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
    let mut classes = Vec::new();
    for (code, libelle) in [("6", "6e A"), ("8", "8e A"), ("9", "9e A")] {
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
    let fanta = inscrire(&p, &admin, &classes[1], "Fanta", "feminin").await;
    let awa = inscrire(&p, &admin, &classes[2], "Awa", "feminin").await;
    let moussa = inscrire(&p, &admin, &classes[2], "Moussa", "masculin").await;

    // Seule la 9e passe un examen, le DEF, tant que l'entrée en 7e est éteinte.
    let ex = p.ok("examens_lire", Some(&admin), json!({})).await;
    assert_eq!(ex["classes"].as_array().unwrap().len(), 1);
    assert_eq!(ex["classes"][0]["examen"], "def");
    assert_eq!(ex["classe_id"], classes[2]);
    assert_eq!(ex["eleves"].as_array().unwrap().len(), 2);
    assert_eq!(ex["eleves"][0]["inscrit"], Value::Null);

    // Admise entraîne présente et inscrite ; absent efface l'admission.
    let r = p
        .ok(
            "resultat_examen_regler",
            Some(&admin),
            json!({ "inscription_id": awa, "inscrit": null, "present": null, "admis": true,
                "mention": " Assez  bien " }),
        )
        .await;
    assert_eq!(r["inscrit"], true);
    assert_eq!(r["present"], true);
    assert_eq!(r["mention"], "Assez bien");
    let r = p
        .ok(
            "resultat_examen_regler",
            Some(&admin),
            json!({ "inscription_id": moussa, "inscrit": true, "present": false, "admis": true,
                "mention": "" }),
        )
        .await;
    assert_eq!(r["present"], true, "admis l'emporte : il s'est présenté");
    let r = p
        .ok(
            "resultat_examen_regler",
            Some(&admin),
            json!({ "inscription_id": moussa, "inscrit": true, "present": false, "admis": null,
                "mention": "" }),
        )
        .await;
    assert_eq!(r["admis"], false);
    // Une classe sans examen ne reçoit pas de résultat.
    assert_eq!(
        p.refus(
            "resultat_examen_regler",
            Some(&admin),
            json!({ "inscription_id": fanta, "inscrit": true, "present": null, "admis": null,
                "mention": "" }),
        )
        .await
        .cle,
        "examens.classe_sans_examen"
    );

    // Des totaux pour le DEF ne servent pas : le détail l'emporte. Ceux du baccalauréat servent.
    for (examen, sexe, n) in [
        ("def", "masculin", 40),
        ("bac", "masculin", 12),
        ("bac", "feminin", 8),
    ] {
        p.ok(
            "total_examen_regler",
            Some(&admin),
            json!({ "annee_id": annee["id"], "examen": examen, "sexe": sexe, "inscrits": n,
                "presentes": n, "admis": n / 2 }),
        )
        .await;
    }
    let ex = p.ok("examens_lire", Some(&admin), json!({})).await;
    assert_eq!(ex["avec_detail"], json!(["def"]));
    assert_eq!(ex["totaux"].as_array().unwrap().len(), 3);

    // Décisions de fin d'année.
    for (inscription, decision) in [(&fanta, "admis"), (&awa, "admis"), (&moussa, "redouble")] {
        p.ok(
            "decision_poser",
            Some(&admin),
            json!({ "inscription_id": inscription, "decision": decision }),
        )
        .await;
    }

    let stats = p
        .ok(
            "statistiques_lire",
            Some(&admin),
            json!({ "annee_id": annee["id"], "date": "2027-01-15", "cycle": null }),
        )
        .await;
    let onglet = &stats["examens"];
    let decisions = tableau(onglet, "decisions_par_niveau");
    let neuf = id_par(&ped["niveaux"], "code", "9");
    let libelle_9 = ped["niveaux"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == neuf)
        .unwrap()["libelle"]
        .clone();
    assert_eq!(case(decisions, &libelle_9, "effectif_total")["nombre"], 2);
    assert_eq!(case(decisions, &libelle_9, "passages_filles")["nombre"], 1);
    assert_eq!(
        case(decisions, &libelle_9, "redoublements_garcons")["nombre"],
        1
    );
    assert_eq!(case(decisions, &libelle_9, "exclusions_total")["nombre"], 0);
    let total = json!("statistiques.total");
    assert_eq!(case(decisions, &total, "effectif_total")["nombre"], 3);

    let resultats = tableau(onglet, "resultats_examens");
    let def = json!("examens.examen.def");
    assert_eq!(case(resultats, &def, "inscrits_total")["nombre"], 2);
    assert_eq!(case(resultats, &def, "presentes_total")["nombre"], 1);
    assert_eq!(case(resultats, &def, "admis_filles")["nombre"], 1);
    assert_eq!(case(resultats, &def, "taux_total")["texte"], "100,0 %");
    assert_eq!(case(resultats, &def, "taux_garcons")["texte"], Value::Null);
    let bac = json!("examens.examen.bac");
    assert_eq!(case(resultats, &bac, "inscrits_total")["nombre"], 20);
    assert_eq!(case(resultats, &bac, "admis_total")["nombre"], 10);
    assert_eq!(case(resultats, &bac, "taux_total")["texte"], "50,0 %");

    let taux = tableau(onglet, "taux_admis_par_cycle");
    let second = json!("niveaux.cycle.second_cycle");
    assert_eq!(case(taux, &second, "decides")["nombre"], 3);
    assert_eq!(case(taux, &second, "taux")["texte"], "66,7 %");

    // 9e probable : le redoublant de 9e et le passant de 8e.
    let probables = tableau(onglet, "effectifs_probables");
    assert_eq!(case(probables, &libelle_9, "total")["nombre"], 2);

    // Pour le premier cycle, les totaux du baccalauréat ne paraissent pas.
    let stats_premier = p
        .ok(
            "statistiques_lire",
            Some(&admin),
            json!({ "annee_id": annee["id"], "date": "2027-01-15", "cycle": "premier_cycle" }),
        )
        .await;
    assert!(
        tableau(&stats_premier["examens"], "resultats_examens")["lignes"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    // L'entrée en 7e allumée : la 6e devient une classe d'examen.
    assert_eq!(
        p.ok(
            "examens_entree_7e_regler",
            Some(&admin),
            json!({ "actif": true })
        )
        .await,
        true
    );
    let ex = p
        .ok(
            "examens_lire",
            Some(&admin),
            json!({ "annee_id": null, "classe_id": classes[0] }),
        )
        .await;
    assert_eq!(ex["entree_7e"], true);
    assert_eq!(ex["classes"][0]["examen"], "entree_7e");
    assert_eq!(ex["classe_id"], classes[0]);
}

#[tokio::test]
async fn le_droit_des_examens_est_tenu() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    p.ok(
        "utilisateur_creer",
        Some(&admin),
        json!({ "identifiant": "caissier", "nom_affiche": "caissier",
            "mot_de_passe_initial": "mot-de-passe-2026", "roles": ["caissier"] }),
    )
    .await;
    let caissier = p.connecter("caissier", "mot-de-passe-2026").await;
    assert_eq!(
        p.refus("examens_lire", Some(&caissier), json!({}))
            .await
            .cle,
        "erreur.permission_refusee"
    );
    assert_eq!(
        p.refus(
            "examens_entree_7e_regler",
            Some(&caissier),
            json!({ "actif": true })
        )
        .await
        .cle,
        "erreur.permission_refusee"
    );
}
