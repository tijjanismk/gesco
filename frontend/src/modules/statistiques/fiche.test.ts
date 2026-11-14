import { describe, expect, it } from "vitest";

import type { FicheEcole, RubriqueFiche, TypeReponse } from "../../types/gesco";
import { feuilleFiche, texteReponse } from "./fiche";

const rubrique = (id: string, type_reponse: TypeReponse, retiree = false): RubriqueFiche => ({
  id,
  etablissement_id: "e",
  code: null,
  theme: "localisation",
  libelle: `Rubrique ${id}`,
  aide: "",
  type_reponse,
  choix: [],
  annuelle: false,
  livree: false,
  ordre: 1,
  retiree_le: retiree ? 1 : null,
});

describe("fiche de l'établissement", () => {
  it("écrit chaque réponse comme elle se lit", () => {
    expect(texteReponse(rubrique("a", "oui_non"), "oui")).toBe("Oui");
    expect(texteReponse(rubrique("a", "date"), "2026-10-14")).toBe("14/10/2026");
    expect(texteReponse(rubrique("a", "nombre"), "12.5")).toBe("12,5");
    expect(texteReponse(rubrique("a", "plusieurs_choix"), "Réseau\nSolaire")).toBe("Réseau, Solaire");
    expect(texteReponse(rubrique("a", "texte"), null)).toBe("");
  });

  it("exporte les rubriques en service, une case vide quand rien n'est saisi", () => {
    const fiche: FicheEcole = {
      annees: [],
      annee_id: null,
      rubriques: [rubrique("a", "texte"), rubrique("b", "texte"), rubrique("c", "texte", true)],
      reponses: [{ rubrique_id: "a", valeur: "Kati", proposee: null }],
    };
    const f = feuilleFiche(fiche);
    expect(f.lignes).toEqual([
      ["Localisation", "Rubrique a", "Kati"],
      ["Localisation", "Rubrique b", ""],
    ]);
    expect(f.entetes).toHaveLength(3);
  });
});
