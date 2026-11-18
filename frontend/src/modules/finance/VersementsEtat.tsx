import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { aujourdHui } from "../../app/aujourdhui";
import { commandeAutorisee } from "../../app/navigation";
import { Bouton } from "../../composants/Bouton";
import { CaseACocher, Champ, Selection } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { Tableau } from "../../composants/Tableau";
import { dateCivile, francs, t } from "../../i18n";
import type { AvertissementPartEtat, ModePaiement, OriginePartEtat, VersementEtatAffiche } from "../../types/gesco";
import { ChoixCompte, modesDu } from "./Depenses";
import { MODES } from "./Encaissement";
import { APRES_PRISE_EN_CHARGE, depassementAttendu, lireFrancs, memeMontantPourChaque, partsDeLaSaisie, piedRepartition, type SaisieRepartition, saisieInitiale } from "./prise_en_charge";
import { filtrerLignes } from "./reprise";

const ORIGINES: Record<OriginePartEtat, "vetat.origine.ecran" | "vetat.origine.import"> = {
  ecran: "vetat.origine.ecran",
  import: "vetat.origine.import",
};

/** Relit tout ce qu'un versement de l'État change : ses listes, les élèves, la trésorerie. */
function useRelire() {
  const client = useQueryClient();
  return () => {
    for (const cle of [...APRES_PRISE_EN_CHARGE, "versement-etat", "recettes", "caisse", "comptes-soldes"]) void client.invalidateQueries({ queryKey: [cle] });
  };
}

function compteAffiche(v: VersementEtatAffiche): string {
  if (v.versement.avant_gesco) return t("vetat.avant_gesco_court");
  const mode = v.mode === null ? "" : ` · ${t(MODES[v.mode])}`;
  return `${v.compte}${mode}`;
}

/** Onglet « Versements de l'État » : la liste de l'année, le nouveau versement, la répartition
 *  d'un versement à modifier, compléter ou annuler. */
export function VersementsEtat({ anneeId, permissions }: { anneeId: string; permissions: readonly string[] }) {
  const liste = useQuery({ queryKey: ["versements-etat", anneeId], queryFn: () => appeler("versements_etat_lister", { annee_id: anneeId }) });
  const gere = commandeAutorisee("versement_etat_enregistrer", permissions);
  const [nouveau, setNouveau] = useState(false);
  const [ouvert, setOuvert] = useState<string | null>(null);
  const d = liste.data;
  return (
    <div className="flex flex-col gap-3">
      <MessageErreur erreur={liste.error} />
      {d === undefined ? null : (
        <>
          <p className="text-sm font-medium text-encre">
            {t("vetat.totaux", { nombre: d.nombre, recu: francs(d.total_recu), reparti: francs(d.total_reparti), non_reparti: francs(d.total_non_reparti) })}
          </p>
          {d.annee_close ? <p className="text-sm text-muet">{t("vetat.annee_close")}</p> : null}
          {gere && !nouveau ? (
            <div>
              <Bouton onClick={() => setNouveau(true)}>{t("vetat.nouveau")}</Bouton>
            </div>
          ) : null}
          {nouveau ? (
            <NouveauVersement
              anneeId={anneeId}
              permissions={permissions}
              surFini={(id) => {
                setNouveau(false);
                setOuvert(id);
              }}
              surAnnuler={() => setNouveau(false)}
            />
          ) : null}
          {ouvert === null ? null : <DetailVersement key={ouvert} versementId={ouvert} permissions={permissions} surFermer={() => setOuvert(null)} />}
          <Tableau
            nomExport="versements-etat"
            titre={t("pec.onglet.versements")}
            lignes={d.versements}
            cle={(v) => v.versement.id}
            surSelection={(v) => setOuvert(v.versement.id)}
            colonnes={[
              { titre: t("vetat.date"), valeur: (v) => dateCivile(v.date_affichee) },
              { titre: t("vetat.reference"), valeur: (v) => v.versement.reference },
              { titre: t("vetat.libelle"), valeur: (v) => v.versement.libelle },
              { titre: t("vetat.compte"), valeur: compteAffiche },
              { titre: t("vetat.montant"), valeur: (v) => francs(v.versement.montant) },
              { titre: t("vetat.reparti"), valeur: (v) => francs(v.reparti) },
              { titre: t("vetat.non_reparti"), valeur: (v) => (v.non_reparti === 0 ? "" : francs(v.non_reparti)) },
              { titre: t("vetat.eleves"), valeur: (v) => String(v.nombre_eleves) },
              {
                titre: t("vetat.etat"),
                valeur: (v) => (v.versement.statut === "annule" ? t("vetat.annule", { motif: v.versement.motif_annulation }) : t("vetat.valide")),
              },
            ]}
          />
        </>
      )}
    </div>
  );
}

/** « Nouveau versement de l'État » : l'argent, puis la répartition sur les élèves cochés. */
function NouveauVersement({
  anneeId,
  permissions,
  surFini,
  surAnnuler,
}: {
  anneeId: string;
  permissions: readonly string[];
  surFini: (versementId: string) => void;
  surAnnuler: () => void;
}) {
  const relire = useRelire();
  const annees = useQuery({ queryKey: ["annees"], queryFn: () => appeler("annees_lister", null) });
  const mois = aujourdHui().slice(0, 7);
  const recettes = useQuery({ queryKey: ["recettes", mois], queryFn: () => appeler("recettes_lire", { mois }) });
  const comptes = recettes.data?.comptes ?? [];
  const banque = comptes.find((c) => c.nature === "banque") ?? comptes[0];
  const avantPermis = commandeAutorisee("prise_en_charge_regler", permissions);
  const [annee, setAnnee] = useState(anneeId);
  const [avantGesco, setAvantGesco] = useState(false);
  const [date, setDate] = useState(aujourdHui());
  const [montant, setMontant] = useState("");
  const [compteChoisi, setCompteChoisi] = useState<string | null>(null);
  const [modeChoisi, setModeChoisi] = useState<ModePaiement | null>(null);
  const [reference, setReference] = useState("");
  const [libelle, setLibelle] = useState(t("vetat.libelle_defaut"));
  const [payeur, setPayeur] = useState(t("vetat.payeur_defaut"));
  const [saisies, setSaisies] = useState<Record<string, SaisieRepartition> | null>(null);
  const compteId = compteChoisi ?? banque?.id ?? "";
  const compte = comptes.find((c) => c.id === compteId);
  const mode = modeChoisi ?? modesDu(compte)[0] ?? "virement";
  const lu = lireFrancs(montant);
  // Un nouveau versement part sans élève coché : sans saisie, rien n'est réparti.
  const parts = partsDeLaSaisie(saisies ?? {});
  const pied = piedRepartition(lu, parts);
  const [avertissements, setAvertissements] = useState<AvertissementPartEtat[]>([]);
  const enregistrement = useMutation({
    mutationFn: () =>
      appeler("versement_etat_enregistrer", {
        annee_id: annee,
        date: avantGesco ? "" : date,
        montant: lu ?? 0,
        mode: avantGesco ? null : mode,
        compte_id: avantGesco || compteId === "" ? null : compteId,
        reference: reference.trim(),
        libelle: libelle.trim(),
        payeur: payeur.trim(),
        avant_gesco: avantGesco,
        parts,
      }),
    onSuccess: (r) => {
      relire();
      if (r.avertissements.length > 0) setAvertissements(r.avertissements);
      else surFini(r.detail.versement.versement.id);
    },
  });
  const referenceManque = !avantGesco && mode !== "especes" && reference.trim() === "";
  if (enregistrement.data !== undefined && avertissements.length > 0) {
    return (
      <Section titre={t("vetat.enregistre")}>
        <Avertissements avertissements={avertissements} permissions={permissions} />
        <div>
          <Bouton onClick={() => surFini(enregistrement.data.detail.versement.versement.id)}>{t("vetat.voir_repartition")}</Bouton>
        </div>
      </Section>
    );
  }
  return (
    <Section titre={t("vetat.nouveau")}>
      <p className="text-sm text-muet">{t("vetat.aide")}</p>
      <MessageErreur erreur={annees.error ?? recettes.error} />
      <p className="text-sm font-medium text-encre">{t("vetat.etape_argent")}</p>
      <div className="flex flex-wrap items-end gap-3">
        <Selection
          id="vetat-annee"
          libelle={t("vetat.annee_concernee")}
          valeur={annee}
          surChangement={(v) => {
            setAnnee(v);
            setSaisies(null);
          }}
          options={(annees.data ?? []).map((a) => ({ valeur: a.id, libelle: a.libelle }))}
        />
        {avantGesco ? null : <Champ id="vetat-date" type="date" libelle={t("vetat.date_reception")} value={date} onChange={(e) => setDate(e.target.value)} />}
        <Champ id="vetat-montant" libelle={t("vetat.montant")} inputMode="numeric" value={montant} onChange={(e) => setMontant(e.target.value)} autoComplete="off" autoFocus />
        <Champ id="vetat-libelle" libelle={t("vetat.libelle")} value={libelle} onChange={(e) => setLibelle(e.target.value)} autoComplete="off" />
        <Champ id="vetat-payeur" libelle={t("vetat.payeur")} value={payeur} onChange={(e) => setPayeur(e.target.value)} autoComplete="off" />
      </div>
      {avantPermis ? (
        <CaseACocher id="vetat-avant-gesco" libelle={t("vetat.avant_gesco")} coche={avantGesco} surChangement={setAvantGesco} />
      ) : null}
      {avantGesco ? (
        <>
          <p className="text-sm text-muet">{t("vetat.avant_gesco_aide")}</p>
          <Champ id="vetat-reference-avant" libelle={t("vetat.reference")} value={reference} onChange={(e) => setReference(e.target.value)} autoComplete="off" />
        </>
      ) : comptes.length === 0 ? null : (
        <ChoixCompte
          prefixe="vetat"
          comptes={comptes}
          compteId={compteId}
          surCompte={(id, m) => {
            setCompteChoisi(id);
            setModeChoisi(m);
          }}
          mode={mode}
          surMode={setModeChoisi}
          reference={reference}
          surReference={setReference}
        />
      )}
      <p className="text-sm font-medium text-encre">{t("vetat.etape_repartition")}</p>
      <EditeurRepartition
        key={annee}
        anneeId={annee}
        versementId={null}
        recu={lu}
        saisies={saisies}
        surSaisies={setSaisies}
        permissions={permissions}
      />
      <MessageErreur erreur={enregistrement.error} />
      <div className="flex flex-wrap gap-2">
        <Bouton onClick={() => enregistrement.mutate()} disabled={enregistrement.isPending || lu === null || lu <= 0 || pied.refuse || referenceManque}>
          {t("vetat.enregistrer")}
        </Bouton>
        <Bouton variante="secondaire" onClick={surAnnuler}>{t("commun.annuler")}</Bouton>
        {referenceManque && lu !== null ? <p className="w-full text-sm text-ocre">{t("vetat.reference_exigee")}</p> : null}
      </div>
    </Section>
  );
}

/** Répartition d'un versement sur les élèves : cases à cocher, montant prérempli = reste dû par
 *  l'État, « même montant pour chaque élève coché », pied reçu / réparti / non réparti, et la
 *  part au-delà de ce que l'État doit, avec le bouton qui porte l'attendu. */
function EditeurRepartition({
  anneeId,
  versementId,
  recu,
  saisies,
  surSaisies,
  permissions,
}: {
  anneeId: string | null;
  versementId: string | null;
  recu: number | null;
  saisies: Record<string, SaisieRepartition> | null;
  surSaisies: (s: Record<string, SaisieRepartition>) => void;
  permissions: readonly string[];
}) {
  const client = useQueryClient();
  const proposee = useQuery({
    queryKey: ["repartition-etat", anneeId, versementId],
    queryFn: () => appeler("versement_etat_repartition_proposee", { annee_id: anneeId, versement_id: versementId }),
  });
  const [recherche, setRecherche] = useState("");
  const [meme, setMeme] = useState("");
  const eleves = proposee.data?.eleves ?? [];
  const courantes = saisies ?? saisieInitiale(eleves);
  const changer = (id: string, s: SaisieRepartition) => surSaisies({ ...courantes, [id]: s });
  const pied = piedRepartition(recu, partsDeLaSaisie(courantes));
  const memeLu = lireFrancs(meme);
  const porteur = commandeAutorisee("prise_en_charge_regler", permissions);
  const porter = useMutation({
    mutationFn: ({ inscriptionId, attendu }: { inscriptionId: string; attendu: number }) =>
      appeler("prise_en_charge_regler", { cibles: [{ inscription_id: inscriptionId, lue_le: null }], geste: "modifier", montant_attendu: attendu }),
    onSuccess: () => {
      for (const cle of APRES_PRISE_EN_CHARGE) void client.invalidateQueries({ queryKey: [cle] });
    },
  });
  const affiches = filtrerLignes(eleves, recherche);
  return (
    <div className="flex flex-col gap-2">
      <MessageErreur erreur={proposee.error} />
      {proposee.data?.annee_close ? <p className="text-sm text-muet">{t("vetat.annee_close")}</p> : null}
      <div className="flex flex-wrap items-end gap-2">
        <Champ id={`vetat-recherche-${versementId ?? "nouveau"}`} type="search" libelle={t("pec.rechercher")} value={recherche} onChange={(e) => setRecherche(e.target.value)} autoComplete="off" />
        <Champ id={`vetat-meme-${versementId ?? "nouveau"}`} libelle={t("vetat.meme_montant")} inputMode="numeric" value={meme} onChange={(e) => setMeme(e.target.value)} autoComplete="off" />
        <Bouton variante="secondaire" disabled={memeLu === null} onClick={() => memeLu === null || surSaisies(memeMontantPourChaque(courantes, memeLu))}>
          {t("vetat.appliquer_meme")}
        </Bouton>
        <Bouton
          variante="secondaire"
          onClick={() => {
            const toutes = affiches.every((e) => courantes[e.inscription_id]?.coche === true);
            const suite = { ...courantes };
            for (const e of affiches) suite[e.inscription_id] = { coche: !toutes, texte: courantes[e.inscription_id]?.texte ?? "" };
            surSaisies(suite);
          }}
        >
          {t("vetat.tout_cocher")}
        </Bouton>
      </div>
      {eleves.length === 0 && proposee.isSuccess ? <p className="text-sm text-muet">{t("vetat.aucun_eleve")}</p> : null}
      {affiches.length === 0 ? null : (
        <div className="overflow-x-auto">
          <table className="w-full border-collapse font-condensee text-sm tabular-nums">
            <thead>
              <tr className="text-left text-muet">
                {(["vetat.col.repartir", "pec.classe", "pec.eleve", "pec.matricule", "pec.reference", "pec.attendu", "vetat.col.deja_recu", "pec.reste_etat", "vetat.col.montant"] as const).map((c) => (
                  <th key={c} className="border-b border-muet px-2 py-1 font-medium">{t(c)}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {affiches.map((e) => {
                const s = courantes[e.inscription_id] ?? { coche: false, texte: "" };
                const lu = lireFrancs(s.texte);
                const plus = s.coche && lu !== null ? depassementAttendu(e, lu) : null;
                return (
                  <tr key={e.inscription_id} className="border-b border-muet/30 text-encre">
                    <td className="px-2 py-1">
                      <input
                        type="checkbox"
                        className="accent-primaire"
                        aria-label={t("vetat.repartir_eleve", { eleve: e.eleve })}
                        checked={s.coche}
                        onChange={(ev) => changer(e.inscription_id, { ...s, coche: ev.target.checked })}
                      />
                    </td>
                    <td className="px-2 py-1">{e.classe}</td>
                    <td className="px-2 py-1">{e.pris_en_charge ? e.eleve : t("vetat.decoche", { eleve: e.eleve })}</td>
                    <td className="px-2 py-1">{e.matricule}</td>
                    <td className="px-2 py-1">{e.reference}</td>
                    <td className="px-2 py-1">{francs(e.attendu)}</td>
                    <td className="px-2 py-1">{e.deja_recu === 0 ? "" : francs(e.deja_recu)}</td>
                    <td className="px-2 py-1">{francs(e.reste_du_etat)}</td>
                    <td className="px-2 py-1">
                      <div className="flex flex-col gap-1">
                        <input
                          aria-label={t("vetat.montant_eleve", { eleve: e.eleve })}
                          className="w-28 rounded border border-muet bg-surface px-2 py-0.5 text-right text-primaire focus:outline-2 focus:outline-primaire"
                          inputMode="numeric"
                          autoComplete="off"
                          value={s.texte}
                          onChange={(ev) => changer(e.inscription_id, { coche: true, texte: ev.target.value })}
                        />
                        {plus === null ? null : (
                          <span className="text-xs text-ocre">
                            {t("vetat.plus_que_attendu", { montant: francs(plus.en_plus) })}{" "}
                            {porteur && plus.attendu_porte !== null ? (
                              <button
                                type="button"
                                className="underline decoration-dotted underline-offset-2 hover:text-encre"
                                disabled={porter.isPending}
                                onClick={() => porter.mutate({ inscriptionId: e.inscription_id, attendu: plus.attendu_porte ?? e.attendu })}
                              >
                                {t("vetat.porter_attendu", { montant: francs(plus.attendu_porte) })}
                              </button>
                            ) : null}
                          </span>
                        )}
                      </div>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
      <MessageErreur erreur={porter.error} />
      <p className={`text-sm font-medium ${pied.refuse ? "text-rouge" : "text-encre"}`}>
        {t("vetat.pied", { recu: francs(pied.recu), reparti: francs(pied.reparti), non_reparti: francs(pied.non_reparti), nombre: pied.nombre })}
      </p>
      {pied.refuse ? <p className="text-sm text-rouge">{t("vetat.depassement", { montant: francs(pied.depassement) })}</p> : null}
    </div>
  );
}

/** Élèves qui ont reçu plus que ce que l'État leur doit, après l'enregistrement. */
function Avertissements({ avertissements, permissions }: { avertissements: readonly AvertissementPartEtat[]; permissions: readonly string[] }) {
  const client = useQueryClient();
  const porteur = commandeAutorisee("prise_en_charge_regler", permissions);
  const [portes, setPortes] = useState<readonly string[]>([]);
  const porter = useMutation({
    mutationFn: (a: AvertissementPartEtat) =>
      appeler("prise_en_charge_regler", { cibles: [{ inscription_id: a.inscription_id, lue_le: null }], geste: "modifier", montant_attendu: a.attendu_porte }),
    onSuccess: (_, a) => {
      setPortes((p) => [...p, a.inscription_id]);
      for (const cle of APRES_PRISE_EN_CHARGE) void client.invalidateQueries({ queryKey: [cle] });
    },
  });
  return (
    <div className="flex flex-col gap-1 text-sm">
      {avertissements.map((a) => (
        <div key={a.inscription_id} className="flex flex-wrap items-center gap-2">
          <span className="text-ocre">{t("vetat.avertissement", { eleve: a.eleve, montant: francs(a.en_plus) })}</span>
          {porteur && a.attendu_porte !== null && !portes.includes(a.inscription_id) ? (
            <Bouton variante="secondaire" onClick={() => porter.mutate(a)} disabled={porter.isPending}>
              {t("vetat.porter_attendu", { montant: francs(a.attendu_porte) })}
            </Bouton>
          ) : null}
          {portes.includes(a.inscription_id) ? <span className="text-muet">{t("vetat.attendu_porte")}</span> : null}
        </div>
      ))}
      <MessageErreur erreur={porter.error} />
    </div>
  );
}

/** Un versement de l'État et sa répartition : modifier ou compléter la répartition, annuler le
 *  versement. Ouvert depuis la liste des versements ou depuis la liste des recettes. */
export function DetailVersement({ versementId, permissions, surFermer }: { versementId: string; permissions: readonly string[]; surFermer: () => void }) {
  const relire = useRelire();
  const detail = useQuery({ queryKey: ["versement-etat", versementId], queryFn: () => appeler("versement_etat_lire", { versement_id: versementId }) });
  const gere = commandeAutorisee("versement_etat_repartir", permissions);
  const [mode, setMode] = useState<"lecture" | "repartition" | "annulation">("lecture");
  const [saisies, setSaisies] = useState<Record<string, SaisieRepartition> | null>(null);
  const [motif, setMotif] = useState("");
  const [avertissements, setAvertissements] = useState<AvertissementPartEtat[]>([]);
  const d = detail.data;
  const repartition = useMutation({
    mutationFn: () =>
      appeler("versement_etat_repartir", { versement_id: versementId, parts: partsDeLaSaisie(saisies ?? {}), lue_le: d?.versement.lue_le ?? null }),
    onSuccess: (r) => {
      setAvertissements(r.avertissements);
      setMode("lecture");
      setSaisies(null);
      relire();
    },
  });
  const annulation = useMutation({
    mutationFn: () => appeler("versement_etat_annuler", { versement_id: versementId, motif, lue_le: d?.versement.lue_le ?? null }),
    onSuccess: () => {
      setMode("lecture");
      relire();
    },
  });
  if (d === undefined) return <MessageErreur erreur={detail.error} />;
  const v = d.versement;
  const valide = v.versement.statut === "valide";
  const pied = piedRepartition(v.versement.montant, partsDeLaSaisie(saisies ?? {}));
  return (
    <Section titre={t("vetat.detail", { date: dateCivile(v.date_affichee), montant: francs(v.versement.montant) })}>
      <div className="flex flex-col gap-1 text-sm text-encre">
        <p>{t("vetat.detail_ligne", { reference: v.versement.reference === "" ? "—" : v.versement.reference, libelle: v.versement.libelle, payeur: v.versement.payeur, compte: compteAffiche(v), annee: d.annee })}</p>
        <p className="font-medium">{t("vetat.detail_repartition", { reparti: francs(v.reparti), non_reparti: francs(v.non_reparti), nombre: v.nombre_eleves })}</p>
        {valide ? null : <p className="text-rouge">{t("vetat.annule", { motif: v.versement.motif_annulation })}</p>}
      </div>
      <Avertissements avertissements={avertissements} permissions={permissions} />
      {mode === "repartition" ? (
        <>
          <EditeurRepartition anneeId={v.versement.annee_id} versementId={versementId} recu={v.versement.montant} saisies={saisies} surSaisies={setSaisies} permissions={permissions} />
          <MessageErreur erreur={repartition.error} surRelire={() => void detail.refetch()} />
          <div className="flex flex-wrap gap-2">
            <Bouton onClick={() => repartition.mutate()} disabled={repartition.isPending || saisies === null || pied.refuse}>{t("vetat.enregistrer_repartition")}</Bouton>
            <Bouton
              variante="secondaire"
              onClick={() => {
                setMode("lecture");
                setSaisies(null);
              }}
            >
              {t("commun.annuler")}
            </Bouton>
          </div>
        </>
      ) : (
        <>
          {d.parts.length === 0 ? (
            <p className="text-sm text-muet">{t("vetat.sans_repartition")}</p>
          ) : (
            <Tableau
              nomExport={`versement-etat-${v.date_affichee}`}
              titre={t("vetat.detail", { date: dateCivile(v.date_affichee), montant: francs(v.versement.montant) })}
              lignes={d.parts}
              cle={(p) => p.inscription_id}
              colonnes={[
                { titre: t("pec.classe"), valeur: (p) => p.classe },
                { titre: t("pec.eleve"), valeur: (p) => p.eleve },
                { titre: t("pec.matricule"), valeur: (p) => p.matricule },
                { titre: t("vetat.montant"), valeur: (p) => francs(p.montant) },
                { titre: t("vetat.origine"), valeur: (p) => t(ORIGINES[p.origine]) },
              ]}
            />
          )}
          {mode === "annulation" ? (
            <div className="flex flex-col gap-2 rounded border border-ocre p-2 text-sm">
              <p className="text-encre">{t(v.versement.avant_gesco ? "vetat.annuler_explication_avant" : "vetat.annuler_explication")}</p>
              <div className="flex flex-wrap items-end gap-2">
                <Champ id={`vetat-motif-${versementId}`} libelle={t("vetat.motif")} value={motif} onChange={(e) => setMotif(e.target.value)} autoFocus autoComplete="off" />
                <Bouton variante="danger" onClick={() => annulation.mutate()} disabled={annulation.isPending || motif.trim() === ""}>{t("vetat.confirmer_annulation")}</Bouton>
                <Bouton variante="secondaire" onClick={() => setMode("lecture")}>{t("commun.annuler")}</Bouton>
              </div>
            </div>
          ) : null}
          <MessageErreur erreur={annulation.error} surRelire={() => void detail.refetch()} />
          <div className="flex flex-wrap gap-2">
            {gere && valide && mode === "lecture" ? (
              <>
                <Bouton variante="secondaire" onClick={() => setMode("repartition")}>{t(v.non_reparti > 0 ? "vetat.completer" : "vetat.modifier")}</Bouton>
                <Bouton variante="secondaire" onClick={() => setMode("annulation")}>{t("vetat.annuler")}</Bouton>
              </>
            ) : null}
            <Bouton variante="secondaire" onClick={surFermer}>{t("commun.fermer")}</Bouton>
          </div>
        </>
      )}
      {repartition.isSuccess && avertissements.length === 0 ? <p role="status" className="text-sm text-muet">{t("vetat.repartition_enregistree")}</p> : null}
    </Section>
  );
}
