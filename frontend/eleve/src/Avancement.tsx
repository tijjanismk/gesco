import { Chargement, Retour } from "./communs";
import { useSuivi } from "./suivi";
import { finiesSur, T } from "./textes";

/** Mon avancement : par matière, leçons finies sur leçons visibles ; dernières leçons ouvertes.
 *  Rien ne classe ni ne compare les élèves. */
export function Avancement() {
  const suivi = useSuivi();
  if (!suivi.actif) {
    return (
      <>
        <Retour vers="#/" texte={T.mesMatieres} />
        <p className="mt-4 text-muet">{T.avancementPasGarde}</p>
      </>
    );
  }
  if (suivi.avancement === null) return <Chargement />;
  const { matieres, recentes } = suivi.avancement;
  return (
    <>
      <Retour vers="#/" texte={T.mesMatieres} />
      <h1 className="mt-2 mb-4 text-2xl font-bold">{T.monAvancement}</h1>
      <ul className="flex flex-col gap-2">
        {matieres.map((m) => (
          <li key={m.cle} className="rounded bg-surface px-4 py-3 shadow">
            <span className="block font-bold">{m.libelle}</span>
            <span className="block text-muet">{finiesSur(m.finies, m.visibles)}</span>
            <span className="mt-2 block h-2 rounded bg-fond" aria-hidden="true">
              <span
                className="block h-2 rounded bg-vert"
                style={{ width: `${m.visibles === 0 ? 0 : Math.round((100 * m.finies) / m.visibles)}%` }}
              />
            </span>
          </li>
        ))}
      </ul>
      {recentes.length > 0 && (
        <>
          <h2 className="mt-6 mb-2 text-xl font-bold">{T.leconsRecentes}</h2>
          <ul className="flex flex-col gap-2">
            {recentes.map((l) => (
              <li key={l.id}>
                <a href={`#/lecon/${encodeURIComponent(l.id)}`} className="cible block rounded bg-surface px-4 py-2 shadow">
                  <span className="block font-bold">{l.titre}</span>
                  <span className="block text-sm text-muet">{l.matiere}</span>
                </a>
              </li>
            ))}
          </ul>
        </>
      )}
    </>
  );
}
