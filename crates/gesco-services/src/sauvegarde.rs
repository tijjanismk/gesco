//! Sauvegarde (spécification §11, blueprint §18) : instantané cohérent de la base, archive
//! chiffrée par la clé de données avec l'enveloppe de secours et le manifeste, copie vers chaque
//! destination, rotation, test de restauration, avertissements permanents.
//!
//! Copie hors de l'école : un dossier synchronisé (Google Drive, OneDrive, Dropbox) ne reçoit que
//! la copie à la demande et la copie du rythme, jamais les autres sauvegardes. Son nettoyage part
//! de la table `sauvegarde`, sans lire le dossier ; une erreur dans ce dossier ne fait jamais
//! échouer une sauvegarde.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use gesco_core::commun::{Horodatage, Id};
use gesco_core::core::sauvegarde::{
    DELAI_NOUVEL_ESSAI_HORS_ECOLE_MILLIS, FORMAT_ARCHIVE, PARAMETRE_RYTHME_HORS_ECOLE,
    POLITIQUE_HORS_ECOLE, RAPPEL_PHRASE, a_conserver, copie_due,
};
use gesco_core::core::{
    CleDonnees, Etablissement, EtatEnvoi, Manifeste, MotifSauvegarde, Operation, Parametre,
    PolitiqueRotation, RythmeHorsEcole, SorteDestination, TypeEvenement, ValeurParametre,
};
use gesco_db::depots::sauvegarde::{self, ResultatSauvegarde, Sauvegarde};
use gesco_db::depots::{audit, etablissement, parametre};
use gesco_db::unite::Durabilite;
use gesco_db::{Base, OptionsBase, nouvel_id};
use serde::{Deserialize, Serialize};

use crate::alea;
use crate::archive::{self, EXTENSION, EntreeArchive, empreinte_fichier};
use crate::cles::EspaceCles;
use crate::erreur::{ErreurService, Resultat};
use crate::espace::EspaceDonnees;
use crate::noyau::Noyau;
use crate::session::Session;

/// Clé du paramètre qui liste les destinations configurées (JSON).
pub const PARAMETRE_DESTINATIONS: &str = "sauvegarde.destinations";
/// Clé du paramètre qui garde l'issue du dernier test de restauration.
pub const PARAMETRE_DERNIER_TEST: &str = "sauvegarde.dernier_test_reussi";
/// Clé du paramètre de confirmation du certificat de clé de secours.
pub const PARAMETRE_CERTIFICAT: &str = "sauvegarde.certificat_confirme";
/// Intervalle du test de restauration automatique : un mois (§11.5).
pub const INTERVALLE_TEST_MILLIS: i64 = 30 * 24 * 3_600_000;

/// Destination de sauvegarde.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Destination {
    /// Dossier.
    pub chemin: String,
    /// Sorte : dossier, support amovible ou dossier synchronisé. Absente des réglages de la 1.2,
    /// elle se déduit de la case « amovible ».
    #[serde(default)]
    pub sorte: SorteDestination,
    /// Case de la 1.2, encore lue : support physiquement déconnectable (clé USB, disque externe),
    /// seule protection réelle contre un rançongiciel (§11.3). Toujours égale à la sorte
    /// « support amovible » une fois la destination relue.
    #[serde(default)]
    pub amovible: bool,
}

impl Destination {
    /// Accorde la sorte et la case de la 1.2 : une case cochée sans sorte donne un support
    /// amovible ; la case suit ensuite la sorte.
    #[must_use]
    pub fn accordee(mut self) -> Self {
        if self.amovible && self.sorte == SorteDestination::Dossier {
            self.sorte = SorteDestination::Amovible;
        }
        self.amovible = self.sorte == SorteDestination::Amovible;
        self
    }

    /// Vrai pour un dossier synchronisé, la copie hors de l'école.
    pub fn hors_ecole(&self) -> bool {
        self.sorte == SorteDestination::Synchronise
    }
}

/// Rythme de la copie hors de l'école, demandé par l'école.
#[derive(Debug, Clone, Copy, Deserialize, specta::Type)]
pub struct DemandeRythmeHorsEcole {
    /// À la demande, chaque semaine ou chaque mois.
    pub rythme: RythmeHorsEcole,
}

/// Résumé d'une archive, lu dans son manifeste, sans clé.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ApercuSauvegarde {
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Nom de l'établissement.
    pub etablissement_nom: String,
    /// Instant de la sauvegarde.
    pub horodatage: Horodatage,
    /// Déclencheur.
    pub motif: MotifSauvegarde,
    /// Version de l'application qui l'a produite.
    pub version_applicative: String,
    /// Version du schéma de la base sauvegardée.
    pub version_schema: i64,
    /// Identifiant de l'enveloppe de secours, à rapprocher du certificat.
    pub enveloppe_secours_id: String,
}

impl From<&Manifeste> for ApercuSauvegarde {
    fn from(m: &Manifeste) -> Self {
        Self {
            etablissement_id: m.etablissement_id,
            etablissement_nom: m.etablissement_nom.clone(),
            horodatage: m.horodatage,
            motif: m.motif,
            version_applicative: m.version_applicative.clone(),
            version_schema: m.version_schema,
            enveloppe_secours_id: m.enveloppe_secours.id.clone(),
        }
    }
}

/// Issue d'un test de restauration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct RapportTest {
    /// Archive testée.
    pub destination: String,
    /// Vrai si l'archive s'est déchiffrée, que la base est intègre et sa chaîne d'audit intacte.
    pub reussi: bool,
    /// Clé de traduction du motif d'échec.
    pub motif_echec: Option<String>,
}

/// Avertissement permanent affiché dans le bandeau tant que sa cause subsiste.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Avertissement {
    /// Clé de traduction.
    pub cle: String,
}

/// Contenu d'une archive : la base et les modèles sur mesure (§11.2). Les paramètres vivent dans
/// la base ; les pièces jointes n'existent pas encore en phase 0.5.
fn entrees(instantane: &Path, espace: &EspaceDonnees) -> Resultat<Vec<EntreeArchive>> {
    let mut entrees = vec![EntreeArchive {
        nom: "gesco.db".to_owned(),
        chemin: instantane.to_path_buf(),
    }];
    let racine = espace.modeles();
    let mut pile = vec![racine.clone()];
    while let Some(dossier) = pile.pop() {
        if !dossier.is_dir() {
            continue;
        }
        let mut contenu: Vec<PathBuf> = std::fs::read_dir(&dossier)?
            .map(|e| e.map(|e| e.path()))
            .collect::<std::io::Result<_>>()?;
        contenu.sort();
        for chemin in contenu {
            if chemin.is_dir() {
                pile.push(chemin);
            } else if let Ok(relatif) = chemin.strip_prefix(&racine) {
                let nom = relatif
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/");
                entrees.push(EntreeArchive {
                    nom: format!("modeles/{nom}"),
                    chemin,
                });
            }
        }
    }
    Ok(entrees)
}

/// Nom d'archive lisible et trié : `gesco-20261004-183512-K3Q9ZD.gsv`.
fn nom_archive(instant: Horodatage) -> String {
    let c = instant.civil_utc();
    let id = nouvel_id::<Sauvegarde>().vers_chaine();
    let suffixe = id.get(20..).unwrap_or(&id);
    format!(
        "gesco-{:04}{:02}{:02}-{:02}{:02}{:02}-{suffixe}.{EXTENSION}",
        c.annee, c.mois, c.jour, c.heure, c.minute, c.seconde
    )
}

/// Prépare le dossier d'une destination. Un dossier synchronisé ne se crée que dans un dossier qui
/// existe déjà : si le logiciel du fournisseur a été désinstallé, ou le lecteur a disparu, GESCO
/// ne recrée pas sur le poste un dossier que rien n'enverrait.
fn preparer_dossier(destination: &Destination) -> std::io::Result<()> {
    let dossier = Path::new(&destination.chemin);
    if destination.hors_ecole() && !dossier.is_dir() {
        if !dossier.parent().is_some_and(Path::is_dir) {
            return Err(std::io::Error::from(std::io::ErrorKind::NotFound));
        }
        return std::fs::create_dir(dossier);
    }
    std::fs::create_dir_all(dossier)
}

/// Ouvre une base posée dans un fichier de travail, avec la clé de données.
pub(crate) async fn ouvrir_fichier(chemin: &Path, cle: &CleDonnees) -> Resultat<Base> {
    Ok(Base::ouvrir_sans_migrer(
        OptionsBase::fichier(chemin).avec_cle_hex(cle.en_hexadecimal().as_str()),
    )
    .await?)
}

/// Service de sauvegarde.
pub struct ServiceSauvegarde<'a> {
    noyau: &'a Noyau,
    espace: &'a EspaceDonnees,
    cles: &'a EspaceCles,
    cle: &'a CleDonnees,
    version_applicative: &'a str,
}

impl<'a> ServiceSauvegarde<'a> {
    /// Service sur une installation ouverte.
    pub const fn nouveau(
        noyau: &'a Noyau,
        espace: &'a EspaceDonnees,
        cles: &'a EspaceCles,
        cle: &'a CleDonnees,
        version_applicative: &'a str,
    ) -> Self {
        Self {
            noyau,
            espace,
            cles,
            cle,
            version_applicative,
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

    /// Destinations configurées, précédées du dossier local du poste.
    pub async fn destinations(
        &self,
        etablissement_id: Id<Etablissement>,
    ) -> Resultat<Vec<Destination>> {
        let contexte = self.noyau.contexte(etablissement_id, None);
        let mut cnx = self.connexion().await?;
        let configurees = match parametre::lire(&mut cnx, PARAMETRE_DESTINATIONS, &contexte).await?
        {
            Some(Parametre {
                valeur: ValeurParametre::Texte(json),
                ..
            }) if !json.is_empty() => serde_json::from_str::<Vec<Destination>>(&json)?,
            _ => Vec::new(),
        };
        let mut toutes = vec![Destination {
            chemin: self
                .espace
                .sauvegardes_locales()
                .to_string_lossy()
                .into_owned(),
            sorte: SorteDestination::Dossier,
            amovible: false,
        }];
        toutes.extend(configurees.into_iter().map(Destination::accordee));
        Ok(toutes)
    }

    /// Remplace les destinations configurées (hors dossier local, toujours présent).
    pub async fn definir_destinations(
        &self,
        session: &Session,
        destinations: Vec<Destination>,
    ) -> Resultat<Vec<Destination>> {
        if destinations.iter().any(|d| d.chemin.trim().is_empty()) {
            return Err(ErreurService::Regle {
                cle: "sauvegarde.destination_vide",
            });
        }
        let destinations: Vec<Destination> = destinations
            .into_iter()
            .map(Destination::accordee)
            .collect();
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        let valeur = serde_json::to_string(&destinations)?;
        parametre::definir(
            unite.connexion(),
            &Parametre {
                etablissement_id: session.etablissement_id,
                cle: PARAMETRE_DESTINATIONS.to_owned(),
                valeur: ValeurParametre::Texte(valeur),
                modifiable_par_utilisateur: false,
            },
            &contexte,
        )
        .await?;
        unite.auditer(
            "parametre",
            PARAMETRE_DESTINATIONS,
            Operation::Modification,
            None,
            Some(serde_json::to_value(&destinations)?),
            "sauvegarde.destinations_definir",
        );
        self.noyau.terminer(unite).await?;
        self.destinations(session.etablissement_id).await
    }

    /// Produit une sauvegarde vers chaque destination, hors les dossiers synchronisés, et
    /// applique la rotation. Une destination absente (clé USB débranchée) laisse une ligne en
    /// échec sans empêcher les autres.
    pub async fn creer(
        &self,
        etablissement_id: Id<Etablissement>,
        session: Option<&Session>,
        motif: MotifSauvegarde,
    ) -> Resultat<Vec<Sauvegarde>> {
        self.creer_vers(etablissement_id, session, motif, false)
            .await
    }

    /// Copie hors de l'école, à la demande (sauvegarde manuelle) ou au rythme choisi (sauvegarde
    /// planifiée) : une sauvegarde vers toutes les destinations, dossiers synchronisés compris.
    /// C'est le seul point par où une copie part hors de l'école.
    pub async fn envoyer_hors_ecole(
        &self,
        etablissement_id: Id<Etablissement>,
        session: Option<&Session>,
        motif: MotifSauvegarde,
    ) -> Resultat<Vec<Sauvegarde>> {
        self.creer_vers(etablissement_id, session, motif, true)
            .await
    }

    async fn creer_vers(
        &self,
        etablissement_id: Id<Etablissement>,
        session: Option<&Session>,
        motif: MotifSauvegarde,
        hors_ecole: bool,
    ) -> Resultat<Vec<Sauvegarde>> {
        let contexte = self.noyau.contexte(etablissement_id, session);
        let secours = self.cles.enveloppe_secours()?.ok_or(ErreurService::Regle {
            cle: "sauvegarde.phrase_secours_absente",
        })?;
        let cle_id = self.cles.cle_id()?.unwrap_or_default();
        let fiche = {
            let mut cnx = self.connexion().await?;
            etablissement::lire(&mut cnx, etablissement_id).await?
        };
        let destinations: Vec<Destination> = self
            .destinations(etablissement_id)
            .await?
            .into_iter()
            .filter(|d| hors_ecole || !d.hors_ecole())
            .collect();

        std::fs::create_dir_all(self.espace.temporaire())?;
        let instantane = self.espace.temporaire().join(format!(
            "instantane-{}.db",
            nouvel_id::<Sauvegarde>().vers_chaine()
        ));
        self.noyau.base().instantane(&instantane).await?;
        let resultat = self
            .archiver(
                &contexte,
                &fiche,
                &cle_id,
                secours,
                motif,
                &instantane,
                &destinations,
            )
            .await;
        let _ = std::fs::remove_file(&instantane);
        let sauvegardes = resultat?;

        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Critique)
            .await?;
        for s in &sauvegardes {
            sauvegarde::inserer(unite.connexion(), s, &contexte).await?;
            unite.auditer(
                "sauvegarde",
                &s.id.vers_chaine(),
                Operation::Creation,
                None,
                Some(serde_json::to_value(s)?),
                "sauvegarde.creer",
            );
        }
        let reussies = sauvegardes
            .iter()
            .filter(|s| s.resultat == ResultatSauvegarde::Reussie)
            .count();
        unite.emettre(
            TypeEvenement::SauvegardeEffectuee,
            "sauvegarde",
            &sauvegardes
                .first()
                .map(|s| s.id.vers_chaine())
                .unwrap_or_default(),
            serde_json::json!({
                "motif": motif,
                "destinations": sauvegardes.len(),
                "reussies": reussies,
                "version_schema": sauvegardes.first().map_or(0, |s| s.version_schema),
            }),
        );
        self.noyau.terminer(unite).await?;

        // Avant une restauration, la rotation attend la sauvegarde suivante : elle supprimerait
        // une archive du même jour, peut-être celle que l'on s'apprête à restaurer.
        if motif != MotifSauvegarde::AvantRestauration {
            let politique = self.politique(etablissement_id).await?;
            for d in destinations.iter().filter(|d| !d.hors_ecole()) {
                self.appliquer_rotation(Path::new(&d.chemin), etablissement_id, politique)?;
            }
            // Le nettoyage du dossier hors de l'école se retente à la copie suivante : son échec
            // ne fait pas échouer la sauvegarde.
            if destinations.iter().any(Destination::hors_ecole) {
                let _ = self.nettoyer_hors_ecole(etablissement_id).await;
            }
        }
        Ok(sauvegardes)
    }

    #[allow(clippy::too_many_arguments)] // Une archive réunit ces sept éléments, tous distincts.
    async fn archiver(
        &self,
        contexte: &gesco_db::Contexte,
        fiche: &Etablissement,
        cle_id: &str,
        secours: gesco_core::core::Enveloppe,
        motif: MotifSauvegarde,
        instantane: &Path,
        destinations: &[Destination],
    ) -> Resultat<Vec<Sauvegarde>> {
        let base_instantane = ouvrir_fichier(instantane, self.cle).await?;
        let version_schema = base_instantane.version_schema().await?;
        let empreinte_contenu = base_instantane.empreinte_contenu().await?;
        base_instantane.fermer().await;
        let manifeste = Manifeste {
            format: FORMAT_ARCHIVE,
            version_applicative: self.version_applicative.to_owned(),
            version_schema,
            etablissement_id: fiche.id,
            etablissement_nom: fiche.nom.clone(),
            horodatage: contexte.maintenant,
            motif,
            cle_id: cle_id.to_owned(),
            enveloppe_secours: secours,
            empreinte_base: empreinte_fichier(instantane)?,
            empreinte_contenu,
            prefixe_nonce: alea::octets()?,
            rappel: RAPPEL_PHRASE.to_owned(),
        };
        let nom = nom_archive(contexte.maintenant);
        let contenu = entrees(instantane, self.espace)?;
        let premiere = destinations.first().ok_or(ErreurService::Regle {
            cle: "sauvegarde.aucune_destination",
        })?;
        let origine = Path::new(&premiere.chemin).join(&nom);
        std::fs::create_dir_all(&premiere.chemin)?;
        let cle = self.cle.clone();
        let (cible, m) = (origine.clone(), manifeste.clone());
        let taille =
            tokio::task::spawn_blocking(move || archive::ecrire(&cible, &m, &cle, &contenu))
                .await
                .map_err(|e| ErreurService::Secret(e.to_string()))??;
        let empreinte = empreinte_fichier(&origine)?;

        let ligne = |destination: String,
                     sorte: SorteDestination,
                     resultat: ResultatSauvegarde,
                     taille: u64| Sauvegarde {
            id: nouvel_id(),
            etablissement_id: fiche.id,
            horodatage: contexte.maintenant,
            destination,
            taille: i64::try_from(taille).unwrap_or(i64::MAX),
            empreinte_sha256: empreinte.clone(),
            version_schema,
            version_applicative: self.version_applicative.to_owned(),
            resultat,
            dernier_test_restauration: None,
            sorte,
            // Au dépôt, une copie hors de l'école est « déposée » si son empreinte est vérifiée ;
            // jamais « envoyée » sans que Windows l'ait dit.
            etat_envoi: (sorte == SorteDestination::Synchronise).then_some(
                if resultat == ResultatSauvegarde::Reussie {
                    EtatEnvoi::Deposee
                } else {
                    EtatEnvoi::NonDeposee
                },
            ),
            etat_verifie_le: (sorte == SorteDestination::Synchronise)
                .then_some(contexte.maintenant),
        };
        let mut sauvegardes = vec![ligne(
            origine.to_string_lossy().into_owned(),
            premiere.sorte,
            ResultatSauvegarde::Reussie,
            taille,
        )];
        for d in destinations.iter().skip(1) {
            let cible = Path::new(&d.chemin).join(&nom);
            let partiel = cible.with_extension("partiel");
            let copie = preparer_dossier(d).and_then(|()| {
                std::fs::copy(&origine, &partiel)?;
                // Windows refuse de vider sur disque un fichier ouvert en lecture seule.
                std::fs::OpenOptions::new()
                    .write(true)
                    .open(&partiel)?
                    .sync_all()?;
                std::fs::rename(&partiel, &cible)
            });
            if copie.is_err() {
                // Disque plein, copie interrompue : rien d'autre que des archives entières ne
                // reste dans le dossier.
                let _ = std::fs::remove_file(&partiel);
            }
            let reussie = copie.is_ok() && empreinte_fichier(&cible).is_ok_and(|e| e == empreinte);
            sauvegardes.push(ligne(
                cible.to_string_lossy().into_owned(),
                d.sorte,
                if reussie {
                    ResultatSauvegarde::Reussie
                } else {
                    ResultatSauvegarde::Echouee
                },
                if reussie { taille } else { 0 },
            ));
        }
        Ok(sauvegardes)
    }

    async fn politique(&self, etablissement_id: Id<Etablissement>) -> Resultat<PolitiqueRotation> {
        let contexte = self.noyau.contexte(etablissement_id, None);
        let mut cnx = self.connexion().await?;
        let defaut = PolitiqueRotation::default();
        let lire = |v: i64| u32::try_from(v).unwrap_or(0);
        Ok(PolitiqueRotation {
            quotidiennes: lire(
                parametre::entier(
                    &mut cnx,
                    "sauvegarde.quotidiennes",
                    i64::from(defaut.quotidiennes),
                    &contexte,
                )
                .await?,
            ),
            hebdomadaires: lire(
                parametre::entier(
                    &mut cnx,
                    "sauvegarde.hebdomadaires",
                    i64::from(defaut.hebdomadaires),
                    &contexte,
                )
                .await?,
            ),
            mensuelles: lire(
                parametre::entier(
                    &mut cnx,
                    "sauvegarde.mensuelles",
                    i64::from(defaut.mensuelles),
                    &contexte,
                )
                .await?,
            ),
        })
    }

    /// Supprime d'un dossier les archives de l'établissement hors politique (7/4/12). Les
    /// fichiers illisibles ou d'un autre établissement ne sont jamais touchés.
    fn appliquer_rotation(
        &self,
        dossier: &Path,
        etablissement_id: Id<Etablissement>,
        politique: PolitiqueRotation,
    ) -> Resultat<()> {
        let Ok(contenu) = std::fs::read_dir(dossier) else {
            return Ok(());
        };
        let mut archives = Vec::new();
        for entree in contenu.flatten() {
            let chemin = entree.path();
            if chemin.extension().is_none_or(|e| e != EXTENSION) {
                continue;
            }
            if let Ok(m) = archive::lire_manifeste(&chemin)
                && m.etablissement_id == etablissement_id
            {
                archives.push((chemin, m.horodatage));
            }
        }
        let gardees = a_conserver(&archives, politique);
        for (chemin, _) in archives {
            if !gardees.contains(&chemin) {
                std::fs::remove_file(&chemin)?;
            }
        }
        Ok(())
    }

    /// Nettoyage des dossiers synchronisés : par dossier, les 4 dernières semaines et les 12
    /// derniers mois. Il part des lignes de la table qui disent ce que GESCO y a déposé pour cet
    /// établissement : il ne lit jamais le dossier, ne fait rien télécharger, et ne supprime que
    /// ses propres copies. Une copie retirée prend l'état « n'est plus dans le dossier ».
    async fn nettoyer_hors_ecole(&self, etablissement_id: Id<Etablissement>) -> Resultat<()> {
        let contexte = self.noyau.contexte(etablissement_id, None);
        let deposees = {
            let mut cnx = self.connexion().await?;
            sauvegarde::deposees_hors_ecole(&mut cnx, &contexte).await?
        };
        let mut par_dossier: BTreeMap<PathBuf, Vec<(Id<Sauvegarde>, Horodatage)>> = BTreeMap::new();
        for s in &deposees {
            if let Some(dossier) = Path::new(&s.destination).parent() {
                par_dossier
                    .entry(dossier.to_path_buf())
                    .or_default()
                    .push((s.id, s.horodatage));
            }
        }
        let mut retirees = Vec::new();
        for copies in par_dossier.values() {
            let gardees = a_conserver(copies, POLITIQUE_HORS_ECOLE);
            for (id, _) in copies {
                if gardees.contains(id) {
                    continue;
                }
                let Some(copie) = deposees.iter().find(|s| s.id == *id) else {
                    continue;
                };
                match std::fs::remove_file(&copie.destination) {
                    Ok(()) => retirees.push(*id),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => retirees.push(*id),
                    // Dossier absent ou refusé : la copie se retirera au prochain nettoyage.
                    Err(_) => {}
                }
            }
        }
        if retirees.is_empty() {
            return Ok(());
        }
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        for id in retirees {
            sauvegarde::noter_etat_envoi(unite.connexion(), id, EtatEnvoi::Absente, &contexte)
                .await?;
        }
        self.noyau.terminer(unite).await?;
        Ok(())
    }

    /// Rythme de la copie hors de l'école ; « à la demande » tant que l'école n'en a pas choisi.
    pub async fn rythme_hors_ecole(
        &self,
        etablissement_id: Id<Etablissement>,
    ) -> Resultat<RythmeHorsEcole> {
        let contexte = self.noyau.contexte(etablissement_id, None);
        let mut cnx = self.connexion().await?;
        Ok(
            match parametre::lire(&mut cnx, PARAMETRE_RYTHME_HORS_ECOLE, &contexte).await? {
                Some(Parametre {
                    valeur: ValeurParametre::Texte(nom),
                    ..
                }) => RythmeHorsEcole::depuis_nom(&nom).unwrap_or_default(),
                _ => RythmeHorsEcole::default(),
            },
        )
    }

    /// Règle le rythme de la copie hors de l'école ; la modification est auditée.
    pub async fn regler_rythme_hors_ecole(
        &self,
        session: &Session,
        rythme: RythmeHorsEcole,
    ) -> Resultat<RythmeHorsEcole> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        parametre::definir(
            unite.connexion(),
            &Parametre {
                etablissement_id: session.etablissement_id,
                cle: PARAMETRE_RYTHME_HORS_ECOLE.to_owned(),
                valeur: ValeurParametre::Texte(rythme.nom().to_owned()),
                modifiable_par_utilisateur: false,
            },
            &contexte,
        )
        .await?;
        unite.auditer(
            "parametre",
            PARAMETRE_RYTHME_HORS_ECOLE,
            Operation::Modification,
            None,
            Some(serde_json::json!({ "rythme": rythme })),
            "sauvegarde.hors_ecole_regler",
        );
        self.noyau.terminer(unite).await?;
        Ok(rythme)
    }

    /// Vrai si la copie du rythme est due : l'école a un dossier synchronisé, un rythme autre
    /// que « à la demande », et la dernière copie bien déposée est assez ancienne. Après un dépôt
    /// manqué, elle ne se retente qu'un jour plus tard.
    pub async fn copie_hors_ecole_due(
        &self,
        etablissement_id: Id<Etablissement>,
    ) -> Resultat<bool> {
        if !self
            .destinations(etablissement_id)
            .await?
            .iter()
            .any(Destination::hors_ecole)
        {
            return Ok(false);
        }
        let rythme = self.rythme_hors_ecole(etablissement_id).await?;
        let contexte = self.noyau.contexte(etablissement_id, None);
        let maintenant = contexte.maintenant;
        let mut cnx = self.connexion().await?;
        let deposee = sauvegarde::derniere_hors_ecole(&mut cnx, true, &contexte).await?;
        if !copie_due(rythme, deposee, maintenant) {
            return Ok(false);
        }
        let essai = sauvegarde::derniere_hors_ecole(&mut cnx, false, &contexte).await?;
        let essai_manque_recent = essai.is_some_and(|e| {
            deposee.is_none_or(|d| d < e)
                && e.difference_millis(&maintenant)
                    .is_ok_and(|ecart| ecart < DELAI_NOUVEL_ESSAI_HORS_ECOLE_MILLIS)
        });
        Ok(!essai_manque_recent)
    }

    /// Vrai si rien ne s'est écrit depuis la dernière sauvegarde : la dernière ligne du journal
    /// d'audit est celle de cette sauvegarde, elle a réussi, son archive est toujours là, et
    /// elle date de moins que l'intervalle réglé. La sauvegarde de fermeture ne se refait pas
    /// alors.
    pub async fn a_jour(&self, etablissement_id: Id<Etablissement>) -> Resultat<bool> {
        let contexte = self.noyau.contexte(etablissement_id, None);
        let mut cnx = self.connexion().await?;
        if gesco_db::depots::audit::derniere_origine(&mut cnx, &contexte)
            .await?
            .as_deref()
            != Some("sauvegarde.creer")
        {
            return Ok(false);
        }
        let Some(derniere) = sauvegarde::lister(&mut cnx, 1, &contexte)
            .await?
            .into_iter()
            .next()
        else {
            return Ok(false);
        };
        let Some(reussie) = sauvegarde::derniere_reussie(&mut cnx, &contexte).await? else {
            return Ok(false);
        };
        let intervalle_min =
            parametre::entier(&mut cnx, "sauvegarde.intervalle_min", 24 * 60, &contexte).await?;
        Ok(reussie.horodatage == derniere.horodatage
            && Path::new(&reussie.destination).is_file()
            && reussie
                .horodatage
                .difference_millis(&contexte.maintenant)
                .is_ok_and(|d| d < intervalle_min.saturating_mul(60_000)))
    }

    /// Sauvegardes de l'établissement, la plus récente d'abord.
    pub async fn lister(
        &self,
        etablissement_id: Id<Etablissement>,
        limite: i64,
    ) -> Resultat<Vec<Sauvegarde>> {
        let contexte = self.noyau.contexte(etablissement_id, None);
        let mut cnx = self.connexion().await?;
        Ok(sauvegarde::lister(&mut cnx, limite, &contexte).await?)
    }

    /// Test de restauration (§11.5, R-49) : l'archive locale la plus récente est déchiffrée dans
    /// un espace temporaire, son empreinte vérifiée, la base ouverte, migrée si besoin, contrôlée
    /// par `PRAGMA integrity_check` et par la vérification complète de la chaîne d'audit.
    pub async fn tester(
        &self,
        etablissement_id: Id<Etablissement>,
        session: Option<&Session>,
    ) -> Resultat<RapportTest> {
        let contexte = self.noyau.contexte(etablissement_id, session);
        let derniere = {
            let mut cnx = self.connexion().await?;
            sauvegarde::derniere_reussie(&mut cnx, &contexte).await?
        }
        .ok_or(ErreurService::Regle {
            cle: "sauvegarde.aucune_archive",
        })?;
        let espace_test = EspaceDonnees::nouveau(
            self.espace
                .temporaire()
                .join(format!("test-{}", nouvel_id::<Sauvegarde>().vers_chaine())),
        );
        let issue = self
            .verifier_archive(
                Path::new(&derniere.destination),
                &espace_test,
                etablissement_id,
            )
            .await;
        let _ = std::fs::remove_dir_all(espace_test.racine());
        let reussi = issue.is_ok();

        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        if reussi {
            sauvegarde::noter_test_restauration(unite.connexion(), derniere.id, &contexte).await?;
        }
        parametre::definir(
            unite.connexion(),
            &Parametre {
                etablissement_id,
                cle: PARAMETRE_DERNIER_TEST.to_owned(),
                valeur: ValeurParametre::Booleen(reussi),
                modifiable_par_utilisateur: false,
            },
            &contexte,
        )
        .await?;
        let motif_echec = issue.err().map(|e| e.cle_traduction().to_owned());
        unite.auditer(
            "sauvegarde",
            &derniere.id.vers_chaine(),
            Operation::Acces,
            None,
            Some(serde_json::json!({ "test_restauration": reussi, "motif": motif_echec })),
            "sauvegarde.tester",
        );
        if !reussi {
            unite.planifier(crate::effets::Effet::AvertirDirection {
                cle_message: "avertissement.test_restauration_echoue".to_owned(),
                donnees: serde_json::json!({ "destination": derniere.destination }),
            });
        }
        self.noyau.terminer(unite).await?;
        Ok(RapportTest {
            destination: derniere.destination,
            reussi,
            motif_echec,
        })
    }

    async fn verifier_archive(
        &self,
        chemin: &Path,
        espace_test: &EspaceDonnees,
        etablissement_id: Id<Etablissement>,
    ) -> Resultat<()> {
        espace_test.preparer()?;
        let cle = self.cle.clone();
        let (source, dossier) = (chemin.to_path_buf(), espace_test.temporaire());
        let manifeste =
            tokio::task::spawn_blocking(move || archive::extraire(&source, &cle, &dossier))
                .await
                .map_err(|e| ErreurService::Secret(e.to_string()))??;
        let fichier = espace_test.temporaire().join("gesco.db");
        if empreinte_fichier(&fichier)? != manifeste.empreinte_base {
            return Err(ErreurService::Regle {
                cle: "sauvegarde.archive_alteree",
            });
        }
        let base = ouvrir_fichier(&fichier, self.cle).await?;
        let issue = async {
            base.migrer().await?;
            base.verifier_integrite().await?;
            let contexte = self.noyau.contexte(etablissement_id, None);
            let mut cnx = base
                .pool()
                .acquire()
                .await
                .map_err(gesco_db::ErreurBase::from)?;
            audit::verifier_chaine(&mut cnx, None, &contexte).await?;
            Ok::<(), ErreurService>(())
        }
        .await;
        base.fermer().await;
        issue
    }

    /// Vrai si le dernier test de restauration réussi date de plus d'un mois.
    pub async fn test_du(&self, etablissement_id: Id<Etablissement>) -> Resultat<bool> {
        let derniere = self.lister(etablissement_id, 50).await?;
        let maintenant = self.noyau.horloge().maintenant();
        let dernier_test = derniere
            .iter()
            .filter_map(|s| s.dernier_test_restauration)
            .max();
        Ok(!derniere.is_empty()
            && dernier_test.is_none_or(|t| {
                t.difference_millis(&maintenant)
                    .is_ok_and(|d| d >= INTERVALLE_TEST_MILLIS)
            }))
    }

    /// Avertissements permanents de la sauvegarde (§11.3, §6.3).
    pub async fn avertissements(
        &self,
        etablissement_id: Id<Etablissement>,
    ) -> Resultat<Vec<Avertissement>> {
        let contexte = self.noyau.contexte(etablissement_id, None);
        let mut cles = Vec::new();
        {
            let mut cnx = self.connexion().await?;
            if !parametre::booleen_param(&mut cnx, PARAMETRE_CERTIFICAT, false, &contexte).await? {
                cles.push("avertissement.certificat_non_confirme");
            }
            if !parametre::booleen_param(&mut cnx, PARAMETRE_DERNIER_TEST, true, &contexte).await? {
                cles.push("avertissement.test_restauration_echoue");
            }
            let recente = gesco_db::depots::audit::derniere_de_l_origine(
                &mut cnx,
                crate::comptes::ORIGINE_REINITIALISATION_PAR_PHRASE,
                &contexte,
            )
            .await?
            .is_some_and(|le| {
                self.noyau
                    .horloge()
                    .maintenant()
                    .millis_epoque()
                    .saturating_sub(le)
                    < crate::comptes::JOURS_SIGNALEMENT_REINITIALISATION * 86_400_000
            });
            if recente {
                cles.push("avertissement.mot_de_passe_reinitialise_par_phrase");
            }
        }
        if !self
            .destinations(etablissement_id)
            .await?
            .iter()
            .any(|d| d.sorte == SorteDestination::Amovible)
        {
            cles.push("avertissement.aucune_destination_amovible");
        }
        Ok(cles
            .into_iter()
            .map(|cle| Avertissement {
                cle: cle.to_owned(),
            })
            .collect())
    }

    /// Enregistre la confirmation « j'ai imprimé et rangé le certificat hors du poste ».
    pub async fn confirmer_certificat(&self, session: &Session) -> Resultat<()> {
        self.confirmer_certificat_de(session.etablissement_id, Some(session))
            .await
    }

    /// Note le certificat imprimé et rangé ; sans session au premier démarrage, où la case des
    /// douze mots vient d'être cochée.
    pub async fn confirmer_certificat_de(
        &self,
        etablissement_id: Id<Etablissement>,
        session: Option<&Session>,
    ) -> Resultat<()> {
        let contexte = self.noyau.contexte(etablissement_id, session);
        let mut unite = self
            .noyau
            .commencer(contexte.clone(), Durabilite::Ordinaire)
            .await?;
        parametre::definir(
            unite.connexion(),
            &Parametre {
                etablissement_id,
                cle: PARAMETRE_CERTIFICAT.to_owned(),
                valeur: ValeurParametre::Booleen(true),
                modifiable_par_utilisateur: false,
            },
            &contexte,
        )
        .await?;
        unite.auditer(
            "parametre",
            PARAMETRE_CERTIFICAT,
            Operation::Modification,
            None,
            Some(serde_json::json!({ "valeur": true })),
            "sauvegarde.certificat_confirmer",
        );
        self.noyau.terminer(unite).await?;
        Ok(())
    }
}
