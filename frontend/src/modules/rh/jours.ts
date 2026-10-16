// Jours des congés du personnel (SPEC_PHASE_6 §6, Q-68), comptés comme le noyau.

const JOUR = 86_400_000;

/** Jours d'un congé, comptés comme le noyau : du lundi au vendredi, le samedi si l'école le
 *  compte ; rien si les dates manquent ou se suivent à l'envers. */
export function joursDeConge(debut: string, fin: string, samedi: boolean): number {
  const de = Date.parse(`${debut}T00:00:00Z`);
  const a = Date.parse(`${fin}T00:00:00Z`);
  if (Number.isNaN(de) || Number.isNaN(a) || a < de || a - de > 400 * JOUR) return 0;
  let n = 0;
  for (let d = de; d <= a; d += JOUR) {
    const j = new Date(d).getUTCDay();
    if ((j >= 1 && j <= 5) || (j === 6 && samedi)) n += 1;
  }
  return n;
}

/** Demi-journées écrites en jours : « 12 », « 12,5 », « −3,5 ». */
export function ecrireDemiJournees(demi: number): string {
  const signe = demi < 0 ? "−" : "";
  const d = Math.abs(demi);
  return `${signe}${Math.floor(d / 2)}${d % 2 === 0 ? "" : ",5"}`;
}
