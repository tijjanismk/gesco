//! Portail des élèves, entrée (phase 12, lot 4, étape 2) : billets d'une classe, code d'un élève,
//! entrer avec son code, même réponse pour un code faux et un matricule inconnu, cinq codes faux
//! qui bloquent le matricule jusqu'au lendemain, lire sans code, sortir ; rien du code au journal.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use crate::commun::*;
use gesco_services::portail::entree::{DemandeEntrer, DemandeLireSansCode, Entree, RefusEntree};
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

struct Classe9 {
    p: Poste_,
    admin: Jeton,
    classe: Value,
    /// (identifiant, matricule) des deux élèves.
    eleves: Vec<(Value, String)>,
}

async fn classe_de_9e(dossier: &std::path::Path) -> Classe9 {
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
    let ped = p
        .ok("pedagogie_lire", Some(&admin), json!({ "id": annee["id"] }))
        .await;
    let neuvieme = id_par(&ped["niveaux"], "code", "9");
    let classe = p
        .ok(
            "classe_creer",
            Some(&admin),
            json!({ "annee_id": annee["id"], "niveau_id": neuvieme, "serie_id": null, "libelle": "9e A" }),
        )
        .await["id"]
        .clone();
    // Les essais donnent leurs propres matricules : l'école les saisit elle-même.
    p.ok(
        "parametre_definir",
        Some(&admin),
        json!({ "cle": "matricule.saisie_manuelle",
            "valeur": { "type": "booleen", "valeur": true } }),
    )
    .await;
    let mut eleves = Vec::new();
    for (prenom, matricule) in [("Adama", "E-901"), ("Kadiatou", "E-902")] {
        let fiche = p
            .ok(
                "eleve_creer",
                Some(&admin),
                json!({ "identite": { "nom": "Coulibaly", "prenoms": prenom, "sexe": "feminin",
                    "date_naissance": "2011-01-05", "date_naissance_estimee": false,
                    "lieu_naissance": "Ségou", "nationalite": "Malienne", "adresse": "",
                    "telephones": [] },
                    "matricule_interne": matricule, "matricule_national": format!("N{matricule}"),
                    "acte_nature": "acte",
                    "acte_numero": "", "acte_date": "", "ecole_origine": "" }),
            )
            .await;
        // Une 9e se valide avec un parent joignable et le matricule national (classe d'examen).
        p.ok(
            "responsable_rattacher",
            Some(&admin),
            json!({ "eleve_id": fiche["eleve"]["id"], "responsable_id": null,
                "saisie": { "identite": { "nom": "Coulibaly", "prenoms": "Mariam", "sexe": "feminin",
                    "date_naissance": "", "date_naissance_estimee": false, "lieu_naissance": "",
                    "nationalite": "", "adresse": "", "telephones": ["76 12 34 56"] },
                    "profession": "", "employeur": "" },
                "qualites": { "lien": "mere", "est_responsable_legal": true, "est_payeur": true,
                    "recoit_notifications": true, "vient_chercher": false, "ordre_priorite": 1 } }),
        )
        .await;
        p.ok(
            "inscription_creer",
            Some(&admin),
            json!({ "eleve_id": fiche["eleve"]["id"], "classe_id": classe, "date_inscription": "",
                "redoublant": false, "etablissement_origine": "" }),
        )
        .await;
        eleves.push((fiche["eleve"]["id"].clone(), matricule.to_owned()));
    }
    let validation = p
        .ok(
            "inscriptions_valider_classe",
            Some(&admin),
            json!({ "id": classe }),
        )
        .await;
    assert_eq!(validation["validees"], 2, "{validation}");
    Classe9 {
        p,
        admin,
        classe,
        eleves,
    }
}

fn demande(matricule: &str, code: &str) -> DemandeEntrer {
    DemandeEntrer {
        matricule: matricule.to_owned(),
        code: code.to_owned(),
        pas_mon_telephone: false,
    }
}

async fn refus(c: &Classe9, matricule: &str, code: &str) -> RefusEntree {
    match c
        .p
        .application
        .portail_entrer(&demande(matricule, code))
        .await
        .unwrap()
    {
        Some(Entree::Refuse(r)) => r,
        _ => panic!("{matricule} devait être refusé"),
    }
}

#[tokio::test]
async fn billets_d_une_classe_et_code_d_un_eleve() {
    let dossier = tempfile::tempdir().unwrap();
    let c = classe_de_9e(dossier.path()).await;
    let compte =
        c.p.ok(
            "portail_codes_classe",
            Some(&c.admin),
            json!({ "classe_id": c.classe }),
        )
        .await;
    assert_eq!(compte, json!({ "eleves": 2, "avec_code": 0 }));
    // Billets des élèves sans code : deux billets, en PDF.
    let billets =
        c.p.ok(
            "portail_billets_produire",
            Some(&c.admin),
            json!({ "classe_id": c.classe, "tous": false }),
        )
        .await;
    assert_eq!(billets["billets"], 2);
    assert!(!billets["pdf"].as_str().unwrap().is_empty());
    // Une seconde fois : tous ont leur code, aucun billet.
    let encore =
        c.p.ok(
            "portail_billets_produire",
            Some(&c.admin),
            json!({ "classe_id": c.classe, "tous": false }),
        )
        .await;
    assert_eq!(encore["billets"], 0);
    // Sur la fiche : remis le …, et « Remettre un code ».
    let etat =
        c.p.ok(
            "portail_code_lire",
            Some(&c.admin),
            json!({ "eleve_id": c.eleves[0].0 }),
        )
        .await;
    assert!(etat["remis_le"].is_string());
    assert_eq!(etat["inscrit"], true);
    let un =
        c.p.ok(
            "portail_code_remettre",
            Some(&c.admin),
            json!({ "eleve_id": c.eleves[0].0 }),
        )
        .await;
    assert_eq!(un["billets"], 1);
    // Au journal : billets et code remis, jamais le code.
    let journal =
        c.p.ok("audit_lister", Some(&c.admin), json!({ "page": 0 }))
            .await;
    let texte = journal.to_string();
    assert!(texte.contains("portail.billets"), "{texte}");
    assert!(texte.contains("portail.code_remettre"), "{texte}");
    // Les lignes du portail ne portent aucun code (d'autres lignes ont le code d'un niveau).
    for ligne in journal["lignes"].as_array().unwrap() {
        if ligne["contexte"]
            .as_str()
            .unwrap_or("")
            .starts_with("portail.")
        {
            assert!(!ligne.to_string().contains("\"code\""), "{ligne}");
        }
    }
}

#[tokio::test]
async fn un_eleve_entre_avec_son_code_et_cinq_codes_faux_bloquent_le_matricule() {
    let dossier = tempfile::tempdir().unwrap();
    let c = classe_de_9e(dossier.path()).await;
    let adama = c.eleves[0].0.as_str().unwrap().to_owned();
    let code =
        c.p.application
            .portail_code_pour_essai(&adama)
            .await
            .unwrap();

    // Le bon code, en minuscules et avec un espace : il entre, voit son prénom et sa classe.
    let saisi = format!("{} {}", &code[..3], &code[3..]).to_lowercase();
    let issue =
        c.p.application
            .portail_entrer(&demande(" e-901 ", &saisi))
            .await
            .unwrap();
    let Some(Entree::Entre { jeton, session }) = issue else {
        panic!("le bon code fait entrer : {issue:?}");
    };
    let vue = session.vue();
    assert_eq!(vue.prenom.as_deref(), Some("Adama"));
    assert_eq!(vue.classe.as_deref(), Some("9e A"));
    assert!(vue.avec_code);
    assert!(c.p.application.portail_session(&jeton).is_some());
    let accueil =
        c.p.application
            .portail_accueil(Some(&jeton))
            .await
            .unwrap()
            .unwrap();
    assert_eq!(accueil.eleve, Some(vue));
    // Rien de la fiche au-delà du prénom et de la classe.
    let texte = serde_json::to_string(&accueil).unwrap();
    assert!(
        !texte.contains("Coulibaly") && !texte.contains("2011"),
        "{texte}"
    );
    c.p.application.portail_sortir(&jeton);
    assert!(c.p.application.portail_session(&jeton).is_none());

    // Code faux, matricule inconnu, élève sans code : même réponse.
    assert_eq!(refus(&c, "E-901", "AAAAAA").await, RefusEntree::Incorrect);
    assert_eq!(refus(&c, "INCONNU", &code).await, RefusEntree::Incorrect);
    assert_eq!(refus(&c, "E-902", &code).await, RefusEntree::Incorrect);

    // Cinq codes faux dans la journée : le matricule est bloqué, même avec le bon code.
    for _ in 0..4 {
        assert_eq!(refus(&c, "E-901", "BBBBBB").await, RefusEntree::Incorrect);
    }
    assert_eq!(refus(&c, "E-901", &code).await, RefusEntree::Bloque);
    let journal =
        c.p.ok("audit_lister", Some(&c.admin), json!({ "page": 0 }))
            .await
            .to_string();
    assert!(journal.contains("portail.matricule_bloque"), "{journal}");
    assert!(!journal.contains(&code), "le code ne va jamais au journal");

    // Le lendemain, il entre de nouveau.
    c.p.horloge.avancer(86_400_000);
    // Un jour sans activité ferme la session de la direction : elle se reconnecte.
    let admin = c.p.connecter("admin", MOT_DE_PASSE).await;
    assert!(matches!(
        c.p.application
            .portail_entrer(&demande("E-901", &code))
            .await
            .unwrap(),
        Some(Entree::Entre { .. })
    ));

    // Réimprimer remplace le code : l'ancien ne marche plus.
    c.p.ok(
        "portail_code_remettre",
        Some(&admin),
        json!({ "eleve_id": c.eleves[0].0 }),
    )
    .await;
    assert_eq!(refus(&c, "E-901", &code).await, RefusEntree::Incorrect);
}

#[tokio::test]
async fn lire_sans_code_au_niveau_choisi() {
    let dossier = tempfile::tempdir().unwrap();
    let c = classe_de_9e(dossier.path()).await;
    let accueil =
        c.p.application
            .portail_accueil(None)
            .await
            .unwrap()
            .unwrap();
    assert!(accueil.eleve.is_none());
    let neuvieme = accueil
        .niveaux
        .iter()
        .find(|n| n.libelle.contains('9'))
        .expect("la 9e a une classe cette année")
        .clone();
    let (jeton, session) =
        c.p.application
            .portail_lire_sans_code(&DemandeLireSansCode {
                niveau_id: neuvieme.niveau_id.clone(),
                serie_id: None,
            })
            .await
            .unwrap()
            .unwrap();
    assert!(session.eleve_id.is_none() && session.prenom.is_none());
    assert!(!session.vue().avec_code);
    assert!(c.p.application.portail_session(&jeton).is_some());
    // Un niveau sans classe cette année n'est pas ouvert.
    assert!(
        c.p.application
            .portail_lire_sans_code(&DemandeLireSansCode {
                niveau_id: "01J00000000000000000000000".to_owned(),
                serie_id: None,
            })
            .await
            .unwrap()
            .is_none()
    );
}
