//! Journal technique du poste : une commande refusée et une panne signalée par l'interface y
//! laissent chacune une ligne, sans les données saisies.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod commun;

use commun::*;
use serde_json::json;

#[tokio::test]
async fn les_erreurs_se_notent_au_journal_technique() {
    let dossier = tempfile::tempdir().unwrap();
    let p = poste_vierge(dossier.path()).await;
    p.installer().await;
    // Une connexion refusée : la commande et la clé de l'erreur, jamais le mot de passe.
    p.refus(
        "connexion",
        None,
        json!({ "identifiant": "admin", "mot_de_passe": "mauvais-mot-de-passe" }),
    )
    .await;
    // Une panne de l'interface, notée avant toute connexion.
    p.ok(
        "journal_technique_noter",
        None,
        json!({ "message": "TypeError: x is undefined" }),
    )
    .await;
    let journal =
        std::fs::read_to_string(p.racine.join(gesco_services::journal_technique::FICHIER)).unwrap();
    // Le lancement et chaque étape du premier démarrage, avec leur durée.
    assert!(journal.contains("demarrage version="), "{journal}");
    for etape in [
        "debut",
        "etablissement_cree",
        "certificat_compose",
        "termine",
    ] {
        assert!(
            journal.contains(&format!("premier_demarrage {etape} apres=")),
            "{journal}"
        );
    }
    assert!(journal.contains("commande=connexion cle="), "{journal}");
    assert!(!journal.contains("mauvais-mot-de-passe"), "{journal}");
    assert!(
        journal.contains("interface TypeError: x is undefined"),
        "{journal}"
    );
}
