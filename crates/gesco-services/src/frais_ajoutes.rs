//! Ajouter un frais facultatif oublié à une facture validée (SPEC_REPRISE_AVANT_GESCO §3.7) : la
//! ligne s'ajoute (rien ne se réécrit), ses échéances se calculent comme à la validation pour
//! cette seule ligne (même plan, même date d'inscription, même règle d'arrivée en cours d'année),
//! puis la facture se recalcule : la reprise avant GESCO absorbe d'abord son excédent. Retirer
//! un frais ajouté par erreur contre-passe ses échéances que rien n'a encore payé.

use gesco_core::commun::{DateCivile, Id, Montant};
use gesco_core::core::{Operation, StatutAnnee, TypeEvenement};
use gesco_core::finance::facture::{ArriveeEnCours, Bornes, LigneCalculee, calculer};
use gesco_core::finance::{
    Echeance, Facture, LigneFacture, LigneTarifaire, StatutEcheance, StatutFacture, TypeFrais,
};
use gesco_core::scolarite::Inscription;
use gesco_db::depots::{annee_scolaire, factures, parametre, tarifs};
use gesco_db::unite::Durabilite;
use gesco_db::{Contexte, UniteDeTravail, nouvel_id};
use serde::{Deserialize, Serialize};

use crate::effets::Effet;
use crate::erreur::{ErreurService, Resultat};
use crate::facturation::{
    FactureAffichee, PARAMETRE_ARRIVEE, etat_echeances, niveau_de_la_facture, recalculer_dans,
};
use crate::inscriptions::{ServiceInscriptions, jour};
use crate::noyau::Noyau;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Inscription dont on regarde les frais ajoutables.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeFraisAjoutables {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
}

/// Échéance qu'un frais ajouté créerait.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct EcheanceProposee {
    /// Libellé.
    pub libelle: String,
    /// Date.
    pub date: DateCivile,
    /// Montant.
    pub montant: Montant,
}

/// Frais facultatif de la grille de la facture qui n'y est pas encore.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct FraisAjoutable {
    /// Ligne de la grille.
    pub ligne_tarifaire_id: Id<LigneTarifaire>,
    /// Nature.
    pub type_frais: TypeFrais,
    /// Libellé.
    pub libelle: String,
    /// Montant que la facture recevrait.
    pub montant: Montant,
    /// Échéances qu'il créerait.
    pub echeances: Vec<EcheanceProposee>,
}

/// Ajouter un frais facultatif à la facture d'une inscription.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeFraisAjoute {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Ligne facultative de la grille.
    pub ligne_tarifaire_id: Id<LigneTarifaire>,
}

/// Retirer un frais ajouté par erreur.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeFraisRetire {
    /// Inscription.
    pub inscription_id: Id<Inscription>,
    /// Ligne de la facture.
    pub ligne_facture_id: Id<LigneFacture>,
}

/// Service des frais ajoutés après la validation.
pub struct ServiceFraisAjoutes<'n> {
    noyau: &'n Noyau,
}

/// Facture vivante et modifiable d'une inscription : ni annulée, ni d'une année close.
async fn facture_modifiable(
    unite: &mut UniteDeTravail<Effet>,
    contexte: &Contexte,
    inscription_id: Id<Inscription>,
) -> Resultat<Facture> {
    let facture = factures::facture_vivante(unite.connexion(), inscription_id, contexte)
        .await?
        .filter(|f| f.statut != StatutFacture::Annulee)
        .ok_or(regle("facture.absente"))?;
    let annee = annee_scolaire::lire(unite.connexion(), facture.annee_id, contexte).await?;
    if !matches!(annee.statut, StatutAnnee::Preparation | StatutAnnee::Active) {
        return Err(regle("frais.annee_close"));
    }
    Ok(facture)
}

impl<'n> ServiceFraisAjoutes<'n> {
    /// Service sur le noyau ouvert.
    pub const fn nouveau(noyau: &'n Noyau) -> Self {
        Self { noyau }
    }

    /// Calcule, dans l'unité, les frais facultatifs de la grille de la facture pour son niveau
    /// qui n'y sont pas encore, avec les échéances qu'ils créeraient.
    async fn ajoutables_dans(
        &self,
        unite: &mut UniteDeTravail<Effet>,
        contexte: &Contexte,
        session: &Session,
        facture: &Facture,
    ) -> Resultat<Vec<LigneCalculee>> {
        let tarif = ServiceInscriptions::nouveau(self.noyau)
            .contexte_tarifaire(session, facture.inscription_id)
            .await?;
        let cnx = unite.connexion();
        let contenu = tarifs::contenu(cnx, facture.grille_id, contexte).await?;
        let lignes = factures::lignes(cnx, facture.id, contexte).await?;
        let tarif = niveau_de_la_facture(tarif, &lignes, &contenu.lignes);
        let plan = contenu
            .plans
            .iter()
            .find(|p| p.id == facture.plan_paiement_id)
            .cloned()
            .ok_or(regle("facture.plan_inconnu"))?;
        let annee = annee_scolaire::lire(cnx, facture.annee_id, contexte).await?;
        let arrivee = ArriveeEnCours::depuis_nom(
            &parametre::texte(
                cnx,
                PARAMETRE_ARRIVEE,
                ArriveeEnCours::DEFAUT.nom(),
                contexte,
            )
            .await?,
        )
        .unwrap_or(ArriveeEnCours::DEFAUT);
        let multiple = parametre::entier(cnx, "montant.multiple_arrondi", 5, contexte).await?;
        let bornes = Bornes {
            debut: jour(annee.debut)?,
            fin: jour(annee.fin)?,
            arrivee,
            multiple,
        };
        let candidats: Vec<Id<LigneTarifaire>> = contenu
            .lignes
            .iter()
            .filter(|l| !l.obligatoire)
            .filter(|l| l.niveau_id == tarif.niveau_id)
            .filter(|l| l.serie_id.is_none() || l.serie_id == tarif.serie_id)
            .filter(|l| !lignes.iter().any(|f| f.ligne_tarifaire_id == l.id))
            .map(|l| l.id)
            .collect();
        if candidats.is_empty() {
            return Ok(Vec::new());
        }
        let calcul =
            calculer(&contenu.lignes, &tarif, &candidats, &plan, &bornes).map_err(regle)?;
        Ok(calcul
            .into_iter()
            .filter(|c| candidats.contains(&c.ligne.id) && !c.echeances.is_empty())
            .collect())
    }

    /// Frais facultatifs ajoutables à la facture d'une inscription, et leurs échéances.
    pub async fn apercu(
        &self,
        session: &Session,
        inscription_id: Id<Inscription>,
    ) -> Resultat<Vec<FraisAjoutable>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        // Unité ouverte pour lire comme à l'écriture ; rien n'y est écrit.
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        let facture = facture_modifiable(&mut unite, &contexte, inscription_id).await?;
        let calcul = self
            .ajoutables_dans(&mut unite, &contexte, session, &facture)
            .await?;
        drop(unite);
        calcul
            .into_iter()
            .map(|c| {
                Ok(FraisAjoutable {
                    montant: c
                        .montant()
                        .map_err(|_| regle("facture.calcul_impossible"))?,
                    ligne_tarifaire_id: c.ligne.id,
                    type_frais: c.ligne.type_frais,
                    libelle: c.ligne.libelle.clone(),
                    echeances: c
                        .echeances
                        .iter()
                        .map(|e| EcheanceProposee {
                            libelle: e.libelle.clone(),
                            date: e.date,
                            montant: e.montant,
                        })
                        .collect(),
                })
            })
            .collect()
    }

    /// Ajoute un frais facultatif à la facture : ligne, échéances, facture recalculée (la reprise
    /// absorbe d'abord son excédent), audit, événement `FraisAjoute`.
    pub async fn ajouter(
        &self,
        session: &Session,
        demande: &DemandeFraisAjoute,
    ) -> Resultat<Facture> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let facture = facture_modifiable(&mut unite, &contexte, demande.inscription_id).await?;
        let calculee = self
            .ajoutables_dans(&mut unite, &contexte, session, &facture)
            .await?
            .into_iter()
            .find(|c| c.ligne.id == demande.ligne_tarifaire_id)
            .ok_or(regle("frais.non_ajoutable"))?;
        let ordre = factures::lignes(unite.connexion(), facture.id, &contexte)
            .await?
            .len();
        let mut rang =
            factures::nombre_d_echeances(unite.connexion(), facture.id, &contexte).await?;
        let ligne = LigneFacture {
            id: nouvel_id(),
            ligne_tarifaire_id: calculee.ligne.id,
            type_frais: calculee.ligne.type_frais,
            libelle: calculee.ligne.libelle.clone(),
            montant: calculee
                .montant()
                .map_err(|_| regle("facture.calcul_impossible"))?,
        };
        factures::inserer_ligne(unite.connexion(), facture.id, ordre, &ligne, &contexte).await?;
        let mut echeances = Vec::new();
        for e in &calculee.echeances {
            rang += 1;
            let echeance = Echeance {
                id: nouvel_id(),
                ligne_facture_id: ligne.id,
                libelle: e.libelle.clone(),
                date: e.date,
                montant: e.montant,
                statut: StatutEcheance::Due,
            };
            factures::inserer_echeance(unite.connexion(), facture.id, rang, &echeance, &contexte)
                .await?;
            echeances.push(echeance);
        }
        let apres = recalculer_dans(&mut unite, &contexte, &facture).await?;
        let id = apres.id.vers_chaine();
        unite.auditer(
            "facture",
            &id,
            Operation::Modification,
            Some(serde_json::to_value(&facture)?),
            Some(serde_json::json!({ "facture": apres, "ligne": ligne, "echeances": echeances })),
            "frais.ajouter",
        );
        unite.emettre(
            TypeEvenement::FraisAjoute,
            "facture",
            &id,
            serde_json::json!({
                "inscription_id": apres.inscription_id,
                "ligne_tarifaire_id": ligne.ligne_tarifaire_id,
                "montant": ligne.montant,
                "total": apres.total,
            }),
        );
        self.noyau.terminer(unite).await?;
        Ok(apres)
    }

    /// Retire un frais facultatif ajouté par erreur : ses échéances que ni un versement ni un
    /// avoir n'a payées sont contre-passées (la reprise se réimpute ailleurs) ; une échéance
    /// payée reste.
    pub async fn retirer(
        &self,
        session: &Session,
        demande: &DemandeFraisRetire,
    ) -> Resultat<Facture> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let facture = facture_modifiable(&mut unite, &contexte, demande.inscription_id).await?;
        let ligne = factures::lignes(unite.connexion(), facture.id, &contexte)
            .await?
            .into_iter()
            .find(|l| l.id == demande.ligne_facture_id)
            .ok_or(regle("frais.non_retirable"))?;
        let contenu = tarifs::contenu(unite.connexion(), facture.grille_id, &contexte).await?;
        if contenu
            .lignes
            .iter()
            .find(|l| l.id == ligne.ligne_tarifaire_id)
            .is_none_or(|l| l.obligatoire)
        {
            return Err(regle("frais.non_retirable"));
        }
        let mut annulees = Vec::new();
        for e in etat_echeances(unite.connexion(), facture.id, &contexte)
            .await?
            .into_iter()
            .filter(|e| e.echeance.ligne_facture_id == ligne.id)
            // Ni versement ni avoir : la reprise et le report du trop-payé se replacent ailleurs.
            .filter(|e| e.verse_parent.est_nul())
        {
            factures::annuler_echeance(unite.connexion(), e.echeance.id, &contexte).await?;
            annulees.push(e.echeance.id);
        }
        let apres = recalculer_dans(&mut unite, &contexte, &facture).await?;
        let id = apres.id.vers_chaine();
        unite.auditer(
            "facture",
            &id,
            Operation::Modification,
            Some(serde_json::to_value(&facture)?),
            Some(
                serde_json::json!({ "facture": apres, "ligne_retiree": ligne.id,
                "echeances_annulees": annulees }),
            ),
            "frais.retirer",
        );
        self.noyau.terminer(unite).await?;
        Ok(apres)
    }

    /// Facture affichée après un geste.
    pub async fn affichee(&self, session: &Session, facture: Facture) -> Resultat<FactureAffichee> {
        crate::facturation::ServiceFacturation::nouveau(self.noyau)
            .facture_affichee(session, facture)
            .await
    }
}
