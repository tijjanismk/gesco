//! Absences et retards (SPEC_PHASE_4 §6, T-37) : le relevé d'une classe pour un jour, au clavier,
//! une case par demi-journée ; la justification après coup (`attendance:justify`) ; les totaux de
//! la période, que le bulletin reprend (T-32).

use gesco_core::commun::{DateCivile, Horodatage, Id};
use gesco_core::core::{Operation, Permission, TypeEvenement};
use gesco_core::pedagogie::assiduite::{lire_saisie_absence, totaux, verifier_jour};
use gesco_core::pedagogie::{Absence, DemiJournee, Periode, PorteeAbsences, TotauxAbsences};
use gesco_core::scolarite::{Classe, Inscription, StatutInscription};
use gesco_db::depots::{absences, academique, enseignants, pedagogie, role};
use gesco_db::unite::Durabilite;
use serde::{Deserialize, Serialize};

use crate::annees;
use crate::erreur::{ErreurService, Resultat};
use crate::inscriptions::ServiceInscriptions;
use crate::noyau::Noyau;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Relevé d'une classe pour un jour.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeReleve {
    /// Classe.
    pub classe_id: Id<Classe>,
    /// Jour.
    pub date: DateCivile,
}

/// Élève du relevé.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct LigneReleve {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Prénoms et nom.
    pub eleve: String,
    /// Matricule.
    pub matricule: String,
    /// Absence ou retard du matin.
    pub matin: Option<Absence>,
    /// Absence ou retard de l'après-midi.
    pub apres_midi: Option<Absence>,
    /// Vrai si une exclusion temporaire le tient hors de la classe ce jour-là : il n'est pas
    /// compté absent (réponse du 13 octobre 2026).
    pub exclu: bool,
}

/// Relevé du jour d'une classe.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ReleveAbsences {
    /// Libellé de la classe.
    pub classe: String,
    /// Jour.
    pub date: DateCivile,
    /// Élèves inscrits ce jour, dans l'ordre de la liste de classe.
    pub lignes: Vec<LigneReleve>,
    /// Quand le relevé a été enregistré, s'il l'a été.
    pub fait_le: Option<Horodatage>,
    /// Par qui.
    pub fait_par: String,
    /// Clé de la raison pour laquelle ce jour ne se relève pas : jour à venir, hors des périodes,
    /// période verrouillée.
    pub refus: Option<String>,
}

/// Cases d'un élève, telles que tapées.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct SaisieReleve {
    /// Élève.
    pub inscription_id: Id<Inscription>,
    /// Matin : rien, `a`, `r` ou `r15`.
    pub matin: String,
    /// Après-midi.
    pub apres_midi: String,
}

/// Enregistrement du relevé d'un jour.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeEnregistrementReleve {
    /// Classe.
    pub classe_id: Id<Classe>,
    /// Jour.
    pub date: DateCivile,
    /// Cases de chaque élève ; un élève omis garde ce qui est enregistré.
    pub lignes: Vec<SaisieReleve>,
}

/// Bilan d'un relevé enregistré.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct BilanReleve {
    /// Demi-journées d'absence relevées ce jour.
    pub absences: u32,
    /// Retards relevés ce jour.
    pub retards: u32,
    /// Cases changées par cet enregistrement.
    pub changees: u32,
}

/// Une demi-journée d'un élève.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, specta::Type)]
pub struct CleAbsence {
    /// Élève.
    pub inscription_id: Id<Inscription>,
    /// Jour.
    pub date: DateCivile,
    /// Matin ou après-midi.
    pub demi_journee: DemiJournee,
}

/// Justification d'absences ou de retards, d'un même motif.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeJustification {
    /// Demi-journées justifiées.
    pub absences: Vec<CleAbsence>,
    /// Motif, obligatoire.
    pub motif: String,
    /// Pièce présentée.
    pub justificatif: String,
}

/// Absences d'une classe sur une période.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeAbsencesPeriode {
    /// Classe.
    pub classe_id: Id<Classe>,
    /// Période.
    pub periode_id: Id<Periode>,
}

/// Absence affichée, avec l'élève.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct AbsenceAffichee {
    /// Absence ou retard.
    pub absence: Absence,
    /// Prénoms et nom.
    pub eleve: String,
    /// Matricule.
    pub matricule: String,
}

/// Totaux d'un élève sur la période.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct TotauxEleve {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Prénoms et nom.
    pub eleve: String,
    /// Matricule.
    pub matricule: String,
    /// Totaux.
    pub totaux: TotauxAbsences,
}

/// Absences d'une classe sur une période : le détail et les totaux par élève.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct AbsencesPeriode {
    /// Absences et retards, par date.
    pub absences: Vec<AbsenceAffichee>,
    /// Totaux de chaque élève, dans l'ordre de la liste de classe.
    pub totaux: Vec<TotauxEleve>,
    /// Jours de la période où le relevé de la classe a été fait.
    pub jours_releves: u32,
}

/// Case changée par un relevé : (élève, demi-journée, avant, après).
type Changement = (
    Id<Inscription>,
    DemiJournee,
    Option<Absence>,
    Option<Absence>,
);

/// Élève de la classe : (inscription, élève, matricule, date d'inscription).
type EleveDeClasse = (Id<Inscription>, String, String, DateCivile);

/// Service des absences.
pub struct ServiceAbsences<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceAbsences<'n> {
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

    /// Classes où l'utilisateur relève les absences : partout s'il justifie, sinon celles de ses
    /// affectations d'enseignant.
    pub async fn portee(&self, session: &Session) -> Resultat<PorteeAbsences> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let justifie = role::permissions_de(&mut cnx, session.utilisateur_id, &contexte)
            .await?
            .contains(&Permission::ATTENDANCE_JUSTIFY);
        let affectations =
            match enseignants::enseignant_du_compte(&mut cnx, session.utilisateur_id, &contexte)
                .await?
            {
                Some(e) => enseignants::affectations_de(&mut cnx, e.id, &contexte).await?,
                None => Vec::new(),
            };
        Ok(PorteeAbsences::de(justifie, &affectations))
    }

    async fn classe_permise(&self, session: &Session, id: Id<Classe>) -> Resultat<Classe> {
        if !self.portee(session).await?.autorise(id) {
            return Err(ErreurService::PorteeRefusee {
                ressource: format!("classe {id}"),
            });
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        Ok(academique::classe(&mut cnx, id, &contexte).await?)
    }

    /// Élèves validés de la classe, dans l'ordre de la liste.
    async fn eleves(&self, session: &Session, classe: &Classe) -> Resultat<Vec<EleveDeClasse>> {
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

    async fn periodes_du_niveau(
        &self,
        session: &Session,
        classe: &Classe,
    ) -> Resultat<Vec<Periode>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        Ok(pedagogie::periodes(&mut cnx, classe.annee_id, &contexte)
            .await?
            .into_iter()
            .filter(|p| p.niveau_id == classe.niveau_id)
            .collect())
    }

    async fn jour_ouvert(
        &self,
        session: &Session,
        classe: &Classe,
        date: DateCivile,
    ) -> Resultat<()> {
        let aujourd_hui = crate::inscriptions::jour(self.noyau.horloge().maintenant())?;
        verifier_jour(
            date,
            aujourd_hui,
            &self.periodes_du_niveau(session, classe).await?,
        )
        .map_err(regle)
    }

    /// Vrai si une exclusion temporaire tient l'élève hors de la classe ce jour-là.
    async fn exclu(
        &self,
        session: &Session,
        inscription_id: Id<Inscription>,
        jour: DateCivile,
    ) -> Resultat<bool> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let exclusions =
            gesco_db::depots::departs::exclusions(&mut cnx, inscription_id, &contexte).await?;
        Ok(gesco_core::scolarite::depart::exclu_le(&exclusions, jour))
    }

    /// Relevé d'une classe pour un jour : les élèves inscrits ce jour-là et leurs cases.
    pub async fn releve(
        &self,
        session: &Session,
        demande: &DemandeReleve,
    ) -> Resultat<ReleveAbsences> {
        let classe = self.classe_permise(session, demande.classe_id).await?;
        let refus = match self.jour_ouvert(session, &classe, demande.date).await {
            Ok(()) => None,
            Err(ErreurService::Regle { cle }) => Some(cle.to_owned()),
            Err(e) => return Err(e),
        };
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let (du_jour, fait) = {
            let mut cnx = self.connexion().await?;
            (
                absences::de_la_classe(&mut cnx, classe.id, demande.date, demande.date, &contexte)
                    .await?,
                absences::releve(&mut cnx, classe.id, demande.date, &contexte).await?,
            )
        };
        let case = |i: Id<Inscription>, d: DemiJournee| {
            du_jour
                .iter()
                .find(|a| a.inscription_id == i && a.demi_journee == d)
                .cloned()
        };
        let presents: Vec<_> = self
            .eleves(session, &classe)
            .await?
            .into_iter()
            .filter(|(_, _, _, inscrit_le)| *inscrit_le <= demande.date)
            .collect();
        let mut lignes = Vec::with_capacity(presents.len());
        for (inscription_id, eleve, matricule, _) in presents {
            lignes.push(LigneReleve {
                matin: case(inscription_id, DemiJournee::Matin),
                apres_midi: case(inscription_id, DemiJournee::ApresMidi),
                exclu: self.exclu(session, inscription_id, demande.date).await?,
                inscription_id,
                eleve,
                matricule,
            });
        }
        let (fait_le, fait_par) = fait.map_or((None, String::new()), |(le, par)| (Some(le), par));
        Ok(ReleveAbsences {
            classe: classe.libelle,
            date: demande.date,
            lignes,
            fait_le,
            fait_par,
            refus,
        })
    }

    /// Enregistre le relevé d'un jour en une transaction : chaque case changée est auditée, le
    /// relevé est noté fait même sans absent. Une absence déjà justifiée ne se change que par
    /// qui justifie.
    pub async fn enregistrer(
        &self,
        session: &Session,
        demande: &DemandeEnregistrementReleve,
    ) -> Resultat<BilanReleve> {
        let classe = self.classe_permise(session, demande.classe_id).await?;
        annees::exiger_modifiable(self.noyau, session, classe.annee_id).await?;
        self.jour_ouvert(session, &classe, demande.date).await?;
        let eleves = self.eleves(session, &classe).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let (avant, justifie) = {
            let mut cnx = self.connexion().await?;
            (
                absences::de_la_classe(&mut cnx, classe.id, demande.date, demande.date, &contexte)
                    .await?,
                role::permissions_de(&mut cnx, session.utilisateur_id, &contexte)
                    .await?
                    .contains(&Permission::ATTENDANCE_JUSTIFY),
            )
        };
        // Toutes les cases sont lues avant d'écrire : une seule illisible refuse le relevé.
        let mut changements: Vec<Changement> = Vec::new();
        for ligne in &demande.lignes {
            if !eleves.iter().any(|(i, _, _, inscrit_le)| {
                *i == ligne.inscription_id && *inscrit_le <= demande.date
            }) {
                return Err(regle("absence.eleve_hors_classe"));
            }
            let exclu = self
                .exclu(session, ligne.inscription_id, demande.date)
                .await?;
            for (demi_journee, texte) in [
                (DemiJournee::Matin, &ligne.matin),
                (DemiJournee::ApresMidi, &ligne.apres_midi),
            ] {
                let saisie = lire_saisie_absence(texte).map_err(regle)?;
                if exclu && saisie.nature().is_some() {
                    return Err(regle("absence.eleve_exclu"));
                }
                let ancienne = avant
                    .iter()
                    .find(|a| {
                        a.inscription_id == ligne.inscription_id && a.demi_journee == demi_journee
                    })
                    .cloned();
                let nouvelle = saisie.nature().map(|(nature, minutes)| {
                    // La même nature garde sa justification.
                    let garde = ancienne.as_ref().filter(|a| a.nature == nature);
                    Absence {
                        inscription_id: ligne.inscription_id,
                        classe_id: classe.id,
                        date: demande.date,
                        demi_journee,
                        nature,
                        minutes,
                        justifiee: garde.is_some_and(|a| a.justifiee),
                        motif: garde.map(|a| a.motif.clone()).unwrap_or_default(),
                        justificatif: garde.map(|a| a.justificatif.clone()).unwrap_or_default(),
                    }
                });
                if nouvelle == ancienne {
                    continue;
                }
                if ancienne.as_ref().is_some_and(|a| a.justifiee) && !justifie {
                    return Err(regle("absence.deja_justifiee"));
                }
                changements.push((ligne.inscription_id, demi_journee, ancienne, nouvelle));
            }
        }
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        for (inscription_id, demi_journee, ancienne, nouvelle) in &changements {
            absences::ecrire(
                unite.connexion(),
                *inscription_id,
                classe.id,
                demande.date,
                *demi_journee,
                nouvelle.as_ref(),
                &contexte,
            )
            .await?;
            unite.auditer(
                "absence",
                &format!("{inscription_id}:{}:{}", demande.date, demi_journee.nom()),
                match (ancienne, nouvelle) {
                    (None, _) => Operation::Creation,
                    (Some(_), None) => Operation::Suppression,
                    (Some(_), Some(_)) => Operation::Modification,
                },
                ancienne.as_ref().map(serde_json::to_value).transpose()?,
                nouvelle.as_ref().map(serde_json::to_value).transpose()?,
                "absences.enregistrer",
            );
        }
        absences::noter_releve(unite.connexion(), classe.id, demande.date, &contexte).await?;
        // L'avis d'absence du jour suit le relevé : créé, complété ou annulé.
        let mut vus = Vec::new();
        for (inscription_id, _, _, _) in &changements {
            if !vus.contains(inscription_id) {
                vus.push(*inscription_id);
                crate::messages_auto::actualiser_avis_absence(
                    unite.connexion(),
                    *inscription_id,
                    demande.date,
                    &contexte,
                )
                .await?;
            }
        }
        // Ce qui reste relevé ce jour, une fois les changements appliqués.
        let mut du_jour: Vec<Absence> = avant
            .into_iter()
            .filter(|a| {
                !changements
                    .iter()
                    .any(|(i, d, _, _)| *i == a.inscription_id && *d == a.demi_journee)
            })
            .collect();
        du_jour.extend(changements.iter().filter_map(|(_, _, _, n)| n.clone()));
        let t = totaux(&du_jour, demande.date, demande.date);
        let bilan = BilanReleve {
            absences: t.demi_journees,
            retards: t.retards,
            changees: u32::try_from(changements.len()).unwrap_or(u32::MAX),
        };
        unite.emettre(
            TypeEvenement::AbsenceRelevee,
            "classe",
            &classe.id.vers_chaine(),
            serde_json::json!({ "date": demande.date, "absences": bilan.absences, "retards": bilan.retards,
                "changees": bilan.changees }),
        );
        self.noyau.terminer(unite).await?;
        Ok(bilan)
    }

    /// Justifie des absences ou des retards d'un même motif (`attendance:justify`), en une
    /// transaction. Rend le nombre de demi-journées justifiées.
    pub async fn justifier(
        &self,
        session: &Session,
        demande: &DemandeJustification,
    ) -> Resultat<u32> {
        let motif = demande.motif.trim();
        if motif.is_empty() || motif.chars().count() > 120 {
            return Err(regle("absence.motif_obligatoire"));
        }
        if demande.absences.is_empty() {
            return Err(regle("absence.rien_a_justifier"));
        }
        let justificatif = demande.justificatif.trim();
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut a_justifier = Vec::new();
        for cle in &demande.absences {
            annees::exiger_inscription_modifiable(self.noyau, session, cle.inscription_id).await?;
            let absence = {
                let mut cnx = self.connexion().await?;
                absences::lire(
                    &mut cnx,
                    cle.inscription_id,
                    cle.date,
                    cle.demi_journee,
                    &contexte,
                )
                .await?
            }
            .ok_or(regle("absence.introuvable"))?;
            if absence.justifiee {
                continue;
            }
            let classe = self.classe_permise(session, absence.classe_id).await?;
            if self
                .periodes_du_niveau(session, &classe)
                .await?
                .iter()
                .any(|p| p.debut <= cle.date && cle.date <= p.fin && p.verrouillee)
            {
                return Err(regle("periode.verrouillee"));
            }
            a_justifier.push(absence);
        }
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        for avant in &a_justifier {
            absences::justifier(
                unite.connexion(),
                avant.inscription_id,
                avant.date,
                avant.demi_journee,
                motif,
                justificatif,
                &contexte,
            )
            .await?;
            let apres = Absence {
                justifiee: true,
                motif: motif.to_owned(),
                justificatif: justificatif.to_owned(),
                ..avant.clone()
            };
            unite.auditer(
                "absence",
                &format!(
                    "{}:{}:{}",
                    avant.inscription_id,
                    avant.date,
                    avant.demi_journee.nom()
                ),
                Operation::Modification,
                Some(serde_json::to_value(avant)?),
                Some(serde_json::to_value(&apres)?),
                "absences.justifier",
            );
            unite.emettre(
                TypeEvenement::AbsenceJustifiee,
                "inscription",
                &avant.inscription_id.vers_chaine(),
                serde_json::json!({ "date": avant.date, "demi_journee": avant.demi_journee,
                    "nature": avant.nature, "motif": motif }),
            );
        }
        // Justifiée pendant le délai de grâce, l'absence n'est plus annoncée.
        let mut vus = Vec::new();
        for avant in &a_justifier {
            if !vus.contains(&(avant.inscription_id, avant.date)) {
                vus.push((avant.inscription_id, avant.date));
                crate::messages_auto::actualiser_avis_absence(
                    unite.connexion(),
                    avant.inscription_id,
                    avant.date,
                    &contexte,
                )
                .await?;
            }
        }
        self.noyau.terminer(unite).await?;
        Ok(u32::try_from(a_justifier.len()).unwrap_or(u32::MAX))
    }

    /// Absences d'une classe sur une période : le détail, à justifier ou justifié, et les totaux
    /// par élève que le bulletin reprend.
    pub async fn de_la_periode(
        &self,
        session: &Session,
        demande: &DemandeAbsencesPeriode,
    ) -> Resultat<AbsencesPeriode> {
        let classe = self.classe_permise(session, demande.classe_id).await?;
        let periode = self
            .periodes_du_niveau(session, &classe)
            .await?
            .into_iter()
            .find(|p| p.id == demande.periode_id)
            .ok_or(regle("evaluation.periode_invalide"))?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let (liste, jours_releves) = {
            let mut cnx = self.connexion().await?;
            let liste =
                absences::de_la_classe(&mut cnx, classe.id, periode.debut, periode.fin, &contexte)
                    .await?;
            let jours =
                absences::jours_releves(&mut cnx, classe.id, periode.debut, periode.fin, &contexte)
                    .await?;
            (liste, jours)
        };
        let eleves = self.eleves(session, &classe).await?;
        let nom = |i: Id<Inscription>| {
            eleves
                .iter()
                .find(|(e, _, _, _)| *e == i)
                .map(|(_, eleve, matricule, _)| (eleve.clone(), matricule.clone()))
                .unwrap_or_default()
        };
        let totaux_eleves = eleves
            .iter()
            .map(|(inscription_id, eleve, matricule, _)| {
                let siennes: Vec<Absence> = liste
                    .iter()
                    .filter(|a| a.inscription_id == *inscription_id)
                    .cloned()
                    .collect();
                TotauxEleve {
                    inscription_id: *inscription_id,
                    eleve: eleve.clone(),
                    matricule: matricule.clone(),
                    totaux: totaux(&siennes, periode.debut, periode.fin),
                }
            })
            .collect();
        Ok(AbsencesPeriode {
            absences: liste
                .into_iter()
                .map(|absence| {
                    let (eleve, matricule) = nom(absence.inscription_id);
                    AbsenceAffichee {
                        absence,
                        eleve,
                        matricule,
                    }
                })
                .collect(),
            totaux: totaux_eleves,
            jours_releves,
        })
    }
}
