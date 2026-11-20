import { useQuery } from "@tanstack/react-query";
import { useEffect, useMemo, useState } from "react";

import { appeler } from "../../api";
import { Bouton } from "../../composants/Bouton";
import { MessageErreur } from "../../composants/Messages";
import { t } from "../../i18n";
import type { ChoixQuestion, Enligne, QuestionQuiz } from "../../types/gesco";
import { Enlignes, imagesParCle, RenduLecon } from "./RenduLecon";

/** Réponse en cours de saisie : lettres choisies, réponse écrite, ou paires d'un appariement. */
interface Saisie {
  lettres: string[];
  ecrite: string;
  paires: Record<string, string>;
}

const SAISIE_VIDE: Saisie = { lettres: [], ecrite: "", paires: {} };

/** La réponse envoyée à la correction : « B », « A,C », « A-2,B-1 » ou la réponse écrite. */
function reponseDe(q: QuestionQuiz, s: Saisie): string {
  switch (q.sorte) {
    case "reponse_courte":
      return s.ecrite;
    case "appariement":
      return q.choix
        .filter((c) => (s.paires[c.repere] ?? "") !== "")
        .map((c) => `${c.repere}-${s.paires[c.repere] ?? ""}`)
        .join(",");
    default:
      return [...s.lettres].sort().join(",");
  }
}

/** Le texte d'éléments de ligne, sans mise en forme (pour une liste à choisir). */
function texteBrut(elements: readonly Enligne[]): string {
  return elements
    .map((x) => {
      switch (x.type) {
        case "texte":
        case "code":
          return x.texte;
        case "gras":
        case "italique":
          return texteBrut(x.contenu);
        case "formule":
          return x.latex;
        case "image":
          return x.description;
        default:
          return "";
      }
    })
    .join("");
}

/** Vrai si quelque chose est répondu. */
function repondu(q: QuestionQuiz, s: Saisie): boolean {
  return reponseDe(q, s).trim() !== "";
}

/** La bonne réponse, écrite pour l'élève. */
function BonneReponse({ q, images }: { q: QuestionQuiz; images: ReadonlyMap<string, string> }) {
  const choix = (repere: string, liste: readonly ChoixQuestion[]) => liste.find((c) => c.repere === repere);
  if (q.sorte === "reponse_courte") {
    return <span>{q.bonnes.map((b) => `« ${b} »`).join(` ${t("quiz.ou")} `)}</span>;
  }
  if (q.sorte === "appariement") {
    return (
      <ul className="list-inside list-disc">
        {q.bonnes.map((b) => {
          const [l, n] = b.split("-");
          const a = choix(l ?? "", q.choix);
          const e = choix(n ?? "", q.elements);
          return (
            <li key={b}>
              {a === undefined ? l : <Enlignes elements={a.texte} images={images} />} → {e === undefined ? n : <Enlignes elements={e.texte} images={images} />}
            </li>
          );
        })}
      </ul>
    );
  }
  return (
    <span className="flex flex-col">
      {q.bonnes.map((b) => {
        const c = choix(b, q.choix);
        return (
          <span key={b}>
            {b}) {c === undefined ? null : <Enlignes elements={c.texte} images={images} />}
          </span>
        );
      })}
    </span>
  );
}

/** Les champs de réponse d'une question. */
function Reponse({
  q,
  saisie,
  surSaisie,
  fige,
  images,
  taille,
}: {
  q: QuestionQuiz;
  saisie: Saisie;
  surSaisie: (s: Saisie) => void;
  fige: boolean;
  images: ReadonlyMap<string, string>;
  taille: string;
}) {
  if (q.sorte === "reponse_courte") {
    return (
      <label className="flex flex-col gap-1 text-encre">
        <span>{t("quiz.ta_reponse")}</span>
        <input
          value={saisie.ecrite}
          onChange={(e) => surSaisie({ ...saisie, ecrite: e.target.value })}
          disabled={fige}
          className={`min-h-11 rounded border border-muet bg-surface px-2 ${taille}`}
        />
      </label>
    );
  }
  if (q.sorte === "appariement") {
    return (
      <div className="flex flex-col gap-2">
        {q.choix.map((c) => (
          <label key={c.repere} className={`flex flex-wrap items-center gap-2 text-encre ${taille}`}>
            <span>
              {c.repere}) <Enlignes elements={c.texte} images={images} />
            </span>
            <select
              value={saisie.paires[c.repere] ?? ""}
              onChange={(e) => surSaisie({ ...saisie, paires: { ...saisie.paires, [c.repere]: e.target.value } })}
              disabled={fige}
              className="min-h-11 rounded border border-muet bg-surface px-2"
            >
              <option value="">{t("quiz.choisir")}</option>
              {q.elements.map((e) => (
                <option key={e.repere} value={e.repere}>
                  {e.repere}) {texteBrut(e.texte)}
                </option>
              ))}
            </select>
          </label>
        ))}
      </div>
    );
  }
  const plusieurs = q.sorte === "choix_multiples";
  return (
    <div role="group" aria-label={plusieurs ? t("quiz.plusieurs") : t("quiz.une")} className="flex flex-col gap-2">
      {plusieurs ? <p className="text-sm text-encre">{t("quiz.plusieurs")}</p> : null}
      {q.choix.map((c) => {
        const choisi = saisie.lettres.includes(c.repere);
        return (
          <button
            key={c.repere}
            type="button"
            aria-pressed={choisi}
            disabled={fige}
            onClick={() =>
              surSaisie({
                ...saisie,
                lettres: plusieurs ? (choisi ? saisie.lettres.filter((l) => l !== c.repere) : [...saisie.lettres, c.repere]) : [c.repere],
              })
            }
            className={`min-h-11 rounded border-2 px-3 py-2 text-left ${taille} ${choisi ? "border-primaire bg-fond font-semibold" : "border-muet bg-surface"}`}
          >
            <span aria-hidden>{choisi ? "● " : "○ "}</span>
            {c.repere}) <Enlignes elements={c.texte} images={images} />
          </button>
        );
      })}
    </div>
  );
}

/** Une question corrigée : le signe et le mot, jamais la couleur seule. */
function Correction({ juste, q, images }: { juste: boolean | null; q: QuestionQuiz; images: ReadonlyMap<string, string> }) {
  return (
    <div role="status" className="flex flex-col gap-2 rounded border border-muet bg-fond p-3 text-encre">
      {juste === null ? null : <p className={`font-semibold ${juste ? "text-vert" : "text-rouge"}`}>{juste ? t("quiz.juste") : t("quiz.pas_tout_a_fait")}</p>}
      <div>
        <p className="font-semibold">{t("quiz.bonne_reponse")}</p>
        <BonneReponse q={q} images={images} />
      </div>
      {q.explication.length === 0 ? null : <RenduLecon lecon={{ blocs: q.explication }} images={images} />}
    </div>
  );
}

/** Passer un quiz sur un poste : rien n'est écrit, aucun nom demandé. En mode classe, le maître
 *  projette la question et montre la réponse ; flèches pour avancer. */
export function PasserQuiz({
  questions,
  images,
  classe = false,
  surFermer,
  titre,
}: {
  questions: readonly QuestionQuiz[];
  images: ReadonlyMap<string, string>;
  classe?: boolean;
  surFermer: () => void;
  titre?: string;
}) {
  const [rang, setRang] = useState(0);
  const [saisie, setSaisie] = useState<Saisie>(SAISIE_VIDE);
  const [resultats, setResultats] = useState<(boolean | null)[]>(() => questions.map(() => null));
  const [montre, setMontre] = useState(false);
  const [erreur, setErreur] = useState<unknown>(null);
  const [enCours, setEnCours] = useState(false);
  const fini = rang >= questions.length;
  const q = questions[rang];
  const corrigee = q !== undefined && (resultats[rang] ?? null) !== null;
  const taille = classe ? "text-2xl" : "text-base";

  const aller = (n: number) => {
    setRang(Math.max(0, Math.min(questions.length, n)));
    setSaisie(SAISIE_VIDE);
    setMontre(false);
  };
  useEffect(() => {
    if (!classe) return;
    const touche = (e: KeyboardEvent) => {
      if (e.key === "ArrowRight") setRang((r) => Math.min(questions.length - 1, r + 1));
      if (e.key === "ArrowLeft") setRang((r) => Math.max(0, r - 1));
      setMontre(false);
    };
    window.addEventListener("keydown", touche);
    return () => window.removeEventListener("keydown", touche);
  }, [classe, questions.length]);

  const verifier = async () => {
    if (q === undefined) return;
    setEnCours(true);
    setErreur(null);
    try {
      const juste = await appeler("bibliotheque_quiz_verifier", { question: q, reponse: reponseDe(q, saisie) });
      setResultats((r) => r.map((x, i) => (i === rang ? juste : x)));
    } catch (e) {
      setErreur(e);
    } finally {
      setEnCours(false);
    }
  };
  const recommencer = () => {
    setResultats(questions.map(() => null));
    aller(0);
  };

  const entete = (
    <div className="flex flex-wrap items-center justify-between gap-2">
      <p className="text-sm text-encre">
        {titre === undefined ? null : <span className="font-semibold">{titre} · </span>}
        {fini ? t("quiz.fin") : t("quiz.question_sur", { n: rang + 1, total: questions.length })}
      </p>
      <Bouton variante="secondaire" onClick={surFermer}>
        {t("quiz.fermer")}
      </Bouton>
    </div>
  );

  if (questions.length === 0) {
    return (
      <section aria-label={t("quiz.titre")} className="flex flex-col gap-3 rounded border border-muet bg-surface p-4">
        {entete}
        <p className="text-encre">{t("quiz.aucune_question")}</p>
      </section>
    );
  }

  if (fini || q === undefined) {
    const justes = resultats.filter((r) => r === true).length;
    return (
      <section aria-label={t("quiz.titre")} className="flex flex-col gap-3 rounded border border-muet bg-surface p-4">
        {entete}
        {classe ? null : <p className="text-3xl font-semibold text-marine">{t("quiz.score", { justes, total: questions.length })}</p>}
        <ol className="flex flex-col gap-1 text-encre">
          {questions.map((x, i) => {
            const r = resultats[i] ?? null;
            return (
              <li key={`${x.numero}-${i}`}>
                {t("quiz.question_n", { n: i + 1 })}
                {r === null ? "" : r ? ` : ${t("quiz.juste")}` : ` : ${t("quiz.pas_tout_a_fait")}`}
              </li>
            );
          })}
        </ol>
        <div>
          <Bouton onClick={recommencer}>{t("quiz.recommencer")}</Bouton>
        </div>
      </section>
    );
  }

  return (
    <section aria-label={t("quiz.titre")} className="flex flex-col gap-3 rounded border border-muet bg-surface p-4">
      {entete}
      <div className={taille}>
        <RenduLecon lecon={{ blocs: q.enonce }} images={images} />
      </div>
      {classe ? null : <Reponse q={q} saisie={saisie} surSaisie={setSaisie} fige={corrigee} images={images} taille={taille} />}
      {classe ? (
        <>
          {q.sorte === "reponse_courte" || q.sorte === "appariement" ? null : (
            <ul className={`flex flex-col gap-2 text-encre ${taille}`}>
              {q.choix.map((c) => (
                <li key={c.repere}>
                  {c.repere}) <Enlignes elements={c.texte} images={images} />
                </li>
              ))}
            </ul>
          )}
          {q.sorte === "appariement" ? (
            <div className={`grid grid-cols-2 gap-2 text-encre ${taille}`}>
              <ul>
                {q.choix.map((c) => (
                  <li key={c.repere}>
                    {c.repere}) <Enlignes elements={c.texte} images={images} />
                  </li>
                ))}
              </ul>
              <ul>
                {q.elements.map((c) => (
                  <li key={c.repere}>
                    {c.repere}) <Enlignes elements={c.texte} images={images} />
                  </li>
                ))}
              </ul>
            </div>
          ) : null}
          {montre ? <Correction juste={null} q={q} images={images} /> : null}
          <div className="flex flex-wrap gap-2">
            <Bouton variante="secondaire" onClick={() => aller(rang - 1)} disabled={rang === 0}>
              {t("quiz.precedente")}
            </Bouton>
            <Bouton onClick={() => setMontre(true)} disabled={montre}>
              {t("quiz.montrer_reponse")}
            </Bouton>
            <Bouton variante="secondaire" onClick={() => aller(rang + 1)} disabled={rang >= questions.length - 1}>
              {t("quiz.suivante")}
            </Bouton>
          </div>
        </>
      ) : (
        <>
          {corrigee ? <Correction juste={resultats[rang] ?? null} q={q} images={images} /> : null}
          <MessageErreur erreur={erreur} />
          <div className="flex flex-wrap gap-2">
            {corrigee ? (
              <Bouton onClick={() => aller(rang + 1)}>{rang + 1 >= questions.length ? t("quiz.voir_score") : t("quiz.suivante")}</Bouton>
            ) : (
              <Bouton onClick={() => void verifier()} disabled={!repondu(q, saisie) || enCours}>
                {t("quiz.verifier")}
              </Bouton>
            )}
          </div>
        </>
      )}
    </section>
  );
}

/** « Réviser le chapitre » sur un poste : dix questions tirées des quiz du chapitre, dans une
 *  fenêtre au-dessus du catalogue. Rien n'est écrit. */
export function RevisionPoste({ chapitreId, surFermer }: { chapitreId: string; surFermer: () => void }) {
  const revision = useQuery({
    queryKey: ["bibliotheque", "revision", chapitreId],
    queryFn: () => appeler("bibliotheque_revision", { chapitre_id: chapitreId }),
    gcTime: 0,
  });
  const images = useMemo(() => imagesParCle(revision.data?.images ?? []), [revision.data?.images]);
  return (
    <div role="dialog" aria-modal="true" aria-label={t("quiz.reviser")} className="fixed inset-0 z-40 overflow-auto bg-encre/40 p-4">
      <div className="mx-auto max-w-3xl">
        <MessageErreur erreur={revision.error} />
        {revision.data === undefined ? null : (
          <PasserQuiz questions={revision.data.questions} images={images} titre={revision.data.titre} surFermer={surFermer} />
        )}
      </div>
    </div>
  );
}
