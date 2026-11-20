import { lazy, Suspense, useCallback, useEffect, useMemo, useState } from "react";

import { envoyer } from "./api";
import { Avancement } from "./Avancement";
import { Attente, Chargement } from "./communs";
import { Chapitres } from "./Chapitres";
import { Devoirs } from "./Devoirs";
import { Entrer } from "./Entrer";
import { ContexteSortie, useLecture } from "./lecture";
import { Matieres } from "./Matieres";
import { ContexteSuivi, type Suivi } from "./suivi";
import { T } from "./textes";
import type { Accueil, MonAvancement } from "./types";

// La leçon apporte KaTeX : elle se charge seulement à l'ouverture d'une leçon.
const Lecon = lazy(() => import("./Lecon").then((m) => ({ default: m.Lecon })));
const Quiz = lazy(() => import("./Quiz").then((m) => ({ default: m.Quiz })));
const Devoir = lazy(() => import("./Devoir").then((m) => ({ default: m.Devoir })));

/** Écran montré, lu dans l'adresse (`#/matiere/…`, `#/lecon/…`) : le bouton « retour » du
 *  téléphone marche comme partout. */
export type Ecran =
  | { nom: "matieres" }
  | { nom: "matiere"; cle: string }
  | { nom: "lecon"; id: string }
  | { nom: "quiz"; id: string }
  | { nom: "revision"; id: string; matiere: string }
  | { nom: "avancement" }
  | { nom: "devoirs" }
  | { nom: "devoir"; id: string };

function lireAdresse(): Ecran {
  const [nom, argument] = window.location.hash.replace(/^#\/?/, "").split("/");
  const valeur = argument === undefined ? "" : decodeURIComponent(argument);
  if (nom === "matiere" && valeur !== "") return { nom: "matiere", cle: valeur };
  if (nom === "lecon" && valeur !== "") return { nom: "lecon", id: valeur };
  if (nom === "quiz" && valeur !== "") return { nom: "quiz", id: valeur };
  if (nom === "revision" && valeur !== "") {
    const matiere = window.location.hash.replace(/^#\/?/, "").split("/")[2];
    return { nom: "revision", id: valeur, matiere: matiere === undefined ? "" : decodeURIComponent(matiere) };
  }
  if (nom === "avancement") return { nom: "avancement" };
  if (nom === "devoirs") return { nom: "devoirs" };
  if (nom === "devoir" && valeur !== "") return { nom: "devoir", id: valeur };
  return { nom: "matieres" };
}

export function App() {
  const [ecran, setEcran] = useState<Ecran>(lireAdresse);
  const [cleAccueil, setCleAccueil] = useState(0);
  const [cleAvancement, setCleAvancement] = useState(0);
  const accueil = useLecture<Accueil>("accueil", cleAccueil);

  useEffect(() => {
    const suivre = () => {
      setEcran(lireAdresse());
      window.scrollTo(0, 0);
    };
    window.addEventListener("hashchange", suivre);
    return () => window.removeEventListener("hashchange", suivre);
  }, []);

  const revenirALEntree = useCallback(() => {
    window.location.hash = "";
    setCleAccueil((c) => c + 1);
  }, []);

  const eleve = accueil.etat === "pret" ? accueil.donnee.eleve : null;
  const suivi = accueil.etat === "pret" && accueil.donnee.suivi && eleve?.avec_code === true;
  const avancement = useLecture<MonAvancement>(suivi ? "mon-avancement" : null, cleAvancement);
  const rafraichir = useCallback(() => setCleAvancement((c) => c + 1), []);
  const contexteSuivi = useMemo<Suivi>(
    () => ({
      actif: suivi,
      avancement: avancement.etat === "pret" ? avancement.donnee : null,
      rafraichir,
    }),
    [suivi, avancement, rafraichir],
  );

  if (accueil.etat === "occupe") return <Attente />;
  if (accueil.etat !== "pret") return <Chargement />;
  const donnees = accueil.donnee;

  const sortir = () => {
    envoyer("sortir", {})
      .catch(() => null)
      .finally(revenirALEntree);
  };

  if (eleve === null) {
    return (
      <ContexteSortie.Provider value={revenirALEntree}>
        <Entrer accueil={donnees} surEntree={revenirALEntree} />
      </ContexteSortie.Provider>
    );
  }

  return (
    <ContexteSortie.Provider value={revenirALEntree}>
      <ContexteSuivi.Provider value={contexteSuivi}>
        <div className="min-h-screen bg-fond text-encre">
          <header className="bg-marine text-sur-marine">
            <div className="mx-auto flex max-w-2xl items-center gap-3 px-4 py-2">
              <a href="#/" className="cible flex min-w-0 flex-1 flex-col justify-center">
                <span className="truncate text-sm">{donnees.ecole}</span>
                <span className="truncate font-bold">
                  {eleve.prenom === null ? eleve.niveau : `${eleve.prenom}, ${eleve.classe ?? eleve.niveau}`}
                </span>
              </a>
              <button type="button" onClick={sortir} className="cible rounded border border-sur-marine px-3 font-bold">
                {T.sortir}
              </button>
            </div>
          </header>
          <main className="mx-auto max-w-2xl px-4 pt-4 pb-12">
            {ecran.nom === "matieres" && <Matieres />}
            {ecran.nom === "matiere" && <Chapitres cle={ecran.cle} />}
            {ecran.nom === "avancement" && <Avancement />}
            {ecran.nom === "devoirs" && <Devoirs />}
            {ecran.nom === "devoir" && (
              <Suspense fallback={<Chargement />}>
                <Devoir id={ecran.id} avecCode={eleve.avec_code} />
              </Suspense>
            )}
            {ecran.nom === "lecon" && (
              <Suspense fallback={<Chargement />}>
                <Lecon id={ecran.id} accueil={donnees} />
              </Suspense>
            )}
            {ecran.nom === "quiz" && (
              <Suspense fallback={<Chargement />}>
                <Quiz chemin={`quiz/${encodeURIComponent(ecran.id)}`} retour={`#/lecon/${encodeURIComponent(ecran.id)}`} />
              </Suspense>
            )}
            {ecran.nom === "revision" && (
              <Suspense fallback={<Chargement />}>
                <Quiz
                  chemin={`revision/${encodeURIComponent(ecran.id)}`}
                  retour={ecran.matiere === "" ? "#/" : `#/matiere/${encodeURIComponent(ecran.matiere)}`}
                />
              </Suspense>
            )}
          </main>
        </div>
      </ContexteSuivi.Provider>
    </ContexteSortie.Provider>
  );
}
