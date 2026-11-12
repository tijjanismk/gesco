//! Compétences du jardin d'enfants (SPEC_PHASE_11_B §3.1, §4.1) : domaines communs aux trois
//! sections, compétences de chaque section, liste livrée posée pour les sections que l'école a
//! ouvertes, puis tenue par la direction (`academic_year:manage`) : ajouter, renommer, monter,
//! descendre, retirer, remettre, recopier d'une section à l'autre. Rien ne se supprime (R-14) ;
//! la liste n'a pas d'année scolaire.

use gesco_core::commun::Id;
use gesco_core::core::{Etablissement, Operation, TypeEvenement, ValeurParametre};
use gesco_core::pedagogie::jardin::{
    DOMAINES_LIVRES, NiveauAcquisition, parametres, verifier_libelle_competence,
    verifier_libelle_domaine,
};
use gesco_core::pedagogie::{Competence, DomaineCompetence};
use gesco_core::scolarite::{Cycle, Niveau};
use gesco_db::depots::{academique, jardin, parametre};
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

/// Sections du jardin que l'école a ouvertes, dans l'ordre.
fn sections_ouvertes(niveaux: &[Niveau]) -> Vec<Niveau> {
    niveaux
        .iter()
        .filter(|n| n.actif && n.cycle == Cycle::Jardin)
        .cloned()
        .collect()
}

/// Pose la liste livrée là où elle manque (SPEC_PHASE_11_B §8) : les domaines livrés absents,
/// reconnus par leur code, puis les compétences de chaque section ouverte qui n'en a encore
/// aucune, retirées comprises. Une école sans jardin ouvert ne reçoit rien ; rien d'existant
/// n'est modifié. Rend le nombre de lignes ajoutées.
pub(crate) async fn poser_competences(
    unite: &mut UniteDeTravail<Effet>,
    etablissement_id: Id<Etablissement>,
    origine: &str,
) -> Resultat<usize> {
    let contexte = unite.contexte().clone();
    let sections = sections_ouvertes(&academique::niveaux(unite.connexion(), &contexte).await?);
    if sections.is_empty() {
        return Ok(0);
    }
    let mut domaines = jardin::domaines(unite.connexion(), &contexte).await?;
    let competences = jardin::competences(unite.connexion(), &contexte).await?;
    let mut ajoutes = 0;
    for livre in DOMAINES_LIVRES {
        if domaines
            .iter()
            .any(|d| d.code.as_deref() == Some(livre.code))
        {
            continue;
        }
        let d = DomaineCompetence {
            id: nouvel_id(),
            etablissement_id,
            code: Some(livre.code.to_owned()),
            libelle: livre.libelle.to_owned(),
            ordre: domaines.iter().map(|d| d.ordre).max().unwrap_or(0) + 1,
            livre: true,
            retire_le: None,
        };
        jardin::inserer_domaine(unite.connexion(), &d, &contexte).await?;
        unite.auditer(
            "domaine_competence",
            &d.id.vers_chaine(),
            Operation::Creation,
            None,
            Some(serde_json::to_value(&d)?),
            origine,
        );
        domaines.push(d);
        ajoutes += 1;
    }
    for section in sections
        .iter()
        .filter(|s| !competences.iter().any(|c| c.niveau_id == s.id))
    {
        let mut posees = 0;
        for livre in DOMAINES_LIVRES {
            let Some(domaine) = domaines
                .iter()
                .find(|d| d.code.as_deref() == Some(livre.code))
            else {
                continue;
            };
            for (ordre, libelle) in (1_i64..).zip(livre.de_la_section(&section.code)) {
                let c = Competence {
                    id: nouvel_id(),
                    etablissement_id,
                    domaine_id: domaine.id,
                    niveau_id: section.id,
                    libelle: (*libelle).to_owned(),
                    ordre,
                    livree: true,
                    retiree_le: None,
                };
                jardin::inserer_competence(unite.connexion(), &c, &contexte).await?;
                posees += 1;
            }
        }
        if posees > 0 {
            unite.auditer(
                "niveau",
                &section.id.vers_chaine(),
                Operation::Modification,
                None,
                Some(serde_json::json!({ "competences_livrees_posees": posees })),
                origine,
            );
        }
        ajoutes += posees;
    }
    Ok(ajoutes)
}

/// Compétences du jardin et réglages de l'échelle, pour l'écran de la direction et les écrans de
/// saisie.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct CompetencesJardin {
    /// Sections que l'école a ouvertes ; vide pour une école sans jardin.
    pub sections: Vec<Niveau>,
    /// Domaines, retirés compris, dans leur ordre.
    pub domaines: Vec<DomaineCompetence>,
    /// Compétences, retirées comprises, dans l'ordre de leur domaine.
    pub competences: Vec<Competence>,
    /// Libellés des trois niveaux : acquis, en cours, non acquis.
    pub echelle: Vec<String>,
    /// Vrai si la ligne de commentaire par domaine se montre.
    pub commentaire_domaine: bool,
}

/// Domaine saisi par la direction ; sans `id`, une création.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeDomaine {
    /// Domaine modifié ; absent pour une création.
    pub id: Option<Id<DomaineCompetence>>,
    /// Libellé.
    pub libelle: String,
    /// Vrai pour le retirer, faux pour le garder ou le remettre.
    pub retire: bool,
}

/// Compétence saisie par la direction ; sans `id`, une création.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeCompetence {
    /// Compétence modifiée ; absente pour une création.
    pub id: Option<Id<Competence>>,
    /// Domaine.
    pub domaine_id: Id<DomaineCompetence>,
    /// Section ; celle d'une compétence existante ne change pas.
    pub niveau_id: Id<Niveau>,
    /// Libellé.
    pub libelle: String,
    /// Vrai pour la retirer, faux pour la garder ou la remettre.
    pub retiree: bool,
}

/// Domaine monté ou descendu d'un rang.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeDeplacementDomaine {
    /// Domaine.
    pub id: Id<DomaineCompetence>,
    /// Vrai pour monter, faux pour descendre.
    pub monter: bool,
}

/// Compétence montée ou descendue d'un rang dans son domaine.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeDeplacementCompetence {
    /// Compétence.
    pub id: Id<Competence>,
    /// Vrai pour monter, faux pour descendre.
    pub monter: bool,
}

/// Compétences d'une section recopiées dans une autre, pour partir d'elles.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeRecopie {
    /// Section dont on prend les compétences.
    pub source_id: Id<Niveau>,
    /// Section qui les reçoit.
    pub cible_id: Id<Niveau>,
}

/// Nouvel ordre d'une liste après un déplacement : les positions de la liste (1, 2, 3…), avec
/// l'élément `i` échangé avec son voisin. `None` s'il n'a pas de voisin de ce côté.
fn echanger(nombre: usize, i: usize, monter: bool) -> Option<Vec<usize>> {
    let j = if monter {
        i.checked_sub(1)?
    } else {
        Some(i + 1).filter(|j| *j < nombre)?
    };
    let mut ordre: Vec<usize> = (0..nombre).collect();
    ordre.swap(i, j);
    Some(ordre)
}

fn meme_libelle(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

/// Service des compétences du jardin.
pub struct ServiceJardin<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceJardin<'n> {
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

    /// Domaines, compétences et échelle.
    pub async fn lire(&self, session: &Session) -> Resultat<CompetencesJardin> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let sections = sections_ouvertes(&academique::niveaux(&mut cnx, &contexte).await?);
        let mut echelle = Vec::with_capacity(3);
        for niveau in NiveauAcquisition::TOUS {
            let lu = parametre::lire(&mut cnx, niveau.parametre(), &contexte).await?;
            echelle.push(match lu.map(|p| p.valeur) {
                Some(ValeurParametre::Texte(t)) if !t.trim().is_empty() => t,
                _ => niveau.libelle_livre().to_owned(),
            });
        }
        let commentaire_domaine = !matches!(
            parametre::lire(&mut cnx, parametres::COMMENTAIRE_DOMAINE, &contexte)
                .await?
                .map(|p| p.valeur),
            Some(ValeurParametre::Booleen(false))
        );
        Ok(CompetencesJardin {
            sections,
            domaines: jardin::domaines(&mut cnx, &contexte).await?,
            competences: jardin::competences(&mut cnx, &contexte).await?,
            echelle,
            commentaire_domaine,
        })
    }

    fn tracer(
        &self,
        unite: &mut UniteDeTravail<Effet>,
        entite: &str,
        id: &str,
        avant: Option<serde_json::Value>,
        apres: serde_json::Value,
        origine: &str,
    ) {
        unite.auditer(
            entite,
            id,
            if avant.is_some() {
                Operation::Modification
            } else {
                Operation::Creation
            },
            avant,
            Some(apres),
            origine,
        );
        unite.emettre(
            TypeEvenement::CompetenceModifiee,
            entite,
            id,
            serde_json::json!({ "origine": origine }),
        );
    }

    /// Crée, renomme, retire ou remet un domaine.
    pub async fn enregistrer_domaine(
        &self,
        session: &Session,
        demande: &DemandeDomaine,
    ) -> Resultat<DomaineCompetence> {
        let libelle = verifier_libelle_domaine(&demande.libelle).map_err(regle)?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let existants = {
            let mut cnx = self.connexion().await?;
            jardin::domaines(&mut cnx, &contexte).await?
        };
        let avant = match demande.id {
            Some(id) => Some(
                existants
                    .iter()
                    .find(|d| d.id == id)
                    .cloned()
                    .ok_or(regle("jardin.domaine_inconnu"))?,
            ),
            None => None,
        };
        let retire_le = match &avant {
            Some(a) if demande.retire => a.retire_le.or(Some(contexte.maintenant)),
            _ => None,
        };
        if retire_le.is_none()
            && existants.iter().any(|d| {
                d.retire_le.is_none()
                    && Some(d.id) != demande.id
                    && meme_libelle(&d.libelle, &libelle)
            })
        {
            return Err(regle("jardin.domaine_existe"));
        }
        let apres = match &avant {
            Some(a) => DomaineCompetence {
                libelle,
                retire_le,
                ..a.clone()
            },
            None => DomaineCompetence {
                id: nouvel_id(),
                etablissement_id: session.etablissement_id,
                code: None,
                libelle,
                ordre: existants.iter().map(|d| d.ordre).max().unwrap_or(0) + 1,
                livre: false,
                retire_le: None,
            },
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        match &avant {
            Some(_) => jardin::ecrire_domaine(unite.connexion(), &apres, &contexte).await?,
            None => jardin::inserer_domaine(unite.connexion(), &apres, &contexte).await?,
        }
        self.tracer(
            &mut unite,
            "domaine_competence",
            &apres.id.vers_chaine(),
            avant.as_ref().map(serde_json::to_value).transpose()?,
            serde_json::to_value(&apres)?,
            "jardin.domaine",
        );
        self.noyau.terminer(unite).await?;
        Ok(apres)
    }

    /// Crée, renomme, change de domaine, retire ou remet une compétence.
    pub async fn enregistrer_competence(
        &self,
        session: &Session,
        demande: &DemandeCompetence,
    ) -> Resultat<Competence> {
        let libelle = verifier_libelle_competence(&demande.libelle).map_err(regle)?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let (niveaux, domaines, existantes) = {
            let mut cnx = self.connexion().await?;
            (
                academique::niveaux(&mut cnx, &contexte).await?,
                jardin::domaines(&mut cnx, &contexte).await?,
                jardin::competences(&mut cnx, &contexte).await?,
            )
        };
        let avant = match demande.id {
            Some(id) => Some(
                existantes
                    .iter()
                    .find(|c| c.id == id)
                    .cloned()
                    .ok_or(regle("jardin.competence_inconnue"))?,
            ),
            None => None,
        };
        let niveau_id = avant.as_ref().map_or(demande.niveau_id, |a| a.niveau_id);
        if !niveaux
            .iter()
            .any(|n| n.id == niveau_id && n.cycle == Cycle::Jardin)
        {
            return Err(regle("jardin.section_invalide"));
        }
        let domaine = domaines
            .iter()
            .find(|d| d.id == demande.domaine_id)
            .ok_or(regle("jardin.domaine_inconnu"))?;
        let change_de_domaine = avant.as_ref().is_none_or(|a| a.domaine_id != domaine.id);
        if change_de_domaine && domaine.retire_le.is_some() {
            return Err(regle("jardin.domaine_retire"));
        }
        let retiree_le = match &avant {
            Some(a) if demande.retiree => a.retiree_le.or(Some(contexte.maintenant)),
            _ => None,
        };
        if retiree_le.is_none()
            && existantes.iter().any(|c| {
                c.retiree_le.is_none()
                    && c.niveau_id == niveau_id
                    && c.domaine_id == domaine.id
                    && Some(c.id) != demande.id
                    && meme_libelle(&c.libelle, &libelle)
            })
        {
            return Err(regle("jardin.competence_existe"));
        }
        let fin_du_domaine = existantes
            .iter()
            .filter(|c| c.niveau_id == niveau_id && c.domaine_id == domaine.id)
            .map(|c| c.ordre)
            .max()
            .unwrap_or(0)
            + 1;
        let apres = match &avant {
            Some(a) => Competence {
                domaine_id: domaine.id,
                libelle,
                ordre: if change_de_domaine {
                    fin_du_domaine
                } else {
                    a.ordre
                },
                retiree_le,
                ..a.clone()
            },
            None => Competence {
                id: nouvel_id(),
                etablissement_id: session.etablissement_id,
                domaine_id: domaine.id,
                niveau_id,
                libelle,
                ordre: fin_du_domaine,
                livree: false,
                retiree_le: None,
            },
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        match &avant {
            Some(_) => jardin::ecrire_competence(unite.connexion(), &apres, &contexte).await?,
            None => jardin::inserer_competence(unite.connexion(), &apres, &contexte).await?,
        }
        self.tracer(
            &mut unite,
            "competence",
            &apres.id.vers_chaine(),
            avant.as_ref().map(serde_json::to_value).transpose()?,
            serde_json::to_value(&apres)?,
            "jardin.competence",
        );
        self.noyau.terminer(unite).await?;
        Ok(apres)
    }

    /// Monte ou descend un domaine d'un rang parmi ceux qui servent ; rien au bord de la liste.
    pub async fn deplacer_domaine(
        &self,
        session: &Session,
        demande: &DemandeDeplacementDomaine,
    ) -> Resultat<Vec<DomaineCompetence>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let tous = {
            let mut cnx = self.connexion().await?;
            jardin::domaines(&mut cnx, &contexte).await?
        };
        let actifs: Vec<&DomaineCompetence> =
            tous.iter().filter(|d| d.retire_le.is_none()).collect();
        let i = actifs
            .iter()
            .position(|d| d.id == demande.id)
            .ok_or(regle("jardin.domaine_inconnu"))?;
        let Some(ordre) = echanger(actifs.len(), i, demande.monter) else {
            return Ok(tous);
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let mut apres = Vec::new();
        for (rang, k) in (1_i64..).zip(ordre) {
            let Some(d) = actifs.get(k) else { continue };
            if d.ordre != rang {
                let nouveau = DomaineCompetence {
                    ordre: rang,
                    ..(*d).clone()
                };
                jardin::ecrire_domaine(unite.connexion(), &nouveau, &contexte).await?;
                apres.push(nouveau);
            }
        }
        self.tracer(
            &mut unite,
            "domaine_competence",
            &demande.id.vers_chaine(),
            Some(serde_json::to_value(&actifs)?),
            serde_json::to_value(&apres)?,
            "jardin.domaine_deplacer",
        );
        self.noyau.terminer(unite).await?;
        let mut cnx = self.connexion().await?;
        Ok(jardin::domaines(&mut cnx, &contexte).await?)
    }

    /// Monte ou descend une compétence d'un rang dans son domaine et sa section, parmi celles
    /// qui servent ; rien au bord de la liste.
    pub async fn deplacer_competence(
        &self,
        session: &Session,
        demande: &DemandeDeplacementCompetence,
    ) -> Resultat<Vec<Competence>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let toutes = {
            let mut cnx = self.connexion().await?;
            jardin::competences(&mut cnx, &contexte).await?
        };
        let visee = toutes
            .iter()
            .find(|c| c.id == demande.id)
            .ok_or(regle("jardin.competence_inconnue"))?;
        let mut voisines: Vec<&Competence> = toutes
            .iter()
            .filter(|c| {
                c.retiree_le.is_none()
                    && c.niveau_id == visee.niveau_id
                    && c.domaine_id == visee.domaine_id
            })
            .collect();
        voisines.sort_by_key(|c| c.ordre);
        let i = voisines
            .iter()
            .position(|c| c.id == demande.id)
            .ok_or(regle("jardin.competence_retiree"))?;
        let Some(ordre) = echanger(voisines.len(), i, demande.monter) else {
            return Ok(toutes);
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let mut apres = Vec::new();
        for (rang, k) in (1_i64..).zip(ordre) {
            let Some(c) = voisines.get(k) else { continue };
            if c.ordre != rang {
                let nouvelle = Competence {
                    ordre: rang,
                    ..(*c).clone()
                };
                jardin::ecrire_competence(unite.connexion(), &nouvelle, &contexte).await?;
                apres.push(nouvelle);
            }
        }
        self.tracer(
            &mut unite,
            "competence",
            &demande.id.vers_chaine(),
            Some(serde_json::to_value(&voisines)?),
            serde_json::to_value(&apres)?,
            "jardin.competence_deplacer",
        );
        self.noyau.terminer(unite).await?;
        let mut cnx = self.connexion().await?;
        Ok(jardin::competences(&mut cnx, &contexte).await?)
    }

    /// Recopie dans une section les compétences qui servent dans une autre, à la fin de chaque
    /// domaine ; celles que la section a déjà, au même libellé, ne sont pas doublées. Rend le
    /// nombre de compétences ajoutées.
    pub async fn recopier(&self, session: &Session, demande: &DemandeRecopie) -> Resultat<u32> {
        if demande.source_id == demande.cible_id {
            return Err(regle("jardin.recopie_meme_section"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let (niveaux, domaines, existantes) = {
            let mut cnx = self.connexion().await?;
            (
                academique::niveaux(&mut cnx, &contexte).await?,
                jardin::domaines(&mut cnx, &contexte).await?,
                jardin::competences(&mut cnx, &contexte).await?,
            )
        };
        let du_jardin = |id: Id<Niveau>| {
            niveaux
                .iter()
                .any(|n| n.id == id && n.cycle == Cycle::Jardin)
        };
        if !du_jardin(demande.source_id) || !du_jardin(demande.cible_id) {
            return Err(regle("jardin.section_invalide"));
        }
        let mut ajoutees: Vec<Competence> = Vec::new();
        for domaine in domaines.iter().filter(|d| d.retire_le.is_none()) {
            let mut source: Vec<&Competence> = existantes
                .iter()
                .filter(|c| {
                    c.retiree_le.is_none()
                        && c.niveau_id == demande.source_id
                        && c.domaine_id == domaine.id
                })
                .collect();
            source.sort_by_key(|c| c.ordre);
            let dans_la_cible: Vec<&Competence> = existantes
                .iter()
                .filter(|c| c.niveau_id == demande.cible_id && c.domaine_id == domaine.id)
                .collect();
            let mut ordre = dans_la_cible.iter().map(|c| c.ordre).max().unwrap_or(0);
            for c in source {
                let deja = dans_la_cible
                    .iter()
                    .any(|x| x.retiree_le.is_none() && meme_libelle(&x.libelle, &c.libelle));
                if deja {
                    continue;
                }
                ordre += 1;
                ajoutees.push(Competence {
                    id: nouvel_id(),
                    etablissement_id: session.etablissement_id,
                    domaine_id: domaine.id,
                    niveau_id: demande.cible_id,
                    libelle: c.libelle.clone(),
                    ordre,
                    livree: false,
                    retiree_le: None,
                });
            }
        }
        if ajoutees.is_empty() {
            return Ok(0);
        }
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        for c in &ajoutees {
            jardin::inserer_competence(unite.connexion(), c, &contexte).await?;
        }
        self.tracer(
            &mut unite,
            "niveau",
            &demande.cible_id.vers_chaine(),
            None,
            serde_json::json!({
                "recopiees_de": demande.source_id.vers_chaine(),
                "competences": ajoutees.len(),
            }),
            "jardin.recopier",
        );
        self.noyau.terminer(unite).await?;
        Ok(u32::try_from(ajoutees.len()).unwrap_or(u32::MAX))
    }
}

#[cfg(test)]
mod tests {
    use super::echanger;

    #[test]
    fn un_deplacement_echange_deux_voisins_et_s_arrete_au_bord() {
        assert_eq!(echanger(3, 1, true), Some(vec![1, 0, 2]));
        assert_eq!(echanger(3, 1, false), Some(vec![0, 2, 1]));
        assert_eq!(echanger(3, 0, true), None);
        assert_eq!(echanger(3, 2, false), None);
        assert_eq!(echanger(1, 0, false), None);
    }
}
