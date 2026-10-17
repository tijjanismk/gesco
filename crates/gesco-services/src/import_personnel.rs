//! Import du personnel (SPEC_VERSION_1_1 §1, étape 8) : une liste Excel ou CSV, sur le modèle
//! fourni, vérifiée ligne par ligne avant l'enregistrement, comme l'import des élèves. Une ligne
//! en erreur, ou une personne déjà au registre, est laissée de côté ; les autres deviennent des
//! fiches du personnel, avec le matricule suivant.

use std::path::Path;

use gesco_core::core::{Operation, TypeEvenement};
use gesco_core::rh::MoyenPaiement;
use gesco_core::scolarite::Sexe;
use gesco_core::scolarite::telephone::normaliser;
use gesco_core::tresorerie::releve::lire_date;
use gesco_db::nouvel_id;
use gesco_db::unite::Durabilite;
use serde::{Deserialize, Serialize};

use crate::erreur::{ErreurService, Resultat};
use crate::export::{DemandeExport, FichierEcrit};
use crate::noyau::Noyau;
use crate::personnel::{DemandeMembre, ServicePersonnel};
use crate::session::Session;

fn regle(cle: &'static str) -> ErreurService {
    ErreurService::Regle { cle }
}

/// Colonnes du modèle, dans l'ordre ; les trois premières sont exigées.
pub const COLONNES_MODELE: [&str; 10] = [
    "Nom",
    "Prénoms",
    "Catégorie",
    "Fonction",
    "Sexe",
    "Date de naissance",
    "Téléphone",
    "Date d'arrivée",
    "Matricule",
    "Fonctionnaire",
];

/// Une ligne de la liste, telle que GESCO la comprend.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct LigneImportPersonnel {
    /// Numéro de la ligne dans le fichier, en-tête compris.
    pub numero: u32,
    /// Nom.
    pub nom: String,
    /// Prénoms.
    pub prenoms: String,
    /// Catégorie, telle qu'écrite.
    pub categorie: String,
    /// Fonction.
    pub fonction: String,
    /// Clés des erreurs qui la laissent de côté.
    pub erreurs: Vec<String>,
    /// Une personne du même nom est déjà au registre : elle n'est pas importée.
    pub doublon: bool,
}

/// Ce que GESCO a compris de la liste.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct AnalyseImportPersonnel {
    /// Lignes.
    pub lignes: Vec<LigneImportPersonnel>,
    /// Lignes prêtes à importer.
    pub pretes: u32,
}

/// Ce que l'import a fait.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct BilanImportPersonnel {
    /// Fiches créées.
    pub importes: u32,
    /// Lignes laissées de côté : en erreur, déjà au registre, ou refusées à l'enregistrement.
    pub ignorees: u32,
}

/// Liste du personnel choisie.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeImportPersonnel {
    /// Chemin du fichier.
    pub chemin: String,
}

/// Classeur du modèle à écrire.
#[derive(Debug, Deserialize, specta::Type)]
pub struct DemandeModelePersonnel {
    /// Chemin du classeur, extension `.xlsx` ; rendu par le dialogue du système.
    pub chemin: String,
}

/// Écrit le modèle : les colonnes attendues, sans ligne.
pub async fn ecrire_modele(demande: DemandeModelePersonnel) -> Resultat<FichierEcrit> {
    crate::export::exporter_xlsx(DemandeExport {
        chemin: demande.chemin,
        titre: "Personnel".to_owned(),
        entetes: COLONNES_MODELE.iter().map(|c| (*c).to_owned()).collect(),
        lignes: Vec::new(),
    })
    .await
}

/// Une ligne lue, et la fiche qu'elle donne si sa catégorie est connue.
struct Lue {
    demande: Option<DemandeMembre>,
    ligne: LigneImportPersonnel,
}

fn sans_accents(texte: &str) -> String {
    texte
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'à' | 'â' => 'a',
            'î' | 'ï' => 'i',
            'ô' => 'o',
            'û' | 'ù' => 'u',
            'ç' => 'c',
            '’' => '\'',
            autre => autre,
        })
        .collect()
}

/// Service de l'import du personnel.
pub struct ServiceImportPersonnel<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceImportPersonnel<'n> {
    /// Service sur le noyau.
    pub fn nouveau(noyau: &'n Noyau) -> Self {
        Self { noyau }
    }

    async fn lire(&self, session: &Session, chemin: &str) -> Resultat<Vec<Lue>> {
        let lignes = crate::import_eleves::lire_fichier(chemin)?
            .into_iter()
            .next()
            .map(|(_, l)| l)
            .ok_or(regle("import_personnel.fichier_vide"))?;
        let entete: Vec<String> = lignes
            .first()
            .map(|l| l.iter().map(|c| sans_accents(c)).collect())
            .unwrap_or_default();
        let colonne = |nom: &str| entete.iter().position(|e| *e == sans_accents(nom));
        let (Some(c_nom), Some(c_prenoms), Some(c_categorie)) =
            (colonne("Nom"), colonne("Prénoms"), colonne("Catégorie"))
        else {
            return Err(regle("import_personnel.colonnes"));
        };
        let c_fonction = colonne("Fonction");
        let c_sexe = colonne("Sexe");
        let c_naissance = colonne("Date de naissance");
        let c_telephone = colonne("Téléphone");
        let c_arrivee = colonne("Date d'arrivée");
        let c_matricule = colonne("Matricule");
        let c_fonctionnaire = colonne("Fonctionnaire");
        let personnel = ServicePersonnel::nouveau(self.noyau);
        let referentiel = personnel.referentiel(session).await?;
        let existants: Vec<(String, String)> = personnel
            .lister(session)
            .await?
            .into_iter()
            .map(|m| (sans_accents(&m.nom), sans_accents(&m.prenoms)))
            .collect();
        let mut lues = Vec::new();
        for (i, l) in lignes.iter().enumerate().skip(1) {
            let cellule = |c: Option<usize>| {
                c.and_then(|c| l.get(c))
                    .map(|t| t.trim().to_owned())
                    .unwrap_or_default()
            };
            let mut erreurs = Vec::new();
            let nom = cellule(Some(c_nom));
            let prenoms = cellule(Some(c_prenoms));
            if nom.is_empty() || prenoms.is_empty() {
                erreurs.push("personnel.nom_obligatoire".to_owned());
            }
            let categorie_texte = cellule(Some(c_categorie));
            let lue = sans_accents(&categorie_texte);
            // La catégorie se reconnaît à son libellé, ou au code d'une catégorie livrée.
            let categorie = referentiel.categories.iter().find(|c| {
                c.active
                    && (sans_accents(&c.libelle) == lue
                        || c.code.as_deref().is_some_and(|code| code == lue))
            });
            if categorie.is_none() {
                erreurs.push("import_personnel.categorie_inconnue".to_owned());
            }
            let sexe = match sans_accents(&cellule(c_sexe)).as_str() {
                "" => None,
                "m" | "masculin" | "homme" | "h" => Some(Sexe::Masculin),
                "f" | "feminin" | "femme" => Some(Sexe::Feminin),
                _ => {
                    erreurs.push("import_personnel.sexe_illisible".to_owned());
                    None
                }
            };
            let date = |c: Option<usize>, erreurs: &mut Vec<String>| {
                let texte = cellule(c);
                if texte.is_empty() {
                    return None;
                }
                let d = lire_date(&texte);
                if d.is_none() {
                    erreurs.push("import_personnel.date_illisible".to_owned());
                }
                d
            };
            let date_naissance = date(c_naissance, &mut erreurs);
            let arrive_le = date(c_arrivee, &mut erreurs);
            let fonctionnaire = matches!(
                sans_accents(&cellule(c_fonctionnaire)).as_str(),
                "oui" | "o" | "x" | "1"
            );
            let telephone = cellule(c_telephone);
            if !telephone.is_empty() && normaliser(&telephone).is_err() {
                erreurs.push("import_personnel.telephone_illisible".to_owned());
            }
            let doublon = existants
                .iter()
                .any(|(n, p)| *n == sans_accents(&nom) && *p == sans_accents(&prenoms));
            let ligne = LigneImportPersonnel {
                numero: u32::try_from(i + 1).unwrap_or(u32::MAX),
                nom: nom.clone(),
                prenoms: prenoms.clone(),
                categorie: categorie_texte,
                fonction: cellule(c_fonction),
                erreurs,
                doublon,
            };
            let demande = categorie.map(|categorie| DemandeMembre {
                id: None,
                nom,
                prenoms,
                sexe,
                date_naissance,
                telephones: if telephone.is_empty() {
                    Vec::new()
                } else {
                    vec![telephone]
                },
                adresse: String::new(),
                matricule: cellule(c_matricule),
                categorie_id: categorie.id,
                fonction: ligne.fonction.clone(),
                arrive_le,
                fonctionnaire,
                numero_fonctionnaire: String::new(),
                numero_inps: String::new(),
                prevenir_nom: String::new(),
                prevenir_telephone: String::new(),
                paiement: MoyenPaiement::Especes,
                diplomes: Vec::new(),
                utilisateur_id: None,
                situation_famille: None,
                enfants_a_charge: None,
            });
            lues.push(Lue { demande, ligne });
        }
        Ok(lues)
    }

    /// Ce que GESCO comprend de la liste, sans rien enregistrer.
    pub async fn analyser(
        &self,
        session: &Session,
        demande: &DemandeImportPersonnel,
    ) -> Resultat<AnalyseImportPersonnel> {
        let lues = self.lire(session, &demande.chemin).await?;
        let pretes = lues
            .iter()
            .filter(|l| l.ligne.erreurs.is_empty() && !l.ligne.doublon)
            .count();
        Ok(AnalyseImportPersonnel {
            pretes: u32::try_from(pretes).unwrap_or(u32::MAX),
            lignes: lues.into_iter().map(|l| l.ligne).collect(),
        })
    }

    /// Crée la fiche de chaque ligne prête ; chacune s'enregistre et se trace comme une fiche
    /// saisie à la main.
    pub async fn appliquer(
        &self,
        session: &Session,
        demande: &DemandeImportPersonnel,
    ) -> Resultat<BilanImportPersonnel> {
        let lues = self.lire(session, &demande.chemin).await?;
        let personnel = ServicePersonnel::nouveau(self.noyau);
        let (mut importes, mut ignorees) = (0u32, 0u32);
        for l in lues {
            let Some(demande) = l
                .demande
                .filter(|_| l.ligne.erreurs.is_empty() && !l.ligne.doublon)
            else {
                ignorees += 1;
                continue;
            };
            match personnel.enregistrer(session, &demande).await {
                Ok(_) => importes += 1,
                Err(ErreurService::Regle { .. }) => ignorees += 1,
                Err(e) => return Err(e),
            }
        }
        let bilan = BilanImportPersonnel { importes, ignorees };
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut unite = self
            .noyau
            .commencer(contexte, Durabilite::Ordinaire)
            .await?;
        let id = nouvel_id::<BilanImportPersonnel>().vers_chaine();
        let fichier = Path::new(&demande.chemin)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_owned();
        unite.auditer(
            "import_personnel",
            &id,
            Operation::Creation,
            None,
            Some(serde_json::json!({ "fichier": fichier, "bilan": bilan })),
            "import_personnel.appliquer",
        );
        unite.emettre(
            TypeEvenement::PersonnelImporte,
            "import_personnel",
            &id,
            serde_json::json!({ "fichier": fichier, "importes": importes }),
        );
        self.noyau.terminer(unite).await?;
        Ok(bilan)
    }
}
