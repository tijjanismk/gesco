//! Fin d'année (SPEC_PHASE_10 §3 et §4, UC-06) : moyenne de passage de chaque cycle, décisions
//! proposées d'après la moyenne annuelle puis validées par la direction, clôture de l'année et
//! sa réouverture tracée.
//!
//! La règle de passage vient du CAP ou de l'académie, l'école la reporte dans son réglage
//! (T-65) ; GESCO ne pose aucune décision seul (Q-30).

use gesco_core::commun::Id;
use gesco_core::core::parametre::{Parametre, ValeurParametre, hors_bornes};
use gesco_core::core::{AnneeScolaire, MachineAEtats, Operation, StatutAnnee, TypeEvenement};
use gesco_core::pedagogie::StatutBulletin;
use gesco_core::pedagogie::regle::bareme_du_cycle;
use gesco_core::scolarite::fin_annee::{cle_passage, code_de_retour, passage_par_defaut, proposer};
use gesco_core::scolarite::{
    Classe, Cycle, DecisionFinAnnee, Inscription, Niveau, StatutInscription, TypeInscription,
};
use gesco_db::Connexion;
use gesco_db::contexte::Contexte;
use gesco_db::depots::{academique, annee_scolaire, bulletins, inscriptions, parametre, personnes};
use gesco_db::nouvel_id;
use gesco_db::unite::Durabilite;
use serde::{Deserialize, Serialize};

use crate::erreur::{ErreurService, Resultat};
use crate::inscriptions::jour;
use crate::noyau::Noyau;
use crate::referentiel::ServiceReferentiel;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Motif d'abandon des brouillons restés en suspens à la clôture.
const MOTIF_BROUILLON_CLOTURE: &str = "Année clôturée sans validation de l'inscription.";

/// Moyenne de passage d'un cycle, en centièmes de point.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct PassageCycle {
    /// Cycle noté.
    pub cycle: Cycle,
    /// Barème du cycle : 1000 pour 10, 2000 pour 20.
    pub bareme: i32,
    /// Moyenne de passage réglée.
    pub passage: i32,
}

/// Décisions d'une classe : où en est chaque classe de l'année.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct BilanClasse {
    /// Classe.
    pub classe_id: Id<Classe>,
    /// Libellé.
    pub classe: String,
    /// Cycle de la classe.
    pub cycle: Cycle,
    /// Élèves inscrits jusqu'à la fin de l'année.
    pub eleves: i64,
    /// Admis au niveau suivant.
    pub admis: i64,
    /// Redoublants.
    pub redoublent: i64,
    /// Exclus.
    pub exclus: i64,
    /// Décisions encore à poser : non statué ou rattrapage en attente.
    pub en_attente: i64,
}

/// Fin de l'année courante.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct FinAnnee {
    /// Année courante.
    pub annee: AnneeScolaire,
    /// Moyenne de passage des cycles enseignés et notés.
    pub passages: Vec<PassageCycle>,
    /// Classes de l'année, dans l'ordre des niveaux.
    pub classes: Vec<BilanClasse>,
    /// Décisions encore à poser dans toute l'année.
    pub en_attente: i64,
    /// Année suivante, quand elle est préparée.
    pub suivante: Option<AnneeSuivante>,
}

/// Année suivante en préparation : ses classes et les réinscriptions qu'elle a reçues.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct AnneeSuivante {
    /// Année.
    pub annee: AnneeScolaire,
    /// Classes créées.
    pub classes: i64,
    /// Inscriptions en attente de validation au guichet.
    pub a_valider: i64,
    /// Inscriptions validées.
    pub validees: i64,
}

/// Ce qu'a fait la préparation des réinscriptions.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct BilanReinscriptions {
    /// Élèves préparés dans une classe de l'année suivante.
    pub preparees: i64,
    /// Élèves qui y avaient déjà une inscription : laissés tels quels.
    pub deja_inscrits: i64,
    /// Admis au dernier niveau que l'école enseigne : ils la quittent.
    pub sortants: i64,
    /// Niveaux où des élèves reviennent mais qui n'ont pas encore de classe.
    pub sans_classe: Vec<String>,
    /// Fin d'année mise à jour.
    pub fin: FinAnnee,
}

/// Décision d'un élève.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct LigneDecision {
    /// Inscription de l'élève.
    pub inscription_id: Id<Inscription>,
    /// Nom complet.
    pub eleve: String,
    /// Matricule de l'école.
    pub matricule: String,
    /// Moyenne annuelle du dernier bulletin, en centièmes.
    pub moyenne_annuelle: Option<i32>,
    /// Décision que propose GESCO.
    pub proposee: DecisionFinAnnee,
    /// Décision posée par la direction.
    pub decision: DecisionFinAnnee,
}

/// Décisions des élèves d'une classe.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct DecisionsClasse {
    /// Classe.
    pub classe_id: Id<Classe>,
    /// Libellé.
    pub classe: String,
    /// Cycle.
    pub cycle: Cycle,
    /// Barème du cycle ; aucun au jardin.
    pub bareme: Option<i32>,
    /// Moyenne de passage du cycle ; aucune au jardin.
    pub passage: Option<i32>,
    /// L'année est clôturée : les décisions ne changent plus.
    pub figee: bool,
    /// Élèves, par nom.
    pub lignes: Vec<LigneDecision>,
}

/// Classe désignée.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeClasseFinAnnee {
    /// Classe.
    pub classe_id: Id<Classe>,
}

/// Décision posée pour un élève.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeDecision {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Décision.
    pub decision: DecisionFinAnnee,
}

/// Moyenne de passage d'un cycle.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandePassage {
    /// Cycle noté.
    pub cycle: Cycle,
    /// Moyenne, en centièmes de point.
    pub passage: i32,
}

/// Réouverture d'une année clôturée, toujours motivée.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeReouverture {
    /// Année clôturée.
    pub annee_id: Id<AnneeScolaire>,
    /// Motif.
    pub motif: String,
}

/// Service de fin d'année.
pub struct ServiceFinAnnee<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceFinAnnee<'n> {
    /// Service sur le noyau.
    pub fn nouveau(noyau: &'n Noyau) -> Self {
        Self { noyau }
    }

    async fn connexion(&self) -> Resultat<gesco_db::ConnexionEmpruntee> {
        Ok(self
            .noyau
            .base()
            .pool()
            .acquire()
            .await
            .map_err(gesco_db::ErreurBase::from)?)
    }

    /// Fin de l'année courante : moyennes de passage et décisions de chaque classe.
    pub async fn lire(&self, session: &Session) -> Resultat<FinAnnee> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let annee = annee_scolaire::courante(&mut cnx, &contexte)
            .await?
            .ok_or(regle("fin_annee.sans_annee"))?;
        let niveaux = academique::niveaux(&mut cnx, &contexte).await?;
        let mut classes = academique::classes(&mut cnx, annee.id, &contexte).await?;
        classes.sort_by(|a, b| {
            ordre_niveau(&niveaux, a)
                .cmp(&ordre_niveau(&niveaux, b))
                .then_with(|| a.libelle.cmp(&b.libelle))
        });
        let toutes = inscriptions::de_l_annee(&mut cnx, annee.id, &contexte).await?;
        let mut bilans = Vec::with_capacity(classes.len());
        let mut cycles: Vec<Cycle> = Vec::new();
        for classe in &classes {
            let cycle = cycle_de(&niveaux, classe)?;
            if !cycles.contains(&cycle) {
                cycles.push(cycle);
            }
            let siennes: Vec<&Inscription> = toutes
                .iter()
                .filter(|i| i.classe_id == classe.id && jusqu_a_la_fin(i))
                .collect();
            let compter = |d: DecisionFinAnnee| {
                siennes.iter().filter(|i| i.decision_fin_annee == d).count() as i64
            };
            bilans.push(BilanClasse {
                classe_id: classe.id,
                classe: classe.libelle.clone(),
                cycle,
                eleves: siennes.len() as i64,
                admis: compter(DecisionFinAnnee::Admis),
                redoublent: compter(DecisionFinAnnee::Redouble),
                exclus: compter(DecisionFinAnnee::Exclu),
                en_attente: siennes
                    .iter()
                    .filter(|i| !i.decision_fin_annee.est_definitive())
                    .count() as i64,
            });
        }
        let mut passages = Vec::new();
        for cycle in Cycle::TOUS.into_iter().filter(|c| cycles.contains(c)) {
            if let (Some(bareme), Some(passage)) = (
                bareme_du_cycle(cycle),
                passage_du_cycle(&mut cnx, cycle, &contexte).await?,
            ) {
                passages.push(PassageCycle {
                    cycle,
                    bareme,
                    passage,
                });
            }
        }
        let en_attente = bilans.iter().map(|b| b.en_attente).sum();
        let suivante = match annee_suivante(&mut cnx, &annee, &contexte).await? {
            Some(suivante) => {
                let classes = academique::classes(&mut cnx, suivante.id, &contexte)
                    .await?
                    .len() as i64;
                let siennes = inscriptions::de_l_annee(&mut cnx, suivante.id, &contexte).await?;
                let compter = |statut: StatutInscription| {
                    siennes.iter().filter(|i| i.statut == statut).count() as i64
                };
                Some(AnneeSuivante {
                    classes,
                    a_valider: compter(StatutInscription::Brouillon),
                    validees: compter(StatutInscription::Validee),
                    annee: suivante,
                })
            }
            None => None,
        };
        Ok(FinAnnee {
            annee,
            passages,
            classes: bilans,
            en_attente,
            suivante,
        })
    }

    /// Crée dans l'année suivante les classes de l'année courante qu'elle n'a pas encore, avec
    /// leur niveau, leur série et leur libellé.
    pub async fn reprendre_classes(
        &self,
        session: &Session,
        annee_id: Id<AnneeScolaire>,
    ) -> Resultat<FinAnnee> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let (source, existantes) = {
            let mut cnx = self.connexion().await?;
            let cible = annee_scolaire::lire(&mut cnx, annee_id, &contexte).await?;
            let precedente = annee_precedente(&mut cnx, &cible, &contexte)
                .await?
                .ok_or(regle("fin_annee.sans_annee_precedente"))?;
            (
                academique::classes(&mut cnx, precedente.id, &contexte).await?,
                academique::classes(&mut cnx, annee_id, &contexte).await?,
            )
        };
        let referentiel = ServiceReferentiel::nouveau(self.noyau);
        for classe in source
            .iter()
            .filter(|c| !existantes.iter().any(|e| e.libelle == c.libelle))
        {
            referentiel
                .creer_classe(
                    session,
                    annee_id,
                    classe.niveau_id,
                    classe.serie_id,
                    &classe.libelle,
                )
                .await?;
        }
        self.lire(session).await
    }

    /// Prépare dans l'année suivante chaque élève admis ou redoublant de l'année qui la précède :
    /// une inscription en brouillon dans une classe du niveau où il revient, que la secrétaire
    /// valide au guichet quand le parent vient (T-66). La facture naît à cette validation.
    pub async fn preparer_reinscriptions(
        &self,
        session: &Session,
        annee_id: Id<AnneeScolaire>,
    ) -> Resultat<BilanReinscriptions> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let cible = annee_scolaire::lire(&mut cnx, annee_id, &contexte).await?;
        if !cible.est_modifiable() {
            return Err(regle("annee.cloturee"));
        }
        let source = annee_precedente(&mut cnx, &cible, &contexte)
            .await?
            .ok_or(regle("fin_annee.sans_annee_precedente"))?;
        let niveaux = academique::niveaux(&mut cnx, &contexte).await?;
        let mut classes_source = academique::classes(&mut cnx, source.id, &contexte).await?;
        classes_source.sort_by(|a, b| a.libelle.cmp(&b.libelle));
        let mut classes_cible = academique::classes(&mut cnx, cible.id, &contexte).await?;
        classes_cible.sort_by(|a, b| a.libelle.cmp(&b.libelle));
        let deja: Vec<Inscription> = inscriptions::de_l_annee(&mut cnx, cible.id, &contexte)
            .await?
            .into_iter()
            .filter(|i| i.statut.est_vivante())
            .collect();
        let eleves = personnes::eleves(&mut cnx, &contexte).await?;
        let date_inscription = jour(cible.debut)?;
        let mut a_inserer = Vec::new();
        let (mut deja_inscrits, mut sortants) = (0_i64, 0_i64);
        let mut sans_classe: Vec<String> = Vec::new();
        for origine in inscriptions::de_l_annee(&mut cnx, source.id, &contexte)
            .await?
            .into_iter()
            .filter(|i| {
                jusqu_a_la_fin(i)
                    && matches!(
                        i.decision_fin_annee,
                        DecisionFinAnnee::Admis | DecisionFinAnnee::Redouble
                    )
            })
        {
            let Some(classe) = classes_source.iter().find(|c| c.id == origine.classe_id) else {
                continue;
            };
            let Some(niveau) = niveaux.iter().find(|n| n.id == classe.niveau_id) else {
                continue;
            };
            let Some(code) = code_de_retour(&niveau.code, origine.decision_fin_annee) else {
                sortants += 1;
                continue;
            };
            let Some(destination) = niveaux.iter().find(|n| n.code == code) else {
                sortants += 1;
                continue;
            };
            let candidates: Vec<&Classe> = classes_cible
                .iter()
                .filter(|c| c.niveau_id == destination.id)
                .collect();
            if candidates.is_empty() {
                if !sans_classe.contains(&destination.libelle) {
                    sans_classe.push(destination.libelle.clone());
                }
                continue;
            }
            if deja.iter().any(|i| i.eleve_id == origine.eleve_id) {
                deja_inscrits += 1;
                continue;
            }
            if !eleves
                .iter()
                .any(|(e, _)| e.id == origine.eleve_id && e.fusionne_dans.is_none())
            {
                continue;
            }
            // La 7e B revient en 8e B : même rang parmi les classes du niveau, la dernière sinon.
            let rang = classes_source
                .iter()
                .filter(|c| c.niveau_id == classe.niveau_id)
                .position(|c| c.id == classe.id)
                .unwrap_or_default();
            let Some(classe_cible) = candidates.get(rang).or(candidates.last()) else {
                continue;
            };
            a_inserer.push(Inscription {
                id: nouvel_id(),
                etablissement_id: session.etablissement_id,
                annee_id: cible.id,
                eleve_id: origine.eleve_id,
                classe_id: classe_cible.id,
                date_inscription,
                type_inscription: TypeInscription::Reinscription,
                statut: StatutInscription::Brouillon,
                redoublant: origine.decision_fin_annee == DecisionFinAnnee::Redouble,
                decision_fin_annee: DecisionFinAnnee::NonStatue,
                classe_precedente_id: None,
                etablissement_origine: String::new(),
                motif_abandon: String::new(),
                date_depart: None,
                etablissement_destination: String::new(),
                motif_depart: String::new(),
            });
        }
        drop(cnx);
        let preparees = a_inserer.len() as i64;
        if !a_inserer.is_empty() {
            let mut unite = self
                .noyau
                .commencer(contexte.clone(), Durabilite::Critique)
                .await?;
            for inscription in &a_inserer {
                inscriptions::inserer(unite.connexion(), inscription, &contexte).await?;
                let id = inscription.id.vers_chaine();
                unite.auditer(
                    "inscription",
                    &id,
                    Operation::Creation,
                    None,
                    Some(serde_json::to_value(inscription)?),
                    "fin_annee.reinscriptions",
                );
                unite.emettre(
                    TypeEvenement::InscriptionPreparee,
                    "inscription",
                    &id,
                    serde_json::json!({
                        "eleve_id": inscription.eleve_id,
                        "classe_id": inscription.classe_id,
                        "annee_id": inscription.annee_id,
                    }),
                );
            }
            self.noyau.terminer(unite).await?;
        }
        Ok(BilanReinscriptions {
            preparees,
            deja_inscrits,
            sortants,
            sans_classe,
            fin: self.lire(session).await?,
        })
    }

    /// Décisions d'une classe, avec la proposition de GESCO pour chaque élève.
    pub async fn decisions(
        &self,
        session: &Session,
        classe_id: Id<Classe>,
    ) -> Resultat<DecisionsClasse> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let classe = academique::classe(&mut cnx, classe_id, &contexte).await?;
        let annee = annee_scolaire::lire(&mut cnx, classe.annee_id, &contexte).await?;
        let niveaux = academique::niveaux(&mut cnx, &contexte).await?;
        let cycle = cycle_de(&niveaux, &classe)?;
        let passage = passage_du_cycle(&mut cnx, cycle, &contexte).await?;
        let eleves = personnes::eleves(&mut cnx, &contexte).await?;
        let mut lignes = Vec::new();
        for inscription in inscriptions::de_l_annee(&mut cnx, classe.annee_id, &contexte)
            .await?
            .into_iter()
            .filter(|i| i.classe_id == classe.id && jusqu_a_la_fin(i))
        {
            let Some((eleve, identite)) = eleves.iter().find(|(e, _)| e.id == inscription.eleve_id)
            else {
                continue;
            };
            let moyenne_annuelle = moyenne_annuelle(&mut cnx, inscription.id, &contexte).await?;
            lignes.push(LigneDecision {
                inscription_id: inscription.id,
                eleve: identite.nom_complet(),
                matricule: eleve.matricule_interne.clone(),
                moyenne_annuelle,
                proposee: proposer(cycle, moyenne_annuelle, passage.unwrap_or_default()),
                decision: inscription.decision_fin_annee,
            });
        }
        lignes.sort_by(|a, b| a.eleve.cmp(&b.eleve));
        Ok(DecisionsClasse {
            classe_id: classe.id,
            classe: classe.libelle,
            cycle,
            bareme: bareme_du_cycle(cycle),
            passage,
            figee: annee.statut != StatutAnnee::Active,
            lignes,
        })
    }

    /// Pose la décision d'un élève ; « non statué » la retire.
    pub async fn poser(
        &self,
        session: &Session,
        demande: &DemandeDecision,
    ) -> Resultat<DecisionsClasse> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let inscription = {
            let mut cnx = self.connexion().await?;
            let inscription =
                inscriptions::lire(&mut cnx, demande.inscription_id, &contexte).await?;
            exiger_annee_active(&mut cnx, inscription.annee_id, &contexte).await?;
            inscription
        };
        if inscription.statut != StatutInscription::Validee {
            return Err(regle("fin_annee.inscription_non_validee"));
        }
        let classe_id = inscription.classe_id;
        if inscription.decision_fin_annee != demande.decision {
            let mut unite = self
                .noyau
                .commencer(contexte.clone(), Durabilite::Ordinaire)
                .await?;
            ecrire_decision(&mut unite, &inscription, demande.decision, &contexte).await?;
            self.noyau.terminer(unite).await?;
        }
        self.decisions(session, classe_id).await
    }

    /// Valide les propositions de GESCO pour les élèves de la classe encore sans décision.
    pub async fn valider_propositions(
        &self,
        session: &Session,
        classe_id: Id<Classe>,
    ) -> Resultat<DecisionsClasse> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let decisions = self.decisions(session, classe_id).await?;
        if decisions.figee {
            return Err(regle("fin_annee.annee_non_active"));
        }
        let a_poser: Vec<&LigneDecision> = decisions
            .lignes
            .iter()
            .filter(|l| {
                l.decision == DecisionFinAnnee::NonStatue
                    && l.proposee != DecisionFinAnnee::NonStatue
            })
            .collect();
        if a_poser.is_empty() {
            return Ok(decisions);
        }
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        for ligne in a_poser {
            let inscription =
                inscriptions::lire(unite.connexion(), ligne.inscription_id, &contexte).await?;
            if inscription.statut == StatutInscription::Validee {
                ecrire_decision(&mut unite, &inscription, ligne.proposee, &contexte).await?;
            }
        }
        self.noyau.terminer(unite).await?;
        self.decisions(session, classe_id).await
    }

    /// Règle la moyenne de passage d'un cycle noté.
    pub async fn regler_passage(
        &self,
        session: &Session,
        demande: &DemandePassage,
    ) -> Resultat<FinAnnee> {
        let cle = cle_passage(demande.cycle).ok_or(regle("fin_annee.passage_invalide"))?;
        let valeur = ValeurParametre::Entier(i64::from(demande.passage));
        if hors_bornes(&cle, &valeur) {
            return Err(regle("fin_annee.passage_invalide"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        let avant = parametre::lire(unite.connexion(), &cle, &contexte)
            .await?
            .ok_or(regle("parametre.inconnu"))?;
        if avant.valeur != valeur {
            let apres = Parametre {
                valeur,
                ..avant.clone()
            };
            parametre::definir(unite.connexion(), &apres, &contexte).await?;
            unite.auditer(
                "parametre",
                &cle,
                Operation::Modification,
                Some(serde_json::to_value(&avant)?),
                Some(serde_json::to_value(&apres)?),
                "fin_annee.passage",
            );
        }
        self.noyau.terminer(unite).await?;
        self.lire(session).await
    }

    /// Vérifie que l'année peut se clôturer : active, et chaque élève a sa décision.
    pub async fn verifier_cloture(
        &self,
        session: &Session,
        annee_id: Id<AnneeScolaire>,
    ) -> Resultat<AnneeScolaire> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let annee = annee_scolaire::lire(&mut cnx, annee_id, &contexte).await?;
        annee.statut.transiter(StatutAnnee::Cloturee)?;
        let en_attente = inscriptions::de_l_annee(&mut cnx, annee_id, &contexte)
            .await?
            .iter()
            .any(|i| jusqu_a_la_fin(i) && !i.decision_fin_annee.est_definitive());
        if en_attente {
            return Err(regle("fin_annee.decisions_en_attente"));
        }
        Ok(annee)
    }

    /// Clôture l'année : chaque inscription validée s'achève avec sa décision, chaque brouillon
    /// resté en suspens s'abandonne, l'année passe en lecture seule. La sauvegarde qui précède
    /// est faite par la frontière.
    pub async fn cloturer(
        &self,
        session: &Session,
        annee_id: Id<AnneeScolaire>,
    ) -> Resultat<AnneeScolaire> {
        let avant = self.verifier_cloture(session, annee_id).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let mut achevees = 0_i64;
        for inscription in inscriptions::de_l_annee(unite.connexion(), annee_id, &contexte).await? {
            let (statut, motif) = match inscription.statut {
                StatutInscription::Validee => (StatutInscription::Achevee, ""),
                StatutInscription::Brouillon => {
                    (StatutInscription::Abandonnee, MOTIF_BROUILLON_CLOTURE)
                }
                _ => continue,
            };
            inscription.statut.transiter(statut)?;
            inscriptions::ecrire_statut(
                unite.connexion(),
                inscription.id,
                statut,
                motif,
                &contexte,
            )
            .await?;
            achevees += i64::from(statut == StatutInscription::Achevee);
            unite.auditer(
                "inscription",
                &inscription.id.vers_chaine(),
                Operation::Modification,
                Some(serde_json::json!({ "statut": inscription.statut })),
                Some(serde_json::json!({ "statut": statut, "motif_abandon": motif })),
                "fin_annee.cloturer",
            );
        }
        annee_scolaire::ecrire_statut(
            unite.connexion(),
            annee_id,
            StatutAnnee::Cloturee,
            &contexte,
        )
        .await?;
        let apres = AnneeScolaire {
            statut: StatutAnnee::Cloturee,
            ..avant.clone()
        };
        let id_texte = annee_id.vers_chaine();
        unite.auditer(
            "annee_scolaire",
            &id_texte,
            Operation::Modification,
            Some(serde_json::to_value(&avant)?),
            Some(serde_json::to_value(&apres)?),
            "fin_annee.cloturer",
        );
        unite.emettre(
            TypeEvenement::AnneeScolaireCloturee,
            "annee_scolaire",
            &id_texte,
            serde_json::json!({ "libelle": apres.libelle, "inscriptions_achevees": achevees }),
        );
        self.noyau.terminer(unite).await?;
        Ok(apres)
    }

    /// Rouvre une année clôturée, avec un motif, tant qu'aucune autre année n'est ouverte : les
    /// inscriptions achevées redeviennent validées, leur décision gardée.
    pub async fn rouvrir(
        &self,
        session: &Session,
        demande: &DemandeReouverture,
    ) -> Resultat<AnneeScolaire> {
        let motif = demande.motif.trim();
        if motif.is_empty() {
            return Err(regle("fin_annee.motif_requis"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let avant = annee_scolaire::lire(unite.connexion(), demande.annee_id, &contexte).await?;
        if avant.statut != StatutAnnee::Cloturee {
            return Err(regle("fin_annee.annee_non_cloturee"));
        }
        avant.statut.transiter(StatutAnnee::Active)?;
        let autre_active = annee_scolaire::lister(unite.connexion(), &contexte)
            .await?
            .iter()
            .any(|a| a.id != avant.id && a.statut == StatutAnnee::Active);
        if autre_active {
            return Err(regle("annee.deja_active"));
        }
        for inscription in
            inscriptions::de_l_annee(unite.connexion(), demande.annee_id, &contexte).await?
        {
            if inscription.statut != StatutInscription::Achevee {
                continue;
            }
            inscription.statut.transiter(StatutInscription::Validee)?;
            inscriptions::ecrire_statut(
                unite.connexion(),
                inscription.id,
                StatutInscription::Validee,
                "",
                &contexte,
            )
            .await?;
            unite.auditer(
                "inscription",
                &inscription.id.vers_chaine(),
                Operation::Modification,
                Some(serde_json::json!({ "statut": inscription.statut })),
                Some(serde_json::json!({ "statut": StatutInscription::Validee })),
                "fin_annee.rouvrir",
            );
        }
        annee_scolaire::ecrire_statut(
            unite.connexion(),
            demande.annee_id,
            StatutAnnee::Active,
            &contexte,
        )
        .await?;
        annee_scolaire::definir_courante(unite.connexion(), demande.annee_id, &contexte).await?;
        let apres = AnneeScolaire {
            statut: StatutAnnee::Active,
            est_courante: true,
            ..avant.clone()
        };
        let id_texte = demande.annee_id.vers_chaine();
        unite.auditer(
            "annee_scolaire",
            &id_texte,
            Operation::Modification,
            Some(serde_json::to_value(&avant)?),
            Some(serde_json::to_value(&apres)?),
            "fin_annee.rouvrir",
        );
        unite.emettre(
            TypeEvenement::AnneeScolaireRouverte,
            "annee_scolaire",
            &id_texte,
            serde_json::json!({ "libelle": apres.libelle, "motif": motif }),
        );
        self.noyau.terminer(unite).await?;
        Ok(apres)
    }
}

/// Année en préparation qui suit l'année donnée, la plus proche.
async fn annee_suivante(
    cnx: &mut Connexion,
    annee: &AnneeScolaire,
    contexte: &Contexte,
) -> Resultat<Option<AnneeScolaire>> {
    Ok(annee_scolaire::lister(cnx, contexte)
        .await?
        .into_iter()
        .filter(|a| a.statut == StatutAnnee::Preparation && a.debut > annee.debut)
        .min_by_key(|a| a.debut))
}

/// Année qui précède l'année donnée, la plus proche.
async fn annee_precedente(
    cnx: &mut Connexion,
    annee: &AnneeScolaire,
    contexte: &Contexte,
) -> Resultat<Option<AnneeScolaire>> {
    Ok(annee_scolaire::lister(cnx, contexte)
        .await?
        .into_iter()
        .filter(|a| a.debut < annee.debut)
        .max_by_key(|a| a.debut))
}

/// Refuse une écriture sur une année qui n'est pas ouverte : une année clôturée ne se modifie
/// plus (ADR-012), une année en préparation n'a pas encore de décisions.
async fn exiger_annee_active(
    cnx: &mut Connexion,
    annee_id: Id<AnneeScolaire>,
    contexte: &Contexte,
) -> Resultat<()> {
    let annee = annee_scolaire::lire(cnx, annee_id, contexte).await?;
    if annee.statut != StatutAnnee::Active {
        return Err(regle("fin_annee.annee_non_active"));
    }
    Ok(())
}

/// Écrit une décision, auditée, avec son événement.
async fn ecrire_decision(
    unite: &mut gesco_db::UniteDeTravail<crate::effets::Effet>,
    inscription: &Inscription,
    decision: DecisionFinAnnee,
    contexte: &Contexte,
) -> Resultat<()> {
    inscriptions::ecrire_decision(unite.connexion(), inscription.id, decision, contexte).await?;
    let id = inscription.id.vers_chaine();
    unite.auditer(
        "inscription",
        &id,
        Operation::Modification,
        Some(serde_json::json!({ "decision_fin_annee": inscription.decision_fin_annee })),
        Some(serde_json::json!({ "decision_fin_annee": decision })),
        "fin_annee.decision",
    );
    unite.emettre(
        TypeEvenement::DecisionFinAnneePosee,
        "inscription",
        &id,
        serde_json::json!({ "decision": decision, "eleve_id": inscription.eleve_id }),
    );
    Ok(())
}

/// Élève présent jusqu'à la fin de l'année : inscription validée, ou achevée à la clôture.
fn jusqu_a_la_fin(inscription: &Inscription) -> bool {
    matches!(
        inscription.statut,
        StatutInscription::Validee | StatutInscription::Achevee
    )
}

fn cycle_de(niveaux: &[Niveau], classe: &Classe) -> Resultat<Cycle> {
    niveaux
        .iter()
        .find(|n| n.id == classe.niveau_id)
        .map(|n| n.cycle)
        .ok_or(regle("fin_annee.niveau_inconnu"))
}

fn ordre_niveau(niveaux: &[Niveau], classe: &Classe) -> i64 {
    niveaux
        .iter()
        .find(|n| n.id == classe.niveau_id)
        .map_or(i64::MAX, |n| n.ordre)
}

/// Moyenne de passage réglée d'un cycle noté.
async fn passage_du_cycle(
    cnx: &mut Connexion,
    cycle: Cycle,
    contexte: &Contexte,
) -> Resultat<Option<i32>> {
    let (Some(cle), Some(defaut)) = (cle_passage(cycle), passage_par_defaut(cycle)) else {
        return Ok(None);
    };
    let valeur = parametre::entier(cnx, &cle, i64::from(defaut), contexte).await?;
    Ok(Some(i32::try_from(valeur).unwrap_or(defaut)))
}

/// Moyenne annuelle du dernier bulletin calculé de l'élève : seul celui de la dernière période
/// de l'année la porte (T-33).
async fn moyenne_annuelle(
    cnx: &mut Connexion,
    inscription_id: Id<Inscription>,
    contexte: &Contexte,
) -> Resultat<Option<i32>> {
    Ok(bulletins::de_l_inscription(cnx, inscription_id, contexte)
        .await?
        .into_iter()
        .filter(|b| b.statut != StatutBulletin::Remplace && b.contenu.moyenne_annuelle.is_some())
        .max_by_key(|b| (b.calcule_le, b.version))
        .and_then(|b| b.contenu.moyenne_annuelle))
}
