import { francs, moisCivil, t } from "../../i18n";
import type { Calcul } from "../../types/gesco";

/** Un taux saisi en pour cent (« 3,06 », « 37 % ») en centièmes de pour cent : 306, 3 700. Deux
 *  décimales au plus, de 0 à 100 % ; rien si illisible. Comme le noyau. */
export function lireTaux(texte: string): number | null {
  const propre = texte.trim().replace(/%$/, "").replace(/\s/g, "").replace(",", ".");
  const morceaux = /^(\d+)(?:\.(\d{1,2}))?$/.exec(propre);
  if (morceaux === null) return null;
  const entier = Number.parseInt(morceaux[1] ?? "0", 10);
  const decimales = Number.parseInt((morceaux[2] ?? "").padEnd(2, "0"), 10);
  const taux = entier * 100 + decimales;
  return taux <= 10_000 ? taux : null;
}

/** Un taux comme on le saisit : « 3,06 % », « 3,6 % », « 37 % ». */
export function ecrireTaux(taux: number): string {
  const entier = Math.trunc(taux / 100);
  const decimales = taux % 100;
  if (decimales === 0) return `${entier}\u00a0%`;
  const texte = String(decimales).padStart(2, "0").replace(/0$/, "");
  return `${entier},${texte}\u00a0%`;
}

/** Un taux à saisir : « 3,06 », « 37 ». */
export function texteTaux(taux: number): string {
  return ecrireTaux(taux).replace("\u00a0%", "");
}

/** Un montant saisi, chiffres seulement : rien si vide. */
export function lireMontant(texte: string): number | null {
  const chiffres = texte.replace(/\D/g, "");
  return chiffres === "" ? null : Number.parseInt(chiffres, 10);
}

/** Le calcul tel qu'il se saisit : des textes, convertis à l'enregistrement. */
export interface SaisieCalcul {
  mode: Calcul["mode"];
  montant: string;
  taux: string;
  plafond: string;
  annuel: boolean;
  tranches: { jusqua: string; taux: string }[];
  points: string;
  famille: boolean;
  marie: string;
  parEnfant: string;
  plafondFamille: string;
}

export function saisieDe(c: Calcul | null): SaisieCalcul {
  const vide: SaisieCalcul = { mode: "saisi", montant: "", taux: "", plafond: "", annuel: false, tranches: [{ jusqua: "", taux: "" }], points: "", famille: false, marie: "", parEnfant: "", plafondFamille: "" };
  if (c === null) return vide;
  switch (c.mode) {
    case "fixe":
      return { ...vide, mode: "fixe", montant: String(c.montant) };
    case "taux":
      return { ...vide, mode: "taux", taux: texteTaux(c.taux), plafond: c.plafond === null ? "" : String(c.plafond) };
    case "bareme":
      return {
        ...vide,
        mode: "bareme",
        annuel: c.annuel,
        tranches: c.tranches.map((x) => ({ jusqua: x.jusqua === null ? "" : String(x.jusqua), taux: texteTaux(x.taux) })),
        points: c.reduction_points === 0 ? "" : texteTaux(c.reduction_points),
        famille: c.famille !== null,
        marie: c.famille === null ? "" : texteTaux(c.famille.marie),
        parEnfant: c.famille === null ? "" : texteTaux(c.famille.par_enfant),
        plafondFamille: c.famille === null ? "" : texteTaux(c.famille.plafond),
      };
    case "saisi":
      return vide;
  }
}

/** Le calcul à enregistrer ; rien si un montant ou un taux est illisible. Le noyau contrôle le
 *  reste (bornes croissantes, taux de 0 à 100 %). */
export function calculDe(s: SaisieCalcul): Calcul | null {
  switch (s.mode) {
    case "fixe": {
      const montant = lireMontant(s.montant);
      return montant === null ? null : { mode: "fixe", montant };
    }
    case "taux": {
      const taux = lireTaux(s.taux);
      if (taux === null) return null;
      return { mode: "taux", taux, plafond: lireMontant(s.plafond) };
    }
    case "bareme": {
      const tranches = [];
      for (const [i, x] of s.tranches.entries()) {
        const taux = lireTaux(x.taux);
        const jusqua = lireMontant(x.jusqua);
        if (taux === null || (jusqua === null && i !== s.tranches.length - 1)) return null;
        tranches.push({ jusqua, taux });
      }
      const points = s.points.trim() === "" ? 0 : lireTaux(s.points);
      if (points === null) return null;
      let famille = null;
      if (s.famille) {
        const marie = lireTaux(s.marie);
        const parEnfant = lireTaux(s.parEnfant);
        const plafond = lireTaux(s.plafondFamille);
        if (marie === null || parEnfant === null || plafond === null) return null;
        famille = { marie, par_enfant: parEnfant, plafond };
      }
      return { mode: "bareme", tranches, annuel: s.annuel, reduction_points: points, famille };
    }
    case "saisi":
      return { mode: "saisi" };
  }
}

/** Le calcul en clair : « 3,6 % du brut moins Retraite INPS », « Barème annuel en 7 tranches,
 *  de 0 % à 37 % ». */
export function texteCalcul(c: Calcul, deduits: readonly string[]): string {
  switch (c.mode) {
    case "fixe":
      return t("paie.texte.fixe", { montant: francs(c.montant) });
    case "taux": {
      const base = deduits.length === 0 ? t("paie.texte.taux", { taux: ecrireTaux(c.taux) }) : t("paie.texte.taux_moins", { taux: ecrireTaux(c.taux), deduit: deduits.join(", ") });
      return c.plafond === null ? base : `${base}, ${t("paie.texte.plafond", { plafond: francs(c.plafond) })}`;
    }
    case "bareme": {
      const taux = c.tranches.map((x) => x.taux);
      return t("paie.texte.bareme", {
        periode: t(c.annuel ? "paie.texte.annuel" : "paie.texte.mensuel"),
        n: c.tranches.length,
        min: ecrireTaux(Math.min(...taux)),
        max: ecrireTaux(Math.max(...taux)),
      });
    }
    case "saisi":
      return t("paie.texte.saisi");
  }
}

/** « d'octobre 2026 », « de novembre 2026 » : le mois avec sa préposition élidée. */
export function duMois(mois: string): string {
  const texte = moisCivil(mois);
  return /^[aeiouéh]/i.test(texte) ? `d'${texte}` : `de ${texte}`;
}
