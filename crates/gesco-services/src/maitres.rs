//! Comptes de maîtres (SPEC_PHASE_11_A §7) : un compte qui ne porte que le rôle Enseignant.
//! L'école décide s'il ouvre une session ; quand il en ouvre une, il ne lit que les élèves de
//! ses classes, et la garde ne lui laisse que les commandes de sa liste.

use std::collections::HashSet;

use gesco_core::commun::Id;
use gesco_core::core::{Role, RoleSysteme};
use gesco_core::scolarite::{Classe, Eleve, StatutInscription};
use gesco_db::depots::{enseignants, inscriptions};
use gesco_db::{Connexion, Contexte};

use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;
use crate::session::Session;

/// Réglage de l'école « Les maîtres saisissent leurs notes », éteint à la livraison.
pub const PARAMETRE_SAISIE: &str = gesco_core::core::parametre::SAISIE_DES_MAITRES;

/// Clé du refus d'une commande hors de la liste d'un compte de maître.
pub const REFUS_COMMANDE: &str = "maitre.commande_refusee";

/// Vrai pour un compte de maître : il porte le rôle Enseignant et aucun autre. Un compte qui
/// porte aussi un autre rôle suit ses autres droits.
pub fn est_maitre(roles: &[Role]) -> bool {
    !roles.is_empty()
        && roles
            .iter()
            .all(|r| r.systeme && r.code == RoleSysteme::Enseignant.code())
}

/// Classes d'un compte de maître, toutes années : celles de ses affectations. `None` pour tout
/// autre compte, que rien ne borne ici.
pub async fn classes(noyau: &Noyau, session: &Session) -> Resultat<Option<Vec<Id<Classe>>>> {
    let contexte = noyau.contexte(session.etablissement_id, Some(session));
    if !noyau
        .droits_de(session.utilisateur_id, &contexte)
        .await?
        .maitre
    {
        return Ok(None);
    }
    let mut cnx = noyau
        .base()
        .pool()
        .acquire()
        .await
        .map_err(gesco_db::ErreurBase::from)?;
    let mut classes: Vec<Id<Classe>> = Vec::new();
    if let Some(e) =
        enseignants::enseignant_du_compte(&mut cnx, session.utilisateur_id, &contexte).await?
    {
        for a in enseignants::affectations_de(&mut cnx, e.id, &contexte).await? {
            if !classes.contains(&a.classe_id) {
                classes.push(a.classe_id);
            }
        }
    }
    Ok(Some(classes))
}

/// Refuse à un compte de maître une classe qui n'est pas la sienne.
pub async fn verifier_classe(noyau: &Noyau, session: &Session, classe: Id<Classe>) -> Resultat<()> {
    match classes(noyau, session).await? {
        Some(siennes) if !siennes.contains(&classe) => Err(ErreurService::PorteeRefusee {
            ressource: format!("classe {classe}"),
        }),
        _ => Ok(()),
    }
}

/// Élèves qu'un compte de maître peut lire : ceux qui ont été inscrits dans une de ses classes,
/// hors brouillon. `None` pour tout autre compte.
pub async fn eleves(
    noyau: &Noyau,
    session: &Session,
    cnx: &mut Connexion,
    contexte: &Contexte,
) -> Resultat<Option<HashSet<Id<Eleve>>>> {
    let Some(siennes) = classes(noyau, session).await? else {
        return Ok(None);
    };
    let mut eleves = HashSet::new();
    for classe in siennes {
        eleves.extend(
            inscriptions::de_la_classe(cnx, classe, contexte)
                .await?
                .into_iter()
                .filter(|i| i.statut != StatutInscription::Brouillon)
                .map(|i| i.eleve_id),
        );
    }
    Ok(Some(eleves))
}

/// Refuse à un compte de maître la fiche d'un élève qui n'est pas de ses classes.
pub async fn verifier_eleve(noyau: &Noyau, session: &Session, eleve: Id<Eleve>) -> Resultat<()> {
    let contexte = noyau.contexte(session.etablissement_id, Some(session));
    let Some(siennes) = classes(noyau, session).await? else {
        return Ok(());
    };
    let mut cnx = noyau
        .base()
        .pool()
        .acquire()
        .await
        .map_err(gesco_db::ErreurBase::from)?;
    let des_siens = inscriptions::de_l_eleve(&mut cnx, eleve, &contexte)
        .await?
        .iter()
        .any(|i| i.statut != StatutInscription::Brouillon && siennes.contains(&i.classe_id));
    if des_siens {
        Ok(())
    } else {
        Err(ErreurService::PorteeRefusee {
            ressource: format!("eleve {eleve}"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gesco_db::nouvel_id;

    fn role(code: &str, systeme: bool) -> Role {
        Role {
            id: nouvel_id(),
            etablissement_id: nouvel_id(),
            code: code.to_owned(),
            libelle: code.to_owned(),
            systeme,
        }
    }

    #[test]
    fn un_compte_de_maitre_ne_porte_que_le_role_enseignant() {
        assert!(est_maitre(&[role("enseignant", true)]));
        assert!(!est_maitre(&[]));
        assert!(!est_maitre(&[role("directeur", true)]));
        assert!(!est_maitre(&[
            role("enseignant", true),
            role("surveillant", true)
        ]));
        assert!(!est_maitre(&[role("enseignant_bis", false)]));
    }
}
