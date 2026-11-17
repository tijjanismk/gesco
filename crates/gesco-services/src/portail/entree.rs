//! L'élève entre au portail (ADR « identité de l'élève ») : avec son matricule et son code, ou
//! sans code en choisissant son niveau. La session vit en mémoire sur le poste principal, sous
//! un jeton de 256 bits ; un redémarrage fait sortir tout le monde. Ni le code ni le jeton ne
//! vont au journal.

use std::collections::HashMap;
use std::sync::Mutex;

use gesco_core::bibliotheque::portail::{
    self as regles, CODES_FAUX_MAX, DUREE_SESSION_DEFAUT, jour_de, matricule_plausible,
    normaliser_matricule,
};
use gesco_core::commun::Id;
use gesco_core::core::{Etablissement, Operation};
use gesco_core::notification::consultation::{code_bien_forme, normaliser_code};
use gesco_db::depots::{parametre, portail as depot};
use gesco_db::unite::Durabilite;
use serde::{Deserialize, Serialize};

use crate::erreur::Resultat;
use crate::noyau::Noyau;

/// Ce que le portail garde d'un élève entré, le temps de sa session.
#[derive(Debug, Clone)]
pub struct SessionEleve {
    /// Élève entré avec son code ; aucun pour une lecture sans code.
    pub eleve_id: Option<String>,
    /// Prénom, pour qu'il sache qu'il ne s'est pas trompé.
    pub prenom: Option<String>,
    /// Classe de l'année en cours.
    pub classe_id: Option<String>,
    /// Libellé de la classe.
    pub classe: Option<String>,
    /// Niveau lu.
    pub niveau_id: String,
    /// Libellé du niveau.
    pub niveau: String,
    /// Série, au lycée.
    pub serie_id: Option<String>,
    /// Année en cours, pour l'avancement.
    pub annee_id: Option<String>,
    /// « Ce n'est pas mon téléphone » : la session finit avec le navigateur.
    pub ephemere: bool,
    /// Durée sans activité au bout de laquelle la session finit (millisecondes).
    pub duree_ms: i64,
    /// Dernière requête (millisecondes).
    pub derniere_activite: i64,
}

impl SessionEleve {
    /// Ce que le téléphone voit de la session.
    pub fn vue(&self) -> VueEleve {
        VueEleve {
            prenom: self.prenom.clone(),
            classe: self.classe.clone(),
            niveau: self.niveau.clone(),
            avec_code: self.eleve_id.is_some(),
        }
    }
}

/// Ce que le téléphone voit de l'élève entré.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VueEleve {
    /// Prénom, pour un élève entré avec son code.
    pub prenom: Option<String>,
    /// Classe, pour un élève entré avec son code.
    pub classe: Option<String>,
    /// Niveau lu.
    pub niveau: String,
    /// Entré avec son code : son avancement peut être gardé.
    pub avec_code: bool,
}

/// Un niveau que l'élève sans code peut choisir.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NiveauALire {
    /// Niveau.
    pub niveau_id: String,
    /// Libellé : « 9e année », ou « 10e année, série Sciences ».
    pub libelle: String,
    /// Série, au lycée.
    pub serie_id: Option<String>,
}

/// Ce que le téléphone lit en arrivant : l'école, l'élève entré s'il l'est, les niveaux de la
/// lecture sans code et ce que la direction a ouvert.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccueilPortail {
    /// Nom de l'école.
    pub ecole: String,
    /// Élève entré, s'il l'est.
    pub eleve: Option<VueEleve>,
    /// Niveaux à choisir pour lire sans code.
    pub niveaux: Vec<NiveauALire>,
    /// Le suivi est allumé : l'élève entré avec son code voit son avancement.
    pub suivi: bool,
    /// Les vidéos se regardent sur le téléphone.
    pub videos: bool,
    /// Le PDF d'une leçon s'enregistre.
    pub pdf: bool,
}

/// Matricule et code saisis.
#[derive(Debug, Clone, Deserialize)]
pub struct DemandeEntrer {
    /// Matricule.
    pub matricule: String,
    /// Code du billet.
    pub code: String,
    /// « Ce n'est pas mon téléphone ».
    #[serde(default)]
    pub pas_mon_telephone: bool,
}

/// Niveau choisi pour lire sans code.
#[derive(Debug, Clone, Deserialize)]
pub struct DemandeLireSansCode {
    /// Niveau.
    pub niveau_id: String,
    /// Série, au lycée.
    #[serde(default)]
    pub serie_id: Option<String>,
}

/// Pourquoi l'élève n'est pas entré.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RefusEntree {
    /// Matricule inconnu, code faux, élève sorti ou non réinscrit : même réponse.
    Incorrect,
    /// Trop de codes faux aujourd'hui pour ce matricule ; « Lire sans code » reste possible.
    Bloque,
}

/// Issue d'une demande d'entrée.
#[derive(Debug, Clone)]
pub enum Entree {
    /// Entré : le jeton du témoin et ce que voit le téléphone.
    Entre {
        /// Jeton de session, à poser dans le témoin.
        jeton: String,
        /// La session.
        session: Box<SessionEleve>,
    },
    /// Refusé.
    Refuse(RefusEntree),
}

/// Sessions des élèves, en mémoire.
#[derive(Debug, Default)]
pub struct SessionsPortail {
    sessions: Mutex<HashMap<String, SessionEleve>>,
}

/// Au-delà, les sessions finies sont retirées à chaque entrée.
const SESSIONS_AVANT_MENAGE: usize = 256;

impl SessionsPortail {
    /// Ouvre une session et rend son jeton.
    pub fn ouvrir(&self, session: SessionEleve, maintenant: i64) -> Resultat<String> {
        let jeton = jeton()?;
        if let Ok(mut s) = self.sessions.lock() {
            if s.len() >= SESSIONS_AVANT_MENAGE {
                s.retain(|_, v| maintenant - v.derniere_activite <= v.duree_ms);
            }
            s.insert(jeton.clone(), session);
        }
        Ok(jeton)
    }

    /// La session de ce jeton si elle vit encore ; son activité est notée.
    pub fn lire(&self, jeton: &str, maintenant: i64) -> Option<SessionEleve> {
        let mut s = self.sessions.lock().ok()?;
        let vivante = s
            .get(jeton)
            .is_some_and(|v| maintenant - v.derniere_activite <= v.duree_ms);
        if !vivante {
            s.remove(jeton);
            return None;
        }
        let session = s.get_mut(jeton)?;
        session.derniere_activite = maintenant;
        Some(session.clone())
    }

    /// Ferme une session.
    pub fn fermer(&self, jeton: &str) {
        if let Ok(mut s) = self.sessions.lock() {
            s.remove(jeton);
        }
    }

    /// Ferme toutes les sessions (portail éteint).
    pub fn tout_fermer(&self) {
        if let Ok(mut s) = self.sessions.lock() {
            s.clear();
        }
    }

    /// Nombre de sessions ouvertes.
    pub fn nombre(&self) -> usize {
        self.sessions.lock().map(|s| s.len()).unwrap_or(0)
    }
}

/// Jeton de 256 bits, en hexadécimal.
fn jeton() -> Resultat<String> {
    let octets = crate::alea::octets::<32>()?;
    Ok(octets.iter().map(|o| format!("{o:02x}")).collect())
}

/// Durée d'une session, lue dans les réglages (minutes vers millisecondes).
async fn duree_ms(noyau: &Noyau, etablissement_id: Id<Etablissement>) -> Resultat<i64> {
    let contexte = noyau.contexte(etablissement_id, None);
    let mut cnx = noyau
        .base()
        .pool()
        .acquire()
        .await
        .map_err(gesco_db::ErreurBase::from)?;
    let minutes = parametre::entier(
        &mut cnx,
        regles::DUREE_SESSION_MIN,
        DUREE_SESSION_DEFAUT,
        &contexte,
    )
    .await?;
    Ok(minutes.clamp(15, 1_440) * 60_000)
}

/// Ce que lit le téléphone en arrivant.
pub async fn accueil(
    noyau: &Noyau,
    etablissement_id: Id<Etablissement>,
    session: Option<&SessionEleve>,
) -> Resultat<AccueilPortail> {
    let contexte = noyau.contexte(etablissement_id, None);
    let mut cnx = noyau
        .base()
        .pool()
        .acquire()
        .await
        .map_err(gesco_db::ErreurBase::from)?;
    let reglages = super::ServicePortail::lire_reglages(&mut cnx, &contexte).await?;
    let ecole = depot::nom_ecole(&mut cnx, &contexte).await?;
    let niveaux = depot::niveaux_ouverts(&mut cnx, &contexte)
        .await?
        .into_iter()
        .map(|n| NiveauALire {
            libelle: match &n.serie {
                Some(serie) => format!("{}, série {serie}", n.niveau),
                None => n.niveau,
            },
            niveau_id: n.niveau_id,
            serie_id: n.serie_id,
        })
        .collect();
    Ok(AccueilPortail {
        ecole,
        eleve: session.map(SessionEleve::vue),
        niveaux,
        suivi: reglages.suivi,
        videos: reglages.videos,
        pdf: reglages.pdf,
    })
}

/// L'élève entre avec son matricule et son code. Un matricule inconnu, un code faux, un élève
/// qui n'est pas inscrit cette année reçoivent la même réponse ; cinq codes faux dans la
/// journée bloquent le matricule jusqu'au lendemain.
pub async fn entrer(
    noyau: &Noyau,
    sessions: &SessionsPortail,
    verifications: &tokio::sync::Semaphore,
    etablissement_id: Id<Etablissement>,
    demande: &DemandeEntrer,
) -> Resultat<Entree> {
    let matricule = normaliser_matricule(&demande.matricule);
    let code = normaliser_code(&demande.code);
    if !matricule_plausible(&matricule) {
        return Ok(Entree::Refuse(RefusEntree::Incorrect));
    }
    let contexte = noyau.contexte(etablissement_id, None);
    let jour = jour_de(contexte.maintenant);
    let (eleve, code_actif) = {
        let mut cnx = noyau
            .base()
            .pool()
            .acquire()
            .await
            .map_err(gesco_db::ErreurBase::from)?;
        if depot::essais_faux(&mut cnx, &matricule, &jour, &contexte).await? >= CODES_FAUX_MAX {
            return Ok(Entree::Refuse(RefusEntree::Bloque));
        }
        let eleve = depot::eleve_pour_entrer(&mut cnx, &matricule, &contexte).await?;
        let code_actif = match &eleve {
            Some(e) => depot::code_actif(&mut cnx, &e.eleve_id, &contexte).await?,
            None => None,
        };
        (eleve, code_actif)
    };
    // La vérification se fait toujours, même sans élève, pour qu'un matricule inconnu ne
    // réponde pas plus vite qu'un code faux.
    let empreinte = code_actif
        .as_ref()
        .map_or(EMPREINTE_LEURRE, |c| c.empreinte.as_str());
    let juste = {
        let _tour = verifications.acquire().await.ok();
        crate::secrets::code_eleve_correspond_a_part(&code, empreinte).await
    } && code_bien_forme(&code)
        && code_actif.is_some();
    match (juste, eleve) {
        (true, Some(eleve)) => {
            let session = SessionEleve {
                eleve_id: Some(eleve.eleve_id),
                prenom: Some(eleve.prenom),
                classe_id: Some(eleve.classe_id),
                classe: Some(eleve.classe.clone()),
                niveau: niveau_de(noyau, etablissement_id, &eleve.niveau_id).await?,
                niveau_id: eleve.niveau_id,
                serie_id: eleve.serie_id,
                annee_id: Some(eleve.annee_id),
                ephemere: demande.pas_mon_telephone,
                duree_ms: duree_ms(noyau, etablissement_id).await?,
                derniere_activite: contexte.maintenant.millis_epoque(),
            };
            let jeton = sessions.ouvrir(session.clone(), contexte.maintenant.millis_epoque())?;
            Ok(Entree::Entre {
                jeton,
                session: Box::new(session),
            })
        }
        _ => {
            compter_faux(noyau, etablissement_id, &matricule, &jour).await?;
            Ok(Entree::Refuse(RefusEntree::Incorrect))
        }
    }
}

/// Empreinte d'un code que personne n'a, vérifiée quand le matricule est inconnu.
const EMPREINTE_LEURRE: &str = "$argon2id$v=19$m=4096,t=2,p=1$Z2VzY29wb3J0YWlsMDAwMA$Wc0V0oK2c3wQq6m8tC2r2Gk0bq3y8m0bS0m6m3r2c1Q";

/// Compte un code faux ; au cinquième de la journée, le blocage va au journal d'audit (sans le
/// code).
async fn compter_faux(
    noyau: &Noyau,
    etablissement_id: Id<Etablissement>,
    matricule: &str,
    jour: &str,
) -> Resultat<()> {
    let contexte = noyau.contexte(etablissement_id, None);
    let mut unite = noyau
        .commencer(contexte.clone(), Durabilite::Ordinaire)
        .await?;
    let compte = depot::compter_essai_faux(
        unite.connexion(),
        matricule,
        jour,
        CODES_FAUX_MAX,
        &contexte,
    )
    .await?;
    if compte == CODES_FAUX_MAX {
        unite.auditer(
            "essai_code_eleve",
            matricule,
            Operation::Modification,
            None,
            Some(serde_json::json!({ "jour": jour, "essais_faux": compte })),
            "portail.matricule_bloque",
        );
    }
    noyau.terminer(unite).await?;
    Ok(())
}

/// Libellé d'un niveau.
async fn niveau_de(
    noyau: &Noyau,
    etablissement_id: Id<Etablissement>,
    niveau_id: &str,
) -> Resultat<String> {
    let contexte = noyau.contexte(etablissement_id, None);
    let mut cnx = noyau
        .base()
        .pool()
        .acquire()
        .await
        .map_err(gesco_db::ErreurBase::from)?;
    Ok(depot::niveaux_ouverts(&mut cnx, &contexte)
        .await?
        .into_iter()
        .find(|n| n.niveau_id == niveau_id)
        .map(|n| n.niveau)
        .unwrap_or_default())
}

/// Remet un code à un élève sans billet, et le rend en clair. Réservé aux essais : aucune
/// commande n'y mène, le code d'un vrai élève n'existe que sur son billet.
#[doc(hidden)]
pub async fn code_pour_essai(
    noyau: &Noyau,
    etablissement_id: Id<Etablissement>,
    eleve_id: &str,
) -> Resultat<String> {
    let code = gesco_core::notification::consultation::code_depuis_octets(&crate::alea::octets::<
        { gesco_core::notification::consultation::LONGUEUR_CODE },
    >()?);
    let empreinte = crate::secrets::empreinte_code_eleve(&code)?;
    let contexte = noyau.contexte(etablissement_id, None);
    let mut unite = noyau
        .commencer(contexte.clone(), Durabilite::Ordinaire)
        .await?;
    let id = gesco_db::nouvel_id::<()>().vers_chaine();
    depot::remplacer_code(unite.connexion(), &id, eleve_id, &empreinte, &contexte).await?;
    noyau.terminer(unite).await?;
    Ok(code)
}

/// L'élève lit sans code : il choisit un niveau (et une série) parmi ceux de l'école. Rien de
/// lui n'est gardé. `None` si le niveau n'est pas ouvert cette année.
pub async fn lire_sans_code(
    noyau: &Noyau,
    sessions: &SessionsPortail,
    etablissement_id: Id<Etablissement>,
    demande: &DemandeLireSansCode,
) -> Resultat<Option<(String, SessionEleve)>> {
    let contexte = noyau.contexte(etablissement_id, None);
    let niveau = {
        let mut cnx = noyau
            .base()
            .pool()
            .acquire()
            .await
            .map_err(gesco_db::ErreurBase::from)?;
        depot::niveaux_ouverts(&mut cnx, &contexte)
            .await?
            .into_iter()
            .find(|n| n.niveau_id == demande.niveau_id && n.serie_id == demande.serie_id)
    };
    let Some(niveau) = niveau else {
        return Ok(None);
    };
    let session = SessionEleve {
        eleve_id: None,
        prenom: None,
        classe_id: None,
        classe: None,
        niveau: match &niveau.serie {
            Some(serie) => format!("{}, série {serie}", niveau.niveau),
            None => niveau.niveau,
        },
        niveau_id: niveau.niveau_id,
        serie_id: niveau.serie_id,
        annee_id: None,
        ephemere: true,
        duree_ms: duree_ms(noyau, etablissement_id).await?,
        derniere_activite: contexte.maintenant.millis_epoque(),
    };
    let jeton = sessions.ouvrir(session.clone(), contexte.maintenant.millis_epoque())?;
    Ok(Some((jeton, session)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(duree_ms: i64, derniere: i64) -> SessionEleve {
        SessionEleve {
            eleve_id: None,
            prenom: None,
            classe_id: None,
            classe: None,
            niveau_id: "n".to_owned(),
            niveau: "9e année".to_owned(),
            serie_id: None,
            annee_id: None,
            ephemere: true,
            duree_ms,
            derniere_activite: derniere,
        }
    }

    #[test]
    fn une_session_finit_apres_sa_duree_sans_activite() -> Resultat<()> {
        let sessions = SessionsPortail::default();
        let jeton = sessions.ouvrir(session(1_000, 0), 0)?;
        assert_eq!(jeton.len(), 64);
        assert!(sessions.lire(&jeton, 900).is_some());
        // L'activité repousse la fin.
        assert!(sessions.lire(&jeton, 1_800).is_some());
        assert!(sessions.lire(&jeton, 2_900).is_none());
        assert_eq!(sessions.nombre(), 0);
        Ok(())
    }

    #[test]
    fn sortir_ferme_la_session() -> Resultat<()> {
        let sessions = SessionsPortail::default();
        let jeton = sessions.ouvrir(session(60_000, 0), 0)?;
        sessions.fermer(&jeton);
        assert!(sessions.lire(&jeton, 1).is_none());
        Ok(())
    }

    #[test]
    fn l_empreinte_leurre_se_lit() {
        assert!(argon2::PasswordHash::new(EMPREINTE_LEURRE).is_ok());
    }
}
