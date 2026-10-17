import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { choisirClasseur, choisirTableur } from "../../api/fichiers";
import { Bouton } from "../../composants/Bouton";
import { MessageErreur, Section } from "../../composants/Messages";
import { t, texteDeCle } from "../../i18n";
import type { AnalyseImportPersonnel, BilanImportPersonnel, CategoriePersonnel } from "../../types/gesco";

/** Import d'une liste du personnel : le modèle à remplir, la liste vérifiée ligne par ligne, puis
 *  les fiches créées. Le fichier est lu par le noyau ; l'écran montre ce qu'il en a compris. */
export function ImportPersonnel({ categories, surTermine }: { categories: readonly CategoriePersonnel[]; surTermine: () => void }) {
  const client = useQueryClient();
  const [chemin, setChemin] = useState<string | null>(null);
  const [analyse, setAnalyse] = useState<AnalyseImportPersonnel | null>(null);
  const [bilan, setBilan] = useState<BilanImportPersonnel | null>(null);
  const [modele, setModele] = useState<string | null>(null);
  const ecritureModele = useMutation({
    mutationFn: async () => {
      const destination = await choisirClasseur(t("import_personnel.nom_modele"), t("commun.classeur_excel"));
      return destination === null ? null : appeler("personnel_import_modele", { chemin: destination });
    },
    onSuccess: (ecrit) => setModele(ecrit?.chemin ?? null),
  });
  const lecture = useMutation({
    mutationFn: async () => {
      const choisi = await choisirTableur(t("import.filtre"));
      if (choisi === null) return null;
      return { chemin: choisi, analyse: await appeler("personnel_import_analyser", { chemin: choisi }) };
    },
    onSuccess: (resultat) => {
      if (resultat === null) return;
      setChemin(resultat.chemin);
      setAnalyse(resultat.analyse);
    },
  });
  const application = useMutation({
    mutationFn: () => appeler("personnel_import_appliquer", { chemin: chemin ?? "" }),
    onSuccess: (resultat) => {
      setBilan(resultat);
      for (const cle of ["personnel", "enseignants", "sauvegardes"]) void client.invalidateQueries({ queryKey: [cle] });
    },
  });
  if (bilan !== null) {
    return (
      <Section titre={t("import_personnel.termine")}>
        <p role="status" className="text-sm text-encre">
          {t("import_personnel.bilan", { importes: bilan.importes, ignorees: bilan.ignorees })}
        </p>
        <div>
          <Bouton onClick={surTermine}>{t("import.fermer")}</Bouton>
        </div>
      </Section>
    );
  }
  return (
    <Section titre={t("import_personnel.titre")}>
      <p className="text-sm text-muet">{t("import_personnel.explication")}</p>
      <p className="text-sm text-muet">
        {t("import_personnel.categories", { categories: categories.filter((c) => c.active).map((c) => c.libelle).join(", ") })}
      </p>
      <div className="flex flex-wrap items-center gap-3">
        <Bouton variante="secondaire" disabled={ecritureModele.isPending} onClick={() => ecritureModele.mutate()}>
          {t("import_personnel.modele")}
        </Bouton>
        <Bouton variante={analyse === null ? "principal" : "secondaire"} disabled={lecture.isPending} onClick={() => lecture.mutate()}>
          {t("import_personnel.choisir")}
        </Bouton>
        <Bouton variante="secondaire" onClick={surTermine}>
          {t("commun.annuler")}
        </Bouton>
      </div>
      {modele === null ? null : <p role="status" className="text-sm text-encre">{t("import_personnel.modele_ecrit", { chemin: modele })}</p>}
      <MessageErreur erreur={ecritureModele.error ?? lecture.error} />
      {analyse === null ? null : (
        <>
          <p role="status" className="text-sm text-encre">
            {t("import_personnel.analyse", { pretes: analyse.pretes, lignes: analyse.lignes.length })}
          </p>
          <div className="max-h-96 overflow-auto">
            <table className="w-full text-left font-condensee text-sm">
              <thead>
                <tr className="text-muet">
                  {(["import.ligne", "personne.prenoms", "personne.nom", "personnel.categorie", "personnel.fonction", "import.motifs"] as const).map((cle) => (
                    <th key={cle} className="px-2 py-1 font-normal">{t(cle)}</th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {analyse.lignes.map((l) => {
                  const motifs = [...l.erreurs.map((e) => texteDeCle(e)), ...(l.doublon ? [t("import_personnel.deja_au_registre")] : [])];
                  return (
                    <tr key={l.numero} className={`border-b border-muet/30 ${l.erreurs.length > 0 ? "text-rouge" : l.doublon ? "text-ocre" : "text-encre"}`}>
                      <td className="px-2 py-1">{String(l.numero)}</td>
                      <td className="px-2 py-1">{l.prenoms}</td>
                      <td className="px-2 py-1">{l.nom}</td>
                      <td className="px-2 py-1">{l.categorie}</td>
                      <td className="px-2 py-1">{l.fonction}</td>
                      <td className="px-2 py-1">{motifs.join(" · ")}</td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
          <MessageErreur erreur={application.error} />
          <div>
            <Bouton disabled={analyse.pretes === 0 || application.isPending} onClick={() => application.mutate()}>
              {t("import_personnel.importer", { nombre: analyse.pretes })}
            </Bouton>
          </div>
        </>
      )}
    </Section>
  );
}
