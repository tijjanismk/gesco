//! Référentiel académique de la phase 2 (SPEC_PHASE_2 §2, §3) : niveaux et séries livrés,
//! classes de l'année. Matières, programmes et périodes arrivent avec la phase 4 ; le cycle
//! technique et ses filières, avec la phase 6 (SPEC_PHASE_6 §9).

use serde::{Deserialize, Serialize};

use crate::commun::Id;
use crate::core::{AnneeScolaire, Etablissement};

/// Cycle d'enseignement ; il porte les règles propres à chaque ordre (Q-18, T-02, T-10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum Cycle {
    /// Jardin d'enfants : photo exigée, personnes qui viennent chercher l'enfant.
    Jardin,
    /// Fondamental, 1re à 6e année.
    PremierCycle,
    /// Fondamental, 7e à 9e année, jusqu'au DEF.
    SecondCycle,
    /// Lycée, 10e à 12e année : matricule national exigé.
    Lycee,
    /// Enseignement technique et professionnel : CAP et BT, filières de l'école, semestres ou
    /// trimestres ; matricule national facultatif (Q-69, Q-70, Q-71).
    Technique,
}

impl Cycle {
    /// Les cinq cycles, du jardin à l'enseignement technique.
    pub const TOUS: [Self; 5] = [
        Self::Jardin,
        Self::PremierCycle,
        Self::SecondCycle,
        Self::Lycee,
        Self::Technique,
    ];

    /// Les cycles de l'enseignement général, du jardin au lycée : ceux qu'une base installée
    /// avant la phase 6 enseignait. Le cycle technique ne s'ouvre que si l'école le choisit.
    pub const GENERAUX: [Self; 4] = [
        Self::Jardin,
        Self::PremierCycle,
        Self::SecondCycle,
        Self::Lycee,
    ];

    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Jardin => "jardin",
            Self::PremierCycle => "premier_cycle",
            Self::SecondCycle => "second_cycle",
            Self::Lycee => "lycee",
            Self::Technique => "technique",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::TOUS.into_iter().find(|c| c.nom() == nom)
    }

    /// Vrai si l'école y crée ses propres filières (SPEC_PHASE_6 §9) : seul le cycle technique ;
    /// les séries du lycée sont celles du référentiel national.
    pub const fn a_filieres(&self) -> bool {
        matches!(self, Self::Technique)
    }
}

/// Mode d'évaluation d'un niveau (ADR-013).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ModeEvaluation {
    /// Notation chiffrée.
    Notes,
    /// Observation de compétences, reportée (Q-03).
    Competences,
}

impl ModeEvaluation {
    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Notes => "notes",
            Self::Competences => "competences",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        [Self::Notes, Self::Competences]
            .into_iter()
            .find(|m| m.nom() == nom)
    }
}

/// Diplôme qui termine un niveau.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum DiplomeTerminal {
    /// Aucun examen.
    Aucun,
    /// Diplôme d'études fondamentales, en 9e année.
    Def,
    /// Baccalauréat, en 12e année.
    Bac,
    /// Certificat d'aptitude professionnelle, en 2e année de CAP.
    Cap,
    /// Brevet de technicien, en 4e année de BT.
    Bt,
}

impl DiplomeTerminal {
    /// Les cinq diplômes.
    pub const TOUS: [Self; 5] = [Self::Aucun, Self::Def, Self::Bac, Self::Cap, Self::Bt];

    /// Nom persisté.
    pub const fn nom(&self) -> &'static str {
        match self {
            Self::Aucun => "aucun",
            Self::Def => "def",
            Self::Bac => "bac",
            Self::Cap => "cap",
            Self::Bt => "bt",
        }
    }

    /// Lecture d'un nom persisté.
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::TOUS.into_iter().find(|d| d.nom() == nom)
    }
}

/// Niveau d'un établissement, copie du référentiel national que l'école renomme ou désactive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Niveau {
    /// Identifiant technique.
    pub id: Id<Niveau>,
    /// Établissement propriétaire.
    pub etablissement_id: Id<Etablissement>,
    /// Code national, stable : `PS`, `1`, `12`.
    pub code: String,
    /// Libellé affiché, modifiable.
    pub libelle: String,
    /// Cycle.
    pub cycle: Cycle,
    /// Ordre d'affichage, de 1 (petite section) à 15 (12e année).
    pub ordre: i64,
    /// Mode d'évaluation.
    pub mode_evaluation: ModeEvaluation,
    /// Diplôme terminal.
    pub diplome_terminal: DiplomeTerminal,
    /// Faux pour un niveau que l'école n'enseigne pas.
    pub actif: bool,
}

/// Série du lycée, rattachée à son niveau.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Serie {
    /// Identifiant technique.
    pub id: Id<Serie>,
    /// Établissement propriétaire.
    pub etablissement_id: Id<Etablissement>,
    /// Niveau de la série.
    pub niveau_id: Id<Niveau>,
    /// Code national, stable : `S`, `TSE`.
    pub code: String,
    /// Libellé affiché, modifiable.
    pub libelle: String,
    /// Faux pour une série que l'école n'ouvre pas.
    pub actif: bool,
}

/// Classe d'une année scolaire (R-16).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Classe {
    /// Identifiant technique.
    pub id: Id<Classe>,
    /// Établissement propriétaire.
    pub etablissement_id: Id<Etablissement>,
    /// Année de la classe.
    pub annee_id: Id<AnneeScolaire>,
    /// Niveau.
    pub niveau_id: Id<Niveau>,
    /// Série, exigée aux niveaux qui en ont.
    pub serie_id: Option<Id<Serie>>,
    /// Libellé libre, unique dans l'année : « 7e A », « 12TSE ».
    pub libelle: String,
}

/// Niveau du référentiel national livré avec le produit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NiveauLivre {
    /// Code national.
    pub code: &'static str,
    /// Libellé par défaut.
    pub libelle: &'static str,
    /// Cycle.
    pub cycle: Cycle,
    /// Mode d'évaluation.
    pub mode_evaluation: ModeEvaluation,
    /// Diplôme terminal.
    pub diplome_terminal: DiplomeTerminal,
    /// Séries du niveau : code et libellé.
    pub series: &'static [(&'static str, &'static str)],
}

const fn livre(
    code: &'static str,
    libelle: &'static str,
    cycle: Cycle,
    diplome_terminal: DiplomeTerminal,
    series: &'static [(&'static str, &'static str)],
) -> NiveauLivre {
    NiveauLivre {
        code,
        libelle,
        cycle,
        mode_evaluation: match cycle {
            Cycle::Jardin => ModeEvaluation::Competences,
            Cycle::PremierCycle | Cycle::SecondCycle | Cycle::Lycee | Cycle::Technique => {
                ModeEvaluation::Notes
            }
        },
        diplome_terminal,
        series,
    }
}

/// Référentiel national, dans l'ordre d'affichage (SPEC_PHASE_2 §2.1, §2.2 ; blueprint §8.2), puis
/// les années du CAP et du BT (SPEC_PHASE_6 §9, Q-69), sans filière : l'école crée les siennes.
pub const NIVEAUX_LIVRES: &[NiveauLivre] = {
    use Cycle::{Jardin, Lycee, PremierCycle, SecondCycle, Technique};
    use DiplomeTerminal::{Aucun, Bac, Bt, Cap, Def};
    &[
        livre("PS", "Petite section", Jardin, Aucun, &[]),
        livre("MS", "Moyenne section", Jardin, Aucun, &[]),
        livre("GS", "Grande section", Jardin, Aucun, &[]),
        livre("1", "1re année", PremierCycle, Aucun, &[]),
        livre("2", "2e année", PremierCycle, Aucun, &[]),
        livre("3", "3e année", PremierCycle, Aucun, &[]),
        livre("4", "4e année", PremierCycle, Aucun, &[]),
        livre("5", "5e année", PremierCycle, Aucun, &[]),
        livre("6", "6e année", PremierCycle, Aucun, &[]),
        livre("7", "7e année", SecondCycle, Aucun, &[]),
        livre("8", "8e année", SecondCycle, Aucun, &[]),
        livre("9", "9e année", SecondCycle, Def, &[]),
        livre("10", "10e année", Lycee, Aucun, &[]),
        livre(
            "11",
            "11e année",
            Lycee,
            Aucun,
            &[
                ("S", "Sciences"),
                ("SES", "Sciences économiques et sociales"),
                ("LL", "Langues et littérature"),
            ],
        ),
        livre(
            "12",
            "12e année",
            Lycee,
            Bac,
            &[
                ("TSE", "Sciences exactes"),
                ("TSExp", "Sciences expérimentales"),
                ("TSEco", "Sciences économiques"),
                ("TSS", "Sciences sociales"),
                ("TLL", "Langues et littérature"),
            ],
        ),
        livre("CAP1", "CAP 1re année", Technique, Aucun, &[]),
        livre("CAP2", "CAP 2e année", Technique, Cap, &[]),
        livre("BT1", "BT 1re année", Technique, Aucun, &[]),
        livre("BT2", "BT 2e année", Technique, Aucun, &[]),
        livre("BT3", "BT 3e année", Technique, Aucun, &[]),
        livre("BT4", "BT 4e année", Technique, Bt, &[]),
    ]
};

/// Longueur maximale d'un code de série ou de filière.
pub const CODE_SERIE_MAX: usize = 10;

/// Code d'une filière créée par l'école : lettres sans accent, chiffres et tiret, 10 au plus,
/// écrit en majuscules (`ELEC`, `MECA-AUTO`). L'erreur est la clé du message.
pub fn code_filiere(code: &str) -> Result<String, &'static str> {
    let propre = code.trim().to_uppercase();
    if propre.is_empty()
        || propre.chars().count() > CODE_SERIE_MAX
        || !propre
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-')
    {
        return Err("filiere.code_invalide");
    }
    Ok(propre)
}

/// Longueur maximale d'un libellé de niveau, de série ou de classe.
pub const LIBELLE_MAX: usize = 60;

/// Libellé saisi, nettoyé : non vide et borné ; l'erreur est la clé du message.
pub fn libelle_valide(libelle: &str, cle_erreur: &'static str) -> Result<String, &'static str> {
    let propre = libelle.split_whitespace().collect::<Vec<_>>().join(" ");
    if propre.is_empty() || propre.chars().count() > LIBELLE_MAX {
        return Err(cle_erreur);
    }
    Ok(propre)
}

/// Vérifie le rattachement d'une classe (SPEC_PHASE_2 §3.1) : niveau actif ; série exigée si le
/// niveau en a, refusée sinon, active et du même niveau.
pub fn verifier_rattachement(
    niveau: &Niveau,
    serie: Option<&Serie>,
    series_du_niveau: usize,
) -> Result<(), &'static str> {
    if !niveau.actif {
        return Err("classe.niveau_inactif");
    }
    match serie {
        None if series_du_niveau > 0 => Err("classe.serie_exigee"),
        None => Ok(()),
        Some(s) if s.niveau_id != niveau.id => Err("classe.serie_hors_niveau"),
        Some(s) if !s.actif => Err("classe.serie_inactive"),
        Some(_) => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn id<T>(n: u128) -> Id<T> {
        Id::depuis_ulid(ulid::Ulid::from(n))
    }

    fn niveau(n: u128, actif: bool) -> Niveau {
        Niveau {
            id: id(n),
            etablissement_id: id(0),
            code: "11".to_owned(),
            libelle: "11e année".to_owned(),
            cycle: Cycle::Lycee,
            ordre: 14,
            mode_evaluation: ModeEvaluation::Notes,
            diplome_terminal: DiplomeTerminal::Aucun,
            actif,
        }
    }

    fn serie(niveau: u128, actif: bool) -> Serie {
        Serie {
            id: id(100),
            etablissement_id: id(0),
            niveau_id: id(niveau),
            code: "S".to_owned(),
            libelle: "Sciences".to_owned(),
            actif,
        }
    }

    #[test]
    fn referentiel_livre_complet() {
        assert_eq!(NIVEAUX_LIVRES.len(), 21);
        let codes: HashSet<&str> = NIVEAUX_LIVRES.iter().map(|n| n.code).collect();
        assert_eq!(codes.len(), 21);
        let techniques: Vec<(&str, DiplomeTerminal)> = NIVEAUX_LIVRES
            .iter()
            .filter(|n| n.cycle == Cycle::Technique)
            .map(|n| (n.code, n.diplome_terminal))
            .collect();
        assert_eq!(
            techniques,
            [
                ("CAP1", DiplomeTerminal::Aucun),
                ("CAP2", DiplomeTerminal::Cap),
                ("BT1", DiplomeTerminal::Aucun),
                ("BT2", DiplomeTerminal::Aucun),
                ("BT3", DiplomeTerminal::Aucun),
                ("BT4", DiplomeTerminal::Bt),
            ]
        );
        let series: Vec<&str> = NIVEAUX_LIVRES
            .iter()
            .flat_map(|n| n.series.iter().map(|(c, _)| *c))
            .collect();
        assert_eq!(
            series,
            ["S", "SES", "LL", "TSE", "TSExp", "TSEco", "TSS", "TLL"]
        );
        for n in NIVEAUX_LIVRES {
            assert_eq!(
                n.mode_evaluation == ModeEvaluation::Competences,
                n.cycle == Cycle::Jardin,
                "{}",
                n.code
            );
            assert!(n.series.is_empty() || n.cycle == Cycle::Lycee, "{}", n.code);
        }
    }

    #[test]
    fn noms_persistes_reversibles() {
        for c in Cycle::TOUS {
            assert_eq!(Cycle::depuis_nom(c.nom()), Some(c));
        }
        for m in [ModeEvaluation::Notes, ModeEvaluation::Competences] {
            assert_eq!(ModeEvaluation::depuis_nom(m.nom()), Some(m));
        }
        for d in DiplomeTerminal::TOUS {
            assert_eq!(DiplomeTerminal::depuis_nom(d.nom()), Some(d));
        }
        assert!(
            Cycle::TOUS
                .iter()
                .all(|c| c.a_filieres() == (*c == Cycle::Technique))
        );
    }

    #[test]
    fn rattachement_d_une_classe() {
        let onze = niveau(1, true);
        assert_eq!(
            verifier_rattachement(&onze, None, 3),
            Err("classe.serie_exigee")
        );
        assert_eq!(
            verifier_rattachement(&onze, Some(&serie(1, true)), 3),
            Ok(())
        );
        assert_eq!(
            verifier_rattachement(&onze, Some(&serie(2, true)), 3),
            Err("classe.serie_hors_niveau")
        );
        assert_eq!(
            verifier_rattachement(&onze, Some(&serie(1, false)), 3),
            Err("classe.serie_inactive")
        );
        assert_eq!(verifier_rattachement(&niveau(1, true), None, 0), Ok(()));
        assert_eq!(
            verifier_rattachement(&niveau(1, false), None, 0),
            Err("classe.niveau_inactif")
        );
    }

    #[test]
    fn le_code_d_une_filiere() {
        assert_eq!(code_filiere(" elec "), Ok("ELEC".to_owned()));
        assert_eq!(code_filiere("MECA-AUTO"), Ok("MECA-AUTO".to_owned()));
        assert_eq!(code_filiere(""), Err("filiere.code_invalide"));
        assert_eq!(code_filiere("Électricité"), Err("filiere.code_invalide"));
        assert_eq!(code_filiere("ABCDEFGHIJK"), Err("filiere.code_invalide"));
    }

    #[test]
    fn libelles() {
        assert_eq!(
            libelle_valide("  12TSE   B ", "x"),
            Ok("12TSE B".to_owned())
        );
        assert_eq!(libelle_valide("   ", "x"), Err("x"));
        assert_eq!(libelle_valide(&"a".repeat(61), "x"), Err("x"));
    }
}
