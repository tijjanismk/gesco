import type { ArticleInventaire, FamilleInventaire, NiveauInventaire, QuantitesInventaire } from "../../types/gesco";

export type NomQuantite = keyof QuantitesInventaire;

/** Familles, dans l'ordre des onglets. */
export const FAMILLES: readonly FamilleInventaire[] = ["local", "mobilier", "materiel", "manuel", "adapte"];

const COLONNES_ARTICLES: readonly NomQuantite[] = ["existant", "bon_etat", "besoins"];

/** Colonnes saisies dans chaque famille ; les cabines s'ajoutent sur la ligne des latrines. */
export const COLONNES_FAMILLE: Readonly<Record<FamilleInventaire, readonly NomQuantite[]>> = {
  local: ["existant", "bon_etat", "besoins", "dur", "semi_dur", "banco", "autre_construction", "neufs", "en_location", "murs_bons", "murs_mauvais", "toits_bons", "toits_mauvais"],
  mobilier: COLONNES_ARTICLES,
  materiel: COLONNES_ARTICLES,
  manuel: COLONNES_ARTICLES,
  adapte: COLONNES_ARTICLES,
};

/** Colonnes des cabines, sur la ligne des latrines. */
export const COLONNES_LATRINES: readonly NomQuantite[] = ["cabines_filles", "cabines_garcons", "cabines_enseignants"];

/** Une ligne sans aucune quantité. */
export const QUANTITES_VIDES: QuantitesInventaire = {
  existant: null,
  bon_etat: null,
  besoins: null,
  dur: null,
  semi_dur: null,
  banco: null,
  autre_construction: null,
  neufs: null,
  en_location: null,
  murs_bons: null,
  murs_mauvais: null,
  toits_bons: null,
  toits_mauvais: null,
  cabines_filles: null,
  cabines_garcons: null,
  cabines_enseignants: null,
};

/** Quantité lue dans une case : vide reste vide, jamais zéro ; un nombre négatif ou à virgule
 *  devient l'entier positif le plus proche par en dessous. */
export function lireQuantite(texte: string): number | null {
  const propre = texte.trim();
  if (propre === "") return null;
  const n = Number(propre.replace(",", "."));
  if (!Number.isFinite(n)) return null;
  return Math.max(0, Math.floor(n));
}

/** Désignation d'un article, avec la matière et le niveau d'un manuel. */
export function libelleArticle(a: ArticleInventaire, niveaux: readonly NiveauInventaire[]): string {
  const niveau = a.niveau_id === null ? undefined : niveaux.find((n) => n.id === a.niveau_id)?.libelle;
  const precisions = [a.matiere, niveau].filter((x): x is string => x !== undefined && x !== "");
  return precisions.length === 0 ? a.designation : `${a.designation} (${precisions.join(", ")})`;
}
