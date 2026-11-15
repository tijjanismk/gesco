import "katex/dist/katex.min.css";

import katex from "katex";
import { useMemo, type ReactNode } from "react";

import type { Alignement, Bloc, Enligne, ImageLecon, Lecon, VideoLecon } from "../../types/gesco";
import { ContexteVideos, VideoDansLecon } from "./Video";

/** Images de la leçon par leur clé (nom du fichier du paquet ou empreinte d'une image de
 *  l'école), en adresse `data:` : rien ne se charge par le réseau. */
export function imagesParCle(images: readonly ImageLecon[]): ReadonlyMap<string, string> {
  return new Map(images.map((i) => [i.cle, `data:${i.type_mime};base64,${i.donnees}`]));
}

/** Une formule affichée par KaTeX. Une formule que KaTeX ne sait pas lire s'affiche telle qu'elle
 *  est écrite : elle ne bloque jamais la leçon. */
function Formule({ latex, seule }: { latex: string; seule: boolean }) {
  const html = useMemo(() => {
    try {
      return katex.renderToString(latex, { displayMode: seule, throwOnError: true, trust: false, strict: "ignore" });
    } catch {
      return null;
    }
  }, [latex, seule]);
  if (html === null) {
    const brut = seule ? `$$${latex}$$` : `$${latex}$`;
    return seule ? <pre className="my-2 whitespace-pre-wrap font-mono">{brut}</pre> : <code className="font-mono">{brut}</code>;
  }
  // KaTeX produit un balisage sans script (trust désactivé) à partir du seul texte LaTeX.
  return seule ? (
    <div className="my-3 overflow-x-auto" dangerouslySetInnerHTML={{ __html: html }} />
  ) : (
    <span dangerouslySetInnerHTML={{ __html: html }} />
  );
}

function Enlignes({ elements, images }: { elements: readonly Enligne[]; images: ReadonlyMap<string, string> }) {
  return (
    <>
      {elements.map((e, i) => (
        <EnligneRendu key={i} element={e} images={images} />
      ))}
    </>
  );
}

function EnligneRendu({ element, images }: { element: Enligne; images: ReadonlyMap<string, string> }): ReactNode {
  switch (element.type) {
    case "texte":
      return element.texte;
    case "code":
      return <code className="font-mono">{element.texte}</code>;
    case "gras":
      return (
        <strong>
          <Enlignes elements={element.contenu} images={images} />
        </strong>
      );
    case "italique":
      return (
        <em>
          <Enlignes elements={element.contenu} images={images} />
        </em>
      );
    case "formule":
      return <Formule latex={element.latex} seule={element.seule} />;
    case "image": {
      const cle = element.source.type === "paquet" ? element.source.nom : element.source.empreinte;
      const adresse = images.get(cle);
      // Une image absente ne laisse aucun trou signalé.
      if (adresse === undefined) return null;
      return <img src={adresse} alt={element.description} className="my-2 inline-block max-w-full bg-surface" />;
    }
    case "video":
      return <VideoDansLecon empreinte={element.empreinte} titre={element.titre} />;
    case "saut":
      return <br />;
  }
}

const ALIGNEMENT: Record<Alignement, string> = {
  aucun: "text-left",
  gauche: "text-left",
  centre: "text-center",
  droite: "text-right",
};

function Blocs({
  blocs,
  images,
  sansNotes,
}: {
  blocs: readonly Bloc[];
  images: ReadonlyMap<string, string>;
  sansNotes: boolean;
}) {
  return (
    <>
      {blocs.map((b, i) => (
        <BlocRendu key={i} bloc={b} images={images} sansNotes={sansNotes} />
      ))}
    </>
  );
}

function BlocRendu({
  bloc,
  images,
  sansNotes,
}: {
  bloc: Bloc;
  images: ReadonlyMap<string, string>;
  sansNotes: boolean;
}): ReactNode {
  switch (bloc.type) {
    case "titre": {
      const contenu = <Enlignes elements={bloc.contenu} images={images} />;
      if (bloc.niveau === 1) return <h2 className="mt-4 mb-2 text-2xl font-semibold text-marine">{contenu}</h2>;
      if (bloc.niveau === 2) return <h3 className="mt-4 mb-2 text-xl font-semibold text-marine">{contenu}</h3>;
      return <h4 className="mt-3 mb-1 text-lg font-semibold text-encre">{contenu}</h4>;
    }
    case "paragraphe":
      return (
        <p className="my-2 leading-relaxed">
          <Enlignes elements={bloc.contenu} images={images} />
        </p>
      );
    case "formule":
      return <Formule latex={bloc.latex} seule />;
    case "liste": {
      const elements = bloc.elements.map((e, i) => (
        <li key={i} className="my-1">
          <Blocs blocs={e} images={images} sansNotes={sansNotes} />
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
                  <th key={i} className={`border border-muet bg-fond px-2 py-1 font-semibold ${ALIGNEMENT[bloc.alignements[i] ?? "aucun"]}`}>
                    <Enlignes elements={c} images={images} />
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {bloc.lignes.map((ligne, j) => (
                <tr key={j}>
                  {ligne.map((c, i) => (
                    <td key={i} className={`border border-muet px-2 py-1 ${ALIGNEMENT[bloc.alignements[i] ?? "aucun"]}`}>
                      <Enlignes elements={c} images={images} />
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
          <Blocs blocs={bloc.blocs} images={images} sansNotes={sansNotes} />
        </blockquote>
      );
    case "note_maitre":
      // Note pour le maître : le noyau ne l'envoie qu'au personnel qui écrit ; le mode classe,
      // tourné vers les élèves, ne la montre pas.
      if (sansNotes) return null;
      return (
        <aside className="my-3 rounded border border-ocre bg-fond px-4 py-2 text-encre">
          <Blocs blocs={bloc.blocs} images={images} sansNotes={sansNotes} />
        </aside>
      );
    case "texte_brut":
      return <pre className="my-2 overflow-x-auto whitespace-pre-wrap rounded bg-fond p-2 font-mono">{bloc.texte}</pre>;
    case "filet":
      return <hr className="my-4 border-muet" />;
  }
}

/** Une leçon, un sujet ou un corrigé, depuis l'arbre de blocs que rend le noyau. */
export function RenduLecon({
  lecon,
  images,
  sansNotes = false,
  videos,
}: {
  lecon: Lecon;
  images: ReadonlyMap<string, string>;
  sansNotes?: boolean;
  /** Vidéos de la leçon et leur présence ; absent dans l'aperçu. */
  videos?: readonly VideoLecon[];
}) {
  const parEmpreinte = useMemo(() => (videos === undefined ? null : new Map(videos.map((v) => [v.empreinte, v]))), [videos]);
  return (
    <ContexteVideos.Provider value={parEmpreinte}>
      <div className="text-encre">
        <Blocs blocs={lecon.blocs} images={images} sansNotes={sansNotes} />
      </div>
    </ContexteVideos.Provider>
  );
}
