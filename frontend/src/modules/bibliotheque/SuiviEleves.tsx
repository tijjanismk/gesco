import { useMutation, useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { ApercuPdf } from "../../composants/ApercuPdf";
import { Bouton } from "../../composants/Bouton";
import { MessageErreur, Section } from "../../composants/Messages";
import { date, t } from "../../i18n";
import type { DemandeSuivi, SuiviClasse } from "../../types/gesco";

const CHAMP = "rounded border border-muet bg-surface px-2 py-1.5 text-primaire focus:outline-2 focus:outline-primaire";

/** Choix de la classe et de la matière. */
function Choix({ suivi, demande, surChanger }: { suivi: SuiviClasse; demande: DemandeSuivi; surChanger: (d: DemandeSuivi) => void }) {
  return (
    <div className="flex flex-wrap items-end gap-3">
      <label htmlFor="suivi-classe" className="flex flex-col gap-1 text-sm text-encre">
        <span>{t("suivi.classe")}</span>
        <select
          id="suivi-classe"
          value={suivi.classe_id ?? ""}
          onChange={(e) => surChanger({ classe_id: e.target.value, matiere_cle: null })}
          className={CHAMP}
        >
          {suivi.classes.map((c) => (
            <option key={c.id} value={c.id}>
              {c.libelle}
            </option>
          ))}
        </select>
      </label>
      {suivi.matieres.length === 0 ? null : (
        <label htmlFor="suivi-matiere" className="flex flex-col gap-1 text-sm text-encre">
          <span>{t("suivi.matiere")}</span>
          <select
            id="suivi-matiere"
            value={suivi.matiere_cle ?? ""}
            onChange={(e) => surChanger({ ...demande, classe_id: suivi.classe_id, matiere_cle: e.target.value })}
            className={CHAMP}
          >
            {suivi.matieres.map((m) => (
              <option key={m.cle} value={m.cle}>
                {m.libelle}
              </option>
            ))}
          </select>
        </label>
      )}
    </div>
  );
}

/** Par leçon : combien d'élèves l'ont ouverte et finie, dans l'ordre du programme. */
function ParLecon({ suivi }: { suivi: SuiviClasse }) {
  return (
    <table className="w-full text-left text-sm text-encre">
      <caption className="sr-only">{t("suivi.par_lecon")}</caption>
      <thead>
        <tr className="border-b border-muet">
          <th scope="col" className="py-1 pr-2">{t("suivi.chapitre")}</th>
          <th scope="col" className="py-1 pr-2">{t("suivi.lecon")}</th>
          <th scope="col" className="py-1 pr-2 text-right">{t("suivi.ouverte_par")}</th>
          <th scope="col" className="py-1 text-right">{t("suivi.finie_par")}</th>
        </tr>
      </thead>
      <tbody>
        {suivi.lecons.map((l) => (
          <tr key={l.id} className="border-b border-muet/40">
            <td className="py-1 pr-2 text-muet">{l.chapitre}</td>
            <td className="py-1 pr-2">{l.titre}</td>
            <td className="py-1 pr-2 text-right">{t("suivi.sur", { nombre: l.ouvertes, eleves: suivi.eleves })}</td>
            <td className="py-1 text-right">{t("suivi.sur", { nombre: l.finies, eleves: suivi.eleves })}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

/** Par élève, dans l'ordre alphabétique : ses leçons ouvertes et finies, sa dernière visite. */
function ParEleve({ suivi }: { suivi: SuiviClasse }) {
  const total = suivi.lecons.length;
  return (
    <table className="w-full text-left text-sm text-encre">
      <caption className="sr-only">{t("suivi.par_eleve")}</caption>
      <thead>
        <tr className="border-b border-muet">
          <th scope="col" className="py-1 pr-2">{t("suivi.eleve")}</th>
          <th scope="col" className="py-1 pr-2 text-right">{t("suivi.ouvertes")}</th>
          <th scope="col" className="py-1 pr-2 text-right">{t("suivi.finies")}</th>
          <th scope="col" className="py-1">{t("suivi.derniere_visite")}</th>
        </tr>
      </thead>
      <tbody>
        {suivi.par_eleve.map((e) => (
          <tr key={e.eleve_id} className="border-b border-muet/40">
            <td className="py-1 pr-2">{e.nom}</td>
            <td className="py-1 pr-2 text-right">{t("suivi.sur", { nombre: e.ouvertes, eleves: total })}</td>
            <td className="py-1 pr-2 text-right">{t("suivi.sur", { nombre: e.finies, eleves: total })}</td>
            <td className="py-1">{e.derniere_visite === null ? t("suivi.jamais") : date(e.derniere_visite)}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

/** « Suivi des élèves » : pour une classe et une matière, qui a ouvert et fini quelles leçons.
 *  Un maître ne voit que ses classes et ses matières ; la direction voit tout. Aucun classement,
 *  aucune note ; suivi éteint, rien ne se montre. */
export function SuiviEleves() {
  const [demande, setDemande] = useState<DemandeSuivi>({});
  const [vue, setVue] = useState<"lecon" | "eleve">("lecon");
  const [pdf, setPdf] = useState<string | null>(null);
  const suivi = useQuery({
    queryKey: ["bibliotheque", "suivi", demande],
    queryFn: () => appeler("bibliotheque_suivi_lire", demande),
    refetchInterval: 10_000,
  });
  const impression = useMutation({
    mutationFn: () => appeler("bibliotheque_suivi_imprimer", { classe_id: suivi.data?.classe_id ?? null, matiere_cle: suivi.data?.matiere_cle ?? null }),
    onSuccess: (r) => setPdf(r.pdf),
  });
  const s = suivi.data;
  const parEleve = s?.grain === "eleve";
  return (
    <Section titre={t("suivi.titre")}>
      <MessageErreur erreur={suivi.error ?? impression.error} />
      {s === undefined ? null : !s.actif ? (
        <p className="text-sm text-encre">{t("suivi.eteint")}</p>
      ) : s.classes.length === 0 ? (
        <p className="text-sm text-encre">{t("suivi.aucune_classe")}</p>
      ) : (
        <div className="flex flex-col gap-3">
          <Choix
            suivi={s}
            demande={demande}
            surChanger={(d) => {
              setPdf(null);
              setDemande(d);
            }}
          />
          {s.lecons.length === 0 ? (
            <p className="text-sm text-encre">{t("suivi.aucune_lecon")}</p>
          ) : (
            <>
              <div className="flex flex-wrap gap-2">
                {parEleve ? (
                  <div role="group" aria-label={t("suivi.vue")} className="flex gap-2">
                    <Bouton variante={vue === "lecon" ? "principal" : "secondaire"} onClick={() => setVue("lecon")} aria-pressed={vue === "lecon"}>
                      {t("suivi.par_lecon")}
                    </Bouton>
                    <Bouton variante={vue === "eleve" ? "principal" : "secondaire"} onClick={() => setVue("eleve")} aria-pressed={vue === "eleve"}>
                      {t("suivi.par_eleve")}
                    </Bouton>
                  </div>
                ) : null}
                <Bouton variante="secondaire" onClick={() => impression.mutate()} disabled={impression.isPending}>
                  {t("suivi.imprimer")}
                </Bouton>
              </div>
              {parEleve && vue === "eleve" ? <ParEleve suivi={s} /> : <ParLecon suivi={s} />}
            </>
          )}
          {pdf === null ? null : <ApercuPdf base64={pdf} />}
        </div>
      )}
    </Section>
  );
}
