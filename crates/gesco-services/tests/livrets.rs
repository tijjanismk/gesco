//! Livrets du jardin par la frontière (SPEC_PHASE_11_B, étape 3) : préparés, validés, publiés et
//! imprimés comme des bulletins ; un enfant sans rien a le sien ; le livret fige ce qu'il imprime ;
//! la publication met « à remettre » et l'avis dit « livret », une seule fois ; le livret du
//! 2e trimestre montre aussi le 1er, celui d'un enfant arrivé au 2e ne montre que le 2e.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use crate::commun::MOT_DE_PASSE;
use crate::observations::{demande, ecole, inscrire};
use serde_json::{Value, json};

/// Jours de 24 heures, en millisecondes.
const JOUR: i64 = 86_400_000;

fn livret_de<'a>(lus: &'a Value, prenom: &str) -> &'a Value {
    lus["bulletins"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| {
            a["bulletin"]["contenu"]["eleve"]
                .as_str()
                .unwrap()
                .starts_with(prenom)
        })
        .unwrap_or_else(|| panic!("pas de livret pour {prenom}"))
}

fn avis(file: &Value) -> Vec<String> {
    file["messages"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|m| m["message"]["nature"] == "bulletin")
        .map(|m| m["message"]["texte"].as_str().unwrap().to_owned())
        .collect()
}

#[tokio::test]
async fn le_livret_se_prepare_se_publie_et_suit_l_annee() {
    let e = ecole().await;
    let p = &e.p;
    let admin = &e.admin;
    p.ok(
        "parametre_definir",
        Some(admin),
        json!({ "cle": "sms.actif", "valeur": { "type": "booleen", "valeur": true } }),
    )
    .await;
    p.ok(
        "parametre_definir",
        Some(admin),
        json!({ "cle": "sms.canal", "valeur": { "type": "texte", "valeur": "simule" } }),
    )
    .await;
    let classe = demande(&e.ms_a, &e.trimestre);
    let g = p
        .ok("observations_grille", Some(admin), classe.clone())
        .await;
    let aminata = g["enfants"][0]["inscription_id"].clone();
    let c1 = g["competences"][0].clone();
    let c2 = g["competences"][1].clone();
    let observer = |competence: &Value, niveau: &str, lue_le: i64, periode: &Value| {
        json!({ "classe_id": e.ms_a, "periode_id": periode, "competence_id": competence["id"],
            "inscription_id": aminata, "niveau": niveau, "lue_le": lue_le })
    };
    let premiere = p
        .ok(
            "observation_saisir",
            Some(admin),
            observer(&c1, "en_cours", 0, &e.trimestre),
        )
        .await;
    p.ok(
        "observation_saisir",
        Some(admin),
        observer(&c2, "acquis", 0, &e.trimestre),
    )
    .await;
    p.ok(
        "observation_apprecier",
        Some(admin),
        json!({ "classe_id": e.ms_a, "periode_id": e.trimestre, "inscription_id": aminata,
            "domaine_id": null, "texte": "Un bon trimestre." }),
    )
    .await;

    // Critère 6 : tous les enfants ont un livret, même sans observation ni appréciation.
    let lus = p
        .ok("resultats_calculer", Some(admin), classe.clone())
        .await;
    assert_eq!(lus["cycle"], "jardin");
    assert_eq!(lus["bulletins"].as_array().unwrap().len(), 3);
    let a = &livret_de(&lus, "Aminata")["bulletin"];
    assert_eq!(a["contenu"]["forme"], "livret");
    assert!(a["contenu"]["moyenne"].is_null());
    assert_eq!(a["regle_version"], 0);
    let l = &a["contenu"]["livret"];
    assert_eq!(l["periodes"], json!(["1er trimestre"]));
    assert_eq!(l["domaines"].as_array().unwrap().len(), 1);
    assert_eq!(l["domaines"][0]["libelle"], "Langage");
    assert_eq!(
        l["domaines"][0]["competences"][0]["niveaux"],
        json!(["en_cours"])
    );
    // Une compétence jamais observée ne s'imprime pas (critère 7).
    assert_eq!(l["domaines"][0]["competences"].as_array().unwrap().len(), 2);
    // L'école ne relève pas les absences du jardin : pas de ligne d'absences.
    assert_eq!(l["absences_relevees"], false);
    assert_eq!(a["contenu"]["appreciation_generale"], "Un bon trimestre.");
    let vide = &livret_de(&lus, "Bakary")["bulletin"]["contenu"]["livret"];
    assert_eq!(vide["domaines"], json!([]));

    // Validé, publié, imprimé ; la remise naît « à remettre », l'avis dit « livret ».
    assert_eq!(
        p.ok("bulletins_valider", Some(admin), classe.clone()).await,
        3
    );
    let publie = p.ok("bulletins_publier", Some(admin), classe.clone()).await;
    assert_eq!(publie["document"]["type_document"], "livret");
    assert!(publie["pdf"].as_str().unwrap().len() > 1000);
    let remises = p
        .ok(
            "remises_classe_lire",
            Some(admin),
            json!({ "classe_id": e.ms_a, "periode_id": e.trimestre }),
        )
        .await;
    assert_eq!(remises["lignes"].as_array().unwrap().len(), 3);
    assert!(
        remises["lignes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|l| l["remise"]["statut"] == "a_remettre"
                && l["periode"] == "Livret du 1er trimestre")
    );
    let file = p
        .ok("messages_lire", Some(admin), json!({ "etat": null }))
        .await;
    let textes = avis(&file);
    assert!(!textes.is_empty(), "{file}");
    assert!(
        textes
            .iter()
            .all(|t| t.contains("le livret de 1er trimestre") && !t.contains("bulletin")),
        "{textes:?}"
    );
    let un = p
        .ok(
            "bulletin_produire",
            Some(admin),
            json!({ "id": livret_de(&lus, "Bakary")["bulletin"]["id"] }),
        )
        .await;
    assert_eq!(un["document"]["type_document"], "livret");

    // Critère 2 et RB-6 : publié, le livret ne change pas quand les observations changent.
    p.horloge.avancer(1_000);
    p.ok(
        "observation_saisir",
        Some(admin),
        observer(
            &c1,
            "acquis",
            premiere["modifiee_le"].as_i64().unwrap(),
            &e.trimestre,
        ),
    )
    .await;
    let relus = p.ok("bulletins_lire", Some(admin), classe.clone()).await;
    assert_eq!(
        livret_de(&relus, "Aminata")["bulletin"]["contenu"]["livret"]["domaines"][0]["competences"]
            [0]["niveaux"],
        json!(["en_cours"])
    );
    // Corrigé puis republié : une version 2, et aucun avis de plus (critère 9).
    let corrige = p
        .ok(
            "bulletin_corriger",
            Some(admin),
            json!({ "id": livret_de(&relus, "Aminata")["bulletin"]["id"], "motif": "Observation revue" }),
        )
        .await;
    assert_eq!(corrige["version"], 2);
    p.ok("bulletins_valider", Some(admin), classe.clone()).await;
    p.ok("bulletins_publier", Some(admin), classe.clone()).await;
    let apres = avis(
        &p.ok("messages_lire", Some(admin), json!({ "etat": null }))
            .await,
    );
    assert_eq!(apres.len(), textes.len(), "pas de second avis");
    let relus = p.ok("bulletins_lire", Some(admin), classe.clone()).await;
    assert_eq!(
        livret_de(&relus, "Aminata")["bulletin"]["contenu"]["livret"]["domaines"][0]["competences"]
            [0]["niveaux"],
        json!(["acquis"])
    );

    // Critère 8 : au 2e trimestre, le livret montre le 1er et le 2e ; un enfant arrivé au 2e
    // trimestre ne montre que le 2e.
    let ped = p
        .ok(
            "pedagogie_lire",
            Some(admin),
            json!({ "id": p.ok("annees_lister", Some(admin), Value::Null).await[0]["id"] }),
        )
        .await;
    let niveau_ms = g["periode"]["niveau_id"].clone();
    let t2 = ped["periodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["periode"]["niveau_id"] == niveau_ms && x["periode"]["rang"] == 2)
        .unwrap()["periode"]["id"]
        .clone();
    p.horloge.avancer(105 * JOUR);
    // Trois mois plus tard, la session de la direction a expiré : elle se reconnecte.
    let reconnectee = p.connecter("admin", MOT_DE_PASSE).await;
    let admin = &reconnectee;
    inscrire(p, admin, "Sidibé", "Djeneba", &e.ms_a).await;
    let bilan = p
        .ok(
            "inscriptions_valider_classe",
            Some(admin),
            json!({ "id": e.ms_a }),
        )
        .await;
    assert_eq!(bilan["validees"], 1, "{bilan}");
    p.ok(
        "observation_saisir",
        Some(admin),
        observer(&c2, "non_acquis", 0, &t2),
    )
    .await;
    let deuxieme = p
        .ok("resultats_calculer", Some(admin), demande(&e.ms_a, &t2))
        .await;
    let l = &livret_de(&deuxieme, "Aminata")["bulletin"]["contenu"]["livret"];
    assert_eq!(l["periodes"], json!(["1er trimestre", "2e trimestre"]));
    let langage = &l["domaines"][0]["competences"];
    let du = |libelle: &Value| {
        langage
            .as_array()
            .unwrap()
            .iter()
            .find(|k| k["libelle"] == *libelle)
            .unwrap()["niveaux"]
            .clone()
    };
    assert_eq!(du(&c1["libelle"]), json!(["acquis", null]));
    assert_eq!(du(&c2["libelle"]), json!(["acquis", "non_acquis"]));
    let arrivee = &livret_de(&deuxieme, "Djeneba")["bulletin"]["contenu"]["livret"];
    assert_eq!(arrivee["periodes"], json!(["2e trimestre"]));
    // Elle n'a pas de livret du 1er trimestre.
    let premier = p.ok("bulletins_lire", Some(admin), classe.clone()).await;
    assert!(!premier["bulletins"].as_array().unwrap().iter().any(|a| {
        a["bulletin"]["contenu"]["eleve"]
            .as_str()
            .unwrap()
            .starts_with("Djeneba")
    }));
}

/// Tables dont la mise à niveau ne doit rien changer, et leur contenu lu en clair.
const TABLES_GARDEES: [&str; 9] = [
    "personne",
    "eleve",
    "inscription",
    "facture",
    "ligne_facture",
    "echeance",
    "versement",
    "bulletin",
    "classe",
];

async fn instantane(p: &crate::commun::Poste_) -> Vec<(String, i64, String)> {
    use sqlx::Connection;
    let mut cnx = crate::commun::base_brute(&p.racine, p.trousseau.clone()).await;
    let mut lignes = Vec::new();
    for table in TABLES_GARDEES {
        let n: i64 =
            sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {table}")))
                .fetch_one(&mut cnx)
                .await
                .unwrap();
        // L'empreinte des identifiants et des dates de modification dit qu'aucune ligne n'a bougé.
        let empreinte: String = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT coalesce(group_concat(id || ':' || coalesce(mis_a_jour_le, ''), ','), '') FROM
             (SELECT * FROM {table} ORDER BY id)"
        )))
        .fetch_one(&mut cnx)
        .await
        .unwrap_or_default();
        lignes.push((table.to_owned(), n, empreinte));
    }
    cnx.close().await.unwrap();
    lignes
}

/// Critère 14 : une école de la 2.0.0, avec des enfants au jardin, passe la mise à niveau sans
/// rien perdre ; ses classes du jardin reçoivent leurs trimestres et la liste livrée.
#[tokio::test]
async fn une_ecole_de_la_2_0_0_passe_la_mise_a_niveau_sans_rien_perdre() {
    use sqlx::Connection;
    let e = ecole().await;
    // La 2.0.0 ne connaissait ni compétences ni trimestres du jardin : on les retire.
    {
        let mut cnx = crate::commun::base_brute(&e.p.racine, e.p.trousseau.clone()).await;
        for sql in [
            "DROP TRIGGER competence_sans_suppression",
            "DROP TRIGGER domaine_competence_sans_suppression",
            "DELETE FROM competence",
            "DELETE FROM domaine_competence",
            "DELETE FROM periode_academique WHERE niveau_id IN
                (SELECT id FROM niveau WHERE cycle = 'jardin')",
        ] {
            sqlx::query(sqlx::AssertSqlSafe(sql.to_owned()))
                .execute(&mut cnx)
                .await
                .unwrap();
        }
        cnx.close().await.unwrap();
    }
    let avant = instantane(&e.p).await;
    assert!(avant.iter().any(|(t, n, _)| t == "inscription" && *n >= 4));
    // GESCO se ferme, la nouvelle version l'ouvre.
    e.p.application.fermer().await.unwrap();
    let p = crate::commun::demarrer(&e.p.racine, e.p.trousseau.clone(), e.p.horloge.clone()).await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    assert_eq!(instantane(&p).await, avant, "rien d'existant ne change");
    let j = p.ok("jardin_lire", Some(&admin), Value::Null).await;
    assert_eq!(j["domaines"].as_array().unwrap().len(), 5);
    assert_eq!(j["competences"].as_array().unwrap().len(), 60);
    let annee = p.ok("annees_lister", Some(&admin), Value::Null).await[0]["id"].clone();
    let ped = p
        .ok("pedagogie_lire", Some(&admin), json!({ "id": annee }))
        .await;
    for section in j["sections"].as_array().unwrap() {
        assert_eq!(
            ped["periodes"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|x| x["periode"]["niveau_id"] == section["id"])
                .count(),
            3,
            "{}",
            section["code"]
        );
    }
    // La classe du jardin s'observe et prépare ses livrets dès l'ouverture.
    let premier = ped["periodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| {
            x["periode"]["rang"] == 1
                && j["sections"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|s| s["code"] == "MS" && s["id"] == x["periode"]["niveau_id"])
        })
        .unwrap()["periode"]["id"]
        .clone();
    let g = p
        .ok(
            "observations_grille",
            Some(&admin),
            demande(&e.ms_a, &premier),
        )
        .await;
    assert_eq!(g["enfants"].as_array().unwrap().len(), 3);
    let livrets = p
        .ok(
            "resultats_calculer",
            Some(&admin),
            demande(&e.ms_a, &premier),
        )
        .await;
    assert_eq!(livrets["bulletins"].as_array().unwrap().len(), 3);
}
