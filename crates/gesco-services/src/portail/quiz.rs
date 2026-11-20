//! Quiz au portail des élèves (SPEC_PHASE_12_LOT_3 §5 et §7).
//!
//! Le téléphone reçoit les questions d'un quiz, avec les bonnes réponses et les explications :
//! il corrige chaque question tout de suite. Quand l'élève a fini, il envoie ses réponses ; le
//! poste principal les **corrige à nouveau** et n'accepte jamais le score du téléphone. La
//! tentative ne s'écrit que pour un élève entré avec son code, suivi allumé ; elle attend dans
//! une file en mémoire, écrite toutes les cinq secondes avec l'avancement. Un quiz abandonné en
//! route n'envoie rien.

use std::sync::Mutex;

use gesco_core::bibliotheque::paquet::empreinte;
use gesco_core::bibliotheque::{QUESTIONS_REVISION, QuestionQuiz, corriger};
use gesco_db::depots::portail::{ReponseQuizAEcrire, TentativeQuizAEcrire};
use gesco_db::nouvel_id;
use serde::{Deserialize, Serialize};

/// Tentatives en attente au plus : au-delà, une tentative nouvelle est oubliée sans rien dire.
pub const FILE_TENTATIVES_MAX: usize = 5_000;

/// Réponses d'une tentative, au plus.
pub const REPONSES_MAX: usize = 100;

/// Longueur d'une réponse gardée, au plus (caractères).
pub const REPONSE_MAX: usize = 200;

/// Une question posée à l'élève, avec le contenu et la version d'où elle vient.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QuestionEleve {
    /// Contenu d'où vient la question.
    pub contenu_id: String,
    /// Empreinte de la version de ce contenu.
    pub version: String,
    /// La question, ses choix, ses bonnes réponses et son explication.
    pub question: QuestionQuiz,
}

/// Un quiz à passer : celui d'un contenu, ou la révision d'un chapitre.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QuizEleve {
    /// Titre du contenu ou du chapitre.
    pub titre: String,
    /// Contenu du quiz.
    pub contenu_id: Option<String>,
    /// Chapitre révisé.
    pub chapitre_id: Option<String>,
    /// Questions, dans l'ordre où elles se posent.
    pub questions: Vec<QuestionEleve>,
    /// Images appelées par les contenus d'où viennent les questions.
    pub images: Vec<super::lecture::ImageEleve>,
}

/// Une réponse envoyée par le téléphone.
#[derive(Debug, Clone, Deserialize)]
pub struct ReponseDonnee {
    /// Contenu de la question.
    pub contenu_id: String,
    /// Version reçue.
    pub version: String,
    /// Numéro de la question.
    pub numero: u32,
    /// Réponse donnée (« B », « A,C », « A-2,B-1 » ou la réponse écrite).
    pub reponse: String,
}

/// Ce que le téléphone envoie à `POST /eleve/api/tentative`, le quiz fini.
#[derive(Debug, Clone, Deserialize)]
pub struct DemandeTentative {
    /// Quiz d'un contenu…
    pub contenu_id: Option<String>,
    /// … ou révision d'un chapitre.
    pub chapitre_id: Option<String>,
    /// Commencée le (millisecondes, horloge du téléphone ; bornée par le poste principal).
    pub commencee_le: Option<i64>,
    /// Les réponses, dans l'ordre.
    pub reponses: Vec<ReponseDonnee>,
}

/// Une réponse corrigée par le poste principal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReponseCorrigee {
    /// Contenu de la question.
    pub contenu_id: String,
    /// Version de la question telle qu'elle a été posée.
    pub version: String,
    /// Numéro de la question.
    pub numero: u32,
    /// Réponse donnée, bornée.
    pub reponse: String,
    /// Juste.
    pub juste: bool,
}

/// Corrige une réponse sur la question telle qu'elle a été posée.
pub fn corriger_donnee(
    question: &QuestionQuiz,
    contenu_id: &str,
    version: &str,
    donnee: &ReponseDonnee,
) -> ReponseCorrigee {
    let reponse: String = donnee.reponse.chars().take(REPONSE_MAX).collect();
    ReponseCorrigee {
        contenu_id: contenu_id.to_owned(),
        version: version.to_owned(),
        numero: donnee.numero,
        juste: corriger(question, &reponse),
        reponse,
    }
}

/// Empreinte d'une révision : les questions tirées, avec leur version.
pub fn empreinte_revision(reponses: &[ReponseCorrigee]) -> String {
    let mut a_hacher = String::new();
    for r in reponses {
        a_hacher.push_str(&r.contenu_id);
        a_hacher.push(':');
        a_hacher.push_str(&r.version);
        a_hacher.push(':');
        a_hacher.push_str(&r.numero.to_string());
        a_hacher.push('\n');
    }
    empreinte(a_hacher.as_bytes())
}

/// Tire au plus [`QUESTIONS_REVISION`] questions au hasard, sans remise ; toutes, mélangées,
/// s'il y en a moins. `graine` vient de l'aléa du système (les essais en donnent une fixe).
pub fn tirer(mut questions: Vec<QuestionEleve>, graine: u64) -> Vec<QuestionEleve> {
    let mut etat = graine | 1;
    let mut suivant = || {
        // xorshift64* : un tirage simple, suffisant pour mélanger des questions.
        etat ^= etat >> 12;
        etat ^= etat << 25;
        etat ^= etat >> 27;
        etat.wrapping_mul(0x2545_F491_4F6C_DD1D)
    };
    let n = questions.len();
    for i in (1..n).rev() {
        let borne = u64::try_from(i.saturating_add(1)).unwrap_or(u64::MAX);
        let j = usize::try_from(suivant() % borne).unwrap_or(0);
        questions.swap(i, j);
    }
    questions.truncate(QUESTIONS_REVISION);
    questions
}

/// File des tentatives finies, écrite toutes les cinq secondes avec l'avancement.
#[derive(Debug, Default)]
pub struct FileTentatives {
    tentatives: Mutex<Vec<TentativeQuizAEcrire>>,
}

impl FileTentatives {
    /// Met une tentative corrigée dans la file.
    pub fn noter(
        &self,
        (eleve_id, annee_id): (&str, &str),
        (contenu_id, chapitre_id): (Option<&str>, Option<&str>),
        version: &str,
        (commencee_le, finie_le): (i64, i64),
        reponses: Vec<ReponseCorrigee>,
    ) {
        let Ok(mut file) = self.tentatives.lock() else {
            return;
        };
        if file.len() >= FILE_TENTATIVES_MAX || reponses.is_empty() {
            return;
        }
        let sur = u32::try_from(reponses.len()).unwrap_or(u32::MAX);
        let score = u32::try_from(reponses.iter().filter(|r| r.juste).count()).unwrap_or(0);
        file.push(TentativeQuizAEcrire {
            id: nouvel_id::<()>().vers_chaine(),
            eleve_id: eleve_id.to_owned(),
            annee_id: annee_id.to_owned(),
            contenu_id: contenu_id.map(str::to_owned),
            chapitre_id: chapitre_id.map(str::to_owned),
            version: version.to_owned(),
            commencee_le: commencee_le.min(finie_le),
            finie_le,
            score,
            sur,
            reponses: reponses
                .into_iter()
                .map(|r| ReponseQuizAEcrire {
                    id: nouvel_id::<()>().vers_chaine(),
                    contenu_id: r.contenu_id,
                    version: r.version,
                    numero: r.numero,
                    reponse: r.reponse,
                    juste: r.juste,
                })
                .collect(),
        });
    }

    /// Prend tout ce qui attend ; la file se vide.
    pub fn prendre(&self) -> Vec<TentativeQuizAEcrire> {
        self.tentatives
            .lock()
            .map(|mut f| std::mem::take(&mut *f))
            .unwrap_or_default()
    }

    /// Remet des tentatives qui n'ont pas pu s'écrire.
    pub fn remettre(&self, tentatives: Vec<TentativeQuizAEcrire>) {
        if let Ok(mut f) = self.tentatives.lock() {
            f.extend(tentatives);
        }
    }

    /// Tentatives en attente d'une année (pour les compter déjà dans le suivi).
    pub fn en_attente(&self, annee_id: &str) -> Vec<TentativeQuizAEcrire> {
        self.tentatives
            .lock()
            .map(|f| {
                f.iter()
                    .filter(|t| t.annee_id == annee_id)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Nombre de tentatives en attente.
    pub fn longueur(&self) -> usize {
        self.tentatives.lock().map(|f| f.len()).unwrap_or(0)
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use gesco_core::bibliotheque::analyser_quiz;

    use super::*;

    fn questions(n: u32) -> Vec<QuestionEleve> {
        let mut texte = String::from("## Quiz\n\n");
        for i in 1..=n {
            texte.push_str(&format!("**Question {i}.** Q{i} ?\nA) un  B) deux\n\n"));
        }
        texte.push_str("### Réponses\n\n");
        for i in 1..=n {
            texte.push_str(&format!("{i}. **B.** Deux.\n"));
        }
        analyser_quiz(&texte, false)
            .questions
            .into_iter()
            .map(|q| QuestionEleve {
                contenu_id: "c".to_owned(),
                version: "v".to_owned(),
                question: q,
            })
            .collect()
    }

    #[test]
    fn la_revision_tire_dix_questions_ou_toutes() {
        let tirees = tirer(questions(25), 42);
        assert_eq!(tirees.len(), QUESTIONS_REVISION);
        let mut numeros: Vec<u32> = tirees.iter().map(|q| q.question.numero).collect();
        numeros.sort_unstable();
        numeros.dedup();
        assert_eq!(numeros.len(), QUESTIONS_REVISION, "sans remise");
        assert_ne!(
            tirer(questions(25), 42)
                .iter()
                .map(|q| q.question.numero)
                .collect::<Vec<_>>(),
            tirer(questions(25), 7)
                .iter()
                .map(|q| q.question.numero)
                .collect::<Vec<_>>(),
        );
        assert_eq!(tirer(questions(4), 42).len(), 4);
        assert!(tirer(Vec::new(), 42).is_empty());
    }

    #[test]
    fn le_score_vient_du_poste_principal() {
        let q = questions(2);
        let donnee = |reponse: &str| ReponseDonnee {
            contenu_id: "c".to_owned(),
            version: "v".to_owned(),
            numero: 1,
            reponse: reponse.to_owned(),
        };
        let juste = corriger_donnee(&q[0].question, "c", "v", &donnee("b"));
        let faux = corriger_donnee(&q[1].question, "c", "v", &donnee("A"));
        assert!(juste.juste);
        assert!(!faux.juste);
        let file = FileTentatives::default();
        file.noter(
            ("e", "a"),
            (Some("c"), None),
            "v",
            (20, 10),
            vec![juste, faux],
        );
        file.noter(("e", "a"), (Some("c"), None), "v", (1, 2), Vec::new());
        assert_eq!(
            file.longueur(),
            1,
            "une tentative sans réponse ne s'écrit pas"
        );
        let prises = file.prendre();
        assert_eq!((prises[0].score, prises[0].sur), (1, 2));
        assert_eq!(
            prises[0].commencee_le, 10,
            "commencée au plus tard à la fin"
        );
        assert_eq!(file.longueur(), 0);
    }
}
