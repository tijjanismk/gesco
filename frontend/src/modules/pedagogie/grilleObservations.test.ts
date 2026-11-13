import { describe, expect, it } from "vitest";

import { caseVoisine, initiales, niveauDeTouche } from "./grilleObservations";

describe("touches de la grille d'observations", () => {
  it("prend les initiales des libellés livrés, sans accent ni casse", () => {
    const lettres = initiales(["Acquis", "En cours", "Non acquis"]);
    expect(lettres).toEqual(["A", "E", "N"]);
    expect(niveauDeTouche("a", lettres)).toBe("acquis");
    expect(niveauDeTouche("E", lettres)).toBe("en_cours");
    expect(niveauDeTouche("n", lettres)).toBe("non_acquis");
    expect(initiales(["Très bien", "Élan", "Pas encore"])).toEqual(["T", "E", "P"]);
    expect(niveauDeTouche("é", initiales(["Très bien", "Élan", "Pas encore"]))).toBe("en_cours");
  });

  it("garde 1, 2, 3 et renonce aux lettres quand deux libellés commencent pareil", () => {
    const lettres = initiales(["Acquis", "A moitié", "Non acquis"]);
    expect(lettres).toEqual([]);
    expect(niveauDeTouche("a", lettres)).toBeNull();
    expect(niveauDeTouche("1", lettres)).toBe("acquis");
    expect(niveauDeTouche("2", lettres)).toBe("en_cours");
    expect(niveauDeTouche("3", lettres)).toBe("non_acquis");
    expect(niveauDeTouche("4", lettres)).toBeNull();
    expect(niveauDeTouche("Delete", ["A", "E", "N"])).toBeNull();
  });

  it("descend d'un enfant, puis passe à la compétence suivante", () => {
    expect(caseVoisine(0, 0, 3, 2, "suivante")).toEqual([1, 0]);
    expect(caseVoisine(2, 0, 3, 2, "suivante")).toEqual([0, 1]);
    expect(caseVoisine(2, 1, 3, 2, "suivante")).toBeNull();
    expect(caseVoisine(0, 0, 3, 2, "ArrowUp")).toBeNull();
    expect(caseVoisine(1, 1, 3, 2, "ArrowLeft")).toEqual([1, 0]);
    expect(caseVoisine(1, 1, 3, 2, "ArrowRight")).toBeNull();
    expect(caseVoisine(2, 0, 3, 2, "ArrowDown")).toBeNull();
  });
});
