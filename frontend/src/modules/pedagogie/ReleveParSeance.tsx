import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { Bouton } from "../../composants/Bouton";
import { MessageErreur, Section } from "../../composants/Messages";
import { t, texteDeCle } from "../../i18n";
import type { ReleveParSeance as Etat } from "../../types/gesco";

/** Relevé par séance (SPEC_VERSION_1_1 §1, T-71), quand l'école le règle : une case par élève
 *  et par séance de l'emploi du temps ; absent à toutes les séances d'une demi-journée, l'élève
 *  la compte au relevé du jour, qui se justifie comme avant. */
export function ReleveParSeance({ classeId, jour }: { classeId: string; jour: string }) {
  const lecture = useQuery({ queryKey: ["releve-seances", classeId, jour], queryFn: () => appeler("absences_seances_lire", { classe_id: classeId, date: jour }) });
  const e = lecture.data;
  if (e === undefined || !e.actif) return <MessageErreur erreur={lecture.error} />;
  return <Grille key={`${classeId}-${jour}-${JSON.stringify(e.lignes)}`} e={e} classeId={classeId} jour={jour} />;
}

function Grille({ e, classeId, jour }: { e: Etat; classeId: string; jour: string }) {
  const client = useQueryClient();
  const [absents, setAbsents] = useState<Record<string, string[]>>(() => Object.fromEntries(e.lignes.map((l) => [l.inscription_id, l.absent_a])));
  const enregistrer = useMutation({
    mutationFn: () =>
      appeler("absences_seances_enregistrer", {
        classe_id: classeId,
        date: jour,
        lignes: e.lignes.filter((l) => !l.exclu).map((l) => ({ inscription_id: l.inscription_id, absent_a: absents[l.inscription_id] ?? [] })),
      }),
    onSuccess: (x) => {
      client.setQueryData(["releve-seances", classeId, jour], x);
      void client.invalidateQueries({ queryKey: ["releve-absences", classeId, jour] });
      void client.invalidateQueries({ queryKey: ["absences-periode", classeId] });
    },
  });
  const basculer = (inscription: string, seance: string) => {
    const siennes = absents[inscription] ?? [];
    setAbsents({ ...absents, [inscription]: siennes.includes(seance) ? siennes.filter((s) => s !== seance) : [...siennes, seance] });
  };
  return (
    <Section titre={t("absences.par_seance")}>
      <p className="text-sm text-muet">{t("absences.par_seance_aide")}</p>
      {e.refus === null ? null : <p className="text-sm text-ocre">{texteDeCle(e.refus)}</p>}
      {e.seances.length === 0 ? (
        <p className="text-sm text-muet">{t("absences.sans_seance_ce_jour")}</p>
      ) : (
        <>
          <table className="text-sm">
            <thead>
              <tr className="text-left text-muet">
                <th className="px-2 py-1 font-normal">{t("discipline.eleve")}</th>
                {e.seances.map((s) => (
                  <th key={s.seance_id} className="px-2 py-1 font-normal">
                    <div>{s.horaire}</div>
                    <div className="text-xs">{s.matiere}</div>
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {e.lignes.map((l) => (
                <tr key={l.inscription_id} className="border-b border-muet/30">
                  <td className="px-2 py-1 text-encre">{l.exclu ? t("absences.exclu_ce_jour", { eleve: l.eleve }) : l.eleve}</td>
                  {e.seances.map((s) => (
                    <td key={s.seance_id} className="px-2 py-1 text-center">
                      <input
                        type="checkbox"
                        aria-label={t("absences.absent_a", { eleve: l.eleve, horaire: s.horaire })}
                        className="accent-primaire"
                        checked={(absents[l.inscription_id] ?? []).includes(s.seance_id)}
                        disabled={l.exclu || e.refus !== null}
                        onChange={() => basculer(l.inscription_id, s.seance_id)}
                      />
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
          <div>
            <Bouton onClick={() => enregistrer.mutate()} disabled={e.refus !== null || enregistrer.isPending}>
              {t("absences.enregistrer_par_seance")}
            </Bouton>
          </div>
        </>
      )}
      <MessageErreur erreur={enregistrer.error} />
    </Section>
  );
}
