//! Tableaux des rapports de rentrée et de fermeture (SPEC_PHASE_9 §3, §4.2, §5.2), calculés à la date d'arrêt sur les
//! mêmes données que les statistiques. Ils reprennent les intitulés et l'ordre des modèles. Une
//! donnée absente donne une case vide ; zéro ne s'écrit que lorsque l'application sait qu'il n'y
//! en a pas (règle 1). Rien n'y dit qu'une donnée manque (règle 3).

use std::collections::{BTreeMap, HashMap};

use gesco_core::core::complements::{
    DiplomeEleve, Fonctionnement, PedagogieClasse, Specialite, StatutRapport,
};
use gesco_core::core::inventaire::CODE_LATRINES;
use gesco_core::core::{ArticleInventaire, Examen, FamilleInventaire, QuantitesInventaire};
use gesco_core::scolarite::Cycle;
use gesco_db::depots::complements::ClasseComplements;
use gesco_db::depots::examens::{ResultatExamen, TotalExamen};
use gesco_db::depots::rapports::EffectifProbable;
use gesco_db::depots::statistiques::{ClasseStat, MembreStat};
use serde::{Deserialize, Serialize};

use crate::statistiques::{DonneesStat, Retenue, date, dernier_diplome, engagement_le, retenir};

/// Un tableau d'un rapport, prêt à montrer, imprimer et exporter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct TableauRapport {
    /// Code stable.
    pub code: String,
    /// Titre, tel que le modèle le nomme.
    pub titre: String,
    /// En-têtes des colonnes ; la première est celle des libellés.
    pub colonnes: Vec<String>,
    /// Lignes, chacune aussi large que les en-têtes ; une case vide est une chaîne vide.
    pub lignes: Vec<Vec<String>>,
    /// Vrai pour un tableau large, imprimé en paysage.
    pub paysage: bool,
    /// Écran où se corrige la donnée : `statistiques` ou `inventaire`.
    pub lien: String,
}

/// Ce que les tableaux lisent en plus des données des statistiques.
pub(crate) struct Annexes<'a> {
    /// Compléments des classes de l'année : fonctionnement et pédagogie saisis.
    pub(crate) classes: &'a [ClasseComplements],
    /// Fonctionnement et pédagogie par défaut, d'après la fiche de l'établissement.
    pub(crate) defauts: (Fonctionnement, PedagogieClasse),
}

/// Cycles du fondamental, dans l'ordre des modèles.
pub(crate) const FONDAMENTAL: [Cycle; 2] = [Cycle::PremierCycle, Cycle::SecondCycle];

fn titre_cycle(c: Cycle) -> &'static str {
    match c {
        Cycle::PremierCycle => "Premier cycle",
        Cycle::SecondCycle => "Second cycle",
        _ => "",
    }
}

/// Ajoute un à la case `k`, si elle existe.
fn compter(cases: &mut [usize], k: usize) {
    if let Some(c) = cases.get_mut(k) {
        *c += 1;
    }
}

/// Ajoute une fiche à la case `k` d'une ligne par sexe, si elle existe.
fn ajouter_a(cases: &mut [Sexes], k: usize, sexe: Option<&str>) {
    if let Some(c) = cases.get_mut(k) {
        c.ajouter(sexe);
    }
}

/// Additionne deux lignes par sexe, case par case.
fn additionner(a: &mut [Sexes], b: &[Sexes]) {
    for (x, y) in a.iter_mut().zip(b) {
        *x = x.plus(*y);
    }
}

fn texte(n: usize) -> String {
    n.to_string()
}

fn optionnel(n: Option<u32>) -> String {
    n.map(|x| x.to_string()).unwrap_or_default()
}

fn colonnes(c: &[&str]) -> Vec<String> {
    c.iter().map(|x| (*x).to_owned()).collect()
}

/// Un niveau du fondamental : ses classes de l'année.
struct NiveauRapport<'a> {
    id: String,
    libelle: String,
    cycle: Cycle,
    classes: Vec<&'a ClasseStat>,
}

/// Les niveaux du fondamental de l'école, dans leur ordre : ceux qui sont en service et ceux qui
/// ont une classe cette année.
fn niveaux<'a>(d: &'a DonneesStat) -> Vec<NiveauRapport<'a>> {
    d.niveaux
        .iter()
        .filter(|n| FONDAMENTAL.contains(&n.cycle))
        .filter_map(|n| {
            let id = n.id.vers_chaine();
            let classes: Vec<&ClasseStat> = d
                .classes
                .iter()
                .filter(|c| c.niveau_id == id && !c.supprimee)
                .collect();
            (n.actif || !classes.is_empty()).then(|| NiveauRapport {
                id,
                libelle: n.libelle.clone(),
                cycle: n.cycle,
                classes,
            })
        })
        .collect()
}

/// Élèves comptés à une date : garçons, filles, total (le total compte aussi ceux dont le sexe
/// n'est pas noté).
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

    fn plus(self, o: Self) -> Self {
        Self {
            g: self.g + o.g,
            f: self.f + o.f,
            t: self.t + o.t,
        }
    }

    fn cases(self, connu: bool) -> [String; 3] {
        if connu {
            [texte(self.g), texte(self.f), texte(self.t)]
        } else {
            [String::new(), String::new(), String::new()]
        }
    }
}

/// Les élèves du fondamental retenus à la date (présents, et partis depuis la rentrée).
fn eleves<'a>(
    d: &'a DonneesStat,
    perimetre: &crate::statistiques::Perimetre<'a>,
) -> Vec<Retenue<'a>> {
    retenir(perimetre, &d.inscriptions)
        .into_iter()
        .filter(|r| Cycle::depuis_nom(&r.classe.cycle).is_some_and(|c| FONDAMENTAL.contains(&c)))
        .collect()
}

/// Les membres du personnel présents à la date et les cycles du fondamental où ils enseignent.
fn maitres(d: &DonneesStat) -> Vec<(&MembreStat, Vec<Cycle>)> {
    d.membres
        .iter()
        .filter(|m| date(m.arrive_le.as_deref()).is_none_or(|a| a <= d.date))
        .filter(|m| date(m.parti_le.as_deref()).is_none_or(|x| x > d.date))
        .filter_map(|m| {
            let mut cycles: Vec<Cycle> = d
                .affectations
                .iter()
                .filter(|a| a.personne_id == m.personne_id)
                .filter_map(|a| d.classes.iter().find(|c| c.id == a.classe_id))
                .filter_map(|c| Cycle::depuis_nom(&c.cycle))
                .filter(|c| FONDAMENTAL.contains(c))
                .collect();
            cycles.sort_by_key(|c| FONDAMENTAL.iter().position(|x| x == c));
            cycles.dedup();
            (!cycles.is_empty()).then_some((m, cycles))
        })
        .collect()
}

pub(crate) fn statut(d: &DonneesStat, m: &MembreStat) -> StatutRapport {
    d.complements
        .membres
        .get(&m.id)
        .and_then(|x| x.statut)
        .unwrap_or_else(|| {
            StatutRapport::deduit(
                m.fonctionnaire,
                engagement_le(&d.engagements, &m.id, d.date).map(|e| e.nature.as_str()),
            )
        })
}

pub(crate) fn specialite(d: &DonneesStat, m: &MembreStat) -> Specialite {
    d.complements
        .membres
        .get(&m.id)
        .and_then(|x| x.specialite)
        .or_else(|| {
            Specialite::proposee(
                d.affectations
                    .iter()
                    .filter(|a| a.personne_id == m.personne_id)
                    .filter(|a| {
                        d.classes
                            .iter()
                            .any(|c| c.id == a.classe_id && c.cycle == "second_cycle")
                    })
                    .map(|a| a.matiere.as_str()),
            )
        })
        .unwrap_or(Specialite::Autre)
}

/// Relevé d'inventaire d'un article, additionné sur les lignes d'une partie de l'école (le
/// fondamental entier : lignes de toute l'école et des deux cycles) ; rien si aucune ligne.
fn releve(
    d: &DonneesStat,
    article: &ArticleInventaire,
    partie: Option<Cycle>,
) -> Option<(QuantitesInventaire, Vec<String>)> {
    releve_si(d, article, |cycle| match partie {
        Some(c) => cycle == Some(c),
        None => cycle.is_none_or(|c| FONDAMENTAL.contains(&c)),
    })
}

/// Relevé d'inventaire d'un article, additionné sur les lignes dont le cycle est retenu.
pub(crate) fn releve_si(
    d: &DonneesStat,
    article: &ArticleInventaire,
    retenir: impl Fn(Option<Cycle>) -> bool,
) -> Option<(QuantitesInventaire, Vec<String>)> {
    let mut somme: Option<QuantitesInventaire> = None;
    let mut observations = Vec::new();
    for r in d.releves.iter().filter(|r| r.article_id == article.id) {
        if !retenir(r.cycle) {
            continue;
        }
        somme = Some(match somme {
            Some(s) => s.additionner(&r.quantites),
            None => r.quantites.clone(),
        });
        if !r.observation.is_empty() {
            observations.push(r.observation.clone());
        }
    }
    somme.map(|s| (s, observations))
}

pub(crate) fn article<'a>(d: &'a DonneesStat, code: &str) -> Option<&'a ArticleInventaire> {
    d.articles.iter().find(|a| a.code.as_deref() == Some(code))
}

/// Somme d'une colonne ; vide si aucune valeur n'est donnée.
fn somme(valeurs: impl IntoIterator<Item = Option<u32>>) -> Option<u32> {
    valeurs
        .into_iter()
        .flatten()
        .fold(None, |acc: Option<u32>, v| {
            Some(acc.unwrap_or(0).saturating_add(v))
        })
}

/// Salles de classe en dur, semi-dur, banco, autres d'une partie : rien si aucun relevé.
fn salles(d: &DonneesStat, partie: Option<Cycle>) -> Option<QuantitesInventaire> {
    article(d, "salle_classe").and_then(|a| releve(d, a, partie).map(|(q, _)| q))
}

/// Les tableaux du rapport succinct de rentrée (SPEC_PHASE_9 §3).
pub(crate) fn succinct(d: &DonneesStat) -> Vec<TableauRapport> {
    let perimetre = d.perimetre(None);
    let retenues = eleves(d, &perimetre);
    let niveaux = niveaux(d);
    let mut lignes = Vec::new();
    for cycle in FONDAMENTAL {
        let mut total_e = Sexes::default();
        let mut total_r = Sexes::default();
        let mut groupes = 0;
        let mut connu = false;
        for n in niveaux.iter().filter(|n| n.cycle == cycle) {
            let (e, r) = effectifs(&retenues, &n.id);
            let a_des_classes = !n.classes.is_empty();
            connu |= a_des_classes;
            groupes += n.classes.len();
            total_e = total_e.plus(e);
            total_r = total_r.plus(r);
            let mut l = vec![
                n.libelle.clone(),
                if a_des_classes {
                    texte(n.classes.len())
                } else {
                    String::new()
                },
            ];
            l.extend(e.cases(a_des_classes));
            l.extend(r.cases(a_des_classes));
            lignes.push(l);
        }
        let mut l = vec![
            format!("Total {}", titre_cycle(cycle).to_lowercase()),
            if connu { texte(groupes) } else { String::new() },
        ];
        l.extend(total_e.cases(connu));
        l.extend(total_r.cases(connu));
        lignes.push(l);
    }
    let tableau2 = TableauRapport {
        code: "groupes_eleves_redoublants".to_owned(),
        titre: "Groupes pédagogiques, élèves et redoublants par année d'études".to_owned(),
        colonnes: colonnes(&[
            "Année d'études",
            "Groupes pédagogiques",
            "Élèves G",
            "Élèves F",
            "Élèves T",
            "Redoublants G",
            "Redoublants F",
            "Redoublants T",
        ]),
        lignes,
        paysage: false,
        lien: "statistiques".to_owned(),
    };

    let maitres = maitres(d);
    let mut lignes3 = Vec::new();
    for cycle in FONDAMENTAL {
        let mut c = [0usize; 4];
        for (m, cycles) in &maitres {
            if !cycles.contains(&cycle) {
                continue;
            }
            let base = if statut(d, m).permanent() { 0 } else { 2 };
            match m.sexe.as_deref() {
                Some("masculin") => compter(&mut c, base),
                Some("feminin") => compter(&mut c, base + 1),
                _ => {}
            }
        }
        let s = salles(d, Some(cycle));
        let mut l = vec![titre_cycle(cycle).to_owned()];
        l.extend(c.iter().map(|x| texte(*x)));
        l.push(optionnel(s.as_ref().and_then(|q| q.dur)));
        l.push(optionnel(s.as_ref().and_then(|q| q.semi_dur)));
        l.push(optionnel(
            s.as_ref()
                .and_then(|q| somme([q.banco, q.autre_construction])),
        ));
        lignes3.push(l);
    }
    let tableau3 = TableauRapport {
        code: "maitres_salles".to_owned(),
        titre: "Maîtres et salles de classe par cycle".to_owned(),
        colonnes: colonnes(&[
            "Cycle",
            "Permanents H",
            "Permanents F",
            "Contractuels H",
            "Contractuels F",
            "Salles en dur",
            "Salles semi-dur",
            "Autres salles",
        ]),
        lignes: lignes3,
        paysage: false,
        lien: "statistiques".to_owned(),
    };
    vec![tableau2, tableau3]
}

/// Élèves présents d'un niveau et redoublants parmi eux.
fn effectifs(retenues: &[Retenue<'_>], niveau_id: &str) -> (Sexes, Sexes) {
    let mut e = Sexes::default();
    let mut r = Sexes::default();
    for x in retenues
        .iter()
        .filter(|x| x.presente && x.classe.niveau_id == niveau_id)
    {
        let sexe = x.inscription.sexe.as_deref();
        e.ajouter(sexe);
        if x.inscription.redoublant {
            r.ajouter(sexe);
        }
    }
    (e, r)
}

/// Libellé d'un statut dans les tableaux du rapport de rentrée.
pub(crate) fn titre_statut(s: StatutRapport) -> &'static str {
    match s {
        StatutRapport::FonctionnaireEtat => "Fonctionnaire de l'État",
        StatutRapport::FonctionnaireCollectivites => "Fonctionnaire des collectivités",
        StatutRapport::ContractuelEtat => "Contractuel de l'État",
        StatutRapport::ContractuelCollectivites => "Contractuel des collectivités",
        StatutRapport::ContractuelEcole => "Contractuel local",
        StatutRapport::Vacataire => "Vacataire",
        StatutRapport::EleveMaitre => "Élève-maître",
        StatutRapport::Autre => "Autre",
    }
}

fn titre_specialite(s: Specialite) -> &'static str {
    match s {
        Specialite::Lhg => "LHG",
        Specialite::Mpc => "MPC",
        Specialite::Snpc => "SNPC",
        Specialite::Ldm => "LDM",
        Specialite::Eps => "EPS",
        Specialite::Autre => "Autres",
    }
}

/// Les tableaux du rapport de rentrée (SPEC_PHASE_9 §4.2).
pub(crate) fn rentree(d: &DonneesStat, annexes: &Annexes<'_>) -> Vec<TableauRapport> {
    let perimetre = d.perimetre(None);
    let retenues = eleves(d, &perimetre);
    let niveaux = niveaux(d);
    let maitres = maitres(d);
    let mut tableaux = Vec::new();

    // Maîtres du premier cycle par statut.
    let mut lignes = Vec::new();
    let mut total = Sexes::default();
    for s in StatutRapport::TOUS {
        let mut x = Sexes::default();
        for (m, cycles) in &maitres {
            if cycles.contains(&Cycle::PremierCycle) && statut(d, m) == *s {
                x.ajouter(m.sexe.as_deref());
            }
        }
        total = total.plus(x);
        let mut l = vec![titre_statut(*s).to_owned()];
        l.extend(x.cases(true));
        lignes.push(l);
    }
    let mut l = vec!["Total".to_owned()];
    l.extend(total.cases(true));
    lignes.push(l);
    tableaux.push(TableauRapport {
        code: "maitres_premier_cycle".to_owned(),
        titre: "Maîtres du premier cycle par statut".to_owned(),
        colonnes: colonnes(&["Statut", "Hommes", "Femmes", "Total"]),
        lignes,
        paysage: false,
        lien: "statistiques".to_owned(),
    });

    // Maîtres du second cycle par spécialité et statut.
    let mut lignes = Vec::new();
    let mut totaux = vec![0usize; StatutRapport::TOUS.len()];
    let mut total = Sexes::default();
    for sp in Specialite::TOUS {
        let mut par_statut = vec![0usize; StatutRapport::TOUS.len()];
        let mut x = Sexes::default();
        for (m, cycles) in &maitres {
            if !cycles.contains(&Cycle::SecondCycle) || specialite(d, m) != *sp {
                continue;
            }
            if let Some(k) = StatutRapport::TOUS.iter().position(|s| *s == statut(d, m)) {
                compter(&mut par_statut, k);
                compter(&mut totaux, k);
            }
            x.ajouter(m.sexe.as_deref());
        }
        total = total.plus(x);
        let mut l = vec![titre_specialite(*sp).to_owned()];
        l.extend(par_statut.iter().map(|n| texte(*n)));
        l.extend(x.cases(true));
        lignes.push(l);
    }
    let mut l = vec!["Total".to_owned()];
    l.extend(totaux.iter().map(|n| texte(*n)));
    l.extend(total.cases(true));
    lignes.push(l);
    let mut cols = vec!["Spécialité".to_owned()];
    cols.extend(
        StatutRapport::TOUS
            .iter()
            .map(|s| titre_statut(*s).to_owned()),
    );
    cols.extend(colonnes(&["Hommes", "Femmes", "Total"]));
    tableaux.push(TableauRapport {
        code: "maitres_second_cycle".to_owned(),
        titre: "Maîtres du second cycle par spécialité et statut".to_owned(),
        colonnes: cols,
        lignes,
        paysage: true,
        lien: "statistiques".to_owned(),
    });

    // Élèves par année d'études : passants, redoublants, exclus, total.
    let mut lignes = Vec::new();
    let mut ensemble = [Sexes::default(); 4];
    for cycle in FONDAMENTAL {
        let mut du_cycle = [Sexes::default(); 4];
        let mut connu = false;
        for n in niveaux.iter().filter(|n| n.cycle == cycle) {
            let a_des_classes = !n.classes.is_empty();
            connu |= a_des_classes;
            let mut c = [Sexes::default(); 4];
            for x in retenues.iter().filter(|x| x.classe.niveau_id == n.id) {
                let sexe = x.inscription.sexe.as_deref();
                if x.presente {
                    ajouter_a(&mut c, usize::from(x.inscription.redoublant), sexe);
                    ajouter_a(&mut c, 3, sexe);
                } else if x.depart.is_some() && x.inscription.statut == "radiee" {
                    ajouter_a(&mut c, 2, sexe);
                }
            }
            additionner(&mut du_cycle, &c);
            let mut l = vec![n.libelle.clone()];
            for x in c {
                l.extend(x.cases(a_des_classes));
            }
            lignes.push(l);
        }
        let mut l = vec![format!("Total {}", titre_cycle(cycle).to_lowercase())];
        for x in du_cycle {
            l.extend(x.cases(connu));
        }
        lignes.push(l);
        additionner(&mut ensemble, &du_cycle);
    }
    let mut l = vec!["Total général".to_owned()];
    for x in ensemble {
        l.extend(x.cases(true));
    }
    lignes.push(l);
    let mut cols = vec!["Année d'études".to_owned()];
    for groupe in ["Passants", "Redoublants", "Exclus", "Total"] {
        for s in ["G", "F", "T"] {
            cols.push(format!("{groupe} {s}"));
        }
    }
    tableaux.push(TableauRapport {
        code: "eleves_par_annee".to_owned(),
        titre: "Élèves par année d'études et par sexe".to_owned(),
        colonnes: cols,
        lignes,
        paysage: true,
        lien: "statistiques".to_owned(),
    });

    // Élèves par année de recrutement et par année de naissance, par cycle.
    for cycle in FONDAMENTAL {
        let du_cycle: Vec<&NiveauRapport<'_>> =
            niveaux.iter().filter(|n| n.cycle == cycle).collect();
        if du_cycle.is_empty() {
            continue;
        }
        let recrutement = |r: &Retenue<'_>| {
            d.complements
                .annees_recrutement
                .get(&r.inscription.eleve_id)
                .copied()
        };
        let naissance =
            |r: &Retenue<'_>| date(r.inscription.date_naissance.as_deref()).map(|x| x.annee());
        tableaux.push(par_annee(
            &format!("recrutement_{}", cycle.nom()),
            &format!(
                "Élèves du {} par année d'études, sexe et année de recrutement",
                titre_cycle(cycle).to_lowercase()
            ),
            "Année de recrutement",
            &du_cycle,
            &retenues,
            recrutement,
        ));
        tableaux.push(par_annee(
            &format!("naissance_{}", cycle.nom()),
            &format!(
                "Élèves du {} par année d'études, sexe et année de naissance",
                titre_cycle(cycle).to_lowercase()
            ),
            "Année de naissance",
            &du_cycle,
            &retenues,
            naissance,
        ));
    }

    // Classes par cours, fonctionnement et pédagogie.
    let mut lignes = Vec::new();
    let mut totaux = [0usize; 7];
    for n in &niveaux {
        let mut c = [0usize; 7];
        for classe in &n.classes {
            let complement = annexes.classes.iter().find(|x| x.id == classe.id);
            let f = complement
                .and_then(|x| x.fonctionnement.as_deref())
                .and_then(Fonctionnement::depuis_nom)
                .unwrap_or(annexes.defauts.0);
            let p = complement
                .and_then(|x| x.pedagogie.as_deref())
                .and_then(PedagogieClasse::depuis_nom)
                .unwrap_or(annexes.defauts.1);
            compter(&mut c, 0);
            if let Some(k) = Fonctionnement::TOUS.iter().position(|x| *x == f) {
                compter(&mut c, 1 + k);
            }
            if let Some(k) = PedagogieClasse::TOUS.iter().position(|x| *x == p) {
                compter(&mut c, 4 + k);
            }
        }
        for (t, x) in totaux.iter_mut().zip(c) {
            *t += x;
        }
        let mut l = vec![n.libelle.clone()];
        l.extend(c.iter().map(|x| texte(*x)));
        lignes.push(l);
    }
    let mut l = vec!["Total".to_owned()];
    l.extend(totaux.iter().map(|x| texte(*x)));
    lignes.push(l);
    tableaux.push(TableauRapport {
        code: "classes_par_cours".to_owned(),
        titre: "Nombre de classes par cours, fonctionnement et pédagogie appliquée".to_owned(),
        colonnes: colonnes(&[
            "Cours",
            "Classes",
            "Simple",
            "Double division",
            "Double vacation",
            "Classique",
            "Pédagogie convergente",
            "Curriculum",
        ]),
        lignes,
        paysage: true,
        lien: "statistiques".to_owned(),
    });

    // Salles de classe par nature de construction.
    let mut lignes = Vec::new();
    for partie in [Some(Cycle::PremierCycle), Some(Cycle::SecondCycle), None] {
        let q = salles(d, partie);
        let libelle = partie.map_or("Ensemble", titre_cycle);
        let v = |f: fn(&QuantitesInventaire) -> Option<u32>| optionnel(q.as_ref().and_then(f));
        lignes.push(vec![
            libelle.to_owned(),
            v(|q| q.dur),
            v(|q| q.semi_dur),
            v(|q| q.banco),
            v(|q| q.autre_construction),
            v(|q| somme([q.dur, q.semi_dur, q.banco, q.autre_construction])),
            v(|q| q.besoins),
        ]);
    }
    tableaux.push(TableauRapport {
        code: "salles_de_classe".to_owned(),
        titre: "Salles de classe par nature de construction".to_owned(),
        colonnes: colonnes(&["", "Dur", "Semi-dur", "Banco", "Autres", "Total", "Besoins"]),
        lignes,
        paysage: false,
        lien: "inventaire".to_owned(),
    });
    let mut lignes = Vec::new();
    for (code, libelle) in [(CODE_LATRINES, "Toilettes"), ("point_eau", "Points d'eau")] {
        let q = article(d, code)
            .and_then(|a| releve(d, a, None))
            .map(|(q, _)| q);
        lignes.push(vec![
            libelle.to_owned(),
            optionnel(q.as_ref().and_then(|q| q.existant)),
            optionnel(q.as_ref().and_then(|q| q.besoins)),
        ]);
    }
    tableaux.push(TableauRapport {
        code: "toilettes_points_eau".to_owned(),
        titre: "Toilettes et points d'eau".to_owned(),
        colonnes: colonnes(&["", "Existant", "Besoins"]),
        lignes,
        paysage: false,
        lien: "inventaire".to_owned(),
    });

    tableaux.push(equipements(
        d,
        FamilleInventaire::Mobilier,
        "mobilier",
        "Mobilier scolaire",
    ));
    tableaux.push(equipements(
        d,
        FamilleInventaire::Materiel,
        "materiel",
        "Matériel didactique",
    ));
    tableaux.push(manuels(d, &niveaux));
    tableaux
}

/// Élèves d'un cycle par année (de recrutement ou de naissance), niveau et sexe ; les élèves
/// dont l'année n'est pas connue ne paraissent que dans le total.
fn par_annee(
    code: &str,
    titre: &str,
    premiere: &str,
    niveaux: &[&NiveauRapport<'_>],
    retenues: &[Retenue<'_>],
    annee: impl Fn(&Retenue<'_>) -> Option<i32>,
) -> TableauRapport {
    let presents: Vec<&Retenue<'_>> = retenues
        .iter()
        .filter(|r| r.presente && niveaux.iter().any(|n| n.id == r.classe.niveau_id))
        .collect();
    let mut par: BTreeMap<i32, Vec<&Retenue<'_>>> = BTreeMap::new();
    for r in &presents {
        if let Some(a) = annee(r) {
            par.entry(a).or_default().push(r);
        }
    }
    let ligne = |libelle: String, eleves: &[&Retenue<'_>]| {
        let mut l = vec![libelle];
        let mut total = Sexes::default();
        for n in niveaux {
            let mut x = Sexes::default();
            for r in eleves.iter().filter(|r| r.classe.niveau_id == n.id) {
                x.ajouter(r.inscription.sexe.as_deref());
            }
            total = total.plus(x);
            l.push(texte(x.g));
            l.push(texte(x.f));
        }
        l.extend(total.cases(true));
        l
    };
    let mut lignes: Vec<Vec<String>> = par
        .iter()
        .map(|(a, eleves)| ligne(a.to_string(), eleves))
        .collect();
    lignes.push(ligne("Total".to_owned(), &presents));
    let mut cols = vec![premiere.to_owned()];
    for n in niveaux {
        cols.push(format!("{} G", n.libelle));
        cols.push(format!("{} F", n.libelle));
    }
    cols.extend(colonnes(&["Total G", "Total F", "Total T"]));
    TableauRapport {
        code: code.to_owned(),
        titre: titre.to_owned(),
        colonnes: cols,
        lignes,
        paysage: true,
        lien: "statistiques".to_owned(),
    }
}

/// Mobilier ou matériel : les articles qui ont quelque chose de saisi pour le fondamental.
fn equipements(
    d: &DonneesStat,
    famille: FamilleInventaire,
    code: &str,
    titre: &str,
) -> TableauRapport {
    let mut lignes = Vec::new();
    let mut existant = Vec::new();
    let mut besoins = Vec::new();
    for a in d.articles.iter().filter(|a| a.famille == famille) {
        let Some((q, observations)) = releve(d, a, None) else {
            continue;
        };
        if q.est_vide() && observations.is_empty() {
            continue;
        }
        existant.push(q.existant);
        besoins.push(q.besoins);
        lignes.push(vec![
            a.designation.clone(),
            optionnel(q.existant),
            optionnel(q.besoins),
            observations.join(" ; "),
        ]);
    }
    lignes.push(vec![
        "Total".to_owned(),
        optionnel(somme(existant)),
        optionnel(somme(besoins)),
        String::new(),
    ]);
    TableauRapport {
        code: code.to_owned(),
        titre: titre.to_owned(),
        colonnes: colonnes(&["Nature", "Existant", "Besoins", "Observations"]),
        lignes,
        paysage: false,
        lien: "inventaire".to_owned(),
    }
}

/// Manuels disponibles : une ligne par désignation, la quantité dans la colonne du niveau du
/// manuel, les totaux par cycle.
fn manuels(d: &DonneesStat, niveaux: &[NiveauRapport<'_>]) -> TableauRapport {
    let niveau_de: HashMap<String, (usize, Cycle)> = d
        .niveaux
        .iter()
        .filter(|n| FONDAMENTAL.contains(&n.cycle))
        .filter_map(|n| {
            niveaux
                .iter()
                .position(|x| x.libelle == n.libelle && x.cycle == n.cycle)
                .map(|k| (n.id.vers_chaine(), (k, n.cycle)))
        })
        .collect();
    let mut par_designation: Vec<(String, Vec<Option<u32>>)> = Vec::new();
    for a in d
        .articles
        .iter()
        .filter(|a| a.famille == FamilleInventaire::Manuel)
    {
        let Some((k, _)) = a
            .niveau_id
            .and_then(|n| niveau_de.get(&n.vers_chaine()).copied())
        else {
            continue;
        };
        let Some(q) = releve(d, a, None).and_then(|(q, _)| q.existant) else {
            continue;
        };
        if !par_designation.iter().any(|(x, _)| *x == a.designation) {
            par_designation.push((a.designation.clone(), vec![None; niveaux.len()]));
        }
        if let Some(c) = par_designation
            .iter_mut()
            .find(|(x, _)| *x == a.designation)
            .and_then(|(_, l)| l.get_mut(k))
        {
            *c = Some(c.unwrap_or(0).saturating_add(q));
        }
    }
    let lignes = par_designation
        .into_iter()
        .map(|(designation, quantites)| {
            let mut l = vec![designation];
            l.extend(quantites.iter().map(|q| optionnel(*q)));
            for cycle in FONDAMENTAL {
                l.push(optionnel(somme(
                    niveaux
                        .iter()
                        .zip(&quantites)
                        .filter(|(n, _)| n.cycle == cycle)
                        .map(|(_, q)| *q),
                )));
            }
            l
        })
        .collect();
    let mut cols = vec!["Désignation".to_owned()];
    cols.extend(niveaux.iter().map(|n| n.libelle.clone()));
    cols.extend(colonnes(&["Total premier cycle", "Total second cycle"]));
    TableauRapport {
        code: "manuels".to_owned(),
        titre: "Manuels disponibles".to_owned(),
        colonnes: cols,
        lignes,
        paysage: niveaux.len() > 6,
        lien: "inventaire".to_owned(),
    }
}

/// Chiffres du jour pour l'introduction : enseignants et élèves du fondamental, filles parmi
/// eux.
pub(crate) fn chiffres_du_jour(d: &DonneesStat) -> (usize, usize, usize) {
    let perimetre = d.perimetre(None);
    let retenues = eleves(d, &perimetre);
    let presents: Vec<&Retenue<'_>> = retenues.iter().filter(|r| r.presente).collect();
    let filles = presents
        .iter()
        .filter(|r| r.inscription.sexe.as_deref() == Some("feminin"))
        .count();
    (maitres(d).len(), presents.len(), filles)
}

/// Ce que les tableaux du rapport de fermeture lisent en plus des données des statistiques.
pub(crate) struct AnnexesFermeture<'a> {
    /// Effectifs probables saisis dans le rapport, par niveau.
    pub(crate) probables: &'a [EffectifProbable],
}

/// Date AAAA-MM-JJ écrite JJ/MM/AAAA ; vide si elle n'est pas lisible.
pub(crate) fn date_courte(texte: Option<&str>) -> String {
    date(texte)
        .map(|d| {
            let s = d.to_string();
            match (s.get(0..4), s.get(5..7), s.get(8..10)) {
                (Some(a), Some(m), Some(j)) => format!("{j}/{m}/{a}"),
                _ => s,
            }
        })
        .unwrap_or_default()
}

fn titre_situation(s: Option<&str>) -> String {
    match s {
        Some("celibataire") => "Célibataire",
        Some("marie") => "Marié(e)",
        Some("veuf") => "Veuf(ve)",
        Some("divorce") => "Divorcé(e)",
        _ => "",
    }
    .to_owned()
}

pub(crate) fn titre_diplome(d: DiplomeEleve) -> &'static str {
    match d {
        DiplomeEleve::Doctorat => "Doctorat",
        DiplomeEleve::Master => "Master",
        DiplomeEleve::Dea => "DEA",
        DiplomeEleve::Ensup => "ENSup",
        DiplomeEleve::Ingenieur => "Ingénieur",
        DiplomeEleve::Maitrise => "Maîtrise",
        DiplomeEleve::Licence => "Licence",
        DiplomeEleve::DeugBts => "DEUG ou BTS",
        DiplomeEleve::Bt => "BT",
        DiplomeEleve::Baccalaureat => "Baccalauréat",
        DiplomeEleve::Cap => "CAP",
        DiplomeEleve::Ifm => "IFM",
        DiplomeEleve::Autre => "Autre",
    }
}

/// Liste nominative du personnel enseignant d'un cycle (SPEC_PHASE_9 §5.2) : une case vide
/// quand la fiche ne porte rien.
fn personnel_nominatif(d: &DonneesStat, cycle: Cycle) -> TableauRapport {
    let mut du_cycle: Vec<&MembreStat> = maitres(d)
        .into_iter()
        .filter(|(_, cycles)| cycles.contains(&cycle))
        .map(|(m, _)| m)
        .collect();
    du_cycle.sort_by(|a, b| (&a.nom, &a.prenoms).cmp(&(&b.nom, &b.prenoms)));
    let lignes = du_cycle
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let diplome = dernier_diplome(&m.diplomes)
                .or_else(|| {
                    d.complements
                        .membres
                        .get(&m.id)
                        .and_then(|x| x.diplome)
                        .map(|x| titre_diplome(x).to_owned())
                })
                .unwrap_or_default();
            vec![
                texte(i + 1),
                format!("{} {}", m.prenoms, m.nom).trim().to_owned(),
                m.matricule.trim().to_owned(),
                titre_situation(m.situation_famille.as_deref()),
                titre_statut(statut(d, m)).to_owned(),
                diplome,
                date_courte(m.arrive_le.as_deref()),
            ]
        })
        .collect();
    TableauRapport {
        code: format!("personnel_{}", cycle.nom()),
        titre: format!("Personnel du {}", titre_cycle(cycle).to_lowercase()),
        colonnes: colonnes(&[
            "N°",
            "Prénoms et nom",
            "N° matricule",
            "Situation de famille",
            "Statut",
            "Diplôme",
            "Date d'arrivée au poste",
        ]),
        lignes,
        paysage: true,
        lien: "statistiques".to_owned(),
    }
}

/// Inscrits, présentés et admis à un examen, par sexe (garçons, filles, total), comme l'onglet
/// des examens les compte : le détail par élève l'emporte sur les totaux saisis ; rien si
/// l'examen n'a ni l'un ni l'autre.
fn chiffres_examen(
    d: &DonneesStat,
    retenues: &[Retenue<'_>],
    examen: Examen,
) -> Option<[[Option<usize>; 3]; 3]> {
    let detail: Vec<(&Retenue<'_>, &ResultatExamen)> = retenues
        .iter()
        .filter(|r| r.presente)
        .filter_map(|r| {
            d.resultats
                .iter()
                .find(|x| {
                    x.inscription_id == r.inscription.inscription_id
                        && x.examen == examen
                        && (x.inscrit.is_some() || x.present.is_some() || x.admis.is_some())
                })
                .map(|x| (r, x))
        })
        .collect();
    if !detail.is_empty() {
        let compter_si = |champ: fn(&ResultatExamen) -> Option<bool>| {
            let mut s = Sexes::default();
            for (r, x) in &detail {
                if champ(x) == Some(true) {
                    s.ajouter(r.inscription.sexe.as_deref());
                }
            }
            [Some(s.g), Some(s.f), Some(s.t)]
        };
        return Some([
            compter_si(|x| x.inscrit),
            compter_si(|x| x.present),
            compter_si(|x| x.admis),
        ]);
    }
    let du_sexe = |s: &str| d.totaux.iter().find(|t| t.examen == examen && t.sexe == s);
    let (g, f) = (du_sexe("masculin"), du_sexe("feminin"));
    if g.is_none() && f.is_none() {
        return None;
    }
    let selon = |champ: fn(&TotalExamen) -> Option<u32>| {
        let g = g.and_then(champ).map(|x| x as usize);
        let f = f.and_then(champ).map(|x| x as usize);
        let t = match (g, f) {
            (None, None) => None,
            (g, f) => Some(g.unwrap_or(0) + f.unwrap_or(0)),
        };
        [g, f, t]
    };
    Some([
        selon(|t| t.inscrits),
        selon(|t| t.presentes),
        selon(|t| t.admis),
    ])
}

/// Résultats d'un examen, une ligne « École » : inscrits, présentés, admis et pourcentage de
/// réussite, par sexe. Cases vides quand l'examen n'a rien de saisi.
fn tableau_examen(
    d: &DonneesStat,
    retenues: &[Retenue<'_>],
    examen: Examen,
    code: &str,
    titre: &str,
) -> TableauRapport {
    let mut l = vec!["École".to_owned()];
    match chiffres_examen(d, retenues, examen) {
        Some([inscrits, presentes, admis]) => {
            for groupe in [inscrits, presentes, admis] {
                l.extend(groupe.iter().map(|n| n.map(texte).unwrap_or_default()));
            }
            for (a, p) in admis.iter().zip(presentes) {
                l.push(match (a, p) {
                    (Some(a), Some(p)) => gesco_core::core::examens::taux(
                        u32::try_from(*a).unwrap_or(u32::MAX),
                        u32::try_from(p).unwrap_or(u32::MAX),
                    )
                    .unwrap_or_default(),
                    _ => String::new(),
                });
            }
        }
        None => l.extend(std::iter::repeat_n(String::new(), 12)),
    }
    let mut cols = vec![String::new()];
    for groupe in ["Inscrits", "Présentés", "Admis", "% de réussite"] {
        for s in ["G", "F", "T"] {
            cols.push(format!("{groupe} {s}"));
        }
    }
    TableauRapport {
        code: code.to_owned(),
        titre: titre.to_owned(),
        colonnes: cols,
        lignes: vec![l],
        paysage: true,
        lien: "statistiques".to_owned(),
    }
}

/// Élèves présents d'un niveau, tous ou ceux d'une décision de fin d'année.
fn presents_du_niveau(retenues: &[Retenue<'_>], niveau_id: &str, decision: Option<&str>) -> Sexes {
    let mut s = Sexes::default();
    for r in retenues.iter().filter(|r| {
        r.presente
            && r.classe.niveau_id == niveau_id
            && decision.is_none_or(|x| r.inscription.decision == x)
    }) {
        s.ajouter(r.inscription.sexe.as_deref());
    }
    s
}

/// Trois lignes G, F, T d'un groupe, le libellé sur la première.
fn lignes_gft(libelle: &str, rangs: [Vec<String>; 3], total: [String; 3]) -> Vec<Vec<String>> {
    rangs
        .into_iter()
        .zip(total)
        .zip(["G", "F", "T"])
        .enumerate()
        .map(|(k, ((rang, total), sexe))| {
            let mut l = vec![
                if k == 0 {
                    libelle.to_owned()
                } else {
                    String::new()
                },
                sexe.to_owned(),
            ];
            l.extend(rang);
            l.push(total);
            l
        })
        .collect()
}

/// Effectifs probables d'un niveau : garçons, filles, total. Proposés : passants du niveau
/// d'en dessous plus redoublants du niveau ; la saisie de la direction l'emporte, case par case.
fn probable(
    niveaux: &[NiveauRapport<'_>],
    k: usize,
    retenues: &[Retenue<'_>],
    probables: &[EffectifProbable],
) -> (Sexes, Sexes) {
    let Some(n) = niveaux.get(k) else {
        return (Sexes::default(), Sexes::default());
    };
    let mut propose = presents_du_niveau(retenues, &n.id, Some("redouble"));
    if let Some(dessous) = k.checked_sub(1).and_then(|j| niveaux.get(j)) {
        propose = propose.plus(presents_du_niveau(retenues, &dessous.id, Some("admis")));
    }
    let saisi = probables.iter().find(|p| p.niveau_id == n.id);
    let vers = |x: Option<i64>| x.map(|x| usize::try_from(x).unwrap_or(0));
    let g_saisi = vers(saisi.and_then(|p| p.garcons));
    let f_saisi = vers(saisi.and_then(|p| p.filles));
    let g = g_saisi.unwrap_or(propose.g);
    let f = f_saisi.unwrap_or(propose.f);
    let t = if g_saisi.is_some() || f_saisi.is_some() {
        g + f
    } else {
        propose.t
    };
    (propose, Sexes { g, f, t })
}

/// Situation des effectifs en fin d'année pour un cycle (SPEC_PHASE_9 §5.2) : effectifs de
/// fermeture, passages, redoublements, exclusions, par niveau et sexe, puis les effectifs probables
/// à la rentrée. « Non statué » et « rattrapage » ne comptent dans aucune décision.
fn effectifs_fin_annee(
    cycle: Cycle,
    niveaux: &[NiveauRapport<'_>],
    retenues: &[Retenue<'_>],
    probables: &[EffectifProbable],
) -> TableauRapport {
    let du_cycle: Vec<usize> = niveaux
        .iter()
        .enumerate()
        .filter(|(_, n)| n.cycle == cycle)
        .map(|(k, _)| k)
        .collect();
    let mut lignes = Vec::new();
    for (libelle, decision) in [
        ("Effectifs de fermeture", None),
        ("Promotions (passages)", Some("admis")),
        ("Redoublements", Some("redouble")),
        ("Exclusions", Some("exclu")),
    ] {
        let mut rangs = [Vec::new(), Vec::new(), Vec::new()];
        let mut total = Sexes::default();
        let mut connu = false;
        for n in du_cycle.iter().filter_map(|k| niveaux.get(*k)) {
            let a_des_classes = !n.classes.is_empty();
            connu |= a_des_classes;
            let s = presents_du_niveau(retenues, &n.id, decision);
            total = total.plus(s);
            for (rang, case) in rangs.iter_mut().zip(s.cases(a_des_classes)) {
                rang.push(case);
            }
        }
        lignes.extend(lignes_gft(libelle, rangs, total.cases(connu)));
    }
    let mut rangs = [Vec::new(), Vec::new(), Vec::new()];
    let mut total = Sexes::default();
    for k in &du_cycle {
        let (_, retenu) = probable(niveaux, *k, retenues, probables);
        total = total.plus(retenu);
        for (rang, case) in rangs.iter_mut().zip(retenu.cases(true)) {
            rang.push(case);
        }
    }
    lignes.extend(lignes_gft(
        "Effectifs probables à la rentrée",
        rangs,
        total.cases(true),
    ));
    let mut cols = colonnes(&["", "Sexe"]);
    cols.extend(
        du_cycle
            .iter()
            .filter_map(|k| niveaux.get(*k))
            .map(|n| n.libelle.clone()),
    );
    cols.push("Total".to_owned());
    TableauRapport {
        code: format!("effectifs_fin_{}", cycle.nom()),
        titre: format!(
            "Situation des effectifs en fin d'année — {}",
            titre_cycle(cycle).to_lowercase()
        ),
        colonnes: cols,
        lignes,
        paysage: du_cycle.len() > 6,
        lien: "statistiques".to_owned(),
    }
}

/// Un niveau du fondamental et ses effectifs probables à la rentrée, pour la saisie.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct EffectifProbableNiveau {
    /// Niveau.
    pub niveau_id: String,
    /// Libellé du niveau.
    pub niveau: String,
    /// Garçons proposés.
    pub garcons_proposes: u32,
    /// Filles proposées.
    pub filles_proposees: u32,
    /// Garçons saisis par la direction.
    pub garcons: Option<u32>,
    /// Filles saisies par la direction.
    pub filles: Option<u32>,
}

/// Les niveaux du fondamental avec leurs effectifs probables proposés et saisis.
pub(crate) fn effectifs_probables(
    d: &DonneesStat,
    probables: &[EffectifProbable],
) -> Vec<EffectifProbableNiveau> {
    let perimetre = d.perimetre(None);
    let retenues = eleves(d, &perimetre);
    let niveaux = niveaux(d);
    let vers = |x: usize| u32::try_from(x).unwrap_or(u32::MAX);
    niveaux
        .iter()
        .enumerate()
        .map(|(k, n)| {
            let (propose, _) = probable(&niveaux, k, &retenues, &[]);
            let saisi = probables.iter().find(|p| p.niveau_id == n.id);
            let lu = |x: Option<i64>| x.and_then(|x| u32::try_from(x).ok());
            EffectifProbableNiveau {
                niveau_id: n.id.clone(),
                niveau: n.libelle.clone(),
                garcons_proposes: vers(propose.g),
                filles_proposees: vers(propose.f),
                garcons: lu(saisi.and_then(|p| p.garcons)),
                filles: lu(saisi.and_then(|p| p.filles)),
            }
        })
        .collect()
}

/// Existant et besoins d'une liste d'articles, par cycle du fondamental de l'école, avec une
/// partie « toute l'école » quand des lignes ont été saisies pour toute l'école. Sans
/// `garder_vides`, seuls les articles qui ont quelque chose de saisi paraissent.
fn existant_besoins(
    d: &DonneesStat,
    code: &str,
    titre: &str,
    articles: &[(String, Option<&ArticleInventaire>)],
    garder_vides: bool,
) -> TableauRapport {
    let mut cycles: Vec<Cycle> = FONDAMENTAL
        .iter()
        .copied()
        .filter(|c| d.cycles.contains(c))
        .collect();
    if cycles.is_empty() {
        cycles = FONDAMENTAL.to_vec();
    }
    let ecole = articles.iter().any(|(_, a)| {
        a.is_some_and(|a| {
            d.releves
                .iter()
                .any(|r| r.article_id == a.id && r.cycle.is_none() && !r.quantites.est_vide())
        })
    });
    let mut parties: Vec<(String, Option<Cycle>)> = cycles
        .iter()
        .map(|c| (titre_cycle(*c).to_owned(), Some(*c)))
        .collect();
    if ecole {
        parties.push(("Toute l'école".to_owned(), None));
    }
    let mut lignes = Vec::new();
    for (libelle, article) in articles {
        let mut l = vec![libelle.clone()];
        let mut saisi = false;
        for (_, partie) in &parties {
            let q = article
                .and_then(|a| releve_si(d, a, |c| c == *partie))
                .map(|(q, _)| q);
            saisi |= q.as_ref().is_some_and(|q| !q.est_vide());
            l.push(optionnel(q.as_ref().and_then(|q| q.existant)));
            l.push(optionnel(q.as_ref().and_then(|q| q.besoins)));
        }
        if garder_vides || saisi {
            lignes.push(l);
        }
    }
    let mut cols = vec![String::new()];
    for (p, _) in &parties {
        cols.push(format!("{p} — existant"));
        cols.push(format!("{p} — besoins"));
    }
    TableauRapport {
        code: code.to_owned(),
        titre: titre.to_owned(),
        colonnes: cols,
        lignes,
        paysage: false,
        lien: "inventaire".to_owned(),
    }
}

/// Une ligne du tableau des enseignants par statut : hommes et femmes de chaque statut, puis le
/// total des hommes et des femmes.
fn ligne_statuts(
    d: &DonneesStat,
    maitres: &[(&MembreStat, Vec<Cycle>)],
    retenir: impl Fn(&MembreStat, &[Cycle]) -> bool,
) -> Vec<usize> {
    let n = StatutRapport::TOUS.len();
    let mut c = vec![0usize; 2 * n + 2];
    for (m, cycles) in maitres {
        if !retenir(m, cycles) {
            continue;
        }
        let decalage = match m.sexe.as_deref() {
            Some("masculin") => 0,
            Some("feminin") => 1,
            _ => continue,
        };
        if let Some(k) = StatutRapport::TOUS.iter().position(|s| *s == statut(d, m)) {
            compter(&mut c, 2 * k + decalage);
        }
        compter(&mut c, 2 * n + decalage);
    }
    c
}

/// Enseignants par spécialité, statut et sexe : les généralistes du premier cycle, puis les
/// spécialistes du second cycle.
fn enseignants_par_statut(d: &DonneesStat) -> TableauRapport {
    let maitres = maitres(d);
    let mut groupes: Vec<(String, Vec<usize>)> = Vec::new();
    if d.cycles.contains(&Cycle::PremierCycle) {
        groupes.push((
            "Premier cycle — généralistes".to_owned(),
            ligne_statuts(d, &maitres, |_, cycles| {
                cycles.contains(&Cycle::PremierCycle)
            }),
        ));
    }
    if d.cycles.contains(&Cycle::SecondCycle) {
        for sp in Specialite::TOUS {
            groupes.push((
                format!("Second cycle — {}", titre_specialite(*sp)),
                ligne_statuts(d, &maitres, |m, cycles| {
                    cycles.contains(&Cycle::SecondCycle) && specialite(d, m) == *sp
                }),
            ));
        }
    }
    let mut totaux = vec![0usize; 2 * StatutRapport::TOUS.len() + 2];
    let mut lignes = Vec::new();
    for (libelle, c) in groupes {
        for (t, x) in totaux.iter_mut().zip(&c) {
            *t += x;
        }
        let mut l = vec![libelle];
        l.extend(c.iter().map(|x| texte(*x)));
        lignes.push(l);
    }
    let mut l = vec!["Total".to_owned()];
    l.extend(totaux.iter().map(|x| texte(*x)));
    lignes.push(l);
    let mut cols = vec!["Spécialité".to_owned()];
    for s in StatutRapport::TOUS {
        cols.push(format!("{} H", titre_statut(*s)));
        cols.push(format!("{} F", titre_statut(*s)));
    }
    cols.extend(colonnes(&["Total H", "Total F"]));
    TableauRapport {
        code: "enseignants_par_statut".to_owned(),
        titre: "Répartition des enseignants par spécialité, statut et sexe".to_owned(),
        colonnes: cols,
        lignes,
        paysage: true,
        lien: "statistiques".to_owned(),
    }
}

/// Codes et intitulés des locaux du rapport de fermeture, dans l'ordre du modèle.
const LOCAUX_FERMETURE: [(&str, &str); 7] = [
    ("salle_classe", "Salles de classe"),
    ("bureau_direction", "Direction"),
    (CODE_LATRINES, "Blocs de latrines"),
    ("magasin", "Magasin"),
    ("point_eau", "Points d'eau potable"),
    ("coin_lecture", "Coins de lecture"),
    ("bibliotheque", "Bibliothèque"),
];

/// Les articles d'une famille, désignés avec leur niveau quand ils en ont un.
fn articles_de_la_famille(
    d: &DonneesStat,
    famille: FamilleInventaire,
) -> Vec<(String, Option<&ArticleInventaire>)> {
    d.articles
        .iter()
        .filter(|a| a.famille == famille)
        .map(|a| {
            let niveau = a
                .niveau_id
                .and_then(|n| d.niveaux.iter().find(|x| x.id == n))
                .map(|n| format!(" ({})", n.libelle))
                .unwrap_or_default();
            (format!("{}{niveau}", a.designation), Some(a))
        })
        .collect()
}

/// Les tableaux du rapport de fermeture (SPEC_PHASE_9 §5.2), à la date d'arrêt.
pub(crate) fn fermeture(d: &DonneesStat, annexes: &AnnexesFermeture<'_>) -> Vec<TableauRapport> {
    let perimetre = d.perimetre(None);
    let retenues = eleves(d, &perimetre);
    let niveaux = niveaux(d);
    let mut tableaux = Vec::new();
    for cycle in FONDAMENTAL {
        if d.cycles.contains(&cycle) {
            tableaux.push(personnel_nominatif(d, cycle));
        }
    }
    if d.cycles.contains(&Cycle::PremierCycle) {
        tableaux.push(tableau_examen(
            d,
            &retenues,
            Examen::Entree7e,
            "examen_entree_7e",
            "Résultats des examens — passage en 7e année",
        ));
    }
    if d.cycles.contains(&Cycle::SecondCycle) {
        tableaux.push(tableau_examen(
            d,
            &retenues,
            Examen::Def,
            "examen_def",
            "Résultats des examens — DEF",
        ));
    }
    for cycle in FONDAMENTAL {
        if niveaux.iter().any(|n| n.cycle == cycle) {
            tableaux.push(effectifs_fin_annee(
                cycle,
                &niveaux,
                &retenues,
                annexes.probables,
            ));
        }
    }
    let locaux: Vec<(String, Option<&ArticleInventaire>)> = LOCAUX_FERMETURE
        .iter()
        .map(|(code, libelle)| ((*libelle).to_owned(), article(d, code)))
        .collect();
    tableaux.push(existant_besoins(
        d,
        "infrastructures",
        "Situation des infrastructures",
        &locaux,
        true,
    ));
    for (famille, code, titre) in [
        (
            FamilleInventaire::Mobilier,
            "mobilier_fermeture",
            "Situation du mobilier",
        ),
        (
            FamilleInventaire::Materiel,
            "materiel_fermeture",
            "Situation du matériel didactique",
        ),
        (
            FamilleInventaire::Manuel,
            "manuels_fermeture",
            "Situation des manuels scolaires",
        ),
    ] {
        tableaux.push(existant_besoins(
            d,
            code,
            titre,
            &articles_de_la_famille(d, famille),
            false,
        ));
    }
    tableaux.push(enseignants_par_statut(d));
    tableaux
}

/// Élèves d'un cycle à la date : garçons, filles, total, et les élèves de chaque niveau qui a
/// des classes.
pub(crate) struct ElevesDuCycle {
    /// Cycle.
    pub(crate) cycle: Cycle,
    /// Garçons.
    pub(crate) garcons: usize,
    /// Filles.
    pub(crate) filles: usize,
    /// Total, sexe inconnu compris.
    pub(crate) total: usize,
    /// Libellé et élèves de chaque niveau qui a des classes.
    pub(crate) niveaux: Vec<(String, usize)>,
    /// Enseignants du cycle.
    pub(crate) enseignants: usize,
    /// Élèves décidés (passage, redoublement, exclusion).
    pub(crate) decides: usize,
    /// Admis parmi eux.
    pub(crate) admis: usize,
}

/// Chiffres de la fin d'année pour les phrases proposées du rapport de fermeture : par cycle du
/// fondamental qui a des niveaux, puis les besoins saisis à l'inventaire (désignation, quantité).
pub(crate) fn chiffres_fermeture(d: &DonneesStat) -> (Vec<ElevesDuCycle>, Vec<(String, u32)>) {
    let perimetre = d.perimetre(None);
    let retenues = eleves(d, &perimetre);
    let niveaux = niveaux(d);
    let maitres = maitres(d);
    let mut cycles = Vec::new();
    for cycle in FONDAMENTAL {
        if !niveaux.iter().any(|n| n.cycle == cycle) {
            continue;
        }
        let mut total = Sexes::default();
        let mut par_niveau = Vec::new();
        for n in niveaux.iter().filter(|n| n.cycle == cycle) {
            let e = presents_du_niveau(&retenues, &n.id, None);
            total = total.plus(e);
            if !n.classes.is_empty() {
                par_niveau.push((n.libelle.clone(), e.t));
            }
        }
        let du_cycle: Vec<&Retenue<'_>> = retenues
            .iter()
            .filter(|r| r.presente && Cycle::depuis_nom(&r.classe.cycle) == Some(cycle))
            .collect();
        cycles.push(ElevesDuCycle {
            cycle,
            garcons: total.g,
            filles: total.f,
            total: total.t,
            niveaux: par_niveau,
            enseignants: maitres.iter().filter(|(_, c)| c.contains(&cycle)).count(),
            decides: du_cycle
                .iter()
                .filter(|r| {
                    matches!(
                        r.inscription.decision.as_str(),
                        "admis" | "redouble" | "exclu"
                    )
                })
                .count(),
            admis: du_cycle
                .iter()
                .filter(|r| r.inscription.decision == "admis")
                .count(),
        });
    }
    let besoins = d
        .articles
        .iter()
        .filter_map(|a| {
            releve(d, a, None)
                .and_then(|(q, _)| q.besoins)
                .filter(|b| *b > 0)
                .map(|b| (a.designation.clone(), b))
        })
        .collect();
    (cycles, besoins)
}
