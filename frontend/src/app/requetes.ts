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

/**
 * Données de référence, que la direction règle rarement : classes, niveaux et séries, années,
 * réglages. À plusieurs postes, elles gardent un délai court au lieu d'être redemandées à chaque
 * écran ; les listes et les fiches, elles, se relisent à l'ouverture et au retour sur l'écran.
 */
const DONNEES_DE_REFERENCE: readonly string[] = [
  "referentiel",
  "classes",
  "annees",
  "parametres",
  "scolarite-reglages",
  "personnel-referentiel",
];

/** Durée pendant laquelle une donnée de référence déjà lue n'est pas redemandée. */
const DELAI_REFERENCE = 30_000;

const estReference = (cle: readonly unknown[]): boolean => typeof cle[0] === "string" && DONNEES_DE_REFERENCE.includes(cle[0]);

/**
 * Refus qui ne disent rien de l'état des données, ou pendant lesquels une relecture serait refusée
 * elle aussi : liaison coupée, session fermée ou verrouillée, opération qui demande d'être seul.
 */
const sansRelecture = (cle: string): boolean =>
  cle === "erreur.transport" ||
  cle === "reseau.poste_principal_injoignable" ||
  cle === "session.verrouillee" ||
  cle === "session.invalide" ||
  cle.startsWith("poste.occupe.");

export const clientRequetes: QueryClient = new QueryClient({
  queryCache: new QueryCache({ onError: signaler }),
  mutationCache: new MutationCache({
    onError: (erreur) => {
      signaler(erreur);
      // Après un refus, ce que l'écran croyait vrai ne l'est peut-être plus : les données de
      // référence affichées se relisent tout de suite, le reste à la prochaine ouverture. Les
      // fiches ouvertes ne sont pas relues d'office : la saisie en cours reste à l'écran.
      if (erreur instanceof ErreurApi && !sansRelecture(erreur.cle)) {
        void clientRequetes.invalidateQueries({ predicate: (q) => estReference(q.queryKey) });
        void clientRequetes.invalidateQueries({ predicate: (q) => !estReference(q.queryKey), refetchType: "none" });
      }
    },
    // Ce poste vient d'écrire : ses données de référence ne profitent plus du délai.
    onSuccess: () => {
      void clientRequetes.invalidateQueries({ predicate: (q) => estReference(q.queryKey), refetchType: "none" });
    },
  }),
  // Une liste ou une fiche qui n'est plus à l'écran n'est pas gardée : en y revenant, l'écran la
  // relit avant de l'afficher, et un formulaire ne se remplit jamais d'une lecture ancienne.
  defaultOptions: { queries: { retry: false, refetchOnWindowFocus: false, gcTime: 0 } },
});

for (const cle of DONNEES_DE_REFERENCE) {
  clientRequetes.setQueryDefaults([cle], { staleTime: DELAI_REFERENCE, gcTime: 10 * DELAI_REFERENCE });
}
