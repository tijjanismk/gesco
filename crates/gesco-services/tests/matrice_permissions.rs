//! Matrice rôle × permission des rôles livrés (SPEC_PHASE_1 §5.5, blueprint §21.1). Le test
//! échoue si `docs/matrice-permissions.md` diffère de la composition du noyau.
//!
//! Régénération :
//! `GESCO_REGENERER_MATRICE=1 cargo test -p gesco-services --test services matrice_permissions`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fmt::Write as _;
use std::path::PathBuf;

use gesco_core::core::permission::{CATALOGUE, COMPOSITION_ROLES_LIVRES, RoleSysteme};

fn matrice() -> String {
    let compositions: Vec<_> = RoleSysteme::TOUS
        .iter()
        .map(|r| (r.libelle(), r.permissions_initiales()))
        .collect();
    let mut texte = format!(
        "# Matrice des permissions des rôles livrés\n\n\
         Fichier généré par `cargo test -p gesco-services --test services matrice_permissions` \
         depuis `RoleSysteme::permissions_initiales` (composition livrée v{COMPOSITION_ROLES_LIVRES}, \
         `SPEC_PHASE_1.md` §5.2). Ne pas modifier à la main.\n\n\
         `x` : permission accordée à la livraison. Les rôles livrés restent modifiables par \
         l'administrateur ; la portée par classe et matière s'ajoute en phase 2.\n\n| Permission |"
    );
    for (libelle, _) in &compositions {
        write!(texte, " {libelle} |").unwrap();
    }
    texte.push_str("\n|---|");
    texte.push_str(&"---|".repeat(compositions.len()));
    texte.push('\n');
    for permission in CATALOGUE {
        write!(texte, "| `{}` |", permission.code()).unwrap();
        for (_, accordees) in &compositions {
            texte.push_str(if accordees.contains(permission) {
                " x |"
            } else {
                " |"
            });
        }
        texte.push('\n');
    }
    texte.push_str("| **Total** |");
    for (_, accordees) in &compositions {
        write!(texte, " {} |", accordees.len()).unwrap();
    }
    texte.push('\n');
    texte
}

#[test]
fn la_matrice_versionnee_suit_la_composition_du_noyau() {
    let chemin = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("docs")
        .join("matrice-permissions.md");
    let attendue = matrice();
    if std::env::var_os("GESCO_REGENERER_MATRICE").is_some() {
        std::fs::write(&chemin, &attendue).unwrap();
    }
    let versionnee = std::fs::read_to_string(&chemin).unwrap_or_default();
    assert!(
        versionnee == attendue,
        "docs/matrice-permissions.md diffère de la composition : \
         GESCO_REGENERER_MATRICE=1 cargo test -p gesco-services --test services matrice_permissions"
    );
}
