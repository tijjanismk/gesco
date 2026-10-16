import { QueryCache, QueryClient, MutationCache } from "@tanstack/react-query";

import { ErreurApi } from "../api";

type ReactionSession = (cle: "session.verrouillee" | "session.invalide") => void;
let reaction: ReactionSession | null = null;

/** Abonne l'interface aux refus de session : verrouillage par inactivité ou session fermée. */
export function surRefusDeSession(fonction: ReactionSession | null): void {
  reaction = fonction;
}

function signaler(erreur: unknown) {
  if (erreur instanceof ErreurApi && (erreur.cle === "session.verrouillee" || erreur.cle === "session.invalide")) {
    reaction?.(erreur.cle);
  }
}

export const clientRequetes = new QueryClient({
  queryCache: new QueryCache({ onError: signaler }),
  mutationCache: new MutationCache({ onError: signaler }),
  defaultOptions: { queries: { retry: false, refetchOnWindowFocus: false } },
});
