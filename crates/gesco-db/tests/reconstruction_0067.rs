//! Migration 0067 (phase 12, lot 3) : la reconstruction de `contenu` pour la nature `quiz` garde
//! ses lignes, ses index, son déclencheur et la recherche (critère 11 du §0.3 de
//! `SPEC_PHASE_12_LOT_3.md`) ; les tentatives ne se suppriment pas.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use gesco_core::commun::{Horodatage, Id};
use gesco_core::core::{Etablissement, Organisation, Poste, StatutEtablissement};
use gesco_db::base::MIGRATIONS;
use gesco_db::depots::etablissement;
use gesco_db::unite::Durabilite;
use gesco_db::{Base, Contexte, OptionsBase, UniteDeTravail, nouvel_id};

fn contexte(etablissement_id: Id<Etablissement>) -> Contexte {
    Contexte {
        etablissement_id,
        utilisateur_id: None,
        role_actif: Some("administrateur".to_owned()),
        poste: Poste("poste-test".to_owned()),
        empreinte_machine: "machine-test".to_owned(),
        maintenant: Horodatage::depuis_millis_epoque(1_791_101_730_250),
        correlation_id: nouvel_id::<()>().vers_chaine(),
    }
}

async fn schema(base: &Base) -> Vec<(String, String, String)> {
    sqlx::query_as(
        "SELECT type, name, COALESCE(sql, '') FROM sqlite_master
         WHERE tbl_name = 'contenu' AND type IN ('index', 'trigger')
         ORDER BY type, name",
    )
    .fetch_all(base.pool())
    .await
    .unwrap()
}

async fn lignes(base: &Base) -> Vec<(String, String, String, String, i64)> {
    sqlx::query_as("SELECT id, cle, nature, titre, ordre FROM contenu ORDER BY id")
        .fetch_all(base.pool())
        .await
        .unwrap()
}

async fn chercher(base: &Base, mot: &str) -> Vec<String> {
    sqlx::query_scalar(
        "SELECT contenu_id FROM recherche_bibliotheque WHERE recherche_bibliotheque MATCH ?
         ORDER BY contenu_id",
    )
    .bind(mot)
    .fetch_all(base.pool())
    .await
    .unwrap()
}

async fn inserer_contenu(base: &Base, e: &str, chapitre: &str, cle: &str, nature: &str) -> bool {
    sqlx::query(
        "INSERT INTO contenu (id, etablissement_id, cle, origine, nature, chapitre_id, titre,
            ordre, cree_le, mis_a_jour_le, identifiant_poste)
         VALUES (?, ?, ?, 'ecole', ?, ?, 'Titre', 9, 1, 1, 'poste')",
    )
    .bind(nouvel_id::<()>().vers_chaine())
    .bind(e)
    .bind(cle)
    .bind(nature)
    .bind(chapitre)
    .execute(base.pool())
    .await
    .is_ok()
}

#[tokio::test]
async fn reconstruction_de_contenu_garde_tout_et_accepte_les_quiz() {
    let dossier = tempfile::tempdir().unwrap();
    let base = Base::ouvrir_sans_migrer(OptionsBase::fichier(dossier.path().join("gesco.db")))
        .await
        .unwrap();
    MIGRATIONS.run_to(66, base.pool_critique()).await.unwrap();
    assert_eq!(base.version_schema().await.unwrap(), 66);

    // Une école en 2.0 (lot 1) : un établissement, un chapitre livré, un document de l'école
    // fait de deux fichiers, et une image appelée par une leçon.
    let organisation = Organisation {
        id: nouvel_id(),
        nom: "Groupe".to_owned(),
    };
    let etab: Id<Etablissement> = nouvel_id();
    let ctx = contexte(etab);
    let mut unite = UniteDeTravail::<()>::commencer(&base, ctx.clone(), Durabilite::Critique)
        .await
        .unwrap();
    etablissement::inserer_organisation(unite.connexion(), &organisation, &ctx)
        .await
        .unwrap();
    etablissement::inserer(
        unite.connexion(),
        &Etablissement {
            id: etab,
            organisation_id: organisation.id,
            nom: "École A".to_owned(),
            statut: StatutEtablissement::Prive,
            code_academie: "AE-BKO-RD".to_owned(),
            code_cap: "CAP-01".to_owned(),
            adresse: "Bamako".to_owned(),
            telephones: vec![],
            mentions_legales: String::new(),
            multiple_arrondi: 5,
        },
        &ctx,
    )
    .await
    .unwrap();
    unite.valider().await.unwrap();

    let e = etab.vers_chaine();
    let chapitre = nouvel_id::<()>().vers_chaine();
    sqlx::query(
        "INSERT INTO chapitre_programme (id, etablissement_id, cle, origine, paquet_code,
            niveau_code, discipline_code, titre, ordre, cree_le, mis_a_jour_le,
            identifiant_poste)
         VALUES (?, ?, 'livre/c1', 'livre', 'p', '9', 'MATH', 'Chapitre', 1, 1, 1, 'poste')",
    )
    .bind(&chapitre)
    .bind(&e)
    .execute(base.pool())
    .await
    .unwrap();
    for (i, (cle, origine, nature, titre)) in [
        ("livre/l1", "livre", "lecon", "La racine carrée"),
        ("ecole/d1", "ecole", "document", "Fiche de révision"),
    ]
    .iter()
    .enumerate()
    {
        let id = nouvel_id::<()>().vers_chaine();
        let livre = *origine == "livre";
        sqlx::query(
            "INSERT INTO contenu (id, etablissement_id, cle, origine, nature, chapitre_id, titre,
                ordre, paquet_code, fichier, empreinte_sha256, cree_le, mis_a_jour_le,
                identifiant_poste)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, 1, 'poste')",
        )
        .bind(&id)
        .bind(&e)
        .bind(cle)
        .bind(origine)
        .bind(nature)
        .bind(&chapitre)
        .bind(titre)
        .bind(i as i64)
        .bind(livre.then_some("p"))
        .bind(livre.then_some("l1.md"))
        .bind(livre.then(|| "a".repeat(64)))
        .execute(base.pool())
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO recherche_bibliotheque (contenu_id, titre, texte) VALUES (?, ?, ?)",
        )
        .bind(&id)
        .bind(titre)
        .bind("Le piège de la somme : la racine carrée d'une somme.")
        .execute(base.pool())
        .await
        .unwrap();
    }

    let schema_avant = schema(&base).await;
    let lignes_avant = lignes(&base).await;
    let trouves_avant = chercher(&base, "piege").await;
    assert_eq!(trouves_avant.len(), 2);

    // Jusqu'à la 0067 seulement : les migrations suivantes ont leurs propres essais.
    MIGRATIONS.run_to(67, base.pool_critique()).await.unwrap();
    assert_eq!(base.version_schema().await.unwrap(), 67);

    assert_eq!(
        schema(&base).await,
        schema_avant,
        "index et déclencheur refaits à l'identique"
    );
    assert_eq!(
        lignes(&base).await,
        lignes_avant,
        "toutes les lignes gardées"
    );
    assert_eq!(
        chercher(&base, "piege").await,
        trouves_avant,
        "la recherche trouve toujours"
    );
    let violations: Vec<(String, i64, String, i64)> = sqlx::query_as("PRAGMA foreign_key_check")
        .fetch_all(base.pool())
        .await
        .unwrap();
    assert!(violations.is_empty(), "{violations:?}");

    // Rien ne se supprime ; un quiz de chapitre est accepté ; une nature inconnue non ; l'unicité
    // par clé tient toujours.
    assert!(
        sqlx::query("DELETE FROM contenu")
            .execute(base.pool())
            .await
            .is_err()
    );
    assert!(inserer_contenu(&base, &e, &chapitre, "ecole/q1", "quiz").await);
    assert!(!inserer_contenu(&base, &e, &chapitre, "ecole/x", "jeu").await);
    assert!(!inserer_contenu(&base, &e, &chapitre, "ecole/q1", "quiz").await);

    // Les tables des tentatives existent, vides, avec leur déclencheur.
    for (table, compte) in [
        ("tentative_quiz", "SELECT count(*) FROM tentative_quiz"),
        ("reponse_quiz", "SELECT count(*) FROM reponse_quiz"),
    ] {
        let n: i64 = sqlx::query_scalar(compte)
            .fetch_one(base.pool())
            .await
            .unwrap();
        assert_eq!(n, 0);
        let declencheur: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM sqlite_master WHERE type = 'trigger' AND tbl_name = ?",
        )
        .bind(table)
        .fetch_one(base.pool())
        .await
        .unwrap();
        assert_eq!(declencheur, 1, "{table}");
    }
}
