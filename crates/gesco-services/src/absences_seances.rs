//! Absences par séance (SPEC_VERSION_1_1 §1, étape 6, T-71) : quand l'école le règle, l'absence
//! se relève à chaque séance de l'emploi du temps. Absent à toutes les séances d'une
//! demi-journée, l'élève compte la demi-journée au relevé du jour, qui se justifie comme avant ;
//! un retard noté reste.

use gesco_core::commun::{DateCivile, Id};
use gesco_core::core::{Operation, TypeEvenement};
use gesco_core::pedagogie::emploi_du_temps::{Seance, demi_journees_absentes};
use gesco_core::pedagogie::{Absence, NatureAbsence};
use gesco_core::scolarite::{Classe, Inscription};
use gesco_db::depots::{absences_seances, parametre};
use gesco_db::nouvel_id;
use gesco_db::unite::Durabilite;
use serde::{Deserialize, Serialize};

use crate::absences::{DemandeEnregistrementReleve, DemandeReleve, SaisieReleve, ServiceAbsences};
use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Réglage de l'école : l'absence se relève à chaque séance.
pub const PARAMETRE_PAR_SEANCE: &str = "absence.par_seance";

/// Séance du jour d'une classe.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct SeanceDuJour {
    /// Séance.
    pub seance_id: Id<Seance>,
    /// « 08:00 – 09:00 ».
    pub horaire: String,
    /// Matière.
    pub matiere: String,
    /// Enseignant.
    pub enseignant: String,
}

/// Élève du relevé par séance.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct EleveParSeance {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Prénoms et nom.
    pub eleve: String,
    /// Séances où il est noté absent.
    pub absent_a: Vec<Id<Seance>>,
    /// Exclu ce jour-là : pas compté absent.
    pub exclu: bool,
}

/// Relevé par séance d'une classe pour un jour.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ReleveParSeance {
    /// L'école relève par séance.
    pub actif: bool,
    /// Jour.
    pub date: DateCivile,
    /// Séances du jour, dans l'ordre des heures.
    pub seances: Vec<SeanceDuJour>,
    /// Élèves.
    pub lignes: Vec<EleveParSeance>,
    /// Clé de la raison pour laquelle ce jour ne se relève pas.
    pub refus: Option<String>,
}

/// Cases d'un élève : les séances où il est absent.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct SaisieParSeance {
    /// Élève.
    pub inscription_id: Id<Inscription>,
    /// Séances où il est absent.
    pub absent_a: Vec<Id<Seance>>,
}

/// Relevé par séance à enregistrer.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeReleveParSeance {
    /// Classe.
    pub classe_id: Id<Classe>,
    /// Jour.
    pub date: DateCivile,
    /// Cases de chaque élève ; un élève omis garde ce qui est enregistré.
    pub lignes: Vec<SaisieParSeance>,
}

/// Case d'une demi-journée telle que le relevé du jour la tape : `a`, `r`, `r15` ou rien.
fn code(case: Option<&Absence>) -> String {
    match case {
        Some(a) if a.nature == NatureAbsence::Absence => "a".to_owned(),
        Some(a) => a
            .minutes
            .map_or_else(|| "r".to_owned(), |m| format!("r{m}")),
        None => String::new(),
    }
}

/// Service du relevé par séance.
pub struct ServiceAbsencesSeances<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceAbsencesSeances<'n> {
    /// Service sur le noyau.
    pub fn nouveau(noyau: &'n Noyau) -> Self {
        Self { noyau }
    }

    async fn actif(&self, session: &Session) -> Resultat<bool> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self
            .noyau
            .base()
            .pool()
            .acquire()
            .await
            .map_err(gesco_db::ErreurBase::from)?;
        Ok(parametre::booleen_param(&mut cnx, PARAMETRE_PAR_SEANCE, false, &contexte).await?)
    }

    /// Relevé par séance d'une classe pour un jour.
    pub async fn lire(
        &self,
        session: &Session,
        demande: &DemandeReleve,
    ) -> Resultat<ReleveParSeance> {
        let actif = self.actif(session).await?;
        let releve = ServiceAbsences::nouveau(self.noyau)
            .releve(session, demande)
            .await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self
            .noyau
            .base()
            .pool()
            .acquire()
            .await
            .map_err(gesco_db::ErreurBase::from)?;
        let (_, du_jour) = crate::emploi_du_temps::seances_du_jour(
            &mut cnx,
            demande.classe_id,
            demande.date,
            &contexte,
        )
        .await?;
        let absents = absences_seances::absents_du_jour(&mut cnx, demande.date, &contexte).await?;
        Ok(ReleveParSeance {
            actif,
            date: demande.date,
            lignes: releve
                .lignes
                .into_iter()
                .map(|l| EleveParSeance {
                    absent_a: absents
                        .iter()
                        .filter(|(i, s)| {
                            *i == l.inscription_id && du_jour.iter().any(|d| d.seance_id == *s)
                        })
                        .map(|(_, s)| *s)
                        .collect(),
                    inscription_id: l.inscription_id,
                    eleve: l.eleve,
                    exclu: l.exclu,
                })
                .collect(),
            seances: du_jour,
            refus: releve.refus,
        })
    }

    /// Enregistre le relevé par séance d'un jour, puis la demi-journée qui s'en déduit au relevé
    /// du jour ; un retard ou une absence justifiée déjà notés restent.
    pub async fn enregistrer(
        &self,
        session: &Session,
        demande: &DemandeReleveParSeance,
    ) -> Resultat<ReleveParSeance> {
        if !self.actif(session).await? {
            return Err(regle("absence.par_seance_inactif"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let lecture = DemandeReleve {
            classe_id: demande.classe_id,
            date: demande.date,
        };
        let releve = ServiceAbsences::nouveau(self.noyau)
            .releve(session, &lecture)
            .await?;
        let (journee, seances) = {
            let mut cnx = self
                .noyau
                .base()
                .pool()
                .acquire()
                .await
                .map_err(gesco_db::ErreurBase::from)?;
            crate::emploi_du_temps::seances_entieres_du_jour(
                &mut cnx,
                demande.classe_id,
                demande.date,
                &contexte,
            )
            .await?
        };
        let Some(journee) = journee else {
            return Err(regle("absence.sans_emploi_du_temps"));
        };
        if seances.is_empty() {
            return Err(regle("absence.sans_seance"));
        }
        let refs: Vec<&Seance> = seances.iter().collect();
        // La demi-journée au relevé du jour : il vérifie le jour, les droits et l'année.
        let mut lignes = Vec::new();
        for saisie in &demande.lignes {
            if saisie
                .absent_a
                .iter()
                .any(|s| !seances.iter().any(|x| x.id == *s))
            {
                return Err(regle("absence.seance_hors_jour"));
            }
            let Some(l) = releve
                .lignes
                .iter()
                .find(|l| l.inscription_id == saisie.inscription_id)
            else {
                return Err(regle("absence.eleve_hors_classe"));
            };
            let (matin, apres_midi) =
                demi_journees_absentes(&journee, demande.date, &refs, &saisie.absent_a);
            let garde = |case: Option<&Absence>, absent: bool| match case {
                Some(a) if a.justifiee => code(Some(a)),
                Some(a) if a.nature == NatureAbsence::Retard && !absent => code(Some(a)),
                _ if absent => "a".to_owned(),
                _ => String::new(),
            };
            lignes.push(SaisieReleve {
                inscription_id: saisie.inscription_id,
                matin: garde(l.matin.as_ref(), matin),
                apres_midi: garde(l.apres_midi.as_ref(), apres_midi),
            });
        }
        ServiceAbsences::nouveau(self.noyau)
            .enregistrer(
                session,
                &DemandeEnregistrementReleve {
                    classe_id: demande.classe_id,
                    date: demande.date,
                    lignes,
                },
            )
            .await?;
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        for saisie in &demande.lignes {
            for s in &seances {
                let id: Id<SeanceDuJour> = nouvel_id();
                absences_seances::marquer(
                    unite.connexion(),
                    &id.vers_chaine(),
                    saisie.inscription_id,
                    demande.date,
                    s.id,
                    saisie.absent_a.contains(&s.id),
                    &contexte,
                )
                .await?;
            }
        }
        let cle = format!("{}:{}", demande.classe_id.vers_chaine(), demande.date);
        unite.auditer(
            "absence_seance",
            &cle,
            Operation::Modification,
            None,
            Some(serde_json::json!({
                "absences": demande.lignes.iter().map(|l| serde_json::json!({
                    "inscription_id": l.inscription_id, "absent_a": l.absent_a })).collect::<Vec<_>>(),
            })),
            "absences.par_seance",
        );
        unite.emettre(
            TypeEvenement::ReleveParSeanceEnregistre,
            "classe",
            &demande.classe_id.vers_chaine(),
            serde_json::json!({ "date": demande.date, "eleves": demande.lignes.len() }),
        );
        self.noyau.terminer(unite).await?;
        self.lire(session, &lecture).await
    }
}
