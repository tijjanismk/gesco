import type { Bloc, Enligne } from "./types";

/** Une suite de blocs de la leçon : montrée telle quelle, ou repliée sous un bouton. */
export type Section = { repliee: null; blocs: Bloc[] } | { repliee: "corriges" | "reponses"; titre: Bloc; blocs: Bloc[] };

function texteDe(elements: readonly Enligne[]): string {
  return elements
    .map((e) => {
      switch (e.type) {
        case "texte":
        case "code":
          return e.texte;
        case "gras":
        case "italique":
          return texteDe(e.contenu);
        default:
          return "";
      }
    })
    .join("");
}

/** Titre ramené à sa forme simple : sans numéro en tête, sans accents, en minuscules (comme le
 *  noyau pour l'impression « sans les corrigés »). */
export function titreSimple(titre: string): string {
  return titre
    .trim()
    .replace(/^[\d.)]+/, "")
    .trim()
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase();
}

/** Découpe la leçon : la section « Corrigés » et, sous le quiz, la sous-section « Réponses »
 *  sont repliées jusqu'au titre suivant de même niveau ou plus haut. Une leçon sans ces titres
 *  reste d'un seul tenant. */
export function replier(blocs: readonly Bloc[]): Section[] {
  const sections: Section[] = [];
  let courante: Section = { repliee: null, blocs: [] };
  let niveauReplie: number | null = null;
  let niveauQuiz: number | null = null;
  const fermer = () => {
    if (courante.blocs.length > 0 || courante.repliee !== null) sections.push(courante);
  };
  for (const bloc of blocs) {
    if (bloc.type === "titre") {
      if (niveauQuiz !== null && bloc.niveau <= niveauQuiz) niveauQuiz = null;
      if (niveauReplie !== null && bloc.niveau <= niveauReplie) {
        niveauReplie = null;
        fermer();
        courante = { repliee: null, blocs: [] };
      }
      if (niveauReplie === null) {
        const titre = titreSimple(texteDe(bloc.contenu));
        const corriges = titre.startsWith("corrige");
        const reponses = niveauQuiz !== null && titre === "reponses";
        if (corriges || reponses) {
          fermer();
          niveauReplie = bloc.niveau;
          courante = { repliee: corriges ? "corriges" : "reponses", titre: bloc, blocs: [] };
          continue;
        }
        if (titre.startsWith("quiz")) niveauQuiz = bloc.niveau;
      }
    }
    courante.blocs.push(bloc);
  }
  fermer();
  return sections;
}
