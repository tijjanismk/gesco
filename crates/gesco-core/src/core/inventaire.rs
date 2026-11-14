//! Inventaire de l'école (SPEC_PHASE_9 §8.4) : un catalogue d'articles — locaux, mobilier,
//! matériel didactique, manuels, équipement adapté —, livrés par GESCO ou ajoutés par l'école, et
//! un relevé par année, par article et, au choix, par cycle. Toutes les quantités sont
//! facultatives : une quantité absente est une case vide, pas un zéro. Rien ne se supprime
//! (R-14) : un article se retire et se remet en service ; une quantité se vide.

use serde::{Deserialize, Serialize};

use crate::commun::{Horodatage, Id};
use crate::core::Etablissement;
use crate::scolarite::{Cycle, Niveau};

/// Longueur maximale d'une désignation.
pub const DESIGNATION_MAX: usize = 120;
/// Longueur maximale d'une matière de manuel.
pub const MATIERE_MAX: usize = 80;
/// Longueur maximale d'une observation.
pub const OBSERVATION_MAX: usize = 300;
/// Plus grande quantité acceptée.
pub const QUANTITE_MAX: u32 = 1_000_000;

/// Famille d'un article : l'onglet de l'écran d'inventaire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum FamilleInventaire {
    /// Salles, bureaux, latrines, points d'eau.
    Local,
    /// Tables-bancs, bureaux, armoires.
    Mobilier,
    /// Matériel didactique.
    Materiel,
    /// Manuels, créés par l'école.
    Manuel,
    /// Équipement adapté aux élèves handicapés.
    Adapte,
}

impl FamilleInventaire {
    /// Toutes les familles, dans l'ordre d'affichage.
    pub const TOUTES: [Self; 5] = [
        Self::Local,
        Self::Mobilier,
        Self::Materiel,
        Self::Manuel,
        Self::Adapte,
    ];

    /// Nom stable, rangé dans la base.
    pub const fn nom(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Mobilier => "mobilier",
            Self::Materiel => "materiel",
            Self::Manuel => "manuel",
            Self::Adapte => "adapte",
        }
    }

    /// Famille d'un nom de la base ; matériel pour un nom inconnu.
    pub fn depuis_nom(nom: &str) -> Self {
        Self::TOUTES
            .into_iter()
            .find(|f| f.nom() == nom)
            .unwrap_or(Self::Materiel)
    }
}

/// Un article du catalogue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ArticleInventaire {
    /// Identifiant technique.
    pub id: Id<ArticleInventaire>,
    /// Établissement propriétaire.
    pub etablissement_id: Id<Etablissement>,
    /// Code stable d'un article livré, que les rapports lisent ; vide pour un article de l'école.
    pub code: Option<String>,
    /// Famille.
    pub famille: FamilleInventaire,
    /// Désignation : « Salles de classe », « Tables-bancs à 2 places », titre d'un manuel.
    pub designation: String,
    /// Matière d'un manuel ; vide sinon.
    pub matiere: String,
    /// Niveau d'un manuel, s'il est donné.
    pub niveau_id: Option<Id<Niveau>>,
    /// Vrai pour un article livré : il ne se renomme pas, il se retire.
    pub livre: bool,
    /// Ordre dans sa famille.
    pub ordre: i64,
    /// Instant du retrait ; vide pour un article en service.
    pub retire_le: Option<Horodatage>,
}

/// Un article livré : son code, sa famille, sa désignation.
#[derive(Debug, Clone, Copy)]
pub struct ArticleLivre {
    /// Code stable.
    pub code: &'static str,
    /// Famille.
    pub famille: FamilleInventaire,
    /// Désignation.
    pub designation: &'static str,
}

const fn livre(
    code: &'static str,
    famille: FamilleInventaire,
    designation: &'static str,
) -> ArticleLivre {
    ArticleLivre {
        code,
        famille,
        designation,
    }
}

/// Code des latrines : la seule ligne qui porte les cabines.
pub const CODE_LATRINES: &str = "latrines";

/// Articles livrés, rédigés d'après les rubriques des rapports et de l'enquête (§3 à §6).
pub const ARTICLES_LIVRES: &[ArticleLivre] = {
    use FamilleInventaire::{Adapte, Local, Materiel, Mobilier};
    &[
        livre("salle_classe", Local, "Salles de classe utilisées"),
        livre(
            "salle_classe_inutilisee",
            Local,
            "Salles de classe non utilisées",
        ),
        livre("bureau_direction", Local, "Bureau de la direction"),
        livre(
            "salle_professeurs",
            Local,
            "Salle des maîtres ou des professeurs",
        ),
        livre("salle_langue", Local, "Salle de langue"),
        livre("salle_informatique", Local, "Salle informatique"),
        livre("bibliotheque", Local, "Bibliothèque"),
        livre("coin_lecture", Local, "Coins de lecture"),
        livre("laboratoire", Local, "Laboratoire"),
        livre("cdi", Local, "Centre de documentation et d'information"),
        livre("salle_conference", Local, "Salle de conférence"),
        livre("logement", Local, "Logements"),
        livre("infirmerie", Local, "Infirmerie"),
        livre("magasin", Local, "Magasin"),
        livre(CODE_LATRINES, Local, "Blocs de latrines"),
        livre("point_eau", Local, "Points d'eau potable"),
        livre("table_banc_1", Mobilier, "Tables-bancs à 1 place"),
        livre("table_banc_2", Mobilier, "Tables-bancs à 2 places"),
        livre("table_banc_3", Mobilier, "Tables-bancs à 3 places"),
        livre("table_banc_4", Mobilier, "Tables-bancs à 4 places"),
        livre("bureau_maitre", Mobilier, "Bureaux du maître"),
        livre("chaise", Mobilier, "Chaises"),
        livre("armoire", Mobilier, "Armoires"),
        livre("tableau", Mobilier, "Tableaux"),
        livre(
            "table_professeurs",
            Mobilier,
            "Tables de la salle des professeurs",
        ),
        livre(
            "compendium",
            Materiel,
            "Compendiums métriques et de sciences",
        ),
        livre("carte_murale", Materiel, "Cartes murales"),
        livre("globe", Materiel, "Globes terrestres"),
        livre("planche", Materiel, "Planches et affiches illustrées"),
        livre("geometrie", Materiel, "Instruments de géométrie du maître"),
        livre("ordinateur", Materiel, "Ordinateurs"),
        livre("videoprojecteur", Materiel, "Vidéoprojecteurs"),
        livre("materiel_eps", Materiel, "Matériel d'éducation physique"),
        livre("materiel_laboratoire", Materiel, "Matériel de laboratoire"),
        livre("braille", Adapte, "Manuels et matériel en braille"),
        livre("gros_caracteres", Adapte, "Manuels en gros caractères"),
        livre("appareil_auditif", Adapte, "Appareils auditifs"),
        livre("fauteuil_roulant", Adapte, "Fauteuils roulants"),
        livre("table_adaptee", Adapte, "Tables adaptées"),
    ]
};

/// Quantités d'une ligne de relevé ; chacune facultative. Les colonnes de construction, d'état
/// des murs et des toits ne servent qu'aux locaux ; les cabines, qu'aux latrines.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct QuantitesInventaire {
    /// Quantité existante.
    pub existant: Option<u32>,
    /// Dont en bon état.
    pub bon_etat: Option<u32>,
    /// Besoins.
    pub besoins: Option<u32>,
    /// Locaux en dur.
    pub dur: Option<u32>,
    /// Locaux en semi-dur.
    pub semi_dur: Option<u32>,
    /// Locaux en banco.
    pub banco: Option<u32>,
    /// Locaux d'une autre construction (paillote, tôle…).
    pub autre_construction: Option<u32>,
    /// Dont neufs de l'année.
    pub neufs: Option<u32>,
    /// Dont en location.
    pub en_location: Option<u32>,
    /// Murs en bon état.
    pub murs_bons: Option<u32>,
    /// Murs en mauvais état.
    pub murs_mauvais: Option<u32>,
    /// Toits en bon état.
    pub toits_bons: Option<u32>,
    /// Toits en mauvais état.
    pub toits_mauvais: Option<u32>,
    /// Cabines des filles.
    pub cabines_filles: Option<u32>,
    /// Cabines des garçons.
    pub cabines_garcons: Option<u32>,
    /// Cabines des enseignants.
    pub cabines_enseignants: Option<u32>,
}

impl QuantitesInventaire {
    /// Noms des quantités, dans l'ordre des colonnes ; ce sont aussi les codes des colonnes des
    /// statistiques.
    pub const NOMS: [&'static str; 16] = [
        "existant",
        "bon_etat",
        "besoins",
        "dur",
        "semi_dur",
        "banco",
        "autre_construction",
        "neufs",
        "en_location",
        "murs_bons",
        "murs_mauvais",
        "toits_bons",
        "toits_mauvais",
        "cabines_filles",
        "cabines_garcons",
        "cabines_enseignants",
    ];

    /// Les quantités, dans l'ordre de [`Self::NOMS`].
    pub const fn valeurs(&self) -> [Option<u32>; 16] {
        [
            self.existant,
            self.bon_etat,
            self.besoins,
            self.dur,
            self.semi_dur,
            self.banco,
            self.autre_construction,
            self.neufs,
            self.en_location,
            self.murs_bons,
            self.murs_mauvais,
            self.toits_bons,
            self.toits_mauvais,
            self.cabines_filles,
            self.cabines_garcons,
            self.cabines_enseignants,
        ]
    }

    /// Quantités rangées dans l'ordre de [`Self::NOMS`].
    pub const fn depuis_valeurs(v: [Option<u32>; 16]) -> Self {
        Self {
            existant: v[0],
            bon_etat: v[1],
            besoins: v[2],
            dur: v[3],
            semi_dur: v[4],
            banco: v[5],
            autre_construction: v[6],
            neufs: v[7],
            en_location: v[8],
            murs_bons: v[9],
            murs_mauvais: v[10],
            toits_bons: v[11],
            toits_mauvais: v[12],
            cabines_filles: v[13],
            cabines_garcons: v[14],
            cabines_enseignants: v[15],
        }
    }

    /// Quantité d'une colonne, par son nom.
    pub fn valeur(&self, nom: &str) -> Option<u32> {
        Self::NOMS
            .iter()
            .zip(self.valeurs())
            .find(|(n, _)| **n == nom)
            .and_then(|(_, v)| v)
    }

    /// Vrai si aucune quantité n'est donnée.
    pub fn est_vide(&self) -> bool {
        self.valeurs().iter().all(Option::is_none)
    }

    /// Somme de deux relevés, colonne par colonne : une colonne vide des deux côtés reste vide.
    pub fn additionner(&self, autre: &Self) -> Self {
        let mut somme = self.valeurs();
        for (s, y) in somme.iter_mut().zip(autre.valeurs()) {
            *s = match (*s, y) {
                (None, None) => None,
                (x, y) => Some(x.unwrap_or(0).saturating_add(y.unwrap_or(0))),
            };
        }
        Self::depuis_valeurs(somme)
    }

    /// Vérifie les quantités : aucune ne dépasse [`QUANTITE_MAX`].
    pub fn verifier(&self) -> Result<(), &'static str> {
        if self.valeurs().iter().flatten().any(|q| *q > QUANTITE_MAX) {
            return Err("inventaire.quantite_trop_grande");
        }
        Ok(())
    }
}

/// Une ligne de relevé : l'article, le cycle (vide pour toute l'école), les quantités et une
/// observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ReleveInventaire {
    /// Article.
    pub article_id: Id<ArticleInventaire>,
    /// Cycle ; vide pour un relevé de toute l'école.
    pub cycle: Option<Cycle>,
    /// Quantités.
    pub quantites: QuantitesInventaire,
    /// Observation libre.
    pub observation: String,
}

impl ReleveInventaire {
    /// Vrai si la ligne ne dit rien.
    pub fn est_vide(&self) -> bool {
        self.quantites.est_vide() && self.observation.is_empty()
    }
}

/// Désignation nettoyée, ou la clé du refus.
pub fn verifier_designation(designation: &str) -> Result<String, &'static str> {
    let d = designation.split_whitespace().collect::<Vec<_>>().join(" ");
    if d.is_empty() {
        return Err("inventaire.designation_requise");
    }
    if d.chars().count() > DESIGNATION_MAX {
        return Err("inventaire.designation_trop_longue");
    }
    Ok(d)
}

/// Matière nettoyée, ou la clé du refus.
pub fn verifier_matiere(matiere: &str) -> Result<String, &'static str> {
    let m = matiere.split_whitespace().collect::<Vec<_>>().join(" ");
    if m.chars().count() > MATIERE_MAX {
        return Err("inventaire.matiere_trop_longue");
    }
    Ok(m)
}

/// Observation nettoyée, ou la clé du refus.
pub fn verifier_observation(observation: &str) -> Result<String, &'static str> {
    let o = observation.trim().to_owned();
    if o.chars().count() > OBSERVATION_MAX {
        return Err("inventaire.observation_trop_longue");
    }
    Ok(o)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_codes_livres_sont_uniques() {
        let mut codes: Vec<_> = ARTICLES_LIVRES.iter().map(|a| a.code).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), ARTICLES_LIVRES.len());
        assert!(
            ARTICLES_LIVRES
                .iter()
                .all(|a| a.famille != FamilleInventaire::Manuel)
        );
    }

    #[test]
    fn une_somme_garde_les_cases_vides() {
        let a = QuantitesInventaire {
            existant: Some(3),
            besoins: None,
            ..QuantitesInventaire::default()
        };
        let b = QuantitesInventaire {
            existant: Some(2),
            bon_etat: Some(1),
            ..QuantitesInventaire::default()
        };
        let s = a.additionner(&b);
        assert_eq!(s.existant, Some(5));
        assert_eq!(s.bon_etat, Some(1));
        assert_eq!(s.besoins, None);
        assert!(QuantitesInventaire::default().est_vide());
        assert_eq!(s.valeur("existant"), Some(5));
        assert_eq!(s.valeur("inconnu"), None);
    }

    #[test]
    fn les_noms_suivent_les_valeurs() {
        let mut v = [None; 16];
        for (i, x) in (0_u32..).zip(v.iter_mut()) {
            *x = Some(i);
        }
        let q = QuantitesInventaire::depuis_valeurs(v);
        assert_eq!(q.valeurs(), v);
        assert_eq!(q.valeur("cabines_enseignants"), Some(15));
        assert_eq!(q.toits_mauvais, Some(12));
    }

    #[test]
    fn les_textes_sont_verifies() {
        assert_eq!(
            verifier_designation("  Lecture   CE1 "),
            Ok("Lecture CE1".into())
        );
        assert!(verifier_designation("   ").is_err());
        assert!(verifier_designation(&"x".repeat(121)).is_err());
        assert!(verifier_matiere(&"x".repeat(81)).is_err());
        assert!(verifier_observation(&"x".repeat(301)).is_err());
        let trop = QuantitesInventaire {
            existant: Some(QUANTITE_MAX + 1),
            ..QuantitesInventaire::default()
        };
        assert!(trop.verifier().is_err());
    }
}
