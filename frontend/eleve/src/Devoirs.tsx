import { Attente, Chargement, Retour } from "./communs";
import { useLecture } from "./lecture";
import { aRendreLe, etatDevoir, T } from "./textes";
import type { EtatPourEleve, MesDevoirs } from "./types";

/** Couleur du mot d'état ; l'état se dit toujours en mots, la couleur n'est qu'un appui. */
function couleur(etat: EtatPourEleve): string {
  switch (etat) {
    case "a_rendre":
    case "papier":
      return "text-primaire";
    case "en_retard":
      return "text-rouge";
    case "rendu":
    case "corrige":
      return "text-vert";
    case "clos":
      return "text-muet";
  }
}

/** Mes devoirs : les devoirs donnés à la classe de l'élève, par date limite. */
export function Devoirs() {
  const lu = useLecture<MesDevoirs>("devoirs");
  if (lu.etat === "occupe") return <Attente />;
  if (lu.etat === "chargement") return <Chargement />;
  if (lu.etat === "introuvable") {
    return (
      <>
        <Retour vers="#/" texte={T.mesMatieres} />
        <p className="mt-4 text-muet">{T.aucunDevoir}</p>
      </>
    );
  }
  const { avec_code, devoirs } = lu.donnee;
  return (
    <>
      <Retour vers="#/" texte={T.mesMatieres} />
      <h1 className="mt-2 mb-4 text-2xl font-bold">{T.mesDevoirs}</h1>
      {!avec_code ? (
        <p className="text-lg">{T.entrerPourRendre}</p>
      ) : devoirs.length === 0 ? (
        <p className="text-muet">{T.aucunDevoir}</p>
      ) : (
        <ul className="flex flex-col gap-3">
          {devoirs.map((d) => (
            <li key={d.id}>
              <a href={`#/devoir/${encodeURIComponent(d.id)}`} className="cible block rounded bg-surface px-4 py-3 shadow">
                <span className="block text-sm text-muet">{d.matiere}</span>
                <span className="block text-lg font-bold">{d.titre}</span>
                <span className="block text-muet">{aRendreLe(d.date_limite)}</span>
                <span className={`block font-bold ${couleur(d.etat)}`}>{etatDevoir(d.etat, d.rendu_le, d.apres_limite)}</span>
              </a>
            </li>
          ))}
        </ul>
      )}
    </>
  );
}
