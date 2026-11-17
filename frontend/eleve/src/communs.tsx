import { T } from "./textes";

export function Chargement() {
  return (
    <p role="status" className="px-4 py-8 text-center text-muet">
      {T.chargement}
    </p>
  );
}

/** Le poste principal est occupé : la page réessaie seule. */
export function Attente() {
  return (
    <p role="status" className="mx-auto max-w-md px-4 py-8 text-center text-lg">
      {T.occupe}
    </p>
  );
}

export function Retour({ vers, texte = T.retour }: { vers: string; texte?: string }) {
  return (
    <a href={vers} className="cible inline-flex items-center gap-1 font-bold text-primaire underline">
      <span aria-hidden="true">←</span> {texte}
    </a>
  );
}
