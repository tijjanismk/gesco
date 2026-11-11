import { useQueryClient } from "@tanstack/react-query";
import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from "react";

import { definirJeton } from "../api";

interface EtatSession {
  jeton: string | null;
  verrouillee: boolean;
}

interface Session extends EtatSession {
  ouvrir: (jeton: string) => void;
  verrouiller: () => void;
  fermer: () => void;
}

const ContexteSession = createContext<Session | null>(null);

// Le jeton survit à une actualisation de la page (Ctrl + R), pas à la fermeture de GESCO : le
// stockage de session s'efface avec la fenêtre. Le noyau reste seul juge : un jeton expiré ou
// verrouillé est refusé à la première commande, et l'écran suit.
const CLE_JETON = "gesco.jeton";

function jetonGarde(): string | null {
  try {
    return window.sessionStorage.getItem(CLE_JETON);
  } catch {
    return null;
  }
}

function garderJeton(jeton: string | null): void {
  try {
    if (jeton === null) window.sessionStorage.removeItem(CLE_JETON);
    else window.sessionStorage.setItem(CLE_JETON, jeton);
  } catch {
    return;
  }
}

/** Ce qui ne dépend pas de la session survit à son ouverture et à sa fermeture : l'état du
 *  démarrage et le rôle du poste. */
const garderHorsSession = (q: { queryKey: readonly unknown[] }): boolean => q.queryKey[0] !== "etat" && q.queryKey[0] !== "poste";

function etatInitial(): EtatSession {
  const jeton = jetonGarde();
  definirJeton(jeton);
  return { jeton, verrouillee: false };
}

/** Session de l'utilisateur au clavier. Le noyau décide ; le frontend ne fait que suivre
 *  (verrouillage, bascule, fermeture) pour afficher le bon écran (RA-13). */
export function FournisseurSession({ children }: { children: ReactNode }) {
  const client = useQueryClient();
  const [etat, setEtat] = useState<EtatSession>(etatInitial);
  const ouvrir = useCallback(
    (jeton: string) => {
      definirJeton(jeton);
      garderJeton(jeton);
      client.removeQueries({ predicate: garderHorsSession });
      setEtat({ jeton, verrouillee: false });
    },
    [client],
  );
  const verrouiller = useCallback(() => setEtat((e) => ({ ...e, verrouillee: true })), []);
  const fermer = useCallback(() => {
    definirJeton(null);
    garderJeton(null);
    client.removeQueries({ predicate: garderHorsSession });
    setEtat({ jeton: null, verrouillee: false });
  }, [client]);
  const valeur = useMemo(
    () => ({ ...etat, ouvrir, verrouiller, fermer }),
    [etat, ouvrir, verrouiller, fermer],
  );
  return <ContexteSession.Provider value={valeur}>{children}</ContexteSession.Provider>;
}

export function useSession(): Session {
  const session = useContext(ContexteSession);
  if (session === null) throw new Error("FournisseurSession absent");
  return session;
}
