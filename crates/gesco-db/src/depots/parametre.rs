//! Paramètres d'établissement.

use gesco_core::core::{Parametre, ValeurParametre};
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::depots::booleen;
use crate::erreur::{ErreurBase, Resultat};

#[derive(FromRow)]
struct Ligne {
    cle: String,
    #[sqlx(rename = "type")]
    type_: String,
    valeur: String,
    modifiable_par_utilisateur: i64,
}

fn decomposer(valeur: &ValeurParametre) -> (&'static str, String) {
    match valeur {
        ValeurParametre::Texte(t) => ("texte", t.clone()),
        ValeurParametre::Entier(e) => ("entier", e.to_string()),
        ValeurParametre::Booleen(b) => {
            ("booleen", if *b { "1".to_owned() } else { "0".to_owned() })
        }
    }
}

fn recomposer(type_: &str, valeur: &str) -> Resultat<ValeurParametre> {
    match type_ {
        "texte" => Ok(ValeurParametre::Texte(valeur.to_owned())),
        "entier" => valeur
            .parse()
            .map(ValeurParametre::Entier)
            .map_err(|e| ErreurBase::illisible("parametre", "valeur", e)),
        "booleen" => Ok(ValeurParametre::Booleen(valeur == "1")),
        autre => Err(ErreurBase::illisible("parametre", "type", autre)),
    }
}

/// Écrit ou remplace un paramètre.
pub async fn definir(
    cnx: &mut Connexion,
    parametre: &Parametre,
    contexte: &Contexte,
) -> Resultat<()> {
    if parametre.etablissement_id != contexte.etablissement_id {
        return Err(ErreurBase::Cloisonnement {
            entite: "parametre",
            id: parametre.cle.clone(),
        });
    }
    let (type_, valeur) = decomposer(&parametre.valeur);
    sqlx::query(
        "INSERT INTO parametre (etablissement_id, cle, type, valeur, modifiable_par_utilisateur, mis_a_jour_le, mis_a_jour_par)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(etablissement_id, cle) DO UPDATE SET type = excluded.type, valeur = excluded.valeur,
            modifiable_par_utilisateur = excluded.modifiable_par_utilisateur,
            mis_a_jour_le = excluded.mis_a_jour_le, mis_a_jour_par = excluded.mis_a_jour_par",
    )
    .bind(parametre.etablissement_id.vers_chaine())
    .bind(&parametre.cle)
    .bind(type_)
    .bind(valeur)
    .bind(i64::from(parametre.modifiable_par_utilisateur))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .execute(cnx)
    .await?;
    Ok(())
}

/// Pose les paramètres absents sans toucher aux existants.
pub async fn definir_si_absent(
    cnx: &mut Connexion,
    parametres: &[Parametre],
    contexte: &Contexte,
) -> Resultat<()> {
    for p in parametres {
        if lire(&mut *cnx, &p.cle, contexte).await?.is_none() {
            definir(&mut *cnx, p, contexte).await?;
        }
    }
    Ok(())
}

/// Lit un paramètre.
pub async fn lire(
    cnx: &mut Connexion,
    cle: &str,
    contexte: &Contexte,
) -> Resultat<Option<Parametre>> {
    let ligne = sqlx::query_as::<_, Ligne>(
        "SELECT cle, type, valeur, modifiable_par_utilisateur FROM parametre WHERE etablissement_id = ?1 AND cle = ?2",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(cle)
    .fetch_optional(cnx)
    .await?;
    ligne
        .map(|l| {
            Ok(Parametre {
                etablissement_id: contexte.etablissement_id,
                cle: l.cle,
                valeur: recomposer(&l.type_, &l.valeur)?,
                modifiable_par_utilisateur: booleen(l.modifiable_par_utilisateur),
            })
        })
        .transpose()
}

/// Valeur entière d'un paramètre, ou la valeur de repli si absent ou d'un autre type.
pub async fn entier(
    cnx: &mut Connexion,
    cle: &str,
    repli: i64,
    contexte: &Contexte,
) -> Resultat<i64> {
    Ok(match lire(cnx, cle, contexte).await? {
        Some(Parametre {
            valeur: ValeurParametre::Entier(e),
            ..
        }) => e,
        _ => repli,
    })
}

/// Valeur booléenne d'un paramètre, ou la valeur de repli.
pub async fn booleen_param(
    cnx: &mut Connexion,
    cle: &str,
    repli: bool,
    contexte: &Contexte,
) -> Resultat<bool> {
    Ok(match lire(cnx, cle, contexte).await? {
        Some(Parametre {
            valeur: ValeurParametre::Booleen(b),
            ..
        }) => b,
        _ => repli,
    })
}

/// Valeur texte d'un paramètre, ou la valeur de repli.
pub async fn texte(
    cnx: &mut Connexion,
    cle: &str,
    repli: &str,
    contexte: &Contexte,
) -> Resultat<String> {
    Ok(match lire(cnx, cle, contexte).await? {
        Some(Parametre {
            valeur: ValeurParametre::Texte(t),
            ..
        }) => t,
        _ => repli.to_owned(),
    })
}

/// Tous les paramètres de l'établissement.
pub async fn lister(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Vec<Parametre>> {
    let lignes = sqlx::query_as::<_, Ligne>(
        "SELECT cle, type, valeur, modifiable_par_utilisateur FROM parametre WHERE etablissement_id = ?1 ORDER BY cle",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .fetch_all(cnx)
    .await?;
    lignes
        .into_iter()
        .map(|l| {
            Ok(Parametre {
                etablissement_id: contexte.etablissement_id,
                cle: l.cle,
                valeur: recomposer(&l.type_, &l.valeur)?,
                modifiable_par_utilisateur: booleen(l.modifiable_par_utilisateur),
            })
        })
        .collect()
}
