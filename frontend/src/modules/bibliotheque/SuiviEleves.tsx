import { useMutation, useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { ApercuPdf } from "../../composants/ApercuPdf";
import { Bouton } from "../../composants/Bouton";
import { MessageErreur, Section } from "../../composants/Messages";
import { date, t } from "../../i18n";
import type { DemandeSuivi, QuizSuivi, ScoreQuiz, SuiviClasse } from "../../types/gesco";

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
          <th scope="col" className="py-1 pr-2 text-right">{t("suivi.quiz_faits")}</th>
          <th scope="col" className="py-1">{t("suivi.derniere_visite")}</th>
        </tr>
      </thead>
      <tbody>
        {suivi.par_eleve.map((e) => (
          <tr key={e.eleve_id} className="border-b border-muet/40">
            <td className="py-1 pr-2">{e.nom}</td>
            <td className="py-1 pr-2 text-right">{t("suivi.sur", { nombre: e.ouvertes, eleves: total })}</td>
            <td className="py-1 pr-2 text-right">{t("suivi.sur", { nombre: e.finies, eleves: total })}</td>
            <td className="py-1 pr-2 text-right">{t("suivi.sur", { nombre: e.quiz_faits, eleves: suivi.quiz.length })}</td>
            <td className="py-1">{e.derniere_visite === null ? t("suivi.jamais") : date(e.derniere_visite)}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function score(s: ScoreQuiz | null): string {
  return s === null ? t("suivi.pas_fait") : t("suivi.score", { score: s.score, sur: s.sur });
}

/** Détail d'un quiz, par élève dans l'ordre alphabétique : premier essai, meilleur, essais. */
function DetailQuiz({ quiz }: { quiz: QuizSuivi }) {
  return (
    <table className="w-full text-left text-sm text-encre">
      <caption className="sr-only">{quiz.titre}</caption>
      <thead>
        <tr className="border-b border-muet">
          <th scope="col" className="py-1 pr-2">{t("suivi.eleve")}</th>
          <th scope="col" className="py-1 pr-2 text-right">{t("suivi.premier_essai")}</th>
          <th scope="col" className="py-1 pr-2 text-right">{t("suivi.meilleur_essai")}</th>
          <th scope="col" className="py-1 text-right">{t("suivi.essais")}</th>
        </tr>
      </thead>
      <tbody>
        {quiz.par_eleve.map((e) => (
          <tr key={e.eleve_id} className="border-b border-muet/40">
            <td className="py-1 pr-2">{e.nom}</td>
            <td className="py-1 pr-2 text-right">{score(e.premier)}</td>
            <td className="py-1 pr-2 text-right">{score(e.meilleur)}</td>
            <td className="py-1 text-right">{e.essais}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

/** Par quiz : combien d'élèves l'ont fait, la moyenne du premier essai, les questions les plus
 *  manquées (version par version) ; au grain « par élève », le détail de chacun. */
function ParQuiz({ suivi }: { suivi: SuiviClasse }) {
  const [ouvert, setOuvert] = useState<string | null>(null);
  return (
    <ul className="flex flex-col gap-3">
      {suivi.quiz.map((q) => (
        <li key={q.id} className="rounded border border-muet/60 p-3 text-sm text-encre">
          <p className="text-muet">{q.chapitre}</p>
          <h3 className="font-semibold">{q.titre}</h3>
          <p>
            {t("suivi.fait_par")} : {t("suivi.sur", { nombre: q.eleves, eleves: suivi.eleves })}
            {" · "}
            {t("suivi.premier_essai")} : {q.moyenne_premier === null ? t("suivi.pas_de_moyenne") : t("suivi.moyenne", { pourcent: q.moyenne_premier })}
          </p>
          {q.eleves === 0 ? null : (
            <div className="mt-1">
              <p className="font-medium">{t("suivi.manquees")}</p>
              {q.manquees.length === 0 ? (
                <p>{t("suivi.aucune_manquee")}</p>
              ) : (
                <ul className="list-disc pl-5">
                  {q.manquees.map((m) => (
                    <li key={(m.ancienne ? "a-" : "v-") + String(m.numero)}>
                      {t("suivi.question", { numero: m.numero })}
                      {m.ancienne ? " (" + t("suivi.ancienne_version") + ")" : ""}
                      {m.enonce === "" ? "" : " : " + m.enonce}
                      {" — "}
                      {t("suivi.fautes", { fautes: m.fautes, reponses: m.reponses })}
                    </li>
                  ))}
                </ul>
              )}
            </div>
          )}
          {q.par_eleve.length === 0 ? null : (
            <div className="mt-2 flex flex-col items-start gap-2">
              <Bouton variante="secondaire" onClick={() => setOuvert(ouvert === q.id ? null : q.id)} aria-expanded={ouvert === q.id}>
                {ouvert === q.id ? t("suivi.fermer_detail") : t("suivi.detail")}
              </Bouton>
              {ouvert === q.id ? <DetailQuiz quiz={q} /> : null}
            </div>
          )}
        </li>
      ))}
    </ul>
  );
}

/** « Suivi des élèves » : pour une classe et une matière, qui a ouvert et fini quelles leçons,
 *  et ce que la classe a fait des quiz.
 *  Un maître ne voit que ses classes et ses matières ; la direction voit tout. Aucun classement,
 *  aucune note ; suivi éteint, rien ne se montre. */
export function SuiviEleves() {
  const [demande, setDemande] = useState<DemandeSuivi>({});
  const [vue, setVue] = useState<"lecon" | "eleve" | "quiz">("lecon");
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
  const aDesQuiz = (s?.quiz.length ?? 0) > 0;
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
                {parEleve || aDesQuiz ? (
                  <div role="group" aria-label={t("suivi.vue")} className="flex flex-wrap gap-2">
                    <Bouton variante={vue === "lecon" ? "principal" : "secondaire"} onClick={() => setVue("lecon")} aria-pressed={vue === "lecon"}>
                      {t("suivi.par_lecon")}
                    </Bouton>
                    {parEleve ? (
                      <Bouton variante={vue === "eleve" ? "principal" : "secondaire"} onClick={() => setVue("eleve")} aria-pressed={vue === "eleve"}>
                        {t("suivi.par_eleve")}
                      </Bouton>
                    ) : null}
                    {aDesQuiz ? (
                      <Bouton variante={vue === "quiz" ? "principal" : "secondaire"} onClick={() => setVue("quiz")} aria-pressed={vue === "quiz"}>
                        {t("suivi.quiz")}
                      </Bouton>
                    ) : null}
                  </div>
                ) : null}
                <Bouton variante="secondaire" onClick={() => impression.mutate()} disabled={impression.isPending}>
                  {t("suivi.imprimer")}
                </Bouton>
              </div>
              {parEleve && vue === "eleve" ? <ParEleve suivi={s} /> : aDesQuiz && vue === "quiz" ? <ParQuiz suivi={s} /> : <ParLecon suivi={s} />}
            </>
          )}
          {pdf === null ? null : <ApercuPdf base64={pdf} />}
        </div>
      )}
    </Section>
  );
}
