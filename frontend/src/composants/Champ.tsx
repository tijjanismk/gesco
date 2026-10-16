import type { InputHTMLAttributes, ReactNode } from "react";

// Règle des encres (§19.6) : ce qu'une personne saisit s'affiche en bleu.
const SAISIE =
  "rounded border border-muet bg-surface px-2 py-1.5 text-primaire focus:outline-2 focus:outline-primaire disabled:bg-fond disabled:text-encre disabled:opacity-70";

export function Champ({
  libelle,
  id,
  aide,
  ...props
}: InputHTMLAttributes<HTMLInputElement> & { libelle: string; id: string; aide?: string }) {
  return (
    <label htmlFor={id} className="flex flex-col gap-1 text-sm text-encre">
      <span>{libelle}</span>
      <input id={id} className={SAISIE} {...props} />
      {aide === undefined ? null : <span className="text-muet">{aide}</span>}
    </label>
  );
}

export function ZoneTexte({
  libelle,
  id,
  aide,
  valeur,
  surChangement,
}: {
  libelle: string;
  id: string;
  aide?: string;
  valeur: string;
  surChangement: (valeur: string) => void;
}) {
  return (
    <label htmlFor={id} className="flex flex-col gap-1 text-sm text-encre">
      <span>{libelle}</span>
      <textarea
        id={id}
        rows={3}
        className={SAISIE}
        value={valeur}
        onChange={(e) => surChangement(e.target.value)}
        autoComplete="off"
        spellCheck={false}
      />
      {aide === undefined ? null : <span className="text-muet">{aide}</span>}
    </label>
  );
}

export function CaseACocher({
  libelle,
  id,
  coche,
  surChangement,
  desactive = false,
}: {
  libelle: string;
  id: string;
  coche: boolean;
  surChangement: (coche: boolean) => void;
  desactive?: boolean;
}) {
  return (
    <label htmlFor={id} className="flex items-center gap-2 text-sm text-encre">
      <input
        id={id}
        type="checkbox"
        className="accent-primaire"
        checked={coche}
        disabled={desactive}
        onChange={(e) => surChangement(e.target.checked)}
      />
      <span>{libelle}</span>
    </label>
  );
}

export function Selection({
  libelle,
  id,
  valeur,
  surChangement,
  options,
}: {
  libelle: string;
  id: string;
  valeur: string;
  surChangement: (valeur: string) => void;
  options: readonly { valeur: string; libelle: string }[];
}) {
  return (
    <label htmlFor={id} className="flex flex-col gap-1 text-sm text-encre">
      <span>{libelle}</span>
      <select
        id={id}
        className={SAISIE}
        value={valeur}
        onChange={(e) => surChangement(e.target.value)}
      >
        {options.map((o) => (
          <option key={o.valeur} value={o.valeur}>
            {o.libelle}
          </option>
        ))}
      </select>
    </label>
  );
}

export function Formulaire({
  surEnvoi,
  children,
}: {
  surEnvoi: () => void;
  children: ReactNode;
}) {
  return (
    <form
      className="flex flex-col gap-3"
      onSubmit={(e) => {
        e.preventDefault();
        surEnvoi();
      }}
    >
      {children}
    </form>
  );
}
