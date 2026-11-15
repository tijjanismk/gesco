import { useQuery } from "@tanstack/react-query";

import { appeler } from "../../api";
import { MessageErreur } from "../../composants/Messages";
import { date, t } from "../../i18n";

/** Les versions en attente de validation, lues pour qui valide ; relues après chaque geste de la
 *  bibliothèque (clé sous « bibliotheque »). */
function useEnAttente(actif: boolean) {
  return useQuery({
    queryKey: ["bibliotheque", "en_attente"],
    queryFn: () => appeler("bibliotheque_en_attente", null),
    enabled: actif,
  });
}

/** Nombre de leçons à valider ; zéro pour qui ne valide pas. */
export function useLeconsAValider(actif: boolean): number {
  return useEnAttente(actif).data?.length ?? 0;
}

/** L'onglet « En attente de validation » : chaque version soumise, à ouvrir dans l'éditeur. */
export function ListeEnAttente({ surOuvrir }: { surOuvrir: (contenuId: string) => void }) {
  const attente = useEnAttente(true);
  const liste = attente.data;
  return (
    <section aria-label={t("bibliotheque.en_attente")} className="flex flex-col gap-2 rounded border border-muet bg-surface p-4">
      <h2 className="text-lg font-semibold text-marine">{t("bibliotheque.en_attente")}</h2>
      <MessageErreur erreur={attente.error} />
      {liste === undefined ? null : liste.length === 0 ? (
        <p className="text-sm text-muet">{t("bibliotheque.en_attente.vide")}</p>
      ) : (
        <ul className="flex flex-col divide-y divide-muet/30">
          {liste.map((v) => (
            <li key={v.version_id} className="py-2">
              <button
                type="button"
                onClick={() => surOuvrir(v.contenu_id)}
                className="flex w-full flex-col items-start gap-0.5 rounded px-2 py-1 text-left hover:bg-fond focus-visible:outline-2 focus-visible:outline-primaire"
              >
                <span className="font-medium text-encre">{v.titre}</span>
                <span className="text-sm text-muet">
                  {[v.niveau, v.chapitre].filter((x) => x !== "").join(" · ")}
                </span>
                <span className="text-sm text-muet">
                  {t("bibliotheque.en_attente.soumise", { auteur: v.auteur, date: date(v.soumise_le) })}
                  {v.retour_livre ? ` · ${t("bibliotheque.historique.retour_livree")}` : ""}
                </span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
