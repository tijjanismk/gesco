// Interdits visuels du blueprint §19.6 vérifiés par l'outillage : aucune couleur de la palette
// Tailwind par défaut, aucune valeur arbitraire, aucun émoji.
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";

import { describe, expect, it } from "vitest";

const SRC = join(import.meta.dirname, "..");

function fichiers(dossier: string): string[] {
  return readdirSync(dossier).flatMap((nom) => {
    const chemin = join(dossier, nom);
    if (statSync(chemin).isDirectory()) return fichiers(chemin);
    return /\.(tsx|ts|css)$/.test(nom) && !nom.endsWith(".test.ts") ? [chemin] : [];
  });
}

const PALETTE =
  /\b(?:bg|text|border|ring|fill|stroke|from|via|to|outline|decoration|divide|accent|caret|shadow|placeholder)-(?:slate|gray|zinc|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose|black|white)(?:-\d{2,3})?\b/;
const ARBITRAIRE = /\b[a-z-]+-\[[^\]]+\]/;
const EMOJI = /[\u{1F300}-\u{1FAFF}\u{2600}-\u{27BF}]/u;

describe("interdits visuels", () => {
  for (const fichier of fichiers(SRC)) {
    it(relative(SRC, fichier), () => {
      const contenu = readFileSync(fichier, "utf8");
      expect(contenu).not.toMatch(PALETTE);
      expect(contenu).not.toMatch(ARBITRAIRE);
      expect(contenu).not.toMatch(EMOJI);
    });
  }
});
