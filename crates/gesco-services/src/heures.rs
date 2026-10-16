//! État mensuel des heures des vacataires (SPEC_PHASE_6 §4, T-41) : la direction arrête chaque
//! mois les heures faites de chaque vacataire, les commente et les vise ; une ligne visée ne
//! change plus que par un déverrouillage motivé. La paie (phase 7) paie les heures visées. Les
//! taux et les montants ne sortent du service que pour un compte qui a `contracts:read` (§7).

use gesco_core::commun::{Id, Montant};
use gesco_core::core::transition::MachineAEtats;
use gesco_core::core::{Operation, TypeEvenement};
use gesco_core::rh::heures::{
    ecrire_heures, lire_heures, minutes_prevues, montant_des_heures, vacation_du_mois,
};
use gesco_core::rh::{Engagement, HeuresVacation, Mois, Remuneration, StatutHeures};
use gesco_db::depots::{personnel, personnes};
use gesco_db::nouvel_id;
use gesco_db::unite::Durabilite;
use serde::{Deserialize, Serialize};

use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Ligne de l'état : une vacation du mois, saisie ou seulement proposée.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct LigneHeures {
    /// Ligne enregistrée ; absente tant que rien n'est saisi.
    pub id: Option<Id<HeuresVacation>>,
    /// Engagement de vacation.
    pub engagement_id: Id<Engagement>,
    /// Prénoms et nom.
    pub membre: String,
    /// Matricule.
    pub matricule: String,
    /// Fonction de l'engagement.
    pub fonction: String,
    /// Taux de l'heure ; absent sans `contracts:read`.
    pub taux: Option<Montant>,
    /// Heures prévues, en minutes.
    pub minutes_prevues: i64,
    /// Heures faites, en minutes.
    pub minutes_faites: Option<i64>,
    /// Montant des heures faites, tronqué au franc ; absent avant la saisie, ou sans
    /// `contracts:read`.
    pub montant: Option<Montant>,
    /// Observation.
    pub observation: String,
    /// État.
    pub statut: StatutHeures,
    /// Motif du dernier déverrouillage.
    pub motif_deverrouillage: String,
}

/// État des heures d'un mois.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct EtatHeures {
    /// Mois.
    pub mois: Mois,
    /// Lignes, par nom.
    pub lignes: Vec<LigneHeures>,
    /// Total des heures faites, en minutes.
    pub minutes_faites: i64,
    /// Total des montants ; absent sans `contracts:read`.
    pub montant: Option<Montant>,
    /// Vrai si le compte voit les taux et les montants.
    pub montants: bool,
}

/// Mois demandé.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeMois {
    /// Mois.
    pub mois: Mois,
}

/// Heures saisies pour une vacation et un mois.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeSaisieHeures {
    /// Engagement de vacation.
    pub engagement_id: Id<Engagement>,
    /// Mois.
    pub mois: Mois,
    /// Heures prévues, telles que saisies.
    pub prevues: String,
    /// Heures faites, telles que saisies ; vide, pas encore saisies.
    pub faites: String,
    /// Observation.
    pub observation: String,
}

/// Visa des heures d'un mois : toutes les lignes saisies, ou celles d'un engagement.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeVisaHeures {
    /// Mois.
    pub mois: Mois,
    /// Engagement seul ; absent, tout le mois.
    pub engagement_id: Option<Id<Engagement>>,
}

/// Déverrouillage motivé d'une ligne visée.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeDeverrouillageHeures {
    /// Ligne.
    pub id: Id<HeuresVacation>,
    /// Motif.
    pub motif: String,
}

/// Service des heures des vacataires.
pub struct ServiceHeures<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceHeures<'n> {
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

    /// État du mois : chaque vacation qui couvre le mois, avec ses heures saisies ou proposées.
    pub async fn lire(&self, session: &Session, mois: Mois) -> Resultat<EtatHeures> {
        let montants = crate::engagements::voit_les_montants(self.noyau, session).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let saisies = personnel::heures_du_mois(&mut cnx, mois, &contexte).await?;
        let membres = personnel::membres(&mut cnx, &contexte).await?;
        let mut lignes = Vec::new();
        for e in personnel::engagements(&mut cnx, &contexte).await? {
            let saisie = saisies.iter().find(|h| h.engagement_id == e.id);
            // Une vacation du mois, ou une ligne déjà saisie pour elle.
            if !vacation_du_mois(&e, mois) && saisie.is_none() {
                continue;
            }
            let Remuneration::Horaire { taux, .. } = e.remuneration else {
                continue;
            };
            let Some(m) = membres.iter().find(|m| m.id == e.membre_id) else {
                continue;
            };
            let p = personnes::personne(&mut cnx, m.personne_id, &contexte).await?;
            let minutes_faites = saisie.and_then(|h| h.minutes_faites);
            lignes.push(LigneHeures {
                id: saisie.map(|h| h.id),
                engagement_id: e.id,
                membre: format!("{} {}", p.prenoms, p.nom),
                matricule: m.matricule.clone(),
                fonction: e.fonction.clone(),
                taux: montants.then_some(taux),
                minutes_prevues: saisie.map_or_else(|| minutes_prevues(&e), |h| h.minutes_prevues),
                minutes_faites,
                montant: minutes_faites
                    .filter(|_| montants)
                    .and_then(|f| montant_des_heures(taux, f)),
                observation: saisie.map(|h| h.observation.clone()).unwrap_or_default(),
                statut: saisie.map_or(StatutHeures::EnSaisie, |h| h.statut),
                motif_deverrouillage: saisie
                    .map(|h| h.motif_deverrouillage.clone())
                    .unwrap_or_default(),
            });
        }
        lignes.sort_by(|a, b| a.membre.cmp(&b.membre));
        let minutes_faites = lignes.iter().filter_map(|l| l.minutes_faites).sum();
        let montant = if montants {
            Some(
                lignes
                    .iter()
                    .filter_map(|l| l.montant)
                    .try_fold(Montant::ZERO, |s, m| s.ajouter(m))
                    .map_err(|_| regle("heures.calcul_impossible"))?,
            )
        } else {
            None
        };
        Ok(EtatHeures {
            mois,
            lignes,
            minutes_faites,
            montant,
            montants,
        })
    }

    /// Saisit les heures d'une vacation pour un mois : la ligne se crée à la première saisie.
    pub async fn saisir(
        &self,
        session: &Session,
        demande: &DemandeSaisieHeures,
    ) -> Resultat<EtatHeures> {
        let prevues = lire_heures(&demande.prevues)
            .map_err(regle)?
            .ok_or(regle("heures.prevues_obligatoires"))?;
        let faites = lire_heures(&demande.faites).map_err(regle)?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let e = personnel::engagement(unite.connexion(), demande.engagement_id, &contexte).await?;
        let existante = personnel::heures_du_mois(unite.connexion(), demande.mois, &contexte)
            .await?
            .into_iter()
            .find(|h| h.engagement_id == e.id);
        if existante.is_none() && !vacation_du_mois(&e, demande.mois) {
            return Err(regle("heures.hors_vacation"));
        }
        if existante
            .as_ref()
            .is_some_and(|h| h.statut == StatutHeures::Visee)
        {
            return Err(regle("heures.deja_visees"));
        }
        let apres = HeuresVacation {
            id: existante.as_ref().map_or_else(nouvel_id, |h| h.id),
            etablissement_id: session.etablissement_id,
            engagement_id: e.id,
            membre_id: e.membre_id,
            mois: demande.mois,
            minutes_prevues: prevues,
            minutes_faites: faites,
            observation: demande.observation.trim().to_owned(),
            statut: StatutHeures::EnSaisie,
            vise_le: None,
            vise_par: None,
            motif_deverrouillage: existante
                .as_ref()
                .map(|h| h.motif_deverrouillage.clone())
                .unwrap_or_default(),
        };
        match &existante {
            Some(_) => personnel::ecrire_heures(unite.connexion(), &apres, &contexte).await?,
            None => personnel::inserer_heures(unite.connexion(), &apres, &contexte).await?,
        }
        let id = apres.id.vers_chaine();
        unite.auditer(
            "heures_vacation",
            &id,
            if existante.is_some() {
                Operation::Modification
            } else {
                Operation::Creation
            },
            existante.as_ref().map(serde_json::to_value).transpose()?,
            Some(serde_json::to_value(&apres)?),
            "heures.saisir",
        );
        unite.emettre(
            TypeEvenement::EtatHeuresSaisi,
            "heures_vacation",
            &id,
            serde_json::json!({ "mois": apres.mois, "faites": apres.minutes_faites }),
        );
        self.noyau.terminer(unite).await?;
        self.lire(session, demande.mois).await
    }

    /// Vise les lignes saisies du mois, ou d'un engagement : chacune doit avoir ses heures faites.
    pub async fn viser(
        &self,
        session: &Session,
        demande: &DemandeVisaHeures,
    ) -> Resultat<EtatHeures> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let a_viser: Vec<HeuresVacation> =
            personnel::heures_du_mois(unite.connexion(), demande.mois, &contexte)
                .await?
                .into_iter()
                .filter(|h| {
                    h.statut == StatutHeures::EnSaisie
                        && demande.engagement_id.is_none_or(|e| e == h.engagement_id)
                })
                .collect();
        if a_viser.is_empty() {
            return Err(regle("heures.rien_a_viser"));
        }
        if a_viser.iter().any(|h| h.minutes_faites.is_none()) {
            return Err(regle("heures.faites_manquantes"));
        }
        for avant in a_viser {
            let apres = HeuresVacation {
                statut: avant.statut.transiter(StatutHeures::Visee)?,
                vise_le: Some(contexte.maintenant),
                vise_par: Some(session.utilisateur_id),
                ..avant.clone()
            };
            personnel::ecrire_heures(unite.connexion(), &apres, &contexte).await?;
            let id = apres.id.vers_chaine();
            unite.auditer(
                "heures_vacation",
                &id,
                Operation::Modification,
                Some(serde_json::to_value(&avant)?),
                Some(serde_json::to_value(&apres)?),
                "heures.viser",
            );
            unite.emettre(
                TypeEvenement::EtatHeuresVise,
                "heures_vacation",
                &id,
                serde_json::json!({ "mois": apres.mois, "faites": apres.minutes_faites }),
            );
        }
        self.noyau.terminer(unite).await?;
        self.lire(session, demande.mois).await
    }

    /// Déverrouille une ligne visée, avec un motif : elle revient en saisie.
    pub async fn deverrouiller(
        &self,
        session: &Session,
        demande: &DemandeDeverrouillageHeures,
    ) -> Resultat<EtatHeures> {
        let motif = demande.motif.trim().to_owned();
        if motif.is_empty() {
            return Err(regle("heures.motif_obligatoire"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let avant = personnel::heures(unite.connexion(), demande.id, &contexte).await?;
        // Des heures payées par une paie validée ne se rouvrent plus (SPEC_PHASE_7 §5).
        if personnel::bulletin_des_heures(unite.connexion(), demande.id, &contexte)
            .await?
            .is_some()
        {
            return Err(regle("heures.payees"));
        }
        let apres = HeuresVacation {
            statut: avant.statut.transiter(StatutHeures::EnSaisie)?,
            vise_le: None,
            vise_par: None,
            motif_deverrouillage: motif,
            ..avant.clone()
        };
        personnel::ecrire_heures(unite.connexion(), &apres, &contexte).await?;
        let id = apres.id.vers_chaine();
        unite.auditer(
            "heures_vacation",
            &id,
            Operation::Modification,
            Some(serde_json::to_value(&avant)?),
            Some(serde_json::to_value(&apres)?),
            "heures.deverrouiller",
        );
        unite.emettre(
            TypeEvenement::EtatHeuresDeverrouille,
            "heures_vacation",
            &id,
            serde_json::json!({ "mois": apres.mois }),
        );
        self.noyau.terminer(unite).await?;
        self.lire(session, apres.mois).await
    }

    /// Données de l'état imprimable du mois.
    pub async fn donnees_etat(&self, session: &Session, mois: Mois) -> Resultat<serde_json::Value> {
        let etat = self.lire(session, mois).await?;
        let lignes: Vec<serde_json::Value> = etat
            .lignes
            .iter()
            .map(|l| {
                serde_json::json!({
                    "membre": l.membre,
                    "matricule": l.matricule,
                    "fonction": l.fonction,
                    "taux": l.taux.map(|t| t.formater()).unwrap_or_default(),
                    "prevues": ecrire_heures(l.minutes_prevues),
                    "faites": l.minutes_faites.map(ecrire_heures),
                    "montant": l.montant.map(|m| m.formater()),
                    "observation": l.observation,
                    "visee": l.statut == StatutHeures::Visee,
                })
            })
            .collect();
        Ok(serde_json::json!({
            "mois": mois_long(mois),
            "lignes": lignes,
            "total_heures": ecrire_heures(etat.minutes_faites),
            "total_montant": etat.montant.map(|m| m.formater()).unwrap_or_default(),
            "date": crate::documents::date_longue(self.noyau.horloge().maintenant()),
        }))
    }
}

/// Mois en toutes lettres : « octobre 2026 ».
pub(crate) fn mois_long(mois: Mois) -> String {
    mois.premier_jour()
        .map(crate::documents::date_civile_longue)
        .map_or_else(
            || mois.to_string(),
            |t| t.trim_start_matches("1er ").to_owned(),
        )
}
