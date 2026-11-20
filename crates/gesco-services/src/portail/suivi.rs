//! Avancement des élèves au portail et suivi par leurs maîtres (SPEC_PHASE_12_LOT_4 §6).
//!
//! L'élève entré avec son code ouvre une leçon, puis appuie sur « J'ai fini cette leçon » :
//! ces gestes vont dans une **file en mémoire** ([`FileAvancement`]) que le poste principal
//! écrit d'un coup toutes les cinq secondes (ADR « charge du portail des élèves ») ; une classe
//! entière qui lit ne prend pas la base à chaque page. Ce qui attend encore dans la file se
//! compte déjà dans « Mon avancement » et dans le suivi du maître : les chiffres restent justes.
//!
//! Le suivi ne classe pas, ne relance pas, ne note pas (§9.5) : des nombres par leçon et, quand
//! la direction le permet, les leçons finies de chaque élève dans l'ordre alphabétique.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use gesco_core::bibliotheque::portail::GrainSuivi;
use gesco_core::bibliotheque::{Lecon, NatureContenu, QuestionQuiz, analyser_quiz};
use gesco_core::core::ModuleEcole;
use gesco_db::depots::portail::{
    self as depot, AvancementAEcrire, LigneAvancement, LigneReponse, LigneTentative,
};
use gesco_db::nouvel_id;
use serde::{Deserialize, Serialize};

use super::entree::SessionEleve;
use super::lecture::{CatalogueEleve, LecturePortail};
use crate::erreur::{ErreurService, Resultat};
use crate::noyau::Noyau;
use crate::session::Session;

/// Dernières leçons ouvertes montrées à l'élève, au plus.
pub const RECENTES_MAX: usize = 5;

/// Derniers scores de quiz montrés à l'élève, au plus.
pub const SCORES_MAX: usize = 5;

/// Questions les plus manquées montrées pour un quiz, au plus.
pub const MANQUEES_MAX: usize = 3;

/// Longueur d'un énoncé montré au suivi, au plus (caractères).
pub const ENONCE_MAX: usize = 160;

/// Versions remplacées dont on relit le texte pour un suivi, au plus.
const ANCIENNES_MAX: usize = 50;

/// Gestes en attente au plus (ADR « charge du portail des élèves ») : au-delà, un geste nouveau
/// est oublié sans rien dire à l'élève ; ceux qui attendent déjà se fondent toujours.
pub const FILE_AVANCEMENT_MAX: usize = 10_000;

/// Geste de l'élève sur une leçon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GesteAvancement {
    /// La leçon s'est ouverte.
    Ouvert,
    /// « J'ai fini cette leçon ».
    Fini,
}

/// Ce que le téléphone envoie à `POST /eleve/api/avancement`.
#[derive(Debug, Clone, Deserialize)]
pub struct DemandeAvancement {
    /// Contenu lu.
    pub contenu_id: String,
    /// Empreinte de la version lue, telle que le contenu l'a donnée.
    pub version: String,
    /// Ouvert ou fini.
    pub geste: GesteAvancement,
}

/// Un geste qui attend d'être écrit.
#[derive(Debug, Clone, PartialEq, Eq)]
struct EnAttente {
    version: String,
    ouvert_le: i64,
    dernier_le: i64,
    fini_le: Option<i64>,
}

/// Clé d'un geste : élève, année, contenu.
type CleGeste = (String, String, String);

/// File des gestes d'avancement, vidée toutes les cinq secondes. Deux gestes sur la même leçon
/// se fondent : la première ouverture reste, la dernière avance, « fini » ne s'efface pas.
#[derive(Debug, Default)]
pub struct FileAvancement {
    gestes: Mutex<HashMap<CleGeste, EnAttente>>,
}

impl FileAvancement {
    /// Note un geste.
    pub fn noter(
        &self,
        (eleve_id, annee_id, contenu_id): (&str, &str, &str),
        version: &str,
        geste: GesteAvancement,
        maintenant: i64,
    ) {
        let Ok(mut gestes) = self.gestes.lock() else {
            return;
        };
        let fini = (geste == GesteAvancement::Fini).then_some(maintenant);
        let cle = (
            eleve_id.to_owned(),
            annee_id.to_owned(),
            contenu_id.to_owned(),
        );
        if gestes.len() >= FILE_AVANCEMENT_MAX && !gestes.contains_key(&cle) {
            return;
        }
        gestes
            .entry(cle)
            .and_modify(|e| {
                version.clone_into(&mut e.version);
                e.dernier_le = e.dernier_le.max(maintenant);
                e.fini_le = e.fini_le.or(fini);
            })
            .or_insert_with(|| EnAttente {
                version: version.to_owned(),
                ouvert_le: maintenant,
                dernier_le: maintenant,
                fini_le: fini,
            });
    }

    /// Prend tout ce qui attend, prêt à écrire ; la file se vide.
    pub fn prendre(&self) -> Vec<AvancementAEcrire> {
        let Ok(mut gestes) = self.gestes.lock() else {
            return Vec::new();
        };
        gestes
            .drain()
            .map(|((eleve_id, annee_id, contenu_id), e)| AvancementAEcrire {
                id: nouvel_id::<()>().vers_chaine(),
                eleve_id,
                annee_id,
                contenu_id,
                version: e.version,
                ouvert_le: e.ouvert_le,
                dernier_le: e.dernier_le,
                fini_le: e.fini_le,
            })
            .collect()
    }

    /// Remet dans la file des gestes qui n'ont pas pu s'écrire ; ils repartent au passage
    /// suivant, fondus avec ce qui est arrivé entre-temps.
    pub fn remettre(&self, gestes: Vec<AvancementAEcrire>) {
        let Ok(mut file) = self.gestes.lock() else {
            return;
        };
        for g in gestes {
            file.entry((g.eleve_id, g.annee_id, g.contenu_id))
                .and_modify(|e| {
                    e.ouvert_le = e.ouvert_le.min(g.ouvert_le);
                    e.dernier_le = e.dernier_le.max(g.dernier_le);
                    e.fini_le = match (e.fini_le, g.fini_le) {
                        (Some(a), Some(b)) => Some(a.min(b)),
                        (a, b) => a.or(b),
                    };
                })
                .or_insert(EnAttente {
                    version: g.version,
                    ouvert_le: g.ouvert_le,
                    dernier_le: g.dernier_le,
                    fini_le: g.fini_le,
                });
        }
    }

    /// Gestes en attente pour une année, comme des lignes d'avancement ; `eleves` borne aux
    /// élèves voulus.
    pub fn en_attente(
        &self,
        annee_id: &str,
        eleves: &dyn Fn(&str) -> bool,
    ) -> Vec<LigneAvancement> {
        let Ok(file) = self.gestes.lock() else {
            return Vec::new();
        };
        file.iter()
            .filter(|((eleve, annee, _), _)| annee == annee_id && eleves(eleve))
            .map(|((eleve, _, contenu), e)| LigneAvancement {
                eleve_id: eleve.clone(),
                contenu_id: contenu.clone(),
                ouvert_le: e.ouvert_le,
                dernier_le: e.dernier_le,
                fini_le: e.fini_le,
            })
            .collect()
    }

    /// Nombre de gestes en attente.
    pub fn longueur(&self) -> usize {
        self.gestes.lock().map(|g| g.len()).unwrap_or(0)
    }
}

/// Fond les lignes écrites et celles qui attendent : une ligne par élève et contenu.
pub fn fondre(
    ecrites: Vec<LigneAvancement>,
    attente: Vec<LigneAvancement>,
) -> Vec<LigneAvancement> {
    let mut par_cle: HashMap<(String, String), LigneAvancement> = HashMap::new();
    for l in ecrites.into_iter().chain(attente) {
        par_cle
            .entry((l.eleve_id.clone(), l.contenu_id.clone()))
            .and_modify(|e| {
                e.ouvert_le = e.ouvert_le.min(l.ouvert_le);
                e.dernier_le = e.dernier_le.max(l.dernier_le);
                e.fini_le = e.fini_le.or(l.fini_le);
            })
            .or_insert(l);
    }
    let mut lignes: Vec<LigneAvancement> = par_cle.into_values().collect();
    lignes.sort_by(|a, b| {
        b.dernier_le
            .cmp(&a.dernier_le)
            .then_with(|| a.contenu_id.cmp(&b.contenu_id))
    });
    lignes
}

// ---------------------------------------------------------------------------------------------
// Ce que voit l'élève
// ---------------------------------------------------------------------------------------------

/// Une matière dans « Mon avancement ».
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MatiereAvancement {
    /// Clé de la matière, comme au catalogue.
    pub cle: String,
    /// Libellé.
    pub libelle: String,
    /// Leçons finies, parmi les visibles.
    pub finies: u32,
    /// Leçons visibles.
    pub visibles: u32,
}

/// Une leçon ouverte récemment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LeconRecente {
    /// Contenu.
    pub id: String,
    /// Titre.
    pub titre: String,
    /// Libellé de sa matière.
    pub matiere: String,
}

/// Un score récent de l'élève à un quiz.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ScoreRecent {
    /// Titre du quiz, ou « Révision : » et le titre du chapitre.
    pub titre: String,
    /// Réponses justes.
    pub score: u32,
    /// Questions posées.
    pub sur: u32,
    /// Fini le (millisecondes).
    pub finie_le: i64,
}

/// « Mon avancement » : par matière, leçons finies sur leçons visibles ; dernières leçons
/// ouvertes, la plus récente d'abord ; contenus finis ; derniers scores aux quiz.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MonAvancement {
    /// Par matière, dans l'ordre du catalogue.
    pub matieres: Vec<MatiereAvancement>,
    /// Dernières leçons ouvertes.
    pub recentes: Vec<LeconRecente>,
    /// Contenus finis, parmi ceux que l'élève voit.
    pub finies: Vec<String>,
    /// Derniers scores aux quiz, le plus récent d'abord.
    pub scores: Vec<ScoreRecent>,
}

/// « Mon avancement » d'après le catalogue de l'élève (ce qu'il voit seulement), ses lignes et
/// ses tentatives de quiz. Un contenu qu'il ne voit plus (retiré) ne compte pas : les nombres
/// restent justes.
pub fn mon_avancement(
    catalogue: &CatalogueEleve,
    lignes: &[LigneAvancement],
    tentatives: &[LigneTentative],
) -> MonAvancement {
    let finis: HashSet<&str> = lignes
        .iter()
        .filter(|l| l.fini_le.is_some())
        .map(|l| l.contenu_id.as_str())
        .collect();
    let mut places: HashMap<&str, (&str, &str)> = HashMap::new();
    let mut chapitres: HashMap<&str, &str> = HashMap::new();
    let mut matieres = Vec::new();
    let mut finies = Vec::new();
    for m in &catalogue.matieres {
        for c in m.parties.iter().flat_map(|p| p.chapitres.iter()) {
            chapitres.insert(c.id.as_str(), c.titre.as_str());
        }
        let mut finies_ici = 0_u32;
        for e in m
            .parties
            .iter()
            .flat_map(|p| p.chapitres.iter())
            .flat_map(|c| c.contenus.iter())
        {
            places.insert(e.id.as_str(), (e.titre.as_str(), m.libelle.as_str()));
            if finis.contains(e.id.as_str()) {
                finies.push(e.id.clone());
                if e.nature == NatureContenu::Lecon {
                    finies_ici = finies_ici.saturating_add(1);
                }
            }
        }
        matieres.push(MatiereAvancement {
            cle: m.cle.clone(),
            libelle: m.libelle.clone(),
            finies: finies_ici,
            visibles: m.lecons,
        });
    }
    let mut ouvertes: Vec<&LigneAvancement> = lignes.iter().collect();
    ouvertes.sort_by_key(|l| std::cmp::Reverse(l.dernier_le));
    let recentes = ouvertes
        .into_iter()
        .filter_map(|l| {
            places
                .get(l.contenu_id.as_str())
                .map(|(titre, matiere)| LeconRecente {
                    id: l.contenu_id.clone(),
                    titre: (*titre).to_owned(),
                    matiere: (*matiere).to_owned(),
                })
        })
        .take(RECENTES_MAX)
        .collect();
    let mut faites: Vec<&LigneTentative> = tentatives.iter().collect();
    faites.sort_by(|a, b| b.finie_le.cmp(&a.finie_le).then_with(|| b.id.cmp(&a.id)));
    let scores = faites
        .into_iter()
        .filter_map(|t| {
            let titre = match (&t.contenu_id, &t.chapitre_id) {
                (Some(c), _) => places.get(c.as_str()).map(|(titre, _)| (*titre).to_owned()),
                (None, Some(ch)) => chapitres
                    .get(ch.as_str())
                    .map(|titre| format!("Révision : {titre}")),
                (None, None) => None,
            }?;
            Some(ScoreRecent {
                titre,
                score: u32::try_from(t.score).unwrap_or(0),
                sur: u32::try_from(t.sur).unwrap_or(0),
                finie_le: t.finie_le,
            })
        })
        .take(SCORES_MAX)
        .collect();
    MonAvancement {
        matieres,
        recentes,
        finies,
        scores,
    }
}

// ---------------------------------------------------------------------------------------------
// Les quiz au suivi
// ---------------------------------------------------------------------------------------------

/// Énoncé d'une question en texte simple, borné à [`ENONCE_MAX`] caractères.
fn enonce_court(question: &QuestionQuiz) -> String {
    let texte = Lecon {
        blocs: question.enonce.clone(),
    }
    .texte_brut();
    let texte = texte.split_whitespace().collect::<Vec<_>>().join(" ");
    if texte.chars().count() <= ENONCE_MAX {
        return texte;
    }
    let mut court: String = texte.chars().take(ENONCE_MAX.saturating_sub(1)).collect();
    court.push('…');
    court
}

/// Énoncés d'une version d'un contenu, par numéro.
pub type Enonces = HashMap<u32, String>;

/// Un quiz que la classe voit, tel que le suivi le résume.
#[derive(Debug, Clone)]
pub struct QuizASuivre {
    /// Contenu.
    pub id: String,
    /// Titre.
    pub titre: String,
    /// Titre de son chapitre.
    pub chapitre: String,
    /// Empreinte de la version visible.
    pub version: String,
    /// Énoncés de la version visible.
    pub enonces: Enonces,
}

/// Rapport de deux scores, pour trouver le meilleur sans flottant : `a` vaut mieux que `b`.
fn meilleur_que(a: ScoreQuiz, b: ScoreQuiz) -> bool {
    let gauche = u64::from(a.score).saturating_mul(u64::from(b.sur.max(1)));
    let droite = u64::from(b.score).saturating_mul(u64::from(a.sur.max(1)));
    gauche > droite || (gauche == droite && a.score > b.score)
}

/// Résume un quiz : élèves qui l'ont fait, moyenne du premier essai, questions les plus
/// manquées (version par version : une question de deux versions compte deux fois, à part) et,
/// quand `eleves` est donné, premier et meilleur essai de chacun. `tentatives` et `reponses`
/// sont celles des élèves de la classe ; une réponse donnée pendant une révision du chapitre
/// compte pour la question manquée. `anciennes` donne les énoncés des versions remplacées.
pub fn resumer_quiz(
    quiz: &QuizASuivre,
    tentatives: &[LigneTentative],
    reponses: &[LigneReponse],
    anciennes: &HashMap<(String, String), Enonces>,
    eleves: Option<&[(String, String)]>,
) -> QuizSuivi {
    let score_de = |t: &LigneTentative| ScoreQuiz {
        score: u32::try_from(t.score).unwrap_or(0),
        sur: u32::try_from(t.sur).unwrap_or(0),
    };
    // Par élève : (premier, meilleur, essais), les tentatives dans l'ordre où elles ont fini.
    let mut siennes: Vec<&LigneTentative> = tentatives
        .iter()
        .filter(|t| t.contenu_id.as_deref() == Some(quiz.id.as_str()) && t.sur > 0)
        .collect();
    siennes.sort_by(|a, b| a.finie_le.cmp(&b.finie_le).then_with(|| a.id.cmp(&b.id)));
    let mut par_eleve: HashMap<&str, (ScoreQuiz, ScoreQuiz, u32)> = HashMap::new();
    for t in &siennes {
        let s = score_de(t);
        par_eleve
            .entry(t.eleve_id.as_str())
            .and_modify(|(_, meilleur, essais)| {
                if meilleur_que(s, *meilleur) {
                    *meilleur = s;
                }
                *essais = essais.saturating_add(1);
            })
            .or_insert((s, s, 1));
    }
    let moyenne_premier = if par_eleve.is_empty() {
        None
    } else {
        // Somme des pour-mille de chaque premier essai, arrondie au pour-cent.
        let somme: u64 = par_eleve
            .values()
            .map(|(p, _, _)| u64::from(p.score).saturating_mul(1000) / u64::from(p.sur.max(1)))
            .sum();
        let n = u64::try_from(par_eleve.len()).unwrap_or(1);
        let pour_mille = somme / n;
        u32::try_from(pour_mille.saturating_add(5) / 10).ok()
    };

    // Questions manquées, version par version.
    let mut par_question: HashMap<(&str, i64), (u32, u32)> = HashMap::new();
    for r in reponses.iter().filter(|r| r.contenu_id == quiz.id) {
        let e = par_question
            .entry((r.empreinte_version.as_str(), r.numero))
            .or_default();
        e.0 = e.0.saturating_add(u32::from(!r.juste));
        e.1 = e.1.saturating_add(1);
    }
    let mut manquees: Vec<QuestionManquee> = par_question
        .into_iter()
        .filter(|(_, (fautes, _))| *fautes > 0)
        .map(|((version, numero), (fautes, donnees))| {
            let numero = u32::try_from(numero).unwrap_or(0);
            let ancienne = version != quiz.version;
            let enonces = if ancienne {
                anciennes.get(&(quiz.id.clone(), version.to_owned()))
            } else {
                Some(&quiz.enonces)
            };
            QuestionManquee {
                numero,
                enonce: enonces
                    .and_then(|e| e.get(&numero))
                    .cloned()
                    .unwrap_or_default(),
                ancienne,
                fautes,
                reponses: donnees,
            }
        })
        .collect();
    manquees.sort_by(|a, b| {
        b.fautes
            .cmp(&a.fautes)
            .then_with(|| a.ancienne.cmp(&b.ancienne))
            .then_with(|| a.numero.cmp(&b.numero))
    });
    manquees.truncate(MANQUEES_MAX);

    QuizSuivi {
        id: quiz.id.clone(),
        titre: quiz.titre.clone(),
        chapitre: quiz.chapitre.clone(),
        questions: u32::try_from(quiz.enonces.len()).unwrap_or(u32::MAX),
        eleves: u32::try_from(par_eleve.len()).unwrap_or(u32::MAX),
        moyenne_premier,
        manquees,
        par_eleve: eleves
            .unwrap_or_default()
            .iter()
            .map(|(id, nom)| {
                let s = par_eleve.get(id.as_str());
                ScoreEleveQuiz {
                    eleve_id: id.clone(),
                    nom: nom.clone(),
                    premier: s.map(|(p, _, _)| *p),
                    meilleur: s.map(|(_, m, _)| *m),
                    essais: s.map(|(_, _, n)| *n).unwrap_or(0),
                }
            })
            .collect(),
    }
}

/// Énoncés des questions d'un texte, par numéro.
fn enonces_de(questions: &[QuestionQuiz]) -> Enonces {
    questions
        .iter()
        .map(|q| (q.numero, enonce_court(q)))
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Ce que voient le maître et la direction
// ---------------------------------------------------------------------------------------------

/// Ce que demande l'écran « Suivi des élèves » : une classe et une matière ; sans elles, la
/// première de la liste.
#[derive(Debug, Clone, Default, Serialize, Deserialize, specta::Type)]
pub struct DemandeSuivi {
    /// Classe voulue.
    #[serde(default)]
    pub classe_id: Option<String>,
    /// Matière voulue (clé du catalogue).
    #[serde(default)]
    pub matiere_cle: Option<String>,
}

/// Une classe que ce compte peut suivre.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ClasseASuivre {
    /// Classe.
    pub id: String,
    /// Libellé.
    pub libelle: String,
}

/// Une matière de la classe que ce compte peut suivre.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct MatiereSuivie {
    /// Clé du catalogue.
    pub cle: String,
    /// Libellé.
    pub libelle: String,
}

/// Une leçon dans le suivi : combien d'élèves de la classe l'ont ouverte et finie.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct LeconSuivie {
    /// Contenu.
    pub id: String,
    /// Titre.
    pub titre: String,
    /// Titre de son chapitre.
    pub chapitre: String,
    /// Élèves qui l'ont ouverte.
    pub ouvertes: u32,
    /// Élèves qui l'ont finie.
    pub finies: u32,
}

/// Un élève dans le suivi (grain « par élève » seulement).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct EleveSuivi {
    /// Élève.
    pub eleve_id: String,
    /// Nom et prénoms.
    pub nom: String,
    /// Leçons de la matière ouvertes.
    pub ouvertes: u32,
    /// Leçons de la matière finies.
    pub finies: u32,
    /// Dernière visite au portail (millisecondes), toutes matières.
    pub derniere_visite: Option<i64>,
    /// Quiz de la matière faits au moins une fois.
    pub quiz_faits: u32,
}

/// Un score à un quiz : réponses justes sur questions posées.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ScoreQuiz {
    /// Réponses justes.
    pub score: u32,
    /// Questions posées.
    pub sur: u32,
}

/// Une question souvent manquée, dans la version où elle a été posée.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct QuestionManquee {
    /// Numéro de la question.
    pub numero: u32,
    /// Énoncé, tel qu'il a été posé (vide s'il n'a pas pu être relu).
    pub enonce: String,
    /// Posée dans une version remplacée depuis.
    pub ancienne: bool,
    /// Réponses fausses.
    pub fautes: u32,
    /// Réponses données.
    pub reponses: u32,
}

/// Les scores d'un élève à un quiz (grain « par élève » seulement).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ScoreEleveQuiz {
    /// Élève.
    pub eleve_id: String,
    /// Nom et prénoms.
    pub nom: String,
    /// Premier essai.
    pub premier: Option<ScoreQuiz>,
    /// Meilleur essai.
    pub meilleur: Option<ScoreQuiz>,
    /// Essais.
    pub essais: u32,
}

/// Un quiz dans le suivi : combien d'élèves l'ont fait, la moyenne du premier essai, les
/// questions les plus manquées et, par élève, le premier essai et le meilleur.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct QuizSuivi {
    /// Contenu.
    pub id: String,
    /// Titre.
    pub titre: String,
    /// Titre de son chapitre.
    pub chapitre: String,
    /// Questions dans la version visible.
    pub questions: u32,
    /// Élèves qui l'ont fait.
    pub eleves: u32,
    /// Moyenne du premier essai, en bonnes réponses pour cent.
    pub moyenne_premier: Option<u32>,
    /// Questions les plus manquées, la plus manquée d'abord.
    pub manquees: Vec<QuestionManquee>,
    /// Élèves, dans l'ordre alphabétique ; vide au grain « par classe ».
    pub par_eleve: Vec<ScoreEleveQuiz>,
}

/// Le suivi d'une classe dans une matière.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct SuiviClasse {
    /// Le suivi est allumé (et l'e-learning aussi) ; éteint, rien n'est montré.
    pub actif: bool,
    /// `eleve` ou `classe`.
    pub grain: String,
    /// Classes que ce compte peut suivre.
    pub classes: Vec<ClasseASuivre>,
    /// Classe montrée.
    pub classe_id: Option<String>,
    /// Libellé de la classe montrée.
    pub classe: String,
    /// Élèves inscrits dans la classe.
    pub eleves: u32,
    /// Matières de la classe que ce compte peut suivre.
    pub matieres: Vec<MatiereSuivie>,
    /// Matière montrée.
    pub matiere_cle: Option<String>,
    /// Leçons de la matière, dans l'ordre du programme.
    pub lecons: Vec<LeconSuivie>,
    /// Élèves, dans l'ordre alphabétique ; vide au grain « par classe ».
    pub par_eleve: Vec<EleveSuivi>,
    /// Quiz de la matière (leçons qui en ont un et quiz de chapitre), dans l'ordre du programme.
    pub quiz: Vec<QuizSuivi>,
}

/// Le suivi demandé, et ce qu'il faut pour l'imprimer.
pub struct SuiviLu {
    /// Ce que l'écran montre.
    pub suivi: SuiviClasse,
    /// Libellé de la matière montrée.
    pub matiere: String,
}

/// Session « de classe » pour lire ce que voient les élèves de cette classe : sans élève, avec
/// la classe et l'année, pour que les ajouts réservés à la classe comptent.
fn session_de_classe(c: &depot::ClasseSuivie) -> SessionEleve {
    SessionEleve {
        eleve_id: None,
        prenom: None,
        classe_id: Some(c.classe_id.clone()),
        classe: Some(c.libelle.clone()),
        niveau_id: c.niveau_id.clone(),
        niveau: c.niveau.clone(),
        serie_id: c.serie_id.clone(),
        annee_id: Some(c.annee_id.clone()),
        ephemere: false,
        duree_ms: 0,
        derniere_activite: 0,
    }
}

/// Le suivi d'une classe, pour ce compte : un compte de maître ne voit que les classes et les
/// matières de ses affectations ; la direction voit tout. `file` et `tentatives` apportent les
/// gestes et les quiz qui attendent encore d'être écrits.
pub async fn lire(
    noyau: &Noyau,
    lecture: &LecturePortail,
    (file, tentatives): (&FileAvancement, &super::quiz::FileTentatives),
    session: &Session,
    demande: &DemandeSuivi,
) -> Resultat<SuiviLu> {
    let contexte = noyau.contexte(session.etablissement_id, Some(session));
    let maitre = noyau
        .droits_de(session.utilisateur_id, &contexte)
        .await?
        .maitre;
    let mut cnx = noyau
        .base()
        .pool()
        .acquire()
        .await
        .map_err(gesco_db::ErreurBase::from)?;
    let reglages = super::ServicePortail::lire_reglages(&mut cnx, &contexte).await?;
    let module = crate::modules::actif(&mut cnx, ModuleEcole::Bibliotheque, &contexte).await?;
    let actif = reglages.suivi && module;
    let grain = GrainSuivi::depuis_nom(&reglages.grain).unwrap_or_default();

    // Classes et matières permises.
    let mut classes = depot::classes_suivies(&mut cnx, &contexte).await?;
    let permises: Option<HashMap<String, HashSet<String>>> = if maitre {
        let mut p: HashMap<String, HashSet<String>> = HashMap::new();
        for (classe, matiere) in depot::affectations_du_compte(
            &mut cnx,
            &session.utilisateur_id.vers_chaine(),
            &contexte,
        )
        .await?
        {
            p.entry(classe).or_default().insert(format!("m:{matiere}"));
        }
        classes.retain(|c| p.contains_key(&c.classe_id));
        Some(p)
    } else {
        None
    };
    let choisie = match &demande.classe_id {
        Some(id) => Some(classes.iter().find(|c| &c.classe_id == id).ok_or_else(|| {
            ErreurService::PorteeRefusee {
                ressource: format!("classe {id}"),
            }
        })?),
        None => classes.first(),
    };
    let liste: Vec<ClasseASuivre> = classes
        .iter()
        .map(|c| ClasseASuivre {
            id: c.classe_id.clone(),
            libelle: c.libelle.clone(),
        })
        .collect();
    let mut suivi = SuiviClasse {
        actif,
        grain: grain.nom().to_owned(),
        classes: liste,
        classe_id: None,
        classe: String::new(),
        eleves: 0,
        matieres: Vec::new(),
        matiere_cle: None,
        lecons: Vec::new(),
        par_eleve: Vec::new(),
        quiz: Vec::new(),
    };
    let Some(classe) = choisie.cloned() else {
        return Ok(SuiviLu {
            suivi,
            matiere: String::new(),
        });
    };
    suivi.classe_id = Some(classe.classe_id.clone());
    suivi.classe = classe.libelle.clone();

    // Ce que voient les élèves de la classe, matière par matière.
    let catalogue = lecture.catalogue(&session_de_classe(&classe));
    let siennes = permises
        .as_ref()
        .and_then(|p| p.get(&classe.classe_id))
        .cloned();
    let matieres: Vec<_> = catalogue
        .matieres
        .iter()
        .filter(|m| siennes.as_ref().is_none_or(|s| s.contains(&m.cle)))
        .collect();
    suivi.matieres = matieres
        .iter()
        .map(|m| MatiereSuivie {
            cle: m.cle.clone(),
            libelle: m.libelle.clone(),
        })
        .collect();
    let matiere = match &demande.matiere_cle {
        Some(cle) => matieres.iter().find(|m| &m.cle == cle).copied(),
        None => None,
    }
    .or_else(|| matieres.first().copied());

    let eleves = depot::eleves_de_la_classe(&mut cnx, &classe.classe_id, &contexte).await?;
    suivi.eleves = u32::try_from(eleves.len()).unwrap_or(u32::MAX);
    let Some(matiere) = matiere else {
        return Ok(SuiviLu {
            suivi,
            matiere: String::new(),
        });
    };
    suivi.matiere_cle = Some(matiere.cle.clone());
    // Éteint, ce qui a été écrit reste mais ne se montre plus.
    if !actif {
        return Ok(SuiviLu {
            suivi,
            matiere: matiere.libelle.clone(),
        });
    }

    let de_la_classe: HashSet<&str> = eleves.iter().map(|e| e.eleve_id.as_str()).collect();
    let ecrites =
        depot::avancement_de_la_classe(&mut cnx, &classe.classe_id, &classe.annee_id, &contexte)
            .await?;
    let attente = file.en_attente(&classe.annee_id, &|e| de_la_classe.contains(e));
    let lignes: Vec<LigneAvancement> = fondre(ecrites, attente)
        .into_iter()
        .filter(|l| de_la_classe.contains(l.eleve_id.as_str()))
        .collect();

    let lecons: Vec<(String, String, String)> = matiere
        .parties
        .iter()
        .flat_map(|p| p.chapitres.iter())
        .flat_map(|c| {
            c.contenus
                .iter()
                .filter(|e| e.nature == NatureContenu::Lecon)
                .map(move |e| (e.id.clone(), e.titre.clone(), c.titre.clone()))
        })
        .collect();
    let de_la_matiere: HashSet<&str> = lecons.iter().map(|(id, _, _)| id.as_str()).collect();
    let mut par_contenu: HashMap<&str, (u32, u32)> = HashMap::new();
    let mut par_eleve: HashMap<&str, (u32, u32)> = HashMap::new();
    let mut visites: HashMap<&str, i64> = HashMap::new();
    for l in &lignes {
        let v = visites.entry(l.eleve_id.as_str()).or_insert(l.dernier_le);
        *v = (*v).max(l.dernier_le);
        if !de_la_matiere.contains(l.contenu_id.as_str()) {
            continue;
        }
        let fini = u32::from(l.fini_le.is_some());
        let c = par_contenu.entry(l.contenu_id.as_str()).or_default();
        c.0 = c.0.saturating_add(1);
        c.1 = c.1.saturating_add(fini);
        let e = par_eleve.entry(l.eleve_id.as_str()).or_default();
        e.0 = e.0.saturating_add(1);
        e.1 = e.1.saturating_add(fini);
    }
    suivi.lecons = lecons
        .iter()
        .map(|(id, titre, chapitre)| {
            let (ouvertes, finies) = par_contenu.get(id.as_str()).copied().unwrap_or_default();
            LeconSuivie {
                id: id.clone(),
                titre: titre.clone(),
                chapitre: chapitre.clone(),
                ouvertes,
                finies,
            }
        })
        .collect();

    // Les quiz de la matière : leçons qui en ont un et quiz de chapitre, dans l'ordre.
    let a_suivre: Vec<QuizASuivre> = matiere
        .parties
        .iter()
        .flat_map(|p| p.chapitres.iter())
        .flat_map(|c| c.contenus.iter().map(move |e| (c, e)))
        .filter_map(|(c, e)| {
            let (questions, version, _) = lecture.questions_de(&e.id)?;
            (!questions.is_empty()).then(|| QuizASuivre {
                id: e.id.clone(),
                titre: e.titre.clone(),
                chapitre: c.titre.clone(),
                version: version.to_owned(),
                enonces: enonces_de(questions),
            })
        })
        .collect();
    let (mut faites, mut donnees) =
        depot::tentatives_de_la_classe(&mut cnx, &classe.classe_id, &classe.annee_id, &contexte)
            .await?;
    let (faites_attente, donnees_attente) =
        tentatives.en_attente(&classe.annee_id, &|e| de_la_classe.contains(e));
    faites.extend(faites_attente);
    donnees.extend(donnees_attente);
    // Énoncés des versions remplacées depuis, relus dans les versions de l'école.
    let mut anciennes: HashMap<(String, String), Enonces> = HashMap::new();
    let mut a_relire: Vec<(String, String)> = donnees
        .iter()
        .filter_map(|r| {
            let q = a_suivre.iter().find(|q| q.id == r.contenu_id)?;
            (q.version != r.empreinte_version)
                .then(|| (r.contenu_id.clone(), r.empreinte_version.clone()))
        })
        .collect();
    a_relire.sort();
    a_relire.dedup();
    for (contenu, version) in a_relire.into_iter().take(ANCIENNES_MAX) {
        let tout = lecture
            .questions_de(&contenu)
            .is_some_and(|(_, _, n)| n == NatureContenu::Quiz);
        if let Some(texte) = depot::texte_de_version(&mut cnx, &contenu, &version).await? {
            let enonces = enonces_de(&analyser_quiz(&texte, tout).questions);
            anciennes.insert((contenu, version), enonces);
        }
    }
    let noms: Vec<(String, String)> = eleves
        .iter()
        .map(|e| {
            (
                e.eleve_id.clone(),
                format!("{} {}", e.nom, e.prenoms).trim().to_owned(),
            )
        })
        .collect();
    let par_eleve_aussi = grain == GrainSuivi::Eleve;
    suivi.quiz = a_suivre
        .iter()
        .map(|q| {
            resumer_quiz(
                q,
                &faites,
                &donnees,
                &anciennes,
                par_eleve_aussi.then_some(noms.as_slice()),
            )
        })
        .collect();
    let mut quiz_faits: HashMap<&str, HashSet<&str>> = HashMap::new();
    for t in &faites {
        if let Some(c) = t.contenu_id.as_deref()
            && a_suivre.iter().any(|q| q.id == c)
        {
            quiz_faits.entry(t.eleve_id.as_str()).or_default().insert(c);
        }
    }

    if par_eleve_aussi {
        suivi.par_eleve = eleves
            .iter()
            .zip(&noms)
            .map(|(e, (_, nom))| {
                let (ouvertes, finies) = par_eleve
                    .get(e.eleve_id.as_str())
                    .copied()
                    .unwrap_or_default();
                EleveSuivi {
                    eleve_id: e.eleve_id.clone(),
                    nom: nom.clone(),
                    ouvertes,
                    finies,
                    derniere_visite: visites.get(e.eleve_id.as_str()).copied(),
                    quiz_faits: quiz_faits
                        .get(e.eleve_id.as_str())
                        .map(|s| u32::try_from(s.len()).unwrap_or(u32::MAX))
                        .unwrap_or(0),
                }
            })
            .collect();
    }
    Ok(SuiviLu {
        suivi,
        matiere: matiere.libelle.clone(),
    })
}

/// Date courte d'un instant (heure du Mali, qui est celle de Greenwich) : `18/11/2026`.
fn date_courte(millis: i64) -> String {
    let c = gesco_core::commun::Horodatage::depuis_millis_epoque(millis).civil_utc();
    format!("{:02}/{:02}/{}", c.jour, c.mois, c.annee)
}

/// Données du modèle d'impression `suivi_eleves`.
pub fn donnees_impression(lu: &SuiviLu, date: &str) -> serde_json::Value {
    let s = &lu.suivi;
    serde_json::json!({
        "classe": s.classe,
        "matiere": lu.matiere,
        "date": date,
        "eleves": s.eleves,
        "par_eleve": s.grain == GrainSuivi::Eleve.nom(),
        "lecons": s.lecons.iter().map(|l| serde_json::json!({
            "chapitre": l.chapitre,
            "titre": l.titre,
            "ouvertes": l.ouvertes,
            "finies": l.finies,
        })).collect::<Vec<_>>(),
        "liste": s.par_eleve.iter().map(|e| serde_json::json!({
            "nom": e.nom,
            "ouvertes": e.ouvertes,
            "finies": e.finies,
            "visite": e.derniere_visite.map(date_courte).unwrap_or_default(),
            "quiz": e.quiz_faits,
        })).collect::<Vec<_>>(),
        "quiz": s.quiz.iter().map(|q| serde_json::json!({
            "chapitre": q.chapitre,
            "titre": q.titre,
            "eleves": q.eleves,
            "moyenne": q.moyenne_premier.map(|m| format!("{m} %")).unwrap_or_default(),
            "manquees": q.manquees.iter().map(|m| serde_json::json!({
                "numero": m.numero,
                "enonce": m.enonce,
                "ancienne": m.ancienne,
                "fautes": m.fautes,
                "reponses": m.reponses,
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ligne(eleve: &str, contenu: &str, dernier: i64, fini: Option<i64>) -> LigneAvancement {
        LigneAvancement {
            eleve_id: eleve.to_owned(),
            contenu_id: contenu.to_owned(),
            ouvert_le: dernier,
            dernier_le: dernier,
            fini_le: fini,
        }
    }

    #[test]
    fn deux_gestes_sur_la_meme_lecon_se_fondent() {
        let file = FileAvancement::default();
        file.noter(("e1", "a1", "c1"), "v1", GesteAvancement::Ouvert, 10);
        file.noter(("e1", "a1", "c1"), "v2", GesteAvancement::Fini, 20);
        file.noter(("e1", "a1", "c1"), "v2", GesteAvancement::Ouvert, 30);
        assert_eq!(file.longueur(), 1);
        let pris = file.prendre();
        assert_eq!(file.longueur(), 0);
        assert_eq!(pris.len(), 1);
        assert_eq!(
            pris.first()
                .map(|g| (g.ouvert_le, g.dernier_le, g.fini_le, g.version.as_str())),
            Some((10, 30, Some(20), "v2"))
        );
        // Un échec d'écriture les remet, fondus avec ce qui est arrivé entre-temps.
        file.noter(("e1", "a1", "c1"), "v3", GesteAvancement::Ouvert, 40);
        file.remettre(pris);
        let repris = file.prendre();
        assert_eq!(repris.len(), 1);
        assert_eq!(
            repris
                .first()
                .map(|g| (g.ouvert_le, g.dernier_le, g.fini_le)),
            Some((10, 40, Some(20)))
        );
    }

    #[test]
    fn l_attente_compte_deja() {
        let file = FileAvancement::default();
        file.noter(("e1", "a1", "c2"), "v", GesteAvancement::Fini, 50);
        file.noter(("e2", "a1", "c2"), "v", GesteAvancement::Ouvert, 60);
        file.noter(("e1", "a0", "c2"), "v", GesteAvancement::Ouvert, 70);
        let attente = file.en_attente("a1", &|e| e == "e1");
        assert_eq!(attente.len(), 1);
        let lignes = fondre(vec![ligne("e1", "c1", 5, None)], attente);
        assert_eq!(lignes.len(), 2);
        assert_eq!(
            lignes.first().map(|l| (l.contenu_id.as_str(), l.fini_le)),
            Some(("c2", Some(50)))
        );
    }

    #[test]
    fn une_lecon_finie_le_reste() {
        let lignes = fondre(
            vec![ligne("e1", "c1", 5, Some(5))],
            vec![ligne("e1", "c1", 9, None)],
        );
        assert_eq!(lignes.len(), 1);
        assert_eq!(
            lignes.first().map(|l| (l.dernier_le, l.fini_le)),
            Some((9, Some(5)))
        );
    }

    fn tentative(id: &str, eleve: &str, fin: i64, score: i64, sur: i64) -> LigneTentative {
        LigneTentative {
            id: id.to_owned(),
            eleve_id: eleve.to_owned(),
            contenu_id: Some("q".to_owned()),
            chapitre_id: None,
            empreinte_version: "v2".to_owned(),
            finie_le: fin,
            score,
            sur,
        }
    }

    fn reponse(tentative: &str, version: &str, numero: i64, juste: bool) -> LigneReponse {
        LigneReponse {
            tentative_id: tentative.to_owned(),
            contenu_id: "q".to_owned(),
            empreinte_version: version.to_owned(),
            numero,
            juste,
        }
    }

    #[test]
    #[allow(clippy::indexing_slicing)]
    fn le_quiz_se_resume_par_premier_et_meilleur_essai() {
        let quiz = QuizASuivre {
            id: "q".to_owned(),
            titre: "La racine carrée".to_owned(),
            chapitre: "Racines".to_owned(),
            version: "v2".to_owned(),
            enonces: HashMap::from([(1, "Un ?".to_owned()), (2, "Deux ?".to_owned())]),
        };
        // Adama : 1 sur 2 puis 2 sur 2 ; Awa : 2 sur 2 ; Bakary n'a rien fait.
        let tentatives = vec![
            tentative("t2", "adama", 20, 2, 2),
            tentative("t1", "adama", 10, 1, 2),
            tentative("t3", "awa", 15, 2, 2),
        ];
        // La question 2 de l'ancienne version a été manquée deux fois, celle de la version
        // visible une fois : elles restent à part.
        let reponses = vec![
            reponse("t1", "v2", 1, true),
            reponse("t1", "v2", 2, false),
            reponse("t0", "v1", 2, false),
            reponse("t9", "v1", 2, false),
            reponse("t3", "v2", 1, true),
        ];
        let anciennes = HashMap::from([(
            ("q".to_owned(), "v1".to_owned()),
            HashMap::from([(2, "Deux, comme avant ?".to_owned())]),
        )]);
        let noms = vec![
            ("adama".to_owned(), "Adama".to_owned()),
            ("awa".to_owned(), "Awa".to_owned()),
            ("bakary".to_owned(), "Bakary".to_owned()),
        ];
        let r = resumer_quiz(&quiz, &tentatives, &reponses, &anciennes, Some(&noms));
        assert_eq!(r.eleves, 2);
        assert_eq!(r.questions, 2);
        assert_eq!(r.moyenne_premier, Some(75), "premier essai : 50 % et 100 %");
        assert_eq!(r.manquees.len(), 2);
        assert_eq!(
            (
                r.manquees[0].numero,
                r.manquees[0].ancienne,
                r.manquees[0].fautes
            ),
            (2, true, 2)
        );
        assert_eq!(r.manquees[0].enonce, "Deux, comme avant ?");
        assert_eq!(
            (
                r.manquees[1].numero,
                r.manquees[1].ancienne,
                r.manquees[1].fautes
            ),
            (2, false, 1)
        );
        assert_eq!(r.manquees[1].enonce, "Deux ?");
        let adama = &r.par_eleve[0];
        assert_eq!(adama.premier, Some(ScoreQuiz { score: 1, sur: 2 }));
        assert_eq!(adama.meilleur, Some(ScoreQuiz { score: 2, sur: 2 }));
        assert_eq!(adama.essais, 2);
        assert_eq!(r.par_eleve[2].premier, None);
        assert_eq!(r.par_eleve[2].essais, 0);
        // Par classe seulement : les nombres, sans noms.
        let sans_noms = resumer_quiz(&quiz, &tentatives, &reponses, &anciennes, None);
        assert!(sans_noms.par_eleve.is_empty());
        assert_eq!(sans_noms.moyenne_premier, Some(75));
        // Personne : pas de moyenne.
        let vide = resumer_quiz(&quiz, &[], &[], &anciennes, None);
        assert_eq!((vide.eleves, vide.moyenne_premier), (0, None));
        assert!(vide.manquees.is_empty());
    }

    #[test]
    fn le_meilleur_essai_compare_les_rapports() {
        let s = |score, sur| ScoreQuiz { score, sur };
        assert!(meilleur_que(s(3, 4), s(2, 4)));
        assert!(meilleur_que(s(2, 2), s(3, 4)));
        assert!(!meilleur_que(s(1, 2), s(2, 4)));
        assert!(
            meilleur_que(s(2, 4), s(1, 2)),
            "à rapport égal, plus de justes"
        );
    }

    #[test]
    fn mon_avancement_donne_les_derniers_scores_visibles() {
        use super::super::lecture::{ChapitreEleve, EntreeEleve, MatiereEleve, PartieEleve};
        let catalogue = CatalogueEleve {
            niveau: "9e".to_owned(),
            matieres: vec![MatiereEleve {
                cle: "m:1".to_owned(),
                libelle: "Mathématiques".to_owned(),
                lecons: 1,
                parties: vec![PartieEleve {
                    titre: String::new(),
                    chapitres: vec![ChapitreEleve {
                        id: "ch".to_owned(),
                        titre: "Racines".to_owned(),
                        contenus: vec![EntreeEleve {
                            id: "q".to_owned(),
                            titre: "La racine carrée".to_owned(),
                            nature: NatureContenu::Lecon,
                            examen: None,
                            session: None,
                        }],
                    }],
                }],
            }],
        };
        let mut revision = tentative("t4", "adama", 40, 7, 10);
        revision.contenu_id = None;
        revision.chapitre_id = Some("ch".to_owned());
        let mut retire = tentative("t5", "adama", 50, 1, 1);
        retire.contenu_id = Some("retire".to_owned());
        let mon = mon_avancement(
            &catalogue,
            &[],
            &[tentative("t1", "adama", 10, 1, 2), revision, retire],
        );
        let scores: Vec<(&str, u32, u32)> = mon
            .scores
            .iter()
            .map(|s| (s.titre.as_str(), s.score, s.sur))
            .collect();
        assert_eq!(
            scores,
            vec![("Révision : Racines", 7, 10), ("La racine carrée", 1, 2)],
            "le plus récent d'abord ; un contenu retiré ne se montre plus"
        );
    }
}
