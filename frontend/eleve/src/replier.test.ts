import { describe, expect, it } from "vitest";

import { replier, titreSimple } from "./replier";
import type { Bloc } from "./types";

const titre = (niveau: number, texte: string): Bloc => ({ type: "titre", niveau, contenu: [{ type: "texte", texte }] });
const para = (texte: string): Bloc => ({ type: "paragraphe", contenu: [{ type: "texte", texte }] });

describe("replier", () => {
  it("garde une leçon sans corrigés d'un seul tenant", () => {
    const sections = replier([titre(1, "La racine carrée"), para("16 a pour racine 4.")]);
    expect(sections).toHaveLength(1);
    expect(sections[0]?.repliee).toBeNull();
  });

  it("replie les corrigés jusqu'au titre suivant de même niveau", () => {
    const sections = replier([
      titre(2, "1. Le cours"),
      para("cours"),
      titre(2, "5. Corrigés"),
      para("réponse 1"),
      titre(3, "Exercice 2"),
      para("réponse 2"),
      titre(2, "6. Pour aller plus loin"),
      para("suite"),
    ]);
    expect(sections.map((s) => s.repliee)).toEqual([null, "corriges", null]);
    expect(sections[1]?.blocs).toHaveLength(3);
  });

  it("replie les réponses du quiz seulement sous le quiz", () => {
    const sections = replier([
      titre(2, "Réponses"),
      para("pas un quiz"),
      titre(2, "Quiz"),
      para("question"),
      titre(3, "Réponses"),
      para("a"),
      titre(2, "Fin"),
    ]);
    expect(sections.map((s) => s.repliee)).toEqual([null, "reponses", null]);
  });

  it("simplifie un titre comme le noyau", () => {
    expect(titreSimple("6. Corrigés")).toBe("corriges");
    expect(titreSimple("RÉPONSES")).toBe("reponses");
  });
});
