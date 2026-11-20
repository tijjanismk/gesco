//! Devoirs à rendre (SPEC_PHASE_12_LOT_5) : le maître écrit un devoir pour une classe dans sa
//! matière, le donne, l'imprime, le clôt. Un compte de maître ne voit et n'écrit que les classes
//! et les matières de ses affectations de l'année en cours ; la direction voit tout.
//!
//! États : brouillon (le maître seul le voit) → donné (visible des élèves de la classe sur le
//! portail, imprimable) → clos (plus de rendu ; jamais automatique, et le maître peut le rouvrir).
//! L'interrupteur de la bibliothèque coupe aussi les devoirs : éteint, la liste le dit et rien ne
//! s'écrit ; les devoirs restent dans la base et reviennent quand on le rallume.

use std::collections::{HashMap, HashSet};

use base64::Engine as _;
use gesco_core::bibliotheque::{Lecon, SourceImage, analyser};
use gesco_core::core::{ModuleEcole, Operation, Permission};
use gesco_db::depots::devoirs::{self as depot, LigneDevoir};
use gesco_db::depots::{academique, annee_scolaire, pedagogie, portail as depot_portail};
use gesco_db::unite::Durabilite;
use gesco_db::{Connexion, Contexte, ErreurBase, nouvel_id};
use serde::{Deserialize, Serialize};

use crate::bibliotheque::ImageLecon;
use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;
use crate::session::Session;

/// Barème le plus petit, en centièmes (sur 10).
pub const BAREME_MIN: i32 = 1_000;
/// Barème le plus grand, en centièmes (sur 100).
pub const BAREME_MAX: i32 = 10_000;
/// Barème par défaut, en centièmes (sur 20).
pub const BAREME_DEFAUT: i32 = 2_000;
/// Longueur d'un titre, au plus.
pub const TITRE_MAX: usize = 200;
/// Longueur des consignes, au plus (caractères).
pub const CONSIGNES_MAX: usize = 50_000;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Façon dont les élèves rendent le devoir.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum FormeRendu {
    /// Un texte tapé.
    Texte,
    /// Des photos de la copie.
    Photos,
    /// Un texte ou des photos (par défaut).
    TexteOuPhotos,
    /// Sur papier seulement : le maître note dans l'écran du devoir.
    Papier,
}

impl FormeRendu {
    /// Nom gardé en base.
    pub const fn nom(self) -> &'static str {
        match self {
            Self::Texte => "texte",
            Self::Photos => "photos",
            Self::TexteOuPhotos => "texte_ou_photos",
            Self::Papier => "papier",
        }
    }

    /// Depuis le nom gardé en base.
    pub fn depuis_nom(nom: &str) -> Self {
        match nom {
            "texte" => Self::Texte,
            "photos" => Self::Photos,
            "papier" => Self::Papier,
            _ => Self::TexteOuPhotos,
        }
    }

    /// Un texte peut être rendu.
    pub const fn accepte_texte(self) -> bool {
        matches!(self, Self::Texte | Self::TexteOuPhotos)
    }

    /// Des photos peuvent être rendues.
    pub const fn accepte_photos(self) -> bool {
        matches!(self, Self::Photos | Self::TexteOuPhotos)
    }
}

/// État d'un devoir.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum EtatDevoir {
    /// Le maître l'écrit ; les élèves ne le voient pas.
    Brouillon,
    /// Visible des élèves de la classe ; on peut rendre.
    Donne,
    /// Plus de rendu.
    Clos,
}

impl EtatDevoir {
    /// Nom gardé en base.
    pub const fn nom(self) -> &'static str {
        match self {
            Self::Brouillon => "brouillon",
            Self::Donne => "donne",
            Self::Clos => "clos",
        }
    }

    /// Depuis le nom gardé en base.
    pub fn depuis_nom(nom: &str) -> Self {
        match nom {
            "donne" => Self::Donne,
            "clos" => Self::Clos,
            _ => Self::Brouillon,
        }
    }
}

/// Une matière où donner un devoir.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct MatiereDevoir {
    /// Identifiant.
    pub id: String,
    /// Libellé.
    pub libelle: String,
}

/// Une classe où donner un devoir, et ses matières permises.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct ClasseDevoir {
    /// Identifiant.
    pub id: String,
    /// Libellé.
    pub libelle: String,
    /// Matières permises dans cette classe.
    pub matieres: Vec<MatiereDevoir>,
}

/// Devoirs d'une classe et d'une matière, ou tous ceux qu'on peut voir.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeDevoirs {
    /// Classe ; absente : toutes.
    pub classe_id: Option<String>,
    /// Matière ; absente : toutes.
    pub matiere_id: Option<String>,
}

/// Un devoir dans la liste.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct DevoirResume {
    /// Identifiant.
    pub id: String,
    /// Titre.
    pub titre: String,
    /// Classe.
    pub classe_id: String,
    /// Libellé de la classe.
    pub classe: String,
    /// Matière.
    pub matiere_id: String,
    /// Libellé de la matière.
    pub matiere: String,
    /// État.
    pub etat: EtatDevoir,
    /// Date et heure limites (millisecondes).
    pub date_limite: i64,
    /// Façon de rendre.
    pub forme_rendu: FormeRendu,
    /// Élèves qui ont rendu.
    pub rendus: u32,
}

/// Les devoirs qu'on peut voir.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct ListeDevoirs {
    /// La bibliothèque est allumée : sinon les devoirs sont coupés (données gardées).
    pub actif: bool,
    /// Le compte peut écrire, donner, clore, corriger.
    pub peut_donner: bool,
    /// Classes et matières où ce compte peut donner un devoir.
    pub classes: Vec<ClasseDevoir>,
    /// Devoirs, du plus récent au plus ancien (date limite).
    pub devoirs: Vec<DevoirResume>,
}

/// Un devoir désigné.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeIdDevoir {
    /// Identifiant.
    pub id: String,
}

/// Ce que le maître écrit d'un devoir.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeDevoir {
    /// Classe.
    pub classe_id: String,
    /// Matière.
    pub matiere_id: String,
    /// Titre.
    pub titre: String,
    /// Consignes, au format des leçons.
    pub consignes: String,
    /// Leçon liée.
    pub contenu_id: Option<String>,
    /// Date et heure limites (millisecondes).
    pub date_limite: i64,
    /// Barème en centièmes (de 1 000 à 10 000).
    pub bareme: i32,
    /// Façon de rendre.
    pub forme_rendu: FormeRendu,
}

/// Modifier un devoir : l'instant lu avec lui protège ce qu'un autre a écrit entre-temps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeDevoirModifie {
    /// Devoir.
    pub id: String,
    /// Instant de dernière modification lu avec le devoir.
    pub lue_le: i64,
    /// Ce qui est écrit.
    pub devoir: DemandeDevoir,
}

/// Donner aussi le devoir à une autre classe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeCopieDevoir {
    /// Devoir copié.
    pub id: String,
    /// Autre classe.
    pub classe_id: String,
}

/// Aperçu des consignes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeApercuDevoir {
    /// Texte tapé.
    pub texte: String,
}

/// Consignes rendues comme une leçon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct ApercuDevoir {
    /// Le texte en arbre de blocs.
    pub lecon: Lecon,
    /// Images appelées.
    pub images: Vec<ImageLecon>,
}

/// Un devoir ouvert.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct DevoirAffiche {
    /// Identifiant.
    pub id: String,
    /// Titre.
    pub titre: String,
    /// Consignes, telles qu'écrites.
    pub consignes: String,
    /// Classe.
    pub classe_id: String,
    /// Libellé de la classe.
    pub classe: String,
    /// Matière.
    pub matiere_id: String,
    /// Libellé de la matière.
    pub matiere: String,
    /// Leçon liée.
    pub contenu_id: Option<String>,
    /// Titre de la leçon liée.
    pub lecon_liee: Option<String>,
    /// Date et heure limites (millisecondes).
    pub date_limite: i64,
    /// Barème en centièmes.
    pub bareme: i32,
    /// Façon de rendre.
    pub forme_rendu: FormeRendu,
    /// État.
    pub etat: EtatDevoir,
    /// Donné le.
    pub donne_le: Option<i64>,
    /// Clos le.
    pub clos_le: Option<i64>,
    /// Copies corrigées rendues aux élèves le.
    pub copies_rendues_le: Option<i64>,
    /// Devoir dont celui-ci est la copie.
    pub copie_de: Option<String>,
    /// Consignes en arbre de blocs.
    pub lecon: Lecon,
    /// Images des consignes.
    pub images: Vec<ImageLecon>,
    /// Instant de dernière modification, à renvoyer avec une modification.
    pub modifie_le: i64,
    /// Le compte peut l'écrire, le donner, le clore.
    pub peut_modifier: bool,
    /// Autres classes où ce compte peut donner le même devoir (même matière).
    pub autres_classes: Vec<MatiereDevoir>,
}

/// Classes et matières qu'un compte peut voir : `None` partout (direction), sinon ses couples
/// (classe, matière) de l'année en cours.
pub(crate) struct Portee {
    couples: Option<HashSet<(String, String)>>,
    peut_donner: bool,
}

impl Portee {
    pub(crate) fn permet(&self, classe_id: &str, matiere_id: &str) -> bool {
        self.couples
            .as_ref()
            .is_none_or(|c| c.contains(&(classe_id.to_owned(), matiere_id.to_owned())))
    }
}

/// Service des devoirs.
pub struct ServiceDevoirs<'n> {
    noyau: &'n Noyau,
}

/// Ce que le service sait d'une classe de l'année.
struct ClasseLue {
    libelle: String,
    niveau_id: String,
    serie_id: Option<String>,
}

/// Classes de l'année, matières et programme, lus d'un coup.
struct Referentiel {
    classes: HashMap<String, ClasseLue>,
    ordre_classes: Vec<String>,
    matieres: HashMap<String, String>,
    /// (niveau, série, matière) au programme.
    programme: HashSet<(String, Option<String>, String)>,
}

impl Referentiel {
    fn classe(&self, id: &str) -> String {
        self.classes
            .get(id)
            .map(|c| c.libelle.clone())
            .unwrap_or_default()
    }

    fn matiere(&self, id: &str) -> String {
        self.matieres.get(id).cloned().unwrap_or_default()
    }

    fn au_programme(&self, classe_id: &str, matiere_id: &str) -> bool {
        self.classes.get(classe_id).is_some_and(|c| {
            self.programme.contains(&(
                c.niveau_id.clone(),
                c.serie_id.clone(),
                matiere_id.to_owned(),
            ))
        })
    }

    /// Classes et matières permises par la portée, dans l'ordre des classes puis des matières.
    fn choix(&self, portee: &Portee) -> Vec<ClasseDevoir> {
        let mut classes = Vec::new();
        for id in &self.ordre_classes {
            let mut matieres: Vec<MatiereDevoir> = self
                .matieres
                .iter()
                .filter(|(m, _)| self.au_programme(id, m) && portee.permet(id, m))
                .map(|(m, libelle)| MatiereDevoir {
                    id: m.clone(),
                    libelle: libelle.clone(),
                })
                .collect();
            // Un maître affecté hors programme garde sa matière.
            if let Some(couples) = &portee.couples {
                for (c, m) in couples {
                    if c == id && !matieres.iter().any(|x| &x.id == m) {
                        matieres.push(MatiereDevoir {
                            id: m.clone(),
                            libelle: self.matiere(m),
                        });
                    }
                }
            }
            if matieres.is_empty() {
                continue;
            }
            matieres.sort_by(|a, b| a.libelle.cmp(&b.libelle));
            classes.push(ClasseDevoir {
                id: id.clone(),
                libelle: self.classe(id),
                matieres,
            });
        }
        classes
    }
}

impl<'n> ServiceDevoirs<'n> {
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
            .map_err(ErreurBase::from)?)
    }

    fn contexte(&self, session: &Session) -> Contexte {
        self.noyau.contexte(session.etablissement_id, Some(session))
    }

    /// Portée du compte (comme le suivi des élèves) : un compte de maître, ses affectations de
    /// l'année en cours ; tout autre compte, toute l'école.
    pub(crate) async fn portee(&self, session: &Session) -> Resultat<Portee> {
        let contexte = self.contexte(session);
        let droits = self
            .noyau
            .droits_de(session.utilisateur_id, &contexte)
            .await?;
        let peut_donner = droits.permissions.contains(&Permission::HOMEWORK_MANAGE);
        let couples = if droits.maitre {
            let mut cnx = self.connexion().await?;
            Some(
                depot_portail::affectations_du_compte(
                    &mut cnx,
                    &session.utilisateur_id.vers_chaine(),
                    &contexte,
                )
                .await?
                .into_iter()
                .collect(),
            )
        } else {
            None
        };
        Ok(Portee {
            couples,
            peut_donner,
        })
    }

    /// La bibliothèque est allumée.
    pub(crate) async fn module_actif(
        &self,
        cnx: &mut Connexion,
        contexte: &Contexte,
    ) -> Resultat<bool> {
        crate::modules::actif(cnx, ModuleEcole::Bibliotheque, contexte).await
    }

    /// Refuse quand la bibliothèque est éteinte (les devoirs sont coupés avec elle).
    pub(crate) async fn exiger_module(&self, session: &Session) -> Resultat<()> {
        let contexte = self.contexte(session);
        let mut cnx = self.connexion().await?;
        if self.module_actif(&mut cnx, &contexte).await? {
            Ok(())
        } else {
            Err(regle("devoir.module_eteint"))
        }
    }

    async fn referentiel(
        cnx: &mut Connexion,
        annee_id: &str,
        contexte: &Contexte,
    ) -> Resultat<Referentiel> {
        let annee = gesco_core::commun::Id::analyser(annee_id).map_err(|_| {
            ErreurService::from(ErreurBase::Introuvable {
                entite: "annee_scolaire",
                id: annee_id.to_owned(),
            })
        })?;
        let liste = academique::classes(cnx, annee, contexte).await?;
        let ordre_classes = liste.iter().map(|c| c.id.vers_chaine()).collect();
        let classes = liste
            .into_iter()
            .map(|c| {
                (
                    c.id.vers_chaine(),
                    ClasseLue {
                        libelle: c.libelle,
                        niveau_id: c.niveau_id.vers_chaine(),
                        serie_id: c.serie_id.map(|s| s.vers_chaine()),
                    },
                )
            })
            .collect();
        let matieres = pedagogie::matieres(cnx, contexte)
            .await?
            .into_iter()
            .map(|m| (m.id.vers_chaine(), m.libelle))
            .collect();
        let programme = pedagogie::programme(cnx, annee, contexte)
            .await?
            .into_iter()
            .map(|l| {
                (
                    l.niveau_id.vers_chaine(),
                    l.serie_id.map(|s| s.vers_chaine()),
                    l.matiere_id.vers_chaine(),
                )
            })
            .collect();
        Ok(Referentiel {
            classes,
            ordre_classes,
            matieres,
            programme,
        })
    }

    /// Année en cours (son identifiant), ou refus s'il n'y en a pas.
    async fn annee_courante(cnx: &mut Connexion, contexte: &Contexte) -> Resultat<Option<String>> {
        Ok(annee_scolaire::courante(cnx, contexte)
            .await?
            .map(|a| a.id.vers_chaine()))
    }

    /// Les devoirs que ce compte peut voir, et où il peut en donner.
    pub async fn lister(
        &self,
        session: &Session,
        demande: &DemandeDevoirs,
    ) -> Resultat<ListeDevoirs> {
        let portee = self.portee(session).await?;
        let contexte = self.contexte(session);
        let mut cnx = self.connexion().await?;
        let actif = self.module_actif(&mut cnx, &contexte).await?;
        let mut liste = ListeDevoirs {
            actif,
            peut_donner: portee.peut_donner,
            classes: Vec::new(),
            devoirs: Vec::new(),
        };
        let Some(annee_id) = Self::annee_courante(&mut cnx, &contexte).await? else {
            return Ok(liste);
        };
        let referentiel = Self::referentiel(&mut cnx, &annee_id, &contexte).await?;
        liste.classes = referentiel.choix(&portee);
        if !actif {
            return Ok(liste);
        }
        let rendus: HashMap<String, i64> = depot::rendus_par_devoir(&mut cnx, &annee_id, &contexte)
            .await?
            .into_iter()
            .collect();
        liste.devoirs = depot::de_l_annee(&mut cnx, &annee_id, &contexte)
            .await?
            .into_iter()
            .filter(|d| portee.permet(&d.classe_id, &d.matiere_id))
            .filter(|d| demande.classe_id.as_ref().is_none_or(|c| c == &d.classe_id))
            .filter(|d| {
                demande
                    .matiere_id
                    .as_ref()
                    .is_none_or(|m| m == &d.matiere_id)
            })
            .map(|d| DevoirResume {
                rendus: u32::try_from(rendus.get(&d.id).copied().unwrap_or(0)).unwrap_or(u32::MAX),
                classe: referentiel.classe(&d.classe_id),
                matiere: referentiel.matiere(&d.matiere_id),
                etat: EtatDevoir::depuis_nom(&d.etat),
                forme_rendu: FormeRendu::depuis_nom(&d.forme_rendu),
                id: d.id,
                titre: d.titre,
                classe_id: d.classe_id,
                matiere_id: d.matiere_id,
                date_limite: d.date_limite,
            })
            .collect();
        Ok(liste)
    }

    /// Le devoir, s'il est dans la portée du compte.
    pub(crate) async fn devoir_permis(
        &self,
        session: &Session,
        id: &str,
    ) -> Resultat<(LigneDevoir, Portee)> {
        let portee = self.portee(session).await?;
        let contexte = self.contexte(session);
        let mut cnx = self.connexion().await?;
        let devoir = depot::lire(&mut cnx, id, &contexte).await?.ok_or_else(|| {
            ErreurService::from(ErreurBase::Introuvable {
                entite: "devoir",
                id: id.to_owned(),
            })
        })?;
        if !portee.permet(&devoir.classe_id, &devoir.matiere_id) {
            return Err(ErreurService::PorteeRefusee {
                ressource: format!("devoir {id}"),
            });
        }
        Ok((devoir, portee))
    }

    /// Images de l'école appelées par des consignes (les images d'un paquet n'ont pas de sens
    /// hors de leur leçon : elles sont passées).
    pub(crate) async fn images(
        cnx: &mut Connexion,
        lecon: &Lecon,
        contexte: &Contexte,
    ) -> Resultat<Vec<ImageLecon>> {
        let mut empreintes: Vec<String> = lecon
            .images()
            .into_iter()
            .filter_map(|s| match s {
                SourceImage::Ecole { empreinte } => Some(empreinte.clone()),
                SourceImage::Paquet { .. } => None,
            })
            .collect();
        empreintes.sort();
        empreintes.dedup();
        Ok(
            gesco_db::depots::bibliotheque::images_par_empreinte(cnx, &empreintes, contexte)
                .await?
                .into_iter()
                .map(|(cle, type_mime, octets)| ImageLecon {
                    cle,
                    type_mime,
                    donnees: base64::engine::general_purpose::STANDARD.encode(octets),
                })
                .collect(),
        )
    }

    /// Un devoir ouvert, pour l'écran.
    pub async fn lire(&self, session: &Session, id: &str) -> Resultat<DevoirAffiche> {
        let (devoir, portee) = self.devoir_permis(session, id).await?;
        self.afficher(session, devoir, &portee).await
    }

    async fn afficher(
        &self,
        session: &Session,
        d: LigneDevoir,
        portee: &Portee,
    ) -> Resultat<DevoirAffiche> {
        let contexte = self.contexte(session);
        let mut cnx = self.connexion().await?;
        let referentiel = Self::referentiel(&mut cnx, &d.annee_id, &contexte).await?;
        let lecon = analyser(&d.consignes).pour_les_eleves();
        let images = Self::images(&mut cnx, &lecon, &contexte).await?;
        let lecon_liee = match &d.contenu_id {
            Some(c) => depot::titre_du_contenu(&mut cnx, c, &contexte).await?,
            None => None,
        };
        let autres_classes = referentiel
            .choix(portee)
            .into_iter()
            .filter(|c| c.id != d.classe_id && c.matieres.iter().any(|m| m.id == d.matiere_id))
            .map(|c| MatiereDevoir {
                id: c.id,
                libelle: c.libelle,
            })
            .collect();
        Ok(DevoirAffiche {
            classe: referentiel.classe(&d.classe_id),
            matiere: referentiel.matiere(&d.matiere_id),
            bareme: i32::try_from(d.bareme).unwrap_or(BAREME_DEFAUT),
            forme_rendu: FormeRendu::depuis_nom(&d.forme_rendu),
            etat: EtatDevoir::depuis_nom(&d.etat),
            peut_modifier: portee.peut_donner,
            modifie_le: d.mis_a_jour_le,
            lecon,
            images,
            lecon_liee,
            autres_classes,
            id: d.id,
            titre: d.titre,
            consignes: d.consignes,
            classe_id: d.classe_id,
            matiere_id: d.matiere_id,
            contenu_id: d.contenu_id,
            date_limite: d.date_limite,
            donne_le: d.donne_le,
            clos_le: d.clos_le,
            copies_rendues_le: d.copies_rendues_le,
            copie_de: d.copie_de,
        })
    }

    /// Aperçu des consignes, rendues comme une leçon.
    pub async fn apercu(
        &self,
        session: &Session,
        demande: &DemandeApercuDevoir,
    ) -> Resultat<ApercuDevoir> {
        let contexte = self.contexte(session);
        let mut cnx = self.connexion().await?;
        let lecon = analyser(&demande.texte).pour_les_eleves();
        let images = Self::images(&mut cnx, &lecon, &contexte).await?;
        Ok(ApercuDevoir { lecon, images })
    }

    /// Vérifie ce qui est écrit et rend le titre nettoyé.
    fn verifier(demande: &DemandeDevoir) -> Resultat<String> {
        let titre = demande.titre.trim();
        if titre.is_empty() {
            return Err(regle("devoir.titre_vide"));
        }
        if titre.chars().count() > TITRE_MAX {
            return Err(regle("devoir.titre_long"));
        }
        if demande.consignes.chars().count() > CONSIGNES_MAX {
            return Err(regle("devoir.consignes_longues"));
        }
        if !(BAREME_MIN..=BAREME_MAX).contains(&demande.bareme) {
            return Err(regle("devoir.bareme_invalide"));
        }
        if demande.date_limite <= 0 {
            return Err(regle("devoir.date_invalide"));
        }
        Ok(titre.to_owned())
    }

    /// Vérifie que la classe et la matière sont permises, et la leçon liée connue ; rend l'année
    /// de la classe.
    async fn verifier_place(
        &self,
        session: &Session,
        portee: &Portee,
        demande: &DemandeDevoir,
    ) -> Resultat<String> {
        if !portee.peut_donner || !portee.permet(&demande.classe_id, &demande.matiere_id) {
            return Err(ErreurService::PorteeRefusee {
                ressource: format!(
                    "classe {}, matière {}",
                    demande.classe_id, demande.matiere_id
                ),
            });
        }
        let contexte = self.contexte(session);
        let mut cnx = self.connexion().await?;
        let Some(annee_id) = Self::annee_courante(&mut cnx, &contexte).await? else {
            return Err(regle("devoir.sans_annee"));
        };
        let referentiel = Self::referentiel(&mut cnx, &annee_id, &contexte).await?;
        if !referentiel.classes.contains_key(&demande.classe_id) {
            return Err(regle("devoir.classe_hors_annee"));
        }
        if !referentiel.matieres.contains_key(&demande.matiere_id) {
            return Err(regle("devoir.matiere_inconnue"));
        }
        if let Some(c) = &demande.contenu_id
            && depot::titre_du_contenu(&mut cnx, c, &contexte)
                .await?
                .is_none()
        {
            return Err(regle("devoir.lecon_introuvable"));
        }
        let classe = gesco_core::commun::Id::analyser(&demande.classe_id).map_err(|_| {
            ErreurService::from(ErreurBase::Introuvable {
                entite: "classe",
                id: demande.classe_id.clone(),
            })
        })?;
        drop(cnx);
        crate::annees::exiger_classe_modifiable(self.noyau, session, classe).await?;
        Ok(annee_id)
    }

    /// Écrit un devoir nouveau, en brouillon.
    pub async fn creer(
        &self,
        session: &Session,
        demande: &DemandeDevoir,
    ) -> Resultat<DevoirAffiche> {
        self.exiger_module(session).await?;
        let titre = Self::verifier(demande)?;
        let portee = self.portee(session).await?;
        let annee_id = self.verifier_place(session, &portee, demande).await?;
        let contexte = self.contexte(session);
        let maintenant = contexte.maintenant.millis_epoque();
        let devoir = LigneDevoir {
            id: nouvel_id::<()>().vers_chaine(),
            annee_id,
            classe_id: demande.classe_id.clone(),
            matiere_id: demande.matiere_id.clone(),
            auteur_id: session.utilisateur_id.vers_chaine(),
            titre,
            consignes: demande.consignes.clone(),
            contenu_id: demande.contenu_id.clone(),
            date_limite: demande.date_limite,
            bareme: i64::from(demande.bareme),
            forme_rendu: demande.forme_rendu.nom().to_owned(),
            etat: EtatDevoir::Brouillon.nom().to_owned(),
            donne_le: None,
            clos_le: None,
            copies_rendues_le: None,
            reporte_le: None,
            copie_de: None,
            cree_le: maintenant,
            mis_a_jour_le: maintenant,
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        depot::inserer(unite.connexion(), &devoir, &contexte).await?;
        unite.auditer(
            "devoir",
            &devoir.id,
            Operation::Creation,
            None,
            Some(audit_devoir(&devoir)),
            "devoirs.creer",
        );
        self.noyau.terminer(unite).await?;
        self.afficher(session, devoir, &portee).await
    }

    /// Récrit un devoir. La classe et la matière ne changent plus une fois le devoir donné (ses
    /// rendus y sont attachés).
    pub async fn enregistrer(
        &self,
        session: &Session,
        demande: &DemandeDevoirModifie,
    ) -> Resultat<DevoirAffiche> {
        self.exiger_module(session).await?;
        let titre = Self::verifier(&demande.devoir)?;
        let (avant, portee) = self.devoir_permis(session, &demande.id).await?;
        let place_changee = avant.classe_id != demande.devoir.classe_id
            || avant.matiere_id != demande.devoir.matiere_id;
        if place_changee && EtatDevoir::depuis_nom(&avant.etat) != EtatDevoir::Brouillon {
            return Err(regle("devoir.classe_fixee"));
        }
        self.verifier_place(session, &portee, &demande.devoir)
            .await?;
        let contexte = self.contexte(session);
        let mut apres = avant.clone();
        apres.classe_id.clone_from(&demande.devoir.classe_id);
        apres.matiere_id.clone_from(&demande.devoir.matiere_id);
        apres.titre = titre;
        apres.consignes.clone_from(&demande.devoir.consignes);
        apres.contenu_id.clone_from(&demande.devoir.contenu_id);
        apres.date_limite = demande.devoir.date_limite;
        apres.bareme = i64::from(demande.devoir.bareme);
        demande
            .devoir
            .forme_rendu
            .nom()
            .clone_into(&mut apres.forme_rendu);
        apres.mis_a_jour_le = contexte.maintenant.millis_epoque();
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        crate::fraicheur::verifier(
            unite.connexion(),
            &[crate::fraicheur::Ligne::par_id("devoir", avant.id.clone())],
            demande.lue_le,
        )
        .await?;
        depot::ecrire(unite.connexion(), &apres, &contexte).await?;
        unite.auditer(
            "devoir",
            &apres.id,
            Operation::Modification,
            Some(audit_devoir(&avant)),
            Some(audit_devoir(&apres)),
            "devoirs.enregistrer",
        );
        self.noyau.terminer(unite).await?;
        self.afficher(session, apres, &portee).await
    }

    /// Passe un devoir d'un état à un autre.
    async fn transition(
        &self,
        session: &Session,
        id: &str,
        depuis: &[EtatDevoir],
        vers: EtatDevoir,
        ecran: &str,
    ) -> Resultat<DevoirAffiche> {
        self.exiger_module(session).await?;
        let (mut devoir, portee) = self.devoir_permis(session, id).await?;
        if !portee.peut_donner {
            return Err(ErreurService::PorteeRefusee {
                ressource: format!("devoir {id}"),
            });
        }
        let etat = EtatDevoir::depuis_nom(&devoir.etat);
        if etat == vers {
            return self.afficher(session, devoir, &portee).await;
        }
        if !depuis.contains(&etat) {
            return Err(regle("devoir.etat_invalide"));
        }
        let classe = gesco_core::commun::Id::analyser(&devoir.classe_id).map_err(|_| {
            ErreurService::from(ErreurBase::Introuvable {
                entite: "classe",
                id: devoir.classe_id.clone(),
            })
        })?;
        crate::annees::exiger_classe_modifiable(self.noyau, session, classe).await?;
        let contexte = self.contexte(session);
        let maintenant = contexte.maintenant.millis_epoque();
        let (donne_le, clos_le) = match vers {
            EtatDevoir::Brouillon => (None, None),
            EtatDevoir::Donne => (devoir.donne_le.or(Some(maintenant)), None),
            EtatDevoir::Clos => (devoir.donne_le.or(Some(maintenant)), Some(maintenant)),
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        depot::changer_etat(
            unite.connexion(),
            &devoir.id,
            (vers.nom(), donne_le, clos_le),
            &contexte,
        )
        .await?;
        unite.auditer(
            "devoir",
            &devoir.id,
            Operation::Transition,
            Some(serde_json::json!({ "etat": devoir.etat })),
            Some(serde_json::json!({ "etat": vers.nom() })),
            ecran,
        );
        self.noyau.terminer(unite).await?;
        vers.nom().clone_into(&mut devoir.etat);
        devoir.donne_le = donne_le;
        devoir.clos_le = clos_le;
        devoir.mis_a_jour_le = maintenant;
        self.afficher(session, devoir, &portee).await
    }

    /// Donne le devoir : les élèves de la classe le voient sur le portail.
    pub async fn donner(&self, session: &Session, id: &str) -> Resultat<DevoirAffiche> {
        self.transition(
            session,
            id,
            &[EtatDevoir::Brouillon],
            EtatDevoir::Donne,
            "devoirs.donner",
        )
        .await
    }

    /// Clôt le devoir : plus de rendu.
    pub async fn clore(&self, session: &Session, id: &str) -> Resultat<DevoirAffiche> {
        self.transition(
            session,
            id,
            &[EtatDevoir::Donne],
            EtatDevoir::Clos,
            "devoirs.clore",
        )
        .await
    }

    /// Rouvre un devoir clos : on peut de nouveau rendre.
    pub async fn rouvrir(&self, session: &Session, id: &str) -> Resultat<DevoirAffiche> {
        self.transition(
            session,
            id,
            &[EtatDevoir::Clos],
            EtatDevoir::Donne,
            "devoirs.rouvrir",
        )
        .await
    }

    /// « Donner aussi à une autre classe » : une copie pour une autre classe de la même matière,
    /// avec ses propres rendus. La copie d'un brouillon reste un brouillon ; sinon elle est
    /// donnée aussitôt.
    pub async fn copier(
        &self,
        session: &Session,
        demande: &DemandeCopieDevoir,
    ) -> Resultat<DevoirAffiche> {
        self.exiger_module(session).await?;
        let (source, portee) = self.devoir_permis(session, &demande.id).await?;
        if source.classe_id == demande.classe_id {
            return Err(regle("devoir.meme_classe"));
        }
        let ecrit = DemandeDevoir {
            classe_id: demande.classe_id.clone(),
            matiere_id: source.matiere_id.clone(),
            titre: source.titre.clone(),
            consignes: source.consignes.clone(),
            contenu_id: source.contenu_id.clone(),
            date_limite: source.date_limite,
            bareme: i32::try_from(source.bareme).unwrap_or(BAREME_DEFAUT),
            forme_rendu: FormeRendu::depuis_nom(&source.forme_rendu),
        };
        let annee_id = self.verifier_place(session, &portee, &ecrit).await?;
        let contexte = self.contexte(session);
        let maintenant = contexte.maintenant.millis_epoque();
        let brouillon = EtatDevoir::depuis_nom(&source.etat) == EtatDevoir::Brouillon;
        let copie = LigneDevoir {
            id: nouvel_id::<()>().vers_chaine(),
            annee_id,
            classe_id: ecrit.classe_id,
            matiere_id: ecrit.matiere_id,
            auteur_id: session.utilisateur_id.vers_chaine(),
            titre: ecrit.titre,
            consignes: ecrit.consignes,
            contenu_id: ecrit.contenu_id,
            date_limite: ecrit.date_limite,
            bareme: source.bareme,
            forme_rendu: source.forme_rendu.clone(),
            etat: if brouillon {
                EtatDevoir::Brouillon
            } else {
                EtatDevoir::Donne
            }
            .nom()
            .to_owned(),
            donne_le: (!brouillon).then_some(maintenant),
            clos_le: None,
            copies_rendues_le: None,
            reporte_le: None,
            copie_de: Some(source.id.clone()),
            cree_le: maintenant,
            mis_a_jour_le: maintenant,
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        depot::inserer(unite.connexion(), &copie, &contexte).await?;
        unite.auditer(
            "devoir",
            &copie.id,
            Operation::Creation,
            None,
            Some(audit_devoir(&copie)),
            "devoirs.copier",
        );
        self.noyau.terminer(unite).await?;
        self.afficher(session, copie, &portee).await
    }
}

/// Ce que l'audit garde d'un devoir (sans les consignes, qui peuvent être longues).
fn audit_devoir(d: &LigneDevoir) -> serde_json::Value {
    serde_json::json!({
        "classe_id": d.classe_id,
        "matiere_id": d.matiere_id,
        "titre": d.titre,
        "contenu_id": d.contenu_id,
        "date_limite": d.date_limite,
        "bareme": d.bareme,
        "forme_rendu": d.forme_rendu,
        "etat": d.etat,
        "copie_de": d.copie_de,
    })
}

mod feuille;
pub use feuille::date_limite_longue;
