//! Fiche de l'établissement pour les rapports (SPEC_PHASE_9 §8.1, §9.1) : les rubriques livrées,
//! posées là où elles manquent, celles que l'école ajoute, et leurs réponses. Une réponse
//! annuelle de l'an dernier est proposée à côté de la case, jamais recopiée sans que la
//! direction le demande. Tout est facultatif ; rien ne se supprime (R-14).

use std::collections::HashMap;

use gesco_core::commun::Id;
use gesco_core::core::fiche::{
    RUBRIQUES_LIVREES, normaliser_reponse, verifier_aide, verifier_choix, verifier_libelle,
};
use gesco_core::core::{
    AnneeScolaire, Etablissement, Operation, RubriqueFiche, ThemeRubrique, TypeReponse,
};
use gesco_db::depots::annee_scolaire;
use gesco_db::depots::fiche::{self, ValeurFiche};
use gesco_db::unite::Durabilite;
use gesco_db::{UniteDeTravail, nouvel_id};
use serde::{Deserialize, Serialize};

use crate::effets::Effet;
use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;
use crate::session::Session;
use crate::statistiques::AnneeStatistiques;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Pose les rubriques livrées qui manquent, reconnues par leur code ; rien d'existant n'est
/// modifié. Rend le nombre de rubriques ajoutées.
pub(crate) async fn poser_rubriques(
    unite: &mut UniteDeTravail<Effet>,
    etablissement_id: Id<Etablissement>,
    origine: &str,
) -> Resultat<usize> {
    let contexte = unite.contexte().clone();
    let existantes = fiche::rubriques(unite.connexion(), &contexte).await?;
    let mut ajoutees = 0;
    for (ordre, livree) in (1_i64..).zip(RUBRIQUES_LIVREES) {
        if existantes
            .iter()
            .any(|r| r.code.as_deref() == Some(livree.code))
        {
            continue;
        }
        let r = RubriqueFiche {
            id: nouvel_id(),
            etablissement_id,
            code: Some(livree.code.to_owned()),
            theme: livree.theme,
            libelle: livree.libelle.to_owned(),
            aide: String::new(),
            type_reponse: livree.type_reponse,
            choix: livree.choix.iter().map(|c| (*c).to_owned()).collect(),
            annuelle: livree.annuelle,
            livree: true,
            ordre,
            retiree_le: None,
        };
        fiche::inserer_rubrique(unite.connexion(), &r, &contexte).await?;
        ajoutees += 1;
    }
    if ajoutees > 0 {
        unite.auditer(
            "rubrique_fiche",
            &etablissement_id.vers_chaine(),
            Operation::Creation,
            None,
            Some(serde_json::json!({ "rubriques_livrees_posees": ajoutees })),
            origine,
        );
    }
    Ok(ajoutees)
}

/// Ce que l'écran demande : l'année des réponses annuelles.
#[derive(Debug, Clone, Default, Deserialize, specta::Type)]
pub struct DemandeFicheEcole {
    /// Année ; l'année courante si rien n'est donné.
    pub annee_id: Option<Id<AnneeScolaire>>,
}

/// La réponse à une rubrique.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct ReponseFiche {
    /// Rubrique.
    pub rubrique_id: Id<RubriqueFiche>,
    /// Réponse de l'année, ou réponse permanente ; vide s'il n'y en a pas.
    pub valeur: Option<String>,
    /// Pour une rubrique annuelle sans réponse cette année : la réponse de l'année d'avant,
    /// proposée à côté de la case.
    pub proposee: Option<String>,
}

/// Fiche de l'établissement : rubriques et réponses de l'année choisie.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct FicheEcole {
    /// Années qu'on peut choisir, de la plus récente à la plus ancienne.
    pub annees: Vec<AnneeStatistiques>,
    /// Année des réponses annuelles, s'il y en a une.
    pub annee_id: Option<Id<AnneeScolaire>>,
    /// Rubriques, désactivées comprises, dans l'ordre des thèmes puis de chaque thème.
    pub rubriques: Vec<RubriqueFiche>,
    /// Réponses, une par rubrique qui a une réponse ou une proposition.
    pub reponses: Vec<ReponseFiche>,
}

/// Réponse saisie dans une case.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeReponseFiche {
    /// Rubrique.
    pub rubrique_id: Id<RubriqueFiche>,
    /// Année, pour une rubrique annuelle ; ignorée pour une rubrique permanente.
    pub annee_id: Option<Id<AnneeScolaire>>,
    /// Réponse ; vide pour vider la case. Plusieurs choix : séparés par un saut de ligne.
    pub valeur: Option<String>,
}

/// Reprise des réponses de l'an dernier.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeRepriseFiche {
    /// Année qui reçoit les réponses.
    pub annee_id: Id<AnneeScolaire>,
    /// Rubriques à reprendre ; toutes celles qui ont une proposition si rien n'est donné.
    pub rubriques: Option<Vec<Id<RubriqueFiche>>>,
}

/// Rubrique ajoutée ou modifiée par l'école.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeRubriqueFiche {
    /// Rubrique de l'école à modifier ; une nouvelle si rien n'est donné.
    pub id: Option<Id<RubriqueFiche>>,
    /// Thème.
    pub theme: ThemeRubrique,
    /// Libellé.
    pub libelle: String,
    /// Aide affichée sous la case.
    pub aide: String,
    /// Type de réponse.
    pub type_reponse: TypeReponse,
    /// Choix, pour un choix ou plusieurs choix.
    pub choix: Vec<String>,
    /// Vrai si la réponse vaut pour une année.
    pub annuelle: bool,
}

/// Désactivation ou réactivation d'une rubrique.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeActivationRubrique {
    /// Rubrique.
    pub id: Id<RubriqueFiche>,
    /// Vrai pour la remettre en service.
    pub active: bool,
}

/// Année choisie et année d'avant, parmi les années de l'école.
fn annees_retenues(
    annees: &[AnneeScolaire],
    demandee: Option<Id<AnneeScolaire>>,
) -> (Option<AnneeScolaire>, Option<AnneeScolaire>) {
    let choisie = match demandee {
        Some(id) => annees.iter().find(|a| a.id == id),
        None => annees
            .iter()
            .find(|a| a.est_courante)
            .or_else(|| annees.first()),
    }
    .cloned();
    let precedente = choisie.as_ref().and_then(|c| {
        annees
            .iter()
            .filter(|a| a.debut.millis_epoque() < c.debut.millis_epoque())
            .max_by_key(|a| a.debut.millis_epoque())
            .cloned()
    });
    (choisie, precedente)
}

/// Réponses de l'année, rubrique par rubrique : la réponse de l'année pour une rubrique
/// annuelle, la réponse permanente sinon, et la proposition tirée de l'année d'avant.
fn reponses(
    rubriques: &[RubriqueFiche],
    valeurs: &[ValeurFiche],
    annee: Option<Id<AnneeScolaire>>,
    precedente: Option<Id<AnneeScolaire>>,
) -> Vec<ReponseFiche> {
    let mut par_cle: HashMap<(Id<RubriqueFiche>, Option<Id<AnneeScolaire>>), &ValeurFiche> =
        HashMap::new();
    for v in valeurs {
        par_cle.insert((v.rubrique_id, v.annee_id), v);
    }
    let lire = |r: Id<RubriqueFiche>, a: Option<Id<AnneeScolaire>>| {
        par_cle.get(&(r, a)).and_then(|v| v.valeur.clone())
    };
    rubriques
        .iter()
        .filter_map(|r| {
            let (valeur, proposee) = if r.annuelle {
                let valeur = annee.and_then(|a| lire(r.id, Some(a)));
                let proposee = if valeur.is_none() {
                    precedente.and_then(|p| lire(r.id, Some(p)))
                } else {
                    None
                };
                (valeur, proposee)
            } else {
                (lire(r.id, None), None)
            };
            (valeur.is_some() || proposee.is_some()).then_some(ReponseFiche {
                rubrique_id: r.id,
                valeur,
                proposee,
            })
        })
        .collect()
}

/// Service de la fiche de l'établissement.
pub struct ServiceFicheEcole<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceFicheEcole<'n> {
    /// Service sur le noyau ouvert.
    pub const fn nouveau(noyau: &'n Noyau) -> Self {
        Self { noyau }
    }

    /// Rubriques et réponses de l'année demandée. Une école sans année voit ses rubriques
    /// permanentes ; jamais un refus.
    pub async fn lire(
        &self,
        session: &Session,
        demande: &DemandeFicheEcole,
    ) -> Resultat<FicheEcole> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self
            .noyau
            .base()
            .pool()
            .acquire()
            .await
            .map_err(gesco_db::ErreurBase::from)?;
        let mut annees = annee_scolaire::lister(&mut cnx, &contexte).await?;
        annees.sort_by_key(|a| std::cmp::Reverse(a.debut.millis_epoque()));
        let (choisie, precedente) = annees_retenues(&annees, demande.annee_id);
        let mut rubriques = fiche::rubriques(&mut cnx, &contexte).await?;
        let valeurs = fiche::valeurs(&mut cnx, &contexte).await?;
        drop(cnx);
        rubriques.sort_by_key(|r| {
            (
                ThemeRubrique::TOUS.iter().position(|t| *t == r.theme),
                r.ordre,
            )
        });
        let reponses = reponses(
            &rubriques,
            &valeurs,
            choisie.as_ref().map(|a| a.id),
            precedente.as_ref().map(|a| a.id),
        );
        Ok(FicheEcole {
            annees: annees
                .iter()
                .map(|a| AnneeStatistiques {
                    id: a.id,
                    libelle: a.libelle.clone(),
                })
                .collect(),
            annee_id: choisie.map(|a| a.id),
            rubriques,
            reponses,
        })
    }

    /// Enregistre la réponse d'une case ; une réponse vide vide la case.
    pub async fn repondre(
        &self,
        session: &Session,
        demande: DemandeReponseFiche,
    ) -> Resultat<FicheEcole> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let rubrique = fiche::rubrique(unite.connexion(), demande.rubrique_id, &contexte).await?;
        let valeur = normaliser_reponse(
            rubrique.type_reponse,
            &rubrique.choix,
            demande.valeur.as_deref(),
        )
        .map_err(regle)?;
        let annee_id = if rubrique.annuelle {
            Some(
                demande
                    .annee_id
                    .ok_or_else(|| regle("fiche.annee_requise"))?,
            )
        } else {
            None
        };
        if let Some(a) = annee_id {
            annee_scolaire::lire(unite.connexion(), a, &contexte).await?;
        }
        let avant = fiche::ecrire_valeur(
            unite.connexion(),
            &ValeurFiche {
                rubrique_id: rubrique.id,
                annee_id,
                valeur: valeur.clone(),
            },
            &contexte,
        )
        .await?;
        if avant == valeur {
            unite.annuler().await?;
        } else {
            unite.auditer(
                "rubrique_fiche",
                &rubrique.id.vers_chaine(),
                Operation::Modification,
                Some(serde_json::json!({ "annee_id": annee_id, "valeur": avant })),
                Some(serde_json::json!({ "annee_id": annee_id, "valeur": valeur })),
                "fiche_ecole.repondre",
            );
            self.noyau.terminer(unite).await?;
        }
        self.lire(
            session,
            &DemandeFicheEcole {
                annee_id: demande.annee_id,
            },
        )
        .await
    }

    /// Recopie dans l'année les réponses annuelles de l'année d'avant, pour les rubriques qui
    /// n'ont pas encore de réponse cette année : celles que la direction a choisies, ou toutes.
    pub async fn reprendre(
        &self,
        session: &Session,
        demande: DemandeRepriseFiche,
    ) -> Resultat<FicheEcole> {
        let fiche_lue = self
            .lire(
                session,
                &DemandeFicheEcole {
                    annee_id: Some(demande.annee_id),
                },
            )
            .await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let mut reprises = 0;
        for r in &fiche_lue.reponses {
            let Some(proposee) = &r.proposee else {
                continue;
            };
            if demande
                .rubriques
                .as_ref()
                .is_some_and(|choisies| !choisies.contains(&r.rubrique_id))
            {
                continue;
            }
            fiche::ecrire_valeur(
                unite.connexion(),
                &ValeurFiche {
                    rubrique_id: r.rubrique_id,
                    annee_id: Some(demande.annee_id),
                    valeur: Some(proposee.clone()),
                },
                &contexte,
            )
            .await?;
            reprises += 1;
        }
        if reprises == 0 {
            unite.annuler().await?;
        } else {
            unite.auditer(
                "annee_scolaire",
                &demande.annee_id.vers_chaine(),
                Operation::Modification,
                None,
                Some(serde_json::json!({ "reponses_fiche_reprises": reprises })),
                "fiche_ecole.reprendre",
            );
            self.noyau.terminer(unite).await?;
        }
        self.lire(
            session,
            &DemandeFicheEcole {
                annee_id: Some(demande.annee_id),
            },
        )
        .await
    }

    /// Ajoute une rubrique de l'école, ou modifie l'une d'elles. Une rubrique livrée ne se
    /// modifie pas : elle se désactive.
    pub async fn enregistrer_rubrique(
        &self,
        session: &Session,
        demande: DemandeRubriqueFiche,
    ) -> Resultat<RubriqueFiche> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let libelle = verifier_libelle(&demande.libelle).map_err(regle)?;
        let aide = verifier_aide(&demande.aide).map_err(regle)?;
        let choix = verifier_choix(demande.type_reponse, &demande.choix).map_err(regle)?;
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let rubriques = fiche::rubriques(unite.connexion(), &contexte).await?;
        if rubriques
            .iter()
            .any(|r| Some(r.id) != demande.id && r.libelle.to_lowercase() == libelle.to_lowercase())
        {
            unite.annuler().await?;
            return Err(regle("fiche.libelle_existant"));
        }
        let apres = match demande.id {
            Some(id) => {
                let avant = rubriques
                    .iter()
                    .find(|r| r.id == id)
                    .cloned()
                    .ok_or_else(|| regle("fiche.rubrique_introuvable"))?;
                if avant.livree {
                    unite.annuler().await?;
                    return Err(regle("fiche.rubrique_livree"));
                }
                let apres = RubriqueFiche {
                    theme: demande.theme,
                    ordre: if avant.theme == demande.theme {
                        avant.ordre
                    } else {
                        suivant(&rubriques, demande.theme)
                    },
                    libelle,
                    aide,
                    type_reponse: demande.type_reponse,
                    choix,
                    annuelle: demande.annuelle,
                    ..avant.clone()
                };
                fiche::ecrire_rubrique(unite.connexion(), &apres, &contexte).await?;
                unite.auditer(
                    "rubrique_fiche",
                    &id.vers_chaine(),
                    Operation::Modification,
                    Some(serde_json::to_value(&avant)?),
                    Some(serde_json::to_value(&apres)?),
                    "fiche_ecole.enregistrer_rubrique",
                );
                apres
            }
            None => {
                let r = RubriqueFiche {
                    id: nouvel_id(),
                    etablissement_id: session.etablissement_id,
                    code: None,
                    theme: demande.theme,
                    libelle,
                    aide,
                    type_reponse: demande.type_reponse,
                    choix,
                    annuelle: demande.annuelle,
                    livree: false,
                    ordre: suivant(&rubriques, demande.theme),
                    retiree_le: None,
                };
                fiche::inserer_rubrique(unite.connexion(), &r, &contexte).await?;
                unite.auditer(
                    "rubrique_fiche",
                    &r.id.vers_chaine(),
                    Operation::Creation,
                    None,
                    Some(serde_json::to_value(&r)?),
                    "fiche_ecole.enregistrer_rubrique",
                );
                r
            }
        };
        self.noyau.terminer(unite).await?;
        Ok(apres)
    }

    /// Désactive une rubrique, livrée ou de l'école, ou la remet en service. Ses réponses
    /// restent.
    pub async fn activer_rubrique(
        &self,
        session: &Session,
        demande: DemandeActivationRubrique,
    ) -> Resultat<RubriqueFiche> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let avant = fiche::rubrique(unite.connexion(), demande.id, &contexte).await?;
        if avant.retiree_le.is_none() == demande.active {
            unite.annuler().await?;
            return Ok(avant);
        }
        let apres = RubriqueFiche {
            retiree_le: (!demande.active).then_some(contexte.maintenant),
            ..avant.clone()
        };
        fiche::ecrire_rubrique(unite.connexion(), &apres, &contexte).await?;
        unite.auditer(
            "rubrique_fiche",
            &apres.id.vers_chaine(),
            Operation::Modification,
            Some(serde_json::to_value(&avant)?),
            Some(serde_json::to_value(&apres)?),
            "fiche_ecole.activer_rubrique",
        );
        self.noyau.terminer(unite).await?;
        Ok(apres)
    }
}

/// Ordre d'une rubrique ajoutée à la fin d'un thème.
fn suivant(rubriques: &[RubriqueFiche], theme: ThemeRubrique) -> i64 {
    rubriques
        .iter()
        .filter(|r| r.theme == theme)
        .map(|r| r.ordre)
        .max()
        .unwrap_or(0)
        + 1
}
