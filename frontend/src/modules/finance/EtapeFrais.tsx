import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { commandeAutorisee } from "../../app/navigation";
import { CaseACocher } from "../../composants/Champ";
import { MessageErreur } from "../../composants/Messages";
import { t } from "../../i18n";
import { FormulaireEncaissement } from "./Encaissement";
import { FraisEleve } from "./FraisEleve";
import { APRES_PRISE_EN_CHARGE } from "./prise_en_charge";

/** Quatrième étape de l'assistant UC-01 (SPEC_PHASE_3 §10) : la facture et l'échéancier nés de la
 *  validation, la remise à proposer, puis le premier versement et son reçu sans quitter l'écran
 *  si l'utilisateur encaisse et que sa caisse est ouverte ; sinon le parent passe à la caisse. */
export function EtapeFrais({ eleveId, permissions }: { eleveId: string; permissions: readonly string[] }) {
  const situation = useQuery({ queryKey: ["situation", eleveId], queryFn: () => appeler("situation_financiere", { id: eleveId }) });
  const caisse = useQuery({ queryKey: ["caisse"], queryFn: () => appeler("caisse_session_courante", null) });
  const [tour, setTour] = useState(0);
  const facturee = situation.data?.find((s) => s.inscription.statut === "validee" && s.facture !== null);
  const encaisse = commandeAutorisee("encaisser", permissions);
  // Classe du lycée ou du technique : la case « Pris en charge par l'État », au seul titulaire de
  // la remise ; cochée, la facture se partage et le premier versement ne porte que la part du
  // parent.
  const client = useQueryClient();
  const case_ = useMutation({
    mutationFn: (coche: boolean) =>
      appeler("prise_en_charge_regler", {
        cibles: [{ inscription_id: facturee?.inscription.id ?? "", lue_le: null }],
        geste: coche ? "cocher" : "decocher",
        motif: coche ? "" : t("pec.motif_propose"),
      }),
    onSuccess: () => {
      for (const cle of APRES_PRISE_EN_CHARGE) void client.invalidateQueries({ queryKey: [cle] });
      setTour((n) => n + 1);
    },
  });
  const coche = facturee?.facture?.prise_en_charge?.statut === "en_cours";
  return (
    <div className="flex flex-col gap-4">
      {situation.isSuccess && facturee === undefined ? <p className="text-sm text-ocre">{t("inscriptions.frais_apres_validation")}</p> : null}
      {facturee !== undefined && facturee.prise_en_charge_possible && commandeAutorisee("prise_en_charge_regler", permissions) ? (
        <div className="flex flex-col gap-1">
          <CaseACocher id="etape-frais-prise-en-charge" libelle={t("pec.case_inscription")} coche={coche} desactive={case_.isPending} surChangement={(c) => case_.mutate(c)} />
          <p className="text-sm text-muet">{t("pec.case_inscription_aide")}</p>
          <MessageErreur erreur={case_.error} />
        </div>
      ) : null}
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
