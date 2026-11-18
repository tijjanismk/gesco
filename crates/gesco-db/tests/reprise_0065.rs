//! Migration 0065 (reprise avant GESCO, essai 8 du §11) : la base refuse la suppression d'une
//! reprise et de ses parts, tient une seule reprise active par inscription, refuse une reprise
//! annulée sans motif et une note trop longue ; la descente rend le schéma d'avant.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use gesco_db::base::MIGRATIONS;
use gesco_db::{Base, OptionsBase};

const ID: [&str; 6] = [
    "01HZZZZZZZZZZZZZZZZZZZZZZ1",
    "01HZZZZZZZZZZZZZZZZZZZZZZ2",
    "01HZZZZZZZZZZZZZZZZZZZZZZ3",
    "01HZZZZZZZZZZZZZZZZZZZZZZ4",
    "01HZZZZZZZZZZZZZZZZZZZZZZ5",
    "01HZZZZZZZZZZZZZZZZZZZZZZ6",
];

#[tokio::test]
async fn une_reprise_ne_se_supprime_pas_et_reste_seule_active() {
    let dossier = tempfile::tempdir().unwrap();
    let base = Base::ouvrir(OptionsBase::fichier(dossier.path().join("gesco.db")))
        .await
        .unwrap();
    assert!(base.version_schema().await.unwrap() >= 65);
    let mut cnx = base.pool_critique().acquire().await.unwrap();
    // Les lignes parentes ne sont pas l'objet de l'essai : clés étrangères suspendues.
    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(&mut *cnx)
        .await
        .unwrap();
    let reprise = |id: &str, statut: &str, motif: &str, note: &str| {
        format!(
            "INSERT INTO reprise_avant_gesco (id, etablissement_id, inscription_id, montant, note,
                origine, statut, motif_annulation, cree_le, mis_a_jour_le, identifiant_poste)
             VALUES ('{id}', '{e}', '{i}', 75000, '{note}', 'ecran', '{statut}', '{motif}', 1, 1, 'p')",
            e = ID[0],
            i = ID[1]
        )
    };
    sqlx::query(sqlx::AssertSqlSafe(reprise(
        ID[2],
        "active",
        "",
        "cahier p. 12",
    )))
    .execute(&mut *cnx)
    .await
    .unwrap();
    // Une seconde reprise active pour la même inscription : refusée par l'index unique.
    assert!(
        sqlx::query(sqlx::AssertSqlSafe(reprise(ID[3], "active", "", "")))
            .execute(&mut *cnx)
            .await
            .is_err()
    );
    // Annulée sans motif : refusée ; avec motif : acceptée.
    assert!(
        sqlx::query(sqlx::AssertSqlSafe(reprise(ID[3], "annulee", "", "")))
            .execute(&mut *cnx)
            .await
            .is_err()
    );
    sqlx::query(sqlx::AssertSqlSafe(reprise(
        ID[3],
        "annulee",
        "Saisie erronée",
        "",
    )))
    .execute(&mut *cnx)
    .await
    .unwrap();
    // Note de plus de 120 caractères : refusée.
    assert!(
        sqlx::query(sqlx::AssertSqlSafe(reprise(
            ID[4],
            "annulee",
            "remplacée",
            &"x".repeat(121)
        )))
        .execute(&mut *cnx)
        .await
        .is_err()
    );
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "INSERT INTO reprise_imputation (id, etablissement_id, reprise_id, echeance_id, montant,
            statut, cree_le, mis_a_jour_le, identifiant_poste)
         VALUES ('{}', '{}', '{}', '{}', 25000, 'active', 1, 1, 'p')",
        ID[5], ID[0], ID[2], ID[4]
    )))
    .execute(&mut *cnx)
    .await
    .unwrap();
    for table in ["reprise_imputation", "reprise_avant_gesco"] {
        let refus = sqlx::query(sqlx::AssertSqlSafe(format!("DELETE FROM {table}")))
            .execute(&mut *cnx)
            .await
            .unwrap_err();
        assert!(refus.to_string().contains("ne se supprime pas"), "{table}");
    }
    // La dépense gagne son lien facultatif à l'inscription.
    let colonnes: Vec<String> = sqlx::query_scalar("SELECT name FROM pragma_table_info('depense')")
        .fetch_all(&mut *cnx)
        .await
        .unwrap();
    assert!(colonnes.iter().any(|c| c == "inscription_id"));
    drop(cnx);
}

#[tokio::test]
async fn la_descente_de_la_0065_rend_le_schema_d_avant() {
    let dossier = tempfile::tempdir().unwrap();
    let base = Base::ouvrir_sans_migrer(OptionsBase::fichier(dossier.path().join("gesco.db")))
        .await
        .unwrap();
    MIGRATIONS.run_to(65, base.pool_critique()).await.unwrap();
    MIGRATIONS.undo(base.pool_critique(), 64).await.unwrap();
    assert_eq!(base.version_schema().await.unwrap(), 64);
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE name LIKE 'reprise%' OR name = 'depense_inscription'",
    )
    .fetch_all(base.pool())
    .await
    .unwrap();
    assert!(tables.is_empty(), "{tables:?}");
}
