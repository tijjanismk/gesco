import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { aujourdHui } from "../../app/aujourdhui";
import { commandeAutorisee } from "../../app/navigation";
import { ApercuPdf } from "../../composants/ApercuPdf";
import { Bouton } from "../../composants/Bouton";
import { Champ, Selection } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { type Cle, dateCivile, francs, t } from "../../i18n";
import type { Engagement, EngagementAffiche, FicheMembre, NatureEngagement, Remuneration } from "../../types/gesco";

export const NATURES: Record<NatureEngagement, Cle> = {
  duree_indeterminee: "engagement.nature.duree_indeterminee",
  duree_determinee: "engagement.nature.duree_determinee",
  vacation: "engagement.nature.vacation",
  stage: "engagement.nature.stage",
  benevole: "engagement.nature.benevole",
};

/** Montant saisi : chiffres seulement, en francs entiers (R-11). */
function lireFrancs(texte: string): number {
  const chiffres = texte.replace(/\D/g, "");
  return chiffres === "" ? 0 : Number.parseInt(chiffres, 10);
}

/** Heures par semaine saisies (« 6 », « 6,5 », « 6.25 ») en minutes ; le noyau exige le quart
 *  d'heure. */
function lireMinutes(texte: string): number {
  const valeur = Number.parseFloat(texte.replace(",", ".").trim());
  return Number.isFinite(valeur) ? Math.round(valeur * 60) : 0;
}

function heures(minutes: number): string {
  const h = Math.floor(minutes / 60);
  const m = minutes % 60;
  return m === 0 ? t("engagement.heures", { h }) : t("engagement.heures_minutes", { h, m: String(m).padStart(2, "0") });
}

/** Rémunération lisible ; rien quand le noyau en a retiré les montants (compte sans
 *  `contracts:read`). */
export function texteRemuneration(r: Remuneration): string {
  if (r.type === "mensuelle") return t("engagement.par_mois", { montant: francs(r.montant) });
  if (r.type === "horaire") return t("engagement.par_heure", { taux: francs(r.taux), heures: heures(r.minutes_par_semaine) });
  if (r.type === "masquee") return "";
  return t("engagement.sans_remuneration");
}

function periode(e: Engagement): string {
  const fin = e.arrete_le ?? e.fin;
  return fin === null ? t("engagement.depuis", { debut: dateCivile(e.debut) }) : t("engagement.du_au", { debut: dateCivile(e.debut), fin: dateCivile(fin) });
}

function etat(e: Engagement): string {
  if (e.statut === "en_cours") return t("engagement.statut.en_cours");
  const motif = e.motif_fin === "" ? "" : ` : ${e.motif_fin}`;
  return `${t(e.statut === "rompu" ? "engagement.statut.rompu" : "engagement.statut.termine")}${motif}`;
}

interface Saisie {
  nature: NatureEngagement;
  debut: string;
  fin: string;
  fonction: string;
  montant: string;
  taux: string;
  heures: string;
  note: string;
  remplace: string | null;
  renouvelle: string | null;
  /** Mois payés : le réglage de l'école, douze mois, ou hors des mois de vacances. */
  douze: "ecole" | "douze" | "annee";
}

/** Les engagements payés au mois : leurs mois payés se règlent. */
function mensuel(nature: NatureEngagement): boolean {
  return nature !== "vacation" && nature !== "benevole";
}

function saisieVide(fonction: string): Saisie {
  return { nature: "duree_indeterminee", debut: aujourdHui(), fin: "", fonction, montant: "", taux: "", heures: "", note: "", remplace: null, renouvelle: null, douze: "ecole" };
}

function remunerationDe(s: Saisie): Remuneration {
  if (s.nature === "vacation") return { type: "horaire", taux: lireFrancs(s.taux), minutes_par_semaine: lireMinutes(s.heures) };
  if (s.nature === "benevole") return { type: "aucune" };
  return { type: "mensuelle", montant: lireFrancs(s.montant) };
}

function saisieDepuis(e: { nature: NatureEngagement; debut: string; fin: string | null; fonction: string; remuneration: Remuneration; note: string; paye_douze_mois: boolean }): Saisie {
  const r = e.remuneration;
  return {
    nature: e.nature,
    debut: e.debut,
    fin: e.fin ?? "",
    fonction: e.fonction,
    montant: r.type === "mensuelle" ? String(r.montant) : "",
    taux: r.type === "horaire" ? String(r.taux) : "",
    heures: r.type === "horaire" ? String(r.minutes_par_semaine / 60).replace(".", ",") : "",
    note: e.note,
    remplace: null,
    renouvelle: null,
    douze: e.paye_douze_mois ? "douze" : "annee",
  };
}

/** Engagements d'un membre (SPEC_PHASE_6 §3) : ce qui le lie à l'école, du plus ancien au plus
 *  récent ; un nouveau salaire ou un nouveau taux est un nouvel engagement, jamais une
 *  correction de l'ancien. Qui gère les contrats les conclut et les clôt ; la lettre, qui porte
 *  les montants, s'imprime avec `contracts:read`. */
export function EngagementsMembre({ fiche, permissions }: { fiche: FicheMembre; permissions: readonly string[] }) {
  const client = useQueryClient();
  const id = fiche.membre.id;
  const engagements = useQuery({ queryKey: ["engagements", id], queryFn: () => appeler("engagements_lister", { id }) });
  const [saisie, setSaisie] = useState<Saisie | null>(null);
  const [lettre, setLettre] = useState<string | null>(null);
  const rafraichir = () => {
    void client.invalidateQueries({ queryKey: ["engagements", id] });
    void client.invalidateQueries({ queryKey: ["engagements-echeance"] });
  };
  const conclure = useMutation({
    mutationFn: (s: Saisie) =>
      appeler("engagement_conclure", {
        membre_id: id,
        nature: s.nature,
        debut: s.debut,
        fin: s.fin === "" ? null : s.fin,
        fonction: s.fonction,
        remuneration: remunerationDe(s),
        note: s.note,
        remplace: s.remplace,
        renouvelle: s.renouvelle,
        paye_douze_mois: s.douze === "ecole" ? null : s.douze === "douze",
      }),
    onSuccess: () => {
      setSaisie(null);
      rafraichir();
    },
  });
  const renouveler = useMutation({
    mutationFn: (engagement: string) => appeler("engagement_renouvellement", { id: engagement }),
    onSuccess: (p) => setSaisie({ ...saisieDepuis(p), renouvelle: p.renouvelle }),
  });
  const imprimer = useMutation({
    mutationFn: (engagement: string) => appeler("lettre_engagement_produire", { id: engagement }),
    onSuccess: (d) => setLettre(d.pdf),
  });
  const liste = engagements.data ?? [];
  const gere = commandeAutorisee("engagement_conclure", permissions);
  const imprime = commandeAutorisee("lettre_engagement_produire", permissions);
  return (
    <Section titre={t("engagement.titre")}>
      {liste.length === 0 ? <p className="text-sm text-muet">{t("engagement.aucun")}</p> : null}
      {liste.some((a) => a.engagement.remuneration.type === "masquee") ? <p className="text-sm text-muet">{t("engagement.montants_reserves")}</p> : null}
      <ul className="flex flex-col gap-3">
        {liste.map((a) => (
          <LigneEngagement
            key={a.engagement.id}
            affiche={a}
            gere={gere}
            imprime={imprime}
            surRemplacer={() =>
              setSaisie({ ...saisieDepuis(a.engagement), debut: aujourdHui(), fin: a.engagement.fin ?? "", remplace: a.engagement.id })
            }
            surRenouveler={() => renouveler.mutate(a.engagement.id)}
            surImprimer={() => imprimer.mutate(a.engagement.id)}
            surClos={rafraichir}
          />
        ))}
      </ul>
      {!gere ? null : saisie === null ? (
        <div>
          <Bouton variante="secondaire" onClick={() => setSaisie(saisieVide(fiche.membre.fonction))}>{t("engagement.nouveau")}</Bouton>
        </div>
      ) : (
        <FormulaireEngagement saisie={saisie} surChangement={setSaisie} surEnregistrer={() => conclure.mutate(saisie)} surAnnuler={() => setSaisie(null)} enCours={conclure.isPending} />
      )}
      <MessageErreur erreur={conclure.error ?? renouveler.error ?? imprimer.error ?? engagements.error} />
      {lettre === null ? null : <ApercuPdf base64={lettre} />}
    </Section>
  );
}

/** Engagements d'une fiche en lecture seule : la fiche personnelle de « Mon compte ». */
export function ListeEngagements({ engagements }: { engagements: readonly EngagementAffiche[] }) {
  if (engagements.length === 0) return <p className="text-sm text-muet">{t("engagement.aucun")}</p>;
  const rien = () => undefined;
  return (
    <ul className="flex flex-col gap-3">
      {engagements.map((a) => (
        <LigneEngagement key={a.engagement.id} affiche={a} gere={false} imprime={false} surRemplacer={rien} surRenouveler={rien} surImprimer={rien} surClos={rien} />
      ))}
    </ul>
  );
}

function LigneEngagement({
  affiche,
  gere,
  imprime,
  surRemplacer,
  surRenouveler,
  surImprimer,
  surClos,
}: {
  affiche: EngagementAffiche;
  gere: boolean;
  imprime: boolean;
  surRemplacer: () => void;
  surRenouveler: () => void;
  surImprimer: () => void;
  surClos: () => void;
}) {
  const e = affiche.engagement;
  const [cloture, setCloture] = useState<"termine" | "rompu" | null>(null);
  const [le, setLe] = useState(e.fin ?? aujourdHui());
  const [motif, setMotif] = useState("");
  const clore = useMutation({
    mutationFn: () => appeler(cloture === "rompu" ? "engagement_rompre" : "engagement_terminer", { id: e.id, le, motif }),
    onSuccess: () => {
      setCloture(null);
      surClos();
    },
  });
  const enCours = e.statut === "en_cours";
  const remuneration = texteRemuneration(e.remuneration);
  const moisPayes = useMutation({
    mutationFn: () => appeler("engagement_mois_payes", { id: e.id, paye_douze_mois: !e.paye_douze_mois }),
    onSuccess: surClos,
  });
  return (
    <li className="flex flex-col gap-2 rounded border border-muet/30 p-2 text-sm text-encre">
      <div className="flex flex-wrap items-baseline gap-x-3">
        <span className="font-semibold">{t(NATURES[e.nature])}</span>
        <span>{periode(e)}</span>
        <span className="text-muet">{e.fonction}</span>
        {remuneration === "" ? null : <span>{remuneration}</span>}
        <span className={enCours ? "text-vert" : "text-muet"}>{etat(e)}</span>
        {affiche.fin_proche ? <span className="font-semibold text-ocre">{t("engagement.fin_proche")}</span> : null}
        {affiche.renouvele && e.statut === "en_cours" ? <span className="text-muet">{t("engagement.renouvele")}</span> : null}
        {mensuel(e.nature) && !e.paye_douze_mois ? <span className="text-muet">{t("engagement.hors_vacances")}</span> : null}
      </div>
      {e.note === "" ? null : <p className="text-muet">{e.note}</p>}
      {gere || imprime ? (
        <div className="flex flex-wrap gap-2">
          {imprime ? <Bouton variante="secondaire" onClick={surImprimer}>{t("engagement.lettre")}</Bouton> : null}
          {gere && enCours ? <Bouton variante="secondaire" onClick={surRemplacer}>{t("engagement.remplacer")}</Bouton> : null}
          {!gere || e.fin === null || affiche.renouvele ? null : <Bouton variante="secondaire" onClick={surRenouveler}>{t("engagement.renouveler")}</Bouton>}
          {gere && enCours ? <Bouton variante="secondaire" onClick={() => setCloture("termine")}>{t("engagement.terminer")}</Bouton> : null}
          {gere && enCours ? <Bouton variante="danger" onClick={() => setCloture("rompu")}>{t("engagement.rompre")}</Bouton> : null}
          {gere && enCours && mensuel(e.nature) ? (
            <Bouton variante="secondaire" onClick={() => moisPayes.mutate()} disabled={moisPayes.isPending}>
              {t(e.paye_douze_mois ? "engagement.payer_hors_vacances" : "engagement.payer_douze_mois")}
            </Bouton>
          ) : null}
        </div>
      ) : null}
      {cloture === null ? null : (
        <div className="flex flex-wrap items-end gap-2">
          <Champ id={`engagement-${e.id}-le`} type="date" libelle={t("engagement.dernier_jour")} value={le} onChange={(x) => setLe(x.target.value)} />
          <Champ id={`engagement-${e.id}-motif`} libelle={t("engagement.motif")} value={motif} onChange={(x) => setMotif(x.target.value)} />
          <Bouton variante={cloture === "rompu" ? "danger" : "principal"} onClick={() => clore.mutate()} disabled={clore.isPending}>
            {t(cloture === "rompu" ? "engagement.confirmer_rupture" : "engagement.confirmer_fin")}
          </Bouton>
          <Bouton variante="secondaire" onClick={() => setCloture(null)}>{t("commun.annuler")}</Bouton>
        </div>
      )}
      <MessageErreur erreur={clore.error ?? moisPayes.error} />
    </li>
  );
}

function FormulaireEngagement({
  saisie,
  surChangement,
  surEnregistrer,
  surAnnuler,
  enCours,
}: {
  saisie: Saisie;
  surChangement: (s: Saisie) => void;
  surEnregistrer: () => void;
  surAnnuler: () => void;
  enCours: boolean;
}) {
  const s = saisie;
  const maj = (champ: Partial<Saisie>) => surChangement({ ...s, ...champ });
  const titre: Cle = s.remplace !== null ? "engagement.titre_remplacement" : s.renouvelle !== null ? "engagement.titre_renouvellement" : "engagement.nouveau";
  return (
    <div className="flex flex-col gap-3 rounded border border-primaire/40 p-3">
      <h3 className="text-base font-semibold text-encre">{t(titre)}</h3>
      {s.remplace === null ? null : <p className="text-sm text-muet">{t("engagement.aide_remplacement")}</p>}
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <Selection id="engagement-nature" libelle={t("engagement.nature")} valeur={s.nature} surChangement={(v) => maj({ nature: v as NatureEngagement })} options={Object.entries(NATURES).map(([v, c]) => ({ valeur: v, libelle: t(c) }))} />
        <Champ id="engagement-fonction" libelle={t("personnel.fonction")} value={s.fonction} onChange={(e) => maj({ fonction: e.target.value })} />
        <Champ id="engagement-debut" type="date" libelle={t("engagement.debut")} value={s.debut} onChange={(e) => maj({ debut: e.target.value })} />
        <Champ
          id="engagement-fin"
          type="date"
          libelle={t("engagement.fin")}
          aide={t(s.nature === "vacation" ? "engagement.fin_aide_vacation" : "engagement.fin_aide")}
          value={s.fin}
          onChange={(e) => maj({ fin: e.target.value })}
        />
        {s.nature === "vacation" ? (
          <>
            <Champ id="engagement-taux" inputMode="numeric" libelle={t("engagement.taux")} value={s.taux} onChange={(e) => maj({ taux: e.target.value })} />
            <Champ id="engagement-heures" inputMode="decimal" libelle={t("engagement.heures_semaine")} aide={t("engagement.heures_aide")} value={s.heures} onChange={(e) => maj({ heures: e.target.value })} />
          </>
        ) : s.nature === "benevole" ? null : (
          <Champ
            id="engagement-montant"
            inputMode="numeric"
            libelle={t(s.nature === "stage" ? "engagement.indemnite" : "engagement.salaire")}
            value={s.montant}
            onChange={(e) => maj({ montant: e.target.value })}
          />
        )}
        {mensuel(s.nature) ? (
          <Selection
            id="engagement-mois-payes"
            libelle={t("engagement.mois_payes")}
            valeur={s.douze}
            surChangement={(v) => maj({ douze: v as Saisie["douze"] })}
            options={[
              { valeur: "ecole", libelle: t("engagement.mois_payes.ecole") },
              { valeur: "douze", libelle: t("engagement.mois_payes.douze") },
              { valeur: "annee", libelle: t("engagement.mois_payes.annee") },
            ]}
          />
        ) : null}
        <Champ id="engagement-note" libelle={t("engagement.note")} value={s.note} onChange={(e) => maj({ note: e.target.value })} />
      </div>
      <div className="flex flex-wrap gap-2">
        <Bouton onClick={surEnregistrer} disabled={enCours}>{t("engagement.conclure")}</Bouton>
        <Bouton variante="secondaire" onClick={surAnnuler}>{t("commun.annuler")}</Bouton>
      </div>
    </div>
  );
}
