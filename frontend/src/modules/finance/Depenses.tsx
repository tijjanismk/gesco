import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { choisirPiece, enregistrerPieceSous } from "../../api/fichiers";
import { aujourdHui } from "../../app/aujourdhui";
import { Bouton } from "../../composants/Bouton";
import { CaseACocher, Champ, Formulaire, Selection } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { Tableau } from "../../composants/Tableau";
import { type Cle, dateCivile, francs, moisCivil, t } from "../../i18n";
import type { CompteTresorerie, DepenseAffichee, Depenses as Etat, ModePaiement, StatutDepense } from "../../types/gesco";
import { lireMontant } from "../paie/calcul";
import { MODES } from "./Encaissement";

export const STATUTS: Record<StatutDepense, Cle> = {
  a_approuver: "depense.statut.a_approuver",
  approuvee: "depense.statut.approuvee",
  refusee: "depense.statut.refusee",
  payee: "depense.statut.payee",
  annulee: "depense.statut.annulee",
};

/** Modes qui conviennent à un compte : espèces pour la caisse, virement ou chèque pour une
 *  banque, mobile money pour un compte mobile money. */
export function modesDu(compte: CompteTresorerie | undefined): ModePaiement[] {
  if (compte === undefined) return ["especes"];
  if (compte.nature === "caisse") return ["especes"];
  if (compte.nature === "banque") return ["virement", "cheque"];
  return ["orange_money", "wave", "moov"];
}

/** Compte, mode et référence d'un paiement de dépense. */
export function ChoixCompte({
  prefixe,
  comptes,
  compteId,
  surCompte,
  mode,
  surMode,
  reference,
  surReference,
}: {
  prefixe: string;
  comptes: readonly CompteTresorerie[];
  compteId: string;
  surCompte: (id: string, mode: ModePaiement) => void;
  mode: ModePaiement;
  surMode: (m: ModePaiement) => void;
  reference: string;
  surReference: (r: string) => void;
}) {
  const compte = comptes.find((c) => c.id === compteId);
  return (
    <div className="flex flex-wrap items-end gap-3">
      <Selection
        id={`${prefixe}-compte`}
        libelle={t("depense.compte")}
        valeur={compteId}
        surChangement={(v) => surCompte(v, modesDu(comptes.find((c) => c.id === v))[0] ?? "especes")}
        options={comptes.map((c) => ({ valeur: c.id, libelle: c.libelle }))}
      />
      <Selection
        id={`${prefixe}-mode`}
        libelle={t("encaissement.mode")}
        valeur={mode}
        surChangement={(v) => surMode(modesDu(compte).find((m) => m === v) ?? "especes")}
        options={modesDu(compte).map((m) => ({ valeur: m, libelle: t(MODES[m]) }))}
      />
      {mode === "especes" ? null : (
        <Champ id={`${prefixe}-reference`} libelle={t("encaissement.reference")} value={reference} onChange={(ev) => surReference(ev.target.value)} autoComplete="off" />
      )}
    </div>
  );
}

/** Dépenses (SPEC_PHASE_8 §3) : la saisie et le paiement d'un geste sous le seuil, l'accord de la
 *  direction au-dessus, les dépenses du mois et leurs justificatifs. */
export function Depenses() {
  const client = useQueryClient();
  const [mois, setMois] = useState(aujourdHui().slice(0, 7));
  const cle = ["depenses", mois];
  const etat = useQuery({ queryKey: cle, queryFn: () => appeler("depenses_lire", { mois }), enabled: /^\d{4}-\d{2}$/.test(mois) });
  const apres = (e: Etat) => {
    client.setQueryData(["depenses", e.mois], e);
    void client.invalidateQueries({ queryKey: ["depenses"] });
    void client.invalidateQueries({ queryKey: ["caisse"] });
    void client.invalidateQueries({ queryKey: ["fournisseurs"] });
  };
  const [choisie, setChoisie] = useState<string | null>(null);
  const e = etat.data;
  const depense = e?.depenses.find((d) => d.depense.id === choisie);
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("depense.titre")}</h1>
      <MessageErreur erreur={etat.error} />
      {e === undefined ? null : (
        <>
          {e.droits.saisir ? <NouvelleDepense e={e} surFait={apres} /> : null}
          <EnAttente e={e} surFait={apres} />
          <Section titre={t("depense.du_mois", { mois: moisCivil(mois), total: francs(e.total_paye) })}>
            <Champ id="depenses-mois" type="month" libelle={t("paie.mois.choisir")} value={mois} onChange={(ev) => setMois(ev.target.value)} />
            <Tableau
              nomExport={`depenses-${mois}`}
              titre={t("depense.du_mois", { mois: moisCivil(mois), total: francs(e.total_paye) })}
              lignes={e.depenses}
              cle={(d) => d.depense.id}
              surSelection={(d) => setChoisie(d.depense.id)}
              colonnes={[
                { titre: t("depense.date"), valeur: (d) => dateCivile(d.depense.date) },
                { titre: t("depense.categorie"), valeur: (d) => d.categorie },
                { titre: t("depense.libelle"), valeur: (d) => d.depense.libelle },
                { titre: t("depense.beneficiaire"), valeur: (d) => d.depense.beneficiaire },
                { titre: t("paie.mois.montant"), valeur: (d) => francs(d.depense.montant) },
                { titre: t("depense.compte"), valeur: (d) => d.compte ?? "" },
                { titre: t("depense.etat"), valeur: (d) => t(STATUTS[d.depense.statut]) },
              ]}
            />
            {depense === undefined ? null : <Detail key={depense.depense.id} d={depense} e={e} surFait={apres} />}
          </Section>
        </>
      )}
    </div>
  );
}

function NouvelleDepense({ e, surFait }: { e: Etat; surFait: (e: Etat) => void }) {
  const caisse = e.comptes.find((c) => c.nature === "caisse");
  const parDefaut = e.caisse_ouverte ? caisse : e.comptes.find((c) => c.nature !== "caisse") ?? caisse;
  const [date, setDate] = useState(aujourdHui());
  const [categorie, setCategorie] = useState(e.categories[0]?.id ?? "");
  const [libelle, setLibelle] = useState("");
  const [beneficiaire, setBeneficiaire] = useState("");
  const [montant, setMontant] = useState("");
  const [payee, setPayee] = useState(e.droits.payer);
  const [compteId, setCompteId] = useState(parDefaut?.id ?? "");
  const [mode, setMode] = useState<ModePaiement>(modesDu(parDefaut)[0] ?? "especes");
  const [reference, setReference] = useState("");
  const lu = lireMontant(montant);
  const attendra = lu !== null && e.seuil > 0 && lu > e.seuil;
  const saisir = useMutation({
    mutationFn: () =>
      appeler("depense_saisir", {
        date,
        categorie_id: categorie,
        libelle,
        beneficiaire,
        montant: lu ?? 0,
        paiement: payee && !attendra ? { compte_id: compteId, mode, reference } : null,
      }),
    onSuccess: (x) => {
      setLibelle("");
      setBeneficiaire("");
      setMontant("");
      setReference("");
      surFait(x);
    },
  });
  return (
    <Section titre={t("depense.nouvelle")}>
      <Formulaire surEnvoi={() => saisir.mutate()}>
        <div className="flex flex-wrap items-end gap-3">
          <Champ id="depense-date" type="date" libelle={t("depense.date")} value={date} onChange={(ev) => setDate(ev.target.value)} />
          <Selection id="depense-categorie" libelle={t("depense.categorie")} valeur={categorie} surChangement={setCategorie} options={e.categories.map((c) => ({ valeur: c.id, libelle: c.libelle }))} />
          <Champ id="depense-libelle" libelle={t("depense.libelle")} value={libelle} onChange={(ev) => setLibelle(ev.target.value)} autoComplete="off" />
          <Champ id="depense-beneficiaire" libelle={t("depense.beneficiaire")} value={beneficiaire} onChange={(ev) => setBeneficiaire(ev.target.value)} autoComplete="off" />
          <Champ id="depense-montant" libelle={t("paie.mois.montant")} value={montant} onChange={(ev) => setMontant(ev.target.value)} autoComplete="off" />
        </div>
        {attendra ? (
          <p className="text-sm text-ocre">{t("depense.attendra", { seuil: francs(e.seuil) })}</p>
        ) : e.droits.payer ? (
          <>
            <CaseACocher id="depense-payee" libelle={t("depense.payee_maintenant")} coche={payee} surChangement={setPayee} />
            {payee ? (
              <ChoixCompte
                prefixe="depense"
                comptes={e.comptes}
                compteId={compteId}
                surCompte={(id, m) => {
                  setCompteId(id);
                  setMode(m);
                }}
                mode={mode}
                surMode={setMode}
                reference={reference}
                surReference={setReference}
              />
            ) : null}
          </>
        ) : null}
        <div>
          <Bouton type="submit" disabled={libelle.trim() === "" || beneficiaire.trim() === "" || lu === null || categorie === "" || saisir.isPending}>
            {t("depense.enregistrer")}
          </Bouton>
        </div>
        <MessageErreur erreur={saisir.error} />
      </Formulaire>
    </Section>
  );
}

function EnAttente({ e, surFait }: { e: Etat; surFait: (e: Etat) => void }) {
  const a_approuver = e.en_attente.filter((d) => d.depense.statut === "a_approuver");
  const a_payer = e.en_attente.filter((d) => d.depense.statut === "approuvee");
  if ((a_approuver.length === 0 || !e.droits.approuver) && (a_payer.length === 0 || !e.droits.payer)) return null;
  return (
    <Section titre={t("depense.en_attente")}>
      {e.droits.approuver
        ? a_approuver.map((d) => <Decision key={d.depense.id} d={d} surFait={surFait} />)
        : null}
      {e.droits.payer ? a_payer.map((d) => <APayer key={d.depense.id} d={d} e={e} surFait={surFait} />) : null}
    </Section>
  );
}

function resume(d: DepenseAffichee): string {
  return `${dateCivile(d.depense.date)} · ${d.categorie} · ${d.depense.libelle} · ${d.depense.beneficiaire} · ${francs(d.depense.montant)}`;
}

function Decision({ d, surFait }: { d: DepenseAffichee; surFait: (e: Etat) => void }) {
  const [motif, setMotif] = useState("");
  const decider = useMutation({
    mutationFn: (accorder: boolean) => appeler("depense_decider", { id: d.depense.id, accorder, motif }),
    onSuccess: surFait,
  });
  return (
    <div className="flex flex-wrap items-end gap-3 text-sm text-encre">
      <span className="min-w-72">
        {resume(d)} <span className="text-muet">{t("depense.saisie_par", { nom: d.saisie_par })}</span>
      </span>
      <Bouton onClick={() => decider.mutate(true)} disabled={decider.isPending}>
        {t("depense.accorder")}
      </Bouton>
      <Champ id={`refus-${d.depense.id}`} libelle={t("depense.motif_refus")} value={motif} onChange={(ev) => setMotif(ev.target.value)} autoComplete="off" />
      <Bouton variante="danger" onClick={() => decider.mutate(false)} disabled={motif.trim() === "" || decider.isPending}>
        {t("depense.refuser")}
      </Bouton>
      <MessageErreur erreur={decider.error} />
    </div>
  );
}

function APayer({ d, e, surFait }: { d: DepenseAffichee; e: Etat; surFait: (e: Etat) => void }) {
  const parDefaut = e.comptes[0];
  const [compteId, setCompteId] = useState(parDefaut?.id ?? "");
  const [mode, setMode] = useState<ModePaiement>(modesDu(parDefaut)[0] ?? "especes");
  const [reference, setReference] = useState("");
  const payer = useMutation({
    mutationFn: () => appeler("depense_payer", { id: d.depense.id, paiement: { compte_id: compteId, mode, reference } }),
    onSuccess: surFait,
  });
  return (
    <Formulaire surEnvoi={() => payer.mutate()}>
      <p className="text-sm text-encre">
        {resume(d)} <span className="text-vert">{t("depense.statut.approuvee")}</span>
      </p>
      <div className="flex flex-wrap items-end gap-3">
        <ChoixCompte
          prefixe={`payer-${d.depense.id}`}
          comptes={e.comptes}
          compteId={compteId}
          surCompte={(id, m) => {
            setCompteId(id);
            setMode(m);
          }}
          mode={mode}
          surMode={setMode}
          reference={reference}
          surReference={setReference}
        />
        <Bouton type="submit" disabled={payer.isPending}>
          {t("depense.payer")}
        </Bouton>
      </div>
      <MessageErreur erreur={payer.error} />
    </Formulaire>
  );
}

function Detail({ d, e, surFait }: { d: DepenseAffichee; e: Etat; surFait: (e: Etat) => void }) {
  const [motif, setMotif] = useState("");
  const joindre = useMutation({
    mutationFn: async () => {
      const chemin = await choisirPiece(t("personnel.piece.filtre"));
      if (chemin === null) return null;
      return appeler("depense_piece_joindre", { id: d.depense.id, chemin });
    },
    onSuccess: (x) => {
      if (x !== null) surFait(x);
    },
  });
  const ouvrir = useMutation({
    mutationFn: async (p: { id: string; nom_fichier: string }) => {
      const chemin = await enregistrerPieceSous(p.nom_fichier);
      if (chemin !== null) await appeler("depense_piece_enregistrer_sous", { piece_id: p.id, chemin });
    },
  });
  const annuler = useMutation({
    mutationFn: () => appeler("depense_annuler", { id: d.depense.id, motif }),
    onSuccess: surFait,
  });
  const annulable = e.droits.payer && d.depense.paiement_id === null && (d.depense.statut === "payee" || d.depense.statut === "approuvee");
  return (
    <div className="flex flex-col gap-2 rounded border border-muet p-3">
      <p className="text-sm font-medium text-encre">{resume(d)}</p>
      <p className="text-sm text-muet">
        {t(STATUTS[d.depense.statut])}
        {d.compte === null ? "" : ` · ${d.compte}`}
        {d.depense.reference === "" ? "" : ` · ${d.depense.reference}`}
        {d.depense.motif === "" ? "" : ` · ${d.depense.motif}`}
      </p>
      <ul className="flex flex-col gap-1">
        {d.pieces.map((p) => (
          <li key={p.id}>
            <button type="button" className="text-sm text-primaire underline" onClick={() => ouvrir.mutate(p)}>
              {p.nom_fichier}
            </button>
          </li>
        ))}
      </ul>
      <div className="flex flex-wrap items-end gap-3">
        {e.droits.saisir ? (
          <Bouton variante="secondaire" onClick={() => joindre.mutate()} disabled={joindre.isPending}>
            {t("depense.joindre")}
          </Bouton>
        ) : null}
        {annulable ? (
          <>
            <Champ id="depense-annulation" libelle={t("depense.motif_annulation")} value={motif} onChange={(ev) => setMotif(ev.target.value)} autoComplete="off" />
            <Bouton variante="danger" onClick={() => annuler.mutate()} disabled={motif.trim() === "" || annuler.isPending}>
              {t("depense.annuler")}
            </Bouton>
          </>
        ) : null}
      </div>
      <MessageErreur erreur={joindre.error ?? ouvrir.error ?? annuler.error} />
    </div>
  );
}
