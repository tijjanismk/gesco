import type { Cle } from "../../i18n";
import type { CritereLibre, PopulationLibre } from "../../types/gesco";

/** Critères proposés pour chaque population ; la fiche n'en a pas : une ligne par rubrique. */
export const CRITERES: Readonly<Record<PopulationLibre, readonly CritereLibre[]>> = {
  eleves: ["classe", "niveau", "cycle", "sexe", "age", "annee_recrutement"],
  personnel: ["statut", "categorie", "specialite", "diplome", "sexe", "cycle"],
  fiche: [],
};

export const TITRES_CRITERES: Readonly<Record<CritereLibre, Cle>> = {
  classe: "statistiques.libre.critere.classe",
  niveau: "statistiques.libre.critere.niveau",
  cycle: "statistiques.libre.critere.cycle",
  sexe: "statistiques.libre.critere.sexe",
  age: "statistiques.libre.critere.age",
  annee_recrutement: "statistiques.libre.critere.annee_recrutement",
  statut: "statistiques.libre.critere.statut",
  categorie: "statistiques.libre.critere.categorie",
  specialite: "statistiques.libre.critere.specialite",
  diplome: "statistiques.libre.critere.diplome",
};

export const TITRES_POPULATIONS: Readonly<Record<PopulationLibre, Cle>> = {
  eleves: "statistiques.libre.population.eleves",
  personnel: "statistiques.libre.population.personnel",
  fiche: "statistiques.libre.population.fiche",
};

/** Lignes et colonnes gardées quand la population change : celles qui s'y appliquent encore,
 *  sinon les premiers critères de la nouvelle population. */
export function choixApres(population: PopulationLibre, lignes: CritereLibre, colonnes: CritereLibre | null): [CritereLibre, CritereLibre | null] {
  const criteres = CRITERES[population];
  if (criteres.length === 0) return [lignes, null];
  const l = criteres.includes(lignes) ? lignes : (criteres[0] ?? lignes);
  const c = colonnes !== null && colonnes !== l && criteres.includes(colonnes) ? colonnes : criteres.includes("sexe") && l !== "sexe" ? "sexe" : null;
  return [l, c];
}
