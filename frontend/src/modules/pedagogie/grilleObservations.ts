import type { NiveauAcquisition } from "../../types/gesco";

/** Les trois niveaux, dans l'ordre de l'échelle et des touches 1, 2, 3. */
export const NIVEAUX: readonly NiveauAcquisition[] = ["acquis", "en_cours", "non_acquis"];

/** Pictogramme noir sur blanc de chaque niveau : jamais la couleur seule (RB-4). */
export const PICTOGRAMMES: Record<NiveauAcquisition, string> = { acquis: "●", en_cours: "◐", non_acquis: "○" };

/** Clé d'une case : l'enfant et la compétence. */
export function cleCase(inscription: string, competence: string): string {
  return `${inscription}:${competence}`;
}

function initiale(libelle: string): string {
  return libelle.trim().normalize("NFD").replace(/[̀-ͯ]/g, "").charAt(0).toUpperCase();
}

/**
 * Initiales des trois libellés réglés (« A », « E », « N » pour la liste livrée), qui posent le
 * niveau au clavier ; vide si deux libellés commencent par la même lettre : 1, 2, 3 suffisent.
 */
export function initiales(echelle: readonly string[]): string[] {
  const lettres = echelle.map(initiale);
  const distinctes = new Set(lettres);
  return lettres.length === 3 && distinctes.size === 3 && !lettres.some((l) => l === "" || /\d/.test(l)) ? lettres : [];
}

/** Niveau posé par une touche : 1, 2, 3, ou l'initiale d'un libellé ; `null` pour une autre. */
export function niveauDeTouche(touche: string, lettres: readonly string[]): NiveauAcquisition | null {
  const chiffre = ["1", "2", "3"].indexOf(touche);
  if (chiffre >= 0) return NIVEAUX[chiffre] ?? null;
  if (touche.length !== 1) return null;
  const rang = lettres.indexOf(initiale(touche));
  return rang >= 0 ? (NIVEAUX[rang] ?? null) : null;
}

/**
 * Case où va le curseur. « Suivante » (après un niveau posé ou Entrée) descend d'un enfant, puis
 * passe en haut de la compétence suivante ; les flèches se déplacent sans sortir de la grille.
 * `null` : le curseur ne bouge pas.
 */
export function caseVoisine(
  ligne: number,
  colonne: number,
  lignes: number,
  colonnes: number,
  sens: "suivante" | "ArrowDown" | "ArrowUp" | "ArrowLeft" | "ArrowRight",
): [number, number] | null {
  switch (sens) {
    case "suivante":
      if (ligne + 1 < lignes) return [ligne + 1, colonne];
      return colonne + 1 < colonnes ? [0, colonne + 1] : null;
    case "ArrowDown":
      return ligne + 1 < lignes ? [ligne + 1, colonne] : null;
    case "ArrowUp":
      return ligne > 0 ? [ligne - 1, colonne] : null;
    case "ArrowLeft":
      return colonne > 0 ? [ligne, colonne - 1] : null;
    case "ArrowRight":
      return colonne + 1 < colonnes ? [ligne, colonne + 1] : null;
  }
}
