//! Utilisateurs et leurs secrets.

use gesco_core::commun::Id;
use gesco_core::core::Utilisateur;
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::depots::{booleen, horodatage_optionnel};
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

/// Empreintes d'authentification, lues et écrites séparément de l'entité (R-25).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Secrets {
    /// Empreinte Argon2id du mot de passe, au format PHC.
    pub empreinte_mot_de_passe: String,
    /// Empreinte Argon2id du code PIN, si défini.
    pub empreinte_pin: Option<String>,
}

#[derive(FromRow)]
struct Ligne {
    id: String,
    etablissement_id: String,
    identifiant: String,
    nom_affiche: String,
    actif: i64,
    doit_changer_mot_de_passe: i64,
    derniere_connexion: Option<i64>,
    echecs_consecutifs: i64,
    verrouille_jusqua: Option<i64>,
}

impl Ligne {
    fn vers_entite(self) -> Resultat<Utilisateur> {
        Ok(Utilisateur {
            id: lire_id("utilisateur", "id", &self.id)?,
            etablissement_id: lire_id("utilisateur", "etablissement_id", &self.etablissement_id)?,
            identifiant: self.identifiant,
            nom_affiche: self.nom_affiche,
            actif: booleen(self.actif),
            doit_changer_mot_de_passe: booleen(self.doit_changer_mot_de_passe),
            derniere_connexion: horodatage_optionnel(self.derniere_connexion),
            echecs_consecutifs: u32::try_from(self.echecs_consecutifs).unwrap_or(u32::MAX),
            verrouille_jusqua: horodatage_optionnel(self.verrouille_jusqua),
        })
    }
}

/// Compose `SELECT <colonnes> FROM <table> <suite>` à partir de littéraux : sqlx refuse les
/// chaînes SQL dynamiques, et les colonnes ne sont écrites qu'une fois.
macro_rules! selectionner {
    ($suite:literal) => {
        concat!("SELECT id, etablissement_id, identifiant, nom_affiche, actif, doit_changer_mot_de_passe, derniere_connexion, echecs_consecutifs, verrouille_jusqua FROM utilisateur ", $suite)
    };
}

/// Crée un utilisateur et ses secrets.
pub async fn inserer(
    cnx: &mut Connexion,
    utilisateur: &Utilisateur,
    secrets: &Secrets,
    contexte: &Contexte,
) -> Resultat<()> {
    if utilisateur.etablissement_id != contexte.etablissement_id {
        return Err(ErreurBase::Cloisonnement {
            entite: "utilisateur",
            id: utilisateur.id.vers_chaine(),
        });
    }
    sqlx::query(
        "INSERT INTO utilisateur (id, etablissement_id, identifiant, nom_affiche, actif, doit_changer_mot_de_passe,
            cree_le, cree_par, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?7, ?8, ?9)",
    )
    .bind(utilisateur.id.vers_chaine())
    .bind(utilisateur.etablissement_id.vers_chaine())
    .bind(&utilisateur.identifiant)
    .bind(&utilisateur.nom_affiche)
    .bind(i64::from(utilisateur.actif))
    .bind(i64::from(utilisateur.doit_changer_mot_de_passe))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(&mut *cnx)
    .await?;
    ecrire_secrets(cnx, utilisateur.id, secrets, contexte).await
}

/// Remplace les empreintes.
pub async fn ecrire_secrets(
    cnx: &mut Connexion,
    id: Id<Utilisateur>,
    secrets: &Secrets,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "INSERT INTO utilisateur_secret (utilisateur_id, empreinte_mot_de_passe, empreinte_pin, mis_a_jour_le)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(utilisateur_id) DO UPDATE SET empreinte_mot_de_passe = excluded.empreinte_mot_de_passe,
            empreinte_pin = excluded.empreinte_pin, mis_a_jour_le = excluded.mis_a_jour_le",
    )
    .bind(id.vers_chaine())
    .bind(&secrets.empreinte_mot_de_passe)
    .bind(&secrets.empreinte_pin)
    .bind(contexte.maintenant.millis_epoque())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Lit les empreintes d'un utilisateur de l'établissement courant.
pub async fn lire_secrets(
    cnx: &mut Connexion,
    id: Id<Utilisateur>,
    contexte: &Contexte,
) -> Resultat<Secrets> {
    let ligne: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT s.empreinte_mot_de_passe, s.empreinte_pin FROM utilisateur_secret s
         JOIN utilisateur u ON u.id = s.utilisateur_id
         WHERE s.utilisateur_id = ?1 AND u.etablissement_id = ?2 AND u.supprime_le IS NULL",
    )
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?;
    ligne
        .map(|(mdp, pin)| Secrets {
            empreinte_mot_de_passe: mdp,
            empreinte_pin: pin,
        })
        .ok_or_else(|| ErreurBase::Introuvable {
            entite: "utilisateur",
            id: id.vers_chaine(),
        })
}

/// Lit un utilisateur de l'établissement courant.
pub async fn lire(
    cnx: &mut Connexion,
    id: Id<Utilisateur>,
    contexte: &Contexte,
) -> Resultat<Utilisateur> {
    sqlx::query_as::<_, Ligne>(selectionner!(
        "WHERE id = ?1 AND etablissement_id = ?2 AND supprime_le IS NULL"
    ))
    .bind(id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .ok_or_else(|| ErreurBase::Introuvable {
        entite: "utilisateur",
        id: id.vers_chaine(),
    })?
    .vers_entite()
}

/// Recherche par identifiant de connexion.
pub async fn par_identifiant(
    cnx: &mut Connexion,
    identifiant: &str,
    contexte: &Contexte,
) -> Resultat<Option<Utilisateur>> {
    sqlx::query_as::<_, Ligne>(selectionner!(
        "WHERE identifiant = ?1 AND etablissement_id = ?2 AND supprime_le IS NULL"
    ))
    .bind(identifiant)
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_optional(cnx)
    .await?
    .map(Ligne::vers_entite)
    .transpose()
}

/// Tous les utilisateurs de l'établissement.
pub async fn lister(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<Utilisateur>> {
    sqlx::query_as::<_, Ligne>(selectionner!(
        "WHERE etablissement_id = ?1 AND supprime_le IS NULL ORDER BY nom_affiche"
    ))
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(Ligne::vers_entite)
    .collect()
}

/// Nombre d'utilisateurs actifs : zéro signifie premier lancement (compte administrateur à créer).
pub async fn compter(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<i64> {
    Ok(sqlx::query_scalar(
        "SELECT COUNT(*) FROM utilisateur WHERE etablissement_id = ?1 AND supprime_le IS NULL",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_one(cnx)
    .await?)
}

/// Met à jour les champs liés aux connexions (compteur d'échecs, verrouillage, dernière connexion).
pub async fn ecrire_etat_connexion(
    cnx: &mut Connexion,
    utilisateur: &Utilisateur,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "UPDATE utilisateur SET derniere_connexion = ?1, echecs_consecutifs = ?2, verrouille_jusqua = ?3,
            doit_changer_mot_de_passe = ?4, actif = ?5, nom_affiche = ?6, mis_a_jour_le = ?7, mis_a_jour_par = ?8
         WHERE id = ?9 AND etablissement_id = ?10 AND supprime_le IS NULL",
    )
    .bind(utilisateur.derniere_connexion.map(|h| h.millis_epoque()))
    .bind(i64::from(utilisateur.echecs_consecutifs))
    .bind(utilisateur.verrouille_jusqua.map(|h| h.millis_epoque()))
    .bind(i64::from(utilisateur.doit_changer_mot_de_passe))
    .bind(i64::from(utilisateur.actif))
    .bind(&utilisateur.nom_affiche)
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(utilisateur.id.vers_chaine())
    .bind(contexte.etablissement_id.vers_chaine())
    .execute(cnx)
    .await?;
    Ok(())
}
