//! Modèles et catalogue (blueprint §13.2). Un modèle est un fichier Typst et un manifeste JSON
//! qui déclare son identifiant, sa version et ses variables. Les modèles de base sont embarqués ;
//! un dossier de modèles sur mesure (niveau 3) peut les remplacer, identifiant par identifiant.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use serde::Deserialize;

use crate::erreur::ErreurDocument;

/// Variable injectée par le moteur dans tout modèle : la variante d'établissement.
pub const VARIABLE_ETABLISSEMENT: &str = "etablissement";

/// Gabarit commun importé par les modèles (`/commun/entete.typ`).
pub(crate) const ENTETE_COMMUN: &str = include_str!("../modeles/commun/entete.typ");

const MODELES_INTEGRES: &[(&str, &str)] = &[
    (
        include_str!("../modeles/essai/modele.json"),
        include_str!("../modeles/essai/modele.typ"),
    ),
    (
        include_str!("../modeles/certificat_cle_secours/modele.json"),
        include_str!("../modeles/certificat_cle_secours/modele.typ"),
    ),
    (
        include_str!("../modeles/numero_annule/modele.json"),
        include_str!("../modeles/numero_annule/modele.typ"),
    ),
    (
        include_str!("../modeles/fiche_inscription/modele.json"),
        include_str!("../modeles/fiche_inscription/modele.typ"),
    ),
    (
        include_str!("../modeles/etat_caisse/modele.json"),
        include_str!("../modeles/etat_caisse/modele.typ"),
    ),
    (
        include_str!("../modeles/recu/modele.json"),
        include_str!("../modeles/recu/modele.typ"),
    ),
    (
        include_str!("../modeles/releve_compte/modele.json"),
        include_str!("../modeles/releve_compte/modele.typ"),
    ),
    (
        include_str!("../modeles/bulletin/modele.json"),
        include_str!("../modeles/bulletin/modele.typ"),
    ),
    (
        include_str!("../modeles/certificat_transfert/modele.json"),
        include_str!("../modeles/certificat_transfert/modele.typ"),
    ),
    (
        include_str!("../modeles/releve_moyennes/modele.json"),
        include_str!("../modeles/releve_moyennes/modele.typ"),
    ),
    (
        include_str!("../modeles/engagement/modele.json"),
        include_str!("../modeles/engagement/modele.typ"),
    ),
    (
        include_str!("../modeles/etat_heures/modele.json"),
        include_str!("../modeles/etat_heures/modele.typ"),
    ),
    (
        include_str!("../modeles/bulletin_paie/modele.json"),
        include_str!("../modeles/bulletin_paie/modele.typ"),
    ),
    (
        include_str!("../modeles/etat_paie/modele.json"),
        include_str!("../modeles/etat_paie/modele.typ"),
    ),
    (
        include_str!("../modeles/etat_retenues/modele.json"),
        include_str!("../modeles/etat_retenues/modele.typ"),
    ),
    (
        include_str!("../modeles/emploi_du_temps/modele.json"),
        include_str!("../modeles/emploi_du_temps/modele.typ"),
    ),
    (
        include_str!("../modeles/liste_cantine/modele.json"),
        include_str!("../modeles/liste_cantine/modele.typ"),
    ),
];

/// Variable déclarée par un modèle.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Variable {
    /// Nom de la clé dans les données.
    pub nom: String,
    /// Le moteur refuse de composer si elle manque ou vaut `null`.
    pub requise: bool,
}

#[derive(Deserialize)]
struct Manifeste {
    id: String,
    version: u32,
    libelle: String,
    variables: Vec<Variable>,
}

/// Un modèle chargé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Modele {
    /// Identifiant stable, enregistré sur chaque document.
    pub id: String,
    /// Version, enregistrée sur chaque document (R-45).
    pub version: u32,
    /// Libellé lisible.
    pub libelle: String,
    /// Jeu de variables déclaré (§10.3).
    pub variables: Vec<Variable>,
    pub(crate) source: String,
}

impl Modele {
    /// Construit un modèle à partir de son manifeste et de sa source Typst.
    pub fn depuis_textes(manifeste: &str, source: &str) -> Result<Self, ErreurDocument> {
        let m: Manifeste =
            serde_json::from_str(manifeste).map_err(|e| ErreurDocument::ManifesteInvalide {
                modele: "?".to_owned(),
                detail: e.to_string(),
            })?;
        let invalide = |detail: &str| ErreurDocument::ManifesteInvalide {
            modele: m.id.clone(),
            detail: detail.to_owned(),
        };
        if m.id.is_empty() || !m.id.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
            return Err(invalide("identifiant attendu en minuscules et soulignés"));
        }
        if m.version == 0 {
            return Err(invalide("la version commence à 1"));
        }
        if m.variables.iter().any(|v| v.nom == VARIABLE_ETABLISSEMENT) {
            return Err(invalide("« etablissement » est réservée au moteur"));
        }
        Ok(Self {
            id: m.id,
            version: m.version,
            libelle: m.libelle,
            variables: m.variables,
            source: source.to_owned(),
        })
    }

    /// Vérifie que les données couvrent les variables requises et n'en ajoutent aucune.
    pub fn verifier(&self, donnees: &serde_json::Value) -> Result<(), ErreurDocument> {
        let objet = donnees
            .as_object()
            .ok_or_else(|| ErreurDocument::DonneesInvalides(self.id.clone()))?;
        for variable in self.variables.iter().filter(|v| v.requise) {
            if objet
                .get(&variable.nom)
                .is_none_or(serde_json::Value::is_null)
            {
                return Err(ErreurDocument::VariableManquante {
                    modele: self.id.clone(),
                    variable: variable.nom.clone(),
                });
            }
        }
        if let Some(inconnue) = objet
            .keys()
            .find(|cle| !self.variables.iter().any(|v| &v.nom == *cle))
        {
            return Err(ErreurDocument::VariableInconnue {
                modele: self.id.clone(),
                variable: inconnue.clone(),
            });
        }
        Ok(())
    }
}

/// Ensemble des modèles disponibles, indexés par identifiant.
#[derive(Debug, Clone)]
pub struct Catalogue {
    modeles: BTreeMap<String, Arc<Modele>>,
}

impl Catalogue {
    /// Modèles de base livrés avec le produit.
    pub fn integre() -> Result<Self, ErreurDocument> {
        let mut modeles = BTreeMap::new();
        for (manifeste, source) in MODELES_INTEGRES {
            let modele = Modele::depuis_textes(manifeste, source)?;
            modeles.insert(modele.id.clone(), Arc::new(modele));
        }
        Ok(Self { modeles })
    }

    /// Remplace ou ajoute les modèles sur mesure d'un dossier : un sous-dossier par modèle, avec
    /// `modele.json` et `modele.typ`. Un dossier absent n'est pas une erreur. Rend le nombre de
    /// modèles chargés.
    pub fn surcharger_depuis(&mut self, dossier: &Path) -> Result<usize, ErreurDocument> {
        if !dossier.is_dir() {
            return Ok(0);
        }
        let mut charges = 0;
        for entree in std::fs::read_dir(dossier)? {
            let chemin = entree?.path();
            let manifeste = chemin.join("modele.json");
            let source = chemin.join("modele.typ");
            if !(manifeste.is_file() && source.is_file()) {
                continue;
            }
            let modele = Modele::depuis_textes(
                &std::fs::read_to_string(manifeste)?,
                &std::fs::read_to_string(source)?,
            )?;
            self.modeles.insert(modele.id.clone(), Arc::new(modele));
            charges += 1;
        }
        Ok(charges)
    }

    /// Modèle par identifiant.
    pub fn modele(&self, id: &str) -> Result<Arc<Modele>, ErreurDocument> {
        self.modeles
            .get(id)
            .cloned()
            .ok_or_else(|| ErreurDocument::ModeleInconnu(id.to_owned()))
    }

    /// Modèles disponibles, par identifiant.
    pub fn modeles(&self) -> impl Iterator<Item = &Modele> {
        self.modeles.values().map(AsRef::as_ref)
    }
}
