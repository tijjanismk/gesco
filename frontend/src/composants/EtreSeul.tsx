import { useQueryClient } from "@tanstack/react-query";
import { type ReactNode, useState } from "react";

import { appeler } from "../api";
import { type Cle, t } from "../i18n";
import type { SessionCourante } from "../types/gesco";

import { Bouton } from "./Bouton";

/** Ce que l'opération fait aux autres postes, dit dans la confirmation. */
type Operation = "seul.cloture" | "seul.restauration" | "seul.import";

/**
 * Opérations qui demandent d'être seul : clôture de l'année, restauration, imports. Sur le poste
 * principal, quand des postes reliés ont une session ouverte, la confirmation les nomme avant de
 * lancer l'opération ; le temps qu'elle dure, le poste principal refuse leurs commandes. Sur un
 * poste seul ou relié, ou sans poste connecté, l'opération part tout de suite.
 */
export function useEtreSeul(operation: Operation): { demander: (lancer: () => void) => void; dialogue: ReactNode } {
  const client = useQueryClient();
  const [attente, setAttente] = useState<{ postes: readonly string[]; lancer: () => void } | null>(null);
  const demander = (lancer: () => void) => {
    const session = client.getQueryData<SessionCourante>(["session"]);
    if (session?.role_poste !== "principal") return lancer();
    appeler("postes_connectes_lire", null).then(
      (postes) => (postes.length === 0 ? lancer() : setAttente({ postes, lancer })),
      // Sans la liste, l'opération dira elle-même ce qui ne va pas.
      () => lancer(),
    );
  };
  const explication: Cle = operation;
  const dialogue =
    attente === null ? null : (
      <div className="fixed inset-0 z-50 flex items-start justify-center bg-encre/40 p-6">
        <div role="alertdialog" aria-modal="true" aria-labelledby="seul-titre" className="mt-24 flex max-w-lg flex-col gap-3 rounded border border-muet/30 bg-surface p-6 shadow-lg">
          <h2 id="seul-titre" className="text-lg font-semibold text-encre">{t("fermeture.titre")}</h2>
          <p className="text-sm text-encre">{t("seul.postes")}</p>
          <ul className="list-disc pl-6 text-sm font-semibold text-encre">
            {attente.postes.map((poste) => (
              <li key={poste}>{poste}</li>
            ))}
          </ul>
          <p className="text-sm text-encre">{t(explication)}</p>
          <p className="text-sm text-muet">{t("seul.aide")}</p>
          <div className="flex flex-wrap gap-2">
            <Bouton variante="secondaire" autoFocus onClick={() => setAttente(null)}>{t("commun.annuler")}</Bouton>
            <Bouton
              variante="danger"
              onClick={() => {
                attente.lancer();
                setAttente(null);
              }}
            >
              {t("seul.continuer")}
            </Bouton>
          </div>
        </div>
      </div>
    );
  return { demander, dialogue };
}
