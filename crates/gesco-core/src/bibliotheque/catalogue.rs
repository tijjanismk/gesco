//! Ce que la base garde de la bibliothèque : les paquets installés, les chapitres du programme et
//! les contenus, livrés ou ajoutés par l'école. Le texte livré reste dans le paquet ; la base en
//! garde le nom de fichier et l'empreinte.

use serde::{Deserialize, Serialize};

use super::paquet::{DisciplinePaquet, ExamenSujet, ManifestePaquet, NatureContenu};
use crate::commun::{Horodatage, Id};
use crate::core::Etablissement;

/// D'où vient un chapitre ou un contenu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum OrigineBibliotheque {
    /// Livré par un paquet : il ne se modifie jamais dans l'école.
    Livre,
    /// Ajouté par l'école.
    Ecole,
}

impl OrigineBibliotheque {
    /// Nom persisté.
    pub const fn nom(self) -> &'static str {
        match self {
            Self::Livre => "livre",
            Self::Ecole => "ecole",
        }
    }

    /// Retrouve une origine par son nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        match nom {
            "livre" => Some(Self::Livre),
            "ecole" => Some(Self::Ecole),
            _ => None,
        }
    }
}

/// Un paquet installé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaquetInstalle {
    /// Identifiant technique.
    pub id: Id<PaquetInstalle>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Code du paquet (`bibliotheque-mali`).
    pub code: String,
    /// Version (`2026.11.1`).
    pub version: String,
    /// Date de fabrication écrite au manifeste.
    pub date_paquet: String,
    /// Version minimale de GESCO.
    pub gesco_minimum: String,
    /// Nom du fichier dans le dossier `bibliotheque` du poste principal.
    pub nom_fichier: String,
    /// Taille du fichier.
    pub taille: i64,
    /// Empreinte SHA-256 du fichier.
    pub empreinte: String,
    /// Libellés de référence des disciplines.
    pub disciplines: Vec<DisciplinePaquet>,
    /// Vrai pour le paquet en service ; faux pour un paquet remplacé.
    pub actif: bool,
    /// Installé le.
    pub installe_le: Horodatage,
}

/// Un chapitre du programme, livré ou de l'école.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChapitreBibliotheque {
    /// Identifiant technique.
    pub id: Id<ChapitreBibliotheque>,
    /// Clé stable.
    pub cle: String,
    /// Origine.
    pub origine: OrigineBibliotheque,
    /// Paquet d'un chapitre livré.
    pub paquet_code: Option<String>,
    /// Code national du niveau (`7`, `12`…).
    pub niveau_code: String,
    /// Discipline de référence d'un chapitre livré (`MATH`…).
    pub discipline_code: Option<String>,
    /// Niveau de l'école d'un chapitre de l'école.
    pub niveau_id: Option<String>,
    /// Matière de l'école d'un chapitre de l'école.
    pub matiere_id: Option<String>,
    /// Partie (« Algèbre », « Examens »…).
    pub partie: String,
    /// Titre.
    pub titre: String,
    /// Rang dans le programme.
    pub ordre: i64,
    /// Chapitre de l'école : clé du chapitre après lequel il se range.
    pub apres_cle: Option<String>,
    /// « Programme à confirmer » (livré seulement, jamais affiché aux élèves).
    pub a_confirmer: bool,
    /// Séries (vide : toutes les séries du niveau).
    pub series: Vec<String>,
    /// Marqué « plus livré » par une mise à jour.
    pub plus_livre: bool,
}

/// Un contenu : leçon, sujet, corrigé ou document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContenuBibliotheque {
    /// Identifiant technique.
    pub id: Id<ContenuBibliotheque>,
    /// Clé stable.
    pub cle: String,
    /// Origine.
    pub origine: OrigineBibliotheque,
    /// Nature.
    pub nature: NatureContenu,
    /// Chapitre.
    pub chapitre_id: Id<ChapitreBibliotheque>,
    /// Titre.
    pub titre: String,
    /// Phrase de présentation d'un document.
    pub presentation: String,
    /// Rang dans le chapitre.
    pub ordre: i64,
    /// Contenu de l'école : clé du contenu après lequel il se range.
    pub apres_cle: Option<String>,
    /// Langue (`fr`).
    pub langue: String,
    /// Clé du contenu lié (le corrigé d'un sujet, et l'inverse).
    pub lie_cle: Option<String>,
    /// Examen d'un sujet ou d'un corrigé.
    pub examen: Option<ExamenSujet>,
    /// Série d'un sujet de l'école.
    pub serie_code: Option<String>,
    /// Session d'un sujet.
    pub session: Option<u16>,
    /// Paquet d'un contenu livré.
    pub paquet_code: Option<String>,
    /// Fichier du texte livré dans le paquet.
    pub fichier: Option<String>,
    /// Empreinte du texte livré.
    pub empreinte: Option<String>,
    /// Liste de faits jointe à une leçon livrée, dans le paquet.
    pub faits: Option<String>,
    /// Marqué « plus livré » par une mise à jour.
    pub plus_livre: bool,
}

/// Fait d'une installation de paquet : ce qui a changé dans l'index.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct BilanInstallation {
    /// Chapitres livrés nouveaux.
    pub chapitres_ajoutes: u32,
    /// Contenus livrés nouveaux.
    pub contenus_ajoutes: u32,
    /// Contenus livrés dont le texte a changé.
    pub contenus_changes: u32,
    /// Contenus qui ne sont plus dans le paquet.
    pub contenus_plus_livres: u32,
}

/// Correspondance proposée à l'installation : pour chaque discipline de référence et chaque
/// niveau où le paquet la livre, la matière active de l'école de même code, s'il y en a une.
/// `matieres` : (code, identifiant, active) des matières de l'école.
pub fn correspondances_par_code(
    manifeste: &ManifestePaquet,
    matieres: &[(String, String, bool)],
) -> Vec<(String, String, Option<String>)> {
    let mut paires: Vec<(String, String)> = manifeste
        .chapitres
        .iter()
        .map(|c| (c.discipline.clone(), c.niveau.clone()))
        .collect();
    paires.sort();
    paires.dedup();
    paires
        .into_iter()
        .map(|(discipline, niveau)| {
            let matiere = matieres
                .iter()
                .find(|(code, _, active)| *active && code.eq_ignore_ascii_case(&discipline))
                .map(|(_, id, _)| id.clone());
            (discipline, niveau, matiere)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bibliotheque::paquet::ChapitrePaquet;

    fn chapitre(discipline: &str, niveau: &str) -> ChapitrePaquet {
        ChapitrePaquet {
            cle: format!("{niveau}/{discipline}"),
            niveau: niveau.to_owned(),
            series: vec![],
            discipline: discipline.to_owned(),
            partie: String::new(),
            titre: "Chapitre".to_owned(),
            ordre: 1,
            a_confirmer: false,
        }
    }

    #[test]
    fn la_correspondance_se_pose_par_code_exact_sur_une_matiere_active() {
        let manifeste = ManifestePaquet {
            code: "p".into(),
            version: "1".into(),
            date: String::new(),
            gesco_minimum: String::new(),
            programme: String::new(),
            disciplines: vec![],
            chapitres: vec![
                chapitre("MATH", "7"),
                chapitre("MATH", "7"),
                chapitre("SVT", "9"),
                chapitre("PC", "9"),
            ],
            contenus: vec![],
            fichiers: vec![],
        };
        let matieres = vec![
            ("MATH".to_owned(), "m".to_owned(), true),
            ("SVT".to_owned(), "s".to_owned(), false),
        ];
        let c = correspondances_par_code(&manifeste, &matieres);
        assert_eq!(
            c,
            vec![
                ("MATH".into(), "7".into(), Some("m".into())),
                ("PC".into(), "9".into(), None),
                ("SVT".into(), "9".into(), None),
            ]
        );
    }

    #[test]
    fn chaque_origine_se_relit() {
        for o in [OrigineBibliotheque::Livre, OrigineBibliotheque::Ecole] {
            assert_eq!(OrigineBibliotheque::depuis_nom(o.nom()), Some(o));
        }
    }
}
