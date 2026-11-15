import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { appeler } from "../../api";
import { Selection } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { t } from "../../i18n";

/** Réglage de la direction : sous quelle matière de l'école se rangent les cours de chaque
 *  discipline de la bibliothèque, niveau par niveau. Sans matière, les cours gardent le nom de
 *  la bibliothèque ; rien ne disparaît. */
export function CorrespondanceBibliotheque() {
  const client = useQueryClient();
  const lecture = useQuery({
    queryKey: ["bibliotheque", "correspondances"],
    queryFn: () => appeler("bibliotheque_correspondances_lire", null),
  });
  const reglage = useMutation({
    mutationFn: (d: { discipline_code: string; niveau_code: string; matiere_id: string | null }) =>
      appeler("bibliotheque_correspondance_regler", d),
    onSuccess: (r) => {
      client.setQueryData(["bibliotheque", "correspondances"], r);
      void client.invalidateQueries({ queryKey: ["bibliotheque", "catalogue"] });
    },
  });
  const donnees = lecture.data;
  return (
    <Section titre={t("bibliotheque.correspondance.titre")}>
      <p className="text-sm text-encre">{t("bibliotheque.correspondance.aide")}</p>
      <MessageErreur erreur={lecture.error} />
      <MessageErreur erreur={reglage.error} />
      {donnees === undefined ? null : donnees.correspondances.length === 0 ? (
        <p className="text-sm text-muet">{t("bibliotheque.correspondance.aucune")}</p>
      ) : (
        <div className="grid gap-3 md:grid-cols-2">
          {donnees.correspondances.map((c) => (
            <Selection
              key={`${c.discipline_code}-${c.niveau_code}`}
              id={`correspondance-${c.discipline_code}-${c.niveau_code}`}
              libelle={t("bibliotheque.correspondance.ligne", { discipline: c.discipline, niveau: c.niveau })}
              valeur={c.matiere_id ?? ""}
              surChangement={(v) =>
                reglage.mutate({ discipline_code: c.discipline_code, niveau_code: c.niveau_code, matiere_id: v === "" ? null : v })
              }
              options={[
                { valeur: "", libelle: t("bibliotheque.correspondance.reference", { discipline: c.discipline }) },
                ...donnees.matieres.map((m) => ({ valeur: m.id, libelle: m.libelle })),
              ]}
            />
          ))}
        </div>
      )}
    </Section>
  );
}
