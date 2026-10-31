//! Code d'accès et consultation par SMS entrant (SPEC_PHASE_5 §9, étape 7) : le code se remet au
//! guichet et ne se garde qu'en empreinte ; « SOLDE code », « ABS code », « NOTE code » reçoivent
//! leur réponse ; un numéro inconnu ne reçoit rien, un code faux une seule réponse par jour, cinq
//! codes faux bloquent le numéro jusqu'au lendemain, le quota coupe les réponses ; un code
//! régénéré ou révoqué n'ouvre plus rien ; la réponse part même hors des heures d'envoi ; la clé
//! simulée reçoit et répond par ses trames PDU.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use crate::commun::*;
use gesco_services::file_sms::BilanFile;
use gesco_services::session::Jeton;
use serde_json::{Value, json};

const JOUR: i64 = 86_400_000;
const MARIAM: &str = "+22376000001";
const FANTA: &str = "+22376000002";

fn id_par(liste: &Value, champ: &str, valeur: &str) -> Value {
    liste
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x[champ] == valeur)
        .unwrap_or_else(|| panic!("{champ} = {valeur} absent"))["id"]
        .clone()
}

fn identite(prenoms: &str, sexe: &str, telephones: Value) -> Value {
    json!({ "nom": "Traoré", "prenoms": prenoms, "sexe": sexe, "date_naissance": "2014-03-02",
        "date_naissance_estimee": false, "lieu_naissance": "Bamako", "nationalite": "Malienne",
        "adresse": "", "telephones": telephones })
}

async fn regler(p: &Poste_, admin: &Jeton, cle: &str, valeur: Value) {
    let type_valeur = match &valeur {
        Value::Bool(_) => "booleen",
        Value::Number(_) => "entier",
        _ => "texte",
    };
    p.ok(
        "parametre_definir",
        Some(admin),
        json!({ "cle": cle, "valeur": { "type": type_valeur, "valeur": valeur } }),
    )
    .await;
}

async fn traiter(p: &Poste_) -> BilanFile {
    p.application.traiter_file_sms().await.unwrap()
}

/// Un SMS arrive sur le canal simulé et la file passe.
async fn recevoir(p: &Poste_, numero: &str, texte: &str) -> BilanFile {
    p.application.canal_simule().recevoir(numero, texte);
    traiter(p).await
}

/// Le dernier SMS reçu.
async fn dernier_recu(p: &Poste_, admin: &Jeton) -> Value {
    p.ok("sms_recus_lire", Some(admin), Value::Null).await["sms"][0].clone()
}

/// Textes partis vers un numéro sur le canal simulé.
fn partis_vers(p: &Poste_, numero: &str) -> Vec<String> {
    p.application
        .canal_simule()
        .envoyes()
        .into_iter()
        .filter(|(n, _)| n == numero)
        .map(|(_, t)| t)
        .collect()
}

struct Famille {
    p: Poste_,
    admin: Jeton,
    mariam: Value,
    fanta: Value,
    awa: Value,
}

async fn famille(dossier: &std::path::Path) -> Famille {
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
    let septieme = id_par(&ped["niveaux"], "code", "7");
    let classe = p
        .ok(
            "classe_creer",
            Some(&admin),
            json!({ "annee_id": annee["id"], "niveau_id": septieme, "serie_id": null, "libelle": "7e A" }),
        )
        .await["id"]
        .clone();
    regler(&p, &admin, "sms.actif", json!(true)).await;
    regler(&p, &admin, "sms.canal", json!("simule")).await;
    regler(&p, &admin, "sms.signature", json!("GS Les Etoiles")).await;

    // Awa et Moussa, enfants de Mariam ; Binta, fille de Fanta.
    let mut fiches = Vec::new();
    for (prenom, sexe) in [
        ("Awa Kadiatou", "feminin"),
        ("Moussa", "masculin"),
        ("Binta", "feminin"),
    ] {
        fiches.push(
            p.ok(
                "eleve_creer",
                Some(&admin),
                json!({ "identite": identite(prenom, sexe, json!([])), "matricule_interne": "",
                    "matricule_national": "", "acte_nature": "acte", "acte_numero": "", "acte_date": "",
                    "ecole_origine": "" }),
            )
            .await,
        );
    }
    let qualites = json!({ "lien": "mere", "est_responsable_legal": true, "est_payeur": true,
        "recoit_notifications": true, "vient_chercher": false, "ordre_priorite": 1 });
    let awa = p
        .ok(
            "responsable_rattacher",
            Some(&admin),
            json!({ "eleve_id": fiches[0]["eleve"]["id"], "responsable_id": null,
                "saisie": { "identite": identite("Mariam", "feminin", json!(["76 00 00 01"])), "profession": "", "employeur": "" },
                "qualites": qualites }),
        )
        .await;
    let mariam = awa["responsables"][0]["responsable"]["id"].clone();
    p.ok(
        "responsable_rattacher",
        Some(&admin),
        json!({ "eleve_id": fiches[1]["eleve"]["id"], "responsable_id": mariam, "saisie": null, "qualites": qualites }),
    )
    .await;
    let binta = p
        .ok(
            "responsable_rattacher",
            Some(&admin),
            json!({ "eleve_id": fiches[2]["eleve"]["id"], "responsable_id": null,
                "saisie": { "identite": identite("Fanta", "feminin", json!(["76 00 00 02"])), "profession": "", "employeur": "" },
                "qualites": qualites }),
        )
        .await;
    let fanta = binta["responsables"][0]["responsable"]["id"].clone();
    let mut inscriptions = Vec::new();
    for fiche in &fiches {
        inscriptions.push(
            p.ok(
                "inscription_creer",
                Some(&admin),
                json!({ "eleve_id": fiche["eleve"]["id"], "classe_id": classe, "date_inscription": "",
                    "redoublant": false, "etablissement_origine": "" }),
            )
            .await["inscription"]["id"]
                .clone(),
        );
    }
    p.ok(
        "inscriptions_valider_classe",
        Some(&admin),
        json!({ "id": classe }),
    )
    .await;
    // Awa absente ce matin, sans justification.
    p.ok(
        "absences_releve_enregistrer",
        Some(&admin),
        json!({ "classe_id": classe, "date": "2026-10-04", "lignes": [
            { "inscription_id": inscriptions[0], "matin": "a", "apres_midi": "" }] }),
    )
    .await;
    // L'avis d'absence du jour n'est pas l'objet de ces essais.
    regler(&p, &admin, "sms.auto_absence", json!(false)).await;
    Famille {
        p,
        admin,
        mariam,
        fanta,
        awa: fiches[0]["eleve"]["id"].clone(),
    }
}

/// Remet un code et rend sa forme à taper, sans espace.
async fn remettre(p: &Poste_, admin: &Jeton, responsable: &Value) -> String {
    let remis = p
        .ok(
            "code_acces_remettre",
            Some(admin),
            json!({ "responsable_id": responsable }),
        )
        .await;
    assert!(remis["pdf"].as_str().unwrap().len() > 1_000, "le billet");
    assert_eq!(remis["etat"]["actif"]["etat"], "actif");
    assert!(
        remis["etat"]["actif"].get("empreinte").is_none(),
        "l'empreinte ne sort pas du cœur"
    );
    let code = remis["code"].as_str().unwrap().replace(' ', "");
    assert_eq!(code.len(), 6);
    assert!(!code.contains(['O', '0', 'I', '1']));
    code
}

/// Un SMS arrive, la file passe : rend l'issue notée au journal.
async fn issue(p: &Poste_, admin: &Jeton, numero: &str, texte: &str) -> String {
    recevoir(p, numero, texte).await;
    dernier_recu(p, admin).await["sms"]["issue"]
        .as_str()
        .unwrap()
        .to_owned()
}

// Le scénario est découpé en étapes : en compilation de débogage, une seule fonction asynchrone
// aussi longue déborderait la pile du test.
#[tokio::test]
async fn un_parent_consulte_par_sms_avec_son_code() {
    let dossier = tempfile::tempdir().unwrap();
    let f = famille(dossier.path()).await;
    let code = remettre(&f.p, &f.admin, &f.mariam).await;
    fermee_puis_ouverte(&f, &code).await;
    reponses_servies(&f, &code).await;
    refus_et_quota(&f, &code).await;
    let code_fanta = blocage(&f).await;
    let admin = f.p.connecter("admin", MOT_DE_PASSE).await;
    regeneration_et_revocation(&f, &admin, &code).await;
    hors_des_heures(&f, &code_fanta).await;
}

async fn fermee_puis_ouverte(f: &Famille, code: &str) {
    let (p, admin) = (&f.p, &f.admin);
    // Le guichet montre que Mariam a un code, jamais le code lui-même.
    let guichet = p
        .ok("guichet_lire", Some(admin), json!({ "eleve_id": f.awa }))
        .await;
    assert_eq!(guichet["droits"]["codes"], true);
    assert_eq!(
        guichet["responsables"][0]["code_acces"]["actif"]["etat"],
        "actif"
    );
    assert!(!guichet.to_string().contains(code));
    let lu = p.ok("sms_recus_lire", Some(admin), Value::Null).await;
    assert_eq!(lu["consultation_active"], false);
    assert_eq!(lu["simulation"], true);

    // Consultation fermée : le SMS est au journal, rien ne part.
    let bilan = recevoir(p, MARIAM, &format!("SOLDE {code}")).await;
    assert_eq!(bilan.recus, 1);
    let recu = dernier_recu(p, admin).await;
    assert_eq!(recu["sms"]["issue"], "inactive");
    assert_eq!(recu["sms"]["texte"], "SOLDE ******", "le code est masqué");
    assert!(partis_vers(p, MARIAM).is_empty());
    regler(p, admin, "sms.consultation_active", json!(true)).await;
}

async fn reponses_servies(f: &Famille, code: &str) {
    let (p, admin) = (&f.p, &f.admin);
    // SOLDE : le reste de chaque enfant, la part échue ; la réponse part au même passage.
    let bilan = recevoir(p, MARIAM, &format!("solde {}", code.to_lowercase())).await;
    assert_eq!((bilan.recus, bilan.envoyes), (1, 1));
    let recu = dernier_recu(p, admin).await;
    assert_eq!(recu["sms"]["issue"], "servie");
    assert_eq!(recu["sms"]["mot"], "solde");
    assert_eq!(recu["responsable"], "Mariam Traoré");
    let solde = partis_vers(p, MARIAM).pop().unwrap();
    assert!(
        solde.starts_with("GS Les Etoiles : reste a payer Awa "),
        "{solde}"
    );
    assert!(solde.contains("dont 10 000 F echus"), "{solde}");
    assert!(solde.contains("Moussa "), "{solde}");
    assert!(!solde.contains("Binta"), "{solde}");
    assert_eq!(recu["reponse"], solde);

    // ABS : les absences de la période en cours.
    assert_eq!(
        issue(p, admin, MARIAM, &format!("ABS {code}")).await,
        "servie"
    );
    let abs = partis_vers(p, MARIAM).pop().unwrap();
    assert!(abs.starts_with("GS Les Etoiles : absences "), "{abs}");
    assert!(
        abs.contains("Awa 1 demi-journee(s) dont 1 non justifiee(s)"),
        "{abs}"
    );
    assert!(abs.contains("Moussa aucune"), "{abs}");

    // NOTE : aucun bulletin publié encore.
    assert_eq!(
        issue(p, admin, MARIAM, &format!("Note {code}")).await,
        "servie"
    );
    assert_eq!(
        partis_vers(p, MARIAM).pop().unwrap(),
        "GS Les Etoiles : moyennes Awa pas encore de bulletin ; Moussa pas encore de bulletin."
    );
}

async fn refus_et_quota(f: &Famille, code: &str) {
    let (p, admin) = (&f.p, &f.admin);
    // Un numéro inconnu ne reçoit rien.
    let avant = p.application.canal_simule().envoyes().len();
    let inconnu = issue(p, admin, "+22370000000", &format!("SOLDE {code}")).await;
    assert_eq!(inconnu, "numero_inconnu");
    assert_eq!(p.application.canal_simule().envoyes().len(), avant);

    // Le code de Mariam depuis le numéro de Fanta : code incorrect, répondu une fois par jour.
    let faux = issue(p, admin, FANTA, &format!("SOLDE {code}")).await;
    assert_eq!(faux, "code_incorrect");
    assert_eq!(
        partis_vers(p, FANTA),
        ["GS Les Etoiles : code incorrect. Votre code est sur le billet remis par l'ecole."]
    );
    let muet = issue(p, admin, FANTA, "SOLDE AAAAAA").await;
    assert_eq!(muet, "code_incorrect_muet");
    assert_eq!(partis_vers(p, FANTA).len(), 1);

    // Le quota du jour : AIDE et un SMS incompris comptent ; au-delà, plus rien.
    assert_eq!(issue(p, admin, MARIAM, "aide").await, "aide");
    assert!(
        partis_vers(p, MARIAM)
            .pop()
            .unwrap()
            .contains("envoyez SOLDE, ABS, NOTE suivi de votre code")
    );
    assert_eq!(issue(p, admin, MARIAM, "Merci beaucoup").await, "aide");
    let partis = partis_vers(p, MARIAM).len();
    let quota = issue(p, admin, MARIAM, &format!("SOLDE {code}")).await;
    assert_eq!(quota, "quota_depasse");
    assert_eq!(partis_vers(p, MARIAM).len(), partis);
}

/// Cinq codes faux bloquent le numéro jusqu'au lendemain, même avec le bon code ; le lendemain,
/// il est débloqué. Rend le code de Fanta.
async fn blocage(f: &Famille) -> String {
    let (p, admin) = (&f.p, &f.admin);
    regler(p, admin, "sms.consultation_quota_jour", json!(20)).await;
    let code_fanta = remettre(p, admin, &f.fanta).await;
    for _ in 0..3 {
        recevoir(p, FANTA, "ABS BBBBBB").await;
    }
    let bloquee = issue(p, admin, FANTA, &format!("ABS {code_fanta}")).await;
    assert_eq!(bloquee, "bloquee");
    assert_eq!(partis_vers(p, FANTA).len(), 1);

    p.horloge.avancer(JOUR);
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    let servie = issue(p, &admin, FANTA, &format!("ABS {code_fanta}")).await;
    assert_eq!(servie, "servie");
    assert!(
        partis_vers(p, FANTA)
            .pop()
            .unwrap()
            .contains("Binta aucune")
    );
    code_fanta
}

async fn regeneration_et_revocation(f: &Famille, admin: &Jeton, code: &str) {
    let p = &f.p;
    // Un code régénéré : l'ancien ne sert plus, le nouveau sert.
    let nouveau = remettre(p, admin, &f.mariam).await;
    assert_ne!(nouveau, code);
    let ancien = issue(p, admin, MARIAM, &format!("SOLDE {code}")).await;
    assert_eq!(ancien, "code_incorrect");
    let servie = issue(p, admin, MARIAM, &format!("SOLDE {nouveau}")).await;
    assert_eq!(servie, "servie");

    // Révoqué, il n'ouvre plus rien ; une seconde révocation n'a plus d'objet.
    let etat = p
        .ok(
            "code_acces_revoquer",
            Some(admin),
            json!({ "responsable_id": f.mariam }),
        )
        .await;
    assert_eq!(etat["actif"], Value::Null);
    assert!(etat["revoque_le"].is_number());
    let refus = p
        .refus(
            "code_acces_revoquer",
            Some(admin),
            json!({ "responsable_id": f.mariam }),
        )
        .await;
    assert!(
        format!("{refus:?}").contains("code_acces.aucun"),
        "{refus:?}"
    );
    let revoque = issue(p, admin, MARIAM, &format!("SOLDE {nouveau}")).await;
    assert_eq!(revoque, "code_incorrect_muet");
    let guichet = p
        .ok("guichet_lire", Some(admin), json!({ "eleve_id": f.awa }))
        .await;
    let code_mariam = &guichet["responsables"][0]["code_acces"];
    assert_eq!(code_mariam["actif"], Value::Null);
    assert!(code_mariam["revoque_le"].is_number());
}

/// Hors des heures d'envoi (21 h), la réponse part ; l'essai attend le matin.
async fn hors_des_heures(f: &Famille, code_fanta: &str) {
    let p = &f.p;
    p.horloge.avancer(13 * 3_600_000);
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    p.ok(
        "message_essai_envoyer",
        Some(&admin),
        json!({ "numero": "76 00 00 09" }),
    )
    .await;
    let bilan = recevoir(p, FANTA, &format!("SOLDE {code_fanta}")).await;
    assert_eq!(bilan.envoyes, 1);
    assert!(
        partis_vers(p, FANTA)
            .pop()
            .unwrap()
            .contains("reste a payer Binta")
    );
    assert!(partis_vers(p, "+22376000009").is_empty());
    let journal = p.ok("sms_recus_lire", Some(&admin), Value::Null).await;
    assert!(journal["sms"].as_array().unwrap().len() >= 18);
}

#[tokio::test]
async fn la_cle_simulee_recoit_et_repond() {
    let dossier = tempfile::tempdir().unwrap();
    let Famille {
        p, admin, mariam, ..
    } = famille(dossier.path()).await;
    let code = remettre(&p, &admin, &mariam).await;
    regler(&p, &admin, "sms.consultation_active", json!(true)).await;
    regler(&p, &admin, "sms.canal", json!("modem")).await;
    p.ok(
        "sms_port_choisir",
        Some(&admin),
        json!({ "port": "simule" }),
    )
    .await;
    // Un numéro qui n'est pas un téléphone est refusé ; le SMS simulé arrive par la clé.
    let refus = p
        .refus(
            "sms_reception_simuler",
            Some(&admin),
            json!({ "numero": "abc", "texte": "SOLDE" }),
        )
        .await;
    assert!(
        format!("{refus:?}").contains("telephone.invalide"),
        "{refus:?}"
    );
    p.ok(
        "sms_reception_simuler",
        Some(&admin),
        json!({ "numero": "76 00 00 01", "texte": format!("Solde {code}") }),
    )
    .await;
    let bilan = traiter(&p).await;
    let recu = dernier_recu(&p, &admin).await;
    assert_eq!((bilan.recus, bilan.envoyes), (1, 1), "{bilan:?} {recu}");
    assert_eq!(
        p.application.modem_simule().sur_la_puce(),
        0,
        "effacé de la puce"
    );
    assert!(
        p.application
            .modem_simule()
            .envoyes()
            .iter()
            .any(|(n, t)| n == MARIAM && t.starts_with("GS Les Etoiles : reste a payer Awa"))
    );
    // Un SMS écrit avec des accents (UCS-2) se comprend aussi.
    p.ok(
        "sms_reception_simuler",
        Some(&admin),
        json!({ "numero": MARIAM, "texte": format!("Absences élève {code}") }),
    )
    .await;
    traiter(&p).await;
    let recu = dernier_recu(&p, &admin).await;
    assert_eq!(recu["sms"]["mot"], "abs");
    // « Absences élève » : le mot est lu, mais « ELEVE » colle au code, qui est donc faux.
    assert_eq!(recu["sms"]["issue"], "code_incorrect");

    // Sur une vraie clé, rien ne se simule.
    regler(&p, &admin, "sms.port", json!("COM99")).await;
    let refus = p
        .refus(
            "sms_reception_simuler",
            Some(&admin),
            json!({ "numero": MARIAM, "texte": "SOLDE" }),
        )
        .await;
    assert!(
        format!("{refus:?}").contains("sms.simulation_impossible"),
        "{refus:?}"
    );
}

async fn compte(p: &Poste_, admin: &Jeton, identifiant: &str, role: &str) -> Jeton {
    let mot_de_passe = format!("ecole-2026-{}", identifiant.len() * 7919);
    p.ok(
        "utilisateur_creer",
        Some(admin),
        json!({ "identifiant": identifiant, "nom_affiche": identifiant,
            "mot_de_passe_initial": mot_de_passe, "roles": [role] }),
    )
    .await;
    p.connecter(identifiant, &mot_de_passe).await
}

/// Rôles livrés en version 10 (SPEC_PHASE_5 §10).
#[tokio::test]
async fn les_roles_livres_et_les_messages_aux_familles() {
    let dossier = tempfile::tempdir().unwrap();
    let f = famille(dossier.path()).await;
    let (p, admin) = (&f.p, &f.admin);
    let secretaire = compte(p, admin, "secretaire", "secretaire").await;
    let caissier = compte(p, admin, "caissier", "caissier").await;
    let surveillant = compte(p, admin, "surveillant", "surveillant").await;
    let lecteur = compte(p, admin, "lecteur", "consultation_seule").await;
    let demande = json!({ "responsable_id": f.mariam });

    // Le Secrétaire remet les codes et voit la file ; il ne règle pas l'envoi.
    p.ok("code_acces_remettre", Some(&secretaire), demande.clone())
        .await;
    p.ok("messages_lire", Some(&secretaire), json!({ "etat": null }))
        .await;
    p.ok("sms_recus_lire", Some(&secretaire), Value::Null).await;
    p.refus(
        "message_essai_envoyer",
        Some(&secretaire),
        json!({ "numero": "76 00 00 09" }),
    )
    .await;

    // Le Caissier remet les bulletins au guichet, pas les codes ; la file ne le regarde pas.
    let guichet = p
        .ok(
            "guichet_lire",
            Some(&caissier),
            json!({ "eleve_id": f.awa }),
        )
        .await;
    assert_eq!(guichet["droits"]["remettre"], true);
    assert_eq!(guichet["droits"]["codes"], false);
    p.refus("code_acces_revoquer", Some(&caissier), demande.clone())
        .await;
    p.refus("messages_lire", Some(&caissier), json!({ "etat": null }))
        .await;

    // Le Surveillant général tient les convocations et décide des messages préparés.
    let file = p
        .ok("messages_lire", Some(&surveillant), json!({ "etat": null }))
        .await;
    assert_eq!(file["droits"]["decider"], true);
    assert_eq!(file["droits"]["configurer"], false);
    let guichet = p
        .ok(
            "guichet_lire",
            Some(&surveillant),
            json!({ "eleve_id": f.awa }),
        )
        .await;
    assert_eq!(guichet["droits"]["convocations"], true);
    p.refus("code_acces_remettre", Some(&surveillant), demande.clone())
        .await;

    // La Consultation seule lit la file et les SMS reçus, sans rien relancer ni décider.
    let file = p
        .ok("messages_lire", Some(&lecteur), json!({ "etat": null }))
        .await;
    assert_eq!(file["droits"]["renvoyer"], false);
    assert_eq!(file["droits"]["decider"], false);
    assert_eq!(file["droits"]["configurer"], false);
    p.ok("sms_recus_lire", Some(&lecteur), Value::Null).await;
    p.refus("code_acces_remettre", Some(&lecteur), demande)
        .await;
}

/// Les messages aux familles éteints : rien ne part ni n'arrive, aucun code ne se remet, leurs
/// écrans quittent la session ; rallumés, la file reprend. Un autre module éteint quitte le
/// tableau de bord.
#[tokio::test]
async fn les_modules_de_l_ecole_s_eteignent_et_se_rallument() {
    let dossier = tempfile::tempdir().unwrap();
    let f = famille(dossier.path()).await;
    let (p, admin) = (&f.p, &f.admin);
    let session = p.ok("session_courante", Some(admin), Value::Null).await;
    let modules = session["modules"].as_array().unwrap().clone();
    assert!(modules.contains(&json!("sms")));
    assert!(modules.contains(&json!("personnel")));
    assert!(
        !modules.contains(&json!("cantine")),
        "la cantine attend que l'école la veuille"
    );

    regler(p, admin, "sms.actif", json!(false)).await;
    p.ok(
        "message_essai_envoyer",
        Some(admin),
        json!({ "numero": "76 00 00 09" }),
    )
    .await;
    let bilan = recevoir(p, MARIAM, "SOLDE AAAAAA").await;
    assert_eq!((bilan.envoyes, bilan.recus), (0, 0));
    let refus = p
        .refus(
            "code_acces_remettre",
            Some(admin),
            json!({ "responsable_id": f.mariam }),
        )
        .await;
    assert!(
        format!("{refus:?}").contains("sms.module_inactif"),
        "{refus:?}"
    );
    let guichet = p
        .ok("guichet_lire", Some(admin), json!({ "eleve_id": f.awa }))
        .await;
    assert_eq!(guichet["sms_actif"], false);
    assert_eq!(guichet["droits"]["codes"], false);
    let session = p.ok("session_courante", Some(admin), Value::Null).await;
    assert!(
        !session["modules"]
            .as_array()
            .unwrap()
            .contains(&json!("sms"))
    );

    // Rallumé : l'essai en attente part, le SMS gardé par le canal arrive.
    regler(p, admin, "sms.actif", json!(true)).await;
    let bilan = traiter(p).await;
    assert_eq!((bilan.envoyes, bilan.recus), (1, 1));

    // Le personnel éteint quitte le tableau de bord, même pour l'administrateur.
    assert!(p.ok("tableau_de_bord_lire", Some(admin), Value::Null).await["personnel"].is_object());
    regler(p, admin, "module.personnel", json!(false)).await;
    let tableau = p.ok("tableau_de_bord_lire", Some(admin), Value::Null).await;
    assert_eq!(tableau["personnel"], Value::Null);
    assert_eq!(tableau["paie"], Value::Null);
    let session = p.ok("session_courante", Some(admin), Value::Null).await;
    assert!(
        !session["modules"]
            .as_array()
            .unwrap()
            .contains(&json!("personnel"))
    );
}
