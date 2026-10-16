import { describe, expect, it } from "vitest";

import type { Niveau, Referentiel, SaisieLigne } from "../../types/gesco";
import { deplier, grouper, resumeNiveaux } from "./groupes";

function niveau(id: string, libelle: string, cycle: Niveau["cycle"], actif = true): Niveau {
  return { id, etablissement_id: "e", code: id, libelle, cycle, ordre: 0, mode_evaluation: "notes", diplome_terminal: "aucun", actif };
}

const referentiel: Referentiel = {
  niveaux: [
    niveau("1", "1re année", "premier_cycle"),
    niveau("2", "2e année", "premier_cycle"),
    niveau("7", "7e année", "second_cycle"),
    niveau("8", "8e année", "second_cycle"),
    niveau("9", "9e année", "second_cycle"),
  ],
  series: [],
};

function ligne(niveau_id: string, montant: number, type: SaisieLigne["type_frais"] = "scolarite"): SaisieLigne {
  return { niveau_id, serie_id: null, type_frais: type, libelle: type, montant, obligatoire: true, exigibilite: type === "scolarite" ? "selon_plan" : "une_fois" };
}

describe("frais groupés par niveaux", () => {
  it("réunit les frais identiques et garde les autres à part", () => {
    const lignes = [ligne("1", 100_000), ligne("7", 150_000), ligne("2", 100_000), ligne("1", 7_500, "tenue"), ligne("7", 7_500, "tenue")];
    const groupes = grouper(lignes);
    expect(groupes.map((g) => g.niveaux)).toEqual([["1", "2"], ["7"], ["1", "7"]]);
    expect(deplier(groupes, ["1", "2", "7"])).toEqual([ligne("1", 100_000), ligne("2", 100_000), ligne("7", 150_000), ligne("1", 7_500, "tenue"), ligne("7", 7_500, "tenue")]);
  });

  it("une ligne de série reste seule", () => {
    const serie = { ...ligne("7", 150_000), serie_id: "S" };
    expect(grouper([serie, { ...serie }]).length).toBe(2);
  });

  it("nomme un cycle entier par son cycle", () => {
    const cycle = (c: string) => `cycle ${c}`;
    expect(resumeNiveaux(referentiel, ["7", "8", "9", "1"], cycle)).toBe("1re année, cycle second_cycle");
    expect(resumeNiveaux(referentiel, ["1", "2", "7"], cycle)).toBe("cycle premier_cycle, 7e année");
  });

  it("nomme aussi l'enseignement technique entier, après le général", () => {
    const avecTechnique: Referentiel = {
      niveaux: [...referentiel.niveaux, niveau("CAP1", "CAP 1re année", "technique"), niveau("CAP2", "CAP 2e année", "technique")],
      series: [],
    };
    const cycle = (c: string) => `cycle ${c}`;
    expect(resumeNiveaux(avecTechnique, ["CAP2", "CAP1", "7"], cycle)).toBe("7e année, cycle technique");
    expect(resumeNiveaux(avecTechnique, ["CAP1"], cycle)).toBe("CAP 1re année");
  });
});
