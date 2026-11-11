import { useQuery, useQueryClient } from "@tanstack/react-query";
import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from "react";

import { appeler, type EtatLiaison, surEtatDeLiaison } from "../api";
import { type InfoPoste, lirePoste, verifierLiaison } from "../api/poste";

/** Délai entre deux essais de reprise quand le poste principal ne répond pas. */
const REPRISE_MS = 5_000;

interface Poste {
  /** Ce que le binaire sait du poste ; `null` tant qu'il n'a pas répondu. */
  info: InfoPoste | null;
  /** Liaison d'un poste relié : établie, coupée (le poste principal ne répond pas), ou refusée
   *  (il répond, mais ne veut plus de ce poste). Toujours établie sur un poste qui a sa base. */
  liaison: EtatLiaison;
}

const ContextePoste = createContext<Poste>({ info: null, liaison: "etablie" });

/** Rôle du poste et état de sa liaison. Sur un poste relié, une commande qui n'atteint pas le
 *  poste principal marque la liaison coupée ; le poste réessaie de lui-même, et quand le poste
 *  principal répond de nouveau, les écrans se relisent. La saisie en cours n'est pas touchée :
 *  rien n'est mis de côté, on enregistre de nouveau une fois la liaison revenue. */
export function FournisseurPoste({ children }: { children: ReactNode }) {
  const client = useQueryClient();
  const poste = useQuery({ queryKey: ["poste"], queryFn: lirePoste, staleTime: Infinity });
  const [liaison, setLiaison] = useState<EtatLiaison>("etablie");
  const relie = poste.data?.relie === true;
  const coupee = liaison === "coupee";

  useEffect(() => {
    surEtatDeLiaison((etat, commande) => {
      setLiaison(etat);
      // Le poste principal refuse ce poste (révoqué, autre version) : l'état du démarrage se
      // relit, et l'écran de la liaison en panne dit quoi faire.
      if (etat === "refusee" && commande !== "etat_demarrage") void client.invalidateQueries({ queryKey: ["etat"] });
    });
    return () => surEtatDeLiaison(null);
  }, [client]);

  useEffect(() => {
    if (!relie || !coupee) return;
    const minuterie = window.setInterval(() => {
      void verifierLiaison()
        .then(() => {
          setLiaison("etablie");
          void client.invalidateQueries();
        })
        .catch(() => undefined);
    }, REPRISE_MS);
    return () => window.clearInterval(minuterie);
  }, [relie, coupee, client]);

  const valeur = useMemo<Poste>(() => ({ info: poste.data ?? null, liaison: relie ? liaison : "etablie" }), [poste.data, relie, liaison]);
  return <ContextePoste.Provider value={valeur}>{children}</ContextePoste.Provider>;
}

export function usePoste(): Poste {
  return useContext(ContextePoste);
}

/** Vrai quand la session en cours vient d'un poste relié : les écrans de ce qui se fait sur le
 *  poste principal seulement n'y paraissent pas. La garde du noyau reste la seule décision. */
export function usePosteRelie(): boolean {
  const courante = useQuery({ queryKey: ["session"], queryFn: () => appeler("session_courante", null) });
  return courante.data?.role_poste === "relie";
}
