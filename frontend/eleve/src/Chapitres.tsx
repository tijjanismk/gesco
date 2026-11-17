import { Attente, Chargement, Retour } from "./communs";
import { useLecture } from "./lecture";
import { useSuivi } from "./suivi";
import { nature, T } from "./textes";
import type { CatalogueEleve } from "./types";

/** Les chapitres d'une matière, dans l'ordre du programme ; une coche pour une leçon finie. */
export function Chapitres({ cle }: { cle: string }) {
  const catalogue = useLecture<CatalogueEleve>("catalogue");
  const suivi = useSuivi();
  if (catalogue.etat === "occupe") return <Attente />;
  if (catalogue.etat !== "pret") return <Chargement />;
  const matiere = catalogue.donnee.matieres.find((m) => m.cle === cle);
  const finies = new Set(suivi.avancement?.finies ?? []);

  return (
    <>
      <Retour vers="#/" texte={T.mesMatieres} />
      {matiere === undefined ? (
        <p className="mt-4 text-muet">{T.introuvable}</p>
      ) : (
        <>
          <h1 className="mt-2 mb-4 text-2xl font-bold">{matiere.libelle}</h1>
          {matiere.parties.map((partie, i) => (
            <section key={`${partie.titre}-${i}`} className="mb-6">
              {partie.titre !== "" && <h2 className="mb-2 text-xl font-bold text-muet">{partie.titre}</h2>}
              {partie.chapitres.map((chapitre) => (
                <div key={chapitre.id} className="mb-4">
                  <h3 className="mb-2 text-lg font-bold">{chapitre.titre}</h3>
                  <ul className="flex flex-col gap-2">
                    {chapitre.contenus.map((c) => {
                      const finie = suivi.actif && finies.has(c.id);
                      return (
                        <li key={c.id}>
                          <a
                            href={`#/lecon/${encodeURIComponent(c.id)}`}
                            className="cible flex items-center gap-3 rounded bg-surface px-4 py-2 shadow"
                          >
                            <span className="min-w-0 flex-1">
                              <span className="block font-bold">{c.titre}</span>
                              <span className="block text-sm text-muet">{nature(c)}</span>
                            </span>
                            {finie && (
                              <span className="shrink-0 font-bold text-vert">
                                <span aria-hidden="true">✓ </span>
                                {T.finie}
                              </span>
                            )}
                          </a>
                        </li>
                      );
                    })}
                  </ul>
                </div>
              ))}
            </section>
          ))}
        </>
      )}
    </>
  );
}
