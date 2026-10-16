import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { Bouton } from "../../composants/Bouton";
import { CaseACocher, Champ, Selection } from "../../composants/Champ";
import { MessageEnregistre, MessageErreur, Section } from "../../composants/Messages";
import { Tableau } from "../../composants/Tableau";
import { type Cle, t } from "../../i18n";
import type {
  ArrondiMoyenne,
  ContenuRegle,
  Cycle,
  FormuleMatiere,
  GroupeMatiere,
  ImpressionMoyenneAnnuelle,
  Matiere,
  Pedagogie as DonneesPedagogie,
  TraitementAbsence,
  TypeEvaluation,
  TypePeriode,
} from "../../types/gesco";
import { CYCLES } from "../scolarite/Cycles";
import { heures, lireHeures, lirePoints, points } from "./saisies";

type Onglet = "programme" | "periodes" | "regles" | "matieres";

const ONGLETS: readonly [Onglet, Cle][] = [
  ["programme", "pedagogie.onglet.programme"],
  ["periodes", "pedagogie.onglet.periodes"],
  ["regles", "pedagogie.onglet.regles"],
  ["matieres", "pedagogie.onglet.matieres"],
];

const GROUPES: Record<GroupeMatiere, Cle> = {
  litteraire: "matiere.groupe.litteraire",
  scientifique: "matiere.groupe.scientifique",
  autre: "matiere.groupe.autre",
};

const TYPES: Record<TypeEvaluation, Cle> = {
  devoir: "evaluation.type.devoir",
  composition: "evaluation.type.composition",
};

const FORMULES: Record<FormuleMatiere, Cle> = {
  classe_et_composition: "regle.formule.classe_et_composition",
  somme_des_baremes: "regle.formule.somme_des_baremes",
};

const ABSENCES: Record<TraitementAbsence, Cle> = {
  exclu: "regle.absent.exclu",
  zero: "regle.absent.zero",
};

const ARRONDIS: Record<ArrondiMoyenne, Cle> = {
  troncature: "regle.arrondi.troncature",
  au_plus_proche: "regle.arrondi.au_plus_proche",
  par_exces: "regle.arrondi.par_exces",
};

const ANNUELLES: Record<ImpressionMoyenneAnnuelle, Cle> = {
  classes_examen: "regle.annuelle.classes_examen",
  toutes: "regle.annuelle.toutes",
};

const CYCLES_NOTES: readonly Cycle[] = ["premier_cycle", "second_cycle", "lycee", "technique"];

/** Excellent, Très bien, Bien, Assez bien et Passable : fixes ; l'école règle ce qui suit. */
const MENTIONS_FIXES = 5;

/** Instant d'un enregistrement, lu au moment où il réussit. */
function maintenant(): number {
  return Date.now();
}

function useRafraichir() {
  const client = useQueryClient();
  return () => void client.invalidateQueries({ queryKey: ["pedagogie"] });
}

/** Matières et notation (SPEC_PHASE_4 §2) : programme et coefficients de chaque niveau, périodes,
 *  règles d'évaluation, matières. */
export function Pedagogie() {
  const annees = useQuery({ queryKey: ["annees"], queryFn: () => appeler("annees_lister", null) });
  const [annee, setAnnee] = useState<string | null>(null);
  const anneeId = annee ?? annees.data?.find((a) => a.est_courante)?.id ?? annees.data?.[0]?.id ?? null;
  const donnees = useQuery({
    queryKey: ["pedagogie", anneeId],
    queryFn: () => appeler("pedagogie_lire", { id: anneeId ?? "" }),
    enabled: anneeId !== null,
  });
  const [onglet, setOnglet] = useState<Onglet>("programme");
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("pedagogie.titre")}</h1>
      <div className="flex flex-wrap items-end gap-3">
        <Selection
          id="pedagogie-annee"
          libelle={t("pedagogie.annee")}
          valeur={anneeId ?? ""}
          surChangement={setAnnee}
          options={(annees.data ?? []).map((a) => ({ valeur: a.id, libelle: a.libelle }))}
        />
        <div className="flex flex-wrap gap-2" role="tablist">
          {ONGLETS.map(([id, cle]) => (
            <Bouton key={id} variante={onglet === id ? "principal" : "secondaire"} onClick={() => setOnglet(id)}>
              {t(cle)}
            </Bouton>
          ))}
        </div>
      </div>
      <MessageErreur erreur={donnees.error} />
      {donnees.data === undefined || anneeId === null ? null : (
        <>
          {onglet === "programme" ? <Programme anneeId={anneeId} donnees={donnees.data} /> : null}
          {onglet === "periodes" ? <Periodes anneeId={anneeId} donnees={donnees.data} /> : null}
          {onglet === "regles" ? <Regles anneeId={anneeId} donnees={donnees.data} /> : null}
          {onglet === "matieres" ? <Matieres matieres={donnees.data.matieres} /> : null}
        </>
      )}
    </div>
  );
}

/** Niveaux notés (pas le jardin), actifs, avec leur cycle. */
function niveauxNotes(donnees: DonneesPedagogie) {
  return donnees.niveaux.filter((n) => n.actif && n.cycle !== "jardin");
}

interface LigneSaisie {
  inclus: boolean;
  coefficient: string;
  heures: string;
}

function Programme({ anneeId, donnees }: { anneeId: string; donnees: DonneesPedagogie }) {
  const rafraichir = useRafraichir();
  const niveaux = niveauxNotes(donnees);
  const [niveauId, setNiveauId] = useState(niveaux[0]?.id ?? "");
  const niveau = niveaux.find((n) => n.id === niveauId);
  const series = donnees.series.filter((s) => s.niveau_id === niveauId && s.actif);
  const [serieChoisie, setSerieChoisie] = useState<string | null>(null);
  const serieId = series.length === 0 ? null : (series.find((s) => s.id === serieChoisie)?.id ?? series[0]?.id ?? null);
  const actuel = donnees.programme.filter((l) => l.niveau_id === niveauId && l.serie_id === serieId);
  const initial = (): Record<string, LigneSaisie> =>
    Object.fromEntries(
      donnees.matieres.map((m) => {
        const l = actuel.find((x) => x.matiere_id === m.id);
        return [m.id, { inclus: l !== undefined, coefficient: l?.coefficient == null ? "" : String(l.coefficient), heures: l?.minutes_hebdo == null ? "" : heures(l.minutes_hebdo) }];
      }),
    );
  const cle = `${niveauId}-${serieId ?? ""}-${actuel.map((l) => l.id).join()}`;
  const [saisie, setSaisie] = useState<{ cle: string; lignes: Record<string, LigneSaisie> }>({ cle, lignes: initial() });
  const lignes = saisie.cle === cle ? saisie.lignes : initial();
  const changer = (id: string, l: Partial<LigneSaisie>) => {
    const base = lignes[id] ?? { inclus: false, coefficient: "", heures: "" };
    setSaisie({ cle, lignes: { ...lignes, [id]: { ...base, ...l } } });
  };
  const premierCycle = niveau?.cycle === "premier_cycle";
  const [enregistre, setEnregistre] = useState<number | null>(null);
  const enregistrer = useMutation({
    mutationFn: () => {
      const choisies = donnees.matieres
        .filter((m) => lignes[m.id]?.inclus === true)
        .map((m) => {
          const l = lignes[m.id];
          const coef = l?.coefficient.trim() ?? "";
          return {
            matiere_id: m.id,
            coefficient: premierCycle || coef === "" ? null : Number.parseInt(coef, 10),
            minutes_hebdo: l === undefined || l.heures.trim() === "" ? null : lireHeures(l.heures),
          };
        });
      return appeler("programme_enregistrer", { annee_id: anneeId, niveau_id: niveauId, serie_id: serieId, lignes: choisies });
    },
    onSuccess: () => {
      setEnregistre(maintenant());
      rafraichir();
    },
  });
  const reprendre = useMutation({ mutationFn: () => appeler("programme_reprendre", { id: anneeId }), onSuccess: rafraichir });
  return (
    <Section titre={t("pedagogie.onglet.programme")}>
      <p className="text-sm text-muet">{t(premierCycle ? "programme.aide_premier_cycle" : "programme.aide")}</p>
      <div className="flex flex-wrap items-end gap-3">
        <Selection id="programme-niveau" libelle={t("programme.niveau")} valeur={niveauId} surChangement={setNiveauId} options={niveaux.map((n) => ({ valeur: n.id, libelle: `${n.libelle} · ${t(CYCLES[n.cycle])}` }))} />
        {series.length === 0 ? null : (
          <Selection id="programme-serie" libelle={t(niveau?.cycle === "technique" ? "classes.filiere" : "programme.serie")} valeur={serieId ?? ""} surChangement={setSerieChoisie} options={series.map((s) => ({ valeur: s.id, libelle: s.libelle }))} />
        )}
      </div>
      <table className="w-full text-left text-sm">
        <thead>
          <tr className="text-muet">
            <th className="px-2 py-1 font-normal">{t("programme.matiere")}</th>
            {premierCycle ? null : <th className="px-2 py-1 font-normal">{t("programme.coefficient")}</th>}
            <th className="px-2 py-1 font-normal">{t("programme.heures")}</th>
          </tr>
        </thead>
        <tbody>
          {donnees.matieres
            .filter((m) => m.active || lignes[m.id]?.inclus === true)
            .map((m) => {
              const l = lignes[m.id];
              return (
                <tr key={m.id} className="border-b border-muet/30">
                  <td className="px-2 py-1">
                    <CaseACocher id={`programme-${m.code}`} libelle={m.libelle} coche={l?.inclus === true} surChangement={(c) => changer(m.id, { inclus: c })} />
                  </td>
                  {premierCycle ? null : (
                    <td className="px-2 py-1">
                      <input
                        id={`programme-${m.code}-coefficient`}
                        aria-label={`${t("programme.coefficient")} ${m.libelle}`}
                        inputMode="numeric"
                        disabled={l?.inclus !== true}
                        className="w-16 rounded border border-muet bg-surface px-2 py-1 text-primaire"
                        value={l?.coefficient ?? ""}
                        onChange={(e) => changer(m.id, { coefficient: e.target.value.replace(/\D/g, "") })}
                      />
                    </td>
                  )}
                  <td className="px-2 py-1">
                    <input
                      id={`programme-${m.code}-heures`}
                      aria-label={`${t("programme.heures")} ${m.libelle}`}
                      disabled={l?.inclus !== true}
                      className="w-24 rounded border border-muet bg-surface px-2 py-1 text-primaire"
                      value={l?.heures ?? ""}
                      onChange={(e) => changer(m.id, { heures: e.target.value })}
                    />
                  </td>
                </tr>
              );
            })}
        </tbody>
      </table>
      <MessageErreur erreur={enregistrer.error ?? reprendre.error} />
      <MessageEnregistre instant={enregistre} />
      {reprendre.data === undefined ? null : <p role="status" className="text-sm text-vert">{t("programme.repris", { nombre: String(reprendre.data) })}</p>}
      <div className="flex flex-wrap gap-2">
        <Bouton onClick={() => enregistrer.mutate()} disabled={enregistrer.isPending}>{t("commun.enregistrer")}</Bouton>
        <Bouton variante="secondaire" onClick={() => reprendre.mutate()} disabled={reprendre.isPending}>{t("programme.reprendre")}</Bouton>
      </div>
    </Section>
  );
}

function Periodes({ anneeId, donnees }: { anneeId: string; donnees: DonneesPedagogie }) {
  const rafraichir = useRafraichir();
  const niveaux = niveauxNotes(donnees);
  const [niveauId, setNiveauId] = useState(niveaux[0]?.id ?? "");
  const niveau = niveaux.find((n) => n.id === niveauId);
  const periodes = donnees.periodes.filter((p) => p.periode.niveau_id === niveauId);
  const cle = periodes.map((p) => `${p.periode.id}${p.periode.debut}${p.periode.fin}${p.periode.ponderation}`).join();
  type Saisie = { debut: string; fin: string; ponderation: string };
  const initial = (): Saisie[] => periodes.map((p) => ({ debut: p.periode.debut, fin: p.periode.fin, ponderation: String(p.periode.ponderation) }));
  const [saisie, setSaisie] = useState<{ cle: string; lignes: Saisie[] }>({ cle, lignes: initial() });
  const lignes = saisie.cle === cle ? saisie.lignes : initial();
  const changer = (i: number, s: Partial<Saisie>) => setSaisie({ cle, lignes: lignes.map((l, j) => (j === i ? { ...l, ...s } : l)) });
  const [enregistre, setEnregistre] = useState<number | null>(null);
  const enregistrer = useMutation({
    mutationFn: () =>
      appeler("calendrier_modifier", {
        annee_id: anneeId,
        niveau_id: niveauId,
        periodes: periodes.map((p, i) => ({ id: p.periode.id, debut: lignes[i]?.debut ?? "", fin: lignes[i]?.fin ?? "", ponderation: Number.parseInt(lignes[i]?.ponderation ?? "1", 10) || 0 })),
      }),
    onSuccess: () => {
      setEnregistre(maintenant());
      rafraichir();
    },
  });
  const definir = useMutation({
    mutationFn: ({ nature, nombre }: { nature: TypePeriode; nombre: number }) => appeler("periodes_definir", { annee_id: anneeId, niveau_id: niveauId, nature, nombre }),
    onSuccess: rafraichir,
  });
  const aide: Cle = niveau?.cycle === "premier_cycle" ? "periodes.aide_premier_cycle" : niveau?.cycle === "technique" ? "periodes.aide_technique" : "periodes.aide";
  return (
    <Section titre={t("pedagogie.onglet.periodes")}>
      <div className="flex flex-wrap items-end gap-3">
        <Selection id="periodes-niveau" libelle={t("programme.niveau")} valeur={niveauId} surChangement={setNiveauId} options={niveaux.map((n) => ({ valeur: n.id, libelle: `${n.libelle} · ${t(CYCLES[n.cycle])}` }))} />
        {niveau === undefined || niveau.cycle === "premier_cycle" ? null : (
          <>
            {niveau.cycle === "technique" ? (
              <Bouton variante="secondaire" onClick={() => definir.mutate({ nature: "semestre", nombre: 2 })} disabled={definir.isPending}>{t("periodes.deux_semestres")}</Bouton>
            ) : null}
            <Bouton variante="secondaire" onClick={() => definir.mutate({ nature: "trimestre", nombre: 3 })} disabled={definir.isPending}>{t("periodes.trois_trimestres")}</Bouton>
            <Bouton variante="secondaire" onClick={() => definir.mutate({ nature: "trimestre", nombre: 2 })} disabled={definir.isPending}>{t("periodes.deux_trimestres")}</Bouton>
          </>
        )}
      </div>
      <p className="text-sm text-muet">{t(aide)}</p>
      <ul className="flex flex-col gap-2">
        {periodes.map((p, i) => (
          <li key={p.periode.id} className="flex flex-wrap items-end gap-3">
            <span className="w-32 text-sm font-medium text-encre">{p.libelle}</span>
            <Champ id={`periode-${p.periode.rang}-debut`} type="date" libelle={t("periodes.debut")} value={lignes[i]?.debut ?? ""} onChange={(e) => changer(i, { debut: e.target.value })} />
            <Champ id={`periode-${p.periode.rang}-fin`} type="date" libelle={t("periodes.fin")} value={lignes[i]?.fin ?? ""} onChange={(e) => changer(i, { fin: e.target.value })} />
            <div className="w-24">
              <Champ id={`periode-${p.periode.rang}-poids`} inputMode="numeric" libelle={t("periodes.poids")} value={lignes[i]?.ponderation ?? ""} onChange={(e) => changer(i, { ponderation: e.target.value.replace(/\D/g, "") })} />
            </div>
          </li>
        ))}
      </ul>
      <MessageErreur erreur={enregistrer.error ?? definir.error} />
      <MessageEnregistre instant={enregistre} />
      <div>
        <Bouton onClick={() => enregistrer.mutate()} disabled={enregistrer.isPending || periodes.length === 0}>{t("commun.enregistrer")}</Bouton>
      </div>
    </Section>
  );
}

function Regles({ anneeId, donnees }: { anneeId: string; donnees: DonneesPedagogie }) {
  const rafraichir = useRafraichir();
  const [cycle, setCycle] = useState<Cycle>("premier_cycle");
  const [texteSeuils, setTexteSeuils] = useState<Record<string, string>>({});
  const regle = donnees.regles.find((r) => r.cycle === cycle);
  const cle = `${cycle}-${regle?.version ?? 0}`;
  const [saisie, setSaisie] = useState<{ cle: string; contenu: ContenuRegle | null }>({ cle, contenu: regle?.contenu ?? null });
  const contenu = saisie.cle === cle ? saisie.contenu : (regle?.contenu ?? null);
  const changer = (c: Partial<ContenuRegle>) => {
    if (contenu !== null) setSaisie({ cle, contenu: { ...contenu, ...c } });
  };
  const bareme = cycle === "premier_cycle" ? 1000 : 2000;
  const enregistrer = useMutation({
    mutationFn: () => appeler("regle_enregistrer", { annee_id: anneeId, cycle, contenu: contenu as ContenuRegle }),
    onSuccess: () => {
      setTexteSeuils({});
      rafraichir();
    },
  });
  if (contenu === null) return null;
  const typeActif = (type: TypeEvaluation) => contenu.types.find((x) => x.type_evaluation === type);
  return (
    <Section titre={t("pedagogie.onglet.regles")}>
      <div className="flex flex-wrap items-end gap-3">
        <Selection id="regles-cycle" libelle={t("regle.cycle")} valeur={cycle} surChangement={(c) => {
            setTexteSeuils({});
            setCycle(c as Cycle);
          }} options={CYCLES_NOTES.filter((c) => c !== "technique" || niveauxNotes(donnees).some((n) => n.cycle === "technique")).map((c) => ({ valeur: c, libelle: t(CYCLES[c]) }))} />
        {regle === undefined ? null : <p className="text-sm text-muet">{t("regle.version", { version: String(regle.version), bareme: points(bareme) })}</p>}
      </div>
      <fieldset className="flex flex-col gap-2">
        <legend className="text-sm text-encre">{t("regle.types")}</legend>
        {(["devoir", "composition"] as const).map((type) => {
          const actif = typeActif(type);
          return (
            <div key={type} className="flex flex-wrap items-end gap-3">
              <div className="w-36">
              <CaseACocher
                id={`regle-type-${type}`}
                libelle={t(TYPES[type])}
                coche={actif !== undefined}
                surChangement={(c) => changer({ types: c ? [...contenu.types, { type_evaluation: type, bareme }] : contenu.types.filter((x) => x.type_evaluation !== type) })}
              />
              </div>
              {actif === undefined ? null : (
                <div key={`${cle}-${type}`} className="w-28">
                  <Champ
                    id={`regle-type-${type}-bareme`}
                    libelle={t("regle.bareme")}
                    defaultValue={points(actif.bareme)}
                    onBlur={(e) => {
                      const v = lirePoints(e.target.value);
                      if (v !== null) changer({ types: contenu.types.map((x) => (x.type_evaluation === type ? { ...x, bareme: v } : x)) });
                    }}
                  />
                </div>
              )}
            </div>
          );
        })}
      </fieldset>
      <div className="flex flex-wrap gap-3">
        <Selection id="regle-formule" libelle={t("regle.formule")} valeur={contenu.formule} surChangement={(v) => changer({ formule: v as FormuleMatiere })} options={Object.entries(FORMULES).map(([v, c]) => ({ valeur: v, libelle: t(c) }))} />
        <Selection id="regle-absent" libelle={t("regle.absent")} valeur={contenu.absent_non_justifie} surChangement={(v) => changer({ absent_non_justifie: v as TraitementAbsence })} options={Object.entries(ABSENCES).map(([v, c]) => ({ valeur: v, libelle: t(c) }))} />
        <Selection id="regle-arrondi" libelle={t("regle.arrondi")} valeur={contenu.arrondi ?? "troncature"} surChangement={(v) => changer({ arrondi: v as ArrondiMoyenne })} options={Object.entries(ARRONDIS).map(([v, c]) => ({ valeur: v, libelle: t(c) }))} />
        <Selection id="regle-annuelle" libelle={t("regle.annuelle")} valeur={contenu.moyenne_annuelle} surChangement={(v) => changer({ moyenne_annuelle: v as ImpressionMoyenneAnnuelle })} options={Object.entries(ANNUELLES).map(([v, c]) => ({ valeur: v, libelle: t(c) }))} />
      </div>
      <CaseACocher id="regle-tronquer" libelle={t("regle.tronquer")} coche={contenu.tronquer_avant_report} surChangement={(c) => changer({ tronquer_avant_report: c })} />
      <CaseACocher id="regle-arrivee" libelle={t("regle.classer_arrivee")} coche={contenu.classer_arrivee_en_cours} surChangement={(c) => changer({ classer_arrivee_en_cours: c })} />
      <fieldset className="flex flex-col gap-2">
        <legend className="text-sm text-encre">{t("regle.mentions", { bareme: points(bareme) })}</legend>
        <ul className="flex flex-col gap-1 text-sm text-encre">
          {contenu.mentions.slice(0, MENTIONS_FIXES).map((m) => (
            <li key={m.libelle}>{t("regle.mention_fixe", { libelle: m.libelle, seuil: points(m.seuil) })}</li>
          ))}
        </ul>
        <p className="text-sm text-muet">{t("regle.mentions_basses")}</p>
        {contenu.mentions.map((m, i) =>
          i < MENTIONS_FIXES ? null : (
            <div key={i} className="flex flex-wrap items-end gap-3">
              <div className="w-24">
                <Champ
                  id={`regle-mention-${i}-seuil`}
                  libelle={t("regle.seuil")}
                  value={texteSeuils[i] ?? points(m.seuil)}
                  onChange={(e) => {
                    setTexteSeuils({ ...texteSeuils, [i]: e.target.value });
                    const v = lirePoints(e.target.value);
                    if (v !== null) changer({ mentions: contenu.mentions.map((x, j) => (j === i ? { ...x, seuil: v } : x)) });
                  }}
                />
              </div>
              <Champ id={`regle-mention-${i}-libelle`} libelle={t("regle.mention")} value={m.libelle} onChange={(e) => changer({ mentions: contenu.mentions.map((x, j) => (j === i ? { ...x, libelle: e.target.value } : x)) })} />
              <Bouton variante="secondaire" onClick={() => changer({ mentions: contenu.mentions.filter((_, j) => j !== i) })}>{t("commun.retirer")}</Bouton>
            </div>
          ),
        )}
        <div>
          <Bouton variante="secondaire" onClick={() => changer({ mentions: [...contenu.mentions, { seuil: 0, libelle: "" }] })}>{t("regle.ajouter_mention")}</Bouton>
        </div>
      </fieldset>
      <MessageErreur erreur={enregistrer.error} />
      {enregistrer.data === undefined ? null : <p role="status" className="text-sm text-vert">{t("regle.enregistree", { version: String(enregistrer.data.version) })}</p>}
      <div>
        <Bouton onClick={() => enregistrer.mutate()} disabled={enregistrer.isPending}>{t("commun.enregistrer")}</Bouton>
      </div>
    </Section>
  );
}

const MATIERE_VIDE = { id: null as string | null, code: "", libelle: "", libelle_court: "", groupe: "autre" as GroupeMatiere, active: true };

function Matieres({ matieres }: { matieres: readonly Matiere[] }) {
  const rafraichir = useRafraichir();
  const [fiche, setFiche] = useState(MATIERE_VIDE);
  const enregistrer = useMutation({
    mutationFn: () => appeler("matiere_enregistrer", fiche),
    onSuccess: () => {
      setFiche(MATIERE_VIDE);
      rafraichir();
    },
  });
  return (
    <Section titre={t("pedagogie.onglet.matieres")}>
      <Tableau
        nomExport="matieres"
        titre={t("pedagogie.onglet.matieres")}
        lignes={matieres}
        cle={(m) => m.id}
        surSelection={(m) => setFiche({ id: m.id, code: m.code, libelle: m.libelle, libelle_court: m.libelle_court, groupe: m.groupe, active: m.active })}
        colonnes={[
          { titre: t("matiere.code"), valeur: (m) => m.code },
          { titre: t("matiere.libelle"), valeur: (m) => m.libelle },
          { titre: t("matiere.libelle_court"), valeur: (m) => m.libelle_court },
          { titre: t("matiere.groupe"), valeur: (m) => t(GROUPES[m.groupe]) },
          { titre: t("matiere.etat"), valeur: (m) => t(m.active ? "matiere.active" : "matiere.inactive") },
        ]}
      />
      <h3 className="text-base font-semibold text-encre">{t(fiche.id === null ? "matiere.nouvelle" : "matiere.modifier")}</h3>
      <div className="flex flex-wrap items-end gap-3">
        <div className="w-28">
          <Champ id="matiere-code" libelle={t("matiere.code")} value={fiche.code} onChange={(e) => setFiche({ ...fiche, code: e.target.value.toUpperCase() })} />
        </div>
        <Champ id="matiere-libelle" libelle={t("matiere.libelle")} value={fiche.libelle} onChange={(e) => setFiche({ ...fiche, libelle: e.target.value })} />
        <Champ id="matiere-court" libelle={t("matiere.libelle_court")} value={fiche.libelle_court} onChange={(e) => setFiche({ ...fiche, libelle_court: e.target.value })} />
        <Selection id="matiere-groupe" libelle={t("matiere.groupe")} valeur={fiche.groupe} surChangement={(g) => setFiche({ ...fiche, groupe: g as GroupeMatiere })} options={Object.entries(GROUPES).map(([v, c]) => ({ valeur: v, libelle: t(c) }))} />
        <CaseACocher id="matiere-active" libelle={t("matiere.active")} coche={fiche.active} surChangement={(c) => setFiche({ ...fiche, active: c })} />
      </div>
      <MessageErreur erreur={enregistrer.error} />
      <div className="flex flex-wrap gap-2">
        <Bouton onClick={() => enregistrer.mutate()} disabled={enregistrer.isPending}>{t("commun.enregistrer")}</Bouton>
        {fiche.id === null ? null : <Bouton variante="secondaire" onClick={() => setFiche(MATIERE_VIDE)}>{t("commun.annuler")}</Bouton>}
      </div>
    </Section>
  );
}
