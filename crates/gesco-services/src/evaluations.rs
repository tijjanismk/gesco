//! Évaluations et saisie des notes (SPEC_PHASE_4 §4, UC-03) : la direction, ou l'enseignant avec
//! son compte dans ses seules classes et matières (§11.3), crée un devoir ou une composition et
//! saisit les notes au clavier, une par une, chacune enregistrée aussitôt. La direction valide ;
//! une évaluation validée ne se rouvre que par un déverrouillage motivé.

use gesco_core::commun::{DateCivile, Id};
use gesco_core::core::{MachineAEtats, Operation, TypeEvenement};
use gesco_core::pedagogie::evaluation::{complete, lire_saisie, verifier_evaluation};
use gesco_core::pedagogie::{
    Enseignant, Evaluation, Matiere, NoteEleve, Periode, PorteeNotes, RegleEvaluation,
    StatutEvaluation, TypeEvaluation,
};
use gesco_core::scolarite::{Classe, Inscription, StatutInscription};
use gesco_db::depots::{academique, enseignants, evaluations, pedagogie};
use gesco_db::nouvel_id;
use gesco_db::unite::Durabilite;
use serde::{Deserialize, Serialize};

use crate::annees;
use crate::enseignants::ServiceEnseignants;
use crate::erreur::{ErreurService, Resultat};
use crate::inscriptions::ServiceInscriptions;
use crate::noyau::Noyau;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Évaluations d'une classe dans une période.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeEvaluations {
    /// Classe.
    pub classe_id: Id<Classe>,
    /// Période.
    pub periode_id: Id<Periode>,
}

/// Évaluation affichée.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct EvaluationAffichee {
    /// Évaluation.
    pub evaluation: Evaluation,
    /// Libellé de la matière.
    pub matiere: String,
    /// Élèves qui ont une note, une absence ou une dispense.
    pub saisies: u32,
    /// Élèves de la classe.
    pub effectif: u32,
}

/// Création d'une évaluation.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeEvaluation {
    /// Classe.
    pub classe_id: Id<Classe>,
    /// Période.
    pub periode_id: Id<Periode>,
    /// Matière.
    pub matiere_id: Id<Matiere>,
    /// Devoir ou composition.
    pub type_evaluation: TypeEvaluation,
    /// Libellé.
    pub libelle: String,
    /// Jour.
    pub date: DateCivile,
    /// Barème en centièmes ; absent : celui de la règle du cycle.
    pub bareme: Option<i32>,
}

/// Élève de la grille.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct LigneGrille {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Prénoms et nom.
    pub eleve: String,
    /// Matricule.
    pub matricule: String,
    /// Note saisie.
    pub note: Option<NoteEleve>,
}

/// Grille de saisie d'une évaluation.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct GrilleNotes {
    /// Évaluation.
    pub evaluation: Evaluation,
    /// Libellé de la matière.
    pub matiere: String,
    /// Libellé de la classe.
    pub classe: String,
    /// Élèves, dans l'ordre de la liste de classe.
    pub lignes: Vec<LigneGrille>,
    /// Vrai tant que l'évaluation est en saisie.
    pub modifiable: bool,
    /// Moyenne des notes saisies, en centièmes du barème de l'évaluation, tronquée.
    pub moyenne: Option<i32>,
}

/// Saisie d'une case de la grille.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeNote {
    /// Évaluation.
    pub evaluation_id: Id<Evaluation>,
    /// Élève.
    pub inscription_id: Id<Inscription>,
    /// Ce qui est tapé : `13,5`, `a`, `aj`, `d`, ou rien.
    pub saisie: String,
}

/// Évaluation désignée.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeIdEvaluation {
    /// Évaluation.
    pub id: Id<Evaluation>,
}

/// Déverrouillage motivé.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeDeverrouillage {
    /// Évaluation.
    pub id: Id<Evaluation>,
    /// Motif, obligatoire.
    pub motif: String,
}

/// Service des évaluations.
pub struct ServiceEvaluations<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceEvaluations<'n> {
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

    async fn portee(&self, session: &Session) -> Resultat<PorteeNotes> {
        ServiceEnseignants::nouveau(self.noyau)
            .portee_notes(session)
            .await
    }

    fn verifier_portee(
        portee: &PorteeNotes,
        classe: Id<Classe>,
        matiere: Id<Matiere>,
    ) -> Resultat<()> {
        if portee.autorise(classe, matiere) {
            Ok(())
        } else {
            Err(ErreurService::PorteeRefusee {
                ressource: format!("classe {classe}, matière {matiere}"),
            })
        }
    }

    /// Élèves validés d'une classe, dans l'ordre de la liste, avec leur date d'inscription.
    async fn eleves_dates(
        &self,
        session: &Session,
        classe: &Classe,
    ) -> Resultat<Vec<(Id<Inscription>, String, String, DateCivile)>> {
        Ok(ServiceInscriptions::nouveau(self.noyau)
            .lister(session, classe.annee_id)
            .await?
            .into_iter()
            .filter(|l| {
                l.inscription.classe_id == classe.id
                    && l.inscription.statut == StatutInscription::Validee
            })
            .map(|l| {
                (
                    l.inscription.id,
                    l.eleve,
                    l.matricule,
                    l.inscription.date_inscription,
                )
            })
            .collect())
    }

    /// Élèves validés d'une classe déjà inscrits le jour d'une évaluation (§4.2) : un élève
    /// arrivé ensuite n'a pas de note à cette évaluation (Q-27). (inscription, élève, matricule).
    async fn eleves(
        &self,
        session: &Session,
        classe: &Classe,
        au: DateCivile,
    ) -> Resultat<Vec<(Id<Inscription>, String, String)>> {
        Ok(self
            .eleves_dates(session, classe)
            .await?
            .into_iter()
            .filter(|(_, _, _, inscrit_le)| *inscrit_le <= au)
            .map(|(i, e, m, _)| (i, e, m))
            .collect())
    }

    /// Période d'une classe, et la règle de son cycle.
    async fn periode_et_regle(
        &self,
        session: &Session,
        classe: &Classe,
        periode_id: Id<Periode>,
    ) -> Resultat<(Periode, RegleEvaluation)> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let niveau = academique::niveau(&mut cnx, classe.niveau_id, &contexte).await?;
        let periode = pedagogie::periodes(&mut cnx, classe.annee_id, &contexte)
            .await?
            .into_iter()
            .find(|p| p.id == periode_id && p.niveau_id == classe.niveau_id)
            .ok_or(regle("evaluation.periode_invalide"))?;
        let regle_cycle = pedagogie::regles_en_vigueur(&mut cnx, classe.annee_id, &contexte)
            .await?
            .into_iter()
            .find(|r| r.cycle == niveau.cycle)
            .ok_or(regle("evaluation.sans_regle"))?;
        Ok((periode, regle_cycle))
    }

    /// Évaluations d'une classe dans une période ; un enseignant ne voit que ses matières.
    pub async fn lister(
        &self,
        session: &Session,
        demande: &DemandeEvaluations,
    ) -> Resultat<Vec<EvaluationAffichee>> {
        let portee = self.portee(session).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let (classe, liste, matieres) = {
            let mut cnx = self.connexion().await?;
            let classe = academique::classe(&mut cnx, demande.classe_id, &contexte).await?;
            let liste =
                evaluations::de_la_classe(&mut cnx, classe.id, demande.periode_id, &contexte)
                    .await?;
            (
                classe,
                liste,
                pedagogie::matieres(&mut cnx, &contexte).await?,
            )
        };
        let tous = self.eleves_dates(session, &classe).await?;
        let mut affichees = Vec::new();
        for e in liste
            .into_iter()
            .filter(|e| portee.autorise(e.classe_id, e.matiere_id))
        {
            let notes = {
                let mut cnx = self.connexion().await?;
                evaluations::notes(&mut cnx, e.id, &contexte).await?
            };
            let effectif: Vec<Id<Inscription>> = tous
                .iter()
                .filter(|(_, _, _, inscrit_le)| *inscrit_le <= e.date)
                .map(|(i, _, _, _)| *i)
                .collect();
            affichees.push(EvaluationAffichee {
                matiere: matieres
                    .iter()
                    .find(|m| m.id == e.matiere_id)
                    .map(|m| m.libelle.clone())
                    .unwrap_or_default(),
                saisies: u32::try_from(
                    notes
                        .iter()
                        .filter(|n| effectif.contains(&n.inscription_id))
                        .count(),
                )
                .unwrap_or(u32::MAX),
                effectif: u32::try_from(effectif.len()).unwrap_or(u32::MAX),
                evaluation: e,
            });
        }
        Ok(affichees)
    }

    /// Crée une évaluation dans une matière de la classe ; le barème par défaut est celui de la
    /// règle du cycle, l'enseignant celui de l'affectation.
    pub async fn creer(
        &self,
        session: &Session,
        demande: &DemandeEvaluation,
    ) -> Resultat<Evaluation> {
        let portee = self.portee(session).await?;
        Self::verifier_portee(&portee, demande.classe_id, demande.matiere_id)?;
        annees::exiger_classe_modifiable(self.noyau, session, demande.classe_id).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let (classe, au_programme, enseignant) = {
            let mut cnx = self.connexion().await?;
            let classe = academique::classe(&mut cnx, demande.classe_id, &contexte).await?;
            let au_programme = pedagogie::programme(&mut cnx, classe.annee_id, &contexte)
                .await?
                .iter()
                .any(|l| {
                    l.niveau_id == classe.niveau_id
                        && l.serie_id == classe.serie_id
                        && l.matiere_id == demande.matiere_id
                });
            let enseignant: Option<Id<Enseignant>> =
                enseignants::affectations(&mut cnx, classe.annee_id, &contexte)
                    .await?
                    .into_iter()
                    .find(|a| a.classe_id == classe.id && a.matiere_id == demande.matiere_id)
                    .map(|a| a.enseignant_id);
            (classe, au_programme, enseignant)
        };
        if !au_programme {
            return Err(regle("affectation.matiere_hors_programme"));
        }
        let (periode, regle_cycle) = self
            .periode_et_regle(session, &classe, demande.periode_id)
            .await?;
        let bareme = demande
            .bareme
            .or_else(|| regle_cycle.contenu.bareme_de(demande.type_evaluation))
            .unwrap_or(0);
        verifier_evaluation(
            &regle_cycle.contenu,
            &periode,
            demande.type_evaluation,
            &demande.libelle,
            demande.date,
            bareme,
            1,
        )
        .map_err(regle)?;
        let e = Evaluation {
            id: nouvel_id(),
            etablissement_id: session.etablissement_id,
            annee_id: classe.annee_id,
            periode_id: periode.id,
            classe_id: classe.id,
            matiere_id: demande.matiere_id,
            type_evaluation: demande.type_evaluation,
            libelle: demande.libelle.trim().to_owned(),
            date: demande.date,
            bareme,
            ponderation: 1,
            enseignant_id: enseignant,
            statut: StatutEvaluation::EnSaisie,
            motif_deverrouillage: String::new(),
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        evaluations::inserer(unite.connexion(), &e, &contexte).await?;
        let id = e.id.vers_chaine();
        unite.auditer(
            "evaluation",
            &id,
            Operation::Creation,
            None,
            Some(serde_json::to_value(&e)?),
            "evaluations.creer",
        );
        unite.emettre(
            TypeEvenement::EvaluationCreee,
            "evaluation",
            &id,
            serde_json::json!({ "classe_id": e.classe_id, "matiere_id": e.matiere_id, "type": e.type_evaluation }),
        );
        self.noyau.terminer(unite).await?;
        Ok(e)
    }

    async fn evaluation_et_classe(
        &self,
        session: &Session,
        id: Id<Evaluation>,
    ) -> Resultat<(Evaluation, Classe)> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let e = evaluations::lire(&mut cnx, id, &contexte).await?;
        let classe = academique::classe(&mut cnx, e.classe_id, &contexte).await?;
        Ok((e, classe))
    }

    /// Grille de saisie : les élèves de la classe et leur note.
    pub async fn grille(&self, session: &Session, id: Id<Evaluation>) -> Resultat<GrilleNotes> {
        let (e, classe) = self.evaluation_et_classe(session, id).await?;
        Self::verifier_portee(&self.portee(session).await?, e.classe_id, e.matiere_id)?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let (notes, matiere) = {
            let mut cnx = self.connexion().await?;
            let notes = evaluations::notes(&mut cnx, e.id, &contexte).await?;
            let matiere = pedagogie::matieres(&mut cnx, &contexte)
                .await?
                .into_iter()
                .find(|m| m.id == e.matiere_id)
                .map(|m| m.libelle)
                .unwrap_or_default();
            (notes, matiere)
        };
        let lignes: Vec<LigneGrille> = self
            .eleves(session, &classe, e.date)
            .await?
            .into_iter()
            .map(|(inscription_id, eleve, matricule)| LigneGrille {
                note: notes
                    .iter()
                    .find(|n| n.inscription_id == inscription_id)
                    .copied(),
                inscription_id,
                eleve,
                matricule,
            })
            .collect();
        let valeurs: Vec<i64> = lignes
            .iter()
            .filter_map(|l| l.note.and_then(|n| n.valeur))
            .map(i64::from)
            .collect();
        let moyenne = if valeurs.is_empty() {
            None
        } else {
            let n = i64::try_from(valeurs.len()).unwrap_or(i64::MAX);
            i32::try_from(valeurs.iter().sum::<i64>().div_euclid(n)).ok()
        };
        Ok(GrilleNotes {
            modifiable: e.statut == StatutEvaluation::EnSaisie,
            matiere,
            classe: classe.libelle,
            lignes,
            moyenne,
            evaluation: e,
        })
    }

    /// Enregistre ce qui est tapé dans une case : une note, `a`, `aj`, `d`, ou rien pour la
    /// retirer. Chaque saisie est une transaction auditée, valeur avant et après (§12.3).
    pub async fn saisir(
        &self,
        session: &Session,
        demande: &DemandeNote,
    ) -> Resultat<Option<NoteEleve>> {
        let (e, classe) = self
            .evaluation_et_classe(session, demande.evaluation_id)
            .await?;
        Self::verifier_portee(&self.portee(session).await?, e.classe_id, e.matiere_id)?;
        annees::exiger_modifiable(self.noyau, session, e.annee_id).await?;
        if e.statut != StatutEvaluation::EnSaisie {
            return Err(regle("evaluation.validee"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let (periode_verrouillee, avant) = {
            let mut cnx = self.connexion().await?;
            let verrou = pedagogie::periodes(&mut cnx, e.annee_id, &contexte)
                .await?
                .iter()
                .any(|p| p.id == e.periode_id && p.verrouillee);
            let avant = evaluations::notes(&mut cnx, e.id, &contexte)
                .await?
                .into_iter()
                .find(|n| n.inscription_id == demande.inscription_id);
            (verrou, avant)
        };
        if periode_verrouillee {
            return Err(regle("periode.verrouillee"));
        }
        if !self
            .eleves(session, &classe, e.date)
            .await?
            .iter()
            .any(|(i, _, _)| *i == demande.inscription_id)
        {
            return Err(regle("note.eleve_hors_classe"));
        }
        let apres = lire_saisie(&demande.saisie, e.bareme)
            .map_err(regle)?
            .en_note(e.id, demande.inscription_id);
        if apres == avant {
            return Ok(apres);
        }
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        evaluations::ecrire_note(
            unite.connexion(),
            e.id,
            demande.inscription_id,
            apres.as_ref(),
            &contexte,
        )
        .await?;
        let id = format!("{}:{}", e.id, demande.inscription_id);
        unite.auditer(
            "note",
            &id,
            match (&avant, &apres) {
                (None, _) => Operation::Creation,
                (Some(_), None) => Operation::Suppression,
                (Some(_), Some(_)) => Operation::Modification,
            },
            avant.as_ref().map(serde_json::to_value).transpose()?,
            apres.as_ref().map(serde_json::to_value).transpose()?,
            "evaluations.saisir",
        );
        unite.emettre(
            TypeEvenement::NoteSaisie,
            "evaluation",
            &e.id.vers_chaine(),
            serde_json::json!({ "inscription_id": demande.inscription_id, "retiree": apres.is_none() }),
        );
        self.noyau.terminer(unite).await?;
        Ok(apres)
    }

    /// Valide une évaluation complète (`grades:validate`) : plus aucune note ne change.
    pub async fn valider(&self, session: &Session, id: Id<Evaluation>) -> Resultat<Evaluation> {
        let (avant, classe) = self.evaluation_et_classe(session, id).await?;
        annees::exiger_modifiable(self.noyau, session, avant.annee_id).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let notes = {
            let mut cnx = self.connexion().await?;
            evaluations::notes(&mut cnx, avant.id, &contexte).await?
        };
        let eleves: Vec<Id<Inscription>> = self
            .eleves(session, &classe, avant.date)
            .await?
            .into_iter()
            .map(|(i, _, _)| i)
            .collect();
        if !complete(&eleves, &notes) {
            return Err(regle("evaluation.incomplete"));
        }
        let apres = Evaluation {
            statut: avant.statut.transiter(StatutEvaluation::Validee)?,
            ..avant.clone()
        };
        self.ecrire_statut(
            session,
            &avant,
            &apres,
            TypeEvenement::EvaluationValidee,
            "evaluations.valider",
        )
        .await?;
        Ok(apres)
    }

    /// Déverrouille une évaluation validée, avec motif (`grades:unlock`).
    pub async fn deverrouiller(
        &self,
        session: &Session,
        demande: &DemandeDeverrouillage,
    ) -> Resultat<Evaluation> {
        let motif = demande.motif.trim();
        if motif.is_empty() {
            return Err(regle("evaluation.motif_obligatoire"));
        }
        let (avant, _) = self.evaluation_et_classe(session, demande.id).await?;
        annees::exiger_modifiable(self.noyau, session, avant.annee_id).await?;
        let apres = Evaluation {
            statut: avant.statut.transiter(StatutEvaluation::EnSaisie)?,
            motif_deverrouillage: motif.to_owned(),
            ..avant.clone()
        };
        self.ecrire_statut(
            session,
            &avant,
            &apres,
            TypeEvenement::EvaluationDeverrouillee,
            "evaluations.deverrouiller",
        )
        .await?;
        Ok(apres)
    }

    async fn ecrire_statut(
        &self,
        session: &Session,
        avant: &Evaluation,
        apres: &Evaluation,
        evenement: TypeEvenement,
        origine: &str,
    ) -> Resultat<()> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        evaluations::ecrire_statut(unite.connexion(), apres, &contexte).await?;
        let id = apres.id.vers_chaine();
        unite.auditer(
            "evaluation",
            &id,
            Operation::Transition,
            Some(serde_json::to_value(avant)?),
            Some(serde_json::to_value(apres)?),
            origine,
        );
        unite.emettre(
            evenement,
            "evaluation",
            &id,
            serde_json::json!({ "statut": apres.statut, "motif": apres.motif_deverrouillage }),
        );
        self.noyau.terminer(unite).await?;
        Ok(())
    }
}

/// Grille remplie par l'enseignant, à importer dans une évaluation (SPEC_PHASE_4 §4.5).
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeImportNotes {
    /// Évaluation.
    pub evaluation_id: Id<Evaluation>,
    /// Classeur ou CSV choisi par l'utilisateur.
    pub chemin: String,
}

/// Ce que l'import ferait d'une ligne du fichier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum EtatImportNote {
    /// L'élève n'avait pas de note : elle sera enregistrée.
    Nouvelle,
    /// La note change.
    Modifiee,
    /// Même note qu'aujourd'hui.
    Inchangee,
    /// Case vide : rien ne change, la note actuelle reste.
    Vide,
    /// Illisible, hors barème ou en double : la ligne est laissée de côté.
    Erreur,
}

/// Ligne du fichier, rapprochée de la grille.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct LigneImportNote {
    /// Numéro de la ligne dans le fichier.
    pub ligne: u32,
    /// Matricule lu.
    pub matricule: String,
    /// Élève de la grille.
    pub eleve: String,
    /// Note actuelle.
    pub avant: String,
    /// Note du fichier, telle qu'elle sera écrite.
    pub apres: String,
    /// Ce que l'import en fera.
    pub etat: EtatImportNote,
    /// Clé de l'erreur.
    pub erreur: Option<String>,
}

/// Aperçu, puis bilan, d'un import de notes.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ApercuImportNotes {
    /// Lignes reconnues.
    pub lignes: Vec<LigneImportNote>,
    /// Matricules du fichier qui ne sont pas dans la classe.
    pub inconnus: Vec<String>,
    /// Élèves de la classe absents du fichier.
    pub absents_du_fichier: Vec<String>,
    /// Notes à écrire (ou écrites).
    pub a_ecrire: u32,
    /// Lignes en erreur.
    pub erreurs: u32,
}

/// Notes à écrire : (inscription, note).
type Ecritures = Vec<(Id<Inscription>, NoteEleve)>;

impl ServiceEvaluations<'_> {
    /// Lit le fichier et le rapproche de la grille : rien n'est écrit.
    async fn rapprocher(
        &self,
        session: &Session,
        demande: &DemandeImportNotes,
    ) -> Resultat<(GrilleNotes, ApercuImportNotes, Ecritures)> {
        let grille = self.grille(session, demande.evaluation_id).await?;
        if !grille.modifiable {
            return Err(regle("evaluation.validee"));
        }
        let chemin = demande.chemin.clone();
        let feuilles =
            tokio::task::spawn_blocking(move || crate::import_eleves::lire_fichier(&chemin))
                .await
                .map_err(|e| ErreurService::Fichier(std::io::Error::other(e)))??;
        // La première feuille dont une ligne porte « Matricule » et une colonne « Note… ».
        let trouvee = feuilles.iter().find_map(|(_, lignes)| {
            lignes.iter().enumerate().find_map(|(rang, cellules)| {
                let entetes: Vec<String> = cellules
                    .iter()
                    .map(|c| gesco_core::scolarite::recherche::normaliser(c))
                    .collect();
                let m = entetes.iter().position(|e| e == "MATRICULE")?;
                let n = entetes.iter().position(|e| e.starts_with("NOTE"))?;
                Some((lignes, rang, m, n))
            })
        });
        let (lignes, rang_entete, col_matricule, col_note) =
            trouvee.ok_or(regle("import_notes.colonnes_introuvables"))?;
        let mut apercu = ApercuImportNotes {
            lignes: Vec::new(),
            inconnus: Vec::new(),
            absents_du_fichier: Vec::new(),
            a_ecrire: 0,
            erreurs: 0,
        };
        let mut a_ecrire = Vec::new();
        let mut vus: Vec<Id<Inscription>> = Vec::new();
        for (index, cellules) in lignes.iter().enumerate().skip(rang_entete + 1) {
            let matricule = cellules.get(col_matricule).map_or("", |c| c.trim());
            if matricule.is_empty() {
                continue;
            }
            let Some(eleve) = grille
                .lignes
                .iter()
                .find(|l| l.matricule.trim().eq_ignore_ascii_case(matricule))
            else {
                apercu.inconnus.push(matricule.to_owned());
                continue;
            };
            let texte = cellules.get(col_note).map_or("", |c| c.trim());
            let avant = eleve
                .note
                .as_ref()
                .map(gesco_core::pedagogie::evaluation::texte_note)
                .unwrap_or_default();
            let mut ligne = LigneImportNote {
                ligne: u32::try_from(index + 1).unwrap_or(u32::MAX),
                matricule: matricule.to_owned(),
                eleve: eleve.eleve.clone(),
                avant: avant.clone(),
                apres: String::new(),
                etat: EtatImportNote::Vide,
                erreur: None,
            };
            if vus.contains(&eleve.inscription_id) {
                ligne.etat = EtatImportNote::Erreur;
                ligne.erreur = Some("import_notes.matricule_en_double".to_owned());
            } else if !texte.is_empty() {
                match lire_saisie(texte, grille.evaluation.bareme) {
                    Err(cle) => {
                        ligne.etat = EtatImportNote::Erreur;
                        ligne.erreur = Some(cle.to_owned());
                        ligne.apres = texte.to_owned();
                    }
                    Ok(saisie) => {
                        if let Some(note) =
                            saisie.en_note(grille.evaluation.id, eleve.inscription_id)
                        {
                            ligne.apres = gesco_core::pedagogie::evaluation::texte_note(&note);
                            ligne.etat = if avant.is_empty() {
                                EtatImportNote::Nouvelle
                            } else if avant == ligne.apres {
                                EtatImportNote::Inchangee
                            } else {
                                EtatImportNote::Modifiee
                            };
                            if matches!(
                                ligne.etat,
                                EtatImportNote::Nouvelle | EtatImportNote::Modifiee
                            ) {
                                a_ecrire.push((eleve.inscription_id, note));
                            }
                        }
                    }
                }
            }
            vus.push(eleve.inscription_id);
            if ligne.etat == EtatImportNote::Erreur {
                apercu.erreurs += 1;
            }
            apercu.lignes.push(ligne);
        }
        apercu.absents_du_fichier = grille
            .lignes
            .iter()
            .filter(|l| !vus.contains(&l.inscription_id))
            .map(|l| l.eleve.clone())
            .collect();
        apercu.a_ecrire = u32::try_from(a_ecrire.len()).unwrap_or(u32::MAX);
        Ok((grille, apercu, a_ecrire))
    }

    /// Aperçu d'un import : ce qui serait écrit, ce qui est en erreur, sans rien écrire.
    pub async fn analyser_import(
        &self,
        session: &Session,
        demande: &DemandeImportNotes,
    ) -> Resultat<ApercuImportNotes> {
        Ok(self.rapprocher(session, demande).await?.1)
    }

    /// Écrit les notes nouvelles ou modifiées du fichier dans une seule transaction, chacune
    /// auditée ; les lignes en erreur et les cases vides sont laissées de côté.
    pub async fn importer(
        &self,
        session: &Session,
        demande: &DemandeImportNotes,
    ) -> Resultat<ApercuImportNotes> {
        let (grille, apercu, a_ecrire) = self.rapprocher(session, demande).await?;
        if a_ecrire.is_empty() {
            return Ok(apercu);
        }
        let e = &grille.evaluation;
        annees::exiger_modifiable(self.noyau, session, e.annee_id).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        {
            let mut cnx = self.connexion().await?;
            if pedagogie::periodes(&mut cnx, e.annee_id, &contexte)
                .await?
                .iter()
                .any(|p| p.id == e.periode_id && p.verrouillee)
            {
                return Err(regle("periode.verrouillee"));
            }
        }
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        for (inscription_id, note) in &a_ecrire {
            let avant = grille
                .lignes
                .iter()
                .find(|l| l.inscription_id == *inscription_id)
                .and_then(|l| l.note);
            evaluations::ecrire_note(
                unite.connexion(),
                e.id,
                *inscription_id,
                Some(note),
                &contexte,
            )
            .await?;
            unite.auditer(
                "note",
                &format!("{}:{}", e.id, inscription_id),
                if avant.is_some() {
                    Operation::Modification
                } else {
                    Operation::Creation
                },
                avant.as_ref().map(serde_json::to_value).transpose()?,
                Some(serde_json::to_value(note)?),
                "evaluations.importer",
            );
        }
        unite.emettre(
            TypeEvenement::GrilleNotesImportee,
            "evaluation",
            &e.id.vers_chaine(),
            serde_json::json!({ "notes": a_ecrire.len(), "erreurs": apercu.erreurs }),
        );
        self.noyau.terminer(unite).await?;
        Ok(apercu)
    }
}
