import { useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { choisirClasseur } from "../../api/fichiers";
import { Bouton } from "../../composants/Bouton";
import { Champ, Selection } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { dateCivile, t, texteDeCle } from "../../i18n";
import type { Cycle, FeuilleStatistique, FicheStatistique, LigneStatistique, OngletStatistique, TableauStatistique } from "../../types/gesco";
import { OngletEtablissement } from "./FicheEcole";
import { feuilleDe, libelleLigne, texteCellule, titreColonne, titrePremiere, titreTableau } from "./mise-en-forme";

type Onglet = "eleves" | "personnel" | "etablissement";

const TITRES_ONGLETS = { eleves: "statistiques.onglet.eleves", personnel: "statistiques.onglet.personnel", etablissement: "statistiques.onglet.etablissement" } as const;

/** Fiches désignées par un chiffre cliqué. */
interface Detail {
  titre: string;
  onglet: "eleves" | "personnel";
  membres: readonly number[];
}

type Exportation = { chemin: string } | { erreur: unknown } | null;

/** Statistiques de l'école : les chiffres des élèves et du personnel, arrêtés à une date, pour
 *  toute l'école ou un cycle ; chaque chiffre mène aux fiches qu'il compte, chaque tableau
 *  s'exporte en Excel. L'écran ne fait que lire. */
export function Statistiques({ surOuvrirEleve }: { surOuvrirEleve: (id: string) => void }) {
  const [anneeId, setAnneeId] = useState<string | null>(null);
  const [date, setDate] = useState<string | null>(null);
  const [cycle, setCycle] = useState<Cycle | null>(null);
  const [onglet, setOnglet] = useState<Onglet>("eleves");
  const [detail, setDetail] = useState<Detail | null>(null);
  const [exportation, setExportation] = useState<Exportation>(null);
  const stats = useQuery({
    queryKey: ["statistiques", anneeId, date, cycle],
    queryFn: () => appeler("statistiques_lire", { annee_id: anneeId, date, cycle }),
  });
  const s = stats.data;
  const exporter = async (titre: string, feuilles: FeuilleStatistique[]) => {
    setExportation(null);
    try {
      const chemin = await choisirClasseur(titre, t("commun.classeur_excel"));
      if (chemin === null) return;
      const ecrit = await appeler("statistiques_exporter", { chemin, titre, feuilles });
      setExportation({ chemin: ecrit.chemin });
    } catch (erreur) {
      setExportation({ erreur });
    }
  };
  const contenu: OngletStatistique | undefined = onglet === "etablissement" ? undefined : s?.[onglet];
  const arret = s === undefined ? "" : dateCivile(s.date);
  const ouvrir = (titre: string, membres: readonly number[]) => {
    if (onglet !== "etablissement") setDetail({ titre, onglet, membres });
  };
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("statistiques.titre")}</h1>
      <MessageErreur erreur={stats.error} />
      {s === undefined ? null : (
        <>
          <div className="flex flex-wrap items-end gap-3">
            {s.annees.length > 0 ? (
              <Selection
                id="statistiques-annee"
                libelle={t("statistiques.annee")}
                valeur={s.annee_id ?? ""}
                surChangement={(v) => {
                  setAnneeId(v);
                  setDate(null);
                }}
                options={s.annees.map((a) => ({ valeur: a.id, libelle: a.libelle }))}
              />
            ) : null}
            <Champ
              id="statistiques-date"
              type="date"
              libelle={t("statistiques.date")}
              value={s.date}
              onChange={(ev) => setDate(/^\d{4}-\d{2}-\d{2}$/.test(ev.target.value) ? ev.target.value : null)}
            />
            <Selection
              id="statistiques-perimetre"
              libelle={t("statistiques.perimetre")}
              valeur={cycle ?? ""}
              surChangement={(v) => setCycle(v === "" ? null : (v as Cycle))}
              options={[{ valeur: "", libelle: t("statistiques.toute_ecole") }, ...s.cycles.map((c) => ({ valeur: c, libelle: t(`niveaux.cycle.${c}`) }))]}
            />
          </div>
          <div role="tablist" className="flex gap-2 border-b border-muet">
            {(["eleves", "personnel", "etablissement"] as const).map((o) => (
              <button
                key={o}
                type="button"
                role="tab"
                aria-selected={onglet === o}
                className={`-mb-px border-b-2 px-3 py-1.5 text-sm ${onglet === o ? "border-primaire font-medium text-encre" : "border-transparent text-muet hover:text-encre"}`}
                onClick={() => {
                  setOnglet(o);
                  setDetail(null);
                }}
              >
                {t(TITRES_ONGLETS[o])}
              </button>
            ))}
          </div>
          {onglet === "etablissement" ? (
            <>
              {exportation !== null && "chemin" in exportation ? <p role="status" className="text-sm text-muet">{t("commun.export_enregistre", { chemin: exportation.chemin })}</p> : null}
              <MessageErreur erreur={exportation !== null && "erreur" in exportation ? exportation.erreur : null} />
              <OngletEtablissement anneeId={s.annee_id} surExporter={(titre, feuilles) => void exporter(titre, feuilles)} />
            </>
          ) : null}
          {contenu === undefined ? null : (
            <>
              <div className="flex flex-wrap items-center gap-2">
                <p className="text-sm text-muet">{t("statistiques.arretes_au", { date: arret })}</p>
                <Bouton
                  variante="secondaire"
                  onClick={() => void exporter(`${t(TITRES_ONGLETS[onglet])} ${s.date}`, contenu.tableaux.map((x) => feuilleDe(x)))}
                >
                  {t("statistiques.exporter_onglet")}
                </Bouton>
                {exportation !== null && "chemin" in exportation ? (
                  <p role="status" className="text-sm text-muet">{t("commun.export_enregistre", { chemin: exportation.chemin })}</p>
                ) : null}
              </div>
              <MessageErreur erreur={exportation !== null && "erreur" in exportation ? exportation.erreur : null} />
              {detail === null || detail.onglet !== onglet ? null : (
                <ListeDetail detail={detail} fiches={contenu.fiches} surFermer={() => setDetail(null)} surOuvrirEleve={onglet === "eleves" ? surOuvrirEleve : null} />
              )}
              {contenu.tableaux.map((x) => (
                <TableauStat key={x.code} tableau={x} surOuvrir={ouvrir} surExporter={() => void exporter(`${titreTableau(x.code)} ${s.date}`, [feuilleDe(x)])} />
              ))}
            </>
          )}
        </>
      )}
    </div>
  );
}

function TableauStat({
  tableau,
  surOuvrir,
  surExporter,
}: {
  tableau: TableauStatistique;
  surOuvrir: (titre: string, membres: readonly number[]) => void;
  surExporter: () => void;
}) {
  const titre = titreTableau(tableau.code);
  const ligne = (l: LigneStatistique, total: boolean) => {
    const libelle = libelleLigne(l);
    return (
      <tr key={total ? "__total" : `${l.libelle}-${l.cle ?? ""}`} className={total ? "font-semibold" : ""}>
        <th scope="row" className="border-b border-muet/30 px-2 py-1 text-left font-normal text-encre">
          {libelle}
        </th>
        {l.cellules.map((c, i) => {
          const colonne = tableau.colonnes[i] ?? "";
          const texte = texteCellule(colonne, c);
          return (
            <td key={colonne} className={`border-b border-muet/30 px-2 py-1 text-encre ${c.texte === null ? "text-right" : ""}`}>
              {c.membres.length > 0 ? (
                <button
                  type="button"
                  className="underline decoration-dotted underline-offset-2 hover:text-primaire"
                  onClick={() => surOuvrir(`${titre} · ${libelle} · ${titreColonne(colonne)}`, c.membres)}
                >
                  {texte}
                </button>
              ) : (
                texte
              )}
            </td>
          );
        })}
      </tr>
    );
  };
  return (
    <Section titre={titre}>
      <div>
        <Bouton variante="secondaire" onClick={surExporter}>
          {t("commun.exporter_excel")}
        </Bouton>
      </div>
      {tableau.lignes.length === 0 ? (
        <p className="text-sm text-muet">{t("commun.aucune_ligne")}</p>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full border-collapse font-condensee text-sm tabular-nums">
            <thead>
              <tr>
                <th className="border-b border-muet px-2 py-1 text-left font-medium text-muet">{titrePremiere(tableau.code)}</th>
                {tableau.colonnes.map((c) => (
                  <th key={c} className="border-b border-muet px-2 py-1 text-left font-medium text-muet">
                    {titreColonne(c)}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {tableau.lignes.map((l) => ligne(l, false))}
              {tableau.total === null ? null : ligne(tableau.total, true)}
            </tbody>
          </table>
        </div>
      )}
      {tableau.hors.map((h) => (
        <button key={h.cle} type="button" className="self-start text-sm text-muet underline decoration-dotted underline-offset-2 hover:text-encre" onClick={() => surOuvrir(texteDeCle(h.cle, { n: h.membres.length }), h.membres)}>
          {texteDeCle(h.cle, { n: h.membres.length })}
        </button>
      ))}
    </Section>
  );
}

function ListeDetail({
  detail,
  fiches,
  surFermer,
  surOuvrirEleve,
}: {
  detail: Detail;
  fiches: readonly FicheStatistique[];
  surFermer: () => void;
  surOuvrirEleve: ((id: string) => void) | null;
}) {
  const liste = detail.membres.map((m) => fiches[m]).filter((f): f is FicheStatistique => f !== undefined);
  return (
    <Section titre={detail.titre}>
      <ul className="flex flex-col gap-1 text-sm text-encre">
        {liste.map((f) => (
          <li key={f.id} className="flex flex-wrap gap-2">
            {surOuvrirEleve === null ? (
              <span className="font-medium">{f.nom}</span>
            ) : (
              <button type="button" className="font-medium underline decoration-dotted underline-offset-2 hover:text-primaire" onClick={() => surOuvrirEleve(f.id)}>
                {f.nom}
              </button>
            )}
            <span className="text-muet">{f.detail}</span>
            {f.motif === "" ? null : <span className="text-muet">· {f.motif}</span>}
          </li>
        ))}
      </ul>
      <div>
        <Bouton variante="secondaire" onClick={surFermer}>
          {t("commun.fermer")}
        </Bouton>
      </div>
    </Section>
  );
}
