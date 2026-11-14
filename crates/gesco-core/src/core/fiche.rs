//! Fiche de l'établissement pour les rapports (SPEC_PHASE_9 §8.1) : un catalogue de rubriques,
//! livrées par GESCO ou ajoutées par l'école, et une réponse par rubrique, pour une année quand
//! la rubrique est annuelle. Tout est facultatif : une rubrique sans réponse n'a pas de ligne.
//! Rien ne se supprime (R-14) : une rubrique se désactive et se réactive ; une réponse se vide.

use serde::{Deserialize, Serialize};

use crate::commun::{DateCivile, Horodatage, Id};
use crate::core::Etablissement;

/// Longueur maximale d'un libellé de rubrique.
pub const LIBELLE_RUBRIQUE_MAX: usize = 120;
/// Longueur maximale d'une aide.
pub const AIDE_RUBRIQUE_MAX: usize = 300;
/// Longueur maximale d'un choix.
pub const CHOIX_MAX: usize = 60;
/// Nombre maximal de choix d'une rubrique.
pub const CHOIX_NOMBRE_MAX: usize = 30;
/// Longueur maximale d'une réponse en texte.
pub const REPONSE_MAX: usize = 1000;
/// Séparateur des choix et des réponses à plusieurs choix dans la base.
pub const SEPARATEUR: char = '\n';

/// Thème d'une rubrique : l'ordre des groupes à l'écran.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ThemeRubrique {
    /// Région, cercle, commune, adresse, distances.
    Localisation,
    /// Sigle, code, confession, type d'enseignement, milieu.
    Identite,
    /// Décision de création, arrêté d'ouverture, constructions.
    Actes,
    /// Eau, latrines, hygiène.
    EauHygiene,
    /// Électricité, internet.
    Energie,
    /// Bibliothèque, laboratoire, terrain de sport, cantine.
    Equipements,
    /// Comité de gestion, associations, conseils, formations.
    VieEcole,
    /// Accès des élèves handicapés.
    Accessibilite,
    /// Rubriques que l'école range à part.
    Autres,
}

impl ThemeRubrique {
    /// Tous les thèmes, dans l'ordre d'affichage.
    pub const TOUS: [Self; 9] = [
        Self::Localisation,
        Self::Identite,
        Self::Actes,
        Self::EauHygiene,
        Self::Energie,
        Self::Equipements,
        Self::VieEcole,
        Self::Accessibilite,
        Self::Autres,
    ];

    /// Nom stable, rangé dans la base.
    pub const fn nom(self) -> &'static str {
        match self {
            Self::Localisation => "localisation",
            Self::Identite => "identite",
            Self::Actes => "actes",
            Self::EauHygiene => "eau_hygiene",
            Self::Energie => "energie",
            Self::Equipements => "equipements",
            Self::VieEcole => "vie_ecole",
            Self::Accessibilite => "accessibilite",
            Self::Autres => "autres",
        }
    }

    /// Thème d'un nom de la base ; « autres » pour un nom inconnu.
    pub fn depuis_nom(nom: &str) -> Self {
        Self::TOUS
            .into_iter()
            .find(|t| t.nom() == nom)
            .unwrap_or(Self::Autres)
    }
}

/// Type de réponse d'une rubrique.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum TypeReponse {
    /// « oui » ou « non ».
    OuiNon,
    /// Un nombre, entier ou décimal.
    Nombre,
    /// Un texte court.
    Texte,
    /// Une date, AAAA-MM-JJ.
    Date,
    /// Un choix dans la liste de la rubrique.
    Choix,
    /// Plusieurs choix dans la liste de la rubrique.
    PlusieursChoix,
}

impl TypeReponse {
    /// Tous les types.
    pub const TOUS: [Self; 6] = [
        Self::OuiNon,
        Self::Nombre,
        Self::Texte,
        Self::Date,
        Self::Choix,
        Self::PlusieursChoix,
    ];

    /// Nom stable, rangé dans la base.
    pub const fn nom(self) -> &'static str {
        match self {
            Self::OuiNon => "oui_non",
            Self::Nombre => "nombre",
            Self::Texte => "texte",
            Self::Date => "date",
            Self::Choix => "choix",
            Self::PlusieursChoix => "plusieurs_choix",
        }
    }

    /// Type d'un nom de la base ; texte pour un nom inconnu.
    pub fn depuis_nom(nom: &str) -> Self {
        Self::TOUS
            .into_iter()
            .find(|t| t.nom() == nom)
            .unwrap_or(Self::Texte)
    }

    /// Vrai si la rubrique a une liste de choix.
    pub const fn a_des_choix(self) -> bool {
        matches!(self, Self::Choix | Self::PlusieursChoix)
    }
}

/// Rubrique de la fiche de l'établissement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct RubriqueFiche {
    /// Identifiant technique.
    pub id: Id<RubriqueFiche>,
    /// Établissement.
    pub etablissement_id: Id<Etablissement>,
    /// Code stable d'une rubrique livrée (`cercle`) ; vide pour une rubrique de l'école.
    pub code: Option<String>,
    /// Thème.
    pub theme: ThemeRubrique,
    /// Libellé.
    pub libelle: String,
    /// Aide affichée sous la case ; vide le plus souvent.
    pub aide: String,
    /// Type de réponse.
    pub type_reponse: TypeReponse,
    /// Choix proposés, pour un choix ou plusieurs choix.
    pub choix: Vec<String>,
    /// Vrai si la réponse vaut pour une année ; faux si elle vaut d'une année à l'autre.
    pub annuelle: bool,
    /// Vrai pour une rubrique livrée : elle ne se renomme pas.
    pub livree: bool,
    /// Ordre dans son thème.
    pub ordre: i64,
    /// Instant de la désactivation ; vide tant que la rubrique sert.
    pub retiree_le: Option<Horodatage>,
}

/// Rubrique livrée avec GESCO.
#[derive(Debug, Clone, Copy)]
pub struct RubriqueLivree {
    /// Code stable.
    pub code: &'static str,
    /// Thème.
    pub theme: ThemeRubrique,
    /// Libellé.
    pub libelle: &'static str,
    /// Type de réponse.
    pub type_reponse: TypeReponse,
    /// Choix proposés.
    pub choix: &'static [&'static str],
    /// Vrai si la réponse vaut pour une année.
    pub annuelle: bool,
}

const fn r(
    code: &'static str,
    theme: ThemeRubrique,
    libelle: &'static str,
    type_reponse: TypeReponse,
    choix: &'static [&'static str],
    annuelle: bool,
) -> RubriqueLivree {
    RubriqueLivree {
        code,
        theme,
        libelle,
        type_reponse,
        choix,
        annuelle,
    }
}

use ThemeRubrique as Th;
use TypeReponse as Ty;

/// Choix « fonctionnement d'une classe » : la classe en reprend la valeur par défaut.
pub const FONCTIONNEMENTS: &[&str] = &["Simple", "Double division", "Double vacation"];
/// Choix « pédagogie appliquée » : la classe en reprend la valeur par défaut.
pub const PEDAGOGIES: &[&str] = &["Classique", "Pédagogie convergente", "Curriculum"];

/// Rubriques livrées, dans l'ordre d'affichage de chaque thème. Les codes ne changent jamais :
/// les rapports et l'enquête les lisent.
pub const RUBRIQUES_LIVREES: &[RubriqueLivree] = &[
    // Localisation : saisie une fois, gardée d'année en année.
    r("region", Th::Localisation, "Région", Ty::Texte, &[], false),
    r("cercle", Th::Localisation, "Cercle", Ty::Texte, &[], false),
    r(
        "commune",
        Th::Localisation,
        "Commune",
        Ty::Texte,
        &[],
        false,
    ),
    r(
        "type_commune",
        Th::Localisation,
        "Type de commune",
        Ty::Choix,
        &["Urbaine", "Rurale"],
        false,
    ),
    r(
        "quartier",
        Th::Localisation,
        "Quartier ou village",
        Ty::Texte,
        &[],
        false,
    ),
    r("rue", Th::Localisation, "Rue", Ty::Texte, &[], false),
    r("porte", Th::Localisation, "Porte", Ty::Texte, &[], false),
    r(
        "coordonnees",
        Th::Localisation,
        "Coordonnées géographiques",
        Ty::Texte,
        &[],
        false,
    ),
    r(
        "boite_postale",
        Th::Localisation,
        "Boîte postale",
        Ty::Texte,
        &[],
        false,
    ),
    r(
        "courriel",
        Th::Localisation,
        "Courriel",
        Ty::Texte,
        &[],
        false,
    ),
    r(
        "distance_cap",
        Th::Localisation,
        "Distance au CAP (km)",
        Ty::Nombre,
        &[],
        false,
    ),
    r(
        "distance_academie",
        Th::Localisation,
        "Distance à l'académie (km)",
        Ty::Nombre,
        &[],
        false,
    ),
    r(
        "distance_chef_lieu",
        Th::Localisation,
        "Distance au chef-lieu de cercle (km)",
        Ty::Nombre,
        &[],
        false,
    ),
    // Identité.
    r("sigle", Th::Identite, "Sigle", Ty::Texte, &[], false),
    r(
        "code_administration",
        Th::Identite,
        "Code donné par l'administration",
        Ty::Texte,
        &[],
        false,
    ),
    r(
        "confession",
        Th::Identite,
        "Confessionnelle ou laïque",
        Ty::Choix,
        &["Laïque", "Catholique", "Protestante", "Musulmane", "Autre"],
        false,
    ),
    r(
        "type_enseignement",
        Th::Identite,
        "Type d'enseignement",
        Ty::Choix,
        &[
            "Classique",
            "Pédagogie convergente",
            "Franco-arabe",
            "Curriculum",
        ],
        false,
    ),
    r(
        "langue",
        Th::Identite,
        "Langue de la pédagogie convergente",
        Ty::Texte,
        &[],
        false,
    ),
    r(
        "mode_recrutement",
        Th::Identite,
        "Mode de recrutement",
        Ty::Choix,
        &["Annuel", "Biennal", "Autre"],
        false,
    ),
    r(
        "milieu",
        Th::Identite,
        "Milieu",
        Ty::Choix,
        &["Urbain", "Rural"],
        false,
    ),
    r(
        "type_ecole",
        Th::Identite,
        "Type de l'école",
        Ty::PlusieursChoix,
        &[
            "École mobile",
            "Double division",
            "Double vacation",
            "Classe unique",
        ],
        false,
    ),
    r(
        "fonctionnement_classes",
        Th::Identite,
        "Fonctionnement des classes, par défaut",
        Ty::Choix,
        FONCTIONNEMENTS,
        false,
    ),
    r(
        "pedagogie_classes",
        Th::Identite,
        "Pédagogie appliquée, par défaut",
        Ty::Choix,
        PEDAGOGIES,
        false,
    ),
    r("option", Th::Identite, "Option", Ty::Texte, &[], false),
    r(
        "education_specialisee",
        Th::Identite,
        "Éducation spécialisée",
        Ty::OuiNon,
        &[],
        false,
    ),
    r(
        "superficie",
        Th::Identite,
        "Superficie du domaine scolaire (m²)",
        Ty::Nombre,
        &[],
        false,
    ),
    // Actes.
    r(
        "creation_reference",
        Th::Actes,
        "Décision de création : référence",
        Ty::Texte,
        &[],
        false,
    ),
    r(
        "creation_date",
        Th::Actes,
        "Décision de création : date",
        Ty::Date,
        &[],
        false,
    ),
    r(
        "ouverture_reference",
        Th::Actes,
        "Arrêté d'ouverture : référence",
        Ty::Texte,
        &[],
        false,
    ),
    r(
        "ouverture_date",
        Th::Actes,
        "Arrêté d'ouverture : date",
        Ty::Date,
        &[],
        false,
    ),
    r(
        "demande_date",
        Th::Actes,
        "Date de la demande d'ouverture",
        Ty::Date,
        &[],
        false,
    ),
    r(
        "annee_construction",
        Th::Actes,
        "Année de construction",
        Ty::Nombre,
        &[],
        false,
    ),
    r(
        "annee_equipement",
        Th::Actes,
        "Année d'équipement",
        Ty::Nombre,
        &[],
        false,
    ),
    // Eau et hygiène : revue chaque année.
    r(
        "statut_terrain",
        Th::EauHygiene,
        "Statut du terrain",
        Ty::Choix,
        &["Titre foncier", "Permis d'occuper", "Autre", "Aucun"],
        true,
    ),
    r("cloture", Th::EauHygiene, "Clôture", Ty::OuiNon, &[], true),
    r(
        "point_eau",
        Th::EauHygiene,
        "Point d'eau potable",
        Ty::OuiNon,
        &[],
        true,
    ),
    r(
        "source_eau",
        Th::EauHygiene,
        "Source de l'eau",
        Ty::Choix,
        &["Robinet", "Forage", "Puits", "Autre"],
        true,
    ),
    r(
        "lavage_mains",
        Th::EauHygiene,
        "Dispositif de lavage des mains",
        Ty::OuiNon,
        &[],
        true,
    ),
    r(
        "savon",
        Th::EauHygiene,
        "Savon disponible",
        Ty::OuiNon,
        &[],
        true,
    ),
    r(
        "latrines",
        Th::EauHygiene,
        "Latrines fonctionnelles",
        Ty::OuiNon,
        &[],
        true,
    ),
    r(
        "latrines_separees",
        Th::EauHygiene,
        "Latrines séparées filles et garçons",
        Ty::OuiNon,
        &[],
        true,
    ),
    r(
        "ordures",
        Th::EauHygiene,
        "Évacuation des ordures",
        Ty::Choix,
        &["Ramassage", "Incinération", "Enfouissement", "Autre"],
        true,
    ),
    // Énergie.
    r(
        "electricite",
        Th::Energie,
        "Électricité",
        Ty::OuiNon,
        &[],
        true,
    ),
    r(
        "source_electricite",
        Th::Energie,
        "Source de l'électricité",
        Ty::PlusieursChoix,
        &["Réseau", "Solaire", "Groupe électrogène", "Autre"],
        true,
    ),
    r(
        "internet",
        Th::Energie,
        "Connexion à internet",
        Ty::OuiNon,
        &[],
        true,
    ),
    // Équipements collectifs.
    r(
        "bibliotheque",
        Th::Equipements,
        "Bibliothèque",
        Ty::OuiNon,
        &[],
        true,
    ),
    r(
        "laboratoire",
        Th::Equipements,
        "Laboratoire",
        Ty::OuiNon,
        &[],
        true,
    ),
    r(
        "salle_informatique",
        Th::Equipements,
        "Salle informatique",
        Ty::OuiNon,
        &[],
        true,
    ),
    r(
        "terrain_sport",
        Th::Equipements,
        "Terrain de sport",
        Ty::OuiNon,
        &[],
        true,
    ),
    r(
        "cantine",
        Th::Equipements,
        "Cantine fonctionnelle",
        Ty::OuiNon,
        &[],
        true,
    ),
    r(
        "infirmerie",
        Th::Equipements,
        "Infirmerie",
        Ty::OuiNon,
        &[],
        true,
    ),
    // Vie de l'école.
    r(
        "comite_gestion",
        Th::VieEcole,
        "Comité de gestion scolaire",
        Ty::OuiNon,
        &[],
        true,
    ),
    r(
        "association_parents",
        Th::VieEcole,
        "Association des parents d'élèves",
        Ty::OuiNon,
        &[],
        true,
    ),
    r(
        "association_meres",
        Th::VieEcole,
        "Association des mères d'élèves",
        Ty::OuiNon,
        &[],
        true,
    ),
    r(
        "gouvernement_enfants",
        Th::VieEcole,
        "Gouvernement des enfants",
        Ty::OuiNon,
        &[],
        true,
    ),
    r(
        "projet_etablissement",
        Th::VieEcole,
        "Projet d'établissement",
        Ty::OuiNon,
        &[],
        true,
    ),
    r(
        "conseils_maitres",
        Th::VieEcole,
        "Conseils des maîtres tenus",
        Ty::Nombre,
        &[],
        true,
    ),
    r(
        "formations_recues",
        Th::VieEcole,
        "Formations reçues par les maîtres",
        Ty::Texte,
        &[],
        true,
    ),
    r(
        "jours_greve",
        Th::VieEcole,
        "Jours de grève",
        Ty::Nombre,
        &[],
        true,
    ),
    // Accessibilité.
    r(
        "acces_route",
        Th::Accessibilite,
        "Route praticable jusqu'à l'école",
        Ty::OuiNon,
        &[],
        true,
    ),
    r(
        "rampes",
        Th::Accessibilite,
        "Rampes d'accès",
        Ty::OuiNon,
        &[],
        true,
    ),
    r(
        "salles_accessibles",
        Th::Accessibilite,
        "Salles accessibles aux élèves handicapés",
        Ty::Nombre,
        &[],
        true,
    ),
    r(
        "toilettes_adaptees",
        Th::Accessibilite,
        "Toilettes adaptées",
        Ty::OuiNon,
        &[],
        true,
    ),
];

fn propre(texte: &str) -> String {
    texte.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Rend le libellé nettoyé d'une rubrique, ou la clé du refus.
pub fn verifier_libelle(libelle: &str) -> Result<String, &'static str> {
    let p = propre(libelle);
    if p.is_empty() || p.chars().count() > LIBELLE_RUBRIQUE_MAX {
        return Err("fiche.libelle_invalide");
    }
    Ok(p)
}

/// Rend l'aide nettoyée d'une rubrique, ou la clé du refus.
pub fn verifier_aide(aide: &str) -> Result<String, &'static str> {
    let p = propre(aide);
    if p.chars().count() > AIDE_RUBRIQUE_MAX {
        return Err("fiche.aide_invalide");
    }
    Ok(p)
}

/// Rend les choix nettoyés, sans doublon ni vide, ou la clé du refus. Un choix ou plusieurs
/// choix en demandent au moins un ; les autres types n'en gardent aucun.
pub fn verifier_choix(
    type_reponse: TypeReponse,
    choix: &[String],
) -> Result<Vec<String>, &'static str> {
    if !type_reponse.a_des_choix() {
        return Ok(Vec::new());
    }
    let mut propres: Vec<String> = Vec::new();
    for c in choix {
        let p = propre(c);
        if p.is_empty() {
            continue;
        }
        if p.chars().count() > CHOIX_MAX {
            return Err("fiche.choix_invalide");
        }
        if !propres.contains(&p) {
            propres.push(p);
        }
    }
    if propres.is_empty() || propres.len() > CHOIX_NOMBRE_MAX {
        return Err("fiche.choix_invalide");
    }
    Ok(propres)
}

/// Rend la réponse rangée dans la base, ou rien pour une réponse vide, ou la clé du refus. Un
/// nombre s'écrit avec un point ; une date en AAAA-MM-JJ ; plusieurs choix gardent l'ordre de
/// la liste.
pub fn normaliser_reponse(
    type_reponse: TypeReponse,
    choix: &[String],
    reponse: Option<&str>,
) -> Result<Option<String>, &'static str> {
    let Some(brute) = reponse else {
        return Ok(None);
    };
    if type_reponse == TypeReponse::PlusieursChoix {
        let retenus: Vec<String> = brute
            .split(SEPARATEUR)
            .map(propre)
            .filter(|c| !c.is_empty())
            .collect();
        if retenus.iter().any(|c| !choix.contains(c)) {
            return Err("fiche.reponse_invalide");
        }
        let ordonnes: Vec<&str> = choix
            .iter()
            .filter(|c| retenus.contains(c))
            .map(String::as_str)
            .collect();
        return Ok((!ordonnes.is_empty()).then(|| ordonnes.join("\n")));
    }
    let p = if type_reponse == TypeReponse::Texte {
        brute.trim().to_owned()
    } else {
        propre(brute)
    };
    if p.is_empty() {
        return Ok(None);
    }
    match type_reponse {
        TypeReponse::OuiNon => match p.as_str() {
            "oui" | "non" => Ok(Some(p)),
            _ => Err("fiche.reponse_invalide"),
        },
        TypeReponse::Nombre => {
            let point = p.replace(',', ".").replace(' ', "");
            match point.parse::<f64>() {
                Ok(n) if n.is_finite() && n >= 0.0 => Ok(Some(point)),
                _ => Err("fiche.nombre_invalide"),
            }
        }
        TypeReponse::Date => DateCivile::analyser(&p)
            .map(|d| Some(d.to_string()))
            .ok_or("fiche.date_invalide"),
        TypeReponse::Choix => {
            if choix.contains(&p) {
                Ok(Some(p))
            } else {
                Err("fiche.reponse_invalide")
            }
        }
        TypeReponse::Texte | TypeReponse::PlusieursChoix => {
            if p.chars().count() > REPONSE_MAX {
                Err("fiche.reponse_invalide")
            } else {
                Ok(Some(p))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_codes_livres_sont_uniques_et_les_libelles_valides() {
        let mut codes = std::collections::HashSet::new();
        for l in RUBRIQUES_LIVREES {
            assert!(codes.insert(l.code), "{}", l.code);
            assert_eq!(verifier_libelle(l.libelle), Ok(l.libelle.to_owned()));
            assert_eq!(
                l.type_reponse.a_des_choix(),
                !l.choix.is_empty(),
                "{}",
                l.code
            );
            assert_ne!(l.theme, ThemeRubrique::Autres);
        }
    }

    #[test]
    fn les_noms_se_relisent() {
        for t in ThemeRubrique::TOUS {
            assert_eq!(ThemeRubrique::depuis_nom(t.nom()), t);
        }
        for t in TypeReponse::TOUS {
            assert_eq!(TypeReponse::depuis_nom(t.nom()), t);
        }
    }

    #[test]
    fn une_reponse_se_range_selon_son_type() {
        let choix = vec!["Urbaine".to_owned(), "Rurale".to_owned()];
        assert_eq!(
            normaliser_reponse(TypeReponse::Nombre, &[], Some(" 12,5 ")),
            Ok(Some("12.5".to_owned()))
        );
        assert_eq!(
            normaliser_reponse(TypeReponse::Nombre, &[], Some("douze")),
            Err("fiche.nombre_invalide")
        );
        assert_eq!(
            normaliser_reponse(TypeReponse::OuiNon, &[], Some("oui")),
            Ok(Some("oui".to_owned()))
        );
        assert_eq!(
            normaliser_reponse(TypeReponse::OuiNon, &[], Some("peut-être")),
            Err("fiche.reponse_invalide")
        );
        assert_eq!(
            normaliser_reponse(TypeReponse::Choix, &choix, Some("Rurale")),
            Ok(Some("Rurale".to_owned()))
        );
        assert_eq!(
            normaliser_reponse(TypeReponse::Choix, &choix, Some("Autre")),
            Err("fiche.reponse_invalide")
        );
        assert_eq!(
            normaliser_reponse(TypeReponse::PlusieursChoix, &choix, Some("Rurale\nUrbaine")),
            Ok(Some("Urbaine\nRurale".to_owned()))
        );
        assert_eq!(
            normaliser_reponse(TypeReponse::Texte, &[], Some("   ")),
            Ok(None)
        );
        assert_eq!(
            normaliser_reponse(TypeReponse::Date, &[], Some("2026-10-14")),
            Ok(Some("2026-10-14".to_owned()))
        );
        assert_eq!(normaliser_reponse(TypeReponse::Date, &[], None), Ok(None));
    }

    #[test]
    fn les_choix_se_nettoient() {
        let c = vec![
            " A ".to_owned(),
            "A".to_owned(),
            String::new(),
            "B".to_owned(),
        ];
        assert_eq!(
            verifier_choix(TypeReponse::Choix, &c),
            Ok(vec!["A".to_owned(), "B".to_owned()])
        );
        assert_eq!(
            verifier_choix(TypeReponse::Choix, &[]),
            Err("fiche.choix_invalide")
        );
        assert_eq!(verifier_choix(TypeReponse::Nombre, &c), Ok(Vec::new()));
    }
}
