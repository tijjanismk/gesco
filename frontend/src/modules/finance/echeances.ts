/** Jours d'échéance permis : du 1er au 28, qui existe dans tous les mois. */
export const JOUR_MIN = 1;
export const JOUR_MAX = 28;

/** Lit un jour d'échéance saisi ; `null` hors du 1er au 28. */
export function lireJour(saisie: string): number | null {
  const texte = saisie.trim();
  if (!/^\d{1,2}$/.test(texte)) return null;
  const jour = Number(texte);
  return jour >= JOUR_MIN && jour <= JOUR_MAX ? jour : null;
}

/** La même échéance au jour choisi de son mois (`AAAA-MM-JJ`) ; une date vide le reste. */
export function auJour(echeance: string, jour: number): string {
  const morceaux = /^(\d{4})-(\d{2})-\d{2}$/.exec(echeance);
  if (morceaux === null) return echeance;
  return `${morceaux[1] ?? ""}-${morceaux[2] ?? ""}-${String(jour).padStart(2, "0")}`;
}
