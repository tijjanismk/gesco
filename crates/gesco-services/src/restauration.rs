//! Restauration (spécification §11.4, UC-07, R-57) : vérification d'empreinte, contrôle de
//! compatibilité, migration automatique, restauration dans une base temporaire puis bascule
//! atomique, nouvelle enveloppe machine sur ce poste.
//!
//! La restauration travaille sur l'espace de données, sans noyau : sur une machine vierge il
//! n'existe encore ni base ni session. Sur un poste en service, l'appelant ferme la base et
//! sauvegarde l'état courant avant d'appeler (R-48).

use std::path::Path;
use std::sync::Mutex;

use gesco_core::commun::{Horodatage, Id};
use gesco_core::core::{CleDonnees, Etablissement, PhraseSecours, Utilisateur};
use gesco_db::depots::audit;
use gesco_db::{Base, nouvel_id};
use serde::{Deserialize, Serialize};

use crate::archive::{self, empreinte_fichier};
use crate::cles::EspaceCles;
use crate::erreur::{ErreurService, MotifRefus, Resultat};
use crate::espace::EspaceDonnees;
use crate::horloge::Horloge;
use crate::sauvegarde::{ApercuSauvegarde, ouvrir_fichier};

/// Phrases fausses tolérées avant l'attente progressive (§11.4).
const ECHECS_TOLERES: u32 = 3;

/// Compteur des phrases de secours fausses, en mémoire : trois échecs consécutifs imposent une
/// attente qui double à chaque nouvel échec, comme pour le mot de passe.
#[derive(Debug, Default)]
pub struct Tentatives(Mutex<(u32, Option<Horodatage>)>);

impl Tentatives {
    fn verifier(&self, maintenant: Horodatage) -> Resultat<()> {
        let attente = self.0.lock().ok().and_then(|etat| etat.1);
        match attente {
            Some(fin) if maintenant.avant(&fin) => Err(ErreurService::Authentification(
                MotifRefus::PhraseVerrouillee {
                    jusqua: fin.millis_epoque(),
                },
            )),
            _ => Ok(()),
        }
    }

    fn echec(&self, maintenant: Horodatage) {
        if let Ok(mut etat) = self.0.lock() {
            etat.0 = etat.0.saturating_add(1);
            // Même progression que le verrouillage des comptes (30 s doublées, plafond 1 h),
            // déclenchée au troisième échec.
            etat.1 = (etat.0 >= ECHECS_TOLERES)
                .then(|| Utilisateur::duree_verrouillage_millis(etat.0, ECHECS_TOLERES))
                .flatten()
                .and_then(|d| maintenant.plus_millis(d).ok());
        }
    }

    fn succes(&self) {
        if let Ok(mut etat) = self.0.lock() {
            *etat = (0, None);
        }
    }

    /// Refuse pendant l'attente imposée.
    pub(crate) fn verifier_public(&self, maintenant: Horodatage) -> Resultat<()> {
        self.verifier(maintenant)
    }

    /// Compte l'issue d'une tentative : un échec allonge l'attente, un succès la remet à zéro.
    pub(crate) fn conclure<T>(&self, issue: Resultat<T>, maintenant: Horodatage) -> Resultat<T> {
        match issue {
            Ok(v) => {
                self.succes();
                Ok(v)
            }
            Err(e) => {
                self.echec(maintenant);
                Err(e)
            }
        }
    }
}

/// Bilan d'une restauration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct RapportRestauration {
    /// Archive restaurée.
    pub apercu: ApercuSauvegarde,
    /// Version du schéma après migration éventuelle.
    pub version_schema: i64,
    /// Empreinte du contenu restauré ; égale à celle du manifeste quand aucune migration n'a eu
    /// lieu (assertion 9).
    pub empreinte_contenu: String,
    /// Vrai si la phrase de secours a été nécessaire (autre poste, trousseau perdu).
    pub par_phrase: bool,
}

/// Lit le manifeste d'une archive pour l'assistant, sans clé.
pub fn apercu(chemin: &Path) -> Resultat<ApercuSauvegarde> {
    Ok(ApercuSauvegarde::from(&archive::lire_manifeste(chemin)?))
}

fn renommer_si_present(source: &Path, cible: &Path) -> std::io::Result<()> {
    if source.exists() {
        std::fs::rename(source, cible)?;
    }
    Ok(())
}

/// Restaure une archive dans l'espace de données. `phrase` n'est exigée que si la clé locale ne
/// déchiffre pas l'archive. Rend le bilan et la clé de données, avec laquelle l'appelant rouvre
/// la base.
pub async fn restaurer(
    espace: &EspaceDonnees,
    cles: &EspaceCles,
    cle_locale: Option<&CleDonnees>,
    tentatives: &Tentatives,
    horloge: &dyn Horloge,
    archive_chemin: &Path,
    phrase: Option<&str>,
) -> Resultat<(RapportRestauration, CleDonnees)> {
    let manifeste = archive::lire_manifeste(archive_chemin)?;
    if manifeste.version_schema > Base::version_schema_cible() {
        return Err(ErreurService::Regle {
            cle: "sauvegarde.version_trop_recente",
        });
    }
    let cle_id_locale = cles.cle_id()?;
    let (cle, par_phrase) = match (cle_locale, phrase) {
        (Some(cle), None) if cle_id_locale.as_deref() == Some(manifeste.cle_id.as_str()) => {
            (cle.clone(), false)
        }
        (_, Some(saisie)) => {
            tentatives.verifier(horloge.maintenant())?;
            let ouverte = PhraseSecours::analyser(saisie)
                .map_err(ErreurService::from)
                .and_then(|p| {
                    let derivation = manifeste
                        .enveloppe_secours
                        .derivation
                        .ok_or(gesco_core::core::ErreurCle::EnveloppeFermee)?;
                    Ok(manifeste
                        .enveloppe_secours
                        .ouvrir(&p.deriver(&derivation)?)?)
                });
            match ouverte {
                Ok(cle) => {
                    tentatives.succes();
                    (cle, true)
                }
                Err(erreur) => {
                    tentatives.echec(horloge.maintenant());
                    return Err(erreur);
                }
            }
        }
        _ => {
            return Err(ErreurService::Regle {
                cle: "sauvegarde.phrase_requise",
            });
        }
    };

    espace.preparer()?;
    let travail = espace.temporaire().join(format!(
        "restauration-{}",
        nouvel_id::<Utilisateur>().vers_chaine()
    ));
    let issue = preparer_base(&travail, archive_chemin, &cle, &manifeste.empreinte_base).await;
    let (version_schema, empreinte_contenu) = match issue {
        Ok(v) => v,
        Err(e) => {
            let _ = std::fs::remove_dir_all(&travail);
            return Err(e);
        }
    };

    // Bascule : l'ancienne base et les anciens modèles sont conservés à côté, renommés.
    let marque = horloge.maintenant().millis_epoque();
    let base = espace.base();
    for suffixe in ["", "-wal", "-shm"] {
        let source = base.with_file_name(format!("gesco.db{suffixe}"));
        renommer_si_present(
            &source,
            &base.with_file_name(format!("gesco.avant-restauration-{marque}.db{suffixe}")),
        )?;
    }
    std::fs::rename(travail.join("gesco.db"), &base)?;
    let modeles_restaures = travail.join("modeles");
    if modeles_restaures.is_dir() {
        renommer_si_present(
            &espace.modeles(),
            &espace
                .racine()
                .join(format!("modeles.avant-restauration-{marque}")),
        )?;
        std::fs::rename(&modeles_restaures, espace.modeles())?;
    }
    let _ = std::fs::remove_dir_all(&travail);

    if par_phrase || cle_id_locale.as_deref() != Some(manifeste.cle_id.as_str()) {
        cles.installer(&cle, manifeste.enveloppe_secours.clone())?;
    }
    Ok((
        RapportRestauration {
            apercu: ApercuSauvegarde::from(&manifeste),
            version_schema,
            empreinte_contenu,
            par_phrase,
        },
        cle,
    ))
}

/// Déchiffre l'archive dans `travail`, vérifie l'empreinte du fichier, ouvre la base, la migre si
/// elle est antérieure, contrôle son intégrité et sa chaîne d'audit. Rend la version de schéma
/// et l'empreinte du contenu.
async fn preparer_base(
    travail: &Path,
    archive_chemin: &Path,
    cle: &CleDonnees,
    empreinte_base: &str,
) -> Resultat<(i64, String)> {
    std::fs::create_dir_all(travail)?;
    let (source, dossier, cle_flux) = (
        archive_chemin.to_path_buf(),
        travail.to_path_buf(),
        cle.clone(),
    );
    tokio::task::spawn_blocking(move || archive::extraire(&source, &cle_flux, &dossier))
        .await
        .map_err(|e| ErreurService::Secret(e.to_string()))??;
    let fichier = travail.join("gesco.db");
    if empreinte_fichier(&fichier)? != empreinte_base {
        return Err(ErreurService::Regle {
            cle: "sauvegarde.archive_alteree",
        });
    }
    let base = ouvrir_fichier(&fichier, cle).await?;
    let issue = async {
        base.migrer().await?;
        base.verifier_integrite().await?;
        let mut cnx = base
            .pool()
            .acquire()
            .await
            .map_err(gesco_db::ErreurBase::from)?;
        for etablissement in gesco_db::depots::etablissement::installes(&mut cnx).await? {
            verifier_chaine(&mut cnx, etablissement.id).await?;
        }
        drop(cnx);
        Ok::<_, ErreurService>((
            base.version_schema().await?,
            base.empreinte_contenu().await?,
        ))
    }
    .await;
    base.fermer().await;
    issue
}

async fn verifier_chaine(
    cnx: &mut gesco_db::Connexion,
    etablissement_id: Id<Etablissement>,
) -> Resultat<()> {
    let contexte = gesco_db::Contexte {
        etablissement_id,
        utilisateur_id: None,
        role_actif: None,
        poste: gesco_core::core::Poste("restauration".to_owned()),
        empreinte_machine: String::new(),
        maintenant: Horodatage::depuis_millis_epoque(0),
        correlation_id: String::new(),
    };
    audit::verifier_chaine(cnx, None, &contexte).await?;
    Ok(())
}
