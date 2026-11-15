import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { Bouton } from "../../composants/Bouton";
import { Champ, Selection } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { t } from "../../i18n";
import type { ArticleInventaire, Cycle, FamilleInventaire, Inventaire as Donnees, QuantitesInventaire, ReleveInventaire } from "../../types/gesco";
import { COLONNES_FAMILLE, COLONNES_LATRINES, FAMILLES, QUANTITES_VIDES, TITRES_FAMILLES, libelleArticle, lireQuantite, type NomQuantite } from "./colonnes";

/** Inventaire de l'école : un onglet par famille, une ligne par article, les quantités de
 *  l'année saisies sur place pour toute l'école ou pour un cycle. La valeur de l'an dernier est
 *  montrée dans la case vide ; « Reprendre l'an dernier » la recopie là où rien n'est saisi. */
export function Inventaire() {
  const [anneeId, setAnneeId] = useState<string | null>(null);
  const [cycle, setCycle] = useState<Cycle | null>(null);
  const [famille, setFamille] = useState<FamilleInventaire>("local");
  const [retires, setRetires] = useState(false);
  const cle = ["inventaire", anneeId, cycle] as const;
  const inventaire = useQuery({ queryKey: cle, queryFn: () => appeler("inventaire_lire", { annee_id: anneeId, cycle }) });
  const client = useQueryClient();
  const poser = (d: Donnees) => {
    client.setQueryData(cle, d);
    void client.invalidateQueries({ queryKey: ["statistiques"] });
  };
  const reprendre = useMutation({
    mutationFn: (annee: string) => appeler("inventaire_reprendre", { annee_id: annee, cycle }),
    onSuccess: poser,
  });
  const d = inventaire.data;
  if (d === undefined) return <MessageErreur erreur={inventaire.error} />;
  const saisis = new Set(d.lignes.filter((l) => !estVide(l)).map((l) => l.article_id));
  const enService = new Set(d.articles.filter((a) => a.retire_le === null).map((a) => a.id));
  const aReprendre = d.an_dernier.filter((l) => !saisis.has(l.article_id) && enService.has(l.article_id)).length;
  const articles = d.articles.filter((a) => a.famille === famille && (retires || a.retire_le === null));
  const colonnes = COLONNES_FAMILLE[famille];
  const avecCabines = famille === "local";
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("inventaire.titre")}</h1>
      <div className="flex flex-wrap items-end gap-3">
        {d.annees.length > 0 ? <Selection id="inventaire-annee" libelle={t("inventaire.annee")} valeur={d.annee_id ?? ""} surChangement={(v) => setAnneeId(v)} options={d.annees.map((a) => ({ valeur: a.id, libelle: a.libelle }))} /> : null}
        <Selection
          id="inventaire-partie"
          libelle={t("inventaire.partie")}
          valeur={cycle ?? ""}
          surChangement={(v) => setCycle(v === "" ? null : (v as Cycle))}
          options={[{ valeur: "", libelle: t("inventaire.toute_ecole") }, ...d.cycles.map((c) => ({ valeur: c, libelle: t(`niveaux.cycle.${c}`) }))]}
        />
        {d.annee_id !== null && aReprendre > 0 ? (
          <Bouton variante="secondaire" onClick={() => d.annee_id !== null && reprendre.mutate(d.annee_id)} disabled={reprendre.isPending}>
            {t("inventaire.reprendre", { n: aReprendre })}
          </Bouton>
        ) : null}
      </div>
      <MessageErreur erreur={reprendre.error} />
      <div role="tablist" className="flex flex-wrap gap-2 border-b border-muet">
        {FAMILLES.map((f) => (
          <button
            key={f}
            type="button"
            role="tab"
            aria-selected={famille === f}
            className={`-mb-px border-b-2 px-3 py-1.5 text-sm ${famille === f ? "border-primaire font-medium text-encre" : "border-transparent text-muet hover:text-encre"}`}
            onClick={() => setFamille(f)}
          >
            {t(TITRES_FAMILLES[f])}
          </button>
        ))}
      </div>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse font-condensee text-sm tabular-nums">
          <thead>
            <tr>
              <th className="border-b border-muet px-2 py-1 text-left font-medium text-muet">{t(famille === "manuel" ? "inventaire.manuel" : "inventaire.article")}</th>
              {[...colonnes, ...(avecCabines ? COLONNES_LATRINES : [])].map((c) => (
                <th key={c} className="border-b border-muet px-2 py-1 text-left font-medium text-muet">
                  {t(`inventaire.colonne.${c}`)}
                </th>
              ))}
              <th className="border-b border-muet px-2 py-1 text-left font-medium text-muet">{t("inventaire.colonne.observation")}</th>
              <th className="border-b border-muet px-2 py-1" />
            </tr>
          </thead>
          <tbody>
            {articles.map((a) => {
              const releve = d.lignes.find((l) => l.article_id === a.id) ?? null;
              const passe = d.an_dernier.find((l) => l.article_id === a.id) ?? null;
              return (
                <LigneArticle
                  key={`${a.id}-${JSON.stringify(releve)}`}
                  article={a}
                  donnees={d}
                  releve={releve}
                  passe={passe}
                  colonnes={colonnes}
                  cabines={avecCabines ? (a.code === "latrines" ? "saisie" : "vide") : "aucune"}
                  surEnregistre={poser}
                />
              );
            })}
          </tbody>
        </table>
      </div>
      <label className="flex items-center gap-2 self-start text-sm text-muet">
        <input type="checkbox" checked={retires} onChange={(e) => setRetires(e.target.checked)} />
        {t("inventaire.voir_retires")}
      </label>
      <AjoutArticle famille={famille} donnees={d} surEnregistre={() => void client.invalidateQueries({ queryKey: ["inventaire"] })} />
    </div>
  );
}

function estVide(l: ReleveInventaire): boolean {
  return l.observation === "" && Object.values(l.quantites).every((q) => q === null);
}

function LigneArticle({
  article,
  donnees,
  releve,
  passe,
  colonnes,
  cabines,
  surEnregistre,
}: {
  article: ArticleInventaire;
  donnees: Donnees;
  releve: ReleveInventaire | null;
  passe: ReleveInventaire | null;
  colonnes: readonly NomQuantite[];
  cabines: "saisie" | "vide" | "aucune";
  surEnregistre: (d: Donnees) => void;
}) {
  const client = useQueryClient();
  const depart = releve?.quantites ?? QUANTITES_VIDES;
  const [quantites, setQuantites] = useState<QuantitesInventaire>(depart);
  const [observation, setObservation] = useState(releve?.observation ?? "");
  const enregistrer = useMutation({
    mutationFn: (r: ReleveInventaire) => appeler("inventaire_releve_enregistrer", { annee_id: donnees.annee_id ?? "", releve: r }),
    onSuccess: surEnregistre,
  });
  const activer = useMutation({
    mutationFn: (actif: boolean) => appeler("article_inventaire_activer", { id: article.id, actif }),
    onSuccess: () => void client.invalidateQueries({ queryKey: ["inventaire"] }),
  });
  const modifiable = donnees.annee_id !== null && article.retire_le === null;
  const envoyer = (q: QuantitesInventaire, o: string) => {
    if (!modifiable) return;
    if (JSON.stringify(q) === JSON.stringify(depart) && o.trim() === (releve?.observation ?? "")) return;
    enregistrer.mutate({ article_id: article.id, cycle: donnees.cycle, quantites: q, observation: o });
  };
  const caseQuantite = (c: NomQuantite) => {
    const ancienne = passe?.quantites[c] ?? null;
    return (
      <td key={c} className="border-b border-muet/30 px-1 py-1">
        <input
          type="number"
          min={0}
          inputMode="numeric"
          aria-label={`${libelleArticle(article, donnees.niveaux)} · ${t(`inventaire.colonne.${c}`)}`}
          title={ancienne === null ? undefined : t("inventaire.an_dernier", { valeur: ancienne })}
          placeholder={ancienne === null ? "" : String(ancienne)}
          disabled={!modifiable}
          className="w-16 rounded border border-muet bg-surface px-1.5 py-1 text-right text-primaire placeholder:text-muet/60"
          value={quantites[c] ?? ""}
          onChange={(e) => setQuantites({ ...quantites, [c]: lireQuantite(e.target.value) })}
          onBlur={() => envoyer(quantites, observation)}
        />
      </td>
    );
  };
  return (
    <tr className={article.retire_le === null ? "" : "opacity-60"}>
      <th scope="row" className="border-b border-muet/30 px-2 py-1 text-left font-normal text-encre">
        {libelleArticle(article, donnees.niveaux)}
        {article.retire_le === null ? null : <span className="ml-1 text-muet">({t("inventaire.retire")})</span>}
        <MessageErreur erreur={enregistrer.error ?? activer.error} />
      </th>
      {colonnes.map(caseQuantite)}
      {cabines === "saisie" ? COLONNES_LATRINES.map(caseQuantite) : null}
      {cabines === "vide" ? COLONNES_LATRINES.map((c) => <td key={c} className="border-b border-muet/30" />) : null}
      <td className="border-b border-muet/30 px-1 py-1">
        <input
          type="text"
          aria-label={`${libelleArticle(article, donnees.niveaux)} · ${t("inventaire.colonne.observation")}`}
          disabled={!modifiable}
          maxLength={300}
          className="w-48 rounded border border-muet bg-surface px-1.5 py-1 text-primaire"
          value={observation}
          onChange={(e) => setObservation(e.target.value)}
          onBlur={() => envoyer(quantites, observation)}
        />
      </td>
      <td className="border-b border-muet/30 px-1 py-1 text-right">
        <button type="button" className="text-sm text-muet underline decoration-dotted underline-offset-2 hover:text-encre" onClick={() => activer.mutate(article.retire_le !== null)} disabled={activer.isPending}>
          {t(article.retire_le === null ? "inventaire.retirer" : "inventaire.remettre")}
        </button>
      </td>
    </tr>
  );
}

/** Ajout d'un article de l'école, ou modification de l'un d'eux ; un manuel a un titre, une
 *  matière et, au choix, un niveau. */
function AjoutArticle({ famille, donnees, surEnregistre }: { famille: FamilleInventaire; donnees: Donnees; surEnregistre: () => void }) {
  const [id, setId] = useState<string | null>(null);
  const [designation, setDesignation] = useState("");
  const [matiere, setMatiere] = useState("");
  const [niveauId, setNiveauId] = useState("");
  const vider = () => {
    setId(null);
    setDesignation("");
    setMatiere("");
    setNiveauId("");
  };
  const enregistrer = useMutation({
    mutationFn: () => appeler("article_inventaire_enregistrer", { id, famille, designation, matiere, niveau_id: niveauId === "" ? null : niveauId }),
    onSuccess: () => {
      vider();
      surEnregistre();
    },
  });
  const manuel = famille === "manuel";
  const deLEcole = donnees.articles.filter((a) => a.famille === famille && !a.livre && a.retire_le === null);
  return (
    <Section titre={t(manuel ? "inventaire.ajouter_manuel" : "inventaire.ajouter_article")}>
      <form
        className="flex flex-wrap items-end gap-3"
        onSubmit={(e) => {
          e.preventDefault();
          enregistrer.mutate();
        }}
      >
        <Champ id="inventaire-designation" libelle={t(manuel ? "inventaire.titre_manuel" : "inventaire.designation")} value={designation} maxLength={120} onChange={(e) => setDesignation(e.target.value)} />
        {manuel ? (
          <>
            <Champ id="inventaire-matiere" libelle={t("inventaire.matiere")} value={matiere} maxLength={80} onChange={(e) => setMatiere(e.target.value)} />
            <Selection id="inventaire-niveau" libelle={t("inventaire.niveau")} valeur={niveauId} surChangement={setNiveauId} options={[{ valeur: "", libelle: "" }, ...donnees.niveaux.map((n) => ({ valeur: n.id, libelle: n.libelle }))]} />
          </>
        ) : null}
        <Bouton type="submit" disabled={designation.trim() === "" || enregistrer.isPending}>
          {t(id === null ? "inventaire.ajouter" : "inventaire.enregistrer")}
        </Bouton>
        {id === null ? null : (
          <Bouton variante="secondaire" onClick={vider}>
            {t("inventaire.annuler")}
          </Bouton>
        )}
      </form>
      <MessageErreur erreur={enregistrer.error} />
      {deLEcole.length === 0 ? null : (
        <ul className="flex flex-col gap-1 text-sm text-encre">
          {deLEcole.map((a) => (
            <li key={a.id} className="flex flex-wrap items-center gap-2">
              <span>{libelleArticle(a, donnees.niveaux)}</span>
              <button
                type="button"
                className="text-muet underline decoration-dotted underline-offset-2 hover:text-encre"
                onClick={() => {
                  setId(a.id);
                  setDesignation(a.designation);
                  setMatiere(a.matiere);
                  setNiveauId(a.niveau_id ?? "");
                }}
              >
                {t("inventaire.modifier")}
              </button>
            </li>
          ))}
        </ul>
      )}
    </Section>
  );
}
