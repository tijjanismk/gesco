//! Trésorerie (SPEC_PHASE_8 §2 et §5) : catégories de recettes et de dépenses que l'école renomme,
//! complète ou retire ; ses comptes (la caisse, livrée, les comptes en banque et mobile money
//! qu'elle ajoute) ; ses réglages (seuil d'accord des dépenses, cantine).

use gesco_core::commun::{DateCivile, Id, Montant};
use gesco_core::core::parametre::{Parametre, ValeurParametre, hors_bornes};
use gesco_core::core::{Etablissement, Operation, TypeEvenement};
use gesco_core::tresorerie::{
    CATEGORIES_LIVREES, CategorieTresorerie, CompteTresorerie, NatureCompte, Sens, verifier_compte,
    verifier_libelle,
};
use gesco_db::depots::{parametre, tresorerie};
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

/// Réglage : montant au-dessus duquel une dépense attend l'accord de la direction (T-61).
pub const PARAMETRE_SEUIL_ACCORD: &str = "depense.seuil_accord";
/// Réglage : l'école a une cantine (T-47).
pub const PARAMETRE_CANTINE: &str = "cantine.active";
/// Réglage : la cantine compte les repas pris.
pub const PARAMETRE_REPAS: &str = "cantine.repas_comptes";

/// Pose les catégories livrées et la caisse qu'une école n'a pas encore.
pub(crate) async fn poser_tresorerie(
    unite: &mut UniteDeTravail<Effet>,
    etablissement_id: Id<Etablissement>,
    origine: &str,
) -> Resultat<usize> {
    let contexte = unite.contexte().clone();
    let existantes = tresorerie::categories(unite.connexion(), &contexte).await?;
    let mut ajoutes = 0;
    for (ordre, (code, sens, libelle)) in (1_i64..).zip(CATEGORIES_LIVREES) {
        if existantes.iter().any(|c| c.code.as_deref() == Some(code)) {
            continue;
        }
        let c = CategorieTresorerie {
            id: nouvel_id(),
            etablissement_id,
            code: Some(code.to_owned()),
            sens,
            libelle: libelle.to_owned(),
            ordre,
            active: true,
        };
        tresorerie::inserer_categorie(unite.connexion(), &c, &contexte).await?;
        unite.auditer(
            "categorie_tresorerie",
            &c.id.vers_chaine(),
            Operation::Creation,
            None,
            Some(serde_json::to_value(&c)?),
            origine,
        );
        ajoutes += 1;
    }
    let comptes = tresorerie::comptes(unite.connexion(), &contexte).await?;
    if !comptes.iter().any(|c| c.nature == NatureCompte::Caisse) {
        let caisse = CompteTresorerie {
            id: nouvel_id(),
            etablissement_id,
            nature: NatureCompte::Caisse,
            libelle: "Caisse".to_owned(),
            organisme: String::new(),
            numero: String::new(),
            solde_ouverture: Montant::ZERO,
            ouvert_le: DateCivile::du_jour(contexte.maintenant)
                .ok_or(regle("tresorerie.date_invalide"))?,
            actif: true,
        };
        tresorerie::inserer_compte(unite.connexion(), &caisse, &contexte).await?;
        unite.auditer(
            "compte_tresorerie",
            &caisse.id.vers_chaine(),
            Operation::Creation,
            None,
            Some(serde_json::to_value(&caisse)?),
            origine,
        );
        ajoutes += 1;
    }
    // Les salaires et les avances payés avant la trésorerie deviennent des dépenses.
    ajoutes += crate::livres::rattacher_salaires(unite).await?;
    Ok(ajoutes)
}

/// Réglages de la trésorerie.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ReglagesTresorerie {
    /// Au-dessus de ce montant, une dépense attend l'accord de la direction ; 0 : jamais.
    pub seuil_accord: Montant,
    /// L'école a une cantine.
    pub cantine_active: bool,
    /// La cantine compte les repas pris.
    pub repas_comptes: bool,
}

/// Catégories, comptes et réglages.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct Tresorerie {
    /// Catégories, recettes puis dépenses.
    pub categories: Vec<CategorieTresorerie>,
    /// Comptes, la caisse d'abord.
    pub comptes: Vec<CompteTresorerie>,
    /// Réglages.
    pub reglages: ReglagesTresorerie,
}

/// Catégorie créée ou modifiée.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeCategorieTresorerie {
    /// Catégorie modifiée ; aucune pour en créer une.
    pub id: Option<Id<CategorieTresorerie>>,
    /// Recette ou dépense, pour une création.
    pub sens: Sens,
    /// Libellé.
    pub libelle: String,
    /// Active.
    pub active: bool,
}

/// Compte créé ou modifié.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeCompteTresorerie {
    /// Compte modifié ; aucun pour en créer un.
    pub id: Option<Id<CompteTresorerie>>,
    /// Banque ou mobile money, pour une création ; la caisse est livrée.
    pub nature: NatureCompte,
    /// Libellé.
    pub libelle: String,
    /// Banque ou opérateur.
    pub organisme: String,
    /// Numéro.
    pub numero: String,
    /// Solde au jour d'ouverture.
    pub solde_ouverture: Montant,
    /// Jour d'ouverture, `AAAA-MM-JJ`.
    pub ouvert_le: String,
    /// Actif.
    pub actif: bool,
}

/// Service de la trésorerie.
pub struct ServiceTresorerie<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceTresorerie<'n> {
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

    /// Catégories, comptes et réglages.
    pub async fn lire(&self, session: &Session) -> Resultat<Tresorerie> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        Ok(Tresorerie {
            categories: tresorerie::categories(&mut cnx, &contexte).await?,
            comptes: tresorerie::comptes(&mut cnx, &contexte).await?,
            reglages: reglages(&mut cnx, &contexte).await?,
        })
    }

    /// Crée une catégorie de l'école, ou renomme, retire ou remet une catégorie.
    pub async fn enregistrer_categorie(
        &self,
        session: &Session,
        demande: &DemandeCategorieTresorerie,
    ) -> Resultat<Tresorerie> {
        verifier_libelle(&demande.libelle).map_err(regle)?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        let existantes = tresorerie::categories(unite.connexion(), &contexte).await?;
        let libelle = demande.libelle.trim().to_owned();
        let (avant, apres) = match demande.id {
            Some(id) => {
                let avant = existantes
                    .iter()
                    .find(|c| c.id == id)
                    .cloned()
                    .ok_or(regle("tresorerie.categorie_inconnue"))?;
                let apres = CategorieTresorerie {
                    libelle,
                    active: demande.active,
                    ..avant.clone()
                };
                (Some(avant), apres)
            }
            None => {
                let c = CategorieTresorerie {
                    id: nouvel_id(),
                    etablissement_id: session.etablissement_id,
                    code: None,
                    sens: demande.sens,
                    libelle,
                    ordre: existantes.iter().map(|c| c.ordre).max().unwrap_or(0) + 1,
                    active: true,
                };
                (None, c)
            }
        };
        if existantes
            .iter()
            .any(|c| c.id != apres.id && c.sens == apres.sens && c.libelle == apres.libelle)
        {
            return Err(regle("tresorerie.categorie_existante"));
        }
        if avant.is_some() {
            tresorerie::ecrire_categorie(unite.connexion(), &apres, &contexte).await?;
        } else {
            tresorerie::inserer_categorie(unite.connexion(), &apres, &contexte).await?;
        }
        let id = apres.id.vers_chaine();
        unite.auditer(
            "categorie_tresorerie",
            &id,
            if avant.is_some() {
                Operation::Modification
            } else {
                Operation::Creation
            },
            avant.map(|a| serde_json::to_value(&a)).transpose()?,
            Some(serde_json::to_value(&apres)?),
            "tresorerie.categorie",
        );
        unite.emettre(
            TypeEvenement::CategorieTresorerieModifiee,
            "categorie_tresorerie",
            &id,
            serde_json::json!({ "libelle": apres.libelle, "sens": apres.sens, "active": apres.active }),
        );
        self.noyau.terminer(unite).await?;
        self.lire(session).await
    }

    /// Ajoute un compte en banque ou mobile money, ou modifie un compte.
    pub async fn enregistrer_compte(
        &self,
        session: &Session,
        demande: &DemandeCompteTresorerie,
    ) -> Resultat<Tresorerie> {
        let ouvert_le =
            DateCivile::analyser(&demande.ouvert_le).ok_or(regle("tresorerie.date_invalide"))?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        let existants = tresorerie::comptes(unite.connexion(), &contexte).await?;
        let avant = match demande.id {
            Some(id) => Some(
                existants
                    .iter()
                    .find(|c| c.id == id)
                    .cloned()
                    .ok_or(regle("tresorerie.compte_inconnu"))?,
            ),
            None => None,
        };
        if avant.is_none() && demande.nature == NatureCompte::Caisse {
            return Err(regle("tresorerie.caisse_unique"));
        }
        let apres = CompteTresorerie {
            id: avant.as_ref().map_or_else(nouvel_id, |a| a.id),
            etablissement_id: session.etablissement_id,
            nature: avant.as_ref().map_or(demande.nature, |a| a.nature),
            libelle: demande.libelle.trim().to_owned(),
            organisme: demande.organisme.trim().to_owned(),
            numero: demande.numero.trim().to_owned(),
            solde_ouverture: demande.solde_ouverture,
            ouvert_le,
            // La caisse ne se ferme pas : les encaissements y passent.
            actif: demande.actif
                || avant
                    .as_ref()
                    .is_some_and(|a| a.nature == NatureCompte::Caisse),
        };
        verifier_compte(&apres).map_err(regle)?;
        match &avant {
            Some(_) => tresorerie::ecrire_compte(unite.connexion(), &apres, &contexte).await?,
            None => tresorerie::inserer_compte(unite.connexion(), &apres, &contexte).await?,
        }
        let id = apres.id.vers_chaine();
        unite.auditer(
            "compte_tresorerie",
            &id,
            if avant.is_some() {
                Operation::Modification
            } else {
                Operation::Creation
            },
            avant.map(|a| serde_json::to_value(&a)).transpose()?,
            Some(serde_json::to_value(&apres)?),
            "tresorerie.compte",
        );
        unite.emettre(
            TypeEvenement::CompteTresorerieModifie,
            "compte_tresorerie",
            &id,
            serde_json::json!({ "libelle": apres.libelle, "nature": apres.nature, "actif": apres.actif }),
        );
        crate::livres::rattacher_salaires(&mut unite).await?;
        self.noyau.terminer(unite).await?;
        self.lire(session).await
    }

    /// Règle le seuil d'accord des dépenses et la cantine.
    pub async fn regler(
        &self,
        session: &Session,
        demande: &ReglagesTresorerie,
    ) -> Resultat<Tresorerie> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        for (cle, valeur) in [
            (
                PARAMETRE_SEUIL_ACCORD,
                ValeurParametre::Entier(demande.seuil_accord.francs()),
            ),
            (
                PARAMETRE_CANTINE,
                ValeurParametre::Booleen(demande.cantine_active),
            ),
            (
                PARAMETRE_REPAS,
                ValeurParametre::Booleen(demande.repas_comptes),
            ),
        ] {
            if hors_bornes(cle, &valeur) {
                return Err(regle("tresorerie.reglage_invalide"));
            }
            let avant = parametre::lire(unite.connexion(), cle, &contexte)
                .await?
                .ok_or(regle("parametre.inconnu"))?;
            if avant.valeur == valeur {
                continue;
            }
            let apres = Parametre {
                valeur,
                ..avant.clone()
            };
            parametre::definir(unite.connexion(), &apres, &contexte).await?;
            unite.auditer(
                "parametre",
                cle,
                Operation::Modification,
                Some(serde_json::to_value(&avant)?),
                Some(serde_json::to_value(&apres)?),
                "tresorerie.reglages",
            );
        }
        self.noyau.terminer(unite).await?;
        self.lire(session).await
    }
}

/// Lit les réglages, avec leurs valeurs livrées si l'école ne les a pas.
pub(crate) async fn reglages(
    cnx: &mut gesco_db::Connexion,
    contexte: &gesco_db::Contexte,
) -> Resultat<ReglagesTresorerie> {
    Ok(ReglagesTresorerie {
        seuil_accord: Montant::depuis_francs(
            parametre::entier(cnx, PARAMETRE_SEUIL_ACCORD, 50_000, contexte).await?,
        ),
        cantine_active: parametre::booleen_param(cnx, PARAMETRE_CANTINE, false, contexte).await?,
        repas_comptes: parametre::booleen_param(cnx, PARAMETRE_REPAS, false, contexte).await?,
    })
}
