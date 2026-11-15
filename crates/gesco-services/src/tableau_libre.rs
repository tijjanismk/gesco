//! Tableau libre (SPEC_PHASE_9 §12 étape 8, T-107) : l'école compose un tableau en choisissant
//! ses lignes et ses colonnes parmi classe, niveau, cycle, sexe, âge, année de recrutement pour
//! les élèves ; statut, catégorie, spécialité, diplôme, sexe, cycle pour le personnel ; ou les
//! rubriques de sa fiche. Mêmes données et même date d'arrêt que les statistiques ; rien ne
//! s'écrit. Une fiche qui n'entre dans aucune ligne ou colonne compte dans le total, et l'écran
//! le dit d'une ligne neutre (règle 7).

use std::collections::BTreeMap;

use gesco_core::commun::{DateCivile, Id};
use gesco_core::core::AnneeScolaire;
use gesco_core::core::complements::{DiplomeEleve, Specialite, StatutRapport};
use gesco_core::scolarite::Cycle;
use gesco_db::depots::fiche;
use gesco_db::depots::statistiques::MembreStat;
use serde::{Deserialize, Serialize};

use crate::enquete::reponse_libre;
use crate::erreur::Resultat;
use crate::noyau::Noyau;
use crate::rapports::reponse_fiche;
use crate::rapports_tableaux::statut;
use crate::session::Session;
use crate::statistiques::{
    CelluleStatistique, DemandeStatistiques, DonneesStat, FicheStatistique, HorsColonnes,
    LigneStatistique, Retenue, ServiceStatistiques, TableauStatistique, date, retenir,
};

/// Ce que le tableau compte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum PopulationLibre {
    /// Les élèves présents à la date d'arrêt.
    #[default]
    Eleves,
    /// Le personnel présent à la date d'arrêt.
    Personnel,
    /// Les rubriques de la fiche de l'établissement et leurs réponses.
    Fiche,
}

/// Ce qui fait les lignes ou les colonnes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum CritereLibre {
    /// Classe de l'élève.
    Classe,
    /// Niveau de l'élève.
    Niveau,
    /// Cycle de l'élève ; pour le personnel, le premier cycle où il tient une classe.
    Cycle,
    /// Sexe.
    Sexe,
    /// Âge de l'élève à la rentrée.
    Age,
    /// Année de recrutement de l'élève.
    AnneeRecrutement,
    /// Statut du membre du personnel.
    Statut,
    /// Catégorie du membre du personnel.
    Categorie,
    /// Spécialité de l'enseignant.
    Specialite,
    /// Diplôme le plus élevé du membre du personnel.
    Diplome,
}

impl CritereLibre {
    const fn nom(self) -> &'static str {
        match self {
            Self::Classe => "classe",
            Self::Niveau => "niveau",
            Self::Cycle => "cycle",
            Self::Sexe => "sexe",
            Self::Age => "age",
            Self::AnneeRecrutement => "annee_recrutement",
            Self::Statut => "statut",
            Self::Categorie => "categorie",
            Self::Specialite => "specialite",
            Self::Diplome => "diplome",
        }
    }
}

/// Ce que l'écran demande.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeTableauLibre {
    /// Année ; l'année courante si rien n'est donné.
    pub annee_id: Option<Id<AnneeScolaire>>,
    /// Date d'arrêt ; aujourd'hui si rien n'est donné, ramené dans l'année.
    pub date: Option<DateCivile>,
    /// Un seul cycle ; toute l'école si rien n'est donné.
    pub cycle: Option<Cycle>,
    /// Ce que le tableau compte.
    pub population: PopulationLibre,
    /// Ce qui fait les lignes.
    pub lignes: CritereLibre,
    /// Ce qui fait les colonnes ; une seule colonne, le total, si rien n'est donné.
    pub colonnes: Option<CritereLibre>,
}

/// Le tableau composé et les fiches qu'il compte.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct TableauLibre {
    /// Fiches comptées ; les cases les désignent par leur place.
    pub fiches: Vec<FicheStatistique>,
    /// Le tableau, au format des statistiques. Une colonne dont le code commence par `cle:`
    /// porte une clé de texte ; par `libre:`, un libellé tiré des données.
    pub tableau: TableauStatistique,
}

/// Valeur d'une fiche pour un critère : son rang de tri, son libellé, sa clé de texte.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Valeur {
    rang: (i64, String),
    libelle: String,
    cle: Option<String>,
}

impl Valeur {
    fn libre(rang: i64, libelle: impl Into<String>) -> Self {
        let libelle = libelle.into();
        Self {
            rang: (rang, libelle.clone()),
            libelle,
            cle: None,
        }
    }

    fn cle(rang: usize, cle: String) -> Self {
        Self {
            rang: (i64::try_from(rang).unwrap_or(i64::MAX), String::new()),
            libelle: String::new(),
            cle: Some(cle),
        }
    }

    /// Code de colonne : la clé de texte, sinon le libellé.
    fn code(&self) -> String {
        match &self.cle {
            Some(c) => format!("cle:{c}"),
            None => format!("libre:{}", self.libelle),
        }
    }
}

/// Valeur « ensemble » d'un critère qui ne s'applique pas à la population : tout le monde.
fn ensemble() -> Valeur {
    Valeur::cle(0, "statistiques.libre.ensemble".to_owned())
}

fn sexe(s: Option<&str>, eleves: bool) -> Option<Valeur> {
    let (g, f) = if eleves {
        (
            "statistiques.colonne.garcons",
            "statistiques.colonne.filles",
        )
    } else {
        ("statistiques.colonne.hommes", "statistiques.colonne.femmes")
    };
    match s {
        Some("masculin") => Some(Valeur::cle(0, g.to_owned())),
        Some("feminin") => Some(Valeur::cle(1, f.to_owned())),
        _ => None,
    }
}

fn cycle(nom: &str) -> Option<Valeur> {
    let c = Cycle::depuis_nom(nom)?;
    let rang = Cycle::TOUS.iter().position(|x| *x == c).unwrap_or(0);
    Some(Valeur::cle(rang, format!("niveaux.cycle.{nom}")))
}

fn valeur_eleve(d: &DonneesStat, r: &Retenue<'_>, critere: CritereLibre) -> Option<Valeur> {
    match critere {
        CritereLibre::Classe => Some(Valeur::libre(r.classe.ordre, r.classe.libelle.clone())),
        CritereLibre::Niveau => Some(Valeur::libre(r.classe.ordre, r.classe.niveau.clone())),
        CritereLibre::Cycle => cycle(&r.classe.cycle),
        CritereLibre::Sexe => sexe(r.inscription.sexe.as_deref(), true),
        CritereLibre::Age => {
            let rentree = d.rentree.unwrap_or(d.date);
            date(r.inscription.date_naissance.as_deref()).map(|n| {
                let a = rentree.annee() - n.annee();
                Valeur::libre(i64::from(a), a.to_string())
            })
        }
        CritereLibre::AnneeRecrutement => d
            .complements
            .annees_recrutement
            .get(&r.inscription.eleve_id)
            .map(|a| Valeur::libre(i64::from(*a), a.to_string())),
        CritereLibre::Statut
        | CritereLibre::Categorie
        | CritereLibre::Specialite
        | CritereLibre::Diplome => Some(ensemble()),
    }
}

fn valeur_membre(d: &DonneesStat, m: &MembreStat, critere: CritereLibre) -> Option<Valeur> {
    let complement = d.complements.membres.get(&m.id);
    match critere {
        CritereLibre::Sexe => sexe(m.sexe.as_deref(), false),
        CritereLibre::Statut => {
            let s = statut(d, m);
            let rang = StatutRapport::TOUS
                .iter()
                .position(|x| *x == s)
                .unwrap_or(0);
            Some(Valeur::cle(rang, format!("complements.statut.{}", s.nom())))
        }
        CritereLibre::Categorie => Some(Valeur::libre(m.categorie_ordre, m.categorie.clone())),
        CritereLibre::Specialite => complement
            .and_then(|c| c.specialite)
            .or_else(|| {
                Specialite::proposee(
                    d.affectations
                        .iter()
                        .filter(|a| a.personne_id == m.personne_id)
                        .map(|a| a.matiere.as_str()),
                )
            })
            .map(|s| {
                let rang = Specialite::TOUS.iter().position(|x| *x == s).unwrap_or(0);
                Valeur::cle(rang, format!("complements.specialite.{}", s.nom()))
            }),
        CritereLibre::Diplome => complement
            .and_then(|c| c.diplome)
            .or_else(|| DiplomeEleve::propose(&m.diplomes))
            .map(|x| {
                let rang = DiplomeEleve::TOUS.iter().position(|y| *y == x).unwrap_or(0);
                Valeur::cle(rang, format!("complements.diplome.{}", x.nom()))
            }),
        CritereLibre::Cycle => d
            .affectations
            .iter()
            .filter(|a| a.personne_id == m.personne_id)
            .filter_map(|a| d.classes.iter().find(|c| c.id == a.classe_id))
            .filter_map(|c| cycle(&c.cycle))
            .min(),
        CritereLibre::Classe
        | CritereLibre::Niveau
        | CritereLibre::Age
        | CritereLibre::AnneeRecrutement => Some(ensemble()),
    }
}

fn nombre(n: usize) -> Option<u32> {
    Some(u32::try_from(n).unwrap_or(u32::MAX))
}

fn place(i: usize) -> u32 {
    u32::try_from(i).unwrap_or(u32::MAX)
}

/// Compose le tableau à partir de la valeur de chaque fiche pour les lignes et les colonnes.
fn composer(
    code: String,
    valeurs: &[(Option<Valeur>, Option<Valeur>)],
    avec_colonnes: bool,
    cle_hors: &str,
) -> TableauStatistique {
    let mut lignes: BTreeMap<Valeur, Vec<u32>> = BTreeMap::new();
    let mut colonnes: BTreeMap<Valeur, Vec<u32>> = BTreeMap::new();
    let mut hors: Vec<u32> = Vec::new();
    for (i, (l, c)) in valeurs.iter().enumerate() {
        let p = place(i);
        if let Some(l) = l {
            lignes.entry(l.clone()).or_default().push(p);
        }
        if avec_colonnes && let Some(c) = c {
            colonnes.entry(c.clone()).or_default().push(p);
        }
        if l.is_none() || (avec_colonnes && c.is_none()) {
            hors.push(p);
        }
    }
    let cellules = |membres: &[u32]| -> Vec<CelluleStatistique> {
        let mut v: Vec<CelluleStatistique> = colonnes
            .values()
            .map(|dans| {
                let m: Vec<u32> = membres
                    .iter()
                    .copied()
                    .filter(|x| dans.contains(x))
                    .collect();
                CelluleStatistique {
                    nombre: nombre(m.len()),
                    texte: None,
                    membres: m,
                }
            })
            .collect();
        v.push(CelluleStatistique {
            nombre: nombre(membres.len()),
            texte: None,
            membres: membres.to_vec(),
        });
        v
    };
    let tous: Vec<u32> = (0..valeurs.len()).map(place).collect();
    let mut codes: Vec<String> = colonnes.keys().map(Valeur::code).collect();
    codes.push("total".to_owned());
    TableauStatistique {
        code,
        colonnes: codes,
        lignes: lignes
            .iter()
            .map(|(v, membres)| LigneStatistique {
                libelle: v.libelle.clone(),
                cle: v.cle.clone(),
                cellules: cellules(membres),
            })
            .collect(),
        total: (!valeurs.is_empty()).then(|| LigneStatistique {
            libelle: String::new(),
            cle: Some("statistiques.colonne.total".to_owned()),
            cellules: cellules(&tous),
        }),
        hors: if hors.is_empty() {
            Vec::new()
        } else {
            vec![HorsColonnes {
                cle: cle_hors.to_owned(),
                membres: hors,
            }]
        },
    }
}

/// Le tableau des élèves.
fn eleves(d: &DonneesStat, demande: &DemandeTableauLibre) -> TableauLibre {
    let perimetre = d.perimetre(demande.cycle);
    let retenues: Vec<Retenue<'_>> = retenir(&perimetre, &d.inscriptions)
        .into_iter()
        .filter(|r| r.presente)
        .collect();
    let fiches = retenues
        .iter()
        .map(|r| FicheStatistique {
            id: r.inscription.eleve_id.clone(),
            nom: format!("{} {}", r.inscription.prenoms, r.inscription.nom)
                .trim()
                .to_owned(),
            detail: r.classe.libelle.clone(),
            motif: String::new(),
        })
        .collect();
    let valeurs: Vec<_> = retenues
        .iter()
        .map(|r| {
            (
                valeur_eleve(d, r, demande.lignes),
                demande.colonnes.and_then(|c| valeur_eleve(d, r, c)),
            )
        })
        .collect();
    TableauLibre {
        fiches,
        tableau: composer(
            format!("libre_{}", demande.lignes.nom()),
            &valeurs,
            demande.colonnes.is_some(),
            "statistiques.hors.libre_eleves",
        ),
    }
}

/// Le tableau du personnel : présent à la date, et qui tient une classe du cycle retenu.
fn personnel(d: &DonneesStat, demande: &DemandeTableauLibre) -> TableauLibre {
    let retenus: Vec<&MembreStat> = d
        .membres
        .iter()
        .filter(|m| date(m.arrive_le.as_deref()).is_none_or(|a| a <= d.date))
        .filter(|m| date(m.parti_le.as_deref()).is_none_or(|x| x > d.date))
        .filter(|m| {
            demande.cycle.is_none_or(|c| {
                d.affectations
                    .iter()
                    .filter(|a| a.personne_id == m.personne_id)
                    .filter_map(|a| d.classes.iter().find(|x| x.id == a.classe_id))
                    .any(|x| x.cycle == c.nom())
            })
        })
        .collect();
    let fiches = retenus
        .iter()
        .map(|m| FicheStatistique {
            id: m.id.clone(),
            nom: format!("{} {}", m.prenoms, m.nom).trim().to_owned(),
            detail: m.categorie.clone(),
            motif: String::new(),
        })
        .collect();
    let valeurs: Vec<_> = retenus
        .iter()
        .map(|m| {
            (
                valeur_membre(d, m, demande.lignes),
                demande.colonnes.and_then(|c| valeur_membre(d, m, c)),
            )
        })
        .collect();
    TableauLibre {
        fiches,
        tableau: composer(
            format!("libre_{}", demande.lignes.nom()),
            &valeurs,
            demande.colonnes.is_some(),
            "statistiques.hors.libre_personnel",
        ),
    }
}

/// Le tableau de la fiche : une ligne par rubrique active, dans l'ordre des thèmes, avec sa
/// réponse de l'année (ou la permanente).
fn fiche_ecole(
    d: &DonneesStat,
    rubriques: &[gesco_core::core::RubriqueFiche],
    valeurs: &[fiche::ValeurFiche],
) -> TableauLibre {
    let annee = d
        .choisie
        .as_ref()
        .map(|a| a.id.vers_chaine())
        .unwrap_or_default();
    let themes = gesco_core::core::ThemeRubrique::TOUS;
    let mut actives: Vec<_> = rubriques
        .iter()
        .filter(|r| r.retiree_le.is_none())
        .collect();
    actives.sort_by_key(|r| (themes.iter().position(|t| *t == r.theme), r.ordre));
    let lignes = actives
        .into_iter()
        .map(|r| {
            let reponse = match &r.code {
                Some(code) => reponse_fiche(rubriques, valeurs, &annee, code),
                None => reponse_libre(r, valeurs, &annee),
            };
            LigneStatistique {
                libelle: r.libelle.clone(),
                cle: None,
                cellules: vec![CelluleStatistique {
                    nombre: None,
                    texte: (!reponse.is_empty()).then_some(reponse),
                    membres: Vec::new(),
                }],
            }
        })
        .collect();
    TableauLibre {
        fiches: Vec::new(),
        tableau: TableauStatistique {
            code: "libre_rubrique".to_owned(),
            colonnes: vec!["reponse".to_owned()],
            lignes,
            total: None,
            hors: Vec::new(),
        },
    }
}

/// Service du tableau libre.
pub struct ServiceTableauLibre<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceTableauLibre<'n> {
    /// Service sur le noyau ouvert.
    pub const fn nouveau(noyau: &'n Noyau) -> Self {
        Self { noyau }
    }

    /// Compose le tableau demandé. Rien ne s'écrit ; une école vide a un tableau vide.
    pub async fn composer(
        &self,
        session: &Session,
        demande: &DemandeTableauLibre,
    ) -> Resultat<TableauLibre> {
        let d = ServiceStatistiques::nouveau(self.noyau)
            .charger(
                session,
                &DemandeStatistiques {
                    annee_id: demande.annee_id,
                    date: demande.date,
                    cycle: demande.cycle,
                },
            )
            .await?;
        Ok(match demande.population {
            PopulationLibre::Eleves => eleves(&d, demande),
            PopulationLibre::Personnel => personnel(&d, demande),
            PopulationLibre::Fiche => {
                let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
                let mut cnx = self
                    .noyau
                    .base()
                    .pool()
                    .acquire()
                    .await
                    .map_err(gesco_db::ErreurBase::from)?;
                let rubriques = fiche::rubriques(&mut cnx, &contexte).await?;
                let valeurs = fiche::valeurs(&mut cnx, &contexte).await?;
                drop(cnx);
                fiche_ecole(&d, &rubriques, &valeurs)
            }
        })
    }
}

#[cfg(test)]
mod essais {
    use super::*;

    fn v(l: &str) -> Option<Valeur> {
        Some(Valeur::libre(0, l))
    }

    #[test]
    fn le_total_compte_aussi_les_fiches_hors_des_colonnes() {
        let valeurs = vec![
            (
                v("10e A"),
                Some(Valeur::cle(0, "statistiques.colonne.garcons".to_owned())),
            ),
            (v("10e A"), None),
            (
                v("11e S"),
                Some(Valeur::cle(1, "statistiques.colonne.filles".to_owned())),
            ),
            (
                None,
                Some(Valeur::cle(1, "statistiques.colonne.filles".to_owned())),
            ),
        ];
        let t = composer(
            "libre_classe".to_owned(),
            &valeurs,
            true,
            "statistiques.hors.libre_eleves",
        );
        assert_eq!(
            t.colonnes,
            vec![
                "cle:statistiques.colonne.garcons".to_owned(),
                "cle:statistiques.colonne.filles".to_owned(),
                "total".to_owned()
            ]
        );
        assert_eq!(t.lignes.len(), 2);
        let premiere: Vec<Option<u32>> = t
            .lignes
            .first()
            .map(|l| l.cellules.iter().map(|c| c.nombre).collect())
            .unwrap_or_default();
        assert_eq!(premiere, vec![Some(1), Some(0), Some(2)]);
        let total: Vec<Option<u32>> = t
            .total
            .as_ref()
            .map(|l| l.cellules.iter().map(|c| c.nombre).collect())
            .unwrap_or_default();
        assert_eq!(total, vec![Some(1), Some(2), Some(4)]);
        assert_eq!(t.hors.first().map(|h| h.membres.clone()), Some(vec![1, 3]));
    }

    #[test]
    fn sans_colonnes_le_tableau_n_a_que_le_total() {
        let valeurs = vec![(v("10e A"), None), (v("10e A"), None)];
        let t = composer(
            "libre_classe".to_owned(),
            &valeurs,
            false,
            "statistiques.hors.libre_eleves",
        );
        assert_eq!(t.colonnes, vec!["total".to_owned()]);
        assert!(t.hors.is_empty());
        assert!(
            composer("libre_age".to_owned(), &[], false, "x.y")
                .total
                .is_none()
        );
    }
}
