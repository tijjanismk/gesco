//! Migration 0063 (phase 12, lot 2) : la reconstruction de `fichier_bibliotheque` garde ses
//! lignes, son index unique, son déclencheur et les liens de `fichier_contenu` (critère 7 du
//! §0.3 de `SPEC_PHASE_12_LOT_2.md`).

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
         WHERE tbl_name IN ('fichier_bibliotheque', 'fichier_contenu')
           AND type IN ('index', 'trigger')
         ORDER BY type, name",
    )
    .fetch_all(base.pool())
    .await
    .unwrap()
}

async fn lignes(base: &Base) -> Vec<(String, String, String, i64, Vec<u8>)> {
    sqlx::query_as(
        "SELECT id, nature, type_mime, taille, contenu FROM fichier_bibliotheque ORDER BY id",
    )
    .fetch_all(base.pool())
    .await
    .unwrap()
}

#[tokio::test]
async fn reconstruction_de_fichier_bibliotheque_garde_tout() {
    let dossier = tempfile::tempdir().unwrap();
    let base = Base::ouvrir_sans_migrer(OptionsBase::fichier(dossier.path().join("gesco.db")))
        .await
        .unwrap();
    MIGRATIONS.run_to(62, base.pool_critique()).await.unwrap();
    assert_eq!(base.version_schema().await.unwrap(), 62);

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
    let contenu = nouvel_id::<()>().vers_chaine();
    let mut fichiers = Vec::new();
    for (i, (nature, mime)) in [
        ("image", "image/png"),
        ("document", "application/pdf"),
        ("document", "image/jpeg"),
    ]
    .iter()
    .enumerate()
    {
        let id = nouvel_id::<()>().vers_chaine();
        sqlx::query(
            "INSERT INTO fichier_bibliotheque (id, etablissement_id, empreinte_sha256, nature,
                type_mime, nom_fichier, taille, contenu, cree_le, mis_a_jour_le,
                identifiant_poste)
             VALUES (?, ?, ?, ?, ?, ?, 3, ?, 1, 1, 'poste')",
        )
        .bind(&id)
        .bind(&e)
        .bind(format!("{i}").repeat(64))
        .bind(nature)
        .bind(mime)
        .bind(format!("f{i}"))
        .bind(vec![i as u8, 2, 3])
        .execute(base.pool())
        .await
        .unwrap();
        fichiers.push(id);
    }
    sqlx::query(
        "INSERT INTO chapitre_programme (id, etablissement_id, cle, origine, paquet_code,
            niveau_code, discipline_code, titre, ordre, cree_le, mis_a_jour_le,
            identifiant_poste)
         VALUES (?, ?, 'livre/c1', 'livre', 'p', '9', 'SVT', 'Chapitre', 1, 1, 1, 'poste')",
    )
    .bind(&chapitre)
    .bind(&e)
    .execute(base.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO contenu (id, etablissement_id, cle, origine, nature, chapitre_id, titre,
            ordre, paquet_code, fichier, empreinte_sha256, cree_le, mis_a_jour_le,
            identifiant_poste)
         VALUES (?, ?, 'livre/d1', 'livre', 'document', ?, 'Doc', 1, 'p', 'd.md', ?, 1, 1,
            'poste')",
    )
    .bind(&contenu)
    .bind(&e)
    .bind(&chapitre)
    .bind("a".repeat(64))
    .execute(base.pool())
    .await
    .unwrap();
    for (ordre, fichier) in fichiers.iter().skip(1).enumerate() {
        sqlx::query(
            "INSERT INTO fichier_contenu (id, etablissement_id, contenu_id, fichier_id, ordre,
                cree_le, mis_a_jour_le, identifiant_poste)
             VALUES (?, ?, ?, ?, ?, 1, 1, 'poste')",
        )
        .bind(nouvel_id::<()>().vers_chaine())
        .bind(&e)
        .bind(&contenu)
        .bind(fichier)
        .bind(ordre as i64)
        .execute(base.pool())
        .await
        .unwrap();
    }

    let schema_avant = schema(&base).await;
    let lignes_avant = lignes(&base).await;
    let liens_avant: i64 = sqlx::query_scalar("SELECT count(*) FROM fichier_contenu")
        .fetch_one(base.pool())
        .await
        .unwrap();

    base.migrer().await.unwrap();
    assert_eq!(base.version_schema().await.unwrap(), 63);

    assert_eq!(
        schema(&base).await,
        schema_avant,
        "index et déclencheurs refaits à l'identique"
    );
    assert_eq!(
        lignes(&base).await,
        lignes_avant,
        "toutes les lignes gardées"
    );
    let liens_apres: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM fichier_contenu fc JOIN fichier_bibliotheque f ON f.id = fc.fichier_id",
    )
    .fetch_one(base.pool())
    .await
    .unwrap();
    assert_eq!(liens_apres, liens_avant);
    let violations: Vec<(String, i64, String, i64)> = sqlx::query_as("PRAGMA foreign_key_check")
        .fetch_all(base.pool())
        .await
        .unwrap();
    assert!(violations.is_empty(), "{violations:?}");

    // Toujours rien ne se supprime ; un son est accepté.
    assert!(
        sqlx::query("DELETE FROM fichier_bibliotheque")
            .execute(base.pool())
            .await
            .is_err()
    );
    sqlx::query(
        "INSERT INTO fichier_bibliotheque (id, etablissement_id, empreinte_sha256, nature,
            type_mime, nom_fichier, taille, contenu, cree_le, mis_a_jour_le, identifiant_poste)
         VALUES (?, ?, ?, 'son', 'audio/ogg', 'voix.ogg', 3, x'010203', 1, 1, 'poste')",
    )
    .bind(nouvel_id::<()>().vers_chaine())
    .bind(&e)
    .bind("f".repeat(64))
    .execute(base.pool())
    .await
    .unwrap();
    // L'unicité par empreinte tient toujours.
    assert!(
        sqlx::query(
            "INSERT INTO fichier_bibliotheque (id, etablissement_id, empreinte_sha256, nature,
                type_mime, nom_fichier, taille, contenu, cree_le, mis_a_jour_le,
                identifiant_poste)
             VALUES (?, ?, ?, 'son', 'audio/ogg', 'voix.ogg', 3, x'010203', 1, 1, 'poste')",
        )
        .bind(nouvel_id::<()>().vers_chaine())
        .bind(&e)
        .bind("f".repeat(64))
        .execute(base.pool())
        .await
        .is_err()
    );
}
