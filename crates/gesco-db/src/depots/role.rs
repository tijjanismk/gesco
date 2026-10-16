//! Rôles, permissions et attributions (§11 du blueprint).

use gesco_core::commun::Id;
use gesco_core::core::{Permission, Role, Utilisateur};
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::depots::booleen;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

#[derive(FromRow)]
struct Ligne {
    id: String,
    etablissement_id: String,
    code: String,
    libelle: String,
    systeme: i64,
}

impl Ligne {
    fn vers_entite(self) -> Resultat<Role> {
        Ok(Role {
            id: lire_id("role", "id", &self.id)?,
            etablissement_id: lire_id("role", "etablissement_id", &self.etablissement_id)?,
            code: self.code,
            libelle: self.libelle,
            systeme: booleen(self.systeme),
        })
    }
}

/// Crée un rôle dans l'établissement du contexte.
pub async fn inserer(cnx: &mut Connexion, role: &Role, contexte: &Contexte) -> Resultat<()> {
    if role.etablissement_id != contexte.etablissement_id {
        return Err(ErreurBase::Cloisonnement {
            entite: "role",
            id: role.id.vers_chaine(),
        });
    }
    sqlx::query(
        "INSERT INTO role (id, etablissement_id, code, libelle, systeme, cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?6, ?7, ?8)",
    )
    .bind(role.id.vers_chaine())
    .bind(role.etablissement_id.vers_chaine())
    .bind(&role.code)
    .bind(&role.libelle)
    .bind(i64::from(role.systeme))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

/// Rôles de l'établissement.
pub async fn lister(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<Role>> {
    sqlx::query_as::<_, Ligne>(
        "SELECT id, etablissement_id, code, libelle, systeme FROM role WHERE etablissement_id = ?1 AND supprime_le IS NULL ORDER BY libelle",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(Ligne::vers_entite)
    .collect()
}

/// Rôle par code.
pub async fn par_code(
    cnx: &mut Connexion,
    code: &str,
    contexte: &Contexte,
) -> Resultat<Option<Role>> {
    sqlx::query_as::<_, Ligne>(
        "SELECT id, etablissement_id, code, libelle, systeme FROM role WHERE code = ?1 AND etablissement_id = ?2 AND supprime_le IS NULL",
    )
    .bind(code)
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .map(Ligne::vers_entite)
    .transpose()
}

/// Accorde une permission à un rôle de l'établissement courant. Idempotent.
pub async fn accorder(
    cnx: &mut Connexion,
    role_id: Id<Role>,
    permission: Permission,
    contexte: &Contexte,
) -> Resultat<()> {
    verifier_role(cnx, role_id, contexte).await?;
    sqlx::query("INSERT OR IGNORE INTO role_permission (role_id, permission_code) VALUES (?1, ?2)")
        .bind(role_id.vers_chaine())
        .bind(permission.code())
        .execute(cnx)
        .await?;
    Ok(())
}

/// Retire une permission à un rôle.
pub async fn retirer(
    cnx: &mut Connexion,
    role_id: Id<Role>,
    permission: Permission,
    contexte: &Contexte,
) -> Resultat<()> {
    verifier_role(cnx, role_id, contexte).await?;
    sqlx::query("DELETE FROM role_permission WHERE role_id = ?1 AND permission_code = ?2")
        .bind(role_id.vers_chaine())
        .bind(permission.code())
        .execute(cnx)
        .await?;
    Ok(())
}

/// Permissions d'un rôle, dans l'ordre du catalogue.
pub async fn permissions_du_role(
    cnx: &mut Connexion,
    role_id: Id<Role>,
    contexte: &Contexte,
) -> Resultat<Vec<Permission>> {
    verifier_role(&mut *cnx, role_id, contexte).await?;
    let codes: Vec<String> =
        sqlx::query_scalar("SELECT permission_code FROM role_permission WHERE role_id = ?1")
            .bind(role_id.vers_chaine())
            .fetch_all(cnx)
            .await?;
    codes_vers_permissions(codes)
}

/// Vrai si un rôle de l'établissement porte déjà ce code, supprimé compris : un code n'est
/// jamais réemployé, l'audit le cite.
pub async fn code_existe(cnx: &mut Connexion, code: &str, contexte: &Contexte) -> Resultat<bool> {
    let existe: Option<i64> =
        sqlx::query_scalar("SELECT 1 FROM role WHERE code = ?1 AND etablissement_id = ?2")
            .bind(code)
            .bind(contexte.etablissement_id.vers_chaine())
            .fetch_optional(cnx)
            .await?;
    Ok(existe.is_some())
}

/// Change le libellé d'un rôle.
pub async fn renommer(
    cnx: &mut Connexion,
    role_id: Id<Role>,
    libelle: &str,
    contexte: &Contexte,
) -> Resultat<()> {
    verifier_role(&mut *cnx, role_id, contexte).await?;
    sqlx::query(
        "UPDATE role SET libelle = ?1, mis_a_jour_le = ?2, mis_a_jour_par = ?3, identifiant_poste = ?4 WHERE id = ?5",
    )
    .bind(libelle)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(role_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Remplace l'ensemble des permissions d'un rôle.
pub async fn definir_permissions(
    cnx: &mut Connexion,
    role_id: Id<Role>,
    permissions: &[Permission],
    contexte: &Contexte,
) -> Resultat<()> {
    verifier_role(&mut *cnx, role_id, contexte).await?;
    sqlx::query("DELETE FROM role_permission WHERE role_id = ?1")
        .bind(role_id.vers_chaine())
        .execute(&mut *cnx)
        .await?;
    for permission in permissions {
        sqlx::query(
            "INSERT OR IGNORE INTO role_permission (role_id, permission_code) VALUES (?1, ?2)",
        )
        .bind(role_id.vers_chaine())
        .bind(permission.code())
        .execute(&mut *cnx)
        .await?;
    }
    Ok(())
}

/// Nombre d'utilisateurs actifs qui portent ce rôle.
pub async fn porteurs_actifs(
    cnx: &mut Connexion,
    role_id: Id<Role>,
    contexte: &Contexte,
) -> Resultat<i64> {
    verifier_role(&mut *cnx, role_id, contexte).await?;
    Ok(sqlx::query_scalar(
        "SELECT COUNT(*) FROM utilisateur_role ur JOIN utilisateur u ON u.id = ur.utilisateur_id
         WHERE ur.role_id = ?1 AND u.actif = 1 AND u.supprime_le IS NULL",
    )
    .bind(role_id.vers_chaine())
    .fetch_one(cnx)
    .await?)
}

/// Utilisateurs actifs qui portent ce rôle.
pub async fn porteurs_actifs_ids(
    cnx: &mut Connexion,
    role_id: Id<Role>,
    contexte: &Contexte,
) -> Resultat<Vec<Id<Utilisateur>>> {
    verifier_role(&mut *cnx, role_id, contexte).await?;
    let ids: Vec<String> = sqlx::query_scalar(
        "SELECT u.id FROM utilisateur_role ur JOIN utilisateur u ON u.id = ur.utilisateur_id
         WHERE ur.role_id = ?1 AND u.actif = 1 AND u.supprime_le IS NULL ORDER BY u.id",
    )
    .bind(role_id.vers_chaine())
    .fetch_all(cnx)
    .await?;
    ids.iter()
        .map(|id| lire_id("utilisateur_role", "utilisateur_id", id))
        .collect()
}

/// Supprime logiquement un rôle et ses attributions restantes, qui n'ouvraient plus rien.
pub async fn supprimer(
    cnx: &mut Connexion,
    role_id: Id<Role>,
    contexte: &Contexte,
) -> Resultat<()> {
    verifier_role(&mut *cnx, role_id, contexte).await?;
    sqlx::query("DELETE FROM utilisateur_role WHERE role_id = ?1")
        .bind(role_id.vers_chaine())
        .execute(&mut *cnx)
        .await?;
    sqlx::query(
        "UPDATE role SET supprime_le = ?1, supprime_par = ?2, mis_a_jour_le = ?1, mis_a_jour_par = ?2, identifiant_poste = ?3 WHERE id = ?4",
    )
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(role_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Version de la composition livrée déjà appliquée à un rôle (SPEC_PHASE_1 §5.6).
pub async fn composition_livree(
    cnx: &mut Connexion,
    role_id: Id<Role>,
    contexte: &Contexte,
) -> Resultat<i64> {
    verifier_role(&mut *cnx, role_id, contexte).await?;
    Ok(
        sqlx::query_scalar("SELECT composition_livree FROM role WHERE id = ?1")
            .bind(role_id.vers_chaine())
            .fetch_one(cnx)
            .await?,
    )
}

/// Note la version de composition livrée appliquée à un rôle.
pub async fn noter_composition_livree(
    cnx: &mut Connexion,
    role_id: Id<Role>,
    version: i64,
    contexte: &Contexte,
) -> Resultat<()> {
    verifier_role(&mut *cnx, role_id, contexte).await?;
    sqlx::query(
        "UPDATE role SET composition_livree = ?1, mis_a_jour_le = ?2, mis_a_jour_par = ?3, identifiant_poste = ?4 WHERE id = ?5",
    )
    .bind(version)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .bind(role_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Attribue un rôle à un utilisateur du même établissement. Idempotent.
pub async fn attribuer(
    cnx: &mut Connexion,
    utilisateur_id: Id<Utilisateur>,
    role_id: Id<Role>,
    contexte: &Contexte,
) -> Resultat<()> {
    verifier_role(&mut *cnx, role_id, contexte).await?;
    crate::depots::utilisateur::lire(&mut *cnx, utilisateur_id, contexte).await?;
    sqlx::query("INSERT OR IGNORE INTO utilisateur_role (utilisateur_id, role_id) VALUES (?1, ?2)")
        .bind(utilisateur_id.vers_chaine())
        .bind(role_id.vers_chaine())
        .execute(cnx)
        .await?;
    Ok(())
}

/// Retire un rôle à un utilisateur.
pub async fn desattribuer(
    cnx: &mut Connexion,
    utilisateur_id: Id<Utilisateur>,
    role_id: Id<Role>,
    contexte: &Contexte,
) -> Resultat<()> {
    verifier_role(&mut *cnx, role_id, contexte).await?;
    crate::depots::utilisateur::lire(&mut *cnx, utilisateur_id, contexte).await?;
    sqlx::query("DELETE FROM utilisateur_role WHERE utilisateur_id = ?1 AND role_id = ?2")
        .bind(utilisateur_id.vers_chaine())
        .bind(role_id.vers_chaine())
        .execute(cnx)
        .await?;
    Ok(())
}

/// Rôles d'un utilisateur.
pub async fn roles_de(
    cnx: &mut Connexion,
    utilisateur_id: Id<Utilisateur>,
    contexte: &Contexte,
) -> Resultat<Vec<Role>> {
    sqlx::query_as::<_, Ligne>(
        "SELECT r.id, r.etablissement_id, r.code, r.libelle, r.systeme FROM role r
         JOIN utilisateur_role ur ON ur.role_id = r.id
         WHERE ur.utilisateur_id = ?1 AND r.etablissement_id = ?2 AND r.supprime_le IS NULL ORDER BY r.libelle",
    )
    .bind(utilisateur_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(Ligne::vers_entite)
    .collect()
}

/// Permissions effectives d'un utilisateur : union des permissions de ses rôles, dans
/// l'établissement courant uniquement (portée établissement, §11.3).
pub async fn permissions_de(
    cnx: &mut Connexion,
    utilisateur_id: Id<Utilisateur>,
    contexte: &Contexte,
) -> Resultat<Vec<Permission>> {
    let codes: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT rp.permission_code FROM role_permission rp
         JOIN utilisateur_role ur ON ur.role_id = rp.role_id
         JOIN role r ON r.id = rp.role_id
         JOIN utilisateur u ON u.id = ur.utilisateur_id
         WHERE ur.utilisateur_id = ?1 AND r.etablissement_id = ?2 AND u.etablissement_id = ?2
           AND r.supprime_le IS NULL AND u.supprime_le IS NULL AND u.actif = 1",
    )
    .bind(utilisateur_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?;
    codes_vers_permissions(codes)
}

/// Codes présents dans la table `permission`, pour vérifier la synchronisation avec le catalogue.
pub async fn codes_du_catalogue_persiste(cnx: &mut Connexion) -> Resultat<Vec<String>> {
    Ok(
        sqlx::query_scalar("SELECT code FROM permission ORDER BY code")
            .fetch_all(cnx)
            .await?,
    )
}

fn codes_vers_permissions(codes: Vec<String>) -> Resultat<Vec<Permission>> {
    let mut permissions = Vec::with_capacity(codes.len());
    for code in codes {
        permissions.push(
            Permission::depuis_code(&code).ok_or_else(|| {
                ErreurBase::illisible("role_permission", "permission_code", &code)
            })?,
        );
    }
    permissions.sort_by_key(|p| {
        gesco_core::core::permission::CATALOGUE
            .iter()
            .position(|c| c == p)
    });
    Ok(permissions)
}

async fn verifier_role(
    cnx: &mut Connexion,
    role_id: Id<Role>,
    contexte: &Contexte,
) -> Resultat<()> {
    let existe: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM role WHERE id = ?1 AND etablissement_id = ?2 AND supprime_le IS NULL",
    )
    .bind(role_id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?;
    existe.map(|_| ()).ok_or_else(|| ErreurBase::Introuvable {
        entite: "role",
        id: role_id.vers_chaine(),
    })
}
