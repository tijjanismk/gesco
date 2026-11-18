import { useMutation, useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { choisirClasseur } from "../../api/fichiers";
import { ApercuPdf } from "../../composants/ApercuPdf";
import { Bouton } from "../../composants/Bouton";
import { CaseACocher, Selection } from "../../composants/Champ";
import { MessageErreur } from "../../composants/Messages";
import { dateCivile, francs, t } from "../../i18n";
import type { DemandeDuEtat, LigneDuEtat, TotauxDuEtat } from "../../types/gesco";

const TOUTES = "";

/** Onglet « Ce que doit l'État » : l'état des sommes dues par l'État à l'écran, regroupé par
 *  classe avec sous-totaux et total général, les versements de l'année et le non réparti ;
 *  « Imprimer » (PDF à joindre à une réclamation) et « Excel » (mêmes colonnes). */
export function DuEtat({ anneeId }: { anneeId: string }) {
  const [classe, setClasse] = useState(TOUTES);
  const [serie, setSerie] = useState(TOUTES);
  const [seulementDus, setSeulementDus] = useState(false);
  const filtres: DemandeDuEtat = {
    annee_id: anneeId,
    classe_id: classe === TOUTES ? null : classe,
    serie_id: serie === TOUTES ? null : serie,
    seulement_dus: seulementDus,
  };
  const etat = useQuery({
    queryKey: ["du-etat", anneeId, classe, serie, seulementDus],
    queryFn: () => appeler("du_etat_lister", filtres),
  });
  const d = etat.data;
  const [pdf, setPdf] = useState<string | null>(null);
  const [ecrit, setEcrit] = useState<string | null>(null);
  const imprimer = useMutation({
    mutationFn: () => appeler("du_etat_produire", filtres),
    onSuccess: (r) => {
      setEcrit(null);
      setPdf(r.document?.pdf ?? null);
    },
  });
  const excel = useMutation({
    mutationFn: async () => {
      const chemin = await choisirClasseur(t("du_etat.nom_fichier", { annee: d?.annee ?? "" }), t("commun.classeur_excel"));
      return chemin === null ? null : appeler("du_etat_produire", { ...filtres, chemin });
    },
    onSuccess: (r) => {
      if (r !== null) setEcrit(r.fichier?.chemin ?? null);
    },
  });
  return (
    <div className="flex flex-col gap-3">
      <MessageErreur erreur={etat.error} />
      <div className="flex flex-wrap items-end gap-3">
        <Selection
          id="du-etat-classe"
          libelle={t("pec.classe")}
          valeur={classe}
          surChangement={setClasse}
          options={[{ valeur: TOUTES, libelle: t("pec.toutes_classes") }, ...(d?.classes_filtre ?? []).map((c) => ({ valeur: c.id, libelle: c.libelle }))]}
        />
        <Selection
          id="du-etat-serie"
          libelle={t("du_etat.serie")}
          valeur={serie}
          surChangement={setSerie}
          options={[{ valeur: TOUTES, libelle: t("du_etat.toutes_series") }, ...(d?.series_filtre ?? []).map((s) => ({ valeur: s.id, libelle: s.libelle }))]}
        />
        <CaseACocher id="du-etat-dus" libelle={t("du_etat.seulement_dus")} coche={seulementDus} surChangement={setSeulementDus} />
      </div>
      {d === undefined ? null : (
        <>
          <div className="flex flex-col gap-1 text-sm text-encre">
            <p className="font-medium">
              {t("du_etat.totaux", {
                nombre: d.totaux.nombre,
                attendu: francs(d.totaux.attendu),
                recu: francs(d.totaux.recu),
                reste: francs(d.totaux.reste),
                echu: francs(d.totaux.echu),
              })}
            </p>
            <p className="text-muet">{t("du_etat.arrete_au", { date: dateCivile(d.arrete_au) })}</p>
            {d.totaux.recu_en_plus === 0 ? null : <p className="text-muet">{t("pec.totaux_recu_en_plus", { montant: francs(d.totaux.recu_en_plus) })}</p>}
            {d.annee_close ? <p className="text-muet">{t("du_etat.annee_close")}</p> : null}
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <Bouton variante="secondaire" disabled={imprimer.isPending} onClick={() => imprimer.mutate()}>
              {t("du_etat.imprimer")}
            </Bouton>
            <Bouton variante="secondaire" disabled={excel.isPending} onClick={() => excel.mutate()}>
              {t("du_etat.excel")}
            </Bouton>
            {ecrit === null ? null : (
              <p role="status" className="text-sm text-muet">
                {t("commun.export_enregistre", { chemin: ecrit })}
              </p>
            )}
          </div>
          <MessageErreur erreur={imprimer.error ?? excel.error} />
          {d.classes.length === 0 ? (
            <p className="text-sm text-muet">{t("du_etat.aucun")}</p>
          ) : (
            <div className="overflow-x-auto">
              <table className="w-full border-collapse font-condensee text-sm tabular-nums">
                <thead>
                  <tr>
                    {[
                      t("pec.eleve"),
                      t("pec.matricule"),
                      t("du_etat.sexe"),
                      t("du_etat.serie"),
                      t("pec.reference"),
                      t("pec.attendu"),
                      t("pec.recu"),
                      t("du_etat.reste"),
                      t("du_etat.echu"),
                    ].map((titre) => (
                      <th key={titre} className="border-b border-muet px-2 py-1 text-left font-medium text-muet">
                        {titre}
                      </th>
                    ))}
                  </tr>
                </thead>
                {d.classes.map((c) => (
                  <tbody key={c.classe_id}>
                    <tr>
                      <th colSpan={9} scope="colgroup" className="border-b border-muet/60 px-2 pb-1 pt-3 text-left font-semibold text-encre">
                        {c.classe}
                      </th>
                    </tr>
                    {c.lignes.map((l) => (
                      <LigneEleve key={l.inscription_id} l={l} />
                    ))}
                    <LigneTotal libelle={t("du_etat.sous_total", { classe: c.classe, nombre: c.totaux.nombre })} totaux={c.totaux} />
                  </tbody>
                ))}
                <tbody>
                  <LigneTotal libelle={t("du_etat.total_general", { nombre: d.totaux.nombre })} totaux={d.totaux} />
                </tbody>
              </table>
            </div>
          )}
          <div className="flex flex-col gap-1 text-sm text-encre">
            <p className="font-medium">{t("du_etat.versements")}</p>
            {d.versements.length === 0 ? (
              <p className="text-muet">{t("du_etat.aucun_versement")}</p>
            ) : (
              <ul className="flex flex-col gap-0.5">
                {d.versements.map((v) => (
                  <li key={v.versement_etat_id}>
                    {t(v.avant_gesco ? "du_etat.versement_avant_gesco" : "du_etat.versement", {
                      date: dateCivile(v.date),
                      reference: v.reference,
                      montant: francs(v.montant),
                    })}
                  </li>
                ))}
              </ul>
            )}
            <p>{t("du_etat.non_reparti", { verse: francs(d.total_verse), montant: francs(d.non_reparti) })}</p>
          </div>
          {pdf === null ? null : <ApercuPdf base64={pdf} />}
        </>
      )}
    </div>
  );
}

function LigneEleve({ l }: { l: LigneDuEtat }) {
  const cellule = "border-b border-muet/30 px-2 py-1 text-encre";
  return (
    <tr>
      <td className={cellule}>
        {l.eleve}
        {l.pris_en_charge ? null : <span className="text-muet"> · {t("du_etat.decoche")}</span>}
      </td>
      <td className={cellule}>{l.matricule}</td>
      <td className={cellule}>{l.sexe}</td>
      <td className={cellule}>{l.serie}</td>
      <td className={cellule}>{l.reference}</td>
      <td className={cellule}>{francs(l.attendu)}</td>
      <td className={cellule}>{francs(l.recu)}</td>
      <td className={`${cellule} font-medium`}>{francs(l.reste)}</td>
      <td className={cellule}>{l.echu === 0 ? "" : francs(l.echu)}</td>
    </tr>
  );
}

function LigneTotal({ libelle, totaux }: { libelle: string; totaux: TotauxDuEtat }) {
  const cellule = "border-b border-muet/60 px-2 py-1 font-semibold text-encre";
  return (
    <tr>
      <td colSpan={5} className={cellule}>
        {libelle}
      </td>
      <td className={cellule}>{francs(totaux.attendu)}</td>
      <td className={cellule}>{francs(totaux.recu)}</td>
      <td className={cellule}>{francs(totaux.reste)}</td>
      <td className={cellule}>{francs(totaux.echu)}</td>
    </tr>
  );
}
