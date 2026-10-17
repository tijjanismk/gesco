//! Autres recettes (SPEC_VERSION_1_1 §1, étape 1) : un don, une location de salle, reçus hors des
//! versements des parents, sur un compte de l'école ; en espèces, une entrée dans la session de
//! caisse ouverte de qui reçoit. Annulée avec un motif, une recette en espèces ressort du tiroir.

use gesco_core::commun::{DateCivile, Id, Montant};
use gesco_core::core::{Operation, TypeEvenement};
use gesco_core::finance::{ModePaiement, MouvementCaisse, NatureMouvement};
use gesco_core::rh::Mois;
use gesco_core::tresorerie::depense::verifier_paiement_depense;
use gesco_core::tresorerie::recette::{Recette, verifier_recette};
use gesco_core::tresorerie::{CategorieTresorerie, CompteTresorerie, NatureCompte, Sens};
use gesco_db::depots::{caisse, recettes, tresorerie};
use gesco_db::nouvel_id;
use gesco_db::unite::Durabilite;
use serde::{Deserialize, Serialize};

use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Code de la catégorie livrée des dons et autres recettes.
const AUTRES_RECETTES: &str = "autres_recettes";

/// Une catégorie reçoit les autres recettes : celle des dons et autres recettes, ou une catégorie
/// de recette de l'école ; pas celles des frais, qui viennent des versements des parents.
fn accepte(c: &CategorieTresorerie) -> bool {
    c.sens == Sens::Recette
        && c.active
        && c.code.as_deref().is_none_or(|code| code == AUTRES_RECETTES)
}

/// Recette affichée.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct RecetteAffichee {
    /// Recette.
    pub recette: Recette,
    /// Libellé de sa catégorie.
    pub categorie: String,
    /// Compte où l'argent est entré.
    pub compte: String,
}

/// Les autres recettes d'un mois.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct RecettesDuMois {
    /// Mois.
    pub mois: Mois,
    /// Recettes du mois, annulées comprises, la plus récente d'abord.
    pub recettes: Vec<RecetteAffichee>,
    /// Total reçu dans le mois, annulations exclues.
    pub total: Montant,
    /// Catégories qui reçoivent les autres recettes.
    pub categories: Vec<CategorieTresorerie>,
    /// Comptes ouverts.
    pub comptes: Vec<CompteTresorerie>,
    /// Qui consulte a une caisse ouverte.
    pub caisse_ouverte: bool,
}

/// Mois des recettes à lire.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeMoisRecettes {
    /// Mois.
    pub mois: Mois,
}

/// Recette reçue.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeRecette {
    /// Jour, `AAAA-MM-JJ` ; vide : aujourd'hui.
    pub date: String,
    /// Catégorie de recette.
    pub categorie_id: Id<CategorieTresorerie>,
    /// Ce qui est reçu.
    pub libelle: String,
    /// Qui paie.
    pub payeur: String,
    /// Montant.
    pub montant: Montant,
    /// Compte où l'argent entre.
    pub compte_id: Id<CompteTresorerie>,
    /// Mode.
    pub mode: ModePaiement,
    /// Référence, hors espèces.
    pub reference: String,
}

/// Annulation motivée d'une recette.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeAnnulationRecette {
    /// Recette.
    pub id: Id<Recette>,
    /// Motif.
    pub motif: String,
}

/// Service des autres recettes.
pub struct ServiceRecettes<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceRecettes<'n> {
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

    /// Les autres recettes d'un mois.
    pub async fn lire(&self, session: &Session, mois: Mois) -> Resultat<RecettesDuMois> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let (Some(debut), Some(fin)) = (mois.premier_jour(), mois.dernier_jour()) else {
            return Err(regle("recette.date_invalide"));
        };
        let categories = tresorerie::categories(&mut cnx, &contexte).await?;
        let comptes = tresorerie::comptes(&mut cnx, &contexte).await?;
        let mut du_mois: Vec<RecetteAffichee> = recettes::recettes(&mut cnx, &contexte)
            .await?
            .into_iter()
            .filter(|r| r.date >= debut && r.date <= fin)
            .map(|r| RecetteAffichee {
                categorie: categories
                    .iter()
                    .find(|c| c.id == r.categorie_id)
                    .map(|c| c.libelle.clone())
                    .unwrap_or_default(),
                compte: comptes
                    .iter()
                    .find(|c| c.id == r.compte_id)
                    .map(|c| c.libelle.clone())
                    .unwrap_or_default(),
                recette: r,
            })
            .collect();
        du_mois.reverse();
        let total = du_mois
            .iter()
            .filter(|r| !r.recette.annulee)
            .try_fold(Montant::ZERO, |s, r| s.ajouter(r.recette.montant))
            .map_err(|_| regle("recette.calcul_impossible"))?;
        let caisse_ouverte = caisse::ouverte_de(&mut cnx, session.utilisateur_id, &contexte)
            .await?
            .is_some();
        Ok(RecettesDuMois {
            mois,
            recettes: du_mois,
            total,
            categories: categories.into_iter().filter(accepte).collect(),
            comptes: comptes.into_iter().filter(|c| c.actif).collect(),
            caisse_ouverte,
        })
    }

    /// Enregistre une recette reçue ; en espèces, elle entre dans la session de caisse ouverte de
    /// qui la reçoit.
    pub async fn saisir(
        &self,
        session: &Session,
        demande: &DemandeRecette,
    ) -> Resultat<RecettesDuMois> {
        verifier_recette(&demande.libelle, &demande.payeur, demande.montant).map_err(regle)?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let date = if demande.date.trim().is_empty() {
            DateCivile::du_jour(contexte.maintenant).ok_or(regle("recette.date_invalide"))?
        } else {
            DateCivile::analyser(&demande.date).ok_or(regle("recette.date_invalide"))?
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let categorie = tresorerie::categories(unite.connexion(), &contexte)
            .await?
            .into_iter()
            .find(|c| c.id == demande.categorie_id && accepte(c))
            .ok_or(regle("recette.categorie_invalide"))?;
        let compte = tresorerie::comptes(unite.connexion(), &contexte)
            .await?
            .into_iter()
            .find(|c| c.id == demande.compte_id)
            .ok_or(regle("tresorerie.compte_inconnu"))?;
        verifier_paiement_depense(&compte, demande.mode, &demande.reference).map_err(regle)?;
        let session_id = if compte.nature == NatureCompte::Caisse {
            Some(
                caisse::ouverte_de(unite.connexion(), session.utilisateur_id, &contexte)
                    .await?
                    .ok_or(regle("recette.caisse_fermee"))?
                    .id,
            )
        } else {
            None
        };
        let recette = Recette {
            id: nouvel_id(),
            etablissement_id: session.etablissement_id,
            date,
            categorie_id: categorie.id,
            libelle: demande.libelle.trim().to_owned(),
            payeur: demande.payeur.trim().to_owned(),
            montant: demande.montant,
            compte_id: compte.id,
            mode: demande.mode,
            reference: demande.reference.trim().to_owned(),
            session_id,
            saisie_par: session.utilisateur_id,
            annulee: false,
            motif_annulation: String::new(),
        };
        recettes::inserer(unite.connexion(), &recette, &contexte).await?;
        if let Some(session_id) = session_id {
            let entree = MouvementCaisse {
                id: nouvel_id(),
                etablissement_id: recette.etablissement_id,
                session_id,
                montant: recette.montant,
                mode: ModePaiement::Especes,
                motif: format!("{} · {}", recette.libelle, recette.payeur),
                versement_id: None,
                le: contexte.maintenant,
                par: session.utilisateur_id,
                nature: NatureMouvement::EntreeRecette,
                paiement_id: None,
                depense_id: None,
                transfert_id: None,
                recette_id: Some(recette.id),
            };
            caisse::inserer_mouvement(unite.connexion(), &entree, &contexte).await?;
        }
        let id = recette.id.vers_chaine();
        unite.auditer(
            "recette_tresorerie",
            &id,
            Operation::Creation,
            None,
            Some(serde_json::to_value(&recette)?),
            "recettes.saisir",
        );
        unite.emettre(
            TypeEvenement::RecetteEnregistree,
            "recette_tresorerie",
            &id,
            serde_json::json!({ "montant": recette.montant, "categorie": categorie.libelle,
                "compte_id": compte.id, "mode": recette.mode }),
        );
        self.noyau.terminer(unite).await?;
        self.lire(session, Mois::de(date)).await
    }

    /// Annule une recette avec un motif ; reçue en espèces, elle ressort par une régularisation
    /// dans la session ouverte de qui annule.
    pub async fn annuler(
        &self,
        session: &Session,
        demande: &DemandeAnnulationRecette,
    ) -> Resultat<RecettesDuMois> {
        let motif = demande.motif.trim();
        if motif.is_empty() {
            return Err(regle("recette.motif_obligatoire"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let recette = recettes::recette(unite.connexion(), demande.id, &contexte).await?;
        if recette.annulee {
            return Err(regle("recette.deja_annulee"));
        }
        if recette.session_id.is_some() {
            let ouverte = caisse::ouverte_de(unite.connexion(), session.utilisateur_id, &contexte)
                .await?
                .ok_or(regle("recette.caisse_fermee_annulation"))?;
            let sortie = MouvementCaisse {
                id: nouvel_id(),
                etablissement_id: recette.etablissement_id,
                session_id: ouverte.id,
                montant: recette.montant,
                mode: ModePaiement::Especes,
                motif: format!("Recette annulée : {motif}"),
                versement_id: None,
                le: contexte.maintenant,
                par: session.utilisateur_id,
                nature: NatureMouvement::SortieRegularisation,
                paiement_id: None,
                depense_id: None,
                transfert_id: None,
                recette_id: Some(recette.id),
            };
            caisse::inserer_mouvement(unite.connexion(), &sortie, &contexte).await?;
        }
        recettes::annuler(unite.connexion(), recette.id, motif, &contexte).await?;
        let id = recette.id.vers_chaine();
        unite.auditer(
            "recette_tresorerie",
            &id,
            Operation::Modification,
            Some(serde_json::json!({ "annulee": false })),
            Some(serde_json::json!({ "annulee": true, "motif": motif })),
            "recettes.annuler",
        );
        unite.emettre(
            TypeEvenement::RecetteAnnulee,
            "recette_tresorerie",
            &id,
            serde_json::json!({ "montant": recette.montant, "motif": motif }),
        );
        self.noyau.terminer(unite).await?;
        self.lire(session, Mois::de(recette.date)).await
    }
}
