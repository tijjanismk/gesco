import { Selection } from "../../composants/Champ";
import { t } from "../../i18n";
import type { CritereLibre, PopulationLibre } from "../../types/gesco";
import { CRITERES, TITRES_CRITERES, TITRES_POPULATIONS, choixApres } from "./tableau-libre";

/** Choix du tableau libre : ce qu'il compte, ce qui fait ses lignes et ses colonnes. Les
 *  rubriques de la fiche se lisent en une colonne, leur réponse. */
export function ChoixTableauLibre({
  population,
  lignes,
  colonnes,
  surChanger,
}: {
  population: PopulationLibre;
  lignes: CritereLibre;
  colonnes: CritereLibre | null;
  surChanger: (population: PopulationLibre, lignes: CritereLibre, colonnes: CritereLibre | null) => void;
}) {
  const criteres = CRITERES[population];
  const options = criteres.map((c) => ({ valeur: c, libelle: t(TITRES_CRITERES[c]) }));
  return (
    <div className="flex flex-wrap items-end gap-3">
      <Selection
        id="libre-population"
        libelle={t("statistiques.libre.population")}
        valeur={population}
        surChangement={(v) => {
          const p = v as PopulationLibre;
          const [l, c] = choixApres(p, lignes, colonnes);
          surChanger(p, l, c);
        }}
        options={(["eleves", "personnel", "fiche"] as const).map((p) => ({ valeur: p, libelle: t(TITRES_POPULATIONS[p]) }))}
      />
      {criteres.length === 0 ? null : (
        <>
          <Selection
            id="libre-lignes"
            libelle={t("statistiques.libre.lignes")}
            valeur={lignes}
            surChangement={(v) => surChanger(population, v as CritereLibre, colonnes === v ? null : colonnes)}
            options={options}
          />
          <Selection
            id="libre-colonnes"
            libelle={t("statistiques.libre.colonnes")}
            valeur={colonnes ?? ""}
            surChangement={(v) => surChanger(population, lignes, v === "" ? null : (v as CritereLibre))}
            options={[{ valeur: "", libelle: t("statistiques.libre.sans_colonnes") }, ...options.filter((o) => o.valeur !== lignes)]}
          />
        </>
      )}
    </div>
  );
}
