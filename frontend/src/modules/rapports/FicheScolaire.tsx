import { useMutation } from "@tanstack/react-query";

import { appeler } from "../../api";
import { ApercuPdf } from "../../composants/ApercuPdf";
import { Bouton } from "../../composants/Bouton";
import { MessageErreur } from "../../composants/Messages";
import { t } from "../../i18n";

/** Fiche scolaire demandée par le CAP : une page par élève, pour un élève ou pour toute une
 *  classe ; composée à la demande, avec l'aperçu et l'impression. */
export function FicheScolaire({ eleveId = null, classeId = null }: { eleveId?: string | null; classeId?: string | null }) {
  const production = useMutation({ mutationFn: () => appeler("fiche_scolaire_produire", { eleve_id: eleveId, classe_id: classeId }) });
  return (
    <div className="flex flex-col gap-2">
      <div>
        <Bouton variante="secondaire" onClick={() => production.mutate()} disabled={production.isPending}>
          {classeId === null ? t("fiche_scolaire.bouton") : t("fiche_scolaire.classe")}
        </Bouton>
      </div>
      <MessageErreur erreur={production.error} />
      {production.data === undefined ? null : <ApercuPdf base64={production.data.pdf} />}
    </div>
  );
}
