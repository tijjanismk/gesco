import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { commandeAutorisee } from "../../app/navigation";
import { Bouton } from "../../composants/Bouton";
import { CaseACocher, Champ, Selection } from "../../composants/Champ";
import { MessageEnregistre, MessageErreur, Section } from "../../composants/Messages";
import { Tableau } from "../../composants/Tableau";
import { type Cle, francs, moisCivil, nomMois, t } from "../../i18n";
import type { ElementAffiche, ElementsPaie, NatureElement, NatureEngagement, Prorata, ReglagesPaie } from "../../types/gesco";
import { NATURES } from "../rh/Engagements";
import { calculDe, lireMontant, type SaisieCalcul, saisieDe, texteCalcul } from "./calcul";
import { Paiements } from "./Paiements";
import { PaieDuMois } from "./PaieDuMois";

const ONGLETS: Record<"mois" | "paiements" | "elements", Cle> = {
  mois: "paie.onglet.mois",
  paiements: "paie.onglet.paiements",
  elements: "paie.onglet.elements",
};

const NATURES_ELEMENT: Record<NatureElement, Cle> = {
  gain: "paie.nature.gain",
  retenue: "paie.nature.retenue",
  cotisation_patronale: "paie.nature.cotisation_patronale",
};

const PRORATAS: Record<Prorata, Cle> = {
  trentieme: "paie.prorata.trentieme",
  calendrier: "paie.prorata.calendrier",
  ouvrables: "paie.prorata.ouvrables",
};

const MODES: Record<SaisieCalcul["mode"], Cle> = {
  fixe: "paie.calcul.fixe",
  taux: "paie.calcul.taux",
  bareme: "paie.calcul.bareme",
  saisi: "paie.calcul.saisi",
};

/** Natures d'engagement payées : toutes sauf le bénévolat. */
const NATURES_PAYEES: readonly NatureEngagement[] = ["duree_indeterminee", "duree_determinee", "vacation", "stage"];

/** Paie du personnel (SPEC_PHASE_7) : la paie du mois, puis les éléments et réglages de l'école. */
export function Paie() {
  const session = useQuery({ queryKey: ["session"], queryFn: () => appeler("session_courante", null) });
  const [onglet, setOnglet] = useState<"mois" | "paiements" | "elements">("mois");
  const permissions = session.data?.permissions ?? [];
  const onglets = (["mois", "paiements", "elements"] as const).filter((o) => o !== "paiements" || commandeAutorisee("paiements_personnel_lire", permissions));
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("paie.titre")}</h1>
      <div role="tablist" className="flex gap-2 border-b border-muet">
        {onglets.map((o) => (
          <button
            key={o}
            type="button"
            role="tab"
            aria-selected={onglet === o}
            className={`-mb-px border-b-2 px-3 py-1.5 text-sm ${onglet === o ? "border-primaire font-medium text-encre" : "border-transparent text-muet hover:text-encre"}`}
            onClick={() => setOnglet(o)}
          >
            {t(ONGLETS[o])}
          </button>
        ))}
      </div>
      {onglet === "mois" ? <PaieDuMois permissions={permissions} /> : null}
      {onglet === "paiements" ? <Paiements /> : null}
      {onglet === "elements" ? <ElementsEtReglages /> : null}
    </div>
  );
}

/** Réglages de l'école, éléments de paie et leurs calculs datés, primes et retenues attribuées. */
function ElementsEtReglages() {
  const client = useQueryClient();
  const etat = useQuery({ queryKey: ["paie-elements"], queryFn: () => appeler("elements_paie_lire", null) });
  const mettreAJour = (e: ElementsPaie) => client.setQueryData(["paie-elements"], e);
  const [choisi, setChoisi] = useState<string | null>(null);
  const e = etat.data;
  return (
    <div className="flex flex-col gap-4">
      <MessageErreur erreur={etat.error} />
      {e === undefined ? null : (
        <>
          <Reglages key={JSON.stringify(e.reglages)} reglages={e.reglages} surEnregistre={mettreAJour} />
          <Section titre={t("paie.elements")}>
            <p className="text-sm text-muet">{t("paie.elements_aide")}</p>
            <div className="flex flex-wrap items-start gap-4">
              <div className="min-w-0 flex-1">
                <Tableau
                  nomExport="elements-de-paie"
                  titre={t("paie.elements")}
                  lignes={e.elements}
                  cle={(x) => x.element.id}
                  surSelection={(x) => setChoisi(x.element.id)}
                  colonnes={[
                    { titre: t("paie.element.libelle"), valeur: (x) => x.element.libelle },
                    { titre: t("paie.element.nature"), valeur: (x) => t(NATURES_ELEMENT[x.element.nature]) },
                    { titre: t("paie.element.calcul"), valeur: (x) => (x.en_vigueur === null ? "" : texteCalcul(x.en_vigueur.calcul, deduits(e, x.en_vigueur.assiette_deduit))) },
                    { titre: t("paie.element.etat"), valeur: (x) => t(x.element.actif ? "paie.element.actif" : "paie.element.inactif") },
                  ]}
                />
              </div>
              <aside className="w-full max-w-xl">
                {choisi === null ? <NouvelElement surEnregistre={mettreAJour} /> : null}
                {e.elements
                  .filter((x) => x.element.id === choisi)
                  .map((x) => (
                    <EditionElement key={`${x.element.id}-${x.versions.length}-${String(x.element.actif)}`} e={e} x={x} surEnregistre={mettreAJour} surFermer={() => setChoisi(null)} />
                  ))}
              </aside>
            </div>
          </Section>
          <Attributions e={e} surEnregistre={mettreAJour} />
        </>
      )}
    </div>
  );
}

function deduits(e: ElementsPaie, ids: readonly string[]): string[] {
  return ids.map((id) => e.elements.find((x) => x.element.id === id)?.element.libelle ?? "");
}

function Reglages({ reglages, surEnregistre }: { reglages: ReglagesPaie; surEnregistre: (e: ElementsPaie) => void }) {
  const [saisie, setSaisie] = useState(reglages);
  const [enregistre, setEnregistre] = useState<number | null>(null);
  const enregistrement = useMutation({
    mutationFn: () => appeler("paie_reglages_enregistrer", saisie),
    onSuccess: (e) => {
      setEnregistre(Date.now());
      surEnregistre(e);
    },
  });
  return (
    <Section titre={t("paie.reglages")}>
      <CaseACocher id="paie-douze-mois" libelle={t("paie.douze_mois")} coche={saisie.douze_mois} surChangement={(c) => setSaisie({ ...saisie, douze_mois: c })} />
      <p className="text-sm text-muet">{t("paie.douze_mois_aide")}</p>
      <fieldset className="flex flex-wrap gap-x-4 gap-y-1">
        <legend className="text-sm text-encre">{t("paie.mois_vacances")}</legend>
        {Array.from({ length: 12 }, (_, i) => i + 1).map((m) => (
          <CaseACocher
            key={m}
            id={`paie-vacances-${m}`}
            libelle={nomMois(m)}
            coche={saisie.mois_vacances.includes(m)}
            surChangement={(c) => setSaisie({ ...saisie, mois_vacances: c ? [...saisie.mois_vacances, m].sort((a, b) => a - b) : saisie.mois_vacances.filter((x) => x !== m) })}
          />
        ))}
      </fieldset>
      <div className="w-96">
        <Selection
          id="paie-prorata"
          libelle={t("paie.prorata")}
          valeur={saisie.prorata}
          surChangement={(v) => setSaisie({ ...saisie, prorata: v as Prorata })}
          options={Object.entries(PRORATAS).map(([v, c]) => ({ valeur: v, libelle: t(c) }))}
        />
      </div>
      <div className="flex items-center gap-3">
        <Bouton onClick={() => enregistrement.mutate()} disabled={enregistrement.isPending}>{t("commun.enregistrer")}</Bouton>
        <MessageEnregistre instant={enregistre} />
      </div>
      <MessageErreur erreur={enregistrement.error} />
    </Section>
  );
}

/** Le calcul d'un élément, saisi : son mode, puis ses montants, taux ou tranches. */
function EditeurCalcul({ nature, saisie, surChangement }: { nature: NatureElement; saisie: SaisieCalcul; surChangement: (s: SaisieCalcul) => void }) {
  const s = saisie;
  const maj = (x: Partial<SaisieCalcul>) => surChangement({ ...s, ...x });
  const modes = Object.entries(MODES).filter(([m]) => (m !== "bareme" || nature === "retenue") && (m !== "saisi" || nature !== "cotisation_patronale"));
  return (
    <div className="flex flex-col gap-2">
      <div className="w-72">
        <Selection id="calcul-mode" libelle={t("paie.calcul.mode")} valeur={s.mode} surChangement={(v) => maj({ mode: v as SaisieCalcul["mode"] })} options={modes.map(([v, c]) => ({ valeur: v, libelle: t(c) }))} />
      </div>
      {s.mode === "fixe" ? (
        <div className="w-48">
          <Champ id="calcul-montant" inputMode="numeric" libelle={t("paie.calcul.montant")} value={s.montant} onChange={(x) => maj({ montant: x.target.value })} />
        </div>
      ) : null}
      {s.mode === "taux" ? (
        <div className="flex flex-wrap gap-3">
          <div className="w-32">
            <Champ id="calcul-taux" inputMode="decimal" libelle={t("paie.calcul.taux_pour_cent")} value={s.taux} onChange={(x) => maj({ taux: x.target.value })} />
          </div>
          <div className="w-56">
            <Champ id="calcul-plafond" inputMode="numeric" libelle={t("paie.calcul.plafond")} value={s.plafond} onChange={(x) => maj({ plafond: x.target.value })} />
          </div>
        </div>
      ) : null}
      {s.mode === "bareme" ? (
        <div className="flex flex-col gap-2">
          <CaseACocher id="calcul-annuel" libelle={t("paie.calcul.annuel")} coche={s.annuel} surChangement={(c) => maj({ annuel: c })} />
          {s.tranches.map((tr, i) => (
            <div key={i} className="flex flex-wrap items-end gap-2">
              <div className="w-48">
                <Champ id={`calcul-tranche-${i + 1}-jusqua`} inputMode="numeric" libelle={t("paie.calcul.tranche_jusqua")} value={tr.jusqua} onChange={(x) => maj({ tranches: s.tranches.map((y, j) => (j === i ? { ...y, jusqua: x.target.value } : y)) })} />
              </div>
              <div className="w-28">
                <Champ id={`calcul-tranche-${i + 1}-taux`} inputMode="decimal" libelle={t("paie.calcul.taux_pour_cent")} value={tr.taux} onChange={(x) => maj({ tranches: s.tranches.map((y, j) => (j === i ? { ...y, taux: x.target.value } : y)) })} />
              </div>
              {s.tranches.length > 1 ? <Bouton variante="secondaire" onClick={() => maj({ tranches: s.tranches.filter((_, j) => j !== i) })}>{t("paie.calcul.retirer_tranche")}</Bouton> : null}
            </div>
          ))}
          <div>
            <Bouton variante="secondaire" onClick={() => maj({ tranches: [...s.tranches, { jusqua: "", taux: "" }] })}>{t("paie.calcul.ajouter_tranche")}</Bouton>
          </div>
          <div className="w-72">
            <Champ id="calcul-points" inputMode="decimal" libelle={t("paie.calcul.points")} value={s.points} onChange={(x) => maj({ points: x.target.value })} />
          </div>
          <CaseACocher id="calcul-famille" libelle={t("paie.calcul.famille")} coche={s.famille} surChangement={(c) => maj({ famille: c })} />
          {s.famille ? (
            <div className="flex flex-wrap gap-3">
              <div className="w-28">
                <Champ id="calcul-marie" inputMode="decimal" libelle={t("paie.calcul.marie")} value={s.marie} onChange={(x) => maj({ marie: x.target.value })} />
              </div>
              <div className="w-28">
                <Champ id="calcul-par-enfant" inputMode="decimal" libelle={t("paie.calcul.par_enfant")} value={s.parEnfant} onChange={(x) => maj({ parEnfant: x.target.value })} />
              </div>
              <div className="w-28">
                <Champ id="calcul-plafond-famille" inputMode="decimal" libelle={t("paie.calcul.plafond_famille")} value={s.plafondFamille} onChange={(x) => maj({ plafondFamille: x.target.value })} />
              </div>
            </div>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}

function ChoixNatures({ natures, fonctionnaires, surChangement }: { natures: readonly NatureEngagement[]; fonctionnaires: boolean; surChangement: (natures: NatureEngagement[], fonctionnaires: boolean) => void }) {
  return (
    <fieldset className="flex flex-col gap-1">
      <legend className="text-sm text-encre">{t("paie.element.natures")}</legend>
      <div className="flex flex-wrap gap-x-4 gap-y-1">
        {NATURES_PAYEES.map((n) => (
          <CaseACocher
            key={n}
            id={`element-nature-${n}`}
            libelle={t(NATURES[n])}
            coche={natures.includes(n)}
            surChangement={(c) => surChangement(c ? [...natures, n] : natures.filter((x) => x !== n), fonctionnaires)}
          />
        ))}
      </div>
      <CaseACocher id="element-fonctionnaires" libelle={t("paie.element.fonctionnaires")} coche={fonctionnaires} surChangement={(c) => surChangement([...natures], c)} />
    </fieldset>
  );
}

function NouvelElement({ surEnregistre }: { surEnregistre: (e: ElementsPaie) => void }) {
  const [libelle, setLibelle] = useState("");
  const [nature, setNature] = useState<NatureElement>("gain");
  const [natures, setNatures] = useState<NatureEngagement[]>([...NATURES_PAYEES]);
  const [fonctionnaires, setFonctionnaires] = useState(true);
  const [calcul, setCalcul] = useState<SaisieCalcul>({ ...saisieDe(null), mode: "fixe" });
  const [illisible, setIllisible] = useState(false);
  const creation = useMutation({
    mutationFn: async () => {
      const c = calculDe(calcul);
      setIllisible(c === null);
      if (c === null) return null;
      return appeler("element_paie_enregistrer", { id: null, libelle, nature, natures, fonctionnaires, actif: true, calcul: c });
    },
    onSuccess: (e) => {
      if (e === null) return;
      setLibelle("");
      setNature("gain");
      setNatures([...NATURES_PAYEES]);
      setFonctionnaires(true);
      setCalcul({ ...saisieDe(null), mode: "fixe" });
      surEnregistre(e);
    },
  });
  return (
    <Section titre={t("paie.element.nouveau")}>
      <Champ id="element-libelle" libelle={t("paie.element.libelle")} value={libelle} onChange={(x) => setLibelle(x.target.value)} />
      <div className="w-72">
        <Selection
          id="element-nature"
          libelle={t("paie.element.nature")}
          valeur={nature}
          surChangement={(v) => {
            setNature(v as NatureElement);
            setCalcul({ ...calcul, mode: "fixe" });
          }}
          options={Object.entries(NATURES_ELEMENT).map(([v, c]) => ({ valeur: v, libelle: t(c) }))}
        />
      </div>
      <ChoixNatures
        natures={natures}
        fonctionnaires={fonctionnaires}
        surChangement={(n, f) => {
          setNatures(n);
          setFonctionnaires(f);
        }}
      />
      <EditeurCalcul nature={nature} saisie={calcul} surChangement={setCalcul} />
      {illisible ? <p className="text-sm text-rouge">{t("paie.calcul.illisible")}</p> : null}
      <MessageErreur erreur={creation.error} />
      <div>
        <Bouton onClick={() => creation.mutate()} disabled={creation.isPending}>{t("paie.element.creer")}</Bouton>
      </div>
    </Section>
  );
}

function EditionElement({ e, x, surEnregistre, surFermer }: { e: ElementsPaie; x: ElementAffiche; surEnregistre: (e: ElementsPaie) => void; surFermer: () => void }) {
  const el = x.element;
  const [libelle, setLibelle] = useState(el.libelle);
  const [natures, setNatures] = useState<NatureEngagement[]>([...el.natures]);
  const [fonctionnaires, setFonctionnaires] = useState(el.fonctionnaires);
  const [actif, setActif] = useState(el.actif);
  const [effet, setEffet] = useState(e.mois);
  const [calcul, setCalcul] = useState<SaisieCalcul>(saisieDe(x.en_vigueur?.calcul ?? null));
  const [deduit, setDeduit] = useState<string[]>([...(x.en_vigueur?.assiette_deduit ?? [])]);
  const [illisible, setIllisible] = useState(false);
  const [enregistre, setEnregistre] = useState<number | null>(null);
  const element = useMutation({
    mutationFn: () => appeler("element_paie_enregistrer", { id: el.id, libelle, nature: el.nature, natures, fonctionnaires, actif, calcul: null }),
    onSuccess: (r) => {
      setEnregistre(Date.now());
      surEnregistre(r);
    },
  });
  const version = useMutation({
    mutationFn: async () => {
      const c = calculDe(calcul);
      setIllisible(c === null);
      if (c === null) return null;
      return appeler("element_paie_version_enregistrer", { element_id: el.id, effet, calcul: c, assiette_deduit: deduit });
    },
    onSuccess: (r) => {
      if (r === null) return;
      setEnregistre(Date.now());
      surEnregistre(r);
    },
  });
  const retenues = e.elements.filter((y) => y.element.nature === "retenue" && y.element.id !== el.id);
  return (
    <Section titre={el.libelle}>
      <p className="text-sm text-muet">{t(NATURES_ELEMENT[el.nature])}</p>
      <Champ id="element-libelle" libelle={t("paie.element.libelle")} value={libelle} onChange={(z) => setLibelle(z.target.value)} />
      <ChoixNatures
        natures={natures}
        fonctionnaires={fonctionnaires}
        surChangement={(n, f) => {
          setNatures(n);
          setFonctionnaires(f);
        }}
      />
      <CaseACocher id="element-actif" libelle={t("paie.element.activer")} coche={actif} surChangement={setActif} />
      <div className="flex flex-wrap gap-2">
        <Bouton onClick={() => element.mutate()} disabled={element.isPending}>{t("paie.element.enregistrer")}</Bouton>
        <Bouton variante="secondaire" onClick={surFermer}>{t("commun.fermer")}</Bouton>
      </div>
      <h3 className="text-base font-semibold text-encre">{t("paie.version.titre")}</h3>
      <p className="text-sm text-muet">{t("paie.version.aide")}</p>
      <div className="w-56">
        <Champ id="version-effet" type="month" libelle={t("paie.version.effet")} value={effet} onChange={(z) => z.target.value !== "" && setEffet(z.target.value)} />
      </div>
      <EditeurCalcul nature={el.nature} saisie={calcul} surChangement={setCalcul} />
      {(calcul.mode === "taux" || calcul.mode === "bareme") && retenues.length > 0 ? (
        <fieldset className="flex flex-wrap gap-x-4 gap-y-1">
          <legend className="text-sm text-encre">{t("paie.calcul.assiette")}</legend>
          {retenues.map((r) => (
            <CaseACocher
              key={r.element.id}
              id={`assiette-${r.element.code ?? r.element.id}`}
              libelle={r.element.libelle}
              coche={deduit.includes(r.element.id)}
              surChangement={(c) => setDeduit(c ? [...deduit, r.element.id] : deduit.filter((d) => d !== r.element.id))}
            />
          ))}
        </fieldset>
      ) : null}
      {illisible ? <p className="text-sm text-rouge">{t("paie.calcul.illisible")}</p> : null}
      <div>
        <Bouton onClick={() => version.mutate()} disabled={version.isPending}>{t("paie.version.enregistrer")}</Bouton>
      </div>
      <MessageEnregistre instant={enregistre} />
      <MessageErreur erreur={element.error ?? version.error} />
      <h3 className="text-base font-semibold text-encre">{t("paie.version.historique")}</h3>
      <ul className="flex flex-col gap-1 text-sm text-encre">
        {[...x.versions].reverse().map((v) => (
          <li key={v.id}>
            <span className="font-medium">{t("paie.version.depuis", { mois: moisCivil(v.effet) })}</span>
            <span className="text-muet">{` · ${texteCalcul(v.calcul, deduits(e, v.assiette_deduit))}`}</span>
          </li>
        ))}
      </ul>
    </Section>
  );
}

function Attributions({ e, surEnregistre }: { e: ElementsPaie; surEnregistre: (e: ElementsPaie) => void }) {
  const personnel = useQuery({ queryKey: ["personnel"], queryFn: () => appeler("personnel_lister", null) });
  const membres = (personnel.data ?? []).filter((m) => m.membre.parti_le === null);
  const attribuables = e.elements.filter((x) => x.element.actif && x.element.nature !== "cotisation_patronale");
  const [membreId, setMembreId] = useState("");
  const [elementId, setElementId] = useState("");
  const [montant, setMontant] = useState("");
  const [debut, setDebut] = useState(e.mois);
  const [fin, setFin] = useState("");
  const ajout = useMutation({
    mutationFn: () =>
      appeler("attribution_enregistrer", { id: null, membre_id: membreId, element_id: elementId, montant: lireMontant(montant) ?? 0, debut, fin: fin === "" ? null : fin }),
    onSuccess: (r) => {
      setMontant("");
      setFin("");
      surEnregistre(r);
    },
  });
  return (
    <Section titre={t("paie.attributions")}>
      <p className="text-sm text-muet">{t("paie.attributions_aide")}</p>
      {e.attributions.length === 0 ? <p className="text-sm text-muet">{t("paie.attribution.aucune")}</p> : null}
      <ul className="flex flex-col gap-1 text-sm text-encre">
        {e.attributions.map((a) => (
          <li key={a.attribution.id}>
            <span className="font-medium">{a.membre}</span>
            <span className="text-muet">{` · ${a.element} · ${francs(a.attribution.montant)} · `}</span>
            <span className="text-muet">
              {a.attribution.fin === null
                ? t("paie.attribution.depuis", { debut: moisCivil(a.attribution.debut) })
                : t("paie.attribution.de_a", { debut: moisCivil(a.attribution.debut), fin: moisCivil(a.attribution.fin) })}
            </span>
          </li>
        ))}
      </ul>
      <div className="flex flex-wrap items-end gap-3">
        <div className="w-64">
          <Selection
            id="attribution-membre"
            libelle={t("paie.attribution.membre")}
            valeur={membreId}
            surChangement={setMembreId}
            options={[{ valeur: "", libelle: t("conges.choisir_membre") }, ...membres.map((m) => ({ valeur: m.membre.id, libelle: `${m.prenoms} ${m.nom} · ${m.membre.fonction}` }))]}
          />
        </div>
        <div className="w-64">
          <Selection
            id="attribution-element"
            libelle={t("paie.attribution.element")}
            valeur={elementId}
            surChangement={setElementId}
            options={[{ valeur: "", libelle: t("conges.choisir_membre") }, ...attribuables.map((x) => ({ valeur: x.element.id, libelle: x.element.libelle }))]}
          />
        </div>
        <div className="w-36">
          <Champ id="attribution-montant" inputMode="numeric" libelle={t("paie.attribution.montant")} value={montant} onChange={(z) => setMontant(z.target.value)} />
        </div>
        <div className="w-44">
          <Champ id="attribution-debut" type="month" libelle={t("paie.attribution.debut")} value={debut} onChange={(z) => z.target.value !== "" && setDebut(z.target.value)} />
        </div>
        <div className="w-44">
          <Champ id="attribution-fin" type="month" libelle={t("paie.attribution.fin")} value={fin} onChange={(z) => setFin(z.target.value)} />
        </div>
        <Bouton onClick={() => ajout.mutate()} disabled={ajout.isPending || membreId === "" || elementId === ""}>{t("paie.attribution.ajouter")}</Bouton>
      </div>
      <MessageErreur erreur={ajout.error ?? personnel.error} />
    </Section>
  );
}
