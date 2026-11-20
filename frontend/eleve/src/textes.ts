// Textes des écrans des élèves : tutoiement, phrases courtes, aucun mot technique.
import type { EntreeEleve, EtatPourEleve, RefusRendu } from "./types";

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
  mesDevoirs: "Mes devoirs",
  aucunDevoir: "Aucun devoir pour le moment.",
  entrerPourRendre: "Pour rendre un devoir, entre avec ton code.",
  devoirIntrouvable: "Ce devoir n'est plus là. Reviens à tes devoirs.",
  aRendrePapier: "À rendre sur papier à ton maître",
  devoirClos: "Ce devoir est clos",
  rendreMonDevoir: "Rendre mon devoir",
  rendreDeNouveau: "Rendre de nouveau",
  taReponseDevoir: "Ta réponse",
  ajouterPhoto: "Ajouter une photo",
  retirer: "Retirer",
  envoiEnCours: "Envoi en cours…",
  pasParti: "Ton devoir n'est pas parti. Le Wi-Fi ou l'ordinateur de l'école ne répond pas.",
  reessayer: "Réessayer",
  apresLimite: "Rendu après la date limite",
  mesRendus: "Ce que tu as rendu",
  photoRetiree: "Photo retirée pour libérer de la place",
  surPapier: "Rendu sur papier",
  correction: "Correction",
  commentaireMaitre: "Mot du maître",
  leconLiee: "Leçon à revoir",
  reduction: "Préparation de la photo…",
  photoIllisible: "Cette photo ne s'ouvre pas. Reprends-la.",
  brouillonGarde: "Ton texte est gardé sur ce téléphone tant que tu n'as pas rendu.",
} as const;

/** « 12 mars à 10 h 40 ». */
export function dateEtHeure(ms: number): string {
  const d = new Date(ms);
  const jour = d.toLocaleDateString("fr-FR", { day: "numeric", month: "long" });
  const minutes = String(d.getMinutes()).padStart(2, "0");
  return `${jour} à ${d.getHours()} h ${minutes}`;
}

/** Une note en centièmes : « 14,5 sur 20 ». */
export function noteSur(note: number, bareme: number): string {
  const ecrire = (n: number) => (n / 100).toLocaleString("fr-FR", { maximumFractionDigits: 2 });
  return `${ecrire(note)} sur ${ecrire(bareme)}`;
}

/** L'état d'un devoir dit en mots. */
export function etatDevoir(etat: EtatPourEleve, renduLe: number | null, apresLimite: boolean): string {
  switch (etat) {
    case "a_rendre":
      return "À rendre";
    case "en_retard":
      return "En retard";
    case "rendu":
      if (renduLe === null) return "Rendu";
      return apresLimite ? `Rendu le ${dateEtHeure(renduLe)}, après la date limite` : `Rendu le ${dateEtHeure(renduLe)}`;
    case "corrige":
      return "Corrigé";
    case "clos":
      return "Clos";
    case "papier":
      return "À rendre sur papier";
  }
}

/** Pourquoi un rendu n'est pas reçu, en une phrase. */
export function refusRendu(refus: RefusRendu): string {
  switch (refus) {
    case "sans_code":
      return "Pour rendre un devoir, entre avec ton code.";
    case "clos":
      return "Ce devoir est clos : tu ne peux plus le rendre.";
    case "papier":
      return "Ce devoir se rend sur papier à ton maître.";
    case "vide":
      return "Écris ta réponse ou ajoute une photo de ta copie.";
    case "texte_non_permis":
      return "Pour ce devoir, envoie seulement des photos de ta copie.";
    case "photos_non_permises":
      return "Pour ce devoir, écris seulement ta réponse.";
    case "trop_de_photos":
      return "Cinq photos au plus.";
    case "pas_une_photo":
      return "Ce fichier n'est pas une photo. Prends ta copie en photo.";
    case "photo_trop_grosse":
      return "Cette photo est trop lourde. Reprends-la de plus loin.";
    case "texte_long":
      return "Ta réponse est trop longue.";
  }
}

/** « Ton devoir est rendu le 12 mars à 10 h 40. » */
export function renduLe(ms: number): string {
  return `Ton devoir est rendu le ${dateEtHeure(ms)}.`;
}

/** « À rendre le 12 mars à 23 h 59 ». */
export function aRendreLe(ms: number): string {
  return `À rendre le ${dateEtHeure(ms)}`;
}

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
