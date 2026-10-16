//! Tableau de bord (réponse du 13 octobre 2026) : à l'ouverture, les effectifs, les finances et
//! la scolarité de l'année en cours, et le personnel du jour (SPEC_PHASE_6 §8), chaque bloc selon
//! les permissions du compte.

use gesco_core::commun::{DateCivile, Montant};
use gesco_core::core::Permission;
use gesco_core::rh::{Mois, NatureAbsencePersonnel, NatureEngagement, StatutConge, StatutHeures};
use gesco_db::depots::{academique, annee_scolaire, personnel, role, tableau};
use serde::Serialize;

use crate::conges::ServiceConges;
use crate::engagements::ServiceEngagements;
use crate::erreur::Resultat;
use crate::heures::ServiceHeures;
use crate::inscriptions::jour;
use crate::noyau::Noyau;
use crate::recouvrement::{DemandeJournal, ServiceRecouvrement, TotalTranche};
use crate::session::Session;

/// Effectif d'une classe.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct EffectifClasse {
    /// Classe.
    pub classe: String,
    /// Niveau.
    pub niveau: String,
    /// Élèves inscrits (inscriptions validées).
    pub eleves: u32,
    /// Dont filles.
    pub filles: u32,
    /// Dont garçons.
    pub garcons: u32,
}

/// Effectif d'un cycle.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct EffectifCycle {
    /// Cycle, nom persisté.
    pub cycle: String,
    /// Élèves inscrits.
    pub eleves: u32,
}

/// Effectifs de l'année.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct Effectifs {
    /// Élèves inscrits.
    pub total: u32,
    /// Dont filles.
    pub filles: u32,
    /// Dont garçons.
    pub garcons: u32,
    /// Inscriptions en brouillon, à valider.
    pub brouillons: u32,
    /// Par cycle.
    pub par_cycle: Vec<EffectifCycle>,
    /// Par classe, dans l'ordre des niveaux.
    pub par_classe: Vec<EffectifClasse>,
}

/// Finances du jour et du mois.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct Finances {
    /// Encaissé aujourd'hui, annulations déduites.
    pub encaisse_jour: Montant,
    /// Reçus d'aujourd'hui.
    pub recus_jour: u32,
    /// Encaissé depuis le début du mois.
    pub encaisse_mois: Montant,
    /// Reste échu de l'école.
    pub reste_echu: Montant,
    /// Élèves en retard de paiement.
    pub eleves_en_retard: u32,
    /// Retards par ancienneté.
    pub par_tranche: Vec<TotalTranche>,
}

/// Scolarité du jour.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct Scolarite {
    /// Demi-journées d'absence relevées aujourd'hui.
    pub absences_jour: u32,
    /// Retards relevés aujourd'hui.
    pub retards_jour: u32,
    /// Classes dont le relevé du jour est fait.
    pub classes_relevees: u32,
    /// Classes de l'année.
    pub classes: u32,
    /// Évaluations encore en saisie.
    pub evaluations_en_saisie: u32,
    /// Bulletins calculés, à valider.
    pub bulletins_a_valider: u32,
    /// Bulletins validés, à publier.
    pub bulletins_a_publier: u32,
    /// Exclusions en cours.
    pub exclusions_en_cours: u32,
}

/// Membres présents d'une catégorie du personnel.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct EffectifCategorie {
    /// Catégorie.
    pub categorie: String,
    /// Membres présents.
    pub membres: u32,
}

/// Engagement qui finit bientôt, sans sa rémunération.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct EcheanceEngagement {
    /// Prénoms et nom.
    pub membre: String,
    /// Nature.
    pub nature: NatureEngagement,
    /// Dernier jour prévu.
    pub fin: DateCivile,
}

/// Absence ou retard du jour d'un membre.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct AbsentDuJour {
    /// Prénoms et nom.
    pub membre: String,
    /// Nature.
    pub nature: NatureAbsencePersonnel,
    /// Minutes d'un retard.
    pub minutes: Option<i64>,
}

/// Membre en congé accordé aujourd'hui.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct EnCongeDuJour {
    /// Prénoms et nom.
    pub membre: String,
    /// Type de congé.
    pub type_libelle: String,
    /// Dernier jour du congé.
    pub fin: DateCivile,
}

/// Personnel du jour (SPEC_PHASE_6 §8).
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct BlocPersonnel {
    /// Membres présents.
    pub presents: u32,
    /// Par catégorie, dans l'ordre des catégories.
    pub par_categorie: Vec<EffectifCategorie>,
    /// Mois en cours.
    pub mois: Mois,
    /// Vacataires du mois.
    pub vacataires_du_mois: u32,
    /// Vacations du mois dont les heures ne sont pas visées.
    pub heures_a_viser: u32,
    /// Mois précédent.
    pub mois_precedent: Mois,
    /// Vacations du mois précédent dont les heures ne sont toujours pas visées.
    pub heures_a_viser_mois_precedent: u32,
    /// Engagements qui finissent dans moins de soixante jours, par date de fin.
    pub echeances: Vec<EcheanceEngagement>,
    /// Absences et retards notés aujourd'hui.
    pub absents: Vec<AbsentDuJour>,
    /// Demandes de congé en attente d'une décision.
    pub conges_en_attente: u32,
    /// Membres en congé aujourd'hui.
    pub en_conge: Vec<EnCongeDuJour>,
}

/// Tableau de bord : chaque bloc n'est rempli que si le compte a le droit d'en voir le détail.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct TableauDeBord {
    /// Année en cours ; rien si aucune n'est ouverte.
    pub annee: Option<String>,
    /// Jour du tableau.
    pub jour: DateCivile,
    /// La paie, avec un droit de paie.
    pub paie: Option<crate::paie_mois::BlocPaie>,
    /// Effectifs (`students:read`).
    pub effectifs: Option<Effectifs>,
    /// Finances (`invoices:issue` ou `cashbox:reconcile`).
    pub finances: Option<Finances>,
    /// Scolarité (notes, absences ou bulletins).
    pub scolarite: Option<Scolarite>,
    /// Personnel (`employees:read`), avec ou sans année ouverte.
    pub personnel: Option<BlocPersonnel>,
    /// Trésorerie (`treasury:read` ou un droit sur les dépenses), avec ou sans année ouverte.
    pub tresorerie: Option<crate::livres::BlocTresorerie>,
}

/// Service du tableau de bord.
pub struct ServiceTableau<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceTableau<'n> {
    /// Service sur le noyau ouvert.
    pub const fn nouveau(noyau: &'n Noyau) -> Self {
        Self { noyau }
    }

    /// Tableau de bord du compte connecté.
    pub async fn lire(&self, session: &Session) -> Resultat<TableauDeBord> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let aujourd_hui = jour(contexte.maintenant)?;
        let mut cnx = self
            .noyau
            .base()
            .pool()
            .acquire()
            .await
            .map_err(gesco_db::ErreurBase::from)?;
        let permissions = role::permissions_de(&mut cnx, session.utilisateur_id, &contexte).await?;
        let a = |p: Permission| permissions.contains(&p);
        drop(cnx);
        let personnel = if a(Permission::EMPLOYEES_READ) {
            Some(self.personnel(session, aujourd_hui).await?)
        } else {
            None
        };
        let paie = if a(Permission::PAYROLL_CALCULATE)
            || a(Permission::PAYROLL_VALIDATE)
            || a(Permission::PAYROLL_PAY)
        {
            Some(
                crate::paie_mois::ServicePaieMois::nouveau(self.noyau)
                    .bloc(session)
                    .await?,
            )
        } else {
            None
        };
        let tresorerie = if a(Permission::TREASURY_READ)
            || a(Permission::EXPENSES_CREATE)
            || a(Permission::EXPENSES_APPROVE)
            || a(Permission::EXPENSES_PAY)
        {
            Some(crate::livres::bloc(self.noyau, session).await?)
        } else {
            None
        };
        let mut cnx = self
            .noyau
            .base()
            .pool()
            .acquire()
            .await
            .map_err(gesco_db::ErreurBase::from)?;
        let Some(annee) = annee_scolaire::courante(&mut cnx, &contexte).await? else {
            return Ok(TableauDeBord {
                annee: None,
                jour: aujourd_hui,
                effectifs: None,
                finances: None,
                scolarite: None,
                personnel,
                paie,
                tresorerie,
            });
        };
        let effectifs = if a(Permission::STUDENTS_READ) {
            let lignes = tableau::effectifs(&mut cnx, annee.id, &contexte).await?;
            let validee = |l: &&tableau::LigneEffectif| l.statut == "validee";
            let somme = |f: &dyn Fn(&tableau::LigneEffectif) -> bool| -> u32 {
                u32::try_from(
                    lignes
                        .iter()
                        .filter(validee)
                        .filter(|l| f(l))
                        .map(|l| l.nombre)
                        .sum::<i64>(),
                )
                .unwrap_or(0)
            };
            let mut par_classe: Vec<EffectifClasse> = Vec::new();
            for l in lignes.iter().filter(validee) {
                let n = u32::try_from(l.nombre).unwrap_or(0);
                if !par_classe
                    .iter()
                    .any(|c| c.classe == l.classe && c.niveau == l.niveau)
                {
                    par_classe.push(EffectifClasse {
                        classe: l.classe.clone(),
                        niveau: l.niveau.clone(),
                        eleves: 0,
                        filles: 0,
                        garcons: 0,
                    });
                }
                if let Some(classe) = par_classe
                    .iter_mut()
                    .find(|c| c.classe == l.classe && c.niveau == l.niveau)
                {
                    classe.eleves += n;
                    match l.sexe.as_str() {
                        "feminin" => classe.filles += n,
                        "masculin" => classe.garcons += n,
                        _ => {}
                    }
                }
            }
            let mut par_cycle: Vec<EffectifCycle> = Vec::new();
            for l in lignes.iter().filter(validee) {
                let n = u32::try_from(l.nombre).unwrap_or(0);
                match par_cycle.iter_mut().find(|c| c.cycle == l.cycle) {
                    Some(c) => c.eleves += n,
                    None => par_cycle.push(EffectifCycle {
                        cycle: l.cycle.clone(),
                        eleves: n,
                    }),
                }
            }
            Some(Effectifs {
                total: somme(&|_| true),
                filles: somme(&|l| l.sexe == "feminin"),
                garcons: somme(&|l| l.sexe == "masculin"),
                brouillons: u32::try_from(
                    lignes
                        .iter()
                        .filter(|l| l.statut == "brouillon")
                        .map(|l| l.nombre)
                        .sum::<i64>(),
                )
                .unwrap_or(0),
                par_cycle,
                par_classe,
            })
        } else {
            None
        };
        let scolarite = if a(Permission::GRADES_CREATE)
            || a(Permission::ATTENDANCE_CREATE)
            || a(Permission::REPORTS_GENERATE)
        {
            let (absences_jour, retards_jour) =
                tableau::absences_du_jour(&mut cnx, aujourd_hui, &contexte).await?;
            Some(Scolarite {
                absences_jour,
                retards_jour,
                classes_relevees: tableau::releves_du_jour(&mut cnx, aujourd_hui, &contexte)
                    .await?,
                classes: u32::try_from(
                    academique::classes(&mut cnx, annee.id, &contexte)
                        .await?
                        .len(),
                )
                .unwrap_or(0),
                evaluations_en_saisie: tableau::evaluations_en_saisie(
                    &mut cnx, annee.id, &contexte,
                )
                .await?,
                bulletins_a_valider: tableau::bulletins_en_etat(
                    &mut cnx, annee.id, "calcule", &contexte,
                )
                .await?,
                bulletins_a_publier: tableau::bulletins_en_etat(
                    &mut cnx, annee.id, "valide", &contexte,
                )
                .await?,
                exclusions_en_cours: tableau::exclusions_en_cours(&mut cnx, aujourd_hui, &contexte)
                    .await?,
            })
        } else {
            None
        };
        drop(cnx);
        let finances = if a(Permission::INVOICES_ISSUE) || a(Permission::CASHBOX_RECONCILE) {
            let recouvrement = ServiceRecouvrement::nouveau(self.noyau);
            let journal = |du: DateCivile| DemandeJournal {
                du,
                au: aujourd_hui,
                caissier_id: None,
                mode: None,
            };
            let du_jour = recouvrement.journal(session, &journal(aujourd_hui)).await?;
            let debut_mois = DateCivile::nouvelle(aujourd_hui.annee(), aujourd_hui.mois(), 1)
                .unwrap_or(aujourd_hui);
            let du_mois = recouvrement.journal(session, &journal(debut_mois)).await?;
            let impayes = recouvrement.impayes(session).await?;
            Some(Finances {
                encaisse_jour: du_jour.total,
                recus_jour: du_jour.nombre,
                encaisse_mois: du_mois.total,
                reste_echu: impayes.echu,
                eleves_en_retard: u32::try_from(impayes.lignes.len()).unwrap_or(u32::MAX),
                par_tranche: impayes.par_tranche,
            })
        } else {
            None
        };
        Ok(TableauDeBord {
            annee: Some(annee.libelle),
            jour: aujourd_hui,
            effectifs,
            finances,
            scolarite,
            personnel,
            paie,
            tresorerie,
        })
    }

    /// Bloc « Personnel » : effectif par catégorie, vacataires et heures à viser, engagements qui
    /// finissent bientôt, absences du jour, congés en attente et en cours.
    async fn personnel(
        &self,
        session: &Session,
        aujourd_hui: DateCivile,
    ) -> Resultat<BlocPersonnel> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self
            .noyau
            .base()
            .pool()
            .acquire()
            .await
            .map_err(gesco_db::ErreurBase::from)?;
        let membres: Vec<_> = personnel::membres(&mut cnx, &contexte)
            .await?
            .into_iter()
            .filter(|m| m.present())
            .collect();
        let par_categorie = personnel::categories(&mut cnx, &contexte)
            .await?
            .into_iter()
            .filter_map(|c| {
                let n = membres.iter().filter(|m| m.categorie_id == c.id).count();
                (n > 0).then(|| EffectifCategorie {
                    categorie: c.libelle,
                    membres: u32::try_from(n).unwrap_or(u32::MAX),
                })
            })
            .collect();
        drop(cnx);
        let compter = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
        let mois = Mois::de(aujourd_hui);
        let mois_precedent = mois.precedent().unwrap_or(mois);
        let heures = ServiceHeures::nouveau(self.noyau);
        let du_mois = heures.lire(session, mois).await?;
        let precedent = heures.lire(session, mois_precedent).await?;
        let a_viser = |e: &crate::heures::EtatHeures| {
            compter(
                e.lignes
                    .iter()
                    .filter(|l| l.statut != StatutHeures::Visee)
                    .count(),
            )
        };
        let echeances = ServiceEngagements::nouveau(self.noyau)
            .a_echeance(session)
            .await?
            .into_iter()
            .filter_map(|a| {
                Some(EcheanceEngagement {
                    fin: a.engagement.fin?,
                    nature: a.engagement.nature,
                    membre: a.membre,
                })
            })
            .collect();
        let conges = ServiceConges::nouveau(self.noyau)
            .lire(session, mois)
            .await?;
        Ok(BlocPersonnel {
            presents: compter(membres.len()),
            par_categorie,
            mois,
            vacataires_du_mois: compter(du_mois.lignes.len()),
            heures_a_viser: a_viser(&du_mois),
            mois_precedent,
            heures_a_viser_mois_precedent: a_viser(&precedent),
            echeances,
            absents: conges
                .absences
                .iter()
                .filter(|a| a.absence.date == aujourd_hui && !a.absence.annulee)
                .map(|a| AbsentDuJour {
                    membre: a.membre.clone(),
                    nature: a.absence.nature,
                    minutes: a.absence.minutes,
                })
                .collect(),
            conges_en_attente: compter(
                conges
                    .conges
                    .iter()
                    .filter(|c| c.conge.statut == StatutConge::Demande)
                    .count(),
            ),
            en_conge: conges
                .conges
                .iter()
                .filter(|c| c.conge.statut == StatutConge::Accorde && c.conge.couvre(aujourd_hui))
                .map(|c| EnCongeDuJour {
                    membre: c.membre.clone(),
                    type_libelle: c.type_libelle.clone(),
                    fin: c.conge.fin,
                })
                .collect(),
        })
    }
}
