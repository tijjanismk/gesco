//! Résultats aux examens (SPEC_PHASE_9 §8.5, §9.2) : une grille par classe d'examen — inscrit,
//! présent, admis, mention —, ou les totaux par sexe pour l'école qui préfère. Quand un examen a
//! des résultats par élève dans l'année, ce détail l'emporte sur les totaux. Tout est facultatif ;
//! un résultat se range sans contradiction, rien n'est refusé pour cela.

use std::collections::HashMap;

use gesco_core::commun::Id;
use gesco_core::core::examens::{ENTREE_7E, ranger_resultat, verifier_mention};
use gesco_core::core::{AnneeScolaire, Examen, Operation, Parametre, ValeurParametre};
use gesco_core::scolarite::{Classe, Inscription, Sexe};
use gesco_db::depots::examens::{self, ResultatExamen, TotalExamen};
use gesco_db::depots::{academique, annee_scolaire, inscriptions, parametre, statistiques};
use gesco_db::unite::Durabilite;
use serde::{Deserialize, Serialize};

use crate::erreur::{ErreurService, Resultat};
use crate::inventaire::annees_retenues;
use crate::noyau::Noyau;
use crate::session::Session;
use crate::statistiques::AnneeStatistiques;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Ce que l'écran demande : l'année et la classe.
#[derive(Debug, Clone, Default, Deserialize, specta::Type)]
pub struct DemandeExamens {
    /// Année ; l'année courante si rien n'est donné.
    pub annee_id: Option<Id<AnneeScolaire>>,
    /// Classe d'examen ; la première si rien n'est donné.
    pub classe_id: Option<Id<Classe>>,
}

/// Une classe dont les élèves passent un examen.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ClasseExamen {
    /// Classe.
    pub id: Id<Classe>,
    /// Libellé.
    pub libelle: String,
    /// Examen.
    pub examen: Examen,
}

/// Le résultat d'un élève, tel que la grille le montre.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct EleveExamen {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Élève.
    pub eleve_id: String,
    /// Nom et prénoms.
    pub nom: String,
    /// Sexe, s'il est connu.
    pub sexe: Option<Sexe>,
    /// Inscrit à l'examen.
    pub inscrit: Option<bool>,
    /// Présent.
    pub present: Option<bool>,
    /// Admis.
    pub admis: Option<bool>,
    /// Mention.
    pub mention: String,
}

/// Les totaux d'un examen pour un sexe.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct TotalAffiche {
    /// Examen.
    pub examen: Examen,
    /// Sexe.
    pub sexe: Sexe,
    /// Inscrits.
    pub inscrits: Option<u32>,
    /// Présentés.
    pub presentes: Option<u32>,
    /// Admis.
    pub admis: Option<u32>,
}

/// Les résultats d'une année.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct Examens {
    /// Années qu'on peut choisir, de la plus récente à la plus ancienne.
    pub annees: Vec<AnneeStatistiques>,
    /// Année, s'il y en a une.
    pub annee_id: Option<Id<AnneeScolaire>>,
    /// Vrai si l'école présente ses élèves de 6e année à l'examen d'entrée en 7e.
    pub entree_7e: bool,
    /// Examens que passent les élèves de l'école cette année.
    pub examens: Vec<Examen>,
    /// Classes d'examen de l'année.
    pub classes: Vec<ClasseExamen>,
    /// Classe montrée.
    pub classe_id: Option<Id<Classe>>,
    /// Élèves de la classe montrée et leurs résultats.
    pub eleves: Vec<EleveExamen>,
    /// Totaux saisis de l'année.
    pub totaux: Vec<TotalAffiche>,
    /// Examens qui ont des résultats par élève : leur détail l'emporte sur les totaux.
    pub avec_detail: Vec<Examen>,
}

/// Résultat saisi d'un élève.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeResultatExamen {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Inscrit.
    pub inscrit: Option<bool>,
    /// Présent.
    pub present: Option<bool>,
    /// Admis.
    pub admis: Option<bool>,
    /// Mention.
    pub mention: String,
}

/// Totaux saisis d'un examen pour un sexe.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeTotalExamen {
    /// Année.
    pub annee_id: Id<AnneeScolaire>,
    /// Examen.
    pub examen: Examen,
    /// Sexe.
    pub sexe: Sexe,
    /// Inscrits.
    pub inscrits: Option<u32>,
    /// Présentés.
    pub presentes: Option<u32>,
    /// Admis.
    pub admis: Option<u32>,
}

/// Réglage de l'examen d'entrée en 7e.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeEntree7e {
    /// Vrai si l'école présente ses élèves de 6e année à l'examen.
    pub actif: bool,
}

/// Service des résultats aux examens.
pub struct ServiceExamens<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceExamens<'n> {
    /// Service sur le noyau ouvert.
    pub const fn nouveau(noyau: &'n Noyau) -> Self {
        Self { noyau }
    }

    /// Classes d'examen de l'année, la grille de la classe demandée, les totaux. Jamais un refus.
    pub async fn lire(&self, session: &Session, demande: &DemandeExamens) -> Resultat<Examens> {
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
        let (choisie, _) = annees_retenues(&annees, demande.annee_id);
        let entree_7e = parametre::booleen_param(&mut cnx, ENTREE_7E, false, &contexte).await?;
        let niveaux = academique::niveaux(&mut cnx, &contexte).await?;
        let (classes, inscrits, resultats, totaux) = match &choisie {
            Some(a) => (
                statistiques::classes(&mut cnx, a.id, &contexte).await?,
                statistiques::inscriptions(&mut cnx, a.id, &contexte).await?,
                examens::resultats(&mut cnx, a.id, &contexte).await?,
                examens::totaux(&mut cnx, a.id, &contexte).await?,
            ),
            None => (Vec::new(), Vec::new(), Vec::new(), Vec::new()),
        };
        drop(cnx);
        let mut classes_examen: Vec<(i64, ClasseExamen)> = Vec::new();
        for c in classes.iter().filter(|c| !c.supprimee) {
            let Some(n) = niveaux.iter().find(|n| n.id.vers_chaine() == c.niveau_id) else {
                continue;
            };
            let Some(examen) = Examen::du_niveau(n, entree_7e) else {
                continue;
            };
            let Ok(id) = Id::analyser(&c.id) else {
                continue;
            };
            classes_examen.push((
                c.ordre,
                ClasseExamen {
                    id,
                    libelle: c.libelle.clone(),
                    examen,
                },
            ));
        }
        classes_examen.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.libelle.cmp(&b.1.libelle)));
        let classes_examen: Vec<ClasseExamen> = classes_examen.into_iter().map(|c| c.1).collect();
        let mut examens_annee: Vec<Examen> = Vec::new();
        for c in &classes_examen {
            if !examens_annee.contains(&c.examen) {
                examens_annee.push(c.examen);
            }
        }
        let classe_id = demande
            .classe_id
            .filter(|id| classes_examen.iter().any(|c| c.id == *id))
            .or_else(|| classes_examen.first().map(|c| c.id));
        let par_inscription: HashMap<&str, &ResultatExamen> = resultats
            .iter()
            .map(|r| (r.inscription_id.as_str(), r))
            .collect();
        let eleves = match classe_id {
            Some(classe) => inscrits
                .iter()
                .filter(|i| i.classe_id == classe.vers_chaine() && i.statut != "brouillon")
                .filter(|i| {
                    !matches!(i.statut.as_str(), "transferee" | "abandonnee" | "radiee")
                        || par_inscription.contains_key(i.inscription_id.as_str())
                })
                .filter_map(|i| {
                    let r = par_inscription.get(i.inscription_id.as_str());
                    Some(EleveExamen {
                        inscription_id: Id::analyser(&i.inscription_id).ok()?,
                        eleve_id: i.eleve_id.clone(),
                        nom: format!("{} {}", i.prenoms, i.nom).trim().to_owned(),
                        sexe: i.sexe.as_deref().and_then(Sexe::depuis_nom),
                        inscrit: r.and_then(|r| r.inscrit),
                        present: r.and_then(|r| r.present),
                        admis: r.and_then(|r| r.admis),
                        mention: r.map(|r| r.mention.clone()).unwrap_or_default(),
                    })
                })
                .collect(),
            None => Vec::new(),
        };
        let mut avec_detail: Vec<Examen> = Vec::new();
        for r in &resultats {
            let utile = r.inscrit.is_some() || r.present.is_some() || r.admis.is_some();
            if utile && !avec_detail.contains(&r.examen) {
                avec_detail.push(r.examen);
            }
        }
        Ok(Examens {
            annees: annees
                .iter()
                .map(|a| AnneeStatistiques {
                    id: a.id,
                    libelle: a.libelle.clone(),
                })
                .collect(),
            annee_id: choisie.map(|a| a.id),
            entree_7e,
            examens: examens_annee,
            classes: classes_examen,
            classe_id,
            eleves,
            totaux: totaux
                .into_iter()
                .filter_map(|t| {
                    Some(TotalAffiche {
                        examen: t.examen,
                        sexe: Sexe::depuis_nom(&t.sexe)?,
                        inscrits: t.inscrits,
                        presentes: t.presentes,
                        admis: t.admis,
                    })
                })
                .collect(),
            avec_detail,
        })
    }

    /// Enregistre le résultat d'un élève, rangé sans contradiction ; l'examen est celui de sa
    /// classe.
    pub async fn regler_resultat(
        &self,
        session: &Session,
        demande: DemandeResultatExamen,
    ) -> Resultat<EleveExamen> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mention = verifier_mention(&demande.mention).map_err(regle)?;
        let (inscrit, present, admis) =
            ranger_resultat(demande.inscrit, demande.present, demande.admis);
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let inscription =
            inscriptions::lire(unite.connexion(), demande.inscription_id, &contexte).await?;
        let classe =
            academique::classe(unite.connexion(), inscription.classe_id, &contexte).await?;
        let niveau = academique::niveau(unite.connexion(), classe.niveau_id, &contexte).await?;
        let entree_7e =
            parametre::booleen_param(unite.connexion(), ENTREE_7E, false, &contexte).await?;
        let Some(examen) = Examen::du_niveau(&niveau, entree_7e) else {
            unite.annuler().await?;
            return Err(regle("examens.classe_sans_examen"));
        };
        let resultat = ResultatExamen {
            inscription_id: inscription.id.vers_chaine(),
            examen,
            inscrit,
            present,
            admis,
            mention,
        };
        let avant = examens::ecrire_resultat(unite.connexion(), &resultat, &contexte).await?;
        let vide = inscrit.is_none() && present.is_none() && admis.is_none();
        let inchange = match &avant {
            Some(a) => *a == resultat,
            None => vide && resultat.mention.is_empty(),
        };
        if inchange {
            unite.annuler().await?;
        } else {
            let json = |r: &ResultatExamen| {
                serde_json::json!({ "examen": r.examen.nom(), "inscrit": r.inscrit,
                    "present": r.present, "admis": r.admis, "mention": r.mention })
            };
            unite.auditer(
                "inscription",
                &inscription.id.vers_chaine(),
                Operation::Modification,
                avant.as_ref().map(json),
                Some(json(&resultat)),
                "examens.regler_resultat",
            );
            self.noyau.terminer(unite).await?;
        }
        let personne = self.nom_eleve(&inscription, &contexte).await?;
        Ok(EleveExamen {
            inscription_id: inscription.id,
            eleve_id: inscription.eleve_id.vers_chaine(),
            nom: personne.0,
            sexe: personne.1,
            inscrit,
            present,
            admis,
            mention: resultat.mention,
        })
    }

    async fn nom_eleve(
        &self,
        inscription: &Inscription,
        contexte: &gesco_db::contexte::Contexte,
    ) -> Resultat<(String, Option<Sexe>)> {
        let mut cnx = self
            .noyau
            .base()
            .pool()
            .acquire()
            .await
            .map_err(gesco_db::ErreurBase::from)?;
        let inscrits = statistiques::inscriptions(&mut cnx, inscription.annee_id, contexte).await?;
        let id = inscription.id.vers_chaine();
        Ok(inscrits
            .iter()
            .find(|i| i.inscription_id == id)
            .map(|i| {
                (
                    format!("{} {}", i.prenoms, i.nom).trim().to_owned(),
                    i.sexe.as_deref().and_then(Sexe::depuis_nom),
                )
            })
            .unwrap_or_default())
    }

    /// Enregistre les totaux d'un examen pour un sexe ; des totaux vides vident la ligne.
    pub async fn regler_total(
        &self,
        session: &Session,
        demande: DemandeTotalExamen,
    ) -> Resultat<TotalAffiche> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        annee_scolaire::lire(unite.connexion(), demande.annee_id, &contexte).await?;
        let total = TotalExamen {
            examen: demande.examen,
            sexe: demande.sexe.nom().to_owned(),
            inscrits: demande.inscrits,
            presentes: demande.presentes,
            admis: demande.admis,
        };
        let avant =
            examens::ecrire_total(unite.connexion(), demande.annee_id, &total, &contexte).await?;
        if avant.as_ref() == Some(&total) {
            unite.annuler().await?;
        } else {
            let json = |t: &TotalExamen| {
                serde_json::json!({ "examen": t.examen.nom(), "sexe": t.sexe,
                    "inscrits": t.inscrits, "presentes": t.presentes, "admis": t.admis })
            };
            unite.auditer(
                "annee_scolaire",
                &demande.annee_id.vers_chaine(),
                Operation::Modification,
                avant.as_ref().map(json),
                Some(json(&total)),
                "examens.regler_total",
            );
            self.noyau.terminer(unite).await?;
        }
        Ok(TotalAffiche {
            examen: demande.examen,
            sexe: demande.sexe,
            inscrits: demande.inscrits,
            presentes: demande.presentes,
            admis: demande.admis,
        })
    }

    /// Allume ou éteint l'examen d'entrée en 7e.
    pub async fn regler_entree_7e(
        &self,
        session: &Session,
        demande: DemandeEntree7e,
    ) -> Resultat<bool> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let avant =
            parametre::booleen_param(unite.connexion(), ENTREE_7E, false, &contexte).await?;
        if avant == demande.actif {
            unite.annuler().await?;
            return Ok(avant);
        }
        parametre::definir(
            unite.connexion(),
            &Parametre {
                etablissement_id: session.etablissement_id,
                cle: ENTREE_7E.to_owned(),
                valeur: ValeurParametre::Booleen(demande.actif),
                modifiable_par_utilisateur: true,
            },
            &contexte,
        )
        .await?;
        unite.auditer(
            "parametre",
            ENTREE_7E,
            Operation::Modification,
            Some(serde_json::json!({ "valeur": avant })),
            Some(serde_json::json!({ "valeur": demande.actif })),
            "examens.regler_entree_7e",
        );
        self.noyau.terminer(unite).await?;
        Ok(demande.actif)
    }
}
