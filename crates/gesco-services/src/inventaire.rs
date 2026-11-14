//! Inventaire de l'école (SPEC_PHASE_9 §8.4, §9.2) : les articles livrés, posés là où ils
//! manquent, ceux que l'école ajoute (les manuels surtout), et le relevé de l'année, par article
//! et, au choix, par cycle. Le relevé de l'an dernier est montré à côté de chaque case ; « Reprendre
//! l'an dernier » le recopie là où rien n'est encore saisi. Tout est facultatif ; rien ne se
//! supprime (R-14). Le droit de tenir l'inventaire est à part : l'école le donne à qui elle veut.

use gesco_core::commun::Id;
use gesco_core::core::inventaire::{
    ARTICLES_LIVRES, verifier_designation, verifier_matiere, verifier_observation,
};
use gesco_core::core::{
    AnneeScolaire, ArticleInventaire, Etablissement, FamilleInventaire, Operation,
    QuantitesInventaire, ReleveInventaire,
};
use gesco_core::scolarite::{Cycle, Niveau};
use gesco_db::depots::{academique, annee_scolaire, inventaire};
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

/// Pose les articles livrés qui manquent, reconnus par leur code ; rien d'existant n'est modifié.
/// Rend le nombre d'articles ajoutés.
pub(crate) async fn poser_articles(
    unite: &mut UniteDeTravail<Effet>,
    etablissement_id: Id<Etablissement>,
    origine: &str,
) -> Resultat<usize> {
    let contexte = unite.contexte().clone();
    let existants = inventaire::articles(unite.connexion(), &contexte).await?;
    let mut ajoutes = 0;
    for (ordre, livre) in (1_i64..).zip(ARTICLES_LIVRES) {
        if existants
            .iter()
            .any(|a| a.code.as_deref() == Some(livre.code))
        {
            continue;
        }
        let a = ArticleInventaire {
            id: nouvel_id(),
            etablissement_id,
            code: Some(livre.code.to_owned()),
            famille: livre.famille,
            designation: livre.designation.to_owned(),
            matiere: String::new(),
            niveau_id: None,
            livre: true,
            ordre,
            retire_le: None,
        };
        inventaire::inserer_article(unite.connexion(), &a, &contexte).await?;
        ajoutes += 1;
    }
    if ajoutes > 0 {
        unite.auditer(
            "article_inventaire",
            &etablissement_id.vers_chaine(),
            Operation::Creation,
            None,
            Some(serde_json::json!({ "articles_livres_poses": ajoutes })),
            origine,
        );
    }
    Ok(ajoutes)
}

/// Ce que l'écran demande : l'année et la partie de l'école.
#[derive(Debug, Clone, Default, Deserialize, specta::Type)]
pub struct DemandeInventaire {
    /// Année ; l'année courante si rien n'est donné.
    pub annee_id: Option<Id<AnneeScolaire>>,
    /// Cycle ; toute l'école si rien n'est donné.
    pub cycle: Option<Cycle>,
}

/// Un niveau qu'on peut donner à un manuel.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct NiveauInventaire {
    /// Niveau.
    pub id: Id<Niveau>,
    /// Libellé.
    pub libelle: String,
    /// Cycle du niveau.
    pub cycle: Cycle,
}

/// L'inventaire d'une année, pour une partie de l'école.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct Inventaire {
    /// Années qu'on peut choisir, de la plus récente à la plus ancienne.
    pub annees: Vec<AnneeStatistiques>,
    /// Année du relevé, s'il y en a une.
    pub annee_id: Option<Id<AnneeScolaire>>,
    /// Partie de l'école : un cycle, ou toute l'école.
    pub cycle: Option<Cycle>,
    /// Cycles des niveaux enseignés, qu'on peut choisir.
    pub cycles: Vec<Cycle>,
    /// Niveaux enseignés, pour les manuels.
    pub niveaux: Vec<NiveauInventaire>,
    /// Articles, retirés compris, par famille puis dans leur ordre.
    pub articles: Vec<ArticleInventaire>,
    /// Lignes de l'année pour cette partie de l'école.
    pub lignes: Vec<ReleveInventaire>,
    /// Lignes de l'année d'avant pour la même partie, montrées à côté des cases.
    pub an_dernier: Vec<ReleveInventaire>,
}

/// Une ligne saisie.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeReleveInventaire {
    /// Année.
    pub annee_id: Id<AnneeScolaire>,
    /// Ligne : article, cycle, quantités, observation.
    pub releve: ReleveInventaire,
}

/// Reprise des lignes de l'an dernier.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeRepriseInventaire {
    /// Année qui reçoit les lignes.
    pub annee_id: Id<AnneeScolaire>,
    /// Partie de l'école ; toute l'école si rien n'est donné.
    pub cycle: Option<Cycle>,
}

/// Article ajouté ou modifié par l'école.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeArticleInventaire {
    /// Article de l'école à modifier ; un nouveau si rien n'est donné.
    pub id: Option<Id<ArticleInventaire>>,
    /// Famille, pour un nouvel article ; ignorée pour un article existant.
    pub famille: FamilleInventaire,
    /// Désignation ou titre.
    pub designation: String,
    /// Matière d'un manuel.
    pub matiere: String,
    /// Niveau d'un manuel.
    pub niveau_id: Option<Id<Niveau>>,
}

/// Retrait ou remise en service d'un article.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeActivationArticle {
    /// Article.
    pub id: Id<ArticleInventaire>,
    /// Vrai pour le remettre en service.
    pub actif: bool,
}

/// Année choisie et année d'avant.
pub(crate) fn annees_retenues(
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

/// Service de l'inventaire.
pub struct ServiceInventaire<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceInventaire<'n> {
    /// Service sur le noyau ouvert.
    pub const fn nouveau(noyau: &'n Noyau) -> Self {
        Self { noyau }
    }

    /// Articles et relevé de l'année demandée, pour la partie demandée. Une école sans année
    /// voit son catalogue sans lignes ; jamais un refus.
    pub async fn lire(
        &self,
        session: &Session,
        demande: &DemandeInventaire,
    ) -> Resultat<Inventaire> {
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
        let mut articles = inventaire::articles(&mut cnx, &contexte).await?;
        let mut niveaux = academique::niveaux(&mut cnx, &contexte).await?;
        let de_la_partie = |r: &ReleveInventaire| r.cycle == demande.cycle;
        let lignes = match &choisie {
            Some(a) => inventaire::releves(&mut cnx, a.id, &contexte)
                .await?
                .into_iter()
                .filter(de_la_partie)
                .collect(),
            None => Vec::new(),
        };
        let an_dernier = match &precedente {
            Some(a) => inventaire::releves(&mut cnx, a.id, &contexte)
                .await?
                .into_iter()
                .filter(|r| de_la_partie(r) && !r.est_vide())
                .collect(),
            None => Vec::new(),
        };
        drop(cnx);
        articles.sort_by_key(|a| {
            (
                FamilleInventaire::TOUTES
                    .iter()
                    .position(|f| *f == a.famille),
                a.ordre,
            )
        });
        niveaux.retain(|n| n.actif);
        niveaux.sort_by_key(|n| n.ordre);
        let mut cycles: Vec<Cycle> = Vec::new();
        for n in &niveaux {
            if !cycles.contains(&n.cycle) {
                cycles.push(n.cycle);
            }
        }
        Ok(Inventaire {
            annees: annees
                .iter()
                .map(|a| AnneeStatistiques {
                    id: a.id,
                    libelle: a.libelle.clone(),
                })
                .collect(),
            annee_id: choisie.map(|a| a.id),
            cycle: demande.cycle,
            cycles,
            niveaux: niveaux
                .into_iter()
                .map(|n| NiveauInventaire {
                    id: n.id,
                    libelle: n.libelle,
                    cycle: n.cycle,
                })
                .collect(),
            articles,
            lignes,
            an_dernier,
        })
    }

    /// Enregistre une ligne du relevé ; des quantités vides vident la ligne, qui reste.
    pub async fn enregistrer_releve(
        &self,
        session: &Session,
        demande: DemandeReleveInventaire,
    ) -> Resultat<Inventaire> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        demande.releve.quantites.verifier().map_err(regle)?;
        let releve = ReleveInventaire {
            observation: verifier_observation(&demande.releve.observation).map_err(regle)?,
            ..demande.releve
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        annee_scolaire::lire(unite.connexion(), demande.annee_id, &contexte).await?;
        inventaire::article(unite.connexion(), releve.article_id, &contexte).await?;
        let avant =
            inventaire::ecrire_releve(unite.connexion(), demande.annee_id, &releve, &contexte)
                .await?;
        let inchangee = match &avant {
            Some(a) => *a == releve,
            None => releve.est_vide(),
        };
        if inchangee {
            unite.annuler().await?;
        } else {
            unite.auditer(
                "article_inventaire",
                &releve.article_id.vers_chaine(),
                Operation::Modification,
                Some(serde_json::json!({ "annee_id": demande.annee_id, "releve": avant })),
                Some(serde_json::json!({ "annee_id": demande.annee_id, "releve": releve })),
                "inventaire.enregistrer_releve",
            );
            self.noyau.terminer(unite).await?;
        }
        self.lire(
            session,
            &DemandeInventaire {
                annee_id: Some(demande.annee_id),
                cycle: releve.cycle,
            },
        )
        .await
    }

    /// Recopie dans l'année les lignes de l'année d'avant, pour la même partie de l'école, là où
    /// l'année n'a encore rien de saisi pour l'article.
    pub async fn reprendre(
        &self,
        session: &Session,
        demande: DemandeRepriseInventaire,
    ) -> Resultat<Inventaire> {
        let lu = self
            .lire(
                session,
                &DemandeInventaire {
                    annee_id: Some(demande.annee_id),
                    cycle: demande.cycle,
                },
            )
            .await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let mut reprises = 0;
        for r in &lu.an_dernier {
            let deja = lu
                .lignes
                .iter()
                .any(|l| l.article_id == r.article_id && !l.est_vide());
            let retire = lu
                .articles
                .iter()
                .any(|a| a.id == r.article_id && a.retire_le.is_some());
            if deja || retire {
                continue;
            }
            inventaire::ecrire_releve(unite.connexion(), demande.annee_id, r, &contexte).await?;
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
                Some(serde_json::json!({
                    "lignes_inventaire_reprises": reprises,
                    "cycle": demande.cycle,
                })),
                "inventaire.reprendre",
            );
            self.noyau.terminer(unite).await?;
        }
        self.lire(
            session,
            &DemandeInventaire {
                annee_id: Some(demande.annee_id),
                cycle: demande.cycle,
            },
        )
        .await
    }

    /// Ajoute un article de l'école, ou modifie l'un d'eux. Un article livré ne se renomme pas :
    /// il se retire.
    pub async fn enregistrer_article(
        &self,
        session: &Session,
        demande: DemandeArticleInventaire,
    ) -> Resultat<ArticleInventaire> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let designation = verifier_designation(&demande.designation).map_err(regle)?;
        let matiere = verifier_matiere(&demande.matiere).map_err(regle)?;
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        if let Some(n) = demande.niveau_id {
            academique::niveau(unite.connexion(), n, &contexte).await?;
        }
        let articles = inventaire::articles(unite.connexion(), &contexte).await?;
        let avant = match demande.id {
            Some(id) => Some(
                articles
                    .iter()
                    .find(|a| a.id == id)
                    .cloned()
                    .ok_or_else(|| regle("inventaire.article_introuvable"))?,
            ),
            None => None,
        };
        let famille = avant.as_ref().map_or(demande.famille, |a| a.famille);
        let (matiere, niveau_id) = if famille == FamilleInventaire::Manuel {
            (matiere, demande.niveau_id)
        } else {
            (String::new(), None)
        };
        if articles.iter().any(|a| {
            Some(a.id) != demande.id
                && a.famille == famille
                && a.designation.to_lowercase() == designation.to_lowercase()
                && a.niveau_id == niveau_id
        }) {
            unite.annuler().await?;
            return Err(regle("inventaire.designation_existante"));
        }
        let apres = match avant {
            Some(avant) => {
                if avant.livre {
                    unite.annuler().await?;
                    return Err(regle("inventaire.article_livre"));
                }
                let apres = ArticleInventaire {
                    designation,
                    matiere,
                    niveau_id,
                    ..avant.clone()
                };
                inventaire::ecrire_article(unite.connexion(), &apres, &contexte).await?;
                unite.auditer(
                    "article_inventaire",
                    &apres.id.vers_chaine(),
                    Operation::Modification,
                    Some(serde_json::to_value(&avant)?),
                    Some(serde_json::to_value(&apres)?),
                    "inventaire.enregistrer_article",
                );
                apres
            }
            None => {
                let a = ArticleInventaire {
                    id: nouvel_id(),
                    etablissement_id: session.etablissement_id,
                    code: None,
                    famille,
                    designation,
                    matiere,
                    niveau_id,
                    livre: false,
                    ordre: articles
                        .iter()
                        .filter(|a| a.famille == famille)
                        .map(|a| a.ordre)
                        .max()
                        .unwrap_or(0)
                        + 1,
                    retire_le: None,
                };
                inventaire::inserer_article(unite.connexion(), &a, &contexte).await?;
                unite.auditer(
                    "article_inventaire",
                    &a.id.vers_chaine(),
                    Operation::Creation,
                    None,
                    Some(serde_json::to_value(&a)?),
                    "inventaire.enregistrer_article",
                );
                a
            }
        };
        self.noyau.terminer(unite).await?;
        Ok(apres)
    }

    /// Retire un article, livré ou de l'école, ou le remet en service. Ses lignes restent.
    pub async fn activer_article(
        &self,
        session: &Session,
        demande: DemandeActivationArticle,
    ) -> Resultat<ArticleInventaire> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let avant = inventaire::article(unite.connexion(), demande.id, &contexte).await?;
        if avant.retire_le.is_none() == demande.actif {
            unite.annuler().await?;
            return Ok(avant);
        }
        let apres = ArticleInventaire {
            retire_le: (!demande.actif).then_some(contexte.maintenant),
            ..avant.clone()
        };
        inventaire::ecrire_article(unite.connexion(), &apres, &contexte).await?;
        unite.auditer(
            "article_inventaire",
            &apres.id.vers_chaine(),
            Operation::Modification,
            Some(serde_json::to_value(&avant)?),
            Some(serde_json::to_value(&apres)?),
            "inventaire.activer_article",
        );
        self.noyau.terminer(unite).await?;
        Ok(apres)
    }
}

/// Relevé de l'année pour une partie de l'école, article par article : pour toute l'école, la
/// somme de toutes les lignes (toute l'école et chaque cycle) ; pour un cycle, ses lignes.
pub(crate) fn releve_par_article(
    releves: &[ReleveInventaire],
    cycle: Option<Cycle>,
) -> std::collections::HashMap<Id<ArticleInventaire>, (QuantitesInventaire, Vec<String>)> {
    let mut par_article: std::collections::HashMap<
        Id<ArticleInventaire>,
        (QuantitesInventaire, Vec<String>),
    > = std::collections::HashMap::new();
    for r in releves {
        if cycle.is_some() && r.cycle != cycle {
            continue;
        }
        let entree = par_article.entry(r.article_id).or_default();
        entree.0 = entree.0.additionner(&r.quantites);
        if !r.observation.is_empty() {
            entree.1.push(r.observation.clone());
        }
    }
    par_article
}
