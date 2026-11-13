//! Évaluation par compétences au jardin d'enfants (SPEC_PHASE_11_B §3) : des domaines communs aux
//! trois sections, les compétences de chaque section, trois niveaux d'acquisition dont seuls les
//! libellés se règlent. La liste n'a pas d'année scolaire ; GESCO en livre une, rédigée à neuf
//! d'après le programme des jardins d'enfants, que l'école retire, complète, renomme et
//! réordonne. Rien ne se supprime (R-14) : un domaine ou une compétence se retire et se remet.

use serde::{Deserialize, Serialize};

use crate::commun::{Horodatage, Id};
use crate::core::{AnneeScolaire, Etablissement};
use crate::pedagogie::{Enseignant, Periode};
use crate::scolarite::{Classe, Inscription, Niveau};

/// Réglages du jardin, dans les paramètres de l'école.
pub mod parametres {
    /// Libellé du niveau « acquis ».
    pub const ECHELLE_ACQUIS: &str = "jardin.echelle_acquis";
    /// Libellé du niveau « en cours ».
    pub const ECHELLE_EN_COURS: &str = "jardin.echelle_en_cours";
    /// Libellé du niveau « non acquis ».
    pub const ECHELLE_NON_ACQUIS: &str = "jardin.echelle_non_acquis";
    /// Vrai si la ligne de commentaire par domaine se montre à la saisie et à l'impression.
    pub const COMMENTAIRE_DOMAINE: &str = "jardin.commentaire_domaine";
}

/// Longueur maximale d'un libellé de domaine.
pub const LIBELLE_DOMAINE_MAX: usize = 60;
/// Longueur maximale d'un libellé de compétence.
pub const LIBELLE_COMPETENCE_MAX: usize = 120;
/// Longueur maximale d'un libellé de l'échelle.
pub const LIBELLE_ECHELLE_MAX: usize = 30;

/// Niveau d'acquisition d'une compétence : trois, toujours (RB-4) ; une observation vidée n'en
/// est pas un quatrième.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum NiveauAcquisition {
    /// La compétence est acquise.
    Acquis,
    /// L'enfant y travaille.
    EnCours,
    /// Pas encore acquise.
    NonAcquis,
}

impl NiveauAcquisition {
    /// Les trois niveaux, du plus avancé au moins avancé.
    pub const TOUS: [Self; 3] = [Self::Acquis, Self::EnCours, Self::NonAcquis];

    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Acquis => "acquis",
            Self::EnCours => "en_cours",
            Self::NonAcquis => "non_acquis",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::TOUS.into_iter().find(|n| n.nom() == nom)
    }

    /// Réglage qui porte le libellé du niveau.
    pub const fn parametre(&self) -> &'static str {
        match self {
            Self::Acquis => parametres::ECHELLE_ACQUIS,
            Self::EnCours => parametres::ECHELLE_EN_COURS,
            Self::NonAcquis => parametres::ECHELLE_NON_ACQUIS,
        }
    }

    /// Libellé livré.
    pub const fn libelle_livre(&self) -> &'static str {
        match self {
            Self::Acquis => "Acquis",
            Self::EnCours => "En cours",
            Self::NonAcquis => "Non acquis",
        }
    }
}

/// Vrai si le libellé d'un niveau de l'échelle est vide ou trop long ; `None` pour une autre clé.
pub fn echelle_hors_bornes(cle: &str, texte: &str) -> Option<bool> {
    NiveauAcquisition::TOUS
        .iter()
        .any(|n| n.parametre() == cle)
        .then(|| {
            let t = texte.trim();
            t.is_empty() || t.chars().count() > LIBELLE_ECHELLE_MAX
        })
}

/// Domaine du jardin, commun aux trois sections.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DomaineCompetence {
    /// Identifiant technique.
    pub id: Id<DomaineCompetence>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Code stable d'un domaine livré (`langage`), qui le retrouve même renommé ; vide pour un
    /// domaine ajouté par l'école.
    pub code: Option<String>,
    /// « Langage », « Motricité ».
    pub libelle: String,
    /// Ordre d'affichage et d'impression.
    pub ordre: i64,
    /// Vrai pour un domaine de la liste livrée.
    pub livre: bool,
    /// Instant du retrait ; vide tant que le domaine sert.
    pub retire_le: Option<Horodatage>,
}

/// Compétence d'une section du jardin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Competence {
    /// Identifiant technique.
    pub id: Id<Competence>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Domaine.
    pub domaine_id: Id<DomaineCompetence>,
    /// Section : petite, moyenne ou grande.
    pub niveau_id: Id<Niveau>,
    /// Ce que l'enfant sait faire, en une phrase.
    pub libelle: String,
    /// Ordre dans le domaine.
    pub ordre: i64,
    /// Vrai pour une compétence de la liste livrée.
    pub livree: bool,
    /// Instant du retrait ; vide tant que la compétence sert.
    pub retiree_le: Option<Horodatage>,
}

/// Observation d'une compétence pour un enfant dans une période (SPEC_PHASE_11_B §3.2) : une
/// seule par enfant, compétence et période (RB-5). Vidée, elle garde sa ligne sans niveau.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Observation {
    /// Inscription de l'enfant.
    pub inscription_id: Id<Inscription>,
    /// Compétence observée.
    pub competence_id: Id<Competence>,
    /// Période.
    pub periode_id: Id<Periode>,
    /// Niveau ; vide pour une observation vidée.
    pub niveau: Option<NiveauAcquisition>,
    /// Instant de la dernière écriture, en millisecondes : celui que renvoie la saisie suivante
    /// pour ne pas écraser en silence ce qu'un autre poste a écrit entre-temps.
    pub modifiee_le: i64,
}

/// Maître d'une classe du jardin (SPEC_PHASE_11_B §7) : une affectation sans matière, puisque
/// le jardin n'en a pas. Elle décide quels enfants il observe quand l'école ouvre la saisie aux
/// maîtres.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct MaitreJardin {
    /// Identifiant technique.
    pub id: Id<MaitreJardin>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Année.
    pub annee_id: Id<AnneeScolaire>,
    /// Classe du jardin.
    pub classe_id: Id<Classe>,
    /// Enseignant.
    pub enseignant_id: Id<Enseignant>,
}

/// Longueur maximale d'une appréciation ou d'un commentaire de domaine.
pub const APPRECIATION_MAX: usize = 300;

/// Clé, dans la table des appréciations, du commentaire d'un domaine pour un enfant ; à côté de
/// `generale`, celle de l'appréciation générale.
pub fn cle_commentaire_domaine(domaine: Id<DomaineCompetence>) -> String {
    format!("domaine:{domaine}")
}

/// Rend le libellé nettoyé d'un domaine, ou la clé du refus.
pub fn verifier_libelle_domaine(libelle: &str) -> Result<String, &'static str> {
    verifier(
        libelle,
        LIBELLE_DOMAINE_MAX,
        "jardin.domaine_libelle_invalide",
    )
}

/// Rend le libellé nettoyé d'une compétence, ou la clé du refus.
pub fn verifier_libelle_competence(libelle: &str) -> Result<String, &'static str> {
    verifier(
        libelle,
        LIBELLE_COMPETENCE_MAX,
        "jardin.competence_libelle_invalide",
    )
}

fn verifier(libelle: &str, max: usize, cle: &'static str) -> Result<String, &'static str> {
    let propre = libelle.split_whitespace().collect::<Vec<_>>().join(" ");
    if propre.is_empty() || propre.chars().count() > max {
        return Err(cle);
    }
    Ok(propre)
}

/// Codes des trois sections du jardin, dans l'ordre des colonnes de [`DomaineLivre`].
pub const SECTIONS: [&str; 3] = ["PS", "MS", "GS"];

/// Domaine de la liste livrée, avec ses compétences par section.
#[derive(Debug, Clone, Copy)]
pub struct DomaineLivre {
    /// Code stable.
    pub code: &'static str,
    /// Libellé.
    pub libelle: &'static str,
    /// Compétences de la petite, de la moyenne et de la grande section.
    pub competences: [&'static [&'static str]; 3],
}

impl DomaineLivre {
    /// Compétences livrées d'une section, reconnue par le code de son niveau ; rien pour un autre
    /// niveau.
    pub fn de_la_section(&self, code_niveau: &str) -> &'static [&'static str] {
        SECTIONS
            .iter()
            .position(|s| *s == code_niveau)
            .and_then(|i| self.competences.get(i))
            .copied()
            .unwrap_or(&[])
    }
}

/// Liste livrée : cinq domaines, quatre compétences par domaine et par section (T-109), rédigée
/// à neuf d'après le programme des jardins d'enfants ; rien n'y est recopié d'un document.
pub const DOMAINES_LIVRES: [DomaineLivre; 5] = [
    DomaineLivre {
        code: "langage",
        libelle: "Langage",
        competences: [
            &[
                "Comprend une consigne simple qu'on lui dit",
                "Se fait comprendre par des mots ou de courtes phrases",
                "Dit une comptine apprise en classe",
                "Nomme les objets de la classe et les parties du corps",
            ],
            &[
                "Décrit une image ou un objet avec ses propres mots",
                "Raconte en quelques phrases ce qu'il a fait",
                "Pose des questions et répond à celles qu'on lui pose",
                "Reconnaît son prénom écrit",
            ],
            &[
                "Raconte dans l'ordre une histoire entendue",
                "Entend et sépare les syllabes d'un mot",
                "Reconnaît des lettres et écrit son prénom",
                "Donne en français le nom de quelques objets et animaux de la maison",
            ],
        ],
    },
    DomaineLivre {
        code: "motricite",
        libelle: "Motricité",
        competences: [
            &[
                "Marche, court et s'arrête au signal",
                "Lance un ballon sans viser et court le rattraper",
                "Tient un crayon et trace des traits et des ronds",
                "Enfile de grosses perles",
            ],
            &[
                "Saute à pieds joints",
                "Se déplace en équilibre sur une ligne",
                "Découpe en suivant un trait droit",
                "Vise une cible en lançant et rattrape un ballon à deux mains",
            ],
            &[
                "Saute à cloche-pied",
                "Joue à un jeu collectif en respectant la règle",
                "Découpe des formes variées et les colle pour composer une image",
                "Tient bien son crayon et forme les lettres",
            ],
        ],
    },
    DomaineLivre {
        code: "vivre_ensemble",
        libelle: "Vivre ensemble",
        competences: [
            &[
                "Dit bonjour et au revoir aux adultes et aux camarades",
                "Joue à côté des autres enfants",
                "Range le matériel avec de l'aide",
                "Lave ses mains avant le repas et prévient quand il a besoin des toilettes",
            ],
            &[
                "Attend son tour",
                "Partage le matériel",
                "Dit merci et pardon quand il le faut",
                "Respecte une règle de la classe",
            ],
            &[
                "Aide un camarade",
                "Dit son désaccord par la parole",
                "Prend une responsabilité dans la classe",
                "Connaît l'hymne national, le chante debout et respecte le drapeau",
            ],
        ],
    },
    DomaineLivre {
        code: "decouverte_monde",
        libelle: "Découverte du monde",
        competences: [
            &[
                "Reconnaît les couleurs principales",
                "Trie des objets selon leur taille",
                "Compte une petite collection jusqu'à trois",
                "Distingue le matin, le midi, le soir et la nuit",
            ],
            &[
                "Dénombre une petite collection jusqu'à cinq",
                "Reconnaît le rond, le carré, le triangle et le rectangle",
                "Situe un objet : dessus, dessous, devant, derrière",
                "Nomme des animaux et des plantes de son milieu",
            ],
            &[
                "Dénombre jusqu'à dix et associe le chiffre",
                "Compare deux quantités",
                "Range des objets du plus petit au plus grand",
                "Dit en quelle saison on est et quel temps il fait",
            ],
        ],
    },
    DomaineLivre {
        code: "arts",
        libelle: "Arts",
        competences: [
            &[
                "Chante avec le groupe",
                "Peint et colorie librement",
                "Modèle avec de la pâte ou de l'argile",
                "Bouge et danse quand on joue du tam-tam ou du balafon",
            ],
            &[
                "Colorie en restant dans la forme",
                "Danse en suivant le rythme",
                "Dessine un bonhomme",
                "Mime une action ou un animal",
            ],
            &[
                "Dessine un bonhomme complet",
                "Enchaîne quelques pas d'une danse de son milieu",
                "Fabrique un objet en suivant des étapes",
                "Joue un personnage d'un conte ou d'une saynète devant les autres",
            ],
        ],
    },
];

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::*;

    #[test]
    fn la_liste_livree_tient_dans_les_bornes() {
        for d in DOMAINES_LIVRES {
            assert_eq!(
                verifier_libelle_domaine(d.libelle),
                Ok(d.libelle.to_owned())
            );
            for section in SECTIONS {
                let competences = d.de_la_section(section);
                assert_eq!(competences.len(), 4, "{} {section}", d.libelle);
                for c in competences {
                    assert_eq!(verifier_libelle_competence(c), Ok((*c).to_owned()));
                }
            }
        }
        assert!(DOMAINES_LIVRES[0].de_la_section("1").is_empty());
    }

    #[test]
    fn une_vingtaine_de_competences_par_section_sans_doublon() {
        for section in SECTIONS {
            let toutes: Vec<&str> = DOMAINES_LIVRES
                .iter()
                .flat_map(|d| d.de_la_section(section).iter().copied())
                .collect();
            assert_eq!(toutes.len(), 20);
            let mut uniques = toutes.clone();
            uniques.sort_unstable();
            uniques.dedup();
            assert_eq!(uniques.len(), toutes.len(), "{section}");
        }
    }

    #[test]
    fn les_libelles_se_nettoient_et_se_bornent() {
        assert_eq!(
            verifier_libelle_competence("  Saute   à pieds joints "),
            Ok("Saute à pieds joints".to_owned())
        );
        assert_eq!(
            verifier_libelle_competence("   "),
            Err("jardin.competence_libelle_invalide")
        );
        assert_eq!(
            verifier_libelle_domaine(&"x".repeat(61)),
            Err("jardin.domaine_libelle_invalide")
        );
        assert!(verifier_libelle_domaine(&"é".repeat(60)).is_ok());
    }

    #[test]
    fn trois_niveaux_dont_seuls_les_libelles_se_reglent() {
        for n in NiveauAcquisition::TOUS {
            assert_eq!(NiveauAcquisition::depuis_nom(n.nom()), Some(n));
            assert_eq!(
                echelle_hors_bornes(n.parametre(), n.libelle_livre()),
                Some(false)
            );
            assert_eq!(echelle_hors_bornes(n.parametre(), " "), Some(true));
        }
        assert_eq!(NiveauAcquisition::depuis_nom("non_observe"), None);
        assert_eq!(
            echelle_hors_bornes(parametres::ECHELLE_ACQUIS, &"a".repeat(31)),
            Some(true)
        );
        assert_eq!(echelle_hors_bornes("conduite.mode", ""), None);
    }
}
