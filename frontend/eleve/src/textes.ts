// Textes des écrans des élèves : tutoiement, phrases courtes, aucun mot technique.
import type { EntreeEleve } from "./types";

export const T = {
  titreEntree: "Les cours de ton école",
  matricule: "Matricule",
  code: "Code",
  pasMonTelephone: "Ce n'est pas mon téléphone",
  entrer: "Entrer",
  lireSansCode: "Lire sans code",
  avancementPasGarde: "Ton avancement n'est pas gardé.",
  choisisNiveau: "Choisis ta classe",
  refusIncorrect: "Matricule ou code incorrect. Demande ton code au secrétariat.",
  refusBloque: "Trop d'essais pour ce matricule aujourd'hui. Tu peux lire sans code.",
  occupe: "Beaucoup d'élèves sont connectés. Réessaie dans un instant.",
  sortir: "Sortir",
  mesMatieres: "Mes matières",
  reprendre: "Reprendre",
  aucuneLecon: "Les leçons de ta classe arrivent bientôt.",
  monAvancement: "Mon avancement",
  retour: "Retour",
  voirCorriges: "Voir les corrigés",
  voirReponses: "Voir les réponses",
  jaiFini: "J'ai fini cette leçon",
  faireLeQuiz: "Faire le quiz",
  reviserChapitre: "Réviser le chapitre",
  verifier: "Vérifier",
  juste: "Juste",
  pasToutAFait: "Pas tout à fait",
  bonneReponse: "La bonne réponse :",
  questionSuivante: "Question suivante",
  voirScore: "Voir mon score",
  recommencer: "Recommencer",
  taReponse: "Ta réponse",
  choisir: "Choisir…",
  plusieurs: "Plusieurs réponses peuvent être justes.",
  ou: "ou",
  pasDeQuestion: "Ce quiz n'a pas encore de question.",
  surTotal: (justes: number, total: number) => `${justes} sur ${total}`,
  questionN: (n: number) => `Question ${n}`,
  questionSur: (n: number, total: number) => `Question ${n} sur ${total}`,
  cestNote: "Bravo, c'est noté.",
  enregistrerPdf: "Enregistrer en PDF",
  taillePlus: "Agrandir le texte",
  tailleMoins: "Réduire le texte",
  finie: "Finie",
  ouvrir: "Ouvrir",
  leconsRecentes: "Dernières leçons ouvertes",
  derniersQuiz: "Mes derniers quiz",
  chargement: "Un instant…",
  introuvable: "Cette leçon n'est plus là. Reviens à tes matières.",
} as const;

export function lecons(n: number): string {
  return n === 1 ? "1 leçon" : `${n} leçons`;
}

export function finies(n: number): string {
  return n === 1 ? "1 finie" : `${n} finies`;
}

export function finiesSur(finies: number, visibles: number): string {
  return `${finies} sur ${visibles} ${visibles === 1 ? "leçon finie" : "leçons finies"}`;
}

/** Nature d'un contenu : « Leçon », « Sujet du DEF 2024 », « Corrigé », « Document ». */
export function nature(c: Pick<EntreeEleve, "nature" | "examen" | "session">): string {
  const examen = c.examen === "def" ? "DEF" : c.examen === "bac" ? "BAC" : null;
  const session = c.session === null ? "" : ` ${c.session}`;
  switch (c.nature) {
    case "lecon":
      return "Leçon";
    case "document":
      return "Document";
    case "quiz":
      return "Quiz";
    case "sujet":
      return examen === null ? "Sujet" : `Sujet du ${examen}${session}`;
    case "corrige":
      return examen === null ? "Corrigé" : `Corrigé du ${examen}${session}`;
  }
}

/** Lien vers le contenu lié : « Voir le corrigé », « Voir le sujet ». */
export function voirLie(n: EntreeEleve["nature"]): string {
  return n === "corrige" ? "Voir le corrigé" : n === "sujet" ? "Voir le sujet" : "Voir aussi";
}
