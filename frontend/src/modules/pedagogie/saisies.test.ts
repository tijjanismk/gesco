import { describe, expect, it } from "vitest";

import { heures, lireAbsence, lireHeures, lirePoints, moyenne, points, texteRang } from "./saisies";

describe("points", () => {
  it("lit les notes au centième, avec virgule ou point", () => {
    expect(lirePoints("13,5")).toBe(1350);
    expect(lirePoints("13.50")).toBe(1350);
    expect(lirePoints(" 20 ")).toBe(2000);
    expect(lirePoints("9,25")).toBe(925);
    expect(lirePoints("0")).toBe(0);
  });

  it("refuse ce qui n'est pas une note", () => {
    expect(lirePoints("")).toBeNull();
    expect(lirePoints("13,555")).toBeNull();
    expect(lirePoints("-2")).toBeNull();
    expect(lirePoints("abs")).toBeNull();
  });

  it("écrit les centièmes sans zéro inutile", () => {
    expect(points(2000)).toBe("20");
    expect(points(1350)).toBe("13,5");
    expect(points(925)).toBe("9,25");
    expect(points(5)).toBe("0,05");
  });
});

describe("heures", () => {
  it("lit les volumes horaires sous plusieurs formes", () => {
    expect(lireHeures("4")).toBe(240);
    expect(lireHeures("4,5")).toBe(270);
    expect(lireHeures("1h30")).toBe(90);
    expect(lireHeures("12 h")).toBe(720);
    expect(lireHeures("deux")).toBeNull();
  });

  it("écrit les minutes en heures", () => {
    expect(heures(720)).toBe("12 h");
    expect(heures(90)).toBe("1 h 30");
    expect(heures(185)).toBe("3 h 05");
  });
});

describe("absences", () => {
  it("lit les codes du relevé comme le noyau", () => {
    expect(lireAbsence("")).toEqual({ nature: "present" });
    expect(lireAbsence(" A ")).toEqual({ nature: "absence" });
    expect(lireAbsence("r")).toEqual({ nature: "retard", minutes: null });
    expect(lireAbsence("R15")).toEqual({ nature: "retard", minutes: 15 });
    expect(lireAbsence("r 20")).toEqual({ nature: "retard", minutes: 20 });
  });

  it("refuse une case illisible ou un retard hors bornes", () => {
    expect(lireAbsence("x")).toBeNull();
    expect(lireAbsence("r0")).toBeNull();
    expect(lireAbsence("r300")).toBeNull();
    expect(lireAbsence("r1h")).toBeNull();
  });
});

describe("bulletin", () => {
  it("écrit les moyennes avec deux décimales et les rangs comme au crayon", () => {
    expect(moyenne(1335)).toBe("13,35");
    expect(moyenne(750)).toBe("7,50");
    expect(moyenne(2000)).toBe("20,00");
    expect(texteRang({ rang: 1, ex_aequo: false })).toBe("1er");
    expect(texteRang({ rang: 3, ex_aequo: true })).toBe("3e ex");
    expect(texteRang(null)).toBe("");
  });
});
