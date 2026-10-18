//! Catalogue des commandes exposées par la frontière. Chaque commande déclare son accès ; la
//! garde (R-21) refuse ce qui n'y figure pas, et un test vérifie l'exhaustivité (§7.3).

use gesco_core::core::Permission;

/// Ce qu'exige une commande avant d'atteindre son service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Acces {
    /// Aucune session : les commandes d'entrée (connexion, écran verrouillé, premier démarrage).
    Public,
    /// Une session ouverte et non verrouillée, sans permission particulière.
    Connecte,
    /// Une session ouverte et la permission donnée, dans la portée de l'établissement.
    Permission(Permission),
}

/// Commande exposée au frontend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Commande {
    /// Nom exposé, en français, tel que la frontière le déclare.
    pub nom: &'static str,
    /// Accès requis.
    pub acces: Acces,
}

impl Commande {
    const fn publique(nom: &'static str) -> Self {
        Self {
            nom,
            acces: Acces::Public,
        }
    }
    const fn connectee(nom: &'static str) -> Self {
        Self {
            nom,
            acces: Acces::Connecte,
        }
    }
    const fn permise(nom: &'static str, permission: Permission) -> Self {
        Self {
            nom,
            acces: Acces::Permission(permission),
        }
    }

    /// Retrouve une commande par son nom.
    pub fn par_nom(nom: &str) -> Option<&'static Commande> {
        COMMANDES.iter().find(|c| c.nom == nom)
    }
}

/// Commandes publiques, énumérées une seule fois : le test d'exhaustivité refuse toute commande
/// publique hors de cette liste. Ce sont les commandes de l'écran d'accueil, de l'écran verrouillé
/// et de la restauration sur un poste vierge (UC-07), où aucune session n'existe.
pub const COMMANDES_PUBLIQUES: &[&str] = &[
    "etat_demarrage",
    "premier_demarrage",
    "connexion",
    "deverrouiller_pin",
    "deverrouiller_mot_de_passe",
    "sessions_du_poste",
    "cles_deverrouiller_phrase",
    "sauvegarde_apercu",
    "sauvegarde_restaurer_initiale",
];

/// Catalogue des commandes, phase 2 comprise.
pub const COMMANDES: &[Commande] = &[
    Commande::publique("etat_demarrage"),
    Commande::publique("premier_demarrage"),
    Commande::publique("connexion"),
    Commande::publique("deverrouiller_pin"),
    Commande::publique("deverrouiller_mot_de_passe"),
    Commande::publique("sessions_du_poste"),
    Commande::publique("cles_deverrouiller_phrase"),
    Commande::publique("sauvegarde_apercu"),
    Commande::publique("sauvegarde_restaurer_initiale"),
    Commande::connectee("deconnexion"),
    Commande::connectee("verrouiller"),
    Commande::connectee("definir_pin"),
    Commande::connectee("changer_mot_de_passe"),
    Commande::connectee("session_courante"),
    Commande::connectee("avertissements"),
    Commande::permise("utilisateurs_lister", Permission::CORE_USERS_READ),
    Commande::permise("utilisateur_creer", Permission::CORE_USERS_CREATE),
    Commande::permise("utilisateur_modifier", Permission::CORE_USERS_UPDATE),
    Commande::permise(
        "utilisateur_reinitialiser_mot_de_passe",
        Permission::CORE_USERS_UPDATE,
    ),
    Commande::permise("utilisateur_deverrouiller", Permission::CORE_USERS_UPDATE),
    Commande::permise("roles_lister", Permission::CORE_USERS_READ),
    Commande::permise("role_attribuer", Permission::CORE_ROLES_ASSIGN),
    Commande::permise("permissions_catalogue", Permission::CORE_USERS_READ),
    Commande::permise("role_creer", Permission::CORE_ROLES_ASSIGN),
    Commande::permise("role_modifier", Permission::CORE_ROLES_ASSIGN),
    Commande::permise("role_retablir", Permission::CORE_ROLES_ASSIGN),
    Commande::permise("role_supprimer", Permission::CORE_ROLES_ASSIGN),
    Commande::connectee("annees_lister"),
    Commande::permise("annee_creer", Permission::ACADEMIC_YEAR_MANAGE),
    Commande::permise("annee_modifier", Permission::ACADEMIC_YEAR_MANAGE),
    Commande::permise("annee_activer", Permission::ACADEMIC_YEAR_MANAGE),
    // Fin d'année (SPEC_PHASE_10 §3 et §4) : décisions et clôture relèvent de la direction ; la
    // réouverture d'une année clôturée, d'un droit à part.
    Commande::permise("fin_annee_lire", Permission::ACADEMIC_YEAR_CLOSE),
    Commande::permise("decisions_classe_lire", Permission::ACADEMIC_YEAR_CLOSE),
    Commande::permise("decision_poser", Permission::ACADEMIC_YEAR_CLOSE),
    Commande::permise("decisions_valider_classe", Permission::ACADEMIC_YEAR_CLOSE),
    Commande::permise("passage_regler", Permission::ACADEMIC_YEAR_CLOSE),
    Commande::permise("annee_cloturer", Permission::ACADEMIC_YEAR_CLOSE),
    Commande::permise("annee_rouvrir", Permission::ACADEMIC_YEAR_REOPEN),
    Commande::permise("classes_reprendre", Permission::ACADEMIC_YEAR_CLOSE),
    Commande::permise("reinscriptions_preparer", Permission::ACADEMIC_YEAR_CLOSE),
    Commande::permise("annee_supprimer", Permission::ACADEMIC_YEAR_MANAGE),
    Commande::connectee("niveaux_lister"),
    Commande::permise("cycles_definir", Permission::ACADEMIC_YEAR_MANAGE),
    Commande::permise("niveau_modifier", Permission::ACADEMIC_YEAR_MANAGE),
    Commande::permise("serie_modifier", Permission::ACADEMIC_YEAR_MANAGE),
    Commande::permise("filiere_creer", Permission::ACADEMIC_YEAR_MANAGE),
    Commande::connectee("classes_lister"),
    Commande::permise("classe_creer", Permission::ACADEMIC_YEAR_MANAGE),
    Commande::permise("classe_modifier", Permission::ACADEMIC_YEAR_MANAGE),
    Commande::permise("classe_supprimer", Permission::ACADEMIC_YEAR_MANAGE),
    Commande::connectee("scolarite_reglages"),
    Commande::permise("eleves_rechercher", Permission::STUDENTS_READ),
    Commande::permise("eleve_lire", Permission::STUDENTS_READ),
    Commande::permise("eleve_creer", Permission::STUDENTS_CREATE),
    Commande::permise("eleve_modifier", Permission::STUDENTS_UPDATE),
    Commande::permise("eleve_photo_definir", Permission::STUDENTS_UPDATE),
    Commande::permise("eleve_doublons_probables", Permission::STUDENTS_READ),
    Commande::permise("eleves_fusion_apercu", Permission::STUDENTS_MERGE),
    Commande::permise("eleves_fusionner", Permission::STUDENTS_MERGE),
    Commande::permise("responsables_rechercher", Permission::STUDENTS_READ),
    Commande::permise("responsable_rattacher", Permission::STUDENTS_UPDATE),
    Commande::permise("inscriptions_lister", Permission::STUDENTS_READ),
    Commande::permise("inscription_creer", Permission::ENROLLMENTS_CREATE),
    Commande::permise("inscription_modifier", Permission::ENROLLMENTS_CREATE),
    Commande::permise("inscription_valider", Permission::ENROLLMENTS_VALIDATE),
    Commande::permise("inscription_abandonner", Permission::ENROLLMENTS_CANCEL),
    Commande::permise(
        "inscription_changer_classe",
        Permission::ENROLLMENTS_TRANSFER,
    ),
    Commande::permise("inscription_fiche_produire", Permission::STUDENTS_READ),
    Commande::permise("responsable_modifier", Permission::STUDENTS_UPDATE),
    Commande::permise("responsable_detacher", Permission::STUDENTS_UPDATE),
    Commande::connectee("etablissement_lire"),
    Commande::permise("etablissement_modifier", Permission::CORE_SETTINGS_UPDATE),
    Commande::permise(
        "etablissement_logo_definir",
        Permission::CORE_SETTINGS_UPDATE,
    ),
    Commande::permise("parametres_lister", Permission::CORE_SETTINGS_UPDATE),
    Commande::permise("parametre_definir", Permission::CORE_SETTINGS_UPDATE),
    Commande::permise("audit_lister", Permission::CORE_AUDIT_READ),
    Commande::permise("audit_verifier", Permission::CORE_AUDIT_READ),
    Commande::permise("sauvegarde_creer", Permission::CORE_BACKUP_CREATE),
    Commande::permise("sauvegardes_lister", Permission::CORE_BACKUP_CREATE),
    Commande::permise("sauvegarde_tester", Permission::CORE_BACKUP_CREATE),
    Commande::permise(
        "sauvegarde_destinations_lister",
        Permission::CORE_BACKUP_CREATE,
    ),
    Commande::permise(
        "sauvegarde_destinations_definir",
        Permission::CORE_BACKUP_CREATE,
    ),
    Commande::permise("certificat_confirmer", Permission::CORE_BACKUP_CREATE),
    Commande::permise("sauvegarde_restaurer", Permission::CORE_BACKUP_RESTORE),
    Commande::permise("phrase_secours_regenerer", Permission::CORE_BACKUP_RESTORE),
    Commande::permise("document_essai_produire", Permission::TEMPLATES_MANAGE),
    Commande::connectee("exporter_xlsx"),
    Commande::permise("import_eleves_analyser", Permission::IMPORT_STUDENTS),
    Commande::permise("import_eleves_valider", Permission::IMPORT_STUDENTS),
    Commande::permise("import_eleves_appliquer", Permission::IMPORT_STUDENTS),
    Commande::permise("grilles_lister", Permission::STUDENTS_READ),
    Commande::permise("grille_creer", Permission::FEES_MANAGE),
    Commande::permise("grille_modifier", Permission::FEES_MANAGE),
    Commande::permise("grille_mettre_en_vigueur", Permission::FEES_MANAGE),
    Commande::permise("situation_financiere", Permission::STUDENTS_READ),
    Commande::permise("echeancier_changer_plan", Permission::ENROLLMENTS_CREATE),
    Commande::permise("factures_rattraper_apercu", Permission::FEES_MANAGE),
    Commande::permise("factures_rattraper", Permission::FEES_MANAGE),
    Commande::permise("remise_proposer", Permission::ENROLLMENTS_CREATE),
    Commande::permise("remise_accorder", Permission::INVOICES_DISCOUNT),
    Commande::permise("remise_refuser", Permission::INVOICES_DISCOUNT),
    Commande::permise("remise_annuler", Permission::INVOICES_DISCOUNT),
    Commande::permise("remises_en_attente", Permission::INVOICES_DISCOUNT),
    Commande::connectee("caisse_session_courante"),
    Commande::permise("caisse_ouvrir", Permission::CASHBOX_OPEN),
    Commande::permise("caisse_cloturer", Permission::CASHBOX_CLOSE),
    Commande::permise("caisse_rapprocher", Permission::CASHBOX_RECONCILE),
    Commande::permise("caisse_sessions_lister", Permission::CASHBOX_RECONCILE),
    // Le caissier imprime son état ; le contrôleur, tous : le service décide (§6.2).
    Commande::connectee("etat_caisse_produire"),
    Commande::permise("encaissement_preparer", Permission::PAYMENTS_CREATE),
    Commande::permise("encaisser", Permission::PAYMENTS_CREATE),
    Commande::permise("encaisser_famille", Permission::PAYMENTS_CREATE),
    Commande::permise("recu_produire", Permission::DOCUMENTS_REPRINT),
    Commande::permise("versement_annuler", Permission::PAYMENTS_CANCEL),
    Commande::permise("releve_compte_produire", Permission::STUDENTS_READ),
    Commande::permise("impayes_lister", Permission::INVOICES_ISSUE),
    // Référentiel pédagogique (SPEC_PHASE_4 §2) : lu par tout utilisateur connecté, tenu par la
    // direction.
    Commande::connectee("pedagogie_lire"),
    Commande::permise("matiere_enregistrer", Permission::ACADEMIC_YEAR_MANAGE),
    Commande::permise("programme_enregistrer", Permission::ACADEMIC_YEAR_MANAGE),
    Commande::permise("programme_reprendre", Permission::ACADEMIC_YEAR_MANAGE),
    Commande::permise("periodes_definir", Permission::ACADEMIC_YEAR_MANAGE),
    Commande::permise("calendrier_modifier", Permission::ACADEMIC_YEAR_MANAGE),
    Commande::permise("regle_enregistrer", Permission::ACADEMIC_YEAR_MANAGE),
    // Enseignants et affectations (SPEC_PHASE_4 §3) : lus par tous, tenus par la direction ; la
    // portée de saisie de chacun se lit sans permission.
    Commande::connectee("enseignants_lister"),
    Commande::permise("enseignant_enregistrer", Permission::ACADEMIC_YEAR_MANAGE),
    Commande::connectee("affectations_lister"),
    Commande::permise("affectation_ajouter", Permission::ACADEMIC_YEAR_MANAGE),
    Commande::permise("affectation_confier", Permission::ACADEMIC_YEAR_MANAGE),
    Commande::permise("affectation_retirer", Permission::ACADEMIC_YEAR_MANAGE),
    Commande::permise("maitre_affecter", Permission::ACADEMIC_YEAR_MANAGE),
    Commande::connectee("portee_notes_lire"),
    // Évaluations et notes (SPEC_PHASE_4 §4) : la portée par classe et matière est vérifiée par
    // le service (§11.3).
    Commande::permise("evaluations_lister", Permission::GRADES_CREATE),
    Commande::permise("evaluation_creer", Permission::GRADES_CREATE),
    Commande::permise("notes_grille", Permission::GRADES_CREATE),
    Commande::permise("note_saisir", Permission::GRADES_CREATE),
    Commande::permise("evaluation_valider", Permission::GRADES_VALIDATE),
    Commande::permise("evaluation_deverrouiller", Permission::GRADES_UNLOCK),
    Commande::permise(
        "inscriptions_valider_classe",
        Permission::ENROLLMENTS_VALIDATE,
    ),
    Commande::permise("notes_import_analyser", Permission::IMPORT_GRADES),
    Commande::permise("notes_importer", Permission::IMPORT_GRADES),
    // Absences et retards (SPEC_PHASE_4 §6) : la portée par classe est vérifiée par le service.
    Commande::connectee("portee_absences_lire"),
    Commande::permise("absences_releve_lire", Permission::ATTENDANCE_CREATE),
    Commande::permise("absences_releve_enregistrer", Permission::ATTENDANCE_CREATE),
    Commande::permise("absences_justifier", Permission::ATTENDANCE_JUSTIFY),
    Commande::permise("absences_periode_lire", Permission::ATTENDANCE_CREATE),
    // Bulletins (SPEC_PHASE_4 §7) : l'appréciation d'une matière suit la portée des notes.
    Commande::permise("bulletins_lire", Permission::GRADES_CREATE),
    Commande::permise("resultats_calculer", Permission::REPORTS_GENERATE),
    Commande::permise("appreciation_saisir", Permission::GRADES_CREATE),
    Commande::permise("bulletins_valider", Permission::GRADES_VALIDATE),
    Commande::permise("bulletins_publier", Permission::REPORTS_PUBLISH),
    Commande::permise("bulletin_corriger", Permission::GRADES_VALIDATE),
    Commande::permise("bulletins_produire", Permission::REPORTS_GENERATE),
    Commande::permise("bulletin_produire", Permission::REPORTS_GENERATE),
    // Moyennes reprises d'un relevé officiel (Q-28) : la direction, qui valide les notes.
    Commande::permise("moyennes_reprises_lire", Permission::GRADES_VALIDATE),
    Commande::permise("moyenne_reprise_saisir", Permission::GRADES_VALIDATE),
    // Départs et exclusions (SPEC_PHASE_4 §9 ; réponse du 13 octobre 2026).
    Commande::permise("inscription_transferer", Permission::ENROLLMENTS_TRANSFER),
    // Tableau de bord : chaque bloc suit les permissions du compte.
    Commande::connectee("tableau_de_bord_lire"),
    Commande::permise(
        "certificat_transfert_produire",
        Permission::ENROLLMENTS_TRANSFER,
    ),
    Commande::permise("releve_moyennes_produire", Permission::REPORTS_GENERATE),
    Commande::permise("inscription_exclure", Permission::ENROLLMENTS_EXPEL),
    Commande::permise("exclusion_lever", Permission::ENROLLMENTS_EXPEL),
    Commande::permise("exclusions_lister", Permission::STUDENTS_READ),
    Commande::permise("encaissements_lister", Permission::CASHBOX_RECONCILE),
    Commande::permise("licence_etat", Permission::LICENCE_READ),
    Commande::permise("licence_installer", Permission::LICENCE_INSTALL),
    // Registre du personnel (SPEC_PHASE_6 §2) : lu avec `employees:read`, tenu avec
    // `employees:update` ; faire d'un membre un enseignant relève de qui tient les enseignants.
    // Chacun lit sa propre fiche sans aucun droit (§7).
    Commande::permise("personnel_referentiel_lire", Permission::EMPLOYEES_READ),
    Commande::permise(
        "categorie_personnel_enregistrer",
        Permission::EMPLOYEES_UPDATE,
    ),
    Commande::permise(
        "fonction_personnel_enregistrer",
        Permission::EMPLOYEES_UPDATE,
    ),
    Commande::permise("personnel_lister", Permission::EMPLOYEES_READ),
    // Import du personnel (SPEC_VERSION_1_1 §1, étape 8).
    Commande::permise("personnel_import_modele", Permission::IMPORT_STAFF),
    Commande::permise("personnel_import_analyser", Permission::IMPORT_STAFF),
    Commande::permise("personnel_import_appliquer", Permission::IMPORT_STAFF),
    Commande::permise("membre_lire", Permission::EMPLOYEES_READ),
    Commande::connectee("ma_fiche_lire"),
    Commande::permise("membre_enregistrer", Permission::EMPLOYEES_UPDATE),
    Commande::permise("membre_partir", Permission::EMPLOYEES_UPDATE),
    Commande::permise("membre_revenir", Permission::EMPLOYEES_UPDATE),
    Commande::permise("membre_enseignant", Permission::ACADEMIC_YEAR_MANAGE),
    Commande::permise("piece_dossier_ajouter", Permission::EMPLOYEES_UPDATE),
    Commande::permise("piece_dossier_enregistrer_sous", Permission::EMPLOYEES_READ),
    Commande::permise("piece_dossier_retirer", Permission::EMPLOYEES_UPDATE),
    // Engagements (SPEC_PHASE_6 §3, §7) : lus avec les fiches, tenus par qui gère les contrats ;
    // le service retire leurs montants à qui n'a pas `contracts:read` (Q-67), et un document qui
    // les imprime l'exige.
    Commande::permise("engagements_lister", Permission::EMPLOYEES_READ),
    Commande::permise("engagement_conclure", Permission::CONTRACTS_MANAGE),
    Commande::permise("engagement_terminer", Permission::CONTRACTS_MANAGE),
    Commande::permise("engagement_rompre", Permission::CONTRACTS_MANAGE),
    Commande::permise("engagement_renouvellement", Permission::CONTRACTS_MANAGE),
    Commande::permise("engagements_a_echeance", Permission::EMPLOYEES_READ),
    Commande::permise("lettre_engagement_produire", Permission::CONTRACTS_READ),
    // Heures des vacataires (SPEC_PHASE_6 §4) : lues avec les fiches, montants selon
    // `contracts:read` ; arrêtées et visées par qui gère les contrats.
    Commande::permise("heures_lire", Permission::EMPLOYEES_READ),
    Commande::permise("heures_saisir", Permission::CONTRACTS_MANAGE),
    Commande::permise("heures_viser", Permission::CONTRACTS_MANAGE),
    Commande::permise("heures_deverrouiller", Permission::CONTRACTS_MANAGE),
    Commande::permise("etat_heures_produire", Permission::CONTRACTS_READ),
    // Absences et congés du personnel (SPEC_PHASE_6 §5, §6, §7) : les absences avec les fiches,
    // la demande d'un congé par le secrétariat, la décision par la direction.
    Commande::permise("absences_conges_lire", Permission::EMPLOYEES_READ),
    Commande::permise("absence_personnel_noter", Permission::EMPLOYEES_UPDATE),
    Commande::permise("absence_personnel_justifier", Permission::EMPLOYEES_UPDATE),
    Commande::permise("absence_personnel_annuler", Permission::EMPLOYEES_UPDATE),
    Commande::permise("conge_demander", Permission::LEAVE_REQUEST),
    Commande::permise("conge_accorder", Permission::LEAVE_APPROVE),
    Commande::permise("conge_refuser", Permission::LEAVE_APPROVE),
    Commande::permise("conge_annuler", Permission::LEAVE_APPROVE),
    Commande::permise("type_conge_enregistrer", Permission::LEAVE_APPROVE),
    Commande::permise("engagement_mois_payes", Permission::CONTRACTS_MANAGE),
    // Paie (SPEC_PHASE_7 §9) : les éléments, les attributions et les réglages relèvent de qui
    // calcule la paie.
    Commande::permise("elements_paie_lire", Permission::PAYROLL_CALCULATE),
    Commande::permise("element_paie_enregistrer", Permission::PAYROLL_CALCULATE),
    Commande::permise(
        "element_paie_version_enregistrer",
        Permission::PAYROLL_CALCULATE,
    ),
    Commande::permise("attribution_enregistrer", Permission::PAYROLL_CALCULATE),
    Commande::permise("paie_reglages_enregistrer", Permission::PAYROLL_CALCULATE),
    // Paie du mois (SPEC_PHASE_7 §5) : qui calcule revoit et saisit ; la validation est à part.
    Commande::permise("paie_mois_lire", Permission::PAYROLL_CALCULATE),
    Commande::permise("paie_calculer", Permission::PAYROLL_CALCULATE),
    Commande::permise("paie_saisir", Permission::PAYROLL_CALCULATE),
    Commande::permise("paie_saisie_retirer", Permission::PAYROLL_CALCULATE),
    Commande::permise("paie_absence_confirmer", Permission::PAYROLL_CALCULATE),
    Commande::permise("paie_valider", Permission::PAYROLL_VALIDATE),
    // Avances et paiements (SPEC_PHASE_7 §4 et §7) : l'argent qui sort relève de qui paie.
    Commande::permise("paiements_personnel_lire", Permission::PAYROLL_PAY),
    Commande::permise("salaire_payer", Permission::PAYROLL_PAY),
    Commande::permise("paie_payer_reste", Permission::PAYROLL_PAY),
    Commande::permise("paiement_personnel_annuler", Permission::PAYROLL_PAY),
    Commande::permise("avance_accorder", Permission::PAYROLL_PAY),
    Commande::permise("avance_annuler", Permission::PAYROLL_PAY),
    // Documents de la paie (SPEC_PHASE_7 §8) ; chacun lit et imprime ses propres bulletins.
    Commande::permise("bulletins_paie_produire", Permission::PAYROLL_CALCULATE),
    Commande::permise("bulletin_paie_produire", Permission::PAYROLL_CALCULATE),
    Commande::permise("etat_paie_produire", Permission::PAYROLL_CALCULATE),
    Commande::permise("etat_retenues_produire", Permission::PAYROLL_CALCULATE),
    Commande::connectee("mes_bulletins_paie_lire"),
    Commande::connectee("mon_bulletin_paie_produire"),
    // Trésorerie (SPEC_PHASE_8 §2 et §5).
    Commande::permise("tresorerie_lire", Permission::TREASURY_READ),
    Commande::permise(
        "categorie_tresorerie_enregistrer",
        Permission::TREASURY_MANAGE,
    ),
    Commande::permise("compte_tresorerie_enregistrer", Permission::TREASURY_MANAGE),
    Commande::permise(
        "tresorerie_reglages_enregistrer",
        Permission::TREASURY_MANAGE,
    ),
    // Dépenses (SPEC_PHASE_8 §3) : la lecture demande un droit sur les dépenses ou la lecture de
    // la trésorerie, que le service vérifie.
    Commande::connectee("depenses_lire"),
    Commande::permise("depense_saisir", Permission::EXPENSES_CREATE),
    Commande::permise("depense_decider", Permission::EXPENSES_APPROVE),
    Commande::permise("depense_payer", Permission::EXPENSES_PAY),
    Commande::permise("depense_annuler", Permission::EXPENSES_PAY),
    Commande::permise("depense_piece_joindre", Permission::EXPENSES_CREATE),
    Commande::connectee("depense_piece_enregistrer_sous"),
    // Fournisseurs (SPEC_PHASE_8 §4) : la lecture, comme celle des dépenses ; leurs factures se
    // paient par des dépenses.
    Commande::connectee("fournisseurs_lire"),
    Commande::permise("fournisseur_enregistrer", Permission::EXPENSES_CREATE),
    Commande::permise(
        "facture_fournisseur_enregistrer",
        Permission::EXPENSES_CREATE,
    ),
    Commande::permise("facture_fournisseur_annuler", Permission::EXPENSES_PAY),
    // Comptes et soldes (SPEC_PHASE_8 §5).
    Commande::permise("comptes_soldes_lire", Permission::TREASURY_READ),
    Commande::permise("livre_compte_lire", Permission::TREASURY_READ),
    Commande::permise("transfert_faire", Permission::TREASURY_MANAGE),
    Commande::permise("ligne_tresorerie_pointer", Permission::TREASURY_MANAGE),
    Commande::permise("compte_du_mode_choisir", Permission::TREASURY_MANAGE),
    // Journal de trésorerie et budget (SPEC_PHASE_8 §7).
    Commande::permise("journal_tresorerie_lire", Permission::TREASURY_READ),
    Commande::permise("budget_lire", Permission::TREASURY_READ),
    Commande::permise("budget_prevoir", Permission::TREASURY_MANAGE),
    // Journée de l'école et emploi du temps (SPEC_PHASE_8 §8) : chacun le lit et l'imprime.
    Commande::connectee("emploi_du_temps_lire"),
    Commande::permise("journee_regler", Permission::TIMETABLE_MANAGE),
    Commande::permise("seance_placer", Permission::TIMETABLE_MANAGE),
    Commande::permise("seance_retirer", Permission::TIMETABLE_MANAGE),
    Commande::connectee("emploi_classe_produire"),
    Commande::connectee("emploi_enseignant_produire"),
    // Cantine activable (SPEC_PHASE_8 §9).
    Commande::permise("cantine_lire", Permission::CANTEEN_MANAGE),
    Commande::permise("repas_pointer", Permission::CANTEEN_MANAGE),
    Commande::permise("liste_cantine_produire", Permission::CANTEEN_MANAGE),
    // Autres recettes (SPEC_VERSION_1_1 §1).
    Commande::permise("recettes_lire", Permission::TREASURY_READ),
    Commande::permise("recette_saisir", Permission::TREASURY_MANAGE),
    Commande::permise("recette_annuler", Permission::TREASURY_MANAGE),
    // Discipline (SPEC_VERSION_1_1 §1, T-70).
    Commande::permise("discipline_registre_lire", Permission::STUDENTS_READ),
    Commande::permise("discipline_dossier_lire", Permission::STUDENTS_READ),
    Commande::permise("sanction_noter", Permission::SANCTIONS_CREATE),
    Commande::permise("sanction_annuler", Permission::SANCTIONS_CANCEL),
    // Numéros et consentement des responsables (SPEC_PHASE_5 §2).
    Commande::permise("responsable_messages_lire", Permission::STUDENTS_READ),
    Commande::permise(
        "responsable_consentement_definir",
        Permission::STUDENTS_UPDATE,
    ),
    Commande::permise("numero_remettre_a_l_essai", Permission::STUDENTS_UPDATE),
    // File des SMS (SPEC_PHASE_5 §3).
    Commande::permise("messages_lire", Permission::NOTIFICATIONS_SEND),
    Commande::permise("message_apercu", Permission::NOTIFICATIONS_SEND),
    Commande::permise("message_decider", Permission::NOTIFICATIONS_SEND_MANUAL),
    Commande::permise("message_annuler", Permission::NOTIFICATIONS_SEND_MANUAL),
    Commande::permise("message_renvoyer", Permission::NOTIFICATIONS_SEND),
    Commande::permise(
        "message_modele_definir",
        Permission::NOTIFICATIONS_CONFIGURE,
    ),
    Commande::permise("message_essai_envoyer", Permission::NOTIFICATIONS_CONFIGURE),
    // Clé modem (SPEC_PHASE_5 §4).
    Commande::permise("sms_ports_lister", Permission::NOTIFICATIONS_CONFIGURE),
    Commande::permise("sms_cle_essayer", Permission::NOTIFICATIONS_CONFIGURE),
    Commande::permise("sms_port_choisir", Permission::NOTIFICATIONS_CONFIGURE),
    Commande::permise("sms_credit_lire", Permission::NOTIFICATIONS_CONFIGURE),
    Commande::permise("sms_reprendre", Permission::NOTIFICATIONS_CONFIGURE),
    // Remise des bulletins et convocations (SPEC_PHASE_5 §6 et §7).
    Commande::permise("remises_classe_lire", Permission::STUDENTS_READ),
    Commande::permise("bulletins_non_retires_lire", Permission::STUDENTS_READ),
    Commande::permise("bulletin_remettre", Permission::DOCUMENTS_DELIVER),
    Commande::permise("bulletin_signer", Permission::DOCUMENTS_DELIVER),
    Commande::permise("bulletin_avis_renvoyer", Permission::DOCUMENTS_DELIVER),
    Commande::permise("convocations_lire", Permission::STUDENTS_READ),
    Commande::permise("convocation_creer", Permission::CONVOCATIONS_CREATE),
    Commande::permise("convocation_accuser", Permission::CONVOCATIONS_UPDATE),
    Commande::permise("convocation_reporter", Permission::CONVOCATIONS_UPDATE),
    Commande::permise("convocation_annuler", Permission::CONVOCATIONS_UPDATE),
    Commande::permise("convocation_clore", Permission::CONVOCATIONS_CLOSE),
    Commande::permise(
        "convocation_billet_produire",
        Permission::CONVOCATIONS_CREATE,
    ),
    // Archive du journal d'audit (blueprint §12.4) : elle s'écrit à la clôture ; on la vérifie.
    Commande::permise("journal_archive_verifier", Permission::CORE_AUDIT_READ),
    // Absences par séance (SPEC_VERSION_1_1 §1, T-71), au choix de l'école.
    Commande::permise("absences_seances_lire", Permission::ATTENDANCE_CREATE),
    Commande::permise(
        "absences_seances_enregistrer",
        Permission::ATTENDANCE_CREATE,
    ),
    // Import des relevés des comptes (SPEC_VERSION_1_1 §1, T-72).
    Commande::permise("releve_apercu", Permission::TREASURY_MANAGE),
    Commande::permise("releve_importer", Permission::TREASURY_MANAGE),
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn chaque_commande_a_un_nom_unique() {
        let noms: HashSet<&str> = COMMANDES.iter().map(|c| c.nom).collect();
        assert_eq!(noms.len(), COMMANDES.len());
    }

    #[test]
    fn les_commandes_publiques_sont_exactement_celles_declarees() {
        let publiques: HashSet<&str> = COMMANDES
            .iter()
            .filter(|c| c.acces == Acces::Public)
            .map(|c| c.nom)
            .collect();
        let attendues: HashSet<&str> = COMMANDES_PUBLIQUES.iter().copied().collect();
        assert_eq!(publiques, attendues);
    }

    #[test]
    fn toute_permission_declaree_appartient_au_catalogue() {
        for c in COMMANDES {
            if let Acces::Permission(p) = c.acces {
                assert!(
                    Permission::depuis_code(p.code()).is_some(),
                    "{} déclare une permission hors catalogue",
                    c.nom
                );
            }
        }
    }

    #[test]
    fn recherche_par_nom() {
        assert_eq!(
            Commande::par_nom("sauvegarde_restaurer").map(|c| c.acces),
            Some(Acces::Permission(Permission::CORE_BACKUP_RESTORE))
        );
        assert!(Commande::par_nom("inexistante").is_none());
    }
}
