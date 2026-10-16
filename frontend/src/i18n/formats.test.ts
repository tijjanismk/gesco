import { describe, expect, it } from "vitest";

import { dateCivile, moisCivil, nomMois } from ".";

describe("formats du Mali", () => {
  it("écrit une date civile en toutes lettres", () => {
    expect(dateCivile("2026-10-14")).toBe("14 octobre 2026");
    expect(dateCivile("2026-06-01")).toBe("1er juin 2026");
    expect(dateCivile("2026-06-11")).toBe("11 juin 2026");
  });

  it("écrit un mois en toutes lettres", () => {
    expect(moisCivil("2026-10")).toBe("octobre 2026");
    expect(moisCivil("2027-01")).toBe("janvier 2027");
    expect(nomMois(7)).toBe("juillet");
  });
});
