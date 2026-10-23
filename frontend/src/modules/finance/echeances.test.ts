import { describe, expect, it } from "vitest";

import { auJour, lireJour } from "./echeances";

describe("jour d'échéance", () => {
  it("se lit du 1er au 28", () => {
    expect(lireJour("5")).toBe(5);
    expect(lireJour(" 28 ")).toBe(28);
    expect(lireJour("0")).toBeNull();
    expect(lireJour("29")).toBeNull();
    expect(lireJour("cinq")).toBeNull();
  });

  it("met une échéance au jour choisi de son mois", () => {
    expect(auJour("2026-10-15", 5)).toBe("2026-10-05");
    expect(auJour("2027-02-15", 28)).toBe("2027-02-28");
    expect(auJour("", 5)).toBe("");
  });
});
