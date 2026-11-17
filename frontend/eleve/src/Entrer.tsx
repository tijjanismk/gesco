import { useState, type FormEvent } from "react";

import { envoyer, Occupe, Refus } from "./api";
import { T } from "./textes";
import type { Accueil, NiveauALire } from "./types";

const champ = "cible w-full rounded border-2 border-muet bg-surface px-3 text-lg text-encre";

/** Entrer avec son matricule et son code, ou lire sans code en choisissant sa classe. */
export function Entrer({ accueil, surEntree }: { accueil: Accueil; surEntree: () => void }) {
  const [matricule, setMatricule] = useState("");
  const [code, setCode] = useState("");
  const [pasMonTelephone, setPasMonTelephone] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [envoi, setEnvoi] = useState(false);
  const [sansCode, setSansCode] = useState(false);

  const echec = (e: unknown) => {
    if (e instanceof Refus && e.statut === 403) {
      const refus = (e.corps as { refus?: string } | null)?.refus;
      setMessage(refus === "bloque" ? T.refusBloque : T.refusIncorrect);
    } else if (e instanceof Occupe) {
      setMessage(T.occupe);
    } else {
      setMessage(T.refusIncorrect);
    }
  };

  const entrer = (ev: FormEvent) => {
    ev.preventDefault();
    if (envoi) return;
    setEnvoi(true);
    setMessage(null);
    envoyer("entrer", { matricule, code, pas_mon_telephone: pasMonTelephone })
      .then(surEntree)
      .catch(echec)
      .finally(() => setEnvoi(false));
  };

  const lireSansCode = (n: NiveauALire) => {
    if (envoi) return;
    setEnvoi(true);
    setMessage(null);
    envoyer("lire-sans-code", { niveau_id: n.niveau_id, serie_id: n.serie_id })
      .then(surEntree)
      .catch((e: unknown) => setMessage(e instanceof Occupe ? T.occupe : T.aucuneLecon))
      .finally(() => setEnvoi(false));
  };

  return (
    <main className="mx-auto min-h-screen max-w-md bg-fond px-4 py-6 text-encre">
      <p className="text-muet">{accueil.ecole}</p>
      <h1 className="mt-1 mb-6 text-2xl font-bold text-encre">{T.titreEntree}</h1>
      <form onSubmit={entrer} className="flex flex-col gap-4">
        <label className="flex flex-col gap-1 font-bold">
          {T.matricule}
          <input
            className={champ}
            value={matricule}
            onChange={(e) => setMatricule(e.target.value)}
            autoComplete="username"
            autoCapitalize="characters"
            spellCheck={false}
            required
          />
        </label>
        <label className="flex flex-col gap-1 font-bold">
          {T.code}
          <input
            className={`${champ} tracking-widest uppercase`}
            value={code}
            onChange={(e) => setCode(e.target.value)}
            autoComplete="one-time-code"
            autoCapitalize="characters"
            spellCheck={false}
            maxLength={12}
            required
          />
        </label>
        <label className="cible flex items-center gap-3">
          <input
            type="checkbox"
            className="h-6 w-6"
            checked={pasMonTelephone}
            onChange={(e) => setPasMonTelephone(e.target.checked)}
          />
          {T.pasMonTelephone}
        </label>
        {message !== null && (
          <p role="alert" className="rounded border-2 border-rouge bg-surface px-3 py-2 font-bold text-rouge">
            {message}
          </p>
        )}
        <button type="submit" disabled={envoi} className="cible rounded bg-primaire px-4 text-lg font-bold text-surface">
          {T.entrer}
        </button>
      </form>

      <section className="mt-8 border-t-2 border-muet pt-6">
        {!sansCode ? (
          <button
            type="button"
            onClick={() => setSansCode(true)}
            className="cible w-full rounded border-2 border-primaire px-4 text-lg font-bold text-primaire"
          >
            {T.lireSansCode}
          </button>
        ) : (
          <>
            <h2 className="text-lg font-bold">{T.choisisNiveau}</h2>
            <p className="mb-3 text-muet">{T.avancementPasGarde}</p>
            <ul className="flex flex-col gap-2">
              {accueil.niveaux.map((n) => (
                <li key={`${n.niveau_id}-${n.serie_id ?? ""}`}>
                  <button
                    type="button"
                    disabled={envoi}
                    onClick={() => lireSansCode(n)}
                    className="cible w-full rounded bg-surface px-4 py-2 text-left text-lg font-bold text-encre shadow"
                  >
                    {n.libelle}
                  </button>
                </li>
              ))}
            </ul>
          </>
        )}
      </section>
    </main>
  );
}
