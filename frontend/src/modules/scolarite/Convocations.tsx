import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { useSmsActif } from "../../app/modules";
import { ApercuPdf } from "../../composants/ApercuPdf";
import { Bouton } from "../../composants/Bouton";
import { CaseACocher, Champ, Formulaire, Selection } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { Tableau } from "../../composants/Tableau";
import { type Cle, dateCivile, t } from "../../i18n";
import type { AccuseConvocation, CanalConvocation, ConvocationAffichee, Convocations as ListeConvocations, SuiteConvocation } from "../../types/gesco";

const CANAUX: Record<CanalConvocation, Cle> = {
  eleve: "convocations.canal.eleve",
  main_propre: "convocations.canal.main_propre",
  sms: "convocations.canal.sms",
  whatsapp: "convocations.canal.whatsapp",
};

const ACCUSES: Record<AccuseConvocation, Cle> = {
  confirme: "convocations.accuse.confirme",
  transmis_non_confirme: "convocations.accuse.transmis_non_confirme",
  sans_reponse: "convocations.accuse.sans_reponse",
};

const SUITES: Record<SuiteConvocation, Cle> = {
  en_attente: "convocations.suite.en_attente",
  honoree: "convocations.suite.honoree",
  non_honoree: "convocations.suite.non_honoree",
  reportee: "convocations.suite.reportee",
};

function suite(c: ConvocationAffichee): string {
  return c.convocation.annulee ? t("convocations.annulee", { motif: c.convocation.motif_annulation }) : t(SUITES[c.convocation.suite]);
}

/** Convocations (SPEC_PHASE_5 §7) : qui est convoqué, quand, pourquoi, par quel canal, et ce que
 *  l'école sait de la réception ; puis la suite. Par SMS, le message attend une relecture dans
 *  « Messages ». */
export function Convocations() {
  const client = useQueryClient();
  const [toutes, setToutes] = useState(false);
  const [choisie, setChoisie] = useState<string | null>(null);
  const liste = useQuery({ queryKey: ["convocations", toutes], queryFn: () => appeler("convocations_lire", { en_cours: !toutes }) });
  const apres = (c: ConvocationAffichee) => {
    setChoisie(c.convocation.id);
    void client.invalidateQueries({ queryKey: ["convocations"] });
  };
  const l = liste.data;
  const convocation = l?.liste.find((c) => c.convocation.id === choisie);
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("convocations.titre")}</h1>
      <MessageErreur erreur={liste.error} />
      {l === undefined ? null : (
        <>
          {l.droits.creer ? <NouvelleConvocation surFait={apres} /> : null}
          <Section titre={toutes ? t("convocations.toutes") : t("convocations.en_cours")}>
            <CaseACocher id="convocations-toutes" libelle={t("convocations.voir_toutes")} coche={toutes} surChangement={setToutes} />
            {l.liste.length === 0 ? <p className="text-sm text-muet">{t("convocations.aucune")}</p> : null}
            <Tableau
              nomExport="convocations"
              titre={t("convocations.titre")}
              lignes={l.liste}
              cle={(c) => c.convocation.id}
              surSelection={(c) => setChoisie(c.convocation.id)}
              colonnes={[
                { titre: t("convocations.colonne.rendez_vous"), valeur: (c) => `${dateCivile(c.convocation.date)} · ${c.convocation.heure}` },
                { titre: t("convocations.colonne.eleve"), valeur: (c) => `${c.eleve} (${c.classe})` },
                { titre: t("convocations.colonne.responsable"), valeur: (c) => (c.responsable === "" ? t("convocations.responsables_legaux") : c.responsable) },
                { titre: t("convocations.colonne.motif"), valeur: (c) => c.convocation.motif },
                { titre: t("convocations.colonne.canal"), valeur: (c) => t(CANAUX[c.convocation.canal]) },
                { titre: t("convocations.colonne.reception"), valeur: (c) => t(ACCUSES[c.convocation.accuse]) },
                { titre: t("convocations.colonne.suite"), valeur: suite },
              ]}
            />
          </Section>
          {convocation === undefined ? null : <ConvocationChoisie key={convocation.convocation.id} c={convocation} droits={l.droits} surFait={apres} />}
        </>
      )}
    </div>
  );
}

function NouvelleConvocation({ surFait }: { surFait: (c: ConvocationAffichee) => void }) {
  const smsActif = useSmsActif();
  const annees = useQuery({ queryKey: ["annees"], queryFn: () => appeler("annees_lister", null) });
  const anneeId = annees.data?.find((a) => a.est_courante)?.id ?? null;
  const inscrits = useQuery({ queryKey: ["inscrits", anneeId], queryFn: () => appeler("inscriptions_lister", { annee_id: anneeId ?? "" }), enabled: anneeId !== null });
  const valides = (inscrits.data ?? []).filter((l) => l.inscription.statut === "validee");
  const [inscriptionId, setInscriptionId] = useState("");
  const inscription = valides.find((l) => l.inscription.id === inscriptionId);
  // Retrouver l'élève comme au guichet : quelques lettres, puis un clic parmi les inscrits.
  const [texte, setTexte] = useState("");
  const cherche = texte.trim().length >= 2 && inscriptionId === "";
  const trouves = useQuery({ queryKey: ["convocation-eleves", texte], queryFn: () => appeler("eleves_rechercher", { texte }), enabled: cherche });
  const proposes = (trouves.data ?? []).flatMap((e) => valides.filter((l) => l.inscription.eleve_id === e.id)).slice(0, 8);
  const fiche = useQuery({
    queryKey: ["eleve", inscription?.inscription.eleve_id],
    queryFn: () => appeler("eleve_lire", { id: inscription?.inscription.eleve_id ?? "" }),
    enabled: inscription !== undefined,
  });
  const dossier = useQuery({
    queryKey: ["dossier-discipline", inscriptionId],
    queryFn: () => appeler("discipline_dossier_lire", { inscription_id: inscriptionId }),
    enabled: inscriptionId !== "",
  });
  const sanctions = (dossier.data?.sanctions ?? []).filter((s) => s.sanction.nature === "convocation_parents" && !s.sanction.annulee);
  const [responsableId, setResponsableId] = useState("");
  const [motif, setMotif] = useState("");
  const [date, setDate] = useState("");
  const [heure, setHeure] = useState("08:00");
  const [canal, setCanal] = useState<CanalConvocation>("eleve");
  const [sanctionId, setSanctionId] = useState("");
  const choisir = (id: string) => {
    setInscriptionId(id);
    setResponsableId("");
    setSanctionId("");
  };
  const creation = useMutation({
    mutationFn: () =>
      appeler("convocation_creer", {
        inscription_id: inscriptionId,
        responsable_id: responsableId === "" ? null : responsableId,
        motif,
        date,
        heure,
        canal,
        sanction_id: sanctionId === "" ? null : sanctionId,
      }),
    onSuccess: (c) => {
      setMotif("");
      surFait(c);
    },
  });
  return (
    <Section titre={t("convocations.nouvelle")}>
      <Formulaire surEnvoi={() => creation.mutate()}>
        {inscription === undefined ? (
          <>
            <Champ id="convocation-eleve" type="search" libelle={t("convocations.eleve")} aide={t("convocations.eleve_aide")} value={texte} onChange={(e) => setTexte(e.target.value)} />
            <ul className="flex flex-col gap-1">
              {proposes.map((l) => (
                <li key={l.inscription.id}>
                  <button type="button" className="text-sm text-primaire underline" onClick={() => choisir(l.inscription.id)}>
                    {`${l.eleve} (${l.classe}) · ${l.matricule}`}
                  </button>
                </li>
              ))}
            </ul>
            {cherche && trouves.isSuccess && proposes.length === 0 ? <p className="text-sm text-muet">{t("convocations.aucun_eleve")}</p> : null}
            <MessageErreur erreur={trouves.error} />
          </>
        ) : (
          <div className="flex flex-wrap items-center gap-3">
            <p className="text-sm text-encre">{`${inscription.eleve} (${inscription.classe}) · ${inscription.matricule}`}</p>
            <Bouton variante="secondaire" onClick={() => choisir("")}>{t("convocations.autre_eleve")}</Bouton>
          </div>
        )}
        <Selection
          id="convocation-responsable"
          libelle={t("convocations.responsable")}
          valeur={responsableId}
          surChangement={setResponsableId}
          options={[{ valeur: "", libelle: t("convocations.responsables_legaux") }, ...(fiche.data?.responsables ?? []).map((r) => ({ valeur: r.responsable.id, libelle: `${r.identite.prenoms} ${r.identite.nom}` }))]}
        />
        {sanctions.length === 0 ? null : (
          <Selection
            id="convocation-sanction"
            libelle={t("convocations.sanction")}
            valeur={sanctionId}
            surChangement={(v) => {
              setSanctionId(v);
              const s = sanctions.find((x) => x.sanction.id === v);
              if (s !== undefined && motif === "") setMotif(s.sanction.motif);
            }}
            options={[{ valeur: "", libelle: t("convocations.sans_sanction") }, ...sanctions.map((s) => ({ valeur: s.sanction.id, libelle: `${dateCivile(s.sanction.date)} · ${s.sanction.motif}` }))]}
          />
        )}
        <Champ id="convocation-motif" libelle={t("convocations.motif")} value={motif} onChange={(e) => setMotif(e.target.value)} />
        <div className="flex flex-wrap gap-3">
          <Champ id="convocation-date" type="date" libelle={t("convocations.date")} value={date} onChange={(e) => setDate(e.target.value)} />
          <Champ id="convocation-heure" type="time" libelle={t("convocations.heure")} value={heure} onChange={(e) => setHeure(e.target.value)} />
          <Selection
            id="convocation-canal"
            libelle={t("convocations.canal")}
            valeur={canal}
            surChangement={(v) => setCanal((Object.keys(CANAUX) as CanalConvocation[]).find((c) => c === v) ?? "eleve")}
            options={(Object.keys(CANAUX) as CanalConvocation[]).filter((c) => smsActif || c !== "sms").map((c) => ({ valeur: c, libelle: t(CANAUX[c]) }))}
          />
        </div>
        {canal === "sms" ? <p className="text-sm text-muet">{t("convocations.sms_aide")}</p> : null}
        <MessageErreur erreur={creation.error} />
        <div><Bouton type="submit" disabled={inscriptionId === "" || creation.isPending}>{t("convocations.convoquer")}</Bouton></div>
      </Formulaire>
    </Section>
  );
}

function ConvocationChoisie({ c, droits, surFait }: { c: ConvocationAffichee; droits: ListeConvocations["droits"]; surFait: (c: ConvocationAffichee) => void }) {
  const [date, setDate] = useState(c.convocation.date);
  const [heure, setHeure] = useState(c.convocation.heure);
  const [motif, setMotif] = useState("");
  const [pdf, setPdf] = useState<string | null>(null);
  const id = c.convocation.id;
  const accuse = useMutation({ mutationFn: (a: AccuseConvocation) => appeler("convocation_accuser", { id, accuse: a }), onSuccess: surFait });
  const report = useMutation({ mutationFn: () => appeler("convocation_reporter", { id, date, heure }), onSuccess: surFait });
  const cloture = useMutation({ mutationFn: (honoree: boolean) => appeler("convocation_clore", { id, honoree }), onSuccess: surFait });
  const annulation = useMutation({ mutationFn: () => appeler("convocation_annuler", { id, motif }), onSuccess: surFait });
  const billet = useMutation({ mutationFn: () => appeler("convocation_billet_produire", { id }), onSuccess: (d) => setPdf(d.pdf) });
  const enCours = !c.convocation.annulee && (c.convocation.suite === "en_attente" || c.convocation.suite === "reportee");
  return (
    <Section titre={t("convocations.choisie", { eleve: c.eleve, date: dateCivile(c.convocation.date), heure: c.convocation.heure })}>
      <p className="text-sm text-muet">{`${t(CANAUX[c.convocation.canal])} · ${t(ACCUSES[c.convocation.accuse])} · ${suite(c)}`}</p>
      <div className="flex flex-wrap gap-2">
        {droits.creer ? <Bouton variante="secondaire" onClick={() => billet.mutate()} disabled={billet.isPending}>{t("convocations.imprimer")}</Bouton> : null}
        {droits.modifier && enCours ? (
          (Object.keys(ACCUSES) as AccuseConvocation[])
            .filter((a) => a !== c.convocation.accuse)
            .map((a) => (
              <Bouton key={a} variante="secondaire" onClick={() => accuse.mutate(a)} disabled={accuse.isPending}>{t("convocations.noter", { accuse: t(ACCUSES[a]) })}</Bouton>
            ))
        ) : null}
        {droits.clore && enCours ? (
          <>
            <Bouton onClick={() => cloture.mutate(true)} disabled={cloture.isPending}>{t("convocations.honoree")}</Bouton>
            <Bouton variante="secondaire" onClick={() => cloture.mutate(false)} disabled={cloture.isPending}>{t("convocations.non_honoree")}</Bouton>
          </>
        ) : null}
      </div>
      {droits.modifier && enCours ? (
        <>
          <Formulaire surEnvoi={() => report.mutate()}>
            <div className="flex flex-wrap items-end gap-3">
              <Champ id="report-date" type="date" libelle={t("convocations.nouvelle_date")} value={date} onChange={(e) => setDate(e.target.value)} />
              <Champ id="report-heure" type="time" libelle={t("convocations.heure")} value={heure} onChange={(e) => setHeure(e.target.value)} />
              <Bouton type="submit" variante="secondaire" disabled={report.isPending}>{t("convocations.reporter")}</Bouton>
            </div>
          </Formulaire>
          <Formulaire surEnvoi={() => annulation.mutate()}>
            <div className="flex flex-wrap items-end gap-3">
              <Champ id="annulation-motif" libelle={t("convocations.motif_annulation")} value={motif} onChange={(e) => setMotif(e.target.value)} />
              <Bouton type="submit" variante="danger" disabled={annulation.isPending}>{t("convocations.annuler")}</Bouton>
            </div>
          </Formulaire>
        </>
      ) : null}
      <MessageErreur erreur={accuse.error ?? report.error ?? cloture.error ?? annulation.error ?? billet.error} />
      {pdf === null ? null : <ApercuPdf base64={pdf} />}
    </Section>
  );
}
