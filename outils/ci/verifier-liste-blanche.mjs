// Vérifie que `gesco-core` ne dépend que de crates sans effet de bord (R-03).
// Lit `cargo metadata`, suit les dépendances directes de gesco-core et échoue à la première
// crate absente de la liste blanche.
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const racine = join(dirname(fileURLToPath(import.meta.url)), "..", "..");
const listeBlanche = JSON.parse(
  readFileSync(join(racine, "outils", "ci", "liste-blanche-gesco-core.json"), "utf8"),
);
const metadonnees = JSON.parse(
  execFileSync("cargo", ["metadata", "--format-version", "1", "--no-deps"], {
    cwd: racine,
    encoding: "utf8",
  }),
);
const noyau = metadonnees.packages.find((p) => p.name === "gesco-core");
if (!noyau) {
  console.error("gesco-core introuvable dans l'espace de travail");
  process.exit(2);
}
const interdites = noyau.dependencies
  .filter((d) => d.kind !== "dev")
  .map((d) => d.name)
  .filter((nom) => !listeBlanche.autorisees.includes(nom));
if (interdites.length > 0) {
  console.error(`Dépendances interdites dans gesco-core (R-03) : ${interdites.join(", ")}`);
  process.exit(1);
}
