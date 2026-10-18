//! Catalogue des permissions (§11.2) et rôles livrés (§11.4).
//!
//! Le catalogue est une donnée du noyau : la table `permission` en base n'est qu'une copie
//! chargée par migration, et un test vérifie qu'elles coïncident. Format `ressource:action`,
//! en anglais pour rester court et stable (R-54, exception 1).

use serde::{Deserialize, Serialize};

use super::etablissement::Etablissement;
use crate::commun::Id;

/// Une permission nommée du catalogue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Permission(&'static str);

// Le code de la permission, `domaine:ressource:action`.
impl specta::Type for Permission {
    fn definition(_types: &mut specta::Types) -> specta::datatype::DataType {
        specta::datatype::DataType::Primitive(specta::datatype::Primitive::str)
    }
}

impl Permission {
    /// Code persisté et cité dans les commandes.
    pub const fn code(&self) -> &'static str {
        self.0
    }

    /// Domaine, c'est-à-dire la partie avant le premier `:`.
    pub fn domaine(&self) -> &'static str {
        self.0.split(':').next().unwrap_or(self.0)
    }

    /// Retrouve une permission du catalogue par son code.
    pub fn depuis_code(code: &str) -> Option<Self> {
        CATALOGUE.iter().copied().find(|p| p.0 == code)
    }
}

macro_rules! catalogue {
    ($( $nom:ident = $code:literal ),+ $(,)?) => {
        impl Permission {
            $(
                #[doc = concat!("`", $code, "`")]
                pub const $nom: Self = Self($code);
            )+
        }
        /// Toutes les permissions connues du blueprint §11.2.
        pub const CATALOGUE: &[Permission] = &[ $( Permission::$nom ),+ ];
    };
}

catalogue! {
    CORE_USERS_CREATE = "core:users:create",
    CORE_USERS_READ = "core:users:read",
    CORE_USERS_UPDATE = "core:users:update",
    CORE_ROLES_ASSIGN = "core:roles:assign",
    CORE_SETTINGS_UPDATE = "core:settings:update",
    CORE_AUDIT_READ = "core:audit:read",
    CORE_BACKUP_CREATE = "core:backup:create",
    CORE_BACKUP_RESTORE = "core:backup:restore",
    STUDENTS_CREATE = "students:create",
    STUDENTS_READ = "students:read",
    STUDENTS_UPDATE = "students:update",
    STUDENTS_DELETE = "students:delete",
    STUDENTS_MERGE = "students:merge",
    ENROLLMENTS_CREATE = "enrollments:create",
    ENROLLMENTS_VALIDATE = "enrollments:validate",
    ENROLLMENTS_TRANSFER = "enrollments:transfer",
    ENROLLMENTS_CANCEL = "enrollments:cancel",
    ENROLLMENTS_EXPEL = "enrollments:expel",
    GRADES_CREATE = "grades:create",
    GRADES_UPDATE = "grades:update",
    GRADES_VALIDATE = "grades:validate",
    GRADES_PUBLISH = "grades:publish",
    GRADES_UNLOCK = "grades:unlock",
    REPORTS_GENERATE = "reports:generate",
    REPORTS_PUBLISH = "reports:publish",
    FEES_MANAGE = "fees:manage",
    INVOICES_ISSUE = "invoices:issue",
    INVOICES_CANCEL = "invoices:cancel",
    INVOICES_DISCOUNT = "invoices:discount",
    PAYMENTS_CREATE = "payments:create",
    PAYMENTS_CANCEL = "payments:cancel",
    PAYMENTS_REFUND = "payments:refund",
    CASHBOX_OPEN = "cashbox:open",
    CASHBOX_CLOSE = "cashbox:close",
    CASHBOX_RECONCILE = "cashbox:reconcile",
    EXPENSES_CREATE = "expenses:create",
    EXPENSES_APPROVE = "expenses:approve",
    EXPENSES_PAY = "expenses:pay",
    EMPLOYEES_READ = "employees:read",
    EMPLOYEES_UPDATE = "employees:update",
    CONTRACTS_READ = "contracts:read",
    CONTRACTS_MANAGE = "contracts:manage",
    LEAVE_REQUEST = "leave:request",
    LEAVE_APPROVE = "leave:approve",
    PAYROLL_CALCULATE = "payroll:calculate",
    PAYROLL_VALIDATE = "payroll:validate",
    PAYROLL_PAY = "payroll:pay",
    TREASURY_READ = "treasury:read",
    TREASURY_MANAGE = "treasury:manage",
    TIMETABLE_MANAGE = "timetable:manage",
    CANTEEN_MANAGE = "canteen:manage",
    ACADEMIC_YEAR_MANAGE = "academic_year:manage",
    ACADEMIC_YEAR_CLOSE = "academic_year:close",
    ACADEMIC_YEAR_REOPEN = "academic_year:reopen",
    ATTENDANCE_CREATE = "attendance:create",
    ATTENDANCE_UPDATE = "attendance:update",
    ATTENDANCE_JUSTIFY = "attendance:justify",
    SANCTIONS_CREATE = "sanctions:create",
    SANCTIONS_UPDATE = "sanctions:update",
    SANCTIONS_CANCEL = "sanctions:cancel",
    CONVOCATIONS_CREATE = "convocations:create",
    CONVOCATIONS_UPDATE = "convocations:update",
    CONVOCATIONS_CLOSE = "convocations:close",
    DOCUMENTS_DELIVER = "documents:deliver",
    DOCUMENTS_REPRINT = "documents:reprint",
    TEMPLATES_MANAGE = "templates:manage",
    PERIODS_LOCK = "periods:lock",
    PERIODS_UNLOCK = "periods:unlock",
    NOTIFICATIONS_CONFIGURE = "notifications:configure",
    NOTIFICATIONS_SEND = "notifications:send",
    NOTIFICATIONS_SEND_MANUAL = "notifications:send_manual",
    GUARDIANS_ACCESS_CODE = "guardians:access_code",
    IMPORT_STUDENTS = "import:students",
    IMPORT_STAFF = "import:staff",
    IMPORT_GRADES = "import:grades",
    IMPORT_STATEMENTS = "import:statements",
    EXPORT_FULL = "export:full",
    LICENCE_READ = "licence:read",
    LICENCE_INSTALL = "licence:install",
}

/// Version de la composition des rôles livrés. Une phase qui ajoute des permissions à
/// [`RoleSysteme::permissions_initiales`] l'incrémente : les bases installées reçoivent les
/// permissions manquantes à leur ouverture suivante (SPEC_PHASE_1 §5.6).
///
/// Version 2 (SPEC_PHASE_2 §15) : `enrollments:validate` rejoint le Secrétaire.
/// Version 3 (SPEC_PHASE_3 §3.3, Q-48) : `fees:manage`, nouvelle, pour le Directeur et le
/// Comptable ; le Directeur reçoit aussi `invoices:discount` et `cashbox:reconcile`, puisque la
/// direction accorde les remises et vise la caisse. `payments:cancel` reste à l'Administrateur et
/// au Comptable : le Directeur valide les notes, et aucun rôle livré ne cumule notes et argent
/// (§11.5).
/// Version 4 (SPEC_PHASE_4 §3.4, Q-56, T-23) : le Directeur reçoit `grades:create` et
/// `grades:update`, puisque la direction saisit les notes à partir du relevé de l'enseignant ; il
/// n'a toujours aucune permission `payments:*`.
/// Version 5 (SPEC_PHASE_4 §4.5, T-35) : le Directeur reçoit `import:grades`, puisque la
/// direction importe la grille Excel remplie par l'enseignant.
/// Version 6 (SPEC_PHASE_4 §6, T-37) : le Directeur reçoit `attendance:create` et
/// `attendance:justify`, puisque dans les petites écoles la direction relève et justifie les
/// absences elle-même.
/// Version 7 (SPEC_PHASE_6 §7, Q-67) : `contracts:read`, nouvelle, ouvre les montants des
/// engagements et des états des heures ; le Directeur reçoit `employees:update`, `contracts:read`,
/// `contracts:manage` et `leave:request`, le Comptable `employees:read` et `contracts:read`, le
/// Secrétaire `employees:read`, `employees:update` et `leave:request`, le Gestionnaire RH
/// `contracts:read`.
/// Version 8 (SPEC_PHASE_7 §9, T-60, Q-80) : le Directeur reçoit `payroll:calculate` et
/// `payroll:pay`, pour l'école tenue par lui seul ; le Comptable `payroll:calculate` et
/// `payroll:pay`. Le Caissier ne paie pas les salaires.
/// Version 9 (SPEC_PHASE_8 §0.2, T-61) : le Directeur reçoit `expenses:create`, `treasury:read`,
/// `treasury:manage`, `timetable:manage` et `canteen:manage`, pour l'école tenue par lui seul ; le
/// Comptable `treasury:read` et `treasury:manage` ; le Caissier `expenses:create` et
/// `expenses:pay`, puisque sous le seuil le comptable ou le caissier paie ; le Directeur des études
/// `timetable:manage` ; le Secrétaire `canteen:manage` ; la Consultation seule `treasury:read`.
pub const COMPOSITION_ROLES_LIVRES: i64 = 9;

/// Rôles livrés avec le produit (§11.4), composés selon SPEC_PHASE_1 §5.2 (Q-44).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum RoleSysteme {
    /// Non supprimable, toutes les permissions.
    Administrateur,
    /// Direction de l'établissement.
    Directeur,
    /// Direction des études.
    DirecteurDesEtudes,
    /// Secrétariat : utilisateur principal en volume horaire (§2.1).
    Secretaire,
    /// Caisse.
    Caissier,
    /// Comptabilité.
    Comptable,
    /// Enseignant.
    Enseignant,
    /// Surveillance générale.
    Surveillant,
    /// Gestion des ressources humaines.
    GestionnaireRh,
    /// Lecture seule.
    ConsultationSeule,
}

impl RoleSysteme {
    /// Les dix rôles livrés.
    pub const TOUS: &'static [Self] = &[
        Self::Administrateur,
        Self::Directeur,
        Self::DirecteurDesEtudes,
        Self::Secretaire,
        Self::Caissier,
        Self::Comptable,
        Self::Enseignant,
        Self::Surveillant,
        Self::GestionnaireRh,
        Self::ConsultationSeule,
    ];

    /// Code persisté.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Administrateur => "administrateur",
            Self::Directeur => "directeur",
            Self::DirecteurDesEtudes => "directeur_des_etudes",
            Self::Secretaire => "secretaire",
            Self::Caissier => "caissier",
            Self::Comptable => "comptable",
            Self::Enseignant => "enseignant",
            Self::Surveillant => "surveillant",
            Self::GestionnaireRh => "gestionnaire_rh",
            Self::ConsultationSeule => "consultation_seule",
        }
    }

    /// Libellé affiché.
    pub const fn libelle(&self) -> &'static str {
        match self {
            Self::Administrateur => "Administrateur",
            Self::Directeur => "Directeur",
            Self::DirecteurDesEtudes => "Directeur des études",
            Self::Secretaire => "Secrétaire",
            Self::Caissier => "Caissier",
            Self::Comptable => "Comptable",
            Self::Enseignant => "Enseignant",
            Self::Surveillant => "Surveillant général",
            Self::GestionnaireRh => "Gestionnaire RH",
            Self::ConsultationSeule => "Consultation seule",
        }
    }

    /// Permissions attribuées à la livraison, y compris celles des modules à venir : un module
    /// livré plus tard trouve ses rôles prêts. `grades:create` du Secrétaire attend Q-39.
    pub fn permissions_initiales(&self) -> Vec<Permission> {
        use Permission as P;
        match self {
            Self::Administrateur => CATALOGUE.to_vec(),
            Self::Directeur => vec![
                P::ACADEMIC_YEAR_MANAGE,
                P::CORE_USERS_READ,
                P::CORE_AUDIT_READ,
                P::CORE_BACKUP_CREATE,
                P::LICENCE_READ,
                P::STUDENTS_READ,
                P::ENROLLMENTS_VALIDATE,
                P::ENROLLMENTS_EXPEL,
                P::GRADES_CREATE,
                P::GRADES_UPDATE,
                P::GRADES_VALIDATE,
                P::GRADES_PUBLISH,
                P::GRADES_UNLOCK,
                P::IMPORT_GRADES,
                P::ATTENDANCE_CREATE,
                P::ATTENDANCE_JUSTIFY,
                P::REPORTS_GENERATE,
                P::REPORTS_PUBLISH,
                P::PERIODS_LOCK,
                P::PERIODS_UNLOCK,
                P::EXPENSES_CREATE,
                P::EXPENSES_APPROVE,
                P::TREASURY_READ,
                P::TREASURY_MANAGE,
                P::TIMETABLE_MANAGE,
                P::CANTEEN_MANAGE,
                P::EMPLOYEES_READ,
                P::EMPLOYEES_UPDATE,
                P::CONTRACTS_READ,
                P::CONTRACTS_MANAGE,
                P::LEAVE_REQUEST,
                P::LEAVE_APPROVE,
                P::PAYROLL_CALCULATE,
                P::PAYROLL_VALIDATE,
                P::PAYROLL_PAY,
                P::SANCTIONS_CREATE,
                P::SANCTIONS_UPDATE,
                P::SANCTIONS_CANCEL,
                P::CONVOCATIONS_CREATE,
                P::CONVOCATIONS_CLOSE,
                P::NOTIFICATIONS_SEND_MANUAL,
                P::ACADEMIC_YEAR_CLOSE,
                P::FEES_MANAGE,
                P::INVOICES_DISCOUNT,
                P::CASHBOX_RECONCILE,
            ],
            Self::DirecteurDesEtudes => vec![
                P::STUDENTS_READ,
                P::GRADES_CREATE,
                P::GRADES_UPDATE,
                P::GRADES_VALIDATE,
                P::REPORTS_GENERATE,
                P::PERIODS_LOCK,
                P::ATTENDANCE_CREATE,
                P::ATTENDANCE_UPDATE,
                P::ATTENDANCE_JUSTIFY,
                P::SANCTIONS_CREATE,
                P::CONVOCATIONS_CREATE,
                P::CONVOCATIONS_UPDATE,
                P::IMPORT_GRADES,
                P::TIMETABLE_MANAGE,
            ],
            Self::Secretaire => vec![
                P::STUDENTS_CREATE,
                P::STUDENTS_READ,
                P::STUDENTS_UPDATE,
                P::STUDENTS_MERGE,
                P::ENROLLMENTS_CREATE,
                P::ENROLLMENTS_VALIDATE,
                P::ENROLLMENTS_TRANSFER,
                P::ENROLLMENTS_CANCEL,
                P::ATTENDANCE_CREATE,
                P::ATTENDANCE_JUSTIFY,
                P::CONVOCATIONS_CREATE,
                P::CONVOCATIONS_UPDATE,
                P::CONVOCATIONS_CLOSE,
                P::DOCUMENTS_DELIVER,
                P::DOCUMENTS_REPRINT,
                P::REPORTS_GENERATE,
                P::IMPORT_STUDENTS,
                P::EMPLOYEES_READ,
                P::EMPLOYEES_UPDATE,
                P::LEAVE_REQUEST,
                P::CANTEEN_MANAGE,
            ],
            Self::Caissier => vec![
                P::STUDENTS_READ,
                P::INVOICES_ISSUE,
                P::PAYMENTS_CREATE,
                P::CASHBOX_OPEN,
                P::CASHBOX_CLOSE,
                P::DOCUMENTS_REPRINT,
                P::EXPENSES_CREATE,
                P::EXPENSES_PAY,
            ],
            Self::Comptable => vec![
                P::FEES_MANAGE,
                P::INVOICES_ISSUE,
                P::INVOICES_CANCEL,
                P::INVOICES_DISCOUNT,
                P::PAYMENTS_CANCEL,
                P::PAYMENTS_REFUND,
                P::CASHBOX_RECONCILE,
                P::EXPENSES_CREATE,
                P::EXPENSES_PAY,
                P::IMPORT_STATEMENTS,
                P::CORE_AUDIT_READ,
                P::EMPLOYEES_READ,
                P::CONTRACTS_READ,
                P::PAYROLL_CALCULATE,
                P::PAYROLL_PAY,
                P::TREASURY_READ,
                P::TREASURY_MANAGE,
            ],
            Self::Enseignant => vec![
                P::STUDENTS_READ,
                P::GRADES_CREATE,
                P::GRADES_UPDATE,
                P::ATTENDANCE_CREATE,
            ],
            Self::Surveillant => vec![
                P::STUDENTS_READ,
                P::ATTENDANCE_CREATE,
                P::ATTENDANCE_UPDATE,
                P::ATTENDANCE_JUSTIFY,
                P::SANCTIONS_CREATE,
                P::SANCTIONS_UPDATE,
                P::CONVOCATIONS_CREATE,
                P::CONVOCATIONS_UPDATE,
            ],
            Self::GestionnaireRh => vec![
                P::EMPLOYEES_READ,
                P::EMPLOYEES_UPDATE,
                P::CONTRACTS_READ,
                P::CONTRACTS_MANAGE,
                P::LEAVE_REQUEST,
                P::LEAVE_APPROVE,
                P::PAYROLL_CALCULATE,
                P::PAYROLL_PAY,
                P::IMPORT_STAFF,
            ],
            Self::ConsultationSeule => vec![
                P::STUDENTS_READ,
                P::EMPLOYEES_READ,
                P::CORE_AUDIT_READ,
                P::LICENCE_READ,
                P::TREASURY_READ,
            ],
        }
    }
}

/// Rôle d'un établissement, système ou créé par l'administrateur.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Role {
    /// Identifiant technique.
    pub id: Id<Role>,
    /// Établissement propriétaire.
    pub etablissement_id: Id<Etablissement>,
    /// Code stable.
    pub code: String,
    /// Libellé affiché.
    pub libelle: String,
    /// Les rôles système ne se suppriment pas ; Administrateur ne se modifie pas.
    pub systeme: bool,
}

/// Contrainte permanente du §11.5 : aucun rôle ne cumule la saisie de notes et l'encaissement.
pub fn cumul_notes_et_caisse(permissions: &[Permission]) -> bool {
    let notes = permissions.iter().any(|p| p.domaine() == "grades");
    let caisse = permissions.iter().any(|p| p.domaine() == "payments");
    notes && caisse
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn codes_uniques_et_au_bon_format() {
        let mut vus = HashSet::new();
        for p in CATALOGUE {
            assert!(vus.insert(p.code()), "doublon {}", p.code());
            let segments: Vec<&str> = p.code().split(':').collect();
            assert!(segments.len() == 2 || segments.len() == 3, "{}", p.code());
            assert!(
                p.code()
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c == ':' || c == '_')
            );
        }
        assert_eq!(
            Permission::depuis_code("grades:create"),
            Some(Permission::GRADES_CREATE)
        );
        assert_eq!(Permission::depuis_code("grades:fly"), None);
        assert_eq!(Permission::CORE_USERS_CREATE.domaine(), "core");
    }

    #[test]
    fn aucun_role_livre_ne_cumule_notes_et_caisse_sauf_l_administrateur() {
        for role in RoleSysteme::TOUS {
            let cumule = cumul_notes_et_caisse(&role.permissions_initiales());
            assert_eq!(cumule, *role == RoleSysteme::Administrateur, "{role:?}");
        }
        assert_eq!(RoleSysteme::TOUS.len(), 10);
    }

    #[test]
    fn chaque_role_livre_a_une_composition_sans_doublon() {
        for role in RoleSysteme::TOUS {
            let permissions = role.permissions_initiales();
            assert!(!permissions.is_empty(), "{role:?}");
            let uniques: HashSet<_> = permissions.iter().collect();
            assert_eq!(uniques.len(), permissions.len(), "{role:?}");
        }
    }
}
