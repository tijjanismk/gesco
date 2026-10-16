//! Remises (SPEC_PHASE_3 §5) : le secrétariat propose, la direction accorde ou refuse, avec un
//! motif. Accordée, la remise se répartit sur les dernières échéances du frais visé dans la
//! même transaction (R-17) ; annulée, elle se contre-passe.

use gesco_core::commun::{Id, Montant};
use gesco_core::core::{MachineAEtats, Operation, TypeEvenement};
use gesco_core::finance::remise::{
    montant_de_la_remise, repartir_depuis_la_fin, verifier_proposition,
};
use gesco_core::finance::{
    Echeance, Facture, Remise, StatutRemise, TypeFrais, TypeRemise, ValeurRemise,
};
use gesco_core::scolarite::{Inscription, StatutInscription};
use gesco_db::depots::{factures, parametre, remises};
use gesco_db::unite::Durabilite;
use gesco_db::{Contexte, UniteDeTravail, nouvel_id};
use serde::{Deserialize, Serialize};

use crate::effets::Effet;
use crate::erreur::{ErreurService, Resultat};
use crate::inscriptions::{ServiceInscriptions, jour};
use crate::noyau::Noyau;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Remise proposée.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeRemise {
    /// Inscription, brouillon ou validée.
    pub inscription_id: Id<Inscription>,
    /// Nature.
    pub type_remise: TypeRemise,
    /// Taux ou montant.
    pub valeur: ValeurRemise,
    /// Frais visé.
    pub type_frais: TypeFrais,
    /// Motif, obligatoire.
    pub motif: String,
}

/// Remise désignée.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeIdRemise {
    /// Remise.
    pub id: Id<Remise>,
}

/// Décision motivée sur une remise : refus ou annulation.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeDecisionRemise {
    /// Remise.
    pub id: Id<Remise>,
    /// Motif, obligatoire.
    pub motif: String,
}

/// Remise en attente, avec l'élève et la classe.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct RemiseEnAttente {
    /// Remise.
    pub remise: Remise,
    /// Élève.
    pub eleve: String,
    /// Classe.
    pub classe: String,
}

/// Répartit une remise accordée sur la facture, dans la transaction ouverte : son montant est
/// calculé sur le frais visé, puis porté par les dernières échéances non soldées. Rend le
/// montant appliqué.
pub(crate) async fn appliquer_dans(
    unite: &mut UniteDeTravail<Effet>,
    contexte: &Contexte,
    remise: &Remise,
    facture: &Facture,
) -> Resultat<Montant> {
    let cnx = unite.connexion();
    let ligne = factures::lignes(cnx, facture.id, contexte)
        .await?
        .into_iter()
        .find(|l| l.type_frais == remise.type_frais)
        .ok_or(regle("remise.frais_absent"))?;
    let multiple = parametre::entier(cnx, "montant.multiple_arrondi", 5, contexte).await?;
    let montant = montant_de_la_remise(&remise.valeur, ligne.montant, multiple)
        .map_err(|_| regle("remise.calcul_impossible"))?;
    // Reste de chaque échéance du frais : dû, moins les autres remises, moins le payé.
    let restes: Vec<(gesco_core::commun::Id<Echeance>, Montant)> =
        crate::facturation::etat_echeances(cnx, facture.id, contexte)
            .await?
            .into_iter()
            .filter(|e| e.echeance.ligne_facture_id == ligne.id)
            .map(|e| (e.echeance.id, e.reste))
            .collect();
    for (echeance_id, part) in repartir_depuis_la_fin(&restes, montant).map_err(regle)? {
        remises::inserer_part(unite.connexion(), remise.id, echeance_id, part, contexte).await?;
    }
    Ok(montant)
}

/// Applique, à la création d'une facture, les remises déjà accordées sur l'inscription
/// (SPEC_PHASE_3 §5.2) ; après un changement de plan, les répartit à nouveau.
pub(crate) async fn reappliquer_dans(
    unite: &mut UniteDeTravail<Effet>,
    contexte: &Contexte,
    facture: &Facture,
) -> Resultat<()> {
    let accordees: Vec<Remise> =
        remises::de_l_inscription(unite.connexion(), facture.inscription_id, contexte)
            .await?
            .into_iter()
            .filter(|r| r.statut == StatutRemise::Accordee)
            .collect();
    for remise in &accordees {
        remises::annuler_parts(unite.connexion(), remise.id, None, contexte).await?;
    }
    for remise in accordees {
        let montant_applique = appliquer_dans(unite, contexte, &remise, facture).await?;
        remises::ecrire(
            unite.connexion(),
            &Remise {
                montant_applique,
                ..remise
            },
            contexte,
        )
        .await?;
    }
    Ok(())
}

/// Service des remises.
pub struct ServiceRemises<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceRemises<'n> {
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

    async fn lire(&self, session: &Session, id: Id<Remise>) -> Resultat<Remise> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        Ok(remises::lire(&mut cnx, id, &contexte).await?)
    }

    /// Remises d'une inscription, toutes.
    pub async fn de_l_inscription(
        &self,
        session: &Session,
        inscription_id: Id<Inscription>,
    ) -> Resultat<Vec<Remise>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        Ok(remises::de_l_inscription(&mut cnx, inscription_id, &contexte).await?)
    }

    /// Propose une remise sur une inscription en brouillon ou validée.
    pub async fn proposer(&self, session: &Session, demande: &DemandeRemise) -> Resultat<Remise> {
        verifier_proposition(&demande.valeur, &demande.motif).map_err(regle)?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let inscription = ServiceInscriptions::nouveau(self.noyau)
            .lire_publique(session, demande.inscription_id)
            .await?;
        if !matches!(
            inscription.statut,
            StatutInscription::Brouillon | StatutInscription::Validee
        ) {
            return Err(regle("remise.inscription_close"));
        }
        let remise = Remise {
            id: nouvel_id(),
            etablissement_id: session.etablissement_id,
            inscription_id: inscription.id,
            type_remise: demande.type_remise,
            valeur: demande.valeur,
            type_frais: demande.type_frais,
            motif: demande.motif.trim().to_owned(),
            statut: StatutRemise::Proposee,
            proposee_par: session.utilisateur_id,
            proposee_le: jour(contexte.maintenant)?,
            decidee_par: None,
            motif_decision: String::new(),
            montant_applique: Montant::ZERO,
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        remises::inserer(unite.connexion(), &remise, &contexte).await?;
        let id = remise.id.vers_chaine();
        unite.auditer(
            "reduction",
            &id,
            Operation::Creation,
            None,
            Some(serde_json::to_value(&remise)?),
            "remises.proposer",
        );
        unite.emettre(
            TypeEvenement::RemiseProposee,
            "reduction",
            &id,
            serde_json::json!({ "inscription_id": remise.inscription_id, "type": remise.type_remise }),
        );
        self.noyau.terminer(unite).await?;
        Ok(remise)
    }

    /// Accorde une remise : elle se répartit sur l'échéancier dans la même transaction, ou à la
    /// création de la facture si l'inscription n'est pas encore validée.
    pub async fn accorder(&self, session: &Session, id: Id<Remise>) -> Resultat<Remise> {
        let avant = self.lire(session, id).await?;
        let statut = avant.statut.transiter(StatutRemise::Accordee)?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let facture =
            factures::facture_vivante(unite.connexion(), avant.inscription_id, &contexte).await?;
        let mut apres = Remise {
            statut,
            decidee_par: Some(session.utilisateur_id),
            ..avant.clone()
        };
        if let Some(facture) = &facture {
            apres.montant_applique = appliquer_dans(&mut unite, &contexte, &apres, facture).await?;
        }
        remises::ecrire(unite.connexion(), &apres, &contexte).await?;
        let id_texte = apres.id.vers_chaine();
        unite.auditer(
            "reduction",
            &id_texte,
            Operation::Transition,
            Some(serde_json::to_value(&avant)?),
            Some(serde_json::to_value(&apres)?),
            "remises.accorder",
        );
        unite.emettre(
            TypeEvenement::RemiseAccordee,
            "reduction",
            &id_texte,
            serde_json::json!({
                "inscription_id": apres.inscription_id,
                "montant_applique": apres.montant_applique,
            }),
        );
        self.noyau.terminer(unite).await?;
        Ok(apres)
    }

    async fn decider(
        &self,
        session: &Session,
        demande: &DemandeDecisionRemise,
        vers: StatutRemise,
    ) -> Resultat<Remise> {
        let motif = demande.motif.trim();
        if motif.is_empty() {
            return Err(regle("remise.motif_obligatoire"));
        }
        let avant = self.lire(session, demande.id).await?;
        let statut = avant.statut.transiter(vers)?;
        let apres = Remise {
            statut,
            decidee_par: Some(session.utilisateur_id),
            motif_decision: motif.to_owned(),
            ..avant.clone()
        };
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        if vers == StatutRemise::Annulee {
            // Contre-passation (§5.2). Un versement imputé au-delà du nouveau dû rendra
            // l'annulation impossible (étape 6).
            remises::annuler_parts(unite.connexion(), apres.id, None, &contexte).await?;
        }
        remises::ecrire(unite.connexion(), &apres, &contexte).await?;
        let id_texte = apres.id.vers_chaine();
        let (operation, evenement, origine) = if vers == StatutRemise::Annulee {
            (
                Operation::Annulation,
                TypeEvenement::RemiseAnnulee,
                "remises.annuler",
            )
        } else {
            (
                Operation::Transition,
                TypeEvenement::RemiseRefusee,
                "remises.refuser",
            )
        };
        unite.auditer(
            "reduction",
            &id_texte,
            operation,
            Some(serde_json::to_value(&avant)?),
            Some(serde_json::to_value(&apres)?),
            origine,
        );
        unite.emettre(
            evenement,
            "reduction",
            &id_texte,
            serde_json::json!({ "inscription_id": apres.inscription_id, "motif": motif }),
        );
        self.noyau.terminer(unite).await?;
        Ok(apres)
    }

    /// Refuse une remise proposée, avec un motif.
    pub async fn refuser(
        &self,
        session: &Session,
        demande: &DemandeDecisionRemise,
    ) -> Resultat<Remise> {
        self.decider(session, demande, StatutRemise::Refusee).await
    }

    /// Annule une remise accordée, avec un motif : ses parts sont contre-passées.
    pub async fn annuler(
        &self,
        session: &Session,
        demande: &DemandeDecisionRemise,
    ) -> Resultat<Remise> {
        self.decider(session, demande, StatutRemise::Annulee).await
    }

    /// Remises proposées, en attente de la direction, avec l'élève et la classe.
    pub async fn en_attente(&self, session: &Session) -> Resultat<Vec<RemiseEnAttente>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let proposees = {
            let mut cnx = self.connexion().await?;
            remises::proposees(&mut cnx, &contexte).await?
        };
        let service = ServiceInscriptions::nouveau(self.noyau);
        let mut lignes = Vec::with_capacity(proposees.len());
        for remise in proposees {
            let inscription = service
                .lire_publique(session, remise.inscription_id)
                .await?;
            let (eleve, classe) = service
                .lister(session, inscription.annee_id)
                .await?
                .into_iter()
                .find(|l| l.inscription.id == inscription.id)
                .map(|l| (l.eleve, l.classe))
                .unwrap_or_default();
            lignes.push(RemiseEnAttente {
                remise,
                eleve,
                classe,
            });
        }
        Ok(lignes)
    }
}
