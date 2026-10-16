import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { type KeyboardEvent, useRef, useState } from "react";

import { appeler, ErreurApi } from "../../api";
import { aujourdHui } from "../../app/aujourdhui";
import { commandeAutorisee } from "../../app/navigation";
import { ApercuPdf } from "../../composants/ApercuPdf";
import { Bouton } from "../../composants/Bouton";
import { Champ } from "../../composants/Champ";
import { MessageErreur } from "../../composants/Messages";
import { francs, t, texteDeCle } from "../../i18n";
import type { EtatHeures, LigneHeures } from "../../types/gesco";

/** Heures écrites comme on les saisit : « 24 », « 24,5 », « 24,25 ». */
export function ecrireHeures(minutes: number): string {
  const h = Math.floor(minutes / 60);
  const m = minutes % 60;
  if (m === 0) return String(h);
  if (m === 15) return `${h},25`;
  if (m === 30) return `${h},5`;
  if (m === 45) return `${h},75`;
  return `${h}h${String(m).padStart(2, "0")}`;
}

interface Textes {
  prevues: string;
  faites: string;
  observation: string;
}

function textesDe(l: LigneHeures): Textes {
  return {
    prevues: ecrireHeures(l.minutes_prevues),
    faites: l.minutes_faites === null ? "" : ecrireHeures(l.minutes_faites),
    observation: l.observation,
  };
}

/** Heures des vacataires (SPEC_PHASE_6 §4) : chaque mois, les heures faites de chaque vacation,
 *  saisies au clavier, puis visées par la direction ; la paie reprend les heures visées. Sans
 *  `contracts:manage`, la grille se lit seulement ; sans `contracts:read`, le noyau n'envoie ni
 *  taux ni montant. */
export function Heures() {
  const session = useQuery({ queryKey: ["session"], queryFn: () => appeler("session_courante", null) });
  const permissions = session.data?.permissions ?? [];
  const [mois, setMois] = useState(() => aujourdHui().slice(0, 7));
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("heures.titre")}</h1>
      <div className="w-56">
        <Champ id="heures-mois" type="month" libelle={t("heures.mois")} value={mois} onChange={(e) => e.target.value !== "" && setMois(e.target.value)} />
      </div>
      <p className="text-sm text-muet">{t("heures.aide")}</p>
      <GrilleHeures key={mois} mois={mois} permissions={permissions} />
    </div>
  );
}

function GrilleHeures({ mois, permissions }: { mois: string; permissions: readonly string[] }) {
  const client = useQueryClient();
  const etat = useQuery({ queryKey: ["heures", mois], queryFn: () => appeler("heures_lire", { mois }) });
  const [textes, setTextes] = useState<Record<string, Textes>>({});
  const [erreurs, setErreurs] = useState<Record<string, string>>({});
  const [document, setDocument] = useState<string | null>(null);
  const [motifs, setMotifs] = useState<Record<string, string>>({});
  const cases = useRef<Record<string, HTMLInputElement | null>>({});
  // Saisies parties au serveur et pas encore revenues : le visa les attend.
  const enCours = useRef(new Set<Promise<void>>());
  const mettreAJour = (e: EtatHeures) => client.setQueryData(["heures", mois], e);
  const viser = useMutation({
    mutationFn: async () => {
      await Promise.all([...enCours.current]);
      return appeler("heures_viser", { mois, engagement_id: null });
    },
    onSuccess: mettreAJour,
  });
  const deverrouiller = useMutation({
    mutationFn: (l: LigneHeures) => appeler("heures_deverrouiller", { id: l.id ?? "", motif: motifs[l.engagement_id] ?? "" }),
    onSuccess: (e, l) => {
      setTextes((x) => ({ ...x, [l.engagement_id]: textesDe(e.lignes.find((y) => y.engagement_id === l.engagement_id) ?? l) }));
      mettreAJour(e);
    },
  });
  const imprimer = useMutation({ mutationFn: () => appeler("etat_heures_produire", { mois }), onSuccess: (d) => setDocument(d.pdf) });
  const e = etat.data;
  if (e === undefined) return <MessageErreur erreur={etat.error} />;
  const saisit = commandeAutorisee("heures_saisir", permissions);
  const imprime = commandeAutorisee("etat_heures_produire", permissions);
  const texteDe = (l: LigneHeures) => textes[l.engagement_id] ?? textesDe(l);
  const envoyer = async (l: LigneHeures) => {
    const x = texteDe(l);
    const avant = textesDe(l);
    if (x.prevues === avant.prevues && x.faites === avant.faites && x.observation === avant.observation) return;
    try {
      const nouvel = await appeler("heures_saisir", { engagement_id: l.engagement_id, mois, prevues: x.prevues, faites: x.faites, observation: x.observation });
      const ligne = nouvel.lignes.find((y) => y.engagement_id === l.engagement_id);
      if (ligne !== undefined) setTextes((t2) => ({ ...t2, [l.engagement_id]: textesDe(ligne) }));
      setErreurs((t2) => ({ ...t2, [l.engagement_id]: "" }));
      mettreAJour(nouvel);
    } catch (erreur) {
      setErreurs((t2) => ({ ...t2, [l.engagement_id]: erreur instanceof ErreurApi ? texteDeCle(erreur.cle) : t("erreur.transport") }));
    }
  };
  const enregistrer = (l: LigneHeures) => {
    const envoi = envoyer(l);
    enCours.current.add(envoi);
    void envoi.finally(() => enCours.current.delete(envoi));
  };
  const surTouche = (i: number, l: LigneHeures) => (ev: KeyboardEvent<HTMLInputElement>) => {
    if (ev.key !== "Enter") return;
    ev.preventDefault();
    enregistrer(l);
    cases.current[e.lignes[i + 1]?.engagement_id ?? ""]?.focus();
  };
  const changer = (l: LigneHeures, champ: keyof Textes, valeur: string) => setTextes((x) => ({ ...x, [l.engagement_id]: { ...texteDe(l), [champ]: valeur } }));
  if (e.lignes.length === 0) return <p className="text-sm text-muet">{t("heures.aucune_vacation")}</p>;
  return (
    <div className="flex flex-col gap-3">
      <table className="w-full text-left text-sm">
        <thead>
          <tr className="text-muet">
            <th className="px-2 py-1 font-normal">{t("heures.vacataire")}</th>
            {e.montants ? <th className="px-2 py-1 font-normal">{t("engagement.taux")}</th> : null}
            <th className="px-2 py-1 font-normal">{t("heures.prevues")}</th>
            <th className="px-2 py-1 font-normal">{t("heures.faites")}</th>
            <th className="px-2 py-1 font-normal">{t("heures.observation")}</th>
            {e.montants ? <th className="px-2 py-1 text-right font-normal">{t("heures.montant")}</th> : null}
            <th className="px-2 py-1 font-normal">{t("heures.etat")}</th>
          </tr>
        </thead>
        <tbody>
          {e.lignes.map((l, i) => {
            const x = texteDe(l);
            const visee = l.statut === "visee";
            const fige = visee || !saisit;
            const erreur = erreurs[l.engagement_id] ?? "";
            return (
              <tr key={l.engagement_id} className="border-b border-muet/30 align-top">
                <td className="px-2 py-1 text-encre">
                  <div>{l.membre}</div>
                  <div className="text-xs text-muet">{`${l.matricule} · ${l.fonction}`}</div>
                </td>
                {e.montants ? <td className="px-2 py-1 text-encre">{l.taux === null ? "" : francs(l.taux)}</td> : null}
                <td className="px-2 py-1">
                  <input
                    id={`heures-prevues-${i + 1}`}
                    aria-label={t("heures.prevues_de", { membre: l.membre })}
                    disabled={fige}
                    className="w-20 rounded border border-muet bg-surface px-2 py-1 text-primaire disabled:bg-fond disabled:text-encre disabled:opacity-70"
                    value={x.prevues}
                    onChange={(ev) => changer(l, "prevues", ev.target.value)}
                    onBlur={() => enregistrer(l)}
                  />
                </td>
                <td className="px-2 py-1">
                  <input
                    id={`heures-faites-${i + 1}`}
                    ref={(el) => {
                      cases.current[l.engagement_id] = el;
                    }}
                    aria-label={t("heures.faites_de", { membre: l.membre })}
                    disabled={fige}
                    className={`w-20 rounded border bg-surface px-2 py-1 text-primaire disabled:bg-fond disabled:text-encre disabled:opacity-70 ${erreur === "" ? "border-muet" : "border-rouge"}`}
                    value={x.faites}
                    onChange={(ev) => changer(l, "faites", ev.target.value)}
                    onKeyDown={surTouche(i, l)}
                    onBlur={() => enregistrer(l)}
                  />
                  {erreur === "" ? null : <div className="text-xs text-rouge">{erreur}</div>}
                </td>
                <td className="px-2 py-1">
                  <input
                    id={`heures-observation-${i + 1}`}
                    aria-label={t("heures.observation_de", { membre: l.membre })}
                    disabled={fige}
                    className="w-full min-w-40 rounded border border-muet bg-surface px-2 py-1 text-primaire disabled:bg-fond disabled:text-encre disabled:opacity-70"
                    value={x.observation}
                    onChange={(ev) => changer(l, "observation", ev.target.value)}
                    onBlur={() => enregistrer(l)}
                  />
                </td>
                {e.montants ? <td className="px-2 py-1 text-right text-encre">{l.montant === null ? "" : francs(l.montant)}</td> : null}
                <td className="px-2 py-1">
                  {visee && !saisit ? <span className="text-vert">{t("heures.visee")}</span> : null}
                  {visee && saisit ? (
                    <div className="flex flex-col gap-1">
                      <span className="text-vert">{t("heures.visee")}</span>
                      <input
                        id={`heures-motif-${i + 1}`}
                        aria-label={t("heures.motif_de", { membre: l.membre })}
                        placeholder={t("heures.motif")}
                        className="w-40 rounded border border-muet bg-surface px-2 py-1 text-xs text-primaire"
                        value={motifs[l.engagement_id] ?? ""}
                        onChange={(ev) => setMotifs((m) => ({ ...m, [l.engagement_id]: ev.target.value }))}
                      />
                      <Bouton variante="secondaire" onClick={() => deverrouiller.mutate(l)} disabled={deverrouiller.isPending}>{t("heures.deverrouiller")}</Bouton>
                    </div>
                  ) : null}
                  {visee ? null : <span className="text-muet">{t(l.id === null ? "heures.a_saisir" : "heures.en_saisie")}</span>}
                  {l.motif_deverrouillage === "" || visee ? null : <div className="text-xs text-ocre">{t("heures.deverrouillee", { motif: l.motif_deverrouillage })}</div>}
                </td>
              </tr>
            );
          })}
        </tbody>
        <tfoot>
          <tr className="font-semibold text-encre">
            <td className="px-2 py-1" colSpan={e.montants ? 3 : 2}>{t("heures.total")}</td>
            <td className="px-2 py-1">{ecrireHeures(e.minutes_faites)}</td>
            <td />
            {e.montants ? <td className="px-2 py-1 text-right">{e.montant === null ? "" : francs(e.montant)}</td> : null}
            <td />
          </tr>
        </tfoot>
      </table>
      {saisit || imprime ? (
        <div className="flex flex-wrap gap-2">
          {saisit ? <Bouton onClick={() => viser.mutate()} disabled={viser.isPending}>{t("heures.viser")}</Bouton> : null}
          {imprime ? <Bouton variante="secondaire" onClick={() => imprimer.mutate()} disabled={imprimer.isPending}>{t("heures.imprimer")}</Bouton> : null}
        </div>
      ) : null}
      <p className="text-sm text-muet">{t(saisit ? "heures.aide_visa" : "heures.lecture_seule")}</p>
      {viser.isSuccess ? <p role="status" className="text-sm text-vert">{t("heures.mois_vise")}</p> : null}
      <MessageErreur erreur={viser.error ?? deverrouiller.error ?? imprimer.error} />
      {document === null ? null : <ApercuPdf base64={document} />}
    </div>
  );
}
