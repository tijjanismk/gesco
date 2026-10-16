//! Paie du mois (SPEC_PHASE_7 §5) : calcul de la paie de tout le personnel payé ce mois-là,
//! revue par la direction (primes et régularisations du mois, retenues saisies, absences non
//! justifiées retenues ou non, une à une), validation qui fige et numérote les bulletins et
//! marque les heures payées. Les mois se suivent ; une paie validée ne se recalcule plus.

use gesco_core::commun::{DateCivile, Id, Montant};
use gesco_core::core::{MachineAEtats, Operation, TypeDocumentNumerote, TypeEvenement};
use gesco_core::paie::calcul::{
    AbsenceRetenue, AvanceARembourser, Cumuls, DonneesBulletin, HeuresAPayer, ResultatBulletin,
    SaisiePaie, SorteLigne, calculer,
};
use gesco_core::paie::element::version_en_vigueur;
use gesco_core::paie::mois::{mois_calculable, verifier_saisie};
use gesco_core::paie::{
    AbsenceConfirmee, Avance, BulletinPaie, Calcul, ElementPaie, PaieMois, SaisieMois,
    StatutAvance, StatutPaie, VersionElement,
};
use gesco_core::rh::{
    AbsencePersonnel, MembrePersonnel, Mois, NatureAbsencePersonnel, Remuneration,
};
use gesco_db::depots::{conges, paie, parametre, personnel, personnes};
use gesco_db::nouvel_id;
use gesco_db::unite::Durabilite;
use gesco_db::{Contexte, UniteDeTravail};
use serde::{Deserialize, Serialize};

use crate::conges::PARAMETRE_SAMEDI;
use crate::effets::Effet;
use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;
use crate::numerotation;
use crate::paie::reglages;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Bulletin affiché dans la paie du mois.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct BulletinPaieAffiche {
    /// Bulletin.
    pub bulletin: BulletinPaie,
    /// Prénoms et nom.
    pub membre: String,
    /// Matricule du personnel.
    pub matricule: String,
    /// Fonction.
    pub fonction: String,
}

/// Absence non justifiée du mois, que la direction retient ou non.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct AbsenceAConfirmer {
    /// Absence.
    pub absence: AbsencePersonnel,
    /// Prénoms et nom du membre.
    pub membre: String,
    /// Décision : retenue, laissée, ou pas encore prise.
    pub retenue: Option<bool>,
}

/// Montant saisi dans la paie, avec le nom du membre et de l'élément.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct SaisieAffichee {
    /// Saisie.
    pub saisie: SaisieMois,
    /// Prénoms et nom du membre.
    pub membre: String,
    /// Libellé de l'élément.
    pub element: String,
}

/// Paie d'un mois telle que l'écran la montre.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct PaieDuMois {
    /// Mois.
    pub mois: Mois,
    /// Paie, si elle a été calculée.
    pub paie: Option<PaieMois>,
    /// Bulletins des membres payés, par nom.
    pub bulletins: Vec<BulletinPaieAffiche>,
    /// Montants saisis et non retirés.
    pub saisies: Vec<SaisieAffichee>,
    /// Absences non justifiées du mois.
    pub absences: Vec<AbsenceAConfirmer>,
    /// Éléments qui se saisissent dans la paie du mois : primes, régularisations, retenues.
    pub elements_saisis: Vec<ElementPaie>,
    /// Membres de l'école encore présents, pour saisir une prime : identifiant et nom.
    pub membres: Vec<(Id<MembrePersonnel>, String)>,
    /// Total des bruts.
    pub brut: Montant,
    /// Total des nets à payer.
    pub net: Montant,
    /// Total des cotisations patronales.
    pub patronales: Montant,
    /// Lignes d'heures du mois encore en saisie : elles attendent leur visa.
    pub heures_en_saisie: i64,
    /// Une paie d'un mois précédent attend sa validation.
    pub precedente_a_valider: Option<Mois>,
}

/// La paie au tableau de bord (SPEC_PHASE_7 §10) : celle du mois écoulé et ce qui reste dû.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct BlocPaie {
    /// Mois écoulé.
    pub mois: Mois,
    /// État de sa paie ; aucune si elle n'est pas encore calculée.
    pub statut: Option<StatutPaie>,
    /// Bulletins de ce mois.
    pub bulletins: u32,
    /// Bulletins de ce mois entièrement payés.
    pub payes: u32,
    /// Net encore dû, tous mois confondus.
    pub reste_du: Montant,
    /// L'école tient sa paie dans GESCO et celle du mois écoulé n'est pas validée après le 5.
    pub a_valider: bool,
}

/// Mois désigné.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeMoisPaie {
    /// Mois.
    pub mois: Mois,
}

/// Montant saisi dans la paie du mois.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeSaisiePaie {
    /// Mois.
    pub mois: Mois,
    /// Membre.
    pub membre_id: Id<MembrePersonnel>,
    /// Élément « saisi ».
    pub element_id: Id<ElementPaie>,
    /// Montant.
    pub montant: Montant,
    /// Libellé : le motif.
    pub libelle: String,
}

/// Montant saisi désigné.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeIdSaisie {
    /// Saisie.
    pub id: Id<SaisieMois>,
}

/// Décision sur une absence non justifiée.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeAbsencePaie {
    /// Mois.
    pub mois: Mois,
    /// Absence.
    pub absence_id: Id<AbsencePersonnel>,
    /// Retenue sur le salaire.
    pub retenue: bool,
}

/// Service de la paie du mois.
pub struct ServicePaieMois<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServicePaieMois<'n> {
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

    /// Paie d'un mois.
    pub async fn lire(&self, session: &Session, mois: Mois) -> Resultat<PaieDuMois> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let paies = paie::paies(&mut cnx, &contexte).await?;
        let paie_du_mois = paies.iter().find(|p| p.mois == mois).cloned();
        let precedente_a_valider = paies
            .iter()
            .find(|p| p.mois < mois && p.statut == StatutPaie::Calculee)
            .map(|p| p.mois);
        let membres = personnel::membres(&mut cnx, &contexte).await?;
        let mut noms = Vec::with_capacity(membres.len());
        for m in &membres {
            let p = personnes::personne(&mut cnx, m.personne_id, &contexte).await?;
            noms.push((m.id, p.nom_complet()));
        }
        let nom = |id: Id<MembrePersonnel>| {
            noms.iter()
                .find(|(m, _)| *m == id)
                .map(|(_, n)| n.clone())
                .unwrap_or_default()
        };
        let elements = paie::elements(&mut cnx, &contexte).await?;
        let versions = paie::versions(&mut cnx, &contexte).await?;
        let elements_saisis: Vec<ElementPaie> = elements
            .iter()
            .filter(|e| {
                e.actif && matches!(calcul_du_mois(&versions, e.id, mois), Some(Calcul::Saisi))
            })
            .cloned()
            .collect();
        let (mut bulletins, saisies, confirmations) = match &paie_du_mois {
            Some(p) => (
                paie::bulletins_de_la_paie(&mut cnx, p.id, &contexte).await?,
                paie::saisies_de_la_paie(&mut cnx, p.id, &contexte).await?,
                paie::confirmations(&mut cnx, p.id, &contexte).await?,
            ),
            None => (Vec::new(), Vec::new(), Vec::new()),
        };
        bulletins.retain(|b| !b.exclu);
        let mut affiches: Vec<BulletinPaieAffiche> = bulletins
            .into_iter()
            .map(|b| {
                let membre = membres.iter().find(|m| m.id == b.membre_id);
                BulletinPaieAffiche {
                    membre: nom(b.membre_id),
                    matricule: membre.map(|m| m.matricule.clone()).unwrap_or_default(),
                    fonction: membre.map(|m| m.fonction.clone()).unwrap_or_default(),
                    bulletin: b,
                }
            })
            .collect();
        affiches.sort_by(|a, b| a.membre.cmp(&b.membre));
        let total = |f: fn(&ResultatBulletin) -> Montant| {
            affiches
                .iter()
                .try_fold(Montant::ZERO, |s, b| s.ajouter(f(&b.bulletin.resultat)))
                .map_err(|_| regle("paie.calcul_impossible"))
        };
        let absences = absences_du_mois(&mut cnx, mois, &contexte)
            .await?
            .into_iter()
            .map(|a| AbsenceAConfirmer {
                membre: nom(a.membre_id),
                retenue: confirmations
                    .iter()
                    .find(|c| c.absence_id == a.id)
                    .map(|c| c.retenue),
                absence: a,
            })
            .collect();
        let heures_en_saisie = personnel::heures_du_mois(&mut cnx, mois, &contexte)
            .await?
            .iter()
            .filter(|h| h.statut != gesco_core::rh::StatutHeures::Visee)
            .count() as i64;
        let saisies = saisies
            .into_iter()
            .filter(|s| !s.retiree)
            .map(|s| SaisieAffichee {
                membre: nom(s.membre_id),
                element: elements
                    .iter()
                    .find(|e| e.id == s.element_id)
                    .map(|e| e.libelle.clone())
                    .unwrap_or_default(),
                saisie: s,
            })
            .collect();
        let presents = membres
            .iter()
            .filter(|m| {
                m.parti_le
                    .is_none_or(|d| mois.premier_jour().is_some_and(|p| d >= p))
            })
            .map(|m| (m.id, nom(m.id)))
            .collect();
        Ok(PaieDuMois {
            mois,
            paie: paie_du_mois,
            brut: total(|r| r.brut)?,
            net: total(|r| r.net)?,
            patronales: total(|r| r.patronales)?,
            bulletins: affiches,
            saisies,
            absences,
            elements_saisis,
            membres: presents,
            heures_en_saisie,
            precedente_a_valider,
        })
    }

    /// La paie au tableau de bord : le mois écoulé, ses bulletins payés, ce qui reste dû.
    pub async fn bloc(&self, session: &Session) -> Resultat<BlocPaie> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let aujourd_hui =
            DateCivile::du_jour(contexte.maintenant).ok_or(regle("paie.calcul_impossible"))?;
        let mois = Mois::de(aujourd_hui)
            .precedent()
            .ok_or(regle("paie.calcul_impossible"))?;
        let (paies, bulletins) = {
            let mut cnx = self.connexion().await?;
            let paies = paie::paies(&mut cnx, &contexte).await?;
            let bulletins = match paies.iter().find(|p| p.mois == mois) {
                Some(p) => paie::bulletins_de_la_paie(&mut cnx, p.id, &contexte)
                    .await?
                    .into_iter()
                    .filter(|b| !b.exclu)
                    .count(),
                None => 0,
            };
            (paies, bulletins)
        };
        let statut = paies.iter().find(|p| p.mois == mois).map(|p| p.statut);
        let a_payer = crate::paiements_personnel::ServicePaiementsPersonnel::nouveau(self.noyau)
            .lire(session)
            .await?;
        let dus_du_mois = a_payer.a_payer.iter().filter(|a| a.mois == mois).count();
        let utilise = paies.iter().any(|p| p.statut == StatutPaie::Validee);
        let compter = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
        Ok(BlocPaie {
            mois,
            statut,
            bulletins: compter(bulletins),
            payes: compter(bulletins.saturating_sub(dus_du_mois)),
            reste_du: a_payer.total_du,
            a_valider: utilise && aujourd_hui.jour() > 5 && statut != Some(StatutPaie::Validee),
        })
    }

    /// Calcule ou refait la paie du mois.
    pub async fn calculer(&self, session: &Session, mois: Mois) -> Resultat<PaieDuMois> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let aujourd_hui =
            DateCivile::du_jour(contexte.maintenant).ok_or(regle("paie.calcul_impossible"))?;
        if !mois_calculable(mois, Mois::de(aujourd_hui)) {
            return Err(regle("paie.mois_a_venir"));
        }
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let paie_mois = preparer_paie(&mut unite, mois, &contexte).await?;
        recalculer(&mut unite, &paie_mois, &contexte).await?;
        self.noyau.terminer(unite).await?;
        self.lire(session, mois).await
    }

    /// Saisit une prime, une régularisation ou une retenue du mois pour un membre, puis refait
    /// la paie.
    pub async fn saisir(
        &self,
        session: &Session,
        demande: &DemandeSaisiePaie,
    ) -> Resultat<PaieDuMois> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let paie_mois = paie_en_revue(&mut unite, demande.mois, &contexte).await?;
        let elements = paie::elements(unite.connexion(), &contexte).await?;
        let versions = paie::versions(unite.connexion(), &contexte).await?;
        let element = elements
            .iter()
            .find(|e| e.id == demande.element_id && e.actif)
            .ok_or(regle("paie.element_non_saisi"))?;
        let calcul = calcul_du_mois(&versions, element.id, demande.mois);
        verifier_saisie(element, calcul.as_ref(), demande.montant, &demande.libelle)
            .map_err(regle)?;
        personnel::membre(unite.connexion(), demande.membre_id, &contexte).await?;
        let saisie = SaisieMois {
            id: nouvel_id(),
            etablissement_id: session.etablissement_id,
            paie_id: paie_mois.id,
            membre_id: demande.membre_id,
            element_id: element.id,
            montant: demande.montant,
            libelle: demande.libelle.trim().to_owned(),
            retiree: false,
        };
        paie::inserer_saisie(unite.connexion(), &saisie, &contexte).await?;
        unite.auditer(
            "saisie_paie",
            &saisie.id.vers_chaine(),
            Operation::Creation,
            None,
            Some(serde_json::to_value(&saisie)?),
            "paie.saisir",
        );
        recalculer(&mut unite, &paie_mois, &contexte).await?;
        self.noyau.terminer(unite).await?;
        self.lire(session, demande.mois).await
    }

    /// Retire un montant saisi, puis refait la paie.
    pub async fn retirer_saisie(
        &self,
        session: &Session,
        demande: &DemandeIdSaisie,
    ) -> Resultat<PaieDuMois> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let saisie = paie::saisie(unite.connexion(), demande.id, &contexte).await?;
        let paie_mois = paie::paies(unite.connexion(), &contexte)
            .await?
            .into_iter()
            .find(|p| p.id == saisie.paie_id)
            .ok_or(regle("paie.non_calculee"))?;
        if paie_mois.statut != StatutPaie::Calculee {
            return Err(regle("paie.validee"));
        }
        if !saisie.retiree {
            paie::retirer_saisie(unite.connexion(), saisie.id, &contexte).await?;
            unite.auditer(
                "saisie_paie",
                &saisie.id.vers_chaine(),
                Operation::Modification,
                Some(serde_json::json!({ "retiree": false })),
                Some(serde_json::json!({ "retiree": true })),
                "paie.retirer_saisie",
            );
            recalculer(&mut unite, &paie_mois, &contexte).await?;
        }
        self.noyau.terminer(unite).await?;
        self.lire(session, paie_mois.mois).await
    }

    /// Retient ou laisse une absence non justifiée du mois, puis refait la paie.
    pub async fn confirmer_absence(
        &self,
        session: &Session,
        demande: &DemandeAbsencePaie,
    ) -> Resultat<PaieDuMois> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let paie_mois = paie_en_revue(&mut unite, demande.mois, &contexte).await?;
        if !absences_du_mois(unite.connexion(), demande.mois, &contexte)
            .await?
            .iter()
            .any(|a| a.id == demande.absence_id)
        {
            return Err(regle("paie.absence_hors_mois"));
        }
        let confirmation = AbsenceConfirmee {
            id: nouvel_id(),
            etablissement_id: session.etablissement_id,
            paie_id: paie_mois.id,
            absence_id: demande.absence_id,
            retenue: demande.retenue,
        };
        paie::poser_confirmation(unite.connexion(), &confirmation, &contexte).await?;
        unite.auditer(
            "absence_personnel",
            &demande.absence_id.vers_chaine(),
            Operation::Modification,
            None,
            Some(serde_json::json!({ "mois": demande.mois, "retenue": demande.retenue })),
            "paie.absence",
        );
        recalculer(&mut unite, &paie_mois, &contexte).await?;
        self.noyau.terminer(unite).await?;
        self.lire(session, demande.mois).await
    }

    /// Valide la paie du mois : elle se fige, chaque bulletin reçoit son numéro, les heures
    /// payées sont marquées.
    pub async fn valider(&self, session: &Session, mois: Mois) -> Resultat<PaieDuMois> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let paies = paie::paies(unite.connexion(), &contexte).await?;
        let avant = paies
            .iter()
            .find(|p| p.mois == mois)
            .cloned()
            .ok_or(regle("paie.non_calculee"))?;
        if avant.statut == StatutPaie::Validee {
            return Err(regle("paie.validee"));
        }
        let statut = avant.statut.transiter(StatutPaie::Validee)?;
        if paies
            .iter()
            .any(|p| p.mois < mois && p.statut == StatutPaie::Calculee)
        {
            return Err(regle("paie.precedente_a_valider"));
        }
        // Le calcul repart des données du jour : rien de ce qui a changé depuis n'est oublié.
        recalculer(&mut unite, &avant, &contexte).await?;
        let bulletins: Vec<BulletinPaie> =
            paie::bulletins_de_la_paie(unite.connexion(), avant.id, &contexte)
                .await?
                .into_iter()
                .filter(|b| !b.exclu)
                .collect();
        if bulletins.iter().any(|b| {
            b.resultat
                .avertissements
                .iter()
                .any(|a| a == "paie.net_negatif")
        }) {
            return Err(regle("paie.net_negatif"));
        }
        let mut par_nom = Vec::with_capacity(bulletins.len());
        for b in bulletins {
            let m = personnel::membre(unite.connexion(), b.membre_id, &contexte).await?;
            let p = personnes::personne(unite.connexion(), m.personne_id, &contexte).await?;
            par_nom.push((p.nom_complet(), b));
        }
        par_nom.sort_by(|a, b| a.0.cmp(&b.0));
        let exercice = mois.annee().to_string();
        let mut net = Montant::ZERO;
        for (_, b) in &par_nom {
            let numero =
                numerotation::attribuer(&mut unite, TypeDocumentNumerote::BulletinPaie, &exercice)
                    .await?
                    .numero;
            paie::numeroter_bulletin(unite.connexion(), b.id, &numero, &contexte).await?;
            let id_bulletin = b.id.vers_chaine();
            for heures_id in b.resultat.lignes.iter().filter_map(|l| l.heures_id) {
                personnel::marquer_heures_payees(
                    unite.connexion(),
                    heures_id,
                    &id_bulletin,
                    &contexte,
                )
                .await?;
            }
            for ligne in b
                .resultat
                .lignes
                .iter()
                .filter(|l| l.sorte == SorteLigne::Avance && l.montant.francs() > 0)
            {
                if let Some(avance_id) = ligne.avance_id {
                    paie::inserer_remboursement(
                        unite.connexion(),
                        &nouvel_id::<Avance>().vers_chaine(),
                        avance_id,
                        b.id,
                        ligne.montant,
                        &contexte,
                    )
                    .await?;
                }
            }
            net = net
                .ajouter(b.resultat.net)
                .map_err(|_| regle("paie.calcul_impossible"))?;
        }
        solder_avances(&mut unite, &contexte).await?;
        let apres = PaieMois {
            statut,
            validee_le: Some(contexte.maintenant),
            validee_par: Some(session.utilisateur_id),
            ..avant.clone()
        };
        paie::ecrire_paie(unite.connexion(), &apres, &contexte).await?;
        let id = apres.id.vers_chaine();
        unite.auditer(
            "paie_mois",
            &id,
            Operation::Modification,
            Some(serde_json::to_value(&avant)?),
            Some(serde_json::to_value(&apres)?),
            "paie.valider",
        );
        unite.emettre(
            TypeEvenement::PaieValidee,
            "paie_mois",
            &id,
            serde_json::json!({ "mois": mois, "bulletins": par_nom.len(), "net": net }),
        );
        self.noyau.terminer(unite).await?;
        self.lire(session, mois).await
    }
}

/// Solde les avances entièrement remboursées.
async fn solder_avances(unite: &mut UniteDeTravail<Effet>, contexte: &Contexte) -> Resultat<()> {
    let rembourses = paie::rembourse_par_avance(unite.connexion(), contexte).await?;
    for avant in paie::avances(unite.connexion(), contexte)
        .await?
        .into_iter()
        .filter(|a| a.statut == StatutAvance::EnCours)
    {
        let rembourse = rembourses
            .iter()
            .find(|(id, _)| *id == avant.id)
            .map_or(0, |(_, m)| m.francs());
        if rembourse < avant.montant.francs() {
            continue;
        }
        let apres = Avance {
            statut: avant.statut.transiter(StatutAvance::Soldee)?,
            ..avant.clone()
        };
        paie::ecrire_statut_avance(unite.connexion(), &apres, contexte).await?;
        unite.auditer(
            "avance_salaire",
            &apres.id.vers_chaine(),
            Operation::Modification,
            Some(serde_json::json!({ "statut": avant.statut })),
            Some(serde_json::json!({ "statut": apres.statut })),
            "paie.avance_soldee",
        );
    }
    Ok(())
}

/// Calcul d'un élément en vigueur dans un mois.
fn calcul_du_mois(
    versions: &[VersionElement],
    element: Id<ElementPaie>,
    mois: Mois,
) -> Option<Calcul> {
    let siennes: Vec<VersionElement> = versions
        .iter()
        .filter(|v| v.element_id == element)
        .cloned()
        .collect();
    version_en_vigueur(&siennes, mois).map(|v| v.calcul.clone())
}

/// Paie du mois à calculer : la crée au premier calcul ; refuse une paie validée, ou un mois
/// dont un précédent attend encore sa validation.
async fn preparer_paie(
    unite: &mut UniteDeTravail<Effet>,
    mois: Mois,
    contexte: &Contexte,
) -> Resultat<PaieMois> {
    let paies = paie::paies(unite.connexion(), contexte).await?;
    if paies
        .iter()
        .any(|p| p.mois < mois && p.statut == StatutPaie::Calculee)
    {
        return Err(regle("paie.precedente_a_valider"));
    }
    match paies.into_iter().find(|p| p.mois == mois) {
        Some(p) if p.statut == StatutPaie::Validee => Err(regle("paie.validee")),
        Some(p) => Ok(p),
        None => {
            let p = PaieMois {
                id: nouvel_id(),
                etablissement_id: contexte.etablissement_id,
                mois,
                statut: StatutPaie::Calculee,
                calculee_le: contexte.maintenant,
                calculee_par: contexte.utilisateur_id,
                validee_le: None,
                validee_par: None,
            };
            paie::inserer_paie(unite.connexion(), &p, contexte).await?;
            unite.auditer(
                "paie_mois",
                &p.id.vers_chaine(),
                Operation::Creation,
                None,
                Some(serde_json::to_value(&p)?),
                "paie.calculer",
            );
            Ok(p)
        }
    }
}

/// Paie du mois calculée et pas encore validée : la seule où la direction saisit et décide.
async fn paie_en_revue(
    unite: &mut UniteDeTravail<Effet>,
    mois: Mois,
    contexte: &Contexte,
) -> Resultat<PaieMois> {
    match paie::paie_du_mois(unite.connexion(), mois, contexte).await? {
        None => Err(regle("paie.non_calculee")),
        Some(p) if p.statut == StatutPaie::Validee => Err(regle("paie.validee")),
        Some(p) => Ok(p),
    }
}

/// Absences non justifiées et non annulées du mois, en journées et demi-journées : un retard ne
/// se retient pas.
async fn absences_du_mois(
    cnx: &mut gesco_db::Connexion,
    mois: Mois,
    contexte: &Contexte,
) -> Resultat<Vec<AbsencePersonnel>> {
    let (Some(debut), Some(fin)) = (mois.premier_jour(), mois.dernier_jour()) else {
        return Err(regle("paie.calcul_impossible"));
    };
    Ok(conges::absences_entre(cnx, debut, fin, contexte)
        .await?
        .into_iter()
        .filter(|a| !a.justifiee && !a.annulee && a.nature != NatureAbsencePersonnel::Retard)
        .collect())
}

/// Refait tous les bulletins de la paie d'après les données du jour : engagements, heures visées
/// à payer, éléments, saisies, absences retenues, cumuls des bulletins validés de l'année.
async fn recalculer(
    unite: &mut UniteDeTravail<Effet>,
    paie_mois: &PaieMois,
    contexte: &Contexte,
) -> Resultat<()> {
    let mois = paie_mois.mois;
    let reglages = reglages(unite.connexion(), contexte).await?;
    let samedi_compte =
        parametre::booleen_param(unite.connexion(), PARAMETRE_SAMEDI, false, contexte).await?;
    let membres = personnel::membres(unite.connexion(), contexte).await?;
    let engagements = personnel::engagements(unite.connexion(), contexte).await?;
    let elements = paie::elements(unite.connexion(), contexte).await?;
    let versions = paie::versions(unite.connexion(), contexte).await?;
    let attributions = paie::attributions(unite.connexion(), contexte).await?;
    let heures = personnel::heures_a_payer(unite.connexion(), mois, contexte).await?;
    let saisies: Vec<SaisieMois> =
        paie::saisies_de_la_paie(unite.connexion(), paie_mois.id, contexte)
            .await?
            .into_iter()
            .filter(|s| !s.retiree)
            .collect();
    let retenues: Vec<Id<AbsencePersonnel>> =
        paie::confirmations(unite.connexion(), paie_mois.id, contexte)
            .await?
            .into_iter()
            .filter(|c| c.retenue)
            .map(|c| c.absence_id)
            .collect();
    let absences = absences_du_mois(unite.connexion(), mois, contexte).await?;
    let existants = paie::bulletins_de_la_paie(unite.connexion(), paie_mois.id, contexte).await?;
    let avances: Vec<Avance> = paie::avances(unite.connexion(), contexte)
        .await?
        .into_iter()
        .filter(|a| a.statut == StatutAvance::EnCours && a.premier_mois <= mois)
        .collect();
    let rembourses = paie::rembourse_par_avance(unite.connexion(), contexte).await?;
    let mut brut_total = Montant::ZERO;
    let mut payes = 0_i64;
    for membre in &membres {
        let siens: Vec<_> = engagements
            .iter()
            .filter(|e| e.membre_id == membre.id)
            .cloned()
            .collect();
        let a_payer: Vec<HeuresAPayer> = heures
            .iter()
            .filter(|h| h.membre_id == membre.id)
            .filter_map(|h| {
                let taux = siens
                    .iter()
                    .find(|e| e.id == h.engagement_id)
                    .and_then(|e| match e.remuneration {
                        Remuneration::Horaire { taux, .. } => Some(taux),
                        _ => None,
                    })?;
                Some(HeuresAPayer {
                    heures_id: h.id,
                    engagement_id: h.engagement_id,
                    mois: h.mois,
                    minutes: h.minutes_faites.unwrap_or_default(),
                    taux,
                })
            })
            .collect();
        let saisies_du_membre: Vec<SaisiePaie> = saisies
            .iter()
            .filter(|s| s.membre_id == membre.id)
            .map(|s| SaisiePaie {
                element_id: s.element_id,
                montant: s.montant,
                libelle: s.libelle.clone(),
            })
            .collect();
        let absences_retenues: Vec<AbsenceRetenue> = absences
            .iter()
            .filter(|a| a.membre_id == membre.id && retenues.contains(&a.id))
            .map(|a| AbsenceRetenue {
                date: a.date,
                demi_journee: a.nature == NatureAbsencePersonnel::DemiJournee,
            })
            .collect();
        let attributions_du_membre: Vec<_> = attributions
            .iter()
            .filter(|a| a.membre_id == membre.id)
            .cloned()
            .collect();
        let avances_du_membre: Vec<AvanceARembourser> = avances
            .iter()
            .filter(|a| a.membre_id == membre.id)
            .filter_map(|a| {
                let rembourse = rembourses
                    .iter()
                    .find(|(id, _)| *id == a.id)
                    .map_or(Montant::ZERO, |(_, m)| *m);
                let reste = a.montant.soustraire(rembourse).ok()?;
                (reste.francs() > 0).then(|| AvanceARembourser {
                    avance_id: Some(a.id),
                    libelle: format!(
                        "Avance du {:02}/{:02}/{}",
                        a.accordee_le.jour(),
                        a.accordee_le.mois(),
                        a.accordee_le.annee()
                    ),
                    mensualite: a.a_retenir(reste),
                    reste,
                })
            })
            .collect();
        let cumuls_anterieurs = cumuls_avant(unite, membre, mois, contexte).await?;
        let resultat = calculer(&DonneesBulletin {
            mois,
            prorata: reglages.prorata,
            mois_vacances: &reglages.mois_vacances,
            samedi_compte,
            fonctionnaire: membre.fonctionnaire,
            situation_famille: membre.situation_famille,
            enfants_a_charge: membre.enfants_a_charge,
            engagements: &siens,
            heures: &a_payer,
            elements: &elements,
            versions: &versions,
            attributions: &attributions_du_membre,
            saisies: &saisies_du_membre,
            absences: &absences_retenues,
            avances: &avances_du_membre,
            cumuls_anterieurs,
        })
        .map_err(regle)?;
        let paye = !resultat.lignes.is_empty();
        match existants.iter().find(|b| b.membre_id == membre.id) {
            Some(b) => {
                let nouveau = BulletinPaie {
                    resultat: if paye {
                        resultat.clone()
                    } else {
                        b.resultat.clone()
                    },
                    exclu: !paye,
                    ..b.clone()
                };
                if nouveau != *b {
                    paie::ecrire_bulletin(unite.connexion(), &nouveau, contexte).await?;
                }
            }
            None if paye => {
                let b = BulletinPaie {
                    id: nouvel_id(),
                    etablissement_id: contexte.etablissement_id,
                    paie_id: paie_mois.id,
                    membre_id: membre.id,
                    numero: None,
                    resultat: resultat.clone(),
                    exclu: false,
                };
                paie::inserer_bulletin(unite.connexion(), &b, contexte).await?;
            }
            None => {}
        }
        if paye {
            payes += 1;
            brut_total = brut_total
                .ajouter(resultat.brut)
                .map_err(|_| regle("paie.calcul_impossible"))?;
        }
    }
    let apres = PaieMois {
        calculee_le: contexte.maintenant,
        calculee_par: contexte.utilisateur_id,
        ..paie_mois.clone()
    };
    paie::ecrire_paie(unite.connexion(), &apres, contexte).await?;
    let id = apres.id.vers_chaine();
    unite.auditer(
        "paie_mois",
        &id,
        Operation::Modification,
        None,
        Some(serde_json::json!({ "mois": mois, "bulletins": payes, "brut": brut_total })),
        "paie.calculer",
    );
    unite.emettre(
        TypeEvenement::PaieCalculee,
        "paie_mois",
        &id,
        serde_json::json!({ "mois": mois, "bulletins": payes, "brut": brut_total }),
    );
    Ok(())
}

/// Cumuls de l'année civile avant ce mois : ceux du dernier bulletin validé de l'année.
async fn cumuls_avant(
    unite: &mut UniteDeTravail<Effet>,
    membre: &MembrePersonnel,
    mois: Mois,
    contexte: &Contexte,
) -> Resultat<Cumuls> {
    Ok(
        paie::bulletins_valides_du_membre(unite.connexion(), membre.id, contexte)
            .await?
            .into_iter()
            .rfind(|(m, _)| m.annee() == mois.annee() && *m < mois)
            .map(|(_, b)| b.resultat.cumuls)
            .unwrap_or_default(),
    )
}
