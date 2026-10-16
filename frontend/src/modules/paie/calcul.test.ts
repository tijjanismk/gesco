import { describe, expect, it } from "vitest";

import { calculDe, ecrireTaux, lireTaux, saisieDe, texteCalcul } from "./calcul";

describe("calcul d'un élément de paie", () => {
  it("lit et écrit un taux comme le noyau", () => {
    expect(lireTaux("3,06")).toBe(306);
    expect(lireTaux(" 3.6 % ")).toBe(360);
    expect(lireTaux("37")).toBe(3_700);
    expect(lireTaux("100,01")).toBeNull();
    expect(lireTaux("3,061")).toBeNull();
    expect(lireTaux("")).toBeNull();
    expect(ecrireTaux(306)).toBe("3,06 %");
    expect(ecrireTaux(360)).toBe("3,6 %");
    expect(ecrireTaux(3_700)).toBe("37 %");
  });

  it("aller et retour d'un barème saisi", () => {
    const bareme = {
      mode: "bareme" as const,
      tranches: [
        { jusqua: 330_000, taux: 0 },
        { jusqua: null, taux: 500 },
      ],
      annuel: true,
      reduction_points: 200,
      famille: { marie: 1_000, par_enfant: 250, plafond: 3_500 },
    };
    expect(calculDe(saisieDe(bareme))).toEqual(bareme);
    const saisie = saisieDe(bareme);
    expect(calculDe({ ...saisie, tranches: [{ jusqua: "", taux: "0" }, { jusqua: "", taux: "5" }] })).toBeNull();
    expect(calculDe({ ...saisieDe(null), mode: "taux", taux: "deux" })).toBeNull();
  });

  it("dit un calcul en clair", () => {
    expect(texteCalcul({ mode: "taux", taux: 360, plafond: null }, [])).toBe("3,6 % du brut");
    expect(texteCalcul({ mode: "taux", taux: 1_000, plafond: null }, ["Retraite INPS"])).toBe("10 % du brut moins Retraite INPS");
    expect(texteCalcul({ mode: "saisi" }, [])).toBe("Saisi dans la paie du mois");
  });
});
