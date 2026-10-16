//! Tests d'intégration des services de la phase 0.5 : premier démarrage, connexion et
//! verrouillage progressif, session rapide par PIN, garde de permission, numérotation sans trou,
//! effets après commit, chaîne d'audit intacte.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use gesco_core::commun::{Horodatage, Id};
use gesco_core::core::Poste;
use gesco_core::core::{Etablissement, StatutEtablissement, TypeDocumentNumerote, TypeEvenement};
use gesco_db::depots::{audit, outbox, sequence};
use gesco_db::unite::Durabilite;
use gesco_db::{Base, OptionsBase, nouvel_id};
use gesco_services::authentification::{NouvelUtilisateur, ServiceAuthentification};
use gesco_services::commandes::{Acces, Commande};
use gesco_services::effets::{Effet, JournalEffets};
use gesco_services::erreur::{ErreurService, MotifRefus};
use gesco_services::garde::{Garde, PorteeTotale};
use gesco_services::initialisation::{PremierDemarrage, ServiceInitialisation};
use gesco_services::numerotation;
use gesco_services::secrets::{MotDePasse, Pin};
use gesco_services::session::PolitiqueSession;
use gesco_services::{HorlogeFixe, Noyau, OptionsNoyau};

const DEBUT: i64 = 1_791_101_730_250;

struct Terrain {
    noyau: Noyau,
    horloge: Arc<HorlogeFixe>,
    effets: Arc<JournalEffets>,
    etablissement_id: Id<Etablissement>,
    _dossier: tempfile::TempDir,
}

async fn terrain() -> Terrain {
    let dossier = tempfile::tempdir().unwrap();
    let base = Base::ouvrir(OptionsBase::fichier(dossier.path().join("gesco.db")))
        .await
        .unwrap();
    let horloge = Arc::new(HorlogeFixe::a(Horodatage::depuis_millis_epoque(DEBUT)));
    let effets = Arc::new(JournalEffets::default());
    let noyau = Noyau::assembler(OptionsNoyau {
        base,
        poste: Poste("poste-test".to_owned()),
        empreinte_machine: "machine-test".to_owned(),
        horloge: horloge.clone(),
        effets: effets.clone(),
        politique_session: PolitiqueSession::default(),
    });
    let etablissement_id = nouvel_id();
    let init = ServiceInitialisation::nouveau(&noyau);
    let resultat = init
        .executer(PremierDemarrage {
            organisation: "Groupe scolaire test".to_owned().as_str(),
            etablissement: Etablissement {
                id: etablissement_id,
                organisation_id: nouvel_id(),
                nom: "Lycée test".to_owned(),
                statut: StatutEtablissement::Prive,
                code_academie: "AE-BKO-RD".to_owned(),
                code_cap: "CAP-01".to_owned(),
                adresse: "Bamako".to_owned(),
                telephones: vec!["+22370000000".to_owned()],
                mentions_legales: String::new(),
                multiple_arrondi: 5,
            },
            administrateur_identifiant: "admin",
            administrateur_nom: "Administrateur",
            administrateur_mot_de_passe: MotDePasse("premier-mot-de-passe"),
            cycles: &gesco_core::scolarite::Cycle::TOUS,
        })
        .await
        .unwrap();
    assert_eq!(resultat.roles.len(), 10);
    Terrain {
        noyau,
        horloge,
        effets,
        etablissement_id,
        _dossier: dossier,
    }
}

#[tokio::test]
async fn le_premier_demarrage_ne_se_rejoue_pas() {
    let t = terrain().await;
    let init = ServiceInitialisation::nouveau(&t.noyau);
    assert!(!init.est_vierge(t.etablissement_id).await.unwrap());
    let erreur = init
        .executer(PremierDemarrage {
            organisation: "Autre",
            etablissement: Etablissement {
                id: t.etablissement_id,
                organisation_id: nouvel_id(),
                nom: "Doublon".to_owned(),
                statut: StatutEtablissement::Public,
                code_academie: String::new(),
                code_cap: String::new(),
                adresse: String::new(),
                telephones: Vec::new(),
                mentions_legales: String::new(),
                multiple_arrondi: 5,
            },
            administrateur_identifiant: "admin2",
            administrateur_nom: "Autre",
            administrateur_mot_de_passe: MotDePasse("autre-mot-de-passe"),
            cycles: &gesco_core::scolarite::Cycle::TOUS,
        })
        .await;
    assert!(matches!(
        erreur,
        Err(ErreurService::Regle {
            cle: "premier_demarrage.deja_effectue"
        })
    ));
}

#[tokio::test]
async fn connexion_verrouillage_progressif_et_session_rapide() {
    let t = terrain().await;
    let auth = ServiceAuthentification::nouveau(&t.noyau);

    for _ in 0..3 {
        let e = auth
            .connecter(
                t.etablissement_id,
                "admin",
                MotDePasse("mauvais-mot-de-passe"),
            )
            .await;
        assert!(matches!(
            e,
            Err(ErreurService::Authentification(
                MotifRefus::IdentifiantsInvalides
            ))
        ));
    }
    let e = auth
        .connecter(
            t.etablissement_id,
            "admin",
            MotDePasse("premier-mot-de-passe"),
        )
        .await;
    assert!(
        matches!(e, Err(ErreurService::Authentification(MotifRefus::Verrouille { jusqua })) if jusqua == DEBUT + 30_000),
        "trois échecs verrouillent trente secondes"
    );
    t.horloge.avancer(30_000);
    let session = auth
        .connecter(
            t.etablissement_id,
            "admin",
            MotDePasse("premier-mot-de-passe"),
        )
        .await
        .unwrap();
    assert_eq!(session.nom_affiche, "Administrateur");
    assert_eq!(session.role_actif.as_deref(), Some("administrateur"));

    let e = auth
        .deverrouiller_par_pin(&session.jeton, Pin("1234"))
        .await;
    assert!(matches!(
        e,
        Err(ErreurService::Authentification(MotifRefus::PinNonDefini))
    ));
    auth.definir_pin(&session, MotDePasse("premier-mot-de-passe"), Pin("4321"))
        .await
        .unwrap();
    let e = auth
        .definir_pin(&session, MotDePasse("faux"), Pin("0000"))
        .await;
    assert!(matches!(
        e,
        Err(ErreurService::Authentification(
            MotifRefus::IdentifiantsInvalides
        ))
    ));

    t.horloge.avancer(2 * 60 * 1000);
    let verrouillee = t.noyau.sessions().lire(&session.jeton).unwrap();
    assert!(
        verrouillee.verrouillee,
        "deux minutes d'inactivité verrouillent"
    );

    let e = auth
        .deverrouiller_par_pin(&session.jeton, Pin("1111"))
        .await;
    assert!(matches!(
        e,
        Err(ErreurService::Authentification(MotifRefus::PinInvalide))
    ));
    let rouverte = auth
        .deverrouiller_par_pin(&session.jeton, Pin("4321"))
        .await
        .unwrap();
    assert!(!rouverte.verrouillee);

    auth.changer_mot_de_passe(
        &session,
        MotDePasse("premier-mot-de-passe"),
        MotDePasse("nouveau-mot-de-passe"),
    )
    .await
    .unwrap();
    auth.deconnecter(&session.jeton).await.unwrap();
    assert!(t.noyau.sessions().lire(&session.jeton).is_none());
    let session2 = auth
        .connecter(
            t.etablissement_id,
            "admin",
            MotDePasse("nouveau-mot-de-passe"),
        )
        .await
        .unwrap();
    assert_ne!(session2.jeton, session.jeton);

    let contexte = t.noyau.contexte(t.etablissement_id, None);
    let mut cnx = t.noyau.base().pool().acquire().await.unwrap();
    let verifiees = audit::verifier_chaine(&mut cnx, None, &contexte)
        .await
        .unwrap();
    assert!(
        verifiees >= 8,
        "chaque connexion, échec, PIN et changement est audité"
    );
}

#[tokio::test]
async fn la_garde_combine_session_et_permission() {
    let t = terrain().await;
    let auth = ServiceAuthentification::nouveau(&t.noyau);
    let garde = Garde::nouvelle(&t.noyau, Arc::new(PorteeTotale));
    let admin = auth
        .connecter(
            t.etablissement_id,
            "admin",
            MotDePasse("premier-mot-de-passe"),
        )
        .await
        .unwrap();
    let contexte_admin = t.noyau.contexte(t.etablissement_id, Some(&admin));
    auth.creer_utilisateur(
        &contexte_admin,
        NouvelUtilisateur {
            identifiant: "awa",
            nom_affiche: "Awa Traoré",
            mot_de_passe_initial: MotDePasse("bienvenue-au-secretariat"),
            roles: &["secretaire"],
        },
    )
    .await
    .unwrap();
    let doublon = auth
        .creer_utilisateur(
            &contexte_admin,
            NouvelUtilisateur {
                identifiant: "awa",
                nom_affiche: "Awa bis",
                mot_de_passe_initial: MotDePasse("bienvenue-au-secretariat"),
                roles: &[],
            },
        )
        .await;
    assert!(matches!(
        doublon,
        Err(ErreurService::Regle {
            cle: "utilisateur.identifiant_deja_pris"
        })
    ));
    let awa = auth
        .connecter(
            t.etablissement_id,
            "awa",
            MotDePasse("bienvenue-au-secretariat"),
        )
        .await
        .unwrap();

    let publique = Commande::par_nom("connexion").unwrap();
    assert!(
        garde
            .autoriser(publique, None, None)
            .await
            .unwrap()
            .is_none()
    );

    let creer = Commande::par_nom("utilisateur_creer").unwrap();
    assert!(
        garde
            .autoriser(creer, Some(&admin.jeton), None)
            .await
            .unwrap()
            .is_some()
    );
    let refus = garde.autoriser(creer, Some(&awa.jeton), None).await;
    assert!(
        matches!(
            refus,
            Err(ErreurService::PermissionRefusee {
                permission: "core:users:create"
            })
        ),
        "le rôle Secrétaire est livré vide en phase 0.5"
    );
    let connectee = Commande::par_nom("verrouiller").unwrap();
    assert!(
        garde
            .autoriser(connectee, Some(&awa.jeton), None)
            .await
            .unwrap()
            .is_some()
    );

    auth.verrouiller(&awa.jeton).unwrap();
    let refus = garde.autoriser(connectee, Some(&awa.jeton), None).await;
    assert!(matches!(
        refus,
        Err(ErreurService::Authentification(
            MotifRefus::SessionVerrouillee
        ))
    ));
    let inconnu = garde
        .autoriser(
            connectee,
            Some(&gesco_services::Jeton("inexistant".to_owned())),
            None,
        )
        .await;
    assert!(matches!(
        inconnu,
        Err(ErreurService::Authentification(MotifRefus::SessionInvalide))
    ));
    assert!(matches!(creer.acces, Acces::Permission(_)));

    let mut cnx = t.noyau.base().pool().acquire().await.unwrap();
    let evenements = outbox::non_consommes(&mut cnx, 50, &contexte_admin)
        .await
        .unwrap();
    let types: Vec<TypeEvenement> = evenements.iter().map(|e| e.type_evenement).collect();
    assert_eq!(
        types
            .iter()
            .filter(|t| **t == TypeEvenement::UtilisateurCree)
            .count(),
        2,
        "l'administrateur du premier démarrage et Awa"
    );
    assert_eq!(
        types
            .iter()
            .filter(|t| **t == TypeEvenement::RoleAttribue)
            .count(),
        2
    );
}

#[tokio::test]
async fn numerotation_sans_trou_abandon_trace_et_effets_apres_commit() {
    let t = terrain().await;
    let contexte = t.noyau.contexte(t.etablissement_id, None);

    let mut unite = t
        .noyau
        .commencer(contexte.clone(), Durabilite::Critique)
        .await
        .unwrap();
    let n1 = numerotation::attribuer(&mut unite, TypeDocumentNumerote::Test, "2026")
        .await
        .unwrap();
    assert_eq!(n1.numero, "TST-2026-000001");
    unite.annuler().await.unwrap();

    let mut unite = t
        .noyau
        .commencer(contexte.clone(), Durabilite::Critique)
        .await
        .unwrap();
    let n1 = numerotation::attribuer(&mut unite, TypeDocumentNumerote::Test, "2026")
        .await
        .unwrap();
    assert_eq!(
        n1.numero, "TST-2026-000001",
        "une transaction annulée ne consomme pas de numéro"
    );
    let n2 = numerotation::attribuer(&mut unite, TypeDocumentNumerote::Test, "2026")
        .await
        .unwrap();
    assert_eq!(n2.numero, "TST-2026-000002");
    let sans_motif = numerotation::abandonner(&mut unite, n2.clone(), "  ").await;
    assert!(matches!(
        sans_motif,
        Err(ErreurService::Regle {
            cle: "numerotation.motif_obligatoire"
        })
    ));
    numerotation::abandonner(&mut unite, n2, "Erreur de saisie")
        .await
        .unwrap();
    assert!(
        t.effets.effets().is_empty(),
        "aucun effet avant le commit (R-18)"
    );
    let executes = t.noyau.terminer(unite).await.unwrap();
    assert_eq!(executes, 1);
    assert!(matches!(
        t.effets.effets().as_slice(),
        [Effet::ComposerDocument { type_document, entite_id, .. }]
            if type_document == "numero_annule" && entite_id == "TST-2026-000002"
    ));

    let mut cnx = t.noyau.base().pool().acquire().await.unwrap();
    let compteur = sequence::compteur(&mut cnx, TypeDocumentNumerote::Test, "2026", &contexte)
        .await
        .unwrap();
    assert_eq!(compteur, 2);
    let abandonnes = sequence::abandonnes(&mut cnx, TypeDocumentNumerote::Test, "2026", &contexte)
        .await
        .unwrap();
    assert_eq!(abandonnes, vec!["TST-2026-000002".to_owned()]);
    let mut unite = t
        .noyau
        .commencer(contexte.clone(), Durabilite::Critique)
        .await
        .unwrap();
    let n3 = numerotation::attribuer(&mut unite, TypeDocumentNumerote::Test, "2026")
        .await
        .unwrap();
    assert_eq!(
        n3.numero, "TST-2026-000003",
        "l'abandon ne rend pas le numéro, il le marque"
    );
    unite.annuler().await.unwrap();
}
