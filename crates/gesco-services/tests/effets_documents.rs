//! L'exécuteur d'effets de production : un numéro abandonné (R-46) fait composer, après le
//! commit, un document « numéro annulé » enregistré avec son modèle et sa version.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::sync::Arc;
use std::time::Duration;

use gesco_core::commun::Horodatage;
use gesco_core::core::{Etablissement, Poste, StatutEtablissement, TypeDocumentNumerote};
use gesco_db::depots::document;
use gesco_db::unite::Durabilite;
use gesco_db::{Base, OptionsBase, nouvel_id};
use gesco_services::documents::{AtelierDocuments, EffetsEchoues, traiter_effets};
use gesco_services::effets::FileEffets;
use gesco_services::initialisation::{PremierDemarrage, ServiceInitialisation};
use gesco_services::numerotation;
use gesco_services::secrets::MotDePasse;
use gesco_services::session::PolitiqueSession;
use gesco_services::{EspaceDonnees, HorlogeFixe, Noyau, OptionsNoyau};

#[tokio::test]
async fn un_numero_abandonne_produit_un_document_annule() {
    let dossier = tempfile::tempdir().unwrap();
    let espace = EspaceDonnees::nouveau(dossier.path());
    espace.preparer().unwrap();
    let base = Base::ouvrir(OptionsBase::fichier(espace.base()))
        .await
        .unwrap();
    let (file, recepteur) = FileEffets::nouvelle();
    let noyau = Arc::new(Noyau::assembler(OptionsNoyau {
        base,
        poste: Poste("caisse".to_owned()),
        empreinte_machine: "machine-test".to_owned(),
        horloge: Arc::new(HorlogeFixe::a(Horodatage::depuis_millis_epoque(
            1_791_101_730_250,
        ))),
        effets: Arc::new(file),
        politique_session: PolitiqueSession::default(),
    }));
    let atelier = Arc::new(AtelierDocuments::nouveau(&espace).unwrap());
    let echecs = EffetsEchoues::default();
    tokio::spawn(traiter_effets(
        Arc::clone(&noyau),
        Arc::clone(&atelier),
        Arc::clone(&echecs),
        recepteur,
    ));

    let etablissement_id = nouvel_id();
    ServiceInitialisation::nouveau(&noyau)
        .executer(PremierDemarrage {
            organisation: "Groupe scolaire Askia",
            etablissement: Etablissement {
                id: etablissement_id,
                organisation_id: nouvel_id(),
                nom: "Lycée Askia Mohamed".to_owned(),
                statut: StatutEtablissement::Prive,
                code_academie: String::new(),
                code_cap: String::new(),
                adresse: "Bamako".to_owned(),
                telephones: Vec::new(),
                mentions_legales: String::new(),
                multiple_arrondi: 5,
            },
            administrateur_identifiant: "admin",
            administrateur_nom: "Awa Traoré",
            administrateur_mot_de_passe: MotDePasse("premier-mot-de-passe"),
            cycles: &gesco_core::scolarite::Cycle::TOUS,
        })
        .await
        .unwrap();

    let contexte = noyau.contexte(etablissement_id, None);
    let mut unite = noyau
        .commencer(contexte.clone(), Durabilite::Critique)
        .await
        .unwrap();
    let numero = numerotation::attribuer(&mut unite, TypeDocumentNumerote::Test, "2026")
        .await
        .unwrap();
    numerotation::abandonner(&mut unite, numero.clone(), "erreur de saisie")
        .await
        .unwrap();
    noyau.terminer(unite).await.unwrap();

    let mut documents = Vec::new();
    for _ in 0..200 {
        let mut cnx = noyau.base().pool().acquire().await.unwrap();
        documents = document::par_entite(&mut cnx, "numero", &numero.numero, &contexte)
            .await
            .unwrap();
        if !documents.is_empty() {
            break;
        }
        drop(cnx);
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert!(
        echecs.lock().unwrap().is_empty(),
        "{:?}",
        echecs.lock().unwrap()
    );
    assert_eq!(documents.len(), 1);
    assert_eq!(documents[0].modele_id, "numero_annule");
    assert_eq!(documents[0].modele_version, 1);
    assert!(std::path::Path::new(&documents[0].chemin).is_file());
}
