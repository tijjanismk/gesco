//! Migration 0068 (phase 12, lot 5) : les tables des devoirs à rendre, et les colonnes `origine`
//! et `devoir_id` de `evaluation` (critère 12 du §0.3 de `SPEC_PHASE_12_LOT_5.md`) : une base
//! menée à la 0067 garde ses évaluations, toutes marquées « saisie » ; rien ne se supprime ; le
//! contenu d'une photo ne se vide qu'avec `libere_le`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use gesco_db::base::MIGRATIONS;
use gesco_db::{Base, OptionsBase, nouvel_id};

fn id() -> String {
    nouvel_id::<()>().vers_chaine()
}

#[tokio::test]
async fn les_devoirs_s_ajoutent_et_les_evaluations_restent_saisies() {
    let dossier = tempfile::tempdir().unwrap();
    let base = Base::ouvrir_sans_migrer(OptionsBase::fichier(dossier.path().join("gesco.db")))
        .await
        .unwrap();
    MIGRATIONS.run_to(67, base.pool_critique()).await.unwrap();

    // Sur une seule connexion, sans les clés étrangères : on ne pose que les lignes utiles.
    let mut cnx = base.pool_critique().acquire().await.unwrap();
    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(&mut *cnx)
        .await
        .unwrap();
    let evaluation = id();
    sqlx::query(
        "INSERT INTO evaluation (id, etablissement_id, annee_id, periode_id, classe_id,
            matiere_id, type_evaluation, libelle, date, bareme, ponderation, statut, cree_le,
            mis_a_jour_le, identifiant_poste)
         VALUES (?, 'e', 'a', 'p', 'c', 'm', 'devoir', 'Devoir 1', '2026-11-20', 2000, 1,
            'validee', 1, 1, 'poste')",
    )
    .bind(&evaluation)
    .execute(&mut *cnx)
    .await
    .unwrap();
    drop(cnx);

    MIGRATIONS.run(base.pool_critique()).await.unwrap();
    assert!(base.version_schema().await.unwrap() >= 68);

    let colonnes: Vec<(String, i64, Option<String>)> = sqlx::query_as(
        "SELECT name, \"notnull\", dflt_value FROM pragma_table_info('evaluation')
         WHERE name IN ('origine', 'devoir_id') ORDER BY name",
    )
    .fetch_all(base.pool())
    .await
    .unwrap();
    assert_eq!(
        colonnes,
        vec![
            ("devoir_id".to_owned(), 0, None),
            ("origine".to_owned(), 1, Some("'saisie'".to_owned())),
        ]
    );
    let (origine, statut): (String, String) =
        sqlx::query_as("SELECT origine, statut FROM evaluation WHERE id = ?")
            .bind(&evaluation)
            .fetch_one(base.pool())
            .await
            .unwrap();
    assert_eq!((origine.as_str(), statut.as_str()), ("saisie", "validee"));

    let mut cnx = base.pool_critique().acquire().await.unwrap();
    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(&mut *cnx)
        .await
        .unwrap();
    let devoir = id();
    sqlx::query(
        "INSERT INTO devoir (id, etablissement_id, annee_id, classe_id, matiere_id, auteur_id,
            titre, date_limite, etat, donne_le, cree_le, mis_a_jour_le, identifiant_poste)
         VALUES (?, 'e', 'a', 'c', 'm', 'u', 'Racines carrées', 10, 'donne', 5, 1, 1, 'poste')",
    )
    .bind(&devoir)
    .execute(&mut *cnx)
    .await
    .unwrap();
    // Un devoir donné sans date de don, ou clos sans date, est refusé.
    let cas: [(&str, Option<i64>, Option<i64>); 2] =
        [("donne", None, None), ("clos", Some(1), None)];
    for (etat, donne, clos) in cas {
        assert!(
            sqlx::query(
                "INSERT INTO devoir (id, etablissement_id, annee_id, classe_id, matiere_id,
                    auteur_id, titre, date_limite, etat, donne_le, clos_le, cree_le,
                    mis_a_jour_le, identifiant_poste)
                 VALUES (?, 'e', 'a', 'c', 'm', 'u', 'X', 10, ?, ?, ?, 1, 1, 'poste')",
            )
            .bind(id())
            .bind(etat)
            .bind(donne)
            .bind(clos)
            .execute(&mut *cnx)
            .await
            .is_err(),
            "{etat} incohérent"
        );
    }
    let rendu = id();
    sqlx::query(
        "INSERT INTO rendu_devoir (id, etablissement_id, devoir_id, eleve_id, numero, origine,
            texte, rendu_le, identifiant_poste)
         VALUES (?, 'e', ?, 'el', 1, 'portail', 'Ma réponse', 7, 'poste')",
    )
    .bind(&rendu)
    .bind(&devoir)
    .execute(&mut *cnx)
    .await
    .unwrap();
    // Même numéro pour le même élève : refusé.
    assert!(
        sqlx::query(
            "INSERT INTO rendu_devoir (id, etablissement_id, devoir_id, eleve_id, numero,
                origine, rendu_le, identifiant_poste)
             VALUES (?, 'e', ?, 'el', 1, 'papier', 8, 'poste')",
        )
        .bind(id())
        .bind(&devoir)
        .execute(&mut *cnx)
        .await
        .is_err()
    );
    let empreinte = "a".repeat(64);
    sqlx::query(
        "INSERT INTO fichier_rendu (empreinte, etablissement_id, contenu, taille, sorte, cree_le)
         VALUES (?, 'e', x'FFD8FFD9', 4, 'jpeg', 1)",
    )
    .bind(&empreinte)
    .execute(&mut *cnx)
    .await
    .unwrap();
    sqlx::query("INSERT INTO photo_rendu (id, rendu_id, ordre, empreinte) VALUES (?, ?, 1, ?)")
        .bind(id())
        .bind(&rendu)
        .bind(&empreinte)
        .execute(&mut *cnx)
        .await
        .unwrap();
    // Une sixième photo est refusée par la table.
    assert!(
        sqlx::query("INSERT INTO photo_rendu (id, rendu_id, ordre, empreinte) VALUES (?, ?, 6, ?)")
            .bind(id())
            .bind(&rendu)
            .bind(&empreinte)
            .execute(&mut *cnx)
            .await
            .is_err()
    );
    // Vider le contenu sans `libere_le` : refusé ; avec : accepté.
    assert!(
        sqlx::query("UPDATE fichier_rendu SET contenu = NULL WHERE empreinte = ?")
            .bind(&empreinte)
            .execute(&mut *cnx)
            .await
            .is_err()
    );
    sqlx::query("UPDATE fichier_rendu SET contenu = NULL, libere_le = 9 WHERE empreinte = ?")
        .bind(&empreinte)
        .execute(&mut *cnx)
        .await
        .unwrap();
    // Un devoir, une évaluation.
    sqlx::query("UPDATE evaluation SET origine = 'devoir_en_ligne', devoir_id = ? WHERE id = ?")
        .bind(&devoir)
        .bind(&evaluation)
        .execute(&mut *cnx)
        .await
        .unwrap();
    assert!(
        sqlx::query(
            "INSERT INTO evaluation (id, etablissement_id, annee_id, periode_id, classe_id,
                matiere_id, type_evaluation, libelle, date, bareme, ponderation, statut,
                cree_le, mis_a_jour_le, identifiant_poste, origine, devoir_id)
             VALUES (?, 'e', 'a', 'p', 'c', 'm', 'devoir', 'Bis', '2026-11-20', 2000, 1,
                'en_saisie', 1, 1, 'poste', 'devoir_en_ligne', ?)",
        )
        .bind(id())
        .bind(&devoir)
        .execute(&mut *cnx)
        .await
        .is_err()
    );
    assert!(
        sqlx::query("UPDATE evaluation SET origine = 'ailleurs' WHERE id = ?")
            .bind(&evaluation)
            .execute(&mut *cnx)
            .await
            .is_err()
    );
    // Rien ne se supprime.
    for requete in [
        "DELETE FROM photo_rendu",
        "DELETE FROM fichier_rendu",
        "DELETE FROM rendu_devoir",
        "DELETE FROM devoir",
    ] {
        assert!(
            sqlx::query(requete).execute(&mut *cnx).await.is_err(),
            "{requete} est refusé"
        );
    }
}
