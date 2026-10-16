import { useMutation, useQueries, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { ApercuPdf } from "../../composants/ApercuPdf";
import { Bouton } from "../../composants/Bouton";
import { CaseACocher, Champ, Formulaire, Selection } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { type Cle, dateCivile, francs, t } from "../../i18n";
import type { ModePaiement, RecuEmis, SituationInscription } from "../../types/gesco";

export const MODES: Record<ModePaiement, Cle> = {
  especes: "caisse.mode.especes",
  orange_money: "caisse.mode.orange_money",
  wave: "caisse.mode.wave",
  moov: "caisse.mode.moov",
  cheque: "caisse.mode.cheque",
  virement: "caisse.mode.virement",
};

const AUTRE = "autre";

function lireFrancs(texte: string): number {
  const chiffres = texte.replace(/\D/g, "");
  return chiffres === "" ? 0 : Number.parseInt(chiffres, 10);
}

/** Clé d'un encaissement : tirée une fois par formulaire, elle rend le geste rejouable sans
 *  doubler la recette (R-35). */
function nouvelleCle(): string {
  return crypto.randomUUID();
}

function useRafraichir() {
  const client = useQueryClient();
  return () => {
    for (const cle of ["caisse", "situation", "sessions-caisse", "preparation"]) void client.invalidateQueries({ queryKey: [cle] });
  };
}

/** Inscription à encaisser d'un élève : la plus ancienne qui doit encore de l'argent, l'arriéré
 *  d'une année close avant les frais de l'année (T-67), sinon la dernière facturée. Les
 *  situations arrivent de la plus récente à la plus ancienne. */
function inscriptionAEncaisser(situations: readonly SituationInscription[]): SituationInscription | undefined {
  return [...situations].reverse().find((s) => s.facture !== null && s.facture.reste > 0) ?? situations.find((s) => s.facture !== null);
}

function ChampsPaiement({
  prefixe,
  mode,
  surMode,
  reference,
  surReference,
}: {
  prefixe: string;
  mode: ModePaiement;
  surMode: (m: ModePaiement) => void;
  reference: string;
  surReference: (r: string) => void;
}) {
  return (
    <div className="flex flex-wrap items-start gap-3">
      <Selection
        id={`${prefixe}-mode`}
        libelle={t("encaissement.mode")}
        valeur={mode}
        surChangement={(v) => surMode((Object.keys(MODES) as ModePaiement[]).find((m) => m === v) ?? "especes")}
        options={(Object.keys(MODES) as ModePaiement[]).map((m) => ({ valeur: m, libelle: t(MODES[m]) }))}
      />
      {mode === "especes" ? null : (
        <Champ id={`${prefixe}-reference`} libelle={t("encaissement.reference")} aide={t("encaissement.reference_aide")} value={reference} onChange={(e) => surReference(e.target.value)} />
      )}
    </div>
  );
}

function RecusEmis({ recus, surNouveau }: { recus: readonly RecuEmis[]; surNouveau: () => void }) {
  return (
    <div className="flex flex-col gap-3">
      <p role="status" className="text-sm font-medium text-encre">
        {t("encaissement.fait", { recus: recus.map((r) => `${r.versement.numero_recu} (${francs(r.versement.montant)})`).join(", ") })}
      </p>
      <div><Bouton onClick={surNouveau}>{t("encaissement.nouveau")}</Bouton></div>
      {recus.map((r) => (
        <ApercuPdf key={r.versement.id} base64={r.recu.pdf} />
      ))}
    </div>
  );
}

/** Encaissement pour un élève (UC-02, SPEC_PHASE_3 §7.1) : trouver l'élève, voir ce qu'il doit,
 *  saisir le montant, Entrée. */
export function Encaissement() {
  const [texte, setTexte] = useState("");
  const [eleveId, setEleveId] = useState<string | null>(null);
  const trouves = useQuery({
    queryKey: ["caisse-eleves", texte],
    queryFn: () => appeler("eleves_rechercher", { texte }),
    enabled: texte.trim().length >= 2 && eleveId === null,
  });
  const situation = useQuery({
    queryKey: ["situation", eleveId],
    queryFn: () => appeler("situation_financiere", { id: eleveId ?? "" }),
    enabled: eleveId !== null,
  });
  const [autre, setAutre] = useState<string | null>(null);
  const dues = (situation.data ?? []).filter((s) => s.facture !== null && s.facture.reste > 0);
  const choisie = dues.find((s) => s.inscription.id === autre) ?? inscriptionAEncaisser(situation.data ?? []);
  const plusRecente = situation.data?.[0];
  const recommencer = () => {
    setEleveId(null);
    setTexte("");
    setAutre(null);
  };
  return (
    <div className="flex flex-col gap-3">
      {eleveId === null ? (
        <>
          <Champ id="encaissement-recherche" type="search" autoFocus libelle={t("encaissement.rechercher")} aide={t("encaissement.rechercher_aide")} value={texte} onChange={(e) => setTexte(e.target.value)} />
          <ul className="flex flex-col gap-1">
            {(trouves.data ?? []).slice(0, 8).map((e) => (
              <li key={e.id}>
                <button type="button" className="text-sm text-primaire underline" onClick={() => setEleveId(e.id)}>
                  {`${e.prenoms} ${e.nom} · ${e.matricule_interne}`}
                </button>
              </li>
            ))}
          </ul>
        </>
      ) : null}
      <MessageErreur erreur={trouves.error ?? situation.error} />
      {eleveId !== null && situation.isSuccess && choisie === undefined ? (
        <div className="flex flex-col gap-2">
          <p className="text-sm text-ocre">{t("encaissement.aucune_facture")}</p>
          <div><Bouton variante="secondaire" onClick={recommencer}>{t("encaissement.autre_eleve")}</Bouton></div>
        </div>
      ) : null}
      {dues.length > 1 && choisie !== undefined ? (
        <Selection
          id="encaissement-annee"
          libelle={t("encaissement.payer_pour")}
          valeur={choisie.inscription.id}
          surChangement={setAutre}
          options={dues.map((s) => ({ valeur: s.inscription.id, libelle: `${s.annee} · ${s.classe} · ${francs(s.facture?.reste ?? 0)}` }))}
        />
      ) : null}
      {choisie !== undefined && plusRecente !== undefined && choisie.inscription.annee_id !== plusRecente.inscription.annee_id ? (
        <p className="text-sm text-ocre">{t("encaissement.arriere", { annee: choisie.annee })}</p>
      ) : null}
      {choisie === undefined ? null : <FormulaireEncaissement key={choisie.inscription.id} inscriptionId={choisie.inscription.id} surTermine={recommencer} />}
    </div>
  );
}

export function FormulaireEncaissement({
  inscriptionId,
  surTermine,
  autreEleve = true,
}: {
  inscriptionId: string;
  surTermine: () => void;
  /** Faux dans l'assistant d'inscription : l'élève est déjà choisi. */
  autreEleve?: boolean;
}) {
  const rafraichir = useRafraichir();
  const [cle, setCle] = useState(nouvelleCle);
  const [montant, setMontant] = useState("");
  const [mode, setMode] = useState<ModePaiement>("especes");
  const [reference, setReference] = useState("");
  const [payeur, setPayeur] = useState<string | null>(null);
  const [payeurNom, setPayeurNom] = useState("");
  const [manuel, setManuel] = useState(false);
  const [parts, setParts] = useState<Record<string, string>>({});
  const [recus, setRecus] = useState<RecuEmis[] | null>(null);
  const valeur = lireFrancs(montant);
  const preparation = useQuery({
    queryKey: ["preparation", inscriptionId, valeur],
    queryFn: () => appeler("encaissement_preparer", { inscription_id: inscriptionId, montant: valeur > 0 ? valeur : null }),
  });
  const p = preparation.data;
  const payeurChoisi = payeur ?? p?.payeurs[0]?.responsable_id ?? AUTRE;
  const encaissement = useMutation({
    mutationFn: () =>
      appeler("encaisser", {
        cle_idempotence: cle,
        inscription_id: inscriptionId,
        montant: valeur,
        mode,
        reference,
        payeur_id: payeurChoisi === AUTRE ? null : payeurChoisi,
        payeur_nom: payeurChoisi === AUTRE ? payeurNom : "",
        repartition: manuel
          ? Object.entries(parts)
              .map(([echeance_id, m]) => ({ echeance_id, montant: lireFrancs(m) }))
              .filter((i) => i.montant > 0)
          : null,
      }),
    onSuccess: (r) => {
      setRecus([r]);
      rafraichir();
    },
  });
  if (recus !== null) {
    return (
      <RecusEmis
        recus={recus}
        surNouveau={() => {
          setCle(nouvelleCle());
          surTermine();
        }}
      />
    );
  }
  if (p === undefined) return <MessageErreur erreur={preparation.error} />;
  const dues = p.echeances.filter((e) => e.reste > 0);
  return (
    <Section titre={t("encaissement.pour", { eleve: p.eleve, matricule: p.matricule, classe: p.classe })}>
      <p className="text-sm text-encre">{t("encaissement.situation", { reste: francs(p.reste), echu: francs(p.reste_echu) })}</p>
      {p.caisse_ouverte ? null : <p className="text-sm text-rouge">{t("caisse.session_fermee")}</p>}
      <table className="w-full text-left font-condensee text-sm tabular-nums">
        <thead>
          <tr className="text-muet">
            {(["facture.echeance", "facture.date", "encaissement.reste", "encaissement.impute"] as const).map((c) => (
              <th key={c} className="px-2 py-1 font-normal">{t(c)}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {dues.map((e) => {
            const propose = p.imputation.find((i) => i.echeance_id === e.echeance.id)?.montant ?? 0;
            return (
              <tr key={e.echeance.id} className="border-b border-muet/30 text-encre">
                <td className="px-2 py-1">{e.echeance.libelle}</td>
                <td className="px-2 py-1">{dateCivile(e.echeance.date)}</td>
                <td className="px-2 py-1">{francs(e.reste)}</td>
                <td className="px-2 py-1">
                  {manuel ? (
                    <input
                      id={`encaissement-part-${e.echeance.id}`}
                      aria-label={e.echeance.libelle}
                      inputMode="numeric"
                      className="w-28 rounded border border-muet bg-surface px-2 py-1 text-primaire"
                      value={parts[e.echeance.id] ?? ""}
                      onChange={(ev) => setParts({ ...parts, [e.echeance.id]: ev.target.value })}
                    />
                  ) : propose === 0 ? (
                    ""
                  ) : (
                    francs(propose)
                  )}
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
      <Formulaire surEnvoi={() => encaissement.mutate()}>
        <div className="flex flex-wrap items-end gap-3">
          <div className="w-48">
            <Champ id="encaissement-montant" libelle={t("encaissement.montant")} inputMode="numeric" autoFocus value={montant} onChange={(e) => setMontant(e.target.value)} />
          </div>
          <Selection
            id="encaissement-payeur"
            libelle={t("encaissement.payeur")}
            valeur={payeurChoisi}
            surChangement={setPayeur}
            options={[...p.payeurs.map((x) => ({ valeur: x.responsable_id, libelle: x.nom })), { valeur: AUTRE, libelle: t("encaissement.autre_payeur") }]}
          />
          {payeurChoisi === AUTRE ? <Champ id="encaissement-payeur-nom" libelle={t("encaissement.payeur_nom")} value={payeurNom} onChange={(e) => setPayeurNom(e.target.value)} /> : null}
        </div>
        <ChampsPaiement prefixe="encaissement" mode={mode} surMode={setMode} reference={reference} surReference={setReference} />
        <CaseACocher id="encaissement-manuel" libelle={t("encaissement.repartir")} coche={manuel} surChangement={setManuel} />
        <MessageErreur erreur={encaissement.error} />
        <div className="flex flex-wrap gap-2">
          <Bouton type="submit" disabled={encaissement.isPending || valeur === 0 || !p.caisse_ouverte}>{t("encaissement.encaisser", { montant: francs(valeur) })}</Bouton>
          {autreEleve ? <Bouton variante="secondaire" onClick={surTermine}>{t("encaissement.autre_eleve")}</Bouton> : null}
        </div>
      </Formulaire>
    </Section>
  );
}

/** Encaissement pour plusieurs enfants d'un même parent (T-15) : un reçu par enfant. */
export function EncaissementFamille() {
  const rafraichir = useRafraichir();
  const [texte, setTexte] = useState("");
  const [responsableId, setResponsableId] = useState<string | null>(null);
  const [cle, setCle] = useState(nouvelleCle);
  const [montants, setMontants] = useState<Record<string, string>>({});
  const [mode, setMode] = useState<ModePaiement>("especes");
  const [reference, setReference] = useState("");
  const [recus, setRecus] = useState<RecuEmis[] | null>(null);
  const trouves = useQuery({
    queryKey: ["caisse-responsables", texte],
    queryFn: () => appeler("responsables_rechercher", { texte }),
    enabled: texte.trim().length >= 2,
  });
  const responsable = trouves.data?.find((r) => r.responsable.id === responsableId);
  const enfants = responsable?.enfants ?? [];
  const situations = useQueries({
    queries: enfants.map((e) => ({ queryKey: ["situation", e.id], queryFn: () => appeler("situation_financiere", { id: e.id }) })),
  });
  const lignes = enfants.map((e, i) => ({ enfant: e, situation: inscriptionAEncaisser(situations[i]?.data ?? []) }));
  const encaissement = useMutation({
    mutationFn: () =>
      appeler("encaisser_famille", {
        cle_idempotence: cle,
        mode,
        reference,
        payeur_id: responsableId,
        payeur_nom: "",
        parts: lignes
          .filter((l) => l.situation !== undefined && lireFrancs(montants[l.enfant.id] ?? "") > 0)
          .map((l) => ({ inscription_id: l.situation?.inscription.id ?? "", montant: lireFrancs(montants[l.enfant.id] ?? "") })),
      }),
    onSuccess: (r) => {
      setRecus(r);
      rafraichir();
    },
  });
  if (recus !== null) {
    return (
      <RecusEmis
        recus={recus}
        surNouveau={() => {
          setRecus(null);
          setCle(nouvelleCle());
          setResponsableId(null);
          setTexte("");
          setMontants({});
        }}
      />
    );
  }
  const total = Object.values(montants).reduce((s, m) => s + lireFrancs(m), 0);
  return (
    <div className="flex flex-col gap-3">
      {responsable === undefined ? (
        <>
          <Champ id="famille-recherche" type="search" libelle={t("encaissement.famille_rechercher")} aide={t("encaissement.famille_aide")} value={texte} onChange={(e) => setTexte(e.target.value)} />
          <ul className="flex flex-col gap-1">
            {(trouves.data ?? [])
              .filter((r) => r.enfants.length > 0)
              .map((r) => (
                <li key={r.responsable.id}>
                  <button type="button" className="text-sm text-primaire underline" onClick={() => setResponsableId(r.responsable.id)}>
                    {`${r.identite.prenoms} ${r.identite.nom} · ${r.enfants.map((e) => e.nom).join(", ")}`}
                  </button>
                </li>
              ))}
          </ul>
        </>
      ) : (
        <Section titre={t("encaissement.famille_pour", { payeur: `${responsable.identite.prenoms} ${responsable.identite.nom}` })}>
          <Formulaire surEnvoi={() => encaissement.mutate()}>
            {lignes.map((l) => (
              <div key={l.enfant.id} className="flex flex-wrap items-end gap-3">
                <p className="w-56 text-sm text-encre">
                  {l.situation?.facture == null ? `${l.enfant.nom} · ${t("encaissement.aucune_facture")}` : `${l.enfant.nom} · ${l.situation.classe} · ${t("encaissement.reste_court", { reste: francs(l.situation.facture.reste) })}`}
                </p>
                {l.situation?.facture == null ? null : (
                  <div className="w-40">
                    <Champ
                      id={`famille-montant-${l.enfant.id}`}
                      libelle={t("encaissement.montant")}
                      inputMode="numeric"
                      value={montants[l.enfant.id] ?? ""}
                      onChange={(e) => setMontants({ ...montants, [l.enfant.id]: e.target.value })}
                    />
                  </div>
                )}
              </div>
            ))}
            <ChampsPaiement prefixe="famille" mode={mode} surMode={setMode} reference={reference} surReference={setReference} />
            <MessageErreur erreur={encaissement.error} />
            <div className="flex flex-wrap gap-2">
              <Bouton type="submit" disabled={encaissement.isPending || total === 0}>{t("encaissement.encaisser", { montant: francs(total) })}</Bouton>
              <Bouton variante="secondaire" onClick={() => setResponsableId(null)}>{t("encaissement.autre_payeur_famille")}</Bouton>
            </div>
          </Formulaire>
        </Section>
      )}
      <MessageErreur erreur={trouves.error} />
    </div>
  );
}
