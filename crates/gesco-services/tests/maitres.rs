//! Saisie des notes par les maîtres (SPEC_PHASE_11_A §7, critère 13) : éteinte à la livraison, un
//! compte de maître n'entre pas ; allumée, il ne lit ni n'écrit hors de ses classes, commande
//! par commande ; une école où un maître a déjà saisi garde la possibilité à la mise à niveau.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use crate::commun::*;
use gesco_services::commandes::{Acces, COMMANDES, COMMANDES_DU_MAITRE};
use gesco_services::session::Jeton;
use serde_json::{Value, json};

const MOT_DE_PASSE_MAITRE: &str = "enseignant-2026";

fn id_par(liste: &Value, champ: &str, valeur: &str) -> Value {
    liste
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x[champ] == valeur)
        .unwrap_or_else(|| panic!("{champ} = {valeur} absent"))["id"]
        .clone()
}

/// Élève inscrit et validé dans la classe ; rend sa fiche.
async fn inscrire(p: &Poste_, admin: &Jeton, nom: &str, prenoms: &str, classe: &Value) -> Value {
    let fiche = p
        .ok(
            "eleve_creer",
            Some(admin),
            json!({ "identite": { "nom": nom, "prenoms": prenoms, "sexe": "feminin",
                "date_naissance": "2014-03-02", "date_naissance_estimee": false,
                "lieu_naissance": "Bamako", "nationalite": "Malienne", "adresse": "", "telephones": [] },
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
                "recoit_notifications": true, "vient_chercher": false, "ordre_priorite": 1 } }),
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

async fn regler_saisie_des_maitres(p: &Poste_, admin: &Jeton, allumee: bool) {
    p.ok(
        "parametre_definir",
        Some(admin),
        json!({ "cle": "notes.saisie_maitres", "valeur": { "type": "booleen", "valeur": allumee } }),
    )
    .await;
}

/// École d'essai : deux classes de 7e, une élève dans chacune, un maître qui enseigne les maths
/// en 7e A seulement, un devoir de maths dans chaque classe.
struct Ecole {
    p: Poste_,
    admin: Jeton,
    a: Value,
    b: Value,
    trimestre: Value,
    math: Value,
    awa: Value,
    binta: Value,
    devoir_a: Value,
    devoir_b: Value,
    _dossier: tempfile::TempDir,
}

async fn ecole() -> Ecole {
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
    let mut classes = Vec::new();
    for libelle in ["7e A", "7e B"] {
        classes.push(
            p.ok(
                "classe_creer",
                Some(&admin),
                json!({ "annee_id": annee["id"], "niveau_id": septieme, "serie_id": null, "libelle": libelle }),
            )
            .await["id"]
                .clone(),
        );
    }
    let (a, b) = (classes[0].clone(), classes[1].clone());
    let awa = inscrire(&p, &admin, "Traoré", "Awa", &a).await;
    let binta = inscrire(&p, &admin, "Keita", "Binta", &b).await;
    for classe in [&a, &b] {
        let bilan = p
            .ok(
                "inscriptions_valider_classe",
                Some(&admin),
                json!({ "id": classe }),
            )
            .await;
        assert_eq!(bilan["validees"], 1);
    }
    let trimestre = ped["periodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["periode"]["niveau_id"] == septieme && x["periode"]["rang"] == 1)
        .unwrap()["periode"]["id"]
        .clone();
    let math = id_par(&ped["matieres"], "code", "MATH");
    let mut devoirs = Vec::new();
    for classe in [&a, &b] {
        devoirs.push(
            p.ok(
                "evaluation_creer",
                Some(&admin),
                json!({ "classe_id": classe, "periode_id": trimestre, "matiere_id": math,
                    "type_evaluation": "devoir", "libelle": "Devoir 1", "date": "2026-10-04", "bareme": null }),
            )
            .await,
        );
    }
    // Le maître : un compte qui ne porte que le rôle Enseignant, lié à sa fiche d'enseignant.
    let compte = p
        .ok(
            "utilisateur_creer",
            Some(&admin),
            json!({ "identifiant": "sekou", "nom_affiche": "Sékou Traoré",
                "mot_de_passe_initial": MOT_DE_PASSE_MAITRE, "roles": ["enseignant"] }),
        )
        .await;
    let sekou = p
        .ok(
            "enseignant_enregistrer",
            Some(&admin),
            json!({ "id": null, "nom": "Traoré", "prenoms": "Sékou", "telephones": [],
                "statut": "titulaire", "utilisateur_id": compte["id"], "actif": true }),
        )
        .await;
    p.ok(
        "affectation_ajouter",
        Some(&admin),
        json!({ "classe_id": a, "matiere_id": math, "enseignant_id": sekou["enseignant"]["id"] }),
    )
    .await;
    Ecole {
        devoir_a: devoirs[0].clone(),
        devoir_b: devoirs[1].clone(),
        p,
        admin,
        a,
        b,
        trimestre,
        math,
        awa,
        binta,
        _dossier: dossier,
    }
}

async fn valeur_du_reglage(p: &Poste_, admin: &Jeton) -> Value {
    p.ok("parametres_lister", Some(admin), json!(null))
        .await
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["cle"] == "notes.saisie_maitres")
        .expect("réglage livré")["valeur"]["valeur"]
        .clone()
}

#[tokio::test]
async fn eteinte_a_la_livraison_un_compte_de_maitre_n_entre_pas() {
    let e = ecole().await;
    let p = &e.p;
    assert_eq!(valeur_du_reglage(p, &e.admin).await, json!(false));
    let connexion = json!({ "identifiant": "sekou", "mot_de_passe": MOT_DE_PASSE_MAITRE });
    assert_eq!(
        p.refus("connexion", None, connexion.clone()).await.cle,
        "connexion.maitres_non_admis"
    );
    // Un mot de passe faux ne dit rien du compte.
    assert_eq!(
        p.refus(
            "connexion",
            None,
            json!({ "identifiant": "sekou", "mot_de_passe": "autre-chose" })
        )
        .await
        .cle,
        "connexion.identifiants_invalides"
    );
    // Un compte qui porte aussi un autre rôle n'est pas un compte de maître : il entre.
    p.ok(
        "utilisateur_creer",
        Some(&e.admin),
        json!({ "identifiant": "fanta", "nom_affiche": "Fanta Diallo",
            "mot_de_passe_initial": MOT_DE_PASSE_MAITRE, "roles": ["enseignant", "surveillant"] }),
    )
    .await;
    let fanta = p.connecter("fanta", MOT_DE_PASSE_MAITRE).await;
    let courante = p.ok("session_courante", Some(&fanta), json!(null)).await;
    assert_eq!(courante["maitre"], false);
    p.ok(
        "eleve_lire",
        Some(&fanta),
        json!({ "id": e.binta["eleve"]["id"] }),
    )
    .await;

    // La direction allume le réglage : le maître entre ; elle l'éteint : sa session se ferme à
    // sa commande suivante, et il n'entre plus.
    regler_saisie_des_maitres(p, &e.admin, true).await;
    let sekou = p.connecter("sekou", MOT_DE_PASSE_MAITRE).await;
    let courante = p.ok("session_courante", Some(&sekou), json!(null)).await;
    assert_eq!(courante["maitre"], true);
    regler_saisie_des_maitres(p, &e.admin, false).await;
    assert_eq!(
        p.refus("session_courante", Some(&sekou), json!(null))
            .await
            .cle,
        "session.invalide"
    );
    assert_eq!(
        p.refus("connexion", None, connexion).await.cle,
        "connexion.maitres_non_admis"
    );
}

#[tokio::test]
async fn allumee_le_maitre_ne_demande_que_les_commandes_de_sa_liste() {
    let e = ecole().await;
    let p = &e.p;
    regler_saisie_des_maitres(p, &e.admin, true).await;
    let sekou = p.connecter("sekou", MOT_DE_PASSE_MAITRE).await;

    // Chaque commande du contrat, jouée par le maître : hors de sa liste, la garde la refuse
    // avant même de lire l'entrée ; dans sa liste, ni la garde ni ses droits ne s'y opposent.
    let mut permises = Vec::new();
    for commande in COMMANDES {
        if commande.acces == Acces::Public
            || commande.nom == "deconnexion"
            || commande.nom == "verrouiller"
        {
            continue;
        }
        let rendu = p.appeler(commande.nom, Some(&sekou), json!(null)).await;
        let cle = rendu.as_ref().err().map(|e| e.cle.clone());
        if COMMANDES_DU_MAITRE.contains(&commande.nom) {
            assert!(
                !matches!(
                    cle.as_deref(),
                    Some("maitre.commande_refusee" | "erreur.permission_refusee")
                ),
                "{} est dans la liste du maître mais lui est refusée : {cle:?}",
                commande.nom
            );
            permises.push(commande.nom);
        } else {
            assert_eq!(
                cle.as_deref(),
                Some("maitre.commande_refusee"),
                "{} n'est pas dans la liste du maître",
                commande.nom
            );
        }
    }
    // La liste écrite est exactement ce qui lui est ouvert.
    permises.extend(["deconnexion", "verrouiller"]);
    permises.sort_unstable();
    let mut liste: Vec<&str> = COMMANDES_DU_MAITRE.to_vec();
    liste.sort_unstable();
    assert_eq!(permises, liste);
    // Rien de ce qui touche à l'argent, aux réglages ou aux comptes n'y figure.
    for nom in [
        "situation_financiere",
        "releve_compte_produire",
        "grilles_lister",
        "guichet_lire",
        "impayes_lister",
        "parametres_lister",
        "utilisateurs_lister",
        "audit_lister",
        "eleve_modifier",
        "evaluation_valider",
        "bulletins_produire",
    ] {
        assert_eq!(
            p.refus(nom, Some(&sekou), json!(null)).await.cle,
            "maitre.commande_refusee",
            "{nom}"
        );
    }
    // Il se déconnecte comme tout le monde.
    p.ok("deconnexion", Some(&sekou), json!(null)).await;
}

#[tokio::test]
async fn allumee_le_maitre_ne_lit_ni_n_ecrit_hors_de_ses_classes() {
    let e = ecole().await;
    let p = &e.p;
    regler_saisie_des_maitres(p, &e.admin, true).await;
    let sekou = p.connecter("sekou", MOT_DE_PASSE_MAITRE).await;
    let hors_portee = "erreur.portee_refusee";

    // Les élèves : ceux de sa classe seulement, à la recherche comme à la fiche.
    let noms = |trouves: &Value| -> Vec<String> {
        trouves
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x["prenoms"].as_str().unwrap().to_owned())
            .collect()
    };
    let trouver = |jeton: Jeton, texte: &'static str| async move {
        p.ok("eleves_rechercher", Some(&jeton), json!({ "texte": texte }))
            .await
    };
    assert_eq!(noms(&trouver(sekou.clone(), "Awa").await), ["Awa"]);
    assert!(noms(&trouver(sekou.clone(), "Binta").await).is_empty());
    assert_eq!(noms(&trouver(e.admin.clone(), "Binta").await), ["Binta"]);
    // Par son matricule non plus.
    let matricule = e.binta["eleve"]["matricule_interne"].as_str().unwrap();
    assert!(
        p.ok(
            "eleves_rechercher",
            Some(&sekou),
            json!({ "texte": matricule })
        )
        .await
        .as_array()
        .unwrap()
        .is_empty()
    );
    let fiche = p
        .ok(
            "eleve_lire",
            Some(&sekou),
            json!({ "id": e.awa["eleve"]["id"] }),
        )
        .await;
    assert_eq!(fiche["identite"]["prenoms"], "Awa");
    assert_eq!(
        p.refus(
            "eleve_lire",
            Some(&sekou),
            json!({ "id": e.binta["eleve"]["id"] })
        )
        .await
        .cle,
        hors_portee
    );
    // La situation financière, même d'une élève de sa classe, ne lui est pas ouverte.
    assert_eq!(
        p.refus(
            "situation_financiere",
            Some(&sekou),
            json!({ "eleve_id": e.awa["eleve"]["id"] })
        )
        .await
        .cle,
        "maitre.commande_refusee"
    );

    // Les notes : dans sa classe et sa matière.
    let lister = |classe: &Value| json!({ "classe_id": classe, "periode_id": e.trimestre });
    let siennes = p.ok("evaluations_lister", Some(&sekou), lister(&e.a)).await;
    assert_eq!(siennes.as_array().unwrap().len(), 1);
    let autres = p.ok("evaluations_lister", Some(&sekou), lister(&e.b)).await;
    assert!(autres.as_array().unwrap().is_empty());
    let grille = p
        .ok(
            "notes_grille",
            Some(&sekou),
            json!({ "id": e.devoir_a["id"] }),
        )
        .await;
    let ligne = grille["lignes"][0]["inscription_id"].clone();
    let note = p
        .ok(
            "note_saisir",
            Some(&sekou),
            json!({ "evaluation_id": e.devoir_a["id"], "inscription_id": ligne, "saisie": "14" }),
        )
        .await;
    assert_eq!(note["valeur"], 1400);
    assert_eq!(
        p.refus(
            "notes_grille",
            Some(&sekou),
            json!({ "id": e.devoir_b["id"] })
        )
        .await
        .cle,
        hors_portee
    );
    let grille_b = p
        .ok(
            "notes_grille",
            Some(&e.admin),
            json!({ "id": e.devoir_b["id"] }),
        )
        .await;
    assert_eq!(
        p.refus(
            "note_saisir",
            Some(&sekou),
            json!({ "evaluation_id": e.devoir_b["id"],
                "inscription_id": grille_b["lignes"][0]["inscription_id"], "saisie": "14" })
        )
        .await
        .cle,
        hors_portee
    );
    assert_eq!(
        p.refus(
            "evaluation_creer",
            Some(&sekou),
            json!({ "classe_id": e.b, "periode_id": e.trimestre, "matiere_id": e.math,
                "type_evaluation": "devoir", "libelle": "Devoir 2", "date": "2026-10-11", "bareme": null })
        )
        .await
        .cle,
        hors_portee
    );

    // Les bulletins : ceux de sa classe.
    let bulletins = |classe: &Value| json!({ "classe_id": classe, "periode_id": e.trimestre });
    p.ok("bulletins_lire", Some(&sekou), bulletins(&e.a)).await;
    assert_eq!(
        p.refus("bulletins_lire", Some(&sekou), bulletins(&e.b))
            .await
            .cle,
        hors_portee
    );
    p.ok("bulletins_lire", Some(&e.admin), bulletins(&e.b))
        .await;

    // Les absences : celles de sa classe.
    let releve = |classe: &Value| json!({ "classe_id": classe, "date": "2026-10-05" });
    p.ok("absences_releve_lire", Some(&sekou), releve(&e.a))
        .await;
    for (commande, entree) in [
        ("absences_releve_lire", releve(&e.b)),
        (
            "absences_releve_enregistrer",
            json!({ "classe_id": e.b, "date": "2026-10-05", "lignes": [] }),
        ),
        (
            "absences_periode_lire",
            json!({ "classe_id": e.b, "periode_id": e.trimestre }),
        ),
        ("absences_seances_lire", releve(&e.b)),
        (
            "absences_seances_enregistrer",
            json!({ "classe_id": e.b, "date": "2026-10-05", "lignes": [] }),
        ),
    ] {
        let cle = p.refus(commande, Some(&sekou), entree).await.cle;
        assert!(
            cle == hors_portee || cle == "absence.par_seance_inactif",
            "{commande} : {cle}"
        );
    }
    assert_eq!(
        p.refus("absences_seances_lire", Some(&sekou), releve(&e.b))
            .await
            .cle,
        hors_portee
    );
}

#[tokio::test]
async fn une_ecole_ou_un_maitre_a_deja_saisi_garde_la_possibilite_a_la_mise_a_niveau() {
    use sqlx::Connection;
    let e = ecole().await;
    regler_saisie_des_maitres(&e.p, &e.admin, true).await;
    let effacer = "DELETE FROM parametre WHERE cle = 'notes.saisie_maitres'";

    // Une base d'avant ce réglage où aucun maître n'a saisi : il arrive éteint.
    let Ecole {
        p,
        admin,
        devoir_a,
        _dossier,
        ..
    } = e;
    let sekou = p.connecter("sekou", MOT_DE_PASSE_MAITRE).await;
    let grille = p
        .ok(
            "notes_grille",
            Some(&sekou),
            json!({ "id": devoir_a["id"] }),
        )
        .await;
    let ligne = grille["lignes"][0]["inscription_id"].clone();
    drop(admin);
    let Poste_ {
        application,
        horloge,
        trousseau,
        racine,
    } = p;
    drop(application);
    let mut cnx = base_brute(&racine, trousseau.clone()).await;
    sqlx::query(effacer).execute(&mut cnx).await.unwrap();
    cnx.close().await.unwrap();
    let p = demarrer(&racine, trousseau.clone(), horloge.clone()).await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    assert_eq!(valeur_du_reglage(&p, &admin).await, json!(false));

    // Le maître saisit une note, puis la base repasse par la mise à niveau : il arrive allumé.
    regler_saisie_des_maitres(&p, &admin, true).await;
    let sekou = p.connecter("sekou", MOT_DE_PASSE_MAITRE).await;
    p.ok(
        "note_saisir",
        Some(&sekou),
        json!({ "evaluation_id": devoir_a["id"], "inscription_id": ligne, "saisie": "11" }),
    )
    .await;
    let Poste_ { application, .. } = p;
    drop(application);
    let mut cnx = base_brute(&racine, trousseau.clone()).await;
    sqlx::query(effacer).execute(&mut cnx).await.unwrap();
    cnx.close().await.unwrap();
    let p = demarrer(&racine, trousseau, horloge).await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    assert_eq!(valeur_du_reglage(&p, &admin).await, json!(true));
    p.connecter("sekou", MOT_DE_PASSE_MAITRE).await;
}
