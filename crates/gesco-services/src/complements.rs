//! Compléments pour les rapports (SPEC_PHASE_9 §8.2, §8.3, §9.2) : saisie en grille, classe par
//! classe, de l'année de recrutement et de la prise en charge par l'État ; fonctionnement et
//! pédagogie de la classe ; difficultés d'un élève, avec leur droit propre ; statut,
//! spécialité et diplôme le plus élevé du personnel. Chaque case vide reste vide ; une valeur
//! proposée se montre à côté, elle n'est jamais écrite d'elle-même.

use std::collections::HashMap;

use gesco_core::commun::{DateCivile, Horodatage, Id};
use gesco_core::core::complements::{
    ANNEE_RECRUTEMENT_MIN, DegreDifficulte, DifficulteEleve, DiplomeEleve, Fonctionnement,
    NOTE_DIFFICULTE_MAX, PedagogieClasse, Specialite, StatutRapport, TypeDifficulte,
};
use gesco_core::core::{Operation, RubriqueFiche};
use gesco_core::scolarite::{Classe, Inscription};
use gesco_db::depots::complements::{self, PremiereInscription};
use gesco_db::depots::{annee_scolaire, fiche, statistiques};
use gesco_db::unite::Durabilite;
use serde::{Deserialize, Serialize};

use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Année de recrutement proposée : celle de la première inscription dans l'école quand elle est
/// une inscription nouvelle en 1re année ; rien sinon.
pub(crate) fn annee_proposee(premiere: Option<&PremiereInscription>) -> Option<i32> {
    let p = premiere?;
    if p.type_inscription != "nouvelle" || p.code_niveau != "1" {
        return None;
    }
    DateCivile::du_jour(Horodatage::depuis_millis_epoque(p.debut)).map(|d| d.annee())
}

/// Fonctionnement et pédagogie par défaut, lus dans la fiche de l'établissement ; « simple » et
/// « classique » quand la fiche ne dit rien.
pub(crate) fn valeurs_par_defaut(
    rubriques: &[RubriqueFiche],
    valeurs: &[fiche::ValeurFiche],
) -> (Fonctionnement, PedagogieClasse) {
    let lire = |code: &str| {
        let r = rubriques.iter().find(|r| r.code.as_deref() == Some(code))?;
        valeurs
            .iter()
            .find(|v| v.rubrique_id == r.id && v.annee_id.is_none())
            .and_then(|v| v.valeur.clone())
    };
    (
        lire("fonctionnement_classes")
            .as_deref()
            .and_then(Fonctionnement::depuis_libelle)
            .unwrap_or(Fonctionnement::Simple),
        lire("pedagogie_classes")
            .as_deref()
            .and_then(PedagogieClasse::depuis_libelle)
            .unwrap_or(PedagogieClasse::Classique),
    )
}

/// Vrai pour un cycle où l'État prend des élèves en charge : le lycée et le technique.
fn prise_en_charge_possible(cycle: &str) -> bool {
    matches!(cycle, "lycee" | "technique")
}

/// Un élève de la grille d'une classe.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ComplementEleve {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Élève.
    pub eleve_id: String,
    /// Nom et prénoms.
    pub nom: String,
    /// Année de recrutement saisie.
    pub annee_recrutement: Option<i32>,
    /// Année proposée d'après la première inscription dans l'école, quand aucune n'est saisie.
    pub annee_proposee: Option<i32>,
    /// Pris en charge par l'État cette année.
    pub prise_en_charge_etat: bool,
}

/// Une classe de l'année et ses compléments.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ClasseRapports {
    /// Classe.
    pub id: Id<Classe>,
    /// Libellé.
    pub libelle: String,
    /// Fonctionnement saisi ; la valeur par défaut sinon.
    pub fonctionnement: Option<Fonctionnement>,
    /// Pédagogie saisie ; la valeur par défaut sinon.
    pub pedagogie: Option<PedagogieClasse>,
}

/// Compléments d'une année : ses classes, et les élèves de la classe choisie.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ComplementsEleves {
    /// Classes de l'année.
    pub classes: Vec<ClasseRapports>,
    /// Fonctionnement par défaut, tiré de la fiche de l'établissement.
    pub fonctionnement_defaut: Fonctionnement,
    /// Pédagogie par défaut, tirée de la fiche de l'établissement.
    pub pedagogie_defaut: PedagogieClasse,
    /// Classe choisie.
    pub classe_id: Option<Id<Classe>>,
    /// Vrai si la classe est d'un cycle où l'État prend des élèves en charge.
    pub prise_en_charge_possible: bool,
    /// Élèves de la classe choisie, par nom.
    pub eleves: Vec<ComplementEleve>,
}

/// Ce que la grille demande.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeComplementsEleves {
    /// Année ; l'année courante si rien n'est donné.
    pub annee_id: Option<Id<gesco_core::core::AnneeScolaire>>,
    /// Classe ; la première de l'année si rien n'est donné.
    pub classe_id: Option<Id<Classe>>,
}

/// Une case de la grille des élèves.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeComplementEleve {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Année de recrutement ; vide pour la vider.
    pub annee_recrutement: Option<i32>,
    /// Prise en charge par l'État.
    pub prise_en_charge_etat: bool,
}

/// Fonctionnement et pédagogie d'une classe.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeClasseRapports {
    /// Classe.
    pub classe_id: Id<Classe>,
    /// Fonctionnement ; vide pour reprendre la valeur par défaut.
    pub fonctionnement: Option<Fonctionnement>,
    /// Pédagogie ; vide pour reprendre la valeur par défaut.
    pub pedagogie: Option<PedagogieClasse>,
}

/// Difficultés d'un élève demandées.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeDifficultes {
    /// Élève.
    pub eleve_id: String,
}

/// Une difficulté saisie.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeDifficulte {
    /// Élève.
    pub eleve_id: String,
    /// Type.
    pub type_difficulte: TypeDifficulte,
    /// Degré ; vide pour retirer la difficulté.
    pub degre: Option<DegreDifficulte>,
    /// Note libre.
    pub note: String,
}

/// Un membre du personnel et ses compléments.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ComplementMembre {
    /// Membre.
    pub membre_id: String,
    /// Nom et prénoms.
    pub nom: String,
    /// Parti à ce jour.
    pub parti: bool,
    /// Statut saisi.
    pub statut: Option<StatutRapport>,
    /// Statut déduit de la fiche, qui vaut tant qu'aucun n'est saisi.
    pub statut_deduit: StatutRapport,
    /// Spécialité saisie.
    pub specialite: Option<Specialite>,
    /// Spécialité proposée d'après les matières enseignées cette année.
    pub specialite_proposee: Option<Specialite>,
    /// Diplôme le plus élevé saisi.
    pub diplome: Option<DiplomeEleve>,
    /// Diplôme proposé d'après les diplômes saisis en texte.
    pub diplome_propose: Option<DiplomeEleve>,
}

/// Compléments d'un membre du personnel saisis.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeComplementMembre {
    /// Membre.
    pub membre_id: String,
    /// Statut ; vide pour revenir au statut déduit.
    pub statut: Option<StatutRapport>,
    /// Spécialité.
    pub specialite: Option<Specialite>,
    /// Diplôme le plus élevé.
    pub diplome: Option<DiplomeEleve>,
}

/// Service des compléments pour les rapports.
pub struct ServiceComplements<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceComplements<'n> {
    /// Service sur le noyau ouvert.
    pub const fn nouveau(noyau: &'n Noyau) -> Self {
        Self { noyau }
    }

    /// Classes de l'année et élèves de la classe choisie ; une école sans année ou sans classe
    /// a une grille vide.
    pub async fn eleves(
        &self,
        session: &Session,
        demande: &DemandeComplementsEleves,
    ) -> Resultat<ComplementsEleves> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self
            .noyau
            .base()
            .pool()
            .acquire()
            .await
            .map_err(gesco_db::ErreurBase::from)?;
        let annee = match demande.annee_id {
            Some(id) => Some(annee_scolaire::lire(&mut cnx, id, &contexte).await?),
            None => annee_scolaire::courante(&mut cnx, &contexte).await?,
        };
        let rubriques = fiche::rubriques(&mut cnx, &contexte).await?;
        let valeurs = fiche::valeurs(&mut cnx, &contexte).await?;
        let (fonctionnement_defaut, pedagogie_defaut) = valeurs_par_defaut(&rubriques, &valeurs);
        let classes = match &annee {
            Some(a) => complements::classes(&mut cnx, &a.id.vers_chaine(), &contexte).await?,
            None => Vec::new(),
        };
        let choisie = demande
            .classe_id
            .map(|c| c.vers_chaine())
            .and_then(|c| classes.iter().find(|x| x.id == c))
            .or_else(|| classes.first())
            .cloned();
        let (eleves, possible, classe_id) = match &choisie {
            Some(c) => {
                let id: Id<Classe> = Id::analyser(&c.id).map_err(|_| regle("erreur.interne"))?;
                let premieres = complements::premieres_inscriptions(&mut cnx, &contexte).await?;
                let eleves = complements::eleves_de_la_classe(&mut cnx, id, &contexte)
                    .await?
                    .into_iter()
                    .map(|e| {
                        let saisie = e.annee_recrutement.and_then(|a| i32::try_from(a).ok());
                        Ok(ComplementEleve {
                            inscription_id: Id::analyser(&e.inscription_id)
                                .map_err(|_| regle("erreur.interne"))?,
                            annee_proposee: if saisie.is_none() {
                                annee_proposee(premieres.get(&e.eleve_id))
                            } else {
                                None
                            },
                            eleve_id: e.eleve_id,
                            nom: format!("{} {}", e.nom, e.prenoms).trim().to_owned(),
                            annee_recrutement: saisie,
                            prise_en_charge_etat: e.prise_en_charge_etat != 0,
                        })
                    })
                    .collect::<Resultat<Vec<_>>>()?;
                (eleves, prise_en_charge_possible(&c.cycle), Some(id))
            }
            None => (Vec::new(), false, None),
        };
        Ok(ComplementsEleves {
            classes: classes
                .iter()
                .filter_map(|c| {
                    Some(ClasseRapports {
                        id: Id::analyser(&c.id).ok()?,
                        libelle: c.libelle.clone(),
                        fonctionnement: c
                            .fonctionnement
                            .as_deref()
                            .and_then(Fonctionnement::depuis_nom),
                        pedagogie: c.pedagogie.as_deref().and_then(PedagogieClasse::depuis_nom),
                    })
                })
                .collect(),
            fonctionnement_defaut,
            pedagogie_defaut,
            classe_id,
            prise_en_charge_possible: possible,
            eleves,
        })
    }

    /// Enregistre une ligne de la grille : année de recrutement et prise en charge par l'État.
    pub async fn regler_eleve(
        &self,
        session: &Session,
        demande: DemandeComplementEleve,
    ) -> Resultat<ComplementsEleves> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        if let Some(a) = demande.annee_recrutement {
            let limite = DateCivile::du_jour(contexte.maintenant).map_or(2200, |d| d.annee() + 1);
            if a < ANNEE_RECRUTEMENT_MIN || a > limite {
                return Err(regle("complements.annee_recrutement_invalide"));
            }
        }
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let (eleve_id, classe_id, annee_avant, prise_avant) =
            complements::inscription(unite.connexion(), demande.inscription_id, &contexte).await?;
        let annee_apres = demande.annee_recrutement.map(i64::from);
        let mut change = false;
        if annee_avant != annee_apres {
            complements::ecrire_annee_recrutement(
                unite.connexion(),
                &eleve_id,
                annee_apres,
                &contexte,
            )
            .await?;
            unite.auditer(
                "eleve",
                &eleve_id,
                Operation::Modification,
                Some(serde_json::json!({ "annee_recrutement": annee_avant })),
                Some(serde_json::json!({ "annee_recrutement": annee_apres })),
                "complements.regler_eleve",
            );
            change = true;
        }
        if prise_avant != demande.prise_en_charge_etat {
            complements::ecrire_prise_en_charge(
                unite.connexion(),
                demande.inscription_id,
                demande.prise_en_charge_etat,
                &contexte,
            )
            .await?;
            unite.auditer(
                "inscription",
                &demande.inscription_id.vers_chaine(),
                Operation::Modification,
                Some(serde_json::json!({ "prise_en_charge_etat": prise_avant })),
                Some(serde_json::json!({ "prise_en_charge_etat": demande.prise_en_charge_etat })),
                "complements.regler_eleve",
            );
            change = true;
        }
        if change {
            self.noyau.terminer(unite).await?;
        } else {
            unite.annuler().await?;
        }
        let classe_id = Id::analyser(&classe_id).map_err(|_| regle("erreur.interne"))?;
        let mut cnx = self
            .noyau
            .base()
            .pool()
            .acquire()
            .await
            .map_err(gesco_db::ErreurBase::from)?;
        let (_, annee) = complements::classe(&mut cnx, classe_id, &contexte).await?;
        drop(cnx);
        self.eleves(
            session,
            &DemandeComplementsEleves {
                annee_id: Id::analyser(&annee).ok(),
                classe_id: Some(classe_id),
            },
        )
        .await
    }

    /// Enregistre le fonctionnement et la pédagogie d'une classe.
    pub async fn regler_classe(
        &self,
        session: &Session,
        demande: DemandeClasseRapports,
    ) -> Resultat<ComplementsEleves> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let (avant, annee) =
            complements::classe(unite.connexion(), demande.classe_id, &contexte).await?;
        let fonctionnement = demande.fonctionnement.map(Fonctionnement::nom);
        let pedagogie = demande.pedagogie.map(PedagogieClasse::nom);
        if avant.fonctionnement.as_deref() == fonctionnement
            && avant.pedagogie.as_deref() == pedagogie
        {
            unite.annuler().await?;
        } else {
            complements::ecrire_classe(
                unite.connexion(),
                demande.classe_id,
                fonctionnement,
                pedagogie,
                &contexte,
            )
            .await?;
            unite.auditer(
                "classe",
                &demande.classe_id.vers_chaine(),
                Operation::Modification,
                Some(serde_json::json!({ "fonctionnement": avant.fonctionnement, "pedagogie": avant.pedagogie })),
                Some(serde_json::json!({ "fonctionnement": fonctionnement, "pedagogie": pedagogie })),
                "complements.regler_classe",
            );
            self.noyau.terminer(unite).await?;
        }
        self.eleves(
            session,
            &DemandeComplementsEleves {
                annee_id: Id::analyser(&annee).ok(),
                classe_id: Some(demande.classe_id),
            },
        )
        .await
    }

    /// Difficultés notées d'un élève, retirées exceptées.
    pub async fn difficultes(
        &self,
        session: &Session,
        demande: &DemandeDifficultes,
    ) -> Resultat<Vec<DifficulteEleve>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self
            .noyau
            .base()
            .pool()
            .acquire()
            .await
            .map_err(gesco_db::ErreurBase::from)?;
        Ok(complements::difficultes(&mut cnx, &contexte)
            .await?
            .into_iter()
            .filter(|d| d.eleve_id == demande.eleve_id && d.degre.is_some())
            .collect())
    }

    /// Note, change ou retire une difficulté d'un élève.
    pub async fn regler_difficulte(
        &self,
        session: &Session,
        demande: DemandeDifficulte,
    ) -> Resultat<Vec<DifficulteEleve>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let note = demande.note.trim().to_owned();
        if note.chars().count() > NOTE_DIFFICULTE_MAX {
            return Err(regle("complements.note_invalide"));
        }
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let existe =
            complements::eleve_existe(unite.connexion(), &demande.eleve_id, &contexte).await?;
        if !existe {
            unite.annuler().await?;
            return Err(regle("complements.eleve_introuvable"));
        }
        let d = DifficulteEleve {
            etablissement_id: session.etablissement_id,
            eleve_id: demande.eleve_id.clone(),
            type_difficulte: demande.type_difficulte,
            degre: demande.degre,
            note: if demande.degre.is_some() {
                note
            } else {
                String::new()
            },
        };
        complements::ecrire_difficulte(unite.connexion(), &d, &contexte).await?;
        // L'audit ne garde ni le type ni le degré : seulement qu'une difficulté a changé.
        unite.auditer(
            "eleve",
            &demande.eleve_id,
            Operation::Modification,
            None,
            Some(serde_json::json!({ "difficulte_modifiee": true })),
            "complements.regler_difficulte",
        );
        self.noyau.terminer(unite).await?;
        self.difficultes(
            session,
            &DemandeDifficultes {
                eleve_id: demande.eleve_id,
            },
        )
        .await
    }

    /// Personnel et ses compléments : statut saisi ou déduit, spécialité et diplôme saisis ou
    /// proposés.
    pub async fn personnel(&self, session: &Session) -> Resultat<Vec<ComplementMembre>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let aujourd_hui = DateCivile::du_jour(contexte.maintenant);
        let mut cnx = self
            .noyau
            .base()
            .pool()
            .acquire()
            .await
            .map_err(gesco_db::ErreurBase::from)?;
        let membres = complements::membres(&mut cnx, &contexte).await?;
        let engagements = statistiques::engagements(&mut cnx, &contexte).await?;
        let annee = annee_scolaire::courante(&mut cnx, &contexte).await?;
        let affectations = match &annee {
            Some(a) => statistiques::affectations(&mut cnx, a.id, &contexte).await?,
            None => Vec::new(),
        };
        drop(cnx);
        let mut matieres: HashMap<&str, Vec<&str>> = HashMap::new();
        for a in &affectations {
            matieres
                .entry(a.personne_id.as_str())
                .or_default()
                .push(a.matiere.as_str());
        }
        Ok(membres
            .iter()
            .map(|m| {
                let nature = aujourd_hui.and_then(|j| {
                    crate::statistiques::nature_engagement_le(&engagements, &m.id, j)
                });
                ComplementMembre {
                    membre_id: m.id.clone(),
                    nom: format!("{} {}", m.nom, m.prenoms).trim().to_owned(),
                    parti: m
                        .parti_le
                        .as_deref()
                        .and_then(DateCivile::analyser)
                        .zip(aujourd_hui)
                        .is_some_and(|(p, j)| p <= j),
                    statut: m
                        .statut_rapport
                        .as_deref()
                        .and_then(StatutRapport::depuis_nom),
                    statut_deduit: StatutRapport::deduit(m.fonctionnaire != 0, nature.as_deref()),
                    specialite: m.specialite.as_deref().and_then(Specialite::depuis_nom),
                    specialite_proposee: Specialite::proposee(
                        matieres
                            .get(m.personne_id.as_str())
                            .cloned()
                            .unwrap_or_default(),
                    ),
                    diplome: m
                        .diplome_eleve
                        .as_deref()
                        .and_then(DiplomeEleve::depuis_nom),
                    diplome_propose: DiplomeEleve::propose(&m.diplomes),
                }
            })
            .collect())
    }

    /// Enregistre le statut, la spécialité et le diplôme le plus élevé d'un membre.
    pub async fn regler_membre(
        &self,
        session: &Session,
        demande: DemandeComplementMembre,
    ) -> Resultat<Vec<ComplementMembre>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let apres = serde_json::json!({
            "statut_rapport": demande.statut.map(StatutRapport::nom),
            "specialite": demande.specialite.map(Specialite::nom),
            "diplome_eleve": demande.diplome.map(DiplomeEleve::nom),
        });
        let n = complements::ecrire_membre(
            unite.connexion(),
            &demande.membre_id,
            demande.statut.map(StatutRapport::nom),
            demande.specialite.map(Specialite::nom),
            demande.diplome.map(DiplomeEleve::nom),
            &contexte,
        )
        .await?;
        if n == 0 {
            unite.annuler().await?;
            return Err(regle("complements.membre_introuvable"));
        }
        unite.auditer(
            "membre_personnel",
            &demande.membre_id,
            Operation::Modification,
            None,
            Some(apres),
            "complements.regler_membre",
        );
        self.noyau.terminer(unite).await?;
        self.personnel(session).await
    }
}
