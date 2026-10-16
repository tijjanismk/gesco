//! Années scolaires (SPEC_PHASE_1 §3) : créer en préparation, corriger, ouvrir, supprimer. La
//! clôture et l'archivage viennent avec la phase 10 (UC-06).

use gesco_core::commun::{Horodatage, Id};
use gesco_core::core::{AnneeScolaire, MachineAEtats, Operation, StatutAnnee, TypeEvenement};
use gesco_core::scolarite::{Classe, Inscription};
use gesco_db::depots::{academique, annee_scolaire, inscriptions};
use gesco_db::nouvel_id;
use gesco_db::unite::Durabilite;

use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Service des années scolaires.
pub struct ServiceAnnees<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceAnnees<'n> {
    /// Service adossé au noyau.
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

    /// Années de l'établissement, la plus récente d'abord.
    pub async fn lister(&self, session: &Session) -> Resultat<Vec<AnneeScolaire>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let mut annees = annee_scolaire::lister(&mut cnx, &contexte).await?;
        annees.sort_by_key(|a| std::cmp::Reverse(a.debut.millis_epoque()));
        Ok(annees)
    }

    /// Vrai si aucune année n'est active : le bandeau le signale en permanence.
    pub async fn aucune_annee_active(&self, session: &Session) -> Resultat<bool> {
        Ok(!self
            .lister(session)
            .await?
            .iter()
            .any(|a| a.statut == StatutAnnee::Active))
    }

    /// Libellé et bornes valides, sans chevauchement d'une autre année de l'établissement.
    async fn verifier(&self, session: &Session, annee: &AnneeScolaire) -> Resultat<()> {
        annee.verifier().map_err(regle)?;
        if self
            .lister(session)
            .await?
            .iter()
            .any(|a| a.id != annee.id && a.chevauche(annee))
        {
            return Err(regle("annee.chevauchement"));
        }
        Ok(())
    }

    async fn lire(&self, session: &Session, id: Id<AnneeScolaire>) -> Resultat<AnneeScolaire> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        Ok(annee_scolaire::lire(&mut cnx, id, &contexte).await?)
    }

    /// Crée une année en préparation.
    pub async fn creer(
        &self,
        session: &Session,
        libelle: &str,
        debut: Horodatage,
        fin: Horodatage,
    ) -> Resultat<AnneeScolaire> {
        let annee = AnneeScolaire {
            id: nouvel_id(),
            etablissement_id: session.etablissement_id,
            libelle: libelle.trim().to_owned(),
            debut,
            fin,
            statut: StatutAnnee::Preparation,
            est_courante: false,
        };
        self.verifier(session, &annee).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        annee_scolaire::inserer(unite.connexion(), &annee, &contexte).await?;
        // Périodes, programme et règles par défaut, que l'école corrige (SPEC_PHASE_4 §2).
        crate::pedagogie::poser_annee(&mut unite, &annee, "annees.creer").await?;
        let id = annee.id.vers_chaine();
        unite.auditer(
            "annee_scolaire",
            &id,
            Operation::Creation,
            None,
            Some(serde_json::to_value(&annee)?),
            "annees.creer",
        );
        unite.emettre(
            TypeEvenement::AnneeScolairePreparee,
            "annee_scolaire",
            &id,
            serde_json::json!({
                "libelle": annee.libelle,
                "debut": annee.debut,
                "fin": annee.fin,
            }),
        );
        self.noyau.terminer(unite).await?;
        Ok(annee)
    }

    /// Corrige le libellé et les bornes d'une année encore en préparation.
    pub async fn modifier(
        &self,
        session: &Session,
        id: Id<AnneeScolaire>,
        libelle: &str,
        debut: Horodatage,
        fin: Horodatage,
    ) -> Resultat<AnneeScolaire> {
        let avant = self.lire(session, id).await?;
        if avant.statut != StatutAnnee::Preparation {
            return Err(regle("annee.non_modifiable"));
        }
        let apres = AnneeScolaire {
            libelle: libelle.trim().to_owned(),
            debut,
            fin,
            ..avant.clone()
        };
        self.verifier(session, &apres).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        annee_scolaire::modifier(unite.connexion(), &apres, &contexte).await?;
        unite.auditer(
            "annee_scolaire",
            &id.vers_chaine(),
            Operation::Modification,
            Some(serde_json::to_value(&avant)?),
            Some(serde_json::to_value(&apres)?),
            "annees.modifier",
        );
        self.noyau.terminer(unite).await?;
        Ok(apres)
    }

    /// Contrôles de l'ouverture, avant la sauvegarde qui la précède : transition permise et
    /// aucune autre année active. Une année active n'en sort que par la clôture (phase 10).
    pub async fn verifier_activation(
        &self,
        session: &Session,
        id: Id<AnneeScolaire>,
    ) -> Resultat<()> {
        let annee = self.lire(session, id).await?;
        annee.statut.transiter(StatutAnnee::Active)?;
        if !self.aucune_annee_active(session).await? {
            return Err(regle("annee.deja_active"));
        }
        Ok(())
    }

    /// Ouvre l'année : elle devient active et courante.
    pub async fn activer(
        &self,
        session: &Session,
        id: Id<AnneeScolaire>,
    ) -> Resultat<AnneeScolaire> {
        self.verifier_activation(session, id).await?;
        let avant = self.lire(session, id).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        annee_scolaire::ecrire_statut(unite.connexion(), id, StatutAnnee::Active, &contexte)
            .await?;
        annee_scolaire::definir_courante(unite.connexion(), id, &contexte).await?;
        let apres = AnneeScolaire {
            statut: StatutAnnee::Active,
            est_courante: true,
            ..avant.clone()
        };
        let id_texte = id.vers_chaine();
        unite.auditer(
            "annee_scolaire",
            &id_texte,
            Operation::Modification,
            Some(serde_json::to_value(&avant)?),
            Some(serde_json::to_value(&apres)?),
            "annees.activer",
        );
        unite.emettre(
            TypeEvenement::AnneeScolaireOuverte,
            "annee_scolaire",
            &id_texte,
            serde_json::json!({ "libelle": apres.libelle }),
        );
        self.noyau.terminer(unite).await?;
        Ok(apres)
    }

    /// Supprime une année restée en préparation : elle n'a jamais été ouverte.
    pub async fn supprimer(&self, session: &Session, id: Id<AnneeScolaire>) -> Resultat<()> {
        let avant = self.lire(session, id).await?;
        if avant.statut != StatutAnnee::Preparation {
            return Err(regle("annee.non_supprimable"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        annee_scolaire::supprimer(unite.connexion(), id, &contexte).await?;
        unite.auditer(
            "annee_scolaire",
            &id.vers_chaine(),
            Operation::Suppression,
            Some(serde_json::to_value(&avant)?),
            None,
            "annees.supprimer",
        );
        self.noyau.terminer(unite).await?;
        Ok(())
    }
}

/// Refuse une écriture sur une année clôturée ou archivée (ADR-012, SPEC_PHASE_10 §4) : elle se
/// rouvre d'abord, avec un motif.
pub(crate) async fn exiger_modifiable(
    noyau: &Noyau,
    session: &Session,
    annee_id: Id<AnneeScolaire>,
) -> Resultat<()> {
    let contexte = noyau.contexte(session.etablissement_id, Some(session));
    let mut cnx = noyau
        .base()
        .pool()
        .acquire()
        .await
        .map_err(gesco_db::ErreurBase::from)?;
    let annee = annee_scolaire::lire(&mut cnx, annee_id, &contexte).await?;
    if !annee.est_modifiable() {
        return Err(regle("annee.cloturee"));
    }
    Ok(())
}

/// Refuse une écriture dans une classe d'une année clôturée.
pub(crate) async fn exiger_classe_modifiable(
    noyau: &Noyau,
    session: &Session,
    classe_id: Id<Classe>,
) -> Resultat<()> {
    let contexte = noyau.contexte(session.etablissement_id, Some(session));
    let annee_id = {
        let mut cnx = noyau
            .base()
            .pool()
            .acquire()
            .await
            .map_err(gesco_db::ErreurBase::from)?;
        academique::classe(&mut cnx, classe_id, &contexte)
            .await?
            .annee_id
    };
    exiger_modifiable(noyau, session, annee_id).await
}

/// Refuse une écriture sur l'inscription d'une année clôturée.
pub(crate) async fn exiger_inscription_modifiable(
    noyau: &Noyau,
    session: &Session,
    inscription_id: Id<Inscription>,
) -> Resultat<()> {
    let contexte = noyau.contexte(session.etablissement_id, Some(session));
    let annee_id = {
        let mut cnx = noyau
            .base()
            .pool()
            .acquire()
            .await
            .map_err(gesco_db::ErreurBase::from)?;
        inscriptions::lire(&mut cnx, inscription_id, &contexte)
            .await?
            .annee_id
    };
    exiger_modifiable(noyau, session, annee_id).await
}
