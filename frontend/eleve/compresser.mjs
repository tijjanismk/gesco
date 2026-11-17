// Écrans des élèves, autour de leur construction :
// - `--vider` (avant) : vide le dossier livré, sauf son LISEZ-MOI et son .gitignore ;
// - sans argument (après) : pose une copie compressée (`.gz`) à côté de chaque fichier texte ;
//   le poste principal sert la copie compressée au téléphone qui l'accepte.
import { readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { gzipSync } from "node:zlib";

const dossier = fileURLToPath(new URL("../../crates/gesco-app/ressources/portail-eleve", import.meta.url));
const gardes = new Set(["LISEZ-MOI.txt", ".gitignore"]);
const aCompresser = /\.(html|js|css|svg|webmanifest|json|ttf)$/;

if (process.argv.includes("--vider")) {
  for (const nom of readdirSync(dossier)) {
    if (!gardes.has(nom)) rmSync(join(dossier, nom), { recursive: true, force: true });
  }
} else {
  let avant = 0;
  let apres = 0;
  const parcourir = (chemin) => {
    for (const nom of readdirSync(chemin)) {
      const fichier = join(chemin, nom);
      if (statSync(fichier).isDirectory()) {
        parcourir(fichier);
      } else if (aCompresser.test(nom)) {
        const octets = readFileSync(fichier);
        const compresse = gzipSync(octets, { level: 9 });
        writeFileSync(`${fichier}.gz`, compresse);
        avant += octets.length;
        apres += compresse.length;
      }
    }
  };
  parcourir(dossier);
  process.stdout.write(`écrans des élèves : ${Math.round(avant / 1024)} Ko, ${Math.round(apres / 1024)} Ko compressés\n`);
}
