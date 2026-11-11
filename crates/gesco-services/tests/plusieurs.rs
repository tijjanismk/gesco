//! Travailler à plusieurs (SPEC_PHASE_11_A §6) : deux personnes sur la même fiche, la liste des
//! fiches protégées comparée au contrat, et la restauration qui ferme toutes les sessions.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::HashSet;
use std::path::PathBuf;

use gesco_core::core::Poste;
use gesco_services::Origine;
use gesco_services::commandes::{COMMANDES, COMMANDES_DE_FICHE};
use gesco_services::session::Jeton;
use serde_json::{Value, json};

use crate::commun::*;
use crate::reseau_local::{champs_des_types, contrat_genere, entrees_du_contrat};

/// Clé du refus d'une fiche modifiée depuis sa lecture.
const MODIFIEE: &str = "fiche.modifiee_entre_temps";

#[test]
fn les_fiches_protegees_sont_celles_dont_l_entree_porte_l_instant_lu() {
    let texte = contrat_genere();
    let types = champs_des_types(&texte);
    let entrees = entrees_du_contrat(&texte);
    let vide = HashSet::new();
    for commande in COMMANDES {
        let entree = &entrees[commande.nom];
        let porte = types.get(entree).unwrap_or(&vide).contains("lue_le");
        let declaree = COMMANDES_DE_FICHE.contains(&commande.nom);
        assert_eq!(
            porte, declaree,
            "{} : `lue_le` dans {entree} = {porte}, déclarée dans COMMANDES_DE_FICHE = {declaree}",
            commande.nom
        );
    }
}

/// École installée, année ouverte, et deux personnes connectées : Awa Traoré et Fanta
/// Coulibaly, toutes deux administratrices.
async fn ecole(dossier: &std::path::Path) -> (Poste_, Jeton, Jeton, Value) {
    let p = poste_vierge(dossier).await;
    p.installer().await;
    let awa = p.connecter("admin", MOT_DE_PASSE).await;
    let annee = p
        .ok(
            "annee_creer",
            Some(&awa),
            json!({ "libelle": "2026-2027", "debut": RENTREE_2026, "fin": FIN_2027 }),
        )
        .await;
    p.ok("annee_activer", Some(&awa), json!({ "id": annee["id"] }))
        .await;
    p.ok(
        "utilisateur_creer",
        Some(&awa),
        json!({
            "identifiant": "fanta",
            "nom_affiche": "Fanta Coulibaly",
            "mot_de_passe_initial": "direction-2026-bureau",
            "roles": ["administrateur"],
        }),
    )
    .await;
    let fanta = p.connecter("fanta", "direction-2026-bureau").await;
    (p, awa, fanta, annee)
}

/// Awa vient d'enregistrer. Fanta, qui avait lu la fiche avant elle, est refusée : l'erreur
/// nomme Awa et l'heure de son enregistrement. Fanta relit et enregistre.
async fn seconde_refusee_puis_acceptee(
    p: &Poste_,
    fanta: &Jeton,
    commande: &str,
    entree: impl Fn(&Value) -> Value,
    lue_avant: &Value,
    lue_apres: &Value,
) -> Value {
    assert_ne!(lue_avant, lue_apres, "{commande} : l'instant n'a pas bougé");
    p.horloge.avancer(1_000);
    let refus = p.refus(commande, Some(fanta), entree(lue_avant)).await;
    assert_eq!(refus.cle, MODIFIEE, "{commande}");
    let modifiee = refus.modifiee.expect("qui et quand");
    assert_eq!(modifiee.par.as_deref(), Some("Awa Traoré"), "{commande}");
    assert_eq!(json!(modifiee.le), *lue_apres, "{commande}");
    p.horloge.avancer(1_000);
    p.ok(commande, Some(fanta), entree(lue_apres)).await
}

fn identite(nom: &str, prenoms: &str, telephones: Value) -> Value {
    json!({
        "nom": nom, "prenoms": prenoms, "sexe": "feminin", "date_naissance": "2014-03-02",
        "date_naissance_estimee": false, "lieu_naissance": "Bamako", "nationalite": "Malienne",
        "adresse": "", "telephones": telephones,
    })
}

#[tokio::test]
async fn la_seconde_modification_de_la_fiche_d_un_eleve_et_de_son_responsable_est_refusee() {
    let dossier = tempfile::tempdir().unwrap();
    let (p, awa, fanta, _) = ecole(dossier.path()).await;
    let saisie = |lieu: &str| {
        let mut identite = identite("Traoré", "Awa", json!([]));
        identite["lieu_naissance"] = json!(lieu);
        json!({
            "identite": identite, "matricule_interne": "", "matricule_national": "",
            "acte_nature": "acte", "acte_numero": "", "acte_date": "", "ecole_origine": "",
        })
    };
    let fiche = p.ok("eleve_creer", Some(&awa), saisie("Bamako")).await;
    let id = fiche["eleve"]["id"].clone();
    let lue = fiche["modifiee_le"].clone();
    assert!(lue.as_i64().unwrap() > 0);

    // Les deux ont la fiche sous les yeux. Awa corrige le lieu de naissance.
    p.horloge.avancer(1_000);
    let entree = |lieu: &'static str| {
        let id = id.clone();
        move |lue_le: &Value| json!({ "id": id, "lue_le": lue_le, "saisie": saisie(lieu) })
    };
    let par_awa = p
        .ok("eleve_modifier", Some(&awa), entree("Kati")(&lue))
        .await;
    // Fanta, qui corrige autre chose sur la fiche d'avant, n'écrase rien.
    let par_fanta = seconde_refusee_puis_acceptee(
        &p,
        &fanta,
        "eleve_modifier",
        entree("Ségou"),
        &lue,
        &par_awa["modifiee_le"],
    )
    .await;
    assert_eq!(par_fanta["identite"]["lieu_naissance"], "Ségou");

    // Même règle pour un responsable de l'élève.
    let mere = |telephone: &str| {
        json!({
            "identite": identite("Coulibaly", "Mariam", json!([telephone])),
            "profession": "", "employeur": "",
        })
    };
    let qualites = json!({
        "lien": "mere", "est_responsable_legal": true, "est_payeur": true,
        "recoit_notifications": true, "vient_chercher": false, "ordre_priorite": 1,
    });
    let fiche = p
        .ok(
            "responsable_rattacher",
            Some(&awa),
            json!({ "eleve_id": id, "responsable_id": null, "saisie": mere("76 12 34 56"), "qualites": qualites }),
        )
        .await;
    let responsable = fiche["responsables"][0].clone();
    let lue = responsable["modifiee_le"].clone();
    let entree = |telephone: &'static str| {
        let (id, responsable, qualites) = (id.clone(), responsable.clone(), qualites.clone());
        move |lue_le: &Value| {
            json!({
                "eleve_id": id, "responsable_id": responsable["responsable"]["id"],
                "lue_le": lue_le, "saisie": mere(telephone), "qualites": qualites,
            })
        }
    };
    p.horloge.avancer(1_000);
    let par_awa = p
        .ok(
            "responsable_modifier",
            Some(&awa),
            entree("66 00 11 22")(&lue),
        )
        .await;
    assert_eq!(
        par_awa["responsables"][0]["identite"]["telephones"],
        json!(["+22366001122"])
    );
    let par_fanta = seconde_refusee_puis_acceptee(
        &p,
        &fanta,
        "responsable_modifier",
        entree("65 43 21 00"),
        &lue,
        &par_awa["responsables"][0]["modifiee_le"],
    )
    .await;
    assert_eq!(
        par_fanta["responsables"][0]["identite"]["telephones"],
        json!(["+22365432100"])
    );
    // Le refus n'a rien écrit : entre les deux, le téléphone était resté celui d'Awa (vérifié
    // par l'instant, qui n'avait pas bougé), et la fiche de l'élève n'a pas changé d'instant.
    let relue = p.ok("eleve_lire", Some(&awa), json!({ "id": id })).await;
    assert_eq!(relue["identite"]["lieu_naissance"], "Ségou");
}

#[tokio::test]
async fn la_seconde_modification_d_une_classe_d_une_grille_et_de_l_etablissement_est_refusee() {
    let dossier = tempfile::tempdir().unwrap();
    let (p, awa, fanta, annee) = ecole(dossier.path()).await;

    // Classe.
    let referentiel = p.ok("niveaux_lister", Some(&awa), Value::Null).await;
    let sept = referentiel["niveaux"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["code"] == "7")
        .unwrap()
        .clone();
    let classe = p
        .ok(
            "classe_creer",
            Some(&awa),
            json!({ "annee_id": annee["id"], "niveau_id": sept["id"], "serie_id": null, "libelle": "7e A" }),
        )
        .await;
    let lue = classe["modifiee_le"].clone();
    let entree = |libelle: &'static str| {
        let id = classe["id"].clone();
        move |lue_le: &Value| json!({ "id": id, "lue_le": lue_le, "libelle": libelle })
    };
    p.horloge.avancer(1_000);
    let par_awa = p
        .ok("classe_modifier", Some(&awa), entree("7e Bleue")(&lue))
        .await;
    let par_fanta = seconde_refusee_puis_acceptee(
        &p,
        &fanta,
        "classe_modifier",
        entree("7e Verte"),
        &lue,
        &par_awa["modifiee_le"],
    )
    .await;
    assert_eq!(par_fanta["libelle"], "7e Verte");
    // La liste rend l'instant que la prochaine modification devra renvoyer.
    let classes = p
        .ok(
            "classes_lister",
            Some(&awa),
            json!({ "annee_id": annee["id"] }),
        )
        .await;
    assert_eq!(
        classes[0]["classe"]["modifiee_le"],
        par_fanta["modifiee_le"]
    );

    // Grille de tarifs, en brouillon.
    let grille = p
        .ok(
            "grille_creer",
            Some(&awa),
            json!({ "annee_id": annee["id"], "copie_de": null }),
        )
        .await;
    let plans: Vec<Value> = grille["contenu"]["plans"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| json!({ "libelle": p["libelle"], "par_defaut": p["par_defaut"], "tranches": p["tranches"] }))
        .collect();
    let lue = grille["modifiee_le"].clone();
    let entree = |montant: i64| {
        let (id, plans, sept) = (grille["grille"]["id"].clone(), plans.clone(), sept.clone());
        move |lue_le: &Value| {
            json!({
                "id": id, "lue_le": lue_le, "libelle": "Frais 2026-2027", "plans": plans,
                "lignes": [{ "niveau_id": sept["id"], "serie_id": null, "type_frais": "scolarite",
                    "libelle": "Scolarité", "montant": montant, "obligatoire": true,
                    "exigibilite": "selon_plan" }],
            })
        }
    };
    p.horloge.avancer(1_000);
    let par_awa = p
        .ok("grille_modifier", Some(&awa), entree(100_000)(&lue))
        .await;
    let par_fanta = seconde_refusee_puis_acceptee(
        &p,
        &fanta,
        "grille_modifier",
        entree(120_000),
        &lue,
        &par_awa["modifiee_le"],
    )
    .await;
    assert_eq!(par_fanta["contenu"]["lignes"][0]["montant"], 120_000);

    // Fiche de l'établissement.
    let fiche = p.ok("etablissement_lire", Some(&awa), Value::Null).await;
    let lue = fiche["modifiee_le"].clone();
    let entree = |adresse: &'static str| {
        move |lue_le: &Value| {
            json!({
                "lue_le": lue_le, "nom": "Lycée Askia Mohamed", "statut": "prive",
                "code_academie": "AE-BKO-RD", "code_cap": "CAP-01", "adresse": adresse,
                "telephones": ["+223 20 22 00 00"], "mentions_legales": "",
            })
        }
    };
    p.horloge.avancer(1_000);
    let par_awa = p
        .ok(
            "etablissement_modifier",
            Some(&awa),
            entree("Badalabougou")(&lue),
        )
        .await;
    let par_fanta = seconde_refusee_puis_acceptee(
        &p,
        &fanta,
        "etablissement_modifier",
        entree("Kalaban Coura"),
        &lue,
        &par_awa["modifiee_le"],
    )
    .await;
    assert_eq!(par_fanta["etablissement"]["adresse"], "Kalaban Coura");
}

#[tokio::test]
async fn la_seconde_modification_d_une_fiche_du_personnel_est_refusee() {
    let dossier = tempfile::tempdir().unwrap();
    let (p, awa, fanta, _) = ecole(dossier.path()).await;
    let referentiel = p
        .ok("personnel_referentiel_lire", Some(&awa), Value::Null)
        .await;
    let categorie = referentiel["categories"][0]["id"].clone();
    let membre = |id: Value, lue_le: Value, fonction: &str| {
        json!({
            "id": id, "lue_le": lue_le, "nom": "Coulibaly", "prenoms": "Awa", "sexe": "feminin",
            "date_naissance": null, "telephones": ["76 11 22 33"], "adresse": "Hamdallaye",
            "matricule": "", "categorie_id": categorie, "fonction": fonction,
            "arrive_le": "2024-10-01", "fonctionnaire": false, "numero_fonctionnaire": "",
            "numero_inps": "", "prevenir_nom": "", "prevenir_telephone": "",
            "paiement": { "mode": "especes" }, "diplomes": [], "utilisateur_id": null,
            "situation_famille": null, "enfants_a_charge": null,
        })
    };
    // Créer ne demande pas d'instant.
    let fiche = p
        .ok(
            "membre_enregistrer",
            Some(&awa),
            membre(Value::Null, Value::Null, "Secrétaire"),
        )
        .await;
    let id = fiche["membre"]["id"].clone();
    let lue = fiche["modifiee_le"].clone();
    // Modifier sans dire quand la fiche a été lue est refusé.
    assert_eq!(
        p.refus(
            "membre_enregistrer",
            Some(&awa),
            membre(id.clone(), Value::Null, "Comptable")
        )
        .await
        .cle,
        "commande.entree_invalide"
    );
    let entree = |fonction: &'static str| {
        let id = id.clone();
        let membre = &membre;
        move |lue_le: &Value| membre(id.clone(), lue_le.clone(), fonction)
    };
    p.horloge.avancer(1_000);
    let par_awa = p
        .ok("membre_enregistrer", Some(&awa), entree("Comptable")(&lue))
        .await;
    let par_fanta = seconde_refusee_puis_acceptee(
        &p,
        &fanta,
        "membre_enregistrer",
        entree("Surveillante"),
        &lue,
        &par_awa["modifiee_le"],
    )
    .await;
    assert_eq!(par_fanta["membre"]["fonction"], "Surveillante");
}

#[tokio::test]
async fn une_restauration_ferme_les_sessions_de_tous_les_postes() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    // Une session ouverte depuis un poste relié.
    let caisse = Origine::Relie(Poste("Caisse".to_owned()));
    let session = p
        .application
        .executer_depuis(
            caisse.clone(),
            "connexion",
            None,
            json!({ "identifiant": "admin", "mot_de_passe": MOT_DE_PASSE }),
        )
        .await
        .unwrap();
    let a_la_caisse: Jeton = serde_json::from_value(session["jeton"].clone()).unwrap();

    p.horloge.avancer(60_000);
    let lignes = p.ok("sauvegarde_creer", Some(&admin), Value::Null).await;
    let archive = PathBuf::from(lignes[0]["destination"].as_str().unwrap());
    p.horloge.avancer(60_000);
    p.ok(
        "sauvegarde_restaurer",
        Some(&admin),
        json!({ "chemin": archive, "phrase": null }),
    )
    .await;

    // Chacun se reconnecte, sur le poste principal comme sur les postes reliés.
    assert_eq!(
        p.refus("session_courante", Some(&admin), Value::Null)
            .await
            .cle,
        "session.invalide"
    );
    let refus = p
        .application
        .executer_depuis(caisse, "session_courante", Some(a_la_caisse), Value::Null)
        .await
        .unwrap_err();
    assert_eq!(refus.cle, "session.invalide");
}
