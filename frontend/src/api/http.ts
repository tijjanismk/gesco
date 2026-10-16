import { versErreurApi } from "./erreur";
import type { Transport } from "./transport";

/** Transport du mode réseau local, déclaré dès la phase 0.5 (spécification §12.2) et activé en
 *  phase 11 (Q-13) : `POST /api/commandes/{nom}`, jeton en en-tête. */
export function transportHttp(adresse: string, recuperer: typeof fetch = fetch): Transport {
  return {
    async appeler(nom, jeton, entree) {
      const reponse = await recuperer(`${adresse}/api/commandes/${nom}`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          ...(jeton === null ? {} : { Authorization: `Bearer ${jeton}` }),
        },
        body: JSON.stringify(entree ?? null),
      });
      const corps: unknown = await reponse.json();
      if (!reponse.ok) throw versErreurApi(corps);
      return corps;
    },
  };
}
