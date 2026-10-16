//! Unité de travail : une transaction, les mutations, leurs lignes d'audit et leurs événements,
//! validés ensemble ou pas du tout (R-17). Les effets de bord sont collectés et rendus à
//! l'appelant après validation, jamais exécutés pendant (R-18).

use gesco_core::commun::Horodatage;
use gesco_core::core::{EmpreinteAudit, Evenement, LigneAudit, Operation, TypeEvenement};
use sqlx::{Sqlite, Transaction};

use crate::Connexion;
use crate::base::Base;
use crate::contexte::Contexte;
use crate::depots::{audit, outbox};
use crate::erreur::Resultat;
use crate::id::nouvel_id;

/// Niveau de durabilité d'une transaction (spec §8.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Durabilite {
    /// `synchronous = FULL` : financier, audit, sécurité.
    Critique,
    /// `synchronous = NORMAL` : le reste.
    Ordinaire,
}

/// Une transaction métier en cours, avec ses écritures annexes en attente.
pub struct UniteDeTravail<E> {
    tx: Transaction<'static, Sqlite>,
    contexte: Contexte,
    audits: Vec<LigneAudit>,
    evenements: Vec<Evenement>,
    effets: Vec<E>,
}

impl<E> UniteDeTravail<E> {
    /// Ouvre une transaction `BEGIN IMMEDIATE` : le verrou d'écriture est pris d'emblée, ce qui
    /// sérialise les écrivains et rend le chaînage d'audit sûr.
    pub async fn commencer(
        base: &Base,
        contexte: Contexte,
        durabilite: Durabilite,
    ) -> Resultat<Self> {
        let pool = match durabilite {
            Durabilite::Critique => base.pool_critique(),
            Durabilite::Ordinaire => base.pool(),
        };
        let tx = pool.begin_with("BEGIN IMMEDIATE").await?;
        Ok(Self {
            tx,
            contexte,
            audits: Vec::new(),
            evenements: Vec::new(),
            effets: Vec::new(),
        })
    }

    /// Connexion de la transaction, à passer aux dépôts.
    pub fn connexion(&mut self) -> &mut Connexion {
        &mut self.tx
    }

    /// Contexte de l'opération.
    pub fn contexte(&self) -> &Contexte {
        &self.contexte
    }

    /// Enregistre une ligne d'audit à écrire à la validation. Les valeurs ne doivent contenir
    /// aucun secret (R-25) : c'est à l'appelant de ne sérialiser que des types sans secret.
    pub fn auditer(
        &mut self,
        entite: &str,
        entite_id: &str,
        operation: Operation,
        avant: Option<serde_json::Value>,
        apres: Option<serde_json::Value>,
        contexte_ecran: &str,
    ) {
        let c = &self.contexte;
        self.audits.push(LigneAudit {
            id: nouvel_id(),
            horodatage: c.maintenant,
            utilisateur_id: c.utilisateur_id,
            role_actif: c.role_actif.clone(),
            etablissement_id: c.etablissement_id,
            entite: entite.to_owned(),
            entite_id: entite_id.to_owned(),
            operation,
            avant,
            apres,
            contexte: contexte_ecran.to_owned(),
            poste: c.poste.clone(),
            empreinte_machine: c.empreinte_machine.clone(),
            correlation_id: c.correlation_id.clone(),
            empreinte_precedente: EmpreinteAudit::amorce(c.etablissement_id),
            empreinte: EmpreinteAudit::amorce(c.etablissement_id),
        });
    }

    /// Enregistre un événement métier à écrire dans l'`outbox` à la validation.
    pub fn emettre(
        &mut self,
        type_evenement: TypeEvenement,
        entite: &str,
        entite_id: &str,
        charge_utile: serde_json::Value,
    ) {
        let c = &self.contexte;
        self.evenements.push(Evenement {
            id: nouvel_id(),
            type_evenement,
            version: type_evenement.version(),
            entite: entite.to_owned(),
            entite_id: entite_id.to_owned(),
            etablissement_id: c.etablissement_id,
            acteur_id: c.utilisateur_id,
            horodatage: c.maintenant,
            poste: c.poste.clone(),
            charge_utile,
            correlation_id: c.correlation_id.clone(),
            consomme_le: None,
        });
    }

    /// Planifie un effet de bord pour après la validation.
    pub fn planifier(&mut self, effet: E) {
        self.effets.push(effet);
    }

    /// Instant de l'opération.
    pub fn maintenant(&self) -> Horodatage {
        self.contexte.maintenant
    }

    /// Écrit l'audit (chaîné sur la dernière empreinte de l'établissement) et l'`outbox`, puis
    /// valide. Rend les effets à exécuter. En cas d'erreur, tout est annulé.
    pub async fn valider(mut self) -> Resultat<Vec<E>> {
        let mut precedente =
            audit::derniere_empreinte(&mut self.tx, self.contexte.etablissement_id).await?;
        for ligne in std::mem::take(&mut self.audits) {
            let scellee = ligne.sceller(precedente);
            precedente = scellee.empreinte;
            audit::inserer(&mut self.tx, &scellee).await?;
        }
        for evenement in std::mem::take(&mut self.evenements) {
            outbox::inserer(&mut self.tx, &evenement).await?;
        }
        self.tx.commit().await?;
        Ok(self.effets)
    }

    /// Abandonne explicitement.
    pub async fn annuler(self) -> Resultat<()> {
        self.tx.rollback().await?;
        Ok(())
    }
}
