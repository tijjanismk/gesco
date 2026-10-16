import { useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { commandeAutorisee } from "../../app/navigation";
import { t } from "../../i18n";
import { FormulaireEncaissement } from "./Encaissement";
import { FraisEleve } from "./FraisEleve";

/** Quatrième étape de l'assistant UC-01 (SPEC_PHASE_3 §10) : la facture et l'échéancier nés de la
 *  validation, la remise à proposer, puis le premier versement et son reçu sans quitter l'écran
 *  si l'utilisateur encaisse et que sa caisse est ouverte ; sinon le parent passe à la caisse. */
export function EtapeFrais({ eleveId, permissions }: { eleveId: string; permissions: readonly string[] }) {
  const situation = useQuery({ queryKey: ["situation", eleveId], queryFn: () => appeler("situation_financiere", { id: eleveId }) });
  const caisse = useQuery({ queryKey: ["caisse"], queryFn: () => appeler("caisse_session_courante", null) });
  const [tour, setTour] = useState(0);
  const facturee = situation.data?.find((s) => s.inscription.statut === "validee" && s.facture !== null);
  const encaisse = commandeAutorisee("encaisser", permissions);
  return (
    <div className="flex flex-col gap-4">
      {situation.isSuccess && facturee === undefined ? <p className="text-sm text-ocre">{t("inscriptions.frais_apres_validation")}</p> : null}
      <FraisEleve eleveId={eleveId} permissions={permissions} />
      {facturee === undefined || !caisse.isSuccess ? null : encaisse && caisse.data !== null ? (
        <div className="flex flex-col gap-2">
          <h2 className="text-lg font-semibold text-encre">{t("inscriptions.premier_versement")}</h2>
          <FormulaireEncaissement key={tour} inscriptionId={facturee.inscription.id} autreEleve={false} surTermine={() => setTour(tour + 1)} />
        </div>
      ) : (
        <p role="status" className="text-sm text-encre">{t("inscriptions.passer_a_la_caisse")}</p>
      )}
    </div>
  );
}
