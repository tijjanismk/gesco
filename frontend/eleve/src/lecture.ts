import { createContext, useContext, useEffect, useState } from "react";

import { lire, Occupe, SortieDemandee } from "./api";

/** Ce que voit un écran de ce qu'il a demandé. */
export type Etat<T> = { etat: "chargement" } | { etat: "pret"; donnee: T } | { etat: "occupe" } | { etat: "introuvable" };

/** Appelé quand la session a fini : l'élève revient à l'entrée. */
export const ContexteSortie = createContext<() => void>(() => undefined);

/** Délai avant de réessayer quand le poste principal est occupé. */
export const ATTENTE_MS = 5000;

/** Lit `chemin` au poste principal ; réessaie seul tant qu'il est occupé ; `cle` relance la
 *  lecture. `null` : rien à lire. */
export function useLecture<T>(chemin: string | null, cle: number = 0): Etat<T> {
  const sortie = useContext(ContexteSortie);
  const [lu, setLu] = useState<{ demande: string; etat: Etat<T> } | null>(null);
  const [tentative, setTentative] = useState(0);
  const demande = `${chemin ?? ""}#${cle}`;
  useEffect(() => {
    if (chemin === null) return;
    let fini = false;
    let minuterie: ReturnType<typeof setTimeout> | undefined;
    lire<T>(chemin)
      .then((donnee) => {
        if (!fini) setLu({ demande, etat: { etat: "pret", donnee } });
      })
      .catch((e: unknown) => {
        if (fini) return;
        if (e instanceof SortieDemandee) {
          sortie();
        } else if (e instanceof Occupe) {
          setLu({ demande, etat: { etat: "occupe" } });
          minuterie = setTimeout(() => setTentative((t) => t + 1), ATTENTE_MS);
        } else {
          setLu({ demande, etat: { etat: "introuvable" } });
        }
      });
    return () => {
      fini = true;
      if (minuterie !== undefined) clearTimeout(minuterie);
    };
  }, [chemin, demande, tentative, sortie]);
  if (lu === null || lu.demande !== demande) return { etat: "chargement" };
  return lu.etat;
}
