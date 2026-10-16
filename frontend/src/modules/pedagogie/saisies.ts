/** Points et heures tels que l'école les écrit, convertis en entiers (RC-6) : aucune note ne
 *  passe par un nombre à virgule. */

/** « 13,5 », « 13.50 », « 20 » → centièmes ; rien d'autre n'est accepté. */
export function lirePoints(texte: string): number | null {
  const m = /^(\d{1,3})(?:[,.](\d{1,2}))?$/.exec(texte.trim());
  if (m === null) return null;
  const entier = Number.parseInt(m[1] ?? "0", 10);
  const decimales = (m[2] ?? "").padEnd(2, "0");
  return entier * 100 + Number.parseInt(decimales, 10);
}

/** Centièmes → « 13,5 », « 20 », « 9,25 ». */
export function points(centiemes: number): string {
  const entier = Math.trunc(centiemes / 100);
  const reste = centiemes % 100;
  if (reste === 0) return String(entier);
  return `${entier},${String(reste).padStart(2, "0").replace(/0$/, "")}`;
}

/** « 4 », « 4,5 », « 4h30 », « 4 h 30 » → minutes. */
export function lireHeures(texte: string): number | null {
  const t = texte.trim().toLowerCase();
  const hm = /^(\d{1,2})\s*h\s*(\d{1,2})?$/.exec(t);
  if (hm !== null) return Number.parseInt(hm[1] ?? "0", 10) * 60 + Number.parseInt(hm[2] ?? "0", 10);
  const dec = /^(\d{1,2})(?:[,.](\d))?$/.exec(t);
  if (dec === null) return null;
  return Number.parseInt(dec[1] ?? "0", 10) * 60 + Number.parseInt(dec[2] ?? "0", 10) * 6;
}

/** Minutes → « 4 h 30 », « 12 h ». */
export function heures(minutes: number): string {
  const h = Math.trunc(minutes / 60);
  const m = minutes % 60;
  return m === 0 ? `${h} h` : `${h} h ${String(m).padStart(2, "0")}`;
}

/** Case du relevé des absences : rien (présent), « a » (absent), « r » ou « r15 » (retard, en
 *  minutes). Même lecture que le noyau ; `null` pour une case illisible. */
export type CaseAbsence = { nature: "present" } | { nature: "absence" } | { nature: "retard"; minutes: number | null };

export function lireAbsence(texte: string): CaseAbsence | null {
  const t = texte.toLowerCase().replaceAll(/\s/g, "");
  if (t === "" || t === "p") return { nature: "present" };
  if (t === "a" || t === "abs") return { nature: "absence" };
  if (t === "r") return { nature: "retard", minutes: null };
  const m = /^r(\d{1,3})$/.exec(t);
  if (m === null) return null;
  const minutes = Number.parseInt(m[1] ?? "0", 10);
  return minutes >= 1 && minutes <= 240 ? { nature: "retard", minutes } : null;
}

/** Centièmes → « 13,35 », « 7,50 » : une moyenne s'écrit toujours avec deux décimales. */
export function moyenne(centiemes: number): string {
  const signe = centiemes < 0 ? "-" : "";
  const v = Math.abs(centiemes);
  return `${signe}${Math.trunc(v / 100)},${String(v % 100).padStart(2, "0")}`;
}

/** Rang imprimé : « 1er », « 2e », « 3e ex ». */
export function texteRang(rang: { rang: number; ex_aequo: boolean } | null): string {
  if (rang === null) return "";
  const base = rang.rang === 1 ? "1er" : `${rang.rang}e`;
  return rang.ex_aequo ? `${base} ex` : base;
}
