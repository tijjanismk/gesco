//! Sessions de caisse par la frontière (SPEC_PHASE_3 §6, assertion 7).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod commun;

use base64::Engine;
use commun::*;
use serde_json::{Value, json};

#[tokio::test]
async fn une_session_s_ouvre_se_cloture_avec_son_ecart_et_se_vise() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    let admin = p.connecter("admin", MOT_DE_PASSE).await;
    p.ok(
        "utilisateur_creer",
        Some(&admin),
        json!({ "identifiant": "oumou", "nom_affiche": "Oumou Diarra",
            "mot_de_passe_initial": "caisse-ecole-2026", "roles": ["caissier"] }),
    )
    .await;
    let caissiere = p.connecter("oumou", "caisse-ecole-2026").await;

    // Pas de session : rien de courant, la clôture est refusée.
    assert_eq!(
        p.ok("caisse_session_courante", Some(&caissiere), Value::Null)
            .await,
        Value::Null
    );
    assert_eq!(
        p.refus(
            "caisse_cloturer",
            Some(&caissiere),
            json!({ "total_compte": 0, "motif_ecart": "" })
        )
        .await
        .cle,
        "caisse.session_fermee"
    );

    // Ouverture avec le fonds ; une seule session ouverte à la fois.
    assert_eq!(
        p.refus(
            "caisse_ouvrir",
            Some(&caissiere),
            json!({ "fonds_initial": -1 })
        )
        .await
        .cle,
        "caisse.fonds_negatif"
    );
    let ouverte = p
        .ok(
            "caisse_ouvrir",
            Some(&caissiere),
            json!({ "fonds_initial": 25_000 }),
        )
        .await;
    assert_eq!(ouverte["session"]["statut"], "ouverte");
    assert_eq!(ouverte["caissier"], "Oumou Diarra");
    assert_eq!(ouverte["totaux"]["theorique"], 25_000);
    assert_eq!(
        p.refus(
            "caisse_ouvrir",
            Some(&caissiere),
            json!({ "fonds_initial": 0 })
        )
        .await
        .cle,
        "caisse.deja_ouverte"
    );
    assert_eq!(
        p.ok("caisse_session_courante", Some(&caissiere), Value::Null)
            .await["session"]["id"],
        ouverte["session"]["id"]
    );
    // Une autre personne a sa propre caisse.
    p.ok("caisse_ouvrir", Some(&admin), json!({ "fonds_initial": 0 }))
        .await;

    // Clôture : un écart sans motif est refusé ; avec un motif, il est enregistré.
    assert_eq!(
        p.refus(
            "caisse_cloturer",
            Some(&caissiere),
            json!({ "total_compte": 24_500, "motif_ecart": " " })
        )
        .await
        .cle,
        "caisse.motif_ecart_obligatoire"
    );
    let cloturee = p
        .ok(
            "caisse_cloturer",
            Some(&caissiere),
            json!({ "total_compte": 24_500, "motif_ecart": "Monnaie rendue en trop" }),
        )
        .await;
    assert_eq!(cloturee["session"]["statut"], "cloturee");
    assert_eq!(cloturee["session"]["total_theorique"], 25_000);
    assert_eq!(cloturee["session"]["ecart"], -500);
    // Clôturée, elle ne se rouvre pas : la caissière en ouvre une nouvelle.
    let nouvelle = p
        .ok(
            "caisse_ouvrir",
            Some(&caissiere),
            json!({ "fonds_initial": 24_500 }),
        )
        .await;
    assert_ne!(nouvelle["session"]["id"], cloturee["session"]["id"]);
    // Sans écart, le motif n'est pas exigé.
    p.ok(
        "caisse_cloturer",
        Some(&caissiere),
        json!({ "total_compte": 24_500, "motif_ecart": "" }),
    )
    .await;

    // La caissière ne vise pas et ne voit pas la liste ; elle imprime son propre état.
    let id = json!({ "id": cloturee["session"]["id"] });
    assert_eq!(
        p.refus(
            "caisse_rapprocher",
            Some(&caissiere),
            json!({ "id": cloturee["session"]["id"], "commentaire": "" })
        )
        .await
        .cle,
        "erreur.permission_refusee"
    );
    assert_eq!(
        p.refus("caisse_sessions_lister", Some(&caissiere), Value::Null)
            .await
            .cle,
        "erreur.permission_refusee"
    );
    let etat = p
        .ok("etat_caisse_produire", Some(&caissiere), id.clone())
        .await;
    let pdf = base64::engine::general_purpose::STANDARD
        .decode(etat["pdf"].as_str().unwrap())
        .unwrap();
    assert!(pdf.starts_with(b"%PDF-"));
    assert_eq!(etat["document"]["modele_id"], "etat_caisse");

    // La direction vise ; une session ouverte ne se vise pas.
    let admin_session = p
        .ok("caisse_session_courante", Some(&admin), Value::Null)
        .await;
    assert_eq!(
        p.refus(
            "caisse_rapprocher",
            Some(&admin),
            json!({ "id": admin_session["session"]["id"], "commentaire": "" })
        )
        .await
        .cle,
        "erreur.transition_interdite"
    );
    let visee = p
        .ok(
            "caisse_rapprocher",
            Some(&admin),
            json!({ "id": cloturee["session"]["id"], "commentaire": "Écart justifié" }),
        )
        .await;
    assert_eq!(visee["session"]["statut"], "rapprochee");
    assert_eq!(visee["vise_par"], "Awa Traoré");
    let sessions = p
        .ok("caisse_sessions_lister", Some(&admin), Value::Null)
        .await;
    assert_eq!(sessions.as_array().unwrap().len(), 3);
    // La caissière n'imprime pas l'état d'une autre caisse.
    assert_eq!(
        p.refus(
            "etat_caisse_produire",
            Some(&caissiere),
            json!({ "id": admin_session["session"]["id"] })
        )
        .await
        .cle,
        "erreur.permission_refusee"
    );
}
