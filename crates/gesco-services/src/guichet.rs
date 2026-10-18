//! Écran guichet (SPEC_PHASE_5 §8, blueprint §19.5) : le parent qui se présente au comptoir est
//! le moment où tout converge. Une recherche, et l'agent voit la famille entière : ce que doit
//! chaque enfant, les bulletins à remettre ou à signer, les convocations en cours, les absences
//! récentes non justifiées, les messages envoyés. Les actions passent par les commandes de chaque
//! module, sans changer d'écran.

use std::collections::BTreeSet;

use gesco_core::commun::{DateCivile, Id, Montant};
use gesco_core::core::Permission;
use gesco_core::finance::recouvrement::impaye;
use gesco_core::pedagogie::assiduite::{DemiJournee, NatureAbsence};
use gesco_core::scolarite::{Eleve, Inscription, Responsable, StatutInscription};
use gesco_db::depots::{
    absences, academique, annee_scolaire, factures, file_sms, inscriptions, personnes,
    remises_documents, role,
};
use gesco_db::{Connexion, Contexte};
use serde::{Deserialize, Serialize};

use crate::convocations::ConvocationAffichee;
use crate::erreur::{ErreurService, Resultat};
use crate::file_sms::MessageAffiche;
use crate::noyau::Noyau;
use crate::remises_documents::LigneRemise;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Jours d'absences et de messages que le guichet montre.
const JOURS_RECENTS: i64 = 30;

/// Un enfant de la famille, avec ce qu'il doit.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct EnfantGuichet {
    /// Élève.
    pub eleve_id: Id<Eleve>,
    /// Inscription de l'année en cours.
    pub inscription_id: Id<Inscription>,
    /// Prénoms et nom.
    pub nom: String,
    /// Matricule de l'école.
    pub matricule: String,
    /// Classe.
    pub classe: String,
    /// Reste dû sur les échéances passées.
    pub echu: Montant,
    /// Reste dû sur toute l'année.
    pub reste: Montant,
    /// Plus ancienne échéance passée non soldée, `AAAA-MM-JJ`.
    pub echu_depuis: Option<String>,
}

/// Responsable de la famille.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ResponsableGuichet {
    /// Responsable.
    pub responsable_id: Id<Responsable>,
    /// Prénoms et nom.
    pub nom: String,
    /// Téléphones.
    pub telephones: Vec<String>,
    /// Lien avec l'élève cherché.
    pub lien: String,
    /// Son code d'accès à la consultation par SMS.
    pub code_acces: crate::consultation::EtatCodeResponsable,
}

/// Absence récente non justifiée.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct AbsenceGuichet {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Prénoms et nom de l'élève.
    pub eleve: String,
    /// Jour, `AAAA-MM-JJ`.
    pub date: String,
    /// Matin ou après-midi.
    pub demi_journee: DemiJournee,
}

/// Ce que peut faire le compte connecté.
#[derive(Debug, Clone, Copy, Serialize, specta::Type)]
pub struct DroitsGuichet {
    /// Encaisser.
    pub encaisser: bool,
    /// Remettre et faire signer les bulletins.
    pub remettre: bool,
    /// Justifier les absences.
    pub justifier: bool,
    /// Noter la suite d'une convocation.
    pub convocations: bool,
    /// Remettre et révoquer les codes d'accès.
    pub codes: bool,
}

/// La famille au guichet.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct Guichet {
    /// Élève cherché.
    pub eleve_id: Id<Eleve>,
    /// Ses responsables.
    pub responsables: Vec<ResponsableGuichet>,
    /// L'élève cherché d'abord, puis ses frères et sœurs inscrits cette année.
    pub enfants: Vec<EnfantGuichet>,
    /// Total dû d'échu par la famille.
    pub echu_famille: Montant,
    /// Bulletins à remettre, non retirés, ou remis sans signature.
    pub documents: Vec<LigneRemise>,
    /// Convocations en cours.
    pub convocations: Vec<ConvocationAffichee>,
    /// Absences des trente derniers jours, non justifiées.
    pub absences: Vec<AbsenceGuichet>,
    /// Messages envoyés à la famille ces trente derniers jours.
    pub messages: Vec<MessageAffiche>,
    /// Droits de qui consulte.
    pub droits: DroitsGuichet,
}

/// Élève cherché au guichet.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeGuichet {
    /// Élève.
    pub eleve_id: Id<Eleve>,
}

/// Service du guichet.
pub struct ServiceGuichet<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceGuichet<'n> {
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

    /// L'enfant inscrit cette année, avec ce qu'il doit ; aucun s'il n'est pas inscrit.
    async fn enfant(
        cnx: &mut Connexion,
        eleve_id: Id<Eleve>,
        inscrites: &[Inscription],
        aujourd_hui: DateCivile,
        contexte: &Contexte,
    ) -> Resultat<Option<EnfantGuichet>> {
        let Some(inscription) = inscrites.iter().find(|i| i.eleve_id == eleve_id) else {
            return Ok(None);
        };
        let (eleve, personne) = personnes::eleve(cnx, eleve_id, contexte).await?;
        let classe = academique::classe(cnx, inscription.classe_id, contexte).await?;
        let (mut echu, mut reste, mut echu_depuis) = (Montant::ZERO, Montant::ZERO, None);
        if let Some(facture) = factures::facture_vivante(cnx, inscription.id, contexte).await? {
            let echeances: Vec<_> = crate::facturation::etat_echeances(cnx, facture.id, contexte)
                .await?
                .iter()
                .map(|e| (e.echeance.date, e.reste))
                .collect();
            if let Ok(Some(i)) = impaye(&echeances, aujourd_hui) {
                echu = i.echu;
                echu_depuis = Some(i.plus_ancienne.to_string());
            }
            for (_, r) in &echeances {
                reste = reste.ajouter(*r).unwrap_or(reste);
            }
        }
        Ok(Some(EnfantGuichet {
            eleve_id,
            inscription_id: inscription.id,
            nom: personne.nom_complet(),
            matricule: eleve.matricule_interne,
            classe: classe.libelle,
            echu,
            reste,
            echu_depuis,
        }))
    }

    /// La famille d'un élève, telle que le guichet la montre.
    pub async fn famille(&self, session: &Session, demande: &DemandeGuichet) -> Resultat<Guichet> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let aujourd_hui =
            DateCivile::du_jour(contexte.maintenant).ok_or(regle("guichet.date_invalide"))?;
        let annee = annee_scolaire::courante(&mut cnx, &contexte)
            .await?
            .ok_or(regle("guichet.annee_absente"))?;
        let inscrites: Vec<Inscription> = inscriptions::de_l_annee(&mut cnx, annee.id, &contexte)
            .await?
            .into_iter()
            .filter(|i| i.statut == StatutInscription::Validee)
            .collect();
        // La fratrie : les autres enfants des mêmes responsables.
        let liens = personnes::liens_de_l_eleve(&mut cnx, demande.eleve_id, &contexte).await?;
        let mut responsables = Vec::new();
        let mut fratrie: BTreeSet<String> = BTreeSet::new();
        let tous_liens = personnes::liens(&mut cnx, &contexte).await?;
        for (_, responsable_id, qualites) in &liens {
            let (_, p) = personnes::responsable(&mut cnx, *responsable_id, &contexte).await?;
            let code_acces =
                crate::consultation::etat_code(&mut cnx, *responsable_id, &contexte).await?;
            responsables.push(ResponsableGuichet {
                responsable_id: *responsable_id,
                code_acces,
                nom: p.nom_complet(),
                telephones: p.telephones.clone(),
                lien: qualites.lien.nom().to_owned(),
            });
            for (eleve_id, r, _) in &tous_liens {
                if r == responsable_id && *eleve_id != demande.eleve_id {
                    fratrie.insert(eleve_id.vers_chaine());
                }
            }
        }
        let mut enfants = Vec::new();
        if let Some(e) = Self::enfant(
            &mut cnx,
            demande.eleve_id,
            &inscrites,
            aujourd_hui,
            &contexte,
        )
        .await?
        {
            enfants.push(e);
        }
        for id in fratrie {
            let Ok(eleve_id) = Id::analyser(&id) else {
                continue;
            };
            if let Some(e) =
                Self::enfant(&mut cnx, eleve_id, &inscrites, aujourd_hui, &contexte).await?
            {
                enfants.push(e);
            }
        }
        let echu_famille = enfants
            .iter()
            .fold(Montant::ZERO, |s, e| s.ajouter(e.echu).unwrap_or(s));
        let inscriptions_famille: Vec<Id<Inscription>> =
            enfants.iter().map(|e| e.inscription_id).collect();
        let eleves_famille: Vec<Id<Eleve>> = enfants.iter().map(|e| e.eleve_id).collect();

        let mut documents = Vec::new();
        for inscription_id in &inscriptions_famille {
            for r in
                remises_documents::de_l_inscription(&mut cnx, *inscription_id, &contexte).await?
            {
                if r.statut.en_attente()
                    || r.statut
                        == gesco_core::scolarite::remise_document::StatutRemiseDocument::Remis
                {
                    documents
                        .push(crate::remises_documents::afficher(&mut cnx, r, &contexte).await?);
                }
            }
        }
        let mut convocations = Vec::new();
        for inscription_id in &inscriptions_famille {
            for c in
                remises_documents::de_l_inscription_convoquee(&mut cnx, *inscription_id, &contexte)
                    .await?
            {
                if c.en_cours() {
                    convocations.push(crate::convocations::afficher(&mut cnx, c, &contexte).await?);
                }
            }
        }
        let mut liste_absences = Vec::new();
        let debut = aujourd_hui.decaler(-JOURS_RECENTS).unwrap_or(aujourd_hui);
        for enfant in &enfants {
            let inscription =
                inscriptions::lire(&mut cnx, enfant.inscription_id, &contexte).await?;
            for a in absences::de_la_classe(
                &mut cnx,
                inscription.classe_id,
                debut,
                aujourd_hui,
                &contexte,
            )
            .await?
            {
                if a.inscription_id == enfant.inscription_id
                    && a.nature == NatureAbsence::Absence
                    && !a.justifiee
                {
                    liste_absences.push(AbsenceGuichet {
                        inscription_id: a.inscription_id,
                        eleve: enfant.nom.clone(),
                        date: a.date.to_string(),
                        demi_journee: a.demi_journee,
                    });
                }
            }
        }
        liste_absences.sort_by(|a, b| b.date.cmp(&a.date));
        let depuis = contexte.maintenant.millis_epoque() - JOURS_RECENTS * 86_400_000;
        let mut messages = Vec::new();
        for m in file_sms::lister(&mut cnx, None, 500, &contexte).await? {
            if m.cree_le.millis_epoque() >= depuis
                && m.eleve_id.is_some_and(|e| eleves_famille.contains(&e))
            {
                let eleve = enfants
                    .iter()
                    .find(|e| Some(e.eleve_id) == m.eleve_id)
                    .map(|e| e.nom.clone())
                    .unwrap_or_default();
                let destinataire = match m.responsable_id {
                    Some(id) => personnes::responsable(&mut cnx, id, &contexte)
                        .await
                        .map(|(_, p)| p.nom_complet())
                        .unwrap_or_default(),
                    None => String::new(),
                };
                messages.push(MessageAffiche {
                    message: m,
                    destinataire,
                    eleve,
                });
            }
        }
        let p = role::permissions_de(&mut cnx, session.utilisateur_id, &contexte).await?;
        Ok(Guichet {
            eleve_id: demande.eleve_id,
            responsables,
            enfants,
            echu_famille,
            documents,
            convocations,
            absences: liste_absences,
            messages,
            droits: DroitsGuichet {
                encaisser: p.contains(&Permission::PAYMENTS_CREATE),
                remettre: p.contains(&Permission::DOCUMENTS_DELIVER),
                justifier: p.contains(&Permission::ATTENDANCE_JUSTIFY),
                convocations: p.contains(&Permission::CONVOCATIONS_CLOSE),
                codes: p.contains(&Permission::GUARDIANS_ACCESS_CODE),
            },
        })
    }
}
