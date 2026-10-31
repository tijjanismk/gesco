//! Tests de bout en bout de la phase 2 par la frontière (SPEC_PHASE_2 §0.4) : niveaux, séries,
//! classes, élèves, responsables, inscriptions.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use crate::commun::*;
use gesco_services::session::Jeton;
use serde_json::{Value, json};

/// Poste installé, administrateur connecté, année 2026-2027 ouverte.
async fn ecole(dossier: &std::path::Path) -> (Poste_, Jeton, Value) {
    let p = poste_vierge(dossier).await;
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
    (p, admin, annee)
}

fn par_code<'a>(liste: &'a Value, code: &str) -> &'a Value {
    liste
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["code"] == code)
        .unwrap_or_else(|| panic!("{code} absent"))
}

#[tokio::test]
async fn niveaux_series_et_classes_de_l_annee() {
    let dossier = tempfile::tempdir().unwrap();
    let (p, admin, annee) = ecole(dossier.path()).await;

    // Assertion 1 : quinze niveaux de l'enseignement général, six du CAP et du BT, et huit séries
    // livrés.
    let referentiel = p.ok("niveaux_lister", Some(&admin), Value::Null).await;
    let niveaux = &referentiel["niveaux"];
    assert_eq!(niveaux.as_array().unwrap().len(), 21);
    assert_eq!(par_code(niveaux, "BT4")["diplome_terminal"], "bt");
    assert_eq!(referentiel["series"].as_array().unwrap().len(), 8);
    assert_eq!(niveaux[0]["code"], "PS");
    assert_eq!(niveaux[0]["cycle"], "jardin");
    assert_eq!(niveaux[0]["mode_evaluation"], "competences");
    assert_eq!(par_code(niveaux, "12")["diplome_terminal"], "bac");

    let sept = par_code(niveaux, "7").clone();
    let renomme = p
        .ok(
            "niveau_modifier",
            Some(&admin),
            json!({ "id": sept["id"], "libelle": "  7ème   année ", "actif": true }),
        )
        .await;
    assert_eq!(renomme["libelle"], "7ème année");
    assert_eq!(renomme["code"], "7");
    let ps = par_code(niveaux, "PS").clone();
    p.ok(
        "niveau_modifier",
        Some(&admin),
        json!({ "id": ps["id"], "libelle": ps["libelle"], "actif": false }),
    )
    .await;

    // Assertion 2 : série exigée en 11e et 12e, refusée ailleurs.
    let onze = par_code(niveaux, "11").clone();
    let douze = par_code(niveaux, "12").clone();
    let series = &referentiel["series"];
    let s = series
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["code"] == "S")
        .unwrap()
        .clone();
    let tse = series
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["code"] == "TSE")
        .unwrap()
        .clone();
    let creer = |niveau: &Value, serie: Value, libelle: &str| json!({ "annee_id": annee["id"], "niveau_id": niveau["id"], "serie_id": serie, "libelle": libelle });
    for (entree, cle) in [
        (creer(&onze, Value::Null, "11"), "classe.serie_exigee"),
        (
            creer(&onze, tse["id"].clone(), "11TSE"),
            "classe.serie_hors_niveau",
        ),
        (
            creer(&sept, s["id"].clone(), "7S"),
            "classe.serie_hors_niveau",
        ),
        (creer(&ps, Value::Null, "Petits"), "classe.niveau_inactif"),
        (creer(&sept, Value::Null, "   "), "classe.libelle_invalide"),
    ] {
        assert_eq!(p.refus("classe_creer", Some(&admin), entree).await.cle, cle);
    }
    let septa = p
        .ok(
            "classe_creer",
            Some(&admin),
            creer(&sept, Value::Null, "7e A"),
        )
        .await;
    p.ok(
        "classe_creer",
        Some(&admin),
        creer(&onze, s["id"].clone(), "11S"),
    )
    .await;
    p.ok(
        "classe_creer",
        Some(&admin),
        creer(&douze, tse["id"].clone(), "12TSE"),
    )
    .await;
    assert_eq!(
        p.refus(
            "classe_creer",
            Some(&admin),
            creer(&sept, Value::Null, "7e A")
        )
        .await
        .cle,
        "classe.libelle_pris"
    );

    let classes = p
        .ok(
            "classes_lister",
            Some(&admin),
            json!({ "annee_id": annee["id"] }),
        )
        .await;
    let libelles: Vec<&str> = classes
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["classe"]["libelle"].as_str().unwrap())
        .collect();
    assert_eq!(libelles, ["7e A", "11S", "12TSE"], "ordre des niveaux");
    assert_eq!(classes[1]["serie"], "S");
    assert_eq!(classes[0]["niveau"], "7ème année");

    p.ok(
        "classe_modifier",
        Some(&admin),
        json!({ "id": septa["id"], "libelle": "7e B" }),
    )
    .await;
    p.ok(
        "classe_supprimer",
        Some(&admin),
        json!({ "id": septa["id"] }),
    )
    .await;
    // Une classe supprimée libère son libellé.
    p.ok(
        "classe_creer",
        Some(&admin),
        creer(&sept, Value::Null, "7e B"),
    )
    .await;

    // La secrétaire voit les classes, ne les organise pas.
    p.ok(
        "utilisateur_creer",
        Some(&admin),
        json!({
            "identifiant": "fanta",
            "nom_affiche": "Fanta Coulibaly",
            "mot_de_passe_initial": "secretariat-2026",
            "roles": ["secretaire"],
        }),
    )
    .await;
    let secretaire = p.connecter("fanta", "secretariat-2026").await;
    p.ok(
        "classes_lister",
        Some(&secretaire),
        json!({ "annee_id": annee["id"] }),
    )
    .await;
    assert_eq!(
        p.refus(
            "classe_creer",
            Some(&secretaire),
            creer(&sept, Value::Null, "7e C")
        )
        .await
        .cle,
        "erreur.permission_refusee"
    );
    let courante = p
        .ok("session_courante", Some(&secretaire), Value::Null)
        .await;
    let permissions: Vec<String> = serde_json::from_value(courante["permissions"].clone()).unwrap();
    assert!(permissions.contains(&"enrollments:validate".to_owned()));

    let journal = p
        .ok(
            "audit_lister",
            Some(&admin),
            json!({ "entite": "classe", "page": 0 }),
        )
        .await;
    assert_eq!(journal["lignes"].as_array().unwrap().len(), 6);
    p.ok("audit_verifier", Some(&admin), Value::Null).await;
}

#[tokio::test]
async fn une_ecole_n_enseigne_que_ses_cycles() {
    let dossier = tempfile::tempdir().unwrap();
    let (p, admin, annee) = ecole(dossier.path()).await;
    assert_eq!(
        p.refus("cycles_definir", Some(&admin), json!({ "cycles": [] }))
            .await
            .cle,
        "referentiel.aucun_cycle"
    );
    // Une école fondamentale du 1er cycle seulement.
    let referentiel = p
        .ok(
            "cycles_definir",
            Some(&admin),
            json!({ "cycles": ["premier_cycle"] }),
        )
        .await;
    let actifs: Vec<&str> = referentiel["niveaux"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|n| n["actif"] == true)
        .map(|n| n["code"].as_str().unwrap())
        .collect();
    assert_eq!(actifs, ["1", "2", "3", "4", "5", "6"]);
    let sept = par_code(&referentiel["niveaux"], "7").clone();
    assert_eq!(
        p.refus(
            "classe_creer",
            Some(&admin),
            json!({ "annee_id": annee["id"], "niveau_id": sept["id"], "serie_id": null, "libelle": "7e A" }),
        )
        .await
        .cle,
        "classe.niveau_inactif"
    );
    // Elle ouvre ensuite le 2e cycle : les niveaux 7 à 9 reviennent.
    let referentiel = p
        .ok(
            "cycles_definir",
            Some(&admin),
            json!({ "cycles": ["premier_cycle", "second_cycle"] }),
        )
        .await;
    assert_eq!(
        referentiel["niveaux"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|n| n["actif"] == true)
            .count(),
        9
    );
    p.ok("audit_verifier", Some(&admin), Value::Null).await;
}

fn identite(nom: &str, prenoms: &str, sexe: Value, naissance: &str) -> Value {
    json!({
        "nom": nom, "prenoms": prenoms, "sexe": sexe, "date_naissance": naissance,
        "date_naissance_estimee": false, "lieu_naissance": "Bamako", "nationalite": "Malienne",
        "adresse": "", "telephones": [],
    })
}

fn saisie_eleve(identite: Value) -> Value {
    json!({
        "identite": identite, "matricule_interne": "", "matricule_national": "", "acte_nature": "acte",
        "acte_numero": "", "acte_date": "", "ecole_origine": "",
    })
}

fn qualites(lien: &str, legal: bool, payeur: bool, ordre: i64) -> Value {
    json!({
        "lien": lien, "est_responsable_legal": legal, "est_payeur": payeur,
        "recoit_notifications": true, "vient_chercher": false, "ordre_priorite": ordre,
    })
}

#[tokio::test]
async fn fiche_eleve_matricule_responsables_et_photo() {
    let dossier = tempfile::tempdir().unwrap();

    // Sans année courante, pas de millésime pour le matricule.
    let sans_annee = poste_vierge(&dossier.path().join("sans-annee")).await;
    sans_annee.installer().await;
    let admin_sans_annee = sans_annee.connecter("admin", MOT_DE_PASSE).await;
    assert_eq!(
        sans_annee
            .refus(
                "eleve_creer",
                Some(&admin_sans_annee),
                saisie_eleve(identite("Traoré", "Awa", json!("feminin"), "2014-03-02")),
            )
            .await
            .cle,
        "eleve.annee_absente"
    );

    let (p, admin, _annee) = ecole(&dossier.path().join("ecole")).await;
    for (saisie, cle) in [
        (
            identite("", "Awa", json!("feminin"), "2014-03-02"),
            "eleve.nom_exige",
        ),
        (
            identite("Traoré", "Awa", Value::Null, "2014-03-02"),
            "eleve.sexe_exige",
        ),
        (
            identite("Traoré", "Awa", json!("feminin"), ""),
            "eleve.date_naissance_exigee",
        ),
        (
            identite("Traoré", "Awa", json!("feminin"), "2014-02-30"),
            "personne.date_invalide",
        ),
        (
            identite("Traoré", "Awa", json!("feminin"), "2030-01-01"),
            "eleve.date_naissance_future",
        ),
    ] {
        assert_eq!(
            p.refus("eleve_creer", Some(&admin), saisie_eleve(saisie))
                .await
                .cle,
            cle
        );
    }

    // Assertion 3 : deux homonymes nés le même jour coexistent, matricules sans trou.
    let awa = p
        .ok(
            "eleve_creer",
            Some(&admin),
            saisie_eleve(identite("  Traoré ", "Awa", json!("feminin"), "2014-03-02")),
        )
        .await;
    let homonyme = p
        .ok(
            "eleve_creer",
            Some(&admin),
            saisie_eleve(identite("Traoré", "Awa", json!("feminin"), "2014-03-02")),
        )
        .await;
    assert_eq!(awa["eleve"]["matricule_interne"], "2026/00001");
    assert_eq!(homonyme["eleve"]["matricule_interne"], "2026/00002");
    assert_eq!(awa["identite"]["nom"], "Traoré");
    let trouves = p
        .ok(
            "eleves_rechercher",
            Some(&admin),
            json!({ "texte": "traore aw" }),
        )
        .await;
    assert_eq!(trouves.as_array().unwrap().len(), 2);
    let par_matricule = p
        .ok(
            "eleves_rechercher",
            Some(&admin),
            json!({ "texte": "2026/00002" }),
        )
        .await;
    assert_eq!(par_matricule[0]["id"], homonyme["eleve"]["id"]);

    // Matricule national unique parmi les valeurs renseignées.
    let mut saisie = saisie_eleve(identite("Traoré", "Awa", json!("feminin"), "2014-03-02"));
    saisie["matricule_national"] = json!(" ml 2026 001 ");
    let rectifiee = p
        .ok(
            "eleve_modifier",
            Some(&admin),
            json!({ "id": awa["eleve"]["id"], "saisie": saisie }),
        )
        .await;
    assert_eq!(rectifiee["eleve"]["matricule_national"], "ML2026001");
    assert_eq!(rectifiee["eleve"]["matricule_interne"], "2026/00001");
    assert_eq!(
        p.refus(
            "eleve_modifier",
            Some(&admin),
            json!({ "id": homonyme["eleve"]["id"], "saisie": saisie }),
        )
        .await
        .cle,
        "eleve.matricule_national_pris"
    );

    // Assertion 5 : téléphones canoniques. Assertion 4 : responsable réutilisé.
    let mut mere = json!({
        "identite": identite("Coulibaly", "Mariam", json!("feminin"), ""),
        "profession": "Commerçante", "employeur": "",
    });
    mere["identite"]["telephones"] = json!(["76 12 34 56", "+223 76.12.34.56"]);
    let sans_telephone = json!({
        "identite": identite("Coulibaly", "Mariam", Value::Null, ""),
        "profession": "", "employeur": "",
    });
    assert_eq!(
        p.refus(
            "responsable_rattacher",
            Some(&admin),
            json!({ "eleve_id": awa["eleve"]["id"], "responsable_id": null, "saisie": sans_telephone, "qualites": qualites("mere", true, true, 1) }),
        )
        .await
        .cle,
        "responsable.telephone_exige"
    );
    let fiche = p
        .ok(
            "responsable_rattacher",
            Some(&admin),
            json!({ "eleve_id": awa["eleve"]["id"], "responsable_id": null, "saisie": mere, "qualites": qualites("mere", true, true, 1) }),
        )
        .await;
    let responsable = &fiche["responsables"][0];
    assert_eq!(
        responsable["identite"]["telephones"],
        json!(["+22376123456"])
    );
    let responsable_id = responsable["responsable"]["id"].clone();

    let trouves = p
        .ok(
            "responsables_rechercher",
            Some(&admin),
            json!({ "texte": "0022376123456" }),
        )
        .await;
    assert_eq!(trouves.as_array().unwrap().len(), 1);
    assert_eq!(trouves[0]["enfants"][0]["nom"], "Awa Traoré");
    assert_eq!(trouves[0]["enfants"][0]["id"], awa["eleve"]["id"]);
    // Pour le frère, la mère est responsable légale mais pas payeuse : qualités propres au lien.
    let frere = p
        .ok(
            "eleve_creer",
            Some(&admin),
            saisie_eleve(identite(
                "Traoré",
                "Moussa",
                json!("masculin"),
                "2016-09-12",
            )),
        )
        .await;
    let fiche_frere = p
        .ok(
            "responsable_rattacher",
            Some(&admin),
            json!({ "eleve_id": frere["eleve"]["id"], "responsable_id": responsable_id, "saisie": null, "qualites": qualites("mere", true, false, 1) }),
        )
        .await;
    assert_eq!(
        fiche_frere["responsables"][0]["qualites"]["est_payeur"],
        false
    );
    assert_eq!(
        p.ok(
            "eleve_lire",
            Some(&admin),
            json!({ "id": awa["eleve"]["id"] })
        )
        .await["responsables"][0]["qualites"]["est_payeur"],
        true
    );
    assert_eq!(
        p.refus(
            "responsable_rattacher",
            Some(&admin),
            json!({ "eleve_id": frere["eleve"]["id"], "responsable_id": responsable_id, "saisie": null, "qualites": qualites("mere", true, false, 1) }),
        )
        .await
        .cle,
        "responsable.deja_lie"
    );
    let trouves = p
        .ok(
            "responsables_rechercher",
            Some(&admin),
            json!({ "texte": "coul mar" }),
        )
        .await;
    assert_eq!(trouves[0]["enfants"].as_array().unwrap().len(), 2);

    // Rectifier, puis détacher : le responsable reste pour la fratrie.
    let mut rectifiee = mere.clone();
    rectifiee["identite"]["telephones"] = json!(["66 00 11 22"]);
    let fiche = p
        .ok(
            "responsable_modifier",
            Some(&admin),
            json!({ "eleve_id": frere["eleve"]["id"], "responsable_id": responsable_id, "saisie": rectifiee, "qualites": qualites("mere", true, true, 1) }),
        )
        .await;
    assert_eq!(
        fiche["responsables"][0]["identite"]["telephones"],
        json!(["+22366001122"])
    );
    let fiche = p
        .ok(
            "responsable_detacher",
            Some(&admin),
            json!({ "eleve_id": frere["eleve"]["id"], "responsable_id": responsable_id }),
        )
        .await;
    assert_eq!(fiche["responsables"].as_array().unwrap().len(), 0);
    assert_eq!(
        p.ok(
            "eleve_lire",
            Some(&admin),
            json!({ "id": awa["eleve"]["id"] })
        )
        .await["responsables"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    // Photo : PNG ou JPEG de 100 Ko au plus.
    let fiche = p
        .ok(
            "eleve_photo_definir",
            Some(&admin),
            json!({ "id": awa["eleve"]["id"], "contenu_base64": PNG_ESSAI }),
        )
        .await;
    assert!(
        fiche["photo"]
            .as_str()
            .unwrap()
            .starts_with("data:image/png;base64,")
    );
    let lourde = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        [b"\x89PNG\r\n\x1a\n".as_slice(), &vec![0_u8; 101 * 1024]].concat(),
    );
    for (contenu, cle) in [
        (lourde, "eleve.photo_trop_lourde"),
        ("R0lGODlhAQABAAAAACw=".to_owned(), "eleve.photo_format"),
    ] {
        assert_eq!(
            p.refus(
                "eleve_photo_definir",
                Some(&admin),
                json!({ "id": awa["eleve"]["id"], "contenu_base64": contenu }),
            )
            .await
            .cle,
            cle
        );
    }

    // Un caissier lit les fiches, ne les crée pas.
    p.ok(
        "utilisateur_creer",
        Some(&admin),
        json!({
            "identifiant": "oumar",
            "nom_affiche": "Oumar Diallo",
            "mot_de_passe_initial": "encaissement-2026",
            "roles": ["caissier"],
        }),
    )
    .await;
    let caissier = p.connecter("oumar", "encaissement-2026").await;
    p.ok(
        "eleve_lire",
        Some(&caissier),
        json!({ "id": awa["eleve"]["id"] }),
    )
    .await;
    assert_eq!(
        p.refus(
            "eleve_creer",
            Some(&caissier),
            saisie_eleve(identite("Keïta", "Fanta", json!("feminin"), "2015-01-09")),
        )
        .await
        .cle,
        "erreur.permission_refusee"
    );
    p.ok("audit_verifier", Some(&admin), Value::Null).await;
}

#[tokio::test]
async fn matricule_selon_les_regles_de_l_ecole_ou_saisi() {
    let dossier = tempfile::tempdir().unwrap();
    let (p, admin, _annee) = ecole(dossier.path()).await;
    let regler = |cle: &str, valeur: Value| json!({ "cle": cle, "valeur": valeur });
    assert_eq!(
        p.refus(
            "parametre_definir",
            Some(&admin),
            regler(
                "matricule.prefixe",
                json!({ "type": "texte", "valeur": "AS-K" })
            ),
        )
        .await
        .cle,
        "parametre.hors_bornes"
    );
    for (cle, valeur) in [
        (
            "matricule.prefixe",
            json!({ "type": "texte", "valeur": "ASK" }),
        ),
        (
            "matricule.chiffres_annee",
            json!({ "type": "entier", "valeur": 2 }),
        ),
        (
            "matricule.chiffres",
            json!({ "type": "entier", "valeur": 3 }),
        ),
        (
            "matricule.separateur",
            json!({ "type": "texte", "valeur": "-" }),
        ),
    ] {
        p.ok("parametre_definir", Some(&admin), regler(cle, valeur))
            .await;
    }
    let genere = p
        .ok(
            "eleve_creer",
            Some(&admin),
            saisie_eleve(identite("Keïta", "Fanta", json!("feminin"), "2015-01-09")),
        )
        .await;
    assert_eq!(genere["eleve"]["matricule_interne"], "ASK-26-001");
    assert_eq!(
        p.ok("scolarite_reglages", Some(&admin), Value::Null).await["matricule_manuel"],
        false
    );

    // L'école saisit désormais ses matricules elle-même.
    p.ok(
        "parametre_definir",
        Some(&admin),
        regler(
            "matricule.saisie_manuelle",
            json!({ "type": "booleen", "valeur": true }),
        ),
    )
    .await;
    assert_eq!(
        p.ok("scolarite_reglages", Some(&admin), Value::Null).await["matricule_manuel"],
        true
    );
    let mut saisie = saisie_eleve(identite(
        "Diarra",
        "Seydou",
        json!("masculin"),
        "2013-06-21",
    ));
    for (matricule, cle) in [
        ("  ", "eleve.matricule_invalide"),
        ("ASK-26-001", "eleve.matricule_pris"),
    ] {
        saisie["matricule_interne"] = json!(matricule);
        assert_eq!(
            p.refus("eleve_creer", Some(&admin), saisie.clone())
                .await
                .cle,
            cle
        );
    }
    saisie["matricule_interne"] = json!(" L/2013/042 ");
    let saisi = p.ok("eleve_creer", Some(&admin), saisie.clone()).await;
    assert_eq!(saisi["eleve"]["matricule_interne"], "L/2013/042");
    // Une faute de frappe se corrige ; le matricule reste unique.
    saisie["matricule_interne"] = json!("L/2013/043");
    let corrige = p
        .ok(
            "eleve_modifier",
            Some(&admin),
            json!({ "id": saisi["eleve"]["id"], "saisie": saisie }),
        )
        .await;
    assert_eq!(corrige["eleve"]["matricule_interne"], "L/2013/043");
    saisie["matricule_interne"] = json!("ASK-26-001");
    assert_eq!(
        p.refus(
            "eleve_modifier",
            Some(&admin),
            json!({ "id": saisi["eleve"]["id"], "saisie": saisie }),
        )
        .await
        .cle,
        "eleve.matricule_pris"
    );
    p.ok("audit_verifier", Some(&admin), Value::Null).await;
}

#[tokio::test]
async fn recherche_tolerante_et_doublons_probables() {
    let dossier = tempfile::tempdir().unwrap();
    let (p, admin, _annee) = ecole(dossier.path()).await;
    for (nom, prenoms, sexe, naissance) in [
        ("Coulibaly", "Mohamed", "masculin", "2013-05-04"),
        ("Keïta", "Fatoumata", "feminin", "2014-11-20"),
        ("Diallo", "Oumar", "masculin", "2012-01-15"),
        ("Coulibaly", "Mariam", "feminin", "2015-02-09"),
    ] {
        p.ok(
            "eleve_creer",
            Some(&admin),
            saisie_eleve(identite(nom, prenoms, json!(sexe), naissance)),
        )
        .await;
    }
    let noms = |resultat: &Value| -> Vec<String> {
        resultat
            .as_array()
            .unwrap()
            .iter()
            .map(|e| {
                format!(
                    "{} {}",
                    e["nom"].as_str().unwrap(),
                    e["prenoms"].as_str().unwrap()
                )
            })
            .collect()
    };
    let chercher = |texte: &'static str| {
        let p = &p;
        let admin = &admin;
        async move {
            p.ok("eleves_rechercher", Some(admin), json!({ "texte": texte }))
                .await
        }
    };
    // Assertion 6 : orthographes proches, accents, prénoms équivalents.
    assert_eq!(
        noms(&chercher("Kouloubaly Mahamadou").await),
        ["Coulibaly Mohamed"]
    );
    assert_eq!(noms(&chercher("keita").await), ["Keïta Fatoumata"]);
    assert_eq!(noms(&chercher("Jallo").await), ["Diallo Oumar"]);
    assert_eq!(noms(&chercher("Fatimata").await), ["Keïta Fatoumata"]);
    assert!(chercher("Sangaré").await.as_array().unwrap().is_empty());
    // L'orthographe exacte passe devant l'orthographe proche.
    let coulibaly = noms(&chercher("Coulibaly").await);
    assert_eq!(coulibaly.len(), 2);

    // Assertion 6 : doublon proposé à la création, jamais fusionné.
    let doublons = p
        .ok(
            "eleve_doublons_probables",
            Some(&admin),
            json!({ "nom": "KOULIBALY", "prenoms": "Mamadou", "date_naissance": "2013-09-01", "sauf": null }),
        )
        .await;
    assert_eq!(noms(&doublons), ["Coulibaly Mohamed"]);
    let aucun = p
        .ok(
            "eleve_doublons_probables",
            Some(&admin),
            json!({ "nom": "Coulibaly", "prenoms": "Mohamed", "date_naissance": "2016-09-01", "sauf": null }),
        )
        .await;
    assert!(aucun.as_array().unwrap().is_empty(), "trois ans d'écart");
    let cree = p
        .ok(
            "eleve_creer",
            Some(&admin),
            saisie_eleve(identite(
                "Koulibaly",
                "Mamadou",
                json!("masculin"),
                "2013-09-01",
            )),
        )
        .await;
    let sans_lui = p
        .ok(
            "eleve_doublons_probables",
            Some(&admin),
            json!({ "nom": "Koulibaly", "prenoms": "Mamadou", "date_naissance": "2013-09-01", "sauf": cree["eleve"]["id"] }),
        )
        .await;
    assert_eq!(noms(&sans_lui), ["Coulibaly Mohamed"]);
    assert_eq!(
        chercher("Coulibaly Mohamed")
            .await
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

/// Crée une classe de l'année par le code de son niveau (et de sa série).
async fn classe(
    p: &Poste_,
    admin: &Jeton,
    annee: &Value,
    niveau: &str,
    serie: Option<&str>,
    libelle: &str,
) -> Value {
    let referentiel = p.ok("niveaux_lister", Some(admin), Value::Null).await;
    let niveau_id = par_code(&referentiel["niveaux"], niveau)["id"].clone();
    let serie_id = serie.map_or(Value::Null, |code| {
        referentiel["series"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["code"] == code && s["niveau_id"] == niveau_id)
            .unwrap()["id"]
            .clone()
    });
    p.ok(
        "classe_creer",
        Some(admin),
        json!({ "annee_id": annee["id"], "niveau_id": niveau_id, "serie_id": serie_id, "libelle": libelle }),
    )
    .await
}

fn inscrire(eleve: &Value, classe: &Value) -> Value {
    json!({
        "eleve_id": eleve["eleve"]["id"], "classe_id": classe["id"], "date_inscription": "",
        "redoublant": false, "etablissement_origine": "",
    })
}

async fn lier_mere(
    p: &Poste_,
    admin: &Jeton,
    eleve: &Value,
    legal: bool,
    vient_chercher: bool,
) -> Value {
    let mut mere = json!({
        "identite": identite("Coulibaly", "Mariam", json!("feminin"), ""),
        "profession": "", "employeur": "",
    });
    mere["identite"]["telephones"] = json!(["76 12 34 56"]);
    let mut q = qualites("mere", legal, true, 1);
    q["vient_chercher"] = json!(vient_chercher);
    p.ok(
        "responsable_rattacher",
        Some(admin),
        json!({ "eleve_id": eleve["eleve"]["id"], "responsable_id": null, "saisie": mere, "qualites": q }),
    )
    .await
}

#[tokio::test]
async fn inscription_brouillon_validation_abandon_et_changement_de_classe() {
    let dossier = tempfile::tempdir().unwrap();
    let (p, admin, annee) = ecole(dossier.path()).await;
    let septa = classe(&p, &admin, &annee, "7", None, "7e A").await;
    let septb = classe(&p, &admin, &annee, "7", None, "7e B").await;
    let neuf = classe(&p, &admin, &annee, "9", None, "9e A").await;
    let onze = classe(&p, &admin, &annee, "11", Some("S"), "11S").await;
    let petits = classe(&p, &admin, &annee, "PS", None, "Petits").await;

    let awa = p
        .ok(
            "eleve_creer",
            Some(&admin),
            saisie_eleve(identite("Traoré", "Awa", json!("feminin"), "2014-03-02")),
        )
        .await;
    let mut hors_annee = inscrire(&awa, &septa);
    hors_annee["date_inscription"] = json!("2025-06-01");
    assert_eq!(
        p.refus("inscription_creer", Some(&admin), hors_annee)
            .await
            .cle,
        "inscription.date_hors_annee"
    );
    let brouillon = p
        .ok("inscription_creer", Some(&admin), inscrire(&awa, &septa))
        .await;
    assert_eq!(brouillon["inscription"]["statut"], "brouillon");
    assert_eq!(brouillon["inscription"]["type_inscription"], "nouvelle");
    let id = json!({ "id": brouillon["inscription"]["id"] });
    // Assertion 8 : une seule inscription vivante par année.
    assert_eq!(
        p.refus("inscription_creer", Some(&admin), inscrire(&awa, &septb))
            .await
            .cle,
        "inscription.deja_inscrit"
    );
    // Assertion 7 : sans responsable légal, pas de validation.
    assert_eq!(
        p.refus("inscription_valider", Some(&admin), id.clone())
            .await
            .cle,
        "inscription.responsable_legal_absent"
    );
    lier_mere(&p, &admin, &awa, true, false).await;
    let validee = p.ok("inscription_valider", Some(&admin), id.clone()).await;
    assert_eq!(validee["inscription"]["statut"], "validee");
    let classes = p
        .ok(
            "classes_lister",
            Some(&admin),
            json!({ "annee_id": annee["id"] }),
        )
        .await;
    let effectif = |libelle: &str| {
        classes
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["classe"]["libelle"] == libelle)
            .unwrap()["effectif"]
            .clone()
    };
    assert_eq!(effectif("7e A"), 1);
    assert_eq!(
        p.refus(
            "classe_supprimer",
            Some(&admin),
            json!({ "id": septa["id"] })
        )
        .await
        .cle,
        "classe.non_vide"
    );

    // Changement de classe : même niveau librement, autre niveau sur décision motivée.
    let changee = p
        .ok(
            "inscription_changer_classe",
            Some(&admin),
            json!({ "id": id["id"], "classe_id": septb["id"], "motif": "" }),
        )
        .await;
    assert_eq!(changee["inscription"]["classe_precedente_id"], septa["id"]);
    assert_eq!(
        p.refus(
            "inscription_changer_classe",
            Some(&admin),
            json!({ "id": id["id"], "classe_id": onze["id"], "motif": " " }),
        )
        .await
        .cle,
        "inscription.motif_obligatoire"
    );

    // 9e et lycée : matricule national exigé (Q-18 révisée).
    let moussa = p
        .ok(
            "eleve_creer",
            Some(&admin),
            saisie_eleve(identite(
                "Diarra",
                "Moussa",
                json!("masculin"),
                "2011-04-18",
            )),
        )
        .await;
    lier_mere(&p, &admin, &moussa, true, false).await;
    let neuvieme = p
        .ok("inscription_creer", Some(&admin), inscrire(&moussa, &neuf))
        .await;
    let id_neuvieme = json!({ "id": neuvieme["inscription"]["id"] });
    assert_eq!(
        p.refus("inscription_valider", Some(&admin), id_neuvieme.clone())
            .await
            .cle,
        "inscription.matricule_national_absent"
    );
    let mut saisie = saisie_eleve(identite(
        "Diarra",
        "Moussa",
        json!("masculin"),
        "2011-04-18",
    ));
    saisie["matricule_national"] = json!("DEF-2027-0042");
    p.ok(
        "eleve_modifier",
        Some(&admin),
        json!({ "id": moussa["eleve"]["id"], "saisie": saisie }),
    )
    .await;
    p.ok("inscription_valider", Some(&admin), id_neuvieme).await;

    // Jardin : photo exigée, une personne qui vient chercher l'enfant au moins.
    let petit = p
        .ok(
            "eleve_creer",
            Some(&admin),
            saisie_eleve(identite("Keïta", "Bakary", json!("masculin"), "2023-02-11")),
        )
        .await;
    lier_mere(&p, &admin, &petit, true, false).await;
    let jardin = p
        .ok("inscription_creer", Some(&admin), inscrire(&petit, &petits))
        .await;
    let id_jardin = json!({ "id": jardin["inscription"]["id"] });
    assert_eq!(
        p.refus("inscription_valider", Some(&admin), id_jardin.clone())
            .await
            .cle,
        "inscription.photo_absente"
    );
    p.ok(
        "eleve_photo_definir",
        Some(&admin),
        json!({ "id": petit["eleve"]["id"], "contenu_base64": PNG_ESSAI }),
    )
    .await;
    assert_eq!(
        p.refus("inscription_valider", Some(&admin), id_jardin.clone())
            .await
            .cle,
        "inscription.personne_autorisee_absente"
    );
    let fiche = p
        .ok(
            "eleve_lire",
            Some(&admin),
            json!({ "id": petit["eleve"]["id"] }),
        )
        .await;
    let mere = &fiche["responsables"][0];
    let mut q = mere["qualites"].clone();
    q["vient_chercher"] = json!(true);
    p.ok(
        "responsable_modifier",
        Some(&admin),
        json!({
            "eleve_id": petit["eleve"]["id"], "responsable_id": mere["responsable"]["id"],
            "saisie": { "identite": { "nom": "Coulibaly", "prenoms": "Mariam", "sexe": "feminin", "date_naissance": "",
                "date_naissance_estimee": false, "lieu_naissance": "", "nationalite": "", "adresse": "",
                "telephones": ["76 12 34 56"] }, "profession": "", "employeur": "" },
            "qualites": q,
        }),
    )
    .await;
    let au_jardin = p.ok("inscription_valider", Some(&admin), id_jardin).await;
    assert_eq!(
        au_jardin["avertissements"],
        json!(["inscription.une_seule_personne_autorisee"])
    );

    // Abandon motivé ; une inscription abandonnée ne revient pas, une nouvelle se crée.
    assert_eq!(
        p.refus(
            "inscription_abandonner",
            Some(&admin),
            json!({ "id": id["id"], "motif": "  " }),
        )
        .await
        .cle,
        "inscription.motif_obligatoire"
    );
    let abandonnee = p
        .ok(
            "inscription_abandonner",
            Some(&admin),
            json!({ "id": id["id"], "motif": "Déménagement à Sikasso" }),
        )
        .await;
    assert_eq!(abandonnee["inscription"]["statut"], "abandonnee");
    assert_eq!(
        p.refus("inscription_valider", Some(&admin), id.clone())
            .await
            .cle,
        "erreur.transition_interdite"
    );
    let retour = p
        .ok("inscription_creer", Some(&admin), inscrire(&awa, &septa))
        .await;
    assert_eq!(retour["inscription"]["statut"], "brouillon");

    // Q-31 : une inscription tardive avertit sans bloquer.
    p.ok(
        "parametre_definir",
        Some(&admin),
        json!({ "cle": "inscription.alerte_apres_jours", "valeur": { "type": "entier", "valeur": 2 } }),
    )
    .await;
    let tardive = p
        .ok(
            "inscription_valider",
            Some(&admin),
            json!({ "id": retour["inscription"]["id"] }),
        )
        .await;
    assert_eq!(tardive["avertissements"], json!(["inscription.tardive"]));

    let fiche = p
        .ok(
            "eleve_lire",
            Some(&admin),
            json!({ "id": awa["eleve"]["id"] }),
        )
        .await;
    assert_eq!(fiche["inscriptions"].as_array().unwrap().len(), 2);
    assert_eq!(fiche["inscriptions"][0]["annee"], "2026-2027");
    let liste = p
        .ok(
            "inscriptions_lister",
            Some(&admin),
            json!({ "annee_id": annee["id"] }),
        )
        .await;
    assert_eq!(liste.as_array().unwrap().len(), 4);
    p.ok("audit_verifier", Some(&admin), Value::Null).await;
}

#[tokio::test]
async fn fiche_d_inscription_imprimee_avec_photo_et_qr() {
    let dossier = tempfile::tempdir().unwrap();
    let (p, admin, annee) = ecole(dossier.path()).await;
    let septa = classe(&p, &admin, &annee, "7", None, "7e A").await;
    let awa = p
        .ok(
            "eleve_creer",
            Some(&admin),
            saisie_eleve(identite("Traoré", "Awa", json!("feminin"), "2014-03-02")),
        )
        .await;
    lier_mere(&p, &admin, &awa, true, false).await;
    p.ok(
        "eleve_photo_definir",
        Some(&admin),
        json!({ "id": awa["eleve"]["id"], "contenu_base64": PNG_ESSAI }),
    )
    .await;
    let brouillon = p
        .ok("inscription_creer", Some(&admin), inscrire(&awa, &septa))
        .await;
    let id = json!({ "id": brouillon["inscription"]["id"] });
    assert_eq!(
        p.refus("inscription_fiche_produire", Some(&admin), id.clone())
            .await
            .cle,
        "inscription.fiche_non_validee"
    );
    p.ok("inscription_valider", Some(&admin), id.clone()).await;

    // Assertion 9 : la fiche se compose par le vrai moteur Typst, s'enregistre et se réimprime.
    let pdf = |document: &Value| {
        base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            document["pdf"].as_str().unwrap(),
        )
        .unwrap()
    };
    let fiche = p
        .ok("inscription_fiche_produire", Some(&admin), id.clone())
        .await;
    assert!(pdf(&fiche).starts_with(b"%PDF-"));
    assert_eq!(fiche["document"]["modele_id"], "fiche_inscription");
    assert_eq!(fiche["document"]["entite"], "inscription");
    assert_eq!(fiche["document"]["entite_id"], id["id"]);
    // Sans le bloc des responsables (T-08), la fiche reste valide et change de contenu.
    p.ok(
        "parametre_definir",
        Some(&admin),
        json!({ "cle": "fiche_inscription.afficher_responsables", "valeur": { "type": "booleen", "valeur": false } }),
    )
    .await;
    let sans = p
        .ok("inscription_fiche_produire", Some(&admin), id.clone())
        .await;
    assert!(pdf(&sans).starts_with(b"%PDF-"));
    assert_ne!(
        sans["document"]["empreinte_sha256"],
        fiche["document"]["empreinte_sha256"]
    );
    assert_ne!(sans["document"]["id"], fiche["document"]["id"]);
    p.ok("audit_verifier", Some(&admin), Value::Null).await;
}

#[tokio::test]
async fn fusion_de_deux_fiches_en_doublon() {
    let dossier = tempfile::tempdir().unwrap();
    let (p, admin, annee) = ecole(dossier.path()).await;
    let septa = classe(&p, &admin, &annee, "7", None, "7e A").await;
    let awa = p
        .ok(
            "eleve_creer",
            Some(&admin),
            saisie_eleve(identite("Traoré", "Awa", json!("feminin"), "2014-03-02")),
        )
        .await;
    let mut doublon = saisie_eleve(identite("TRAORE", "Hawa", json!("feminin"), "2014-03-02"));
    doublon["matricule_national"] = json!("ML-2014-77");
    doublon["identite"]["adresse"] = json!("Badalabougou, Bamako");
    let hawa = p.ok("eleve_creer", Some(&admin), doublon).await;
    lier_mere(&p, &admin, &hawa, true, false).await;
    let mut pere = json!({
        "identite": identite("Traoré", "Seydou", json!("masculin"), ""),
        "profession": "", "employeur": "",
    });
    pere["identite"]["telephones"] = json!(["65 43 21 09"]);
    p.ok(
        "responsable_rattacher",
        Some(&admin),
        json!({ "eleve_id": awa["eleve"]["id"], "responsable_id": null, "saisie": pere, "qualites": qualites("pere", true, true, 1) }),
    )
    .await;
    let inscription_awa = p
        .ok("inscription_creer", Some(&admin), inscrire(&awa, &septa))
        .await;
    let inscription_hawa = p
        .ok("inscription_creer", Some(&admin), inscrire(&hawa, &septa))
        .await;

    let demande = json!({ "conservee_id": awa["eleve"]["id"], "absorbee_id": hawa["eleve"]["id"] });
    // L'aperçu montre le résultat et l'obstacle : deux inscriptions en cours la même année.
    let apercu = p
        .ok("eleves_fusion_apercu", Some(&admin), demande.clone())
        .await;
    assert_eq!(apercu["annees_concurrentes"], json!(["2026-2027"]));
    assert_eq!(
        apercu["eleve"]["matricule_interne"],
        awa["eleve"]["matricule_interne"]
    );
    assert_eq!(apercu["eleve"]["matricule_national"], "ML-2014-77");
    assert_eq!(apercu["identite"]["adresse"], "Badalabougou, Bamako");
    assert_eq!(
        apercu["identite"]["prenoms"], "Awa",
        "la conservée garde ses valeurs"
    );
    assert_eq!(
        p.refus("eleves_fusionner", Some(&admin), demande.clone())
            .await
            .cle,
        "fusion.inscriptions_concurrentes"
    );
    p.ok(
        "inscription_abandonner",
        Some(&admin),
        json!({ "id": inscription_hawa["inscription"]["id"], "motif": "Doublon de Traoré Awa" }),
    )
    .await;

    // Assertion 10 : inscriptions et responsables passent sur la conservée.
    let fusionnee = p
        .ok("eleves_fusionner", Some(&admin), demande.clone())
        .await;
    assert_eq!(fusionnee["eleve"]["matricule_national"], "ML-2014-77");
    assert_eq!(fusionnee["responsables"].as_array().unwrap().len(), 2);
    assert_eq!(
        fusionnee["responsables"][1]["qualites"]["ordre_priorite"],
        2
    );
    assert_eq!(fusionnee["inscriptions"].as_array().unwrap().len(), 2);
    assert!(
        fusionnee["inscriptions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["inscription"]["id"] == inscription_awa["inscription"]["id"])
    );
    let absorbee = p
        .ok(
            "eleve_lire",
            Some(&admin),
            json!({ "id": hawa["eleve"]["id"] }),
        )
        .await;
    assert_eq!(absorbee["eleve"]["fusionne_dans"], awa["eleve"]["id"]);
    assert_eq!(
        absorbee["eleve"]["matricule_interne"],
        hawa["eleve"]["matricule_interne"]
    );

    // L'absorbée sort des recherches ; son matricule mène à la conservée.
    let trouves = p
        .ok(
            "eleves_rechercher",
            Some(&admin),
            json!({ "texte": "Traore" }),
        )
        .await;
    assert_eq!(trouves.as_array().unwrap().len(), 1);
    let par_ancien = p
        .ok(
            "eleves_rechercher",
            Some(&admin),
            json!({ "texte": hawa["eleve"]["matricule_interne"] }),
        )
        .await;
    assert_eq!(par_ancien[0]["id"], awa["eleve"]["id"]);
    for (commande, entree, cle) in [
        ("eleves_fusionner", demande.clone(), "eleve.absorbe"),
        (
            "eleves_fusionner",
            json!({ "conservee_id": awa["eleve"]["id"], "absorbee_id": awa["eleve"]["id"] }),
            "fusion.meme_fiche",
        ),
        (
            "eleve_modifier",
            json!({ "id": hawa["eleve"]["id"], "saisie": saisie_eleve(identite("Traoré", "Hawa", json!("feminin"), "2014-03-02")) }),
            "eleve.absorbe",
        ),
    ] {
        assert_eq!(
            p.refus(commande, Some(&admin), entree).await.cle,
            cle,
            "{commande}"
        );
    }
    let journal = p
        .ok(
            "audit_lister",
            Some(&admin),
            json!({ "entite": "eleve", "page": 0 }),
        )
        .await;
    assert!(
        journal["lignes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|l| l["contexte"] == "fusion.conserver")
    );
    p.ok("audit_verifier", Some(&admin), Value::Null).await;
}

#[tokio::test]
async fn le_sexe_d_un_pere_ou_d_une_mere_ne_se_redemande_pas() {
    let dossier = tempfile::tempdir().unwrap();
    let (p, admin, _annee) = ecole(dossier.path()).await;
    let eleve = p
        .ok(
            "eleve_creer",
            Some(&admin),
            saisie_eleve(identite("Keïta", "Bakary", json!("masculin"), "2014-02-11")),
        )
        .await;
    let mut pere = json!({
        "identite": identite("Keïta", "Moussa", Value::Null, ""),
        "profession": "", "employeur": "",
    });
    pere["identite"]["telephones"] = json!(["70 00 00 01"]);
    let fiche = p
        .ok(
            "responsable_rattacher",
            Some(&admin),
            json!({ "eleve_id": eleve["eleve"]["id"], "responsable_id": null, "saisie": pere,
                "qualites": qualites("pere", true, true, 1) }),
        )
        .await;
    assert_eq!(fiche["responsables"][0]["identite"]["sexe"], "masculin");
    // Devenu « mère » par erreur de saisie puis corrigé : le lien fait foi.
    let responsable_id = fiche["responsables"][0]["responsable"]["id"].clone();
    let mut saisie = pere.clone();
    saisie["identite"]["sexe"] = json!("masculin");
    let corrige = p
        .ok(
            "responsable_modifier",
            Some(&admin),
            json!({ "eleve_id": eleve["eleve"]["id"], "responsable_id": responsable_id, "saisie": saisie,
                "qualites": qualites("mere", true, true, 1) }),
        )
        .await;
    assert_eq!(corrige["responsables"][0]["identite"]["sexe"], "feminin");
}
