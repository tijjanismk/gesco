// Réduction des photos de copie dans la page, avant l'envoi : 1 600 pixels au plus sur le grand
// côté, en JPEG de qualité moyenne (environ 300 Ko). Le canvas ne garde aucune métadonnée ; le
// navigateur redresse la photo d'après son orientation avant de la dessiner.

/** Plus grand côté d'une photo réduite, en pixels. */
export const COTE_MAX = 1600;

/** Qualité du JPEG produit. */
export const QUALITE = 0.72;

/** Photos d'un rendu, au plus. */
export const PHOTOS_MAX = 5;

/** Taille d'une photo, au plus, une fois réduite (le poste principal refuse au-delà). */
export const TAILLE_MAX = 2 * 1024 * 1024;

/** Dimensions réduites : le grand côté à `max` au plus, les proportions gardées ; une petite
 *  photo n'est pas agrandie. */
export function dimensionsReduites(largeur: number, hauteur: number, max: number = COTE_MAX): { largeur: number; hauteur: number } {
  if (largeur <= 0 || hauteur <= 0) return { largeur: 0, hauteur: 0 };
  const facteur = Math.min(1, max / Math.max(largeur, hauteur));
  return { largeur: Math.max(1, Math.round(largeur * facteur)), hauteur: Math.max(1, Math.round(hauteur * facteur)) };
}

async function ouvrirImage(fichier: Blob): Promise<{ source: CanvasImageSource; largeur: number; hauteur: number; fermer: () => void }> {
  if (typeof createImageBitmap === "function") {
    try {
      const image = await createImageBitmap(fichier, { imageOrientation: "from-image" });
      return { source: image, largeur: image.width, hauteur: image.height, fermer: () => image.close() };
    } catch {
      // Un vieux navigateur sans l'option : la voie de l'élément image.
    }
  }
  const adresse = URL.createObjectURL(fichier);
  try {
    const image = new Image();
    image.src = adresse;
    await image.decode();
    return {
      source: image,
      largeur: image.naturalWidth,
      hauteur: image.naturalHeight,
      fermer: () => URL.revokeObjectURL(adresse),
    };
  } catch (e) {
    URL.revokeObjectURL(adresse);
    throw e;
  }
}

/** Réduit une photo et la rend en adresse `data:` JPEG. Un navigateur sans canvas rend la photo
 *  telle quelle (le poste principal la refuse si elle dépasse 2 Mo). */
export async function reduirePhoto(fichier: Blob): Promise<string> {
  const image = await ouvrirImage(fichier);
  try {
    const { largeur, hauteur } = dimensionsReduites(image.largeur, image.hauteur);
    const canvas = document.createElement("canvas");
    canvas.width = largeur;
    canvas.height = hauteur;
    const contexte = canvas.getContext("2d");
    if (contexte === null) return await lireEnAdresse(fichier);
    // Fond blanc : une capture PNG transparente reste lisible en JPEG.
    contexte.fillStyle = "#ffffff";
    contexte.fillRect(0, 0, largeur, hauteur);
    contexte.drawImage(image.source, 0, 0, largeur, hauteur);
    return canvas.toDataURL("image/jpeg", QUALITE);
  } finally {
    image.fermer();
  }
}

function lireEnAdresse(fichier: Blob): Promise<string> {
  return new Promise((resoudre, rejeter) => {
    const lecteur = new FileReader();
    lecteur.onload = () => resoudre(String(lecteur.result));
    lecteur.onerror = () => rejeter(lecteur.error ?? new Error("lecture"));
    lecteur.readAsDataURL(fichier);
  });
}

/** Taille en octets d'une adresse `data:` en base 64. */
export function tailleDe(adresse: string): number {
  const virgule = adresse.indexOf(",");
  const b64 = virgule < 0 ? adresse : adresse.slice(virgule + 1);
  const rembourrage = b64.endsWith("==") ? 2 : b64.endsWith("=") ? 1 : 0;
  return Math.floor((b64.length * 3) / 4) - rembourrage;
}
