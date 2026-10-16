import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { aujourdHui } from "../../app/aujourdhui";
import { commandeAutorisee } from "../../app/navigation";
import { Bouton } from "../../composants/Bouton";
import { Champ, Formulaire, Selection } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { Tableau } from "../../composants/Tableau";
import { francs, moisCivil, t } from "../../i18n";
import type { Budget as EtatBudget, LigneBudget, LigneJournalTresorerie } from "../../types/gesco";
import { lireMontant } from "../paie/calcul";

/** Journal et budget (SPEC_PHASE_8 §7) : ce qui est entré et sorti dans le mois, par catégorie,
 *  et le solde de chaque compte ; le budget de l'année, prévu et réalisé. Chaque liste
 *  s'exporte en Excel pour le comptable. */
export function JournalTresorerie() {
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("journal_tresorerie.titre")}</h1>
      <JournalDuMois />
      <Budget />
    </div>
  );
}

function JournalDuMois() {
  const [mois, setMois] = useState(aujourdHui().slice(0, 7));
  const journal = useQuery({ queryKey: ["journal-tresorerie", mois], queryFn: () => appeler("journal_tresorerie_lire", { mois }), enabled: /^\d{4}-\d{2}$/.test(mois) });
  const j = journal.data;
  const colonnes = [
    { titre: t("depense.categorie"), valeur: (l: LigneJournalTresorerie) => l.categorie },
    { titre: t("paie.mois.montant"), valeur: (l: LigneJournalTresorerie) => francs(l.montant) },
  ];
  const titreRecettes = t("journal_tresorerie.recettes", { mois: moisCivil(mois), total: francs(j?.total_recettes ?? 0) });
  const titreDepenses = t("journal_tresorerie.depenses", { mois: moisCivil(mois), total: francs(j?.total_depenses ?? 0) });
  return (
    <Section titre={t("journal_tresorerie.du_mois", { mois: moisCivil(mois) })}>
      <Champ id="journal-mois" type="month" libelle={t("paie.mois.choisir")} value={mois} onChange={(ev) => setMois(ev.target.value)} />
      <MessageErreur erreur={journal.error} />
      {j === undefined ? null : (
        <>
          <p className="text-sm font-medium text-encre">{titreRecettes}</p>
          <Tableau nomExport={`recettes-${mois}`} titre={titreRecettes} lignes={j.recettes} cle={(l) => l.categorie} colonnes={colonnes} />
          <p className="text-sm font-medium text-encre">{titreDepenses}</p>
          <Tableau nomExport={`depenses-par-categorie-${mois}`} titre={titreDepenses} lignes={j.depenses} cle={(l) => l.categorie} colonnes={colonnes} />
          <p className="text-sm font-medium text-encre">{t("journal_tresorerie.soldes")}</p>
          <Tableau
            nomExport={`soldes-${mois}`}
            titre={t("journal_tresorerie.soldes")}
            lignes={j.soldes}
            cle={(s) => s.compte}
            colonnes={[
              { titre: t("tresorerie.libelle"), valeur: (s) => s.compte },
              { titre: t("journal_tresorerie.debut"), valeur: (s) => francs(s.debut) },
              { titre: t("journal_tresorerie.fin"), valeur: (s) => francs(s.fin) },
            ]}
          />
        </>
      )}
    </Section>
  );
}

/** Ce qui reste du prévu : rien à dire sans prévu ; une dépense au-delà est dépassée, une
 *  recette au-delà rapporte plus que prévu. */
function reste(l: LigneBudget): string {
  if (l.prevu === 0) return "—";
  if (l.depasse) return t("budget.depasse", { ecart: francs(-l.ecart) });
  if (l.ecart < 0) return t("budget.au_dela", { ecart: francs(-l.ecart) });
  return francs(l.ecart);
}

function Budget() {
  const client = useQueryClient();
  const session = useQuery({ queryKey: ["session"], queryFn: () => appeler("session_courante", null) });
  const gere = commandeAutorisee("budget_prevoir", session.data?.permissions ?? []);
  const [annee, setAnnee] = useState<string | null>(null);
  const budget = useQuery({ queryKey: ["budget", annee], queryFn: () => appeler("budget_lire", { annee_id: annee }) });
  const b = budget.data;
  const colonnes = [
    { titre: t("depense.categorie"), valeur: (l: LigneBudget) => l.categorie.libelle },
    { titre: t("budget.prevu"), valeur: (l: LigneBudget) => francs(l.prevu) },
    { titre: t("budget.realise"), valeur: (l: LigneBudget) => francs(l.realise) },
    { titre: t("budget.ecart"), valeur: reste },
  ];
  return (
    <Section titre={t("budget.titre", { annee: b?.annee ?? "" })}>
      <MessageErreur erreur={budget.error} />
      {b === undefined ? null : (
        <>
          <Selection id="budget-annee" libelle={t("budget.annee")} valeur={b.annee_id} surChangement={setAnnee} options={b.annees.map((a) => ({ valeur: a.id, libelle: a.libelle }))} />
          {b.depassements > 0 ? <p className="text-sm text-ocre">{t("budget.avertissement", { nombre: String(b.depassements) })}</p> : null}
          <p className="text-sm font-medium text-encre">{t("budget.recettes")}</p>
          <Tableau nomExport={`budget-recettes-${b.annee}`} titre={t("budget.recettes")} lignes={b.recettes} cle={(l) => l.categorie.id} colonnes={colonnes} />
          <p className="text-sm font-medium text-encre">{t("budget.depenses")}</p>
          <Tableau nomExport={`budget-depenses-${b.annee}`} titre={t("budget.depenses")} lignes={b.depenses} cle={(l) => l.categorie.id} colonnes={colonnes} />
          {gere ? <Prevoir key={b.annee_id} b={b} surFait={(x) => client.setQueryData(["budget", annee], x)} /> : null}
        </>
      )}
    </Section>
  );
}

function Prevoir({ b, surFait }: { b: EtatBudget; surFait: (b: EtatBudget) => void }) {
  const lignes = [...b.recettes, ...b.depenses];
  const [categorie, setCategorie] = useState(lignes[0]?.categorie.id ?? "");
  const [montant, setMontant] = useState("");
  const lu = lireMontant(montant);
  const prevoir = useMutation({
    mutationFn: () => appeler("budget_prevoir", { annee_id: b.annee_id, categorie_id: categorie, montant: lu ?? 0 }),
    onSuccess: (x) => {
      setMontant("");
      surFait(x);
    },
  });
  return (
    <Formulaire surEnvoi={() => prevoir.mutate()}>
      <p className="text-sm font-medium text-encre">{t("budget.prevoir")}</p>
      <div className="flex flex-wrap items-end gap-3">
        <Selection
          id="budget-categorie"
          libelle={t("depense.categorie")}
          valeur={categorie}
          surChangement={setCategorie}
          options={lignes.map((l) => ({ valeur: l.categorie.id, libelle: `${t(l.categorie.sens === "recette" ? "budget.recette" : "budget.depense")} · ${l.categorie.libelle}` }))}
        />
        <Champ id="budget-montant" libelle={t("budget.prevu")} value={montant} onChange={(ev) => setMontant(ev.target.value)} autoComplete="off" />
        <Bouton type="submit" disabled={lu === null || categorie === "" || prevoir.isPending}>
          {t("commun.enregistrer")}
        </Bouton>
      </div>
      <MessageErreur erreur={prevoir.error} />
    </Formulaire>
  );
}
