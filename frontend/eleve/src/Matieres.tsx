import { Attente, Chargement } from "./communs";
import { useLecture } from "./lecture";
import { useSuivi } from "./suivi";
import { finies, lecons, T } from "./textes";
import type { CatalogueEleve } from "./types";

/** Mes matières : une carte par matière ; la dernière leçon ouverte en tête. */
export function Matieres() {
  const catalogue = useLecture<CatalogueEleve>("catalogue");
  const suivi = useSuivi();
  if (catalogue.etat === "occupe") return <Attente />;
  if (catalogue.etat !== "pret") return <Chargement />;
  const { matieres } = catalogue.donnee;
  const derniere = suivi.avancement?.recentes[0];
  const finiesPar = new Map(suivi.avancement?.matieres.map((m) => [m.cle, m.finies]) ?? []);

  return (
    <>
      {derniere !== undefined && (
        <a href={`#/lecon/${encodeURIComponent(derniere.id)}`} className="cible mb-4 block rounded bg-primaire px-4 py-3 text-surface">
          <span className="block text-sm font-bold">{T.reprendre}</span>
          <span className="block text-lg">{derniere.titre}</span>
        </a>
      )}
      <a href="#/devoirs" className="cible mb-4 flex items-center justify-between rounded border-2 border-primaire bg-surface px-4 text-lg font-bold text-primaire">
        {T.mesDevoirs}
        <span aria-hidden="true">→</span>
      </a>
      <h1 className="mb-3 text-2xl font-bold">{T.mesMatieres}</h1>
      {matieres.length === 0 ? (
        <p className="text-muet">{T.aucuneLecon}</p>
      ) : (
        <ul className="grid grid-cols-1 gap-3 sm:grid-cols-2">
          {matieres.map((m) => {
            const n = finiesPar.get(m.cle);
            return (
              <li key={m.cle}>
                <a
                  href={`#/matiere/${encodeURIComponent(m.cle)}`}
                  className="cible block h-full rounded bg-surface px-4 py-3 shadow"
                >
                  <span className="block text-lg font-bold">{m.libelle}</span>
                  <span className="block text-muet">
                    {lecons(m.lecons)}
                    {suivi.actif && n !== undefined && n > 0 ? ` · ${finies(n)}` : ""}
                  </span>
                </a>
              </li>
            );
          })}
        </ul>
      )}
      {suivi.actif && (
        <p className="mt-6">
          <a href="#/avancement" className="cible inline-flex items-center font-bold text-primaire underline">
            {T.monAvancement}
          </a>
        </p>
      )}
    </>
  );
}
