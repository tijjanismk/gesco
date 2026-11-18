//! Migration 0066 (prise en charge par l'État, essai 10 du §12, partie base) : chaque geste
//! laisse la version ancienne `remplacee` ; une seule version courante par inscription ; motif
//! exigé pour une version terminée et pour un abandon ; un versement de l'État a une recette
//! exactement quand il n'est pas reçu avant GESCO ; la base refuse toute suppression sur les
//! quatre tables ; la descente rend le schéma d'avant.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use gesco_core::commun::{DateCivile, Horodatage, Id, Montant};
use gesco_core::core::{Etablissement, Poste};
use gesco_core::finance::prise_en_charge::{
    MontantEtat, OriginePartEtat, PriseEnChargeEtat, StatutPriseEnCharge, StatutVersementEtat,
    VersementEtat,
};
use gesco_core::scolarite::Inscription;
use gesco_db::base::MIGRATIONS;
use gesco_db::depots::prise_en_charge as depot;
use gesco_db::{Base, Contexte, Connexion, OptionsBase, nouvel_id};

fn contexte(etablissement_id: Id<Etablissement>) -> Contexte {
    Contexte {
        etablissement_id,
        utilisateur_id: None,
        role_actif: Some("directeur".to_owned()),
        poste: Poste("poste-test".to_owned()),
        empreinte_machine: "machine-test".to_owned(),
        maintenant: Horodatage::depuis_millis_epoque(1_794_000_000_000),
        correlation_id: nouvel_id::<()>().vers_chaine(),
    }
}

fn version(
    etablissement_id: Id<Etablissement>,
    inscription_id: Id<Inscription>,
    statut: StatutPriseEnCharge,
    attendu: i64,
    abandonne: i64,
    motif: &str,
) -> PriseEnChargeEtat {
    PriseEnChargeEtat {
        id: nouvel_id(),
        etablissement_id,
        inscription_id,
        statut,
        montant_attendu: Montant::depuis_francs(attendu),
        abandonne: Montant::depuis_francs(abandonne),
        reference: "DEF-0001".to_owned(),
        motif: motif.to_owned(),
        remplace_id: None,
        cree_le: Horodatage::depuis_millis_epoque(0),
        cree_par: None,
        mis_a_jour_le: Horodatage::depuis_millis_epoque(0),
        mis_a_jour_par: None,
    }
}

/// Insère une version brute, sans passer par le dépôt : rend vrai si la base l'accepte.
async fn version_brute(
    cnx: &mut Connexion,
    etablissement: &str,
    inscription: &str,
    statut: &str,
    attendu: i64,
    abandonne: i64,
    motif: &str,
) -> bool {
    sqlx::query(
        "INSERT INTO prise_en_charge_etat (id, etablissement_id, inscription_id, statut,
            montant_attendu, abandonne, motif, cree_le, mis_a_jour_le, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, 1, 'p')",
    )
    .bind(nouvel_id::<()>().vers_chaine())
    .bind(etablissement)
    .bind(inscription)
    .bind(statut)
    .bind(attendu)
    .bind(abandonne)
    .bind(motif)
    .execute(cnx)
    .await
    .is_ok()
}

#[tokio::test]
async fn essai_10_trace_de_la_prise_en_charge_rien_ne_se_supprime() {
    let dossier = tempfile::tempdir().unwrap();
    let base = Base::ouvrir(OptionsBase::fichier(dossier.path().join("gesco.db")))
        .await
        .unwrap();
    assert!(base.version_schema().await.unwrap() >= 66);
    let mut cnx = base.pool_critique().acquire().await.unwrap();
    // Les lignes parentes ne sont pas l'objet de l'essai : clés étrangères suspendues.
    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(&mut *cnx)
        .await
        .unwrap();
    let etab: Id<Etablissement> = nouvel_id();
    let ctx = contexte(etab);
    let inscription: Id<Inscription> = nouvel_id();

    // Cocher, puis décocher : la première version passe `remplacee`, la seconde la désigne.
    let cochee = version(
        etab,
        inscription,
        StatutPriseEnCharge::EnCours,
        120_000,
        0,
        "",
    );
    let ecrite = depot::inserer_version(&mut cnx, &cochee, &ctx)
        .await
        .unwrap();
    assert_eq!(ecrite.remplace_id, None);
    let decochee = version(
        etab,
        inscription,
        StatutPriseEnCharge::Terminee,
        120_000,
        0,
        "Pas orienté par l'État",
    );
    let ecrite = depot::inserer_version(&mut cnx, &decochee, &ctx)
        .await
        .unwrap();
    assert_eq!(ecrite.remplace_id, Some(cochee.id));
    let historique = depot::historique(&mut cnx, inscription, &ctx).await.unwrap();
    assert_eq!(historique.len(), 2);
    assert_eq!(historique[0].id, decochee.id);
    assert_eq!(historique[0].statut, StatutPriseEnCharge::Terminee);
    assert_eq!(historique[1].id, cochee.id);
    assert_eq!(historique[1].statut, StatutPriseEnCharge::Remplacee);
    let courante = depot::courante(&mut cnx, inscription, &ctx).await.unwrap();
    assert_eq!(courante.map(|c| c.id), Some(decochee.id));

    // Une seconde version courante pour la même inscription : refusée par l'index unique.
    let (e, i) = (etab.vers_chaine(), inscription.vers_chaine());
    assert!(!version_brute(&mut cnx, &e, &i, "en_cours", 100_000, 0, "").await);
    // Une version remplacée de plus : acceptée.
    assert!(version_brute(&mut cnx, &e, &i, "remplacee", 100_000, 0, "").await);
    // Motif exigé pour une version terminée et pour un abandon ; abandon borné par l'attendu.
    let autre = nouvel_id::<()>().vers_chaine();
    assert!(!version_brute(&mut cnx, &e, &autre, "terminee", 0, 0, " ").await);
    assert!(!version_brute(&mut cnx, &e, &autre, "en_cours", 100_000, 10_000, "").await);
    assert!(!version_brute(&mut cnx, &e, &autre, "en_cours", 100_000, 100_001, "Fin").await);
    assert!(!version_brute(&mut cnx, &e, &autre, "en_cours", -1, 0, "").await);
    assert!(version_brute(&mut cnx, &e, &autre, "en_cours", 100_000, 10_000, "Fin").await);

    // Montant attendu : une ligne par année, niveau et série, réglée deux fois.
    let annee = nouvel_id();
    let niveau = nouvel_id();
    let mut ligne = MontantEtat {
        id: nouvel_id(),
        etablissement_id: etab,
        annee_id: annee,
        niveau_id: niveau,
        serie_id: None,
        montant: Some(Montant::depuis_francs(126_000)),
    };
    depot::regler_montant_etat(&mut cnx, &ligne, &ctx)
        .await
        .unwrap();
    ligne.id = nouvel_id();
    ligne.montant = Some(Montant::depuis_francs(96_000));
    depot::regler_montant_etat(&mut cnx, &ligne, &ctx)
        .await
        .unwrap();
    assert_eq!(
        depot::montants_etat(&mut cnx, annee, &ctx)
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        depot::montant_etat(&mut cnx, annee, niveau, Some(nouvel_id()), &ctx)
            .await
            .unwrap(),
        Some(Montant::depuis_francs(96_000))
    );

    // Versement de l'État reçu avant GESCO (sans recette), une part, puis l'annulation.
    let versement = VersementEtat {
        id: nouvel_id(),
        etablissement_id: etab,
        annee_id: annee,
        date: DateCivile::nouvelle(2026, 11, 2).unwrap(),
        montant: Montant::depuis_francs(240_000),
        reference: "OV-2026-0042".to_owned(),
        libelle: "Scolarité des élèves orientés".to_owned(),
        payeur: "État".to_owned(),
        recette_id: None,
        avant_gesco: true,
        statut: StatutVersementEtat::Valide,
        motif_annulation: String::new(),
        cree_le: Horodatage::depuis_millis_epoque(0),
        cree_par: None,
        mis_a_jour_le: Horodatage::depuis_millis_epoque(0),
        mis_a_jour_par: None,
    };
    depot::inserer_versement(&mut cnx, &versement, &ctx)
        .await
        .unwrap();
    // Ni reçu avant GESCO ni recette : refusé.
    let sans_recette = VersementEtat {
        id: nouvel_id(),
        avant_gesco: false,
        ..versement.clone()
    };
    assert!(
        depot::inserer_versement(&mut cnx, &sans_recette, &ctx)
            .await
            .is_err()
    );
    depot::inserer_part(
        &mut cnx,
        versement.id,
        inscription,
        Montant::depuis_francs(120_000),
        OriginePartEtat::Ecran,
        &ctx,
    )
    .await
    .unwrap();
    assert_eq!(
        depot::paye_etat(&mut cnx, inscription, &ctx).await.unwrap(),
        Montant::depuis_francs(120_000)
    );
    assert_eq!(
        depot::parts_recues(&mut cnx, inscription, &ctx)
            .await
            .unwrap()
            .len(),
        1
    );
    depot::annuler_versement(&mut cnx, versement.id, "Saisie erronée", &ctx)
        .await
        .unwrap();
    assert_eq!(
        depot::paye_etat(&mut cnx, inscription, &ctx).await.unwrap(),
        Montant::ZERO
    );
    assert!(
        depot::parts_du_versement(&mut cnx, versement.id, &ctx)
            .await
            .unwrap()
            .is_empty()
    );
    let annule = depot::versement(&mut cnx, versement.id, &ctx)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(annule.statut, StatutVersementEtat::Annule);

    // Rien ne se supprime, sur les quatre tables.
    for table in [
        "versement_etat_part",
        "versement_etat",
        "montant_etat",
        "prise_en_charge_etat",
    ] {
        let refus = sqlx::query(sqlx::AssertSqlSafe(format!("DELETE FROM {table}")))
            .execute(&mut *cnx)
            .await
            .unwrap_err();
        assert!(refus.to_string().contains("ne se supprime pas"), "{table}");
    }
    drop(cnx);
}

#[tokio::test]
async fn la_descente_de_la_0066_rend_le_schema_d_avant() {
    let dossier = tempfile::tempdir().unwrap();
    let base = Base::ouvrir_sans_migrer(OptionsBase::fichier(dossier.path().join("gesco.db")))
        .await
        .unwrap();
    MIGRATIONS.run_to(66, base.pool_critique()).await.unwrap();
    MIGRATIONS.undo(base.pool_critique(), 65).await.unwrap();
    assert_eq!(base.version_schema().await.unwrap(), 65);
    let restes: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master
         WHERE name LIKE 'prise_en_charge_etat%' OR name LIKE 'montant_etat%'
            OR name LIKE 'versement_etat%'",
    )
    .fetch_all(base.pool())
    .await
    .unwrap();
    assert!(restes.is_empty(), "{restes:?}");
}
