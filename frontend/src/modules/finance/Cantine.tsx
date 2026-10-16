import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { aujourdHui } from "../../app/aujourdhui";
import { ApercuPdf } from "../../composants/ApercuPdf";
import { Bouton } from "../../composants/Bouton";
import { Champ } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { Tableau } from "../../composants/Tableau";
import { dateCivile, francs, moisCivil, t } from "../../i18n";
import type { AbonneCantine } from "../../types/gesco";

/** Cantine (SPEC_PHASE_8 §9) : les abonnés du jour d'après leurs factures, le repas pointé
 *  quand l'école les compte, la liste du jour imprimée, le bilan du mois. Les vivres
 *  s'achètent dans les dépenses, catégorie « Cantine (vivres) ». */
export function Cantine() {
  const client = useQueryClient();
  const [date, setDate] = useState(aujourdHui());
  const cle = ["cantine", date];
  const etat = useQuery({ queryKey: cle, queryFn: () => appeler("cantine_lire", { date }), enabled: /^\d{4}-\d{2}-\d{2}$/.test(date) });
  const pointer = useMutation({
    mutationFn: (a: AbonneCantine) => appeler("repas_pointer", { date, inscription_id: a.inscription_id, pris: !a.repas_pris }),
    onSuccess: (x) => client.setQueryData(cle, x),
  });
  const [document, setDocument] = useState<string | null>(null);
  const imprimer = useMutation({
    mutationFn: () => appeler("liste_cantine_produire", { date }),
    onSuccess: (d) => setDocument(d.pdf),
  });
  const c = etat.data;
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("cantine.titre")}</h1>
      <MessageErreur erreur={etat.error} />
      {c === undefined ? null : !c.active ? (
        <p className="text-sm text-muet">{t("cantine.inactive")}</p>
      ) : (
        <>
          <Section titre={t("cantine.du_jour", { date: dateCivile(c.date), abonnes: String(c.abonnes.length) })}>
            <Champ id="cantine-date" type="date" libelle={t("depense.date")} value={date} onChange={(ev) => setDate(ev.target.value)} />
            {c.repas_comptes ? <p className="text-sm text-muet">{t("cantine.aide_pointage", { repas: String(c.repas_du_jour) })}</p> : null}
            <Tableau
              nomExport={`cantine-${date}`}
              titre={t("cantine.du_jour", { date: dateCivile(c.date), abonnes: String(c.abonnes.length) })}
              lignes={c.abonnes}
              cle={(a) => a.inscription_id}
              surSelection={c.repas_comptes ? (a) => pointer.mutate(a) : undefined}
              colonnes={[
                { titre: t("cantine.classe"), valeur: (a) => a.classe },
                { titre: t("cantine.eleve"), valeur: (a) => a.eleve },
                { titre: t("cantine.reste_echu"), valeur: (a) => (a.reste_echu === 0 ? t("cantine.a_jour") : francs(a.reste_echu)) },
                ...(c.repas_comptes ? [{ titre: t("cantine.repas"), valeur: (a: AbonneCantine) => (a.repas_pris ? t("cantine.repas_pris") : "") }] : []),
              ]}
            />
            <MessageErreur erreur={pointer.error ?? imprimer.error} />
            <div>
              <Bouton variante="secondaire" onClick={() => imprimer.mutate()} disabled={imprimer.isPending}>
                {t("cantine.imprimer")}
              </Bouton>
            </div>
            {document === null ? null : <ApercuPdf base64={document} />}
          </Section>
          <Section titre={t("cantine.bilan", { mois: moisCivil(c.bilan.mois) })}>
            <p className="text-sm text-encre">
              {t("cantine.bilan_detail", { recettes: francs(c.bilan.recettes), vivres: francs(c.bilan.vivres), repas: String(c.bilan.repas) })}
            </p>
            <p className="text-sm text-muet">{t("cantine.aide_vivres")}</p>
          </Section>
        </>
      )}
    </div>
  );
}
