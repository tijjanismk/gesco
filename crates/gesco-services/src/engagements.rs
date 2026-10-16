//! Engagements du personnel (SPEC_PHASE_6 §3) : conclus, renouvelés, remplacés à une date par un
//! nouveau salaire ou un nouveau taux, terminés ou rompus ; jamais modifiés après coup (Q-65).
//! Leurs montants ne sortent du service que pour un compte qui a `contracts:read`, ou pour le
//! membre lui-même (SPEC_PHASE_6 §7, Q-67).

use gesco_core::commun::{DateCivile, Id};
use gesco_core::core::transition::MachineAEtats;
use gesco_core::core::{Operation, Permission, TypeEvenement};
use gesco_core::rh::engagement::{
    chevauche_un_engagement_en_cours, dates_de_renouvellement, fin_proche, heures_lisibles,
    verifier_engagement,
};
use gesco_core::rh::{
    Engagement, MembrePersonnel, NatureEngagement, Remuneration, StatutEngagement,
};
use gesco_db::depots::{annee_scolaire, parametre, personnel, personnes, role};
use gesco_db::nouvel_id;
use gesco_db::unite::Durabilite;
use serde::{Deserialize, Serialize};

use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Vrai si le compte voit les montants des engagements et des états des heures
/// (`contracts:read`) ; sans ce droit, le service les retire avant de répondre (Q-67).
pub(crate) async fn voit_les_montants(noyau: &Noyau, session: &Session) -> Resultat<bool> {
    let contexte = noyau.contexte(session.etablissement_id, Some(session));
    let mut cnx = noyau
        .base()
        .pool()
        .acquire()
        .await
        .map_err(gesco_db::ErreurBase::from)?;
    Ok(
        role::permissions_de(&mut cnx, session.utilisateur_id, &contexte)
            .await?
            .contains(&Permission::CONTRACTS_READ),
    )
}

/// Motif écrit sur l'engagement qu'un nouveau remplace.
const MOTIF_REMPLACE: &str = "Remplacé par un nouvel engagement";

/// Engagement à conclure.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeEngagement {
    /// Membre engagé.
    pub membre_id: Id<MembrePersonnel>,
    /// Nature.
    pub nature: NatureEngagement,
    /// Premier jour.
    pub debut: DateCivile,
    /// Dernier jour prévu ; pour une vacation, la fin de l'année scolaire si absent.
    pub fin: Option<DateCivile>,
    /// Fonction ; celle de la fiche si vide.
    pub fonction: String,
    /// Rémunération.
    pub remuneration: Remuneration,
    /// Note libre.
    pub note: String,
    /// Engagement en cours qu'il remplace à sa date de début : nouveau salaire, nouveau taux.
    pub remplace: Option<Id<Engagement>>,
    /// Engagement qu'il renouvelle.
    pub renouvelle: Option<Id<Engagement>>,
    /// Payé douze mois, ou hors des mois de vacances ; absent : le réglage de l'école, ou ce que
    /// faisait l'engagement remplacé (SPEC_PHASE_7 §2).
    pub paye_douze_mois: Option<bool>,
}

/// Mois payés d'un engagement mensuel (SPEC_PHASE_7 §2, Q-73).
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeMoisPayes {
    /// Engagement.
    pub id: Id<Engagement>,
    /// Payé douze mois ; faux : seulement hors des mois de vacances de l'école.
    pub paye_douze_mois: bool,
}

/// Clôture d'un engagement.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeClotureEngagement {
    /// Engagement.
    pub id: Id<Engagement>,
    /// Dernier jour.
    pub le: DateCivile,
    /// Motif ; obligatoire avant la fin prévue, et pour une rupture.
    pub motif: String,
}

/// Engagement désigné.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeIdEngagement {
    /// Engagement.
    pub id: Id<Engagement>,
}

/// Engagement affiché sur la fiche.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct EngagementAffiche {
    /// Engagement.
    pub engagement: Engagement,
    /// Vrai s'il finit dans moins de soixante jours, ou est échu sans avoir été clos, et n'a pas
    /// encore été renouvelé.
    pub fin_proche: bool,
    /// Vrai si un engagement le renouvelle ou le remplace déjà.
    pub renouvele: bool,
}

/// Vrai si un autre engagement renouvelle ou remplace celui-ci.
fn a_une_suite(e: &Engagement, tous: &[Engagement]) -> bool {
    tous.iter().any(|x| x.precedent_id == Some(e.id))
}

/// Engagement qui arrive à son terme, pour le tableau de bord.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct EngagementAEcheance {
    /// Engagement.
    pub engagement: Engagement,
    /// Prénoms et nom du membre.
    pub membre: String,
}

/// Service des engagements.
pub struct ServiceEngagements<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceEngagements<'n> {
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

    fn aujourd_hui(&self) -> Resultat<DateCivile> {
        crate::inscriptions::jour(self.noyau.horloge().maintenant())
    }

    /// L'engagement tel que le compte a le droit de le lire.
    async fn selon_droits(&self, session: &Session, e: Engagement) -> Resultat<Engagement> {
        Ok(if voit_les_montants(self.noyau, session).await? {
            e
        } else {
            e.sans_montants()
        })
    }

    /// Engagements d'un membre, du plus ancien au plus récent ; le membre lié au compte lit les
    /// siens montants compris.
    pub async fn lister(
        &self,
        session: &Session,
        membre_id: Id<MembrePersonnel>,
    ) -> Resultat<Vec<EngagementAffiche>> {
        let montants = voit_les_montants(self.noyau, session).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let aujourd_hui = self.aujourd_hui()?;
        let mut cnx = self.connexion().await?;
        let membre = personnel::membre(&mut cnx, membre_id, &contexte).await?;
        let montants = montants || membre.utilisateur_id == Some(session.utilisateur_id);
        let tous = personnel::engagements_du_membre(&mut cnx, membre_id, &contexte).await?;
        Ok(tous
            .iter()
            .map(|e| {
                let renouvele = a_une_suite(e, &tous);
                EngagementAffiche {
                    fin_proche: !renouvele && fin_proche(e, aujourd_hui),
                    renouvele,
                    engagement: if montants {
                        e.clone()
                    } else {
                        e.clone().sans_montants()
                    },
                }
            })
            .collect())
    }

    /// Engagements en cours qui finissent dans moins de soixante jours, par date de fin.
    pub async fn a_echeance(&self, session: &Session) -> Resultat<Vec<EngagementAEcheance>> {
        let montants = voit_les_montants(self.noyau, session).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let aujourd_hui = self.aujourd_hui()?;
        let mut cnx = self.connexion().await?;
        let membres = personnel::membres(&mut cnx, &contexte).await?;
        let mut liste = Vec::new();
        let tous = personnel::engagements(&mut cnx, &contexte).await?;
        for e in &tous {
            if !fin_proche(e, aujourd_hui) || a_une_suite(e, &tous) {
                continue;
            }
            let Some(m) = membres.iter().find(|m| m.id == e.membre_id) else {
                continue;
            };
            let p = personnes::personne(&mut cnx, m.personne_id, &contexte).await?;
            liste.push(EngagementAEcheance {
                membre: format!("{} {}", p.prenoms, p.nom),
                engagement: if montants {
                    e.clone()
                } else {
                    e.clone().sans_montants()
                },
            });
        }
        liste.sort_by_key(|a| a.engagement.fin);
        Ok(liste)
    }

    /// Conclut un engagement ; s'il en remplace un en cours, celui-ci se termine la veille.
    pub async fn conclure(
        &self,
        session: &Session,
        demande: &DemandeEngagement,
    ) -> Resultat<Engagement> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let membre = personnel::membre(unite.connexion(), demande.membre_id, &contexte).await?;
        if !membre.present() {
            return Err(regle("personnel.deja_parti"));
        }
        // Une vacation court jusqu'à la fin de l'année scolaire, sauf autre date (§3).
        let fin = match (demande.fin, demande.nature) {
            (Some(f), _) => Some(f),
            (None, NatureEngagement::Vacation) => {
                annee_scolaire::courante(unite.connexion(), &contexte)
                    .await?
                    .and_then(|a| DateCivile::du_jour(a.fin))
            }
            (None, _) => None,
        };
        let existants =
            personnel::engagements_du_membre(unite.connexion(), membre.id, &contexte).await?;
        let remplace = match demande.remplace {
            Some(id) => {
                let r = existants
                    .iter()
                    .find(|e| e.id == id && e.statut == StatutEngagement::EnCours)
                    .cloned()
                    .ok_or(regle("engagement.remplace_inconnu"))?;
                if demande.debut <= r.debut {
                    return Err(regle("engagement.remplacement_trop_tot"));
                }
                Some(r)
            }
            None => None,
        };
        if let Some(id) = demande.renouvelle
            && !existants.iter().any(|e| e.id == id)
        {
            return Err(regle("engagement.inconnu"));
        }
        let fonction = match demande.fonction.trim() {
            "" => membre.fonction.clone(),
            f => f.to_owned(),
        };
        let paye_douze_mois = match (demande.paye_douze_mois, &remplace) {
            (Some(choix), _) => choix,
            (None, Some(r)) => r.paye_douze_mois,
            (None, None) => {
                parametre::booleen_param(
                    unite.connexion(),
                    gesco_core::paie::reglages::PARAMETRE_DOUZE_MOIS,
                    true,
                    &contexte,
                )
                .await?
            }
        };
        let nouveau = Engagement {
            id: nouvel_id(),
            etablissement_id: session.etablissement_id,
            membre_id: membre.id,
            nature: demande.nature,
            debut: demande.debut,
            fin,
            fonction,
            remuneration: demande.remuneration.clone(),
            paye_douze_mois,
            statut: StatutEngagement::EnCours,
            arrete_le: None,
            motif_fin: String::new(),
            precedent_id: demande.remplace.or(demande.renouvelle),
            note: demande.note.trim().to_owned(),
        };
        verifier_engagement(&nouveau).map_err(regle)?;
        if chevauche_un_engagement_en_cours(&existants, &nouveau, demande.remplace) {
            return Err(regle("engagement.chevauchement"));
        }
        if let Some(r) = remplace {
            let veille = demande
                .debut
                .decaler(-1)
                .ok_or(regle("engagement.fin_avant_debut"))?;
            let clos = Engagement {
                statut: r.statut.transiter(StatutEngagement::Termine)?,
                arrete_le: Some(veille),
                motif_fin: MOTIF_REMPLACE.to_owned(),
                ..r.clone()
            };
            personnel::clore_engagement(unite.connexion(), &clos, &contexte).await?;
            let id = clos.id.vers_chaine();
            unite.auditer(
                "engagement",
                &id,
                Operation::Modification,
                Some(serde_json::to_value(&r)?),
                Some(serde_json::to_value(&clos)?),
                "engagements.conclure",
            );
            unite.emettre(
                TypeEvenement::EngagementTermine,
                "engagement",
                &id,
                serde_json::json!({ "le": veille, "remplace": true }),
            );
        }
        personnel::inserer_engagement(unite.connexion(), &nouveau, &contexte).await?;
        let id = nouveau.id.vers_chaine();
        unite.auditer(
            "engagement",
            &id,
            Operation::Creation,
            None,
            Some(serde_json::to_value(&nouveau)?),
            "engagements.conclure",
        );
        unite.emettre(
            TypeEvenement::EngagementConclu,
            "engagement",
            &id,
            serde_json::json!({ "membre": nouveau.membre_id, "nature": nouveau.nature }),
        );
        self.noyau.terminer(unite).await?;
        self.selon_droits(session, nouveau).await
    }

    /// Termine un engagement à une date : sa fin prévue, ou plus tôt avec un motif.
    pub async fn terminer(
        &self,
        session: &Session,
        demande: &DemandeClotureEngagement,
    ) -> Resultat<Engagement> {
        self.clore(session, demande, StatutEngagement::Termine)
            .await
    }

    /// Rompt un engagement, avec un motif.
    pub async fn rompre(
        &self,
        session: &Session,
        demande: &DemandeClotureEngagement,
    ) -> Resultat<Engagement> {
        self.clore(session, demande, StatutEngagement::Rompu).await
    }

    async fn clore(
        &self,
        session: &Session,
        demande: &DemandeClotureEngagement,
        vers: StatutEngagement,
    ) -> Resultat<Engagement> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let avant = personnel::engagement(unite.connexion(), demande.id, &contexte).await?;
        let motif = demande.motif.trim().to_owned();
        let avant_terme = avant.fin.is_none_or(|f| demande.le < f);
        if motif.is_empty() && (vers == StatutEngagement::Rompu || avant_terme) {
            return Err(regle("engagement.motif_obligatoire"));
        }
        if demande.le < avant.debut || avant.fin.is_some_and(|f| demande.le > f) {
            return Err(regle("engagement.date_de_fin_invalide"));
        }
        let apres = Engagement {
            statut: avant.statut.transiter(vers)?,
            arrete_le: Some(demande.le),
            motif_fin: motif,
            ..avant.clone()
        };
        personnel::clore_engagement(unite.connexion(), &apres, &contexte).await?;
        let id = apres.id.vers_chaine();
        unite.auditer(
            "engagement",
            &id,
            Operation::Modification,
            Some(serde_json::to_value(&avant)?),
            Some(serde_json::to_value(&apres)?),
            if vers == StatutEngagement::Rompu {
                "engagements.rompre"
            } else {
                "engagements.terminer"
            },
        );
        unite.emettre(
            if vers == StatutEngagement::Rompu {
                TypeEvenement::EngagementRompu
            } else {
                TypeEvenement::EngagementTermine
            },
            "engagement",
            &id,
            serde_json::json!({ "le": demande.le }),
        );
        self.noyau.terminer(unite).await?;
        self.selon_droits(session, apres).await
    }

    /// Change les mois payés d'un engagement mensuel : douze, ou hors des mois de vacances de
    /// l'école. Les paies déjà validées ne changent pas.
    pub async fn changer_mois_payes(
        &self,
        session: &Session,
        demande: &DemandeMoisPayes,
    ) -> Resultat<Engagement> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let avant = personnel::engagement(unite.connexion(), demande.id, &contexte).await?;
        if matches!(
            avant.nature,
            NatureEngagement::Vacation | NatureEngagement::Benevole
        ) {
            return Err(regle("engagement.mois_payes_mensuel"));
        }
        let apres = Engagement {
            paye_douze_mois: demande.paye_douze_mois,
            ..avant.clone()
        };
        personnel::ecrire_mois_payes(unite.connexion(), &apres, &contexte).await?;
        unite.auditer(
            "engagement",
            &apres.id.vers_chaine(),
            Operation::Modification,
            Some(serde_json::to_value(&avant)?),
            Some(serde_json::to_value(&apres)?),
            "engagements.mois_payes",
        );
        self.noyau.terminer(unite).await?;
        self.selon_droits(session, apres).await
    }

    /// Engagement proposé au renouvellement : mêmes nature, fonction et rémunération, pour
    /// l'année scolaire suivante (vacation) ou la même durée ; à modifier avant de conclure.
    pub async fn proposer_renouvellement(
        &self,
        session: &Session,
        id: Id<Engagement>,
    ) -> Resultat<DemandeEngagementProposee> {
        let montants = voit_les_montants(self.noyau, session).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let e = personnel::engagement(&mut cnx, id, &contexte).await?;
        let annee_suivante = annee_scolaire::lister(&mut cnx, &contexte)
            .await?
            .into_iter()
            .filter_map(|a| Some((DateCivile::du_jour(a.debut)?, DateCivile::du_jour(a.fin)?)))
            .filter(|(debut, _)| e.fin.is_some_and(|f| *debut > f))
            .min();
        let (debut, fin) =
            dates_de_renouvellement(&e, annee_suivante).ok_or(regle("engagement.sans_fin"))?;
        Ok(DemandeEngagementProposee {
            membre_id: e.membre_id,
            nature: e.nature,
            debut,
            fin: Some(fin),
            fonction: e.fonction,
            remuneration: if montants {
                e.remuneration
            } else {
                e.remuneration.sans_montants()
            },
            paye_douze_mois: e.paye_douze_mois,
            note: e.note,
            renouvelle: e.id,
        })
    }

    /// Données de la lettre d'engagement.
    pub async fn donnees_lettre(
        &self,
        session: &Session,
        id: Id<Engagement>,
    ) -> Resultat<serde_json::Value> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let e = personnel::engagement(&mut cnx, id, &contexte).await?;
        let membre = personnel::membre(&mut cnx, e.membre_id, &contexte).await?;
        let p = personnes::personne(&mut cnx, membre.personne_id, &contexte).await?;
        let date = crate::documents::date_civile_longue;
        let (remuneration, heures) = match &e.remuneration {
            Remuneration::Mensuelle { montant } => {
                (format!("{} par mois", montant.formater()), None)
            }
            Remuneration::Horaire {
                taux,
                minutes_par_semaine,
            } => (
                format!("{} l'heure", taux.formater()),
                Some(heures_lisibles(*minutes_par_semaine)),
            ),
            Remuneration::Aucune | Remuneration::Masquee => (String::new(), None),
        };
        Ok(serde_json::json!({
            "membre": format!("{} {}", p.prenoms, p.nom),
            "matricule": membre.matricule,
            "fonctionnaire": membre.fonctionnaire,
            "numero_fonctionnaire": membre.numero_fonctionnaire,
            "nature": e.nature.nom(),
            "fonction": e.fonction,
            "debut": date(e.debut),
            "fin": e.fin.map(date),
            "remuneration": remuneration,
            "heures": heures,
            "note": e.note,
            "date": date(self.aujourd_hui()?),
        }))
    }
}

/// Engagement proposé au renouvellement, à envoyer tel quel ou corrigé à la conclusion.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct DemandeEngagementProposee {
    /// Membre.
    pub membre_id: Id<MembrePersonnel>,
    /// Nature.
    pub nature: NatureEngagement,
    /// Premier jour proposé.
    pub debut: DateCivile,
    /// Dernier jour proposé.
    pub fin: Option<DateCivile>,
    /// Fonction.
    pub fonction: String,
    /// Rémunération.
    pub remuneration: Remuneration,
    /// Payé douze mois, ou hors des mois de vacances.
    pub paye_douze_mois: bool,
    /// Note.
    pub note: String,
    /// Engagement renouvelé.
    pub renouvelle: Id<Engagement>,
}
