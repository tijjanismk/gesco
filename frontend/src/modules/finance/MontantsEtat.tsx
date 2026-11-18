import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { commandeAutorisee } from "../../app/navigation";
import { Bouton } from "../../composants/Bouton";
import { MessageErreur, Section } from "../../composants/Messages";
import { francs, t } from "../../i18n";
import type { LigneMontantEtat, MontantEtatSaisi } from "../../types/gesco";
import { type GesteEnCours, PanneauGeste } from "./PriseEnChargeEtat";
import { lireSaisieReprise } from "./reprise";

function cleLigne(l: Pick<LigneMontantEtat, "niveau_id" | "serie_id">): string {
  return `${l.niveau_id}|${l.serie_id ?? ""}`;
}

/** Montant attendu de l'État par niveau et série (réglages de la finance) : une case par ligne,
 *  vide = les frais de l'école. Visible seulement si l'école a une classe du lycée ou du
 *  technique. Changer le tableau ne change pas les élèves déjà cochés : « Appliquer aux élèves
 *  déjà cochés » le propose ensuite, avec l'aperçu. */
export function MontantsEtat({ anneeId, permissions }: { anneeId: string; permissions: readonly string[] }) {
  const client = useQueryClient();
  const cle = ["montants-etat", anneeId];
  const donnees = useQuery({ queryKey: cle, queryFn: () => appeler("montants_etat_lire", { annee_id: anneeId }) });
  const regle = commandeAutorisee("montants_etat_regler", permissions);
  const [saisies, setSaisies] = useState<Record<string, string>>({});
  const [geste, setGeste] = useState<GesteEnCours | null>(null);
  const d = donnees.data;
  const changees: MontantEtatSaisi[] = (d?.lignes ?? []).flatMap((l) => {
    const texte = saisies[cleLigne(l)];
    if (texte === undefined) return [];
    const lu = lireSaisieReprise(texte);
    if (lu.sorte === "illisible") return [];
    const montant = lu.sorte === "vide" ? null : lu.montant;
    return montant === l.montant ? [] : [{ niveau_id: l.niveau_id, serie_id: l.serie_id, montant }];
  });
  const illisible = (d?.lignes ?? []).some((l) => {
    const texte = saisies[cleLigne(l)];
    return texte !== undefined && lireSaisieReprise(texte).sorte === "illisible";
  });
  const enregistrement = useMutation({
    mutationFn: () => appeler("montants_etat_regler", { annee_id: anneeId, lignes: changees, lue_le: d?.lue_le ?? null }),
    onSuccess: (m) => {
      client.setQueryData(cle, m);
      setSaisies({});
      void client.invalidateQueries({ queryKey: ["prises-en-charge"] });
    },
  });
  if (d === undefined) return <MessageErreur erreur={donnees.error} />;
  if (d.lignes.length === 0) return null;
  const aChanger = d.inscriptions_a_changer.length;
  return (
    <Section titre={t("pec.montants.titre")}>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse font-condensee text-sm tabular-nums">
          <thead>
            <tr className="text-left text-muet">
              {(["pec.montants.niveau", "pec.montants.montant", "pec.montants.frais_ecole", "pec.montants.coches"] as const).map((c) => (
                <th key={c} className="border-b border-muet px-2 py-1 font-medium">{t(c)}</th>
              ))}
            </tr>
          </thead>
          <tbody>
            {d.lignes.map((l) => {
              const k = cleLigne(l);
              const texte = saisies[k] ?? (l.montant === null ? "" : String(l.montant));
              const libelle = l.serie === null ? l.niveau : `${l.niveau} · ${l.serie}`;
              return (
                <tr key={k} className="border-b border-muet/30 text-encre">
                  <td className="px-2 py-1">{libelle}</td>
                  <td className="px-2 py-1">
                    {regle ? (
                      <div className="flex flex-col">
                        <input
                          aria-label={t("pec.montants.montant_de", { niveau: libelle })}
                          className="w-32 rounded border border-muet bg-surface px-2 py-0.5 text-right text-primaire focus:outline-2 focus:outline-primaire"
                          inputMode="numeric"
                          autoComplete="off"
                          placeholder={t("pec.montants.vide")}
                          value={texte}
                          onChange={(e) => setSaisies({ ...saisies, [k]: e.target.value })}
                        />
                        {lireSaisieReprise(texte).sorte === "illisible" ? <span className="text-xs text-rouge">{t("pec.illisible")}</span> : null}
                      </div>
                    ) : l.montant === null ? (
                      t("pec.montants.vide")
                    ) : (
                      francs(l.montant)
                    )}
                  </td>
                  <td className="px-2 py-1">{francs(l.frais_ecole)}</td>
                  <td className="px-2 py-1">
                    {l.eleves_coches === 0 ? "" : String(l.eleves_coches)}
                    {l.eleves_a_changer === 0 ? "" : ` · ${t("pec.montants.a_changer", { nombre: l.eleves_a_changer })}`}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
      <p className="text-sm text-muet">{t("pec.montants.aide")}</p>
      <MessageErreur erreur={enregistrement.error} surRelire={() => void donnees.refetch()} />
      <div className="flex flex-wrap gap-2">
        {regle ? (
          <Bouton onClick={() => enregistrement.mutate()} disabled={enregistrement.isPending || changees.length === 0 || illisible}>
            {t("pec.montants.enregistrer")}
          </Bouton>
        ) : null}
        {aChanger > 0 && commandeAutorisee("prise_en_charge_regler", permissions) && geste === null ? (
          <Bouton
            variante="secondaire"
            onClick={() => setGeste({ sorte: "appliquer", lignes: d.inscriptions_a_changer.map((id) => ({ inscription_id: id, eleve: "", lue_le: null })) })}
          >
            {t("pec.montants.appliquer", { nombre: aChanger })}
          </Bouton>
        ) : null}
      </div>
      {geste === null ? null : (
        <PanneauGeste
          geste={geste}
          surFini={() => {
            setGeste(null);
            void donnees.refetch();
          }}
          surAnnuler={() => setGeste(null)}
        />
      )}
    </Section>
  );
}
