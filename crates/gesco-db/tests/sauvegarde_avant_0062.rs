//! La sauvegarde qui précède les migrations s'écrit dans la base encore ancienne : une école de
//! la 1.2, ou de la 2.0 du lot 1 (base à la migration 0061), qui passe à une version plus récente
//! ne doit pas s'arrêter sur la colonne `sorte`, arrivée avec la migration 0062.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use gesco_core::commun::{Horodatage, Id};
use gesco_core::core::{Etablissement, Organisation, Poste, SorteDestination, StatutEtablissement};
use gesco_db::base::MIGRATIONS;
use gesco_db::depots::{etablissement, sauvegarde};
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

#[tokio::test]
async fn la_sauvegarde_d_avant_migration_s_ecrit_dans_une_base_a_la_0061() {
    let dossier = tempfile::tempdir().unwrap();
    let base = Base::ouvrir_sans_migrer(OptionsBase::fichier(dossier.path().join("gesco.db")))
        .await
        .unwrap();
    MIGRATIONS.run_to(61, base.pool_critique()).await.unwrap();

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
    let avant = sauvegarde::Sauvegarde {
        id: nouvel_id(),
        etablissement_id: etab,
        horodatage: ctx.maintenant,
        destination: "sauvegardes".to_owned(),
        taille: 1_024,
        empreinte_sha256: "a".repeat(64),
        version_schema: 61,
        version_applicative: "2.0.0".to_owned(),
        resultat: sauvegarde::ResultatSauvegarde::Reussie,
        dernier_test_restauration: None,
        sorte: SorteDestination::Dossier,
        etat_envoi: None,
        etat_verifie_le: None,
    };
    sauvegarde::inserer(unite.connexion(), &avant, &ctx)
        .await
        .unwrap();
    unite.valider().await.unwrap();

    // Puis toutes les migrations : la ligne prend la sorte « dossier » et se relit entière.
    MIGRATIONS.run(base.pool_critique()).await.unwrap();
    let mut cnx = base.pool().acquire().await.unwrap();
    let lues = sauvegarde::lister(&mut cnx, 10, &ctx).await.unwrap();
    assert_eq!(lues, vec![avant]);
}
