//! Reporter au bulletin (SPEC_PHASE_12_LOT_5 §6, ADR « report d'une note de devoir au
//! bulletin ») : d'un geste, une évaluation « devoir » ordinaire, en saisie, marquée « devoir en
//! ligne » et liée au devoir, porte les notes du devoir ; ensuite tout suit les règles de la
//! phase 4 (validation, déverrouillage, calcul). Sans ce geste, aucune note de devoir n'entre au
//! bulletin.
//!
//! Un élève qui n'a pas de note dans le devoir reste sans note dans l'évaluation : le maître le
//! complète là comme d'habitude, et un nouveau report n'efface pas ce qu'il y a saisi.

use std::collections::HashMap;

use gesco_core::commun::{DateCivile, Id};
use gesco_core::core::{Operation, Permission, TypeEvenement};
use gesco_core::pedagogie::evaluation::verifier_evaluation;
use gesco_core::pedagogie::{Evaluation, NoteEleve, Periode, StatutEvaluation, TypeEvaluation};
use gesco_db::depots::devoirs::{self as depot, EvaluationRecrite, LigneDevoir, LigneRendu};
use gesco_db::depots::{academique, enseignants, evaluations, pedagogie};
use gesco_db::unite::Durabilite;
use gesco_db::{ErreurBase, nouvel_id};
use serde::{Deserialize, Serialize};

use super::{EtatDevoir, ServiceDevoirs, regle};
use crate::enseignants::ServiceEnseignants;
use crate::erreur::{ErreurService, Resultat};
use crate::evaluations::ServiceEvaluations;
use crate::session::Session;

/// Libellé d'une évaluation, au plus (règle des évaluations).
const LIBELLE_MAX: usize = 60;

/// Une période où reporter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct PeriodeReport {
    /// Identifiant.
    pub id: String,
    /// « 1er trimestre ».
    pub libelle: String,
}

/// L'évaluation née d'un report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct EvaluationReportee {
    /// Identifiant.
    pub id: String,
    /// Période.
    pub periode_id: String,
    /// Validée par la direction : elle ne bouge plus sans déverrouillage.
    pub validee: bool,
    /// Pondération.
    pub ponderation: i32,
}

/// Ce que la confirmation du report dit, et où en est le report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct ReportDevoir {
    /// Le compte peut reporter (création des évaluations de cette classe et de cette matière).
    pub peut_reporter: bool,
    /// Périodes de la classe.
    pub periodes: Vec<PeriodeReport>,
    /// Période proposée : celle de l'évaluation déjà reportée, sinon celle de la date limite.
    pub periode_id: Option<String>,
    /// Barème en centièmes (celui du devoir).
    pub bareme: i32,
    /// Pondération proposée.
    pub ponderation: i32,
    /// Notes qui entreront.
    pub notes: u32,
    /// L'évaluation déjà reportée.
    pub evaluation: Option<EvaluationReportee>,
    /// Une note a changé dans le devoir depuis le dernier report.
    pub notes_changees: bool,
}

/// Reporter (ou reporter de nouveau) un devoir.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeReportDevoir {
    /// Devoir.
    pub id: String,
    /// Période.
    pub periode_id: String,
    /// Pondération (1 à 10).
    pub ponderation: i32,
}

/// Jour d'une date limite (millisecondes).
fn jour_de(millis: i64) -> Option<DateCivile> {
    DateCivile::depuis_jours_epoque(millis.div_euclid(86_400_000))
}

/// Période proposée pour une date limite : celle qui la contient ; sinon la dernière commencée ;
/// sinon la première.
pub(crate) fn periode_de(periodes: &[Periode], jour: Option<DateCivile>) -> Option<&Periode> {
    let Some(jour) = jour else {
        return periodes.first();
    };
    periodes
        .iter()
        .find(|p| p.debut <= jour && jour <= p.fin)
        .or_else(|| periodes.iter().rev().find(|p| p.debut <= jour))
        .or_else(|| periodes.first())
}

/// Jour de l'évaluation : la date limite, ramenée dans la période choisie.
pub(crate) fn jour_dans(periode: &Periode, jour: Option<DateCivile>) -> DateCivile {
    match jour {
        Some(j) if j < periode.debut => periode.debut,
        Some(j) if j > periode.fin => periode.fin,
        Some(j) => j,
        None => periode.fin,
    }
}

/// Libellé de l'évaluation : le titre du devoir, coupé à 60 caractères.
pub(crate) fn libelle_de(titre: &str) -> String {
    let titre = titre.trim();
    if titre.chars().count() <= LIBELLE_MAX {
        return titre.to_owned();
    }
    let coupe: String = titre.chars().take(LIBELLE_MAX - 1).collect();
    format!("{}…", coupe.trim_end())
}

/// Note du devoir de chaque élève : celle de son dernier rendu ; et l'instant où elle a été
/// mise.
pub(crate) fn notes_du_devoir(rendus: &[LigneRendu]) -> HashMap<String, (i64, Option<i64>)> {
    let mut derniers: HashMap<&str, &LigneRendu> = HashMap::new();
    for r in rendus {
        let e = derniers.entry(r.eleve_id.as_str()).or_insert(r);
        if r.numero > e.numero {
            *e = r;
        }
    }
    derniers
        .into_iter()
        .filter_map(|(eleve, r)| r.note.map(|n| (eleve.to_owned(), (n, r.note_le))))
        .collect()
}

/// Une note a été mise ou changée après le dernier report.
pub(crate) fn changees_depuis(
    notes: &HashMap<String, (i64, Option<i64>)>,
    reporte_le: Option<i64>,
) -> bool {
    reporte_le.is_some_and(|r| notes.values().any(|(_, le)| le.is_some_and(|l| l > r)))
}

/// Ce que le service sait de la classe, des périodes et de l'évaluation reportée.
struct Lieu {
    classe: gesco_core::scolarite::Classe,
    periodes: Vec<Periode>,
    evaluation: Option<Evaluation>,
    peut_reporter: bool,
}

impl ServiceDevoirs<'_> {
    async fn lieu_du_report(&self, session: &Session, devoir: &LigneDevoir) -> Resultat<Lieu> {
        let contexte = self.contexte(session);
        let introuvable = |entite: &'static str, id: &str| {
            ErreurService::from(ErreurBase::Introuvable {
                entite,
                id: id.to_owned(),
            })
        };
        let classe_id = Id::analyser(&devoir.classe_id)
            .map_err(|_| introuvable("classe", &devoir.classe_id))?;
        let matiere_id = Id::analyser(&devoir.matiere_id)
            .map_err(|_| introuvable("matiere", &devoir.matiere_id))?;
        let droits = self
            .noyau
            .droits_de(session.utilisateur_id, &contexte)
            .await?;
        let portee = ServiceEnseignants::nouveau(self.noyau)
            .portee_notes(session)
            .await?;
        let peut_reporter = droits.permissions.contains(&Permission::GRADES_CREATE)
            && (!droits.maitre || droits.saisie_admise)
            && portee.autorise(classe_id, matiere_id);
        let mut cnx = self.connexion().await?;
        let classe = academique::classe(&mut cnx, classe_id, &contexte).await?;
        let periodes: Vec<Periode> = pedagogie::periodes(&mut cnx, classe.annee_id, &contexte)
            .await?
            .into_iter()
            .filter(|p| p.niveau_id == classe.niveau_id)
            .collect();
        let evaluation = match depot::evaluation_du_devoir(&mut cnx, &devoir.id, &contexte).await? {
            Some(id) => Some(
                evaluations::lire(
                    &mut cnx,
                    Id::analyser(&id).map_err(|_| introuvable("evaluation", &id))?,
                    &contexte,
                )
                .await?,
            ),
            None => None,
        };
        Ok(Lieu {
            classe,
            periodes,
            evaluation,
            peut_reporter,
        })
    }

    /// Notes du devoir qui entreront, par inscription : (inscription, note) pour chaque élève
    /// inscrit dans la classe le jour de l'évaluation qui a une note.
    async fn notes_a_reporter(
        &self,
        session: &Session,
        classe: &gesco_core::scolarite::Classe,
        jour: DateCivile,
        notes: &HashMap<String, (i64, Option<i64>)>,
    ) -> Resultat<Vec<(Id<gesco_core::scolarite::Inscription>, i64)>> {
        Ok(ServiceEvaluations::nouveau(self.noyau)
            .inscrits_au(session, classe, jour)
            .await?
            .into_iter()
            .filter_map(|(i, eleve)| notes.get(&eleve.vers_chaine()).map(|(n, _)| (i, *n)))
            .collect())
    }

    /// Ce que dit la confirmation : période proposée, barème, pondération, nombre de notes ; et
    /// l'évaluation déjà reportée, avec « notes changées depuis le report ».
    pub async fn report(&self, session: &Session, id: &str) -> Resultat<ReportDevoir> {
        self.exiger_module(session).await?;
        let (devoir, _) = self.devoir_permis(session, id).await?;
        let lieu = self.lieu_du_report(session, &devoir).await?;
        let contexte = self.contexte(session);
        let rendus = {
            let mut cnx = self.connexion().await?;
            depot::rendus_du_devoir(&mut cnx, &devoir.id, &contexte).await?
        };
        let notes = notes_du_devoir(&rendus);
        let jour = jour_de(devoir.date_limite);
        let periode = match &lieu.evaluation {
            Some(e) => lieu.periodes.iter().find(|p| p.id == e.periode_id),
            None => periode_de(&lieu.periodes, jour),
        };
        let nombre = match periode {
            Some(p) => self
                .notes_a_reporter(session, &lieu.classe, jour_dans(p, jour), &notes)
                .await?
                .len(),
            None => 0,
        };
        Ok(ReportDevoir {
            peut_reporter: lieu.peut_reporter
                && EtatDevoir::depuis_nom(&devoir.etat) != EtatDevoir::Brouillon,
            periodes: lieu
                .periodes
                .iter()
                .map(|p| PeriodeReport {
                    id: p.id.vers_chaine(),
                    libelle: p.libelle(),
                })
                .collect(),
            periode_id: periode.map(|p| p.id.vers_chaine()),
            bareme: i32::try_from(devoir.bareme).unwrap_or(super::BAREME_DEFAUT),
            ponderation: lieu.evaluation.as_ref().map_or(1, |e| e.ponderation),
            notes: u32::try_from(nombre).unwrap_or(u32::MAX),
            evaluation: lieu.evaluation.as_ref().map(|e| EvaluationReportee {
                id: e.id.vers_chaine(),
                periode_id: e.periode_id.vers_chaine(),
                validee: e.statut == StatutEvaluation::Validee,
                ponderation: e.ponderation,
            }),
            notes_changees: lieu.evaluation.is_some() && changees_depuis(&notes, devoir.reporte_le),
        })
    }

    /// « Reporter au bulletin », ou « Reporter de nouveau » tant que l'évaluation est en saisie.
    pub async fn reporter(
        &self,
        session: &Session,
        demande: &DemandeReportDevoir,
    ) -> Resultat<ReportDevoir> {
        self.exiger_module(session).await?;
        let (devoir, _) = self.devoir_permis(session, &demande.id).await?;
        let lieu = self.lieu_du_report(session, &devoir).await?;
        if !lieu.peut_reporter {
            return Err(ErreurService::PorteeRefusee {
                ressource: format!("devoir {}", devoir.id),
            });
        }
        if EtatDevoir::depuis_nom(&devoir.etat) == EtatDevoir::Brouillon {
            return Err(regle("devoir.pas_donne"));
        }
        if lieu
            .evaluation
            .as_ref()
            .is_some_and(|e| e.statut == StatutEvaluation::Validee)
        {
            return Err(regle("devoir.evaluation_validee"));
        }
        crate::annees::exiger_classe_modifiable(self.noyau, session, lieu.classe.id).await?;
        let periode = lieu
            .periodes
            .iter()
            .find(|p| p.id.vers_chaine() == demande.periode_id)
            .ok_or(regle("evaluation.periode_invalide"))?
            .clone();
        let (_, regle_cycle) = ServiceEvaluations::nouveau(self.noyau)
            .periode_et_regle(session, &lieu.classe, periode.id)
            .await?;
        let bareme = i32::try_from(devoir.bareme).unwrap_or(super::BAREME_DEFAUT);
        let libelle = libelle_de(&devoir.titre);
        let jour = jour_dans(&periode, jour_de(devoir.date_limite));
        verifier_evaluation(
            &regle_cycle.contenu,
            &periode,
            TypeEvaluation::Devoir,
            &libelle,
            jour,
            bareme,
            demande.ponderation,
        )
        .map_err(regle)?;
        let contexte = self.contexte(session);
        let rendus = {
            let mut cnx = self.connexion().await?;
            depot::rendus_du_devoir(&mut cnx, &devoir.id, &contexte).await?
        };
        let notes = notes_du_devoir(&rendus);
        let a_reporter = self
            .notes_a_reporter(session, &lieu.classe, jour, &notes)
            .await?;
        if a_reporter.iter().any(|(_, n)| *n > i64::from(bareme)) {
            return Err(regle("devoir.notes_hors_bareme"));
        }
        let enseignant = {
            let mut cnx = self.connexion().await?;
            enseignants::affectations(&mut cnx, lieu.classe.annee_id, &contexte)
                .await?
                .into_iter()
                .find(|a| {
                    a.classe_id == lieu.classe.id && a.matiere_id.vers_chaine() == devoir.matiere_id
                })
                .map(|a| a.enseignant_id)
        };
        let evaluation = match &lieu.evaluation {
            Some(e) => Evaluation {
                periode_id: periode.id,
                date: jour,
                libelle: libelle.clone(),
                bareme,
                ponderation: demande.ponderation,
                ..e.clone()
            },
            None => Evaluation {
                id: nouvel_id(),
                etablissement_id: session.etablissement_id,
                annee_id: lieu.classe.annee_id,
                periode_id: periode.id,
                classe_id: lieu.classe.id,
                matiere_id: Id::analyser(&devoir.matiere_id).map_err(|_| {
                    ErreurService::from(ErreurBase::Introuvable {
                        entite: "matiere",
                        id: devoir.matiere_id.clone(),
                    })
                })?,
                type_evaluation: TypeEvaluation::Devoir,
                libelle: libelle.clone(),
                date: jour,
                bareme,
                ponderation: demande.ponderation,
                enseignant_id: enseignant,
                statut: StatutEvaluation::EnSaisie,
                motif_deverrouillage: String::new(),
            },
        };
        let deja: Vec<NoteEleve> = match &lieu.evaluation {
            Some(e) => {
                let mut cnx = self.connexion().await?;
                evaluations::notes(&mut cnx, e.id, &contexte).await?
            }
            None => Vec::new(),
        };
        let eval_id = evaluation.id.vers_chaine();
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        match &lieu.evaluation {
            Some(avant) => {
                depot::recrire_evaluation_reportee(
                    unite.connexion(),
                    &EvaluationRecrite {
                        id: &eval_id,
                        periode_id: &periode.id.vers_chaine(),
                        date: jour.to_string(),
                        libelle: &libelle,
                        bareme,
                        ponderation: demande.ponderation,
                    },
                    &contexte,
                )
                .await?;
                unite.auditer(
                    "evaluation",
                    &eval_id,
                    Operation::Modification,
                    Some(serde_json::to_value(avant)?),
                    Some(serde_json::to_value(&evaluation)?),
                    "devoirs.reporter",
                );
            }
            None => {
                evaluations::inserer(unite.connexion(), &evaluation, &contexte).await?;
                depot::lier_evaluation(unite.connexion(), &eval_id, &devoir.id, &contexte).await?;
                unite.auditer(
                    "evaluation",
                    &eval_id,
                    Operation::Creation,
                    None,
                    Some(serde_json::to_value(&evaluation)?),
                    "devoirs.reporter",
                );
                unite.emettre(
                    TypeEvenement::EvaluationCreee,
                    "evaluation",
                    &eval_id,
                    serde_json::json!({ "classe_id": evaluation.classe_id,
                        "matiere_id": evaluation.matiere_id, "type": evaluation.type_evaluation }),
                );
            }
        }
        let mut ecrites = 0_u32;
        for (inscription, valeur) in &a_reporter {
            let note = NoteEleve {
                evaluation_id: evaluation.id,
                inscription_id: *inscription,
                valeur: Some(i32::try_from(*valeur).unwrap_or(bareme)),
                absent: false,
                justifie: false,
                dispense: false,
            };
            if deja.contains(&note) {
                continue;
            }
            evaluations::ecrire_note(
                unite.connexion(),
                evaluation.id,
                *inscription,
                Some(&note),
                &contexte,
            )
            .await?;
            unite.emettre(
                TypeEvenement::NoteSaisie,
                "evaluation",
                &eval_id,
                serde_json::json!({ "inscription_id": inscription, "retiree": false }),
            );
            ecrites += 1;
        }
        depot::marquer_reporte(unite.connexion(), &devoir.id, &contexte).await?;
        unite.auditer(
            "devoir",
            &devoir.id,
            Operation::Transition,
            Some(serde_json::json!({ "reporte_le": devoir.reporte_le })),
            Some(serde_json::json!({
                "evaluation_id": eval_id,
                "nouveau_report": lieu.evaluation.is_some(),
                "notes": a_reporter.len(),
                "notes_ecrites": ecrites,
            })),
            "devoirs.reporter",
        );
        self.noyau.terminer(unite).await?;
        self.report(session, &devoir.id).await
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn rendu(eleve: &str, numero: i64, note: Option<i64>, note_le: Option<i64>) -> LigneRendu {
        LigneRendu {
            id: format!("{eleve}{numero}"),
            devoir_id: "d".into(),
            eleve_id: eleve.into(),
            numero,
            origine: "portail".into(),
            texte: String::new(),
            rendu_le: 0,
            en_retard: false,
            note,
            commentaire: String::new(),
            note_le,
            note_par: None,
        }
    }

    #[test]
    fn seul_le_dernier_rendu_note_entre_au_report() {
        let rendus = [
            rendu("a", 1, Some(1200), Some(10)),
            rendu("a", 2, Some(1400), Some(30)),
            rendu("b", 1, Some(900), Some(10)),
            rendu("b", 2, None, None),
            rendu("c", 1, None, None),
        ];
        let notes = notes_du_devoir(&rendus);
        assert_eq!(
            notes.len(),
            1,
            "b a rendu de nouveau sans note, c n'a pas de note"
        );
        assert_eq!(notes.get("a"), Some(&(1400, Some(30))));
        assert!(!changees_depuis(&notes, None));
        assert!(changees_depuis(&notes, Some(20)));
        assert!(!changees_depuis(&notes, Some(30)));
    }

    #[test]
    fn le_libelle_tient_en_soixante_caracteres() {
        assert_eq!(libelle_de("  Racines carrées "), "Racines carrées");
        let long = "a".repeat(80);
        let l = libelle_de(&long);
        assert_eq!(l.chars().count(), 60);
        assert!(l.ends_with('…'));
    }

    #[test]
    fn la_periode_proposee_est_celle_de_la_date_limite() {
        let jour = |a, m, j| DateCivile::nouvelle(a, m, j).expect("date");
        let periode = |rang: i64, debut, fin| Periode {
            id: Id::analyser(&format!("0000000000000000000000000{rang}")).expect("id"),
            etablissement_id: Id::analyser("00000000000000000000000000").expect("id"),
            annee_id: Id::analyser("00000000000000000000000000").expect("id"),
            niveau_id: Id::analyser("00000000000000000000000000").expect("id"),
            rang,
            type_periode: gesco_core::pedagogie::TypePeriode::Trimestre,
            debut,
            fin,
            ponderation: 1,
            verrouillee: false,
        };
        let periodes = [
            periode(1, jour(2026, 10, 1), jour(2026, 12, 20)),
            periode(2, jour(2027, 1, 5), jour(2027, 3, 31)),
        ];
        let dans = periode_de(&periodes, Some(jour(2026, 11, 15))).expect("période");
        assert_eq!(dans.rang, 1);
        // Pendant les vacances : la dernière période commencée, le jour ramené à sa fin.
        let vacances = Some(jour(2026, 12, 28));
        let p = periode_de(&periodes, vacances).expect("période");
        assert_eq!(p.rang, 1);
        assert_eq!(jour_dans(p, vacances), jour(2026, 12, 20));
        // Avant l'année : la première, le jour ramené à son début.
        let avant = Some(jour(2026, 9, 1));
        let p = periode_de(&periodes, avant).expect("période");
        assert_eq!(p.rang, 1);
        assert_eq!(jour_dans(p, avant), jour(2026, 10, 1));
    }
}
