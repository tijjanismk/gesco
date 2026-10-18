//! Messages qui partent d'eux-mêmes (SPEC_PHASE_5 §5, étape 4) : l'avis d'absence attend son
//! délai de grâce, se complète, s'annule si l'absence est corrigée ou justifiée ; la relance
//! d'impayé respecte le délai, l'intervalle, le nombre maximal et le consentement, et tombe si
//! l'échéance est soldée avant l'envoi ; l'avis de sanction préparé attend une décision.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod commun;

use commun::*;
use gesco_services::file_sms::BilanFile;
use gesco_services::session::Jeton;
use serde_json::{Value, json};

const JOUR: i64 = 86_400_000;

fn id_par(liste: &Value, champ: &str, valeur: &str) -> Value {
    liste
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x[champ] == valeur)
        .unwrap_or_else(|| panic!("{champ} = {valeur} absent"))["id"]
        .clone()
}

/// Élève avec sa mère, responsable légale et payeuse, joignable à son propre numéro.
async fn eleve(p: &Poste_, admin: &Jeton, prenoms: &str, telephone: &str) -> Value {
    let fiche = p
        .ok(
            "eleve_creer",
            Some(admin),
            json!({ "identite": { "nom": "Traoré", "prenoms": prenoms, "sexe": "feminin",
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
                "adresse": "", "telephones": [telephone] }, "profession": "", "employeur": "" },
            "qualites": { "lien": "mere", "est_responsable_legal": true, "est_payeur": true,
                "recoit_notifications": true, "vient_chercher": false, "ordre_priorite": 1 } }),
    )
    .await
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

/// Messages vers un numéro, du plus récent au plus ancien.
async fn vers(p: &Poste_, admin: &Jeton, numero: &str) -> Vec<Value> {
    p.ok("messages_lire", Some(admin), json!({ "etat": null }))
        .await["messages"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|m| m["message"]["numero"] == numero)
        .map(|m| m["message"].clone())
        .collect()
}

struct Ecole {
    p: Poste_,
    admin: Jeton,
    classe: Value,
    /// Inscriptions d'Awa, Binta et Coumba, et la fiche de chacune (responsable compris).
    eleves: Vec<(Value, Value)>,
}

async fn ecole(dossier: &std::path::Path) -> Ecole {
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
    let mut fiches = Vec::new();
    for (prenom, telephone) in [
        ("Awa", "76 00 00 01"),
        ("Binta", "76 00 00 02"),
        ("Coumba", "76 00 00 03"),
    ] {
        let fiche = eleve(&p, &admin, prenom, telephone).await;
        p.ok(
            "inscription_creer",
            Some(&admin),
            json!({ "eleve_id": fiche["eleve"]["id"], "classe_id": classe, "date_inscription": "",
                "redoublant": false, "etablissement_origine": "" }),
        )
        .await;
        fiches.push(fiche);
    }
    p.ok(
        "inscriptions_valider_classe",
        Some(&admin),
        json!({ "id": classe }),
    )
    .await;
    let releve = p
        .ok(
            "absences_releve_lire",
            Some(&admin),
            json!({ "classe_id": classe, "date": "2026-10-04" }),
        )
        .await;
    let eleves = releve["lignes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["inscription_id"].clone())
        .zip(fiches)
        .collect();
    regler(&p, &admin, "sms.canal", json!("simule")).await;
    regler(&p, &admin, "sms.fenetre_envoi", json!("0-24")).await;
    Ecole {
        p,
        admin,
        classe,
        eleves,
    }
}

#[tokio::test]
async fn l_avis_d_absence_attend_se_complete_et_s_annule() {
    let dossier = tempfile::tempdir().unwrap();
    let e = ecole(dossier.path()).await;
    let (p, admin) = (&e.p, &e.admin);
    let releve = |lignes: Vec<(usize, &str, &str)>| {
        json!({ "classe_id": e.classe, "date": "2026-10-04", "lignes": lignes.into_iter()
            .map(|(i, matin, apres_midi)| json!({ "inscription_id": e.eleves[i].0, "matin": matin, "apres_midi": apres_midi }))
            .collect::<Vec<_>>() })
    };

    // Awa absente le matin : l'avis attend le délai de grâce de 30 minutes.
    p.ok(
        "absences_releve_enregistrer",
        Some(admin),
        releve(vec![(0, "a", ""), (1, "a", ""), (2, "a", "")]),
    )
    .await;
    assert_eq!(traiter(p).await.envoyes, 0);
    let avis = vers(p, admin, "+22376000001").await;
    assert_eq!(avis.len(), 1);
    assert_eq!(avis[0]["nature"], "absence");
    assert_eq!(avis[0]["etat"], "en_attente");
    assert!(
        avis[0]["texte"]
            .as_str()
            .unwrap()
            .contains("le 04/10/2026, matinee, sans justification")
    );

    // L'après-midi aussi : le même avis se complète. Binta n'était pas absente : la correction
    // annule son avis. Coumba est justifiée : le sien aussi.
    p.ok(
        "absences_releve_enregistrer",
        Some(admin),
        releve(vec![(0, "a", "a"), (1, "", ""), (2, "a", "")]),
    )
    .await;
    p.ok(
        "absences_justifier",
        Some(admin),
        json!({ "absences": [{ "inscription_id": e.eleves[2].0, "date": "2026-10-04", "demi_journee": "matin" }],
            "motif": "Maladie", "justificatif": "Mot des parents" }),
    )
    .await;
    let avis = vers(p, admin, "+22376000001").await;
    assert_eq!(avis.len(), 1, "un avis par jour");
    assert!(
        avis[0]["texte"]
            .as_str()
            .unwrap()
            .contains("matinee et apres-midi")
    );
    let binta = vers(p, admin, "+22376000002").await;
    assert_eq!(binta[0]["etat"], "annule");
    assert_eq!(vers(p, admin, "+22376000003").await[0]["etat"], "annule");

    // Le délai passé, l'avis d'Awa part.
    p.horloge.avancer(31 * 60_000);
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    assert_eq!(traiter(p).await.envoyes, 1);
    assert_eq!(vers(p, &admin, "+22376000001").await[0]["etat"], "remis");
}

#[tokio::test]
async fn la_relance_d_impaye_suit_ses_regles() {
    let dossier = tempfile::tempdir().unwrap();
    let e = ecole(dossier.path()).await;
    let (p, mut admin) = (&e.p, e.admin.clone());
    // La mère de Binta ne veut pas de relances.
    let mere_binta = e.eleves[1].1["responsables"][0]["responsable"]["id"].clone();
    p.ok(
        "responsable_consentement_definir",
        Some(&admin),
        json!({ "responsable_id": mere_binta, "type_message": "impaye", "accepte": false }),
    )
    .await;

    // Le jour de l'inscription, les frais d'inscription sont échus mais rien n'est relancé avant
    // sept jours.
    traiter(p).await;
    assert!(vers(p, &admin, "+22376000001").await.is_empty());

    // Sept jours plus tard : une relance pour Awa et Coumba, pas pour Binta.
    p.horloge.avancer(7 * JOUR);
    admin = p.connecter("admin", MOT_DE_PASSE).await;
    assert_eq!(traiter(p).await.envoyes, 2);
    let relance = vers(p, &admin, "+22376000001").await;
    assert_eq!(relance.len(), 1);
    assert_eq!(relance[0]["nature"], "impaye");
    let texte = relance[0]["texte"].as_str().unwrap();
    assert!(
        texte.contains(
            "il reste 10 000 F a payer pour Awa Traore (7e A), echu depuis le 04/10/2026"
        ),
        "{texte}"
    );
    assert!(vers(p, &admin, "+22376000002").await.is_empty());

    // Le même jour et le lendemain, rien de plus ; quinze jours après, la deuxième, avec la
    // 1re tranche échue le 15 octobre.
    traiter(p).await;
    p.horloge.avancer(JOUR);
    admin = p.connecter("admin", MOT_DE_PASSE).await;
    traiter(p).await;
    assert_eq!(vers(p, &admin, "+22376000001").await.len(), 1);
    p.horloge.avancer(14 * JOUR);
    admin = p.connecter("admin", MOT_DE_PASSE).await;
    traiter(p).await;
    let relances = vers(p, &admin, "+22376000001").await;
    assert_eq!(relances.len(), 2);
    assert!(
        relances[0]["texte"]
            .as_str()
            .unwrap()
            .contains("il reste 50 000 F")
    );

    // Hors des heures d'envoi, la troisième relance attend ; Awa paie tout entre-temps : la
    // relance tombe au moment de partir.
    regler(p, &admin, "sms.fenetre_envoi", json!("0-1")).await;
    p.horloge.avancer(15 * JOUR);
    admin = p.connecter("admin", MOT_DE_PASSE).await;
    traiter(p).await;
    let attente = vers(p, &admin, "+22376000001").await;
    assert_eq!(attente.len(), 3);
    assert_eq!(attente[0]["etat"], "en_attente");
    p.ok("caisse_ouvrir", Some(&admin), json!({ "fonds_initial": 0 }))
        .await;
    let mere_awa = e.eleves[0].1["responsables"][0]["responsable"]["id"].clone();
    let situation = p
        .ok(
            "situation_financiere",
            Some(&admin),
            json!({ "id": e.eleves[0].1["eleve"]["id"] }),
        )
        .await;
    let echu = situation[0]["impaye"]["echu"].as_i64().unwrap_or(50_000);
    p.ok(
        "encaisser",
        Some(&admin),
        json!({ "cle_idempotence": "awa-relance-1", "inscription_id": e.eleves[0].0, "montant": echu,
            "mode": "especes", "reference": "", "payeur_id": mere_awa, "payeur_nom": "", "repartition": null }),
    )
    .await;
    regler(p, &admin, "sms.fenetre_envoi", json!("0-24")).await;
    traiter(p).await;
    let tombee = vers(p, &admin, "+22376000001").await;
    assert_eq!(tombee[0]["etat"], "annule");
    assert_eq!(
        tombee[0]["motif_annulation"],
        "Échéance soldée avant l'envoi"
    );

    // Coumba en est à sa troisième : il n'y en aura pas de quatrième.
    assert_eq!(vers(p, &admin, "+22376000003").await.len(), 3);
    p.horloge.avancer(15 * JOUR);
    admin = p.connecter("admin", MOT_DE_PASSE).await;
    traiter(p).await;
    assert_eq!(vers(p, &admin, "+22376000003").await.len(), 3);
}

#[tokio::test]
async fn l_avis_de_sanction_se_prepare_et_attend_une_decision() {
    let dossier = tempfile::tempdir().unwrap();
    let e = ecole(dossier.path()).await;
    let (p, admin) = (&e.p, &e.admin);
    let sanction = json!({ "inscription_id": e.eleves[0].0, "nature": "avertissement", "date": "",
        "motif": "Bavardages répétés", "decidee_par": "Le directeur", "periode_id": null });

    // Sans le réglage, rien n'est préparé.
    p.ok("sanction_noter", Some(admin), sanction.clone()).await;
    assert!(vers(p, admin, "+22376000001").await.is_empty());

    // Avec lui, l'avis attend la décision d'une personne, et ne part pas seul.
    regler(p, admin, "sms.sanction_preparer", json!(true)).await;
    p.ok("sanction_noter", Some(admin), sanction).await;
    let avis = vers(p, admin, "+22376000001").await;
    assert_eq!(avis.len(), 1);
    assert_eq!(
        (avis[0]["nature"].as_str(), avis[0]["etat"].as_str()),
        (Some("sanction"), Some("a_decider"))
    );
    assert!(
        avis[0]["texte"]
            .as_str()
            .unwrap()
            .contains("a recu le 04/10/2026 : Avertissement")
    );
    assert_eq!(traiter(p).await.envoyes, 0);
}
