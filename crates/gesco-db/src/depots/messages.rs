//! Messages aux familles (SPEC_PHASE_5 §2) : joignabilité des numéros, consentement des
//! responsables à chaque type de message.

use gesco_core::commun::Id;
use gesco_core::notification::{Consentement, EtatJoignabilite, Joignabilite, TypeMessage};
use gesco_core::scolarite::Responsable;
use sqlx::FromRow;

use crate::Connexion;
use crate::contexte::Contexte;
use crate::depots::booleen;
use crate::erreur::{ErreurBase, Resultat};
use crate::id::lire_id;

#[derive(FromRow)]
struct LigneJoignabilite {
    numero: String,
    etat: String,
    echecs_consecutifs: i64,
}

impl LigneJoignabilite {
    fn vers_entite(self) -> Resultat<Joignabilite> {
        let t = "joignabilite_numero";
        Ok(Joignabilite {
            etat: EtatJoignabilite::depuis_nom(&self.etat)
                .ok_or_else(|| ErreurBase::illisible(t, "etat", &self.etat))?,
            echecs_consecutifs: u8::try_from(self.echecs_consecutifs).map_err(|_| {
                ErreurBase::illisible(t, "echecs_consecutifs", self.echecs_consecutifs.to_string())
            })?,
            numero: self.numero,
        })
    }
}

/// Joignabilité des numéros qui en ont une ; un numéro absent n'est pas encore vérifié.
pub async fn joignabilites(
    cnx: &mut Connexion,
    numeros: &[String],
    contexte: &Contexte,
) -> Resultat<Vec<Joignabilite>> {
    let mut lues = Vec::new();
    for numero in numeros {
        if let Some(j) = joignabilite(cnx, numero, contexte).await? {
            lues.push(j);
        }
    }
    Ok(lues)
}

/// Joignabilité d'un numéro, s'il en a une.
pub async fn joignabilite(
    cnx: &mut Connexion,
    numero: &str,
    contexte: &Contexte,
) -> Resultat<Option<Joignabilite>> {
    sqlx::query_as::<_, LigneJoignabilite>(
        "SELECT numero, etat, echecs_consecutifs FROM joignabilite_numero
         WHERE etablissement_id = ?1 AND numero = ?2",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(numero)
    .fetch_optional(cnx)
    .await?
    .map(LigneJoignabilite::vers_entite)
    .transpose()
}

/// Enregistre la joignabilité d'un numéro.
pub async fn enregistrer_joignabilite(
    cnx: &mut Connexion,
    j: &Joignabilite,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "INSERT INTO joignabilite_numero (etablissement_id, numero, etat, echecs_consecutifs,
            mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT (etablissement_id, numero) DO UPDATE SET etat = excluded.etat,
            echecs_consecutifs = excluded.echecs_consecutifs,
            mis_a_jour_le = excluded.mis_a_jour_le, mis_a_jour_par = excluded.mis_a_jour_par,
            identifiant_poste = excluded.identifiant_poste",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(&j.numero)
    .bind(j.etat.nom())
    .bind(i64::from(j.echecs_consecutifs))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}

#[derive(FromRow)]
struct LigneConsentement {
    responsable_id: String,
    type_message: String,
    accepte: i64,
}

/// Consentements enregistrés d'un responsable ; un type sans ligne est accepté.
pub async fn consentements(
    cnx: &mut Connexion,
    responsable_id: Id<Responsable>,
    contexte: &Contexte,
) -> Resultat<Vec<Consentement>> {
    let t = "consentement_message";
    sqlx::query_as::<_, LigneConsentement>(
        "SELECT responsable_id, type_message, accepte FROM consentement_message
         WHERE etablissement_id = ?1 AND responsable_id = ?2",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(responsable_id.vers_chaine())
    .fetch_all(cnx)
    .await?
    .into_iter()
    .map(|l| {
        Ok(Consentement {
            responsable_id: lire_id(t, "responsable_id", &l.responsable_id)?,
            type_message: TypeMessage::depuis_nom(&l.type_message)
                .ok_or_else(|| ErreurBase::illisible(t, "type_message", &l.type_message))?,
            accepte: booleen(l.accepte),
        })
    })
    .collect()
}

/// Enregistre le consentement d'un responsable à un type de message.
pub async fn definir_consentement(
    cnx: &mut Connexion,
    c: &Consentement,
    contexte: &Contexte,
) -> Resultat<()> {
    sqlx::query(
        "INSERT INTO consentement_message (etablissement_id, responsable_id, type_message,
            accepte, mis_a_jour_le, mis_a_jour_par, identifiant_poste)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT (responsable_id, type_message) DO UPDATE SET accepte = excluded.accepte,
            mis_a_jour_le = excluded.mis_a_jour_le, mis_a_jour_par = excluded.mis_a_jour_par,
            identifiant_poste = excluded.identifiant_poste",
    )
    .bind(contexte.etablissement_id.vers_chaine())
    .bind(c.responsable_id.vers_chaine())
    .bind(c.type_message.nom())
    .bind(i64::from(c.accepte))
    .bind(contexte.maintenant.millis_epoque())
    .bind(contexte.auteur())
    .bind(&contexte.poste.0)
    .execute(cnx)
    .await?;
    Ok(())
}
