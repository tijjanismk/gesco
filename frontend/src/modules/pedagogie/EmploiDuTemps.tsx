import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { ApercuPdf } from "../../composants/ApercuPdf";
import { Bouton } from "../../composants/Bouton";
import { CaseACocher, Champ, Formulaire, Selection } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { type Cle, t } from "../../i18n";
import { CYCLES } from "../scolarite/Cycles";
import type { ClasseEmploi, EmploiDuTemps as Etat, JourneeCycle, NatureConflit, Plage, SeanceAffichee } from "../../types/gesco";

const JOURS: Record<number, Cle> = {
  1: "emploi.jour.1",
  2: "emploi.jour.2",
  3: "emploi.jour.3",
  4: "emploi.jour.4",
  5: "emploi.jour.5",
  6: "emploi.jour.6",
};

const CONFLITS: Record<NatureConflit, Cle> = {
  enseignant: "emploi.conflit.enseignant",
  salle: "emploi.conflit.salle",
  recreation: "emploi.conflit.recreation",
  volume: "emploi.conflit.volume",
};

/** « 08:00 » d'après des minutes depuis minuit. */
function heure(minutes: number): string {
  return `${String(Math.floor(minutes / 60)).padStart(2, "0")}:${String(minutes % 60).padStart(2, "0")}`;
}

/** Minutes depuis minuit d'après « 08:00 » ; `null` si illisible. */
function minutes(texte: string): number | null {
  const m = /^(\d{1,2}):(\d{2})$/.exec(texte.trim());
  if (m === null) return null;
  const h = Number(m[1]);
  const mn = Number(m[2]);
  return h < 24 && mn < 60 ? h * 60 + mn : null;
}

/** Durée lisible : « 1 h 30 », « 45 min ». */
function duree(total: number): string {
  const h = Math.floor(total / 60);
  const mn = total % 60;
  if (h === 0) return `${String(mn)} min`;
  return mn === 0 ? `${String(h)} h` : `${String(h)} h ${String(mn).padStart(2, "0")}`;
}

function plage(p: Plage): string {
  return `${heure(p.debut)} – ${heure(p.fin)}`;
}

/** Emploi du temps (SPEC_PHASE_8 §8) : les séances de chaque classe placées à la main dans sa
 *  journée, les conflits signalés sans refus, l'impression par classe et par enseignant ; la
 *  journée de chaque cycle et l'horaire de canicule. */
export function EmploiDuTemps() {
  const client = useQueryClient();
  const etat = useQuery({ queryKey: ["emploi-du-temps"], queryFn: () => appeler("emploi_du_temps_lire", null) });
  const apres = (e: Etat) => client.setQueryData(["emploi-du-temps"], e);
  const [classeId, setClasseId] = useState<string | null>(null);
  const e = etat.data;
  const classe = e?.classes.find((c) => c.classe_id === classeId) ?? e?.classes[0];
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("emploi.titre")}</h1>
      <MessageErreur erreur={etat.error} />
      {e === undefined ? null : e.classes.length === 0 ? (
        <p className="text-sm text-muet">{t("emploi.sans_classe")}</p>
      ) : (
        <>
          <Selection id="emploi-classe" libelle={t("emploi.classe")} valeur={classe?.classe_id ?? ""} surChangement={setClasseId} options={e.classes.map((c) => ({ valeur: c.classe_id, libelle: c.libelle }))} />
          {classe === undefined ? null : <EmploiDeClasse key={classe.classe_id} e={e} classe={classe} surFait={apres} />}
          <Impressions e={e} classe={classe} />
          {e.gere ? <Journees e={e} surFait={apres} /> : null}
        </>
      )}
    </div>
  );
}

function EmploiDeClasse({ e, classe, surFait }: { e: Etat; classe: ClasseEmploi; surFait: (e: Etat) => void }) {
  const journee = e.journees.find((j) => j.cycle === classe.cycle);
  const siennes = e.seances.filter((s) => s.seance.classe_id === classe.classe_id);
  const [choisie, setChoisie] = useState<SeanceAffichee | null>(null);
  const [tour, setTour] = useState(0);
  const conflitsDe = (id: string) => e.conflits.filter((c) => c.seance_id === id);
  const jours = journee?.jours ?? [1, 2, 3, 4, 5];
  return (
    <Section titre={t("emploi.de_la_classe", { classe: classe.libelle, cycle: t(CYCLES[classe.cycle]) })}>
      {journee === undefined ? null : <p className="text-sm text-muet">{resumeJournee(journee)}</p>}
      <div className="grid gap-2" style={{ gridTemplateColumns: `repeat(${String(jours.length)}, minmax(0, 1fr))` }}>
        {jours.map((j) => (
          <div key={j} className="flex flex-col gap-1">
            <p className="text-sm font-semibold text-encre">{t(JOURS[j] ?? "emploi.jour.6")}</p>
            {siennes
              .filter((s) => s.seance.jour === j)
              .map((s) => {
                const conflits = conflitsDe(s.seance.id);
                return (
                  <button
                    key={s.seance.id}
                    type="button"
                    onClick={() => setChoisie(s)}
                    className={`rounded border p-1 text-left text-xs ${conflits.length > 0 ? "border-ocre" : "border-muet"} ${choisie?.seance.id === s.seance.id ? "bg-fond" : ""}`}
                  >
                    <span className="font-medium text-encre">{plage(s.seance.plage)}</span>
                    <br />
                    {s.matiere}
                    {s.enseignant === "" ? null : (
                      <>
                        <br />
                        <span className="text-muet">{s.enseignant}</span>
                      </>
                    )}
                    {s.seance.salle === "" ? null : <span className="text-muet">{` · ${s.seance.salle}`}</span>}
                    {conflits.map((c) => (
                      <span key={`${c.nature}-${c.avec ?? ""}`} className="block text-ocre">
                        {t(CONFLITS[c.nature])}
                      </span>
                    ))}
                  </button>
                );
              })}
          </div>
        ))}
      </div>
      <ul className="flex flex-col gap-1 text-sm text-encre">
        {classe.matieres.map((m) => (
          <li key={m.matiere_id} className={m.minutes_prevues !== null && m.minutes_placees > m.minutes_prevues ? "text-ocre" : ""}>
            {m.minutes_prevues === null
              ? t("emploi.volume_libre", { matiere: m.libelle, placees: duree(m.minutes_placees) })
              : t("emploi.volume", { matiere: m.libelle, placees: duree(m.minutes_placees), prevues: duree(m.minutes_prevues) })}
          </li>
        ))}
      </ul>
      {e.gere ? (
        <FicheSeance
          key={`${choisie?.seance.id ?? "nouvelle"}-${String(tour)}`}
          e={e}
          classe={classe}
          jours={jours}
          seance={choisie}
          surFait={(x) => {
            setChoisie(null);
            setTour(tour + 1);
            surFait(x);
          }}
        />
      ) : null}
    </Section>
  );
}

function resumeJournee(j: JourneeCycle): string {
  const jours = j.jours.map((d) => t(JOURS[d] ?? "emploi.jour.6").toLowerCase()).join(", ");
  const apresMidi = j.apres_midi === null ? "" : ` · ${plage(j.apres_midi)}`;
  const recreations = j.recreations.map(plage).join(", ");
  return t("emploi.resume_journee", { jours, matin: plage(j.matin), apres_midi: apresMidi, recreations: recreations === "" ? "—" : recreations });
}

function FicheSeance({ e, classe, jours, seance, surFait }: { e: Etat; classe: ClasseEmploi; jours: readonly number[]; seance: SeanceAffichee | null; surFait: (e: Etat) => void }) {
  const premiere = classe.matieres[0];
  const [jour, setJour] = useState(String(seance?.seance.jour ?? jours[0] ?? 1));
  const [debut, setDebut] = useState(seance === null ? "08:00" : heure(seance.seance.plage.debut));
  const [fin, setFin] = useState(seance === null ? "09:00" : heure(seance.seance.plage.fin));
  const [matiere, setMatiere] = useState(seance?.seance.matiere_id ?? premiere?.matiere_id ?? "");
  const [enseignant, setEnseignant] = useState(seance === null ? (premiere?.enseignant_id ?? "") : (seance.seance.enseignant_id ?? ""));
  const [salle, setSalle] = useState(seance?.seance.salle ?? "");
  const d = minutes(debut);
  const f = minutes(fin);
  const placer = useMutation({
    mutationFn: () =>
      appeler("seance_placer", {
        id: seance?.seance.id ?? null,
        classe_id: classe.classe_id,
        jour: Number(jour),
        debut: d ?? 0,
        fin: f ?? 0,
        matiere_id: matiere,
        enseignant_id: enseignant === "" ? null : enseignant,
        salle,
      }),
    onSuccess: surFait,
  });
  const retirer = useMutation({
    mutationFn: () => appeler("seance_retirer", { id: seance?.seance.id ?? "" }),
    onSuccess: surFait,
  });
  return (
    <Formulaire surEnvoi={() => placer.mutate()}>
      <p className="text-sm font-medium text-encre">{seance === null ? t("emploi.nouvelle_seance") : t("emploi.seance_choisie", { horaire: plage(seance.seance.plage), matiere: seance.matiere })}</p>
      <div className="flex flex-wrap items-end gap-3">
        <Selection id="seance-jour" libelle={t("emploi.jour")} valeur={jour} surChangement={setJour} options={jours.map((j) => ({ valeur: String(j), libelle: t(JOURS[j] ?? "emploi.jour.6") }))} />
        <Champ id="seance-debut" type="time" libelle={t("emploi.debut")} value={debut} onChange={(ev) => setDebut(ev.target.value)} />
        <Champ id="seance-fin" type="time" libelle={t("emploi.fin")} value={fin} onChange={(ev) => setFin(ev.target.value)} />
        <Selection
          id="seance-matiere"
          libelle={t("emploi.matiere")}
          valeur={matiere}
          surChangement={(v) => {
            setMatiere(v);
            setEnseignant(classe.matieres.find((m) => m.matiere_id === v)?.enseignant_id ?? enseignant);
          }}
          options={classe.matieres.map((m) => ({ valeur: m.matiere_id, libelle: m.libelle }))}
        />
        <Selection id="seance-enseignant" libelle={t("emploi.enseignant")} valeur={enseignant} surChangement={setEnseignant} options={[{ valeur: "", libelle: t("emploi.sans_enseignant") }, ...e.enseignants.map((x) => ({ valeur: x.enseignant_id, libelle: x.nom }))]} />
        <Champ id="seance-salle" libelle={t("emploi.salle")} value={salle} onChange={(ev) => setSalle(ev.target.value)} autoComplete="off" />
        <Bouton type="submit" disabled={d === null || f === null || matiere === "" || placer.isPending}>
          {t(seance === null ? "emploi.placer" : "commun.enregistrer")}
        </Bouton>
        {seance === null ? null : (
          <Bouton variante="danger" onClick={() => retirer.mutate()} disabled={retirer.isPending}>
            {t("emploi.retirer")}
          </Bouton>
        )}
      </div>
      <MessageErreur erreur={placer.error ?? retirer.error} />
    </Formulaire>
  );
}

function Impressions({ e, classe }: { e: Etat; classe: ClasseEmploi | undefined }) {
  const [enseignant, setEnseignant] = useState(e.enseignants[0]?.enseignant_id ?? "");
  const [document, setDocument] = useState<string | null>(null);
  const deClasse = useMutation({
    mutationFn: () => appeler("emploi_classe_produire", { classe_id: classe?.classe_id ?? "" }),
    onSuccess: (d) => setDocument(d.pdf),
  });
  const dEnseignant = useMutation({
    mutationFn: () => appeler("emploi_enseignant_produire", { enseignant_id: enseignant }),
    onSuccess: (d) => setDocument(d.pdf),
  });
  return (
    <Section titre={t("emploi.impressions")}>
      <div className="flex flex-wrap items-end gap-3">
        <Bouton variante="secondaire" onClick={() => deClasse.mutate()} disabled={classe === undefined || deClasse.isPending}>
          {t("emploi.imprimer_classe", { classe: classe?.libelle ?? "" })}
        </Bouton>
        {e.enseignants.length === 0 ? null : (
          <>
            <Selection id="emploi-enseignant" libelle={t("emploi.enseignant")} valeur={enseignant} surChangement={setEnseignant} options={e.enseignants.map((x) => ({ valeur: x.enseignant_id, libelle: x.nom }))} />
            <Bouton variante="secondaire" onClick={() => dEnseignant.mutate()} disabled={enseignant === "" || dEnseignant.isPending}>
              {t("emploi.imprimer_enseignant")}
            </Bouton>
          </>
        )}
      </div>
      <MessageErreur erreur={deClasse.error ?? dEnseignant.error} />
      {document === null ? null : <ApercuPdf base64={document} />}
    </Section>
  );
}

function Journees({ e, surFait }: { e: Etat; surFait: (e: Etat) => void }) {
  return (
    <Section titre={t("emploi.journees")}>
      <p className="text-sm text-muet">{t("emploi.journees_aide")}</p>
      {e.journees.map((j) => (
        <FicheJournee key={`${j.cycle}-${JSON.stringify(j)}`} j={j} surFait={surFait} />
      ))}
    </Section>
  );
}

function FicheJournee({ j, surFait }: { j: JourneeCycle; surFait: (e: Etat) => void }) {
  const p = `journee-${j.cycle}`;
  const [jours, setJours] = useState<number[]>(j.jours);
  const [matinDebut, setMatinDebut] = useState(heure(j.matin.debut));
  const [matinFin, setMatinFin] = useState(heure(j.matin.fin));
  const [avecApresMidi, setAvecApresMidi] = useState(j.apres_midi !== null);
  const [apDebut, setApDebut] = useState(heure(j.apres_midi?.debut ?? 15 * 60));
  const [apFin, setApFin] = useState(heure(j.apres_midi?.fin ?? 17 * 60));
  const [sansApresMidi, setSansApresMidi] = useState<number[]>(j.jours_sans_apres_midi);
  const [recreation, setRecreation] = useState(j.recreations[0] === undefined ? "" : heure(j.recreations[0].debut));
  const [recreationFin, setRecreationFin] = useState(j.recreations[0] === undefined ? "" : heure(j.recreations[0].fin));
  const [canicule, setCanicule] = useState(j.canicule !== null);
  const [du, setDu] = useState(j.canicule?.du ?? "");
  const [au, setAu] = useState(j.canicule?.au ?? "");
  const [cDebut, setCDebut] = useState(heure(j.canicule?.matin.debut ?? 7 * 60 + 30));
  const [cFin, setCFin] = useState(heure(j.canicule?.matin.fin ?? 12 * 60));
  const lire = (debut: string, fin: string): Plage | null => {
    const d = minutes(debut);
    const f = minutes(fin);
    return d === null || f === null ? null : { debut: d, fin: f };
  };
  const matin = lire(matinDebut, matinFin);
  const regler = useMutation({
    mutationFn: () => {
      const recre = lire(recreation, recreationFin);
      const matinCanicule = lire(cDebut, cFin);
      return appeler("journee_regler", {
        cycle: j.cycle,
        jours,
        matin: matin ?? j.matin,
        apres_midi: avecApresMidi ? lire(apDebut, apFin) : null,
        jours_sans_apres_midi: avecApresMidi ? sansApresMidi.filter((d) => jours.includes(d)) : [],
        recreations: recre === null ? [] : [recre],
        inter_classe: j.inter_classe,
        canicule: j.cycle === "premier_cycle" && canicule && matinCanicule !== null && du !== "" && au !== "" ? { du, au, matin: matinCanicule } : null,
      });
    },
    onSuccess: surFait,
  });
  const basculer = (liste: number[], jour: number) => (liste.includes(jour) ? liste.filter((x) => x !== jour) : [...liste, jour].sort());
  return (
    <Formulaire surEnvoi={() => regler.mutate()}>
      <p className="text-sm font-medium text-encre">{t(CYCLES[j.cycle])}</p>
      <div className="flex flex-wrap items-end gap-3">
        {[1, 2, 3, 4, 5, 6].map((d) => (
          <CaseACocher key={d} id={`${p}-jour-${String(d)}`} libelle={t(JOURS[d] ?? "emploi.jour.6")} coche={jours.includes(d)} surChangement={() => setJours(basculer(jours, d))} />
        ))}
      </div>
      <div className="flex flex-wrap items-end gap-3">
        <Champ id={`${p}-matin-debut`} type="time" libelle={t("emploi.matin_debut")} value={matinDebut} onChange={(ev) => setMatinDebut(ev.target.value)} />
        <Champ id={`${p}-matin-fin`} type="time" libelle={t("emploi.matin_fin")} value={matinFin} onChange={(ev) => setMatinFin(ev.target.value)} />
        <Champ id={`${p}-recreation`} type="time" libelle={t("emploi.recreation_debut")} value={recreation} onChange={(ev) => setRecreation(ev.target.value)} />
        <Champ id={`${p}-recreation-fin`} type="time" libelle={t("emploi.recreation_fin")} value={recreationFin} onChange={(ev) => setRecreationFin(ev.target.value)} />
        <CaseACocher id={`${p}-apres-midi`} libelle={t("emploi.apres_midi")} coche={avecApresMidi} surChangement={setAvecApresMidi} />
        {avecApresMidi ? (
          <>
            <Champ id={`${p}-ap-debut`} type="time" libelle={t("emploi.apres_midi_debut")} value={apDebut} onChange={(ev) => setApDebut(ev.target.value)} />
            <Champ id={`${p}-ap-fin`} type="time" libelle={t("emploi.apres_midi_fin")} value={apFin} onChange={(ev) => setApFin(ev.target.value)} />
          </>
        ) : null}
      </div>
      {avecApresMidi ? (
        <div className="flex flex-wrap items-end gap-3">
          <span className="text-sm text-encre">{t("emploi.sans_apres_midi")}</span>
          {jours.map((d) => (
            <CaseACocher key={d} id={`${p}-sans-${String(d)}`} libelle={t(JOURS[d] ?? "emploi.jour.6")} coche={sansApresMidi.includes(d)} surChangement={() => setSansApresMidi(basculer(sansApresMidi, d))} />
          ))}
        </div>
      ) : null}
      {j.cycle === "premier_cycle" ? (
        <div className="flex flex-wrap items-end gap-3">
          <CaseACocher id={`${p}-canicule`} libelle={t("emploi.canicule")} coche={canicule} surChangement={setCanicule} />
          {canicule ? (
            <>
              <Champ id={`${p}-canicule-du`} type="date" libelle={t("emploi.canicule_du")} value={du} onChange={(ev) => setDu(ev.target.value)} />
              <Champ id={`${p}-canicule-au`} type="date" libelle={t("emploi.canicule_au")} value={au} onChange={(ev) => setAu(ev.target.value)} />
              <Champ id={`${p}-canicule-debut`} type="time" libelle={t("emploi.matin_debut")} value={cDebut} onChange={(ev) => setCDebut(ev.target.value)} />
              <Champ id={`${p}-canicule-fin`} type="time" libelle={t("emploi.matin_fin")} value={cFin} onChange={(ev) => setCFin(ev.target.value)} />
            </>
          ) : null}
        </div>
      ) : null}
      <div>
        <Bouton type="submit" disabled={matin === null || jours.length === 0 || regler.isPending}>
          {t("emploi.regler_journee")}
        </Bouton>
      </div>
      <MessageErreur erreur={regler.error} />
    </Formulaire>
  );
}
