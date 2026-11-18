import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { choisirClasseur, choisirTableur } from "../../api/fichiers";
import { Bouton } from "../../composants/Bouton";
import { useEtreSeul } from "../../composants/EtreSeul";
import { MessageErreur, Section } from "../../composants/Messages";
import { Tableau } from "../../composants/Tableau";
import { francs, t, texteDeCle } from "../../i18n";
import type { ApercuRepartitionEtat, RapportRepartitionEtat } from "../../types/gesco";
import { COULEURS, messagesLigne, SORTS } from "./ImportReprises";
import { APRES_PRISE_EN_CHARGE, importRepartition } from "./prise_en_charge";

/** Écrit le modèle prérempli des élèves pris en charge de l'année ; avec un versement, la colonne
 *  « Montant versé » reprend sa répartition actuelle. */
function useModele(anneeId: string | null, versementId: string | null) {
  const [ecrit, setEcrit] = useState<string | null>(null);
  const ecriture = useMutation({
    mutationFn: async () => {
      const destination = await choisirClasseur(t("import_repartition_etat.nom_modele"), t("commun.classeur_excel"));
      return destination === null ? null : appeler("versement_etat_modele", { chemin: destination, annee_id: anneeId, versement_id: versementId });
    },
    onSuccess: (r) => setEcrit(r?.chemin ?? null),
  });
  return { ecriture, ecrit };
}

/** « Télécharger le modèle » seul, pour un versement pas encore enregistré. */
export function ModeleRepartitionEtat({ anneeId }: { anneeId: string }) {
  const { ecriture, ecrit } = useModele(anneeId, null);
  return (
    <div className="flex flex-col gap-1">
      <p className="text-sm text-muet">{t("vetat.fichier_apres")}</p>
      <div>
        <Bouton variante="secondaire" disabled={ecriture.isPending} onClick={() => ecriture.mutate()}>
          {t("import_repartition_etat.modele")}
        </Bouton>
      </div>
      {ecrit === null ? null : <p role="status" className="text-sm text-encre">{t("import_repartition_etat.modele_ecrit", { chemin: ecrit })}</p>}
      <MessageErreur erreur={ecriture.error} />
    </div>
  );
}

/** Répartition d'un versement de l'État par fichier : le modèle prérempli, l'aperçu ligne par
 *  ligne et ses totaux, puis l'écriture des lignes valides et en avertissement après une
 *  sauvegarde automatique. Le fichier donne la répartition entière du versement ; les lignes
 *  refusées s'exportent. */
export function ImportRepartitionEtat({ versementId, anneeId, surFini }: { versementId: string; anneeId: string; surFini: () => void }) {
  const client = useQueryClient();
  const [apercu, setApercu] = useState<ApercuRepartitionEtat | null>(null);
  const [rapport, setRapport] = useState<RapportRepartitionEtat | null>(null);
  const { ecriture: ecritureModele, ecrit: modele } = useModele(anneeId, versementId);
  const seul = useEtreSeul("seul.import");
  const lecture = useMutation({
    mutationFn: async () => {
      const choisi = await choisirTableur(t("import.filtre"));
      return choisi === null ? null : appeler("versement_etat_analyser_import", { chemin: choisi, versement_id: versementId });
    },
    onSuccess: (resultat) => {
      if (resultat !== null) {
        setApercu(resultat);
        setRapport(null);
      }
    },
  });
  const application = useMutation({
    mutationFn: () => appeler("versement_etat_importer", { fichier_id: apercu?.fichier_id ?? "", versement_id: versementId, lue_le: apercu?.lue_le ?? null }),
    onSuccess: (resultat) => {
      setRapport(resultat);
      setApercu(null);
      for (const cle of [...APRES_PRISE_EN_CHARGE, "versement-etat", "sauvegardes"]) void client.invalidateQueries({ queryKey: [cle] });
    },
  });
  const effet = apercu === null ? null : importRepartition(apercu);
  return (
    <Section titre={t("import_repartition_etat.titre")}>
      <p className="text-sm text-muet">{t("import_repartition_etat.explication")}</p>
      <div className="flex flex-wrap items-center gap-3">
        <Bouton variante="secondaire" disabled={ecritureModele.isPending} onClick={() => ecritureModele.mutate()}>
          {t("import_repartition_etat.modele")}
        </Bouton>
        <Bouton variante={apercu === null ? "principal" : "secondaire"} disabled={lecture.isPending} onClick={() => lecture.mutate()}>
          {t("import_repartition_etat.choisir")}
        </Bouton>
        <Bouton variante="secondaire" onClick={surFini}>
          {t("import_repartition_etat.fermer")}
        </Bouton>
      </div>
      {modele === null ? null : <p role="status" className="text-sm text-encre">{t("import_repartition_etat.modele_ecrit", { chemin: modele })}</p>}
      <MessageErreur erreur={ecritureModele.error ?? lecture.error} />
      {apercu === null || effet === null ? null : (
        <>
          <p role="status" className="text-sm text-encre">
            {t("import_repartition_etat.resume", {
              fichier: apercu.nom,
              valides: String(apercu.valides),
              avertissements: String(apercu.avertissements),
              erreurs: String(apercu.erreurs),
              ignorees: String(apercu.ignorees),
            })}
          </p>
          <div className="max-h-96 overflow-auto">
            <table className="w-full text-left font-condensee text-sm tabular-nums">
              <thead>
                <tr className="text-muet">
                  {(["import.ligne", "pec.matricule", "pec.eleve", "pec.classe", "pec.reference", "vetat.col.montant", "import_reprises.sort", "import.motifs"] as const).map((cle) => (
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
                    <td className="px-2 py-1">{l.reference}</td>
                    <td className="px-2 py-1">{l.montant === null ? l.saisi : francs(l.montant)}</td>
                    <td className="px-2 py-1">{t(SORTS[l.sort])}</td>
                    <td className="px-2 py-1">{messagesLigne(l)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <p className={`text-sm font-medium ${apercu.refus === null ? "text-encre" : "text-rouge"}`}>
            {t("vetat.pied", { recu: francs(apercu.recu), reparti: francs(apercu.reparti), non_reparti: francs(apercu.non_reparti), nombre: apercu.nombre_eleves })}
          </p>
          {apercu.refus === null ? null : (
            <p className="text-sm text-rouge">
              {apercu.depassement > 0 ? t("import_repartition_etat.depassement", { montant: francs(apercu.depassement) }) : texteDeCle(apercu.refus)}
            </p>
          )}
          {apercu.parts_actuelles === 0 ? null : (
            <p className="text-sm text-ocre">{t("import_repartition_etat.remplace", { nombre: String(apercu.parts_actuelles), montant: francs(apercu.montant_actuel) })}</p>
          )}
          {apercu.retirees === 0 ? null : (
            <p className="text-sm text-ocre">{t("import_repartition_etat.retirees", { nombre: String(apercu.retirees), montant: francs(apercu.montant_retire) })}</p>
          )}
          <MessageErreur
            erreur={application.error}
            surRelire={() => {
              setApercu(null);
              application.reset();
            }}
          />
          <div>
            <Bouton disabled={!effet.possible || application.isPending} onClick={() => seul.demander(() => application.mutate())}>
              {t("import_repartition_etat.importer", { nombre: String(effet.aEcrire) })}
            </Bouton>
            {seul.dialogue}
          </div>
        </>
      )}
      {rapport === null ? null : (
        <div className="flex flex-col gap-2">
          <p role="status" className="text-sm text-encre">
            {rapport.change
              ? t("import_repartition_etat.bilan", {
                  nombre: String(rapport.nombre_eleves),
                  reparti: francs(rapport.reparti),
                  non_reparti: francs(rapport.non_reparti),
                  ignorees: String(rapport.ignorees),
                  refusees: String(rapport.refusees.length),
                })
              : t("import_repartition_etat.inchange")}
          </p>
          {rapport.refusees.length === 0 ? null : (
            <>
              <p className="text-sm text-muet">{t("import_repartition_etat.refusees_aide")}</p>
              <Tableau
                nomExport="repartition-etat-refusees"
                titre={t("import_repartition_etat.refusees")}
                lignes={rapport.refusees}
                cle={(l) => String(l.numero)}
                colonnes={[
                  { titre: t("import_repartition_etat.colonne.matricule"), valeur: (l) => l.matricule },
                  { titre: t("import_repartition_etat.colonne.nom"), valeur: (l) => l.nom },
                  { titre: t("import_repartition_etat.colonne.prenoms"), valeur: (l) => l.prenoms },
                  { titre: t("import_repartition_etat.colonne.classe"), valeur: (l) => l.classe },
                  { titre: t("import_repartition_etat.colonne.reference"), valeur: (l) => l.reference },
                  { titre: t("import_repartition_etat.colonne.montant"), valeur: (l) => l.saisi },
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
