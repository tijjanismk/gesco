import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { choisirClasseur, choisirTableur } from "../../api/fichiers";
import { Bouton } from "../../composants/Bouton";
import { MessageErreur } from "../../composants/Messages";
import { t, texteDeCle } from "../../i18n";
import type { AnalyseImportReferentiel, BilanImportReferentiel, NatureLigneReferentiel } from "../../types/gesco";

const NATURES: Record<NatureLigneReferentiel, "import_referentiel.nature.classe" | "import_referentiel.nature.matiere"> = {
  classe: "import_referentiel.nature.classe",
  matiere: "import_referentiel.nature.matiere",
};

/** Import des classes et des matières seules, sans le programme, rangé dans les paramètres et
 *  replié : un classeur à deux feuilles, chacune facultative ; ce qui existe déjà reste tel quel. */
export function ImportReferentiel() {
  const client = useQueryClient();
  const [chemin, setChemin] = useState<string | null>(null);
  const [analyse, setAnalyse] = useState<AnalyseImportReferentiel | null>(null);
  const [bilan, setBilan] = useState<BilanImportReferentiel | null>(null);
  const [modele, setModele] = useState<string | null>(null);
  const ecritureModele = useMutation({
    mutationFn: async () => {
      const destination = await choisirClasseur(t("import_referentiel.nom_modele"), t("commun.classeur_excel"));
      return destination === null ? null : appeler("referentiel_import_modele", { chemin: destination });
    },
    onSuccess: (ecrit) => setModele(ecrit?.chemin ?? null),
  });
  const lecture = useMutation({
    mutationFn: async () => {
      const choisi = await choisirTableur(t("import.filtre"));
      if (choisi === null) return null;
      return { chemin: choisi, analyse: await appeler("referentiel_import_analyser", { chemin: choisi }) };
    },
    onSuccess: (resultat) => {
      if (resultat === null) return;
      setChemin(resultat.chemin);
      setAnalyse(resultat.analyse);
      setBilan(null);
    },
  });
  const application = useMutation({
    mutationFn: () => appeler("referentiel_import_appliquer", { chemin: chemin ?? "" }),
    onSuccess: (resultat) => {
      setBilan(resultat);
      setAnalyse(null);
      for (const cle of ["classes", "pedagogie", "sauvegardes"]) void client.invalidateQueries({ queryKey: [cle] });
    },
  });
  const aCreer = analyse === null ? 0 : analyse.classes_a_creer + analyse.matieres_a_creer;
  return (
    <details className="rounded border border-muet/30 bg-surface p-3">
      <summary className="cursor-pointer text-sm font-medium text-encre">{t("import_referentiel.titre")}</summary>
      <div className="mt-3 flex flex-col gap-3">
        <p className="text-sm text-muet">{t("import_referentiel.explication")}</p>
        <div className="flex flex-wrap items-center gap-3">
          <Bouton variante="secondaire" disabled={ecritureModele.isPending} onClick={() => ecritureModele.mutate()}>
            {t("import_referentiel.modele")}
          </Bouton>
          <Bouton variante={analyse === null ? "principal" : "secondaire"} disabled={lecture.isPending} onClick={() => lecture.mutate()}>
            {t("import_referentiel.choisir")}
          </Bouton>
        </div>
        {modele === null ? null : <p role="status" className="text-sm text-encre">{t("import_referentiel.modele_ecrit", { chemin: modele })}</p>}
        {bilan === null ? null : (
          <p role="status" className="text-sm text-vert">
            {t("import_referentiel.bilan", { classes: bilan.classes, matieres: bilan.matieres, ignorees: bilan.ignorees })}
          </p>
        )}
        <MessageErreur erreur={ecritureModele.error ?? lecture.error ?? application.error} />
        {analyse === null ? null : (
          <>
            <p role="status" className="text-sm text-encre">
              {t("import_referentiel.analyse", { classes: analyse.classes_a_creer, matieres: analyse.matieres_a_creer })}
            </p>
            <div className="max-h-96 overflow-auto">
              <table className="w-full text-left font-condensee text-sm">
                <thead>
                  <tr className="text-muet">
                    {(["import_referentiel.colonne.feuille", "import.ligne", "import_referentiel.colonne.nature", "import_referentiel.colonne.nom", "import_referentiel.colonne.detail", "import.motifs"] as const).map((cle) => (
                      <th key={cle} className="px-2 py-1 font-normal">{t(cle)}</th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {analyse.lignes.map((l) => (
                    <tr key={`${l.feuille}-${String(l.numero)}`} className={`border-b border-muet/30 ${l.erreurs.length > 0 ? "text-rouge" : l.deja ? "text-muet" : "text-encre"}`}>
                      <td className="px-2 py-1">{l.feuille}</td>
                      <td className="px-2 py-1">{String(l.numero)}</td>
                      <td className="px-2 py-1">{t(NATURES[l.nature])}</td>
                      <td className="px-2 py-1">{l.nom}</td>
                      <td className="px-2 py-1">{l.detail}</td>
                      <td className="px-2 py-1">{[...l.erreurs.map((e) => texteDeCle(e)), ...(l.deja ? [t("import_referentiel.deja")] : [])].join(" · ")}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <div>
              <Bouton disabled={aCreer === 0 || application.isPending} onClick={() => application.mutate()}>
                {t("import_referentiel.importer", { nombre: aCreer })}
              </Bouton>
            </div>
          </>
        )}
      </div>
    </details>
  );
}
