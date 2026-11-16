//! « Pour qui » : un chapitre ou un contenu de l'école réservé à certaines classes du niveau
//! (phase 12, lot 2, §5 ; T-125). Le livré, et la version de l'école d'une leçon livrée, restent
//! pour tout le niveau. Le catalogue du personnel montre tout, avec la marque « 9e A
//! seulement » ; le mode classe et l'impression ne changent pas. La fonction « visible pour une
//! classe » sert le portail de l'élève (lot 4). Décocher ne supprime rien : la ligne garde
//! l'instant et l'auteur du retrait.

use std::collections::{HashMap, HashSet};

use gesco_core::bibliotheque::{
    ChapitreBibliotheque, ClasseChoisie, ContenuBibliotheque, OrigineBibliotheque,
    visible_pour_classe,
};
use gesco_core::core::Operation;
use gesco_db::contexte::Contexte;
use gesco_db::depots::bibliotheque::{self as depot, LigneClasseChoisie};
use gesco_db::unite::Durabilite;
use gesco_db::{Connexion, ErreurBase};
use serde::{Deserialize, Serialize};

use super::{ServiceBibliotheque, regle};
use crate::erreur::Resultat;
use crate::session::Session;

/// Lire « Pour qui » d'un contenu ou d'un chapitre de l'école.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandePourQui {
    /// Contenu visé.
    pub contenu_id: Option<String>,
    /// Chapitre visé.
    pub chapitre_id: Option<String>,
}

/// Régler « Pour qui » : aucune classe, tout le niveau.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeReglerPourQui {
    /// Contenu visé.
    pub contenu_id: Option<String>,
    /// Chapitre visé.
    pub chapitre_id: Option<String>,
    /// Classes choisies, de l'année en cours.
    pub classes: Vec<String>,
}

/// Une classe du niveau, cochée ou non.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ClassePourQui {
    /// Identifiant de la classe.
    pub id: String,
    /// Libellé.
    pub libelle: String,
    /// Le contenu lui est réservé.
    pub choisie: bool,
    /// L'utilisateur peut la cocher (un maître : ses classes).
    pub permise: bool,
}

/// « Pour qui » d'un contenu ou d'un chapitre de l'école.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct PourQuiContenu {
    /// Classes du niveau de l'année en cours.
    pub classes: Vec<ClassePourQui>,
    /// L'utilisateur peut changer le choix.
    pub modifiable: bool,
}

/// Ce que vise « Pour qui ».
struct CiblePourQui {
    chapitre: ChapitreBibliotheque,
    contenu: Option<ContenuBibliotheque>,
}

impl CiblePourQui {
    /// Ajout de l'école : le contenu (ou le chapitre) est de l'école.
    fn de_l_ecole(&self) -> bool {
        self.contenu
            .as_ref()
            .map_or(self.chapitre.origine, |c| c.origine)
            == OrigineBibliotheque::Ecole
    }

    /// Vrai si la ligne de choix vise cette cible.
    fn vise(&self, ligne: &LigneClasseChoisie) -> bool {
        match &self.contenu {
            Some(c) => ligne.contenu_id.as_deref() == Some(c.id.vers_chaine().as_str()),
            None => ligne.chapitre_id.as_deref() == Some(self.chapitre.id.vers_chaine().as_str()),
        }
    }
}

fn introuvable(entite: &'static str, id: &str) -> crate::ErreurService {
    ErreurBase::Introuvable {
        entite,
        id: id.to_owned(),
    }
    .into()
}

/// Choix d'une ligne, pour la règle du noyau.
fn choisie(l: &LigneClasseChoisie) -> ClasseChoisie {
    ClasseChoisie {
        classe_id: l.classe_id.clone(),
        annee_id: l.annee_id.clone(),
    }
}

impl ServiceBibliotheque<'_> {
    /// Le chapitre et, s'il est visé, le contenu.
    async fn cible_pour_qui(
        cnx: &mut Connexion,
        contenu_id: Option<&str>,
        chapitre_id: Option<&str>,
        contexte: &Contexte,
    ) -> Resultat<CiblePourQui> {
        let chapitres = depot::chapitres(cnx, contexte).await?;
        match (contenu_id, chapitre_id) {
            (Some(id), None) => {
                let contenu = depot::contenu(cnx, id, contexte)
                    .await?
                    .ok_or_else(|| introuvable("contenu", id))?;
                let chapitre = chapitres
                    .into_iter()
                    .find(|c| c.id == contenu.chapitre_id)
                    .ok_or_else(|| introuvable("chapitre", id))?;
                Ok(CiblePourQui {
                    chapitre,
                    contenu: Some(contenu),
                })
            }
            (None, Some(id)) => Ok(CiblePourQui {
                chapitre: chapitres
                    .into_iter()
                    .find(|c| c.id.vers_chaine() == id)
                    .ok_or_else(|| introuvable("chapitre", id))?,
                contenu: None,
            }),
            _ => Err(regle("commande.entree_invalide")),
        }
    }

    /// Lit « Pour qui ».
    pub async fn pour_qui(
        &self,
        session: &Session,
        demande: &DemandePourQui,
    ) -> Resultat<PourQuiContenu> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let portee = self.portee(session).await?;
        let mut cnx = self.connexion().await?;
        let cible = Self::cible_pour_qui(
            &mut cnx,
            demande.contenu_id.as_deref(),
            demande.chapitre_id.as_deref(),
            &contexte,
        )
        .await?;
        self.etat_pour_qui(&mut cnx, session, &portee, &cible, &contexte)
            .await
    }

    /// Les classes du niveau, cochées ou non, et ce que l'utilisateur peut y changer.
    async fn etat_pour_qui(
        &self,
        cnx: &mut Connexion,
        session: &Session,
        portee: &super::ecrire::Portee,
        cible: &CiblePourQui,
        contexte: &Contexte,
    ) -> Resultat<PourQuiContenu> {
        let modifiable = cible.de_l_ecole() && portee.permet_chapitre(&cible.chapitre);
        let choisies: HashSet<String> = depot::classes_choisies(cnx, contexte)
            .await?
            .into_iter()
            .filter(|l| l.courante && cible.vise(l))
            .map(|l| l.classe_id)
            .collect();
        let miennes: Option<HashSet<String>> = if portee.partout() {
            None
        } else {
            Some(
                depot::classes_du_compte(cnx, &session.utilisateur_id.vers_chaine(), contexte)
                    .await?
                    .into_iter()
                    .collect(),
            )
        };
        let classes =
            depot::classes_courantes_du_niveau(cnx, &cible.chapitre.niveau_code, contexte)
                .await?
                .into_iter()
                .map(|(id, libelle)| ClassePourQui {
                    choisie: choisies.contains(&id),
                    permise: modifiable && miennes.as_ref().is_none_or(|m| m.contains(&id)),
                    id,
                    libelle,
                })
                .collect();
        Ok(PourQuiContenu {
            classes,
            modifiable,
        })
    }

    /// Règle « Pour qui » pour l'année en cours. Un maître ne change que ses classes : les autres
    /// gardent leur état. Les choix des années passées restent, et ne comptent plus.
    pub async fn regler_pour_qui(
        &self,
        session: &Session,
        demande: &DemandeReglerPourQui,
    ) -> Resultat<PourQuiContenu> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let portee = self.portee(session).await?;
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let cible = Self::cible_pour_qui(
            unite.connexion(),
            demande.contenu_id.as_deref(),
            demande.chapitre_id.as_deref(),
            &contexte,
        )
        .await?;
        if !cible.de_l_ecole() {
            return Err(regle("bibliotheque.pour_qui_livre"));
        }
        let etat = self
            .etat_pour_qui(unite.connexion(), session, &portee, &cible, &contexte)
            .await?;
        if !etat.modifiable {
            return Err(crate::ErreurService::PorteeRefusee {
                ressource: format!("chapitre {}", cible.chapitre.id.vers_chaine()),
            });
        }
        let demandees: HashSet<&str> = demande.classes.iter().map(String::as_str).collect();
        if demandees
            .iter()
            .any(|d| !etat.classes.iter().any(|c| c.id == *d))
        {
            return Err(regle("bibliotheque.pour_qui_classe_inconnue"));
        }
        let voulues: HashSet<&str> = etat
            .classes
            .iter()
            .filter(|c| {
                if c.permise {
                    demandees.contains(c.id.as_str())
                } else {
                    c.choisie
                }
            })
            .map(|c| c.id.as_str())
            .collect();
        let lignes: Vec<LigneClasseChoisie> = depot::classes_choisies(unite.connexion(), &contexte)
            .await?
            .into_iter()
            .filter(|l| l.courante && cible.vise(l))
            .collect();
        let avant: Vec<&str> = lignes.iter().map(|l| l.libelle.as_str()).collect();
        let mut change = false;
        for l in &lignes {
            if !voulues.contains(l.classe_id.as_str()) {
                depot::retirer_classe_choisie(unite.connexion(), &l.id, &contexte).await?;
                change = true;
            }
        }
        let contenu_id = cible.contenu.as_ref().map(|c| c.id.vers_chaine());
        let chapitre_id = cible
            .contenu
            .is_none()
            .then(|| cible.chapitre.id.vers_chaine());
        for classe in &voulues {
            if !lignes.iter().any(|l| l.classe_id == *classe) {
                depot::choisir_classe(
                    unite.connexion(),
                    contenu_id.as_deref(),
                    chapitre_id.as_deref(),
                    classe,
                    &contexte,
                )
                .await?;
                change = true;
            }
        }
        if change {
            let apres: Vec<&str> = etat
                .classes
                .iter()
                .filter(|c| voulues.contains(c.id.as_str()))
                .map(|c| c.libelle.as_str())
                .collect();
            let (table, id) = match (&contenu_id, &chapitre_id) {
                (Some(id), _) => ("contenu", id.clone()),
                (None, Some(id)) => ("chapitre_programme", id.clone()),
                (None, None) => ("chapitre_programme", String::new()),
            };
            unite.auditer(
                table,
                &id,
                Operation::Modification,
                Some(serde_json::json!({ "classes": avant })),
                Some(serde_json::json!({ "classes": apres })),
                "bibliotheque.classes_regler",
            );
        }
        let etat = self
            .etat_pour_qui(unite.connexion(), session, &portee, &cible, &contexte)
            .await?;
        self.noyau.terminer(unite).await?;
        Ok(etat)
    }

    /// Libellés des classes de l'année en cours auxquelles chaque chapitre ou contenu de l'école
    /// est réservé, par identifiant : la marque « 9e A seulement ».
    pub(super) async fn marques_classes(
        cnx: &mut Connexion,
        contexte: &Contexte,
    ) -> Resultat<HashMap<String, Vec<String>>> {
        let mut marques: HashMap<String, Vec<String>> = HashMap::new();
        for l in depot::classes_choisies(cnx, contexte).await? {
            if !l.courante {
                continue;
            }
            if let Some(id) = l.contenu_id.or(l.chapitre_id) {
                marques.entry(id).or_default().push(l.libelle);
            }
        }
        Ok(marques)
    }

    /// « Visible pour une classe » : vrai si le contenu se montre aux élèves de cette classe,
    /// d'après le choix de classes du contenu et de son chapitre, pour l'année de la classe. Le
    /// portail de l'élève (lot 4) l'appelle ; le reste de la visibilité (publication, retraits)
    /// se décide ailleurs. Un contenu livré est toujours pour tout le niveau.
    pub async fn visible_pour_classe(
        &self,
        session: &Session,
        contenu_id: &str,
        classe_id: &str,
    ) -> Resultat<bool> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let cible = Self::cible_pour_qui(&mut cnx, Some(contenu_id), None, &contexte).await?;
        let Some(annee_id) = depot::annee_de_la_classe(&mut cnx, classe_id, &contexte).await?
        else {
            return Ok(false);
        };
        let lignes = depot::classes_choisies(&mut cnx, &contexte).await?;
        let du_contenu: Vec<ClasseChoisie> = lignes
            .iter()
            .filter(|l| l.contenu_id.as_deref() == Some(contenu_id))
            .map(choisie)
            .collect();
        let du_chapitre: Vec<ClasseChoisie> =
            if cible.chapitre.origine == OrigineBibliotheque::Ecole {
                let chapitre_id = cible.chapitre.id.vers_chaine();
                lignes
                    .iter()
                    .filter(|l| l.chapitre_id.as_deref() == Some(chapitre_id.as_str()))
                    .map(choisie)
                    .collect()
            } else {
                Vec::new()
            };
        if cible.contenu.as_ref().map(|c| c.origine) == Some(OrigineBibliotheque::Livre) {
            return Ok(true);
        }
        Ok(visible_pour_classe(
            &du_contenu,
            &du_chapitre,
            classe_id,
            &annee_id,
        ))
    }
}
