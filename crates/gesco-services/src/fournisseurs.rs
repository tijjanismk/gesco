//! Fournisseurs et leurs factures (SPEC_PHASE_8 §4) : une facture se paie en une ou plusieurs
//! dépenses qui la portent, sous le seuil d'un geste, au-dessus après l'accord de la direction ;
//! ce que l'école doit à chaque fournisseur se lit. Une facture saisie par erreur s'annule avec
//! un motif tant qu'aucun paiement ne la touche.

use gesco_core::commun::{DateCivile, Id, Montant};
use gesco_core::core::Operation;
use gesco_core::core::TypeEvenement;
use gesco_core::tresorerie::fournisseur::{
    FactureFournisseur, Fournisseur, verifier_facture, verifier_fournisseur,
};
use gesco_core::tresorerie::{CategorieTresorerie, CompteTresorerie, Depense, Sens, StatutDepense};
use gesco_db::depots::{caisse, depenses, fournisseurs, tresorerie};
use gesco_db::unite::Durabilite;
use gesco_db::{Contexte, nouvel_id};
use serde::{Deserialize, Serialize};

use crate::depenses::{DroitsDepenses, ServiceDepenses};
use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Un paiement d'une facture.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ReglementFacture {
    /// Dépense qui paie.
    pub depense_id: Id<Depense>,
    /// Jour.
    pub date: DateCivile,
    /// Montant.
    pub montant: Montant,
    /// État de la dépense.
    pub statut: StatutDepense,
}

/// Facture affichée, avec ce qui en est payé.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct FactureFournisseurAffichee {
    /// Facture.
    pub facture: FactureFournisseur,
    /// Nom du fournisseur.
    pub fournisseur: String,
    /// Libellé de la catégorie.
    pub categorie: String,
    /// Payé.
    pub paye: Montant,
    /// Engagé, en attente d'accord ou de paiement.
    pub en_cours: Montant,
    /// Ce que l'école doit encore : le montant moins le payé ; zéro pour une facture annulée.
    pub reste: Montant,
    /// Ce qu'un nouveau paiement peut couvrir : le reste moins l'engagé.
    pub a_payer: Montant,
    /// L'échéance est passée et la facture n'est pas soldée.
    pub en_retard: bool,
    /// Paiements, refusés et annulés compris.
    pub reglements: Vec<ReglementFacture>,
}

/// Fournisseur affiché, avec ce que l'école lui doit.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct FournisseurAffiche {
    /// Fournisseur.
    pub fournisseur: Fournisseur,
    /// Factures non soldées.
    pub factures_ouvertes: u32,
    /// Ce que l'école lui doit.
    pub du: Montant,
    /// Une de ses factures a passé son échéance.
    pub en_retard: bool,
}

/// Les fournisseurs et leurs factures.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct Fournisseurs {
    /// Fournisseurs, par nom.
    pub fournisseurs: Vec<FournisseurAffiche>,
    /// Factures, la plus récente d'abord.
    pub factures: Vec<FactureFournisseurAffichee>,
    /// Ce que l'école doit à l'ensemble de ses fournisseurs.
    pub total_du: Montant,
    /// Catégories de dépense actives.
    pub categories: Vec<CategorieTresorerie>,
    /// Comptes ouverts.
    pub comptes: Vec<CompteTresorerie>,
    /// Seuil d'accord de l'école.
    pub seuil: Montant,
    /// Qui consulte a une caisse ouverte.
    pub caisse_ouverte: bool,
    /// Droits de qui consulte.
    pub droits: DroitsDepenses,
}

/// Fournisseur nouveau (`id` vide) ou modifié.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeFournisseur {
    /// Fournisseur modifié ; vide pour un nouveau.
    pub id: Option<Id<Fournisseur>>,
    /// Nom.
    pub nom: String,
    /// Téléphone.
    pub telephone: String,
    /// Ce qu'il fournit.
    pub fourniture: String,
    /// Faux pour le retirer.
    pub actif: bool,
}

/// Facture reçue d'un fournisseur.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeFacture {
    /// Fournisseur.
    pub fournisseur_id: Id<Fournisseur>,
    /// Numéro porté par le fournisseur.
    pub numero: String,
    /// Jour, `AAAA-MM-JJ`.
    pub date: String,
    /// Échéance, `AAAA-MM-JJ` ; vide : aucune.
    pub echeance: String,
    /// Ce qui est facturé.
    pub libelle: String,
    /// Catégorie de dépense des paiements.
    pub categorie_id: Id<CategorieTresorerie>,
    /// Montant.
    pub montant: Montant,
}

/// Annulation motivée d'une facture.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeAnnulationFacture {
    /// Facture.
    pub id: Id<FactureFournisseur>,
    /// Motif.
    pub motif: String,
}

/// Une dépense engage une facture tant qu'elle n'est ni refusée ni annulée.
fn engage(statut: StatutDepense) -> bool {
    matches!(
        statut,
        StatutDepense::AApprouver | StatutDepense::Approuvee | StatutDepense::Payee
    )
}

/// Ce qu'une facture peut encore recevoir : son montant moins ce qui l'engage déjà.
pub(crate) fn disponible(
    facture: &FactureFournisseur,
    reglements: &[Depense],
) -> Resultat<Montant> {
    let engage = reglements
        .iter()
        .filter(|d| d.facture_id == Some(facture.id) && engage(d.statut))
        .try_fold(Montant::ZERO, |s, d| s.ajouter(d.montant))
        .map_err(|_| regle("depense.calcul_impossible"))?;
    facture
        .montant
        .soustraire(engage)
        .map_err(|_| regle("depense.calcul_impossible"))
}

fn somme(reglements: &[&Depense], statuts: &[StatutDepense]) -> Resultat<Montant> {
    reglements
        .iter()
        .filter(|d| statuts.contains(&d.statut))
        .try_fold(Montant::ZERO, |s, d| s.ajouter(d.montant))
        .map_err(|_| regle("depense.calcul_impossible"))
}

/// Service des fournisseurs.
pub struct ServiceFournisseurs<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceFournisseurs<'n> {
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

    /// Les fournisseurs, leurs factures et ce que l'école leur doit ; il faut un droit sur les
    /// dépenses ou la lecture de la trésorerie.
    pub async fn lire(&self, session: &Session) -> Resultat<Fournisseurs> {
        let droits = ServiceDepenses::nouveau(self.noyau)
            .droits_lecture(session)
            .await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let aujourd_hui =
            DateCivile::du_jour(contexte.maintenant).ok_or(regle("depense.date_invalide"))?;
        let categories = tresorerie::categories(&mut cnx, &contexte).await?;
        let comptes = tresorerie::comptes(&mut cnx, &contexte).await?;
        let liste = fournisseurs::fournisseurs(&mut cnx, &contexte).await?;
        let reglements = depenses::de_factures(&mut cnx, &contexte).await?;
        let mut factures = Vec::new();
        for f in fournisseurs::factures(&mut cnx, &contexte).await? {
            let siens: Vec<&Depense> = reglements
                .iter()
                .filter(|d| d.facture_id == Some(f.id))
                .collect();
            let paye = somme(&siens, &[StatutDepense::Payee])?;
            let en_cours = somme(
                &siens,
                &[StatutDepense::AApprouver, StatutDepense::Approuvee],
            )?;
            let (reste, a_payer) = if f.annulee {
                (Montant::ZERO, Montant::ZERO)
            } else {
                let reste = f
                    .montant
                    .soustraire(paye)
                    .map_err(|_| regle("depense.calcul_impossible"))?;
                let a_payer = reste
                    .soustraire(en_cours)
                    .map_err(|_| regle("depense.calcul_impossible"))?;
                (reste, a_payer)
            };
            factures.push(FactureFournisseurAffichee {
                fournisseur: liste
                    .iter()
                    .find(|x| x.id == f.fournisseur_id)
                    .map(|x| x.nom.clone())
                    .unwrap_or_default(),
                categorie: categories
                    .iter()
                    .find(|c| c.id == f.categorie_id)
                    .map(|c| c.libelle.clone())
                    .unwrap_or_default(),
                paye,
                en_cours,
                reste,
                a_payer,
                en_retard: reste.francs() > 0 && f.echeance.is_some_and(|e| e < aujourd_hui),
                reglements: siens
                    .iter()
                    .map(|d| ReglementFacture {
                        depense_id: d.id,
                        date: d.date,
                        montant: d.montant,
                        statut: d.statut,
                    })
                    .collect(),
                facture: f,
            });
        }
        factures.reverse();
        let mut affiches = Vec::with_capacity(liste.len());
        let mut total_du = Montant::ZERO;
        for f in liste {
            let ouvertes: Vec<&FactureFournisseurAffichee> = factures
                .iter()
                .filter(|x| x.facture.fournisseur_id == f.id && x.reste.francs() > 0)
                .collect();
            let du = ouvertes
                .iter()
                .try_fold(Montant::ZERO, |s, x| s.ajouter(x.reste))
                .map_err(|_| regle("depense.calcul_impossible"))?;
            total_du = total_du
                .ajouter(du)
                .map_err(|_| regle("depense.calcul_impossible"))?;
            affiches.push(FournisseurAffiche {
                factures_ouvertes: u32::try_from(ouvertes.len()).unwrap_or(u32::MAX),
                en_retard: ouvertes.iter().any(|x| x.en_retard),
                du,
                fournisseur: f,
            });
        }
        let caisse_ouverte = caisse::ouverte_de(&mut cnx, session.utilisateur_id, &contexte)
            .await?
            .is_some();
        let seuil = crate::tresorerie::reglages(&mut cnx, &contexte)
            .await?
            .seuil_accord;
        Ok(Fournisseurs {
            fournisseurs: affiches,
            factures,
            total_du,
            categories: categories
                .into_iter()
                .filter(|c| c.sens == Sens::Depense && c.active)
                .collect(),
            comptes: comptes.into_iter().filter(|c| c.actif).collect(),
            seuil,
            caisse_ouverte,
            droits,
        })
    }

    /// Enregistre un fournisseur nouveau ou modifié.
    pub async fn enregistrer_fournisseur(
        &self,
        session: &Session,
        demande: &DemandeFournisseur,
    ) -> Resultat<Fournisseurs> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        let existants = fournisseurs::fournisseurs(unite.connexion(), &contexte).await?;
        let avant = match demande.id {
            Some(id) => Some(
                existants
                    .iter()
                    .find(|f| f.id == id)
                    .cloned()
                    .ok_or(regle("fournisseur.inconnu"))?,
            ),
            None => None,
        };
        let apres = Fournisseur {
            id: demande.id.unwrap_or_else(nouvel_id),
            etablissement_id: session.etablissement_id,
            nom: demande.nom.trim().to_owned(),
            telephone: demande.telephone.trim().to_owned(),
            fourniture: demande.fourniture.trim().to_owned(),
            actif: demande.actif,
        };
        verifier_fournisseur(&apres).map_err(regle)?;
        if existants.iter().any(|f| {
            f.id != apres.id && f.actif && f.nom.to_lowercase() == apres.nom.to_lowercase()
        }) {
            return Err(regle("fournisseur.nom_en_double"));
        }
        if avant.is_some() {
            fournisseurs::ecrire_fournisseur(unite.connexion(), &apres, &contexte).await?;
        } else {
            fournisseurs::inserer_fournisseur(unite.connexion(), &apres, &contexte).await?;
        }
        let id = apres.id.vers_chaine();
        unite.auditer(
            "fournisseur",
            &id,
            if avant.is_some() {
                Operation::Modification
            } else {
                Operation::Creation
            },
            avant.map(|a| serde_json::to_value(&a)).transpose()?,
            Some(serde_json::to_value(&apres)?),
            "fournisseurs.enregistrer",
        );
        unite.emettre(
            TypeEvenement::FournisseurModifie,
            "fournisseur",
            &id,
            serde_json::json!({ "nom": apres.nom, "actif": apres.actif }),
        );
        self.noyau.terminer(unite).await?;
        self.lire(session).await
    }

    /// Enregistre une facture reçue d'un fournisseur actif.
    pub async fn enregistrer_facture(
        &self,
        session: &Session,
        demande: &DemandeFacture,
    ) -> Resultat<Fournisseurs> {
        let date = DateCivile::analyser(&demande.date).ok_or(regle("depense.date_invalide"))?;
        let echeance = if demande.echeance.trim().is_empty() {
            None
        } else {
            Some(DateCivile::analyser(&demande.echeance).ok_or(regle("depense.date_invalide"))?)
        };
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        let fournisseur = fournisseurs::fournisseurs(unite.connexion(), &contexte)
            .await?
            .into_iter()
            .find(|f| f.id == demande.fournisseur_id && f.actif)
            .ok_or(regle("fournisseur.inconnu"))?;
        tresorerie::categories(unite.connexion(), &contexte)
            .await?
            .into_iter()
            .find(|c| c.id == demande.categorie_id && c.sens == Sens::Depense && c.active)
            .ok_or(regle("depense.categorie_invalide"))?;
        let facture = FactureFournisseur {
            id: nouvel_id(),
            etablissement_id: session.etablissement_id,
            fournisseur_id: fournisseur.id,
            numero: demande.numero.trim().to_owned(),
            date,
            echeance,
            libelle: demande.libelle.trim().to_owned(),
            categorie_id: demande.categorie_id,
            montant: demande.montant,
            annulee: false,
            motif_annulation: String::new(),
        };
        verifier_facture(&facture).map_err(regle)?;
        if !facture.numero.is_empty()
            && fournisseurs::factures(unite.connexion(), &contexte)
                .await?
                .iter()
                .any(|f| {
                    f.fournisseur_id == fournisseur.id && !f.annulee && f.numero == facture.numero
                })
        {
            return Err(regle("fournisseur.facture_en_double"));
        }
        fournisseurs::inserer_facture(unite.connexion(), &facture, &contexte).await?;
        let id = facture.id.vers_chaine();
        unite.auditer(
            "facture_fournisseur",
            &id,
            Operation::Creation,
            None,
            Some(serde_json::to_value(&facture)?),
            "fournisseurs.facture",
        );
        unite.emettre(
            TypeEvenement::FactureFournisseurEnregistree,
            "facture_fournisseur",
            &id,
            serde_json::json!({ "fournisseur": fournisseur.nom, "montant": facture.montant }),
        );
        self.noyau.terminer(unite).await?;
        self.lire(session).await
    }

    /// Annule une facture saisie par erreur, avec un motif, tant qu'aucun paiement ne l'engage.
    pub async fn annuler_facture(
        &self,
        session: &Session,
        demande: &DemandeAnnulationFacture,
    ) -> Resultat<Fournisseurs> {
        let motif = demande.motif.trim();
        if motif.is_empty() {
            return Err(regle("depense.motif_obligatoire"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        let facture = facture(unite.connexion(), demande.id, &contexte).await?;
        if facture.annulee {
            return Err(regle("fournisseur.facture_annulee"));
        }
        if depenses::de_factures(unite.connexion(), &contexte)
            .await?
            .iter()
            .any(|d| d.facture_id == Some(facture.id) && engage(d.statut))
        {
            return Err(regle("fournisseur.facture_reglee"));
        }
        fournisseurs::annuler_facture(unite.connexion(), facture.id, motif, &contexte).await?;
        let id = facture.id.vers_chaine();
        unite.auditer(
            "facture_fournisseur",
            &id,
            Operation::Modification,
            Some(serde_json::json!({ "annulee": false })),
            Some(serde_json::json!({ "annulee": true, "motif": motif })),
            "fournisseurs.annuler_facture",
        );
        unite.emettre(
            TypeEvenement::FactureFournisseurAnnulee,
            "facture_fournisseur",
            &id,
            serde_json::json!({ "montant": facture.montant, "motif": motif }),
        );
        self.noyau.terminer(unite).await?;
        self.lire(session).await
    }
}

/// Une facture de l'école.
pub(crate) async fn facture(
    cnx: &mut gesco_db::Connexion,
    id: Id<FactureFournisseur>,
    contexte: &Contexte,
) -> Resultat<FactureFournisseur> {
    fournisseurs::factures(cnx, contexte)
        .await?
        .into_iter()
        .find(|f| f.id == id)
        .ok_or(regle("fournisseur.facture_inconnue"))
}
