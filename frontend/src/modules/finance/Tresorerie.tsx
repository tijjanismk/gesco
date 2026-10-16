import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { commandeAutorisee } from "../../app/navigation";
import { Bouton } from "../../composants/Bouton";
import { CaseACocher, Champ, Formulaire, Selection } from "../../composants/Champ";
import { MessageEnregistre, MessageErreur, Section } from "../../composants/Messages";
import { Tableau } from "../../composants/Tableau";
import { type Cle, dateCivile, francs, t } from "../../i18n";
import type { CategorieTresorerie, CompteTresorerie, NatureCompte, ReglagesTresorerie, Sens, Tresorerie as Etat } from "../../types/gesco";
import { lireMontant } from "../paie/calcul";

const NATURES: Record<NatureCompte, Cle> = {
  caisse: "tresorerie.nature.caisse",
  banque: "tresorerie.nature.banque",
  mobile_money: "tresorerie.nature.mobile_money",
};

/** Trésorerie (SPEC_PHASE_8) : les comptes de l'école, ses catégories de recettes et de
 *  dépenses, ses réglages. */
export function Tresorerie() {
  const session = useQuery({ queryKey: ["session"], queryFn: () => appeler("session_courante", null) });
  const etat = useQuery({ queryKey: ["tresorerie"], queryFn: () => appeler("tresorerie_lire", null) });
  const client = useQueryClient();
  const apres = (e: Etat) => client.setQueryData(["tresorerie"], e);
  const gere = commandeAutorisee("compte_tresorerie_enregistrer", session.data?.permissions ?? []);
  const e = etat.data;
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("tresorerie.titre")}</h1>
      <MessageErreur erreur={etat.error} />
      {e === undefined ? null : (
        <>
          <Comptes e={e} gere={gere} surFait={apres} />
          <Categories e={e} gere={gere} surFait={apres} />
          {gere ? <Reglages key={JSON.stringify(e.reglages)} reglages={e.reglages} surFait={apres} /> : null}
        </>
      )}
    </div>
  );
}

function Comptes({ e, gere, surFait }: { e: Etat; gere: boolean; surFait: (e: Etat) => void }) {
  const [choisi, setChoisi] = useState<string | null>(null);
  // Chaque enregistrement remet le formulaire à neuf.
  const [tour, setTour] = useState(0);
  const compte = e.comptes.find((c) => c.id === choisi);
  return (
    <Section titre={t("tresorerie.comptes")}>
      <p className="text-sm text-muet">{t("tresorerie.comptes_aide")}</p>
      <Tableau
        nomExport="comptes-de-tresorerie"
        titre={t("tresorerie.comptes")}
        lignes={e.comptes}
        cle={(c) => c.id}
        surSelection={gere ? (c) => setChoisi(c.id) : undefined}
        colonnes={[
          { titre: t("tresorerie.libelle"), valeur: (c) => c.libelle },
          { titre: t("tresorerie.nature"), valeur: (c) => t(NATURES[c.nature]) },
          { titre: t("tresorerie.organisme"), valeur: (c) => [c.organisme, c.numero].filter((x) => x !== "").join(" · ") },
          { titre: t("tresorerie.solde_ouverture"), valeur: (c) => `${francs(c.solde_ouverture)} · ${dateCivile(c.ouvert_le)}` },
          { titre: t("tresorerie.etat"), valeur: (c) => t(c.actif ? "tresorerie.actif" : "tresorerie.ferme") },
        ]}
      />
      {gere ? (
        <FicheCompte
          key={`${choisi ?? "nouveau"}-${String(tour)}`}
          compte={compte}
          surFait={(x) => {
            setChoisi(null);
            setTour(tour + 1);
            surFait(x);
          }}
        />
      ) : null}
    </Section>
  );
}

function FicheCompte({ compte, surFait }: { compte: CompteTresorerie | undefined; surFait: (e: Etat) => void }) {
  const [nature, setNature] = useState<NatureCompte>(compte?.nature ?? "banque");
  const [libelle, setLibelle] = useState(compte?.libelle ?? "");
  const [organisme, setOrganisme] = useState(compte?.organisme ?? "");
  const [numero, setNumero] = useState(compte?.numero ?? "");
  const [solde, setSolde] = useState(compte === undefined ? "0" : String(compte.solde_ouverture));
  const [ouvert, setOuvert] = useState(compte?.ouvert_le ?? "");
  const [actif, setActif] = useState(compte?.actif ?? true);
  const enregistrer = useMutation({
    mutationFn: () =>
      appeler("compte_tresorerie_enregistrer", {
        id: compte?.id ?? null,
        nature,
        libelle,
        organisme,
        numero,
        solde_ouverture: lireMontant(solde) ?? 0,
        ouvert_le: ouvert,
        actif,
      }),
    onSuccess: surFait,
  });
  const caisse = compte?.nature === "caisse";
  return (
    <Formulaire surEnvoi={() => enregistrer.mutate()}>
      <p className="text-sm font-medium text-encre">{compte === undefined ? t("tresorerie.nouveau_compte") : compte.libelle}</p>
      <div className="flex flex-wrap items-end gap-3">
        {compte === undefined ? (
          <Selection
            id="compte-nature"
            libelle={t("tresorerie.nature")}
            valeur={nature}
            surChangement={(v) => setNature(v === "mobile_money" ? "mobile_money" : "banque")}
            options={(["banque", "mobile_money"] as const).map((n) => ({ valeur: n, libelle: t(NATURES[n]) }))}
          />
        ) : null}
        <Champ id="compte-libelle" libelle={t("tresorerie.libelle")} value={libelle} onChange={(ev) => setLibelle(ev.target.value)} autoComplete="off" />
        {caisse ? null : (
          <>
            <Champ id="compte-organisme" libelle={t(nature === "mobile_money" ? "tresorerie.operateur" : "tresorerie.banque")} value={organisme} onChange={(ev) => setOrganisme(ev.target.value)} autoComplete="off" />
            <Champ id="compte-numero" libelle={t("tresorerie.numero")} value={numero} onChange={(ev) => setNumero(ev.target.value)} autoComplete="off" />
          </>
        )}
        <Champ id="compte-solde" libelle={t("tresorerie.solde_ouverture")} value={solde} onChange={(ev) => setSolde(ev.target.value)} autoComplete="off" />
        <Champ id="compte-ouvert" type="date" libelle={t("tresorerie.ouvert_le")} value={ouvert} onChange={(ev) => setOuvert(ev.target.value)} />
        {compte === undefined || caisse ? null : <CaseACocher id="compte-actif" libelle={t("tresorerie.actif")} coche={actif} surChangement={setActif} />}
        <Bouton type="submit" disabled={libelle.trim() === "" || ouvert === "" || enregistrer.isPending}>
          {t(compte === undefined ? "tresorerie.ajouter_compte" : "commun.enregistrer")}
        </Bouton>
      </div>
      <MessageErreur erreur={enregistrer.error} />
    </Formulaire>
  );
}

function Categories({ e, gere, surFait }: { e: Etat; gere: boolean; surFait: (e: Etat) => void }) {
  return (
    <Section titre={t("tresorerie.categories")}>
      <p className="text-sm text-muet">{t("tresorerie.categories_aide")}</p>
      <div className="flex flex-wrap items-start gap-6">
        {(["recette", "depense"] as const).map((sens) => (
          <ListeCategories key={sens} sens={sens} categories={e.categories.filter((c) => c.sens === sens)} gere={gere} surFait={surFait} />
        ))}
      </div>
    </Section>
  );
}

function ListeCategories({ sens, categories, gere, surFait }: { sens: Sens; categories: readonly CategorieTresorerie[]; gere: boolean; surFait: (e: Etat) => void }) {
  const [nouvelle, setNouvelle] = useState("");
  const enregistrer = useMutation({
    mutationFn: (d: { id: string | null; libelle: string; active: boolean }) => appeler("categorie_tresorerie_enregistrer", { ...d, sens }),
    onSuccess: (x) => {
      setNouvelle("");
      surFait(x);
    },
  });
  return (
    <div className="flex min-w-72 flex-1 flex-col gap-2">
      <p className="text-sm font-medium text-encre">{t(sens === "recette" ? "tresorerie.recettes" : "tresorerie.depenses")}</p>
      <ul className="flex flex-col gap-1">
        {categories.map((c) => (
          <LigneCategorie key={`${c.id}-${c.libelle}-${String(c.active)}`} c={c} gere={gere} surEnregistrer={(libelle, active) => enregistrer.mutate({ id: c.id, libelle, active })} />
        ))}
      </ul>
      {gere ? (
        <Formulaire surEnvoi={() => enregistrer.mutate({ id: null, libelle: nouvelle, active: true })}>
          <div className="flex items-end gap-2">
            <Champ id={`categorie-${sens}`} libelle={t("tresorerie.nouvelle_categorie")} value={nouvelle} onChange={(ev) => setNouvelle(ev.target.value)} autoComplete="off" />
            <Bouton type="submit" variante="secondaire" disabled={nouvelle.trim() === "" || enregistrer.isPending}>
              {t("tresorerie.ajouter")}
            </Bouton>
          </div>
        </Formulaire>
      ) : null}
      <MessageErreur erreur={enregistrer.error} />
    </div>
  );
}

function LigneCategorie({ c, gere, surEnregistrer }: { c: CategorieTresorerie; gere: boolean; surEnregistrer: (libelle: string, active: boolean) => void }) {
  const [libelle, setLibelle] = useState(c.libelle);
  if (!gere) return <li className={`text-sm ${c.active ? "text-encre" : "text-muet line-through"}`}>{c.libelle}</li>;
  return (
    <li className="flex items-center gap-2 text-sm">
      <input
        aria-label={t("tresorerie.libelle")}
        className={`rounded border border-muet bg-surface px-2 py-1 ${c.active ? "text-primaire" : "text-muet line-through"}`}
        value={libelle}
        onChange={(ev) => setLibelle(ev.target.value)}
        onBlur={() => {
          if (libelle.trim() !== "" && libelle.trim() !== c.libelle) surEnregistrer(libelle, c.active);
        }}
      />
      <button type="button" className="text-xs text-primaire underline" onClick={() => surEnregistrer(c.libelle, !c.active)}>
        {t(c.active ? "tresorerie.retirer" : "tresorerie.remettre")}
      </button>
    </li>
  );
}

function Reglages({ reglages, surFait }: { reglages: ReglagesTresorerie; surFait: (e: Etat) => void }) {
  const [seuil, setSeuil] = useState(String(reglages.seuil_accord));
  const [cantine, setCantine] = useState(reglages.cantine_active);
  const [repas, setRepas] = useState(reglages.repas_comptes);
  const [enregistre, setEnregistre] = useState<number | null>(null);
  const enregistrer = useMutation({
    mutationFn: () => appeler("tresorerie_reglages_enregistrer", { seuil_accord: lireMontant(seuil) ?? 0, cantine_active: cantine, repas_comptes: repas }),
    onSuccess: (x) => {
      setEnregistre(Date.now());
      surFait(x);
    },
  });
  return (
    <Section titre={t("tresorerie.reglages")}>
      <Formulaire surEnvoi={() => enregistrer.mutate()}>
        <Champ id="reglage-seuil" libelle={t("tresorerie.seuil_accord")} aide={t("tresorerie.seuil_accord_aide")} value={seuil} onChange={(ev) => setSeuil(ev.target.value)} autoComplete="off" />
        <CaseACocher id="reglage-cantine" libelle={t("tresorerie.cantine_active")} coche={cantine} surChangement={setCantine} />
        <CaseACocher id="reglage-repas" libelle={t("tresorerie.repas_comptes")} coche={repas} surChangement={setRepas} desactive={!cantine} />
        <div>
          <Bouton type="submit" disabled={enregistrer.isPending}>
            {t("commun.enregistrer")}
          </Bouton>
        </div>
        <MessageErreur erreur={enregistrer.error} />
        <MessageEnregistre instant={enregistre} />
      </Formulaire>
    </Section>
  );
}
