//! Écrire un quiz (SPEC_PHASE_12_LOT_3 §4) : le bouton Question, l'essai du quiz dans l'éditeur
//! et l'import de questions depuis un classeur Excel. Tout passe par le texte : une question
//! importée est écrite selon la convention des quiz, puis relue par l'analyse ; ce qu'elle ne
//! reconnaît pas n'est pas écrit et la ligne dit pourquoi.

use gesco_core::bibliotheque::{
    NatureContenu, QuestionQuiz, TEXTE_MAX, ajouter_question, analyser_quiz, corriger,
    dernier_numero, gabarit_question,
};
use serde::{Deserialize, Serialize};

use super::{ServiceBibliotheque, regle};
use crate::erreur::Resultat;
use crate::export::{DemandeExport, FichierEcrit};
use crate::session::Session;

/// Colonnes du modèle d'import, dans l'ordre.
const COLONNES_MODELE: [&str; 9] = [
    "Question",
    "Choix A",
    "Choix B",
    "Choix C",
    "Choix D",
    "Choix E",
    "Choix F",
    "Bonne réponse",
    "Explication",
];

/// Lignes d'un import, au plus.
const LIGNES_IMPORT_MAX: usize = 500;

/// Ajouter une question au texte de l'éditeur.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeQuestionQuiz {
    /// Contenu écrit (sa nature dit où va la question).
    pub contenu_id: String,
    /// Texte tapé.
    pub texte: String,
}

/// Le texte de l'éditeur après le geste.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct TexteQuiz {
    /// Texte complet.
    pub texte: String,
}

/// Corriger une réponse pendant un essai, sur un poste (rien n'est écrit).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeCorrectionQuiz {
    /// La question telle qu'elle est posée.
    pub question: QuestionQuiz,
    /// Réponse donnée (« B », « A,C », « A-2,B-1 » ou la réponse écrite).
    pub reponse: String,
}

/// Écrire le modèle d'import.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeModeleQuiz {
    /// Chemin du classeur à écrire.
    pub chemin: String,
}

/// Importer des questions depuis un classeur, dans le texte de l'éditeur.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeImportQuiz {
    /// Contenu écrit.
    pub contenu_id: String,
    /// Classeur choisi.
    pub chemin: String,
    /// Texte tapé, auquel les questions s'ajoutent.
    pub texte: String,
}

/// Une ligne du classeur qui n'est pas écrite, et pourquoi.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct LigneQuizRefusee {
    /// Numéro de la ligne dans le classeur.
    pub ligne: u32,
    /// Raison, en une phrase.
    pub raison: String,
}

/// Le texte après l'import, à enregistrer dans le brouillon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ImportQuiz {
    /// Texte complet, questions ajoutées.
    pub texte: String,
    /// Questions écrites.
    pub ecrites: u32,
    /// Lignes non écrites.
    pub refusees: Vec<LigneQuizRefusee>,
}

/// Réviser un chapitre sur un poste.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DemandeRevisionChapitre {
    /// Chapitre de la bibliothèque.
    pub chapitre_id: String,
}

/// Les questions d'une révision, tirées au hasard, et les images qu'elles appellent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct RevisionChapitre {
    /// Titre du chapitre.
    pub titre: String,
    /// Dix questions au plus.
    pub questions: Vec<QuestionQuiz>,
    /// Images des contenus d'où viennent les questions.
    pub images: Vec<super::ImageLecon>,
}

/// Une cellule ramenée à une ligne de texte.
fn une_ligne(cellule: &str) -> String {
    cellule.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Écrit la question d'une ligne du classeur selon la convention, avec son numéro ; ou la raison
/// pour laquelle elle n'est pas écrite.
pub(crate) fn question_de_ligne(
    numero: u32,
    cellules: &[String],
) -> std::result::Result<(String, String), String> {
    let cellule = |i: usize| cellules.get(i).map(|c| une_ligne(c)).unwrap_or_default();
    let enonce = cellule(0);
    if enonce.is_empty() {
        return Err("la question est vide.".to_owned());
    }
    let choix: Vec<String> = (1..=6).map(cellule).collect();
    let dernier = choix.iter().rposition(|c| !c.is_empty());
    let bonne = cellule(7);
    let explication = cellule(8);
    if bonne.is_empty() {
        return Err("la bonne réponse manque.".to_owned());
    }
    let (ligne_choix, reponse) = match dernier {
        None => {
            // Réponse courte : les réponses admises, séparées par « ou » ou « ; ».
            let admises: Vec<String> = bonne
                .split(';')
                .flat_map(|r| r.split(" ou "))
                .map(|r| r.trim().trim_matches(['«', '»', '"']).trim().to_owned())
                .filter(|r| !r.is_empty())
                .collect();
            let ecrites: Vec<String> = admises.iter().map(|r| format!("« {r} »")).collect();
            (None, format!("**{}.**", ecrites.join(" ou ")))
        }
        Some(d) => {
            if let Some(vide) = choix.iter().take(d).position(String::is_empty) {
                let lettre = char::from(b'A'.saturating_add(u8::try_from(vide).unwrap_or(0)));
                return Err(format!("le choix {lettre} est vide."));
            }
            if d == 0 {
                return Err("une question à choix en demande au moins deux.".to_owned());
            }
            let lettres: Vec<String> = bonne
                .split([',', ';', ' '])
                .map(|r| r.trim().trim_end_matches(['.', ')']).to_uppercase())
                .filter(|r| !r.is_empty())
                .collect();
            for l in &lettres {
                let dedans = l.len() == 1
                    && l.bytes()
                        .next()
                        .is_some_and(|o| o >= b'A' && usize::from(o - b'A') <= d);
                if !dedans {
                    return Err(format!("la bonne réponse {l} n'est pas parmi les choix."));
                }
            }
            let ecrits: Vec<String> = choix
                .iter()
                .take(d.saturating_add(1))
                .enumerate()
                .map(|(i, c)| {
                    let lettre = char::from(b'A'.saturating_add(u8::try_from(i).unwrap_or(0)));
                    format!("{lettre}) {c}")
                })
                .collect();
            (
                Some(ecrits.join("  ")),
                format!("**{}.**", lettres.join(", ")),
            )
        }
    };
    let question = match ligne_choix {
        Some(c) => format!("**Question {numero}.** {enonce}\n{c}\n"),
        None => format!("**Question {numero}.** {enonce}\n"),
    };
    let reponse = if explication.is_empty() {
        format!("{numero}. {reponse}")
    } else {
        format!("{numero}. {reponse} {explication}")
    };
    // La question écrite doit être reconnue telle quelle ; sinon elle n'est pas écrite.
    let seule = ajouter_question("", true, &question, &reponse);
    let quiz = analyser_quiz(&seule, true);
    if let Some(e) = quiz.ecartees.first() {
        let raison = e
            .raison
            .split_once(':')
            .map_or(e.raison.as_str(), |(_, r)| r)
            .trim()
            .to_owned();
        return Err(raison);
    }
    if quiz.questions.is_empty() {
        return Err("la question n'est pas reconnue.".to_owned());
    }
    Ok((question, reponse))
}

/// Ajoute au texte les questions des lignes lues (en-tête comprise en première ligne).
pub(crate) fn importer_lignes(
    texte: &str,
    tout_le_texte: bool,
    lignes: &[Vec<String>],
) -> ImportQuiz {
    let mut texte = texte.to_owned();
    let mut numero = dernier_numero(&texte);
    let mut ecrites = 0u32;
    let mut refusees = Vec::new();
    // L'en-tête se reconnaît à sa première cellule ; un classeur sans en-tête est lu en entier.
    let saute = usize::from(
        lignes
            .first()
            .and_then(|l| l.first())
            .is_some_and(|c| une_ligne(c).eq_ignore_ascii_case("question")),
    );
    for (i, cellules) in lignes.iter().enumerate().skip(saute) {
        if cellules.iter().all(|c| c.trim().is_empty()) {
            continue;
        }
        let ligne = u32::try_from(i.saturating_add(1)).unwrap_or(u32::MAX);
        if i >= LIGNES_IMPORT_MAX.saturating_add(saute) {
            refusees.push(LigneQuizRefusee {
                ligne,
                raison: format!("un import compte au plus {LIGNES_IMPORT_MAX} questions."),
            });
            continue;
        }
        match question_de_ligne(numero.saturating_add(1), cellules) {
            Ok((question, reponse)) => {
                numero = numero.saturating_add(1);
                texte = ajouter_question(&texte, tout_le_texte, &question, &reponse);
                ecrites = ecrites.saturating_add(1);
            }
            Err(raison) => refusees.push(LigneQuizRefusee { ligne, raison }),
        }
    }
    ImportQuiz {
        texte,
        ecrites,
        refusees,
    }
}

impl ServiceBibliotheque<'_> {
    /// Vrai si le contenu est un quiz de chapitre (tout son texte suit la convention).
    pub(super) async fn est_un_quiz(&self, session: &Session, contenu_id: &str) -> Resultat<bool> {
        let contexte = self.noyau.contexte(session.etablissement_id, Some(session));
        let mut cnx = self.connexion().await?;
        let contenu =
            gesco_db::depots::bibliotheque::contenu(&mut cnx, contenu_id, &contexte).await?;
        Ok(contenu.is_some_and(|c| c.nature == NatureContenu::Quiz))
    }

    /// Le bouton Question : le gabarit d'une question, avec sa ligne sous « Réponses ».
    pub async fn ajouter_question_quiz(
        &self,
        session: &Session,
        demande: &DemandeQuestionQuiz,
    ) -> Resultat<TexteQuiz> {
        if demande.texte.len() > TEXTE_MAX {
            return Err(regle("bibliotheque.texte_trop_long"));
        }
        let tout = self.est_un_quiz(session, &demande.contenu_id).await?;
        let (question, reponse) = gabarit_question(&demande.texte);
        Ok(TexteQuiz {
            texte: ajouter_question(&demande.texte, tout, &question, &reponse),
        })
    }

    /// Le modèle d'import : les colonnes et une ligne d'exemple.
    pub async fn ecrire_modele_quiz(&self, demande: DemandeModeleQuiz) -> Resultat<FichierEcrit> {
        let exemple = vec![
            "Combien font 7 × 8 ?".to_owned(),
            "54".to_owned(),
            "56".to_owned(),
            "64".to_owned(),
            "15".to_owned(),
            String::new(),
            String::new(),
            "B".to_owned(),
            "7 × 8 = 56. La réponse 15 vient de 7 + 8 : on a additionné au lieu de multiplier."
                .to_owned(),
        ];
        crate::export::exporter_xlsx(DemandeExport {
            chemin: demande.chemin,
            titre: "Questions".to_owned(),
            entetes: COLONNES_MODELE.iter().map(|c| (*c).to_owned()).collect(),
            lignes: vec![exemple],
        })
        .await
    }

    /// Importe les questions d'un classeur dans le texte de l'éditeur ; le brouillon s'enregistre
    /// ensuite comme toute modification.
    pub async fn importer_quiz(
        &self,
        session: &Session,
        demande: &DemandeImportQuiz,
    ) -> Resultat<ImportQuiz> {
        let tout = self.est_un_quiz(session, &demande.contenu_id).await?;
        let lignes = crate::import_eleves::lire_fichier(&demande.chemin)?
            .into_iter()
            .next()
            .map(|(_, l)| l)
            .unwrap_or_default();
        let import = importer_lignes(&demande.texte, tout, &lignes);
        if import.texte.len() > TEXTE_MAX {
            return Err(regle("bibliotheque.texte_trop_long"));
        }
        Ok(import)
    }
}

impl ServiceBibliotheque<'_> {
    /// « Réviser le chapitre » sur un poste : dix questions tirées au hasard des quiz des
    /// contenus que le catalogue montre dans ce chapitre ; toutes s'il y en a moins. Rien n'est
    /// écrit.
    pub async fn reviser_chapitre(
        &self,
        session: &Session,
        demande: &DemandeRevisionChapitre,
        graine: u64,
    ) -> Resultat<RevisionChapitre> {
        let catalogue = self.catalogue(session).await?;
        let chapitre = catalogue
            .niveaux
            .iter()
            .flat_map(|n| n.series.iter())
            .flat_map(|s| s.matieres.iter())
            .flat_map(|m| m.parties.iter())
            .flat_map(|p| p.chapitres.iter())
            .find(|c| c.id == demande.chapitre_id);
        let Some(chapitre) = chapitre else {
            return Ok(RevisionChapitre {
                titre: String::new(),
                questions: Vec::new(),
                images: Vec::new(),
            });
        };
        let mut questions = Vec::new();
        let mut images: Vec<super::ImageLecon> = Vec::new();
        for c in &chapitre.contenus {
            if !matches!(c.nature, NatureContenu::Lecon | NatureContenu::Quiz) {
                continue;
            }
            let lu = self
                .lire(
                    session,
                    &super::DemandeLectureContenu {
                        contenu_id: c.id.clone(),
                        vue: None,
                    },
                )
                .await?;
            if lu.quiz.is_empty() {
                continue;
            }
            questions.extend(
                lu.quiz
                    .into_iter()
                    .map(|q| crate::portail::quiz::QuestionEleve {
                        contenu_id: c.id.clone(),
                        version: String::new(),
                        question: q,
                    }),
            );
            for i in lu.images {
                if !images.iter().any(|x| x.cle == i.cle) {
                    images.push(i);
                }
            }
        }
        let questions = crate::portail::quiz::tirer(questions, graine)
            .into_iter()
            .map(|q| q.question)
            .collect();
        Ok(RevisionChapitre {
            titre: chapitre.titre.clone(),
            questions,
            images,
        })
    }
}

/// Corrige une réponse (essai sur un poste, rien n'est écrit).
pub fn corriger_reponse(demande: &DemandeCorrectionQuiz) -> bool {
    corriger(&demande.question, &demande.reponse)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn ligne(cellules: &[&str]) -> Vec<String> {
        cellules.iter().map(|c| (*c).to_owned()).collect()
    }

    #[test]
    fn vingt_lignes_dont_deux_fausses_donnent_dix_huit_questions() {
        let mut lignes = vec![ligne(&COLONNES_MODELE)];
        for i in 0..20 {
            let bonne = match i {
                6 => "D",
                13 => "",
                _ => "B",
            };
            lignes.push(ligne(&[
                &format!("Question numéro {i} ?"),
                "un",
                "deux",
                "trois",
                "",
                "",
                "",
                bonne,
                "Parce que.",
            ]));
        }
        let import = importer_lignes("", true, &lignes);
        assert_eq!(import.ecrites, 18);
        assert_eq!(
            import.refusees,
            vec![
                LigneQuizRefusee {
                    ligne: 8,
                    raison: "la bonne réponse D n'est pas parmi les choix.".to_owned()
                },
                LigneQuizRefusee {
                    ligne: 15,
                    raison: "la bonne réponse manque.".to_owned()
                },
            ]
        );
        let quiz = analyser_quiz(&import.texte, true);
        assert_eq!(quiz.questions.len(), 18);
        assert!(quiz.ecartees.is_empty());
        let numeros: Vec<u32> = quiz.questions.iter().map(|q| q.numero).collect();
        assert_eq!(numeros, (1..=18).collect::<Vec<_>>());
    }

    #[test]
    fn les_cinq_formes_d_une_ligne() {
        // Plusieurs bonnes réponses.
        let (q, r) = question_de_ligne(
            3,
            &ligne(&["Nombres pairs ?", "2", "3", "4", "", "", "", "A, C", ""]),
        )
        .unwrap();
        let quiz = analyser_quiz(&ajouter_question("", true, &q, &r), true);
        assert_eq!(quiz.questions[0].bonnes, vec!["A", "C"]);
        // Réponse courte avec deux réponses admises.
        let (q, r) = question_de_ligne(
            1,
            &ligne(&["7 / 2 ?", "", "", "", "", "", "", "3,5 ou 7/2", ""]),
        )
        .unwrap();
        let quiz = analyser_quiz(&ajouter_question("", true, &q, &r), true);
        assert!(corriger(&quiz.questions[0], "3.5"));
        assert!(corriger(&quiz.questions[0], "7/2"));
        // Vrai ou faux.
        let (q, r) = question_de_ligne(
            1,
            &ligne(&["0 est pair.", "Vrai", "Faux", "", "", "", "", "a", ""]),
        )
        .unwrap();
        let quiz = analyser_quiz(&ajouter_question("", true, &q, &r), true);
        assert!(corriger(&quiz.questions[0], "A"));
        // Un trou dans les choix, un seul choix, une question vide.
        assert_eq!(
            question_de_ligne(1, &ligne(&["Q ?", "un", "", "trois", "", "", "", "A", ""])),
            Err("le choix B est vide.".to_owned())
        );
        assert_eq!(
            question_de_ligne(1, &ligne(&["Q ?", "un", "", "", "", "", "", "A", ""])),
            Err("une question à choix en demande au moins deux.".to_owned())
        );
        assert_eq!(
            question_de_ligne(1, &ligne(&["", "un", "deux", "", "", "", "", "A", ""])),
            Err("la question est vide.".to_owned())
        );
    }

    #[test]
    fn l_import_s_ajoute_au_quiz_d_une_lecon() {
        let texte = "# Leçon\n\n## Cours\n\nTexte.\n\n## Quiz\n\n**Question 1.** Un ?\nA) 1  B) 2\n\n### Réponses\n\n1. **A.** Oui.\n";
        let lignes = vec![ligne(&["Deux ?", "1", "2", "", "", "", "", "B", "Oui."])];
        let import = importer_lignes(texte, false, &lignes);
        assert_eq!(import.ecrites, 1);
        let quiz = analyser_quiz(&import.texte, false);
        assert_eq!(quiz.questions.len(), 2);
        assert_eq!(quiz.questions[1].numero, 2);
    }
}
