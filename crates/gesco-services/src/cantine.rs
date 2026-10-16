//! Cantine activable (SPEC_PHASE_8 §9, T-47) : une école qui a une cantine l'active ; les
//! abonnés du jour se lisent dans les factures (frais de cantine), avec ce qui reste échu ; les
//! repas se pointent si l'école les compte ; le bilan du mois réunit les recettes de cantine, les
//! vivres achetés (dépenses « Cantine ») et les repas. La liste du jour s'imprime.

use gesco_core::commun::{DateCivile, Horodatage, Id, Montant};
use gesco_core::core::{Operation, TypeEvenement};
use gesco_core::rh::Mois;
use gesco_core::scolarite::Inscription;
use gesco_db::depots::{annee_scolaire, budget, cantine, tresorerie};
use gesco_db::unite::Durabilite;
use gesco_db::{Contexte, nouvel_id};
use serde::{Deserialize, Serialize};

use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Code de la catégorie de dépense des vivres.
const CATEGORIE_VIVRES: &str = "cantine";

/// Élève abonné, au jour dit.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct AbonneCantine {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Prénoms et nom.
    pub eleve: String,
    /// Classe.
    pub classe: String,
    /// Cantine échue et pas encore payée.
    pub reste_echu: Montant,
    /// Repas pointé ce jour.
    pub repas_pris: bool,
}

/// Bilan de la cantine d'un mois.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct BilanCantine {
    /// Mois.
    pub mois: Mois,
    /// Recettes de cantine encaissées.
    pub recettes: Montant,
    /// Vivres payés.
    pub vivres: Montant,
    /// Repas pris.
    pub repas: u32,
}

/// La cantine au jour dit.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct Cantine {
    /// La cantine est activée.
    pub active: bool,
    /// Les repas se pointent.
    pub repas_comptes: bool,
    /// Jour.
    pub date: DateCivile,
    /// Abonnés, par classe et par nom.
    pub abonnes: Vec<AbonneCantine>,
    /// Repas pointés ce jour.
    pub repas_du_jour: u32,
    /// Bilan du mois du jour.
    pub bilan: BilanCantine,
}

/// Jour de la cantine à lire ; vide : aujourd'hui.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeCantine {
    /// Jour, `AAAA-MM-JJ`.
    pub date: String,
}

/// Repas pointé ou décoché.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeRepas {
    /// Jour, `AAAA-MM-JJ`.
    pub date: String,
    /// Élève.
    pub inscription_id: Id<Inscription>,
    /// Vrai pour un repas pris.
    pub pris: bool,
}

fn instant(jour: DateCivile) -> Horodatage {
    Horodatage::depuis_millis_epoque(jour.jours_depuis_epoque() * 86_400_000)
}

/// Service de la cantine.
pub struct ServiceCantine<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceCantine<'n> {
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

    fn jour(&self, texte: &str, contexte: &Contexte) -> Resultat<DateCivile> {
        if texte.trim().is_empty() {
            DateCivile::du_jour(contexte.maintenant).ok_or(regle("cantine.date_invalide"))
        } else {
            DateCivile::analyser(texte).ok_or(regle("cantine.date_invalide"))
        }
    }

    /// La cantine au jour dit : abonnés, repas pointés, bilan du mois.
    pub async fn lire(&self, session: &Session, demande: &DemandeCantine) -> Resultat<Cantine> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let jour = self.jour(&demande.date, &contexte)?;
        let mut cnx = self.connexion().await?;
        let reglages = crate::tresorerie::reglages(&mut cnx, &contexte).await?;
        let mois = Mois::de(jour);
        let (Some(premier), Some(dernier)) = (mois.premier_jour(), mois.dernier_jour()) else {
            return Err(regle("cantine.date_invalide"));
        };
        let lendemain = dernier.decaler(1).ok_or(regle("cantine.date_invalide"))?;
        let mut abonnes = Vec::new();
        let mut repas_du_jour = 0;
        if reglages.cantine_active
            && let Some(annee) = annee_scolaire::courante(&mut cnx, &contexte).await?
        {
            let pris = cantine::repas_du_jour(&mut cnx, jour, &contexte).await?;
            for a in cantine::abonnes(&mut cnx, annee.id, jour, &contexte).await? {
                let repas_pris = pris.contains(&a.inscription_id);
                repas_du_jour += u32::from(repas_pris);
                abonnes.push(AbonneCantine {
                    inscription_id: a.inscription_id,
                    eleve: a.eleve,
                    classe: a.classe,
                    reste_echu: a.reste_echu,
                    repas_pris,
                });
            }
        }
        let recettes =
            budget::recettes_entre(&mut cnx, instant(premier), instant(lendemain), &contexte)
                .await?
                .into_iter()
                .filter(|(t, _)| t.as_deref() == Some("cantine"))
                .map(|(_, m)| m)
                .next()
                .unwrap_or(Montant::ZERO);
        let vivres_id = tresorerie::categories(&mut cnx, &contexte)
            .await?
            .into_iter()
            .find(|c| c.code.as_deref() == Some(CATEGORIE_VIVRES))
            .map(|c| c.id);
        let vivres =
            budget::depenses_entre(&mut cnx, instant(premier), instant(lendemain), &contexte)
                .await?
                .into_iter()
                .find(|(c, _)| Some(*c) == vivres_id)
                .map_or(Montant::ZERO, |(_, m)| m);
        let repas = cantine::repas_entre(&mut cnx, premier, dernier, &contexte).await?;
        Ok(Cantine {
            active: reglages.cantine_active,
            repas_comptes: reglages.repas_comptes,
            date: jour,
            abonnes,
            repas_du_jour,
            bilan: BilanCantine {
                mois,
                recettes,
                vivres,
                repas: u32::try_from(repas).unwrap_or(u32::MAX),
            },
        })
    }

    /// Pointe ou décoche le repas d'un abonné, quand l'école compte les repas.
    pub async fn pointer(&self, session: &Session, demande: &DemandeRepas) -> Resultat<Cantine> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let jour = self.jour(&demande.date, &contexte)?;
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        let reglages = crate::tresorerie::reglages(unite.connexion(), &contexte).await?;
        if !reglages.cantine_active || !reglages.repas_comptes {
            return Err(regle("cantine.repas_non_comptes"));
        }
        let annee = annee_scolaire::courante(unite.connexion(), &contexte)
            .await?
            .ok_or(regle("cantine.sans_annee"))?;
        let abonne = cantine::abonnes(unite.connexion(), annee.id, jour, &contexte)
            .await?
            .into_iter()
            .find(|a| a.inscription_id == demande.inscription_id)
            .ok_or(regle("cantine.non_abonne"))?;
        let id: Id<Cantine> = nouvel_id();
        cantine::pointer(
            unite.connexion(),
            &id.vers_chaine(),
            abonne.inscription_id,
            jour,
            demande.pris,
            &contexte,
        )
        .await?;
        let cle = format!("{}:{jour}", abonne.inscription_id.vers_chaine());
        unite.auditer(
            "repas_cantine",
            &cle,
            Operation::Modification,
            None,
            Some(serde_json::json!({ "eleve": abonne.eleve, "date": jour, "pris": demande.pris })),
            "cantine.pointer",
        );
        unite.emettre(
            TypeEvenement::RepasPointe,
            "repas_cantine",
            &cle,
            serde_json::json!({ "date": jour, "pris": demande.pris }),
        );
        self.noyau.terminer(unite).await?;
        self.lire(
            session,
            &DemandeCantine {
                date: jour.to_string(),
            },
        )
        .await
    }

    /// Données de la liste du jour imprimée : les abonnés par classe.
    pub async fn donnees_liste(
        &self,
        session: &Session,
        demande: &DemandeCantine,
    ) -> Resultat<serde_json::Value> {
        let c = self.lire(session, demande).await?;
        if !c.active {
            return Err(regle("cantine.inactive"));
        }
        let mut classes: Vec<serde_json::Value> = Vec::new();
        let mut courante: Option<(String, Vec<serde_json::Value>)> = None;
        for a in &c.abonnes {
            if courante
                .as_ref()
                .is_none_or(|(classe, _)| *classe != a.classe)
            {
                if let Some((classe, eleves)) = courante.take() {
                    classes.push(serde_json::json!({ "classe": classe, "eleves": eleves }));
                }
                courante = Some((a.classe.clone(), Vec::new()));
            }
            if let Some((_, eleves)) = courante.as_mut() {
                eleves.push(serde_json::json!({
                    "nom": a.eleve,
                    "reste": if a.reste_echu.est_nul() { String::new() }
                        else { a.reste_echu.formater() },
                }));
            }
        }
        if let Some((classe, eleves)) = courante {
            classes.push(serde_json::json!({ "classe": classe, "eleves": eleves }));
        }
        Ok(serde_json::json!({
            "date": crate::documents::date_longue(instant(c.date)),
            "classes": classes,
            "total": c.abonnes.len(),
            "repas_comptes": c.repas_comptes,
        }))
    }
}
