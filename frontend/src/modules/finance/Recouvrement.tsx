import { useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { Champ, Selection } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { Tableau } from "../../composants/Tableau";
import { dateCivile, dateHeure, francs, t } from "../../i18n";
import type { LigneImpaye, LigneJournal, ModePaiement, TotalTranche, TrancheAnciennete } from "../../types/gesco";
import { telephoneLisible } from "../scolarite/Identite";
import { MODES } from "./Encaissement";
import { aujourdHui } from "../../app/aujourdhui";

/** Libellé d'une tranche d'après les limites que l'école a réglées : « 16 à 30 jours ». */
export function libelleTranche(tr: TotalTranche): string {
  return tr.a === null ? t("impayes.tranche.plus_de", { jours: String(tr.de) }) : t("impayes.tranche.de_a", { de: String(tr.de), a: String(tr.a) });
}

const TOUTES = "";

function payeurs(ligne: LigneImpaye): string {
  return ligne.payeurs.map((p) => (p.telephones.length === 0 ? p.nom : `${p.nom} (${p.telephones.map(telephoneLisible).join(", ")})`)).join(" ; ");
}

/** Impayés de l'année (UC-05, SPEC_PHASE_3 §9.1) : par classe et par tranche d'ancienneté, avec
 *  les téléphones des payeurs, exportables pour les relances. */
export function Impayes({ surOuvrirEleve }: { surOuvrirEleve: (id: string) => void }) {
  const liste = useQuery({ queryKey: ["impayes"], queryFn: () => appeler("impayes_lister", null) });
  const [classe, setClasse] = useState(TOUTES);
  const [tranche, setTranche] = useState(TOUTES);
  const donnees = liste.data;
  const lignes = (donnees?.lignes ?? []).filter(
    (l) => (classe === TOUTES || l.classe === classe) && (tranche === TOUTES || l.impaye.tranche === tranche),
  );
  const libelles = new Map<TrancheAnciennete, string>((donnees?.par_tranche ?? []).map((tr) => [tr.tranche, libelleTranche(tr)]));
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("impayes.titre")}</h1>
      <MessageErreur erreur={liste.error} />
      {donnees === undefined ? null : (
        <>
          <p className="text-sm text-muet">
            {t("impayes.resume", { annee: donnees.annee, date: dateCivile(donnees.date), echu: francs(donnees.echu), eleves: String(donnees.lignes.length) })}
          </p>
          <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
            {donnees.par_tranche.map((tr) => (
              <button
                key={tr.tranche}
                type="button"
                onClick={() => setTranche(tranche === tr.tranche ? TOUTES : tr.tranche)}
                className={`flex flex-col rounded border p-2 text-left ${tranche === tr.tranche ? "border-primaire" : "border-muet/40"} ${tr.tranche === "derniere" && tr.eleves > 0 ? "text-rouge" : "text-encre"}`}
              >
                <span className="text-sm">{libelleTranche(tr)}</span>
                <span className="text-lg font-semibold">{francs(tr.echu)}</span>
                <span className="text-xs text-muet">{t("impayes.nombre_eleves", { nombre: String(tr.eleves) })}</span>
              </button>
            ))}
          </div>
          <div className="flex flex-wrap gap-3">
            <Selection
              id="impayes-classe"
              libelle={t("impayes.classe")}
              valeur={classe}
              surChangement={setClasse}
              options={[{ valeur: TOUTES, libelle: t("impayes.toute_l_ecole") }, ...donnees.par_classe.map((c) => ({ valeur: c.classe, libelle: c.classe }))]}
            />
            <Selection
              id="impayes-tranche"
              libelle={t("impayes.anciennete")}
              valeur={tranche}
              surChangement={setTranche}
              options={[{ valeur: TOUTES, libelle: t("impayes.toutes_tranches") }, ...donnees.par_tranche.map((tr) => ({ valeur: tr.tranche, libelle: libelleTranche(tr) }))]}
            />
          </div>
          {donnees.lignes.length === 0 ? (
            <p className="text-encre">{t("impayes.aucun")}</p>
          ) : (
            <Tableau
              nomExport="impayes"
              titre={t("impayes.titre")}
              lignes={lignes}
              cle={(l) => l.inscription_id}
              surSelection={(l) => surOuvrirEleve(l.eleve_id)}
              colonnes={[
                { titre: t("impayes.classe"), valeur: (l) => l.classe },
                { titre: t("impayes.eleve"), valeur: (l) => l.eleve },
                { titre: t("impayes.matricule"), valeur: (l) => l.matricule },
                { titre: t("impayes.echu"), valeur: (l) => francs(l.impaye.echu) },
                { titre: t("impayes.total"), valeur: (l) => francs(l.impaye.total) },
                { titre: t("impayes.depuis"), valeur: (l) => dateCivile(l.impaye.plus_ancienne) },
                { titre: t("impayes.jours"), valeur: (l) => String(l.impaye.jours) },
                { titre: t("impayes.anciennete"), valeur: (l) => libelles.get(l.impaye.tranche) ?? "" },
                { titre: t("impayes.payeurs"), valeur: payeurs },
              ]}
            />
          )}
          {donnees.par_classe.length === 0 ? null : (
            <Section titre={t("impayes.par_classe")}>
              <Tableau
                nomExport="impayes-par-classe"
                titre={t("impayes.par_classe")}
                lignes={donnees.par_classe}
                cle={(c) => c.classe}
                colonnes={[
                  { titre: t("impayes.classe"), valeur: (c) => c.classe },
                  { titre: t("impayes.eleves"), valeur: (c) => String(c.eleves) },
                  { titre: t("impayes.echu"), valeur: (c) => francs(c.echu) },
                  { titre: t("impayes.total"), valeur: (c) => francs(c.total) },
                ]}
              />
            </Section>
          )}
        </>
      )}
    </div>
  );
}

function etat(l: LigneJournal): string {
  return l.versement.statut === "annule" ? t("journal_caisse.annule", { motif: l.versement.motif_annulation }) : t("journal_caisse.valide");
}

/** Journal des encaissements (SPEC_PHASE_3 §9.2) : reçus d'une période, par caissier et par mode,
 *  annulations comprises, avec les totaux par jour et par mode. L'export du comptable. */
export function JournalEncaissements() {
  const [du, setDu] = useState(() => `${aujourdHui().slice(0, 8)}01`);
  const [au, setAu] = useState(aujourdHui);
  const [caissier, setCaissier] = useState(TOUTES);
  const [mode, setMode] = useState(TOUTES);
  const journal = useQuery({
    queryKey: ["journal-encaissements", du, au, caissier, mode],
    queryFn: () =>
      appeler("encaissements_lister", {
        du,
        au,
        caissier_id: caissier === TOUTES ? null : caissier,
        mode: mode === TOUTES ? null : (mode as ModePaiement),
      }),
    enabled: du !== "" && au !== "",
  });
  const donnees = journal.data;
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("journal_caisse.titre")}</h1>
      <div className="flex flex-wrap items-end gap-3">
        <Champ id="journal-du" type="date" libelle={t("journal_caisse.du")} value={du} onChange={(e) => setDu(e.target.value)} />
        <Champ id="journal-au" type="date" libelle={t("journal_caisse.au")} value={au} onChange={(e) => setAu(e.target.value)} />
        <Selection
          id="journal-caissier"
          libelle={t("journal_caisse.caissier")}
          valeur={caissier}
          surChangement={setCaissier}
          options={[{ valeur: TOUTES, libelle: t("journal_caisse.tous") }, ...(donnees?.caissiers ?? []).map((c) => ({ valeur: c.id, libelle: c.nom }))]}
        />
        <Selection
          id="journal-mode"
          libelle={t("journal_caisse.mode")}
          valeur={mode}
          surChangement={setMode}
          options={[{ valeur: TOUTES, libelle: t("journal_caisse.tous") }, ...Object.entries(MODES).map(([v, c]) => ({ valeur: v, libelle: t(c) }))]}
        />
      </div>
      <MessageErreur erreur={journal.error} />
      {donnees === undefined ? null : (
        <>
          <p className="text-encre" role="status">
            {t("journal_caisse.resume", {
              nombre: String(donnees.nombre),
              total: francs(donnees.total),
              annules: String(donnees.annules),
              montant_annule: francs(donnees.montant_annule),
            })}
          </p>
          <Tableau
            nomExport={`encaissements-${du}-${au}`}
            titre={t("journal_caisse.titre")}
            lignes={donnees.lignes}
            cle={(l) => l.versement.id}
            colonnes={[
              { titre: t("journal_caisse.date"), valeur: (l) => dateHeure(l.versement.encaisse_le) },
              { titre: t("journal_caisse.recu"), valeur: (l) => l.versement.numero_recu },
              { titre: t("impayes.eleve"), valeur: (l) => l.eleve },
              { titre: t("impayes.classe"), valeur: (l) => l.classe },
              { titre: t("journal_caisse.payeur"), valeur: (l) => l.versement.payeur_nom },
              { titre: t("journal_caisse.mode"), valeur: (l) => t(MODES[l.versement.mode]) },
              { titre: t("journal_caisse.reference"), valeur: (l) => l.versement.reference },
              { titre: t("journal_caisse.montant"), valeur: (l) => francs(l.versement.montant) },
              { titre: t("journal_caisse.caissier"), valeur: (l) => l.caissier },
              { titre: t("journal_caisse.etat"), valeur: etat },
            ]}
          />
          <div className="flex flex-wrap items-start gap-4">
            <Section titre={t("journal_caisse.par_jour")}>
              <Tableau
                nomExport={`encaissements-par-jour-${du}-${au}`}
                lignes={donnees.par_jour}
                cle={(j) => j.jour}
                colonnes={[
                  { titre: t("journal_caisse.jour"), valeur: (j) => dateCivile(j.jour) },
                  { titre: t("journal_caisse.recus"), valeur: (j) => String(j.nombre) },
                  { titre: t("journal_caisse.montant"), valeur: (j) => francs(j.montant) },
                ]}
              />
            </Section>
            <Section titre={t("journal_caisse.par_mode")}>
              <Tableau
                nomExport={`encaissements-par-mode-${du}-${au}`}
                lignes={donnees.par_mode}
                cle={(m) => m.mode}
                colonnes={[
                  { titre: t("journal_caisse.mode"), valeur: (m) => t(MODES[m.mode]) },
                  { titre: t("journal_caisse.recus"), valeur: (m) => String(m.nombre) },
                  { titre: t("journal_caisse.montant"), valeur: (m) => francs(m.montant) },
                ]}
              />
            </Section>
          </div>
        </>
      )}
    </div>
  );
}
