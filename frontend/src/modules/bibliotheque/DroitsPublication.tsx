import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { Bouton } from "../../composants/Bouton";
import { Selection } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { date, t } from "../../i18n";
import type { DroitsPublication as Droits } from "../../types/gesco";

/** « Peuvent publier eux-mêmes » : les maîtres à qui la direction donne nommément le droit de
 *  publier sans soumettre, dans les matières et niveaux de leurs affectations. Le retrait vaut
 *  aussitôt. */
export function DroitsPublication() {
  const client = useQueryClient();
  const [choisi, setChoisi] = useState("");
  const lecture = useQuery({
    queryKey: ["bibliotheque", "droits_publication"],
    queryFn: () => appeler("bibliotheque_droits_publication_lire", null),
  });
  const apres = (d: Droits) => {
    client.setQueryData(["bibliotheque", "droits_publication"], d);
    setChoisi("");
  };
  const donner = useMutation({
    mutationFn: (utilisateur_id: string) => appeler("bibliotheque_droit_publication_donner", { utilisateur_id }),
    onSuccess: apres,
  });
  const retirer = useMutation({
    mutationFn: (utilisateur_id: string) => appeler("bibliotheque_droit_publication_retirer", { utilisateur_id }),
    onSuccess: apres,
  });
  const d = lecture.data;
  return (
    <Section titre={t("bibliotheque.droits.titre")}>
      <p className="text-sm text-encre">{t("bibliotheque.droits.aide")}</p>
      <MessageErreur erreur={lecture.error} />
      {d === undefined ? null : (
        <>
          {d.droits.length === 0 ? (
            <p className="text-sm text-muet">{t("bibliotheque.droits.aucun")}</p>
          ) : (
            <ul className="flex flex-col divide-y divide-muet/30">
              {d.droits.map((m) => (
                <li key={m.utilisateur_id} className="flex flex-wrap items-center justify-between gap-2 py-2 text-sm text-encre">
                  <span>
                    <span className="font-medium">{m.nom}</span>
                    <span className="text-muet"> · {t("bibliotheque.droits.donne", { nom: m.donne_par, date: date(m.donne_le) })}</span>
                  </span>
                  <Bouton variante="secondaire" onClick={() => retirer.mutate(m.utilisateur_id)} disabled={retirer.isPending}>
                    {t("bibliotheque.droits.retirer")}
                  </Bouton>
                </li>
              ))}
            </ul>
          )}
          {d.maitres.length === 0 ? null : (
            <div className="flex flex-wrap items-end gap-2">
              <Selection
                id="bibliotheque-droit-maitre"
                libelle={t("bibliotheque.droits.maitre")}
                valeur={choisi}
                surChangement={setChoisi}
                options={[
                  { valeur: "", libelle: t("bibliotheque.droits.choisir") },
                  ...d.maitres.map((m) => ({ valeur: m.utilisateur_id, libelle: m.nom })),
                ]}
              />
              <Bouton onClick={() => donner.mutate(choisi)} disabled={choisi === "" || donner.isPending}>
                {t("bibliotheque.droits.donner")}
              </Bouton>
            </div>
          )}
          <MessageErreur erreur={donner.error} />
          <MessageErreur erreur={retirer.error} />
        </>
      )}
    </Section>
  );
}
