import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { Bouton } from "../../composants/Bouton";
import { Selection } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { t } from "../../i18n";
import type { DevoirAffiche } from "../../types/gesco";
import type { NotesAOuvrir } from "../pedagogie/Notes";

/** Points lisibles : « 20 », « 12,5 ». */
function points(centiemes: number): string {
  return (centiemes / 100).toLocaleString("fr-FR", { maximumFractionDigits: 2 });
}

const PONDERATIONS = ["1", "2", "3", "4", "5", "6", "7", "8", "9", "10"];

/** « Reporter au bulletin » : un geste, avec la période, le barème, la pondération et le nombre de notes. */
export function Report({ devoir, surNotes }: { devoir: DevoirAffiche; surNotes?: (cible: NotesAOuvrir) => void }) {
  const client = useQueryClient();
  const cle = ["devoirs", "report", devoir.id];
  const lu = useQuery({ queryKey: cle, queryFn: () => appeler("devoir_report", { id: devoir.id }) });
  const [periode, setPeriode] = useState<string | null>(null);
  const [ponderation, setPonderation] = useState<string | null>(null);
  const reporter = useMutation({
    mutationFn: (demande: { periode_id: string; ponderation: number }) => appeler("devoir_reporter", { id: devoir.id, ...demande }),
    onSuccess: (r) => {
      client.setQueryData(cle, r);
      void client.invalidateQueries({ queryKey: ["evaluations"] });
      void client.invalidateQueries({ queryKey: ["grille"] });
    },
  });
  const r = lu.data;
  if (r === undefined) return <MessageErreur erreur={lu.error} />;
  const periodeId = periode ?? r.periode_id ?? r.periodes[0]?.id ?? "";
  const poids = Number(ponderation ?? String(r.ponderation));
  return (
    <Section titre={t("devoirs.report")}>
      {r.evaluation === null ? null : <p className="text-sm text-encre">{t("devoirs.reporte", { ponderation: String(r.evaluation.ponderation) })}</p>}
      {r.notes_changees ? (
        <p role="status" className="text-sm font-medium text-ocre">
          {t("devoirs.notes_changees")}
        </p>
      ) : null}
      {r.evaluation?.validee === true ? (
        <div className="flex flex-wrap items-center gap-2">
          <p className="text-sm text-ocre">{t("devoirs.evaluation_validee")}</p>
          {surNotes === undefined ? null : (
            <Bouton
              variante="secondaire"
              onClick={() => {
                // L'évaluation du devoir s'ouvre aussitôt, dans sa classe, sa période et sa matière.
                const evaluation = r.evaluation;
                if (evaluation !== null) {
                  surNotes({ classeId: devoir.classe_id, periodeId: evaluation.periode_id, matiereId: devoir.matiere_id, evaluationId: evaluation.id });
                }
              }}
            >
              {t("devoirs.aller_aux_notes")}
            </Bouton>
          )}
        </div>
      ) : r.peut_reporter ? (
        <div className="flex flex-col gap-2">
          <div className="flex flex-wrap items-end gap-3">
            <Selection id="report-periode" libelle={t("devoirs.periode")} valeur={periodeId} surChangement={setPeriode} options={r.periodes.map((p) => ({ valeur: p.id, libelle: p.libelle }))} />
            <Selection id="report-ponderation" libelle={t("devoirs.ponderation")} valeur={String(poids)} surChangement={setPonderation} options={PONDERATIONS.map((p) => ({ valeur: p, libelle: p }))} />
          </div>
          <p className="text-sm text-encre">{t("devoirs.report_resume", { notes: String(r.notes), points: points(r.bareme), ponderation: String(poids) })}</p>
          <div>
            <Bouton onClick={() => reporter.mutate({ periode_id: periodeId, ponderation: poids })} disabled={reporter.isPending || periodeId === ""}>
              {r.evaluation === null ? t("devoirs.reporter") : t("devoirs.reporter_de_nouveau")}
            </Bouton>
          </div>
        </div>
      ) : (
        <p className="text-sm text-muet">{t("devoirs.report_reserve")}</p>
      )}
      <MessageErreur erreur={reporter.error} />
      {r.evaluation === null ? <p className="text-xs text-muet">{t("devoirs.report_aide")}</p> : null}
    </Section>
  );
}
