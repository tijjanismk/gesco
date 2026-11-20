// Correction d'une réponse sur le téléphone, pour la montrer tout de suite. Elle suit pas à pas
// celle du poste principal (`gesco-core/src/bibliotheque/quiz.rs`), qui corrige de nouveau
// quand le quiz est fini : seul son score s'écrit.
import type { QuestionQuiz } from "./types";

const ACCENTS: Record<string, string> = {
  é: "e", è: "e", ê: "e", ë: "e", É: "e", È: "e", Ê: "e", Ë: "e",
  à: "a", â: "a", ä: "a", À: "a", Â: "a", Ä: "a",
  î: "i", ï: "i", Î: "i", Ï: "i",
  ô: "o", ö: "o", Ô: "o", Ö: "o",
  û: "u", ù: "u", ü: "u", Û: "u", Ù: "u", Ü: "u",
  ç: "c", Ç: "c",
  œ: "o", Œ: "o",
};

function simplifier(texte: string): string {
  return [...texte].map((c) => ACCENTS[c] ?? c.toLowerCase()).join("");
}

/** Un nombre écrit comme le poste principal le lit. */
const NOMBRE = /^[+-]?(\d+\.?\d*|\.\d+)(e[+-]?\d+)?$/i;

/** Réponse ramenée à sa forme de comparaison : sans accents, en minuscules, sans espaces, sans
 *  point final ; « 3,5 », « 3.5 » et « 3,50 » deviennent le même nombre. */
export function normaliserReponse(texte: string): string {
  const simple = [...simplifier(texte.trim())]
    .filter((c) => !/\s/.test(c) && c !== " " && c !== " ")
    .join("")
    .replace(/\.+$/, "");
  const nombre = simple.replaceAll(",", ".");
  if (NOMBRE.test(nombre)) {
    const n = Number(nombre);
    if (Number.isFinite(n)) return String(n);
  }
  return simple;
}

/** Lettres ou paires d'une réponse, rangées, sans doublon. */
function morceaux(reponse: string): string[] {
  const m = reponse
    .split(/[,; ]/)
    .map((x) => x.trim().toUpperCase())
    .filter((x) => x !== "");
  return [...new Set(m)].sort();
}

/** Vrai si la réponse donnée est juste. Aucune réponse : faux. Plusieurs bonnes réponses ou un
 *  appariement : juste seulement si tout est juste, et rien de plus. */
export function corriger(question: QuestionQuiz, reponse: string): boolean {
  if (reponse.trim() === "") return false;
  if (question.sorte === "reponse_courte") {
    const donnee = normaliserReponse(reponse);
    return question.bonnes.some((b) => normaliserReponse(b) === donnee);
  }
  const m = morceaux(reponse);
  return m.length === question.bonnes.length && m.every((x, i) => x === question.bonnes[i]);
}

/** Réponse en cours : lettres choisies, réponse écrite, ou paires d'un appariement. */
export interface Saisie {
  lettres: string[];
  ecrite: string;
  paires: Record<string, string>;
}

export const SAISIE_VIDE: Saisie = { lettres: [], ecrite: "", paires: {} };

/** La réponse envoyée : « B », « A,C », « A-2,B-1 » ou la réponse écrite. */
export function reponseDe(q: QuestionQuiz, s: Saisie): string {
  if (q.sorte === "reponse_courte") return s.ecrite;
  if (q.sorte === "appariement") {
    return q.choix
      .filter((c) => (s.paires[c.repere] ?? "") !== "")
      .map((c) => `${c.repere}-${s.paires[c.repere] ?? ""}`)
      .join(",");
  }
  return [...s.lettres].sort().join(",");
}
