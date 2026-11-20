import { describe, expect, it } from "vitest";

import { corriger, normaliserReponse, reponseDe, SAISIE_VIDE } from "./corriger";
import type { QuestionQuiz } from "./types";

function question(sorte: QuestionQuiz["sorte"], bonnes: string[]): QuestionQuiz {
  return {
    numero: 1,
    sorte,
    enonce: [],
    choix: ["A", "B", "C"].map((repere) => ({ repere, texte: [] })),
    elements: ["1", "2", "3"].map((repere) => ({ repere, texte: [] })),
    bonnes,
    explication: [],
  };
}

describe("la correction sur le téléphone suit celle du poste principal", () => {
  it("lit les nombres et les mots comme lui", () => {
    expect(normaliserReponse("3,5")).toBe(normaliserReponse("3.5"));
    expect(normaliserReponse("3,50")).toBe("3.5");
    expect(normaliserReponse(" Élève. ")).toBe("eleve");
    expect(normaliserReponse("7/2")).toBe("7/2");
    expect(normaliserReponse("")).toBe("");
  });

  it("juge chaque sorte de question", () => {
    expect(corriger(question("choix_unique", ["B"]), "b")).toBe(true);
    expect(corriger(question("choix_unique", ["B"]), "")).toBe(false);
    expect(corriger(question("choix_multiples", ["A", "C"]), "C,A")).toBe(true);
    expect(corriger(question("choix_multiples", ["A", "C"]), "A")).toBe(false);
    expect(corriger(question("choix_multiples", ["A", "C"]), "A,B,C")).toBe(false);
    expect(corriger(question("appariement", ["A-2", "B-1"]), "B-1,A-2")).toBe(true);
    expect(corriger(question("appariement", ["A-2", "B-1"]), "A-2")).toBe(false);
    expect(corriger(question("reponse_courte", ["3,5", "7/2"]), "3.5")).toBe(true);
    expect(corriger(question("reponse_courte", ["Bamako"]), "bamako.")).toBe(true);
  });

  it("écrit la réponse comme le poste principal l'attend", () => {
    const q = question("appariement", ["A-2", "B-1"]);
    expect(reponseDe(q, { ...SAISIE_VIDE, paires: { B: "1", A: "2" } })).toBe("A-2,B-1");
    expect(reponseDe(question("choix_multiples", []), { ...SAISIE_VIDE, lettres: ["C", "A"] })).toBe("A,C");
  });
});
