// Toute permission du catalogue du noyau doit apparaître dans un domaine de l'écran Rôles,
// sans quoi elle ne pourrait être ni accordée ni retirée (SPEC_PHASE_1 §5.1).
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

import { DOMAINES } from "./Roles";

const PERMISSIONS = join(import.meta.dirname, "..", "..", "..", "..", "crates", "gesco-core", "src", "core", "permission.rs");

describe("écran Rôles", () => {
  it("range chaque domaine du catalogue dans un groupe", () => {
    const source = readFileSync(PERMISSIONS, "utf8");
    const prefixes = new Set([...source.matchAll(/^\s+[A-Z_]+ = "([a-z_]+):[a-z_:]+",$/gm)].map((m) => m[1] ?? ""));
    const ranges = new Set(DOMAINES.flatMap((d) => d.prefixes));
    expect(prefixes.size).toBeGreaterThan(20);
    expect([...prefixes].filter((p) => !ranges.has(p))).toEqual([]);
  });
});
