import "katex/dist/katex.min.css";
// Andika arrive avec la leçon, hors du premier chargement (voir `police.css`).
import "./police.css";

import katex from "katex";
import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from "react";

import { adresseFichier, adressePdf, envoyer } from "./api";
import { Attente, Chargement, Retour } from "./communs";
import { useLecture } from "./lecture";
import { replier } from "./replier";
import { useSuivi } from "./suivi";
import { nature, T, voirLie } from "./textes";
import type { Accueil, Bloc, ContenuEleve, Enligne } from "./types";

/** Ce que la leçon appelle : images par clé, sons et vidéos présents par empreinte. */
type Medias = {
  images: ReadonlyMap<string, string>;
  sons: ReadonlyMap<string, string>;
  videos: ReadonlyMap<string, string>;
};

const ContexteMedias = createContext<Medias>({ images: new Map(), sons: new Map(), videos: new Map() });

/** Tailles du texte de la leçon, en rem ; la deuxième est celle d'arrivée. */
const TAILLES = [1, 1.125, 1.25, 1.4, 1.6] as const;
const CLE_TAILLE = "gesco-eleve-taille";

function tailleRetenue(): number {
  try {
    const n = Number(window.localStorage.getItem(CLE_TAILLE));
    return Number.isInteger(n) && n >= 0 && n < TAILLES.length ? n : 1;
  } catch {
    return 1;
  }
}

/** Une formule par KaTeX ; une formule illisible s'affiche telle qu'elle est écrite. */
function Formule({ latex, seule }: { latex: string; seule: boolean }) {
  const html = useMemo(() => {
    try {
      return katex.renderToString(latex, { displayMode: seule, throwOnError: true, trust: false, strict: "ignore" });
    } catch {
      return null;
    }
  }, [latex, seule]);
  if (html === null) return seule ? <pre className="my-2 whitespace-pre-wrap">{`$$${latex}$$`}</pre> : <code>{`$${latex}$`}</code>;
  // KaTeX produit un balisage sans script (trust désactivé) à partir du seul texte LaTeX.
  return seule ? (
    <div className="my-3" dangerouslySetInnerHTML={{ __html: html }} />
  ) : (
    <span dangerouslySetInnerHTML={{ __html: html }} />
  );
}

function Enlignes({ elements }: { elements: readonly Enligne[] }) {
  return (
    <>
      {elements.map((e, i) => (
        <EnligneRendu key={i} element={e} />
      ))}
    </>
  );
}

function EnligneRendu({ element }: { element: Enligne }): ReactNode {
  const medias = useContext(ContexteMedias);
  switch (element.type) {
    case "texte":
      return element.texte;
    case "code":
      return <code>{element.texte}</code>;
    case "gras":
      return (
        <strong>
          <Enlignes elements={element.contenu} />
        </strong>
      );
    case "italique":
      return (
        <em>
          <Enlignes elements={element.contenu} />
        </em>
      );
    case "formule":
      return <Formule latex={element.latex} seule={element.seule} />;
    case "image": {
      const cle = element.source.type === "paquet" ? element.source.nom : element.source.empreinte;
      const adresse = medias.images.get(cle);
      if (adresse === undefined) return null;
      return <img src={adresse} alt={element.description} loading="lazy" className="my-2 inline-block bg-surface" />;
    }
    case "video": {
      // Une vidéo absente, ou coupée par la direction, n'est pas montrée.
      const adresse = medias.videos.get(element.empreinte);
      if (adresse === undefined) return null;
      return (
        <figure className="my-3">
          <video controls playsInline preload="metadata" src={adresse} className="w-full rounded bg-marine" />
          {element.titre !== "" && <figcaption className="mt-1 text-muet">{element.titre}</figcaption>}
        </figure>
      );
    }
    case "son": {
      const adresse = medias.sons.get(element.empreinte);
      if (adresse === undefined) return null;
      return (
        <figure className="my-3">
          {element.titre !== "" && <figcaption className="mb-1 font-bold">{element.titre}</figcaption>}
          <audio controls preload="none" src={adresse} className="w-full" />
        </figure>
      );
    }
    case "saut":
      return <br />;
  }
}

function Blocs({ blocs }: { blocs: readonly Bloc[] }) {
  return (
    <>
      {blocs.map((b, i) => (
        <BlocRendu key={i} bloc={b} />
      ))}
    </>
  );
}

function BlocRendu({ bloc }: { bloc: Bloc }): ReactNode {
  switch (bloc.type) {
    case "titre": {
      const contenu = <Enlignes elements={bloc.contenu} />;
      if (bloc.niveau === 1) return <h2 className="mt-5 mb-2 text-[1.5em] font-bold">{contenu}</h2>;
      if (bloc.niveau === 2) return <h3 className="mt-5 mb-2 text-[1.3em] font-bold">{contenu}</h3>;
      return <h4 className="mt-4 mb-1 text-[1.1em] font-bold">{contenu}</h4>;
    }
    case "paragraphe":
      return (
        <p className="my-2">
          <Enlignes elements={bloc.contenu} />
        </p>
      );
    case "formule":
      return <Formule latex={bloc.latex} seule />;
    case "liste": {
      const elements = bloc.elements.map((e, i) => (
        <li key={i} className="my-1">
          <Blocs blocs={e} />
        </li>
      ));
      return bloc.numerotee ? (
        <ol start={bloc.debut ?? 1} className="my-2 list-decimal pl-6">
          {elements}
        </ol>
      ) : (
        <ul className="my-2 list-disc pl-6">{elements}</ul>
      );
    }
    case "tableau":
      return (
        <div className="my-3 overflow-x-auto">
          <table className="border-collapse text-left">
            <thead>
              <tr>
                {bloc.entete.map((c, i) => (
                  <th key={i} className="border border-muet bg-fond px-2 py-1 font-bold">
                    <Enlignes elements={c} />
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {bloc.lignes.map((ligne, j) => (
                <tr key={j}>
                  {ligne.map((c, i) => (
                    <td key={i} className="border border-muet px-2 py-1">
                      <Enlignes elements={c} />
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      );
    case "citation":
      return (
        <blockquote className="my-3 border-l-4 border-primaire pl-4">
          <Blocs blocs={bloc.blocs} />
        </blockquote>
      );
    case "note_maitre":
      // Le poste principal ne l'envoie jamais aux élèves.
      return null;
    case "texte_brut":
      return <pre className="my-2 overflow-x-auto rounded bg-fond p-2 whitespace-pre-wrap">{bloc.texte}</pre>;
    case "filet":
      return <hr className="my-4 border-muet" />;
  }
}

/** La leçon, avec ses corrigés et les réponses du quiz repliés. */
function Texte({ blocs }: { blocs: readonly Bloc[] }) {
  const sections = useMemo(() => replier(blocs), [blocs]);
  return (
    <>
      {sections.map((s, i) =>
        s.repliee === null ? (
          <Blocs key={i} blocs={s.blocs} />
        ) : (
          <details key={i} className="my-4 rounded border-2 border-primaire bg-surface px-3">
            <summary className="cible flex cursor-pointer items-center font-bold text-primaire">
              {s.repliee === "corriges" ? T.voirCorriges : T.voirReponses}
            </summary>
            <BlocRendu bloc={s.titre} />
            <Blocs blocs={s.blocs} />
          </details>
        ),
      )}
    </>
  );
}

/** Un contenu ouvert : la leçon, la taille du texte, « J'ai fini », le PDF. */
export function Lecon({ id, accueil }: { id: string; accueil: Accueil }) {
  const contenu = useLecture<ContenuEleve>(`contenu/${encodeURIComponent(id)}`);
  if (contenu.etat === "occupe") return <Attente />;
  if (contenu.etat === "chargement") return <Chargement />;
  if (contenu.etat === "introuvable") {
    return (
      <>
        <Retour vers="#/" texte={T.mesMatieres} />
        <p className="mt-4 text-muet">{T.introuvable}</p>
      </>
    );
  }
  return <LeconOuverte key={contenu.donnee.id} contenu={contenu.donnee} pdfPermis={accueil.pdf} />;
}

function LeconOuverte({ contenu, pdfPermis }: { contenu: ContenuEleve; pdfPermis: boolean }) {
  const suivi = useSuivi();
  const [taille, setTaille] = useState(tailleRetenue);
  const [fini, setFini] = useState(false);
  const dejaFinie = suivi.avancement?.finies.includes(contenu.id) === true;
  const rafraichir = suivi.rafraichir;

  // L'ouverture est notée pour un élève suivi ; une panne ne gêne jamais la lecture.
  useEffect(() => {
    if (!suivi.actif) return;
    envoyer("avancement", { contenu_id: contenu.id, version: contenu.version, geste: "ouvert" })
      .then(rafraichir)
      .catch(() => null);
  }, [suivi.actif, contenu.id, contenu.version, rafraichir]);

  const medias = useMemo<Medias>(
    () => ({
      images: new Map(contenu.images.map((i) => [i.cle, adresseFichier(i.empreinte)])),
      sons: new Map(contenu.sons.map((s) => [s.empreinte, adresseFichier(s.empreinte)])),
      videos: new Map(contenu.videos.map((v) => [v.empreinte, adresseFichier(v.empreinte)])),
    }),
    [contenu],
  );

  const changerTaille = (pas: number) => {
    const suivante = Math.min(TAILLES.length - 1, Math.max(0, taille + pas));
    setTaille(suivante);
    try {
      window.localStorage.setItem(CLE_TAILLE, String(suivante));
    } catch {
      // Le réglage n'est pas gardé : rien de grave.
    }
  };

  const finir = () => {
    setFini(true);
    envoyer("avancement", { contenu_id: contenu.id, version: contenu.version, geste: "fini" })
      .then(rafraichir)
      .catch(() => null);
  };

  return (
    <article>
      <div className="flex items-center justify-between gap-2">
        <Retour vers={`#/matiere/${encodeURIComponent(contenu.matiere_cle)}`} texte={contenu.matiere} />
        <div className="flex gap-2">
          <button
            type="button"
            onClick={() => changerTaille(-1)}
            aria-label={T.tailleMoins}
            className="cible rounded border-2 border-muet bg-surface px-2 font-bold"
          >
            A−
          </button>
          <button
            type="button"
            onClick={() => changerTaille(1)}
            aria-label={T.taillePlus}
            className="cible rounded border-2 border-muet bg-surface px-2 text-lg font-bold"
          >
            A+
          </button>
        </div>
      </div>
      <p className="mt-2 text-muet">
        {nature(contenu)} · {contenu.chapitre}
      </p>
      <h1 className="mb-2 text-2xl font-bold">{contenu.titre}</h1>

      <ContexteMedias.Provider value={medias}>
        <div className="lecon" style={{ fontSize: `${TAILLES[taille] ?? 1.125}rem` }}>
          <Texte blocs={contenu.lecon.blocs} />
        </div>
      </ContexteMedias.Provider>

      {contenu.fichiers.length > 0 && (
        <ul className="my-4 flex flex-col gap-2">
          {contenu.fichiers.map((f) => (
            <li key={f.empreinte}>
              <a
                href={adresseFichier(f.empreinte)}
                target="_blank"
                rel="noopener"
                className="cible flex items-center rounded bg-surface px-4 py-2 font-bold text-primaire underline shadow"
              >
                {T.ouvrir} {f.nom}
              </a>
            </li>
          ))}
        </ul>
      )}

      {contenu.lie !== null && (
        <p className="my-4">
          <a href={`#/lecon/${encodeURIComponent(contenu.lie.id)}`} className="cible inline-flex items-center font-bold text-primaire underline">
            {voirLie(contenu.lie.nature)} : {contenu.lie.titre}
          </a>
        </p>
      )}

      <div className="mt-8 flex flex-col gap-3">
        {suivi.actif &&
          (fini || dejaFinie ? (
            <p role="status" className="cible flex items-center justify-center rounded bg-surface px-4 font-bold text-vert">
              <span aria-hidden="true">✓&nbsp;</span>
              {T.cestNote}
            </p>
          ) : (
            <button type="button" onClick={finir} className="cible rounded bg-vert px-4 text-lg font-bold text-surface">
              {T.jaiFini}
            </button>
          ))}
        {pdfPermis && contenu.pdf && (
          <a
            href={adressePdf(contenu.id)}
            download
            className="cible flex items-center justify-center rounded border-2 border-primaire px-4 text-lg font-bold text-primaire"
          >
            {T.enregistrerPdf}
          </a>
        )}
      </div>
    </article>
  );
}
