//! Observations du jardin d'enfants (SPEC_PHASE_11_B §4.2, §6) : la direction, ou le maître de
//! la classe quand l'école ouvre la saisie aux maîtres, pose pour chaque enfant le niveau de
//! chaque compétence de sa section, case par case ou toute une colonne d'un coup, et écrit le
//! commentaire d'un domaine et l'appréciation générale. Rien ne bloque et rien ne signale ce qui
//! manque (RB-1, RB-2) : une case vide est vide. Deux postes qui écrivent la même case ne
//! s'écrasent pas en silence : la saisie renvoie l'instant lu, et le second reçoit le refus
//! « modifiée entre-temps » (SPEC_PHASE_11_A §6.1).

use std::collections::BTreeMap;

use gesco_core::commun::Id;
use gesco_core::core::{Operation, TypeEvenement};
use gesco_core::pedagogie::jardin::{APPRECIATION_MAX, cle_commentaire_domaine};
use gesco_core::pedagogie::{
    Competence, DomaineCompetence, NiveauAcquisition, Observation, Periode,
};
use gesco_core::scolarite::{Classe, Inscription, ModeEvaluation, StatutInscription};
use gesco_db::depots::bulletins::{APPRECIATION_GENERALE, appreciations, ecrire_appreciation};
use gesco_db::depots::jardin::CleObservation;
use gesco_db::depots::{academique, jardin, pedagogie, personnes};
use gesco_db::unite::Durabilite;
use gesco_db::{UniteDeTravail, nouvel_id};
use serde::{Deserialize, Serialize};

use crate::effets::Effet;
use crate::enseignants::ServiceEnseignants;
use crate::erreur::{ErreurService, Resultat};
use crate::fraicheur::{self, Ligne};
use crate::inscriptions::ServiceInscriptions;
use crate::jardin::ServiceJardin;
use crate::noyau::Noyau;
use crate::session::Session;
use crate::{annees, image};

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Grille d'une classe du jardin dans une période.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeGrilleObservations {
    /// Classe du jardin.
    pub classe_id: Id<Classe>,
    /// Période.
    pub periode_id: Id<Periode>,
}

/// Enfant d'une grille d'observations.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct EnfantObserve {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Prénoms et nom.
    pub eleve: String,
    /// Matricule.
    pub matricule: String,
    /// Photo en adresse de données, si la fiche en a une.
    pub photo: Option<String>,
}

/// Appréciation générale (`domaine_id` vide) ou commentaire d'un domaine, pour un enfant.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct AppreciationJardin {
    /// Inscription de l'enfant.
    pub inscription_id: Id<Inscription>,
    /// Domaine commenté ; vide pour l'appréciation générale.
    pub domaine_id: Option<Id<DomaineCompetence>>,
    /// Texte.
    pub texte: String,
}

/// Grille de saisie des observations d'une classe du jardin pour une période.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct GrilleObservations {
    /// Libellé de la classe.
    pub classe: String,
    /// Libellé de la section.
    pub section: String,
    /// Période.
    pub periode: Periode,
    /// Vrai si la période et l'année se modifient encore.
    pub modifiable: bool,
    /// Domaines qui servent et ont au moins une compétence de la section, dans leur ordre.
    pub domaines: Vec<DomaineCompetence>,
    /// Compétences de la section qui servent, dans l'ordre de leur domaine.
    pub competences: Vec<Competence>,
    /// Enfants de la classe inscrits pendant la période, dans l'ordre de la liste.
    pub enfants: Vec<EnfantObserve>,
    /// Observations de ces enfants pour ces compétences, vidées comprises.
    pub observations: Vec<Observation>,
    /// Appréciations générales et commentaires de domaine de ces enfants.
    pub appreciations: Vec<AppreciationJardin>,
    /// Libellés des trois niveaux : acquis, en cours, non acquis.
    pub echelle: Vec<String>,
    /// Vrai si la ligne de commentaire par domaine se montre.
    pub commentaire_domaine: bool,
}

/// Saisie d'une case : le niveau d'une compétence pour un enfant.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeObservation {
    /// Classe du jardin.
    pub classe_id: Id<Classe>,
    /// Période.
    pub periode_id: Id<Periode>,
    /// Compétence.
    pub competence_id: Id<Competence>,
    /// Enfant.
    pub inscription_id: Id<Inscription>,
    /// Niveau ; vide pour vider la case.
    pub niveau: Option<NiveauAcquisition>,
    /// Instant de la case lu avec la grille ; zéro si elle n'avait pas d'observation.
    pub lue_le: i64,
}

/// « Toute la colonne » : un niveau posé à tous les enfants pour une compétence.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeColonne {
    /// Classe du jardin.
    pub classe_id: Id<Classe>,
    /// Période.
    pub periode_id: Id<Periode>,
    /// Compétence.
    pub competence_id: Id<Competence>,
    /// Niveau posé.
    pub niveau: NiveauAcquisition,
    /// Instant le plus récent des cases de la colonne lu avec la grille ; zéro si aucune n'avait
    /// d'observation.
    pub lue_le: i64,
}

/// Appréciation générale ou commentaire d'un domaine écrit pour un enfant.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeAppreciationJardin {
    /// Classe du jardin.
    pub classe_id: Id<Classe>,
    /// Période.
    pub periode_id: Id<Periode>,
    /// Enfant.
    pub inscription_id: Id<Inscription>,
    /// Domaine commenté ; vide pour l'appréciation générale.
    pub domaine_id: Option<Id<DomaineCompetence>>,
    /// Texte, 300 caractères au plus ; vide pour l'effacer.
    pub texte: String,
}

/// Ce que la saisie a besoin de savoir d'une classe et d'une période.
struct Cadre {
    classe: Classe,
    section: String,
    periode: Periode,
    modifiable: bool,
    domaines: Vec<DomaineCompetence>,
    competences: Vec<Competence>,
    enfants: Vec<(Id<Inscription>, String, String)>,
    echelle: Vec<String>,
    commentaire_domaine: bool,
}

/// Ligne de la table `observation` qui porte une case.
fn ligne(cle: &CleObservation) -> Ligne {
    Ligne {
        table: "observation",
        cles: vec![
            ("inscription_id", cle.inscription_id.vers_chaine()),
            ("competence_id", cle.competence_id.vers_chaine()),
            ("periode_id", cle.periode_id.vers_chaine()),
        ],
    }
}

/// Service des observations du jardin.
pub struct ServiceObservations<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceObservations<'n> {
    /// Service sur le noyau ouvert.
    pub const fn nouveau(noyau: &'n Noyau) -> Self {
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

    /// Classe du jardin, période, compétences et enfants ; refuse une classe évaluée par notes
    /// (RB-10), une période d'un autre niveau, et un utilisateur qui n'observe pas cette classe.
    async fn cadre(
        &self,
        session: &Session,
        classe_id: Id<Classe>,
        periode_id: Id<Periode>,
    ) -> Resultat<Cadre> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let (classe, niveau, periodes, annee) = {
            let mut cnx = self.connexion().await?;
            let classe = academique::classe(&mut cnx, classe_id, &contexte).await?;
            let niveau = academique::niveau(&mut cnx, classe.niveau_id, &contexte).await?;
            let periodes = pedagogie::periodes(&mut cnx, classe.annee_id, &contexte).await?;
            let annee =
                gesco_db::depots::annee_scolaire::lire(&mut cnx, classe.annee_id, &contexte)
                    .await?;
            (classe, niveau, periodes, annee)
        };
        if niveau.mode_evaluation != ModeEvaluation::Competences {
            return Err(regle("jardin.classe_en_notes"));
        }
        let portee = ServiceEnseignants::nouveau(self.noyau)
            .portee_notes(session)
            .await?;
        if !portee.observe(classe.id) {
            return Err(ErreurService::PorteeRefusee {
                ressource: format!("classe {}", classe.id),
            });
        }
        let periode = periodes
            .into_iter()
            .find(|p| p.id == periode_id && p.niveau_id == classe.niveau_id)
            .ok_or(regle("evaluation.periode_invalide"))?;
        let liste = ServiceJardin::nouveau(self.noyau).lire(session).await?;
        let competences: Vec<Competence> = {
            let mut c: Vec<Competence> = liste
                .competences
                .into_iter()
                .filter(|c| c.retiree_le.is_none() && c.niveau_id == classe.niveau_id)
                .collect();
            let rang = |d: Id<DomaineCompetence>| {
                liste
                    .domaines
                    .iter()
                    .position(|x| x.id == d)
                    .unwrap_or(usize::MAX)
            };
            c.retain(|c| {
                liste
                    .domaines
                    .iter()
                    .any(|d| d.id == c.domaine_id && d.retire_le.is_none())
            });
            c.sort_by_key(|c| (rang(c.domaine_id), c.ordre));
            c
        };
        let domaines: Vec<DomaineCompetence> = liste
            .domaines
            .into_iter()
            .filter(|d| d.retire_le.is_none() && competences.iter().any(|c| c.domaine_id == d.id))
            .collect();
        let enfants = ServiceInscriptions::nouveau(self.noyau)
            .de_la_classe(session, &classe)
            .await?
            .into_iter()
            .filter(|l| {
                l.inscription.classe_id == classe.id
                    && l.inscription.statut == StatutInscription::Validee
                    && l.inscription.date_inscription <= periode.fin
            })
            .map(|l| (l.inscription.id, l.eleve, l.matricule))
            .collect();
        let modifiable = !periode.verrouillee && annee.est_modifiable();
        Ok(Cadre {
            section: niveau.libelle,
            classe,
            modifiable,
            periode,
            domaines,
            competences,
            enfants,
            echelle: liste.echelle,
            commentaire_domaine: liste.commentaire_domaine,
        })
    }

    /// Refuse une écriture dans une période verrouillée ou une année close (RB-14).
    async fn exiger_modifiable(&self, session: &Session, cadre: &Cadre) -> Resultat<()> {
        annees::exiger_classe_modifiable(self.noyau, session, cadre.classe.id).await?;
        if cadre.periode.verrouillee {
            return Err(regle("periode.verrouillee"));
        }
        Ok(())
    }

    /// Grille de saisie : enfants, compétences par domaine, observations et appréciations.
    pub async fn grille(
        &self,
        session: &Session,
        demande: &DemandeGrilleObservations,
    ) -> Resultat<GrilleObservations> {
        let cadre = self
            .cadre(session, demande.classe_id, demande.periode_id)
            .await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let est_enfant = |i: &Id<Inscription>| cadre.enfants.iter().any(|(e, _, _)| e == i);
        let observations: Vec<Observation> =
            jardin::observations_de_la_periode(&mut cnx, cadre.periode.id, &contexte)
                .await?
                .into_iter()
                .filter(|o| {
                    est_enfant(&o.inscription_id)
                        && cadre.competences.iter().any(|c| c.id == o.competence_id)
                })
                .collect();
        let appreciations: Vec<AppreciationJardin> =
            appreciations(&mut cnx, cadre.periode.id, &contexte)
                .await?
                .into_iter()
                .filter(|a| est_enfant(&a.inscription_id))
                .filter_map(|a| {
                    let domaine_id = if a.matiere_cle == APPRECIATION_GENERALE {
                        None
                    } else {
                        Some(
                            cadre
                                .domaines
                                .iter()
                                .find(|d| cle_commentaire_domaine(d.id) == a.matiere_cle)?
                                .id,
                        )
                    };
                    Some(AppreciationJardin {
                        inscription_id: a.inscription_id,
                        domaine_id,
                        texte: a.texte,
                    })
                })
                .collect();
        // La photo de chaque enfant qui en a une, en vignette ; aucune place pour les autres.
        let personnes: BTreeMap<String, Id<gesco_core::scolarite::Personne>> =
            personnes::eleves_de_la_classe(&mut cnx, cadre.classe.id, &contexte)
                .await?
                .into_iter()
                .map(|(e, p)| (e.id.vers_chaine(), p.id))
                .collect();
        let inscriptions =
            gesco_db::depots::inscriptions::de_la_classe(&mut cnx, cadre.classe.id, &contexte)
                .await?;
        let mut enfants = Vec::with_capacity(cadre.enfants.len());
        for (inscription_id, eleve, matricule) in &cadre.enfants {
            let personne = inscriptions
                .iter()
                .find(|i| i.id == *inscription_id)
                .and_then(|i| personnes.get(&i.eleve_id.vers_chaine()));
            let photo = match personne {
                Some(p) => personnes::photo(&mut cnx, *p, &contexte)
                    .await?
                    .map(|(octets, format)| image::url_de_donnees(&octets, &format)),
                None => None,
            };
            enfants.push(EnfantObserve {
                inscription_id: *inscription_id,
                eleve: eleve.clone(),
                matricule: matricule.clone(),
                photo,
            });
        }
        Ok(GrilleObservations {
            classe: cadre.classe.libelle,
            section: cadre.section,
            periode: cadre.periode,
            modifiable: cadre.modifiable,
            domaines: cadre.domaines,
            competences: cadre.competences,
            enfants,
            observations,
            appreciations,
            echelle: cadre.echelle,
            commentaire_domaine: cadre.commentaire_domaine,
        })
    }

    /// Écrit des cases dans l'unité de travail : audit de chacune, valeur avant et après, et un
    /// événement par enfant et par domaine (SPEC_PHASE_11_B §3.6).
    async fn ecrire(
        unite: &mut UniteDeTravail<Effet>,
        cadre: &Cadre,
        cases: &[(
            CleObservation,
            Option<Observation>,
            Option<NiveauAcquisition>,
        )],
        origine: &str,
    ) -> Resultat<()> {
        let contexte = unite.contexte().clone();
        let mut evenements: BTreeMap<(String, String), u32> = BTreeMap::new();
        for (cle, avant, niveau) in cases {
            jardin::ecrire_observation(
                unite.connexion(),
                &nouvel_id::<Observation>().vers_chaine(),
                *cle,
                *niveau,
                &contexte,
            )
            .await?;
            unite.auditer(
                "observation",
                &format!(
                    "{}:{}:{}",
                    cle.inscription_id, cle.competence_id, cle.periode_id
                ),
                if avant.is_some() {
                    Operation::Modification
                } else {
                    Operation::Creation
                },
                avant
                    .as_ref()
                    .map(|a| serde_json::json!({ "niveau": a.niveau })),
                Some(serde_json::json!({ "niveau": niveau })),
                origine,
            );
            let domaine = cadre
                .competences
                .iter()
                .find(|c| c.id == cle.competence_id)
                .map(|c| c.domaine_id.vers_chaine())
                .unwrap_or_default();
            *evenements
                .entry((cle.inscription_id.vers_chaine(), domaine))
                .or_default() += 1;
        }
        for ((inscription, domaine), nombre) in evenements {
            unite.emettre(
                TypeEvenement::ObservationSaisie,
                "inscription",
                &inscription,
                serde_json::json!({ "periode_id": cadre.periode.id, "domaine_id": domaine,
                    "cases": nombre }),
            );
        }
        Ok(())
    }

    /// Relit les observations d'une période dans la connexion donnée.
    async fn observations(
        cnx: &mut gesco_db::Connexion,
        periode: Id<Periode>,
        contexte: &gesco_db::Contexte,
    ) -> Resultat<Vec<Observation>> {
        Ok(jardin::observations_de_la_periode(cnx, periode, contexte).await?)
    }

    /// Pose, remplace ou vide une case. Refusée si la case a changé depuis sa lecture.
    pub async fn saisir(
        &self,
        session: &Session,
        demande: &DemandeObservation,
    ) -> Resultat<Observation> {
        let cadre = self
            .cadre(session, demande.classe_id, demande.periode_id)
            .await?;
        self.exiger_modifiable(session, &cadre).await?;
        if !cadre
            .enfants
            .iter()
            .any(|(i, _, _)| *i == demande.inscription_id)
        {
            return Err(regle("observation.enfant_hors_classe"));
        }
        if !cadre
            .competences
            .iter()
            .any(|c| c.id == demande.competence_id)
        {
            return Err(regle("jardin.competence_inconnue"));
        }
        let cle = CleObservation {
            inscription_id: demande.inscription_id,
            competence_id: demande.competence_id,
            periode_id: cadre.periode.id,
        };
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        fraicheur::verifier(unite.connexion(), &[ligne(&cle)], demande.lue_le).await?;
        let avant = Self::observations(unite.connexion(), cadre.periode.id, &contexte)
            .await?
            .into_iter()
            .find(|o| {
                o.inscription_id == cle.inscription_id && o.competence_id == cle.competence_id
            });
        let inchangee = match &avant {
            Some(a) => a.niveau == demande.niveau,
            None => demande.niveau.is_none(),
        };
        if inchangee {
            // Rien à écrire : l'unité se termine sans écriture.
            self.noyau.terminer(unite).await?;
            return Ok(avant.unwrap_or(Observation {
                inscription_id: cle.inscription_id,
                competence_id: cle.competence_id,
                periode_id: cle.periode_id,
                niveau: None,
                modifiee_le: 0,
            }));
        }
        Self::ecrire(
            &mut unite,
            &cadre,
            &[(cle, avant, demande.niveau)],
            "observations.saisir",
        )
        .await?;
        let apres = Self::observations(unite.connexion(), cadre.periode.id, &contexte)
            .await?
            .into_iter()
            .find(|o| {
                o.inscription_id == cle.inscription_id && o.competence_id == cle.competence_id
            })
            .ok_or(regle("jardin.competence_inconnue"))?;
        self.noyau.terminer(unite).await?;
        Ok(apres)
    }

    /// Pose un niveau à tous les enfants de la grille pour une compétence ; les cases qui l'ont
    /// déjà ne changent pas. Refusée si une case de la colonne a changé depuis la lecture. Rend
    /// les observations de la colonne.
    pub async fn colonne(
        &self,
        session: &Session,
        demande: &DemandeColonne,
    ) -> Resultat<Vec<Observation>> {
        let cadre = self
            .cadre(session, demande.classe_id, demande.periode_id)
            .await?;
        self.exiger_modifiable(session, &cadre).await?;
        if !cadre
            .competences
            .iter()
            .any(|c| c.id == demande.competence_id)
        {
            return Err(regle("jardin.competence_inconnue"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let de_la_colonne = |o: &Observation| {
            o.competence_id == demande.competence_id
                && cadre.enfants.iter().any(|(i, _, _)| *i == o.inscription_id)
        };
        let colonne: Vec<Observation> =
            Self::observations(unite.connexion(), cadre.periode.id, &contexte)
                .await?
                .into_iter()
                .filter(de_la_colonne)
                .collect();
        // La case la plus récente dit si quelqu'un a écrit dans la colonne depuis la lecture.
        if let Some(recente) = colonne.iter().max_by_key(|o| o.modifiee_le)
            && recente.modifiee_le != demande.lue_le
        {
            let cle = CleObservation {
                inscription_id: recente.inscription_id,
                competence_id: recente.competence_id,
                periode_id: recente.periode_id,
            };
            fraicheur::verifier(unite.connexion(), &[ligne(&cle)], demande.lue_le).await?;
        }
        let cases: Vec<(
            CleObservation,
            Option<Observation>,
            Option<NiveauAcquisition>,
        )> = cadre
            .enfants
            .iter()
            .filter_map(|(inscription_id, _, _)| {
                let avant = colonne
                    .iter()
                    .find(|o| o.inscription_id == *inscription_id)
                    .cloned();
                if avant.as_ref().and_then(|a| a.niveau) == Some(demande.niveau) {
                    return None;
                }
                Some((
                    CleObservation {
                        inscription_id: *inscription_id,
                        competence_id: demande.competence_id,
                        periode_id: cadre.periode.id,
                    },
                    avant,
                    Some(demande.niveau),
                ))
            })
            .collect();
        if !cases.is_empty() {
            Self::ecrire(&mut unite, &cadre, &cases, "observations.colonne").await?;
        }
        let apres: Vec<Observation> =
            Self::observations(unite.connexion(), cadre.periode.id, &contexte)
                .await?
                .into_iter()
                .filter(de_la_colonne)
                .collect();
        self.noyau.terminer(unite).await?;
        Ok(apres)
    }

    /// Écrit l'appréciation générale ou le commentaire d'un domaine d'un enfant ; un texte vide
    /// l'efface. La dernière écriture gagne, comme pour les appréciations des bulletins.
    pub async fn apprecier(
        &self,
        session: &Session,
        demande: &DemandeAppreciationJardin,
    ) -> Resultat<String> {
        let texte = demande.texte.trim().to_owned();
        if texte.chars().count() > APPRECIATION_MAX {
            return Err(regle("appreciation.trop_longue"));
        }
        let cadre = self
            .cadre(session, demande.classe_id, demande.periode_id)
            .await?;
        self.exiger_modifiable(session, &cadre).await?;
        if !cadre
            .enfants
            .iter()
            .any(|(i, _, _)| *i == demande.inscription_id)
        {
            return Err(regle("observation.enfant_hors_classe"));
        }
        let cle = match demande.domaine_id {
            Some(domaine) => {
                if !cadre.domaines.iter().any(|d| d.id == domaine) {
                    return Err(regle("jardin.domaine_inconnu"));
                }
                cle_commentaire_domaine(domaine)
            }
            None => APPRECIATION_GENERALE.to_owned(),
        };
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        ecrire_appreciation(
            unite.connexion(),
            demande.inscription_id,
            cadre.periode.id,
            &cle,
            &texte,
            false,
            &contexte,
        )
        .await?;
        unite.auditer(
            "appreciation",
            &format!("{}:{}:{cle}", demande.inscription_id, cadre.periode.id),
            Operation::Modification,
            None,
            Some(serde_json::json!({ "texte": texte })),
            "observations.apprecier",
        );
        unite.emettre(
            TypeEvenement::AppreciationSaisie,
            "inscription",
            &demande.inscription_id.vers_chaine(),
            serde_json::json!({ "periode_id": cadre.periode.id, "cle": cle }),
        );
        self.noyau.terminer(unite).await?;
        Ok(texte)
    }
}
