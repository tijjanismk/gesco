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

/** Session de l'utilisateur au clavier. Le noyau décide ; le frontend ne fait que suivre
 *  (verrouillage, bascule, fermeture) pour afficher le bon écran (RA-13). */
export function FournisseurSession({ children }: { children: ReactNode }) {
  const client = useQueryClient();
  const [etat, setEtat] = useState<EtatSession>({ jeton: null, verrouillee: false });
  const ouvrir = useCallback(
    (jeton: string) => {
      definirJeton(jeton);
      client.removeQueries({ predicate: (q) => q.queryKey[0] !== "etat" });
      setEtat({ jeton, verrouillee: false });
    },
    [client],
  );
  const verrouiller = useCallback(() => setEtat((e) => ({ ...e, verrouillee: true })), []);
  const fermer = useCallback(() => {
    definirJeton(null);
    client.removeQueries({ predicate: (q) => q.queryKey[0] !== "etat" });
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
