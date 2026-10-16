import { useState } from "react";

import { appeler } from "../api";
import { choisirClasseur } from "../api/fichiers";
import { t } from "../i18n";
import { Bouton } from "./Bouton";
import { MessageErreur } from "./Messages";

export interface Colonne<T> {
  titre: string;
  valeur: (ligne: T) => string;
}

/** CSV au séparateur point-virgule, celui qu'ouvre Excel en français. */
function versCsv<T>(colonnes: readonly Colonne<T>[], lignes: readonly T[]): string {
  const citer = (texte: string) =>
    /[;"\n]/.test(texte) ? `"${texte.replaceAll('"', '""')}"` : texte;
  return [
    colonnes.map((c) => citer(c.titre)).join(";"),
    ...lignes.map((l) => colonnes.map((c) => citer(c.valeur(l))).join(";")),
  ].join("\r\n");
}

function telecharger(nom: string, contenu: string) {
  const url = URL.createObjectURL(new Blob(["﻿", contenu], { type: "text/csv;charset=utf-8" }));
  const lien = document.createElement("a");
  lien.href = url;
  lien.download = `${nom}.csv`;
  lien.click();
  URL.revokeObjectURL(url);
}

type Exportation = { chemin: string } | { erreur: unknown } | null;

/** Liste dense, exportable en Excel et en CSV (R-41, §19.1). Le classeur Excel est composé par
 *  le noyau à partir de ce que la liste montre, et écrit où l'utilisateur le choisit. */
export function Tableau<T>({
  colonnes,
  lignes,
  cle,
  nomExport,
  titre,
  surSelection,
}: {
  colonnes: readonly Colonne<T>[];
  lignes: readonly T[];
  cle: (ligne: T) => string;
  nomExport: string;
  titre?: string;
  surSelection?: (ligne: T) => void;
}) {
  const [exportation, setExportation] = useState<Exportation>(null);
  const [enCours, setEnCours] = useState(false);
  const exporterExcel = async () => {
    setExportation(null);
    setEnCours(true);
    try {
      const chemin = await choisirClasseur(nomExport, t("commun.classeur_excel"));
      if (chemin === null) return;
      const ecrit = await appeler("exporter_xlsx", {
        chemin,
        titre: titre ?? nomExport,
        entetes: colonnes.map((c) => c.titre),
        lignes: lignes.map((l) => colonnes.map((c) => c.valeur(l))),
      });
      setExportation({ chemin: ecrit.chemin });
    } catch (erreur) {
      setExportation({ erreur });
    } finally {
      setEnCours(false);
    }
  };
  return (
    <div className="flex flex-col gap-2">
      <div className="flex flex-wrap items-center gap-2">
        <Bouton variante="secondaire" disabled={enCours} onClick={() => void exporterExcel()}>
          {t("commun.exporter_excel")}
        </Bouton>
        <Bouton variante="secondaire" onClick={() => telecharger(nomExport, versCsv(colonnes, lignes))}>
          {t("commun.exporter_csv")}
        </Bouton>
        {exportation !== null && "chemin" in exportation ? (
          <p role="status" className="text-sm text-muet">{t("commun.export_enregistre", { chemin: exportation.chemin })}</p>
        ) : null}
      </div>
      <MessageErreur erreur={exportation !== null && "erreur" in exportation ? exportation.erreur : null} />
      {lignes.length === 0 ? (
        <p className="text-sm text-muet">{t("commun.aucune_ligne")}</p>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full border-collapse font-condensee text-sm tabular-nums">
            <thead>
              <tr>
                {colonnes.map((c) => (
                  <th key={c.titre} className="border-b border-muet px-2 py-1 text-left font-medium text-muet">
                    {c.titre}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {lignes.map((l) => (
                <tr
                  key={cle(l)}
                  className={surSelection === undefined ? "" : "cursor-pointer hover:bg-fond"}
                  onClick={surSelection === undefined ? undefined : () => surSelection(l)}
                >
                  {colonnes.map((c) => (
                    <td key={c.titre} className="border-b border-muet/30 px-2 py-1 text-encre">
                      {c.valeur(l)}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
