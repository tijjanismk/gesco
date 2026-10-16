// R-39 et ADR-015 : toute chaîne visible passe par le catalogue, toute clé du noyau y figure, et
// aucune clé n'y dort.
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";

import ts from "typescript";
import { describe, expect, it } from "vitest";

import { catalogue } from "./fr-ML";

const SRC = join(import.meta.dirname, "..");
const CRATES = join(SRC, "..", "..", "crates");

function fichiers(dossier: string, extensions: readonly string[]): string[] {
  return readdirSync(dossier).flatMap((nom) => {
    const chemin = join(dossier, nom);
    if (statSync(chemin).isDirectory()) return fichiers(chemin, extensions);
    return extensions.some((e) => nom.endsWith(e)) ? [chemin] : [];
  });
}

const sources = fichiers(SRC, [".ts", ".tsx"]).filter(
  (f) => !f.endsWith(".test.ts") && !f.includes(`${join("src", "types")}`) && !f.endsWith("fr-ML.ts"),
);

describe("catalogue fr-ML", () => {
  it("contient toutes les clés d'erreur et d'avertissement du noyau", () => {
    // Les règles pures du noyau rendent leur clé par `Err("…")` ; les services par `cle:` ou
    // `regle("…")`. Toutes atteignent l'écran.
    const rust = fichiers(join(CRATES, "gesco-services", "src"), [".rs"])
      .concat(fichiers(join(CRATES, "gesco-docs", "src"), [".rs"]))
      .concat(fichiers(join(CRATES, "gesco-core", "src", "scolarite"), [".rs"]))
      .concat(fichiers(join(CRATES, "gesco-core", "src", "finance"), [".rs"]))
      .concat(fichiers(join(CRATES, "gesco-core", "src", "rh"), [".rs"]));
    const motif = /(?:cle: |=> |cle_message: |regle\(|Err\()"([a-z_]+\.[a-z_.]+)"|"(avertissement\.[a-z_]+)"/g;
    const cles = new Set<string>();
    for (const fichier of rust) {
      for (const m of readFileSync(fichier, "utf8").matchAll(motif)) cles.add(m[1] ?? m[2] ?? "");
    }
    const absentes = [...cles].filter((c) => !Object.hasOwn(catalogue, c));
    expect(cles.size).toBeGreaterThan(40);
    expect(absentes).toEqual([]);
  });

  it("donne un libellé à chaque paramètre d'établissement", () => {
    const sources = [
      join(CRATES, "gesco-core", "src", "core", "parametre.rs"),
      ...fichiers(join(CRATES, "gesco-services", "src"), [".rs"]),
    ].map((f) => readFileSync(f, "utf8"));
    const motif = /\bp\(\s*"([a-z_]+\.[a-z_]+)"|PARAMETRE_[A-Z_]+: &str = "([a-z_]+\.[a-z_]+)"/g;
    const cles = new Set(sources.flatMap((s) => [...s.matchAll(motif)].map((m) => m[1] ?? m[2] ?? "")));
    expect(cles.size).toBeGreaterThan(10);
    expect([...cles].filter((c) => !Object.hasOwn(catalogue, c))).toEqual([]);
  });

  it("donne un libellé à chaque permission du catalogue", () => {
    const source = readFileSync(join(CRATES, "gesco-core", "src", "core", "permission.rs"), "utf8");
    const codes = [...source.matchAll(/^\s+[A-Z_]+ = "([a-z_]+:[a-z_:]+)",$/gm)].map((m) => m[1] ?? "");
    expect(codes.length).toBeGreaterThan(70);
    expect(codes.filter((c) => !Object.hasOwn(catalogue, c))).toEqual([]);
  });

  it("n'a pas de clé inutilisée", () => {
    const code = sources.map((f) => readFileSync(f, "utf8")).join("\n");
    const rust = fichiers(CRATES, [".rs"]).map((f) => readFileSync(f, "utf8")).join("\n");
    const inutilisees = Object.keys(catalogue).filter(
      (cle) => !code.includes(`"${cle}"`) && !rust.includes(`"${cle}"`),
    );
    expect(inutilisees).toEqual([]);
  });
});

describe("chaînes visibles", () => {
  const ATTRIBUTS = new Set(["placeholder", "title", "alt", "aria-label", "libelle", "aide"]);

  it("aucun texte en dur dans le JSX (R-39)", () => {
    const fautes: string[] = [];
    for (const fichier of sources.filter((f) => f.endsWith(".tsx"))) {
      const source = ts.createSourceFile(fichier, readFileSync(fichier, "utf8"), ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
      const visiter = (noeud: ts.Node) => {
        if (ts.isJsxText(noeud) && /\p{L}/u.test(noeud.text)) {
          fautes.push(`${relative(SRC, fichier)} : « ${noeud.text.trim()} »`);
        }
        if (
          ts.isJsxAttribute(noeud) &&
          ATTRIBUTS.has(noeud.name.getText(source)) &&
          noeud.initializer !== undefined &&
          ts.isStringLiteral(noeud.initializer) &&
          /\p{L}/u.test(noeud.initializer.text)
        ) {
          fautes.push(`${relative(SRC, fichier)} : ${noeud.name.getText(source)}="${noeud.initializer.text}"`);
        }
        ts.forEachChild(noeud, visiter);
      };
      visiter(source);
    }
    expect(fautes).toEqual([]);
  });
});
