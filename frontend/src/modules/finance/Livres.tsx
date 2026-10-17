import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { choisirTableur } from "../../api/fichiers";
import { aujourdHui } from "../../app/aujourdhui";
import { commandeAutorisee } from "../../app/navigation";
import { Bouton } from "../../composants/Bouton";
import { Champ, Formulaire, Selection } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { Tableau } from "../../composants/Tableau";
import { type Cle, dateCivile, francs, moisCivil, t } from "../../i18n";
import type { ApercuReleve, CompteTresorerie, EtatComptes, FormatReleve, LigneLivre, ModePaiement, NatureCompte, OrigineLigne } from "../../types/gesco";
import { lireMontant } from "../paie/calcul";
import { MODES } from "./Encaissement";

const NATURES: Record<NatureCompte, Cle> = {
  caisse: "tresorerie.nature.caisse",
  banque: "tresorerie.nature.banque",
  mobile_money: "tresorerie.nature.mobile_money",
};

const ORIGINES: Record<OrigineLigne, Cle> = {
  versement: "livre.origine.versement",
  mouvement: "livre.origine.mouvement",
  depense: "livre.origine.depense",
  transfert: "livre.origine.transfert",
  recette: "livre.origine.recette",
};

/** Nature des comptes qui reçoivent un mode. */
function natureDu(mode: ModePaiement): NatureCompte {
  if (mode === "especes") return "caisse";
  if (mode === "virement" || mode === "cheque") return "banque";
  return "mobile_money";
}

/** Comptes et soldes (SPEC_PHASE_8 §5) : le solde de chaque compte, son livre du mois coché
 *  contre le relevé, les transferts entre comptes, le compte de chaque mode. */
export function Livres() {
  const client = useQueryClient();
  const session = useQuery({ queryKey: ["session"], queryFn: () => appeler("session_courante", null) });
  const tresorerie = useQuery({ queryKey: ["tresorerie"], queryFn: () => appeler("tresorerie_lire", null) });
  const etat = useQuery({ queryKey: ["comptes-soldes"], queryFn: () => appeler("comptes_soldes_lire", null) });
  const gere = commandeAutorisee("transfert_faire", session.data?.permissions ?? []);
  const [choisi, setChoisi] = useState<string | null>(null);
  const apres = (e: EtatComptes) => {
    client.setQueryData(["comptes-soldes"], e);
    void client.invalidateQueries({ queryKey: ["livre"] });
    void client.invalidateQueries({ queryKey: ["caisse"] });
  };
  const comptes = tresorerie.data?.comptes ?? [];
  const e = etat.data;
  const compte = comptes.find((c) => c.id === choisi);
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("livre.titre")}</h1>
      <MessageErreur erreur={etat.error ?? tresorerie.error} />
      {e === undefined ? null : (
        <>
          <Section titre={t("livre.soldes")}>
            {e.salaires_sans_compte > 0 ? <p className="text-sm text-ocre">{t("livre.salaires_sans_compte", { nombre: String(e.salaires_sans_compte) })}</p> : null}
            <Tableau
              nomExport="soldes-des-comptes"
              titre={t("livre.soldes")}
              lignes={comptes}
              cle={(c) => c.id}
              surSelection={(c) => setChoisi(c.id)}
              colonnes={[
                { titre: t("tresorerie.libelle"), valeur: (c) => c.libelle },
                { titre: t("tresorerie.nature"), valeur: (c) => t(NATURES[c.nature]) },
                { titre: t("livre.solde"), valeur: (c) => francs(e.soldes.find((s) => s.compte_id === c.id)?.soldes.livre ?? 0) },
                { titre: t("livre.solde_rapproche"), valeur: (c) => francs(e.soldes.find((s) => s.compte_id === c.id)?.soldes.rapproche ?? 0) },
                { titre: t("livre.a_pointer"), valeur: (c) => String(e.soldes.find((s) => s.compte_id === c.id)?.soldes.a_pointer ?? 0) },
              ]}
            />
          </Section>
          {compte === undefined ? null : <Livre key={compte.id} compte={compte} gere={gere} surPointe={() => void client.invalidateQueries({ queryKey: ["comptes-soldes"] })} />}
          {gere ? <NouveauTransfert comptes={comptes.filter((c) => c.actif)} surFait={apres} /> : null}
          {gere ? <ModesComptes e={e} comptes={comptes.filter((c) => c.actif)} surFait={apres} /> : null}
        </>
      )}
    </div>
  );
}

function Livre({ compte, gere, surPointe }: { compte: CompteTresorerie; gere: boolean; surPointe: () => void }) {
  const client = useQueryClient();
  const [mois, setMois] = useState(aujourdHui().slice(0, 7));
  const cle = ["livre", compte.id, mois];
  const livre = useQuery({ queryKey: cle, queryFn: () => appeler("livre_compte_lire", { compte_id: compte.id, mois }), enabled: /^\d{4}-\d{2}$/.test(mois) });
  const pointer = useMutation({
    mutationFn: (l: LigneLivre) => appeler("ligne_tresorerie_pointer", { compte_id: compte.id, mois, origine: l.origine, origine_id: l.origine_id, pointee: !l.pointee }),
    onSuccess: (x) => {
      client.setQueryData(cle, x);
      surPointe();
    },
  });
  const l = livre.data;
  const titre = t("livre.du_mois", { compte: compte.libelle, mois: moisCivil(mois) });
  return (
    <Section titre={titre}>
      <Champ id="livre-mois" type="month" libelle={t("paie.mois.choisir")} value={mois} onChange={(ev) => setMois(ev.target.value)} />
      <MessageErreur erreur={livre.error ?? pointer.error} />
      {l === undefined ? null : (
        <>
          <p className="text-sm text-encre">{t("livre.report", { montant: francs(l.report) })}</p>
          {gere ? <p className="text-sm text-muet">{t("livre.aide_pointage")}</p> : null}
          <Tableau
            nomExport={`livre-${compte.libelle}-${mois}`}
            titre={titre}
            lignes={l.lignes}
            cle={(x) => `${x.origine}-${x.origine_id}`}
            surSelection={gere ? (x) => pointer.mutate(x) : undefined}
            colonnes={[
              { titre: t("depense.date"), valeur: (x) => dateCivile(x.date) },
              { titre: t("livre.origine"), valeur: (x) => t(ORIGINES[x.origine]) },
              { titre: t("depense.libelle"), valeur: (x) => x.libelle },
              { titre: t("encaissement.reference"), valeur: (x) => x.reference },
              { titre: t("livre.entree"), valeur: (x) => (x.entree === 0 ? "" : francs(x.entree)) },
              { titre: t("livre.sortie"), valeur: (x) => (x.sortie === 0 ? "" : francs(x.sortie)) },
              { titre: t("livre.releve"), valeur: (x) => (x.pointee ? t("livre.pointee") : "") },
            ]}
          />
          <p className="text-sm font-medium text-encre">
            {t("livre.fin", { montant: francs(l.fin), rapproche: francs(l.soldes.rapproche) })}
          </p>
        </>
      )}
      {gere ? (
        <ImportReleve
          compte={compte}
          surFait={() => {
            void client.invalidateQueries({ queryKey: ["livre", compte.id] });
            surPointe();
          }}
        />
      ) : null}
    </Section>
  );
}

/** Import d'un relevé du compte (T-72) : les colonnes se disent une fois et se gardent ; les
 *  lignes retrouvées dans le livre se cochent, les autres se montrent. */
function ImportReleve({ compte, surFait }: { compte: CompteTresorerie; surFait: () => void }) {
  const [chemin, setChemin] = useState<string | null>(null);
  const [apercu, setApercu] = useState<ApercuReleve | null>(null);
  const [format, setFormat] = useState<FormatReleve | null>(null);
  const choisir = useMutation({
    mutationFn: async () => {
      const choisi = await choisirTableur(t("livre.filtre_releve"));
      if (choisi === null) return null;
      const lu = await appeler("releve_apercu", { compte_id: compte.id, chemin: choisi });
      return { choisi, lu };
    },
    onSuccess: (x) => {
      if (x === null) return;
      setChemin(x.choisi);
      setApercu(x.lu);
      setFormat(x.lu.format ?? { date: 0, montant: null, debit: null, credit: null, reference: null });
    },
  });
  const importer = useMutation({
    mutationFn: () => appeler("releve_importer", { compte_id: compte.id, chemin: chemin ?? "", format: format ?? { date: 0, montant: null, debit: null, credit: null, reference: null } }),
    onSuccess: surFait,
  });
  const colonne = (id: string, libelle: string, valeur: number | null, facultative: boolean, changer: (v: number | null) => void) => (
    <Selection
      id={id}
      libelle={libelle}
      valeur={valeur === null ? "" : String(valeur)}
      surChangement={(v) => changer(v === "" ? null : Number(v))}
      options={[...(facultative ? [{ valeur: "", libelle: t("livre.colonne_aucune") }] : []), ...(apercu?.entetes ?? []).map((e, i) => ({ valeur: String(i), libelle: e === "" ? t("livre.colonne_n", { n: String(i + 1) }) : e }))]}
    />
  );
  const r = importer.data;
  return (
    <div className="flex flex-col gap-2 rounded border border-muet p-3">
      <p className="text-sm font-medium text-encre">{t("livre.import_releve")}</p>
      <p className="text-sm text-muet">{t("livre.import_releve_aide")}</p>
      <div>
        <Bouton variante="secondaire" onClick={() => choisir.mutate()} disabled={choisir.isPending}>
          {t("livre.choisir_releve")}
        </Bouton>
      </div>
      {apercu === null || format === null ? null : (
        <>
          <div className="flex flex-wrap items-end gap-3">
            {colonne("releve-date", t("livre.colonne_date"), format.date, false, (v) => setFormat({ ...format, date: v ?? 0 }))}
            {colonne("releve-montant", t("livre.colonne_montant"), format.montant, true, (v) => setFormat({ ...format, montant: v }))}
            {colonne("releve-debit", t("livre.colonne_debit"), format.debit, true, (v) => setFormat({ ...format, debit: v }))}
            {colonne("releve-credit", t("livre.colonne_credit"), format.credit, true, (v) => setFormat({ ...format, credit: v }))}
            {colonne("releve-reference", t("livre.colonne_reference"), format.reference, true, (v) => setFormat({ ...format, reference: v }))}
            <Bouton onClick={() => importer.mutate()} disabled={importer.isPending}>
              {t("livre.rapprocher")}
            </Bouton>
          </div>
          <p className="text-xs text-muet">{apercu.exemples.slice(0, 3).map((l) => l.join(" · ")).join(" | ")}</p>
        </>
      )}
      {r === undefined ? null : (
        <>
          <p role="status" className="text-sm text-vert">
            {t("livre.releve_rapproche", { cochees: String(r.cochees), lues: String(r.lues), a_pointer: String(r.a_pointer) })}
          </p>
          {r.non_retrouvees.length === 0 ? null : (
            <ul className="text-sm text-ocre">
              {r.non_retrouvees.map((x, i) => (
                <li key={`${x.date}-${String(i)}`}>{t("livre.non_retrouvee", { date: dateCivile(x.date), montant: francs(x.montant), reference: x.reference })}</li>
              ))}
            </ul>
          )}
        </>
      )}
      <MessageErreur erreur={choisir.error ?? importer.error} />
    </div>
  );
}

function NouveauTransfert({ comptes, surFait }: { comptes: readonly CompteTresorerie[]; surFait: (e: EtatComptes) => void }) {
  const caisse = comptes.find((c) => c.nature === "caisse");
  const banque = comptes.find((c) => c.nature !== "caisse");
  const [date, setDate] = useState(aujourdHui());
  const [source, setSource] = useState(caisse?.id ?? "");
  const [destination, setDestination] = useState(banque?.id ?? "");
  const [montant, setMontant] = useState("");
  const [reference, setReference] = useState("");
  const [motif, setMotif] = useState("");
  const lu = lireMontant(montant);
  const transferer = useMutation({
    mutationFn: () => appeler("transfert_faire", { date, source_id: source, destination_id: destination, montant: lu ?? 0, reference, motif }),
    onSuccess: (x) => {
      setMontant("");
      setReference("");
      setMotif("");
      surFait(x);
    },
  });
  const options = comptes.map((c) => ({ valeur: c.id, libelle: c.libelle }));
  return (
    <Section titre={t("livre.transfert")}>
      <Formulaire surEnvoi={() => transferer.mutate()}>
        <p className="text-sm text-muet">{t("livre.transfert_aide")}</p>
        <div className="flex flex-wrap items-end gap-3">
          <Champ id="transfert-date" type="date" libelle={t("depense.date")} value={date} onChange={(ev) => setDate(ev.target.value)} />
          <Selection id="transfert-source" libelle={t("livre.de")} valeur={source} surChangement={setSource} options={options} />
          <Selection id="transfert-destination" libelle={t("livre.vers")} valeur={destination} surChangement={setDestination} options={options} />
          <Champ id="transfert-montant" libelle={t("paie.mois.montant")} value={montant} onChange={(ev) => setMontant(ev.target.value)} autoComplete="off" />
          <Champ id="transfert-reference" libelle={t("encaissement.reference")} value={reference} onChange={(ev) => setReference(ev.target.value)} autoComplete="off" />
          <Champ id="transfert-motif" libelle={t("livre.motif")} value={motif} onChange={(ev) => setMotif(ev.target.value)} autoComplete="off" />
          <Bouton type="submit" disabled={lu === null || source === destination || reference.trim() === "" || motif.trim() === "" || transferer.isPending}>
            {t("livre.transferer")}
          </Bouton>
        </div>
        <MessageErreur erreur={transferer.error} />
      </Formulaire>
    </Section>
  );
}

function ModesComptes({ e, comptes, surFait }: { e: EtatComptes; comptes: readonly CompteTresorerie[]; surFait: (e: EtatComptes) => void }) {
  const choisir = useMutation({
    mutationFn: (d: { mode: ModePaiement; compte_id: string }) => appeler("compte_du_mode_choisir", d),
    onSuccess: surFait,
  });
  return (
    <Section titre={t("livre.modes")}>
      <p className="text-sm text-muet">{t("livre.modes_aide")}</p>
      <div className="flex flex-wrap items-end gap-3">
        {e.modes.map((m) => {
          const possibles = comptes.filter((c) => c.nature === natureDu(m.mode));
          return possibles.length === 0 ? (
            <p key={m.mode} className="text-sm text-muet">
              {t("livre.mode_sans_compte", { mode: t(MODES[m.mode]) })}
            </p>
          ) : (
            <Selection
              key={m.mode}
              id={`mode-${m.mode}`}
              libelle={t(MODES[m.mode])}
              valeur={m.compte_id ?? ""}
              surChangement={(v) => choisir.mutate({ mode: m.mode, compte_id: v })}
              options={possibles.map((c) => ({ valeur: c.id, libelle: c.libelle }))}
            />
          );
        })}
      </div>
      <MessageErreur erreur={choisir.error} />
    </Section>
  );
}
