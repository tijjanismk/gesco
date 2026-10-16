//! Le noyau applicatif : base, horloge, identité du poste, sessions et exécuteur d'effets.
//! Les services s'y adossent ; la frontière (`gesco-app`) le détient en état partagé.

use std::sync::Arc;

use gesco_core::commun::Id;
use gesco_core::core::{Etablissement, Poste, Utilisateur};
use gesco_db::unite::Durabilite;
use gesco_db::{Base, Contexte, UniteDeTravail, nouvel_id};

use crate::effets::{Effet, ExecuteurEffets, JournalEffets};
use crate::erreur::Resultat;
use crate::horloge::{Horloge, HorlogeSysteme};
use crate::session::{PolitiqueSession, Session, Sessions};

/// Options d'assemblage du noyau.
pub struct OptionsNoyau {
    /// Base ouverte et migrée.
    pub base: Base,
    /// Nom du poste (ADR-014 : identifie l'origine d'une écriture).
    pub poste: Poste,
    /// Empreinte machine, portée par l'audit et la licence.
    pub empreinte_machine: String,
    /// Horloge ; [`HorlogeSysteme`] en production.
    pub horloge: Arc<dyn Horloge>,
    /// Exécuteur des effets après commit.
    pub effets: Arc<dyn ExecuteurEffets>,
    /// Politique de session.
    pub politique_session: PolitiqueSession,
}

impl OptionsNoyau {
    /// Options de production sur une base donnée, avec le journal d'effets de la phase 0.5.
    pub fn production(base: Base, poste: Poste, empreinte_machine: String) -> Self {
        Self {
            base,
            poste,
            empreinte_machine,
            horloge: Arc::new(HorlogeSysteme),
            effets: Arc::new(JournalEffets::default()),
            politique_session: PolitiqueSession::default(),
        }
    }
}

/// Noyau applicatif partagé.
pub struct Noyau {
    base: Base,
    poste: Poste,
    empreinte_machine: String,
    horloge: Arc<dyn Horloge>,
    effets: Arc<dyn ExecuteurEffets>,
    sessions: Sessions,
    politique_session: PolitiqueSession,
}

impl Noyau {
    /// Assemble le noyau.
    pub fn assembler(options: OptionsNoyau) -> Self {
        Self {
            base: options.base,
            poste: options.poste,
            empreinte_machine: options.empreinte_machine,
            horloge: options.horloge,
            effets: options.effets,
            sessions: Sessions::default(),
            politique_session: options.politique_session,
        }
    }

    /// Base de données.
    pub fn base(&self) -> &Base {
        &self.base
    }

    /// Poste.
    pub fn poste(&self) -> &Poste {
        &self.poste
    }

    /// Empreinte de la machine, comparée à celle de la licence.
    pub fn empreinte_machine(&self) -> &str {
        &self.empreinte_machine
    }

    /// Horloge.
    pub fn horloge(&self) -> &dyn Horloge {
        self.horloge.as_ref()
    }

    /// Sessions du poste, après application de la politique d'inactivité à l'instant courant.
    pub fn sessions(&self) -> &Sessions {
        self.sessions
            .appliquer(self.politique_session, self.horloge.maintenant());
        &self.sessions
    }

    /// Politique de session en vigueur.
    pub fn politique_session(&self) -> PolitiqueSession {
        self.politique_session
    }

    /// Contexte d'exécution pour un utilisateur (ou pour le système quand `session` est `None`).
    pub fn contexte(
        &self,
        etablissement_id: Id<Etablissement>,
        session: Option<&Session>,
    ) -> Contexte {
        Contexte {
            etablissement_id,
            utilisateur_id: session.map(|s| s.utilisateur_id),
            role_actif: session.and_then(|s| s.role_actif.clone()),
            poste: self.poste.clone(),
            empreinte_machine: self.empreinte_machine.clone(),
            maintenant: self.horloge.maintenant(),
            correlation_id: nouvel_id::<Utilisateur>().vers_chaine(),
        }
    }

    /// Ouvre une unité de travail.
    pub async fn commencer(
        &self,
        contexte: Contexte,
        durabilite: Durabilite,
    ) -> Resultat<UniteDeTravail<Effet>> {
        Ok(UniteDeTravail::commencer(&self.base, contexte, durabilite).await?)
    }

    /// Valide l'unité de travail puis exécute ses effets, strictement après le commit (R-18).
    /// Retourne le nombre d'effets exécutés sans erreur.
    pub async fn terminer(&self, unite: UniteDeTravail<Effet>) -> Resultat<usize> {
        let contexte = unite.contexte().clone();
        let effets = unite.valider().await?;
        let mut reussis = 0;
        for effet in effets {
            if self.effets.executer(&contexte, effet).is_ok() {
                reussis += 1;
            }
        }
        Ok(reussis)
    }
}
