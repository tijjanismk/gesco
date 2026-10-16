import { useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../api";
import { dateCivile, t } from "../i18n";
import { telephoneLisible } from "../modules/scolarite/Identite";

interface Resultat {
  cle: string;
  eleveId: string;
  titre: string;
  detail: string;
}

/** Recherche globale (§19.3), ouverte par Ctrl+K : élèves et responsables, tolérante aux fautes,
 *  entièrement au clavier (R-42). Un responsable ouvre la fiche de son premier enfant. */
export function RechercheGlobale({ surOuvrir, surFermer }: { surOuvrir: (eleveId: string) => void; surFermer: () => void }) {
  const [texte, setTexte] = useState("");
  const [actif, setActif] = useState(0);
  const cherche = texte.trim().length >= 2;
  const eleves = useQuery({ queryKey: ["recherche-eleves", texte], queryFn: () => appeler("eleves_rechercher", { texte }), enabled: cherche });
  const responsables = useQuery({ queryKey: ["recherche-responsables", texte], queryFn: () => appeler("responsables_rechercher", { texte }), enabled: cherche });
  const resultats: Resultat[] = cherche
    ? [
        ...(eleves.data ?? []).slice(0, 8).map((e) => ({
          cle: `e-${e.id}`,
          eleveId: e.id,
          titre: `${e.prenoms} ${e.nom}`,
          detail: [e.matricule_interne, e.date_naissance === null ? "" : dateCivile(e.date_naissance)].filter((x) => x !== "").join(" · "),
        })),
        ...(responsables.data ?? []).slice(0, 5).flatMap((r) => {
          const enfant = r.enfants[0];
          return enfant === undefined
            ? []
            : [
                {
                  cle: `r-${r.responsable.id}`,
                  eleveId: enfant.id,
                  titre: `${r.identite.prenoms} ${r.identite.nom}`,
                  detail: t("recherche.responsable_de", { enfants: r.enfants.map((x) => x.nom).join(", "), telephone: r.identite.telephones.map(telephoneLisible).join(" · ") }),
                },
              ];
        }),
      ]
    : [];
  const choisi = Math.min(actif, Math.max(resultats.length - 1, 0));
  const ouvrir = (r: Resultat | undefined) => {
    if (r !== undefined) surOuvrir(r.eleveId);
  };
  return (
    <div className="fixed inset-0 z-10 flex items-start justify-center bg-encre/40 pt-24" onClick={surFermer}>
      <div role="dialog" aria-label={t("recherche.titre")} className="w-full max-w-xl rounded border border-muet bg-surface p-3 shadow" onClick={(e) => e.stopPropagation()}>
        <input
          autoFocus
          type="search"
          aria-label={t("recherche.titre")}
          placeholder={t("recherche.aide")}
          className="w-full rounded border border-muet bg-surface px-3 py-2 text-primaire focus:outline-2 focus:outline-primaire"
          value={texte}
          onChange={(e) => {
            setTexte(e.target.value);
            setActif(0);
          }}
          onKeyDown={(e) => {
            if (e.key === "ArrowDown") {
              e.preventDefault();
              setActif(Math.min(choisi + 1, resultats.length - 1));
            } else if (e.key === "ArrowUp") {
              e.preventDefault();
              setActif(Math.max(choisi - 1, 0));
            } else if (e.key === "Enter") {
              e.preventDefault();
              ouvrir(resultats[choisi]);
            } else if (e.key === "Escape") {
              surFermer();
            }
          }}
        />
        {cherche && resultats.length === 0 && !eleves.isPending ? <p className="px-1 pt-3 text-sm text-muet">{t("recherche.aucun")}</p> : null}
        <ul className="mt-2 flex flex-col">
          {resultats.map((r, i) => (
            <li key={r.cle}>
              <button
                type="button"
                className={`w-full rounded px-3 py-2 text-left ${i === choisi ? "bg-fond" : ""}`}
                onMouseEnter={() => setActif(i)}
                onClick={() => ouvrir(r)}
              >
                <span className="block text-sm font-medium text-encre">{r.titre}</span>
                <span className="block text-xs text-muet">{r.detail}</span>
              </button>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}
