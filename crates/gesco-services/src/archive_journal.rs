//! Archive du journal d'audit (blueprint §12.4, SPEC_VERSION_1_1 §1, étape 5, T-73) : à la
//! clôture de l'année, les lignes du journal de l'année s'écrivent dans un fichier signé, rangé
//! dans le dossier des sauvegardes du poste et copié sur leurs destinations ; le journal reste
//! dans la base. Un fichier d'archive choisi se vérifie : sa signature, son école, son année.

use std::path::{Path, PathBuf};

use gesco_core::commun::Horodatage;
use gesco_core::core::archive_journal::{
    EXTENSION_ARCHIVE_JOURNAL, FORMAT_ARCHIVE_JOURNAL, signer, verifier,
};
use gesco_core::core::{AnneeScolaire, CleDonnees, LigneAudit, Operation, TypeEvenement};
use gesco_db::depots::{audit, etablissement};
use gesco_db::unite::Durabilite;
use serde::{Deserialize, Serialize};

use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Ce que la signature couvre.
#[derive(Debug, Serialize, Deserialize)]
struct ContenuArchive {
    format: String,
    etablissement: String,
    annee: String,
    du: Horodatage,
    au: Horodatage,
    lignes: Vec<LigneAudit>,
}

/// Fichier d'archive : le contenu et sa signature.
#[derive(Debug, Serialize, Deserialize)]
struct FichierArchive {
    contenu: ContenuArchive,
    signature: String,
}

/// Archive écrite à la clôture.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ArchiveJournal {
    /// Nom du fichier.
    pub fichier: String,
    /// Lignes archivées.
    pub lignes: u32,
    /// Dossiers où le fichier est écrit.
    pub ecrite: u32,
    /// Dossiers où l'écriture a échoué (clé USB absente).
    pub manquee: u32,
}

/// Ce que dit la vérification d'un fichier d'archive.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct VerificationArchive {
    /// La signature est celle de cette école et le fichier n'a pas changé.
    pub valide: bool,
    /// École.
    pub etablissement: String,
    /// Année.
    pub annee: String,
    /// Premier instant couvert.
    pub du: Horodatage,
    /// Dernier instant couvert.
    pub au: Horodatage,
    /// Lignes du journal.
    pub lignes: u32,
}

/// Fichier d'archive à vérifier.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeVerificationArchive {
    /// Chemin du fichier.
    pub chemin: String,
}

/// Nom du fichier d'archive d'une année : `journal-2026-2027.gja`.
pub fn nom_du_fichier(annee: &AnneeScolaire) -> String {
    let libelle: String = annee
        .libelle
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();
    format!("journal-{libelle}.{EXTENSION_ARCHIVE_JOURNAL}")
}

/// Écrit l'archive signée du journal de l'année dans chaque dossier ; un dossier absent ne fait
/// pas échouer les autres. L'écriture se trace dans le journal.
pub(crate) async fn archiver(
    noyau: &Noyau,
    session: &Session,
    cle: &CleDonnees,
    annee: &AnneeScolaire,
    dossiers: &[PathBuf],
) -> Resultat<ArchiveJournal> {
    let contexte = noyau.contexte(session.etablissement_id, Some(session));
    let (fiche, lignes) = {
        let mut cnx = noyau
            .base()
            .pool()
            .acquire()
            .await
            .map_err(gesco_db::ErreurBase::from)?;
        let fiche = etablissement::lire(&mut cnx, session.etablissement_id).await?;
        let lignes = audit::entre(
            &mut cnx,
            annee.debut.millis_epoque(),
            contexte.maintenant.millis_epoque().saturating_add(1),
            &contexte,
        )
        .await?;
        (fiche, lignes)
    };
    let contenu = ContenuArchive {
        format: FORMAT_ARCHIVE_JOURNAL.to_owned(),
        etablissement: fiche.nom,
        annee: annee.libelle.clone(),
        du: annee.debut,
        au: contexte.maintenant,
        lignes,
    };
    let signature = signer(cle.octets(), &serde_json::to_vec(&contenu)?);
    let nombre = u32::try_from(contenu.lignes.len()).unwrap_or(u32::MAX);
    let fichier = FichierArchive { contenu, signature };
    let octets = serde_json::to_vec_pretty(&fichier)?;
    let nom = nom_du_fichier(annee);
    let (mut ecrite, mut manquee) = (0u32, 0u32);
    for dossier in dossiers {
        let ecrit = (|| -> std::io::Result<()> {
            std::fs::create_dir_all(dossier)?;
            std::fs::write(dossier.join(&nom), &octets)
        })();
        if ecrit.is_ok() {
            ecrite += 1;
        } else {
            manquee += 1;
        }
    }
    if ecrite == 0 {
        return Err(regle("archive_journal.ecriture_impossible"));
    }
    let mut unite = noyau
        .commencer(contexte.clone(), Durabilite::Ordinaire)
        .await?;
    let id = annee.id.vers_chaine();
    unite.auditer(
        "archive_journal",
        &id,
        Operation::Creation,
        None,
        Some(
            serde_json::json!({ "fichier": nom, "lignes": nombre, "ecrite": ecrite,
            "manquee": manquee, "signature": fichier.signature }),
        ),
        "archive_journal.archiver",
    );
    unite.emettre(
        TypeEvenement::JournalArchive,
        "annee_scolaire",
        &id,
        serde_json::json!({ "annee": annee.libelle, "lignes": nombre }),
    );
    noyau.terminer(unite).await?;
    Ok(ArchiveJournal {
        fichier: nom,
        lignes: nombre,
        ecrite,
        manquee,
    })
}

/// Vérifie un fichier d'archive du journal sous la clé de l'école.
pub(crate) fn verifier_fichier(cle: &CleDonnees, chemin: &Path) -> Resultat<VerificationArchive> {
    let octets = std::fs::read(chemin)?;
    let fichier: FichierArchive =
        serde_json::from_slice(&octets).map_err(|_| regle("archive_journal.illisible"))?;
    if fichier.contenu.format != FORMAT_ARCHIVE_JOURNAL {
        return Err(regle("archive_journal.illisible"));
    }
    let valide = verifier(
        cle.octets(),
        &serde_json::to_vec(&fichier.contenu)?,
        &fichier.signature,
    );
    Ok(VerificationArchive {
        valide,
        lignes: u32::try_from(fichier.contenu.lignes.len()).unwrap_or(u32::MAX),
        etablissement: fichier.contenu.etablissement,
        annee: fichier.contenu.annee,
        du: fichier.contenu.du,
        au: fichier.contenu.au,
    })
}
