import { createContext, useContext } from "react";

import type { MonAvancement } from "./types";

/** L'avancement de l'élève, quand il est entré avec son code et que le suivi est allumé. */
export type Suivi = {
  actif: boolean;
  avancement: MonAvancement | null;
  rafraichir: () => void;
};

export const ContexteSuivi = createContext<Suivi>({ actif: false, avancement: null, rafraichir: () => undefined });

export function useSuivi(): Suivi {
  return useContext(ContexteSuivi);
}
