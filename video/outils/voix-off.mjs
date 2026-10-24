// Voix off : synthèse locale (Kokoro, par `npx hyperframes tts`) de chaque phrase de
// outils/voix-off.json, mise à -16 LUFS (crête -1,5 dBTP) et écrite en MP3 dans assets/voix/.
//   node outils/voix-off.mjs            toutes les phrases
//   node outils/voix-off.mjs 16x9-s3    une seule
// Après un changement de texte, recaler les data-start des <audio> si la durée a changé.
import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const projet = join(dirname(fileURLToPath(import.meta.url)), "..");
const reglage = JSON.parse(readFileSync(join(projet, "outils/voix-off.json"), "utf8"));
const seules = process.argv.slice(2);
const travail = mkdtempSync(join(tmpdir(), "voix-off-"));
mkdirSync(join(projet, "assets/voix"), { recursive: true });

for (const [id, { texte, vitesse }] of Object.entries(reglage.lignes)) {
  if (seules.length && !seules.includes(id)) continue;
  const brut = join(travail, `${id}.wav`);
  const sortie = join(projet, "assets/voix", `${id}.mp3`);
  execFileSync(
    "npx",
    ["hyperframes", "tts", texte, "-v", reglage.voix, "-l", reglage.langue, "-s", String(vitesse), "-o", brut],
    { cwd: projet, stdio: "ignore" },
  );
  execFileSync("ffmpeg", [
    "-v", "error", "-y", "-i", brut,
    "-af", "loudnorm=I=-16:TP=-1.5:LRA=11,aresample=44100",
    "-ac", "1", "-c:a", "libmp3lame", "-b:a", "128k", sortie,
  ]);
  const duree = execFileSync("ffprobe", ["-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", sortie], {
    encoding: "utf8",
  });
  console.log(`${id}  ${Number(duree).toFixed(2)} s`);
}
