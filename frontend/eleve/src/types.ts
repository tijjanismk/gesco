// Ce que le poste principal envoie aux écrans des élèves (`gesco-services/src/portail`).
import type { Lecon } from "../../src/types/gesco";

export type { Bloc, Enligne, Lecon } from "../../src/types/gesco";

export type VueEleve = {
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

export type Nature = "lecon" | "sujet" | "corrige" | "document";

export type EntreeEleve = {
  id: string;
  titre: string;
  nature: Nature;
  examen: "def" | "bac" | null;
  session: number | null;
};

export type ChapitreEleve = { id: string; titre: string; contenus: EntreeEleve[] };
export type PartieEleve = { titre: string; chapitres: ChapitreEleve[] };
export type MatiereEleve = { cle: string; libelle: string; lecons: number; parties: PartieEleve[] };
export type CatalogueEleve = { niveau: string; matieres: MatiereEleve[] };

export type ImageEleve = { cle: string; empreinte: string };
export type MediaEleve = { empreinte: string; type_mime: string };
export type FichierEleve = { empreinte: string; nom: string; type_mime: string; taille: number };
export type LienEleve = { id: string; titre: string; nature: Nature };

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
};

/** Avancement d'un élève entré avec son code, quand le suivi est allumé. */
export type MatiereAvancement = { cle: string; libelle: string; finies: number; visibles: number };
export type LeconRecente = { id: string; titre: string; matiere: string };
export type MonAvancement = {
  matieres: MatiereAvancement[];
  recentes: LeconRecente[];
  finies: string[];
};
