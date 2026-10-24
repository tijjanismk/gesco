import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { Bouton } from "../../composants/Bouton";
import { Selection } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { t } from "../../i18n";
import type { ConduiteClasse, DecompteConduite, LigneConduite, Retraits } from "../../types/gesco";
import { moyenne, points } from "../pedagogie/saisies";

function valeur(c: number | null): string {
  return c === null ? "—" : moyenne(c);
}

/** Ce qui pèse sur la conduite, en peu de mots : « 1 avertissement · 3 demi-journées ». */
function decompte(d: DecompteConduite): string {
  const morceaux = [
    d.avertissements > 0 ? t("conduite.decompte.avertissements", { n: d.avertissements }) : null,
    d.blames > 0 ? t("conduite.decompte.blames", { n: d.blames }) : null,
    d.retenues > 0 ? t("conduite.decompte.retenues", { n: d.retenues }) : null,
    d.convocations > 0 ? t("conduite.decompte.convocations", { n: d.convocations }) : null,
    d.exclusions > 0 ? t("conduite.decompte.exclusions", { n: d.exclusions }) : null,
    d.absences > 0 ? t("conduite.decompte.absences", { n: d.absences }) : null,
    d.retards > 0 ? t("conduite.decompte.retards", { n: d.retards }) : null,
  ].filter((m) => m !== null);
  return morceaux.length === 0 ? t("conduite.decompte.rien") : morceaux.join(" · ");
}

function bareme(r: Retraits, b: number): string {
  return t("conduite.aide_calculee", {
    bareme: points(b),
    avertissement: points(r.avertissement),
    blame: points(r.blame),
    retenue: points(r.retenue),
    convocation: points(r.convocation),
    exclusion: points(r.exclusion),
    absence: points(r.absence),
    retard: points(r.retard),
  });
}

/** Note de conduite (2e cycle, lycée, technique) : la direction ou le surveillant la saisit, ou
 *  GESCO la calcule d'après la discipline et les absences, et elle se corrige avec un motif. Le
 *  calcul des bulletins la reprend. */
export function Conduite() {
  const classes = useQuery({ queryKey: ["conduite-classes"], queryFn: () => appeler("conduite_classes_lire", null) });
  const [classeChoisie, setClasseChoisie] = useState<string | null>(null);
  const [periodeChoisie, setPeriodeChoisie] = useState<string | null>(null);
  const liste = classes.data?.classes ?? [];
  const classe = liste.find((c) => c.classe_id === classeChoisie) ?? liste[0];
  const periode = classe?.periodes.find((p) => p.periode_id === periodeChoisie) ?? classe?.periodes[0];
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("conduite.titre")}</h1>
      <MessageErreur erreur={classes.error} />
      {classes.data === undefined ? null : classe === undefined ? (
        <p className="text-sm text-muet">{t("conduite.aucune_classe")}</p>
      ) : (
        <>
          <div className="flex flex-wrap items-end gap-3">
            <Selection id="conduite-classe" libelle={t("discipline.classe")} valeur={classe.classe_id} surChangement={setClasseChoisie} options={liste.map((c) => ({ valeur: c.classe_id, libelle: c.libelle }))} />
            <Selection id="conduite-periode" libelle={t("notes.periode")} valeur={periode?.periode_id ?? ""} surChangement={setPeriodeChoisie} options={classe.periodes.map((p) => ({ valeur: p.periode_id, libelle: p.libelle }))} />
          </div>
          {periode === undefined ? null : <ConduiteDeLaClasse key={`${classe.classe_id}-${periode.periode_id}`} classeId={classe.classe_id} periodeId={periode.periode_id} />}
        </>
      )}
    </div>
  );
}

function ConduiteDeLaClasse({ classeId, periodeId }: { classeId: string; periodeId: string }) {
  const client = useQueryClient();
  const cle = ["conduite", classeId, periodeId];
  const lecture = useQuery({ queryKey: cle, queryFn: () => appeler("conduite_lire", { classe_id: classeId, periode_id: periodeId }) });
  const apres = (d: ConduiteClasse) => client.setQueryData(cle, d);
  const c = lecture.data;
  if (c === undefined) return <MessageErreur erreur={lecture.error} />;
  const calculee = c.mode === "automatique";
  return (
    <Section titre={t(calculee ? "conduite.mode.automatique" : "conduite.mode.manuelle")}>
      <p className="text-sm text-muet">{calculee ? bareme(c.retraits, c.bareme) : t("conduite.aide_saisie", { bareme: points(c.bareme) })}</p>
      <p className="text-sm text-muet">
        {c.comptee ? t("conduite.comptee_coefficient", { coefficient: c.coefficient }) : t("conduite.a_part")} {t("conduite.recalculer")}
      </p>
      {c.verrouillee ? <p className="text-sm text-ocre">{t("conduite.verrouillee")}</p> : null}
      {c.lignes.length === 0 ? (
        <p className="text-sm text-muet">{t("conduite.aucun_eleve")}</p>
      ) : (
        <table className="w-full text-left text-sm">
          <thead>
            <tr className="text-muet">
              <th className="px-2 py-1 font-normal">{t("discipline.eleve")}</th>
              {calculee ? <th className="px-2 py-1 font-normal">{t("conduite.ce_qui_pese")}</th> : null}
              {calculee ? <th className="px-2 py-1 font-normal">{t("conduite.calculee")}</th> : null}
              <th className="px-2 py-1 font-normal">{t("conduite.note")}</th>
              {c.verrouillee ? null : <th className="px-2 py-1 font-normal">{t(calculee ? "conduite.corriger" : "conduite.saisir")}</th>}
            </tr>
          </thead>
          <tbody>
            {c.lignes.map((l, i) => (
              <LigneEleve key={l.inscription_id} ligne={l} rang={i + 1} conduite={c} classeId={classeId} periodeId={periodeId} surFait={apres} />
            ))}
          </tbody>
        </table>
      )}
    </Section>
  );
}

function LigneEleve({ ligne, rang, conduite, classeId, periodeId, surFait }: { ligne: LigneConduite; rang: number; conduite: ConduiteClasse; classeId: string; periodeId: string; surFait: (d: ConduiteClasse) => void }) {
  const calculee = conduite.mode === "automatique";
  const [note, setNote] = useState("");
  const [motif, setMotif] = useState("");
  const noter = useMutation({
    mutationFn: () => appeler("conduite_noter", { classe_id: classeId, periode_id: periodeId, inscription_id: ligne.inscription_id, note, motif }),
    onSuccess: (d) => {
      setNote("");
      setMotif("");
      surFait(d);
    },
  });
  const retirer = useMutation({
    mutationFn: () => appeler("conduite_retirer", { classe_id: classeId, periode_id: periodeId, inscription_id: ligne.inscription_id, motif }),
    onSuccess: (d) => {
      setMotif("");
      surFait(d);
    },
  });
  const s = ligne.saisie;
  return (
    <tr className="border-b border-muet/30 align-top">
      <td className="px-2 py-1 text-encre">
        {ligne.eleve}
        <span className="block text-xs text-muet">{ligne.matricule}</span>
      </td>
      {calculee ? <td className="px-2 py-1 text-encre">{ligne.decompte === null ? "—" : decompte(ligne.decompte)}</td> : null}
      {calculee ? <td className="px-2 py-1 text-encre">{valeur(ligne.calculee)}</td> : null}
      <td className="px-2 py-1 text-encre">
        <strong>{valeur(ligne.note)}</strong>
        {s === null ? null : <span className="block text-xs text-muet">{s.motif === "" ? t("conduite.saisie") : t("conduite.saisie_motif", { motif: s.motif })}</span>}
      </td>
      {conduite.verrouillee ? null : (
        <td className="px-2 py-1">
          <div className="flex flex-wrap items-center gap-2">
            <input
              id={`conduite-note-${rang}`}
              aria-label={t("conduite.note_de", { eleve: ligne.eleve })}
              inputMode="decimal"
              className="w-20 rounded border border-muet bg-surface px-2 py-1 text-primaire"
              value={note}
              onChange={(e) => setNote(e.target.value)}
            />
            <input
              id={`conduite-motif-${rang}`}
              aria-label={t("conduite.motif_de", { eleve: ligne.eleve })}
              placeholder={t(calculee ? "conduite.motif_obligatoire_court" : "conduite.motif_facultatif")}
              className="w-56 rounded border border-muet bg-surface px-2 py-1 text-primaire"
              value={motif}
              onChange={(e) => setMotif(e.target.value)}
            />
            <Bouton variante="secondaire" onClick={() => noter.mutate()} disabled={note.trim() === "" || noter.isPending}>
              {t("commun.enregistrer")}
            </Bouton>
            {s === null ? null : (
              <Bouton variante="secondaire" onClick={() => retirer.mutate()} disabled={motif.trim() === "" || retirer.isPending}>
                {t(calculee ? "conduite.retirer_correction" : "conduite.retirer")}
              </Bouton>
            )}
          </div>
          <MessageErreur erreur={noter.error ?? retirer.error} />
        </td>
      )}
    </tr>
  );
}
