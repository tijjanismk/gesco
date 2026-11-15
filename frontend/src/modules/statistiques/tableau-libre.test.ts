import { describe, expect, it } from "vitest";

import { choixApres } from "./tableau-libre";

describe("tableau libre", () => {
  it("garde les critères qui s'appliquent encore", () => {
    expect(choixApres("personnel", "sexe", "cycle")).toEqual(["sexe", "cycle"]);
  });

  it("prend les premiers critères de la nouvelle population", () => {
    expect(choixApres("personnel", "classe", "age")).toEqual(["statut", "sexe"]);
    expect(choixApres("eleves", "statut", null)).toEqual(["classe", "sexe"]);
  });

  it("n'a pas de colonnes pour la fiche", () => {
    expect(choixApres("fiche", "classe", "sexe")).toEqual(["classe", null]);
  });
});
