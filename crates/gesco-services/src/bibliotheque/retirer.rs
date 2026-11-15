//! Retirer et remettre (§6 de la spécification, ADR « retirer un contenu ») : la direction cache
//! un contenu, un chapitre, une matière d'un niveau, un niveau ou tout un paquet ; rien n'est
//! effacé. Un retrait vise une clé stable : ce qu'une mise à jour ajoute dans un bloc retiré reste
//! caché. La remise fait revenir ce qui était visible, versions de l'école comprises.
//!
//! Et la correspondance des disciplines : sous quelle matière de l'école se range chaque
//! discipline de la bibliothèque, dans chaque niveau.

use std::collections::{HashMap, HashSet};

use gesco_core::bibliotheque::{CibleRetrait, GrainRetrait, NatureContenu, places_du_catalogue};
use gesco_core::core::{Operation, TypeEvenement};
use gesco_db::depots::bibliotheque as depot;
use gesco_db::depots::{academique, pedagogie};
use gesco_db::unite::Durabilite;
use serde::{Deserialize, Serialize};

use super::{ServiceBibliotheque, regle};
use crate::erreur::Resultat;
use crate::session::Session;

/// Retirer, ou demander ce qu'un retrait cacherait.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeRetrait {
    /// Grain.
    pub grain: GrainRetrait,
    /// Clé du contenu ou du chapitre, clé de la matière au catalogue, code du niveau ou du paquet.
    pub cible: String,
    /// Niveau d'une matière.
    pub niveau_code: Option<String>,
    /// Série d'une matière au lycée.
    pub serie_code: Option<String>,
}

/// Ce qu'un retrait cacherait, pour la confirmation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ApercuRetrait {
    /// Leçons.
    pub lecons: u32,
    /// Sujets d'examen.
    pub sujets: u32,
    /// Corrigés.
    pub corriges: u32,
    /// Documents.
    pub documents: u32,
}

/// Un retrait en cours, tel que la liste « Retirés » le montre.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct RetraitAffiche {
    /// Identifiant.
    pub id: String,
    /// Grain.
    pub grain: GrainRetrait,
    /// Cible.
    pub cible: String,
    /// Niveau d'une matière.
    pub niveau_code: Option<String>,
    /// Série d'une matière.
    pub serie_code: Option<String>,
    /// Titre du contenu ou du chapitre, libellé de la matière ou du niveau ; vide pour un paquet.
    pub titre: String,
    /// Libellé du niveau d'une matière, d'un chapitre ou d'un contenu.
    pub niveau: Option<String>,
    /// Retiré le, millisecondes depuis l'époque.
    pub retire_le: i64,
    /// Qui a retiré.
    pub retire_par: String,
    /// Autres retraits en cours qui cachent aussi une part de ce que celui-ci cache.
    pub aussi: Vec<String>,
}

/// Remettre un retrait.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeRemiseBibliotheque {
    /// Identifiant du retrait.
    pub retrait_id: String,
}

/// Une discipline de la bibliothèque dans un niveau, et la matière de l'école qui la reçoit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct CorrespondanceAffichee {
    /// Code de la discipline (`SVT`…).
    pub discipline_code: String,
    /// Libellé de la discipline dans la bibliothèque.
    pub discipline: String,
    /// Code du niveau.
    pub niveau_code: String,
    /// Libellé du niveau dans l'école.
    pub niveau: String,
    /// Matière de l'école ; aucune : sous le nom de la bibliothèque.
    pub matiere_id: Option<String>,
}

/// Une matière de l'école qui peut recevoir une discipline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct MatiereChoisissable {
    /// Identifiant.
    pub id: String,
    /// Libellé.
    pub libelle: String,
}

/// La correspondance des disciplines, pour les réglages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct CorrespondancesBibliotheque {
    /// Une ligne par discipline et par niveau actif de l'école.
    pub correspondances: Vec<CorrespondanceAffichee>,
    /// Matières actives de l'école.
    pub matieres: Vec<MatiereChoisissable>,
}

/// Régler la correspondance d'une discipline dans un niveau.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeCorrespondance {
    /// Code de la discipline.
    pub discipline_code: String,
    /// Code du niveau.
    pub niveau_code: String,
    /// Matière de l'école ; aucune : sous le nom de la bibliothèque.
    pub matiere_id: Option<String>,
}

impl DemandeRetrait {
    fn cible(&self) -> Resultat<CibleRetrait> {
        let cible = self.cible.trim();
        if cible.is_empty() || cible.chars().count() > 300 {
            return Err(regle("bibliotheque.retrait_incomplet"));
        }
        let (niveau_code, serie_code) = match self.grain {
            GrainRetrait::Matiere => {
                let niveau = self
                    .niveau_code
                    .as_deref()
                    .map(str::trim)
                    .filter(|n| !n.is_empty())
                    .ok_or(regle("bibliotheque.retrait_incomplet"))?;
                (
                    Some(niveau.to_owned()),
                    self.serie_code
                        .as_deref()
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_owned),
                )
            }
            _ => (None, None),
        };
        Ok(CibleRetrait {
            grain: self.grain,
            cible: cible.to_owned(),
            niveau_code,
            serie_code,
        })
    }
}

impl ServiceBibliotheque<'_> {
    /// Ce qu'un retrait cacherait de ce qui se voit aujourd'hui.
    pub async fn apercu_retrait(
        &self,
        session: &Session,
        demande: &DemandeRetrait,
    ) -> Resultat<ApercuRetrait> {
        let cible = demande.cible()?;
        let range = self.ranger_catalogue(session, true, true).await?;
        let mut vus = HashSet::new();
        let mut apercu = ApercuRetrait::default();
        for place in places_du_catalogue(&range.niveaux, &range.contenus) {
            if cible.couvre(&place) && vus.insert(place.contenu_id.clone()) {
                match place.nature {
                    NatureContenu::Lecon => apercu.lecons += 1,
                    NatureContenu::Sujet => apercu.sujets += 1,
                    NatureContenu::Corrige => apercu.corriges += 1,
                    NatureContenu::Document => apercu.documents += 1,
                }
            }
        }
        Ok(apercu)
    }

    /// Retire. Un retrait identique déjà en cours n'est pas doublé.
    pub async fn retirer(
        &self,
        session: &Session,
        demande: &DemandeRetrait,
    ) -> Resultat<Vec<RetraitAffiche>> {
        let cible = demande.cible()?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let deja = {
            let mut cnx = self.connexion().await?;
            depot::retraits(&mut cnx, true, &contexte)
                .await?
                .into_iter()
                .any(|r| {
                    r.grain == cible.grain.nom()
                        && r.cible == cible.cible
                        && r.niveau_code == cible.niveau_code
                        && r.serie_code == cible.serie_code
                })
        };
        if !deja {
            let mut unite = self
                .noyau
                .commencer(contexte.clone(), Durabilite::Critique)
                .await?;
            let id = depot::inserer_retrait(
                unite.connexion(),
                cible.grain.nom(),
                &cible.cible,
                cible.niveau_code.as_deref(),
                cible.serie_code.as_deref(),
                &contexte,
            )
            .await?;
            let valeur = serde_json::to_value(&cible)?;
            unite.auditer(
                "retrait_bibliotheque",
                &id,
                Operation::Creation,
                None,
                Some(valeur.clone()),
                "bibliotheque.retirer",
            );
            unite.emettre(
                TypeEvenement::BibliothequeRetiree,
                "retrait_bibliotheque",
                &id,
                valeur,
            );
            self.noyau.terminer(unite).await?;
        }
        self.retraits(session).await
    }

    /// Remet un retrait en cours.
    pub async fn remettre(
        &self,
        session: &Session,
        demande: &DemandeRemiseBibliotheque,
    ) -> Resultat<Vec<RetraitAffiche>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let avant = depot::retraits(unite.connexion(), true, &contexte)
            .await?
            .into_iter()
            .find(|r| r.id == demande.retrait_id)
            .ok_or(regle("bibliotheque.retrait_inconnu"))?;
        if !depot::remettre_retrait(unite.connexion(), &demande.retrait_id, &contexte).await? {
            return Err(regle("bibliotheque.retrait_inconnu"));
        }
        let valeur = serde_json::json!({
            "grain": avant.grain,
            "cible": avant.cible,
            "niveau_code": avant.niveau_code,
            "serie_code": avant.serie_code,
        });
        unite.auditer(
            "retrait_bibliotheque",
            &demande.retrait_id,
            Operation::Modification,
            Some(valeur.clone()),
            Some(serde_json::json!({ "remis": true })),
            "bibliotheque.remettre",
        );
        unite.emettre(
            TypeEvenement::BibliothequeRemise,
            "retrait_bibliotheque",
            &demande.retrait_id,
            valeur,
        );
        self.noyau.terminer(unite).await?;
        self.retraits(session).await
    }

    /// Les retraits en cours, les plus récents d'abord, avec ce qu'ils cachent et les autres
    /// retraits qui le cachent aussi.
    pub async fn retraits(&self, session: &Session) -> Resultat<Vec<RetraitAffiche>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let lignes = {
            let mut cnx = self.connexion().await?;
            depot::retraits(&mut cnx, true, &contexte).await?
        };
        // Le catalogue sans aucun retrait : ce que chaque retrait cacherait seul.
        let range = self.ranger_catalogue(session, true, false).await?;
        let places = places_du_catalogue(&range.niveaux, &range.contenus);
        let cibles: Vec<Option<CibleRetrait>> = lignes
            .iter()
            .map(|r| {
                Some(CibleRetrait {
                    grain: GrainRetrait::depuis_nom(&r.grain)?,
                    cible: r.cible.clone(),
                    niveau_code: r.niveau_code.clone(),
                    serie_code: r.serie_code.clone(),
                })
            })
            .collect();
        let couverts: Vec<HashSet<&str>> = cibles
            .iter()
            .map(|c| match c {
                Some(c) => places
                    .iter()
                    .filter(|p| c.couvre(p))
                    .map(|p| p.contenu_id.as_str())
                    .collect(),
                None => HashSet::new(),
            })
            .collect();
        let titres_contenus: HashMap<&str, &str> = range
            .contenus
            .iter()
            .map(|c| (c.cle.as_str(), c.titre.as_str()))
            .collect();
        let titres_chapitres: HashMap<&str, &str> = range
            .chapitres
            .iter()
            .map(|c| (c.cle.as_str(), c.titre.as_str()))
            .collect();
        let niveaux: HashMap<String, String> = {
            let mut cnx = self.connexion().await?;
            academique::niveaux(&mut cnx, &contexte)
                .await?
                .into_iter()
                .map(|n| (n.code, n.libelle))
                .collect()
        };

        let mut affiches = Vec::new();
        for (i, (ligne, cible)) in lignes.iter().zip(&cibles).enumerate() {
            let Some(cible) = cible else { continue };
            let place = places.iter().find(|p| cible.couvre(p));
            let (titre, niveau) = match cible.grain {
                GrainRetrait::Contenu => (
                    titres_contenus
                        .get(cible.cible.as_str())
                        .map_or_else(|| cible.cible.clone(), |t| (*t).to_owned()),
                    place.map(|p| p.niveau.clone()),
                ),
                GrainRetrait::Chapitre => (
                    titres_chapitres
                        .get(cible.cible.as_str())
                        .map_or_else(|| cible.cible.clone(), |t| (*t).to_owned()),
                    place.map(|p| p.niveau.clone()),
                ),
                GrainRetrait::Matiere => (
                    place.map_or_else(|| cible.cible.clone(), |p| p.matiere.clone()),
                    cible
                        .niveau_code
                        .as_ref()
                        .map(|n| niveaux.get(n).cloned().unwrap_or_else(|| n.clone())),
                ),
                GrainRetrait::Niveau => (
                    niveaux
                        .get(&cible.cible)
                        .cloned()
                        .unwrap_or_else(|| cible.cible.clone()),
                    None,
                ),
                GrainRetrait::Paquet => (String::new(), None),
            };
            // Un retrait plus large qui cache aussi une part de ce que celui-ci cache.
            let aussi = lignes
                .iter()
                .zip(&cibles)
                .enumerate()
                .filter(|(j, (_, autre))| {
                    *j != i
                        && autre.as_ref().is_some_and(|a| a.grain > cible.grain)
                        && couverts
                            .get(*j)
                            .zip(couverts.get(i))
                            .is_some_and(|(a, b)| !a.is_disjoint(b))
                })
                .map(|(_, (autre, _))| autre.id.clone())
                .collect();
            affiches.push(RetraitAffiche {
                id: ligne.id.clone(),
                grain: cible.grain,
                cible: cible.cible.clone(),
                niveau_code: cible.niveau_code.clone(),
                serie_code: cible.serie_code.clone(),
                titre,
                niveau,
                retire_le: ligne.retire_le,
                retire_par: ligne.retire_par.clone(),
                aussi,
            });
        }
        Ok(affiches)
    }

    /// La correspondance des disciplines, pour chaque niveau actif de l'école où la bibliothèque
    /// livre la discipline.
    pub async fn correspondances(
        &self,
        session: &Session,
    ) -> Resultat<CorrespondancesBibliotheque> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let niveaux: Vec<(String, String)> = academique::niveaux(&mut cnx, &contexte)
            .await?
            .into_iter()
            .filter(|n| n.actif)
            .map(|n| (n.code, n.libelle))
            .collect();
        let disciplines: HashMap<String, String> = depot::paquets(&mut cnx, &contexte)
            .await?
            .into_iter()
            .filter(|p| p.actif)
            .flat_map(|p| p.disciplines)
            .map(|d| (d.code, d.libelle))
            .collect();
        let lignes = depot::correspondances(&mut cnx, &contexte).await?;
        let mut correspondances = Vec::new();
        for (code, libelle) in &niveaux {
            for (discipline, niveau, matiere) in lignes.iter().filter(|l| &l.1 == code) {
                correspondances.push(CorrespondanceAffichee {
                    discipline_code: discipline.clone(),
                    discipline: disciplines
                        .get(discipline)
                        .cloned()
                        .unwrap_or_else(|| discipline.clone()),
                    niveau_code: niveau.clone(),
                    niveau: libelle.clone(),
                    matiere_id: matiere.clone(),
                });
            }
        }
        let matieres = pedagogie::matieres(&mut cnx, &contexte)
            .await?
            .into_iter()
            .filter(|m| m.active)
            .map(|m| MatiereChoisissable {
                id: m.id.vers_chaine(),
                libelle: m.libelle,
            })
            .collect();
        Ok(CorrespondancesBibliotheque {
            correspondances,
            matieres,
        })
    }

    /// Règle sous quelle matière de l'école se range une discipline dans un niveau. Une mise à
    /// jour de la bibliothèque ne la change plus.
    pub async fn regler_correspondance(
        &self,
        session: &Session,
        demande: &DemandeCorrespondance,
    ) -> Resultat<CorrespondancesBibliotheque> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        if demande.discipline_code.trim().is_empty() || demande.niveau_code.trim().is_empty() {
            return Err(regle("bibliotheque.retrait_incomplet"));
        }
        if let Some(id) = &demande.matiere_id {
            let mut cnx = self.connexion().await?;
            let connue = pedagogie::matieres(&mut cnx, &contexte)
                .await?
                .iter()
                .any(|m| m.active && &m.id.vers_chaine() == id);
            if !connue {
                return Err(regle("bibliotheque.matiere_inconnue"));
            }
        }
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let avant = depot::regler_correspondance(
            unite.connexion(),
            demande.discipline_code.trim(),
            demande.niveau_code.trim(),
            demande.matiere_id.as_deref(),
            &contexte,
        )
        .await?;
        let cle = format!(
            "{}/{}",
            demande.discipline_code.trim(),
            demande.niveau_code.trim()
        );
        unite.auditer(
            "correspondance_discipline",
            &cle,
            Operation::Modification,
            Some(serde_json::json!({ "matiere_id": avant })),
            Some(serde_json::to_value(demande)?),
            "bibliotheque.correspondance",
        );
        unite.emettre(
            TypeEvenement::CorrespondanceDisciplineModifiee,
            "correspondance_discipline",
            &cle,
            serde_json::to_value(demande)?,
        );
        self.noyau.terminer(unite).await?;
        self.correspondances(session).await
    }
}
