import { useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { commandeAutorisee } from "../../app/navigation";
import { Bouton } from "../../composants/Bouton";
import { MessageErreur } from "../../composants/Messages";
import { dateCivile, dateHeure, francs, t } from "../../i18n";
import type { SituationInscription } from "../../types/gesco";
import { type GesteEnCours, PanneauGeste, type SorteGeste } from "./PriseEnChargeEtat";

/** « Pris en charge par l'État », dans la section « Frais » de la fiche : la ligne de la case
 *  (référence, montant attendu), Modifier, Décocher, Ne plus attendre et l'historique pour qui en
 *  a le droit ; ce que doit l'État, avec chaque versement reçu ; ce que le parent avait payé sur
 *  la part de l'État et son report. L'à rembourser et « Rembourser » restent sur la ligne du payé
 *  avant GESCO : une seule ligne, un seul bouton. */
export function PriseEnChargeInscription({
  situation,
  eleve,
  permissions,
  surChange,
}: {
  situation: SituationInscription;
  eleve: string;
  permissions: readonly string[];
  surChange: () => void;
}) {
  const facture = situation.facture;
  const version = facture?.prise_en_charge ?? null;
  const inscriptionId = situation.inscription.id;
  const regle = commandeAutorisee("prise_en_charge_regler", permissions);
  const [geste, setGeste] = useState<GesteEnCours | null>(null);
  const [historique, setHistorique] = useState(false);
  const versions = useQuery({
    queryKey: ["pec-historique", inscriptionId],
    queryFn: () => appeler("prise_en_charge_historique", { inscription_id: inscriptionId }),
    enabled: historique,
  });
  const vivante = situation.inscription.statut === "brouillon" || situation.inscription.statut === "validee";
  if (version === null && situation.versements_etat.length === 0 && !(regle && vivante && situation.prise_en_charge_possible && facture !== null)) return null;
  const ouvrir = (sorte: SorteGeste) =>
    setGeste({ sorte, lignes: [{ inscription_id: inscriptionId, eleve: eleve === "" ? situation.classe : eleve, lue_le: null }], montant: sorte === "modifier" ? version?.montant_attendu : undefined });
  const enCours = version?.statut === "en_cours";
  const recu = situation.versements_etat.reduce((s, v) => s + v.montant, 0);
  const resteEtat = facture === null ? 0 : facture.reste_etat + Math.max(0, facture.complement_etat - facture.paye_complement);
  const peutHistorique = commandeAutorisee("prise_en_charge_historique", permissions);
  return (
    <div className="flex flex-col gap-1 border-t border-muet/30 pt-1">
      <div className="flex flex-wrap items-center gap-3">
        <span className="text-encre">
          {version === null
            ? t("pec.fiche.non_coche")
            : enCours
              ? t("pec.fiche.coche", { montant: francs(version.montant_attendu) })
              : t("pec.fiche.terminee", { motif: version.motif })}
          {version === null || version.reference === "" ? "" : ` · ${t("pec.fiche.reference", { reference: version.reference })}`}
          {version === null || version.abandonne === 0 ? "" : ` · ${t("pec.abandonne", { montant: francs(version.abandonne) })}`}
        </span>
        {regle && geste === null ? (
          enCours ? (
            <>
              <Bouton variante="secondaire" onClick={() => ouvrir("modifier")}>{t("pec.fiche.modifier")}</Bouton>
              {vivante ? <Bouton variante="secondaire" onClick={() => ouvrir("decocher")}>{t("pec.fiche.decocher")}</Bouton> : null}
              <Bouton variante="secondaire" onClick={() => ouvrir("abandonner")}>{t("pec.ne_plus_attendre")}</Bouton>
              {version.abandonne > 0 ? <Bouton variante="secondaire" onClick={() => ouvrir("defaire")}>{t("pec.fiche.attendre_de_nouveau")}</Bouton> : null}
            </>
          ) : vivante && situation.prise_en_charge_possible ? (
            <Bouton variante="secondaire" onClick={() => ouvrir("cocher")}>{t("pec.fiche.cocher")}</Bouton>
          ) : null
        ) : null}
        {version !== null && peutHistorique ? (
          <Bouton variante="secondaire" onClick={() => setHistorique(!historique)}>{t(historique ? "paye_avant.masquer_historique" : "paye_avant.historique")}</Bouton>
        ) : null}
      </div>
      {geste === null ? null : (
        <PanneauGeste
          geste={geste}
          surFini={() => {
            setGeste(null);
            surChange();
          }}
          surAnnuler={() => setGeste(null)}
        />
      )}
      {historique ? (
        <ul className="flex flex-col gap-1 pl-3 text-muet">
          <MessageErreur erreur={versions.error} />
          {(versions.data ?? []).map((v) => (
            <li key={v.id}>
              {t(v.statut === "terminee" ? "pec.fiche.version_terminee" : "pec.fiche.version", {
                date: dateHeure(v.cree_le),
                montant: francs(v.montant_attendu),
                abandonne: francs(v.abandonne),
                motif: v.motif === "" ? "—" : v.motif,
              })}
              {v.statut === "remplacee" ? ` · ${t("pec.fiche.remplacee")}` : ""}
            </li>
          ))}
        </ul>
      ) : null}
      {facture === null || (version === null && recu === 0) ? null : (
        <div className="flex flex-col gap-1 rounded border border-muet/30 p-2">
          <p className="font-medium text-encre">{t("pec.fiche.doit_titre")}</p>
          <p className="text-encre">
            {t("pec.fiche.doit", {
              du: francs(facture.part_etat + facture.complement_etat),
              recu: francs(recu),
              reste: francs(resteEtat),
            })}
          </p>
          {facture.complement_etat === 0 ? null : <p className="text-muet">{t("pec.fiche.complement", { montant: francs(facture.complement_etat) })}</p>}
          {situation.versements_etat.map((v) => (
            <p key={v.versement_etat_id} className="text-muet">
              {t("pec.fiche.versement", { date: dateCivile(v.date), reference: v.reference === "" ? "—" : v.reference, montant: francs(v.montant) })}
            </p>
          ))}
          {facture.recu_en_plus_etat === 0 ? null : <p className="text-ocre">{t("pec.fiche.recu_en_plus", { montant: francs(facture.recu_en_plus_etat) })}</p>}
        </div>
      )}
      {facture === null || facture.trop_paye === 0 ? null : (
        <div className="flex flex-col gap-1">
          <p className="text-encre">{t("pec.fiche.trop_paye", { montant: francs(facture.trop_paye) })}</p>
          {facture.trop_paye_reporte === 0 ? null : <p className="text-encre">{t("pec.fiche.reporte", { montant: francs(facture.trop_paye_reporte) })}</p>}
        </div>
      )}
    </div>
  );
}
