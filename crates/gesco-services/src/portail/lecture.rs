//! Ce que l'élève lit au portail (SPEC_PHASE_12_LOT_4 §3 et §7 ; ADR « charge du portail des
//! élèves ») : le catalogue de son niveau, une leçon en arbre de blocs, les fichiers qu'elle
//! appelle, le PDF de la leçon complète.
//!
//! Tout se lit dans une **lecture en mémoire** ([`LecturePortail`]), construite à la première
//! demande et refaite après chaque geste du personnel qui change ce que voient les élèves
//! (publication, retrait, import, installation d'un paquet, classes choisies, réglages) : une
//! classe entière qui lit ne touche presque pas la base. Seules les versions visibles y entrent,
//! sans les notes pour le maître ; ni brouillon, ni version soumise, ni liste de faits.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::future::Future;
use std::io::{Read as _, Seek as _, SeekFrom};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use gesco_core::bibliotheque::{
    ClasseChoisie, ExamenSujet, Lecon, NatureContenu, OrigineBibliotheque, SourceImage, analyser,
    classes_de_l_annee, visible_pour_classe,
};
use gesco_core::commun::Id;
use gesco_core::core::Etablissement;
use gesco_db::depots::{academique, annee_scolaire, bibliotheque as depot};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::entree::SessionEleve;
use crate::bibliotheque::{ServiceBibliotheque, type_mime_image};
use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;

/// Au-delà, la lecture se refait même sans geste signalé (un niveau ou une matière renommés
/// ailleurs) : cinq minutes.
const DUREE_LECTURE_MS: i64 = 5 * 60_000;

/// Morceau d'un son ou d'une vidéo servi par réponse, au plus : jamais un fichier entier en
/// mémoire.
pub const MORCEAU_MEDIA: u64 = 1024 * 1024;

/// Commandes du personnel après lesquelles la lecture du portail se refait : tout ce qui change
/// ce qu'un élève voit.
pub const COMMANDES_QUI_CHANGENT_LA_LECTURE: &[&str] = &[
    "bibliotheque_installer",
    "bibliotheque_publier",
    "bibliotheque_import_publier",
    "bibliotheque_importer",
    "bibliotheque_retirer",
    "bibliotheque_remettre",
    "bibliotheque_retour_livree",
    "bibliotheque_renvoyer",
    "bibliotheque_classes_regler",
    "bibliotheque_correspondance_regler",
    "bibliotheque_chapitre_ajouter",
    "bibliotheque_contenu_ajouter",
    "bibliotheque_document_fichier_ajouter",
    "bibliotheque_document_fichier_retirer",
    "bibliotheque_image_ajouter",
    "bibliotheque_son_ajouter",
    "bibliotheque_video_ajouter",
    "bibliotheque_videos_reprendre",
    "parametre_definir",
    "portail_regler",
    "sauvegarde_restaurer",
    "sauvegarde_restaurer_initiale",
];

/// Vrai si la commande change ce que lisent les élèves.
pub fn change_la_lecture(commande: &str) -> bool {
    COMMANDES_QUI_CHANGENT_LA_LECTURE.contains(&commande)
}

/// Réponse d'une route de lecture du portail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReponseLecture<T> {
    /// Le portail ne sert pas (base fermée, école absente) : 503.
    Ferme,
    /// Aucune session vivante : l'élève doit entrer de nouveau (401).
    SansSession,
    /// Rien de visible pour cette session à cette adresse (404).
    Introuvable,
    /// Ce qui est lu.
    Lu(T),
}

impl<T> ReponseLecture<T> {
    /// Ce qui est lu passe par `f` ; `None` devient « introuvable ».
    pub fn et_puis<U>(self, f: impl FnOnce(T) -> Option<U>) -> ReponseLecture<U> {
        match self {
            Self::Ferme => ReponseLecture::Ferme,
            Self::SansSession => ReponseLecture::SansSession,
            Self::Introuvable => ReponseLecture::Introuvable,
            Self::Lu(t) => f(t).map_or(ReponseLecture::Introuvable, ReponseLecture::Lu),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Ce que voit le téléphone
// ---------------------------------------------------------------------------------------------

/// Un contenu dans la liste d'un chapitre.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EntreeEleve {
    /// Identifiant.
    pub id: String,
    /// Titre.
    pub titre: String,
    /// Nature : leçon, sujet, corrigé, document.
    pub nature: NatureContenu,
    /// Examen d'un sujet ou d'un corrigé.
    pub examen: Option<ExamenSujet>,
    /// Session d'un sujet ou d'un corrigé.
    pub session: Option<u16>,
}

/// Un chapitre et ses contenus, dans l'ordre du programme.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChapitreEleve {
    /// Identifiant.
    pub id: String,
    /// Titre.
    pub titre: String,
    /// Contenus.
    pub contenus: Vec<EntreeEleve>,
}

/// Une partie d'une matière (« Algèbre ») ; titre vide : sans partie.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PartieEleve {
    /// Titre.
    pub titre: String,
    /// Chapitres.
    pub chapitres: Vec<ChapitreEleve>,
}

/// Une matière du niveau.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MatiereEleve {
    /// Clé stable de la matière.
    pub cle: String,
    /// Libellé de l'école.
    pub libelle: String,
    /// Nombre de leçons visibles.
    pub lecons: u32,
    /// Parties.
    pub parties: Vec<PartieEleve>,
}

/// Le catalogue de l'élève : les matières de son niveau.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CatalogueEleve {
    /// Libellé du niveau lu.
    pub niveau: String,
    /// Matières, dans l'ordre de l'école.
    pub matieres: Vec<MatiereEleve>,
}

/// Une image appelée par la leçon : la leçon la nomme par `cle`, le téléphone la demande par
/// son empreinte.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImageEleve {
    /// Nom dans la leçon (fichier du paquet) ou empreinte (image de l'école).
    pub cle: String,
    /// Empreinte, pour `/eleve/api/fichier/{empreinte}`.
    pub empreinte: String,
}

/// Un son ou une vidéo présents sur le poste principal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MediaEleve {
    /// Empreinte, telle que la leçon l'appelle.
    pub empreinte: String,
    /// Type MIME.
    pub type_mime: String,
}

/// Un fichier d'un document de l'école (un PDF s'ouvre dans le lecteur du navigateur).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FichierEleve {
    /// Empreinte.
    pub empreinte: String,
    /// Nom du fichier.
    pub nom: String,
    /// Type MIME.
    pub type_mime: String,
    /// Taille en octets.
    pub taille: i64,
}

/// Le contenu lié (le corrigé d'un sujet, et l'inverse), quand l'élève le voit aussi.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LienEleve {
    /// Identifiant.
    pub id: String,
    /// Titre.
    pub titre: String,
    /// Nature.
    pub nature: NatureContenu,
}

/// Un contenu ouvert par l'élève : la version visible, sans les notes pour le maître.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ContenuEleve {
    /// Identifiant.
    pub id: String,
    /// Titre.
    pub titre: String,
    /// Nature.
    pub nature: NatureContenu,
    /// Examen d'un sujet ou d'un corrigé.
    pub examen: Option<ExamenSujet>,
    /// Session d'un sujet ou d'un corrigé.
    pub session: Option<u16>,
    /// Clé de la matière.
    pub matiere_cle: String,
    /// Matière.
    pub matiere: String,
    /// Chapitre.
    pub chapitre: String,
    /// Texte en arbre de blocs.
    pub lecon: Lecon,
    /// Images appelées.
    pub images: Vec<ImageEleve>,
    /// Sons présents.
    pub sons: Vec<MediaEleve>,
    /// Vidéos présentes (aucune quand la direction a coupé les vidéos).
    pub videos: Vec<MediaEleve>,
    /// Fichiers d'un document.
    pub fichiers: Vec<FichierEleve>,
    /// Contenu lié visible.
    pub lie: Option<LienEleve>,
    /// Empreinte de la version lue (pour l'avancement).
    pub version: String,
    /// « Enregistrer en PDF » se montre.
    pub pdf: bool,
}

// ---------------------------------------------------------------------------------------------
// Lecture en mémoire
// ---------------------------------------------------------------------------------------------

/// D'où vient un fichier servi au portail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceFichier {
    /// Image d'un paquet livré, lue en mémoire.
    Paquet {
        /// Code du paquet.
        paquet: String,
        /// Clé du contenu (dossier des images).
        contenu_cle: String,
        /// Nom de l'image dans la leçon.
        nom: String,
    },
    /// Image ou fichier d'un document de l'école, dans la base.
    Base,
    /// Son de l'école, dans la base, lu par morceaux.
    Son,
    /// Vidéo de l'école, sur le disque du poste principal, lue par morceaux.
    Video(PathBuf),
}

/// Un fichier que des contenus visibles appellent.
#[derive(Debug, Clone)]
struct FichierLu {
    source: SourceFichier,
    type_mime: String,
    contenus: Vec<String>,
}

/// Un contenu visible, et ce qui décide pour qui.
#[derive(Debug, Clone)]
struct ContenuLu {
    eleve: ContenuEleve,
    livre: bool,
    chapitre_id: String,
    chapitre_ecole: bool,
    /// (code du niveau, code de la série) où il paraît.
    places: Vec<(String, Option<String>)>,
    lie_id: Option<String>,
    /// Texte visible, pour le PDF.
    paquet: Option<(String, String)>,
}

/// La bibliothèque telle que les élèves la voient, en mémoire.
pub struct LecturePortail {
    construite_le: i64,
    niveaux: Vec<gesco_core::bibliotheque::NiveauCatalogue>,
    niveau_codes: HashMap<String, String>,
    serie_codes: HashMap<String, String>,
    contenus: HashMap<String, ContenuLu>,
    fichiers: HashMap<String, FichierLu>,
    choix_contenus: HashMap<String, Vec<ClasseChoisie>>,
    choix_chapitres: HashMap<String, Vec<ClasseChoisie>>,
    annee_courante: Option<String>,
    pdf: bool,
}

impl fmt::Debug for LecturePortail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LecturePortail")
            .field("construite_le", &self.construite_le)
            .field("contenus", &self.contenus.len())
            .field("fichiers", &self.fichiers.len())
            .finish_non_exhaustive()
    }
}

/// La lecture en mémoire et sa génération : un geste du personnel l'invalide, la demande
/// suivante d'un élève la refait (une seule fois, les autres attendent).
#[derive(Default)]
pub struct CacheLecture {
    generation: AtomicU64,
    courante: tokio::sync::Mutex<Option<(u64, Arc<LecturePortail>)>>,
}

impl fmt::Debug for CacheLecture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CacheLecture")
            .field("generation", &self.generation.load(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

impl CacheLecture {
    /// La lecture doit se refaire.
    pub fn invalider(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
    }

    /// La lecture à jour ; `construire` la refait si elle a vieilli ou été invalidée.
    pub(crate) async fn obtenir<F, Fut>(
        &self,
        maintenant: i64,
        construire: F,
    ) -> Resultat<Arc<LecturePortail>>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Resultat<LecturePortail>>,
    {
        let mut courante = self.courante.lock().await;
        let generation = self.generation.load(Ordering::SeqCst);
        if let Some((g, lecture)) = courante.as_ref()
            && *g == generation
            && maintenant - lecture.construite_le < DUREE_LECTURE_MS
        {
            return Ok(Arc::clone(lecture));
        }
        let lecture = Arc::new(construire().await?);
        *courante = Some((generation, Arc::clone(&lecture)));
        Ok(lecture)
    }
}

/// Empreinte SHA-256 en hexadécimal.
fn empreinte(octets: &[u8]) -> String {
    hex::encode(Sha256::digest(octets))
}

/// Vrai pour une empreinte SHA-256 écrite en hexadécimal minuscule.
pub fn empreinte_sure(texte: &str) -> bool {
    texte.len() == 64
        && texte
            .bytes()
            .all(|o| o.is_ascii_digit() || (b'a'..=b'f').contains(&o))
}

impl LecturePortail {
    /// Construit la lecture : le catalogue des élèves (versions visibles, retraits appliqués)
    /// et chaque contenu qui a un texte ou des fichiers.
    pub(crate) async fn construire(
        bibliotheque: &ServiceBibliotheque<'_>,
        noyau: &Noyau,
        etablissement_id: Id<Etablissement>,
    ) -> Resultat<Self> {
        let range = bibliotheque.catalogue_des_eleves(etablissement_id).await?;
        let contexte = noyau.contexte(etablissement_id, None);
        let mut cnx = noyau
            .base()
            .pool()
            .acquire()
            .await
            .map_err(gesco_db::ErreurBase::from)?;
        let reglages = super::ServicePortail::lire_reglages(&mut cnx, &contexte).await?;
        let niveau_codes: HashMap<String, String> = academique::niveaux(&mut cnx, &contexte)
            .await?
            .into_iter()
            .map(|n| (n.id.vers_chaine(), n.code))
            .collect();
        let serie_codes: HashMap<String, String> = academique::series(&mut cnx, &contexte)
            .await?
            .into_iter()
            .map(|s| (s.id.vers_chaine(), s.code))
            .collect();
        let publiees: HashMap<String, (String, bool)> =
            depot::versions_publiees(&mut cnx, &contexte)
                .await?
                .into_iter()
                .map(|(id, texte, retour)| (id, (texte, retour)))
                .collect();
        let mut choix_contenus: HashMap<String, Vec<ClasseChoisie>> = HashMap::new();
        let mut choix_chapitres: HashMap<String, Vec<ClasseChoisie>> = HashMap::new();
        for l in depot::classes_choisies(&mut cnx, &contexte).await? {
            let choix = ClasseChoisie {
                classe_id: l.classe_id,
                annee_id: l.annee_id,
            };
            match (l.contenu_id, l.chapitre_id) {
                (Some(c), _) => choix_contenus.entry(c).or_default().push(choix),
                (None, Some(c)) => choix_chapitres.entry(c).or_default().push(choix),
                (None, None) => {}
            }
        }
        let annee_courante = annee_scolaire::courante(&mut cnx, &contexte)
            .await?
            .map(|a| a.id.vers_chaine());
        let dossier_videos = bibliotheque.dossier_videos();
        let videos_connues: HashSet<String> = if reglages.videos {
            depot::videos(&mut cnx, &contexte)
                .await?
                .into_iter()
                .map(|v| v.empreinte_sha256)
                .filter(|e| dossier_videos.join(format!("{e}.mp4")).exists())
                .collect()
        } else {
            HashSet::new()
        };

        let par_id: HashMap<String, &gesco_core::bibliotheque::ContenuBibliotheque> = range
            .contenus
            .iter()
            .map(|c| (c.id.vers_chaine(), c))
            .collect();
        let par_cle: HashMap<&str, String> = range
            .contenus
            .iter()
            .map(|c| (c.cle.as_str(), c.id.vers_chaine()))
            .collect();
        let chapitres_ecole: HashSet<String> = range
            .chapitres
            .iter()
            .filter(|c| c.origine == OrigineBibliotheque::Ecole)
            .map(|c| c.id.vers_chaine())
            .collect();

        let mut contenus: HashMap<String, ContenuLu> = HashMap::new();
        let mut fichiers: HashMap<String, FichierLu> = HashMap::new();
        let mut sans_texte: HashSet<String> = HashSet::new();
        for niveau in &range.niveaux {
            for serie in &niveau.series {
                for matiere in &serie.matieres {
                    for partie in &matiere.parties {
                        for chapitre in &partie.chapitres {
                            for entree in &chapitre.contenus {
                                let place = (niveau.code.clone(), serie.code.clone());
                                if let Some(lu) = contenus.get_mut(&entree.id) {
                                    lu.places.push(place);
                                    continue;
                                }
                                if sans_texte.contains(&entree.id) {
                                    continue;
                                }
                                let Some(contenu) = par_id.get(&entree.id) else {
                                    continue;
                                };
                                let texte = match publiees.get(&entree.id) {
                                    Some((texte, false)) => Some(texte.clone()),
                                    _ if contenu.plus_livre => None,
                                    _ => bibliotheque.texte_livre(contenu),
                                }
                                .unwrap_or_default();
                                let lecon = analyser(&texte).pour_les_eleves();
                                let documents = if contenu.nature == NatureContenu::Document {
                                    depot::fichiers_du_contenu(&mut cnx, &entree.id, &contexte)
                                        .await?
                                } else {
                                    Vec::new()
                                };
                                // Un contenu sans texte n'est pas montré.
                                if lecon.blocs.is_empty() && documents.is_empty() {
                                    sans_texte.insert(entree.id.clone());
                                    continue;
                                }
                                let mut noter = |emp: &str, source: SourceFichier, mime: &str| {
                                    let f = fichiers.entry(emp.to_owned()).or_insert_with(|| {
                                        FichierLu {
                                            source,
                                            type_mime: mime.to_owned(),
                                            contenus: Vec::new(),
                                        }
                                    });
                                    if !f.contenus.contains(&entree.id) {
                                        f.contenus.push(entree.id.clone());
                                    }
                                };
                                let mut images = Vec::new();
                                for source in lecon.images() {
                                    match source {
                                        SourceImage::Paquet { nom } => {
                                            let Some(paquet) = contenu.paquet_code.as_deref()
                                            else {
                                                continue;
                                            };
                                            let Some(octets) = bibliotheque.image_du_paquet(
                                                paquet,
                                                &contenu.cle,
                                                nom,
                                            ) else {
                                                continue;
                                            };
                                            let e = empreinte(&octets);
                                            noter(
                                                &e,
                                                SourceFichier::Paquet {
                                                    paquet: paquet.to_owned(),
                                                    contenu_cle: contenu.cle.clone(),
                                                    nom: nom.clone(),
                                                },
                                                type_mime_image(nom),
                                            );
                                            images.push(ImageEleve {
                                                cle: nom.clone(),
                                                empreinte: e,
                                            });
                                        }
                                        SourceImage::Ecole { empreinte: e } => {
                                            if !empreinte_sure(e) {
                                                continue;
                                            }
                                            noter(e, SourceFichier::Base, "");
                                            images.push(ImageEleve {
                                                cle: e.clone(),
                                                empreinte: e.clone(),
                                            });
                                        }
                                    }
                                }
                                images.sort_by(|a, b| a.cle.cmp(&b.cle));
                                images.dedup_by(|a, b| a.cle == b.cle);
                                let appeles: Vec<String> =
                                    lecon.sons().into_iter().map(str::to_owned).collect();
                                let mut sons = Vec::new();
                                for (e, mime, _) in
                                    depot::sons_par_empreinte(&mut cnx, &appeles, &contexte).await?
                                {
                                    noter(&e, SourceFichier::Son, &mime);
                                    sons.push(MediaEleve {
                                        empreinte: e,
                                        type_mime: mime,
                                    });
                                }
                                let mut videos = Vec::new();
                                for e in lecon.videos() {
                                    if videos_connues.contains(e)
                                        && !videos.iter().any(|v: &MediaEleve| v.empreinte == e)
                                    {
                                        noter(
                                            e,
                                            SourceFichier::Video(
                                                dossier_videos.join(format!("{e}.mp4")),
                                            ),
                                            "video/mp4",
                                        );
                                        videos.push(MediaEleve {
                                            empreinte: e.to_owned(),
                                            type_mime: "video/mp4".to_owned(),
                                        });
                                    }
                                }
                                let mut fichiers_doc = Vec::new();
                                for f in documents {
                                    noter(&f.empreinte_sha256, SourceFichier::Base, &f.type_mime);
                                    fichiers_doc.push(FichierEleve {
                                        empreinte: f.empreinte_sha256,
                                        nom: f.nom_fichier,
                                        type_mime: f.type_mime,
                                        taille: f.taille,
                                    });
                                }
                                let mut a_hacher = texte.clone().into_bytes();
                                for f in &fichiers_doc {
                                    a_hacher.extend_from_slice(f.empreinte.as_bytes());
                                }
                                let chapitre_id = contenu.chapitre_id.vers_chaine();
                                let pdf = reglages.pdf && !lecon.blocs.is_empty();
                                contenus.insert(
                                    entree.id.clone(),
                                    ContenuLu {
                                        eleve: ContenuEleve {
                                            id: entree.id.clone(),
                                            titre: contenu.titre.clone(),
                                            nature: contenu.nature,
                                            examen: contenu.examen,
                                            session: contenu.session,
                                            matiere_cle: matiere.cle.clone(),
                                            matiere: matiere.libelle.clone(),
                                            chapitre: chapitre.titre.clone(),
                                            lecon,
                                            images,
                                            sons,
                                            videos,
                                            fichiers: fichiers_doc,
                                            lie: None,
                                            version: empreinte(&a_hacher),
                                            pdf,
                                        },
                                        livre: contenu.origine == OrigineBibliotheque::Livre,
                                        chapitre_ecole: chapitres_ecole.contains(&chapitre_id),
                                        chapitre_id,
                                        places: vec![place],
                                        lie_id: contenu
                                            .lie_cle
                                            .as_deref()
                                            .and_then(|cle| par_cle.get(cle).cloned()),
                                        paquet: contenu
                                            .paquet_code
                                            .clone()
                                            .map(|p| (p, contenu.cle.clone())),
                                    },
                                );
                            }
                        }
                    }
                }
            }
        }
        Ok(Self {
            construite_le: noyau.horloge().maintenant().millis_epoque(),
            niveaux: range.niveaux,
            niveau_codes,
            serie_codes,
            contenus,
            fichiers,
            choix_contenus,
            choix_chapitres,
            annee_courante,
            pdf: reglages.pdf,
        })
    }

    /// Codes du niveau et de la série de la session.
    fn codes_de(&self, session: &SessionEleve) -> (Option<&String>, Option<&String>) {
        (
            self.niveau_codes.get(&session.niveau_id),
            session
                .serie_id
                .as_ref()
                .and_then(|s| self.serie_codes.get(s)),
        )
    }

    /// Une série du catalogue se lit dans cette session : sans code (tout le niveau), ou celle
    /// de l'élève.
    fn serie_lue(code: Option<&String>, serie_eleve: Option<&String>) -> bool {
        code.is_none() || code == serie_eleve
    }

    /// Le contenu se montre à cet élève d'après les classes choisies : un contenu livré est pour
    /// tout le niveau ; un ajout de l'école réservé à des classes ne se montre qu'à elles, et
    /// jamais à qui lit sans code.
    fn pour_la_classe(&self, lu: &ContenuLu, session: &SessionEleve) -> bool {
        if lu.livre {
            return true;
        }
        let vide = Vec::new();
        let du_contenu = self.choix_contenus.get(&lu.eleve.id).unwrap_or(&vide);
        let du_chapitre = if lu.chapitre_ecole {
            self.choix_chapitres.get(&lu.chapitre_id).unwrap_or(&vide)
        } else {
            &vide
        };
        match (&session.classe_id, &session.annee_id) {
            (Some(classe), Some(annee)) => {
                visible_pour_classe(du_contenu, du_chapitre, classe, annee)
            }
            _ => match &self.annee_courante {
                Some(annee) => {
                    classes_de_l_annee(du_contenu, annee).is_empty()
                        && classes_de_l_annee(du_chapitre, annee).is_empty()
                }
                None => true,
            },
        }
    }

    /// Le contenu est visible pour cette session : de son niveau (et de sa série), pour sa
    /// classe.
    pub fn visible(&self, contenu_id: &str, session: &SessionEleve) -> bool {
        let Some(lu) = self.contenus.get(contenu_id) else {
            return false;
        };
        let (niveau, serie) = self.codes_de(session);
        let Some(niveau) = niveau else {
            return false;
        };
        lu.places
            .iter()
            .any(|(n, s)| n == niveau && Self::serie_lue(s.as_ref(), serie))
            && self.pour_la_classe(lu, session)
    }

    /// Le catalogue de l'élève : les matières de son niveau, leurs parties et chapitres dans
    /// l'ordre du programme, avec seulement ce qu'il voit ; une matière ou un chapitre vides ne
    /// paraissent pas.
    pub fn catalogue(&self, session: &SessionEleve) -> CatalogueEleve {
        let (code_niveau, code_serie) = self.codes_de(session);
        let mut matieres: Vec<MatiereEleve> = Vec::new();
        let niveau = self.niveaux.iter().find(|n| Some(&n.code) == code_niveau);
        for serie in niveau.iter().flat_map(|n| n.series.iter()) {
            if !Self::serie_lue(serie.code.as_ref(), code_serie) {
                continue;
            }
            for matiere in &serie.matieres {
                let mut parties = Vec::new();
                for partie in &matiere.parties {
                    let chapitres: Vec<ChapitreEleve> = partie
                        .chapitres
                        .iter()
                        .map(|c| ChapitreEleve {
                            id: c.id.clone(),
                            titre: c.titre.clone(),
                            contenus: c
                                .contenus
                                .iter()
                                .filter(|e| self.visible(&e.id, session))
                                .map(|e| EntreeEleve {
                                    id: e.id.clone(),
                                    titre: e.titre.clone(),
                                    nature: e.nature,
                                    examen: e.examen,
                                    session: e.session,
                                })
                                .collect(),
                        })
                        .filter(|c| !c.contenus.is_empty())
                        .collect();
                    if !chapitres.is_empty() {
                        parties.push(PartieEleve {
                            titre: partie.titre.clone(),
                            chapitres,
                        });
                    }
                }
                if parties.is_empty() {
                    continue;
                }
                let lecons = parties
                    .iter()
                    .flat_map(|p| p.chapitres.iter())
                    .flat_map(|c| c.contenus.iter())
                    .filter(|e| e.nature == NatureContenu::Lecon)
                    .count();
                let lecons = u32::try_from(lecons).unwrap_or(u32::MAX);
                match matieres.iter_mut().find(|m| m.cle == matiere.cle) {
                    Some(m) => {
                        m.parties.extend(parties);
                        m.lecons = m.lecons.saturating_add(lecons);
                    }
                    None => matieres.push(MatiereEleve {
                        cle: matiere.cle.clone(),
                        libelle: matiere.libelle.clone(),
                        lecons,
                        parties,
                    }),
                }
            }
        }
        CatalogueEleve {
            niveau: session.niveau.clone(),
            matieres,
        }
    }

    /// Un contenu visible pour cette session ; son lien seulement s'il est visible aussi.
    pub fn contenu(&self, contenu_id: &str, session: &SessionEleve) -> Option<ContenuEleve> {
        if !self.visible(contenu_id, session) {
            return None;
        }
        let lu = self.contenus.get(contenu_id)?;
        let mut eleve = lu.eleve.clone();
        eleve.lie = lu
            .lie_id
            .as_deref()
            .filter(|id| self.visible(id, session))
            .and_then(|id| self.contenus.get(id))
            .map(|l| LienEleve {
                id: l.eleve.id.clone(),
                titre: l.eleve.titre.clone(),
                nature: l.eleve.nature,
            });
        Some(eleve)
    }

    /// Un fichier appelé par un contenu visible pour cette session : sa source et son type.
    pub fn fichier(
        &self,
        empreinte: &str,
        session: &SessionEleve,
    ) -> Option<(SourceFichier, String)> {
        let f = self.fichiers.get(empreinte)?;
        f.contenus
            .iter()
            .any(|id| self.visible(id, session))
            .then(|| (f.source.clone(), f.type_mime.clone()))
    }

    /// Le PDF est ouvert par la direction pour ce contenu.
    pub fn pdf_permis(&self, contenu_id: &str) -> bool {
        self.pdf && self.contenus.get(contenu_id).is_some_and(|l| l.eleve.pdf)
    }

    /// Paquet et clé d'un contenu livré, pour retrouver ses images.
    fn paquet_de(&self, contenu_id: &str) -> Option<&(String, String)> {
        self.contenus.get(contenu_id)?.paquet.as_ref()
    }

    /// Nombre de contenus visibles (essais, mesure).
    pub fn nombre_de_contenus(&self) -> usize {
        self.contenus.len()
    }
}

// ---------------------------------------------------------------------------------------------
// Fichiers et PDF
// ---------------------------------------------------------------------------------------------

/// Une plage d'octets demandée : début, fin incluse.
pub type Plage = (u64, Option<u64>);

/// Un fichier, ou un morceau d'un son ou d'une vidéo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MorceauFichier {
    /// Type MIME.
    pub type_mime: String,
    /// Taille du fichier entier.
    pub taille: u64,
    /// Premier octet servi.
    pub debut: u64,
    /// Octets servis.
    pub octets: Vec<u8>,
    /// Réponse partielle (206).
    pub partiel: bool,
}

/// Issue de la lecture d'un fichier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FichierLuPortail {
    /// Le fichier ou son morceau.
    Servi(MorceauFichier),
    /// La plage tombe hors du fichier (416).
    HorsPlage {
        /// Taille du fichier.
        taille: u64,
    },
    /// Fichier absent (effacé, paquet non lu) : la leçon s'ouvre sans lui.
    Absent,
}

/// Début et longueur à servir d'un média de `taille` octets ; `None` hors du fichier.
pub fn morceau_a_servir(plage: Option<Plage>, taille: u64) -> Option<(u64, u64)> {
    let (debut, fin) = plage.unwrap_or((0, None));
    if debut >= taille && taille > 0 {
        return None;
    }
    let fin = fin.unwrap_or(u64::MAX).min(taille.saturating_sub(1));
    let longueur = (fin.saturating_sub(debut) + 1).min(MORCEAU_MEDIA);
    Some((debut, if taille == 0 { 0 } else { longueur }))
}

/// Lit un fichier du portail. Un son et une vidéo se lisent par morceaux, jamais en entier ;
/// une image et un fichier de document se servent entiers.
pub(crate) async fn lire_fichier(
    bibliotheque: &ServiceBibliotheque<'_>,
    noyau: &Noyau,
    etablissement_id: Id<Etablissement>,
    empreinte_demandee: &str,
    source: SourceFichier,
    type_mime: String,
    plage: Option<Plage>,
) -> Resultat<FichierLuPortail> {
    let entier = |type_mime: String, octets: Vec<u8>| {
        FichierLuPortail::Servi(MorceauFichier {
            type_mime,
            taille: octets.len() as u64,
            debut: 0,
            octets,
            partiel: false,
        })
    };
    match source {
        SourceFichier::Paquet {
            paquet,
            contenu_cle,
            nom,
        } => Ok(
            match bibliotheque.image_du_paquet(&paquet, &contenu_cle, &nom) {
                Some(octets) => entier(type_mime, octets),
                None => FichierLuPortail::Absent,
            },
        ),
        SourceFichier::Base => {
            let contexte = noyau.contexte(etablissement_id, None);
            let mut cnx = noyau
                .base()
                .pool()
                .acquire()
                .await
                .map_err(gesco_db::ErreurBase::from)?;
            Ok(
                match depot::fichier_par_empreinte(&mut cnx, empreinte_demandee, &contexte).await? {
                    Some((_, mime, octets)) => entier(mime, octets),
                    None => FichierLuPortail::Absent,
                },
            )
        }
        SourceFichier::Son => {
            let contexte = noyau.contexte(etablissement_id, None);
            let mut cnx = noyau
                .base()
                .pool()
                .acquire()
                .await
                .map_err(gesco_db::ErreurBase::from)?;
            let Some((taille, _)) =
                depot::morceau_son(&mut cnx, empreinte_demandee, 0, 0, &contexte).await?
            else {
                return Ok(FichierLuPortail::Absent);
            };
            let taille = u64::try_from(taille).unwrap_or(0);
            let Some((debut, longueur)) = morceau_a_servir(plage, taille) else {
                return Ok(FichierLuPortail::HorsPlage { taille });
            };
            let octets = depot::morceau_son(
                &mut cnx,
                empreinte_demandee,
                i64::try_from(debut).unwrap_or(i64::MAX),
                i64::try_from(longueur).unwrap_or(0),
                &contexte,
            )
            .await?
            .map(|(_, o)| o)
            .unwrap_or_default();
            Ok(FichierLuPortail::Servi(MorceauFichier {
                type_mime,
                taille,
                debut,
                partiel: plage.is_some() || (octets.len() as u64) < taille,
                octets,
            }))
        }
        SourceFichier::Video(chemin) => {
            tokio::task::spawn_blocking(move || -> Resultat<FichierLuPortail> {
                let Ok(mut fichier) = std::fs::File::open(&chemin) else {
                    return Ok(FichierLuPortail::Absent);
                };
                let taille = fichier.metadata()?.len();
                let Some((debut, longueur)) = morceau_a_servir(plage, taille) else {
                    return Ok(FichierLuPortail::HorsPlage { taille });
                };
                fichier.seek(SeekFrom::Start(debut))?;
                let mut octets = Vec::new();
                fichier.take(longueur).read_to_end(&mut octets)?;
                Ok(FichierLuPortail::Servi(MorceauFichier {
                    type_mime,
                    taille,
                    debut,
                    partiel: plage.is_some() || (octets.len() as u64) < taille,
                    octets,
                }))
            })
            .await
            .map_err(|e| ErreurService::Secret(e.to_string()))?
        }
    }
}

/// Chemin du PDF gardé d'une version visible.
fn chemin_pdf(dossier: &std::path::Path, contenu: &ContenuEleve) -> PathBuf {
    let version = contenu.version.get(..16).unwrap_or(&contenu.version);
    dossier.join(format!("{}-{version}.pdf", contenu.id))
}

/// Ce qu'il faut pour composer le PDF d'une leçon : la leçon, sa place, ses images.
pub(crate) struct APdf {
    /// Le contenu.
    pub contenu: ContenuEleve,
    /// Libellé du niveau.
    pub niveau: String,
    /// Images : (clé dans la leçon, type MIME, octets).
    pub images: Vec<(String, String, Vec<u8>)>,
}

/// Le PDF déjà fabriqué pour cette version, s'il est gardé.
pub(crate) fn pdf_garde(dossier: &std::path::Path, contenu: &ContenuEleve) -> Option<Vec<u8>> {
    std::fs::read(chemin_pdf(dossier, contenu)).ok()
}

/// Garde le PDF fabriqué ; une écriture manquée n'empêche rien (il se refera).
pub(crate) fn garder_pdf(dossier: &std::path::Path, contenu: &ContenuEleve, pdf: &[u8]) {
    if std::fs::create_dir_all(dossier).is_ok() {
        let _ = crate::espace::ecrire_atomiquement(&chemin_pdf(dossier, contenu), pdf);
    }
}

/// Rassemble les images d'une leçon pour son PDF.
pub(crate) async fn preparer_pdf(
    lecture: &LecturePortail,
    bibliotheque: &ServiceBibliotheque<'_>,
    noyau: &Noyau,
    etablissement_id: Id<Etablissement>,
    contenu: ContenuEleve,
    niveau: String,
) -> Resultat<APdf> {
    let mut images = Vec::new();
    let paquet = lecture.paquet_de(&contenu.id);
    let contexte = noyau.contexte(etablissement_id, None);
    let mut cnx = noyau
        .base()
        .pool()
        .acquire()
        .await
        .map_err(gesco_db::ErreurBase::from)?;
    let mut de_l_ecole = Vec::new();
    for source in contenu.lecon.images() {
        match source {
            SourceImage::Paquet { nom } => {
                if let Some((code, cle)) = paquet
                    && let Some(octets) = bibliotheque.image_du_paquet(code, cle, nom)
                {
                    images.push((nom.clone(), type_mime_image(nom).to_owned(), octets));
                }
            }
            SourceImage::Ecole { empreinte } => de_l_ecole.push(empreinte.clone()),
        }
    }
    de_l_ecole.sort();
    de_l_ecole.dedup();
    for (e, mime, octets) in depot::images_par_empreinte(&mut cnx, &de_l_ecole, &contexte).await? {
        images.push((e, mime, octets));
    }
    Ok(APdf {
        contenu,
        niveau,
        images,
    })
}

/// Nom du PDF proposé au téléphone : le titre, sans caractère qui gêne un nom de fichier.
pub fn nom_du_pdf(titre: &str) -> String {
    let propre: String = titre
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '\'') {
                c
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let propre: String = propre.chars().take(80).collect();
    if propre.is_empty() {
        "Lecon.pdf".to_owned()
    } else {
        format!("{propre}.pdf")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_media_se_sert_par_morceaux() {
        let taille = 5 * MORCEAU_MEDIA;
        assert_eq!(morceau_a_servir(None, taille), Some((0, MORCEAU_MEDIA)));
        assert_eq!(
            morceau_a_servir(Some((100, Some(199))), taille),
            Some((100, 100))
        );
        assert_eq!(
            morceau_a_servir(Some((taille - 10, None)), taille),
            Some((taille - 10, 10))
        );
        assert_eq!(morceau_a_servir(Some((taille, None)), taille), None);
        assert_eq!(morceau_a_servir(None, 10), Some((0, 10)));
    }

    #[test]
    fn seules_les_empreintes_propres_passent() {
        assert!(empreinte_sure(&"a1".repeat(32)));
        assert!(!empreinte_sure("../../base"));
        assert!(!empreinte_sure(&"A1".repeat(32)));
    }

    #[test]
    fn le_nom_du_pdf_vient_du_titre() {
        assert_eq!(
            nom_du_pdf("La racine carrée / d'un nombre"),
            "La racine carrée d'un nombre.pdf"
        );
        assert_eq!(nom_du_pdf("///"), "Lecon.pdf");
    }

    #[test]
    fn les_gestes_qui_changent_la_lecture_sont_connus() {
        assert!(change_la_lecture("bibliotheque_publier"));
        assert!(change_la_lecture("bibliotheque_retirer"));
        assert!(!change_la_lecture("bibliotheque_lire"));
        assert!(!change_la_lecture("caisse_encaisser"));
    }
}
