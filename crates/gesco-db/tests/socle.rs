//! Tests d'intégration de la persistance : migrations rejouées sur base vide (R-08),
//! cloisonnement (R-12), atomicité mutation + audit + outbox (R-17), chaînage (R-27),
//! numérotation sans trou (R-46).

// Dans un test, une précondition qui échoue doit arrêter le test avec son message : les lints
// `unwrap_used` / `panic` du blueprint (§5.5, règle 5) ne visent que le code livré.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use gesco_core::commun::{Horodatage, Id};
use gesco_core::core::permission::CATALOGUE;
use gesco_core::core::{
    AnneeScolaire, Etablissement, MachineAEtats, Operation, Organisation, Permission, Poste, Role,
    StatutAnnee, StatutEtablissement, TypeDocumentNumerote, TypeEvenement, Utilisateur,
};
use gesco_db::depots::utilisateur::Secrets;
use gesco_db::depots::{
    annee_scolaire, audit, etablissement, outbox, parametre, role, sequence, utilisateur,
};
use gesco_db::unite::Durabilite;
use gesco_db::{Base, Contexte, ErreurBase, OptionsBase, UniteDeTravail, nouvel_id};

type Effet = ();

struct Terrain {
    base: Base,
    etab_a: Id<Etablissement>,
    etab_b: Id<Etablissement>,
    _dossier: tempfile::TempDir,
}

fn contexte(
    etablissement_id: Id<Etablissement>,
    utilisateur_id: Option<Id<Utilisateur>>,
) -> Contexte {
    Contexte {
        etablissement_id,
        utilisateur_id,
        role_actif: Some("administrateur".to_owned()),
        poste: Poste("poste-test".to_owned()),
        empreinte_machine: "machine-test".to_owned(),
        maintenant: Horodatage::depuis_millis_epoque(1_791_101_730_250),
        correlation_id: nouvel_id::<()>().vers_chaine(),
    }
}

fn etablissement(
    id: Id<Etablissement>,
    organisation_id: Id<Organisation>,
    nom: &str,
) -> Etablissement {
    Etablissement {
        id,
        organisation_id,
        nom: nom.to_owned(),
        statut: StatutEtablissement::Prive,
        code_academie: "AE-BKO-RD".to_owned(),
        code_cap: "CAP-01".to_owned(),
        adresse: "Bamako".to_owned(),
        telephones: vec!["+22370000000".to_owned()],
        mentions_legales: String::new(),
        multiple_arrondi: 5,
    }
}

async fn terrain() -> Terrain {
    let dossier = tempfile::tempdir().unwrap_or_else(|e| panic!("dossier temporaire : {e}"));
    let base = Base::ouvrir(OptionsBase::fichier(dossier.path().join("gesco.db")))
        .await
        .unwrap_or_else(|e| panic!("ouverture : {e}"));
    let organisation = Organisation {
        id: nouvel_id(),
        nom: "Groupe scolaire test".to_owned(),
    };
    let etab_a = nouvel_id();
    let etab_b = nouvel_id();
    let ctx = contexte(etab_a, None);
    let mut unite = UniteDeTravail::<Effet>::commencer(&base, ctx.clone(), Durabilite::Critique)
        .await
        .unwrap_or_else(|e| panic!("transaction : {e}"));
    etablissement::inserer_organisation(unite.connexion(), &organisation, &ctx)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    etablissement::inserer(
        unite.connexion(),
        &etablissement(etab_a, organisation.id, "École A"),
        &ctx,
    )
    .await
    .unwrap_or_else(|e| panic!("{e}"));
    etablissement::inserer(
        unite.connexion(),
        &etablissement(etab_b, organisation.id, "École B"),
        &ctx,
    )
    .await
    .unwrap_or_else(|e| panic!("{e}"));
    unite
        .valider()
        .await
        .unwrap_or_else(|e| panic!("validation : {e}"));
    Terrain {
        base,
        etab_a,
        etab_b,
        _dossier: dossier,
    }
}

fn annee(etablissement_id: Id<Etablissement>, libelle: &str, courante: bool) -> AnneeScolaire {
    AnneeScolaire {
        id: nouvel_id(),
        etablissement_id,
        libelle: libelle.to_owned(),
        debut: Horodatage::depuis_millis_epoque(1_000),
        fin: Horodatage::depuis_millis_epoque(2_000),
        statut: StatutAnnee::Preparation,
        est_courante: courante,
    }
}

fn utilisateur(etablissement_id: Id<Etablissement>, identifiant: &str) -> Utilisateur {
    Utilisateur {
        id: nouvel_id(),
        etablissement_id,
        identifiant: identifiant.to_owned(),
        nom_affiche: identifiant.to_owned(),
        actif: true,
        doit_changer_mot_de_passe: true,
        derniere_connexion: None,
        echecs_consecutifs: 0,
        verrouille_jusqua: None,
    }
}

fn secrets() -> Secrets {
    Secrets {
        empreinte_mot_de_passe: "$argon2id$test".to_owned(),
        empreinte_pin: None,
    }
}

#[tokio::test]
async fn migrations_rejouees_sur_base_vide_et_integrite() {
    let t = terrain().await;
    assert_eq!(t.base.version_schema().await.unwrap_or(0), 48);
    assert!(t.base.verifier_integrite().await.is_ok());
    let mut cnx = t
        .base
        .pool()
        .acquire()
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    let codes = role::codes_du_catalogue_persiste(&mut cnx)
        .await
        .unwrap_or_default();
    let mut attendus: Vec<String> = CATALOGUE.iter().map(|p| p.code().to_owned()).collect();
    attendus.sort();
    assert_eq!(
        codes, attendus,
        "la table permission est la copie exacte du catalogue"
    );
}

#[tokio::test]
async fn cloisonnement_entre_etablissements() {
    let t = terrain().await;
    let ctx_a = contexte(t.etab_a, None);
    let ctx_b = contexte(t.etab_b, None);
    let annee_a = annee(t.etab_a, "2026-2027", true);

    let mut unite =
        UniteDeTravail::<Effet>::commencer(&t.base, ctx_a.clone(), Durabilite::Ordinaire)
            .await
            .unwrap_or_else(|e| panic!("{e}"));
    annee_scolaire::inserer(unite.connexion(), &annee_a, &ctx_a)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    // Une année de B insérée depuis le contexte A est refusée avant toute requête.
    let refus = annee_scolaire::inserer(
        unite.connexion(),
        &annee(t.etab_b, "2026-2027", true),
        &ctx_a,
    )
    .await;
    assert!(matches!(
        refus,
        Err(ErreurBase::Cloisonnement {
            entite: "annee_scolaire",
            ..
        })
    ));
    unite.valider().await.unwrap_or_else(|e| panic!("{e}"));

    let mut cnx = t
        .base
        .pool()
        .acquire()
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(
        annee_scolaire::lire(&mut cnx, annee_a.id, &ctx_a)
            .await
            .is_ok()
    );
    assert!(matches!(
        annee_scolaire::lire(&mut cnx, annee_a.id, &ctx_b).await,
        Err(ErreurBase::Introuvable { .. })
    ));
    assert_eq!(
        annee_scolaire::courante(&mut cnx, &ctx_b)
            .await
            .unwrap_or(None),
        None
    );
    assert_eq!(
        annee_scolaire::lister(&mut cnx, &ctx_a)
            .await
            .map(|v| v.len())
            .unwrap_or(0),
        1
    );
}

#[tokio::test]
async fn une_seule_annee_courante_garantie_en_base() {
    let t = terrain().await;
    let ctx = contexte(t.etab_a, None);
    let premiere = annee(t.etab_a, "2025-2026", true);
    let seconde = annee(t.etab_a, "2026-2027", false);
    let mut unite = UniteDeTravail::<Effet>::commencer(&t.base, ctx.clone(), Durabilite::Ordinaire)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    annee_scolaire::inserer(unite.connexion(), &premiere, &ctx)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    annee_scolaire::inserer(unite.connexion(), &seconde, &ctx)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    // Deux courantes par insertion directe : l'index unique partiel refuse.
    let doublon =
        annee_scolaire::inserer(unite.connexion(), &annee(t.etab_a, "2027-2028", true), &ctx).await;
    assert!(matches!(doublon, Err(ErreurBase::Sqlx(_))));
    // La bascule officielle, elle, passe.
    annee_scolaire::definir_courante(unite.connexion(), seconde.id, &ctx)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    let statut = StatutAnnee::Preparation
        .transiter(StatutAnnee::Active)
        .unwrap_or(StatutAnnee::Preparation);
    annee_scolaire::ecrire_statut(unite.connexion(), seconde.id, statut, &ctx)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    unite.valider().await.unwrap_or_else(|e| panic!("{e}"));

    let mut cnx = t
        .base
        .pool()
        .acquire()
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    let courante = annee_scolaire::courante(&mut cnx, &ctx)
        .await
        .unwrap_or(None);
    assert_eq!(courante.as_ref().map(|a| a.id), Some(seconde.id));
    assert_eq!(courante.map(|a| a.statut), Some(StatutAnnee::Active));
}

#[tokio::test]
async fn mutation_audit_et_outbox_sont_atomiques() {
    let t = terrain().await;
    let ctx = contexte(t.etab_a, None);
    let u = utilisateur(t.etab_a, "secretaire");

    // Opération validée : les trois écritures sont présentes et corrélées.
    let mut unite = UniteDeTravail::<Effet>::commencer(&t.base, ctx.clone(), Durabilite::Critique)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    utilisateur::inserer(unite.connexion(), &u, &secrets(), &ctx)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    unite.auditer(
        "utilisateur",
        &u.id.vers_chaine(),
        Operation::Creation,
        None,
        Some(serde_json::json!({"identifiant": "secretaire"})),
        "core.utilisateurs",
    );
    unite.emettre(
        TypeEvenement::UtilisateurCree,
        "utilisateur",
        &u.id.vers_chaine(),
        serde_json::json!({"identifiant": "secretaire"}),
    );
    unite.valider().await.unwrap_or_else(|e| panic!("{e}"));

    // Opération abandonnée après mutation : rien ne subsiste, ni la ligne, ni l'audit, ni l'événement.
    let ctx2 = contexte(t.etab_a, None);
    let u2 = utilisateur(t.etab_a, "caissier");
    let mut unite = UniteDeTravail::<Effet>::commencer(&t.base, ctx2.clone(), Durabilite::Critique)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    utilisateur::inserer(unite.connexion(), &u2, &secrets(), &ctx2)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    unite.auditer(
        "utilisateur",
        &u2.id.vers_chaine(),
        Operation::Creation,
        None,
        None,
        "core.utilisateurs",
    );
    unite.emettre(
        TypeEvenement::UtilisateurCree,
        "utilisateur",
        &u2.id.vers_chaine(),
        serde_json::json!({}),
    );
    unite.annuler().await.unwrap_or_else(|e| panic!("{e}"));

    let mut cnx = t
        .base
        .pool()
        .acquire()
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(utilisateur::compter(&mut cnx, &ctx).await.unwrap_or(0), 1);
    let lignes = audit::dernieres(&mut cnx, 10, &ctx)
        .await
        .unwrap_or_default();
    assert_eq!(lignes.len(), 1);
    let evenements = outbox::non_consommes(&mut cnx, 10, &ctx)
        .await
        .unwrap_or_default();
    assert_eq!(evenements.len(), 1);
    assert_eq!(
        lignes.first().map(|l| l.correlation_id.as_str()),
        evenements.first().map(|e| e.correlation_id.as_str())
    );
    assert_eq!(
        evenements.first().map(|e| e.type_evenement),
        Some(TypeEvenement::UtilisateurCree)
    );
    assert!(matches!(
        utilisateur::lire(&mut cnx, u2.id, &ctx).await,
        Err(ErreurBase::Introuvable { .. })
    ));
    // Les secrets ne sont pas dans l'entité ; ils se lisent à part et sont cloisonnés.
    assert_eq!(
        utilisateur::lire_secrets(&mut cnx, u.id, &ctx)
            .await
            .map(|s| s.empreinte_mot_de_passe)
            .unwrap_or_default(),
        "$argon2id$test"
    );
    assert!(
        utilisateur::lire_secrets(&mut cnx, u.id, &contexte(t.etab_b, None))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn chaine_d_audit_verifiee_et_falsification_detectee() {
    let t = terrain().await;
    let ctx = contexte(t.etab_a, None);
    for i in 0..5 {
        let c = contexte(t.etab_a, None);
        let mut unite = UniteDeTravail::<Effet>::commencer(&t.base, c, Durabilite::Critique)
            .await
            .unwrap_or_else(|e| panic!("{e}"));
        unite.auditer(
            "parametre",
            &format!("cle-{i}"),
            Operation::Modification,
            None,
            Some(serde_json::json!(i)),
            "core.parametres",
        );
        unite.valider().await.unwrap_or_else(|e| panic!("{e}"));
    }
    let mut cnx = t
        .base
        .pool()
        .acquire()
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        audit::verifier_chaine(&mut cnx, None, &ctx)
            .await
            .unwrap_or(0),
        5
    );
    assert_eq!(
        audit::verifier_chaine(&mut cnx, Some(2), &ctx)
            .await
            .unwrap_or(0),
        2
    );
    // L'établissement B a sa propre chaîne, vide.
    assert_eq!(
        audit::verifier_chaine(&mut cnx, None, &contexte(t.etab_b, None))
            .await
            .unwrap_or(99),
        0
    );

    // Les déclencheurs refusent toute modification par SQL...
    let modification = sqlx::query("UPDATE journal_audit SET apres = '99'")
        .execute(t.base.pool())
        .await;
    assert!(modification.is_err(), "le journal est en ajout seul");
    // ...donc on falsifie en désactivant les déclencheurs, comme le ferait un éditeur de fichier.
    sqlx::query("DROP TRIGGER journal_audit_ajout_seul_update")
        .execute(t.base.pool())
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    sqlx::query("UPDATE journal_audit SET apres = '99' WHERE entite_id = 'cle-2'")
        .execute(t.base.pool())
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    let rupture = audit::verifier_chaine(&mut cnx, None, &ctx).await;
    assert!(
        matches!(rupture, Err(ErreurBase::ChaineRompue { index: 2, .. })),
        "{rupture:?}"
    );
}

#[tokio::test]
async fn numerotation_sans_trou_sous_acces_concurrent() {
    let t = terrain().await;
    let base = t.base.clone();
    let etab = t.etab_a;
    let mut taches = Vec::new();
    for _ in 0..20 {
        let base = base.clone();
        taches.push(tokio::spawn(async move {
            let ctx = contexte(etab, None);
            let mut unite =
                UniteDeTravail::<Effet>::commencer(&base, ctx.clone(), Durabilite::Critique)
                    .await
                    .unwrap_or_else(|e| panic!("{e}"));
            let numero = sequence::prochain_numero(
                unite.connexion(),
                TypeDocumentNumerote::Recu,
                "2026",
                &ctx,
            )
            .await
            .unwrap_or_else(|e| panic!("{e}"));
            unite.valider().await.unwrap_or_else(|e| panic!("{e}"));
            numero
        }));
    }
    let mut numeros = Vec::new();
    for tache in taches {
        numeros.push(tache.await.unwrap_or_else(|e| panic!("{e}")));
    }
    numeros.sort();
    let attendus: Vec<String> = (1..=20).map(|i| format!("REC-2026-{i:06}")).collect();
    assert_eq!(numeros, attendus);

    // Un numéro pris dans une transaction annulée n'est pas perdu : la séquence revient en arrière
    // avec la transaction, donc pas de trou (R-46).
    let ctx = contexte(etab, None);
    let mut unite = UniteDeTravail::<Effet>::commencer(&base, ctx.clone(), Durabilite::Critique)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    let abandonne =
        sequence::prochain_numero(unite.connexion(), TypeDocumentNumerote::Recu, "2026", &ctx)
            .await
            .unwrap_or_default();
    assert_eq!(abandonne, "REC-2026-000021");
    unite.annuler().await.unwrap_or_else(|e| panic!("{e}"));
    let mut cnx = base
        .pool()
        .acquire()
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        sequence::compteur(&mut cnx, TypeDocumentNumerote::Recu, "2026", &ctx)
            .await
            .unwrap_or(0),
        20
    );
    // Autre exercice, autre séquence ; autre établissement, autre séquence.
    assert_eq!(
        sequence::compteur(&mut cnx, TypeDocumentNumerote::Recu, "2027", &ctx)
            .await
            .unwrap_or(99),
        0
    );
    assert_eq!(
        sequence::compteur(
            &mut cnx,
            TypeDocumentNumerote::Recu,
            "2026",
            &contexte(t.etab_b, None)
        )
        .await
        .unwrap_or(99),
        0
    );
}

#[tokio::test]
async fn roles_permissions_et_parametres() {
    let t = terrain().await;
    let ctx = contexte(t.etab_a, None);
    let u = utilisateur(t.etab_a, "directeur");
    let r = Role {
        id: nouvel_id(),
        etablissement_id: t.etab_a,
        code: "directeur".to_owned(),
        libelle: "Directeur".to_owned(),
        systeme: true,
    };
    let mut unite = UniteDeTravail::<Effet>::commencer(&t.base, ctx.clone(), Durabilite::Critique)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    utilisateur::inserer(unite.connexion(), &u, &secrets(), &ctx)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    role::inserer(unite.connexion(), &r, &ctx)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    role::accorder(unite.connexion(), r.id, Permission::CORE_USERS_READ, &ctx)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    role::accorder(unite.connexion(), r.id, Permission::CORE_AUDIT_READ, &ctx)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    role::accorder(unite.connexion(), r.id, Permission::CORE_AUDIT_READ, &ctx)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    role::attribuer(unite.connexion(), u.id, r.id, &ctx)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    parametre::definir_si_absent(
        unite.connexion(),
        &gesco_core::core::parametre::parametres_par_defaut(t.etab_a),
        &ctx,
    )
    .await
    .unwrap_or_else(|e| panic!("{e}"));
    unite.valider().await.unwrap_or_else(|e| panic!("{e}"));

    let mut cnx = t
        .base
        .pool()
        .acquire()
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        role::permissions_de(&mut cnx, u.id, &ctx)
            .await
            .unwrap_or_default(),
        vec![Permission::CORE_USERS_READ, Permission::CORE_AUDIT_READ]
    );
    assert!(
        role::permissions_de(&mut cnx, u.id, &contexte(t.etab_b, None))
            .await
            .unwrap_or_default()
            .is_empty()
    );
    assert_eq!(
        parametre::entier(&mut cnx, "session.verrouillage_inactivite_s", 0, &ctx)
            .await
            .unwrap_or(0),
        900
    );
    assert_eq!(
        parametre::entier(
            &mut cnx,
            "session.verrouillage_inactivite_s",
            7,
            &contexte(t.etab_b, None)
        )
        .await
        .unwrap_or(0),
        7
    );
    assert!(
        !parametre::booleen_param(&mut cnx, "sauvegarde.certificat_confirme", true, &ctx)
            .await
            .unwrap_or(true)
    );
}
