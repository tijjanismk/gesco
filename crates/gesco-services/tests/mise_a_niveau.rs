//! Mise à niveau d'une base installée (SPEC_PHASE_1 §5.6 et §6) : une base de la phase 0.5, aux
//! rôles vides, reçoit les permissions manquantes ; un retrait volontaire est respecté ; chaque
//! rôle complété est audité et annoncé dans l'`outbox` ; les paramètres apparus depuis sont
//! ajoutés sans écraser les valeurs choisies ; les niveaux et séries livrés sont posés
//! (SPEC_PHASE_2 §2.3).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::sync::Arc;

use gesco_core::commun::{Horodatage, Id};
use gesco_core::core::permission::{COMPOSITION_ROLES_LIVRES, RoleSysteme};
use gesco_core::core::{Etablissement, Permission, Poste, StatutEtablissement, TypeEvenement};
use gesco_db::depots::{academique, audit, outbox, role};
use gesco_db::{Base, OptionsBase, nouvel_id};
use gesco_services::effets::JournalEffets;
use gesco_services::initialisation::{PremierDemarrage, ServiceInitialisation};
use gesco_services::mise_a_niveau;
use gesco_services::secrets::MotDePasse;
use gesco_services::session::PolitiqueSession;
use gesco_services::{HorlogeFixe, Noyau, OptionsNoyau};

async fn installer() -> (Noyau, Id<Etablissement>, tempfile::TempDir) {
    let dossier = tempfile::tempdir().unwrap();
    let base = Base::ouvrir(OptionsBase::fichier(dossier.path().join("gesco.db")))
        .await
        .unwrap();
    let noyau = Noyau::assembler(OptionsNoyau {
        base,
        poste: Poste("poste-test".to_owned()),
        empreinte_machine: "machine-test".to_owned(),
        horloge: Arc::new(HorlogeFixe::a(Horodatage::depuis_millis_epoque(
            1_791_101_730_250,
        ))),
        effets: Arc::new(JournalEffets::default()),
        politique_session: PolitiqueSession::default(),
    });
    let etablissement_id = nouvel_id();
    ServiceInitialisation::nouveau(&noyau)
        .executer(PremierDemarrage {
            organisation: "Groupe scolaire test",
            etablissement: Etablissement {
                id: etablissement_id,
                organisation_id: nouvel_id(),
                nom: "Complexe scolaire test".to_owned(),
                statut: StatutEtablissement::Prive,
                code_academie: "AE-BKO-RD".to_owned(),
                code_cap: "CAP-01".to_owned(),
                adresse: "Bamako".to_owned(),
                telephones: vec![],
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
    (noyau, etablissement_id, dossier)
}

async fn permissions(
    noyau: &Noyau,
    etablissement_id: Id<Etablissement>,
    systeme: RoleSysteme,
) -> Vec<Permission> {
    let contexte = noyau.contexte(etablissement_id, None);
    let mut cnx = noyau.base().pool().acquire().await.unwrap();
    let r = role::par_code(&mut cnx, systeme.code(), &contexte)
        .await
        .unwrap()
        .unwrap();
    role::permissions_du_role(&mut cnx, r.id, &contexte)
        .await
        .unwrap()
}

async fn executer(noyau: &Noyau, sql: &str) {
    sqlx::query(sqlx::AssertSqlSafe(sql.to_owned()))
        .execute(noyau.base().pool_critique())
        .await
        .unwrap();
}

#[tokio::test]
async fn une_installation_neuve_est_deja_a_jour() {
    let (noyau, etablissement_id, _dossier) = installer().await;
    for systeme in RoleSysteme::TOUS {
        let mut attendues = systeme.permissions_initiales();
        let mut obtenues = permissions(&noyau, etablissement_id, *systeme).await;
        attendues.sort_by_key(|p| p.code());
        obtenues.sort_by_key(|p| p.code());
        assert_eq!(obtenues, attendues, "{systeme:?}");
    }
    assert_eq!(
        mise_a_niveau::completer_roles(&noyau, etablissement_id)
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn une_base_de_la_phase_0_5_recoit_les_permissions_manquantes() {
    let (noyau, etablissement_id, _dossier) = installer().await;
    // État d'une base créée en phase 0.5 : neuf rôles vides, un Administrateur sans la
    // permission apparue depuis, aucune composition notée.
    executer(&noyau, "DELETE FROM role_permission WHERE role_id IN (SELECT id FROM role WHERE code <> 'administrateur')").await;
    executer(
        &noyau,
        "DELETE FROM role_permission WHERE permission_code = 'academic_year:manage'",
    )
    .await;
    executer(&noyau, "UPDATE role SET composition_livree = 0").await;
    // Le Caissier, lui, a déjà été modifié par l'administrateur : il a perdu une permission et
    // porte la composition courante. La montée ne doit pas la lui rendre.
    executer(&noyau, "INSERT INTO role_permission (role_id, permission_code) SELECT id, 'payments:create' FROM role WHERE code = 'caissier'").await;
    executer(
        &noyau,
        &format!("UPDATE role SET composition_livree = {COMPOSITION_ROLES_LIVRES} WHERE code = 'caissier'"),
    )
    .await;

    assert_eq!(
        mise_a_niveau::completer_roles(&noyau, etablissement_id)
            .await
            .unwrap(),
        9
    );

    let secretaire = permissions(&noyau, etablissement_id, RoleSysteme::Secretaire).await;
    assert_eq!(
        secretaire.len(),
        RoleSysteme::Secretaire.permissions_initiales().len()
    );
    assert!(
        permissions(&noyau, etablissement_id, RoleSysteme::Administrateur)
            .await
            .contains(&Permission::ACADEMIC_YEAR_MANAGE)
    );
    assert_eq!(
        permissions(&noyau, etablissement_id, RoleSysteme::Caissier).await,
        vec![Permission::PAYMENTS_CREATE]
    );

    let contexte = noyau.contexte(etablissement_id, None);
    let mut cnx = noyau.base().pool().acquire().await.unwrap();
    let lignes = audit::dernieres(&mut cnx, 50, &contexte).await.unwrap();
    assert_eq!(
        lignes
            .iter()
            .filter(|l| l.contexte == "mise_a_niveau.roles")
            .count(),
        9
    );
    let evenements = outbox::non_consommes(&mut cnx, 200, &contexte)
        .await
        .unwrap();
    assert_eq!(
        evenements
            .iter()
            .filter(|e| e.type_evenement == TypeEvenement::RoleRecompose)
            .count(),
        9
    );
    audit::verifier_chaine(&mut cnx, None, &contexte)
        .await
        .unwrap();
    drop(cnx);

    assert_eq!(
        mise_a_niveau::completer_roles(&noyau, etablissement_id)
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn une_base_ancienne_recoit_les_parametres_apparus_depuis() {
    let (noyau, etablissement_id, _dossier) = installer().await;
    assert_eq!(
        mise_a_niveau::completer_parametres(&noyau, etablissement_id)
            .await
            .unwrap(),
        0
    );
    executer(&noyau, "DELETE FROM parametre WHERE cle LIKE 'mot_de_passe.refuser_%' OR cle = 'connexion.echecs_avant_verrouillage'").await;
    executer(
        &noyau,
        "UPDATE parametre SET valeur = '12' WHERE cle = 'mot_de_passe.longueur_min'",
    )
    .await;
    assert_eq!(
        mise_a_niveau::completer_parametres(&noyau, etablissement_id)
            .await
            .unwrap(),
        3
    );
    let contexte = noyau.contexte(etablissement_id, None);
    let mut cnx = noyau.base().pool().acquire().await.unwrap();
    let longueur =
        gesco_db::depots::parametre::entier(&mut cnx, "mot_de_passe.longueur_min", 0, &contexte)
            .await
            .unwrap();
    assert_eq!(longueur, 12, "une valeur choisie n'est jamais écrasée");
    audit::verifier_chaine(&mut cnx, None, &contexte)
        .await
        .unwrap();
}

#[tokio::test]
async fn une_base_de_la_phase_1_donne_la_validation_des_inscriptions_au_secretaire() {
    let (noyau, etablissement_id, _dossier) = installer().await;
    // Composition 1 (phase 1) : le Secrétaire n'avait pas `enrollments:validate`.
    executer(&noyau, "DELETE FROM role_permission WHERE permission_code = 'enrollments:validate' AND role_id IN (SELECT id FROM role WHERE code = 'secretaire')").await;
    executer(&noyau, "UPDATE role SET composition_livree = 1").await;
    mise_a_niveau::completer_roles(&noyau, etablissement_id)
        .await
        .unwrap();
    assert!(
        permissions(&noyau, etablissement_id, RoleSysteme::Secretaire)
            .await
            .contains(&Permission::ENROLLMENTS_VALIDATE)
    );
}

#[tokio::test]
async fn une_base_de_la_phase_2_recoit_les_permissions_des_frais() {
    let (noyau, etablissement_id, _dossier) = installer().await;
    // Composition 2 (phase 2) : `fees:manage` n'existait pas ; le Directeur n'accordait pas de
    // remise et ne visait pas la caisse.
    executer(&noyau, "DELETE FROM role_permission WHERE permission_code IN ('fees:manage', 'invoices:discount', 'cashbox:reconcile') AND role_id IN (SELECT id FROM role WHERE code IN ('directeur', 'comptable'))").await;
    executer(&noyau, "UPDATE role SET composition_livree = 2").await;
    mise_a_niveau::completer_roles(&noyau, etablissement_id)
        .await
        .unwrap();
    let directeur = permissions(&noyau, etablissement_id, RoleSysteme::Directeur).await;
    for attendue in [
        Permission::FEES_MANAGE,
        Permission::INVOICES_DISCOUNT,
        Permission::CASHBOX_RECONCILE,
    ] {
        assert!(directeur.contains(&attendue), "{attendue:?}");
    }
    assert!(!directeur.contains(&Permission::PAYMENTS_CANCEL));
    assert!(
        permissions(&noyau, etablissement_id, RoleSysteme::Comptable)
            .await
            .contains(&Permission::FEES_MANAGE)
    );
}

#[tokio::test]
async fn une_base_de_la_phase_1_recoit_les_niveaux_et_series() {
    let (noyau, etablissement_id, _dossier) = installer().await;
    assert_eq!(
        mise_a_niveau::completer_referentiel(&noyau, etablissement_id)
            .await
            .unwrap(),
        0
    );
    // Base de la phase 1 : la migration 0006 a créé des tables vides ; une base qui aurait déjà
    // des niveaux garde ses libellés.
    executer(&noyau, "DELETE FROM serie").await;
    executer(&noyau, "DELETE FROM niveau WHERE code <> '7'").await;
    executer(
        &noyau,
        "UPDATE niveau SET libelle = '7ème' WHERE code = '7'",
    )
    .await;
    assert_eq!(
        mise_a_niveau::completer_referentiel(&noyau, etablissement_id)
            .await
            .unwrap(),
        20 + 8
    );
    let contexte = noyau.contexte(etablissement_id, None);
    let mut cnx = noyau.base().pool().acquire().await.unwrap();
    let niveaux = academique::niveaux(&mut cnx, &contexte).await.unwrap();
    assert_eq!(niveaux.len(), 21);
    // Les années du CAP et du BT arrivent fermées : l'école ouvre le cycle si elle l'enseigne.
    let techniques: Vec<_> = niveaux
        .iter()
        .filter(|n| n.cycle == gesco_core::scolarite::Cycle::Technique)
        .collect();
    assert_eq!(techniques.len(), 6);
    assert!(techniques.iter().all(|n| !n.actif));
    assert!(niveaux.iter().filter(|n| n.code == "10").all(|n| n.actif));
    assert_eq!(
        niveaux.iter().find(|n| n.code == "7").unwrap().libelle,
        "7ème"
    );
    assert_eq!(
        academique::series(&mut cnx, &contexte).await.unwrap().len(),
        8
    );
    audit::verifier_chaine(&mut cnx, None, &contexte)
        .await
        .unwrap();
}

#[tokio::test]
async fn une_annee_de_la_phase_3_recoit_son_referentiel_pedagogique() {
    let (noyau, etablissement_id, _dossier) = installer().await;
    // Une année créée avant la phase 4 n'a ni périodes, ni programme, ni règles.
    let contexte = noyau.contexte(etablissement_id, None);
    let annee = gesco_core::core::AnneeScolaire {
        id: nouvel_id(),
        etablissement_id,
        libelle: "2026-2027".to_owned(),
        debut: Horodatage::depuis_millis_epoque(1_790_812_800_000),
        fin: Horodatage::depuis_millis_epoque(1_816_992_000_000),
        statut: gesco_core::core::StatutAnnee::Active,
        est_courante: true,
    };
    {
        let mut cnx = noyau.base().pool().acquire().await.unwrap();
        gesco_db::depots::annee_scolaire::inserer(&mut cnx, &annee, &contexte)
            .await
            .unwrap();
    }
    let ajoutes = mise_a_niveau::completer_pedagogie(&noyau, etablissement_id)
        .await
        .unwrap();
    assert!(ajoutes > 100, "{ajoutes}");
    let mut cnx = noyau.base().pool().acquire().await.unwrap();
    use gesco_db::depots::pedagogie;
    assert_eq!(
        pedagogie::matieres(&mut cnx, &contexte)
            .await
            .unwrap()
            .len(),
        44
    );
    // 6 niveaux du 1er cycle × 9 mois + 6 niveaux × 3 trimestres + 6 années du CAP et du BT × 2
    // semestres.
    assert_eq!(
        pedagogie::periodes(&mut cnx, annee.id, &contexte)
            .await
            .unwrap()
            .len(),
        6 * 9 + 6 * 3 + 6 * 2
    );
    assert_eq!(
        pedagogie::regles_en_vigueur(&mut cnx, annee.id, &contexte)
            .await
            .unwrap()
            .len(),
        4
    );
    drop(cnx);
    // Une seconde ouverture n'ajoute rien.
    assert_eq!(
        mise_a_niveau::completer_pedagogie(&noyau, etablissement_id)
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn une_base_de_la_phase_3_donne_la_saisie_des_notes_au_directeur() {
    let (noyau, etablissement_id, _dossier) = installer().await;
    // Composition 3 (phase 3) : la direction validait les notes sans les saisir.
    executer(&noyau, "DELETE FROM role_permission WHERE permission_code IN ('grades:create', 'grades:update') AND role_id IN (SELECT id FROM role WHERE code = 'directeur')").await;
    executer(&noyau, "UPDATE role SET composition_livree = 3").await;
    mise_a_niveau::completer_roles(&noyau, etablissement_id)
        .await
        .unwrap();
    let directeur = permissions(&noyau, etablissement_id, RoleSysteme::Directeur).await;
    assert!(directeur.contains(&Permission::GRADES_CREATE));
    assert!(directeur.contains(&Permission::GRADES_UPDATE));
    assert!(!directeur.iter().any(|p| p.code().starts_with("payments:")));
}

#[tokio::test]
async fn une_base_de_l_etape_4_donne_l_import_des_notes_au_directeur() {
    let (noyau, etablissement_id, _dossier) = installer().await;
    // Composition 4 : le Directeur saisissait les notes sans importer de grille.
    executer(&noyau, "DELETE FROM role_permission WHERE permission_code = 'import:grades' AND role_id IN (SELECT id FROM role WHERE code = 'directeur')").await;
    executer(&noyau, "UPDATE role SET composition_livree = 4").await;
    mise_a_niveau::completer_roles(&noyau, etablissement_id)
        .await
        .unwrap();
    let directeur = permissions(&noyau, etablissement_id, RoleSysteme::Directeur).await;
    assert!(directeur.contains(&Permission::IMPORT_GRADES));
}

#[tokio::test]
async fn une_base_de_l_etape_5_donne_les_absences_au_directeur() {
    let (noyau, etablissement_id, _dossier) = installer().await;
    // Composition 5 : le Directeur ne relevait ni ne justifiait les absences.
    executer(&noyau, "DELETE FROM role_permission WHERE permission_code IN ('attendance:create', 'attendance:justify') AND role_id IN (SELECT id FROM role WHERE code = 'directeur')").await;
    executer(&noyau, "UPDATE role SET composition_livree = 5").await;
    mise_a_niveau::completer_roles(&noyau, etablissement_id)
        .await
        .unwrap();
    let directeur = permissions(&noyau, etablissement_id, RoleSysteme::Directeur).await;
    assert!(directeur.contains(&Permission::ATTENDANCE_CREATE));
    assert!(directeur.contains(&Permission::ATTENDANCE_JUSTIFY));
}

#[tokio::test]
async fn les_enseignants_d_avant_la_phase_6_entrent_au_registre_du_personnel() {
    let (noyau, etablissement_id, _dossier) = installer().await;
    let contexte = noyau.contexte(etablissement_id, None);
    use gesco_core::pedagogie::{AffectationEnseignement, Enseignant, StatutEnseignant};
    use gesco_core::scolarite::{Classe, Personne};
    use gesco_db::depots::{annee_scolaire, enseignants, pedagogie, personnel, personnes};
    // Deux enseignants de la phase 4, sans fiche du personnel ; l'un est maître d'une 1re année.
    let annee = gesco_core::core::AnneeScolaire {
        id: nouvel_id(),
        etablissement_id,
        libelle: "2026-2027".to_owned(),
        debut: Horodatage::depuis_millis_epoque(1_790_812_800_000),
        fin: Horodatage::depuis_millis_epoque(1_816_992_000_000),
        statut: gesco_core::core::StatutAnnee::Active,
        est_courante: true,
    };
    let mut ids = Vec::new();
    {
        let mut cnx = noyau.base().pool().acquire().await.unwrap();
        annee_scolaire::inserer(&mut cnx, &annee, &contexte)
            .await
            .unwrap();
        for (nom, prenoms) in [("Diallo", "Oumou"), ("Cissé", "Adama")] {
            let personne = Personne {
                id: nouvel_id(),
                etablissement_id,
                nom: nom.to_owned(),
                prenoms: prenoms.to_owned(),
                sexe: None,
                date_naissance: None,
                date_naissance_estimee: false,
                lieu_naissance: String::new(),
                nationalite: String::new(),
                adresse: String::new(),
                telephones: Vec::new(),
            };
            personnes::inserer_personne(&mut cnx, &personne, &contexte)
                .await
                .unwrap();
            let e = Enseignant {
                id: nouvel_id(),
                etablissement_id,
                personne_id: personne.id,
                statut: StatutEnseignant::Titulaire,
                utilisateur_id: None,
                actif: true,
            };
            enseignants::inserer_enseignant(&mut cnx, &e, &contexte)
                .await
                .unwrap();
            ids.push(e.id);
        }
        let premiere = academique::niveaux(&mut cnx, &contexte)
            .await
            .unwrap()
            .into_iter()
            .find(|n| n.code == "1")
            .unwrap();
        let classe = Classe {
            id: nouvel_id(),
            etablissement_id,
            annee_id: annee.id,
            niveau_id: premiere.id,
            serie_id: None,
            libelle: "1re A".to_owned(),
        };
        academique::inserer_classe(&mut cnx, &classe, &contexte)
            .await
            .unwrap();
        let matiere = pedagogie::matieres(&mut cnx, &contexte).await.unwrap()[0].id;
        enseignants::inserer_affectation(
            &mut cnx,
            &AffectationEnseignement {
                id: nouvel_id(),
                etablissement_id,
                annee_id: annee.id,
                classe_id: classe.id,
                matiere_id: matiere,
                enseignant_id: ids[0],
            },
            &contexte,
        )
        .await
        .unwrap();
    }
    assert_eq!(
        mise_a_niveau::completer_personnel(&noyau, etablissement_id)
            .await
            .unwrap(),
        2
    );
    let mut cnx = noyau.base().pool().acquire().await.unwrap();
    let membres = personnel::membres(&mut cnx, &contexte).await.unwrap();
    assert_eq!(membres.len(), 2);
    let fonctions: Vec<&str> = membres.iter().map(|m| m.fonction.as_str()).collect();
    assert_eq!(fonctions, ["Maître", "Enseignant"]);
    let matricules: Vec<&str> = membres.iter().map(|m| m.matricule.as_str()).collect();
    assert_eq!(matricules, ["P-0001", "P-0002"]);
    // Les affectations n'ont pas bougé.
    assert_eq!(
        enseignants::affectations(&mut cnx, annee.id, &contexte)
            .await
            .unwrap()
            .len(),
        1
    );
    audit::verifier_chaine(&mut cnx, None, &contexte)
        .await
        .unwrap();
    drop(cnx);
    // Une seconde ouverture n'ajoute rien.
    assert_eq!(
        mise_a_niveau::completer_personnel(&noyau, etablissement_id)
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn une_base_de_la_composition_6_recoit_les_droits_du_personnel() {
    let (noyau, etablissement_id, _dossier) = installer().await;
    // Composition 6 : `contracts:read` n'existait pas, le registre du personnel restait au
    // Gestionnaire RH.
    executer(&noyau, "DELETE FROM role_permission WHERE permission_code IN ('contracts:read', 'contracts:manage', 'employees:read', 'employees:update', 'leave:request') AND role_id IN (SELECT id FROM role WHERE code IN ('directeur', 'secretaire', 'comptable'))").await;
    executer(
        &noyau,
        "DELETE FROM role_permission WHERE permission_code = 'contracts:read'",
    )
    .await;
    executer(&noyau, "UPDATE role SET composition_livree = 6").await;
    mise_a_niveau::completer_roles(&noyau, etablissement_id)
        .await
        .unwrap();
    use Permission as P;
    for (role, attendues) in [
        (
            RoleSysteme::Directeur,
            &[
                P::EMPLOYEES_UPDATE,
                P::CONTRACTS_READ,
                P::CONTRACTS_MANAGE,
                P::LEAVE_REQUEST,
            ][..],
        ),
        (
            RoleSysteme::Comptable,
            &[P::EMPLOYEES_READ, P::CONTRACTS_READ][..],
        ),
        (
            RoleSysteme::Secretaire,
            &[P::EMPLOYEES_READ, P::EMPLOYEES_UPDATE, P::LEAVE_REQUEST][..],
        ),
        (RoleSysteme::GestionnaireRh, &[P::CONTRACTS_READ][..]),
    ] {
        let obtenues = permissions(&noyau, etablissement_id, role).await;
        for p in attendues {
            assert!(obtenues.contains(p), "{role:?} sans {}", p.code());
        }
    }
    // Le Secrétaire lit le personnel sans les montants : pas de `contracts:read`.
    assert!(
        !permissions(&noyau, etablissement_id, RoleSysteme::Secretaire)
            .await
            .contains(&P::CONTRACTS_READ)
    );
}

#[tokio::test]
async fn une_base_de_la_composition_7_recoit_les_droits_de_la_paie() {
    let (noyau, etablissement_id, _dossier) = installer().await;
    // Composition 7 : seul le Gestionnaire RH calculait et payait la paie.
    executer(&noyau, "DELETE FROM role_permission WHERE permission_code IN ('payroll:calculate', 'payroll:pay') AND role_id IN (SELECT id FROM role WHERE code IN ('directeur', 'comptable'))").await;
    executer(&noyau, "UPDATE role SET composition_livree = 7").await;
    mise_a_niveau::completer_roles(&noyau, etablissement_id)
        .await
        .unwrap();
    use Permission as P;
    for role in [RoleSysteme::Directeur, RoleSysteme::Comptable] {
        let obtenues = permissions(&noyau, etablissement_id, role).await;
        for p in [P::PAYROLL_CALCULATE, P::PAYROLL_PAY] {
            assert!(obtenues.contains(&p), "{role:?} sans {}", p.code());
        }
    }
    // Le Caissier ne paie pas les salaires.
    assert!(
        !permissions(&noyau, etablissement_id, RoleSysteme::Caissier)
            .await
            .contains(&P::PAYROLL_PAY)
    );
}

#[tokio::test]
async fn une_base_de_la_composition_8_recoit_les_droits_de_la_tresorerie() {
    let (noyau, etablissement_id, _dossier) = installer().await;
    // Composition 8 : la trésorerie, l'emploi du temps et la cantine n'avaient pas de rôle.
    executer(&noyau, "DELETE FROM role_permission WHERE permission_code IN ('treasury:read', 'treasury:manage', 'timetable:manage', 'canteen:manage', 'expenses:create', 'expenses:pay') AND role_id IN (SELECT id FROM role WHERE code IN ('directeur', 'comptable', 'caissier', 'secretaire'))").await;
    executer(&noyau, "UPDATE role SET composition_livree = 8").await;
    mise_a_niveau::completer_roles(&noyau, etablissement_id)
        .await
        .unwrap();
    use Permission as P;
    let attendues = [
        (
            RoleSysteme::Directeur,
            vec![
                P::EXPENSES_CREATE,
                P::TREASURY_READ,
                P::TREASURY_MANAGE,
                P::TIMETABLE_MANAGE,
                P::CANTEEN_MANAGE,
            ],
        ),
        (
            RoleSysteme::Comptable,
            vec![P::TREASURY_READ, P::TREASURY_MANAGE, P::EXPENSES_CREATE],
        ),
        (
            RoleSysteme::Caissier,
            vec![P::EXPENSES_CREATE, P::EXPENSES_PAY],
        ),
        (RoleSysteme::Secretaire, vec![P::CANTEEN_MANAGE]),
    ];
    for (role, permissions_attendues) in attendues {
        let obtenues = permissions(&noyau, etablissement_id, role).await;
        for p in permissions_attendues {
            assert!(obtenues.contains(&p), "{role:?} sans {}", p.code());
        }
    }
    // Le Caissier n'accorde pas les dépenses au-dessus du seuil.
    assert!(
        !permissions(&noyau, etablissement_id, RoleSysteme::Caissier)
            .await
            .contains(&P::EXPENSES_APPROVE)
    );
}
