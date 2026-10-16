import type { ErreurCommande } from "../types/gesco";

/** Erreur d'une commande : la clé du noyau, traduite à l'affichage (R-39). */
export class ErreurApi extends Error {
  readonly cle: string;
  readonly jusqua: number | null;
  readonly detail: string | null;

  constructor(erreur: ErreurCommande) {
    super(erreur.cle);
    this.cle = erreur.cle;
    this.jusqua = erreur.jusqua;
    this.detail = erreur.detail;
  }
}

function estErreurCommande(valeur: unknown): valeur is ErreurCommande {
  return (
    typeof valeur === "object" &&
    valeur !== null &&
    "cle" in valeur &&
    typeof (valeur as { cle: unknown }).cle === "string"
  );
}

/** Convertit ce que rend le transport en erreur d'application. */
export function versErreurApi(valeur: unknown): ErreurApi {
  if (valeur instanceof ErreurApi) return valeur;
  if (estErreurCommande(valeur)) {
    return new ErreurApi({
      cle: valeur.cle,
      jusqua: valeur.jusqua ?? null,
      detail: valeur.detail ?? null,
    });
  }
  return new ErreurApi({ cle: "erreur.transport", jusqua: null, detail: String(valeur) });
}
