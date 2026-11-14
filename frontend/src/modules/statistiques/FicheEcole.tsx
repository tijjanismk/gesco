import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { commandeAutorisee } from "../../app/navigation";
import { Bouton } from "../../composants/Bouton";
import { CaseACocher, Champ, Selection, ZoneTexte } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { t } from "../../i18n";
import type { FeuilleStatistique, FicheEcole as Donnees, RubriqueFiche, ThemeRubrique, TypeReponse } from "../../types/gesco";
import { THEMES_RUBRIQUES, TYPES_REPONSE, feuilleFiche, texteReponse } from "./fiche";

function useRafraichir() {
  const client = useQueryClient();
  return () => void client.invalidateQueries({ queryKey: ["fiche-ecole"] });
}

/** Onglet Établissement : la fiche de l'école par thème, réponses de l'année choisie, saisie
 *  sur place pour qui tient la fiche ; une réponse de l'an dernier est proposée à côté de la
 *  case, jamais recopiée d'elle-même. */
export function OngletEtablissement({ anneeId, surExporter }: { anneeId: string | null; surExporter: (titre: string, feuilles: FeuilleStatistique[]) => void }) {
  const fiche = useQuery({ queryKey: ["fiche-ecole", anneeId], queryFn: () => appeler("fiche_ecole_lire", { annee_id: anneeId }) });
  const session = useQuery({ queryKey: ["session"], queryFn: () => appeler("session_courante", null) });
  const peutModifier = commandeAutorisee("fiche_ecole_repondre", session.data?.permissions ?? []);
  const client = useQueryClient();
  const reprendre = useMutation({
    mutationFn: (rubriques: string[] | null) => appeler("fiche_ecole_reprendre", { annee_id: fiche.data?.annee_id ?? "", rubriques }),
    onSuccess: (donnees) => client.setQueryData(["fiche-ecole", anneeId], donnees),
  });
  const d = fiche.data;
  if (d === undefined) return <MessageErreur erreur={fiche.error} />;
  const actives = d.rubriques.filter((r) => r.retiree_le === null);
  const propositions = d.reponses.filter((r) => r.proposee !== null).length;
  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center gap-2">
        <Bouton variante="secondaire" onClick={() => surExporter(t("fiche.titre"), [feuilleFiche(d)])}>
          {t("commun.exporter_excel")}
        </Bouton>
        {peutModifier && propositions > 0 && d.annee_id !== null ? (
          <Bouton variante="secondaire" onClick={() => reprendre.mutate(null)} disabled={reprendre.isPending}>
            {t("fiche.reprendre_tout", { n: propositions })}
          </Bouton>
        ) : null}
      </div>
      <MessageErreur erreur={reprendre.error} />
      {THEMES_RUBRIQUES.map((theme) => {
        const du_theme = actives.filter((r) => r.theme === theme);
        if (du_theme.length === 0) return null;
        return (
          <Section key={theme} titre={t(`fiche.theme.${theme}`)}>
            <dl className="grid gap-x-4 gap-y-2 sm:grid-cols-[minmax(12rem,1fr)_2fr]">
              {du_theme.map((r) => (
                <LigneRubrique key={r.id} rubrique={r} donnees={d} anneeId={anneeId} peutModifier={peutModifier} surReprendre={() => reprendre.mutate([r.id])} />
              ))}
            </dl>
          </Section>
        );
      })}
      {peutModifier ? <RubriquesEcole rubriques={d.rubriques} /> : null}
    </div>
  );
}

function LigneRubrique({
  rubrique,
  donnees,
  anneeId,
  peutModifier,
  surReprendre,
}: {
  rubrique: RubriqueFiche;
  donnees: Donnees;
  anneeId: string | null;
  peutModifier: boolean;
  surReprendre: () => void;
}) {
  const client = useQueryClient();
  const reponse = donnees.reponses.find((x) => x.rubrique_id === rubrique.id);
  const valeur = reponse?.valeur ?? null;
  const repondre = useMutation({
    mutationFn: (v: string | null) => appeler("fiche_ecole_repondre", { rubrique_id: rubrique.id, annee_id: donnees.annee_id, valeur: v }),
    onSuccess: (d) => client.setQueryData(["fiche-ecole", anneeId], d),
  });
  const sansAnnee = rubrique.annuelle && donnees.annee_id === null;
  const id = `rubrique-${rubrique.id}`;
  return (
    <>
      <dt className="pt-1.5 text-sm text-encre">
        <label htmlFor={id}>{rubrique.libelle}</label>
        {rubrique.aide === "" ? null : <span className="block text-muet">{rubrique.aide}</span>}
      </dt>
      <dd className="flex flex-col gap-1">
        {peutModifier && !sansAnnee ? (
          <Saisie id={id} rubrique={rubrique} valeur={valeur} surEnregistrer={(v) => repondre.mutate(v)} />
        ) : (
          <span className="pt-1.5 text-sm text-primaire">{texteReponse(rubrique, valeur)}</span>
        )}
        {reponse?.proposee == null ? null : (
          <span className="flex flex-wrap items-center gap-2 text-sm text-muet">
            {t("fiche.an_dernier", { valeur: texteReponse(rubrique, reponse.proposee) })}
            {peutModifier ? (
              <button type="button" className="underline decoration-dotted underline-offset-2 hover:text-encre" onClick={surReprendre}>
                {t("fiche.reprendre")}
              </button>
            ) : null}
          </span>
        )}
        <MessageErreur erreur={repondre.error} />
      </dd>
    </>
  );
}

/** Case de saisie selon le type de la rubrique ; une case de texte s'enregistre en la quittant,
 *  une liste dès qu'on choisit. */
function Saisie({ id, rubrique, valeur, surEnregistrer }: { id: string; rubrique: RubriqueFiche; valeur: string | null; surEnregistrer: (v: string | null) => void }) {
  const [brouillon, setBrouillon] = useState<string | null>(null);
  const enregistrer = (v: string) => {
    setBrouillon(null);
    const propre = v.trim() === "" ? null : v;
    if (propre !== valeur) surEnregistrer(propre);
  };
  switch (rubrique.type_reponse) {
    case "oui_non":
    case "choix": {
      const options = rubrique.type_reponse === "oui_non" ? [{ valeur: "oui", libelle: t("commun.oui") }, { valeur: "non", libelle: t("commun.non") }] : rubrique.choix.map((c) => ({ valeur: c, libelle: c }));
      return (
        <select id={id} className="self-start rounded border border-muet bg-surface px-2 py-1.5 text-sm text-primaire" value={valeur ?? ""} onChange={(e) => enregistrer(e.target.value)}>
          <option value="" />
          {options.map((o) => (
            <option key={o.valeur} value={o.valeur}>
              {o.libelle}
            </option>
          ))}
        </select>
      );
    }
    case "plusieurs_choix": {
      const retenus = (valeur ?? "").split("\n").filter((c) => c !== "");
      return (
        <div id={id} className="flex flex-wrap gap-3">
          {rubrique.choix.map((c) => (
            <CaseACocher
              key={c}
              id={`${id}-${c}`}
              libelle={c}
              coche={retenus.includes(c)}
              surChangement={(coche) => enregistrer((coche ? [...retenus, c] : retenus.filter((x) => x !== c)).join("\n"))}
            />
          ))}
        </div>
      );
    }
    default:
      return (
        <input
          id={id}
          type={rubrique.type_reponse === "date" ? "date" : "text"}
          inputMode={rubrique.type_reponse === "nombre" ? "decimal" : undefined}
          className="rounded border border-muet bg-surface px-2 py-1.5 text-sm text-primaire"
          value={brouillon ?? valeur ?? ""}
          onChange={(e) => setBrouillon(e.target.value)}
          onBlur={(e) => enregistrer(e.target.value)}
        />
      );
  }
}

interface Formulaire {
  id: string | null;
  libelle: string;
  theme: ThemeRubrique;
  type_reponse: TypeReponse;
  choix: string;
  aide: string;
  annuelle: boolean;
}

const VIDE: Formulaire = { id: null, libelle: "", theme: "autres", type_reponse: "texte", choix: "", aide: "", annuelle: true };

/** Rubriques ajoutées par l'école, et rubriques désactivées : on ajoute, on modifie celles de
 *  l'école, on désactive et on remet en service ; rien ne se supprime. */
function RubriquesEcole({ rubriques }: { rubriques: readonly RubriqueFiche[] }) {
  const rafraichir = useRafraichir();
  const [form, setForm] = useState<Formulaire | null>(null);
  const enregistrer = useMutation({
    mutationFn: (f: Formulaire) =>
      appeler("rubrique_fiche_enregistrer", {
        id: f.id,
        libelle: f.libelle,
        theme: f.theme,
        type_reponse: f.type_reponse,
        choix: f.choix.split("\n"),
        aide: f.aide,
        annuelle: f.annuelle,
      }),
    onSuccess: () => {
      setForm(null);
      rafraichir();
    },
  });
  const activer = useMutation({
    mutationFn: ({ id, active }: { id: string; active: boolean }) => appeler("rubrique_fiche_activer", { id, active }),
    onSuccess: rafraichir,
  });
  const ecole = rubriques.filter((r) => !r.livree && r.retiree_le === null);
  const desactivees = rubriques.filter((r) => r.retiree_le !== null);
  const livrees = rubriques.filter((r) => r.livree && r.retiree_le === null);
  const [aDesactiver, setADesactiver] = useState("");
  return (
    <Section titre={t("fiche.rubriques_ecole")}>
      <ul className="flex flex-col gap-1 text-sm text-encre">
        {ecole.map((r) => (
          <li key={r.id} className="flex flex-wrap items-center gap-2">
            <span>{r.libelle}</span>
            <span className="text-muet">· {t(`fiche.theme.${r.theme}`)}</span>
            <Bouton variante="secondaire" onClick={() => setForm({ id: r.id, libelle: r.libelle, theme: r.theme, type_reponse: r.type_reponse, choix: r.choix.join("\n"), aide: r.aide, annuelle: r.annuelle })}>
              {t("fiche.modifier")}
            </Bouton>
            <Bouton variante="secondaire" onClick={() => activer.mutate({ id: r.id, active: false })}>
              {t("fiche.desactiver")}
            </Bouton>
          </li>
        ))}
      </ul>
      {form === null ? (
        <div>
          <Bouton variante="secondaire" onClick={() => setForm(VIDE)}>
            {t("fiche.ajouter_rubrique")}
          </Bouton>
        </div>
      ) : (
        <div className="flex flex-col gap-2 rounded border border-muet/30 p-3">
          <Champ id="rubrique-libelle" libelle={t("fiche.libelle")} value={form.libelle} maxLength={120} onChange={(e) => setForm({ ...form, libelle: e.target.value })} />
          <div className="flex flex-wrap gap-3">
            <Selection id="rubrique-theme" libelle={t("fiche.theme")} valeur={form.theme} surChangement={(v) => setForm({ ...form, theme: v as ThemeRubrique })} options={THEMES_RUBRIQUES.map((th) => ({ valeur: th, libelle: t(`fiche.theme.${th}`) }))} />
            <Selection id="rubrique-type" libelle={t("fiche.type")} valeur={form.type_reponse} surChangement={(v) => setForm({ ...form, type_reponse: v as TypeReponse })} options={TYPES_REPONSE.map((ty) => ({ valeur: ty, libelle: t(`fiche.type.${ty}`) }))} />
          </div>
          {form.type_reponse === "choix" || form.type_reponse === "plusieurs_choix" ? (
            <ZoneTexte id="rubrique-choix" libelle={t("fiche.choix")} aide={t("fiche.choix_aide")} valeur={form.choix} surChangement={(v) => setForm({ ...form, choix: v })} />
          ) : null}
          <Champ id="rubrique-aide" libelle={t("fiche.aide")} value={form.aide} maxLength={300} onChange={(e) => setForm({ ...form, aide: e.target.value })} />
          <CaseACocher id="rubrique-annuelle" libelle={t("fiche.annuelle")} coche={form.annuelle} surChangement={(c) => setForm({ ...form, annuelle: c })} />
          <div className="flex gap-2">
            <Bouton onClick={() => enregistrer.mutate(form)} disabled={enregistrer.isPending}>
              {t("commun.enregistrer")}
            </Bouton>
            <Bouton variante="secondaire" onClick={() => setForm(null)}>
              {t("commun.annuler")}
            </Bouton>
          </div>
          <MessageErreur erreur={enregistrer.error} />
        </div>
      )}
      <div className="flex flex-wrap items-end gap-2">
        <Selection id="rubrique-a-desactiver" libelle={t("fiche.desactiver_livree")} valeur={aDesactiver} surChangement={setADesactiver} options={[{ valeur: "", libelle: "" }, ...livrees.map((r) => ({ valeur: r.id, libelle: r.libelle }))]} />
        <Bouton
          variante="secondaire"
          disabled={aDesactiver === ""}
          onClick={() => {
            activer.mutate({ id: aDesactiver, active: false });
            setADesactiver("");
          }}
        >
          {t("fiche.desactiver")}
        </Bouton>
      </div>
      {desactivees.length === 0 ? null : (
        <>
          <h3 className="text-sm font-semibold text-encre">{t("fiche.desactivees")}</h3>
          <ul className="flex flex-col gap-1 text-sm text-muet">
            {desactivees.map((r) => (
              <li key={r.id} className="flex flex-wrap items-center gap-2">
                <span>{r.libelle}</span>
                <Bouton variante="secondaire" onClick={() => activer.mutate({ id: r.id, active: true })}>
                  {t("fiche.reactiver")}
                </Bouton>
              </li>
            ))}
          </ul>
        </>
      )}
      <MessageErreur erreur={activer.error} />
    </Section>
  );
}
