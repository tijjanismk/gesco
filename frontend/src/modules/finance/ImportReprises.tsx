import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { choisirClasseur, choisirTableur } from "../../api/fichiers";
import { Bouton } from "../../composants/Bouton";
import { useEtreSeul } from "../../composants/EtreSeul";
import { MessageErreur, Section } from "../../composants/Messages";
import { Tableau } from "../../composants/Tableau";
import { type Cle, francs, t, texteDeCle } from "../../i18n";
import type { ApercuReprises, LigneImportReprise, RapportReprises, SortLigneReprise } from "../../types/gesco";

const SORTS: Record<SortLigneReprise, Cle> = {
  valide: "import_reprises.sort.valide",
  avertissement: "import_reprises.sort.avertissement",
  erreur: "import_reprises.sort.erreur",
  ignoree: "import_reprises.sort.ignoree",
};

const COULEURS: Record<SortLigneReprise, string> = {
  valide: "text-encre",
  avertissement: "text-ocre",
  erreur: "text-rouge",
  ignoree: "text-muet",
};

/** Messages d'une ligne, avec leurs montants. */
export function messagesLigne(l: LigneImportReprise): string {
  return l.messages
    .map((m) => texteDeCle(m.cle, { montant: m.montant === null ? "" : francs(m.montant), autre: m.autre === null ? "" : francs(m.autre) }))
    .join(" · ");
}

/** Import Excel de ce qui était payé avant GESCO : le modèle prérempli, l'aperçu ligne par ligne,
 *  puis l'écriture des lignes valides après une sauvegarde automatique ; les lignes refusées
 *  s'exportent pour être corrigées et réimportées. */
export function ImportReprises({ anneeId, classeId, surFini }: { anneeId: string | null; classeId: string | null; surFini: () => void }) {
  const client = useQueryClient();
  const [apercu, setApercu] = useState<ApercuReprises | null>(null);
  const [rapport, setRapport] = useState<RapportReprises | null>(null);
  const [modele, setModele] = useState<string | null>(null);
  const seul = useEtreSeul("seul.import");
  const ecritureModele = useMutation({
    mutationFn: async () => {
      const destination = await choisirClasseur(t("import_reprises.nom_modele"), t("commun.classeur_excel"));
      return destination === null ? null : appeler("reprises_modele", { chemin: destination, annee_id: anneeId, classe_id: classeId });
    },
    onSuccess: (ecrit) => setModele(ecrit?.chemin ?? null),
  });
  const lecture = useMutation({
    mutationFn: async () => {
      const choisi = await choisirTableur(t("import.filtre"));
      return choisi === null ? null : appeler("reprises_analyser_import", { chemin: choisi, annee_id: anneeId });
    },
    onSuccess: (resultat) => {
      if (resultat !== null) setApercu(resultat);
    },
  });
  const application = useMutation({
    mutationFn: () => appeler("reprises_importer", { fichier_id: apercu?.fichier_id ?? "", annee_id: anneeId }),
    onSuccess: (resultat) => {
      setRapport(resultat);
      setApercu(null);
      for (const cle of ["reprises", "situation", "impayes", "sauvegardes"]) void client.invalidateQueries({ queryKey: [cle] });
    },
  });
  const aEcrire = apercu === null ? 0 : apercu.valides + apercu.avertissements;
  return (
    <Section titre={t("import_reprises.titre")}>
      <p className="text-sm text-muet">{t("import_reprises.explication")}</p>
      <div className="flex flex-wrap items-center gap-3">
        <Bouton variante="secondaire" disabled={ecritureModele.isPending} onClick={() => ecritureModele.mutate()}>
          {t("import_reprises.modele")}
        </Bouton>
        <Bouton variante={apercu === null ? "principal" : "secondaire"} disabled={lecture.isPending} onClick={() => lecture.mutate()}>
          {t("import_reprises.choisir")}
        </Bouton>
        <Bouton variante="secondaire" onClick={surFini}>
          {t("import_reprises.fermer")}
        </Bouton>
      </div>
      {modele === null ? null : <p role="status" className="text-sm text-encre">{t("import_reprises.modele_ecrit", { chemin: modele })}</p>}
      <MessageErreur erreur={ecritureModele.error ?? lecture.error} />
      {apercu === null ? null : (
        <>
          <p role="status" className="text-sm text-encre">
            {t("import_reprises.resume", {
              fichier: apercu.nom,
              valides: String(apercu.valides),
              avertissements: String(apercu.avertissements),
              erreurs: String(apercu.erreurs),
              ignorees: String(apercu.ignorees),
              total: francs(apercu.total),
            })}
          </p>
          <div className="max-h-96 overflow-auto">
            <table className="w-full text-left font-condensee text-sm">
              <thead>
                <tr className="text-muet">
                  {(["import.ligne", "paye_avant.matricule", "paye_avant.eleve", "paye_avant.classe", "paye_avant.paye_avant", "import_reprises.sort", "import.motifs"] as const).map((cle) => (
                    <th key={cle} className="px-2 py-1 font-normal">{t(cle)}</th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {apercu.lignes.map((l) => (
                  <tr key={l.numero} className={`border-b border-muet/30 ${COULEURS[l.sort]}`}>
                    <td className="px-2 py-1">{String(l.numero)}</td>
                    <td className="px-2 py-1">{l.matricule}</td>
                    <td className="px-2 py-1">{l.eleve === "" ? `${l.prenoms} ${l.nom}` : l.eleve}</td>
                    <td className="px-2 py-1">{l.classe}</td>
                    <td className="px-2 py-1">{l.montant === null ? l.saisi : francs(l.montant)}</td>
                    <td className="px-2 py-1">{t(SORTS[l.sort])}</td>
                    <td className="px-2 py-1">{messagesLigne(l)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <MessageErreur erreur={application.error} />
          <div>
            <Bouton disabled={aEcrire === 0 || application.isPending} onClick={() => seul.demander(() => application.mutate())}>
              {t("import_reprises.importer", { nombre: String(aEcrire) })}
            </Bouton>
            {seul.dialogue}
          </div>
        </>
      )}
      {rapport === null ? null : (
        <div className="flex flex-col gap-2">
          <p role="status" className="text-sm text-encre">
            {t("import_reprises.bilan", {
              enregistrees: String(rapport.enregistrees),
              annulees: String(rapport.annulees),
              ignorees: String(rapport.ignorees),
              refusees: String(rapport.refusees.length),
              total: francs(rapport.total),
            })}
          </p>
          {rapport.refusees.length === 0 ? null : (
            <>
              <p className="text-sm text-muet">{t("import_reprises.refusees_aide")}</p>
              <Tableau
                nomExport="paye-avant-gesco-refusees"
                titre={t("import_reprises.refusees")}
                lignes={rapport.refusees}
                cle={(l) => String(l.numero)}
                colonnes={[
                  { titre: t("import_reprises.colonne.matricule"), valeur: (l) => l.matricule },
                  { titre: t("import_reprises.colonne.nom"), valeur: (l) => l.nom },
                  { titre: t("import_reprises.colonne.prenoms"), valeur: (l) => l.prenoms },
                  { titre: t("import_reprises.colonne.classe"), valeur: (l) => l.classe },
                  { titre: t("import_reprises.colonne.montant"), valeur: (l) => l.saisi },
                  { titre: t("import_reprises.colonne.note"), valeur: (l) => l.note },
                  { titre: t("import.motifs"), valeur: messagesLigne },
                ]}
              />
            </>
          )}
        </div>
      )}
    </Section>
  );
}
