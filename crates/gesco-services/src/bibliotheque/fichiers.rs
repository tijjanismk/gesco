//! Images, documents et vidéos de l'école (§10 de la spécification ; ADR « stockage des
//! contenus »).
//!
//! - Une image (SVG nettoyé, PNG, JPEG ; 5 Mo au plus) se range dans la base par son empreinte ;
//!   le texte l'appelle par `image:<empreinte>`.
//! - Un document de l'école reçoit ses fichiers (PDF ; 20 Mo au plus, ou une image) dans l'ordre ;
//!   un fichier retiré du document reste dans la base.
//! - Une vidéo MP4 est copiée dans `videos` du poste principal, nommée par son empreinte, hors de
//!   la base et de la sauvegarde ; le texte l'appelle par `video:<empreinte>`. Au-delà de la
//!   taille conseillée, l'écran demande une confirmation : jamais de refus pour la taille. Elle
//!   se lit par morceaux, sur le poste principal comme depuis un poste relié ; un poste relié
//!   l'envoie aussi par morceaux. Deux gestes la copient vers une clé et l'en reprennent.
//! - Un son (MP3, M4A, OGG, WAV ; 10 Mo au plus) se range dans la base comme une image, donc
//!   dans la sauvegarde ; le texte l'appelle par `son:<empreinte>`. Il se lit par morceaux par la
//!   même commande que les vidéos, sur le poste principal comme depuis un poste relié.

use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use base64::Engine as _;
use gesco_core::bibliotheque::paquet::empreinte;
use gesco_core::bibliotheque::reglages::{TAILLE_VIDEO_CONSEILLEE, TAILLE_VIDEO_CONSEILLEE_DEFAUT};
use gesco_core::bibliotheque::{
    MORCEAU_VIDEO, NatureContenu, OrigineBibliotheque, SorteFichier, TAILLE_DOCUMENT_MAX,
    TAILLE_IMAGE_MAX, TAILLE_SON_MAX, est_empreinte, nettoyer_svg, reconnaitre, reconnaitre_son,
};
use gesco_core::core::{Operation, TypeEvenement};
use gesco_db::depots::bibliotheque::{self as depot, FichierJoint};
use gesco_db::depots::parametre;
use gesco_db::unite::Durabilite;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{ServiceBibliotheque, regle};
use crate::erreur::{ErreurService, Resultat};
use crate::session::Session;

/// Ajouter une image à la bibliothèque.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeImageBibliotheque {
    /// Chemin du fichier.
    pub chemin: String,
}

/// Une image rangée : ce que l'éditeur écrit dans le texte.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ImageAjoutee {
    /// Appel à écrire dans le texte : `image:<empreinte>`.
    pub reference: String,
    /// Nom du fichier, proposé comme description.
    pub nom: String,
}

/// Ajouter un son à la bibliothèque.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeSonBibliotheque {
    /// Chemin du fichier.
    pub chemin: String,
}

/// Un son rangé : ce que l'éditeur écrit dans le texte.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct SonAjoute {
    /// Appel à écrire dans le texte : `son:<empreinte>`.
    pub reference: String,
    /// Nom du fichier sans son extension, proposé comme titre.
    pub nom: String,
}

/// Un son appelé par une leçon, tel que la lecture le décrit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct SonLecon {
    /// Empreinte.
    pub empreinte: String,
    /// Le son est dans la base de l'école.
    pub disponible: bool,
    /// Type MIME, vide s'il est absent.
    pub type_mime: String,
}

/// Joindre un fichier à un document de l'école.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeFichierDocument {
    /// Identifiant du document.
    pub contenu_id: String,
    /// Chemin du fichier.
    pub chemin: String,
}

/// Retirer un fichier d'un document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeRetraitFichierDocument {
    /// Identifiant du document.
    pub contenu_id: String,
    /// Ligne qui joint le fichier.
    pub lien_id: String,
}

/// Un fichier d'un document, tel que l'écran le montre.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct FichierDocument {
    /// Ligne qui joint le fichier.
    pub lien_id: String,
    /// Empreinte du fichier.
    pub empreinte: String,
    /// Nom du fichier.
    pub nom: String,
    /// Type MIME.
    pub type_mime: String,
    /// Taille en octets.
    pub taille: i64,
}

/// Enregistrer un fichier de la bibliothèque sous un chemin choisi.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeFichierEnregistrer {
    /// Empreinte du fichier.
    pub empreinte: String,
    /// Chemin où l'écrire.
    pub chemin: String,
}

/// Ajouter une vidéo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeVideoBibliotheque {
    /// Chemin du fichier.
    pub chemin: String,
    /// L'utilisateur a confirmé l'ajout d'une vidéo plus lourde que la taille conseillée.
    pub confirme: bool,
}

/// Ce que devient une vidéo proposée.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct VideoAjoutee {
    /// Faux tant qu'une vidéo lourde attend la confirmation.
    pub ajoutee: bool,
    /// La vidéo dépasse la taille conseillée.
    pub lourde: bool,
    /// Appel à écrire dans le texte : `video:<empreinte>` ; vide si elle n'est pas ajoutée.
    pub reference: String,
    /// Nom du fichier, proposé comme titre.
    pub nom: String,
    /// Première vidéo de l'école : l'écran dit une fois qu'elle n'est pas dans la sauvegarde.
    pub premiere: bool,
}

/// Lire un morceau de vidéo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeMorceauVideo {
    /// Empreinte de la vidéo.
    pub empreinte: String,
    /// Position du morceau, en octets.
    pub position: i64,
}

/// Un morceau de vidéo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct MorceauVideo {
    /// Taille de la vidéo entière, en octets.
    pub taille: i64,
    /// Morceau en base 64 ; vide après la fin.
    pub contenu: String,
}

/// Un morceau de vidéo envoyé par un poste relié.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeEnvoiMorceauVideo {
    /// Identifiant de l'envoi, tiré par le poste relié.
    pub envoi: String,
    /// Position du morceau, en octets : la taille déjà reçue.
    pub position: i64,
    /// Morceau en base 64.
    pub contenu: String,
}

/// Copier les vidéos vers un dossier, ou les en reprendre.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeDossierVideos {
    /// Dossier choisi (une clé, un disque).
    pub dossier: String,
}

/// Bilan d'une copie ou d'une reprise de vidéos.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct BilanVideos {
    /// Vidéos copiées.
    pub copiees: u32,
    /// Vidéos déjà présentes.
    pub deja_la: u32,
    /// Vidéos de l'école qu'on n'a trouvées nulle part.
    pub absentes: u32,
}

/// Une vidéo appelée par une leçon, telle que la lecture la décrit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct VideoLecon {
    /// Empreinte.
    pub empreinte: String,
    /// Le fichier est présent sur le poste principal.
    pub disponible: bool,
    /// Taille en octets.
    pub taille: i64,
}

/// Sous-dossier du dossier des vidéos où l'école les copie sur une clé.
const DOSSIER_COPIE: &str = "GESCO-videos";

/// Fichier d'une vidéo dans `videos`.
fn chemin_video(dossier: &Path, empreinte: &str) -> PathBuf {
    dossier.join(format!("{empreinte}.mp4"))
}

/// Nom de fichier sans dossier, pour la base.
fn nom_du_fichier(chemin: &Path) -> String {
    chemin
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "fichier".to_owned())
}

/// Empreinte SHA-256 d'un fichier lu par morceaux, et sa taille.
fn empreinte_fichier(chemin: &Path) -> std::io::Result<(String, u64)> {
    let mut fichier = std::fs::File::open(chemin)?;
    let mut hacheur = Sha256::new();
    let mut tampon = vec![0_u8; 1024 * 1024];
    let mut taille = 0_u64;
    loop {
        let lus = fichier.read(&mut tampon)?;
        if lus == 0 {
            break;
        }
        hacheur.update(tampon.get(..lus).unwrap_or_default());
        taille += lus as u64;
    }
    Ok((hex::encode(hacheur.finalize()), taille))
}

/// Premiers octets d'un fichier.
fn debut_du_fichier(chemin: &Path) -> std::io::Result<Vec<u8>> {
    let mut tampon = Vec::with_capacity(4096);
    std::fs::File::open(chemin)?
        .take(4096)
        .read_to_end(&mut tampon)?;
    Ok(tampon)
}

/// Copie un fichier sous un nom temporaire, puis le renomme : une copie interrompue ne laisse
/// jamais un fichier tronqué sous le nom final.
fn copier_sur(source: &Path, destination: &Path) -> Resultat<()> {
    let provisoire = destination.with_extension("part");
    std::fs::copy(source, &provisoire).map_err(|e| {
        let _ = std::fs::remove_file(&provisoire);
        if e.kind() == std::io::ErrorKind::StorageFull {
            regle("bibliotheque.place_insuffisante_video")
        } else {
            ErreurService::Fichier(e)
        }
    })?;
    std::fs::rename(&provisoire, destination)?;
    Ok(())
}

/// Identifiant d'envoi sûr : lettres et chiffres, de 16 à 64 caractères.
pub(crate) fn envoi_sur(envoi: &str) -> bool {
    (16..=64).contains(&envoi.len()) && envoi.bytes().all(|o| o.is_ascii_alphanumeric())
}

/// Fichier où s'assemblent les morceaux d'un envoi.
pub(crate) fn chemin_envoi(espace: &crate::espace::EspaceDonnees, envoi: &str) -> PathBuf {
    crate::fichiers_relies::dossier_de_travail(espace)
        .join("envois")
        .join(format!("{envoi}.part"))
}

fn vers_ecran(f: FichierJoint) -> FichierDocument {
    FichierDocument {
        lien_id: f.lien_id,
        empreinte: f.empreinte_sha256,
        nom: f.nom_fichier,
        type_mime: f.type_mime,
        taille: f.taille,
    }
}

impl ServiceBibliotheque<'_> {
    /// Lit, borne et nettoie un fichier : (sorte, octets à ranger).
    fn lire_fichier(chemin: &Path, documents: bool) -> Resultat<(SorteFichier, Vec<u8>)> {
        let taille = std::fs::metadata(chemin)?.len();
        let debut = debut_du_fichier(chemin)?;
        let sorte =
            reconnaitre(&debut).filter(|s| s.est_image() || (documents && *s == SorteFichier::Pdf));
        let Some(sorte) = sorte else {
            return Err(regle(if documents {
                "bibliotheque.document_inconnu"
            } else {
                "bibliotheque.image_inconnue"
            }));
        };
        if sorte == SorteFichier::Pdf {
            if taille > TAILLE_DOCUMENT_MAX {
                return Err(regle("bibliotheque.document_trop_lourd"));
            }
        } else if taille > TAILLE_IMAGE_MAX {
            return Err(regle("bibliotheque.image_trop_lourde"));
        }
        let octets = std::fs::read(chemin)?;
        let octets = if sorte == SorteFichier::Svg {
            nettoyer_svg(&octets)
                .map_err(|_| regle("bibliotheque.svg_refuse"))?
                .into_bytes()
        } else {
            octets
        };
        Ok((sorte, octets))
    }

    /// Range un fichier lu dans la base, avec son audit et son événement s'il est nouveau.
    async fn ranger_fichier(
        &self,
        session: &Session,
        chemin: &Path,
        sorte: SorteFichier,
        octets: &[u8],
        joindre_a: Option<&str>,
    ) -> Resultat<String> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let portee = self.portee(session).await?;
        let cle = empreinte(octets);
        let nom = nom_du_fichier(chemin);
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        if let Some(contenu_id) = joindre_a {
            let contenu =
                Self::contenu_permis(unite.connexion(), &portee, contenu_id, &contexte).await?;
            if contenu.nature != NatureContenu::Document
                || contenu.origine != OrigineBibliotheque::Ecole
            {
                return Err(regle("bibliotheque.pas_un_document"));
            }
        }
        let nature = if sorte == SorteFichier::Pdf {
            "document"
        } else if sorte.est_son() {
            "son"
        } else {
            "image"
        };
        let (fichier_id, nouveau) = depot::enregistrer_fichier(
            unite.connexion(),
            nature,
            sorte.type_mime(),
            &nom,
            &cle,
            octets,
            &contexte,
        )
        .await?;
        if nouveau {
            unite.auditer(
                "fichier_bibliotheque",
                &fichier_id,
                Operation::Creation,
                None,
                Some(serde_json::json!({
                    "nom_fichier": nom,
                    "type_mime": sorte.type_mime(),
                    "taille": octets.len(),
                    "empreinte_sha256": cle,
                })),
                "bibliotheque.fichier_ajouter",
            );
            unite.emettre(
                TypeEvenement::FichierBibliothequeAjoute,
                "fichier_bibliotheque",
                &fichier_id,
                serde_json::json!({ "empreinte_sha256": cle, "nature": nature }),
            );
        }
        if let Some(contenu_id) = joindre_a {
            let lien =
                depot::joindre_fichier(unite.connexion(), contenu_id, &fichier_id, &contexte)
                    .await?;
            unite.auditer(
                "fichier_contenu",
                &lien,
                Operation::Creation,
                None,
                Some(serde_json::json!({ "contenu_id": contenu_id, "fichier_id": fichier_id })),
                "bibliotheque.document_fichier_ajouter",
            );
        }
        self.noyau.terminer(unite).await?;
        Ok(cle)
    }

    /// Ajoute une image : rend l'appel à écrire dans le texte.
    pub async fn ajouter_image(
        &self,
        session: &Session,
        demande: &DemandeImageBibliotheque,
    ) -> Resultat<ImageAjoutee> {
        let chemin = Path::new(&demande.chemin);
        let (sorte, octets) = Self::lire_fichier(chemin, false)?;
        let cle = self
            .ranger_fichier(session, chemin, sorte, &octets, None)
            .await?;
        let nom = nom_du_fichier(chemin);
        let description = nom
            .rsplit_once('.')
            .map_or(nom.as_str(), |(base, _)| base)
            .to_owned();
        Ok(ImageAjoutee {
            reference: format!("image:{cle}"),
            nom: description,
        })
    }

    /// Ajoute un son : reconnu par ses premiers octets, 10 Mo au plus ; rend l'appel à écrire
    /// dans le texte.
    pub async fn ajouter_son(
        &self,
        session: &Session,
        demande: &DemandeSonBibliotheque,
    ) -> Resultat<SonAjoute> {
        let chemin = Path::new(&demande.chemin);
        let Some(sorte) = reconnaitre_son(&debut_du_fichier(chemin)?) else {
            return Err(regle("bibliotheque.son_inconnu"));
        };
        if std::fs::metadata(chemin)?.len() > TAILLE_SON_MAX {
            return Err(regle("bibliotheque.son_trop_lourd"));
        }
        let octets = std::fs::read(chemin)?;
        if octets.len() as u64 > TAILLE_SON_MAX {
            return Err(regle("bibliotheque.son_trop_lourd"));
        }
        let cle = self
            .ranger_fichier(session, chemin, sorte, &octets, None)
            .await?;
        let nom = nom_du_fichier(chemin);
        let titre = nom
            .rsplit_once('.')
            .map_or(nom.as_str(), |(base, _)| base)
            .to_owned();
        Ok(SonAjoute {
            reference: format!("son:{cle}"),
            nom: titre,
        })
    }

    /// Sons appelés par une leçon, avec leur présence dans la base.
    pub(super) async fn sons_de(
        &self,
        cnx: &mut gesco_db::Connexion,
        empreintes: &[String],
        contexte: &gesco_db::contexte::Contexte,
    ) -> Resultat<Vec<SonLecon>> {
        let mut empreintes = empreintes.to_vec();
        empreintes.sort();
        empreintes.dedup();
        let connus = depot::sons_par_empreinte(cnx, &empreintes, contexte).await?;
        Ok(empreintes
            .into_iter()
            .map(|e| {
                let connu = connus.iter().find(|(c, _, _)| *c == e);
                SonLecon {
                    disponible: connu.is_some(),
                    type_mime: connu.map(|(_, t, _)| t.clone()).unwrap_or_default(),
                    empreinte: e,
                }
            })
            .collect())
    }

    /// Fichiers d'un document de l'école.
    pub async fn fichiers_document(
        &self,
        session: &Session,
        contenu_id: &str,
    ) -> Resultat<Vec<FichierDocument>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        Ok(depot::fichiers_du_contenu(&mut cnx, contenu_id, &contexte)
            .await?
            .into_iter()
            .map(vers_ecran)
            .collect())
    }

    /// Joint un fichier (PDF ou image) à un document de l'école.
    pub async fn ajouter_fichier_document(
        &self,
        session: &Session,
        demande: &DemandeFichierDocument,
    ) -> Resultat<Vec<FichierDocument>> {
        let chemin = Path::new(&demande.chemin);
        let (sorte, octets) = Self::lire_fichier(chemin, true)?;
        self.ranger_fichier(session, chemin, sorte, &octets, Some(&demande.contenu_id))
            .await?;
        self.fichiers_document(session, &demande.contenu_id).await
    }

    /// Retire un fichier d'un document ; le fichier reste dans la base.
    pub async fn retirer_fichier_document(
        &self,
        session: &Session,
        demande: &DemandeRetraitFichierDocument,
    ) -> Resultat<Vec<FichierDocument>> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let portee = self.portee(session).await?;
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        Self::contenu_permis(unite.connexion(), &portee, &demande.contenu_id, &contexte).await?;
        if depot::retirer_fichier_joint(
            unite.connexion(),
            &demande.contenu_id,
            &demande.lien_id,
            &contexte,
        )
        .await?
        {
            unite.auditer(
                "fichier_contenu",
                &demande.lien_id,
                Operation::Modification,
                Some(serde_json::json!({ "retire": false })),
                Some(serde_json::json!({ "retire": true })),
                "bibliotheque.document_fichier_retirer",
            );
        }
        self.noyau.terminer(unite).await?;
        self.fichiers_document(session, &demande.contenu_id).await
    }

    /// Écrit un fichier de la bibliothèque (le PDF d'un document) au chemin choisi.
    pub async fn enregistrer_fichier_sous(
        &self,
        session: &Session,
        demande: &DemandeFichierEnregistrer,
    ) -> Resultat<()> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let (_, _, octets) = {
            let mut cnx = self.connexion().await?;
            depot::fichier_par_empreinte(&mut cnx, &demande.empreinte, &contexte)
                .await?
                .ok_or(regle("bibliotheque.fichier_introuvable"))?
        };
        crate::espace::ecrire_atomiquement(Path::new(&demande.chemin), &octets)?;
        Ok(())
    }

    /// Taille conseillée d'une vidéo, en octets.
    async fn taille_conseillee(&self, session: &Session) -> Resultat<u64> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let mo = parametre::entier(
            &mut cnx,
            TAILLE_VIDEO_CONSEILLEE,
            TAILLE_VIDEO_CONSEILLEE_DEFAUT,
            &contexte,
        )
        .await?;
        Ok(u64::try_from(mo).unwrap_or(0).saturating_mul(1024 * 1024))
    }

    /// Ajoute une vidéo : copiée dans `videos` sous son empreinte, connue de la base. Une vidéo
    /// plus lourde que la taille conseillée attend la confirmation de l'utilisateur.
    pub async fn ajouter_video(
        &self,
        session: &Session,
        demande: &DemandeVideoBibliotheque,
    ) -> Resultat<VideoAjoutee> {
        let chemin = Path::new(&demande.chemin);
        if reconnaitre(&debut_du_fichier(chemin)?) != Some(SorteFichier::Mp4) {
            return Err(regle("bibliotheque.video_inconnue"));
        }
        let nom = nom_du_fichier(chemin);
        let taille = std::fs::metadata(chemin)?.len();
        let lourde = taille > self.taille_conseillee(session).await?;
        if lourde && !demande.confirme {
            return Ok(VideoAjoutee {
                ajoutee: false,
                lourde,
                reference: String::new(),
                nom,
                premiere: false,
            });
        }
        let source = chemin.to_path_buf();
        let (cle, taille) = tokio::task::spawn_blocking(move || empreinte_fichier(&source))
            .await
            .map_err(|e| ErreurService::Secret(e.to_string()))??;
        let dossier = self.espace.videos();
        std::fs::create_dir_all(&dossier)?;
        let destination = chemin_video(&dossier, &cle);
        if !destination.exists() {
            let source = chemin.to_path_buf();
            let cible = destination.clone();
            tokio::task::spawn_blocking(move || copier_sur(&source, &cible))
                .await
                .map_err(|e| ErreurService::Secret(e.to_string()))??;
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let premiere = {
            let mut cnx = self.connexion().await?;
            depot::videos(&mut cnx, &contexte).await?.is_empty()
        };
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let taille_base = i64::try_from(taille).unwrap_or(i64::MAX);
        if depot::enregistrer_video(unite.connexion(), &cle, &nom, taille_base, &contexte).await? {
            unite.auditer(
                "video_bibliotheque",
                &cle,
                Operation::Creation,
                None,
                Some(serde_json::json!({
                    "nom_fichier": nom,
                    "taille": taille,
                    "empreinte_sha256": cle,
                })),
                "bibliotheque.video_ajouter",
            );
            unite.emettre(
                TypeEvenement::VideoBibliothequeAjoutee,
                "video_bibliotheque",
                &cle,
                serde_json::json!({ "empreinte_sha256": cle, "taille": taille }),
            );
        }
        self.noyau.terminer(unite).await?;
        Ok(VideoAjoutee {
            ajoutee: true,
            lourde,
            reference: format!("video:{cle}"),
            nom,
            premiere,
        })
    }

    /// Reçoit un morceau de vidéo d'un poste relié, à la suite des morceaux déjà reçus.
    pub fn recevoir_morceau_video(&self, demande: &DemandeEnvoiMorceauVideo) -> Resultat<()> {
        if !envoi_sur(&demande.envoi) {
            return Err(regle("commande.entree_invalide"));
        }
        let octets = base64::engine::general_purpose::STANDARD
            .decode(&demande.contenu)
            .map_err(|_| regle("commande.entree_invalide"))?;
        if octets.len() as u64 > MORCEAU_VIDEO {
            return Err(regle("commande.entree_invalide"));
        }
        let chemin = chemin_envoi(self.espace, &demande.envoi);
        if let Some(dossier) = chemin.parent() {
            std::fs::create_dir_all(dossier)?;
        }
        let deja = std::fs::metadata(&chemin).map(|m| m.len()).unwrap_or(0);
        // Un envoi qui recommence à zéro efface ce qui restait d'un envoi interrompu.
        if demande.position <= 0 {
            let _ = std::fs::remove_file(&chemin);
        } else if i64::try_from(deja).ok() != Some(demande.position) {
            return Err(regle("bibliotheque.envoi_interrompu"));
        }
        let mut fichier = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&chemin)?;
        fichier.write_all(&octets)?;
        Ok(())
    }

    /// Lit un morceau d'une vidéo de l'école, ou d'un son : un son se lit dans la base, par la
    /// même route, pour qu'un poste relié l'écoute sans que le fichier voyage d'un bloc.
    pub async fn lire_morceau_video(
        &self,
        session: &Session,
        demande: &DemandeMorceauVideo,
    ) -> Resultat<MorceauVideo> {
        if !est_empreinte(&demande.empreinte) {
            return Err(regle("bibliotheque.video_absente"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let connue = {
            let mut cnx = self.connexion().await?;
            let connue = depot::videos(&mut cnx, &contexte)
                .await?
                .iter()
                .any(|v| v.empreinte_sha256 == demande.empreinte);
            if !connue
                && let Some((taille, octets)) = depot::morceau_son(
                    &mut cnx,
                    &demande.empreinte,
                    demande.position,
                    i64::try_from(MORCEAU_VIDEO).unwrap_or(i64::MAX),
                    &contexte,
                )
                .await?
            {
                return Ok(MorceauVideo {
                    taille,
                    contenu: base64::engine::general_purpose::STANDARD.encode(&octets),
                });
            }
            connue
        };
        let chemin = chemin_video(&self.espace.videos(), &demande.empreinte);
        if !connue || !chemin.exists() {
            return Err(regle("bibliotheque.video_absente"));
        }
        let position = u64::try_from(demande.position).unwrap_or(0);
        tokio::task::spawn_blocking(move || -> Resultat<MorceauVideo> {
            let mut fichier = std::fs::File::open(&chemin)?;
            let taille = fichier.metadata()?.len();
            fichier.seek(SeekFrom::Start(position.min(taille)))?;
            let mut tampon = Vec::new();
            fichier.take(MORCEAU_VIDEO).read_to_end(&mut tampon)?;
            Ok(MorceauVideo {
                taille: i64::try_from(taille).unwrap_or(i64::MAX),
                contenu: base64::engine::general_purpose::STANDARD.encode(&tampon),
            })
        })
        .await
        .map_err(|e| ErreurService::Secret(e.to_string()))?
    }

    /// Vidéos appelées par une leçon, avec leur présence sur le poste principal.
    pub(super) async fn videos_de(
        &self,
        cnx: &mut gesco_db::Connexion,
        empreintes: &[String],
        contexte: &gesco_db::contexte::Contexte,
    ) -> Resultat<Vec<VideoLecon>> {
        if empreintes.is_empty() {
            return Ok(Vec::new());
        }
        let connues = depot::videos(cnx, contexte).await?;
        let dossier = self.espace.videos();
        let mut videos: Vec<VideoLecon> = empreintes
            .iter()
            .map(|e| {
                let connue = connues.iter().find(|v| &v.empreinte_sha256 == e);
                let disponible = connue.is_some() && chemin_video(&dossier, e).exists();
                VideoLecon {
                    empreinte: e.clone(),
                    disponible,
                    taille: connue.map_or(0, |v| v.taille),
                }
            })
            .collect();
        videos.sort_by(|a, b| a.empreinte.cmp(&b.empreinte));
        videos.dedup_by(|a, b| a.empreinte == b.empreinte);
        Ok(videos)
    }

    /// Copie les vidéos de l'école vers un dossier (une clé) : sous-dossier `GESCO-videos`,
    /// une vidéo déjà là n'est pas recopiée.
    pub async fn copier_videos(
        &self,
        session: &Session,
        demande: &DemandeDossierVideos,
    ) -> Resultat<BilanVideos> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let videos = {
            let mut cnx = self.connexion().await?;
            depot::videos(&mut cnx, &contexte).await?
        };
        let source = self.espace.videos();
        let cible = Path::new(&demande.dossier).join(DOSSIER_COPIE);
        tokio::task::spawn_blocking(move || -> Resultat<BilanVideos> {
            if !cible.parent().is_some_and(Path::is_dir) {
                return Err(regle("bibliotheque.dossier_introuvable"));
            }
            std::fs::create_dir_all(&cible)?;
            let mut bilan = BilanVideos {
                copiees: 0,
                deja_la: 0,
                absentes: 0,
            };
            for v in videos {
                let depart = chemin_video(&source, &v.empreinte_sha256);
                let arrivee = chemin_video(&cible, &v.empreinte_sha256);
                if !depart.exists() {
                    bilan.absentes += 1;
                } else if arrivee.metadata().is_ok_and(|m| m.len() as i64 == v.taille) {
                    bilan.deja_la += 1;
                } else {
                    copier_sur(&depart, &arrivee)?;
                    bilan.copiees += 1;
                }
            }
            Ok(bilan)
        })
        .await
        .map_err(|e| ErreurService::Secret(e.to_string()))?
    }

    /// Reprend des vidéos depuis un dossier (une clé) : seulement les fichiers dont l'empreinte
    /// est connue de l'école et absents de `videos`, vérifiés par leur empreinte.
    pub async fn reprendre_videos(
        &self,
        session: &Session,
        demande: &DemandeDossierVideos,
    ) -> Resultat<BilanVideos> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let videos = {
            let mut cnx = self.connexion().await?;
            depot::videos(&mut cnx, &contexte).await?
        };
        let cible = self.espace.videos();
        let choisi = PathBuf::from(&demande.dossier);
        tokio::task::spawn_blocking(move || -> Resultat<BilanVideos> {
            if !choisi.is_dir() {
                return Err(regle("bibliotheque.dossier_introuvable"));
            }
            // Le dossier choisi, ou son sous-dossier `GESCO-videos` s'il en a un.
            let source = if choisi.join(DOSSIER_COPIE).is_dir() {
                choisi.join(DOSSIER_COPIE)
            } else {
                choisi
            };
            std::fs::create_dir_all(&cible)?;
            let mut bilan = BilanVideos {
                copiees: 0,
                deja_la: 0,
                absentes: 0,
            };
            for v in videos {
                let arrivee = chemin_video(&cible, &v.empreinte_sha256);
                if arrivee.exists() {
                    bilan.deja_la += 1;
                    continue;
                }
                let depart = chemin_video(&source, &v.empreinte_sha256);
                if !depart.exists() {
                    bilan.absentes += 1;
                    continue;
                }
                copier_sur(&depart, &arrivee)?;
                match empreinte_fichier(&arrivee) {
                    Ok((cle, _)) if cle == v.empreinte_sha256 => bilan.copiees += 1,
                    _ => {
                        let _ = std::fs::remove_file(&arrivee);
                        bilan.absentes += 1;
                    }
                }
            }
            Ok(bilan)
        })
        .await
        .map_err(|e| ErreurService::Secret(e.to_string()))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_identifiant_d_envoi_ne_sort_pas_du_dossier() {
        assert!(envoi_sur("0123456789abcdef0123"));
        assert!(!envoi_sur("../../windows/system"));
        assert!(!envoi_sur("court"));
    }

    #[test]
    fn une_video_se_range_sous_son_empreinte() {
        let e = "a".repeat(64);
        assert!(chemin_video(Path::new("videos"), &e).ends_with(format!("{e}.mp4")));
    }
}
