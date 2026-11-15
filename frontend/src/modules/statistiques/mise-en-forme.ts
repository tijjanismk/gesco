import { type Cle, dateCivile, t, texteDeCle } from "../../i18n";
import type { CelluleStatistique, FeuilleStatistique, LigneStatistique, TableauStatistique } from "../../types/gesco";

/** Titre de chaque tableau et de sa première colonne, par code du tableau. */
const TABLEAUX: Readonly<Record<string, readonly [Cle, Cle]>> = {
  eleves_par_classe: ["statistiques.tableau.eleves_par_classe", "statistiques.premiere.eleves_par_classe"],
  eleves_par_niveau: ["statistiques.tableau.eleves_par_niveau", "statistiques.premiere.eleves_par_niveau"],
  eleves_par_cycle: ["statistiques.tableau.eleves_par_cycle", "statistiques.premiere.eleves_par_cycle"],
  redoublants_par_niveau: ["statistiques.tableau.redoublants_par_niveau", "statistiques.premiere.redoublants_par_niveau"],
  eleves_par_annee_naissance: ["statistiques.tableau.eleves_par_annee_naissance", "statistiques.premiere.eleves_par_annee_naissance"],
  eleves_par_age: ["statistiques.tableau.eleves_par_age", "statistiques.premiere.eleves_par_age"],
  nouveaux_par_niveau: ["statistiques.tableau.nouveaux_par_niveau", "statistiques.premiere.nouveaux_par_niveau"],
  entrees_depuis_rentree: ["statistiques.tableau.entrees_depuis_rentree", "statistiques.premiere.entrees_depuis_rentree"],
  departs_depuis_rentree: ["statistiques.tableau.departs_depuis_rentree", "statistiques.premiere.departs_depuis_rentree"],
  personnel_par_categorie: ["statistiques.tableau.personnel_par_categorie", "statistiques.premiere.personnel_par_categorie"],
  personnel_par_situation: ["statistiques.tableau.personnel_par_situation", "statistiques.premiere.personnel_par_situation"],
  personnel_par_engagement: ["statistiques.tableau.personnel_par_engagement", "statistiques.premiere.personnel_par_engagement"],
  enseignants_par_cycle: ["statistiques.tableau.enseignants_par_cycle", "statistiques.premiere.enseignants_par_cycle"],
  enseignants_par_discipline: ["statistiques.tableau.enseignants_par_discipline", "statistiques.premiere.enseignants_par_discipline"],
  heures_par_discipline: ["statistiques.tableau.heures_par_discipline", "statistiques.premiere.heures_par_discipline"],
  liste_nominative: ["statistiques.tableau.liste_nominative", "statistiques.premiere.liste_nominative"],
  eleves_par_annee_recrutement: ["statistiques.tableau.eleves_par_annee_recrutement", "statistiques.premiere.eleves_par_annee_recrutement"],
  eleves_pris_en_charge: ["statistiques.tableau.eleves_pris_en_charge", "statistiques.premiere.eleves_pris_en_charge"],
  eleves_handicapes: ["statistiques.tableau.eleves_handicapes", "statistiques.premiere.eleves_handicapes"],
  personnel_par_statut: ["statistiques.tableau.personnel_par_statut", "statistiques.premiere.personnel_par_statut"],
  enseignants_par_specialite: ["statistiques.tableau.enseignants_par_specialite", "statistiques.premiere.enseignants_par_specialite"],
  enseignants_par_diplome: ["statistiques.tableau.enseignants_par_diplome", "statistiques.premiere.enseignants_par_diplome"],
  inventaire_locaux: ["statistiques.tableau.inventaire_locaux", "inventaire.article"],
  inventaire_latrines: ["statistiques.tableau.inventaire_latrines", "inventaire.article"],
  inventaire_mobilier: ["statistiques.tableau.inventaire_mobilier", "inventaire.article"],
  inventaire_materiel: ["statistiques.tableau.inventaire_materiel", "inventaire.article"],
  inventaire_manuels: ["statistiques.tableau.inventaire_manuels", "inventaire.manuel"],
  inventaire_adapte: ["statistiques.tableau.inventaire_adapte", "inventaire.article"],
  decisions_par_niveau: ["statistiques.tableau.decisions_par_niveau", "statistiques.premiere.eleves_par_niveau"],
  resultats_examens: ["statistiques.tableau.resultats_examens", "examens.examen"],
  taux_admis_par_cycle: ["statistiques.tableau.taux_admis_par_cycle", "statistiques.premiere.eleves_par_cycle"],
  effectifs_probables: ["statistiques.tableau.effectifs_probables", "statistiques.premiere.eleves_par_niveau"],
  libre_classe: ["statistiques.tableau.libre", "statistiques.libre.critere.classe"],
  libre_niveau: ["statistiques.tableau.libre", "statistiques.libre.critere.niveau"],
  libre_cycle: ["statistiques.tableau.libre", "statistiques.libre.critere.cycle"],
  libre_sexe: ["statistiques.tableau.libre", "statistiques.libre.critere.sexe"],
  libre_age: ["statistiques.tableau.libre", "statistiques.libre.critere.age"],
  libre_annee_recrutement: ["statistiques.tableau.libre", "statistiques.libre.critere.annee_recrutement"],
  libre_statut: ["statistiques.tableau.libre", "statistiques.libre.critere.statut"],
  libre_categorie: ["statistiques.tableau.libre", "statistiques.libre.critere.categorie"],
  libre_specialite: ["statistiques.tableau.libre", "statistiques.libre.critere.specialite"],
  libre_diplome: ["statistiques.tableau.libre", "statistiques.libre.critere.diplome"],
  libre_rubrique: ["statistiques.tableau.libre", "statistiques.libre.rubrique"],
};

/** Titre de chaque colonne, par code de colonne. */
const COLONNES: Readonly<Record<string, Cle>> = {
  garcons: "statistiques.colonne.garcons",
  filles: "statistiques.colonne.filles",
  hommes: "statistiques.colonne.hommes",
  femmes: "statistiques.colonne.femmes",
  total: "statistiques.colonne.total",
  heures_hebdomadaires: "statistiques.colonne.heures_hebdomadaires",
  matricule: "statistiques.colonne.matricule",
  situation_famille: "statistiques.colonne.situation_famille",
  categorie: "statistiques.colonne.categorie",
  fonction: "statistiques.colonne.fonction",
  fonctionnaire: "statistiques.colonne.fonctionnaire",
  diplome: "statistiques.colonne.diplome",
  arrive_le: "statistiques.colonne.arrive_le",
  existant: "inventaire.colonne.existant",
  bon_etat: "inventaire.colonne.bon_etat",
  besoins: "inventaire.colonne.besoins",
  dur: "inventaire.colonne.dur",
  semi_dur: "inventaire.colonne.semi_dur",
  banco: "inventaire.colonne.banco",
  autre_construction: "inventaire.colonne.autre_construction",
  neufs: "inventaire.colonne.neufs",
  en_location: "inventaire.colonne.en_location",
  murs_bons: "inventaire.colonne.murs_bons",
  murs_mauvais: "inventaire.colonne.murs_mauvais",
  toits_bons: "inventaire.colonne.toits_bons",
  toits_mauvais: "inventaire.colonne.toits_mauvais",
  cabines_filles: "inventaire.colonne.cabines_filles",
  cabines_garcons: "inventaire.colonne.cabines_garcons",
  cabines_enseignants: "inventaire.colonne.cabines_enseignants",
  observation: "inventaire.colonne.observation",
  effectif_garcons: "statistiques.colonne.effectif_garcons",
  effectif_filles: "statistiques.colonne.effectif_filles",
  effectif_total: "statistiques.colonne.effectif_total",
  passages_garcons: "statistiques.colonne.passages_garcons",
  passages_filles: "statistiques.colonne.passages_filles",
  passages_total: "statistiques.colonne.passages_total",
  redoublements_garcons: "statistiques.colonne.redoublements_garcons",
  redoublements_filles: "statistiques.colonne.redoublements_filles",
  redoublements_total: "statistiques.colonne.redoublements_total",
  exclusions_garcons: "statistiques.colonne.exclusions_garcons",
  exclusions_filles: "statistiques.colonne.exclusions_filles",
  exclusions_total: "statistiques.colonne.exclusions_total",
  inscrits_garcons: "statistiques.colonne.inscrits_garcons",
  inscrits_filles: "statistiques.colonne.inscrits_filles",
  inscrits_total: "statistiques.colonne.inscrits_total",
  presentes_garcons: "statistiques.colonne.presentes_garcons",
  presentes_filles: "statistiques.colonne.presentes_filles",
  presentes_total: "statistiques.colonne.presentes_total",
  admis_garcons: "statistiques.colonne.admis_garcons",
  admis_filles: "statistiques.colonne.admis_filles",
  admis_total: "statistiques.colonne.admis_total",
  taux_garcons: "statistiques.colonne.taux_garcons",
  taux_filles: "statistiques.colonne.taux_filles",
  taux_total: "statistiques.colonne.taux_total",
  decides: "statistiques.colonne.decides",
  admis: "statistiques.colonne.admis",
  taux: "statistiques.colonne.taux",
  reponse: "statistiques.colonne.reponse",
};

/** Titre d'un tableau. */
export function titreTableau(code: string): string {
  const cles = TABLEAUX[code];
  return cles === undefined ? code : t(cles[0]);
}

/** Titre de la première colonne d'un tableau. */
export function titrePremiere(code: string): string {
  const cles = TABLEAUX[code];
  return cles === undefined ? "" : t(cles[1]);
}

/** Titre d'une colonne. Le tableau libre nomme ses colonnes d'une clé de texte (`cle:…`) ou d'un
 *  libellé tiré des données (`libre:…`). */
export function titreColonne(code: string): string {
  if (code.startsWith("cle:")) return texteDeCle(code.slice(4));
  if (code.startsWith("libre:")) return code.slice(6);
  const cle = COLONNES[code];
  return cle === undefined ? code : t(cle);
}

/** Durée hebdomadaire lisible : « 3 h », « 1 h 30 ». */
export function heures(minutes: number): string {
  const h = Math.floor(minutes / 60);
  const m = minutes % 60;
  return m === 0 ? `${h} h` : `${h} h ${String(m).padStart(2, "0")}`;
}

/** Libellé d'une ligne : son nom fixe traduit, sinon le libellé tiré des données. */
export function libelleLigne(l: LigneStatistique): string {
  return l.cle === null ? l.libelle : texteDeCle(l.cle);
}

/** Texte d'une case selon sa colonne. Une case vide reste vide : rien ne la signale. */
export function texteCellule(colonne: string, c: CelluleStatistique): string {
  if (c.nombre !== null) return colonne === "heures_hebdomadaires" ? heures(c.nombre) : String(c.nombre);
  if (c.texte === null) return "";
  switch (colonne) {
    case "situation_famille":
      return texteDeCle(`personnel.situation.${c.texte}`);
    case "fonctionnaire":
      return c.texte === "oui" ? t("commun.oui") : t("commun.non");
    case "arrive_le":
      return dateCivile(c.texte);
    default:
      return c.texte;
  }
}

/** Feuille Excel d'un tableau, telle que l'écran le montre : sans la ligne neutre des fiches
 *  hors colonnes, qui n'existe qu'à l'écran. */
export function feuilleDe(tableau: TableauStatistique): FeuilleStatistique {
  const ligne = (l: LigneStatistique) => [libelleLigne(l), ...l.cellules.map((c, i) => texteCellule(tableau.colonnes[i] ?? "", c))];
  return {
    titre: titreTableau(tableau.code),
    entetes: [titrePremiere(tableau.code), ...tableau.colonnes.map(titreColonne)],
    lignes: [...tableau.lignes.map(ligne), ...(tableau.total === null ? [] : [ligne(tableau.total)])],
  };
}
