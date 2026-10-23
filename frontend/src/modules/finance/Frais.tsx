import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { commandeAutorisee } from "../../app/navigation";
import { Bouton } from "../../composants/Bouton";
import { CaseACocher, Champ, Selection } from "../../composants/Champ";
import { MessageEnregistre, MessageErreur, Section } from "../../composants/Messages";
import { type Cle, dateCivile, francs, t, texteDeCle } from "../../i18n";
import { TYPES_REMISE, valeurRemise } from "./FraisEleve";
import type { Cycle, EtatGrille, Exigibilite, GrilleAffichee, Referentiel, SaisiePlan, TypeFrais } from "../../types/gesco";
import { CYCLES } from "../scolarite/Cycles";
import { auJour, lireJour } from "./echeances";
import { deplier, type GroupeLigne, grouper, resumeNiveaux } from "./groupes";

const TYPES_FRAIS: Record<TypeFrais, Cle> = {
  inscription: "frais.type.inscription",
  scolarite: "frais.type.scolarite",
  cantine: "frais.type.cantine",
  transport: "frais.type.transport",
  tenue: "frais.type.tenue",
  examen: "frais.type.examen",
  autre: "frais.type.autre",
};

const ETATS: Record<EtatGrille, Cle> = {
  brouillon: "frais.etat.brouillon",
  en_vigueur: "frais.etat.en_vigueur",
  remplacee: "frais.etat.remplacee",
};

const EXIGIBILITES: Record<Exigibilite, Cle> = {
  une_fois: "frais.exigibilite.une_fois",
  selon_plan: "frais.exigibilite.selon_plan",
};

/** Montant saisi : chiffres seulement, en francs entiers (R-11). */
function lireFrancs(texte: string): number {
  const chiffres = texte.replace(/\D/g, "");
  return chiffres === "" ? 0 : Number.parseInt(chiffres, 10);
}

/** Frais de l'année et plans de paiement (SPEC_PHASE_3 §3). */
export function Frais() {
  const client = useQueryClient();
  const session = useQuery({ queryKey: ["session"], queryFn: () => appeler("session_courante", null) });
  const annees = useQuery({ queryKey: ["annees"], queryFn: () => appeler("annees_lister", null) });
  const referentiel = useQuery({ queryKey: ["referentiel"], queryFn: () => appeler("niveaux_lister", null) });
  const [annee, setAnnee] = useState<string | null>(null);
  const anneeId = annee ?? annees.data?.find((a) => a.est_courante)?.id ?? annees.data?.[0]?.id ?? null;
  const grilles = useQuery({
    queryKey: ["grilles", anneeId],
    queryFn: () => appeler("grilles_lister", { annee_id: anneeId ?? "" }),
    enabled: anneeId !== null,
  });
  // L'année précédente, pour reprendre ses frais quand l'année n'a pas encore de grille.
  const listeAnnees = annees.data ?? [];
  const courante = listeAnnees.find((a) => a.id === anneeId);
  const precedente = listeAnnees
    .filter((a) => courante !== undefined && a.libelle < courante.libelle)
    .sort((a, b) => b.libelle.localeCompare(a.libelle))[0];
  const grillesPrecedentes = useQuery({
    queryKey: ["grilles", precedente?.id],
    queryFn: () => appeler("grilles_lister", { annee_id: precedente?.id ?? "" }),
    enabled: precedente !== undefined && (grilles.data ?? []).length === 0,
  });
  const [choisie, setChoisie] = useState<string | null>(null);
  const [sansScolarite, setSansScolarite] = useState<string[]>([]);
  const gere = commandeAutorisee("grille_creer", session.data?.permissions ?? []);
  const liste = grilles.data ?? [];
  const affichee = liste.find((g) => g.grille.id === choisie) ?? liste[0];
  const creation = useMutation({
    mutationFn: (copieDe: string | null) => appeler("grille_creer", { annee_id: anneeId ?? "", copie_de: copieDe }),
    onSuccess: (g) => {
      void client.invalidateQueries({ queryKey: ["grilles"] });
      setChoisie(g.grille.id);
    },
  });
  const aReprendre = (grillesPrecedentes.data ?? []).find((g) => g.grille.etat === "en_vigueur");
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("frais.titre")}</h1>
      <div className="flex flex-wrap items-end gap-3">
        <div className="w-60">
          <Selection
            id="frais-annee"
            libelle={t("classes.annee")}
            valeur={anneeId ?? ""}
            surChangement={(v) => {
              setAnnee(v);
              setChoisie(null);
            }}
            options={listeAnnees.map((a) => ({ valeur: a.id, libelle: a.libelle }))}
          />
        </div>
        {liste.length > 1 ? (
          <Selection
            id="frais-grille"
            libelle={t("frais.version")}
            valeur={affichee?.grille.id ?? ""}
            surChangement={setChoisie}
            options={liste.map((g) => ({ valeur: g.grille.id, libelle: `${g.grille.libelle} · ${t(ETATS[g.grille.etat])}` }))}
          />
        ) : null}
        {gere && anneeId !== null ? (
          <>
            {liste.length === 0 ? <Bouton onClick={() => creation.mutate(null)} disabled={creation.isPending}>{t("frais.nouvelle")}</Bouton> : null}
            {liste.length === 0 && aReprendre !== undefined ? (
              <Bouton variante="secondaire" onClick={() => creation.mutate(aReprendre.grille.id)} disabled={creation.isPending}>
                {t("frais.reprendre", { annee: precedente?.libelle ?? "" })}
              </Bouton>
            ) : null}
            {affichee !== undefined && affichee.grille.etat !== "brouillon" && !liste.some((g) => g.grille.etat === "brouillon") ? (
              <Bouton variante="secondaire" onClick={() => creation.mutate(affichee.grille.id)} disabled={creation.isPending}>
                {t("frais.nouvelle_version")}
              </Bouton>
            ) : null}
          </>
        ) : null}
      </div>
      {anneeId === null ? <p className="text-sm text-ocre">{t("classes.aucune_annee")}</p> : null}
      {anneeId !== null && liste.length === 0 && grilles.isSuccess ? <p className="text-sm text-muet">{t("frais.aucune_grille")}</p> : null}
      <MessageErreur erreur={annees.error ?? referentiel.error ?? grilles.error ?? creation.error} />
      {gere ? <Rattrapage /> : null}
      {commandeAutorisee("remises_en_attente", session.data?.permissions ?? []) ? <RemisesEnAttente /> : null}
      {sansScolarite.length > 0 ? (
        <p role="status" className="text-sm text-ocre">{t("frais.niveaux_sans_scolarite", { niveaux: sansScolarite.join(", ") })}</p>
      ) : null}
      {affichee !== undefined && referentiel.data !== undefined ? (
        gere && affichee.grille.etat === "brouillon" ? (
          <EditionGrille key={affichee.grille.id} affichee={affichee} referentiel={referentiel.data} surVigueur={setSansScolarite} />
        ) : (
          <LectureGrille affichee={affichee} referentiel={referentiel.data} />
        )
      ) : null}
    </div>
  );
}

function libelleNiveau(referentiel: Referentiel, niveauId: string, serieId: string | null): string {
  const niveau = referentiel.niveaux.find((n) => n.id === niveauId)?.libelle ?? "";
  const serie = serieId === null ? null : referentiel.series.find((s) => s.id === serieId)?.code;
  return serie === null || serie === undefined ? niveau : `${niveau} · ${serie}`;
}

/** Grille en vigueur ou remplacée : elle ne change plus (SPEC_PHASE_3 §3.2). */
function LectureGrille({ affichee, referentiel }: { affichee: GrilleAffichee; referentiel: Referentiel }) {
  const { grille, contenu } = affichee;
  return (
    <Section titre={`${grille.libelle} · ${t(ETATS[grille.etat])}`}>
      <table className="w-full text-left font-condensee text-sm tabular-nums">
        <thead>
          <tr className="text-muet">
            {(["frais.niveau", "frais.type", "frais.libelle", "frais.montant", "frais.obligatoire", "frais.exigibilite"] as const).map((cle) => (
              <th key={cle} className="border-b border-muet px-2 py-1 font-normal">{t(cle)}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {grouper(contenu.lignes).map((l) => (
            <tr key={`${l.niveaux.join("-")}-${l.type_frais}-${l.libelle}`} className="border-b border-muet/30 text-encre">
              <td className="px-2 py-1">
                {l.serie_id === null ? resumeNiveaux(referentiel, l.niveaux, (c) => t(CYCLES[c])) : libelleNiveau(referentiel, l.niveaux[0] ?? "", l.serie_id)}
              </td>
              <td className="px-2 py-1">{t(TYPES_FRAIS[l.type_frais])}</td>
              <td className="px-2 py-1">{l.libelle}</td>
              <td className="px-2 py-1">{francs(l.montant)}</td>
              <td className="px-2 py-1">{t(l.obligatoire ? "commun.oui" : "commun.non")}</td>
              <td className="px-2 py-1">{t(EXIGIBILITES[l.exigibilite])}</td>
            </tr>
          ))}
        </tbody>
      </table>
      <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
        {contenu.plans.map((p) => (
          <div key={p.id} className="rounded border border-muet/30 p-3 text-sm">
            <p className="font-medium text-encre">{p.par_defaut ? t("frais.plan_par_defaut_libelle", { plan: p.libelle }) : p.libelle}</p>
            <ul className="mt-1 text-muet">
              {p.tranches.map((tr) => (
                <li key={`${tr.libelle}-${tr.echeance}`}>{`${tr.libelle} · ${dateCivile(tr.echeance)} · ${tr.part} %`}</li>
              ))}
            </ul>
          </div>
        ))}
      </div>
    </Section>
  );
}

function groupeVide(): GroupeLigne {
  return {
    niveaux: [],
    serie_id: null,
    type_frais: "scolarite",
    libelle: t("frais.type.scolarite"),
    montant: 0,
    obligatoire: true,
    exigibilite: "selon_plan",
  };
}

/** Niveaux d'un frais : une case par niveau actif, rangées par cycle ; un cycle se coche d'un
 *  geste. */
function ChoixNiveaux({ prefixe, referentiel, choisis, surChangement }: { prefixe: string; referentiel: Referentiel; choisis: readonly string[]; surChangement: (niveaux: string[]) => void }) {
  const cycles: Cycle[] = ["jardin", "premier_cycle", "second_cycle", "lycee", "technique"];
  return (
    <fieldset className="flex w-full flex-col gap-1">
      <legend className="text-sm text-encre">{t("frais.niveaux")}</legend>
      {cycles.map((cycle) => {
        const niveaux = referentiel.niveaux.filter((n) => n.actif && n.cycle === cycle);
        if (niveaux.length === 0) return null;
        const tous = niveaux.every((n) => choisis.includes(n.id));
        return (
          <div key={cycle} className="flex flex-wrap items-center gap-3">
            <button
              type="button"
              className="w-28 text-left text-sm text-primaire underline"
              onClick={() => surChangement(tous ? choisis.filter((id) => !niveaux.some((n) => n.id === id)) : [...new Set([...choisis, ...niveaux.map((n) => n.id)])])}
            >
              {t(CYCLES[cycle])}
            </button>
            {niveaux.map((n) => (
              <CaseACocher
                key={n.id}
                id={`${prefixe}-niveau-${n.code}`}
                libelle={n.libelle}
                coche={choisis.includes(n.id)}
                surChangement={(c) => surChangement(c ? [...choisis, n.id] : choisis.filter((id) => id !== n.id))}
              />
            ))}
          </div>
        );
      })}
    </fieldset>
  );
}

/** Brouillon : lignes et plans se modifient, s'enregistrent, puis la grille se met en vigueur. */
function EditionGrille({ affichee, referentiel, surVigueur }: { affichee: GrilleAffichee; referentiel: Referentiel; surVigueur: (niveaux: string[]) => void }) {
  const client = useQueryClient();
  const [libelle, setLibelle] = useState(affichee.grille.libelle);
  const [groupes, setGroupes] = useState<GroupeLigne[]>(() => grouper(affichee.contenu.lignes));
  const [plans, setPlans] = useState<SaisiePlan[]>(() => affichee.contenu.plans.map((p) => ({ libelle: p.libelle, par_defaut: p.par_defaut, tranches: p.tranches })));
  const [enregistre, setEnregistre] = useState<number | null>(null);
  const ordre = referentiel.niveaux.map((n) => n.id);
  const lignes = () => deplier(groupes, ordre);
  const incomplet = groupes.some((g) => g.niveaux.length === 0);
  const enregistrement = useMutation({
    mutationFn: () => appeler("grille_modifier", { id: affichee.grille.id, libelle, lignes: lignes(), plans }),
    onSuccess: () => {
      setEnregistre(Date.now());
      void client.invalidateQueries({ queryKey: ["grilles"] });
    },
  });
  const vigueur = useMutation({
    mutationFn: async () => {
      await appeler("grille_modifier", { id: affichee.grille.id, libelle, lignes: lignes(), plans });
      return appeler("grille_mettre_en_vigueur", { id: affichee.grille.id });
    },
    onSuccess: (r) => {
      surVigueur(r.niveaux_sans_scolarite);
      void client.invalidateQueries({ queryKey: ["grilles"] });
    },
  });
  const changerGroupe = (i: number, g: Partial<GroupeLigne>) => setGroupes(groupes.map((x, j) => (j === i ? { ...x, ...g } : x)));
  const changerPlan = (i: number, p: Partial<SaisiePlan>) => setPlans(plans.map((x, j) => (j === i ? { ...x, ...p } : x)));
  // Le jour des échéances, proposé d'après la première tranche : un clic met toutes les tranches
  // de tous les plans à ce jour de leur mois.
  const [jour, setJour] = useState(() => String(Number(plans[0]?.tranches[0]?.echeance.slice(8, 10) ?? "5") || 5));
  const jourLu = lireJour(jour);
  return (
    <div className="flex flex-col gap-4">
      <Section titre={t("frais.lignes")}>
        <div className="w-96">
          <Champ id="frais-libelle" libelle={t("frais.libelle_grille")} value={libelle} onChange={(e) => setLibelle(e.target.value)} />
        </div>
        <p className="text-sm text-muet">{t("frais.lignes_aide")}</p>
        {groupes.map((g, i) => {
          const seul = g.niveaux.length === 1 ? g.niveaux[0] : undefined;
          const series = seul === undefined ? [] : referentiel.series.filter((s) => s.niveau_id === seul && s.actif);
          return (
            <div key={i} className="flex flex-col gap-2 border-b border-muet/30 pb-3">
              <ChoixNiveaux prefixe={`frais-ligne-${i}`} referentiel={referentiel} choisis={g.niveaux} surChangement={(niveaux) => changerGroupe(i, { niveaux, serie_id: null })} />
              {g.niveaux.length === 0 ? <p className="text-sm text-ocre">{t("frais.aucun_niveau")}</p> : null}
              <div className="flex flex-wrap items-end gap-2">
                {series.length === 0 ? null : (
                  <Selection
                    id={`frais-ligne-${i}-serie`}
                    libelle={t("classes.serie")}
                    valeur={g.serie_id ?? ""}
                    surChangement={(v) => changerGroupe(i, { serie_id: v === "" ? null : v })}
                    options={[{ valeur: "", libelle: t("frais.toutes_series") }, ...series.map((x) => ({ valeur: x.id, libelle: x.code }))]}
                  />
                )}
                <Selection
                  id={`frais-ligne-${i}-type`}
                  libelle={t("frais.type")}
                  valeur={g.type_frais}
                  surChangement={(v) => {
                    const type = (Object.keys(TYPES_FRAIS) as TypeFrais[]).find((x) => x === v) ?? "autre";
                    changerGroupe(i, {
                      type_frais: type,
                      libelle: t(TYPES_FRAIS[type]),
                      exigibilite: type === "scolarite" ? "selon_plan" : "une_fois",
                      obligatoire: type === "inscription" || type === "scolarite",
                    });
                  }}
                  options={(Object.keys(TYPES_FRAIS) as TypeFrais[]).map((x) => ({ valeur: x, libelle: t(TYPES_FRAIS[x]) }))}
                />
                <Champ id={`frais-ligne-${i}-libelle`} libelle={t("frais.libelle")} value={g.libelle} onChange={(e) => changerGroupe(i, { libelle: e.target.value })} />
                <Champ
                  id={`frais-ligne-${i}-montant`}
                  libelle={t("frais.montant")}
                  inputMode="numeric"
                  value={g.montant === 0 ? "" : String(g.montant)}
                  onChange={(e) => changerGroupe(i, { montant: lireFrancs(e.target.value) })}
                />
                <CaseACocher id={`frais-ligne-${i}-obligatoire`} libelle={t("frais.obligatoire")} coche={g.obligatoire} surChangement={(c) => changerGroupe(i, { obligatoire: c })} />
                {g.type_frais === "scolarite" ? null : (
                  <Selection
                    id={`frais-ligne-${i}-exigibilite`}
                    libelle={t("frais.exigibilite")}
                    valeur={g.exigibilite}
                    surChangement={(v) => changerGroupe(i, { exigibilite: v === "selon_plan" ? "selon_plan" : "une_fois" })}
                    options={(Object.keys(EXIGIBILITES) as Exigibilite[]).map((x) => ({ valeur: x, libelle: t(EXIGIBILITES[x]) }))}
                  />
                )}
                <Bouton variante="secondaire" onClick={() => setGroupes(groupes.filter((_, j) => j !== i))}>{t("commun.retirer")}</Bouton>
              </div>
            </div>
          );
        })}
        <div><Bouton variante="secondaire" onClick={() => setGroupes([...groupes, groupeVide()])}>{t("frais.ajouter_ligne")}</Bouton></div>
      </Section>
      <Section titre={t("frais.plans")}>
        <p className="text-sm text-muet">{t("frais.plans_aide")}</p>
        <div className="flex flex-wrap items-end gap-2">
          <div className="w-32">
            <Champ id="frais-jour-echeance" libelle={t("frais.jour_du_mois")} inputMode="numeric" value={jour} onChange={(e) => setJour(e.target.value)} />
          </div>
          <Bouton
            variante="secondaire"
            disabled={jourLu === null}
            onClick={() => {
              if (jourLu !== null) setPlans(plans.map((p) => ({ ...p, tranches: p.tranches.map((tr) => ({ ...tr, echeance: auJour(tr.echeance, jourLu) })) })));
            }}
          >
            {t("frais.aligner_jour")}
          </Bouton>
        </div>
        <div className="grid grid-cols-1 gap-3 lg:grid-cols-2">
          {plans.map((p, i) => (
            <div key={i} className="flex flex-col gap-2 rounded border border-muet/30 p-3">
              <Champ id={`frais-plan-${i}-libelle`} libelle={t("frais.plan")} value={p.libelle} onChange={(e) => changerPlan(i, { libelle: e.target.value })} />
              <CaseACocher
                id={`frais-plan-${i}-defaut`}
                libelle={t("frais.case_par_defaut")}
                coche={p.par_defaut}
                surChangement={(c) => setPlans(plans.map((x, j) => ({ ...x, par_defaut: j === i ? c : c ? false : x.par_defaut })))}
              />
              {p.tranches.map((tr, k) => (
                <div key={k} className="flex items-end gap-2">
                  <div className="min-w-0 flex-1">
                    <Champ
                      id={`frais-plan-${i}-tranche-${k}-libelle`}
                      libelle={t("frais.tranche")}
                      value={tr.libelle}
                      onChange={(e) => changerPlan(i, { tranches: p.tranches.map((x, m) => (m === k ? { ...x, libelle: e.target.value } : x)) })}
                    />
                  </div>
                  <div className="w-40 shrink-0">
                    <Champ
                      id={`frais-plan-${i}-tranche-${k}-echeance`}
                      type="date"
                      libelle={t("frais.echeance")}
                      value={tr.echeance}
                      onChange={(e) => changerPlan(i, { tranches: p.tranches.map((x, m) => (m === k ? { ...x, echeance: e.target.value } : x)) })}
                    />
                  </div>
                  <div className="w-24 shrink-0">
                    <Champ
                      id={`frais-plan-${i}-tranche-${k}-part`}
                      libelle={t("frais.part")}
                      inputMode="numeric"
                      value={String(tr.part)}
                      onChange={(e) => changerPlan(i, { tranches: p.tranches.map((x, m) => (m === k ? { ...x, part: lireFrancs(e.target.value) } : x)) })}
                    />
                  </div>
                  <div className="shrink-0">
                    <Bouton variante="secondaire" onClick={() => changerPlan(i, { tranches: p.tranches.filter((_, m) => m !== k) })}>{t("commun.retirer")}</Bouton>
                  </div>
                </div>
              ))}
              <p className="text-sm text-muet">{t("frais.total_parts", { total: p.tranches.reduce((s, x) => s + x.part, 0) })}</p>
              <div className="flex flex-wrap gap-2">
                <Bouton
                  variante="secondaire"
                  onClick={() => changerPlan(i, { tranches: [...p.tranches, { libelle: t("frais.nouvelle_tranche"), echeance: p.tranches.at(-1)?.echeance ?? "", part: 0 }] })}
                >
                  {t("frais.ajouter_tranche")}
                </Bouton>
                <Bouton variante="secondaire" onClick={() => setPlans(plans.filter((_, j) => j !== i))}>{t("frais.retirer_plan")}</Bouton>
              </div>
            </div>
          ))}
        </div>
        {plans.length < 4 ? (
          <div>
            <Bouton
              variante="secondaire"
              onClick={() => setPlans([...plans, { libelle: t("frais.nouveau_plan"), par_defaut: plans.length === 0, tranches: [{ libelle: t("frais.nouvelle_tranche"), echeance: "", part: 100 }] }])}
            >
              {t("frais.ajouter_plan")}
            </Bouton>
          </div>
        ) : null}
      </Section>
      <MessageErreur erreur={enregistrement.error ?? vigueur.error} />
      <MessageEnregistre instant={enregistre} />
      <div className="flex flex-wrap gap-2">
        <Bouton onClick={() => enregistrement.mutate()} disabled={enregistrement.isPending || incomplet}>{t("commun.enregistrer")}</Bouton>
        <Bouton variante="secondaire" onClick={() => vigueur.mutate()} disabled={vigueur.isPending || incomplet}>{t("frais.mettre_en_vigueur")}</Bouton>
      </div>
      <p className="text-sm text-muet">{t("frais.vigueur_aide")}</p>
    </div>
  );
}

/** Inscriptions validées avant la facturation (SPEC_PHASE_3 §0.3, Q-50) : la direction voit ce
 *  qui sera facturé, puis facture tout d'un coup, après une sauvegarde. */
function Rattrapage() {
  const client = useQueryClient();
  const apercu = useQuery({ queryKey: ["rattrapage"], queryFn: () => appeler("factures_rattraper_apercu", null) });
  const rattrapage = useMutation({
    mutationFn: () => appeler("factures_rattraper", null),
    onSuccess: () => {
      for (const cle of ["rattrapage", "situation", "avertissements", "sauvegardes"]) void client.invalidateQueries({ queryKey: [cle] });
    },
  });
  const lignes = apercu.data ?? [];
  if (lignes.length === 0 && rattrapage.data === undefined) return null;
  const facturables = lignes.filter((l) => l.erreur === null).length;
  return (
    <Section titre={t("rattrapage.titre")}>
      {rattrapage.data === undefined ? null : (
        <p role="status" className="text-sm text-encre">{t("rattrapage.fait", { facturees: rattrapage.data.facturees, ignorees: rattrapage.data.ignorees })}</p>
      )}
      {lignes.length === 0 ? null : (
        <>
          <p className="text-sm text-muet">{t("rattrapage.explication")}</p>
          <ul className="flex flex-col gap-1 text-sm">
            {lignes.map((l) => (
              <li key={l.inscription_id} className={l.erreur === null ? "text-encre" : "text-rouge"}>
                {`${l.eleve} · ${l.classe} · ${l.total === null ? texteDeCle(l.erreur ?? "") : francs(l.total)}`}
              </li>
            ))}
          </ul>
          <div>
            <Bouton onClick={() => rattrapage.mutate()} disabled={rattrapage.isPending || facturables === 0}>
              {t("rattrapage.facturer", { nombre: facturables })}
            </Bouton>
          </div>
        </>
      )}
      <MessageErreur erreur={apercu.error ?? rattrapage.error} />
    </Section>
  );
}

/** Remises proposées par le secrétariat, à décider par la direction (SPEC_PHASE_3 §5). */
function RemisesEnAttente() {
  const client = useQueryClient();
  const attente = useQuery({ queryKey: ["remises-en-attente"], queryFn: () => appeler("remises_en_attente", null) });
  const decision = useMutation({
    mutationFn: (id: string) => appeler("remise_accorder", { id }),
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: ["remises-en-attente"] });
      void client.invalidateQueries({ queryKey: ["situation"] });
    },
  });
  const lignes = attente.data ?? [];
  if (lignes.length === 0) return null;
  return (
    <Section titre={t("remise.en_attente")}>
      <p className="text-sm text-muet">{t("remise.en_attente_aide")}</p>
      <ul className="flex flex-col gap-2 text-sm">
        {lignes.map((l) => (
          <li key={l.remise.id} className="flex flex-wrap items-center gap-3 text-encre">
            <span>{`${l.eleve} · ${l.classe} · ${t(TYPES_REMISE[l.remise.type_remise])} ${valeurRemise(l.remise.valeur)} · ${l.remise.motif}`}</span>
            <Bouton variante="secondaire" onClick={() => decision.mutate(l.remise.id)} disabled={decision.isPending}>{t("remise.accorder")}</Bouton>
          </li>
        ))}
      </ul>
      <MessageErreur erreur={attente.error ?? decision.error} />
    </Section>
  );
}
