//! Statistiques de l'école (SPEC_PHASE_9 §9.1, §10) : les chiffres des élèves et du personnel,
//! arrêtés à une date, pour toute l'école ou un cycle. Rien ne s'écrit, rien n'est refusé parce
//! qu'une donnée manque : une fiche qui n'entre dans aucune colonne compte dans le total, et
//! l'écran le dit d'une ligne neutre (règle 7).
//!
//! Un élève compte à une date si son inscription est validée (ou achevée), datée de ce jour ou
//! d'avant, et qu'il n'est pas parti ce jour-là ou avant (règle 6). Un membre du personnel compte
//! sur sa fiche, arrivé et pas encore parti (règle 4) ; ni son dossier ni la paie n'entrent en
//! compte.

use std::collections::{BTreeMap, HashMap, HashSet};

use gesco_core::commun::{DateCivile, Horodatage, Id};
use gesco_core::core::complements::{
    DegreDifficulte, DifficulteEleve, DiplomeEleve, Specialite, StatutRapport, TypeDifficulte,
};
use gesco_core::core::{AnneeScolaire, Permission};
use gesco_core::scolarite::Cycle;
use gesco_db::depots::statistiques::{
    AffectationStat, ClasseStat, EngagementStat, InscriptionStat, MembreStat,
};
use gesco_db::depots::{annee_scolaire, complements, statistiques};
use serde::{Deserialize, Serialize};

use crate::erreur::Resultat;
use crate::export::{Feuille, FichierEcrit, exporter_xlsx_feuilles};
use crate::inscriptions::jour;
use crate::noyau::Noyau;
use crate::session::Session;

/// Ce que l'écran demande : l'année, la date d'arrêt, le périmètre.
#[derive(Debug, Clone, Default, Deserialize, specta::Type)]
pub struct DemandeStatistiques {
    /// Année ; l'année courante si rien n'est donné.
    pub annee_id: Option<Id<AnneeScolaire>>,
    /// Date d'arrêt des chiffres ; aujourd'hui si rien n'est donné, ramené dans l'année.
    pub date: Option<DateCivile>,
    /// Un seul cycle ; toute l'école si rien n'est donné.
    pub cycle: Option<Cycle>,
}

/// Une année qu'on peut choisir.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct AnneeStatistiques {
    /// Identifiant.
    pub id: Id<AnneeScolaire>,
    /// Libellé.
    pub libelle: String,
}

/// Une fiche comptée : un élève ou un membre du personnel. Les cellules la désignent par sa place
/// dans la liste de l'onglet.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct FicheStatistique {
    /// Identifiant de l'élève ou du membre du personnel.
    pub id: String,
    /// Nom et prénoms.
    pub nom: String,
    /// Classe de l'élève à la date, ou catégorie du membre du personnel.
    pub detail: String,
    /// Motif du départ d'un élève parti ; vide sinon.
    pub motif: String,
}

/// Une case d'un tableau.
#[derive(Debug, Clone, Default, Serialize, specta::Type)]
pub struct CelluleStatistique {
    /// Nombre ; en minutes pour une colonne d'heures. Rien pour une case de texte.
    pub nombre: Option<u32>,
    /// Texte d'une liste nominative, brut : l'écran le met en forme selon sa colonne.
    pub texte: Option<String>,
    /// Places, dans les fiches de l'onglet, de ce que le nombre compte.
    pub membres: Vec<u32>,
}

/// Une ligne d'un tableau.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct LigneStatistique {
    /// Libellé tiré des données : classe, niveau, année, matière…
    pub libelle: String,
    /// Clé du texte affiché, quand le libellé est un nom fixe (cycle, type, nature…).
    pub cle: Option<String>,
    /// Cases, dans l'ordre des colonnes.
    pub cellules: Vec<CelluleStatistique>,
}

/// Fiches comptées dans le total sans entrer dans aucune colonne ou ligne : l'écran le dit d'une
/// ligne neutre ; rien de tel n'est jamais exporté ni imprimé.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct HorsColonnes {
    /// Clé du texte : « {n} élèves sans date de naissance ».
    pub cle: String,
    /// Places des fiches dans la liste de l'onglet.
    pub membres: Vec<u32>,
}

/// Un tableau d'un onglet.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct TableauStatistique {
    /// Code stable du tableau, d'où l'écran tire son titre.
    pub code: String,
    /// Codes des colonnes, d'où l'écran tire leurs titres.
    pub colonnes: Vec<String>,
    /// Lignes.
    pub lignes: Vec<LigneStatistique>,
    /// Ligne du total ; elle compte toutes les fiches, même celles hors des colonnes.
    pub total: Option<LigneStatistique>,
    /// Fiches comptées sans entrer dans une colonne ou une ligne.
    pub hors: Vec<HorsColonnes>,
}

/// Un onglet : ses fiches et ses tableaux.
#[derive(Debug, Clone, Default, Serialize, specta::Type)]
pub struct OngletStatistique {
    /// Fiches comptées dans l'onglet.
    pub fiches: Vec<FicheStatistique>,
    /// Tableaux, dans l'ordre d'affichage.
    pub tableaux: Vec<TableauStatistique>,
}

/// Statistiques de l'école à une date.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct Statistiques {
    /// Années qu'on peut choisir, de la plus récente à la plus ancienne.
    pub annees: Vec<AnneeStatistiques>,
    /// Année des chiffres, s'il y en a une.
    pub annee_id: Option<Id<AnneeScolaire>>,
    /// Premier jour de l'année.
    pub rentree: Option<DateCivile>,
    /// Dernier jour de l'année.
    pub fin: Option<DateCivile>,
    /// Date d'arrêt des chiffres.
    pub date: DateCivile,
    /// Cycle du périmètre, si un seul est retenu.
    pub cycle: Option<Cycle>,
    /// Cycles qui ont une classe cette année, dans l'ordre de l'école.
    pub cycles: Vec<Cycle>,
    /// Onglet Élèves.
    pub eleves: OngletStatistique,
    /// Onglet Personnel.
    pub personnel: OngletStatistique,
}

/// Une feuille d'un export des statistiques, telle que l'écran la montre.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct FeuilleStatistique {
    /// Titre du tableau, nom de la feuille.
    pub titre: String,
    /// En-têtes des colonnes.
    pub entetes: Vec<String>,
    /// Lignes, chacune aussi large que les en-têtes.
    pub lignes: Vec<Vec<String>>,
}

/// Export d'un tableau ou d'un onglet entier : une feuille par tableau.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeExportStatistiques {
    /// Chemin du classeur à écrire, extension `.xlsx`.
    pub chemin: String,
    /// Titre du classeur.
    pub titre: String,
    /// Feuilles, dans l'ordre de l'écran.
    pub feuilles: Vec<FeuilleStatistique>,
}

/// Colonnes des tableaux d'élèves.
const COLONNES_ELEVES: [&str; 3] = ["garcons", "filles", "total"];
/// Colonnes des tableaux du personnel.
const COLONNES_PERSONNEL: [&str; 3] = ["hommes", "femmes", "total"];
/// Colonnes de la liste nominative du personnel.
const COLONNES_NOMINATIVES: [&str; 7] = [
    "matricule",
    "situation_famille",
    "categorie",
    "fonction",
    "fonctionnaire",
    "diplome",
    "arrive_le",
];

/// Service des statistiques.
pub struct ServiceStatistiques<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceStatistiques<'n> {
    /// Service sur le noyau ouvert.
    pub const fn nouveau(noyau: &'n Noyau) -> Self {
        Self { noyau }
    }

    /// Chiffres de l'école à la date demandée. Une école sans année, sans élève ou sans
    /// personnel a ses tableaux vides, jamais un refus.
    pub async fn lire(
        &self,
        session: &Session,
        demande: &DemandeStatistiques,
    ) -> Resultat<Statistiques> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let aujourd_hui = jour(contexte.maintenant)?;
        let mut cnx = self
            .noyau
            .base()
            .pool()
            .acquire()
            .await
            .map_err(gesco_db::ErreurBase::from)?;
        let mut annees = annee_scolaire::lister(&mut cnx, &contexte).await?;
        annees.sort_by_key(|a| std::cmp::Reverse(a.debut.millis_epoque()));
        let choisie = match demande.annee_id {
            Some(id) => annees.iter().find(|a| a.id == id),
            None => annees
                .iter()
                .find(|a| a.est_courante)
                .or_else(|| annees.first()),
        }
        .cloned();
        let rentree = choisie.as_ref().and_then(|a| DateCivile::du_jour(a.debut));
        let fin = choisie.as_ref().and_then(|a| DateCivile::du_jour(a.fin));
        let date = demande.date.unwrap_or_else(|| {
            let mut d = aujourd_hui;
            if let Some(r) = rentree
                && d < r
            {
                d = r;
            }
            if let Some(f) = fin
                && d > f
            {
                d = f;
            }
            d
        });
        let (classes, inscriptions, affectations, seances) = match &choisie {
            Some(a) => (
                statistiques::classes(&mut cnx, a.id, &contexte).await?,
                statistiques::inscriptions(&mut cnx, a.id, &contexte).await?,
                statistiques::affectations(&mut cnx, a.id, &contexte).await?,
                statistiques::seances(&mut cnx, a.id, &contexte).await?,
            ),
            None => (Vec::new(), Vec::new(), Vec::new(), Vec::new()),
        };
        let membres = statistiques::membres(&mut cnx, &contexte).await?;
        let engagements = statistiques::engagements(&mut cnx, &contexte).await?;
        let complements = self
            .complements(&mut cnx, session, choisie.as_ref(), &contexte)
            .await?;
        drop(cnx);
        let mut cycles: Vec<Cycle> = Vec::new();
        for c in &classes {
            if let Some(cycle) = Cycle::depuis_nom(&c.cycle)
                && !cycles.contains(&cycle)
            {
                cycles.push(cycle);
            }
        }
        let perimetre = Perimetre {
            date,
            rentree: rentree.unwrap_or(date),
            cycle: demande.cycle,
            classes: classes.iter().map(|c| (c.id.as_str(), c)).collect(),
        };
        let eleves = onglet_eleves(&perimetre, &classes, &inscriptions, &complements);
        let personnel = onglet_personnel(
            &perimetre,
            &membres,
            &engagements,
            &affectations,
            &seances,
            &complements,
        );
        Ok(Statistiques {
            annees: annees
                .iter()
                .map(|a| AnneeStatistiques {
                    id: a.id,
                    libelle: a.libelle.clone(),
                })
                .collect(),
            annee_id: choisie.map(|a| a.id),
            rentree,
            fin,
            date,
            cycle: demande.cycle,
            cycles,
            eleves,
            personnel,
        })
    }

    /// Compléments des fiches pour les tableaux de l'étape 3 : années de recrutement (saisies,
    /// sinon déduites), prises en charge par l'État de l'année, difficultés pour qui en a le
    /// droit, statut, spécialité et diplôme saisis du personnel.
    async fn complements(
        &self,
        cnx: &mut gesco_db::Connexion,
        session: &Session,
        annee: Option<&AnneeScolaire>,
        contexte: &gesco_db::contexte::Contexte,
    ) -> Resultat<ComplementsStat> {
        let premieres = complements::premieres_inscriptions(cnx, contexte).await?;
        let saisies = complements::annees_recrutement(cnx, contexte).await?;
        let mut annees_recrutement: HashMap<String, i32> = HashMap::new();
        for (eleve, p) in &premieres {
            if let Some(a) = crate::complements::annee_proposee(Some(p)) {
                annees_recrutement.insert(eleve.clone(), a);
            }
        }
        for (eleve, a) in saisies {
            if let Ok(a) = i32::try_from(a) {
                annees_recrutement.insert(eleve, a);
            }
        }
        let pris_en_charge = match annee {
            Some(a) => complements::prises_en_charge(cnx, &a.id.vers_chaine(), contexte)
                .await?
                .into_iter()
                .collect(),
            None => HashSet::new(),
        };
        let droits = self
            .noyau
            .permissions_de(cnx, session.utilisateur_id, contexte)
            .await?;
        let difficultes = if droits.contains(&Permission::STUDENTS_DISABILITY) {
            Some(complements::difficultes(cnx, contexte).await?)
        } else {
            None
        };
        let membres = complements::membres(cnx, contexte)
            .await?
            .into_iter()
            .map(|m| {
                (
                    m.id,
                    MembreComplete {
                        statut: m
                            .statut_rapport
                            .as_deref()
                            .and_then(StatutRapport::depuis_nom),
                        specialite: m.specialite.as_deref().and_then(Specialite::depuis_nom),
                        diplome: m
                            .diplome_eleve
                            .as_deref()
                            .and_then(DiplomeEleve::depuis_nom),
                    },
                )
            })
            .collect();
        Ok(ComplementsStat {
            annees_recrutement,
            pris_en_charge,
            difficultes,
            membres,
        })
    }

    /// Écrit un classeur d'une feuille par tableau, tel que l'écran les montre.
    pub async fn exporter(&self, demande: DemandeExportStatistiques) -> Resultat<FichierEcrit> {
        let feuilles = demande
            .feuilles
            .into_iter()
            .map(|f| Feuille {
                titre: f.titre,
                entetes: f.entetes,
                lignes: f.lignes,
            })
            .collect();
        exporter_xlsx_feuilles(demande.chemin, demande.titre, feuilles).await
    }
}

/// Ce que les fiches disent en plus pour les rapports (étape 3).
struct ComplementsStat {
    /// Année de recrutement par élève : saisie, sinon déduite.
    annees_recrutement: HashMap<String, i32>,
    /// Élèves pris en charge par l'État cette année.
    pris_en_charge: HashSet<String>,
    /// Difficultés des élèves, pour qui a le droit de les lire.
    difficultes: Option<Vec<DifficulteEleve>>,
    /// Compléments saisis du personnel, par membre.
    membres: HashMap<String, MembreComplete>,
}

/// Statut, spécialité et diplôme saisis d'un membre du personnel.
struct MembreComplete {
    statut: Option<StatutRapport>,
    specialite: Option<Specialite>,
    diplome: Option<DiplomeEleve>,
}

/// Ce qui borne les chiffres : la date, la rentrée, le cycle, les classes de l'année.
struct Perimetre<'a> {
    date: DateCivile,
    rentree: DateCivile,
    cycle: Option<Cycle>,
    classes: HashMap<&'a str, &'a ClasseStat>,
}

impl Perimetre<'_> {
    /// Vrai si la classe est dans le périmètre.
    fn retient(&self, classe: &ClasseStat) -> bool {
        self.cycle.is_none_or(|c| c.nom() == classe.cycle)
    }
}

fn date(texte: Option<&str>) -> Option<DateCivile> {
    texte.and_then(DateCivile::analyser)
}

fn place(i: usize) -> u32 {
    u32::try_from(i).unwrap_or(u32::MAX)
}

fn nombre(n: usize) -> Option<u32> {
    Some(u32::try_from(n).unwrap_or(u32::MAX))
}

/// Une ligne en cours de composition : son ordre, son libellé, les fiches qu'elle compte.
struct Groupe {
    libelle: String,
    cle: Option<String>,
    membres: Vec<u32>,
}

impl Groupe {
    fn nouveau(libelle: impl Into<String>, cle: Option<String>) -> Self {
        Self {
            libelle: libelle.into(),
            cle,
            membres: Vec::new(),
        }
    }
}

/// Ligne par sexe : masculin, féminin, puis le total, qui compte aussi les fiches sans sexe.
fn ligne_par_sexe(
    libelle: String,
    cle: Option<String>,
    membres: &[u32],
    sexes: &[Option<String>],
) -> LigneStatistique {
    let du_sexe = |s: &str| -> Vec<u32> {
        membres
            .iter()
            .copied()
            .filter(|m| {
                usize::try_from(*m)
                    .ok()
                    .and_then(|i| sexes.get(i))
                    .and_then(|x| x.as_deref())
                    == Some(s)
            })
            .collect()
    };
    let cellule = |m: Vec<u32>| CelluleStatistique {
        nombre: nombre(m.len()),
        texte: None,
        membres: m,
    };
    LigneStatistique {
        libelle,
        cle,
        cellules: vec![
            cellule(du_sexe("masculin")),
            cellule(du_sexe("feminin")),
            cellule(membres.to_vec()),
        ],
    }
}

/// Fiches sans sexe parmi celles comptées.
fn sans_sexe(membres: &[u32], sexes: &[Option<String>], cle: &str) -> Option<HorsColonnes> {
    let hors: Vec<u32> = membres
        .iter()
        .copied()
        .filter(|m| {
            usize::try_from(*m)
                .ok()
                .and_then(|i| sexes.get(i))
                .is_none_or(Option::is_none)
        })
        .collect();
    (!hors.is_empty()).then(|| HorsColonnes {
        cle: cle.to_owned(),
        membres: hors,
    })
}

/// Tableau par sexe : une ligne par groupe, la ligne du total sur `tous` (l'union des groupes si
/// rien n'est donné), et les fiches sans sexe.
fn tableau_par_sexe(
    code: &str,
    colonnes: [&str; 3],
    groupes: Vec<Groupe>,
    tous: Option<Vec<u32>>,
    sexes: &[Option<String>],
    cle_sans_sexe: &str,
) -> TableauStatistique {
    let tous = tous.unwrap_or_else(|| {
        let mut vus: Vec<u32> = groupes
            .iter()
            .flat_map(|g| g.membres.iter().copied())
            .collect();
        vus.sort_unstable();
        vus.dedup();
        vus
    });
    let lignes: Vec<LigneStatistique> = groupes
        .into_iter()
        .map(|g| ligne_par_sexe(g.libelle, g.cle, &g.membres, sexes))
        .collect();
    let hors = sans_sexe(&tous, sexes, cle_sans_sexe).into_iter().collect();
    TableauStatistique {
        code: code.to_owned(),
        colonnes: colonnes.iter().map(|c| (*c).to_owned()).collect(),
        total: Some(ligne_par_sexe(
            String::new(),
            Some("statistiques.total".to_owned()),
            &tous,
            sexes,
        )),
        lignes,
        hors,
    }
}

/// Une inscription retenue : présente à la date, entrée ou partie depuis la rentrée.
struct Retenue<'a> {
    inscription: &'a InscriptionStat,
    classe: &'a ClasseStat,
    presente: bool,
    entree: bool,
    depart: Option<DateCivile>,
}

/// Jour du départ d'une inscription close : celui qui est noté, sinon le jour où elle a été
/// close (un abandon ne porte pas de date).
fn jour_du_depart(i: &InscriptionStat) -> Option<DateCivile> {
    match i.statut.as_str() {
        "transferee" | "abandonnee" | "radiee" => date(i.date_depart.as_deref())
            .or_else(|| DateCivile::du_jour(Horodatage::depuis_millis_epoque(i.mis_a_jour_le))),
        _ => None,
    }
}

/// Classe de l'inscription un jour donné : avant un changement de classe en cours d'année,
/// l'élève était dans la précédente.
fn classe_le(i: &InscriptionStat, jour: DateCivile) -> &str {
    match (date(i.classe_depuis.as_deref()), &i.classe_precedente_id) {
        (Some(depuis), Some(precedente)) if jour < depuis => precedente,
        _ => &i.classe_id,
    }
}

fn retenir<'a>(p: &Perimetre<'a>, inscriptions: &'a [InscriptionStat]) -> Vec<Retenue<'a>> {
    let mut presents: HashSet<&str> = HashSet::new();
    let mut retenues = Vec::new();
    for i in inscriptions {
        if i.statut == "brouillon" {
            continue;
        }
        let Some(inscrite) = date(Some(&i.date_inscription)) else {
            continue;
        };
        let depart = jour_du_depart(i);
        let mut presente = inscrite <= p.date && depart.is_none_or(|d| d > p.date);
        // Un élève ne compte qu'une fois, même si deux inscriptions le disent présent.
        if presente && !presents.insert(i.eleve_id.as_str()) {
            presente = false;
        }
        let entree = p.rentree < inscrite && inscrite <= p.date;
        let partie = depart.filter(|d| p.rentree <= *d && *d <= p.date);
        if !(presente || entree || partie.is_some()) {
            continue;
        }
        // Un élève parti est dans la classe qu'il a quittée.
        let jour = match partie {
            Some(d) if !presente => d.decaler(-1).unwrap_or(d),
            _ => p.date,
        };
        let Some(classe) = p.classes.get(classe_le(i, jour)).copied() else {
            continue;
        };
        if !p.retient(classe) {
            continue;
        }
        retenues.push(Retenue {
            inscription: i,
            classe,
            presente,
            entree,
            depart: partie,
        });
    }
    retenues
}

/// Ajoute une fiche à la ligne de la clé donnée, créée à la fin si elle n'existe pas.
fn ajouter<'a>(
    groupes: &mut Vec<(&'a str, Groupe)>,
    cle: &'a str,
    nouveau: impl FnOnce() -> Groupe,
    membre: u32,
) {
    match groupes.iter_mut().find(|(id, _)| *id == cle) {
        Some((_, g)) => g.membres.push(membre),
        None => {
            let mut g = nouveau();
            g.membres.push(membre);
            groupes.push((cle, g));
        }
    }
}

/// Clé de traduction d'un cycle.
fn cle_cycle(cycle: &str) -> String {
    format!("niveaux.cycle.{cycle}")
}

fn onglet_eleves(
    p: &Perimetre<'_>,
    classes: &[ClasseStat],
    inscriptions: &[InscriptionStat],
    c: &ComplementsStat,
) -> OngletStatistique {
    let retenues = retenir(p, inscriptions);
    let fiches: Vec<FicheStatistique> = retenues
        .iter()
        .map(|r| FicheStatistique {
            id: r.inscription.eleve_id.clone(),
            nom: format!("{} {}", r.inscription.nom, r.inscription.prenoms)
                .trim()
                .to_owned(),
            detail: r.classe.libelle.clone(),
            motif: if r.depart.is_some() {
                r.inscription.motif.clone()
            } else {
                String::new()
            },
        })
        .collect();
    let sexes: Vec<Option<String>> = retenues
        .iter()
        .map(|r| r.inscription.sexe.clone())
        .collect();
    let presents: Vec<u32> = retenues
        .iter()
        .enumerate()
        .filter(|(_, r)| r.presente)
        .map(|(i, _)| place(i))
        .collect();
    let presentes = || {
        retenues
            .iter()
            .enumerate()
            .filter(|(_, r)| r.presente)
            .map(|(i, r)| (place(i), r))
    };
    let sans_sexe_cle = "statistiques.hors.eleves_sans_sexe";

    // Par classe : toutes les classes du périmètre, même vides ; une classe retirée ne paraît
    // que si un élève y compte encore.
    let mut par_classe: Vec<(&str, Groupe)> = Vec::new();
    for c in classes.iter().filter(|c| p.retient(c) && !c.supprimee) {
        par_classe.push((c.id.as_str(), Groupe::nouveau(c.libelle.clone(), None)));
    }
    let mut par_niveau: Vec<(&str, Groupe)> = Vec::new();
    for c in classes.iter().filter(|c| p.retient(c) && !c.supprimee) {
        if !par_niveau.iter().any(|(id, _)| *id == c.niveau_id) {
            par_niveau.push((
                c.niveau_id.as_str(),
                Groupe::nouveau(c.niveau.clone(), None),
            ));
        }
    }
    let mut par_cycle: Vec<(&str, Groupe)> = Vec::new();
    for c in classes.iter().filter(|c| p.retient(c) && !c.supprimee) {
        if !par_cycle.iter().any(|(id, _)| *id == c.cycle) {
            par_cycle.push((
                c.cycle.as_str(),
                Groupe::nouveau("", Some(cle_cycle(&c.cycle))),
            ));
        }
    }
    let mut redoublants: Vec<(&str, Groupe)> = par_niveau
        .iter()
        .map(|(id, g)| (*id, Groupe::nouveau(g.libelle.clone(), None)))
        .collect();
    let mut nouveaux: Vec<(&str, Groupe)> = par_niveau
        .iter()
        .map(|(id, g)| (*id, Groupe::nouveau(g.libelle.clone(), None)))
        .collect();
    let mut par_naissance: BTreeMap<i32, Vec<u32>> = BTreeMap::new();
    let mut par_age: BTreeMap<i32, Vec<u32>> = BTreeMap::new();
    let mut sans_naissance: Vec<u32> = Vec::new();
    for (m, r) in presentes() {
        ajouter(
            &mut par_classe,
            &r.classe.id,
            || Groupe::nouveau(r.classe.libelle.clone(), None),
            m,
        );
        ajouter(
            &mut par_niveau,
            &r.classe.niveau_id,
            || Groupe::nouveau(r.classe.niveau.clone(), None),
            m,
        );
        ajouter(
            &mut par_cycle,
            &r.classe.cycle,
            || Groupe::nouveau("", Some(cle_cycle(&r.classe.cycle))),
            m,
        );
        if r.inscription.redoublant {
            ajouter(
                &mut redoublants,
                &r.classe.niveau_id,
                || Groupe::nouveau(r.classe.niveau.clone(), None),
                m,
            );
        }
        if r.inscription.type_inscription != "reinscription" {
            ajouter(
                &mut nouveaux,
                &r.classe.niveau_id,
                || Groupe::nouveau(r.classe.niveau.clone(), None),
                m,
            );
        }
        match date(r.inscription.date_naissance.as_deref()) {
            Some(naissance) => {
                par_naissance.entry(naissance.annee()).or_default().push(m);
                par_age
                    .entry(p.rentree.annee() - naissance.annee())
                    .or_default()
                    .push(m);
            }
            None => sans_naissance.push(m),
        }
    }
    let groupes = |v: Vec<(&str, Groupe)>| v.into_iter().map(|(_, g)| g).collect::<Vec<_>>();
    let par_annees = |v: BTreeMap<i32, Vec<u32>>| {
        v.into_iter()
            .map(|(a, membres)| Groupe {
                libelle: a.to_string(),
                cle: None,
                membres,
            })
            .collect::<Vec<_>>()
    };
    let avec_naissance = |mut t: TableauStatistique| {
        if !sans_naissance.is_empty() {
            t.hors.insert(
                0,
                HorsColonnes {
                    cle: "statistiques.hors.eleves_sans_naissance".to_owned(),
                    membres: sans_naissance.clone(),
                },
            );
        }
        t
    };

    // Entrées et départs depuis la rentrée.
    let mut entrees: Vec<(&str, Groupe)> = ["nouvelle", "transfert_entrant", "reinscription"]
        .iter()
        .map(|t| {
            (
                *t,
                Groupe::nouveau("", Some(format!("inscriptions.type.{t}"))),
            )
        })
        .collect();
    let mut departs: Vec<(&str, Groupe)> = [
        ("transferee", "statistiques.depart.transferee"),
        ("abandonnee", "statistiques.depart.abandonnee"),
        ("radiee", "statistiques.depart.radiee"),
    ]
    .iter()
    .map(|(t, cle)| (*t, Groupe::nouveau("", Some((*cle).to_owned()))))
    .collect();
    for (i, r) in retenues.iter().enumerate() {
        if r.entree
            && let Some((_, g)) = entrees
                .iter_mut()
                .find(|(t, _)| *t == r.inscription.type_inscription)
        {
            g.membres.push(place(i));
        }
        if r.depart.is_some()
            && let Some((_, g)) = departs.iter_mut().find(|(t, _)| *t == r.inscription.statut)
        {
            g.membres.push(place(i));
        }
    }
    let sans_vide = |v: Vec<(&str, Groupe)>| {
        v.into_iter()
            .map(|(_, g)| g)
            .filter(|g| !g.membres.is_empty())
            .collect::<Vec<_>>()
    };

    let tableaux = vec![
        tableau_par_sexe(
            "eleves_par_classe",
            COLONNES_ELEVES,
            groupes(par_classe),
            Some(presents.clone()),
            &sexes,
            sans_sexe_cle,
        ),
        tableau_par_sexe(
            "eleves_par_niveau",
            COLONNES_ELEVES,
            groupes(par_niveau),
            Some(presents.clone()),
            &sexes,
            sans_sexe_cle,
        ),
        tableau_par_sexe(
            "eleves_par_cycle",
            COLONNES_ELEVES,
            groupes(par_cycle),
            Some(presents.clone()),
            &sexes,
            sans_sexe_cle,
        ),
        tableau_par_sexe(
            "redoublants_par_niveau",
            COLONNES_ELEVES,
            groupes(redoublants),
            None,
            &sexes,
            sans_sexe_cle,
        ),
        avec_naissance(tableau_par_sexe(
            "eleves_par_annee_naissance",
            COLONNES_ELEVES,
            par_annees(par_naissance),
            Some(presents.clone()),
            &sexes,
            sans_sexe_cle,
        )),
        avec_naissance(tableau_par_sexe(
            "eleves_par_age",
            COLONNES_ELEVES,
            par_annees(par_age),
            Some(presents),
            &sexes,
            sans_sexe_cle,
        )),
        tableau_par_sexe(
            "nouveaux_par_niveau",
            COLONNES_ELEVES,
            groupes(nouveaux),
            None,
            &sexes,
            sans_sexe_cle,
        ),
        tableau_par_sexe(
            "entrees_depuis_rentree",
            COLONNES_ELEVES,
            sans_vide(entrees),
            None,
            &sexes,
            sans_sexe_cle,
        ),
        tableau_par_sexe(
            "departs_depuis_rentree",
            COLONNES_ELEVES,
            sans_vide(departs),
            None,
            &sexes,
            sans_sexe_cle,
        ),
    ];
    let mut tableaux = tableaux;
    tableaux.extend(tableaux_complements_eleves(&retenues, &sexes, c));
    OngletStatistique { fiches, tableaux }
}

/// Engagement en cours à la date : commencé, pas encore arrêté ; le plus récent s'il y en a
/// plusieurs.
fn engagement_le<'a>(
    engagements: &'a [EngagementStat],
    membre: &str,
    jour: DateCivile,
) -> Option<&'a EngagementStat> {
    engagements
        .iter()
        .filter(|e| e.membre_id == membre)
        .filter(|e| date(Some(&e.debut)).is_some_and(|d| d <= jour))
        .filter(|e| date(e.arrete_le.as_deref()).is_none_or(|a| a >= jour))
        .max_by(|a, b| a.debut.cmp(&b.debut))
}

/// Nature de l'engagement en cours d'un membre à une date.
pub(crate) fn nature_engagement_le(
    engagements: &[EngagementStat],
    membre: &str,
    jour: DateCivile,
) -> Option<String> {
    engagement_le(engagements, membre, jour).map(|e| e.nature.clone())
}

/// Intitulé du diplôme le plus récent, d'après les diplômes saisis.
fn dernier_diplome(diplomes: &str) -> Option<String> {
    let liste: Vec<serde_json::Value> = serde_json::from_str(diplomes).ok()?;
    liste
        .iter()
        .enumerate()
        .max_by_key(|(i, d)| (d["annee"].as_i64().unwrap_or(i64::MIN), *i))
        .and_then(|(_, d)| d["intitule"].as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

fn onglet_personnel(
    p: &Perimetre<'_>,
    membres: &[MembreStat],
    engagements: &[EngagementStat],
    affectations: &[AffectationStat],
    seances: &[statistiques::SeanceStat],
    c: &ComplementsStat,
) -> OngletStatistique {
    let cycles_de = |personne: &str| -> Vec<&str> {
        let mut v: Vec<&str> = affectations
            .iter()
            .filter(|a| a.personne_id == personne)
            .filter_map(|a| p.classes.get(a.classe_id.as_str()))
            .map(|c| c.cycle.as_str())
            .collect();
        v.sort_unstable();
        v.dedup();
        v
    };
    let retenus: Vec<&MembreStat> = membres
        .iter()
        .filter(|m| date(m.arrive_le.as_deref()).is_none_or(|a| a <= p.date))
        .filter(|m| date(m.parti_le.as_deref()).is_none_or(|d| d > p.date))
        .filter(|m| {
            p.cycle
                .is_none_or(|c| cycles_de(&m.personne_id).contains(&c.nom()))
        })
        .collect();
    let fiches: Vec<FicheStatistique> = retenus
        .iter()
        .map(|m| FicheStatistique {
            id: m.id.clone(),
            nom: format!("{} {}", m.nom, m.prenoms).trim().to_owned(),
            detail: m.categorie.clone(),
            motif: String::new(),
        })
        .collect();
    let sexes: Vec<Option<String>> = retenus.iter().map(|m| m.sexe.clone()).collect();
    let tous: Vec<u32> = (0..retenus.len()).map(place).collect();
    let sans_sexe_cle = "statistiques.hors.personnel_sans_sexe";

    // Par catégorie, dans l'ordre de l'école.
    let mut categories: Vec<(i64, Groupe)> = Vec::new();
    for (i, m) in retenus.iter().enumerate() {
        match categories
            .iter_mut()
            .find(|(_, g)| g.libelle == m.categorie)
        {
            Some((_, g)) => g.membres.push(place(i)),
            None => {
                let mut g = Groupe::nouveau(m.categorie.clone(), None);
                g.membres.push(place(i));
                categories.push((m.categorie_ordre, g));
            }
        }
    }
    categories.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.libelle.cmp(&b.1.libelle)));

    let mut fonctionnaires = Groupe::nouveau("", Some("statistiques.fonctionnaire".to_owned()));
    let mut autres = Groupe::nouveau("", Some("statistiques.non_fonctionnaire".to_owned()));
    for (i, m) in retenus.iter().enumerate() {
        if m.fonctionnaire {
            fonctionnaires.membres.push(place(i));
        } else {
            autres.membres.push(place(i));
        }
    }

    let natures = [
        "duree_indeterminee",
        "duree_determinee",
        "vacation",
        "stage",
        "benevole",
    ];
    let mut par_nature: Vec<(&str, Groupe)> = natures
        .iter()
        .map(|n| {
            (
                *n,
                Groupe::nouveau("", Some(format!("engagement.nature.{n}"))),
            )
        })
        .collect();
    let mut sans_engagement: Vec<u32> = Vec::new();
    for (i, m) in retenus.iter().enumerate() {
        match engagement_le(engagements, &m.id, p.date)
            .and_then(|e| par_nature.iter_mut().find(|(n, _)| *n == e.nature))
        {
            Some((_, g)) => g.membres.push(place(i)),
            None => sans_engagement.push(place(i)),
        }
    }
    let mut tableau_engagements = tableau_par_sexe(
        "personnel_par_engagement",
        COLONNES_PERSONNEL,
        par_nature
            .into_iter()
            .map(|(_, g)| g)
            .filter(|g| !g.membres.is_empty())
            .collect(),
        Some(tous.clone()),
        &sexes,
        sans_sexe_cle,
    );
    if !sans_engagement.is_empty() {
        tableau_engagements.hors.insert(
            0,
            HorsColonnes {
                cle: "statistiques.hors.personnel_sans_engagement".to_owned(),
                membres: sans_engagement,
            },
        );
    }

    // Enseignants par cycle et par discipline, d'après les affectations de l'année : un maître
    // de deux cycles compte dans chacun, et une seule fois au total.
    let place_de: HashMap<&str, u32> = retenus
        .iter()
        .enumerate()
        .map(|(i, m)| (m.personne_id.as_str(), place(i)))
        .collect();
    let mut par_cycle: Vec<(String, Groupe)> = Vec::new();
    for c in Cycle::TOUS {
        par_cycle.push((
            c.nom().to_owned(),
            Groupe::nouveau("", Some(cle_cycle(c.nom()))),
        ));
    }
    let mut par_matiere: Vec<(i64, String, Groupe)> = Vec::new();
    for a in affectations {
        let Some(m) = place_de.get(a.personne_id.as_str()).copied() else {
            continue;
        };
        let Some(classe) = p.classes.get(a.classe_id.as_str()) else {
            continue;
        };
        if !p.retient(classe) {
            continue;
        }
        if let Some((_, g)) = par_cycle.iter_mut().find(|(n, _)| *n == classe.cycle)
            && !g.membres.contains(&m)
        {
            g.membres.push(m);
        }
        match par_matiere
            .iter_mut()
            .find(|(_, id, _)| *id == a.matiere_id)
        {
            Some((_, _, g)) => {
                if !g.membres.contains(&m) {
                    g.membres.push(m);
                }
            }
            None => {
                let mut g = Groupe::nouveau(a.matiere.clone(), None);
                g.membres.push(m);
                par_matiere.push((a.matiere_ordre, a.matiere_id.clone(), g));
            }
        }
    }
    par_matiere.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.2.libelle.cmp(&b.2.libelle)));

    // Statut pour les rapports : saisi, sinon déduit de la fiche ; jamais vide.
    let mut par_statut: Vec<Groupe> = StatutRapport::TOUS
        .iter()
        .map(|s| Groupe::nouveau("", Some(format!("complements.statut.{}", s.nom()))))
        .collect();
    for (i, m) in retenus.iter().enumerate() {
        let statut = c
            .membres
            .get(&m.id)
            .and_then(|x| x.statut)
            .unwrap_or_else(|| {
                StatutRapport::deduit(
                    m.fonctionnaire,
                    engagement_le(engagements, &m.id, p.date).map(|e| e.nature.as_str()),
                )
            });
        if let Some(g) = StatutRapport::TOUS
            .iter()
            .position(|s| *s == statut)
            .and_then(|k| par_statut.get_mut(k))
        {
            g.membres.push(place(i));
        }
    }
    // Spécialité des maîtres du second cycle : saisie, sinon proposée d'après leurs matières,
    // sinon « autres » ; diplôme le plus élevé des enseignants : saisi, sinon proposé.
    let mut matieres_de: HashMap<u32, Vec<&str>> = HashMap::new();
    for a in affectations {
        if let (Some(m), Some(classe)) = (
            place_de.get(a.personne_id.as_str()),
            p.classes.get(a.classe_id.as_str()),
        ) && p.retient(classe)
        {
            matieres_de.entry(*m).or_default().push(a.matiere.as_str());
        }
    }
    let second: Vec<u32> = par_cycle
        .iter()
        .find(|(n, _)| n == "second_cycle")
        .map(|(_, g)| g.membres.clone())
        .unwrap_or_default();
    let mut enseignants: Vec<u32> = par_cycle
        .iter()
        .flat_map(|(_, g)| g.membres.iter().copied())
        .collect();
    enseignants.sort_unstable();
    enseignants.dedup();
    let membre_a = |m: u32| usize::try_from(m).ok().and_then(|k| retenus.get(k));
    let mut par_specialite: Vec<Groupe> = Specialite::TOUS
        .iter()
        .map(|s| Groupe::nouveau("", Some(format!("complements.specialite.{}", s.nom()))))
        .collect();
    for m in &second {
        let Some(membre) = membre_a(*m) else {
            continue;
        };
        let specialite = c
            .membres
            .get(&membre.id)
            .and_then(|x| x.specialite)
            .or_else(|| Specialite::proposee(matieres_de.get(m).cloned().unwrap_or_default()))
            .unwrap_or(Specialite::Autre);
        if let Some(g) = Specialite::TOUS
            .iter()
            .position(|s| *s == specialite)
            .and_then(|k| par_specialite.get_mut(k))
        {
            g.membres.push(*m);
        }
    }
    let mut par_diplome: Vec<Groupe> = DiplomeEleve::TOUS
        .iter()
        .map(|d| Groupe::nouveau("", Some(format!("complements.diplome.{}", d.nom()))))
        .collect();
    let mut sans_diplome: Vec<u32> = Vec::new();
    for m in &enseignants {
        let Some(membre) = membre_a(*m) else {
            continue;
        };
        let diplome = c
            .membres
            .get(&membre.id)
            .and_then(|x| x.diplome)
            .or_else(|| DiplomeEleve::propose(&membre.diplomes));
        match diplome.and_then(|d| {
            DiplomeEleve::TOUS
                .iter()
                .position(|x| *x == d)
                .and_then(|k| par_diplome.get_mut(k))
        }) {
            Some(g) => g.membres.push(*m),
            None => sans_diplome.push(*m),
        }
    }
    let non_vides = |v: Vec<Groupe>| {
        v.into_iter()
            .filter(|g| !g.membres.is_empty())
            .collect::<Vec<_>>()
    };
    let tableau_statut = tableau_par_sexe(
        "personnel_par_statut",
        COLONNES_PERSONNEL,
        non_vides(par_statut),
        Some(tous.clone()),
        &sexes,
        sans_sexe_cle,
    );
    let tableau_specialite = tableau_par_sexe(
        "enseignants_par_specialite",
        COLONNES_PERSONNEL,
        non_vides(par_specialite),
        Some(second),
        &sexes,
        sans_sexe_cle,
    );
    let mut tableau_diplome = tableau_par_sexe(
        "enseignants_par_diplome",
        COLONNES_PERSONNEL,
        non_vides(par_diplome),
        Some(enseignants),
        &sexes,
        sans_sexe_cle,
    );
    if !sans_diplome.is_empty() {
        tableau_diplome.hors.insert(
            0,
            HorsColonnes {
                cle: "statistiques.hors.enseignants_sans_diplome".to_owned(),
                membres: sans_diplome,
            },
        );
    }

    // Heures hebdomadaires par discipline, d'après l'emploi du temps quand l'école le tient.
    let mut heures: Vec<(i64, String, String, i64)> = Vec::new();
    for s in seances {
        if !p
            .classes
            .get(s.classe_id.as_str())
            .is_some_and(|c| p.retient(c))
        {
            continue;
        }
        match heures.iter_mut().find(|(_, id, _, _)| *id == s.matiere_id) {
            Some((_, _, _, minutes)) => *minutes += s.minutes,
            None => heures.push((
                s.matiere_ordre,
                s.matiere_id.clone(),
                s.matiere.clone(),
                s.minutes,
            )),
        }
    }
    heures.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.2.cmp(&b.2)));
    let total_minutes: i64 = heures.iter().map(|h| h.3).sum();
    let minutes = |n: i64| CelluleStatistique {
        nombre: u32::try_from(n).ok(),
        texte: None,
        membres: Vec::new(),
    };
    let tableau_heures = TableauStatistique {
        code: "heures_par_discipline".to_owned(),
        colonnes: vec!["heures_hebdomadaires".to_owned()],
        total: (!heures.is_empty()).then(|| LigneStatistique {
            libelle: String::new(),
            cle: Some("statistiques.total".to_owned()),
            cellules: vec![minutes(total_minutes)],
        }),
        lignes: heures
            .into_iter()
            .map(|(_, _, matiere, n)| LigneStatistique {
                libelle: matiere,
                cle: None,
                cellules: vec![minutes(n)],
            })
            .collect(),
        hors: Vec::new(),
    };

    // Liste nominative : ce que la fiche porte, une case vide quand elle ne le porte pas.
    let texte = |t: Option<String>| CelluleStatistique {
        nombre: None,
        texte: t.filter(|s| !s.trim().is_empty()),
        membres: Vec::new(),
    };
    let nominative = TableauStatistique {
        code: "liste_nominative".to_owned(),
        colonnes: COLONNES_NOMINATIVES
            .iter()
            .map(|c| (*c).to_owned())
            .collect(),
        lignes: retenus
            .iter()
            .map(|m| LigneStatistique {
                libelle: format!("{} {}", m.nom, m.prenoms).trim().to_owned(),
                cle: None,
                cellules: vec![
                    texte(Some(m.matricule.clone())),
                    texte(m.situation_famille.clone()),
                    texte(Some(m.categorie.clone())),
                    texte(Some(m.fonction.clone())),
                    texte(Some(if m.fonctionnaire { "oui" } else { "non" }.to_owned())),
                    texte(dernier_diplome(&m.diplomes)),
                    texte(m.arrive_le.clone()),
                ],
            })
            .collect(),
        total: None,
        hors: Vec::new(),
    };

    let tableaux = vec![
        tableau_par_sexe(
            "personnel_par_categorie",
            COLONNES_PERSONNEL,
            categories.into_iter().map(|(_, g)| g).collect(),
            Some(tous.clone()),
            &sexes,
            sans_sexe_cle,
        ),
        tableau_par_sexe(
            "personnel_par_situation",
            COLONNES_PERSONNEL,
            [fonctionnaires, autres]
                .into_iter()
                .filter(|g| !g.membres.is_empty())
                .collect(),
            Some(tous),
            &sexes,
            sans_sexe_cle,
        ),
        tableau_engagements,
        tableau_par_sexe(
            "enseignants_par_cycle",
            COLONNES_PERSONNEL,
            par_cycle
                .into_iter()
                .map(|(_, g)| g)
                .filter(|g| !g.membres.is_empty())
                .collect(),
            None,
            &sexes,
            sans_sexe_cle,
        ),
        tableau_par_sexe(
            "enseignants_par_discipline",
            COLONNES_PERSONNEL,
            par_matiere.into_iter().map(|(_, _, g)| g).collect(),
            None,
            &sexes,
            sans_sexe_cle,
        ),
        tableau_heures,
        nominative,
    ];
    let mut tableaux = tableaux;
    tableaux.insert(1.min(tableaux.len()), tableau_statut);
    let apres = tableaux
        .iter()
        .position(|t| t.code == "enseignants_par_discipline")
        .map_or(tableaux.len(), |k| k + 1);
    tableaux.splice(apres..apres, [tableau_specialite, tableau_diplome]);
    OngletStatistique { fiches, tableaux }
}

/// Tableaux des élèves tirés des compléments : par année de recrutement, pris en charge par
/// l'État, et, pour qui en a le droit, handicapés par type et degré, sans nom.
fn tableaux_complements_eleves(
    retenues: &[Retenue<'_>],
    sexes: &[Option<String>],
    c: &ComplementsStat,
) -> Vec<TableauStatistique> {
    let sans_sexe_cle = "statistiques.hors.eleves_sans_sexe";
    let presentes: Vec<(u32, &Retenue<'_>)> = retenues
        .iter()
        .enumerate()
        .filter(|(_, r)| r.presente)
        .map(|(i, r)| (place(i), r))
        .collect();
    let presents: Vec<u32> = presentes.iter().map(|(m, _)| *m).collect();

    let mut par_annee: BTreeMap<i32, Vec<u32>> = BTreeMap::new();
    let mut sans_annee: Vec<u32> = Vec::new();
    for (m, r) in &presentes {
        match c.annees_recrutement.get(&r.inscription.eleve_id) {
            Some(a) => par_annee.entry(*a).or_default().push(*m),
            None => sans_annee.push(*m),
        }
    }
    let mut recrutement = tableau_par_sexe(
        "eleves_par_annee_recrutement",
        COLONNES_ELEVES,
        par_annee
            .into_iter()
            .map(|(a, membres)| Groupe {
                libelle: a.to_string(),
                cle: None,
                membres,
            })
            .collect(),
        Some(presents),
        sexes,
        sans_sexe_cle,
    );
    if !sans_annee.is_empty() {
        recrutement.hors.insert(
            0,
            HorsColonnes {
                cle: "statistiques.hors.eleves_sans_recrutement".to_owned(),
                membres: sans_annee,
            },
        );
    }

    let mut par_niveau: Vec<(i64, String, Groupe)> = Vec::new();
    for (m, r) in &presentes {
        if !c.pris_en_charge.contains(&r.inscription.eleve_id) {
            continue;
        }
        match par_niveau
            .iter_mut()
            .find(|(_, id, _)| *id == r.classe.niveau_id)
        {
            Some((_, _, g)) => g.membres.push(*m),
            None => {
                let mut g = Groupe::nouveau(r.classe.niveau.clone(), None);
                g.membres.push(*m);
                par_niveau.push((r.classe.ordre, r.classe.niveau_id.clone(), g));
            }
        }
    }
    par_niveau.sort_by_key(|(ordre, _, _)| *ordre);
    let mut tableaux = vec![
        recrutement,
        tableau_par_sexe(
            "eleves_pris_en_charge",
            COLONNES_ELEVES,
            par_niveau.into_iter().map(|(_, _, g)| g).collect(),
            None,
            sexes,
            sans_sexe_cle,
        ),
    ];

    if let Some(difficultes) = &c.difficultes {
        let mut groupes: Vec<Groupe> = Vec::new();
        for t in TypeDifficulte::TOUS {
            for d in DegreDifficulte::TOUS {
                let mut g = Groupe::nouveau(
                    "",
                    Some(format!(
                        "complements.ligne_difficulte.{}.{}",
                        t.nom(),
                        d.nom()
                    )),
                );
                for (m, r) in &presentes {
                    if difficultes.iter().any(|x| {
                        x.eleve_id == r.inscription.eleve_id
                            && x.type_difficulte == *t
                            && x.degre == Some(*d)
                    }) {
                        g.membres.push(*m);
                    }
                }
                if !g.membres.is_empty() {
                    groupes.push(g);
                }
            }
        }
        let mut t = tableau_par_sexe(
            "eleves_handicapes",
            COLONNES_ELEVES,
            groupes,
            None,
            sexes,
            sans_sexe_cle,
        );
        // Des chiffres sans nom : aucune case ne mène aux fiches.
        for ligne in t.lignes.iter_mut().chain(t.total.iter_mut()) {
            for cellule in &mut ligne.cellules {
                cellule.membres.clear();
            }
        }
        t.hors.clear();
        tableaux.push(t);
    }
    tableaux
}
