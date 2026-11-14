import { describe, expect, it } from "vitest";

import type { TableauStatistique } from "../../types/gesco";
import { feuilleDe, heures, texteCellule } from "./mise-en-forme";

const nombre = (n: number, membres: number[] = []) => ({ nombre: n, texte: null, membres });
const texte = (x: string | null) => ({ nombre: null, texte: x, membres: [] });

describe("mise en forme des statistiques", () => {
  it("écrit les heures de la semaine en heures et minutes", () => {
    expect(heures(180)).toBe("3 h");
    expect(heures(90)).toBe("1 h 30");
    expect(heures(5)).toBe("0 h 05");
  });

  it("laisse vide une case que la fiche ne porte pas, sans la signaler", () => {
    expect(texteCellule("diplome", texte(null))).toBe("");
    expect(texteCellule("situation_famille", texte("marie"))).toBe("Marié ou mariée");
    expect(texteCellule("fonctionnaire", texte("non"))).toBe("Non");
    expect(texteCellule("total", nombre(0))).toBe("0");
  });

  it("exporte les lignes et le total, jamais la ligne des fiches hors colonnes", () => {
    const tableau: TableauStatistique = {
      code: "eleves_par_classe",
      colonnes: ["garcons", "filles", "total"],
      lignes: [{ libelle: "1re A", cle: null, cellules: [nombre(2, [0, 1]), nombre(1, [2]), nombre(4, [0, 1, 2, 3])] }],
      total: { libelle: "", cle: "statistiques.total", cellules: [nombre(2), nombre(1), nombre(4)] },
      hors: [{ cle: "statistiques.hors.eleves_sans_sexe", membres: [3] }],
    };
    const feuille = feuilleDe(tableau);
    expect(feuille.titre).toBe("Élèves par classe");
    expect(feuille.entetes).toEqual(["Classe", "Garçons", "Filles", "Total"]);
    expect(feuille.lignes).toEqual([
      ["1re A", "2", "1", "4"],
      ["Total", "2", "1", "4"],
    ]);
  });
});
