//! Libérer la place des photos (SPEC_PHASE_12_LOT_5 §7, ADR « copies photographiées ») : par
//! année, la place prise par les photos des copies ; une fois l'année close, la direction peut
//! vider leur contenu. Les rendus, les notes, les commentaires et les évaluations restent ;
//! l'écran de la copie montre « Photo retirée pour libérer de la place ». Le geste va au journal
//! d'audit ; il est refusé sur une année ouverte et n'est jamais automatique.

use gesco_core::core::{Operation, Permission, StatutAnnee};
use gesco_db::depots::annee_scolaire;
use gesco_db::depots::devoirs as depot;
use gesco_db::unite::Durabilite;
use serde::{Deserialize, Serialize};

use super::{ServiceDevoirs, regle};
use crate::erreur::{ErreurService, Resultat};
use crate::session::Session;

/// La place des photos d'une année.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct PlaceAnnee {
    /// Année.
    pub annee_id: String,
    /// « 2025-2026 ».
    pub libelle: String,
    /// L'année est close : sa place peut être libérée.
    pub close: bool,
    /// Photos gardées.
    pub photos: u32,
    /// Octets pris par les photos gardées.
    pub octets: i64,
    /// Photos déjà retirées.
    pub retirees: u32,
}

/// La place des photos, année par année (de la plus récente à la plus ancienne).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct PlacePhotos {
    /// Années qui ont des photos de copies.
    pub annees: Vec<PlaceAnnee>,
}

/// Une année désignée.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeLibererPhotos {
    /// Année.
    pub annee_id: String,
}

fn nombre(n: i64) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

impl ServiceDevoirs<'_> {
    async fn exiger_gestion(&self, session: &Session) -> Resultat<()> {
        let contexte = self.contexte(session);
        let droits = self
            .noyau
            .droits_de(session.utilisateur_id, &contexte)
            .await?;
        if droits.permissions.contains(&Permission::LIBRARY_MANAGE) && !droits.maitre {
            Ok(())
        } else {
            Err(ErreurService::PorteeRefusee {
                ressource: "place des photos".to_owned(),
            })
        }
    }

    /// La place prise par les photos des copies, par année.
    pub async fn place(&self, session: &Session) -> Resultat<PlacePhotos> {
        self.exiger_gestion(session).await?;
        let contexte = self.contexte(session);
        let mut cnx = self.connexion().await?;
        let mut annees = annee_scolaire::lister(&mut cnx, &contexte).await?;
        annees.sort_by_key(|a| std::cmp::Reverse(a.debut.millis_epoque()));
        let mut lignes = Vec::new();
        for a in annees {
            let id = a.id.vers_chaine();
            let (photos, octets, retirees) =
                depot::place_des_photos(&mut cnx, &id, &contexte).await?;
            if photos == 0 && retirees == 0 {
                continue;
            }
            lignes.push(PlaceAnnee {
                annee_id: id,
                libelle: a.libelle,
                close: matches!(a.statut, StatutAnnee::Cloturee | StatutAnnee::Archivee),
                photos: nombre(photos),
                octets,
                retirees: nombre(retirees),
            });
        }
        Ok(PlacePhotos { annees: lignes })
    }

    /// « Libérer la place des photos de cette année » : refusé tant que l'année n'est pas close.
    pub async fn liberer_photos(
        &self,
        session: &Session,
        demande: &DemandeLibererPhotos,
    ) -> Resultat<PlacePhotos> {
        self.exiger_gestion(session).await?;
        let contexte = self.contexte(session);
        let annee = {
            let mut cnx = self.connexion().await?;
            annee_scolaire::lister(&mut cnx, &contexte)
                .await?
                .into_iter()
                .find(|a| a.id.vers_chaine() == demande.annee_id)
                .ok_or(regle("devoir.annee_inconnue"))?
        };
        if !matches!(annee.statut, StatutAnnee::Cloturee | StatutAnnee::Archivee) {
            return Err(regle("devoir.annee_ouverte"));
        }
        let (photos, octets, _) = {
            let mut cnx = self.connexion().await?;
            depot::place_des_photos(&mut cnx, &demande.annee_id, &contexte).await?
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let retirees =
            depot::liberer_les_photos(unite.connexion(), &demande.annee_id, &contexte).await?;
        unite.auditer(
            "annee_scolaire",
            &demande.annee_id,
            Operation::Modification,
            Some(serde_json::json!({ "photos_de_copies": photos, "octets": octets })),
            Some(serde_json::json!({ "photos_retirees": retirees })),
            "devoirs.liberer_photos",
        );
        self.noyau.terminer(unite).await?;
        self.place(session).await
    }
}
