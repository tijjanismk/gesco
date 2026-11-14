//! Observations du jardin par la frontière (SPEC_PHASE_11_B, étape 2) : saisie case par case et
//! par colonne, ressaisie, vidage, appréciations ; refus d'une classe à notes et d'une case
//! modifiée entre-temps ; le maître n'observe que les enfants de ses classes du jardin.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use crate::commun::*;
use gesco_services::session::Jeton;
use serde_json::{Value, json};

const MOT_DE_PASSE_MAITRE: &str = "maitresse-2026";

fn id_par(liste: &Value, champ: &str, valeur: &str) -> Value {
    liste
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x[champ] == valeur)
        .unwrap_or_else(|| panic!("{champ} = {valeur} absent"))["id"]
        .clone()
}

/// Enfant inscrit dans la classe ; rend sa fiche. La classe se valide ensuite d'un geste.
pub(crate) async fn inscrire(
    p: &Poste_,
    admin: &Jeton,
    nom: &str,
    prenoms: &str,
    classe: &Value,
) -> Value {
    let fiche = p
        .ok(
            "eleve_creer",
            Some(admin),
            json!({ "identite": { "nom": nom, "prenoms": prenoms, "sexe": "feminin",
                "date_naissance": "2022-03-02", "date_naissance_estimee": false,
                "lieu_naissance": "Bamako", "nationalite": "Malienne", "adresse": "", "telephones": [] },
                "matricule_interne": "", "matricule_national": "", "acte_nature": "acte",
                "acte_numero": "", "acte_date": "", "ecole_origine": "" }),
        )
        .await;
    // Au jardin, l'inscription se valide avec une photo et une personne qui vient chercher
    // l'enfant, joignable.
    p.ok(
        "responsable_rattacher",
        Some(admin),
        json!({ "eleve_id": fiche["eleve"]["id"], "responsable_id": null,
            "saisie": { "identite": { "nom": nom, "prenoms": format!("Mère de {prenoms}"), "sexe": "feminin",
                "date_naissance": "", "date_naissance_estimee": false, "lieu_naissance": "", "nationalite": "",
                "adresse": "", "telephones": ["76 12 34 56"] }, "profession": "", "employeur": "" },
            "qualites": { "lien": "mere", "est_responsable_legal": true, "est_payeur": true,
                "recoit_notifications": true, "vient_chercher": true, "ordre_priorite": 1 } }),
    )
    .await;
    p.ok(
        "eleve_photo_definir",
        Some(admin),
        json!({ "id": fiche["eleve"]["id"], "contenu_base64": PNG_ESSAI }),
    )
    .await;
    p.ok(
        "inscription_creer",
        Some(admin),
        json!({ "eleve_id": fiche["eleve"]["id"], "classe_id": classe, "date_inscription": "",
            "redoublant": false, "etablissement_origine": "" }),
    )
    .await;
    fiche
}

/// École d'essai : deux classes de moyenne section, un enfant dans la seconde et trois dans la
/// première, une 7e pour les refus ; le 1er trimestre de la moyenne section.
pub(crate) struct Ecole {
    pub(crate) p: Poste_,
    pub(crate) admin: Jeton,
    pub(crate) ms_a: Value,
    pub(crate) ms_b: Value,
    pub(crate) septieme_a: Value,
    pub(crate) trimestre: Value,
    pub(crate) math: Value,
    _dossier: tempfile::TempDir,
}

pub(crate) async fn ecole() -> Ecole {
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
    let ms = id_par(&ped["niveaux"], "code", "MS");
    let septieme = id_par(&ped["niveaux"], "code", "7");
    let mut classes = Vec::new();
    for (niveau, libelle) in [(&ms, "MS A"), (&ms, "MS B"), (&septieme, "7e A")] {
        classes.push(
            p.ok(
                "classe_creer",
                Some(&admin),
                json!({ "annee_id": annee["id"], "niveau_id": niveau, "serie_id": null, "libelle": libelle }),
            )
            .await["id"]
                .clone(),
        );
    }
    for (nom, prenoms) in [
        ("Diarra", "Aminata"),
        ("Keita", "Bakary"),
        ("Sangaré", "Coumba"),
    ] {
        inscrire(&p, &admin, nom, prenoms, &classes[0]).await;
    }
    inscrire(&p, &admin, "Touré", "Djénéba", &classes[1]).await;
    for (classe, nombre) in [(&classes[0], 3), (&classes[1], 1)] {
        let bilan = p
            .ok(
                "inscriptions_valider_classe",
                Some(&admin),
                json!({ "id": classe }),
            )
            .await;
        assert_eq!(bilan["validees"], nombre, "{bilan}");
    }
    let trimestre = ped["periodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["periode"]["niveau_id"] == ms && x["periode"]["rang"] == 1)
        .unwrap()["periode"]["id"]
        .clone();
    Ecole {
        math: id_par(&ped["matieres"], "code", "MATH"),
        ms_a: classes[0].clone(),
        ms_b: classes[1].clone(),
        septieme_a: classes[2].clone(),
        p,
        admin,
        trimestre,
        _dossier: dossier,
    }
}

pub(crate) fn demande(classe: &Value, periode: &Value) -> Value {
    json!({ "classe_id": classe, "periode_id": periode })
}

/// Observation d'un enfant pour une compétence dans la grille ; `Null` s'il n'en a pas.
fn observation(g: &Value, enfant: &Value, competence: &Value) -> Value {
    g["observations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["inscription_id"] == *enfant && o["competence_id"] == *competence)
        .cloned()
        .unwrap_or(Value::Null)
}

#[tokio::test]
async fn la_direction_observe_case_par_case_puis_par_colonne() {
    let e = ecole().await;
    let p = &e.p;
    let admin = &e.admin;
    let g = p
        .ok(
            "observations_grille",
            Some(admin),
            demande(&e.ms_a, &e.trimestre),
        )
        .await;
    assert_eq!(g["classe"], "MS A");
    assert_eq!(g["modifiable"], true);
    assert_eq!(g["enfants"].as_array().unwrap().len(), 3);
    assert_eq!(g["domaines"].as_array().unwrap().len(), 5);
    assert_eq!(g["competences"].as_array().unwrap().len(), 20);
    assert_eq!(g["observations"], json!([]));
    assert_eq!(g["echelle"], json!(["Acquis", "En cours", "Non acquis"]));
    assert!(
        g["enfants"][0]["photo"]
            .as_str()
            .unwrap()
            .starts_with("data:image/png")
    );
    // Rien ne fait remarquer ce qui manque : ni compteur, ni pourcentage (RB-2).
    for cle in ["saisies", "effectif", "manquantes", "complete"] {
        assert!(g.get(cle).is_none(), "{cle}");
    }
    let enfant = g["enfants"][0]["inscription_id"].clone();
    let c1 = g["competences"][0]["id"].clone();
    let c2 = g["competences"][1]["id"].clone();
    let case = |niveau: Value, lue_le: Value| {
        json!({ "classe_id": e.ms_a, "periode_id": e.trimestre, "competence_id": c1,
            "inscription_id": enfant, "niveau": niveau, "lue_le": lue_le })
    };

    // Une case posée, puis ressaisie : la seconde remplace la première (RB-5).
    let premiere = p
        .ok(
            "observation_saisir",
            Some(admin),
            case(json!("acquis"), json!(0)),
        )
        .await;
    assert_eq!(premiere["niveau"], "acquis");
    p.horloge.avancer(1_000);
    let seconde = p
        .ok(
            "observation_saisir",
            Some(admin),
            case(json!("en_cours"), premiere["modifiee_le"].clone()),
        )
        .await;
    assert_eq!(seconde["niveau"], "en_cours");
    assert_ne!(seconde["modifiee_le"], premiere["modifiee_le"]);

    // Un autre poste qui avait lu la case avant la seconde saisie est refusé, et l'erreur dit
    // qui a écrit (critère 13) ; rien n'est écrasé.
    p.horloge.avancer(1_000);
    let refus = p
        .refus(
            "observation_saisir",
            Some(admin),
            case(json!("non_acquis"), premiere["modifiee_le"].clone()),
        )
        .await;
    assert_eq!(refus.cle, "fiche.modifiee_entre_temps");
    assert_eq!(refus.modifiee.unwrap().par.as_deref(), Some("Awa Traoré"));
    // Une case vide lue à zéro, et déjà remplie par un autre, est refusée de même.
    assert_eq!(
        p.refus(
            "observation_saisir",
            Some(admin),
            case(json!("acquis"), json!(0))
        )
        .await
        .cle,
        "fiche.modifiee_entre_temps"
    );

    // Vidée, l'observation garde sa ligne, sans niveau ; l'audit garde les valeurs.
    let videe = p
        .ok(
            "observation_saisir",
            Some(admin),
            case(Value::Null, seconde["modifiee_le"].clone()),
        )
        .await;
    assert!(videe["niveau"].is_null());
    let g = p
        .ok(
            "observations_grille",
            Some(admin),
            demande(&e.ms_a, &e.trimestre),
        )
        .await;
    assert!(observation(&g, &enfant, &c1)["niveau"].is_null());
    let journal = p
        .ok("audit_lister", Some(admin), json!({ "page": 0 }))
        .await
        .to_string();
    assert!(journal.contains("observations.saisir"));
    assert!(journal.contains("en_cours"));

    // « Toute la colonne » pose le niveau à tous les enfants ; la maîtresse corrige ensuite.
    let colonne = p
        .ok(
            "observations_colonne",
            Some(admin),
            json!({ "classe_id": e.ms_a, "periode_id": e.trimestre, "competence_id": c2,
                "niveau": "acquis", "lue_le": 0 }),
        )
        .await;
    assert_eq!(colonne.as_array().unwrap().len(), 3);
    assert!(
        colonne
            .as_array()
            .unwrap()
            .iter()
            .all(|o| o["niveau"] == "acquis")
    );
    let lue_colonne = colonne
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["modifiee_le"].as_i64().unwrap())
        .max()
        .unwrap();
    p.horloge.avancer(1_000);
    let corrigee = p
        .ok(
            "observation_saisir",
            Some(admin),
            json!({ "classe_id": e.ms_a, "periode_id": e.trimestre, "competence_id": c2,
                "inscription_id": enfant, "niveau": "non_acquis", "lue_le": lue_colonne }),
        )
        .await;
    assert_eq!(corrigee["niveau"], "non_acquis");
    // Une colonne lue avant la correction ne l'écrase pas.
    assert_eq!(
        p.refus(
            "observations_colonne",
            Some(admin),
            json!({ "classe_id": e.ms_a, "periode_id": e.trimestre, "competence_id": c2,
                "niveau": "en_cours", "lue_le": lue_colonne }),
        )
        .await
        .cle,
        "fiche.modifiee_entre_temps"
    );

    // Appréciation générale et commentaire d'un domaine ; 300 caractères au plus.
    let domaine = g["domaines"][0]["id"].clone();
    for d in [Value::Null, domaine.clone()] {
        p.ok(
            "observation_apprecier",
            Some(admin),
            json!({ "classe_id": e.ms_a, "periode_id": e.trimestre, "inscription_id": enfant,
                "domaine_id": d, "texte": "  Aminata parle volontiers en classe.  " }),
        )
        .await;
    }
    assert_eq!(
        p.refus(
            "observation_apprecier",
            Some(admin),
            json!({ "classe_id": e.ms_a, "periode_id": e.trimestre, "inscription_id": enfant,
                "domaine_id": null, "texte": "a".repeat(301) }),
        )
        .await
        .cle,
        "appreciation.trop_longue"
    );
    let g = p
        .ok(
            "observations_grille",
            Some(admin),
            demande(&e.ms_a, &e.trimestre),
        )
        .await;
    let appreciations = g["appreciations"].as_array().unwrap();
    assert_eq!(appreciations.len(), 2);
    assert!(
        appreciations
            .iter()
            .all(|a| a["texte"] == "Aminata parle volontiers en classe.")
    );
    assert!(appreciations.iter().any(|a| a["domaine_id"] == domaine));
    assert_eq!(observation(&g, &enfant, &c2)["niveau"], "non_acquis");

    // Une compétence retirée disparaît de la grille et garde ses observations ; remise, elle
    // revient telle quelle (RB-11).
    let competence = g["competences"][1].clone();
    let retirer = |retiree: bool| {
        json!({ "id": competence["id"], "domaine_id": competence["domaine_id"],
            "niveau_id": competence["niveau_id"], "libelle": competence["libelle"], "retiree": retiree })
    };
    p.ok("competence_enregistrer", Some(admin), retirer(true))
        .await;
    let g = p
        .ok(
            "observations_grille",
            Some(admin),
            demande(&e.ms_a, &e.trimestre),
        )
        .await;
    assert_eq!(g["competences"].as_array().unwrap().len(), 19);
    assert!(observation(&g, &enfant, &c2).is_null());
    p.ok("competence_enregistrer", Some(admin), retirer(false))
        .await;
    let g = p
        .ok(
            "observations_grille",
            Some(admin),
            demande(&e.ms_a, &e.trimestre),
        )
        .await;
    assert_eq!(observation(&g, &enfant, &c2)["niveau"], "non_acquis");

    // Un enfant d'une autre classe ne s'observe pas dans celle-ci.
    let g_b = p
        .ok(
            "observations_grille",
            Some(admin),
            demande(&e.ms_b, &e.trimestre),
        )
        .await;
    assert_eq!(
        p.refus(
            "observation_saisir",
            Some(admin),
            json!({ "classe_id": e.ms_a, "periode_id": e.trimestre, "competence_id": c1,
                "inscription_id": g_b["enfants"][0]["inscription_id"], "niveau": "acquis", "lue_le": 0 }),
        )
        .await
        .cle,
        "observation.enfant_hors_classe"
    );
}

#[tokio::test]
async fn les_notes_et_les_observations_ne_se_melent_pas() {
    let e = ecole().await;
    let p = &e.p;
    // Critère 12 : les commandes d'observations refusent une classe du fondamental…
    let refus = p
        .refus(
            "observations_grille",
            Some(&e.admin),
            demande(&e.septieme_a, &e.trimestre),
        )
        .await;
    assert_eq!(refus.cle, "jardin.classe_en_notes");
    assert_eq!(
        p.refus(
            "observations_colonne",
            Some(&e.admin),
            json!({ "classe_id": e.septieme_a, "periode_id": e.trimestre,
                "competence_id": e.math, "niveau": "acquis", "lue_le": 0 }),
        )
        .await
        .cle,
        "jardin.classe_en_notes"
    );
    // … et celles des notes refusent une classe du jardin : elle n'a aucune matière.
    assert_eq!(
        p.refus(
            "evaluation_creer",
            Some(&e.admin),
            json!({ "classe_id": e.ms_a, "periode_id": e.trimestre, "matiere_id": e.math,
                "type_evaluation": "devoir", "libelle": "Devoir 1", "date": "2026-10-05", "bareme": null }),
        )
        .await
        .cle,
        "affectation.matiere_hors_programme"
    );
}

#[tokio::test]
async fn le_maitre_observe_les_enfants_de_ses_seules_classes_du_jardin() {
    let e = ecole().await;
    let p = &e.p;
    let admin = &e.admin;
    let compte = p
        .ok(
            "utilisateur_creer",
            Some(admin),
            json!({ "identifiant": "mariam", "nom_affiche": "Mariam Cissé",
                "mot_de_passe_initial": MOT_DE_PASSE_MAITRE, "roles": ["enseignant"] }),
        )
        .await;
    let mariam = p
        .ok(
            "enseignant_enregistrer",
            Some(admin),
            json!({ "id": null, "nom": "Cissé", "prenoms": "Mariam", "telephones": [],
                "statut": "titulaire", "utilisateur_id": compte["id"], "actif": true }),
        )
        .await["enseignant"]["id"]
        .clone();
    // Le jardin n'a pas de matière : « Toute la classe » y affecte le maître à la classe.
    let maitre = json!({ "classe_id": e.ms_a, "enseignant_id": mariam });
    assert_eq!(
        p.ok("maitre_affecter", Some(admin), maitre.clone()).await,
        1
    );
    assert_eq!(
        p.refus("maitre_affecter", Some(admin), maitre).await.cle,
        "affectation.deja"
    );
    let annee = p.ok("annees_lister", Some(admin), Value::Null).await[0]["id"].clone();
    let maitres = p
        .ok("maitres_jardin_lister", Some(admin), json!({ "id": annee }))
        .await;
    assert_eq!(maitres.as_array().unwrap().len(), 1);
    assert_eq!(maitres[0]["classe"], "MS A");
    assert_eq!(maitres[0]["enseignant"], "Mariam Cissé");

    // Critère 11 : réglage éteint, le compte de maître n'entre pas, donc n'observe rien.
    assert_eq!(
        p.refus(
            "connexion",
            None,
            json!({ "identifiant": "mariam", "mot_de_passe": MOT_DE_PASSE_MAITRE }),
        )
        .await
        .cle,
        "connexion.maitres_non_admis"
    );
    p.admettre_les_maitres(admin).await;
    let jeton = p.connecter("mariam", MOT_DE_PASSE_MAITRE).await;
    let portee = p.ok("portee_notes_lire", Some(&jeton), Value::Null).await;
    assert_eq!(portee["classes_jardin"], json!([e.ms_a]));

    // Allumé : il observe dans sa classe, commande par commande…
    let g = p
        .ok(
            "observations_grille",
            Some(&jeton),
            demande(&e.ms_a, &e.trimestre),
        )
        .await;
    let enfant = g["enfants"][0]["inscription_id"].clone();
    let competence = g["competences"][0]["id"].clone();
    p.ok(
        "observation_saisir",
        Some(&jeton),
        json!({ "classe_id": e.ms_a, "periode_id": e.trimestre, "competence_id": competence,
            "inscription_id": enfant, "niveau": "acquis", "lue_le": 0 }),
    )
    .await;
    p.ok(
        "observations_colonne",
        Some(&jeton),
        json!({ "classe_id": e.ms_a, "periode_id": e.trimestre, "competence_id": g["competences"][1]["id"],
            "niveau": "en_cours", "lue_le": 0 }),
    )
    .await;
    p.ok(
        "observation_apprecier",
        Some(&jeton),
        json!({ "classe_id": e.ms_a, "periode_id": e.trimestre, "inscription_id": enfant,
            "domaine_id": null, "texte": "Progresse bien." }),
    )
    .await;
    // … et nulle part ailleurs.
    let g_b = p
        .ok(
            "observations_grille",
            Some(admin),
            demande(&e.ms_b, &e.trimestre),
        )
        .await;
    let enfant_b = g_b["enfants"][0]["inscription_id"].clone();
    for (commande, entree) in [
        ("observations_grille", demande(&e.ms_b, &e.trimestre)),
        (
            "observation_saisir",
            json!({ "classe_id": e.ms_b, "periode_id": e.trimestre, "competence_id": competence,
                "inscription_id": enfant_b, "niveau": "acquis", "lue_le": 0 }),
        ),
        (
            "observations_colonne",
            json!({ "classe_id": e.ms_b, "periode_id": e.trimestre, "competence_id": competence,
                "niveau": "acquis", "lue_le": 0 }),
        ),
        (
            "observation_apprecier",
            json!({ "classe_id": e.ms_b, "periode_id": e.trimestre, "inscription_id": enfant_b,
                "domaine_id": null, "texte": "x" }),
        ),
    ] {
        assert_eq!(
            p.refus(commande, Some(&jeton), entree).await.cle,
            "erreur.portee_refusee",
            "{commande}"
        );
    }
    // Retiré de sa classe, il n'y observe plus.
    p.ok(
        "maitre_jardin_retirer",
        Some(admin),
        json!({ "id": maitres[0]["maitre"]["id"] }),
    )
    .await;
    assert_eq!(
        p.refus(
            "observations_grille",
            Some(&jeton),
            demande(&e.ms_a, &e.trimestre)
        )
        .await
        .cle,
        "erreur.portee_refusee"
    );
}
