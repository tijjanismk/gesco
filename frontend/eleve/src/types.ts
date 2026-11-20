// Ce que le poste principal envoie aux écrans des élèves (`gesco-services/src/portail`).
import type { Lecon, QuestionQuiz } from "../../src/types/gesco";

export type { Bloc, ChoixQuestion, Enligne, QuestionQuiz } from "../../src/types/gesco";

type VueEleve = {
  prenom: string | null;
  classe: string | null;
  niveau: string;
  avec_code: boolean;
};

export type NiveauALire = { niveau_id: string; libelle: string; serie_id: string | null };

export type Accueil = {
  ecole: string;
  eleve: VueEleve | null;
  niveaux: NiveauALire[];
  suivi: boolean;
  videos: boolean;
  pdf: boolean;
};

type Nature = "lecon" | "sujet" | "corrige" | "document" | "quiz";

export type EntreeEleve = {
  id: string;
  titre: string;
  nature: Nature;
  examen: "def" | "bac" | null;
  session: number | null;
};

type ChapitreEleve = { id: string; titre: string; contenus: EntreeEleve[] };
type PartieEleve = { titre: string; chapitres: ChapitreEleve[] };
type MatiereEleve = { cle: string; libelle: string; lecons: number; parties: PartieEleve[] };
export type CatalogueEleve = { niveau: string; matieres: MatiereEleve[] };

export type ImageEleve = { cle: string; empreinte: string };
type MediaEleve = { empreinte: string; type_mime: string };
type FichierEleve = { empreinte: string; nom: string; type_mime: string; taille: number };
type LienEleve = { id: string; titre: string; nature: Nature };

export type ContenuEleve = EntreeEleve & {
  matiere_cle: string;
  matiere: string;
  chapitre: string;
  lecon: Lecon;
  images: ImageEleve[];
  sons: MediaEleve[];
  videos: MediaEleve[];
  fichiers: FichierEleve[];
  lie: LienEleve | null;
  version: string;
  pdf: boolean;
  /** Questions reconnues du quiz : « Faire le quiz » se montre s'il y en a. */
  questions: number;
};

/** Une question d'un quiz, avec le contenu et la version d'où elle vient. */
export type QuestionEleve = { contenu_id: string; version: string; question: QuestionQuiz };

/** Le quiz d'une leçon, ou la révision d'un chapitre. */
export type QuizEleve = {
  titre: string;
  contenu_id: string | null;
  chapitre_id: string | null;
  questions: QuestionEleve[];
  images: ImageEleve[];
};

/** Avancement d'un élève entré avec son code, quand le suivi est allumé. */
type MatiereAvancement = { cle: string; libelle: string; finies: number; visibles: number };
type LeconRecente = { id: string; titre: string; matiere: string };
export type ScoreRecent = {
  titre: string;
  score: number;
  sur: number;
  finie_le: number;
};

export type MonAvancement = {
  matieres: MatiereAvancement[];
  recentes: LeconRecente[];
  finies: string[];
  scores: ScoreRecent[];
};

/** Où en est un devoir pour l'élève. */
export type EtatPourEleve = "a_rendre" | "en_retard" | "rendu" | "corrige" | "clos" | "papier";
export type FormeRendu = "texte" | "photos" | "texte_ou_photos" | "papier";

/** Un devoir dans « Mes devoirs ». */
export type DevoirDeLEleve = {
  id: string;
  titre: string;
  matiere: string;
  date_limite: number;
  forme_rendu: FormeRendu;
  etat: EtatPourEleve;
  rendu_le: number | null;
  apres_limite: boolean;
};

export type MesDevoirs = { avec_code: boolean; devoirs: DevoirDeLEleve[] };

type PhotoEleve = { ordre: number; retiree: boolean };

export type RenduEleve = {
  id: string;
  numero: number;
  rendu_le: number;
  apres_limite: boolean;
  sur_papier: boolean;
  texte: string;
  photos: PhotoEleve[];
};

/** Une image des consignes, envoyée avec le devoir. */
type ImageDevoir = { cle: string; type_mime: string; donnees: string };

/** Un devoir ouvert par l'élève. */
export type DevoirOuvertEleve = {
  id: string;
  titre: string;
  matiere: string;
  date_limite: number;
  bareme: number;
  forme_rendu: FormeRendu;
  clos: boolean;
  etat: EtatPourEleve;
  lecon: Lecon;
  images: ImageDevoir[];
  lecon_liee: { id: string; titre: string } | null;
  rendus: RenduEleve[];
  correction: { note: number; bareme: number; commentaire: string } | null;
};

/** Pourquoi un rendu n'est pas reçu. */
export type RefusRendu =
  | "sans_code"
  | "clos"
  | "papier"
  | "vide"
  | "texte_non_permis"
  | "photos_non_permises"
  | "trop_de_photos"
  | "pas_une_photo"
  | "photo_trop_grosse"
  | "texte_long";

export type RenduFait = { numero: number; rendu_le: number; apres_limite: boolean };
