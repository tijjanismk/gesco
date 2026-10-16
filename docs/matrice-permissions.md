# Matrice des permissions des rôles livrés

Fichier généré par `cargo test -p gesco-services --test matrice_permissions` depuis `RoleSysteme::permissions_initiales` (composition livrée v9, `SPEC_PHASE_1.md` §5.2). Ne pas modifier à la main.

`x` : permission accordée à la livraison. Les rôles livrés restent modifiables par l'administrateur ; la portée par classe et matière s'ajoute en phase 2.

| Permission | Administrateur | Directeur | Directeur des études | Secrétaire | Caissier | Comptable | Enseignant | Surveillant général | Gestionnaire RH | Consultation seule |
|---|---|---|---|---|---|---|---|---|---|---|
| `core:users:create` | x | | | | | | | | | |
| `core:users:read` | x | x | | | | | | | | |
| `core:users:update` | x | | | | | | | | | |
| `core:roles:assign` | x | | | | | | | | | |
| `core:settings:update` | x | | | | | | | | | |
| `core:audit:read` | x | x | | | | x | | | | x |
| `core:backup:create` | x | x | | | | | | | | |
| `core:backup:restore` | x | | | | | | | | | |
| `students:create` | x | | | x | | | | | | |
| `students:read` | x | x | x | x | x | | x | x | | x |
| `students:update` | x | | | x | | | | | | |
| `students:delete` | x | | | | | | | | | |
| `students:merge` | x | | | x | | | | | | |
| `enrollments:create` | x | | | x | | | | | | |
| `enrollments:validate` | x | x | | x | | | | | | |
| `enrollments:transfer` | x | | | x | | | | | | |
| `enrollments:cancel` | x | | | x | | | | | | |
| `enrollments:expel` | x | x | | | | | | | | |
| `grades:create` | x | x | x | | | | x | | | |
| `grades:update` | x | x | x | | | | x | | | |
| `grades:validate` | x | x | x | | | | | | | |
| `grades:publish` | x | x | | | | | | | | |
| `grades:unlock` | x | x | | | | | | | | |
| `reports:generate` | x | x | x | x | | | | | | |
| `reports:publish` | x | x | | | | | | | | |
| `fees:manage` | x | x | | | | x | | | | |
| `invoices:issue` | x | | | | x | x | | | | |
| `invoices:cancel` | x | | | | | x | | | | |
| `invoices:discount` | x | x | | | | x | | | | |
| `payments:create` | x | | | | x | | | | | |
| `payments:cancel` | x | | | | | x | | | | |
| `payments:refund` | x | | | | | x | | | | |
| `cashbox:open` | x | | | | x | | | | | |
| `cashbox:close` | x | | | | x | | | | | |
| `cashbox:reconcile` | x | x | | | | x | | | | |
| `expenses:create` | x | x | | | x | x | | | | |
| `expenses:approve` | x | x | | | | | | | | |
| `expenses:pay` | x | | | | x | x | | | | |
| `employees:read` | x | x | | x | | x | | | x | x |
| `employees:update` | x | x | | x | | | | | x | |
| `contracts:read` | x | x | | | | x | | | x | |
| `contracts:manage` | x | x | | | | | | | x | |
| `leave:request` | x | x | | x | | | | | x | |
| `leave:approve` | x | x | | | | | | | x | |
| `payroll:calculate` | x | x | | | | x | | | x | |
| `payroll:validate` | x | x | | | | | | | | |
| `payroll:pay` | x | x | | | | x | | | x | |
| `treasury:read` | x | x | | | | x | | | | x |
| `treasury:manage` | x | x | | | | x | | | | |
| `timetable:manage` | x | x | x | | | | | | | |
| `canteen:manage` | x | x | | x | | | | | | |
| `academic_year:manage` | x | x | | | | | | | | |
| `academic_year:close` | x | x | | | | | | | | |
| `academic_year:reopen` | x | | | | | | | | | |
| `attendance:create` | x | x | x | x | | | x | x | | |
| `attendance:update` | x | | x | | | | | x | | |
| `attendance:justify` | x | x | x | x | | | | x | | |
| `sanctions:create` | x | x | x | | | | | x | | |
| `sanctions:update` | x | x | | | | | | x | | |
| `sanctions:cancel` | x | x | | | | | | | | |
| `convocations:create` | x | x | x | x | | | | x | | |
| `convocations:update` | x | | x | x | | | | x | | |
| `convocations:close` | x | x | | x | | | | | | |
| `documents:deliver` | x | | | x | | | | | | |
| `documents:reprint` | x | | | x | x | | | | | |
| `templates:manage` | x | | | | | | | | | |
| `periods:lock` | x | x | x | | | | | | | |
| `periods:unlock` | x | x | | | | | | | | |
| `notifications:configure` | x | | | | | | | | | |
| `notifications:send` | x | | | | | | | | | |
| `notifications:send_manual` | x | x | | | | | | | | |
| `import:students` | x | | | x | | | | | | |
| `import:staff` | x | | | | | | | | x | |
| `import:grades` | x | x | x | | | | | | | |
| `import:statements` | x | | | | | x | | | | |
| `export:full` | x | | | | | | | | | |
| `licence:read` | x | x | | | | | | | | x |
| `licence:install` | x | | | | | | | | | |
| **Total** | 78 | 45 | 14 | 21 | 8 | 17 | 4 | 8 | 9 | 5 |
