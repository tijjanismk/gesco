import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { aujourdHui } from "../../app/aujourdhui";
import { Bouton } from "../../composants/Bouton";
import { Champ, Formulaire, Selection } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { Tableau } from "../../composants/Tableau";
import { type Cle, dateHeure, francs, moisCivil, t } from "../../i18n";
import type { AvanceAffichee, ModePaiement, PaiementFait, PaiementsPersonnel, SalaireAPayer, SourcePaiement, StatutAvance } from "../../types/gesco";
import { MODES } from "../finance/Encaissement";
import { duMois, lireMontant } from "./calcul";

const STATUTS_AVANCE: Record<StatutAvance, Cle> = {
  en_cours: "avance.statut.en_cours",
  soldee: "avance.statut.soldee",
  annulee: "avance.statut.annulee",
};

const SOURCES: Record<SourcePaiement, Cle> = {
  caisse: "paiement.source.caisse",
  hors_caisse: "paiement.source.hors_caisse",
};

/** Mode, référence et source d'un paiement, à saisir. */
function ChoixPaiement({
  prefixe,
  mode,
  surMode,
  reference,
  surReference,
  source,
  surSource,
  caisseOuverte,
}: {
  prefixe: string;
  mode: ModePaiement;
  surMode: (m: ModePaiement) => void;
  reference: string;
  surReference: (r: string) => void;
  source: SourcePaiement;
  surSource: (s: SourcePaiement) => void;
  caisseOuverte: boolean;
}) {
  const sources: SourcePaiement[] = mode === "especes" && caisseOuverte ? ["caisse", "hors_caisse"] : ["hors_caisse"];
  return (
    <div className="flex flex-wrap items-end gap-3">
      <Selection
        id={`${prefixe}-mode`}
        libelle={t("encaissement.mode")}
        valeur={mode}
        surChangement={(v) => {
          const m = (Object.keys(MODES) as ModePaiement[]).find((x) => x === v) ?? "especes";
          surMode(m);
          surSource(m === "especes" && caisseOuverte ? "caisse" : "hors_caisse");
        }}
        options={(Object.keys(MODES) as ModePaiement[]).map((m) => ({ valeur: m, libelle: t(MODES[m]) }))}
      />
      <Selection
        id={`${prefixe}-source`}
        libelle={t("paiement.source")}
        valeur={sources.includes(source) ? source : "hors_caisse"}
        surChangement={(v) => surSource(v === "caisse" ? "caisse" : "hors_caisse")}
        options={sources.map((s) => ({ valeur: s, libelle: t(SOURCES[s]) }))}
      />
      {mode === "especes" ? null : (
        <Champ id={`${prefixe}-reference`} libelle={t("encaissement.reference")} value={reference} onChange={(e) => surReference(e.target.value)} autoComplete="off" />
      )}
    </div>
  );
}

/** Paiements du personnel (SPEC_PHASE_7 §4 et §7) : les salaires validés qui restent à payer,
 *  en une ou plusieurs fois ; les avances ; les paiements, qu'une erreur fait annuler. */
export function Paiements() {
  const client = useQueryClient();
  const etat = useQuery({ queryKey: ["paiements-personnel"], queryFn: () => appeler("paiements_personnel_lire", null) });
  const [avertissements, setAvertissements] = useState<string[]>([]);
  const apres = (f: PaiementFait) => {
    client.setQueryData(["paiements-personnel"], f.etat);
    setAvertissements(f.avertissements);
    void client.invalidateQueries({ queryKey: ["caisse"] });
  };
  const e = etat.data;
  return (
    <div className="flex flex-col gap-4">
      <MessageErreur erreur={etat.error} />
      {e === undefined ? null : (
        <>
          <p className="text-sm text-encre">
            {e.caisse_ouverte === null ? t("paiement.caisse_absente") : t("paiement.caisse_ouverte", { tiroir: francs(e.caisse_ouverte) })}
          </p>
          {avertissements.map((a) => (
            <p key={a} role="status" className="text-sm text-ocre">
              {t(a as Cle)}
            </p>
          ))}
          <SalairesAPayer e={e} surFait={apres} />
          <Avances e={e} surFait={apres} />
          <Historique e={e} surFait={apres} />
        </>
      )}
    </div>
  );
}

function SalairesAPayer({ e, surFait }: { e: PaiementsPersonnel; surFait: (f: PaiementFait) => void }) {
  const [choisi, setChoisi] = useState<string | null>(null);
  const [confirmer, setConfirmer] = useState<string | null>(null);
  const reste = useMutation({
    mutationFn: (mois: string) => appeler("paie_payer_reste", { mois }),
    onSuccess: (f) => {
      setConfirmer(null);
      surFait(f);
    },
  });
  const mois = [...new Set(e.a_payer.map((a) => a.mois))];
  const salaire = e.a_payer.find((a) => a.bulletin_id === choisi);
  return (
    <Section titre={t("paiement.a_payer", { total: francs(e.total_du) })}>
      {e.a_payer.length === 0 ? (
        <p className="text-sm text-muet">{t("paiement.rien_du")}</p>
      ) : (
        <>
          <Tableau
            nomExport="salaires-a-payer"
            titre={t("paiement.a_payer", { total: francs(e.total_du) })}
            lignes={e.a_payer}
            cle={(a) => a.bulletin_id}
            surSelection={(a) => setChoisi(a.bulletin_id)}
            colonnes={[
              { titre: t("paiement.mois"), valeur: (a) => moisCivil(a.mois) },
              { titre: t("paie.mois.membre"), valeur: (a) => a.membre },
              { titre: t("paie.mois.numero"), valeur: (a) => a.numero },
              { titre: t("paie.mois.net"), valeur: (a) => francs(a.net) },
              { titre: t("paiement.paye"), valeur: (a) => francs(a.paye) },
              { titre: t("paiement.reste"), valeur: (a) => francs(a.reste) },
            ]}
          />
          <div className="flex flex-wrap gap-2">
            {mois.map((m) => {
              const dus = e.a_payer.filter((a) => a.mois === m);
              const total = dus.reduce((s, a) => s + a.reste, 0);
              return confirmer === m ? (
                <span key={m} className="flex gap-2">
                  <Bouton variante="danger" onClick={() => reste.mutate(m)} disabled={reste.isPending}>
                    {t("paiement.confirmer_reste", { nombre: dus.length, total: francs(total) })}
                  </Bouton>
                  <Bouton variante="secondaire" onClick={() => setConfirmer(null)}>
                    {t("commun.annuler")}
                  </Bouton>
                </span>
              ) : (
                <Bouton key={m} variante="secondaire" onClick={() => setConfirmer(m)}>
                  {t("paiement.payer_reste", { mois: duMois(m) })}
                </Bouton>
              );
            })}
          </div>
          <MessageErreur erreur={reste.error} />
        </>
      )}
      {salaire === undefined ? null : <PayerSalaire key={salaire.bulletin_id} salaire={salaire} caisseOuverte={e.caisse_ouverte !== null} surFait={surFait} />}
    </Section>
  );
}

function PayerSalaire({ salaire, caisseOuverte, surFait }: { salaire: SalaireAPayer; caisseOuverte: boolean; surFait: (f: PaiementFait) => void }) {
  const [montant, setMontant] = useState(String(salaire.reste));
  const [mode, setMode] = useState<ModePaiement>(salaire.mode_propose);
  const [reference, setReference] = useState("");
  const [source, setSource] = useState<SourcePaiement>(salaire.mode_propose === "especes" && caisseOuverte ? "caisse" : "hors_caisse");
  const lu = lireMontant(montant);
  const payer = useMutation({
    mutationFn: () => appeler("salaire_payer", { bulletin_id: salaire.bulletin_id, montant: lu ?? 0, mode, reference, source }),
    onSuccess: surFait,
  });
  return (
    <Formulaire surEnvoi={() => payer.mutate()}>
      <p className="text-sm font-medium text-encre">{t("paiement.payer_de", { membre: salaire.membre, mois: moisCivil(salaire.mois), reste: francs(salaire.reste) })}</p>
      <div className="flex flex-wrap items-end gap-3">
        <Champ id="paiement-montant" libelle={t("paie.mois.montant")} value={montant} onChange={(ev) => setMontant(ev.target.value)} autoComplete="off" />
        <ChoixPaiement prefixe="paiement" mode={mode} surMode={setMode} reference={reference} surReference={setReference} source={source} surSource={setSource} caisseOuverte={caisseOuverte} />
        <Bouton type="submit" disabled={lu === null || payer.isPending}>
          {t("paiement.payer")}
        </Bouton>
      </div>
      <MessageErreur erreur={payer.error} />
    </Formulaire>
  );
}

function Avances({ e, surFait }: { e: PaiementsPersonnel; surFait: (f: PaiementFait) => void }) {
  const [membreId, setMembreId] = useState("");
  const [montant, setMontant] = useState("");
  const [mensualites, setMensualites] = useState("3");
  const [premier, setPremier] = useState(aujourdHui().slice(0, 7));
  const [motif, setMotif] = useState("");
  const [mode, setMode] = useState<ModePaiement>("especes");
  const [reference, setReference] = useState("");
  const [source, setSource] = useState<SourcePaiement>(e.caisse_ouverte === null ? "hors_caisse" : "caisse");
  const lu = lireMontant(montant);
  const accorder = useMutation({
    mutationFn: () =>
      appeler("avance_accorder", {
        membre_id: membreId,
        montant: lu ?? 0,
        mensualites: Number.parseInt(mensualites, 10) || 0,
        premier_mois: premier,
        motif,
        mode,
        reference,
        source,
      }),
    onSuccess: (f) => {
      setMontant("");
      setMotif("");
      setReference("");
      surFait(f);
    },
  });
  return (
    <Section titre={t("avance.titre")}>
      <p className="text-sm text-muet">{t("avance.aide")}</p>
      {e.avances.length === 0 ? null : (
        <ul className="flex flex-col gap-1">
          {e.avances.map((a) => (
            <LigneAvance key={a.avance.id} a={a} surFait={surFait} />
          ))}
        </ul>
      )}
      <Formulaire surEnvoi={() => accorder.mutate()}>
        <div className="flex flex-wrap items-end gap-3">
          <Selection
            id="avance-membre"
            libelle={t("paie.mois.membre")}
            valeur={membreId}
            surChangement={setMembreId}
            options={[{ valeur: "", libelle: "" }, ...e.membres.map(([id, nom]) => ({ valeur: id, libelle: nom }))]}
          />
          <Champ id="avance-montant" libelle={t("paie.mois.montant")} value={montant} onChange={(ev) => setMontant(ev.target.value)} autoComplete="off" />
          <Champ id="avance-mensualites" type="number" min={1} max={24} libelle={t("avance.mensualites")} value={mensualites} onChange={(ev) => setMensualites(ev.target.value)} />
          <Champ id="avance-premier" type="month" libelle={t("avance.premier_mois")} value={premier} onChange={(ev) => setPremier(ev.target.value)} />
          <Champ id="avance-motif" libelle={t("paie.mois.motif")} value={motif} onChange={(ev) => setMotif(ev.target.value)} autoComplete="off" />
        </div>
        <ChoixPaiement prefixe="avance" mode={mode} surMode={setMode} reference={reference} surReference={setReference} source={source} surSource={setSource} caisseOuverte={e.caisse_ouverte !== null} />
        <div>
          <Bouton type="submit" disabled={membreId === "" || lu === null || motif.trim() === "" || accorder.isPending}>
            {t("avance.accorder")}
          </Bouton>
        </div>
        <MessageErreur erreur={accorder.error} />
      </Formulaire>
    </Section>
  );
}

function LigneAvance({ a, surFait }: { a: AvanceAffichee; surFait: (f: PaiementFait) => void }) {
  const [motif, setMotif] = useState<string | null>(null);
  const annuler = useMutation({
    mutationFn: () => appeler("avance_annuler", { id: a.avance.id, motif: motif ?? "" }),
    onSuccess: (f) => {
      setMotif(null);
      surFait(f);
    },
  });
  const annulable = a.avance.statut === "en_cours" && a.rembourse === 0;
  return (
    <li className="flex flex-wrap items-center gap-3 text-sm text-encre">
      <span>
        {t("avance.ligne", { membre: a.membre, montant: francs(a.avance.montant), mensualite: francs(a.mensualite), reste: francs(a.reste) })}
      </span>
      <span className="text-muet">{t(STATUTS_AVANCE[a.avance.statut])}</span>
      {annulable && motif === null ? (
        <Bouton variante="secondaire" onClick={() => setMotif("")}>
          {t("avance.annuler")}
        </Bouton>
      ) : null}
      {motif === null ? null : (
        <span className="flex items-end gap-2">
          <Champ id={`avance-annulation-${a.avance.id}`} libelle={t("paie.mois.motif")} value={motif} onChange={(ev) => setMotif(ev.target.value)} autoComplete="off" />
          <Bouton variante="danger" onClick={() => annuler.mutate()} disabled={motif.trim() === "" || annuler.isPending}>
            {t("avance.confirmer_annulation")}
          </Bouton>
        </span>
      )}
      <MessageErreur erreur={annuler.error} />
    </li>
  );
}

function Historique({ e, surFait }: { e: PaiementsPersonnel; surFait: (f: PaiementFait) => void }) {
  const [choisi, setChoisi] = useState<string | null>(null);
  const [motif, setMotif] = useState("");
  const annuler = useMutation({
    mutationFn: () => appeler("paiement_personnel_annuler", { id: choisi ?? "", motif }),
    onSuccess: (f) => {
      setChoisi(null);
      setMotif("");
      surFait(f);
    },
  });
  if (e.paiements.length === 0) return null;
  const paiement = e.paiements.find((p) => p.paiement.id === choisi);
  return (
    <Section titre={t("paiement.historique")}>
      <Tableau
        nomExport="paiements-du-personnel"
        titre={t("paiement.historique")}
        lignes={e.paiements}
        cle={(p) => p.paiement.id}
        surSelection={(p) => setChoisi(p.paiement.id)}
        colonnes={[
          { titre: t("paiement.le"), valeur: (p) => dateHeure(p.paiement.le) },
          { titre: t("paie.mois.membre"), valeur: (p) => p.membre },
          { titre: t("paiement.objet"), valeur: (p) => (p.paiement.objet === "avance" ? t("paiement.objet.avance") : t("paiement.objet.salaire", { mois: p.mois === null ? "" : moisCivil(p.mois) })) },
          { titre: t("paie.mois.montant"), valeur: (p) => francs(p.paiement.montant) },
          { titre: t("encaissement.mode"), valeur: (p) => `${t(MODES[p.paiement.mode])}${p.paiement.reference === "" ? "" : ` · ${p.paiement.reference}`}` },
          { titre: t("paiement.source"), valeur: (p) => t(SOURCES[p.paiement.source]) },
          { titre: t("paiement.etat"), valeur: (p) => (p.paiement.annule ? t("paiement.annule", { motif: p.paiement.motif_annulation }) : "") },
        ]}
      />
      {paiement === undefined || paiement.paiement.annule || paiement.paiement.objet === "avance" ? null : (
        <Formulaire surEnvoi={() => annuler.mutate()}>
          <div className="flex flex-wrap items-end gap-3">
            <Champ id="paiement-annulation" libelle={t("paiement.motif_annulation", { membre: paiement.membre, montant: francs(paiement.paiement.montant) })} value={motif} onChange={(ev) => setMotif(ev.target.value)} autoComplete="off" />
            <Bouton type="submit" variante="danger" disabled={motif.trim() === "" || annuler.isPending}>
              {t("paiement.annuler")}
            </Bouton>
          </div>
          <MessageErreur erreur={annuler.error} />
        </Formulaire>
      )}
    </Section>
  );
}
