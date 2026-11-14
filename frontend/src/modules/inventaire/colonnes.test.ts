import { describe, expect, it } from "vitest";

import type { ArticleInventaire } from "../../types/gesco";
import { COLONNES_FAMILLE, QUANTITES_VIDES, libelleArticle, lireQuantite } from "./colonnes";

const manuel: ArticleInventaire = {
  id: "A",
  etablissement_id: "E",
  code: null,
  famille: "manuel",
  designation: "Je lis",
  matiere: "Lecture",
  niveau_id: "N1",
  livre: false,
  ordre: 1,
  retire_le: null,
};

describe("inventaire", () => {
  it("une case vide reste vide, jamais zéro", () => {
    expect(lireQuantite("")).toBeNull();
    expect(lireQuantite("  ")).toBeNull();
    expect(lireQuantite("0")).toBe(0);
    expect(lireQuantite("12")).toBe(12);
    expect(lireQuantite("-3")).toBe(0);
    expect(lireQuantite("2,7")).toBe(2);
    expect(lireQuantite("abc")).toBeNull();
  });

  it("un manuel montre sa matière et son niveau", () => {
    expect(libelleArticle(manuel, [{ id: "N1", libelle: "1re année", cycle: "premier_cycle" }])).toBe("Je lis (Lecture, 1re année)");
    expect(libelleArticle({ ...manuel, matiere: "", niveau_id: null }, [])).toBe("Je lis");
  });

  it("chaque colonne saisie est une quantité", () => {
    for (const colonnes of Object.values(COLONNES_FAMILLE)) {
      for (const c of colonnes) expect(QUANTITES_VIDES[c]).toBeNull();
    }
  });
});
