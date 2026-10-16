import { invoke } from "@tauri-apps/api/core";

import { versErreurApi } from "./erreur";
import type { Transport } from "./transport";

/** Transport du poste : une seule commande Tauri, `executer`, derrière laquelle la garde Rust
 *  décide de tout (R-21). */
export const transportTauri: Transport = {
  async appeler(nom, jeton, entree) {
    try {
      return await invoke("executer", { nom, jeton, entree });
    } catch (erreur) {
      throw versErreurApi(erreur);
    }
  },
};
