import { open, save } from "@tauri-apps/plugin-dialog";

/** Dialogue d'ouverture du système, filtré sur les archives de sauvegarde `.gsv`. Il rend un
 *  chemin, jamais le contenu : c'est la frontière Rust qui lit le fichier. `null` si l'utilisateur
 *  annule. */
export async function choisirArchive(libelleFiltre: string): Promise<string | null> {
  return open({ multiple: false, directory: false, filters: [{ name: libelleFiltre, extensions: ["gsv"] }] });
}

/** Dialogue d'enregistrement du système pour un classeur Excel : il rend le chemin choisi, que
 *  la frontière Rust écrit ensuite (R-44). `null` si l'utilisateur annule. */
export async function choisirClasseur(nom: string, libelleFiltre: string): Promise<string | null> {
  return save({ defaultPath: `${nom}.xlsx`, filters: [{ name: libelleFiltre, extensions: ["xlsx"] }] });
}

/** Dialogue d'ouverture pour un fichier d'élèves : tableur ou CSV. Le noyau lit le fichier. */
export async function choisirTableur(libelleFiltre: string): Promise<string | null> {
  return open({ multiple: false, directory: false, filters: [{ name: libelleFiltre, extensions: ["xlsx", "xls", "ods", "csv"] }] });
}

/** Dialogue d'ouverture pour une pièce du dossier du personnel : PDF ou image. */
export async function choisirPiece(libelleFiltre: string): Promise<string | null> {
  return open({ multiple: false, directory: false, filters: [{ name: libelleFiltre, extensions: ["pdf", "png", "jpg", "jpeg"] }] });
}

/** Dialogue d'enregistrement d'une pièce du dossier, sous son nom d'origine. */
export async function enregistrerPieceSous(nom: string): Promise<string | null> {
  return save({ defaultPath: nom });
}

/** Dialogue d'ouverture du système, filtré sur les archives du journal d'audit `.gja`. */
export async function choisirArchiveJournal(libelleFiltre: string): Promise<string | null> {
  return open({ multiple: false, directory: false, filters: [{ name: libelleFiltre, extensions: ["gja"] }] });
}
