import { describe, expect, it } from "vitest";

import { ecrireDemiJournees, joursDeConge } from "./jours";

describe("jours de congé", () => {
  it("compte du lundi au vendredi, le samedi sur réglage", () => {
    // Du lundi 19 au dimanche 25 octobre 2026.
    expect(joursDeConge("2026-10-19", "2026-10-25", false)).toBe(5);
    expect(joursDeConge("2026-10-19", "2026-10-25", true)).toBe(6);
    expect(joursDeConge("2026-10-24", "2026-10-25", false)).toBe(0);
    expect(joursDeConge("2026-12-07", "2026-12-31", false)).toBe(19);
  });

  it("ne compte rien sans dates dans l'ordre", () => {
    expect(joursDeConge("", "2026-10-25", false)).toBe(0);
    expect(joursDeConge("2026-10-25", "2026-10-19", false)).toBe(0);
  });

  it("écrit les demi-journées en jours", () => {
    expect(ecrireDemiJournees(30)).toBe("15");
    expect(ecrireDemiJournees(25)).toBe("12,5");
    expect(ecrireDemiJournees(-7)).toBe("−3,5");
  });
});
