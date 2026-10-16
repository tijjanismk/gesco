/** Côté le plus long d'une photo d'élève, en pixels : de quoi l'imprimer sur une fiche ou une
 *  carte scolaire, en restant sous 100 Ko (T-10). */
const COTE_MAX = 480;

/** Réduit une image et la rend en JPEG base64. Le noyau revérifie le format et la taille (R-02) :
 *  cette réduction n'est qu'un confort de transfert. */
function reduire(source: CanvasImageSource, largeur: number, hauteur: number): string {
  const echelle = Math.min(1, COTE_MAX / Math.max(largeur, hauteur));
  const toile = document.createElement("canvas");
  toile.width = Math.round(largeur * echelle);
  toile.height = Math.round(hauteur * echelle);
  toile.getContext("2d")?.drawImage(source, 0, 0, toile.width, toile.height);
  const url = toile.toDataURL("image/jpeg", 0.85);
  return url.slice(url.indexOf(",") + 1);
}

/** Photo choisie dans un fichier. */
export async function reduirePhoto(fichier: File): Promise<string> {
  const image = await createImageBitmap(fichier);
  const base64 = reduire(image, image.width, image.height);
  image.close();
  return base64;
}

/** Image courante d'une vidéo de webcam. */
export function capturerVideo(video: HTMLVideoElement): string {
  return reduire(video, video.videoWidth, video.videoHeight);
}
