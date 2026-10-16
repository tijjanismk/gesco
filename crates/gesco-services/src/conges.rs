//! Absences, retards et congés du personnel (SPEC_PHASE_6 §5 et §6, T-43, T-44) : les absences et
//! les retards se notent sans pointage, se justifient après coup, s'annulent avec un motif ; un
//! congé se demande, la direction l'accorde ou le refuse, et l'annule avant son début. Le décompte
//! des jours acquis, désactivé par défaut, signale un congé annuel au-delà du solde sans le bloquer.

use gesco_core::commun::{DateCivile, Id};
use gesco_core::core::transition::MachineAEtats;
use gesco_core::core::{Etablissement, Operation, TypeEvenement};
use gesco_core::rh::conge::{
    JOURS_PAR_MOIS_DEFAUT, TYPES_CONGE_LIVRES, jours_de_conge, lire_jours_par_mois, solde_au,
    verifier_absence, verifier_conge,
};
use gesco_core::rh::{
    AbsencePersonnel, Conge, MembrePersonnel, Mois, NatureAbsencePersonnel, StatutConge, TypeConge,
};
use gesco_db::depots::{conges, parametre, personnel, personnes};
use gesco_db::unite::Durabilite;
use gesco_db::{UniteDeTravail, nouvel_id};
use serde::{Deserialize, Serialize};

use crate::effets::Effet;
use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Le samedi compte dans les jours d'un congé (Q-68) : non par défaut.
pub const PARAMETRE_SAMEDI: &str = "conge.samedi_compte";
/// Décompte des jours acquis (T-44) : désactivé par défaut.
pub const PARAMETRE_DECOMPTE: &str = "conge.decompte_actif";
/// Jours acquis par mois quand le décompte est actif.
pub const PARAMETRE_JOURS_PAR_MOIS: &str = "conge.jours_par_mois";

/// Absence affichée, avec le membre.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct AbsencePersonnelAffichee {
    /// Absence.
    pub absence: AbsencePersonnel,
    /// Prénoms et nom.
    pub membre: String,
    /// Matricule.
    pub matricule: String,
}

/// Congé affiché, avec le membre, son type et, quand le décompte est actif, le solde.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct CongeAffiche {
    /// Congé.
    pub conge: Conge,
    /// Prénoms et nom.
    pub membre: String,
    /// Matricule.
    pub matricule: String,
    /// Libellé du type.
    pub type_libelle: String,
    /// Solde du membre au premier jour, hors ce congé, en demi-journées : seulement si le
    /// décompte est actif et que le type se déduit du solde.
    pub solde: Option<i64>,
    /// Vrai si le congé dépasse ce solde : signalé, jamais bloqué (T-44).
    pub depasse_solde: bool,
}

/// Absences et congés d'un mois.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct AbsencesEtConges {
    /// Mois.
    pub mois: Mois,
    /// Types de congé, retirés compris.
    pub types: Vec<TypeConge>,
    /// Congés demandés en attente d'une décision, de tout mois, puis les autres congés du mois.
    pub conges: Vec<CongeAffiche>,
    /// Absences et retards du mois, par jour.
    pub absences: Vec<AbsencePersonnelAffichee>,
    /// Le samedi compte dans les jours d'un congé.
    pub samedi_compte: bool,
    /// Le décompte des jours acquis est actif.
    pub decompte_actif: bool,
}

/// Mois des absences et congés.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeMoisConges {
    /// Mois.
    pub mois: Mois,
}

/// Absence ou retard à noter.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeAbsencePersonnel {
    /// Membre.
    pub membre_id: Id<MembrePersonnel>,
    /// Jour.
    pub date: DateCivile,
    /// Journée, demi-journée ou retard.
    pub nature: NatureAbsencePersonnel,
    /// Durée du retard, en minutes.
    pub minutes: Option<i64>,
    /// Motif donné.
    pub motif: String,
    /// Déjà justifiée.
    pub justifiee: bool,
}

/// Justification ou annulation d'une absence, avec son motif.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeMotifAbsence {
    /// Absence.
    pub id: Id<AbsencePersonnel>,
    /// Motif.
    pub motif: String,
}

/// Congé demandé.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeConge {
    /// Membre.
    pub membre_id: Id<MembrePersonnel>,
    /// Type.
    pub type_id: Id<TypeConge>,
    /// Premier jour.
    pub debut: DateCivile,
    /// Dernier jour.
    pub fin: DateCivile,
    /// Motif.
    pub motif: String,
}

/// Congé à accorder.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeIdConge {
    /// Congé.
    pub id: Id<Conge>,
}

/// Refus ou annulation d'un congé, avec son motif.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeMotifConge {
    /// Congé.
    pub id: Id<Conge>,
    /// Motif.
    pub motif: String,
}

/// Type de congé à créer ou à modifier.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeTypeConge {
    /// Type modifié ; absent, un type nouveau.
    pub id: Option<Id<TypeConge>>,
    /// Libellé.
    pub libelle: String,
    /// Proposé aux demandes.
    pub actif: bool,
    /// Ses jours se déduisent du solde.
    pub deduit_du_solde: bool,
}

/// Pose dans l'unité les types de congé livrés qui manquent, reconnus par leur code. Rend le
/// nombre de types ajoutés.
pub(crate) async fn poser_types_conge(
    unite: &mut UniteDeTravail<Effet>,
    etablissement_id: Id<Etablissement>,
    origine: &str,
) -> Resultat<usize> {
    let contexte = unite.contexte().clone();
    let existants = conges::types(unite.connexion(), &contexte).await?;
    let mut ajoutes = 0;
    for (ordre, (code, libelle, deduit)) in (1_i64..).zip(TYPES_CONGE_LIVRES) {
        if existants.iter().any(|t| t.code.as_deref() == Some(code)) {
            continue;
        }
        let t = TypeConge {
            id: nouvel_id(),
            etablissement_id,
            code: Some(code.to_owned()),
            libelle: libelle.to_owned(),
            ordre,
            actif: true,
            deduit_du_solde: deduit,
        };
        conges::inserer_type(unite.connexion(), &t, &contexte).await?;
        unite.auditer(
            "type_conge",
            &t.id.vers_chaine(),
            Operation::Creation,
            None,
            Some(serde_json::to_value(&t)?),
            origine,
        );
        ajoutes += 1;
    }
    Ok(ajoutes)
}

/// Service des absences et des congés du personnel.
pub struct ServiceConges<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceConges<'n> {
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

    /// Absences et congés d'un mois ; les demandes en attente, de tout mois, en tête.
    pub async fn lire(&self, session: &Session, mois: Mois) -> Resultat<AbsencesEtConges> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let (Some(premier), Some(dernier)) = (mois.premier_jour(), mois.dernier_jour()) else {
            return Err(regle("conge.mois_invalide"));
        };
        let samedi_compte =
            parametre::booleen_param(&mut cnx, PARAMETRE_SAMEDI, false, &contexte).await?;
        let decompte_actif =
            parametre::booleen_param(&mut cnx, PARAMETRE_DECOMPTE, false, &contexte).await?;
        let par_mois = lire_jours_par_mois(
            &parametre::texte(
                &mut cnx,
                PARAMETRE_JOURS_PAR_MOIS,
                JOURS_PAR_MOIS_DEFAUT,
                &contexte,
            )
            .await?,
        )
        .unwrap_or(5);
        let types = conges::types(&mut cnx, &contexte).await?;
        let tous = conges::conges(&mut cnx, &contexte).await?;
        let membres = personnel::membres(&mut cnx, &contexte).await?;
        let mut noms = Vec::new();
        for m in &membres {
            let p = personnes::personne(&mut cnx, m.personne_id, &contexte).await?;
            noms.push((
                m.id,
                format!("{} {}", p.prenoms, p.nom),
                m.matricule.clone(),
            ));
        }
        let nom = |id: Id<MembrePersonnel>| {
            noms.iter()
                .find(|(m, _, _)| *m == id)
                .map(|(_, n, matricule)| (n.clone(), matricule.clone()))
                .unwrap_or_default()
        };
        let mut retenus: Vec<&Conge> = tous
            .iter()
            .filter(|c| {
                c.statut == StatutConge::Demande || (c.debut <= dernier && premier <= c.fin)
            })
            .collect();
        retenus.sort_by_key(|c| (c.statut != StatutConge::Demande, c.debut));
        let conges = retenus
            .into_iter()
            .map(|c| {
                let type_conge = types.iter().find(|t| t.id == c.type_id);
                let solde = membres
                    .iter()
                    .find(|m| m.id == c.membre_id)
                    .filter(|_| decompte_actif && type_conge.is_some_and(|t| t.deduit_du_solde))
                    .and_then(|m| solde_au(m, &tous, &types, par_mois, c.debut, Some(c.id)));
                let (membre, matricule) = nom(c.membre_id);
                CongeAffiche {
                    conge: c.clone(),
                    membre,
                    matricule,
                    type_libelle: type_conge.map(|t| t.libelle.clone()).unwrap_or_default(),
                    solde,
                    depasse_solde: c.vivant() && solde.is_some_and(|s| s < c.jours * 2),
                }
            })
            .collect();
        let absences = conges::absences_entre(&mut cnx, premier, dernier, &contexte)
            .await?
            .into_iter()
            .map(|a| {
                let (membre, matricule) = nom(a.membre_id);
                AbsencePersonnelAffichee {
                    absence: a,
                    membre,
                    matricule,
                }
            })
            .collect();
        Ok(AbsencesEtConges {
            mois,
            types,
            conges,
            absences,
            samedi_compte,
            decompte_actif,
        })
    }

    /// Note une absence ou un retard.
    pub async fn noter_absence(
        &self,
        session: &Session,
        demande: &DemandeAbsencePersonnel,
    ) -> Resultat<AbsencePersonnel> {
        let aujourd_hui = self.aujourd_hui()?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let membre = personnel::membre(unite.connexion(), demande.membre_id, &contexte).await?;
        let a = AbsencePersonnel {
            id: nouvel_id(),
            etablissement_id: session.etablissement_id,
            membre_id: membre.id,
            date: demande.date,
            nature: demande.nature,
            minutes: demande.minutes,
            motif: demande.motif.trim().to_owned(),
            justifiee: demande.justifiee,
            annulee: false,
            motif_annulation: String::new(),
        };
        let tous = conges::conges(unite.connexion(), &contexte).await?;
        let du_jour = conges::absences_entre(unite.connexion(), a.date, a.date, &contexte).await?;
        verifier_absence(&a, aujourd_hui, &membre, &tous, &du_jour).map_err(regle)?;
        conges::inserer_absence(unite.connexion(), &a, &contexte).await?;
        let id = a.id.vers_chaine();
        unite.auditer(
            "absence_personnel",
            &id,
            Operation::Creation,
            None,
            Some(serde_json::to_value(&a)?),
            "absence_personnel.noter",
        );
        unite.emettre(
            TypeEvenement::AbsencePersonnelNotee,
            "absence_personnel",
            &id,
            serde_json::json!({ "date": a.date, "nature": a.nature }),
        );
        self.noyau.terminer(unite).await?;
        Ok(a)
    }

    /// Justifie une absence après coup ; le motif donné remplace le précédent.
    pub async fn justifier_absence(
        &self,
        session: &Session,
        demande: &DemandeMotifAbsence,
    ) -> Resultat<AbsencePersonnel> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let avant = conges::absence(unite.connexion(), demande.id, &contexte).await?;
        if avant.annulee {
            return Err(regle("absence_personnel.annulee"));
        }
        let motif = demande.motif.trim();
        let apres = AbsencePersonnel {
            justifiee: true,
            motif: if motif.is_empty() {
                avant.motif.clone()
            } else {
                motif.to_owned()
            },
            ..avant.clone()
        };
        conges::ecrire_absence(unite.connexion(), &apres, &contexte).await?;
        let id = apres.id.vers_chaine();
        unite.auditer(
            "absence_personnel",
            &id,
            Operation::Modification,
            Some(serde_json::to_value(&avant)?),
            Some(serde_json::to_value(&apres)?),
            "absence_personnel.justifier",
        );
        unite.emettre(
            TypeEvenement::AbsencePersonnelJustifiee,
            "absence_personnel",
            &id,
            serde_json::json!({ "date": apres.date }),
        );
        self.noyau.terminer(unite).await?;
        Ok(apres)
    }

    /// Annule une absence notée par erreur, avec un motif (R-14).
    pub async fn annuler_absence(
        &self,
        session: &Session,
        demande: &DemandeMotifAbsence,
    ) -> Resultat<AbsencePersonnel> {
        let motif = demande.motif.trim().to_owned();
        if motif.is_empty() {
            return Err(regle("absence_personnel.motif_obligatoire"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let avant = conges::absence(unite.connexion(), demande.id, &contexte).await?;
        if avant.annulee {
            return Err(regle("absence_personnel.annulee"));
        }
        let apres = AbsencePersonnel {
            annulee: true,
            motif_annulation: motif,
            ..avant.clone()
        };
        conges::ecrire_absence(unite.connexion(), &apres, &contexte).await?;
        let id = apres.id.vers_chaine();
        unite.auditer(
            "absence_personnel",
            &id,
            Operation::Modification,
            Some(serde_json::to_value(&avant)?),
            Some(serde_json::to_value(&apres)?),
            "absence_personnel.annuler",
        );
        unite.emettre(
            TypeEvenement::AbsencePersonnelAnnulee,
            "absence_personnel",
            &id,
            serde_json::json!({ "date": apres.date }),
        );
        self.noyau.terminer(unite).await?;
        Ok(apres)
    }

    /// Enregistre une demande de congé ; ses jours se comptent à la demande.
    pub async fn demander(&self, session: &Session, demande: &DemandeConge) -> Resultat<Conge> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let membre = personnel::membre(unite.connexion(), demande.membre_id, &contexte).await?;
        let type_conge = conges::types(unite.connexion(), &contexte)
            .await?
            .into_iter()
            .find(|t| t.id == demande.type_id)
            .ok_or(regle("conge.type_inconnu"))?;
        let samedi =
            parametre::booleen_param(unite.connexion(), PARAMETRE_SAMEDI, false, &contexte).await?;
        let jours = if demande.fin < demande.debut {
            0
        } else {
            jours_de_conge(demande.debut, demande.fin, samedi)
        };
        let c = Conge {
            id: nouvel_id(),
            etablissement_id: session.etablissement_id,
            membre_id: membre.id,
            type_id: type_conge.id,
            debut: demande.debut,
            fin: demande.fin,
            jours,
            motif: demande.motif.trim().to_owned(),
            statut: StatutConge::Demande,
            decide_le: None,
            decide_par: None,
            motif_refus: String::new(),
            motif_annulation: String::new(),
        };
        let tous = conges::conges(unite.connexion(), &contexte).await?;
        verifier_conge(&c, &membre, &type_conge, &tous).map_err(regle)?;
        conges::inserer_conge(unite.connexion(), &c, &contexte).await?;
        let id = c.id.vers_chaine();
        unite.auditer(
            "conge",
            &id,
            Operation::Creation,
            None,
            Some(serde_json::to_value(&c)?),
            "conge.demander",
        );
        unite.emettre(
            TypeEvenement::CongeDemande,
            "conge",
            &id,
            serde_json::json!({ "debut": c.debut, "fin": c.fin, "jours": c.jours }),
        );
        self.noyau.terminer(unite).await?;
        Ok(c)
    }

    /// Accorde un congé demandé.
    pub async fn accorder(&self, session: &Session, demande: &DemandeIdConge) -> Resultat<Conge> {
        self.decider(session, demande.id, StatutConge::Accorde, String::new())
            .await
    }

    /// Refuse un congé demandé, avec un motif.
    pub async fn refuser(&self, session: &Session, demande: &DemandeMotifConge) -> Resultat<Conge> {
        self.decider(
            session,
            demande.id,
            StatutConge::Refuse,
            demande.motif.trim().to_owned(),
        )
        .await
    }

    /// Annule un congé accordé avant son début, avec un motif.
    pub async fn annuler(&self, session: &Session, demande: &DemandeMotifConge) -> Resultat<Conge> {
        self.decider(
            session,
            demande.id,
            StatutConge::Annule,
            demande.motif.trim().to_owned(),
        )
        .await
    }

    async fn decider(
        &self,
        session: &Session,
        id: Id<Conge>,
        vers: StatutConge,
        motif: String,
    ) -> Resultat<Conge> {
        if vers != StatutConge::Accorde && motif.is_empty() {
            return Err(regle("conge.motif_obligatoire"));
        }
        let aujourd_hui = self.aujourd_hui()?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let avant = conges::conge(unite.connexion(), id, &contexte).await?;
        let statut = avant.statut.transiter(vers)?;
        if statut == StatutConge::Annule && avant.debut <= aujourd_hui {
            return Err(regle("conge.deja_commence"));
        }
        let apres = match statut {
            StatutConge::Annule => Conge {
                statut,
                motif_annulation: motif,
                ..avant.clone()
            },
            _ => Conge {
                statut,
                decide_le: Some(contexte.maintenant),
                decide_par: Some(session.utilisateur_id),
                motif_refus: motif,
                ..avant.clone()
            },
        };
        conges::ecrire_statut_conge(unite.connexion(), &apres, &contexte).await?;
        let (evenement, origine) = match statut {
            StatutConge::Accorde => (TypeEvenement::CongeAccorde, "conge.accorder"),
            StatutConge::Refuse => (TypeEvenement::CongeRefuse, "conge.refuser"),
            _ => (TypeEvenement::CongeAnnule, "conge.annuler"),
        };
        let id = apres.id.vers_chaine();
        unite.auditer(
            "conge",
            &id,
            Operation::Modification,
            Some(serde_json::to_value(&avant)?),
            Some(serde_json::to_value(&apres)?),
            origine,
        );
        unite.emettre(
            evenement,
            "conge",
            &id,
            serde_json::json!({ "debut": apres.debut, "fin": apres.fin }),
        );
        self.noyau.terminer(unite).await?;
        Ok(apres)
    }

    /// Crée ou modifie un type de congé ; un type retiré reste sur les congés qui le portent.
    pub async fn enregistrer_type(
        &self,
        session: &Session,
        demande: &DemandeTypeConge,
    ) -> Resultat<TypeConge> {
        let libelle = demande.libelle.trim().to_owned();
        if libelle.is_empty() {
            return Err(regle("personnel.libelle_obligatoire"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let existants = conges::types(unite.connexion(), &contexte).await?;
        if existants.iter().any(|t| {
            t.actif
                && Some(t.id) != demande.id
                && t.libelle.to_lowercase() == libelle.to_lowercase()
        }) {
            return Err(regle("personnel.libelle_existe"));
        }
        let avant = match demande.id {
            Some(id) => Some(
                existants
                    .iter()
                    .find(|t| t.id == id)
                    .cloned()
                    .ok_or(regle("conge.type_inconnu"))?,
            ),
            None => None,
        };
        let apres = match &avant {
            Some(a) => TypeConge {
                libelle,
                actif: demande.actif,
                deduit_du_solde: demande.deduit_du_solde,
                ..a.clone()
            },
            None => TypeConge {
                id: nouvel_id(),
                etablissement_id: session.etablissement_id,
                code: None,
                libelle,
                ordre: existants.iter().map(|t| t.ordre).max().unwrap_or(0) + 1,
                actif: true,
                deduit_du_solde: demande.deduit_du_solde,
            },
        };
        match &avant {
            Some(_) => conges::ecrire_type(unite.connexion(), &apres, &contexte).await?,
            None => conges::inserer_type(unite.connexion(), &apres, &contexte).await?,
        }
        let id = apres.id.vers_chaine();
        unite.auditer(
            "type_conge",
            &id,
            if avant.is_some() {
                Operation::Modification
            } else {
                Operation::Creation
            },
            avant.as_ref().map(serde_json::to_value).transpose()?,
            Some(serde_json::to_value(&apres)?),
            "conge.type",
        );
        unite.emettre(
            TypeEvenement::ReferentielPersonnelModifie,
            "type_conge",
            &id,
            serde_json::json!({ "actif": apres.actif }),
        );
        self.noyau.terminer(unite).await?;
        Ok(apres)
    }
}
