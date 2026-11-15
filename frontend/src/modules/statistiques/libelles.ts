import type { Cle } from "../../i18n";
import type { DegreDifficulte, DiplomeEleve, Examen, Fonctionnement, PedagogieClasse, Specialite, StatutRapport, ThemeRubrique, TypeDifficulte, TypeReponse } from "../../types/gesco";

// Libellés des valeurs des énumérations de la phase 9, écrits en toutes lettres : le catalogue
// vérifie qu'aucune de ses clés ne dort.

export const TITRES_FONCTIONNEMENTS: Readonly<Record<Fonctionnement, Cle>> = {
  simple: "complements.fonctionnement.simple",
  double_division: "complements.fonctionnement.double_division",
  double_vacation: "complements.fonctionnement.double_vacation",
};

export const TITRES_PEDAGOGIES: Readonly<Record<PedagogieClasse, Cle>> = {
  classique: "complements.pedagogie.classique",
  pedagogie_convergente: "complements.pedagogie.pedagogie_convergente",
  curriculum: "complements.pedagogie.curriculum",
};

export const TITRES_STATUTS: Readonly<Record<StatutRapport, Cle>> = {
  fonctionnaire_etat: "complements.statut.fonctionnaire_etat",
  fonctionnaire_collectivites: "complements.statut.fonctionnaire_collectivites",
  contractuel_etat: "complements.statut.contractuel_etat",
  contractuel_collectivites: "complements.statut.contractuel_collectivites",
  contractuel_ecole: "complements.statut.contractuel_ecole",
  vacataire: "complements.statut.vacataire",
  eleve_maitre: "complements.statut.eleve_maitre",
  autre: "complements.statut.autre",
};

export const TITRES_SPECIALITES: Readonly<Record<Specialite, Cle>> = {
  lhg: "complements.specialite.lhg",
  mpc: "complements.specialite.mpc",
  snpc: "complements.specialite.snpc",
  ldm: "complements.specialite.ldm",
  eps: "complements.specialite.eps",
  autre: "complements.specialite.autre",
};

export const TITRES_DIPLOMES: Readonly<Record<DiplomeEleve, Cle>> = {
  doctorat: "complements.diplome.doctorat",
  master: "complements.diplome.master",
  dea: "complements.diplome.dea",
  ensup: "complements.diplome.ensup",
  ingenieur: "complements.diplome.ingenieur",
  maitrise: "complements.diplome.maitrise",
  licence: "complements.diplome.licence",
  deug_bts: "complements.diplome.deug_bts",
  bt: "complements.diplome.bt",
  baccalaureat: "complements.diplome.baccalaureat",
  cap: "complements.diplome.cap",
  ifm: "complements.diplome.ifm",
  autre: "complements.diplome.autre",
};

export const TITRES_DIFFICULTES: Readonly<Record<TypeDifficulte, Cle>> = {
  vue: "complements.difficulte.vue",
  ouie: "complements.difficulte.ouie",
  motricite_globale: "complements.difficulte.motricite_globale",
  motricite_fine: "complements.difficulte.motricite_fine",
  intellect: "complements.difficulte.intellect",
  communication: "complements.difficulte.communication",
  comportement: "complements.difficulte.comportement",
  multiple: "complements.difficulte.multiple",
  autre: "complements.difficulte.autre",
};

export const TITRES_DEGRES: Readonly<Record<DegreDifficulte, Cle>> = {
  quelques: "complements.degre.quelques",
  beaucoup: "complements.degre.beaucoup",
  impossible: "complements.degre.impossible",
};

export const TITRES_THEMES: Readonly<Record<ThemeRubrique, Cle>> = {
  localisation: "fiche.theme.localisation",
  identite: "fiche.theme.identite",
  actes: "fiche.theme.actes",
  eau_hygiene: "fiche.theme.eau_hygiene",
  energie: "fiche.theme.energie",
  equipements: "fiche.theme.equipements",
  vie_ecole: "fiche.theme.vie_ecole",
  accessibilite: "fiche.theme.accessibilite",
  autres: "fiche.theme.autres",
};

export const TITRES_TYPES_REPONSE: Readonly<Record<TypeReponse, Cle>> = {
  oui_non: "fiche.type.oui_non",
  nombre: "fiche.type.nombre",
  texte: "fiche.type.texte",
  date: "fiche.type.date",
  choix: "fiche.type.choix",
  plusieurs_choix: "fiche.type.plusieurs_choix",
};

export const TITRES_EXAMENS: Readonly<Record<Examen, Cle>> = {
  entree_7e: "examens.examen.entree_7e",
  def: "examens.examen.def",
  bac: "examens.examen.bac",
  cap: "examens.examen.cap",
  bt: "examens.examen.bt",
};
