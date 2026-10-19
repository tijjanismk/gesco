import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { choisirClasseur, choisirTableur } from "../../api/fichiers";
import { Bouton } from "../../composants/Bouton";
import { MessageErreur } from "../../composants/Messages";
import { t, texteDeCle } from "../../i18n";
import type { AnalyseImportProgramme, BilanImportProgramme } from "../../types/gesco";

/** Heures par semaine écrites comme sur l'emploi du temps : `1 h 30`. */
function heures(minutes: number | null): string {
  if (minutes === null) return "";
  const m = minutes % 60;
  return m === 0 ? `${String(minutes / 60)} h` : `${String(Math.floor(minutes / 60))} h ${String(m).padStart(2, "0")}`;
}

/** Import du programme depuis Excel, rangé dans les paramètres et replié : le modèle part du
 *  programme en vigueur de l'année ; le fichier corrigé est vérifié ligne par ligne, puis le
 *  programme des niveaux qu'il nomme est récrit. */
export function ImportProgramme() {
  const client = useQueryClient();
  const [chemin, setChemin] = useState<string | null>(null);
  const [analyse, setAnalyse] = useState<AnalyseImportProgramme | null>(null);
  const [bilan, setBilan] = useState<BilanImportProgramme | null>(null);
  const [modele, setModele] = useState<string | null>(null);
  const ecritureModele = useMutation({
    mutationFn: async () => {
      const destination = await choisirClasseur(t("import_programme.nom_modele"), t("commun.classeur_excel"));
      return destination === null ? null : appeler("programme_import_modele", { chemin: destination });
    },
    onSuccess: (ecrit) => setModele(ecrit?.chemin ?? null),
  });
  const lecture = useMutation({
    mutationFn: async () => {
      const choisi = await choisirTableur(t("import.filtre"));
      if (choisi === null) return null;
      return { chemin: choisi, analyse: await appeler("programme_import_analyser", { chemin: choisi }) };
    },
    onSuccess: (resultat) => {
      if (resultat === null) return;
      setChemin(resultat.chemin);
      setAnalyse(resultat.analyse);
      setBilan(null);
    },
  });
  const application = useMutation({
    mutationFn: () => appeler("programme_import_appliquer", { chemin: chemin ?? "" }),
    onSuccess: (resultat) => {
      setBilan(resultat);
      setAnalyse(null);
      for (const cle of ["pedagogie", "sauvegardes"]) void client.invalidateQueries({ queryKey: [cle] });
    },
  });
  return (
    <details className="rounded border border-muet/30 bg-surface p-3">
      <summary className="cursor-pointer text-sm font-medium text-encre">{t("import_programme.titre")}</summary>
      <div className="mt-3 flex flex-col gap-3">
        <p className="text-sm text-muet">{t("import_programme.explication")}</p>
        <div className="flex flex-wrap items-center gap-3">
          <Bouton variante="secondaire" disabled={ecritureModele.isPending} onClick={() => ecritureModele.mutate()}>
            {t("import_programme.modele")}
          </Bouton>
          <Bouton variante={analyse === null ? "principal" : "secondaire"} disabled={lecture.isPending} onClick={() => lecture.mutate()}>
            {t("import_programme.choisir")}
          </Bouton>
        </div>
        {modele === null ? null : <p role="status" className="text-sm text-encre">{t("import_programme.modele_ecrit", { chemin: modele })}</p>}
        {bilan === null ? null : (
          <p role="status" className="text-sm text-vert">
            {t("import_programme.bilan", { niveaux: bilan.niveaux, matieres: bilan.matieres_creees, ignores: bilan.niveaux_ignores })}
          </p>
        )}
        <MessageErreur erreur={ecritureModele.error ?? lecture.error ?? application.error} />
        {analyse === null ? null : (
          <>
            <p role="status" className="text-sm text-encre">
              {t("import_programme.analyse", { prets: analyse.niveaux_prets, ignores: analyse.niveaux_ignores })}
            </p>
            {analyse.nouvelles_matieres.length === 0 ? null : (
              <p className="text-sm text-ocre">{t("import_programme.nouvelles", { matieres: analyse.nouvelles_matieres.join(", ") })}</p>
            )}
            <div className="max-h-96 overflow-auto">
              <table className="w-full text-left font-condensee text-sm">
                <thead>
                  <tr className="text-muet">
                    {(["import.ligne", "import_programme.colonne.niveau", "import_programme.colonne.serie", "import_programme.colonne.matiere", "import_programme.colonne.coefficient", "import_programme.colonne.heures", "import.motifs"] as const).map((cle) => (
                      <th key={cle} className="px-2 py-1 font-normal">{t(cle)}</th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {analyse.lignes.map((l) => (
                    <tr key={l.numero} className={`border-b border-muet/30 ${l.erreurs.length > 0 ? "text-rouge" : "text-encre"}`}>
                      <td className="px-2 py-1">{String(l.numero)}</td>
                      <td className="px-2 py-1">{l.niveau}</td>
                      <td className="px-2 py-1">{l.serie}</td>
                      <td className="px-2 py-1">{l.nouvelle_matiere ? `${l.matiere} (${t("import_programme.nouvelle")})` : l.matiere}</td>
                      <td className="px-2 py-1">{l.coefficient === null ? "" : String(l.coefficient)}</td>
                      <td className="px-2 py-1">{heures(l.minutes_hebdo)}</td>
                      <td className="px-2 py-1">{l.erreurs.map((e) => texteDeCle(e)).join(" · ")}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <div>
              <Bouton disabled={analyse.niveaux_prets === 0 || application.isPending} onClick={() => application.mutate()}>
                {t("import_programme.importer", { niveaux: analyse.niveaux_prets })}
              </Bouton>
            </div>
          </>
        )}
      </div>
    </details>
  );
}
