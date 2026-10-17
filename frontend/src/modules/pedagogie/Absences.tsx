import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { type KeyboardEvent, useRef, useState } from "react";

import { appeler } from "../../api";
import { commandeAutorisee } from "../../app/navigation";
import { Bouton } from "../../composants/Bouton";
import { CaseACocher, Champ, Selection } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { type Cle, date as dateDuJour, dateCivile, heure, t, texteDeCle } from "../../i18n";
import type { Absence, AbsenceAffichee, CleAbsence, DemiJournee, LigneReleve } from "../../types/gesco";
import { ReleveParSeance } from "./ReleveParSeance";
import { lireAbsence } from "./saisies";
import { aujourdHui } from "../../app/aujourdhui";

type Onglet = "releve" | "periode";

const ONGLETS: readonly [Onglet, Cle][] = [
  ["releve", "absences.onglet.releve"],
  ["periode", "absences.onglet.periode"],
];

const DEMI_JOURNEES: Record<DemiJournee, Cle> = {
  matin: "absences.matin",
  apres_midi: "absences.apres_midi",
};

/** Motifs et pièces les plus courants, proposés au clavier ; le texte reste libre. */
const MOTIFS = ["Maladie", "Décès dans la famille", "Événement familial", "Convocation officielle", "Voyage"];
const PIECES = ["Mot des parents", "Certificat médical", "Convocation"];

/** Ce que la case affiche pour une absence enregistrée : rien, « a », « r » ou « r15 ». */
function texteAbsence(absence: Absence | null): string {
  if (absence === null) return "";
  if (absence.nature === "absence") return "a";
  return absence.minutes === null ? "r" : `r${absence.minutes}`;
}

function cle(inscription: string, demi: DemiJournee): string {
  return `${inscription}:${demi}`;
}

/** Absences et retards (SPEC_PHASE_4 §6, T-37) : le relevé du jour d'une classe, au clavier, et
 *  les absences de la période, à justifier, avec les totaux du bulletin. */
export function Absences() {
  const session = useQuery({ queryKey: ["session"], queryFn: () => appeler("session_courante", null) });
  const annees = useQuery({ queryKey: ["annees"], queryFn: () => appeler("annees_lister", null) });
  const anneeId = annees.data?.find((a) => a.est_courante)?.id ?? null;
  const classes = useQuery({ queryKey: ["classes", anneeId], queryFn: () => appeler("classes_lister", { annee_id: anneeId ?? "" }), enabled: anneeId !== null });
  const pedagogie = useQuery({ queryKey: ["pedagogie", anneeId], queryFn: () => appeler("pedagogie_lire", { id: anneeId ?? "" }), enabled: anneeId !== null });
  const portee = useQuery({ queryKey: ["portee-absences"], queryFn: () => appeler("portee_absences_lire", null) });
  const [onglet, setOnglet] = useState<Onglet>("releve");
  const [classeChoisie, setClasseChoisie] = useState<string | null>(null);
  const [periodeChoisie, setPeriodeChoisie] = useState<string | null>(null);
  const [jour, setJour] = useState(aujourdHui);
  const permissions = session.data?.permissions ?? [];
  const visibles = (classes.data ?? []).filter((c) => portee.data === undefined || portee.data.type === "etablissement" || portee.data.classes.includes(c.classe.id));
  const classe = visibles.find((c) => c.classe.id === classeChoisie) ?? visibles[0];
  const periodes = (pedagogie.data?.periodes ?? []).filter((p) => classe !== undefined && p.periode.niveau_id === classe.classe.niveau_id);
  const enCours = periodes.find((p) => p.periode.debut <= jour && jour <= p.periode.fin);
  const periode = periodes.find((p) => p.periode.id === periodeChoisie) ?? enCours ?? periodes[0];
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("absences.titre")}</h1>
      <MessageErreur erreur={classes.error ?? pedagogie.error ?? portee.error} />
      {classe === undefined ? (
        <p className="text-sm text-muet">{t("absences.aucune_classe")}</p>
      ) : (
        <>
          <div className="flex flex-wrap gap-2">
            {ONGLETS.map(([id, libelle]) => (
              <Bouton key={id} variante={onglet === id ? "principal" : "secondaire"} onClick={() => setOnglet(id)}>
                {t(libelle)}
              </Bouton>
            ))}
          </div>
          <div className="flex flex-wrap items-end gap-3">
            <Selection id="absences-classe" libelle={t("enseignants.classe")} valeur={classe.classe.id} surChangement={setClasseChoisie} options={visibles.map((c) => ({ valeur: c.classe.id, libelle: `${c.classe.libelle} · ${c.niveau}` }))} />
            {onglet === "releve" ? (
              <Champ id="absences-date" type="date" libelle={t("absences.jour")} value={jour} onChange={(e) => setJour(e.target.value)} />
            ) : (
              <Selection id="absences-periode" libelle={t("notes.periode")} valeur={periode?.periode.id ?? ""} surChangement={setPeriodeChoisie} options={periodes.map((p) => ({ valeur: p.periode.id, libelle: p.libelle }))} />
            )}
          </div>
          {onglet === "releve" ? (
            jour === "" ? null : (
              <>
                <ReleveParSeance key={`seances-${classe.classe.id}-${jour}`} classeId={classe.classe.id} jour={jour} />
                <Releve key={`${classe.classe.id}-${jour}`} classeId={classe.classe.id} jour={jour} peutJustifier={commandeAutorisee("absences_justifier", permissions)} />
              </>
            )
          ) : periode === undefined ? null : (
            <AbsencesDeLaPeriode classeId={classe.classe.id} periodeId={periode.periode.id} peutJustifier={commandeAutorisee("absences_justifier", permissions)} />
          )}
        </>
      )}
    </div>
  );
}

function Releve({ classeId, jour, peutJustifier }: { classeId: string; jour: string; peutJustifier: boolean }) {
  const client = useQueryClient();
  const releve = useQuery({ queryKey: ["releve-absences", classeId, jour], queryFn: () => appeler("absences_releve_lire", { classe_id: classeId, date: jour }) });
  const [textes, setTextes] = useState<Record<string, string>>({});
  const cases = useRef<Record<string, HTMLInputElement | null>>({});
  const enregistrement = useMutation({
    mutationFn: (lignes: readonly LigneReleve[]) =>
      appeler("absences_releve_enregistrer", {
        classe_id: classeId,
        date: jour,
        lignes: lignes.map((l) => ({
          inscription_id: l.inscription_id,
          matin: textes[cle(l.inscription_id, "matin")] ?? texteAbsence(l.matin),
          apres_midi: textes[cle(l.inscription_id, "apres_midi")] ?? texteAbsence(l.apres_midi),
        })),
      }),
    onSuccess: () => {
      setTextes({});
      void client.invalidateQueries({ queryKey: ["releve-absences", classeId, jour] });
      void client.invalidateQueries({ queryKey: ["absences-periode", classeId] });
    },
  });
  const r = releve.data;
  if (r === undefined) return <MessageErreur erreur={releve.error} />;
  const ouvert = r.refus === null;
  const texteDe = (l: LigneReleve, demi: DemiJournee) => textes[cle(l.inscription_id, demi)] ?? texteAbsence(demi === "matin" ? l.matin : l.apres_midi);
  const lues = r.lignes.flatMap((l) => (["matin", "apres_midi"] as const).map((d) => lireAbsence(texteDe(l, d))));
  const illisibles = lues.filter((c) => c === null).length;
  const absents = lues.filter((c) => c?.nature === "absence").length;
  const retards = lues.filter((c) => c?.nature === "retard").length;
  const surTouche = (i: number, demi: DemiJournee) => (e: KeyboardEvent<HTMLInputElement>) => {
    const pas = e.key === "Enter" || e.key === "ArrowDown" ? 1 : e.key === "ArrowUp" ? -1 : 0;
    if (pas === 0) return;
    e.preventDefault();
    const suivante = r.lignes[i + pas];
    if (suivante !== undefined) cases.current[cle(suivante.inscription_id, demi)]?.focus();
    else if (e.key === "Enter" && demi === "matin" && r.lignes[0] !== undefined) cases.current[cle(r.lignes[0].inscription_id, "apres_midi")]?.focus();
  };
  return (
    <Section titre={t("absences.releve_du", { classe: r.classe, jour: dateCivile(r.date) })}>
      <p className="text-sm text-muet">{t("absences.aide")}</p>
      {r.refus === null ? null : <p className="text-sm text-ocre">{texteDeCle(r.refus)}</p>}
      <p className="text-sm text-muet">
        {r.fait_le === null ? t("absences.pas_encore_releve") : t("absences.deja_releve", { jour: dateDuJour(r.fait_le), heure: heure(r.fait_le), par: r.fait_par })}
      </p>
      {r.lignes.length === 0 ? (
        <p className="text-sm text-muet">{t("absences.aucun_eleve")}</p>
      ) : (
        <table className="w-full max-w-3xl text-left text-sm">
          <thead>
            <tr className="text-muet">
              <th className="px-2 py-1 font-normal">{t("notes.numero")}</th>
              <th className="px-2 py-1 font-normal">{t("impayes.eleve")}</th>
              <th className="px-2 py-1 font-normal">{t("impayes.matricule")}</th>
              <th className="px-2 py-1 font-normal">{t("absences.matin")}</th>
              <th className="px-2 py-1 font-normal">{t("absences.apres_midi")}</th>
              <th className="px-2 py-1 font-normal" />
            </tr>
          </thead>
          <tbody>
            {r.lignes.map((l, i) => (
              <tr key={l.inscription_id} className="border-b border-muet/30">
                <td className="px-2 py-1 text-muet">{i + 1}</td>
                <td className="px-2 py-1 text-encre">{l.eleve}</td>
                <td className="px-2 py-1 text-muet">{l.matricule}</td>
                {(["matin", "apres_midi"] as const).map((demi) => {
                  const enregistree = demi === "matin" ? l.matin : l.apres_midi;
                  const texte = texteDe(l, demi);
                  const verrouillee = (enregistree?.justifiee === true && !peutJustifier) || l.exclu;
                  return (
                    <td key={demi} className="px-2 py-1">
                      <input
                        id={`${demi === "matin" ? "matin" : "apres-midi"}-${i + 1}`}
                        ref={(el) => {
                          cases.current[cle(l.inscription_id, demi)] = el;
                        }}
                        aria-label={t("absences.case_de", { eleve: l.eleve, moment: t(DEMI_JOURNEES[demi]) })}
                        autoFocus={i === 0 && demi === "matin" && ouvert}
                        disabled={!ouvert || verrouillee}
                        className={`w-20 rounded border bg-surface px-2 py-1 text-primaire ${lireAbsence(texte) === null ? "border-rouge" : "border-muet"}`}
                        value={texte}
                        onChange={(e) => setTextes((x) => ({ ...x, [cle(l.inscription_id, demi)]: e.target.value }))}
                        onKeyDown={surTouche(i, demi)}
                      />
                    </td>
                  );
                })}
                <td className="px-2 py-1 text-sm text-vert">
                  {l.exclu ? <span className="text-ocre">{t("absences.exclu")}</span> : l.matin?.justifiee === true || l.apres_midi?.justifiee === true ? t("absences.justifiee") : null}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      <p role="status" className="text-sm text-encre">
        {t("absences.bilan_jour", { absences: String(absents), retards: String(retards) })}
        {illisibles === 0 ? "" : ` ${t("absences.illisibles", { nombre: String(illisibles) })}`}
      </p>
      {enregistrement.data ? (
        <p role="status" className="text-sm text-vert">
          {t("absences.enregistre", { absences: String(enregistrement.data.absences), retards: String(enregistrement.data.retards) })}
        </p>
      ) : null}
      <MessageErreur erreur={enregistrement.error} />
      <div>
        <Bouton onClick={() => enregistrement.mutate(r.lignes)} disabled={!ouvert || illisibles > 0 || enregistrement.isPending || r.lignes.length === 0}>
          {t("absences.enregistrer")}
        </Bouton>
      </div>
    </Section>
  );
}

function libelleAbsence(a: AbsenceAffichee): string {
  const nature =
    a.absence.nature === "absence"
      ? t("absences.nature.absence")
      : a.absence.minutes === null
        ? t("absences.nature.retard")
        : t("absences.nature.retard_de", { minutes: String(a.absence.minutes) });
  return `${dateCivile(a.absence.date)} · ${t(DEMI_JOURNEES[a.absence.demi_journee])} · ${a.eleve} · ${nature}`;
}

function AbsencesDeLaPeriode({ classeId, periodeId, peutJustifier }: { classeId: string; periodeId: string; peutJustifier: boolean }) {
  const client = useQueryClient();
  const donnees = useQuery({ queryKey: ["absences-periode", classeId, periodeId], queryFn: () => appeler("absences_periode_lire", { classe_id: classeId, periode_id: periodeId }) });
  const [choisies, setChoisies] = useState<readonly CleAbsence[]>([]);
  const [motif, setMotif] = useState("");
  const [piece, setPiece] = useState("");
  const justification = useMutation({
    mutationFn: () => appeler("absences_justifier", { absences: [...choisies], motif, justificatif: piece }),
    onSuccess: () => {
      setChoisies([]);
      setMotif("");
      setPiece("");
      void client.invalidateQueries({ queryKey: ["absences-periode", classeId] });
      void client.invalidateQueries({ queryKey: ["releve-absences", classeId] });
    },
  });
  const d = donnees.data;
  if (d === undefined) return <MessageErreur erreur={donnees.error} />;
  const memeCle = (a: CleAbsence, b: CleAbsence) => a.inscription_id === b.inscription_id && a.date === b.date && a.demi_journee === b.demi_journee;
  const aJustifier = d.absences.filter((a) => !a.absence.justifiee);
  const justifiees = d.absences.filter((a) => a.absence.justifiee);
  return (
    <>
      <Section titre={t("absences.totaux")}>
        <p className="text-sm text-muet">{t("absences.jours_releves", { nombre: String(d.jours_releves) })}</p>
        <table className="w-full max-w-3xl text-left text-sm">
          <thead>
            <tr className="text-muet">
              <th className="px-2 py-1 font-normal">{t("impayes.eleve")}</th>
              <th className="px-2 py-1 font-normal">{t("absences.demi_journees")}</th>
              <th className="px-2 py-1 font-normal">{t("absences.dont_justifiees")}</th>
              <th className="px-2 py-1 font-normal">{t("absences.retards")}</th>
            </tr>
          </thead>
          <tbody>
            {d.totaux.map((x) => (
              <tr key={x.inscription_id} className="border-b border-muet/30">
                <td className="px-2 py-1 text-encre">{x.eleve}</td>
                <td className="px-2 py-1 text-encre">{x.totaux.demi_journees}</td>
                <td className="px-2 py-1 text-encre">{x.totaux.justifiees}</td>
                <td className="px-2 py-1 text-encre">
                  {x.totaux.minutes_de_retard === 0 ? x.totaux.retards : t("absences.retards_minutes", { retards: String(x.totaux.retards), minutes: String(x.totaux.minutes_de_retard) })}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </Section>
      <Section titre={t("absences.a_justifier", { nombre: String(aJustifier.length) })}>
        {aJustifier.length === 0 ? <p className="text-sm text-muet">{t("absences.rien_a_justifier")}</p> : null}
        <ul className="flex flex-col gap-1">
          {aJustifier.map((a, i) => {
            const c: CleAbsence = { inscription_id: a.absence.inscription_id, date: a.absence.date, demi_journee: a.absence.demi_journee };
            const coche = choisies.some((x) => memeCle(x, c));
            return (
              <li key={`${c.inscription_id}-${c.date}-${c.demi_journee}`}>
                {peutJustifier ? (
                  <CaseACocher id={`a-justifier-${i + 1}`} libelle={libelleAbsence(a)} coche={coche} surChangement={(oui) => setChoisies((x) => (oui ? [...x, c] : x.filter((y) => !memeCle(y, c))))} />
                ) : (
                  <span className="text-sm text-encre">{libelleAbsence(a)}</span>
                )}
              </li>
            );
          })}
        </ul>
        {peutJustifier && aJustifier.length > 0 ? (
          <div className="flex flex-wrap items-end gap-3">
            <Champ id="justification-motif" libelle={t("absences.motif")} list="motifs-absence" value={motif} onChange={(e) => setMotif(e.target.value)} />
            <Champ id="justification-piece" libelle={t("absences.piece")} list="pieces-absence" value={piece} onChange={(e) => setPiece(e.target.value)} />
            <datalist id="motifs-absence">
              {MOTIFS.map((m) => (
                <option key={m} value={m} />
              ))}
            </datalist>
            <datalist id="pieces-absence">
              {PIECES.map((m) => (
                <option key={m} value={m} />
              ))}
            </datalist>
            <Bouton onClick={() => justification.mutate()} disabled={choisies.length === 0 || motif.trim() === "" || justification.isPending}>
              {t("absences.justifier", { nombre: String(choisies.length) })}
            </Bouton>
          </div>
        ) : null}
        {justification.data === undefined ? null : <p role="status" className="text-sm text-vert">{t("absences.justifiees_ok", { nombre: String(justification.data) })}</p>}
        <MessageErreur erreur={justification.error} />
      </Section>
      {justifiees.length === 0 ? null : (
        <Section titre={t("absences.deja_justifiees")}>
          <ul className="flex flex-col gap-1 text-sm text-encre">
            {justifiees.map((a) => (
              <li key={`${a.absence.inscription_id}-${a.absence.date}-${a.absence.demi_journee}`}>
                {`${libelleAbsence(a)} — ${a.absence.motif}${a.absence.justificatif === "" ? "" : ` (${a.absence.justificatif})`}`}
              </li>
            ))}
          </ul>
        </Section>
      )}
    </>
  );
}
