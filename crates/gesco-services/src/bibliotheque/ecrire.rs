//! Écrire dans la bibliothèque (§7 et §8 de la spécification) : chapitres, leçons, sujets et
//! documents de l'école ; le brouillon unique d'un contenu, son compteur de révision et son
//! auteur ; l'historique des versions, la comparaison ligne à ligne, repartir d'une version et
//! revenir à la version livrée. Le livré n'est jamais touché : une version de l'école part de lui.
//!
//! Un maître écrit dans les matières et les niveaux de ses affectations de l'année en cours, à
//! travers la correspondance des disciplines ; les autres comptes qui écrivent, partout.

use std::collections::{HashMap, HashSet};

use gesco_core::bibliotheque::paquet::empreinte;
use gesco_core::bibliotheque::{
    ChapitreBibliotheque, ContenuBibliotheque, EtatVersion, ExamenSujet, Lecon, NatureContenu,
    OrigineBibliotheque, PLAN_LECON, RangeeComparaison, TEXTE_MAX, analyser, comparer_lignes,
    images_a_ajouter,
};
use gesco_core::core::{Operation, TypeEvenement};
use gesco_db::ErreurBase;
use gesco_db::contexte::Contexte;
use gesco_db::depots::bibliotheque::{self as depot, LigneVersion};
use gesco_db::depots::{academique, pedagogie};
use gesco_db::unite::Durabilite;
use gesco_db::{Connexion, nouvel_id};
use serde::{Deserialize, Serialize};

use super::lire::ImageLecon;
use super::retirer::MatiereChoisissable;
use super::{ServiceBibliotheque, regle};
use crate::erreur::{ErreurService, Resultat};
use crate::session::Session;

/// Titre d'un chapitre de l'école qui reçoit les sujets d'un examen.
fn titre_chapitre_sujets(examen: ExamenSujet) -> &'static str {
    match examen {
        ExamenSujet::Def => "Sujets du DEF",
        ExamenSujet::Bac => "Sujets du BAC",
    }
}

/// Partie des chapitres de sujets.
const PARTIE_EXAMENS: &str = "Examens";

/// Désigner un contenu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeContenuBibliotheque {
    /// Identifiant du contenu.
    pub contenu_id: String,
}

/// Le brouillon en cours d'un contenu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct BrouillonOuvert {
    /// Identifiant de la version.
    pub version_id: String,
    /// Numéro de la version.
    pub numero: u32,
    /// Texte.
    pub texte: String,
    /// Révision lue : l'enregistrement la rend, pour qu'aucun autre ne soit écrasé.
    pub revision: u32,
    /// Nom de l'auteur.
    pub auteur: String,
    /// L'utilisateur en est l'auteur.
    pub mien: bool,
    /// « Revenir à la version livrée » : aucun texte à écrire.
    pub retour_livre: bool,
    /// Dernier enregistrement, millisecondes depuis l'époque.
    pub enregistre_le: i64,
    /// Le renvoi d'où vient ce brouillon, s'il vient d'une version renvoyée.
    pub renvoi: Option<RenvoiVersion>,
}

/// Le renvoi d'une version par la direction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct RenvoiVersion {
    /// Commentaire.
    pub commentaire: String,
    /// Nom de qui a renvoyé.
    pub par: String,
    /// Quand, millisecondes depuis l'époque.
    pub le: i64,
}

/// La version soumise d'un contenu, en attente de validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct VersionSoumise {
    /// Identifiant de la version.
    pub version_id: String,
    /// Numéro de la version.
    pub numero: u32,
    /// Texte.
    pub texte: String,
    /// Nom de l'auteur.
    pub auteur: String,
    /// L'utilisateur en est l'auteur.
    pub mien: bool,
    /// « Revenir à la version livrée ».
    pub retour_livre: bool,
    /// Soumise le, millisecondes depuis l'époque.
    pub soumise_le: i64,
}

/// Un contenu ouvert pour l'écriture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct EcritureOuverte {
    /// Identifiant du contenu.
    pub contenu_id: String,
    /// Titre.
    pub titre: String,
    /// Nature.
    pub nature: NatureContenu,
    /// Origine.
    pub origine: OrigineBibliotheque,
    /// L'utilisateur peut écrire ce contenu (ses matières pour un maître).
    pub peut_ecrire: bool,
    /// L'utilisateur valide (`library:publish`).
    pub peut_valider: bool,
    /// L'utilisateur publie son brouillon sans le soumettre : il valide, ou la direction lui a
    /// donné le droit de publier et le contenu est dans ses matières.
    pub peut_publier: bool,
    /// Le brouillon en cours, s'il y en a un.
    pub brouillon: Option<BrouillonOuvert>,
    /// La version soumise, en attente de validation ; montrée à qui valide et à qui écrit ce
    /// contenu.
    pub soumise: Option<VersionSoumise>,
    /// Texte d'où partirait un brouillon nouveau : la version visible, sinon le plan d'une leçon.
    pub texte_depart: String,
    /// Le contenu a une version livrée lisible sur ce poste.
    pub a_une_version_livree: bool,
    /// La version visible est celle de l'école.
    pub version_ecole: bool,
    /// La bibliothèque a une version plus récente que celle d'où la version de l'école est
    /// partie.
    pub livree_plus_recente: bool,
}

/// D'où part un brouillon nouveau.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DepartBrouillon {
    /// La version visible (le plan pour une leçon de l'école qui n'en a pas).
    Visible,
    /// La version livrée, telle que le paquet installé la donne.
    Livree,
    /// Une version de l'école, ancienne ou non (« Repartir de cette version »).
    Version {
        /// Identifiant de la version.
        version_id: String,
    },
    /// Une version reçue d'une autre école (« Repartir de cette version »).
    Recue {
        /// Identifiant de la version reçue.
        version_recue_id: String,
    },
}

/// Commencer un brouillon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeBrouillonCommencer {
    /// Identifiant du contenu.
    pub contenu_id: String,
    /// D'où part le brouillon.
    pub depart: DepartBrouillon,
}

/// Enregistrer un brouillon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeBrouillon {
    /// Identifiant du contenu.
    pub contenu_id: String,
    /// Identifiant du brouillon ouvert.
    pub version_id: String,
    /// Révision lue à l'ouverture ou au dernier enregistrement.
    pub revision: u32,
    /// Texte.
    pub texte: String,
}

/// Reprendre ou abandonner un brouillon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeBrouillonVersion {
    /// Identifiant du contenu.
    pub contenu_id: String,
    /// Identifiant du brouillon.
    pub version_id: String,
}

/// Aperçu d'un texte dans l'éditeur.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeApercuLecon {
    /// Identifiant du contenu (pour ses images livrées).
    pub contenu_id: String,
    /// Texte tapé.
    pub texte: String,
}

/// L'aperçu, rendu comme la lecture, avec les notes de l'auteur.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ApercuLecon {
    /// Le texte, en arbre de blocs.
    pub lecon: Lecon,
    /// Images appelées.
    pub images: Vec<ImageLecon>,
    /// Formules que l'impression montrera telles qu'elles sont écrites.
    pub formules_brutes: Vec<String>,
    /// Images que les notes de la bibliothèque demandent d'ajouter.
    pub images_a_ajouter: Vec<String>,
    /// Sons appelés et leur présence dans la base.
    pub sons: Vec<super::SonLecon>,
    /// Le quiz tel que l'élève le passera, et les questions écartées (pour l'auteur seul).
    pub quiz: gesco_core::bibliotheque::Quiz,
}

/// Ajouter un chapitre de l'école.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeChapitreBibliotheque {
    /// Niveau de l'école.
    pub niveau_id: String,
    /// Matière de l'école.
    pub matiere_id: String,
    /// Séries du lycée ; aucune : toutes.
    pub series: Vec<String>,
    /// Partie, choisie parmi celles de la matière ou nouvelle ; vide si aucune.
    pub partie: String,
    /// Titre.
    pub titre: String,
    /// Clé du chapitre après lequel il se range ; aucune : en tête.
    pub apres_cle: Option<String>,
}

/// Un chapitre ajouté.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ChapitreAjoute {
    /// Identifiant.
    pub id: String,
    /// Clé.
    pub cle: String,
}

/// Ajouter une leçon, un sujet, un corrigé ou un document de l'école.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeAjoutContenu {
    /// Nature.
    pub nature: NatureContenu,
    /// Chapitre ; un sujet peut ne pas en donner : il va sous « Sujets du DEF » ou « Sujets du
    /// BAC » de son niveau et de sa matière, créé au besoin.
    pub chapitre_id: Option<String>,
    /// Niveau d'un sujet donné sans chapitre.
    pub niveau_id: Option<String>,
    /// Matière d'un sujet donné sans chapitre.
    pub matiere_id: Option<String>,
    /// Titre.
    pub titre: String,
    /// Phrase de présentation d'un document.
    pub presentation: String,
    /// Examen d'un sujet.
    pub examen: Option<ExamenSujet>,
    /// Série d'un sujet du lycée.
    pub serie_code: Option<String>,
    /// Session d'un sujet.
    pub session: Option<u16>,
    /// Le sujet d'un corrigé.
    pub sujet_id: Option<String>,
    /// Clé du contenu après lequel il se range ; aucune : en tête.
    pub apres_cle: Option<String>,
}

/// Une version, telle que l'historique la montre.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct VersionAffichee {
    /// Identifiant.
    pub id: String,
    /// Numéro.
    pub numero: u32,
    /// État.
    pub etat: EtatVersion,
    /// « Revenir à la version livrée ».
    pub retour_livre: bool,
    /// Nom de l'auteur.
    pub auteur: String,
    /// Créée le, millisecondes depuis l'époque.
    pub cree_le: i64,
    /// Dernier enregistrement.
    pub enregistre_le: i64,
    /// Soumise le.
    pub soumise_le: Option<i64>,
    /// Publiée, renvoyée ou abandonnée le.
    pub decidee_le: Option<i64>,
    /// Qui a décidé.
    pub decidee_par: Option<String>,
    /// Commentaire d'un renvoi.
    pub commentaire: String,
    /// Numéro de la version de l'école d'où elle est partie.
    pub depart_numero: Option<u32>,
    /// Elle est partie de la version livrée.
    pub depart_livree: bool,
}

/// Ce qui est arrivé à un contenu hors de ses versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum SorteMouvement {
    /// Retiré par la direction.
    Retrait,
    /// Remis.
    Remise,
}

/// Un retrait ou une remise du contenu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct MouvementContenu {
    /// Sorte.
    pub sorte: SorteMouvement,
    /// Quand, millisecondes depuis l'époque.
    pub quand: i64,
    /// Qui.
    pub qui: String,
}

/// Une version reçue d'une autre école alors qu'un brouillon existait.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct VersionRecueAffichee {
    /// Identifiant.
    pub id: String,
    /// Reçue le, millisecondes depuis l'époque.
    pub recue_le: i64,
    /// Nom de qui a importé le fichier.
    pub importe_par: String,
}

/// L'historique d'un contenu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct HistoriqueContenu {
    /// Identifiant du contenu.
    pub contenu_id: String,
    /// Titre.
    pub titre: String,
    /// Versions, les plus récentes d'abord.
    pub versions: Vec<VersionAffichee>,
    /// Retraits et remises, les plus récents d'abord.
    pub mouvements: Vec<MouvementContenu>,
    /// Versions reçues d'une autre école, les plus récentes d'abord.
    pub recues: Vec<VersionRecueAffichee>,
    /// Le contenu a une version livrée lisible sur ce poste.
    pub a_une_version_livree: bool,
    /// La bibliothèque a une version plus récente que celle d'où la version de l'école est
    /// partie.
    pub livree_plus_recente: bool,
    /// L'utilisateur peut écrire ce contenu.
    pub peut_ecrire: bool,
}

/// Un des deux textes d'une comparaison.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TexteCompare {
    /// La version livrée.
    Livree,
    /// La version visible.
    Visible,
    /// Une version de l'école.
    Version {
        /// Identifiant de la version.
        version_id: String,
    },
    /// Une version reçue d'une autre école.
    Recue {
        /// Identifiant de la version reçue.
        version_recue_id: String,
    },
}

/// Comparer deux versions d'un contenu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeComparaison {
    /// Identifiant du contenu.
    pub contenu_id: String,
    /// Texte de gauche.
    pub gauche: TexteCompare,
    /// Texte de droite.
    pub droite: TexteCompare,
}

/// Deux textes comparés côte à côte.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ComparaisonTextes {
    /// Rangées, dans l'ordre des textes.
    pub rangees: Vec<RangeeComparaison>,
    /// Nombre de lignes retirées ou changées à gauche.
    pub retirees: u32,
    /// Nombre de lignes ajoutées ou changées à droite.
    pub ajoutees: u32,
}

/// Une série d'un niveau, pour les formulaires d'ajout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct SerieChoisissable {
    /// Code.
    pub code: String,
    /// Libellé.
    pub libelle: String,
}

/// Un niveau de l'école, pour les formulaires d'ajout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct NiveauChoisissable {
    /// Identifiant.
    pub id: String,
    /// Code.
    pub code: String,
    /// Libellé.
    pub libelle: String,
    /// Séries actives du niveau.
    pub series: Vec<SerieChoisissable>,
}

/// Une matière d'un niveau où l'utilisateur écrit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct PlaceEcriture {
    /// Matière de l'école.
    pub matiere_id: String,
    /// Code du niveau.
    pub niveau_code: String,
}

/// Ce qu'on peut choisir en ajoutant un chapitre ou un sujet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ChoixEcriture {
    /// Niveaux actifs de l'école.
    pub niveaux: Vec<NiveauChoisissable>,
    /// Matières actives de l'école.
    pub matieres: Vec<MatiereChoisissable>,
    /// L'utilisateur écrit partout ; sinon, dans `places` seulement.
    pub partout: bool,
    /// Matières et niveaux d'un maître.
    pub places: Vec<PlaceEcriture>,
}

/// Où l'utilisateur peut écrire.
pub(super) struct Portee {
    /// Compte de maître : il n'écrit que dans ses matières.
    maitre: bool,
    /// (matière, code du niveau) de ses affectations de l'année en cours.
    matieres: HashSet<(String, String)>,
    /// (discipline, code du niveau) → matière de l'école.
    correspondances: HashMap<(String, String), String>,
}

impl Portee {
    /// Matière de l'école sous laquelle se range un chapitre.
    fn matiere_du_chapitre(&self, c: &ChapitreBibliotheque) -> Option<String> {
        match c.origine {
            OrigineBibliotheque::Ecole => c.matiere_id.clone(),
            OrigineBibliotheque::Livre => self
                .correspondances
                .get(&(
                    c.discipline_code.clone().unwrap_or_default(),
                    c.niveau_code.clone(),
                ))
                .cloned(),
        }
    }

    /// Vrai si l'utilisateur écrit dans ce chapitre.
    pub(super) fn permet_chapitre(&self, c: &ChapitreBibliotheque) -> bool {
        !self.maitre
            || self
                .matiere_du_chapitre(c)
                .is_some_and(|m| self.matieres.contains(&(m, c.niveau_code.clone())))
    }

    /// Vrai si l'utilisateur écrit dans cette matière de ce niveau.
    fn permet(&self, matiere_id: &str, niveau_code: &str) -> bool {
        !self.maitre
            || self
                .matieres
                .contains(&(matiere_id.to_owned(), niveau_code.to_owned()))
    }

    /// Vrai si l'utilisateur écrit partout.
    pub(super) fn partout(&self) -> bool {
        !self.maitre
    }
}

fn introuvable(entite: &'static str, id: &str) -> ErreurService {
    ErreurBase::Introuvable {
        entite,
        id: id.to_owned(),
    }
    .into()
}

fn hors_portee(id: &str) -> ErreurService {
    ErreurService::PorteeRefusee {
        ressource: format!("contenu {id}"),
    }
}

pub(super) fn nombre(n: i64) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn titre_valide(titre: &str) -> Resultat<String> {
    let titre = titre.trim();
    if titre.is_empty() || titre.chars().count() > 200 {
        return Err(regle("bibliotheque.titre_invalide"));
    }
    Ok(titre.to_owned())
}

impl ServiceBibliotheque<'_> {
    /// Où l'utilisateur peut écrire.
    pub(super) async fn portee(&self, session: &Session) -> Resultat<Portee> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let maitre = self
            .noyau
            .droits_de(session.utilisateur_id, &contexte)
            .await?
            .maitre;
        let mut cnx = self.connexion().await?;
        let matieres = if maitre {
            depot::matieres_du_compte(&mut cnx, &session.utilisateur_id.vers_chaine(), &contexte)
                .await?
                .into_iter()
                .collect()
        } else {
            HashSet::new()
        };
        let correspondances = depot::correspondances(&mut cnx, &contexte)
            .await?
            .into_iter()
            .filter_map(|(d, n, m)| Some(((d, n), m?)))
            .collect();
        Ok(Portee {
            maitre,
            matieres,
            correspondances,
        })
    }

    /// Le contenu, son chapitre, et vrai si l'utilisateur peut l'écrire.
    pub(super) async fn contenu_et_portee(
        cnx: &mut Connexion,
        portee: &Portee,
        contenu_id: &str,
        contexte: &Contexte,
    ) -> Resultat<(ContenuBibliotheque, bool)> {
        let contenu = depot::contenu(cnx, contenu_id, contexte)
            .await?
            .ok_or_else(|| introuvable("contenu", contenu_id))?;
        let chapitre = depot::chapitres(cnx, contexte)
            .await?
            .into_iter()
            .find(|c| c.id == contenu.chapitre_id)
            .ok_or_else(|| introuvable("chapitre", &contenu.chapitre_id.vers_chaine()))?;
        Ok((contenu, portee.permet_chapitre(&chapitre)))
    }

    /// Le contenu, s'il est dans la portée de l'utilisateur ; sinon un refus.
    pub(super) async fn contenu_permis(
        cnx: &mut Connexion,
        portee: &Portee,
        contenu_id: &str,
        contexte: &Contexte,
    ) -> Resultat<ContenuBibliotheque> {
        let (contenu, permis) = Self::contenu_et_portee(cnx, portee, contenu_id, contexte).await?;
        if !permis {
            return Err(hors_portee(contenu_id));
        }
        Ok(contenu)
    }

    /// La version livrée plus récente que celle d'où la version de l'école est partie.
    fn livree_plus_recente(contenu: &ContenuBibliotheque, versions: &[LigneVersion]) -> bool {
        let Some(actuelle) = contenu.empreinte.as_deref() else {
            return false;
        };
        versions
            .iter()
            .filter(|v| v.etat == "brouillon" || (v.etat == "publiee" && v.retour_livre == 0))
            .filter_map(|v| v.depart_livre_sha256.as_deref())
            .any(|depart| depart != actuelle)
    }

    /// Le texte visible et la version publiée non « retour » d'où il vient, s'il y en a une.
    pub(super) fn texte_visible<'v>(
        &self,
        contenu: &ContenuBibliotheque,
        versions: &'v [LigneVersion],
    ) -> (Option<String>, Option<&'v LigneVersion>) {
        match versions
            .iter()
            .find(|v| v.etat == "publiee" && v.retour_livre == 0)
        {
            Some(v) => (Some(v.texte.clone()), Some(v)),
            None => (self.texte_livre(contenu), None),
        }
    }

    /// Ouvre un contenu pour l'écriture : son brouillon, sinon le texte d'où partir.
    pub async fn ecriture(
        &self,
        session: &Session,
        demande: &DemandeContenuBibliotheque,
    ) -> Resultat<EcritureOuverte> {
        let (_, peut_valider) = self.permissions(session).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let portee = self.portee(session).await?;
        let mut cnx = self.connexion().await?;
        let (contenu, peut_ecrire) =
            Self::contenu_et_portee(&mut cnx, &portee, &demande.contenu_id, &contexte).await?;
        let versions = depot::versions(&mut cnx, &demande.contenu_id, &contexte).await?;
        let moi = session.utilisateur_id.vers_chaine();
        let peut_publier = peut_valider
            || (peut_ecrire && depot::a_le_droit_de_publier(&mut cnx, &moi, &contexte).await?);
        let brouillon = versions
            .iter()
            .find(|v| v.etat == "brouillon")
            .map(|v| BrouillonOuvert {
                version_id: v.id.clone(),
                numero: nombre(v.numero),
                texte: v.texte.clone(),
                revision: nombre(v.revision),
                auteur: v.auteur.clone(),
                mien: v.auteur_id == moi,
                retour_livre: v.retour_livre != 0,
                enregistre_le: v.mis_a_jour_le,
                renvoi: v
                    .depart_version_id
                    .as_deref()
                    .and_then(|d| versions.iter().find(|r| r.id == d))
                    .filter(|r| r.etat == "renvoyee")
                    .map(|r| RenvoiVersion {
                        commentaire: r.commentaire.clone(),
                        par: r.decidee_par.clone().unwrap_or_default(),
                        le: r.decidee_le.unwrap_or(r.mis_a_jour_le),
                    }),
            });
        let soumise = versions
            .iter()
            .find(|v| v.etat == "soumise")
            .filter(|_| peut_ecrire || peut_valider)
            .map(|v| VersionSoumise {
                version_id: v.id.clone(),
                numero: nombre(v.numero),
                texte: v.texte.clone(),
                auteur: v.auteur.clone(),
                mien: v.auteur_id == moi,
                retour_livre: v.retour_livre != 0,
                soumise_le: v.soumise_le.unwrap_or(v.mis_a_jour_le),
            });
        let (visible, publiee) = self.texte_visible(&contenu, &versions);
        let texte_depart = match visible {
            Some(t) => t,
            None if contenu.nature == NatureContenu::Lecon => PLAN_LECON.to_owned(),
            None => String::new(),
        };
        Ok(EcritureOuverte {
            contenu_id: contenu.id.vers_chaine(),
            titre: contenu.titre.clone(),
            nature: contenu.nature,
            origine: contenu.origine,
            peut_ecrire,
            peut_valider,
            peut_publier,
            brouillon,
            soumise,
            texte_depart,
            a_une_version_livree: self.texte_livre(&contenu).is_some(),
            version_ecole: publiee.is_some(),
            livree_plus_recente: Self::livree_plus_recente(&contenu, &versions),
        })
    }

    /// Écrit un brouillon nouveau dans une unité de travail, avec son audit et son événement.
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn creer_brouillon(
        &self,
        unite: &mut gesco_db::UniteDeTravail<crate::effets::Effet>,
        session: &Session,
        contenu_id: &str,
        texte: &str,
        retour_livre: bool,
        depart_version_id: Option<&str>,
        depart_livre_sha256: Option<&str>,
        origine: &str,
    ) -> Resultat<String> {
        let empreinte_texte = empreinte(texte.as_bytes());
        let auteur = session.utilisateur_id.vers_chaine();
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let (id, numero) = depot::inserer_brouillon(
            unite.connexion(),
            &depot::NouvelleVersion {
                contenu_id,
                texte,
                empreinte: &empreinte_texte,
                retour_livre,
                depart_version_id,
                depart_livre_sha256,
                auteur_id: &auteur,
            },
            &contexte,
        )
        .await?;
        let apres = serde_json::json!({
            "contenu_id": contenu_id,
            "numero": numero,
            "etat": EtatVersion::Brouillon.nom(),
            "retour_livre": retour_livre,
            "depart_version_id": depart_version_id,
            "depart_livree": depart_livre_sha256.is_some() && depart_version_id.is_none(),
            "texte": texte,
        });
        unite.auditer(
            "version_contenu",
            &id,
            Operation::Creation,
            None,
            Some(apres),
            origine,
        );
        unite.emettre(
            TypeEvenement::VersionContenuModifiee,
            "version_contenu",
            &id,
            serde_json::json!({
                "contenu_id": contenu_id,
                "numero": numero,
                "etat": EtatVersion::Brouillon.nom(),
            }),
        );
        Ok(id)
    }

    /// Commence un brouillon : depuis la version visible, la version livrée ou une version de
    /// l'école. Refusé si un brouillon existe déjà (un seul à la fois).
    pub async fn commencer_brouillon(
        &self,
        session: &Session,
        demande: &DemandeBrouillonCommencer,
    ) -> Resultat<EcritureOuverte> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let portee = self.portee(session).await?;
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let contenu =
            Self::contenu_permis(unite.connexion(), &portee, &demande.contenu_id, &contexte)
                .await?;
        let versions = depot::versions(unite.connexion(), &demande.contenu_id, &contexte).await?;
        if versions.iter().any(|v| v.etat == "brouillon") {
            return Err(regle("bibliotheque.brouillon_existe"));
        }
        if versions.iter().any(|v| v.etat == "soumise") {
            return Err(regle("bibliotheque.en_attente_de_validation"));
        }
        let livree = contenu.empreinte.clone();
        let (texte, depart_version, depart_livre) = match &demande.depart {
            DepartBrouillon::Visible => match self.texte_visible(&contenu, &versions) {
                (Some(t), Some(v)) => (t, Some(v.id.clone()), v.depart_livre_sha256.clone()),
                (Some(t), None) => (t, None, livree),
                (None, _) if contenu.nature == NatureContenu::Lecon => {
                    (PLAN_LECON.to_owned(), None, None)
                }
                (None, _) => (String::new(), None, None),
            },
            DepartBrouillon::Livree => (
                self.texte_livre(&contenu)
                    .ok_or(regle("bibliotheque.sans_version_livree"))?,
                None,
                livree,
            ),
            DepartBrouillon::Version { version_id } => {
                let v = versions
                    .iter()
                    .find(|v| &v.id == version_id)
                    .ok_or_else(|| introuvable("version_contenu", version_id))?;
                if v.retour_livre != 0 {
                    (
                        self.texte_livre(&contenu)
                            .ok_or(regle("bibliotheque.sans_version_livree"))?,
                        None,
                        livree,
                    )
                } else {
                    (
                        v.texte.clone(),
                        Some(v.id.clone()),
                        v.depart_livre_sha256.clone(),
                    )
                }
            }
            DepartBrouillon::Recue { version_recue_id } => {
                let r = depot::versions_recues(unite.connexion(), &demande.contenu_id, &contexte)
                    .await?
                    .into_iter()
                    .find(|r| &r.id == version_recue_id)
                    .ok_or_else(|| introuvable("version_recue", version_recue_id))?;
                (r.texte, None, livree)
            }
        };
        self.creer_brouillon(
            &mut unite,
            session,
            &demande.contenu_id,
            &texte,
            false,
            depart_version.as_deref(),
            depart_livre.as_deref(),
            "bibliotheque.brouillon_commencer",
        )
        .await?;
        self.noyau.terminer(unite).await?;
        self.ecriture(
            session,
            &DemandeContenuBibliotheque {
                contenu_id: demande.contenu_id.clone(),
            },
        )
        .await
    }

    /// Le brouillon ouvert, s'il est toujours celui que l'éditeur montre.
    pub(super) async fn brouillon_attendu(
        cnx: &mut Connexion,
        contenu_id: &str,
        version_id: &str,
        contexte: &Contexte,
    ) -> Resultat<LigneVersion> {
        depot::versions(cnx, contenu_id, contexte)
            .await?
            .into_iter()
            .find(|v| v.etat == "brouillon" && v.id == version_id)
            .ok_or(regle("bibliotheque.modifiee_ailleurs"))
    }

    /// Enregistre le brouillon. Refusé s'il a été enregistré ailleurs depuis sa lecture, ou s'il
    /// appartient à un autre auteur. Chaque enregistrement s'écrit au journal, avant et après.
    pub async fn enregistrer_brouillon(
        &self,
        session: &Session,
        demande: &DemandeBrouillon,
    ) -> Resultat<EcritureOuverte> {
        if demande.texte.len() > TEXTE_MAX {
            return Err(regle("bibliotheque.texte_trop_long"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let portee = self.portee(session).await?;
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        Self::contenu_permis(unite.connexion(), &portee, &demande.contenu_id, &contexte).await?;
        let avant = Self::brouillon_attendu(
            unite.connexion(),
            &demande.contenu_id,
            &demande.version_id,
            &contexte,
        )
        .await?;
        if nombre(avant.revision) != demande.revision {
            return Err(regle("bibliotheque.modifiee_ailleurs"));
        }
        if avant.auteur_id != session.utilisateur_id.vers_chaine() {
            return Err(regle("bibliotheque.brouillon_d_un_autre"));
        }
        if avant.retour_livre != 0 {
            return Err(regle("bibliotheque.retour_sans_texte"));
        }
        let empreinte_texte = empreinte(demande.texte.as_bytes());
        if !depot::reecrire_brouillon(
            unite.connexion(),
            &demande.version_id,
            avant.revision,
            &demande.texte,
            &empreinte_texte,
            &contexte,
        )
        .await?
        {
            return Err(regle("bibliotheque.modifiee_ailleurs"));
        }
        unite.auditer(
            "version_contenu",
            &demande.version_id,
            Operation::Modification,
            Some(serde_json::json!({ "revision": avant.revision, "texte": avant.texte })),
            Some(serde_json::json!({ "revision": avant.revision + 1, "texte": demande.texte })),
            "bibliotheque.brouillon_enregistrer",
        );
        unite.emettre(
            TypeEvenement::VersionContenuModifiee,
            "version_contenu",
            &demande.version_id,
            serde_json::json!({
                "contenu_id": demande.contenu_id,
                "numero": avant.numero,
                "etat": EtatVersion::Brouillon.nom(),
            }),
        );
        self.noyau.terminer(unite).await?;
        self.ecriture(
            session,
            &DemandeContenuBibliotheque {
                contenu_id: demande.contenu_id.clone(),
            },
        )
        .await
    }

    /// Reprend le brouillon d'un autre auteur de la même matière ; s'écrit au journal.
    pub async fn reprendre_brouillon(
        &self,
        session: &Session,
        demande: &DemandeBrouillonVersion,
    ) -> Resultat<EcritureOuverte> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let portee = self.portee(session).await?;
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        Self::contenu_permis(unite.connexion(), &portee, &demande.contenu_id, &contexte).await?;
        let avant = Self::brouillon_attendu(
            unite.connexion(),
            &demande.contenu_id,
            &demande.version_id,
            &contexte,
        )
        .await?;
        let moi = session.utilisateur_id.vers_chaine();
        if avant.auteur_id != moi {
            if !depot::reprendre_brouillon(unite.connexion(), &demande.version_id, &moi, &contexte)
                .await?
            {
                return Err(regle("bibliotheque.modifiee_ailleurs"));
            }
            unite.auditer(
                "version_contenu",
                &demande.version_id,
                Operation::Modification,
                Some(serde_json::json!({ "auteur_id": avant.auteur_id })),
                Some(serde_json::json!({ "auteur_id": moi })),
                "bibliotheque.brouillon_reprendre",
            );
            unite.emettre(
                TypeEvenement::VersionContenuModifiee,
                "version_contenu",
                &demande.version_id,
                serde_json::json!({
                    "contenu_id": demande.contenu_id,
                    "numero": avant.numero,
                    "etat": EtatVersion::Brouillon.nom(),
                    "repris": true,
                }),
            );
        }
        self.noyau.terminer(unite).await?;
        self.ecriture(
            session,
            &DemandeContenuBibliotheque {
                contenu_id: demande.contenu_id.clone(),
            },
        )
        .await
    }

    /// Abandonne le brouillon : il passe à « abandonnée » et reste dans l'historique. Son auteur
    /// l'abandonne, ou qui valide.
    pub async fn abandonner_brouillon(
        &self,
        session: &Session,
        demande: &DemandeBrouillonVersion,
    ) -> Resultat<EcritureOuverte> {
        let (_, peut_valider) = self.permissions(session).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let portee = self.portee(session).await?;
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        Self::contenu_permis(unite.connexion(), &portee, &demande.contenu_id, &contexte).await?;
        let avant = Self::brouillon_attendu(
            unite.connexion(),
            &demande.contenu_id,
            &demande.version_id,
            &contexte,
        )
        .await?;
        if avant.auteur_id != session.utilisateur_id.vers_chaine() && !peut_valider {
            return Err(regle("bibliotheque.brouillon_d_un_autre"));
        }
        if !depot::abandonner_brouillon(unite.connexion(), &demande.version_id, &contexte).await? {
            return Err(regle("bibliotheque.modifiee_ailleurs"));
        }
        unite.auditer(
            "version_contenu",
            &demande.version_id,
            Operation::Modification,
            Some(serde_json::json!({ "etat": EtatVersion::Brouillon.nom() })),
            Some(serde_json::json!({ "etat": EtatVersion::Abandonnee.nom() })),
            "bibliotheque.brouillon_abandonner",
        );
        unite.emettre(
            TypeEvenement::VersionContenuModifiee,
            "version_contenu",
            &demande.version_id,
            serde_json::json!({
                "contenu_id": demande.contenu_id,
                "numero": avant.numero,
                "etat": EtatVersion::Abandonnee.nom(),
            }),
        );
        self.noyau.terminer(unite).await?;
        self.ecriture(
            session,
            &DemandeContenuBibliotheque {
                contenu_id: demande.contenu_id.clone(),
            },
        )
        .await
    }

    /// « Revenir à la version livrée » : une version sans texte, qui suit le même circuit ;
    /// publiée, elle rend visible le texte livré.
    pub async fn retour_livree(
        &self,
        session: &Session,
        demande: &DemandeContenuBibliotheque,
    ) -> Resultat<EcritureOuverte> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let portee = self.portee(session).await?;
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let contenu =
            Self::contenu_permis(unite.connexion(), &portee, &demande.contenu_id, &contexte)
                .await?;
        if contenu.origine != OrigineBibliotheque::Livre {
            return Err(regle("bibliotheque.sans_version_livree"));
        }
        let versions = depot::versions(unite.connexion(), &demande.contenu_id, &contexte).await?;
        if versions.iter().any(|v| v.etat == "brouillon") {
            return Err(regle("bibliotheque.brouillon_existe"));
        }
        if versions.iter().any(|v| v.etat == "soumise") {
            return Err(regle("bibliotheque.en_attente_de_validation"));
        }
        if !versions
            .iter()
            .any(|v| v.etat == "publiee" && v.retour_livre == 0)
        {
            return Err(regle("bibliotheque.deja_livree"));
        }
        self.creer_brouillon(
            &mut unite,
            session,
            &demande.contenu_id,
            "",
            true,
            None,
            contenu.empreinte.as_deref(),
            "bibliotheque.retour_livree",
        )
        .await?;
        self.noyau.terminer(unite).await?;
        self.ecriture(session, demande).await
    }

    /// Aperçu d'un texte tapé : rendu comme la lecture (notes pour le maître comprises), avec les
    /// formules que l'impression montrera telles qu'elles sont écrites et les images à ajouter.
    pub async fn apercu(
        &self,
        session: &Session,
        demande: &DemandeApercuLecon,
    ) -> Resultat<ApercuLecon> {
        if demande.texte.len() > TEXTE_MAX {
            return Err(regle("bibliotheque.texte_trop_long"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let contenu = depot::contenu(&mut cnx, &demande.contenu_id, &contexte).await?;
        let lecon = analyser(&demande.texte);
        let quiz = gesco_core::bibliotheque::analyser_quiz(
            &demande.texte,
            contenu
                .as_ref()
                .is_some_and(|c| c.nature == NatureContenu::Quiz),
        );
        let images = self
            .images_de(&mut cnx, contenu.as_ref(), &lecon, &contexte)
            .await?;
        let mut formules_brutes: Vec<String> = Vec::new();
        for f in lecon.formules() {
            if gesco_docs::formules::latex_vers_typst(&f.latex).is_err()
                && !formules_brutes.contains(&f.latex)
            {
                formules_brutes.push(f.latex);
            }
        }
        let images_a_ajouter = images_a_ajouter(&lecon);
        let empreintes: Vec<String> = lecon.sons().into_iter().map(str::to_owned).collect();
        let sons = self.sons_de(&mut cnx, &empreintes, &contexte).await?;
        Ok(ApercuLecon {
            lecon,
            images,
            formules_brutes,
            images_a_ajouter,
            sons,
            quiz,
        })
    }

    /// Niveaux, séries et matières pour les formulaires d'ajout, avec la portée de l'utilisateur.
    pub async fn choix_ecriture(&self, session: &Session) -> Resultat<ChoixEcriture> {
        let portee = self.portee(session).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let series = academique::series(&mut cnx, &contexte).await?;
        let niveaux = academique::niveaux(&mut cnx, &contexte)
            .await?
            .into_iter()
            .filter(|n| n.actif)
            .map(|n| NiveauChoisissable {
                series: series
                    .iter()
                    .filter(|s| s.actif && s.niveau_id == n.id)
                    .map(|s| SerieChoisissable {
                        code: s.code.clone(),
                        libelle: s.libelle.clone(),
                    })
                    .collect(),
                id: n.id.vers_chaine(),
                code: n.code,
                libelle: n.libelle,
            })
            .collect();
        let matieres = pedagogie::matieres(&mut cnx, &contexte)
            .await?
            .into_iter()
            .filter(|m| m.active)
            .map(|m| MatiereChoisissable {
                id: m.id.vers_chaine(),
                libelle: m.libelle,
            })
            .collect();
        let mut places: Vec<PlaceEcriture> = portee
            .matieres
            .iter()
            .map(|(m, n)| PlaceEcriture {
                matiere_id: m.clone(),
                niveau_code: n.clone(),
            })
            .collect();
        places
            .sort_by(|a, b| (&a.niveau_code, &a.matiere_id).cmp(&(&b.niveau_code, &b.matiere_id)));
        Ok(ChoixEcriture {
            niveaux,
            matieres,
            partout: portee.partout(),
            places,
        })
    }

    /// Niveau actif de l'école et matière active, vérifiés.
    async fn niveau_et_matiere(
        cnx: &mut Connexion,
        niveau_id: &str,
        matiere_id: &str,
        contexte: &Contexte,
    ) -> Resultat<(String, Vec<String>)> {
        let niveau = academique::niveaux(cnx, contexte)
            .await?
            .into_iter()
            .find(|n| n.actif && n.id.vers_chaine() == niveau_id)
            .ok_or_else(|| introuvable("niveau", niveau_id))?;
        let series = academique::series(cnx, contexte)
            .await?
            .into_iter()
            .filter(|s| s.actif && s.niveau_id == niveau.id)
            .map(|s| s.code)
            .collect();
        if !pedagogie::matieres(cnx, contexte)
            .await?
            .iter()
            .any(|m| m.active && m.id.vers_chaine() == matiere_id)
        {
            return Err(regle("bibliotheque.matiere_inconnue"));
        }
        Ok((niveau.code, series))
    }

    /// Écrit un chapitre de l'école dans une unité, avec son audit et son événement.
    async fn creer_chapitre(
        unite: &mut gesco_db::UniteDeTravail<crate::effets::Effet>,
        c: &depot::NouveauChapitre<'_>,
        contexte: &Contexte,
    ) -> Resultat<String> {
        let id = depot::inserer_chapitre_ecole(unite.connexion(), c, contexte).await?;
        let valeur = serde_json::json!({
            "cle": c.cle,
            "niveau_id": c.niveau_id,
            "matiere_id": c.matiere_id,
            "series": c.series,
            "partie": c.partie,
            "titre": c.titre,
            "apres_cle": c.apres_cle,
        });
        unite.auditer(
            "chapitre_programme",
            &id,
            Operation::Creation,
            None,
            Some(valeur.clone()),
            "bibliotheque.chapitre_ajouter",
        );
        unite.emettre(
            TypeEvenement::ChapitreBibliothequeEnregistre,
            "chapitre_programme",
            &id,
            valeur,
        );
        Ok(id)
    }

    /// Ajoute un chapitre de l'école sous un niveau, une série au besoin et une matière.
    pub async fn ajouter_chapitre(
        &self,
        session: &Session,
        demande: &DemandeChapitreBibliotheque,
    ) -> Resultat<ChapitreAjoute> {
        let titre = titre_valide(&demande.titre)?;
        let partie = demande.partie.trim().to_owned();
        if partie.chars().count() > 100 {
            return Err(regle("bibliotheque.titre_invalide"));
        }
        let portee = self.portee(session).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let (niveau_code, series_du_niveau) = Self::niveau_et_matiere(
            unite.connexion(),
            &demande.niveau_id,
            &demande.matiere_id,
            &contexte,
        )
        .await?;
        if !portee.permet(&demande.matiere_id, &niveau_code) {
            return Err(ErreurService::PorteeRefusee {
                ressource: format!("matiere {}", demande.matiere_id),
            });
        }
        let mut series: Vec<String> = demande
            .series
            .iter()
            .filter(|s| series_du_niveau.contains(s))
            .cloned()
            .collect();
        series.sort();
        series.dedup();
        let cles: HashSet<String> = depot::chapitres(unite.connexion(), &contexte)
            .await?
            .into_iter()
            .map(|c| c.cle)
            .collect();
        // Une place qui n'existe plus range le chapitre en tête.
        let apres_cle = demande.apres_cle.as_deref().filter(|k| cles.contains(*k));
        let cle = format!(
            "ecole/{}",
            nouvel_id::<ChapitreBibliotheque>().vers_chaine()
        );
        let id = Self::creer_chapitre(
            &mut unite,
            &depot::NouveauChapitre {
                cle: &cle,
                niveau_id: &demande.niveau_id,
                niveau_code: &niveau_code,
                matiere_id: &demande.matiere_id,
                partie: &partie,
                titre: &titre,
                apres_cle,
                series: &series,
            },
            &contexte,
        )
        .await?;
        self.noyau.terminer(unite).await?;
        Ok(ChapitreAjoute { id, cle })
    }

    /// Ajoute une leçon, un sujet, un corrigé ou un document de l'école ; il naît brouillon (le
    /// plan des leçons livrées pour une leçon) et s'ouvre dans l'éditeur.
    pub async fn ajouter_contenu(
        &self,
        session: &Session,
        demande: &DemandeAjoutContenu,
    ) -> Resultat<EcritureOuverte> {
        let titre = titre_valide(&demande.titre)?;
        let presentation = demande.presentation.trim().to_owned();
        if presentation.chars().count() > 500 {
            return Err(regle("bibliotheque.titre_invalide"));
        }
        if demande.session.is_some_and(|s| !(1950..=2200).contains(&s)) {
            return Err(regle("bibliotheque.session_invalide"));
        }
        let portee = self.portee(session).await?;
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let chapitres = depot::chapitres(unite.connexion(), &contexte).await?;
        let contenus = depot::contenus(unite.connexion(), &contexte).await?;

        // Le sujet d'un corrigé donne le chapitre, l'examen, la série et la session.
        let sujet = match (&demande.nature, &demande.sujet_id) {
            (NatureContenu::Corrige, Some(id)) => Some(
                contenus
                    .iter()
                    .find(|c| c.id.vers_chaine() == *id && c.nature == NatureContenu::Sujet)
                    .ok_or_else(|| introuvable("contenu", id))?,
            ),
            _ => None,
        };
        let (examen, serie_code, session_examen) = match sujet {
            Some(s) => (s.examen, s.serie_code.clone(), s.session),
            None => (
                demande.examen,
                demande
                    .serie_code
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned),
                demande.session,
            ),
        };
        if demande.nature == NatureContenu::Sujet && examen.is_none() {
            return Err(regle("bibliotheque.examen_manquant"));
        }

        let chapitre_id: String = match (sujet, &demande.chapitre_id) {
            (Some(s), _) => s.chapitre_id.vers_chaine(),
            (None, Some(id)) => id.clone(),
            (None, None) => {
                // Un sujet sans chapitre : « Sujets du DEF » ou « Sujets du BAC » de l'école,
                // dans son niveau et sa matière, créé au besoin.
                let (Some(examen), NatureContenu::Sujet, Some(niveau_id), Some(matiere_id)) = (
                    examen,
                    demande.nature,
                    demande.niveau_id.as_deref(),
                    demande.matiere_id.as_deref(),
                ) else {
                    return Err(regle("bibliotheque.chapitre_manquant"));
                };
                let (niveau_code, _) =
                    Self::niveau_et_matiere(unite.connexion(), niveau_id, matiere_id, &contexte)
                        .await?;
                if !portee.permet(matiere_id, &niveau_code) {
                    return Err(ErreurService::PorteeRefusee {
                        ressource: format!("matiere {matiere_id}"),
                    });
                }
                let titre_chapitre = titre_chapitre_sujets(examen);
                match chapitres.iter().find(|c| {
                    c.origine == OrigineBibliotheque::Ecole
                        && c.niveau_id.as_deref() == Some(niveau_id)
                        && c.matiere_id.as_deref() == Some(matiere_id)
                        && c.titre == titre_chapitre
                }) {
                    Some(c) => c.id.vers_chaine(),
                    None => {
                        // En fin de matière : après le dernier chapitre du niveau.
                        let apres = chapitres
                            .iter()
                            .filter(|c| c.niveau_code == niveau_code)
                            .max_by_key(|c| (c.origine == OrigineBibliotheque::Livre, c.ordre))
                            .map(|c| c.cle.clone());
                        let cle = format!(
                            "ecole/{}",
                            nouvel_id::<ChapitreBibliotheque>().vers_chaine()
                        );
                        Self::creer_chapitre(
                            &mut unite,
                            &depot::NouveauChapitre {
                                cle: &cle,
                                niveau_id,
                                niveau_code: &niveau_code,
                                matiere_id,
                                partie: PARTIE_EXAMENS,
                                titre: titre_chapitre,
                                apres_cle: apres.as_deref(),
                                series: &[],
                            },
                            &contexte,
                        )
                        .await?
                    }
                }
            }
        };
        let chapitre = depot::chapitres(unite.connexion(), &contexte)
            .await?
            .into_iter()
            .find(|c| c.id.vers_chaine() == chapitre_id)
            .ok_or_else(|| introuvable("chapitre", &chapitre_id))?;
        if !portee.permet_chapitre(&chapitre) {
            return Err(ErreurService::PorteeRefusee {
                ressource: format!("chapitre {chapitre_id}"),
            });
        }
        let cles: HashSet<&str> = contenus.iter().map(|c| c.cle.as_str()).collect();
        let apres_cle = match (&demande.apres_cle, sujet) {
            (Some(k), _) if cles.contains(k.as_str()) => Some(k.clone()),
            // Un corrigé se range après son sujet.
            (_, Some(s)) => Some(s.cle.clone()),
            _ => None,
        };
        let cle = format!("ecole/{}", nouvel_id::<ContenuBibliotheque>().vers_chaine());
        let id = depot::inserer_contenu_ecole(
            unite.connexion(),
            &depot::NouveauContenu {
                cle: &cle,
                nature: demande.nature,
                chapitre_id: &chapitre_id,
                titre: &titre,
                presentation: &presentation,
                apres_cle: apres_cle.as_deref(),
                lie_cle: sujet.map(|s| s.cle.as_str()),
                examen,
                serie_code: serie_code.as_deref(),
                session: session_examen,
            },
            &contexte,
        )
        .await?;
        if let Some(s) = sujet
            && s.lie_cle.is_none()
        {
            // Le sujet de l'école renvoie à son corrigé ; un sujet livré n'est jamais touché.
            depot::lier_contenu_ecole(unite.connexion(), &s.id.vers_chaine(), &cle, &contexte)
                .await?;
        }
        let valeur = serde_json::json!({
            "cle": cle,
            "nature": demande.nature.nom(),
            "chapitre_id": chapitre_id,
            "titre": titre,
            "presentation": presentation,
            "examen": examen,
            "serie_code": serie_code,
            "session": session_examen,
            "sujet": sujet.map(|s| s.cle.clone()),
        });
        unite.auditer(
            "contenu",
            &id,
            Operation::Creation,
            None,
            Some(valeur.clone()),
            "bibliotheque.contenu_ajouter",
        );
        unite.emettre(
            TypeEvenement::ContenuBibliothequeEnregistre,
            "contenu",
            &id,
            valeur,
        );
        // Un quiz de chapitre naît avec une première question à remplir.
        let gabarit_quiz = {
            let (question, reponse) = gesco_core::bibliotheque::gabarit_question("");
            gesco_core::bibliotheque::ajouter_question("", true, &question, &reponse)
        };
        let texte = match demande.nature {
            NatureContenu::Lecon => PLAN_LECON,
            NatureContenu::Quiz => gabarit_quiz.as_str(),
            _ => "",
        };
        self.creer_brouillon(
            &mut unite,
            session,
            &id,
            texte,
            false,
            None,
            None,
            "bibliotheque.contenu_ajouter",
        )
        .await?;
        self.noyau.terminer(unite).await?;
        self.ecriture(session, &DemandeContenuBibliotheque { contenu_id: id })
            .await
    }

    /// L'historique d'un contenu : ses versions, ses retraits et remises.
    pub async fn historique(
        &self,
        session: &Session,
        demande: &DemandeContenuBibliotheque,
    ) -> Resultat<HistoriqueContenu> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let portee = self.portee(session).await?;
        let mut cnx = self.connexion().await?;
        let (contenu, peut_ecrire) =
            Self::contenu_et_portee(&mut cnx, &portee, &demande.contenu_id, &contexte).await?;
        let lignes = depot::versions(&mut cnx, &demande.contenu_id, &contexte).await?;
        let numeros: HashMap<&str, i64> =
            lignes.iter().map(|v| (v.id.as_str(), v.numero)).collect();
        let mut versions: Vec<VersionAffichee> = lignes
            .iter()
            .filter_map(|v| {
                Some(VersionAffichee {
                    id: v.id.clone(),
                    numero: nombre(v.numero),
                    etat: EtatVersion::depuis_nom(&v.etat)?,
                    retour_livre: v.retour_livre != 0,
                    auteur: v.auteur.clone(),
                    cree_le: v.cree_le,
                    enregistre_le: v.mis_a_jour_le,
                    soumise_le: v.soumise_le,
                    decidee_le: v.decidee_le,
                    decidee_par: v.decidee_par.clone(),
                    commentaire: v.commentaire.clone(),
                    depart_numero: v
                        .depart_version_id
                        .as_deref()
                        .and_then(|d| numeros.get(d))
                        .map(|n| nombre(*n)),
                    depart_livree: v.depart_version_id.is_none() && v.depart_livre_sha256.is_some(),
                })
            })
            .collect();
        versions.reverse();
        let mut mouvements = Vec::new();
        for r in depot::retraits(&mut cnx, false, &contexte)
            .await?
            .into_iter()
            .filter(|r| r.grain == "contenu" && r.cible == contenu.cle)
        {
            mouvements.push(MouvementContenu {
                sorte: SorteMouvement::Retrait,
                quand: r.retire_le,
                qui: r.retire_par.clone(),
            });
            if let Some(remis) = r.remis_le {
                mouvements.push(MouvementContenu {
                    sorte: SorteMouvement::Remise,
                    quand: remis,
                    qui: r.remis_par.clone(),
                });
            }
        }
        mouvements.sort_by_key(|m| std::cmp::Reverse(m.quand));
        let mut recues: Vec<VersionRecueAffichee> =
            depot::versions_recues(&mut cnx, &demande.contenu_id, &contexte)
                .await?
                .into_iter()
                .map(|r| VersionRecueAffichee {
                    id: r.id,
                    recue_le: r.recue_le,
                    importe_par: r.importe_par,
                })
                .collect();
        recues.reverse();
        Ok(HistoriqueContenu {
            contenu_id: contenu.id.vers_chaine(),
            titre: contenu.titre.clone(),
            versions,
            mouvements,
            recues,
            a_une_version_livree: self.texte_livre(&contenu).is_some(),
            livree_plus_recente: Self::livree_plus_recente(&contenu, &lignes),
            peut_ecrire,
        })
    }

    /// Compare deux versions d'un contenu, livrée comprise, ligne à ligne.
    pub async fn comparer(
        &self,
        session: &Session,
        demande: &DemandeComparaison,
    ) -> Resultat<ComparaisonTextes> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let contenu = depot::contenu(&mut cnx, &demande.contenu_id, &contexte)
            .await?
            .ok_or_else(|| introuvable("contenu", &demande.contenu_id))?;
        let versions = depot::versions(&mut cnx, &demande.contenu_id, &contexte).await?;
        let recues = depot::versions_recues(&mut cnx, &demande.contenu_id, &contexte).await?;
        let texte = |t: &TexteCompare| -> Resultat<String> {
            Ok(match t {
                TexteCompare::Livree => self.texte_livre(&contenu).unwrap_or_default(),
                TexteCompare::Visible => self
                    .texte_visible(&contenu, &versions)
                    .0
                    .unwrap_or_default(),
                TexteCompare::Version { version_id } => {
                    let v = versions
                        .iter()
                        .find(|v| &v.id == version_id)
                        .ok_or_else(|| introuvable("version_contenu", version_id))?;
                    if v.retour_livre != 0 {
                        self.texte_livre(&contenu).unwrap_or_default()
                    } else {
                        v.texte.clone()
                    }
                }
                TexteCompare::Recue { version_recue_id } => recues
                    .iter()
                    .find(|r| &r.id == version_recue_id)
                    .ok_or_else(|| introuvable("version_recue", version_recue_id))?
                    .texte
                    .clone(),
            })
        };
        let rangees = comparer_lignes(&texte(&demande.gauche)?, &texte(&demande.droite)?);
        let retirees = rangees
            .iter()
            .filter(|r| r.change && r.gauche.is_some())
            .count();
        let ajoutees = rangees
            .iter()
            .filter(|r| r.change && r.droite.is_some())
            .count();
        Ok(ComparaisonTextes {
            retirees: u32::try_from(retirees).unwrap_or(u32::MAX),
            ajoutees: u32::try_from(ajoutees).unwrap_or(u32::MAX),
            rangees,
        })
    }

    /// Vrai si l'utilisateur peut écrire dans le chapitre du contenu (pour la barre de la
    /// leçon).
    pub(super) async fn peut_modifier(
        cnx: &mut Connexion,
        portee: &Portee,
        contenu: &ContenuBibliotheque,
        contexte: &Contexte,
    ) -> Resultat<bool> {
        Ok(depot::chapitres(cnx, contexte)
            .await?
            .iter()
            .find(|c| c.id == contenu.chapitre_id)
            .is_some_and(|c| portee.permet_chapitre(c)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_titre_vide_ou_trop_long_est_refuse() {
        assert!(titre_valide("  ").is_err());
        assert!(titre_valide(&"a".repeat(201)).is_err());
        assert_eq!(
            titre_valide(" Les fractions ").ok(),
            Some("Les fractions".into())
        );
    }

    #[test]
    fn les_chapitres_de_sujets_portent_le_nom_de_leur_examen() {
        assert_eq!(titre_chapitre_sujets(ExamenSujet::Def), "Sujets du DEF");
        assert_eq!(titre_chapitre_sujets(ExamenSujet::Bac), "Sujets du BAC");
    }
}
