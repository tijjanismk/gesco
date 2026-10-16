import type { Cycle, Referentiel, SaisieLigne } from "../../types/gesco";

/** Un frais saisi une fois pour plusieurs niveaux : scolarité du 1er cycle, tenue scolaire de
 *  toute l'école. Le noyau garde une ligne par niveau ; le regroupement n'existe qu'à l'écran. */
export interface GroupeLigne extends Omit<SaisieLigne, "niveau_id"> {
  niveaux: string[];
}

function cleDe(l: SaisieLigne): string {
  return JSON.stringify([l.type_frais, l.libelle.trim(), l.montant, l.obligatoire, l.exigibilite]);
}

/** Réunit les lignes identiques à leur niveau près, dans l'ordre de leur première apparition.
 *  Une ligne propre à une série reste seule : la série dépend de son niveau. */
export function grouper(lignes: readonly SaisieLigne[]): GroupeLigne[] {
  const groupes: GroupeLigne[] = [];
  const index = new Map<string, GroupeLigne>();
  for (const { niveau_id, ...reste } of lignes) {
    const cle = cleDe({ niveau_id, ...reste });
    const existant = reste.serie_id === null ? index.get(cle) : undefined;
    if (existant !== undefined && !existant.niveaux.includes(niveau_id)) {
      existant.niveaux.push(niveau_id);
      continue;
    }
    const groupe: GroupeLigne = { ...reste, niveaux: [niveau_id] };
    groupes.push(groupe);
    if (reste.serie_id === null) index.set(cle, groupe);
  }
  return groupes;
}

/** Une ligne par niveau, dans l'ordre des groupes puis des niveaux du référentiel. */
export function deplier(groupes: readonly GroupeLigne[], ordre: readonly string[]): SaisieLigne[] {
  const rang = (id: string) => {
    const i = ordre.indexOf(id);
    return i === -1 ? ordre.length : i;
  };
  return groupes.flatMap(({ niveaux, ...reste }) =>
    [...niveaux].sort((a, b) => rang(a) - rang(b)).map((niveau_id) => ({ ...reste, niveau_id })),
  );
}

/** Niveaux d'un groupe en clair : un cycle entier se nomme par son cycle. */
export function resumeNiveaux(referentiel: Referentiel, niveaux: readonly string[], libelleCycle: (c: Cycle) => string): string {
  const actifs = referentiel.niveaux.filter((n) => n.actif);
  const morceaux: string[] = [];
  const cycles: Cycle[] = ["jardin", "premier_cycle", "second_cycle", "lycee", "technique"];
  for (const cycle of cycles) {
    const ducycle = actifs.filter((n) => n.cycle === cycle);
    const choisis = ducycle.filter((n) => niveaux.includes(n.id));
    if (choisis.length === 0) continue;
    if (choisis.length === ducycle.length && ducycle.length > 1) morceaux.push(libelleCycle(cycle));
    else morceaux.push(...choisis.map((n) => n.libelle));
  }
  const inactifs = referentiel.niveaux.filter((n) => !n.actif && niveaux.includes(n.id)).map((n) => n.libelle);
  return [...morceaux, ...inactifs].join(", ");
}
