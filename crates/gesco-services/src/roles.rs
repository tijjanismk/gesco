//! Rôles personnalisés et composition des rôles (SPEC_PHASE_1 §5) : créer, recomposer,
//! rétablir, supprimer. Chaque mutation est auditée avant et après (§12.3 du blueprint) et
//! publiée dans l'`outbox`.

use gesco_core::core::permission::{
    CATALOGUE, COMPOSITION_ROLES_LIVRES, RoleSysteme, cumul_notes_et_caisse,
};
use gesco_core::core::{Operation, Permission, Role, TypeEvenement};
use gesco_db::depots::role;
use gesco_db::unite::Durabilite;
use gesco_db::{Contexte, nouvel_id};

use crate::administration::FicheRole;
use crate::effets::Effet;
use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;
use crate::session::Session;

/// Longueur maximale d'un libellé de rôle.
const LIBELLE_MAX: usize = 60;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

fn codes(permissions: &[Permission]) -> Vec<&'static str> {
    permissions.iter().map(Permission::code).collect()
}

/// Permissions demandées, lues dans le catalogue, sans doublon et dans l'ordre du catalogue.
fn lire_permissions(demandees: &[String]) -> Resultat<Vec<Permission>> {
    for code in demandees {
        Permission::depuis_code(code).ok_or(regle("role.permission_inconnue"))?;
    }
    Ok(CATALOGUE
        .iter()
        .copied()
        .filter(|p| demandees.iter().any(|d| d == p.code()))
        .collect())
}

/// Code stable d'un rôle personnalisé, tiré de son libellé : `perso_` et le libellé en
/// minuscules sans accent, les autres caractères remplacés par `_`.
fn code_depuis_libelle(libelle: &str) -> String {
    let mut code = String::from("perso_");
    for c in libelle.trim().to_lowercase().chars() {
        let simple = match c {
            'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            c if c.is_ascii_alphanumeric() => c,
            _ => '_',
        };
        if simple != '_' || !code.ends_with('_') {
            code.push(simple);
        }
    }
    code.trim_end_matches('_').to_owned()
}

/// Service des rôles.
pub struct ServiceRoles<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceRoles<'n> {
    /// Service adossé au noyau.
    pub const fn nouveau(noyau: &'n Noyau) -> Self {
        Self { noyau }
    }

    /// Le catalogue complet, dans son ordre.
    pub fn catalogue() -> Vec<Permission> {
        CATALOGUE.to_vec()
    }

    /// Contrôles communs à toute composition : pas de cumul notes et caisse (Q-43), et aucune
    /// permission que l'auteur ne détient pas lui-même, pour qu'aucun rôle ne serve à s'élever.
    async fn verifier_composition(
        unite: &mut gesco_db::UniteDeTravail<Effet>,
        session: &Session,
        contexte: &Contexte,
        ajoutees: &[Permission],
        composition: &[Permission],
    ) -> Resultat<()> {
        if cumul_notes_et_caisse(composition) {
            return Err(regle("role.cumul_notes_caisse"));
        }
        let detenues =
            role::permissions_de(unite.connexion(), session.utilisateur_id, contexte).await?;
        if ajoutees.iter().any(|p| !detenues.contains(p)) {
            return Err(regle("role.permission_non_detenue"));
        }
        Ok(())
    }

    async fn verifier_libelle(
        unite: &mut gesco_db::UniteDeTravail<Effet>,
        contexte: &Contexte,
        libelle: &str,
        sauf: Option<&Role>,
    ) -> Resultat<()> {
        if libelle.is_empty() || libelle.chars().count() > LIBELLE_MAX {
            return Err(regle("role.libelle_invalide"));
        }
        let pris = role::lister(unite.connexion(), contexte)
            .await?
            .into_iter()
            .any(|r| {
                sauf.is_none_or(|s| s.id != r.id)
                    && r.libelle.to_lowercase() == libelle.to_lowercase()
            });
        if pris {
            return Err(regle("role.libelle_existant"));
        }
        Ok(())
    }

    async fn rendre(
        unite: &mut gesco_db::UniteDeTravail<Effet>,
        r: Role,
        contexte: &Contexte,
    ) -> Resultat<FicheRole> {
        let permissions = role::permissions_du_role(unite.connexion(), r.id, contexte).await?;
        Ok(FicheRole {
            role: r,
            permissions,
        })
    }

    /// Crée un rôle personnalisé.
    pub async fn creer(
        &self,
        session: &Session,
        libelle: &str,
        permissions: &[String],
    ) -> Resultat<FicheRole> {
        let libelle = libelle.trim();
        let composition = lire_permissions(permissions)?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        Self::verifier_libelle(&mut unite, &contexte, libelle, None).await?;
        Self::verifier_composition(&mut unite, session, &contexte, &composition, &composition)
            .await?;
        let base = code_depuis_libelle(libelle);
        let mut code = base.clone();
        let mut rang = 2;
        while role::code_existe(unite.connexion(), &code, &contexte).await? {
            code = format!("{base}_{rang}");
            rang += 1;
        }
        let r = Role {
            id: nouvel_id(),
            etablissement_id: session.etablissement_id,
            code,
            libelle: libelle.to_owned(),
            systeme: false,
        };
        role::inserer(unite.connexion(), &r, &contexte).await?;
        role::definir_permissions(unite.connexion(), r.id, &composition, &contexte).await?;
        let id = r.id.vers_chaine();
        unite.auditer(
            "role",
            &id,
            Operation::Creation,
            None,
            Some(serde_json::json!({
                "code": r.code,
                "libelle": r.libelle,
                "permissions": codes(&composition),
            })),
            "roles.creer",
        );
        unite.emettre(
            TypeEvenement::RoleCree,
            "role",
            &id,
            serde_json::json!({
                "code": r.code,
                "libelle": r.libelle,
                "permissions": codes(&composition),
            }),
        );
        let fiche = Self::rendre(&mut unite, r, &contexte).await?;
        self.noyau.terminer(unite).await?;
        Ok(fiche)
    }

    /// Change la composition d'un rôle, et le libellé d'un rôle personnalisé. Le rôle
    /// Administrateur ne se modifie pas ; un rôle livré garde son libellé.
    pub async fn modifier(
        &self,
        session: &Session,
        code: &str,
        libelle: &str,
        permissions: &[String],
    ) -> Resultat<FicheRole> {
        let composition = lire_permissions(permissions)?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let mut r = role::par_code(unite.connexion(), code, &contexte)
            .await?
            .ok_or(regle("role.inconnu"))?;
        if r.code == RoleSysteme::Administrateur.code() {
            return Err(regle("role.administrateur_intouchable"));
        }
        let avant = role::permissions_du_role(unite.connexion(), r.id, &contexte).await?;
        let ajoutees: Vec<Permission> = composition
            .iter()
            .copied()
            .filter(|p| !avant.contains(p))
            .collect();
        let retirees: Vec<Permission> = avant
            .iter()
            .copied()
            .filter(|p| !composition.contains(p))
            .collect();
        Self::verifier_composition(&mut unite, session, &contexte, &ajoutees, &composition).await?;
        let libelle_avant = r.libelle.clone();
        let libelle = libelle.trim();
        if !r.systeme && libelle != r.libelle {
            Self::verifier_libelle(&mut unite, &contexte, libelle, Some(&r)).await?;
            role::renommer(unite.connexion(), r.id, libelle, &contexte).await?;
            libelle.clone_into(&mut r.libelle);
        }
        role::definir_permissions(unite.connexion(), r.id, &composition, &contexte).await?;
        let id = r.id.vers_chaine();
        unite.auditer(
            "role",
            &id,
            Operation::Modification,
            Some(serde_json::json!({ "libelle": libelle_avant, "permissions": codes(&avant) })),
            Some(serde_json::json!({ "libelle": r.libelle, "permissions": codes(&composition) })),
            "roles.modifier",
        );
        unite.emettre(
            TypeEvenement::RoleRecompose,
            "role",
            &id,
            serde_json::json!({
                "role": r.code,
                "ajoutees": codes(&ajoutees),
                "retirees": codes(&retirees),
            }),
        );
        let fiche = Self::rendre(&mut unite, r, &contexte).await?;
        self.noyau.terminer(unite).await?;
        Ok(fiche)
    }

    /// Rend à un rôle livré sa composition par défaut, retraits et ajouts compris.
    pub async fn retablir(&self, session: &Session, code: &str) -> Resultat<FicheRole> {
        let systeme = RoleSysteme::TOUS
            .iter()
            .find(|s| s.code() == code && **s != RoleSysteme::Administrateur)
            .ok_or(regle("role.non_retablissable"))?;
        let composition = systeme.permissions_initiales();
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let r = role::par_code(unite.connexion(), code, &contexte)
            .await?
            .ok_or(regle("role.inconnu"))?;
        let avant = role::permissions_du_role(unite.connexion(), r.id, &contexte).await?;
        let ajoutees: Vec<Permission> = composition
            .iter()
            .copied()
            .filter(|p| !avant.contains(p))
            .collect();
        let retirees: Vec<Permission> = avant
            .iter()
            .copied()
            .filter(|p| !composition.contains(p))
            .collect();
        Self::verifier_composition(&mut unite, session, &contexte, &ajoutees, &composition).await?;
        role::definir_permissions(unite.connexion(), r.id, &composition, &contexte).await?;
        role::noter_composition_livree(
            unite.connexion(),
            r.id,
            COMPOSITION_ROLES_LIVRES,
            &contexte,
        )
        .await?;
        let id = r.id.vers_chaine();
        unite.auditer(
            "role",
            &id,
            Operation::Modification,
            Some(serde_json::json!({ "permissions": codes(&avant) })),
            Some(serde_json::json!({ "permissions": codes(&composition) })),
            "roles.retablir",
        );
        unite.emettre(
            TypeEvenement::RoleRecompose,
            "role",
            &id,
            serde_json::json!({
                "role": r.code,
                "ajoutees": codes(&ajoutees),
                "retirees": codes(&retirees),
            }),
        );
        let fiche = Self::rendre(&mut unite, r, &contexte).await?;
        self.noyau.terminer(unite).await?;
        Ok(fiche)
    }

    /// Supprime logiquement un rôle personnalisé qu'aucun utilisateur actif ne porte.
    pub async fn supprimer(&self, session: &Session, code: &str) -> Resultat<()> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let r = role::par_code(unite.connexion(), code, &contexte)
            .await?
            .ok_or(regle("role.inconnu"))?;
        if r.systeme {
            return Err(regle("role.livre_non_supprimable"));
        }
        if role::porteurs_actifs(unite.connexion(), r.id, &contexte).await? > 0 {
            return Err(regle("role.encore_attribue"));
        }
        let permissions = role::permissions_du_role(unite.connexion(), r.id, &contexte).await?;
        role::supprimer(unite.connexion(), r.id, &contexte).await?;
        let id = r.id.vers_chaine();
        unite.auditer(
            "role",
            &id,
            Operation::Suppression,
            Some(serde_json::json!({
                "code": r.code,
                "libelle": r.libelle,
                "permissions": codes(&permissions),
            })),
            None,
            "roles.supprimer",
        );
        unite.emettre(
            TypeEvenement::RoleSupprime,
            "role",
            &id,
            serde_json::json!({ "code": r.code, "libelle": r.libelle }),
        );
        self.noyau.terminer(unite).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_code_suit_le_libelle_sans_accent() {
        assert_eq!(
            code_depuis_libelle("  Économe / Caisse  "),
            "perso_econome_caisse"
        );
        assert_eq!(
            code_depuis_libelle("Agent d'accueil"),
            "perso_agent_d_accueil"
        );
    }

    #[test]
    fn les_permissions_inconnues_sont_refusees() {
        assert!(lire_permissions(&["grades:fly".to_owned()]).is_err());
        let lues = lire_permissions(&["students:read".to_owned(), "core:users:read".to_owned()])
            .unwrap_or_default();
        assert_eq!(
            lues,
            vec![Permission::CORE_USERS_READ, Permission::STUDENTS_READ]
        );
    }
}
