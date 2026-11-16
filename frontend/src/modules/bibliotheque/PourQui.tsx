import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { Bouton } from "../../composants/Bouton";
import { CaseACocher } from "../../composants/Champ";
import { MessageErreur } from "../../composants/Messages";
import { t } from "../../i18n";
import type { PourQuiContenu } from "../../types/gesco";

/** Ce que vise « Pour qui » : un contenu ou un chapitre de l'école. */
export interface CiblePourQui {
  contenu_id: string | null;
  chapitre_id: string | null;
}

/** « Pour qui » d'un ajout de l'école : tout le niveau, ou certaines classes du niveau de
 *  l'année en cours. Un maître ne coche que ses classes ; les autres gardent leur état. */
export function PourQui({
  cible,
  surFermer,
}: {
  cible: CiblePourQui;
  surFermer: () => void;
}) {
  const lu = useQuery({
    queryKey: ["bibliotheque", "pour-qui", cible.contenu_id, cible.chapitre_id],
    queryFn: () => appeler("bibliotheque_classes_lire", cible),
  });
  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="pour-qui-titre"
      className="fixed inset-0 z-50 flex items-center justify-center bg-encre/40 p-4"
    >
      <div className="flex max-h-full w-full max-w-md flex-col gap-3 overflow-y-auto rounded bg-surface p-5 text-encre">
        <h2 id="pour-qui-titre" className="text-lg font-semibold text-marine">
          {t("bibliotheque.pour_qui.titre")}
        </h2>
        <MessageErreur erreur={lu.error} />
        {lu.data === undefined ? (
          <div className="flex justify-end">
            <Bouton variante="secondaire" onClick={surFermer}>
              {t("commun.annuler")}
            </Bouton>
          </div>
        ) : (
          <Choix cible={cible} lu={lu.data} surFermer={surFermer} />
        )}
      </div>
    </div>
  );
}

/** Le choix, initialisé une fois d'après ce qui est lu. */
function Choix({
  cible,
  lu,
  surFermer,
}: {
  cible: CiblePourQui;
  lu: PourQuiContenu;
  surFermer: () => void;
}) {
  const client = useQueryClient();
  const [certaines, setCertaines] = useState(() =>
    lu.classes.some((c) => c.choisie),
  );
  const [choisies, setChoisies] = useState<ReadonlySet<string>>(
    () => new Set(lu.classes.filter((c) => c.choisie).map((c) => c.id)),
  );
  const regler = useMutation({
    mutationFn: () =>
      appeler("bibliotheque_classes_regler", {
        ...cible,
        classes: certaines ? [...choisies] : [],
      }),
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: ["bibliotheque"] });
      surFermer();
    },
  });
  const basculer = (id: string, coche: boolean) => {
    const suite = new Set(choisies);
    if (coche) suite.add(id);
    else suite.delete(id);
    setChoisies(suite);
  };
  return (
    <>
      <fieldset className="flex flex-col gap-2">
        <legend className="sr-only">{t("bibliotheque.pour_qui.titre")}</legend>
        <label className="flex items-center gap-2 text-sm">
          <input
            type="radio"
            name="pour-qui"
            checked={!certaines}
            onChange={() => setCertaines(false)}
            disabled={!lu.modifiable}
          />
          {t("bibliotheque.pour_qui.tout_le_niveau")}
        </label>
        <label className="flex items-center gap-2 text-sm">
          <input
            type="radio"
            name="pour-qui"
            checked={certaines}
            onChange={() => setCertaines(true)}
            disabled={!lu.modifiable}
          />
          {t("bibliotheque.pour_qui.certaines")}
        </label>
        {certaines ? (
          <div className="flex flex-col gap-1 pl-6">
            {lu.classes.map((c) => (
              <CaseACocher
                key={c.id}
                id={`pour-qui-${c.id}`}
                libelle={c.libelle}
                coche={choisies.has(c.id)}
                desactive={!lu.modifiable || !c.permise}
                surChangement={(coche) => basculer(c.id, coche)}
              />
            ))}
          </div>
        ) : null}
      </fieldset>
      <MessageErreur erreur={regler.error} />
      <div className="flex justify-end gap-2">
        <Bouton variante="secondaire" onClick={surFermer}>
          {t("commun.annuler")}
        </Bouton>
        <Bouton
          onClick={() => regler.mutate()}
          disabled={!lu.modifiable || regler.isPending}
        >
          {t("commun.enregistrer")}
        </Bouton>
      </div>
    </>
  );
}

/** La marque « 9e A seulement » d'un ajout réservé à certaines classes. */
export function MarqueClasses({ classes }: { classes: readonly string[] }) {
  if (classes.length === 0) return null;
  return (
    <span className="rounded bg-fond px-1.5 text-xs text-encre">
      {t("bibliotheque.pour_qui.marque", { classes: classes.join(", ") })}
    </span>
  );
}
