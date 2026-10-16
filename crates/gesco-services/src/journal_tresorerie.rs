//! Journal de trésorerie et budget (SPEC_PHASE_8 §7, T-63) : par mois, les recettes par type de
//! frais et les dépenses par catégorie, le solde de chaque compte au début et à la fin ; le budget
//! de l'année scolaire, prévu et réalisé, un dépassement qui avertit sans jamais bloquer. Les
//! listes s'exportent en Excel pour le comptable.

use gesco_core::commun::{DateCivile, Horodatage, Id, Montant};
use gesco_core::core::{AnneeScolaire, Operation, TypeEvenement};
use gesco_core::rh::Mois;
use gesco_core::tresorerie::livre::soldes;
use gesco_core::tresorerie::{CategorieTresorerie, Sens};
use gesco_db::depots::{annee_scolaire, budget, livres, tresorerie};
use gesco_db::nouvel_id;
use gesco_db::unite::Durabilite;
use serde::{Deserialize, Serialize};

use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

fn calcul() -> ErreurService {
    regle("tresorerie.calcul_impossible")
}

/// Code de la catégorie de recette d'un type de frais.
fn code_du_type(type_frais: Option<&str>) -> String {
    match type_frais {
        Some(t) => format!("frais_{t}"),
        None => "autres_recettes".to_owned(),
    }
}

/// Une catégorie et son montant.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct LigneJournalTresorerie {
    /// Catégorie.
    pub categorie: String,
    /// Montant.
    pub montant: Montant,
}

/// Solde d'un compte au début et à la fin du mois.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct SoldeDuMois {
    /// Compte.
    pub compte: String,
    /// Au premier jour.
    pub debut: Montant,
    /// Au dernier jour.
    pub fin: Montant,
}

/// Journal de trésorerie d'un mois.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct JournalTresorerie {
    /// Mois.
    pub mois: Mois,
    /// Recettes, par catégorie.
    pub recettes: Vec<LigneJournalTresorerie>,
    /// Dépenses, par catégorie.
    pub depenses: Vec<LigneJournalTresorerie>,
    /// Total des recettes.
    pub total_recettes: Montant,
    /// Total des dépenses.
    pub total_depenses: Montant,
    /// Solde de chaque compte.
    pub soldes: Vec<SoldeDuMois>,
}

/// Une ligne du budget.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct LigneBudget {
    /// Catégorie.
    pub categorie: CategorieTresorerie,
    /// Prévu.
    pub prevu: Montant,
    /// Réalisé à ce jour.
    pub realise: Montant,
    /// Prévu moins réalisé.
    pub ecart: Montant,
    /// Une dépense au-delà de son prévu.
    pub depasse: bool,
}

/// Année qui peut recevoir un budget.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct AnneeBudget {
    /// Année.
    pub id: Id<AnneeScolaire>,
    /// Libellé.
    pub libelle: String,
}

/// Budget d'une année scolaire.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct Budget {
    /// Année.
    pub annee_id: Id<AnneeScolaire>,
    /// Libellé de l'année.
    pub annee: String,
    /// Années qui peuvent recevoir un budget.
    pub annees: Vec<AnneeBudget>,
    /// Recettes.
    pub recettes: Vec<LigneBudget>,
    /// Dépenses.
    pub depenses: Vec<LigneBudget>,
    /// Catégories de dépense au-delà de leur prévu.
    pub depassements: u32,
}

/// Mois du journal.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeJournalTresorerie {
    /// Mois.
    pub mois: Mois,
}

/// Budget d'une année ; vide : l'année courante.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeBudget {
    /// Année.
    pub annee_id: Option<Id<AnneeScolaire>>,
}

/// Montant prévu d'une catégorie.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeLigneBudget {
    /// Année.
    pub annee_id: Id<AnneeScolaire>,
    /// Catégorie.
    pub categorie_id: Id<CategorieTresorerie>,
    /// Montant prévu.
    pub montant: Montant,
}

fn instant(jour: DateCivile) -> Horodatage {
    Horodatage::depuis_millis_epoque(jour.jours_depuis_epoque() * 86_400_000)
}

/// Service du journal et du budget.
pub struct ServiceJournal<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceJournal<'n> {
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

    /// Recettes et dépenses réalisées entre deux instants, par catégorie.
    async fn realise(
        cnx: &mut gesco_db::Connexion,
        categories: &[CategorieTresorerie],
        debut: Horodatage,
        fin: Horodatage,
        contexte: &gesco_db::Contexte,
    ) -> Resultat<Vec<(Id<CategorieTresorerie>, Montant)>> {
        let mut lignes: Vec<(Id<CategorieTresorerie>, Montant)> = Vec::new();
        let mut ajouter = |id: Id<CategorieTresorerie>, m: Montant| -> Resultat<()> {
            match lignes.iter_mut().find(|(c, _)| *c == id) {
                Some((_, total)) => *total = total.ajouter(m).map_err(|_| calcul())?,
                None => lignes.push((id, m)),
            }
            Ok(())
        };
        for (type_frais, montant) in budget::recettes_entre(cnx, debut, fin, contexte).await? {
            let code = code_du_type(type_frais.as_deref());
            if let Some(c) = categories.iter().find(|c| c.code.as_deref() == Some(&code)) {
                ajouter(c.id, montant)?;
            }
        }
        for (categorie, montant) in budget::depenses_entre(cnx, debut, fin, contexte).await? {
            ajouter(categorie, montant)?;
        }
        Ok(lignes)
    }

    /// Journal de trésorerie d'un mois.
    pub async fn journal(
        &self,
        session: &Session,
        demande: &DemandeJournalTresorerie,
    ) -> Resultat<JournalTresorerie> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let (Some(premier), Some(dernier)) =
            (demande.mois.premier_jour(), demande.mois.dernier_jour())
        else {
            return Err(regle("tresorerie.date_invalide"));
        };
        let lendemain = dernier.decaler(1).ok_or(calcul())?;
        let categories = tresorerie::categories(&mut cnx, &contexte).await?;
        let realise = Self::realise(
            &mut cnx,
            &categories,
            instant(premier),
            instant(lendemain),
            &contexte,
        )
        .await?;
        let par_sens = |sens: Sens| -> Vec<LigneJournalTresorerie> {
            categories
                .iter()
                .filter(|c| c.sens == sens)
                .filter_map(|c| {
                    realise
                        .iter()
                        .find(|(id, m)| *id == c.id && !m.est_nul())
                        .map(|(_, m)| LigneJournalTresorerie {
                            categorie: c.libelle.clone(),
                            montant: *m,
                        })
                })
                .collect()
        };
        let total = |l: &[LigneJournalTresorerie]| {
            l.iter()
                .try_fold(Montant::ZERO, |s, x| s.ajouter(x.montant))
                .map_err(|_| calcul())
        };
        let recettes = par_sens(Sens::Recette);
        let depenses = par_sens(Sens::Depense);
        let comptes = tresorerie::comptes(&mut cnx, &contexte).await?;
        let choix = livres::comptes_des_modes(&mut cnx, &contexte).await?;
        let mut soldes_du_mois = Vec::with_capacity(comptes.len());
        for c in comptes.iter().filter(|c| c.actif) {
            let lignes =
                crate::livres::lignes_du_compte(&mut cnx, c, &comptes, &choix, &contexte).await?;
            let jusqua = |borne: DateCivile| -> Resultat<Montant> {
                let avant: Vec<_> = lignes.iter().filter(|l| l.date < borne).cloned().collect();
                Ok(soldes(c.solde_ouverture, &avant)
                    .map_err(|_| calcul())?
                    .livre)
            };
            soldes_du_mois.push(SoldeDuMois {
                compte: c.libelle.clone(),
                debut: jusqua(premier)?,
                fin: jusqua(lendemain)?,
            });
        }
        Ok(JournalTresorerie {
            mois: demande.mois,
            total_recettes: total(&recettes)?,
            total_depenses: total(&depenses)?,
            recettes,
            depenses,
            soldes: soldes_du_mois,
        })
    }

    /// Budget d'une année scolaire : prévu, réalisé à ce jour, écart.
    pub async fn budget(&self, session: &Session, demande: &DemandeBudget) -> Resultat<Budget> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let annees = annee_scolaire::lister(&mut cnx, &contexte).await?;
        let annee = match demande.annee_id {
            Some(id) => annees.iter().find(|a| a.id == id),
            None => annees
                .iter()
                .find(|a| a.est_courante)
                .or_else(|| annees.last()),
        }
        .cloned()
        .ok_or(regle("budget.sans_annee"))?;
        let categories = tresorerie::categories(&mut cnx, &contexte).await?;
        let realise =
            Self::realise(&mut cnx, &categories, annee.debut, annee.fin, &contexte).await?;
        let prevu = budget::budget(&mut cnx, annee.id, &contexte).await?;
        let mut lignes = Vec::new();
        for c in categories {
            let p = prevu
                .iter()
                .find(|(id, _)| *id == c.id)
                .map_or(Montant::ZERO, |(_, m)| *m);
            let r = realise
                .iter()
                .find(|(id, _)| *id == c.id)
                .map_or(Montant::ZERO, |(_, m)| *m);
            if !c.active && p.est_nul() && r.est_nul() {
                continue;
            }
            lignes.push(LigneBudget {
                ecart: p.soustraire(r).map_err(|_| calcul())?,
                depasse: c.sens == Sens::Depense && !p.est_nul() && r > p,
                categorie: c,
                prevu: p,
                realise: r,
            });
        }
        let (recettes, depenses): (Vec<_>, Vec<_>) = lignes
            .into_iter()
            .partition(|l| l.categorie.sens == Sens::Recette);
        Ok(Budget {
            annee_id: annee.id,
            annee: annee.libelle.clone(),
            annees: annees
                .iter()
                .map(|a| AnneeBudget {
                    id: a.id,
                    libelle: a.libelle.clone(),
                })
                .collect(),
            depassements: u32::try_from(depenses.iter().filter(|l| l.depasse).count())
                .unwrap_or(u32::MAX),
            recettes,
            depenses,
        })
    }

    /// Prévoit le montant d'une catégorie pour une année.
    pub async fn prevoir(
        &self,
        session: &Session,
        demande: &DemandeLigneBudget,
    ) -> Resultat<Budget> {
        if demande.montant.est_negatif() {
            return Err(regle("budget.montant_negatif"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        annee_scolaire::lister(unite.connexion(), &contexte)
            .await?
            .iter()
            .find(|a| a.id == demande.annee_id)
            .ok_or(regle("budget.sans_annee"))?;
        let categorie = tresorerie::categories(unite.connexion(), &contexte)
            .await?
            .into_iter()
            .find(|c| c.id == demande.categorie_id)
            .ok_or(regle("tresorerie.categorie_inconnue"))?;
        let id: Id<Budget> = nouvel_id();
        budget::prevoir(
            unite.connexion(),
            &id.vers_chaine(),
            demande.annee_id,
            categorie.id,
            demande.montant,
            &contexte,
        )
        .await?;
        let cle = format!(
            "{}:{}",
            demande.annee_id.vers_chaine(),
            categorie.id.vers_chaine()
        );
        unite.auditer(
            "budget_tresorerie",
            &cle,
            Operation::Modification,
            None,
            Some(serde_json::json!({ "categorie": categorie.libelle, "montant": demande.montant })),
            "tresorerie.budget",
        );
        unite.emettre(
            TypeEvenement::BudgetPrevu,
            "budget_tresorerie",
            &cle,
            serde_json::json!({ "categorie": categorie.libelle, "montant": demande.montant }),
        );
        self.noyau.terminer(unite).await?;
        self.budget(
            session,
            &DemandeBudget {
                annee_id: Some(demande.annee_id),
            },
        )
        .await
    }
}
