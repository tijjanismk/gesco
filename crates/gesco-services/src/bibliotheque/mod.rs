//! Bibliothèque de cours (phase 12, lot 1) : installation et mise à jour du paquet livré.
//!
//! Le paquet signé vit dans le dossier `bibliotheque` du poste principal, hors de la base et de
//! la sauvegarde ; la base n'en garde que l'index (chapitres, contenus, empreintes) et l'état de
//! l'école. À chaque ouverture de la base, GESCO compare les paquets du dossier à ceux que la
//! base dit installés : il installe un paquet nouveau ou plus récent (déposé par l'installeur),
//! et après une restauration sur un autre ordinateur il refait l'index depuis le paquet présent.
//! La direction installe une mise à jour depuis un fichier : le nouveau paquet n'est activé
//! qu'une fois copié et relu, l'ancien reste en place jusque-là. Une mise à jour réécrit le
//! livré par sa clé et ne touche jamais ce que l'école a retiré, ajouté ou écrit.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use gesco_core::bibliotheque::paquet::{FichierPaquet, empreinte};
use gesco_core::bibliotheque::{
    BilanInstallation, ErreurPaquet, NatureContenu, PaquetInstalle, PaquetLu, analyser,
    comparer_versions, correspondances_par_code, lire_paquet,
};
use gesco_core::commun::Id;
use gesco_core::core::{Etablissement, Operation, TypeEvenement};
use gesco_db::depots::bibliotheque::{self as depot, EcritureContenu};
use gesco_db::unite::Durabilite;
use gesco_db::{Connexion, nouvel_id};
use serde::{Deserialize, Serialize};

use crate::erreur::{ErreurService, Resultat};
use crate::espace::EspaceDonnees;
use crate::noyau::Noyau;
use crate::session::Session;

mod classes;
mod echange;
mod ecrire;
mod fichiers;
mod imprimer;
mod lire;
mod publier;
mod quiz;
mod retirer;
mod word;

pub use classes::{ClassePourQui, DemandePourQui, DemandeReglerPourQui, PourQuiContenu};
pub use echange::{
    ApercuExport, ApercuImport, ArriveeLecon, DemandeExportApercu, DemandeExportLecons,
    DemandeImportLecons, DemandeImportPublier, ExportFait, ImportAffiche, ImportFait, LeconArrivee,
    LeconExportee, PublicationImport, SelectionExport,
};
pub use ecrire::{
    ApercuLecon, BrouillonOuvert, ChapitreAjoute, ChoixEcriture, ComparaisonTextes,
    DemandeAjoutContenu, DemandeApercuLecon, DemandeBrouillon, DemandeBrouillonCommencer,
    DemandeBrouillonVersion, DemandeChapitreBibliotheque, DemandeComparaison,
    DemandeContenuBibliotheque, DepartBrouillon, EcritureOuverte, HistoriqueContenu,
    MouvementContenu, NiveauChoisissable, PlaceEcriture, SerieChoisissable, SorteMouvement,
    TexteCompare, VersionAffichee, VersionRecueAffichee,
};

pub use fichiers::{
    BilanVideos, DemandeDossierVideos, DemandeEnvoiMorceauVideo, DemandeFichierDocument,
    DemandeFichierEnregistrer, DemandeImageBibliotheque, DemandeMorceauVideo,
    DemandeRetraitFichierDocument, DemandeSonBibliotheque, DemandeVideoBibliotheque,
    FichierDocument, ImageAjoutee, MorceauVideo, SonAjoute, SonLecon, VideoAjoutee, VideoLecon,
};
pub(crate) use fichiers::{chemin_envoi, envoi_sur};
pub use imprimer::{DemandeImpressionChapitre, DemandeImpressionContenu, FormeImpression};
pub(crate) use imprimer::{Impression, ecrire_pdf};
pub(crate) use lire::type_mime as type_mime_image;

pub use publier::{
    DemandeDroitPublication, DemandeRenvoiVersion, DroitPublicationAffiche, DroitsPublication,
    MaitreChoisissable, VersionEnAttente,
};

pub use lire::{
    CatalogueBibliotheque, ContenuLie, DemandeLectureContenu, DemandeRechercheBibliotheque,
    ImageLecon, LectureContenu, ResultatRechercheBibliotheque, VueContenu,
};
pub use quiz::{
    DemandeCorrectionQuiz, DemandeImportQuiz, DemandeModeleQuiz, DemandeQuestionQuiz, ImportQuiz,
    LigneQuizRefusee, TexteQuiz, corriger_reponse,
};
pub use retirer::{
    ApercuRetrait, CorrespondanceAffichee, CorrespondancesBibliotheque, DemandeCorrespondance,
    DemandeRemiseBibliotheque, DemandeRetrait, MatiereChoisissable, RetraitAffiche,
};
pub use word::{DemandeImportWord, ImportWord};

/// Clé publique des paquets de la bibliothèque, distincte de celle des licences.
const CLE_PUBLIQUE_PAQUETS_HEX: &str = include_str!("../../cle-publique-bibliotheque.txt");

/// Extension d'un paquet de la bibliothèque.
pub const EXTENSION_PAQUET: &str = "gbib";

/// Clé publique contre laquelle les paquets sont vérifiés.
pub fn cle_publique_paquets() -> Resultat<[u8; 32]> {
    hex::decode(CLE_PUBLIQUE_PAQUETS_HEX.trim())
        .ok()
        .and_then(|o| o.try_into().ok())
        .ok_or(ErreurService::Regle {
            cle: "bibliotheque.paquet_abime",
        })
}

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Paquets lus et vérifiés, gardés en mémoire le temps que la base est ouverte : la lecture d'une
/// leçon livrée ne relit pas le fichier.
#[derive(Debug, Default)]
pub struct PaquetsCharges {
    paquets: RwLock<BTreeMap<String, Arc<PaquetLu>>>,
}

impl PaquetsCharges {
    /// Le paquet chargé de ce code, s'il y en a un.
    pub fn paquet(&self, code: &str) -> Option<Arc<PaquetLu>> {
        self.paquets.read().ok()?.get(code).cloned()
    }

    /// Tous les paquets chargés, par code.
    pub fn tous(&self) -> BTreeMap<String, Arc<PaquetLu>> {
        self.paquets.read().map(|p| p.clone()).unwrap_or_default()
    }

    fn poser(&self, paquet: Arc<PaquetLu>) {
        if let Ok(mut p) = self.paquets.write() {
            p.insert(paquet.manifeste.code.clone(), paquet);
        }
    }

    /// Oublie tous les paquets (base fermée).
    pub fn vider(&self) {
        if let Ok(mut p) = self.paquets.write() {
            p.clear();
        }
    }
}

/// Un paquet trouvé dans le dossier, lu et vérifié.
struct PaquetTrouve {
    lu: Arc<PaquetLu>,
    nom_fichier: String,
    taille: i64,
    empreinte: String,
}

/// Paquet installé tel que l'écran des réglages le montre.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct PaquetAffiche {
    /// Code du paquet.
    pub code: String,
    /// Version installée.
    pub version: String,
    /// Installé le, millisecondes depuis l'époque.
    pub installe_le: i64,
    /// Nombre de leçons livrées.
    pub lecons: u32,
    /// Nombre de sujets d'examen livrés.
    pub sujets: u32,
    /// Vrai si le fichier du paquet est présent et lu sur ce poste.
    pub present: bool,
    /// Version plus récente déjà installée un jour dans cette école (une sauvegarde venue d'un
    /// ordinateur mieux à jour) : l'écran propose d'installer la mise à jour.
    pub version_plus_recente: Option<String>,
}

/// État de la bibliothèque pour l'écran des réglages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct EtatBibliotheque {
    /// Paquets installés.
    pub paquets: Vec<PaquetAffiche>,
}

/// Installer une mise à jour de la bibliothèque depuis un fichier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeInstallationBibliotheque {
    /// Chemin du fichier `.gbib` (sur un poste relié, son contenu voyage à sa place).
    pub chemin: String,
}

/// Résultat d'une installation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct InstallationBibliotheque {
    /// Version installée.
    pub version: String,
    /// Ce qui a changé.
    pub bilan: BilanInstallation,
    /// État après l'installation.
    pub etat: EtatBibliotheque,
}

/// Service de la bibliothèque.
pub struct ServiceBibliotheque<'a> {
    noyau: &'a Noyau,
    espace: &'a EspaceDonnees,
    paquets: &'a PaquetsCharges,
    cle: [u8; 32],
    version_gesco: &'a str,
}

impl<'a> ServiceBibliotheque<'a> {
    /// Service sur une base ouverte.
    pub fn nouveau(
        noyau: &'a Noyau,
        espace: &'a EspaceDonnees,
        paquets: &'a PaquetsCharges,
        cle: [u8; 32],
        version_gesco: &'a str,
    ) -> Self {
        Self {
            noyau,
            espace,
            paquets,
            cle,
            version_gesco,
        }
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

    /// Paquets du dossier qui se lisent et se vérifient, le plus récent de chaque code.
    fn paquets_du_dossier(&self) -> BTreeMap<String, Vec<PaquetTrouve>> {
        let mut trouves: BTreeMap<String, Vec<PaquetTrouve>> = BTreeMap::new();
        let Ok(entrees) = std::fs::read_dir(self.espace.bibliotheque()) else {
            return trouves;
        };
        let mut chemins: Vec<PathBuf> = entrees
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == EXTENSION_PAQUET))
            .collect();
        chemins.sort();
        for chemin in chemins {
            let Ok(octets) = std::fs::read(&chemin) else {
                continue;
            };
            let Ok(lu) = lire_paquet(&octets, &self.cle) else {
                self.noter(&format!(
                    "bibliotheque paquet refusé : {}",
                    chemin.file_name().and_then(|n| n.to_str()).unwrap_or("?")
                ));
                continue;
            };
            let nom_fichier = chemin
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_owned();
            trouves
                .entry(lu.manifeste.code.clone())
                .or_default()
                .push(PaquetTrouve {
                    nom_fichier,
                    taille: i64::try_from(octets.len()).unwrap_or(i64::MAX),
                    empreinte: empreinte(&octets),
                    lu: Arc::new(lu),
                });
        }
        for liste in trouves.values_mut() {
            liste.sort_by(|a, b| {
                comparer_versions(&b.lu.manifeste.version, &a.lu.manifeste.version)
            });
        }
        trouves
    }

    fn noter(&self, ligne: &str) {
        crate::journal_technique::noter(
            self.espace.racine(),
            self.noyau.horloge().maintenant(),
            ligne,
        );
    }

    fn compatible(&self, lu: &PaquetLu) -> bool {
        lu.manifeste.gesco_minimum.trim().is_empty()
            || comparer_versions(&lu.manifeste.gesco_minimum, self.version_gesco)
                != std::cmp::Ordering::Greater
    }

    /// À l'ouverture de la base : charge les paquets installés, installe un paquet nouveau ou
    /// plus récent déposé dans le dossier, et refait l'index depuis le paquet présent quand celui
    /// que la base connaît manque (restauration sur un autre ordinateur). Ne bloque jamais :
    /// un paquet absent ou refusé laisse la bibliothèque de l'école seule.
    pub async fn synchroniser(&self, etablissement_id: Id<Etablissement>) -> Resultat<()> {
        let contexte = self.noyau.contexte(etablissement_id, None);
        let installes = {
            let mut cnx = self.connexion().await?;
            depot::paquets(&mut cnx, &contexte).await?
        };
        let dossier = self.paquets_du_dossier();
        let mut codes: HashSet<String> = dossier.keys().cloned().collect();
        codes.extend(installes.iter().filter(|p| p.actif).map(|p| p.code.clone()));
        for code in codes {
            let actif = installes.iter().find(|p| p.actif && p.code == code);
            let presents = dossier.get(&code).map(Vec::as_slice).unwrap_or_default();
            let le_meme = actif.and_then(|a| presents.iter().find(|t| t.empreinte == a.empreinte));
            if let Some(t) = le_meme {
                self.paquets.poser(Arc::clone(&t.lu));
            }
            let candidat = presents.iter().find(|t| self.compatible(&t.lu));
            let a_installer = match (actif, le_meme, candidat) {
                (_, _, None) => None,
                // Rien d'installé : la première ouverture.
                (None, _, Some(c)) => Some(c),
                // Le paquet installé est là : seul un plus récent le remplace.
                (Some(a), Some(_), Some(c)) => {
                    (comparer_versions(&c.lu.manifeste.version, &a.version)
                        == std::cmp::Ordering::Greater)
                        .then_some(c)
                }
                // Le paquet installé manque sur ce poste : l'index se refait depuis celui qui
                // est là, même plus ancien ; l'écran dira qu'une mise à jour existe.
                (Some(_), None, Some(c)) => Some(c),
            };
            if let Some(c) = a_installer {
                let issue = self
                    .installer_trouve(etablissement_id, None, c, "bibliotheque.ouverture")
                    .await;
                if let Err(e) = issue {
                    self.noter(&format!("bibliotheque installation à l'ouverture : {e}"));
                }
            }
        }
        Ok(())
    }

    /// Installe une mise à jour désignée par la direction.
    pub async fn installer_fichier(
        &self,
        session: &Session,
        chemin: &Path,
    ) -> Resultat<InstallationBibliotheque> {
        let octets = std::fs::read(chemin)?;
        let lu = lire_paquet(&octets, &self.cle).map_err(|e| match e {
            ErreurPaquet::FormatInconnu => regle("bibliotheque.gesco_trop_ancien"),
            _ => regle("bibliotheque.paquet_abime"),
        })?;
        if !self.compatible(&lu) {
            return Err(regle("bibliotheque.gesco_trop_ancien"));
        }
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let installes = {
            let mut cnx = self.connexion().await?;
            depot::paquets(&mut cnx, &contexte).await?
        };
        if let Some(actif) = installes
            .iter()
            .find(|p| p.actif && p.code == lu.manifeste.code)
        {
            match comparer_versions(&lu.manifeste.version, &actif.version) {
                std::cmp::Ordering::Less => return Err(regle("bibliotheque.plus_ancienne")),
                std::cmp::Ordering::Equal => return Err(regle("bibliotheque.deja_installee")),
                std::cmp::Ordering::Greater => {}
            }
        }
        // Copie sous un nom propre, relue avant d'être activée : une copie interrompue laisse
        // l'ancien paquet en place.
        let nom_fichier = format!(
            "{}-{}.{EXTENSION_PAQUET}",
            nom_sur(&lu.manifeste.code),
            nom_sur(&lu.manifeste.version)
        );
        let destination = self.espace.bibliotheque().join(&nom_fichier);
        std::fs::create_dir_all(self.espace.bibliotheque())?;
        crate::espace::ecrire_atomiquement(&destination, &octets).map_err(|e| {
            if e.kind() == std::io::ErrorKind::StorageFull {
                regle("bibliotheque.place_insuffisante")
            } else {
                ErreurService::Fichier(e)
            }
        })?;
        let relu = std::fs::read(&destination)?;
        if empreinte(&relu) != empreinte(&octets) {
            let _ = std::fs::remove_file(&destination);
            return Err(regle("bibliotheque.place_insuffisante"));
        }
        let trouve = PaquetTrouve {
            nom_fichier,
            taille: i64::try_from(octets.len()).unwrap_or(i64::MAX),
            empreinte: empreinte(&octets),
            lu: Arc::new(lu),
        };
        let bilan = match self
            .installer_trouve(
                session.etablissement_id,
                Some(session),
                &trouve,
                "bibliotheque.installer",
            )
            .await
        {
            Ok(b) => b,
            Err(e) => {
                // La base n'a rien enregistré : le fichier copié ne doit pas s'installer seul à
                // l'ouverture suivante.
                let _ = std::fs::remove_file(&destination);
                return Err(e);
            }
        };
        Ok(InstallationBibliotheque {
            version: trouve.lu.manifeste.version.clone(),
            bilan,
            etat: self.etat(session.etablissement_id).await?,
        })
    }

    /// Écrit un paquet lu dans la base, en une seule transaction : le paquet, ses chapitres et
    /// ses contenus par leur clé, les contenus qui n'y sont plus, la correspondance des
    /// disciplines là où elle manque, puis l'index de recherche.
    async fn installer_trouve(
        &self,
        etablissement_id: Id<Etablissement>,
        session: Option<&Session>,
        t: &PaquetTrouve,
        origine: &str,
    ) -> Resultat<BilanInstallation> {
        let manifeste = &t.lu.manifeste;
        let contexte = self.noyau.contexte(etablissement_id, session);
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        let avant: Option<String> = depot::paquets(unite.connexion(), &contexte)
            .await?
            .into_iter()
            .find(|p| p.actif && p.code == manifeste.code)
            .map(|p| p.version);
        let paquet = PaquetInstalle {
            id: nouvel_id(),
            etablissement_id,
            code: manifeste.code.clone(),
            version: manifeste.version.clone(),
            date_paquet: manifeste.date.clone(),
            gesco_minimum: manifeste.gesco_minimum.clone(),
            nom_fichier: t.nom_fichier.clone(),
            taille: t.taille,
            empreinte: t.empreinte.clone(),
            disciplines: manifeste.disciplines.clone(),
            actif: true,
            installe_le: contexte.maintenant,
        };
        depot::enregistrer_paquet(unite.connexion(), &paquet, &contexte).await?;

        let mut bilan = BilanInstallation::default();
        let mut chapitres: HashMap<&str, String> = HashMap::new();
        for c in &manifeste.chapitres {
            let (id, nouveau) =
                depot::ecrire_chapitre_livre(unite.connexion(), c, &manifeste.code, &contexte)
                    .await?;
            if nouveau {
                bilan.chapitres_ajoutes += 1;
            }
            chapitres.insert(c.cle.as_str(), id);
        }
        let empreintes: HashMap<&str, &FichierPaquet> = manifeste
            .fichiers
            .iter()
            .map(|f| (f.nom.as_str(), f))
            .collect();
        for c in &manifeste.contenus {
            let (Some(chapitre_id), Some(fichier)) = (
                chapitres.get(c.chapitre.as_str()),
                empreintes.get(c.fichier.as_str()),
            ) else {
                // `lire_paquet` a vérifié chaque référence : rien ne peut manquer ici.
                return Err(regle("bibliotheque.paquet_abime"));
            };
            match depot::ecrire_contenu_livre(
                unite.connexion(),
                c,
                chapitre_id,
                &manifeste.code,
                &fichier.empreinte,
                &contexte,
            )
            .await?
            {
                EcritureContenu::Nouveau => bilan.contenus_ajoutes += 1,
                EcritureContenu::TexteChange => bilan.contenus_changes += 1,
                EcritureContenu::TextePareil => {}
            }
        }
        let cles_chapitres: HashSet<&str> =
            manifeste.chapitres.iter().map(|c| c.cle.as_str()).collect();
        let cles_contenus: HashSet<&str> =
            manifeste.contenus.iter().map(|c| c.cle.as_str()).collect();
        bilan.contenus_plus_livres = depot::marquer_plus_livres(
            unite.connexion(),
            &manifeste.code,
            &cles_chapitres,
            &cles_contenus,
            &contexte,
        )
        .await?;

        let matieres = depot::matieres_par_code(unite.connexion(), &contexte).await?;
        for (discipline, niveau, matiere) in correspondances_par_code(manifeste, &matieres) {
            depot::poser_correspondance_si_absente(
                unite.connexion(),
                &discipline,
                &niveau,
                matiere.as_deref(),
                &contexte,
            )
            .await?;
        }

        let mut charges = self.paquets.tous();
        charges.insert(manifeste.code.clone(), Arc::clone(&t.lu));
        reindexer(unite.connexion(), &contexte, &charges).await?;

        let id = paquet.id.vers_chaine();
        unite.auditer(
            "bibliotheque_paquet",
            &id,
            Operation::Creation,
            avant
                .as_ref()
                .map(|v| serde_json::json!({ "code": manifeste.code, "version": v })),
            Some(serde_json::json!({
                "code": manifeste.code,
                "version": manifeste.version,
                "empreinte": t.empreinte,
                "bilan": bilan,
            })),
            origine,
        );
        unite.emettre(
            TypeEvenement::PaquetBibliothequeInstalle,
            "bibliotheque_paquet",
            &id,
            serde_json::json!({
                "code": manifeste.code,
                "version": manifeste.version,
                "avant": avant,
            }),
        );
        self.noyau.terminer(unite).await?;
        self.paquets.poser(Arc::clone(&t.lu));
        Ok(bilan)
    }

    /// Paquets installés et ce qu'ils livrent.
    pub async fn etat(&self, etablissement_id: Id<Etablissement>) -> Resultat<EtatBibliotheque> {
        let contexte = self.noyau.contexte(etablissement_id, None);
        let mut cnx = self.connexion().await?;
        let installes = depot::paquets(&mut cnx, &contexte).await?;
        let contenus = depot::contenus(&mut cnx, &contexte).await?;
        let mut paquets = Vec::new();
        for p in installes.iter().filter(|p| p.actif) {
            let livres = contenus
                .iter()
                .filter(|c| c.paquet_code.as_deref() == Some(p.code.as_str()) && !c.plus_livre);
            let (mut lecons, mut sujets) = (0, 0);
            for c in livres {
                match c.nature {
                    NatureContenu::Lecon => lecons += 1,
                    NatureContenu::Sujet => sujets += 1,
                    NatureContenu::Corrige | NatureContenu::Document | NatureContenu::Quiz => {}
                }
            }
            let version_plus_recente = installes
                .iter()
                .filter(|q| q.code == p.code)
                .map(|q| q.version.as_str())
                .filter(|v| comparer_versions(v, &p.version) == std::cmp::Ordering::Greater)
                .max_by(|a, b| comparer_versions(a, b))
                .map(str::to_owned);
            paquets.push(PaquetAffiche {
                code: p.code.clone(),
                version: p.version.clone(),
                installe_le: p.installe_le.millis_epoque(),
                lecons,
                sujets,
                present: self
                    .paquets
                    .paquet(&p.code)
                    .is_some_and(|l| l.manifeste.version == p.version),
                version_plus_recente,
            });
        }
        Ok(EtatBibliotheque { paquets })
    }
}

/// Garde d'un nom de fichier les lettres, chiffres, points et tirets.
fn nom_sur(texte: &str) -> String {
    texte
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Texte visible d'un contenu, pour l'index : sa version publiée par l'école, sinon son texte
/// livré (si son paquet est chargé et qu'il est encore livré). Sans les notes pour le maître.
fn texte_visible(
    contenu: &gesco_core::bibliotheque::ContenuBibliotheque,
    publiee: Option<&(String, bool)>,
    paquets: &BTreeMap<String, Arc<PaquetLu>>,
) -> Option<String> {
    let brut = match publiee {
        Some((texte, false)) => Some(texte.clone()),
        _ if contenu.plus_livre => None,
        _ => {
            let paquet = paquets.get(contenu.paquet_code.as_deref()?)?;
            paquet.texte(contenu.fichier.as_deref()?).map(str::to_owned)
        }
    }?;
    Some(analyser(&brut).pour_les_eleves().texte_brut())
}

/// Refait l'index de recherche de l'établissement : titre et texte visible de chaque contenu.
pub(crate) async fn reindexer(
    cnx: &mut Connexion,
    contexte: &gesco_db::contexte::Contexte,
    paquets: &BTreeMap<String, Arc<PaquetLu>>,
) -> Resultat<()> {
    let contenus = depot::contenus(cnx, contexte).await?;
    let publiees: HashMap<String, (String, bool)> = depot::versions_publiees(cnx, contexte)
        .await?
        .into_iter()
        .map(|(id, texte, retour)| (id, (texte, retour)))
        .collect();
    depot::vider_index(cnx, contexte).await?;
    for c in &contenus {
        let id = c.id.vers_chaine();
        if let Some(texte) = texte_visible(c, publiees.get(&id), paquets) {
            depot::indexer(cnx, &id, &c.titre, &texte).await?;
        }
    }
    Ok(())
}

/// Réécrit l'entrée d'un seul contenu dans l'index de recherche, d'après sa version visible ;
/// l'enlève s'il n'en a plus.
pub(crate) async fn reindexer_contenu(
    cnx: &mut Connexion,
    contexte: &gesco_db::contexte::Contexte,
    contenu: &gesco_core::bibliotheque::ContenuBibliotheque,
    paquets: &BTreeMap<String, Arc<PaquetLu>>,
) -> Resultat<()> {
    let id = contenu.id.vers_chaine();
    let publiee = depot::version_dans_etat(cnx, &id, "publiee", contexte).await?;
    match texte_visible(contenu, publiee.as_ref(), paquets) {
        Some(texte) => depot::indexer(cnx, &id, &contenu.titre, &texte).await?,
        None => depot::desindexer(cnx, &id).await?,
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_nom_de_fichier_ne_garde_que_des_caracteres_surs() {
        assert_eq!(nom_sur("bibliotheque-mali"), "bibliotheque-mali");
        assert_eq!(nom_sur("2026.11.1"), "2026.11.1");
        assert_eq!(nom_sur("../a b"), ".._a_b");
    }

    #[test]
    fn la_cle_publique_des_paquets_se_lit() {
        assert!(cle_publique_paquets().is_ok());
    }
}
