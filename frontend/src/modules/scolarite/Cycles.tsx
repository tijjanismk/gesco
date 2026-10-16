import { CaseACocher } from "../../composants/Champ";
import { type Cle, t } from "../../i18n";
import type { Cycle } from "../../types/gesco";

export const CYCLES: Record<Cycle, Cle> = {
  jardin: "niveaux.cycle.jardin",
  premier_cycle: "niveaux.cycle.premier_cycle",
  second_cycle: "niveaux.cycle.second_cycle",
  lycee: "niveaux.cycle.lycee",
  technique: "niveaux.cycle.technique",
};

/** Les cycles dans l'ordre : l'enseignement général, puis le technique. */
export const ORDRE_CYCLES: readonly Cycle[] = ["jardin", "premier_cycle", "second_cycle", "lycee", "technique"];
const ORDRE = ORDRE_CYCLES;

/** Cycles enseignés par l'école : seuls leurs niveaux servent aux classes et aux inscriptions. */
export function ChoixCycles({ choisis, surChangement }: { choisis: readonly Cycle[]; surChangement: (cycles: Cycle[]) => void }) {
  return (
    <fieldset className="flex flex-col gap-1">
      <legend className="text-sm text-encre">{t("niveaux.cycles_enseignes")}</legend>
      {ORDRE.map((cycle) => (
        <CaseACocher
          key={cycle}
          id={`cycle-${cycle}`}
          libelle={t(CYCLES[cycle])}
          coche={choisis.includes(cycle)}
          surChangement={(coche) => surChangement(ORDRE.filter((c) => (c === cycle ? coche : choisis.includes(c))))}
        />
      ))}
    </fieldset>
  );
}
