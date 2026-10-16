import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler, ErreurApi } from "../../api";
import { commandeAutorisee } from "../../app/navigation";
import { ApercuPdf } from "../../composants/ApercuPdf";
import { Bouton } from "../../composants/Bouton";
import { Champ, Selection } from "../../composants/Champ";
import { MessageErreur } from "../../composants/Messages";
import { type Cle, dateCivile, francs, t } from "../../i18n";
import type { Exclusion, NatureExclusion, StatutExclusion } from "../../types/gesco";
import { aujourdHui } from "../../app/aujourdhui";

const STATUTS: Record<StatutExclusion, Cle> = {
  en_cours: "exclusions.statut.en_cours",
  levee: "exclusions.statut.levee",
  revoquee: "exclusions.statut.revoquee",
};

/** Départ d'un élève (SPEC_PHASE_4 §9 ; réponse du 13 octobre 2026) : transfert vers une autre
 *  école, ou exclusion, temporaire avec date de retour ou définitive. */
export function DepartEleve({
  inscriptionId,
  permissions,
  surChange,
  surAnnonce,
}: {
  inscriptionId: string;
  permissions: readonly string[];
  surChange: () => void;
  /** Le formulaire disparaît avec l'inscription transférée : la confirmation remonte à la section. */
  surAnnonce: (texte: string) => void;
}) {
  const [choix, setChoix] = useState<"" | "transfert" | "exclusion">("");
  return (
    <div className="flex flex-col gap-2">
      <div className="flex flex-wrap gap-2">
        {commandeAutorisee("inscription_transferer", permissions) ? (
          <Bouton variante={choix === "transfert" ? "principal" : "secondaire"} onClick={() => setChoix(choix === "transfert" ? "" : "transfert")}>{t("depart.transferer")}</Bouton>
        ) : null}
        {commandeAutorisee("inscription_exclure", permissions) ? (
          <Bouton variante={choix === "exclusion" ? "principal" : "secondaire"} onClick={() => setChoix(choix === "exclusion" ? "" : "exclusion")}>{t("depart.exclure")}</Bouton>
        ) : null}
      </div>
      {choix === "transfert" ? <Transfert inscriptionId={inscriptionId} surChange={surChange} surAnnonce={surAnnonce} /> : null}
      {choix === "exclusion" ? <NouvelleExclusion inscriptionId={inscriptionId} surChange={surChange} /> : null}
    </div>
  );
}

function Transfert({ inscriptionId, surChange, surAnnonce }: { inscriptionId: string; surChange: () => void; surAnnonce: (texte: string) => void }) {
  const [date, setDate] = useState(aujourdHui);
  const [destination, setDestination] = useState("");
  const [motif, setMotif] = useState("");
  const [passerOutre, setPasserOutre] = useState("");
  const transfert = useMutation({
    mutationFn: () => appeler("inscription_transferer", { inscription_id: inscriptionId, date_depart: date, etablissement_destination: destination, motif, passer_outre: passerOutre }),
    onSuccess: (r) => {
      const reste = r.reste_du === 0 ? "" : ` ${t("depart.reste_du", { montant: francs(r.reste_du) })}`;
      surAnnonce(`${t("depart.transfere", { numero: r.numero_certificat, annulees: String(r.echeances_annulees) })}${reste}`);
      surChange();
    },
  });
  const soldeExige = transfert.error instanceof ErreurApi && transfert.error.cle === "transfert.solde_non_nul";
  return (
    <div className="flex flex-col gap-2 rounded border border-muet/30 p-3">
      <p className="text-sm text-muet">{t("depart.transfert_aide")}</p>
      <div className="flex flex-wrap items-end gap-3">
        <Champ id={`depart-date-${inscriptionId}`} type="date" libelle={t("depart.date")} value={date} onChange={(e) => setDate(e.target.value)} />
        <Champ id={`depart-destination-${inscriptionId}`} libelle={t("depart.destination")} value={destination} onChange={(e) => setDestination(e.target.value)} />
        <Champ id={`depart-motif-${inscriptionId}`} libelle={t("depart.motif")} value={motif} onChange={(e) => setMotif(e.target.value)} />
      </div>
      {soldeExige ? <Champ id={`depart-outre-${inscriptionId}`} libelle={t("depart.passer_outre")} value={passerOutre} onChange={(e) => setPasserOutre(e.target.value)} /> : null}
      <div>
        <Bouton variante="danger" onClick={() => transfert.mutate()} disabled={transfert.isPending}>{t("depart.confirmer_transfert")}</Bouton>
      </div>
      <MessageErreur erreur={transfert.error} />
    </div>
  );
}

function NouvelleExclusion({ inscriptionId, surChange }: { inscriptionId: string; surChange: () => void }) {
  const [nature, setNature] = useState<NatureExclusion>("temporaire");
  const [du, setDu] = useState(aujourdHui);
  const [au, setAu] = useState(aujourdHui);
  const [motif, setMotif] = useState("");
  const [decision, setDecision] = useState("");
  const exclusion = useMutation({
    mutationFn: () => appeler("inscription_exclure", { inscription_id: inscriptionId, nature, du, au: nature === "temporaire" ? au : null, motif, decision }),
    onSuccess: surChange,
  });
  return (
    <div className="flex flex-col gap-2 rounded border border-muet/30 p-3">
      <p className="text-sm text-muet">{t(nature === "temporaire" ? "exclusions.aide_temporaire" : "exclusions.aide_definitive")}</p>
      <div className="flex flex-wrap items-end gap-3">
        <Selection
          id={`exclusion-nature-${inscriptionId}`}
          libelle={t("exclusions.nature")}
          valeur={nature}
          surChangement={(v) => setNature(v as NatureExclusion)}
          options={[
            { valeur: "temporaire", libelle: t("exclusions.nature.temporaire") },
            { valeur: "definitive", libelle: t("exclusions.nature.definitive") },
          ]}
        />
        <Champ id={`exclusion-du-${inscriptionId}`} type="date" libelle={t("exclusions.du")} value={du} onChange={(e) => setDu(e.target.value)} />
        {nature === "temporaire" ? <Champ id={`exclusion-au-${inscriptionId}`} type="date" libelle={t("exclusions.au")} value={au} onChange={(e) => setAu(e.target.value)} /> : null}
      </div>
      <div className="flex flex-wrap items-end gap-3">
        <Champ id={`exclusion-motif-${inscriptionId}`} libelle={t("exclusions.motif")} value={motif} onChange={(e) => setMotif(e.target.value)} />
        <Champ id={`exclusion-decision-${inscriptionId}`} libelle={t("exclusions.decision")} value={decision} onChange={(e) => setDecision(e.target.value)} />
      </div>
      <div>
        <Bouton variante="danger" onClick={() => exclusion.mutate()} disabled={exclusion.isPending}>{t("exclusions.confirmer")}</Bouton>
      </div>
      <MessageErreur erreur={exclusion.error} />
    </div>
  );
}

/** Exclusions d'un élève : historique, levée d'une exclusion temporaire, réintégration après une
 *  exclusion définitive. Rien ne s'efface. */
export function ExclusionsEleve({ inscriptionId, permissions, surChange }: { inscriptionId: string; permissions: readonly string[]; surChange: () => void }) {
  const liste = useQuery({ queryKey: ["exclusions", inscriptionId], queryFn: () => appeler("exclusions_lister", { inscription_id: inscriptionId }) });
  if (liste.data === undefined || liste.data.length === 0) return <MessageErreur erreur={liste.error} />;
  return (
    <ul className="mt-2 flex flex-col gap-2">
      {liste.data.map((e) => (
        <LigneExclusion key={e.id} exclusion={e} peutLever={commandeAutorisee("exclusion_lever", permissions)} surChange={surChange} />
      ))}
    </ul>
  );
}

function LigneExclusion({ exclusion: e, peutLever, surChange }: { exclusion: Exclusion; peutLever: boolean; surChange: () => void }) {
  const client = useQueryClient();
  const [motif, setMotif] = useState("");
  const levee = useMutation({
    mutationFn: () => appeler("exclusion_lever", { id: e.id, motif }),
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: ["exclusions", e.inscription_id] });
      surChange();
    },
  });
  const terminee = e.nature === "temporaire" && e.au !== null && e.au < aujourdHui();
  return (
    <li className="rounded border border-ocre/50 p-2 text-sm">
      <p className="text-encre">
        {e.nature === "temporaire"
          ? t("exclusions.resume_temporaire", { du: dateCivile(e.du), au: e.au === null ? "" : dateCivile(e.au), motif: e.motif })
          : t("exclusions.resume_definitive", { du: dateCivile(e.du), motif: e.motif })}
        {e.decision === "" ? "" : ` · ${e.decision}`}
      </p>
      <p className="text-muet">
        {t(STATUTS[e.statut])}
        {e.leve_le === null ? "" : ` · ${dateCivile(e.leve_le)} · ${e.motif_levee}`}
      </p>
      {e.statut === "en_cours" && peutLever && !terminee ? (
        <div className="mt-1 flex flex-wrap items-end gap-2">
          <Champ id={`levee-motif-${e.id}`} libelle={t("exclusions.motif_levee")} value={motif} onChange={(x) => setMotif(x.target.value)} />
          <Bouton variante="secondaire" onClick={() => levee.mutate()} disabled={levee.isPending}>
            {t(e.nature === "temporaire" ? "exclusions.lever" : "exclusions.reintegrer")}
          </Bouton>
        </div>
      ) : null}
      <MessageErreur erreur={levee.error} />
    </li>
  );
}

/** Documents d'un élève transféré : certificat de transfert numéroté et relevé des moyennes. */
export function DocumentsDepart({ inscriptionId, permissions }: { inscriptionId: string; permissions: readonly string[] }) {
  const [pdf, setPdf] = useState<string | null>(null);
  const certificat = useMutation({ mutationFn: () => appeler("certificat_transfert_produire", { inscription_id: inscriptionId }), onSuccess: (d) => setPdf(d.pdf) });
  const releve = useMutation({ mutationFn: () => appeler("releve_moyennes_produire", { inscription_id: inscriptionId }), onSuccess: (d) => setPdf(d.pdf) });
  return (
    <div className="mt-2 flex flex-col gap-2">
      <div className="flex flex-wrap gap-2">
        {commandeAutorisee("certificat_transfert_produire", permissions) ? (
          <Bouton variante="secondaire" onClick={() => certificat.mutate()} disabled={certificat.isPending}>{t("depart.certificat")}</Bouton>
        ) : null}
        {commandeAutorisee("releve_moyennes_produire", permissions) ? (
          <Bouton variante="secondaire" onClick={() => releve.mutate()} disabled={releve.isPending}>{t("depart.releve")}</Bouton>
        ) : null}
      </div>
      <MessageErreur erreur={certificat.error ?? releve.error} />
      {pdf === null ? null : <ApercuPdf base64={pdf} />}
    </div>
  );
}
