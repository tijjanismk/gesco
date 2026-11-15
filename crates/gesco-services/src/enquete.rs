//! Données pour l'enquête annuelle de la CPS, secondaire général, technique et professionnel
//! (SPEC_PHASE_9 §6). GESCO ne reproduit pas le questionnaire : il écrit un classeur d'une feuille
//! par partie, dans l'ordre du questionnaire, avec pour chaque rubrique sa valeur, ou une case
//! vide, ou la mention de ce qui reste à remplir à la main. Les chiffres sont ceux des
//! statistiques, à la même date d'arrêt ; rien ne s'écrit en base, rien n'est refusé parce
//! qu'une donnée manque.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use gesco_core::commun::{DateCivile, Id};
use gesco_core::core::complements::{DegreDifficulte, DiplomeEleve, StatutRapport, TypeDifficulte};
use gesco_core::core::{AnneeScolaire, FamilleInventaire, RubriqueFiche, ThemeRubrique};
use gesco_core::scolarite::Cycle;
use gesco_db::depots::statistiques::{FraisNiveau, MembreStat};
use gesco_db::depots::{etablissement, fiche, statistiques};
use serde::Deserialize;

use crate::erreur::Resultat;
use crate::export::{Feuille, FichierEcrit, exporter_xlsx_feuilles};
use crate::noyau::Noyau;
use crate::rapports::reponse_fiche;
use crate::rapports_tableaux::{releve_si, statut, titre_diplome, titre_statut};
use crate::session::Session;
use crate::statistiques::{DemandeStatistiques, DonneesStat, Retenue, date, retenir};

/// Ce que l'écran demande : l'année, la date d'arrêt, le classeur à écrire.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DemandeEnquete {
    /// Année ; l'année courante si rien n'est donné.
    pub annee_id: Option<Id<AnneeScolaire>>,
    /// Date d'arrêt des chiffres ; aujourd'hui si rien n'est donné, ramené dans l'année.
    pub date: Option<DateCivile>,
    /// Chemin du classeur à écrire, extension `.xlsx`.
    pub chemin: String,
}

/// Mention d'une rubrique que GESCO ne tient pas.
pub(crate) const A_LA_MAIN: &str = "À remplir à la main";

/// Cycles du secondaire.
const SECONDAIRE: [Cycle; 2] = [Cycle::Lycee, Cycle::Technique];

fn du_secondaire(cycle: &str) -> bool {
    Cycle::depuis_nom(cycle).is_some_and(|c| SECONDAIRE.contains(&c))
}

fn texte(n: usize) -> String {
    n.to_string()
}

fn ligne(cases: &[&str]) -> Vec<String> {
    cases.iter().map(|c| (*c).to_owned()).collect()
}

/// Un tableau d'une feuille : son titre (vide pour le premier), ses en-têtes, ses lignes.
struct Bloc {
    titre: String,
    entetes: Vec<String>,
    lignes: Vec<Vec<String>>,
}

impl Bloc {
    fn nouveau(titre: &str, entetes: &[&str]) -> Self {
        Self {
            titre: titre.to_owned(),
            entetes: ligne(entetes),
            lignes: Vec::new(),
        }
    }
}

/// Une feuille de plusieurs tableaux, l'un sous l'autre : le premier donne les en-têtes de la
/// feuille ; chaque suivant vient après une ligne vide, son titre et ses en-têtes. Toutes les
/// lignes ont la largeur de la plus large.
fn feuille(titre: &str, blocs: Vec<Bloc>) -> Feuille {
    let largeur = blocs
        .iter()
        .flat_map(|b| std::iter::once(&b.entetes).chain(b.lignes.iter()))
        .map(Vec::len)
        .max()
        .unwrap_or(1);
    let caler = |mut l: Vec<String>| {
        l.resize(largeur, String::new());
        l
    };
    let mut blocs = blocs.into_iter();
    let premier = blocs.next();
    let entetes = caler(
        premier
            .as_ref()
            .map(|b| b.entetes.clone())
            .unwrap_or_default(),
    );
    let mut lignes: Vec<Vec<String>> = premier
        .map(|b| b.lignes.into_iter().map(&caler).collect())
        .unwrap_or_default();
    for b in blocs {
        lignes.push(caler(Vec::new()));
        lignes.push(caler(vec![b.titre]));
        lignes.push(caler(b.entetes));
        lignes.extend(b.lignes.into_iter().map(&caler));
    }
    Feuille {
        titre: titre.to_owned(),
        entetes,
        lignes,
    }
}

/// Garçons, filles, total (le total compte aussi ceux dont le sexe n'est pas noté).
#[derive(Clone, Copy, Default)]
struct Sexes {
    g: usize,
    f: usize,
    t: usize,
}

impl Sexes {
    fn ajouter(&mut self, sexe: Option<&str>) {
        match sexe {
            Some("masculin") => self.g += 1,
            Some("feminin") => self.f += 1,
            _ => {}
        }
        self.t += 1;
    }

    fn cases(self) -> [String; 3] {
        [texte(self.g), texte(self.f), texte(self.t)]
    }
}

/// Un groupe d'élèves du secondaire : un niveau, et sa série s'il en a.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Groupe {
    ordre: i64,
    niveau: String,
    serie: String,
}

impl Groupe {
    fn libelle(&self) -> String {
        if self.serie.is_empty() {
            self.niveau.clone()
        } else {
            format!("{} {}", self.niveau, self.serie)
        }
    }
}

/// Les élèves du secondaire présents à la date d'arrêt.
fn eleves_presents<'a>(d: &'a DonneesStat) -> Vec<Retenue<'a>> {
    let perimetre = d.perimetre(None);
    retenir(&perimetre, &d.inscriptions)
        .into_iter()
        .filter(|r| r.presente && du_secondaire(&r.classe.cycle))
        .collect()
}

fn groupe_de(r: &Retenue<'_>) -> Groupe {
    Groupe {
        ordre: r.classe.ordre,
        niveau: r.classe.niveau.clone(),
        serie: r.classe.serie.clone().unwrap_or_default(),
    }
}

/// Âge d'un élève : année civile de la rentrée moins année de naissance (règle 8).
fn age(d: &DonneesStat, r: &Retenue<'_>) -> Option<i32> {
    let rentree = d.rentree.unwrap_or(d.date);
    date(r.inscription.date_naissance.as_deref()).map(|n| rentree.annee() - n.annee())
}

/// Les membres du personnel présents à la date : ceux qui ont une classe du secondaire cette
/// année, et ceux qui n'ont aucune classe (personnel de direction, d'administration, de
/// service). Le booléen dit s'ils tiennent une classe du secondaire.
fn personnel(d: &DonneesStat) -> Vec<(&MembreStat, bool)> {
    let classes: HashMap<&str, &str> = d
        .classes
        .iter()
        .map(|c| (c.id.as_str(), c.cycle.as_str()))
        .collect();
    d.membres
        .iter()
        .filter(|m| date(m.arrive_le.as_deref()).is_none_or(|a| a <= d.date))
        .filter(|m| date(m.parti_le.as_deref()).is_none_or(|x| x > d.date))
        .filter_map(|m| {
            let cycles: Vec<&str> = d
                .affectations
                .iter()
                .filter(|a| a.personne_id == m.personne_id)
                .filter_map(|a| classes.get(a.classe_id.as_str()).copied())
                .collect();
            let tient = cycles.iter().any(|c| du_secondaire(c));
            (cycles.is_empty() || tient).then_some((m, tient))
        })
        .collect()
}

fn diplome(d: &DonneesStat, m: &MembreStat) -> Option<DiplomeEleve> {
    d.complements
        .membres
        .get(&m.id)
        .and_then(|x| x.diplome)
        .or_else(|| DiplomeEleve::propose(&m.diplomes))
}

fn titre_type(t: TypeDifficulte) -> &'static str {
    match t {
        TypeDifficulte::Vue => "Voir",
        TypeDifficulte::Ouie => "Entendre",
        TypeDifficulte::MotriciteGlobale => "Marcher, se déplacer",
        TypeDifficulte::MotriciteFine => "Se servir de ses mains",
        TypeDifficulte::Intellect => "Apprendre, comprendre",
        TypeDifficulte::Communication => "Communiquer",
        TypeDifficulte::Comportement => "Se comporter",
        TypeDifficulte::Multiple => "Plusieurs difficultés",
        TypeDifficulte::Autre => "Autre",
    }
}

fn titre_degre(d: Option<DegreDifficulte>) -> &'static str {
    match d {
        Some(DegreDifficulte::Quelques) => "Quelques difficultés",
        Some(DegreDifficulte::Beaucoup) => "Beaucoup de difficultés",
        Some(DegreDifficulte::Impossible) => "N'y parvient pas",
        None => "",
    }
}

fn titre_statut_ecole(nom: &str) -> &'static str {
    match nom {
        "public" => "Public",
        "prive" => "Privé",
        "communautaire" => "Communautaire",
        "medersa" => "Medersa",
        _ => "",
    }
}

/// Montant en francs, avec une espace entre les milliers.
fn francs(montant: i64) -> String {
    let chiffres = montant.unsigned_abs().to_string();
    let mut sortie = String::new();
    for (i, c) in chiffres.chars().enumerate() {
        if i > 0 && (chiffres.len() - i).is_multiple_of(3) {
            sortie.push('\u{a0}');
        }
        sortie.push(c);
    }
    if montant < 0 {
        sortie.insert(0, '-');
    }
    sortie
}

/// Durée hebdomadaire lisible : « 3 h », « 1 h 30 ».
fn heures(minutes: i64) -> String {
    let (h, m) = (minutes / 60, minutes % 60);
    if m == 0 {
        format!("{h} h")
    } else {
        format!("{h} h {m:02}")
    }
}

/// Ce que le classeur lit en plus des données des statistiques.
pub(crate) struct Annexes<'a> {
    /// Nom, statut, académie, CAP, adresse, téléphones de l'école.
    pub(crate) ecole: &'a gesco_core::core::Etablissement,
    /// Rubriques de la fiche de l'établissement.
    pub(crate) rubriques: &'a [RubriqueFiche],
    /// Réponses de la fiche.
    pub(crate) valeurs: &'a [fiche::ValeurFiche],
    /// Frais de scolarité de l'année et de l'année précédente.
    pub(crate) frais: (&'a [FraisNiveau], &'a [FraisNiveau]),
    /// Données de l'année précédente, à son dernier jour, s'il y en a une.
    pub(crate) precedente: Option<&'a DonneesStat>,
}

/// Les rubriques actives de la fiche, des thèmes donnés, dans leur ordre, avec leur réponse.
fn rubriques(d: &DonneesStat, a: &Annexes<'_>, themes: &[ThemeRubrique]) -> Vec<Vec<String>> {
    let annee = d
        .choisie
        .as_ref()
        .map(|x| x.id.vers_chaine())
        .unwrap_or_default();
    let mut retenues: Vec<&RubriqueFiche> = a
        .rubriques
        .iter()
        .filter(|r| r.retiree_le.is_none() && themes.contains(&r.theme))
        .collect();
    retenues.sort_by_key(|r| (themes.iter().position(|t| *t == r.theme), r.ordre));
    retenues
        .into_iter()
        .map(|r| {
            let valeur = match &r.code {
                Some(code) => reponse_fiche(a.rubriques, a.valeurs, &annee, code),
                None => reponse_libre(r, a.valeurs, &annee),
            };
            vec![r.libelle.clone(), valeur]
        })
        .collect()
}

/// Réponse à une rubrique ajoutée par l'école (sans code) : celle de l'année, sinon la
/// permanente.
pub(crate) fn reponse_libre(
    r: &RubriqueFiche,
    valeurs: &[fiche::ValeurFiche],
    annee: &str,
) -> String {
    let de = |x: Option<&str>| {
        valeurs
            .iter()
            .find(|v| v.rubrique_id == r.id && v.annee_id.map(|a| a.vers_chaine()).as_deref() == x)
            .and_then(|v| v.valeur.clone())
    };
    de(Some(annee))
        .or_else(|| de(None))
        .unwrap_or_default()
        .replace(gesco_core::core::fiche::SEPARATEUR, ", ")
}

/// Partie I : identification et localisation.
fn identification(d: &DonneesStat, a: &Annexes<'_>) -> Feuille {
    let mut b = Bloc::nouveau("", &["Rubrique", "Valeur"]);
    let e = a.ecole;
    b.lignes
        .push(vec!["Nom de l'établissement".to_owned(), e.nom.clone()]);
    b.lignes.push(vec![
        "Statut".to_owned(),
        titre_statut_ecole(e.statut.nom()).to_owned(),
    ]);
    b.lignes
        .push(vec!["Académie".to_owned(), e.code_academie.clone()]);
    b.lignes.push(vec![
        "Centre d'animation pédagogique".to_owned(),
        e.code_cap.clone(),
    ]);
    b.lignes.push(vec!["Adresse".to_owned(), e.adresse.clone()]);
    b.lignes
        .push(vec!["Téléphone".to_owned(), e.telephones.join(", ")]);
    b.lignes.extend(rubriques(
        d,
        a,
        &[
            ThemeRubrique::Localisation,
            ThemeRubrique::Identite,
            ThemeRubrique::Actes,
        ],
    ));
    b.lignes
        .push(ligne(&["Nom et téléphone du proviseur", A_LA_MAIN]));
    feuille("I. Identification", vec![b])
}

/// Partie II : informations générales.
fn generales(
    d: &DonneesStat,
    a: &Annexes<'_>,
    handicapes: Option<&[(TypeDifficulte, Sexes)]>,
) -> Feuille {
    let mut b = Bloc::nouveau("", &["Rubrique", "Valeur"]);
    b.lignes.extend(rubriques(
        d,
        a,
        &[
            ThemeRubrique::EauHygiene,
            ThemeRubrique::Energie,
            ThemeRubrique::Equipements,
            ThemeRubrique::VieEcole,
            ThemeRubrique::Autres,
        ],
    ));
    match handicapes {
        Some(par_type) => {
            for (t, s) in par_type {
                b.lignes.push(vec![
                    format!("Élèves handicapés : {}", titre_type(*t)),
                    texte(s.t),
                ]);
            }
        }
        None => b
            .lignes
            .push(ligne(&["Élèves handicapés par type", A_LA_MAIN])),
    }
    for r in [
        "Nombre de cas d'incidents",
        "Nombre de grossesses",
        "Nombre d'attaques",
    ] {
        b.lignes.push(ligne(&[r, A_LA_MAIN]));
    }
    feuille("II. Informations générales", vec![b])
}

/// Relevé d'un article sur les lignes de toute l'école et du secondaire.
fn releve_secondaire(
    d: &DonneesStat,
    article: &gesco_core::core::ArticleInventaire,
) -> Option<gesco_core::core::QuantitesInventaire> {
    releve_si(d, article, |c| c.is_none_or(|c| SECONDAIRE.contains(&c))).map(|(q, _)| q)
}

fn articles<'a>(
    d: &'a DonneesStat,
    familles: &[FamilleInventaire],
) -> Vec<&'a gesco_core::core::ArticleInventaire> {
    let mut v: Vec<_> = d
        .articles
        .iter()
        .filter(|a| a.retire_le.is_none() && familles.contains(&a.famille))
        .collect();
    v.sort_by_key(|a| (familles.iter().position(|f| *f == a.famille), a.ordre));
    v
}

fn case(n: Option<u32>) -> String {
    n.map(|x| x.to_string()).unwrap_or_default()
}

/// Partie III : nombre et état des locaux.
fn locaux(d: &DonneesStat) -> Feuille {
    let mut b = Bloc::nouveau(
        "",
        &[
            "Type de local",
            "Nombre",
            "Dont neufs",
            "Dont en location",
            "Murs bons",
            "Murs mauvais",
            "Toits bons",
            "Toits mauvais",
        ],
    );
    for a in articles(d, &[FamilleInventaire::Local]) {
        let q = releve_secondaire(d, a).unwrap_or_default();
        b.lignes.push(vec![
            a.designation.clone(),
            case(q.existant),
            case(q.neufs),
            case(q.en_location),
            case(q.murs_bons),
            case(q.murs_mauvais),
            case(q.toits_bons),
            case(q.toits_mauvais),
        ]);
    }
    feuille("III. Locaux", vec![b])
}

/// Bloc existant, bon état, besoins des articles des familles données.
fn bloc_quantites(d: &DonneesStat, titre: &str, familles: &[FamilleInventaire]) -> Bloc {
    let mut b = Bloc::nouveau(titre, &["Article", "Existant", "En bon état", "Besoins"]);
    for a in articles(d, familles) {
        let q = releve_secondaire(d, a).unwrap_or_default();
        b.lignes.push(vec![
            a.designation.clone(),
            case(q.existant),
            case(q.bon_etat),
            case(q.besoins),
        ]);
    }
    b
}

/// Partie IV : mobilier et équipements.
fn mobilier(d: &DonneesStat) -> Feuille {
    feuille(
        "IV. Mobilier et équipements",
        vec![bloc_quantites(
            d,
            "",
            &[FamilleInventaire::Mobilier, FamilleInventaire::Materiel],
        )],
    )
}

/// Partie V : frais de scolarité annuels, année précédente et année en cours.
fn financieres(a: &Annexes<'_>) -> Feuille {
    let mut b = Bloc::nouveau("", &["Niveau", "Année précédente", "Année en cours"]);
    let (courants, precedents) = a.frais;
    let mut groupes: BTreeMap<Groupe, (Option<i64>, Option<i64>)> = BTreeMap::new();
    let cle = |f: &FraisNiveau| Groupe {
        ordre: f.ordre,
        niveau: f.niveau.clone(),
        serie: f.serie.clone().unwrap_or_default(),
    };
    for f in precedents.iter().filter(|f| du_secondaire(&f.cycle)) {
        groupes.entry(cle(f)).or_default().0 = Some(f.montant);
    }
    for f in courants.iter().filter(|f| du_secondaire(&f.cycle)) {
        groupes.entry(cle(f)).or_default().1 = Some(f.montant);
    }
    for (g, (avant, maintenant)) in groupes {
        b.lignes.push(vec![
            g.libelle(),
            avant.map(francs).unwrap_or_default(),
            maintenant.map(francs).unwrap_or_default(),
        ]);
    }
    feuille("V. Données financières", vec![b])
}

/// Partie VI.1 : personnel par statut et sexe ; enseignants par diplôme le plus élevé.
fn personnel_statut(d: &DonneesStat) -> Feuille {
    let membres = personnel(d);
    let mut b = Bloc::nouveau(
        "",
        &["Statut", "Hommes", "Femmes", "Total", "Dont tenant classe"],
    );
    let mut total = Sexes::default();
    let mut total_tenant = 0;
    for s in StatutRapport::TOUS {
        let mut n = Sexes::default();
        let mut tenant = 0;
        for (m, tient) in membres.iter().filter(|(m, _)| statut(d, m) == *s) {
            n.ajouter(m.sexe.as_deref());
            total.ajouter(m.sexe.as_deref());
            if *tient {
                tenant += 1;
                total_tenant += 1;
            }
        }
        let [h, f, t] = n.cases();
        b.lignes
            .push(vec![titre_statut(*s).to_owned(), h, f, t, texte(tenant)]);
    }
    let [h, f, t] = total.cases();
    b.lignes
        .push(vec!["Total".to_owned(), h, f, t, texte(total_tenant)]);

    let mut c = Bloc::nouveau(
        "Enseignants par diplôme le plus élevé",
        &["Diplôme", "Hommes", "Femmes", "Total"],
    );
    let enseignants: Vec<&MembreStat> = membres
        .iter()
        .filter(|(_, tient)| *tient)
        .map(|(m, _)| *m)
        .collect();
    for dip in DiplomeEleve::TOUS {
        let mut n = Sexes::default();
        for m in enseignants.iter().filter(|m| diplome(d, m) == Some(*dip)) {
            n.ajouter(m.sexe.as_deref());
        }
        let [h, f, t] = n.cases();
        c.lignes.push(vec![titre_diplome(*dip).to_owned(), h, f, t]);
    }
    let mut tous = Sexes::default();
    for m in &enseignants {
        tous.ajouter(m.sexe.as_deref());
    }
    let [h, f, t] = tous.cases();
    c.lignes.push(vec!["Total".to_owned(), h, f, t]);
    feuille("VI.1 Personnel", vec![b, c])
}

/// Enseignants d'une discipline et ses minutes hebdomadaires, si l'école tient l'emploi du temps.
type Discipline<'a> = (BTreeSet<&'a str>, Option<i64>);

/// Partie VI.2 : enseignants et heures hebdomadaires par discipline.
fn disciplines(d: &DonneesStat) -> Feuille {
    let secondaires: HashSet<&str> = d
        .classes
        .iter()
        .filter(|c| du_secondaire(&c.cycle))
        .map(|c| c.id.as_str())
        .collect();
    let mut matieres: BTreeMap<(i64, String), Discipline<'_>> = BTreeMap::new();
    for a in d
        .affectations
        .iter()
        .filter(|a| secondaires.contains(a.classe_id.as_str()))
    {
        matieres
            .entry((a.matiere_ordre, a.matiere.clone()))
            .or_default()
            .0
            .insert(a.personne_id.as_str());
    }
    let tient_emploi = d
        .seances
        .iter()
        .any(|s| secondaires.contains(s.classe_id.as_str()));
    for s in d
        .seances
        .iter()
        .filter(|s| secondaires.contains(s.classe_id.as_str()))
    {
        let e = matieres
            .entry((s.matiere_ordre, s.matiere.clone()))
            .or_default();
        e.1 = Some(e.1.unwrap_or(0) + s.minutes);
    }
    let mut b = Bloc::nouveau("", &["Discipline", "Enseignants", "Heures hebdomadaires"]);
    for ((_, matiere), (personnes, minutes)) in matieres {
        let heures = if tient_emploi {
            heures(minutes.unwrap_or(0))
        } else {
            String::new()
        };
        b.lignes.push(vec![matiere, texte(personnes.len()), heures]);
    }
    feuille("VI.2 Disciplines", vec![b])
}

/// Bloc par âge et sexe d'une liste d'élèves ; le total compte aussi ceux dont l'âge ou le sexe
/// n'est pas noté.
fn par_age(d: &DonneesStat, titre: &str, eleves: &[&Retenue<'_>]) -> Bloc {
    let mut b = Bloc::nouveau(titre, &["Âge", "Garçons", "Filles", "Total"]);
    let mut ages: BTreeMap<i32, Sexes> = BTreeMap::new();
    let mut total = Sexes::default();
    for r in eleves {
        let sexe = r.inscription.sexe.as_deref();
        if let Some(a) = age(d, r) {
            ages.entry(a).or_default().ajouter(sexe);
        }
        total.ajouter(sexe);
    }
    for (a, s) in ages {
        let [g, f, t] = s.cases();
        b.lignes.push(vec![format!("{a} ans"), g, f, t]);
    }
    let [g, f, t] = total.cases();
    b.lignes.push(vec!["Total".to_owned(), g, f, t]);
    b
}

/// Partie VII : nouveaux inscrits en première année du secondaire, par âge et sexe.
fn nouveaux(d: &DonneesStat, eleves: &[Retenue<'_>]) -> Feuille {
    // Première année de chaque cycle du secondaire : le niveau de plus petit ordre.
    let mut premiers: HashMap<&str, i64> = HashMap::new();
    for r in eleves {
        let e = premiers
            .entry(r.classe.cycle.as_str())
            .or_insert(r.classe.ordre);
        *e = (*e).min(r.classe.ordre);
    }
    let retenus: Vec<&Retenue<'_>> = eleves
        .iter()
        .filter(|r| premiers.get(r.classe.cycle.as_str()) == Some(&r.classe.ordre))
        .filter(|r| !r.inscription.redoublant)
        .collect();
    feuille("VII. Nouveaux inscrits", vec![par_age(d, "", &retenus)])
}

/// Partie VIII : élèves par année d'études et série, groupes pédagogiques, redoublants, prise en
/// charge par l'État ; puis par âge.
fn eleves_par_groupe(d: &DonneesStat, eleves: &[Retenue<'_>]) -> Feuille {
    let mut b = Bloc::nouveau(
        "",
        &[
            "Année d'études",
            "Série",
            "Groupes pédagogiques",
            "Garçons",
            "Filles",
            "Total",
            "Redoublants garçons",
            "Redoublants filles",
            "Redoublants total",
            "Pris en charge par l'État",
        ],
    );
    let mut groupes: BTreeMap<Groupe, (Sexes, Sexes, usize)> = BTreeMap::new();
    for c in d
        .classes
        .iter()
        .filter(|c| !c.supprimee && du_secondaire(&c.cycle))
    {
        groupes
            .entry(Groupe {
                ordre: c.ordre,
                niveau: c.niveau.clone(),
                serie: c.serie.clone().unwrap_or_default(),
            })
            .or_default();
    }
    for r in eleves {
        let e = groupes.entry(groupe_de(r)).or_default();
        let sexe = r.inscription.sexe.as_deref();
        e.0.ajouter(sexe);
        if r.inscription.redoublant {
            e.1.ajouter(sexe);
        }
        if d.complements
            .pris_en_charge
            .contains(&r.inscription.eleve_id)
        {
            e.2 += 1;
        }
    }
    let classes_du = |g: &Groupe| {
        d.classes
            .iter()
            .filter(|c| !c.supprimee && du_secondaire(&c.cycle))
            .filter(|c| c.ordre == g.ordre && c.niveau == g.niveau)
            .filter(|c| c.serie.clone().unwrap_or_default() == g.serie)
            .count()
    };
    let (mut tous, mut redoublants, mut pec, mut nb_classes) =
        (Sexes::default(), Sexes::default(), 0, 0);
    for (g, (s, r, p)) in &groupes {
        let [eg, ef, et] = s.cases();
        let [rg, rf, rt] = r.cases();
        let n = classes_du(g);
        nb_classes += n;
        tous.t += s.t;
        tous.g += s.g;
        tous.f += s.f;
        redoublants.t += r.t;
        redoublants.g += r.g;
        redoublants.f += r.f;
        pec += p;
        b.lignes.push(vec![
            g.niveau.clone(),
            g.serie.clone(),
            texte(n),
            eg,
            ef,
            et,
            rg,
            rf,
            rt,
            texte(*p),
        ]);
    }
    let [eg, ef, et] = tous.cases();
    let [rg, rf, rt] = redoublants.cases();
    b.lignes.push(vec![
        "Total".to_owned(),
        String::new(),
        texte(nb_classes),
        eg,
        ef,
        et,
        rg,
        rf,
        rt,
        texte(pec),
    ]);

    // Par âge : une colonne garçons et une colonne filles par groupe.
    let noms: Vec<Groupe> = groupes.keys().cloned().collect();
    let mut entetes = vec!["Âge".to_owned()];
    for g in &noms {
        entetes.push(format!("{} garçons", g.libelle()));
        entetes.push(format!("{} filles", g.libelle()));
    }
    entetes.push("Total".to_owned());
    let mut c = Bloc {
        titre: "Élèves par âge".to_owned(),
        entetes,
        lignes: Vec::new(),
    };
    let mut ages: BTreeMap<i32, Vec<Sexes>> = BTreeMap::new();
    for r in eleves {
        let Some(a) = age(d, r) else { continue };
        let Some(k) = noms.iter().position(|g| *g == groupe_de(r)) else {
            continue;
        };
        let cases = ages
            .entry(a)
            .or_insert_with(|| vec![Sexes::default(); noms.len()]);
        if let Some(x) = cases.get_mut(k) {
            x.ajouter(r.inscription.sexe.as_deref());
        }
    }
    for (a, cases) in ages {
        let mut l = vec![format!("{a} ans")];
        for s in &cases {
            l.push(texte(s.g));
            l.push(texte(s.f));
        }
        l.push(texte(cases.iter().map(|s| s.t).sum()));
        c.lignes.push(l);
    }
    feuille("VIII. Élèves", vec![b, c])
}

/// Partie IX : élèves handicapés par type, degré et sexe, sans nom ; à la main pour qui n'a pas
/// le droit de les lire.
fn handicap(
    d: &DonneesStat,
    eleves: &[Retenue<'_>],
) -> (Feuille, Option<Vec<(TypeDifficulte, Sexes)>>) {
    let Some(liste) = d.complements.difficultes.as_ref() else {
        let mut b = Bloc::nouveau("", &["Rubrique", "Valeur"]);
        b.lignes.push(ligne(&[
            "Élèves handicapés par type, degré, série et sexe",
            A_LA_MAIN,
        ]));
        return (feuille("IX. Élèves handicapés", vec![b]), None);
    };
    let sexes: HashMap<&str, Option<&str>> = eleves
        .iter()
        .map(|r| {
            (
                r.inscription.eleve_id.as_str(),
                r.inscription.sexe.as_deref(),
            )
        })
        .collect();
    let mut b = Bloc::nouveau("", &["Type", "Degré", "Garçons", "Filles", "Total"]);
    let mut par_type: Vec<(TypeDifficulte, Sexes)> = Vec::new();
    let mut total = Sexes::default();
    for t in TypeDifficulte::TOUS {
        let mut du_type = Sexes::default();
        let degres = DegreDifficulte::TOUS
            .iter()
            .copied()
            .map(Some)
            .chain([None]);
        for degre in degres {
            let mut n = Sexes::default();
            for x in liste
                .iter()
                .filter(|x| x.type_difficulte == *t && x.degre == degre)
            {
                if let Some(sexe) = sexes.get(x.eleve_id.as_str()) {
                    n.ajouter(*sexe);
                    du_type.ajouter(*sexe);
                    total.ajouter(*sexe);
                }
            }
            if n.t > 0 {
                let [g, f, tt] = n.cases();
                b.lignes.push(vec![
                    titre_type(*t).to_owned(),
                    titre_degre(degre).to_owned(),
                    g,
                    f,
                    tt,
                ]);
            }
        }
        par_type.push((*t, du_type));
    }
    let [g, f, t] = total.cases();
    b.lignes
        .push(vec!["Total".to_owned(), String::new(), g, f, t]);
    (feuille("IX. Élèves handicapés", vec![b]), Some(par_type))
}

/// Partie X : obstacles physiques et matériel adapté.
fn accessibilite(d: &DonneesStat, a: &Annexes<'_>) -> Feuille {
    let mut b = Bloc::nouveau("", &["Rubrique", "Valeur"]);
    b.lignes
        .extend(rubriques(d, a, &[ThemeRubrique::Accessibilite]));
    let c = bloc_quantites(d, "Matériel adapté", &[FamilleInventaire::Adapte]);
    feuille("X. Accessibilité", vec![b, c])
}

/// Partie XI : ressources humaines pour le handicap, que GESCO ne tient pas.
fn ressources_handicap() -> Feuille {
    let mut b = Bloc::nouveau("", &["Rubrique", "Valeur"]);
    for r in [
        "Enseignants handicapés",
        "Enseignants formés à l'éducation inclusive",
        "Spécialistes",
        "Enseignants itinérants",
        "Consultants",
    ] {
        b.lignes.push(ligne(&[r, A_LA_MAIN]));
    }
    feuille("XI. Handicap, personnel", vec![b])
}

/// Partie XII : élèves de la dernière année de l'année précédente, dont admis, dont handicapés.
fn resultats(a: &Annexes<'_>) -> Feuille {
    let mut b = Bloc::nouveau(
        "",
        &["Année d'études", "Élèves", "Dont admis", "Dont handicapés"],
    );
    if let Some(p) = a.precedente {
        let eleves = eleves_presents(p);
        let mut derniers: HashMap<&str, i64> = HashMap::new();
        for r in &eleves {
            let e = derniers
                .entry(r.classe.cycle.as_str())
                .or_insert(r.classe.ordre);
            *e = (*e).max(r.classe.ordre);
        }
        let mut groupes: BTreeMap<Groupe, Vec<&Retenue<'_>>> = BTreeMap::new();
        for r in eleves
            .iter()
            .filter(|r| derniers.get(r.classe.cycle.as_str()) == Some(&r.classe.ordre))
        {
            groupes.entry(groupe_de(r)).or_default().push(r);
        }
        let handicapes: Option<HashSet<&str>> = p
            .complements
            .difficultes
            .as_ref()
            .map(|l| l.iter().map(|x| x.eleve_id.as_str()).collect());
        for (g, liste) in groupes {
            let resultats: Vec<_> = liste
                .iter()
                .filter_map(|r| {
                    p.resultats
                        .iter()
                        .find(|x| x.inscription_id == r.inscription.inscription_id)
                })
                .collect();
            let admis = if resultats.iter().any(|x| x.admis.is_some()) {
                texte(resultats.iter().filter(|x| x.admis == Some(true)).count())
            } else {
                String::new()
            };
            let dont_handicapes = handicapes
                .as_ref()
                .map(|h| {
                    texte(
                        liste
                            .iter()
                            .filter(|r| h.contains(r.inscription.eleve_id.as_str()))
                            .count(),
                    )
                })
                .unwrap_or_default();
            b.lignes.push(vec![
                g.libelle(),
                texte(liste.len()),
                admis,
                dont_handicapes,
            ]);
        }
    }
    feuille("XII. Résultats scolaires", vec![b])
}

/// Partie XIII : observations et signatures.
fn observations() -> Feuille {
    let mut b = Bloc::nouveau("", &["Rubrique", "Valeur"]);
    b.lignes.push(ligne(&["Observations", A_LA_MAIN]));
    b.lignes
        .push(ligne(&["Date, signature et cachet", A_LA_MAIN]));
    feuille("XIII. Observations", vec![b])
}

/// Les feuilles du classeur, dans l'ordre du questionnaire.
pub(crate) fn feuilles(d: &DonneesStat, a: &Annexes<'_>) -> Vec<Feuille> {
    let eleves = eleves_presents(d);
    let (ix, par_type) = handicap(d, &eleves);
    vec![
        identification(d, a),
        generales(d, a, par_type.as_deref()),
        locaux(d),
        mobilier(d),
        financieres(a),
        personnel_statut(d),
        disciplines(d),
        nouveaux(d, &eleves),
        eleves_par_groupe(d, &eleves),
        ix,
        accessibilite(d, a),
        ressources_handicap(),
        resultats(a),
        observations(),
    ]
}

/// Service du classeur « Données pour l'enquête ».
pub struct ServiceEnquete<'n> {
    noyau: &'n Noyau,
}

impl<'n> ServiceEnquete<'n> {
    /// Service sur le noyau ouvert.
    pub const fn nouveau(noyau: &'n Noyau) -> Self {
        Self { noyau }
    }

    /// Écrit le classeur. Une école sans année, sans élève, sans inventaire a ses feuilles,
    /// avec des cases vides : jamais un refus.
    pub async fn exporter(
        &self,
        session: &Session,
        demande: DemandeEnquete,
    ) -> Resultat<FichierEcrit> {
        let stats = crate::statistiques::ServiceStatistiques::nouveau(self.noyau);
        let d = stats
            .charger(
                session,
                &DemandeStatistiques {
                    annee_id: demande.annee_id,
                    date: demande.date,
                    cycle: None,
                },
            )
            .await?;
        let precedente_id = d.choisie.as_ref().and_then(|c| {
            d.annees
                .iter()
                .filter(|a| a.debut.millis_epoque() < c.debut.millis_epoque())
                .max_by_key(|a| a.debut.millis_epoque())
                .map(|a| a.id)
        });
        let precedente = match precedente_id {
            Some(id) => Some(
                stats
                    .charger(
                        session,
                        &DemandeStatistiques {
                            annee_id: Some(id),
                            date: None,
                            cycle: None,
                        },
                    )
                    .await?,
            ),
            None => None,
        };
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self
            .noyau
            .base()
            .pool()
            .acquire()
            .await
            .map_err(gesco_db::ErreurBase::from)?;
        let ecole = etablissement::lire(&mut cnx, session.etablissement_id).await?;
        let rubriques = fiche::rubriques(&mut cnx, &contexte).await?;
        let valeurs = fiche::valeurs(&mut cnx, &contexte).await?;
        let frais = match &d.choisie {
            Some(a) => statistiques::frais_scolarite(&mut cnx, a.id, &contexte).await?,
            None => Vec::new(),
        };
        let frais_avant = match precedente_id {
            Some(id) => statistiques::frais_scolarite(&mut cnx, id, &contexte).await?,
            None => Vec::new(),
        };
        drop(cnx);
        let annexes = Annexes {
            ecole: &ecole,
            rubriques: &rubriques,
            valeurs: &valeurs,
            frais: (&frais, &frais_avant),
            precedente: precedente.as_ref(),
        };
        let titre = match &d.choisie {
            Some(a) => format!("Données pour l'enquête {}", a.libelle),
            None => "Données pour l'enquête".to_owned(),
        };
        exporter_xlsx_feuilles(demande.chemin, titre, feuilles(&d, &annexes)).await
    }
}

#[cfg(test)]
mod essais {
    use super::*;

    #[test]
    fn une_feuille_aligne_ses_tableaux_sur_le_plus_large() {
        let mut a = Bloc::nouveau("", &["Rubrique", "Valeur"]);
        a.lignes.push(ligne(&["Nom", "École"]));
        let mut b = Bloc::nouveau("Second", &["Âge", "Garçons", "Filles", "Total"]);
        b.lignes.push(ligne(&["15 ans", "1", "2", "3"]));
        let f = feuille("Essai", vec![a, b]);
        assert_eq!(f.entetes, ligne(&["Rubrique", "Valeur", "", ""]));
        assert_eq!(f.lignes.len(), 5);
        assert!(f.lignes.iter().all(|l| l.len() == 4));
        assert_eq!(
            f.lignes.get(2).and_then(|l| l.first()).map(String::as_str),
            Some("Second")
        );
    }

    #[test]
    fn les_montants_et_les_heures_se_lisent() {
        assert_eq!(francs(125_000), "125\u{a0}000");
        assert_eq!(francs(900), "900");
        assert_eq!(heures(90), "1 h 30");
        assert_eq!(heures(180), "3 h");
    }
}
