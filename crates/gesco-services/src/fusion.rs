//! Fusion de deux fiches élèves en doublon (SPEC_PHASE_2 §8), à la demande d'un utilisateur qui
//! détient `students:merge` : la recherche propose, elle ne fusionne jamais (T-07).

use gesco_core::commun::Id;
use gesco_core::core::{Operation, TypeEvenement};
use gesco_core::scolarite::fusion::{fiche_fusionnee, verifier_fusion};
use gesco_core::scolarite::{Eleve, Personne, Qualites};
use gesco_db::depots::{inscriptions, personnes};
use gesco_db::unite::Durabilite;
use serde::{Deserialize, Serialize};

use crate::eleves::{FicheEleve, ServiceEleves};
use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Fiches désignées pour la fusion.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeFusion {
    /// Fiche gardée : son matricule et ses valeurs l'emportent.
    pub conservee_id: Id<Eleve>,
    /// Fiche absorbée : elle reste lisible, désignée comme absorbée.
    pub absorbee_id: Id<Eleve>,
}

/// Ce que la fusion produira, montré avant la confirmation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct ApercuFusion {
    /// Fiche conservée, telle qu'elle est.
    pub conservee: FicheEleve,
    /// Fiche absorbée, telle qu'elle est.
    pub absorbee: FicheEleve,
    /// Élève après la fusion.
    pub eleve: Eleve,
    /// Identité après la fusion.
    pub identite: Personne,
    /// Années où les deux fiches ont chacune une inscription en cours : la fusion est alors
    /// refusée tant que l'une n'est pas abandonnée.
    pub annees_concurrentes: Vec<String>,
}

/// Service de fusion.
pub struct ServiceFusion<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceFusion<'n> {
    /// Service adossé au noyau.
    pub const fn nouveau(noyau: &'n Noyau) -> Self {
        Self { noyau }
    }

    /// Montre le résultat d'une fusion sans rien écrire.
    pub async fn apercu(
        &self,
        session: &Session,
        demande: &DemandeFusion,
    ) -> Resultat<ApercuFusion> {
        let eleves = ServiceEleves::nouveau(self.noyau);
        let conservee = eleves.fiche(session, demande.conservee_id).await?;
        let absorbee = eleves.fiche(session, demande.absorbee_id).await?;
        verifier_fusion(&conservee.eleve, &absorbee.eleve).map_err(regle)?;
        let (eleve, identite) = fiche_fusionnee(
            (&conservee.eleve, &conservee.identite),
            (&absorbee.eleve, &absorbee.identite),
        );
        let vivantes = |f: &FicheEleve| -> Vec<(String, String)> {
            f.inscriptions
                .iter()
                .filter(|i| i.inscription.statut.est_vivante())
                .map(|i| (i.inscription.annee_id.vers_chaine(), i.annee.clone()))
                .collect()
        };
        let des_deux = vivantes(&absorbee);
        let annees_concurrentes = vivantes(&conservee)
            .into_iter()
            .filter(|(id, _)| des_deux.iter().any(|(autre, _)| autre == id))
            .map(|(_, libelle)| libelle)
            .collect();
        Ok(ApercuFusion {
            conservee,
            absorbee,
            eleve,
            identite,
            annees_concurrentes,
        })
    }

    /// Fusionne : inscriptions et responsables de l'absorbée passent sur la conservée, ses champs
    /// vides sont complétés, l'absorbée est désignée comme absorbée. Une seule transaction.
    pub async fn fusionner(
        &self,
        session: &Session,
        demande: &DemandeFusion,
    ) -> Resultat<FicheEleve> {
        let apercu = self.apercu(session, demande).await?;
        if !apercu.annees_concurrentes.is_empty() {
            return Err(regle("fusion.inscriptions_concurrentes"));
        }
        let (conservee, absorbee) = (&apercu.conservee, &apercu.absorbee);
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let photo_absorbee = if conservee.photo.is_none() {
            let mut cnx = self
                .noyau
                .base()
                .pool()
                .acquire()
                .await
                .map_err(gesco_db::ErreurBase::from)?;
            personnes::photo(&mut cnx, absorbee.identite.id, &contexte).await?
        } else {
            None
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        // L'absorbée d'abord : son matricule national quitte l'index d'unicité avant que la
        // conservée ne le reprenne.
        personnes::absorber(
            unite.connexion(),
            absorbee.eleve.id,
            conservee.eleve.id,
            &contexte,
        )
        .await?;
        personnes::modifier_personne(unite.connexion(), &apercu.identite, &contexte).await?;
        personnes::modifier_eleve(unite.connexion(), &apercu.eleve, &contexte).await?;
        if let Some((octets, format)) = &photo_absorbee {
            personnes::definir_photo(
                unite.connexion(),
                conservee.identite.id,
                Some((octets.as_slice(), format.as_str())),
                &contexte,
            )
            .await?;
        }
        let inscriptions_deplacees = inscriptions::reattribuer(
            unite.connexion(),
            absorbee.eleve.id,
            conservee.eleve.id,
            &contexte,
        )
        .await?;
        let mut ordre = conservee
            .responsables
            .iter()
            .map(|r| r.qualites.ordre_priorite)
            .max()
            .unwrap_or(0);
        let mut responsables_ajoutes = Vec::new();
        for r in &absorbee.responsables {
            if conservee
                .responsables
                .iter()
                .any(|c| c.responsable.id == r.responsable.id)
            {
                continue;
            }
            ordre += 1;
            let qualites = Qualites {
                ordre_priorite: ordre,
                ..r.qualites
            };
            personnes::lier(
                unite.connexion(),
                conservee.eleve.id,
                r.responsable.id,
                &qualites,
                &contexte,
            )
            .await?;
            responsables_ajoutes.push(r.responsable.id);
        }
        unite.auditer(
            "eleve",
            &absorbee.eleve.id.vers_chaine(),
            Operation::Modification,
            Some(serde_json::json!({ "eleve": absorbee.eleve })),
            Some(serde_json::json!({ "fusionne_dans": conservee.eleve.id })),
            "fusion.absorber",
        );
        unite.auditer(
            "eleve",
            &conservee.eleve.id.vers_chaine(),
            Operation::Modification,
            Some(serde_json::json!({ "eleve": conservee.eleve, "identite": conservee.identite })),
            Some(serde_json::json!({
                "eleve": apercu.eleve,
                "identite": apercu.identite,
                "absorbee": absorbee.eleve.id,
                "inscriptions_deplacees": inscriptions_deplacees,
                "responsables_ajoutes": responsables_ajoutes,
                "photo_reprise": photo_absorbee.is_some(),
            })),
            "fusion.conserver",
        );
        unite.emettre(
            TypeEvenement::EleveFusionne,
            "eleve",
            &conservee.eleve.id.vers_chaine(),
            serde_json::json!({
                "conservee": conservee.eleve.id,
                "absorbee": absorbee.eleve.id,
                "matricule_conserve": conservee.eleve.matricule_interne,
                "matricule_absorbe": absorbee.eleve.matricule_interne,
            }),
        );
        self.noyau.terminer(unite).await?;
        ServiceEleves::nouveau(self.noyau)
            .fiche(session, conservee.eleve.id)
            .await
    }
}
