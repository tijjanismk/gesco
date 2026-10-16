import type { ReactNode } from "react";

import { ErreurApi } from "../api";
import { heure, t, texteDeCle } from "../i18n";

/** Erreur affichée là où l'on agit (§19.6, interdit 9) : ce qui ne va pas, en français simple. */
export function MessageErreur({ erreur }: { erreur: unknown }) {
  if (erreur === null || erreur === undefined) return null;
  const api = erreur instanceof ErreurApi ? erreur : null;
  const valeurs: Record<string, string> = api?.jusqua == null ? {} : { heure: heure(api.jusqua) };
  return (
    <div role="alert" className="text-sm text-rouge">
      <p>{api === null ? t("erreur.transport") : texteDeCle(api.cle, valeurs)}</p>
      {api?.detail == null ? null : <p className="font-condensee text-muet">{api.detail}</p>}
    </div>
  );
}

/** Confirmation sobre : « Enregistré à 10 h 42 ». */
export function MessageEnregistre({ instant }: { instant: number | null }) {
  if (instant === null) return null;
  return (
    <p role="status" className="text-sm text-vert">
      {t("commun.enregistre_a", { heure: heure(instant) })}
    </p>
  );
}

export function Section({ titre, children }: { titre: string; children: ReactNode }) {
  return (
    <section className="flex flex-col gap-3 rounded border border-muet/30 bg-surface p-4">
      <h2 className="text-base font-semibold text-encre">{titre}</h2>
      {children}
    </section>
  );
}
