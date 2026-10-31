//! Recouvrement (SPEC_PHASE_3 §9) : la liste des impayés (UC-05), par classe et par tranche
//! d'ancienneté avec les téléphones des payeurs, et le journal des encaissements par période,
//! caissier et mode, que demande le comptable externe. Tout est recalculé (R-20).

use std::collections::HashMap;

use gesco_core::commun::{DateCivile, Horodatage, Id, Montant};
use gesco_core::core::{StatutAnnee, Utilisateur};
use gesco_core::finance::recouvrement::impaye;
use gesco_core::finance::{
    Impaye, ModePaiement, PARAMETRE_TRANCHES, StatutVersement, TrancheAnciennete, Tranches,
    Versement,
};
use gesco_core::scolarite::{Eleve, Inscription};
use gesco_db::depots::{annee_scolaire, factures, parametre, personnes, versements};
use serde::{Deserialize, Serialize};

use crate::erreur::{ErreurService, Resultat};
use crate::facturation::etat_echeances;
use crate::inscriptions::{ServiceInscriptions, jour};
use crate::noyau::Noyau;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

fn calcul<T>(r: Result<T, gesco_core::commun::ErreurMontant>) -> Resultat<T> {
    r.map_err(|_| regle("recouvrement.calcul_impossible"))
}

fn nombre(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// Payeur d'un élève et ses téléphones.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct PayeurAffiche {
    /// Prénoms et nom.
    pub nom: String,
    /// Téléphones, forme canonique.
    pub telephones: Vec<String>,
}

/// Élève en retard de paiement.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct LigneImpaye {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Élève.
    pub eleve_id: Id<Eleve>,
    /// Prénoms et nom.
    pub eleve: String,
    /// Matricule interne.
    pub matricule: String,
    /// Classe.
    pub classe: String,
    /// Numéro de la facture.
    pub facture: String,
    /// Échu, total, ancienneté.
    pub impaye: Impaye,
    /// Payeurs, le premier d'abord.
    pub payeurs: Vec<PayeurAffiche>,
    /// Année clôturée dont reste cet arriéré (T-67) ; aucune pour l'année en cours.
    pub arriere: Option<String>,
}

/// Total d'une classe.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct TotalClasse {
    /// Classe.
    pub classe: String,
    /// Élèves en retard.
    pub eleves: u32,
    /// Reste échu.
    pub echu: Montant,
    /// Reste dû total.
    pub total: Montant,
}

/// Total d'une tranche d'ancienneté.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct TotalTranche {
    /// Tranche.
    pub tranche: TrancheAnciennete,
    /// Premier jour de retard de la tranche.
    pub de: i64,
    /// Dernier jour de retard compris ; aucun pour la dernière tranche (« plus de `de` jours »).
    pub a: Option<i64>,
    /// Élèves.
    pub eleves: u32,
    /// Reste échu.
    pub echu: Montant,
}

/// Impayés de l'année en cours à la date du jour.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ListeImpayes {
    /// Année scolaire.
    pub annee: String,
    /// Date de calcul.
    pub date: DateCivile,
    /// Élèves en retard, par classe puis par nom.
    pub lignes: Vec<LigneImpaye>,
    /// Totaux par classe.
    pub par_classe: Vec<TotalClasse>,
    /// Totaux par tranche, de la plus récente à la plus ancienne.
    pub par_tranche: Vec<TotalTranche>,
    /// Reste échu de l'école.
    pub echu: Montant,
    /// Reste dû total de ces élèves.
    pub total: Montant,
}

/// Période et filtres du journal.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeJournal {
    /// Premier jour.
    pub du: DateCivile,
    /// Dernier jour, compris.
    pub au: DateCivile,
    /// Un caissier ; absent : tous.
    pub caissier_id: Option<Id<Utilisateur>>,
    /// Un mode ; absent : tous.
    pub mode: Option<ModePaiement>,
}

/// Versement du journal.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct LigneJournal {
    /// Versement, annulé ou non.
    pub versement: Versement,
    /// Élève : prénoms et nom.
    pub eleve: String,
    /// Matricule.
    pub matricule: String,
    /// Classe.
    pub classe: String,
    /// Caissier.
    pub caissier: String,
}

/// Total d'un jour.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct TotalJour {
    /// Jour.
    pub jour: DateCivile,
    /// Reçus valides.
    pub nombre: u32,
    /// Montant encaissé.
    pub montant: Montant,
}

/// Total d'un mode.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct TotalModeJournal {
    /// Mode.
    pub mode: ModePaiement,
    /// Reçus valides.
    pub nombre: u32,
    /// Montant encaissé.
    pub montant: Montant,
}

/// Caissier présent dans la période, pour le filtre.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct CaissierJournal {
    /// Utilisateur.
    pub id: Id<Utilisateur>,
    /// Nom affiché.
    pub nom: String,
}

/// Journal des encaissements d'une période.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct JournalEncaissements {
    /// Versements, annulés compris, dans l'ordre des reçus.
    pub lignes: Vec<LigneJournal>,
    /// Totaux des versements valides par jour.
    pub par_jour: Vec<TotalJour>,
    /// Totaux des versements valides par mode.
    pub par_mode: Vec<TotalModeJournal>,
    /// Reçus valides.
    pub nombre: u32,
    /// Total encaissé, annulations déduites.
    pub total: Montant,
    /// Versements annulés.
    pub annules: u32,
    /// Montant des versements annulés.
    pub montant_annule: Montant,
    /// Caissiers de la période, tous filtres de caissier et de mode ignorés.
    pub caissiers: Vec<CaissierJournal>,
}

/// Service du recouvrement.
pub struct ServiceRecouvrement<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceRecouvrement<'n> {
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

    /// Impayés de l'année en cours (UC-05) : chaque élève dont une échéance passée n'est pas
    /// soldée, avec ce qui est échu, ce qui reste en tout, l'ancienneté et ses payeurs ; puis les
    /// arriérés des années clôturées, qui suivent l'élève (T-67).
    pub async fn impayes(&self, session: &Session) -> Resultat<ListeImpayes> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let aujourd_hui = jour(contexte.maintenant)?;
        let (annee, mut closes) = {
            let mut cnx = self.connexion().await?;
            let annee = annee_scolaire::courante(&mut cnx, &contexte)
                .await?
                .ok_or(regle("recouvrement.annee_absente"))?;
            let closes: Vec<_> = annee_scolaire::lister(&mut cnx, &contexte)
                .await?
                .into_iter()
                .filter(|a| {
                    a.id != annee.id
                        && matches!(a.statut, StatutAnnee::Cloturee | StatutAnnee::Archivee)
                })
                .collect();
            (annee, closes)
        };
        closes.sort_by_key(|a| std::cmp::Reverse(a.debut));
        let mut inscriptions = Vec::new();
        for a in std::iter::once(&annee).chain(&closes) {
            let arriere = (a.id != annee.id).then(|| a.libelle.clone());
            for mut l in ServiceInscriptions::nouveau(self.noyau)
                .lister(session, a.id)
                .await?
            {
                if let Some(libelle) = &arriere {
                    l.classe = format!("{} · {libelle}", l.classe);
                }
                inscriptions.push((l, arriere.clone()));
            }
        }
        let mut cnx = self.connexion().await?;
        // Les limites des tranches sont celles de l'école ; un réglage illisible garde les livrées.
        let tranches =
            Tranches::lire(&parametre::texte(&mut cnx, PARAMETRE_TRANCHES, "", &contexte).await?)
                .unwrap_or(Tranches::DEFAUT);
        let mut ouvertes = HashMap::new();
        for a in std::iter::once(&annee).chain(&closes) {
            for facture in factures::factures_ouvertes(&mut cnx, a.id, &contexte).await? {
                let echeances: Vec<_> = etat_echeances(&mut cnx, facture.id, &contexte)
                    .await?
                    .iter()
                    .map(|e| (e.echeance.date, e.reste))
                    .collect();
                if let Some(mut i) = calcul(impaye(&echeances, aujourd_hui))? {
                    i.tranche = tranches.classer(i.jours);
                    ouvertes.insert(facture.inscription_id, (facture.numero, i));
                }
            }
        }
        let mut payeurs: HashMap<Id<Eleve>, Vec<PayeurAffiche>> = HashMap::new();
        for p in personnes::payeurs(&mut cnx, &contexte).await? {
            payeurs.entry(p.eleve_id).or_default().push(PayeurAffiche {
                nom: p.nom,
                telephones: p.telephones,
            });
        }
        drop(cnx);

        let mut lignes = Vec::new();
        for (l, arriere) in inscriptions {
            let Some((facture, impaye)) = ouvertes.remove(&l.inscription.id) else {
                continue;
            };
            lignes.push(LigneImpaye {
                inscription_id: l.inscription.id,
                eleve_id: l.inscription.eleve_id,
                payeurs: payeurs
                    .get(&l.inscription.eleve_id)
                    .cloned()
                    .unwrap_or_default(),
                eleve: l.eleve,
                matricule: l.matricule,
                classe: l.classe,
                facture,
                impaye,
                arriere,
            });
        }
        let mut par_classe: Vec<TotalClasse> = Vec::new();
        for l in &lignes {
            if par_classe.last().is_none_or(|c| c.classe != l.classe) {
                par_classe.push(TotalClasse {
                    classe: l.classe.clone(),
                    eleves: 0,
                    echu: Montant::ZERO,
                    total: Montant::ZERO,
                });
            }
            if let Some(c) = par_classe.last_mut() {
                c.eleves += 1;
                c.echu = calcul(c.echu.ajouter(l.impaye.echu))?;
                c.total = calcul(c.total.ajouter(l.impaye.total))?;
            }
        }
        let mut par_tranche = Vec::new();
        for tranche in TrancheAnciennete::TOUTES {
            let dans: Vec<_> = lignes
                .iter()
                .filter(|l| l.impaye.tranche == *tranche)
                .collect();
            let (de, a) = tranches.bornes(*tranche);
            par_tranche.push(TotalTranche {
                tranche: *tranche,
                de,
                a,
                eleves: nombre(dans.len()),
                echu: calcul(
                    dans.iter()
                        .try_fold(Montant::ZERO, |s, l| s.ajouter(l.impaye.echu)),
                )?,
            });
        }
        let echu = calcul(
            par_classe
                .iter()
                .try_fold(Montant::ZERO, |s, c| s.ajouter(c.echu)),
        )?;
        let total = calcul(
            par_classe
                .iter()
                .try_fold(Montant::ZERO, |s, c| s.ajouter(c.total)),
        )?;
        Ok(ListeImpayes {
            annee: annee.libelle,
            date: aujourd_hui,
            lignes,
            par_classe,
            par_tranche,
            echu,
            total,
        })
    }

    /// Journal des encaissements d'une période (§9.2) : les reçus, annulés compris, et les
    /// totaux par jour et par mode des versements valides.
    pub async fn journal(
        &self,
        session: &Session,
        demande: &DemandeJournal,
    ) -> Resultat<JournalEncaissements> {
        if demande.au < demande.du {
            return Err(regle("journal.periode_invalide"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let debut =
            |d: DateCivile| Horodatage::depuis_millis_epoque(d.jours_depuis_epoque() * 86_400_000);
        let fin =
            Horodatage::depuis_millis_epoque((demande.au.jours_depuis_epoque() + 1) * 86_400_000);
        let tous = {
            let mut cnx = self.connexion().await?;
            versements::de_la_periode(&mut cnx, debut(demande.du), fin, &contexte).await?
        };
        let mut caissiers: Vec<CaissierJournal> = Vec::new();
        for v in &tous {
            if !caissiers.iter().any(|c| c.id == v.versement.encaisse_par) {
                caissiers.push(CaissierJournal {
                    id: v.versement.encaisse_par,
                    nom: v.caissier.clone(),
                });
            }
        }
        caissiers.sort_by(|a, b| a.nom.cmp(&b.nom));
        let lignes: Vec<LigneJournal> = tous
            .into_iter()
            .filter(|v| {
                demande
                    .caissier_id
                    .is_none_or(|c| v.versement.encaisse_par == c)
            })
            .filter(|v| demande.mode.is_none_or(|m| v.versement.mode == m))
            .map(|v| LigneJournal {
                versement: v.versement,
                eleve: v.eleve,
                matricule: v.matricule,
                classe: v.classe,
                caissier: v.caissier,
            })
            .collect();
        let mut par_jour: Vec<TotalJour> = Vec::new();
        let mut par_mode: Vec<TotalModeJournal> = Vec::new();
        let (mut nombre_valides, mut total) = (0usize, Montant::ZERO);
        let (mut annules, mut montant_annule) = (0usize, Montant::ZERO);
        for l in &lignes {
            let v = &l.versement;
            if v.statut == StatutVersement::Annule {
                annules += 1;
                montant_annule = calcul(montant_annule.ajouter(v.montant))?;
                continue;
            }
            nombre_valides += 1;
            total = calcul(total.ajouter(v.montant))?;
            let j = jour(v.encaisse_le)?;
            if par_jour.last().is_none_or(|t| t.jour != j) {
                par_jour.push(TotalJour {
                    jour: j,
                    nombre: 0,
                    montant: Montant::ZERO,
                });
            }
            if let Some(t) = par_jour.last_mut() {
                t.nombre += 1;
                t.montant = calcul(t.montant.ajouter(v.montant))?;
            }
            match par_mode.iter_mut().find(|t| t.mode == v.mode) {
                Some(t) => {
                    t.nombre += 1;
                    t.montant = calcul(t.montant.ajouter(v.montant))?;
                }
                None => par_mode.push(TotalModeJournal {
                    mode: v.mode,
                    nombre: 1,
                    montant: v.montant,
                }),
            }
        }
        par_mode.sort_by_key(|t| ModePaiement::TOUS.iter().position(|m| *m == t.mode));
        Ok(JournalEncaissements {
            lignes,
            par_jour,
            par_mode,
            nombre: nombre(nombre_valides),
            total,
            annules: nombre(annules),
            montant_annule,
            caissiers,
        })
    }
}
