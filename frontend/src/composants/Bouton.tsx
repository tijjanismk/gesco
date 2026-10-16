import type { ButtonHTMLAttributes } from "react";

type Variante = "principal" | "secondaire" | "danger";

const STYLES: Record<Variante, string> = {
  principal: "bg-primaire text-surface hover:opacity-90",
  secondaire: "border border-muet text-encre hover:bg-fond",
  danger: "bg-rouge text-surface hover:opacity-90",
};

export function Bouton({
  variante = "principal",
  className = "",
  type = "button",
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & { variante?: Variante }) {
  return (
    <button
      type={type}
      className={`rounded px-3 py-1.5 text-sm font-medium focus-visible:outline-2 focus-visible:outline-primaire disabled:opacity-50 ${STYLES[variante]} ${className}`}
      {...props}
    />
  );
}
