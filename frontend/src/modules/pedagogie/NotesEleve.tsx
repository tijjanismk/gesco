import { useQuery } from "@tanstack/react-query";

import { appeler } from "../../api";
import { MessageErreur, Section } from "../../composants/Messages";
import { dateCivile, t } from "../../i18n";
import type { NoteDeLEleve } from "../../types/gesco";
import { points } from "./saisies";

function texteNote(n: NoteDeLEleve): string {
  if (n.note.dispense) return t("notes.dispense");
  if (n.note.absent) return t(n.note.justifie ? "notes.absent_justifie" : "notes.absent");
  return n.note.valeur === null ? "" : `${points(n.note.valeur)} / ${points(n.evaluation.bareme)}`;
}

/** Notes de l'élève, par classe et par période : celles prises dans une classe quittée en cours
 *  d'année y restent, même si le bulletin de la nouvelle classe ne compte que les siennes. */
export function NotesEleve({ eleveId }: { eleveId: string }) {
  const notes = useQuery({ queryKey: ["notes-eleve", eleveId], queryFn: () => appeler("eleve_notes", { id: eleveId }) });
  const groupes: { cle: string; titre: string; notes: NoteDeLEleve[] }[] = [];
  for (const n of notes.data ?? []) {
    const cle = `${n.evaluation.classe_id}-${n.evaluation.periode_id}`;
    const groupe = groupes.find((g) => g.cle === cle);
    if (groupe === undefined) groupes.push({ cle, titre: t("notes_eleve.groupe", { classe: n.classe, periode: n.periode }), notes: [n] });
    else groupe.notes.push(n);
  }
  return (
    <Section titre={t("notes_eleve.titre")}>
      <MessageErreur erreur={notes.error} />
      {notes.isSuccess && groupes.length === 0 ? <p className="text-sm text-muet">{t("notes_eleve.aucune")}</p> : null}
      {groupes.map((g) => (
        <div key={g.cle} className="flex flex-col gap-1 text-sm">
          <h3 className="font-medium text-encre">{g.titre}</h3>
          <table className="w-full text-left font-condensee tabular-nums">
            <thead>
              <tr className="text-muet">
                {(["notes_eleve.matiere", "notes_eleve.evaluation", "notes_eleve.date", "notes_eleve.note"] as const).map((cle) => (
                  <th key={cle} className="px-2 py-1 font-normal">{t(cle)}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {g.notes.map((n) => (
                <tr key={n.evaluation.id} className="border-b border-muet/30 text-encre">
                  <td className="px-2 py-1">{n.matiere}</td>
                  <td className="px-2 py-1">{n.evaluation.libelle}</td>
                  <td className="px-2 py-1">{dateCivile(n.evaluation.date)}</td>
                  <td className="px-2 py-1 font-medium">{texteNote(n)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ))}
    </Section>
  );
}
