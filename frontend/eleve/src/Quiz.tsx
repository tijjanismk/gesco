import { useMemo, useState } from "react";

import { adresseFichier, envoyer } from "./api";
import { Attente, Chargement, Retour } from "./communs";
import { corriger, reponseDe, SAISIE_VIDE, type Saisie } from "./corriger";
import { Blocs, ContexteMedias, Enlignes, type Medias } from "./Lecon";
import { useLecture } from "./lecture";
import { useSuivi } from "./suivi";
import { T } from "./textes";
import type { ChoixQuestion, Enligne, QuestionEleve, QuestionQuiz, QuizEleve } from "./types";

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

function BonneReponse({ q }: { q: QuestionQuiz }) {
  const trouver = (repere: string, liste: readonly ChoixQuestion[]) => liste.find((c) => c.repere === repere);
  if (q.sorte === "reponse_courte") return <p>{q.bonnes.map((b) => `« ${b} »`).join(` ${T.ou} `)}</p>;
  if (q.sorte === "appariement") {
    return (
      <ul className="list-inside list-disc">
        {q.bonnes.map((b) => {
          const [l = "", n = ""] = b.split("-");
          const a = trouver(l, q.choix);
          const e = trouver(n, q.elements);
          return (
            <li key={b}>
              {a === undefined ? l : <Enlignes elements={a.texte} />} → {e === undefined ? n : <Enlignes elements={e.texte} />}
            </li>
          );
        })}
      </ul>
    );
  }
  return (
    <ul>
      {q.bonnes.map((b) => {
        const c = trouver(b, q.choix);
        return (
          <li key={b}>
            {b}) {c === undefined ? null : <Enlignes elements={c.texte} />}
          </li>
        );
      })}
    </ul>
  );
}

function Champs({ q, saisie, surSaisie, fige }: { q: QuestionQuiz; saisie: Saisie; surSaisie: (s: Saisie) => void; fige: boolean }) {
  if (q.sorte === "reponse_courte") {
    return (
      <label className="flex flex-col gap-1">
        <span className="font-bold">{T.taReponse}</span>
        <input
          value={saisie.ecrite}
          onChange={(e) => surSaisie({ ...saisie, ecrite: e.target.value })}
          disabled={fige}
          className="cible rounded border-2 border-muet bg-surface px-3 text-lg"
        />
      </label>
    );
  }
  if (q.sorte === "appariement") {
    return (
      <div className="flex flex-col gap-3">
        {q.choix.map((c) => (
          <label key={c.repere} className="flex flex-col gap-1">
            <span>
              {c.repere}) <Enlignes elements={c.texte} />
            </span>
            <select
              value={saisie.paires[c.repere] ?? ""}
              onChange={(e) => surSaisie({ ...saisie, paires: { ...saisie.paires, [c.repere]: e.target.value } })}
              disabled={fige}
              className="cible rounded border-2 border-muet bg-surface px-2 text-lg"
            >
              <option value="">{T.choisir}</option>
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
    <div className="flex flex-col gap-2">
      {plusieurs && <p className="text-muet">{T.plusieurs}</p>}
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
            className={`cible rounded border-2 px-4 py-2 text-left text-lg ${choisi ? "border-primaire bg-surface font-bold" : "border-muet bg-surface"}`}
          >
            <span aria-hidden="true">{choisi ? "● " : "○ "}</span>
            {c.repere}) <Enlignes elements={c.texte} />
          </button>
        );
      })}
    </div>
  );
}

/** Passer un quiz : une question par écran, la correction et l'explication tout de suite, le
 *  score à la fin, autant d'essais qu'on veut. Aucun chronomètre, aucun classement. Le quiz fini
 *  part au poste principal, qui l'écrit seulement pour un élève suivi. */
function Deroulement({ quiz, retour }: { quiz: QuizEleve; retour: string }) {
  const suivi = useSuivi();
  const [rang, setRang] = useState(0);
  const [saisie, setSaisie] = useState<Saisie>(SAISIE_VIDE);
  const [reponses, setReponses] = useState<{ q: QuestionEleve; reponse: string; juste: boolean }[]>([]);
  const [commencee, setCommencee] = useState(() => Date.now());
  const medias = useMemo<Medias>(
    () => ({ images: new Map(quiz.images.map((i) => [i.cle, adresseFichier(i.empreinte)])), sons: new Map(), videos: new Map() }),
    [quiz.images],
  );
  const total = quiz.questions.length;
  const courante = quiz.questions[rang];
  const corrigee = reponses.length > rang ? reponses[rang] : undefined;

  const verifier = () => {
    if (courante === undefined) return;
    const reponse = reponseDe(courante.question, saisie);
    const suite = [...reponses, { q: courante, reponse, juste: corriger(courante.question, reponse) }];
    setReponses(suite);
    if (suite.length === total) {
      // Rien ne bloque l'élève : une panne ou un élève sans code, et rien ne s'écrit.
      envoyer("tentative", {
        contenu_id: quiz.contenu_id,
        chapitre_id: quiz.chapitre_id,
        commencee_le: commencee,
        reponses: suite.map((r) => ({ contenu_id: r.q.contenu_id, version: r.q.version, numero: r.q.question.numero, reponse: r.reponse })),
      })
        .then(() => suivi.rafraichir())
        .catch(() => null);
    }
  };
  const suivante = () => {
    setRang((r) => r + 1);
    setSaisie(SAISIE_VIDE);
    window.scrollTo(0, 0);
  };
  const recommencer = () => {
    setRang(0);
    setSaisie(SAISIE_VIDE);
    setReponses([]);
    setCommencee(Date.now());
  };

  if (total === 0) {
    return (
      <>
        <Retour vers={retour} />
        <p className="mt-4 text-muet">{T.pasDeQuestion}</p>
      </>
    );
  }

  if (courante === undefined) {
    const justes = reponses.filter((r) => r.juste).length;
    return (
      <article>
        <Retour vers={retour} />
        <h1 className="mt-2 text-2xl font-bold">{quiz.titre}</h1>
        <p role="status" className="my-4 text-4xl font-bold text-marine">
          {T.surTotal(justes, total)}
        </p>
        <ol className="mb-6 flex flex-col gap-1 text-lg">
          {reponses.map((r, i) => (
            <li key={`${r.q.contenu_id}-${r.q.question.numero}-${i}`}>
              <span aria-hidden="true">{r.juste ? "✓ " : "✗ "}</span>
              {T.questionN(i + 1)} : {r.juste ? T.juste : T.pasToutAFait}
            </li>
          ))}
        </ol>
        <div className="flex flex-col gap-3">
          <button type="button" onClick={recommencer} className="cible rounded bg-primaire px-4 text-lg font-bold text-surface">
            {T.recommencer}
          </button>
          <a href={retour} className="cible flex items-center justify-center rounded border-2 border-primaire px-4 text-lg font-bold text-primaire">
            {T.retour}
          </a>
        </div>
      </article>
    );
  }

  const q = courante.question;
  const pret = reponseDe(q, saisie).trim() !== "";
  return (
    <article>
      <Retour vers={retour} />
      <p className="mt-2 text-muet">
        {quiz.titre} · {T.questionSur(rang + 1, total)}
      </p>
      <ContexteMedias.Provider value={medias}>
        <div className="lecon my-3 text-xl">
          <Blocs blocs={q.enonce} />
        </div>
        <Champs q={q} saisie={saisie} surSaisie={setSaisie} fige={corrigee !== undefined} />
        {corrigee === undefined ? (
          <button type="button" onClick={verifier} disabled={!pret} className="cible mt-4 w-full rounded bg-primaire px-4 text-lg font-bold text-surface disabled:opacity-50">
            {T.verifier}
          </button>
        ) : (
          <>
            <div role="status" className="mt-4 flex flex-col gap-2 rounded bg-surface p-4 shadow">
              <p className={`text-xl font-bold ${corrigee.juste ? "text-vert" : "text-rouge"}`}>
                <span aria-hidden="true">{corrigee.juste ? "✓ " : "✗ "}</span>
                {corrigee.juste ? T.juste : T.pasToutAFait}
              </p>
              <p className="font-bold">{T.bonneReponse}</p>
              <BonneReponse q={q} />
              {q.explication.length > 0 && (
                <div className="lecon">
                  <Blocs blocs={q.explication} />
                </div>
              )}
            </div>
            <button type="button" onClick={suivante} className="cible mt-4 w-full rounded bg-primaire px-4 text-lg font-bold text-surface">
              {rang + 1 >= total ? T.voirScore : T.questionSuivante}
            </button>
          </>
        )}
      </ContexteMedias.Provider>
    </article>
  );
}

/** Le quiz d'une leçon (`quiz/…`) ou la révision d'un chapitre (`revision/…`). */
export function Quiz({ chemin, retour }: { chemin: string; retour: string }) {
  const quiz = useLecture<QuizEleve>(chemin);
  if (quiz.etat === "occupe") return <Attente />;
  if (quiz.etat === "chargement") return <Chargement />;
  if (quiz.etat === "introuvable") {
    return (
      <>
        <Retour vers={retour} />
        <p className="mt-4 text-muet">{T.pasDeQuestion}</p>
      </>
    );
  }
  return <Deroulement key={chemin} quiz={quiz.donnee} retour={retour} />;
}
