import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { aujourdHui } from "../../app/aujourdhui";
import { commandeAutorisee } from "../../app/navigation";
import { Bouton } from "../../composants/Bouton";
import { CaseACocher, Champ, Selection } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { type Cle, dateCivile, t } from "../../i18n";
import type { AbsencePersonnelAffichee, AbsencesEtConges, CongeAffiche, MembreAffiche, NatureAbsencePersonnel, StatutConge, TypeConge } from "../../types/gesco";
import { ecrireDemiJournees, joursDeConge } from "./jours";

const STATUTS: Record<StatutConge, Cle> = {
  demande: "conges.statut.demande",
  accorde: "conges.statut.accorde",
  refuse: "conges.statut.refuse",
  annule: "conges.statut.annule",
};

const NATURES: Record<NatureAbsencePersonnel, Cle> = {
  journee: "conges.nature.journee",
  demi_journee: "conges.nature.demi_journee",
  retard: "conges.nature.retard",
};

function useRafraichir() {
  const client = useQueryClient();
  return () => void client.invalidateQueries({ queryKey: ["absences-conges"] });
}

/** Absences, retards et congés du personnel (SPEC_PHASE_6 §5 et §6) : les absences se notent
 *  sans pointage ; un congé se demande, la direction l'accorde ou le refuse. */
export function Conges() {
  const session = useQuery({ queryKey: ["session"], queryFn: () => appeler("session_courante", null) });
  const membres = useQuery({ queryKey: ["personnel"], queryFn: () => appeler("personnel_lister", null) });
  const [mois, setMois] = useState(() => aujourdHui().slice(0, 7));
  const etat = useQuery({ queryKey: ["absences-conges", mois], queryFn: () => appeler("absences_conges_lire", { mois }) });
  const permissions = session.data?.permissions ?? [];
  const presents = (membres.data ?? []).filter((m) => m.membre.parti_le === null);
  const e = etat.data;
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("conges.titre")}</h1>
      <div className="w-56">
        <Champ id="conges-mois" type="month" libelle={t("heures.mois")} value={mois} onChange={(ev) => ev.target.value !== "" && setMois(ev.target.value)} />
      </div>
      <MessageErreur erreur={etat.error ?? membres.error} />
      {e === undefined ? null : (
        <>
          <Section titre={t("conges.section_conges")}>
            {commandeAutorisee("conge_demander", permissions) ? <DemandeConge membres={presents} types={e.types} samedi={e.samedi_compte} /> : null}
            <ListeConges etat={e} peutDecider={commandeAutorisee("conge_accorder", permissions)} />
          </Section>
          <Section titre={t("conges.section_absences")}>
            {commandeAutorisee("absence_personnel_noter", permissions) ? <NoterAbsence membres={presents} /> : null}
            <ListeAbsences absences={e.absences} peutModifier={commandeAutorisee("absence_personnel_annuler", permissions)} />
          </Section>
          {commandeAutorisee("type_conge_enregistrer", permissions) ? <TypesConge types={e.types} /> : null}
        </>
      )}
    </div>
  );
}

function DemandeConge({ membres, types, samedi }: { membres: readonly MembreAffiche[]; types: readonly TypeConge[]; samedi: boolean }) {
  const rafraichir = useRafraichir();
  const actifs = types.filter((x) => x.actif);
  const [membre, setMembre] = useState("");
  const [type, setType] = useState(actifs[0]?.id ?? "");
  const [debut, setDebut] = useState(aujourdHui);
  const [fin, setFin] = useState(aujourdHui);
  const [motif, setMotif] = useState("");
  const demande = useMutation({
    mutationFn: () => appeler("conge_demander", { membre_id: membre, type_id: type, debut, fin, motif }),
    onSuccess: () => {
      setMotif("");
      rafraichir();
    },
  });
  const jours = joursDeConge(debut, fin, samedi);
  return (
    <div className="flex flex-col gap-2 rounded border border-muet/30 p-3">
      <h3 className="text-sm font-semibold text-encre">{t("conges.demander")}</h3>
      <div className="flex flex-wrap items-end gap-3">
        <Selection
          id="conge-membre"
          libelle={t("conges.membre")}
          valeur={membre}
          surChangement={setMembre}
          options={[{ valeur: "", libelle: t("conges.choisir_membre") }, ...membres.map((m) => ({ valeur: m.membre.id, libelle: `${m.prenoms} ${m.nom} · ${m.membre.fonction}` }))]}
        />
        <Selection id="conge-type" libelle={t("conges.type")} valeur={type} surChangement={setType} options={actifs.map((x) => ({ valeur: x.id, libelle: x.libelle }))} />
        <Champ id="conge-debut" type="date" libelle={t("conges.du")} value={debut} onChange={(ev) => setDebut(ev.target.value)} />
        <Champ id="conge-fin" type="date" libelle={t("conges.au")} value={fin} onChange={(ev) => setFin(ev.target.value)} />
        <div className="min-w-60 flex-1">
          <Champ id="conge-motif" libelle={t("conges.motif")} value={motif} onChange={(ev) => setMotif(ev.target.value)} />
        </div>
      </div>
      <p className="text-sm text-muet">{t(samedi ? "conges.jours_comptes_samedi" : "conges.jours_comptes", { jours })}</p>
      <div>
        <Bouton onClick={() => demande.mutate()} disabled={membre === "" || type === "" || demande.isPending}>
          {t("conges.enregistrer_demande")}
        </Bouton>
      </div>
      {demande.isSuccess ? <p role="status" className="text-sm text-vert">{t("conges.demande_enregistree")}</p> : null}
      <MessageErreur erreur={demande.error} />
    </div>
  );
}

function ListeConges({ etat, peutDecider }: { etat: AbsencesEtConges; peutDecider: boolean }) {
  const rafraichir = useRafraichir();
  const [motifs, setMotifs] = useState<Record<string, string>>({});
  const decision = useMutation({
    mutationFn: ({ c, action }: { c: CongeAffiche; action: "conge_accorder" | "conge_refuser" | "conge_annuler" }) =>
      action === "conge_accorder" ? appeler(action, { id: c.conge.id }) : appeler(action, { id: c.conge.id, motif: motifs[c.conge.id] ?? "" }),
    onSuccess: rafraichir,
  });
  if (etat.conges.length === 0) return <p className="text-sm text-muet">{t("conges.aucun_conge")}</p>;
  const aujourd_hui = aujourdHui();
  return (
    <div className="flex flex-col gap-2">
      <table className="w-full text-left text-sm">
        <thead>
          <tr className="text-muet">
            <th className="px-2 py-1 font-normal">{t("conges.membre")}</th>
            <th className="px-2 py-1 font-normal">{t("conges.type")}</th>
            <th className="px-2 py-1 font-normal">{t("conges.periode")}</th>
            <th className="px-2 py-1 text-right font-normal">{t("conges.jours")}</th>
            <th className="px-2 py-1 font-normal">{t("conges.motif")}</th>
            <th className="px-2 py-1 font-normal">{t("conges.etat")}</th>
          </tr>
        </thead>
        <tbody>
          {etat.conges.map((c) => {
            const k = c.conge;
            const annulable = k.statut === "accorde" && k.debut > aujourd_hui;
            return (
              <tr key={k.id} className="border-b border-muet/30 align-top">
                <td className="px-2 py-1 text-encre">
                  <div>{c.membre}</div>
                  <div className="text-xs text-muet">{c.matricule}</div>
                </td>
                <td className="px-2 py-1 text-encre">{c.type_libelle}</td>
                <td className="px-2 py-1 text-encre">{t("conges.du_au", { debut: dateCivile(k.debut), fin: dateCivile(k.fin) })}</td>
                <td className="px-2 py-1 text-right text-encre">{k.jours}</td>
                <td className="px-2 py-1 text-encre">{k.motif}</td>
                <td className="px-2 py-1">
                  <div className={k.statut === "accorde" ? "text-vert" : k.statut === "demande" ? "text-ocre" : "text-muet"}>{t(STATUTS[k.statut])}</div>
                  {k.statut === "refuse" ? <div className="text-xs text-muet">{k.motif_refus}</div> : null}
                  {k.statut === "annule" ? <div className="text-xs text-muet">{k.motif_annulation}</div> : null}
                  {c.solde === null ? null : (
                    <div className={`text-xs ${c.depasse_solde ? "text-ocre" : "text-muet"}`}>
                      {t(c.depasse_solde ? "conges.solde_depasse" : "conges.solde", { jours: ecrireDemiJournees(c.solde) })}
                    </div>
                  )}
                  {peutDecider && (k.statut === "demande" || annulable) ? (
                    <div className="mt-1 flex flex-col gap-1">
                      <input
                        id={`conge-motif-${k.id}`}
                        aria-label={t(k.statut === "demande" ? "conges.motif_refus_de" : "conges.motif_annulation_de", { membre: c.membre })}
                        placeholder={t(k.statut === "demande" ? "conges.motif_refus" : "conges.motif_annulation")}
                        className="w-48 rounded border border-muet bg-surface px-2 py-1 text-xs text-primaire"
                        value={motifs[k.id] ?? ""}
                        onChange={(ev) => setMotifs((m) => ({ ...m, [k.id]: ev.target.value }))}
                      />
                      <div className="flex flex-wrap gap-1">
                        {k.statut === "demande" ? (
                          <>
                            <Bouton onClick={() => decision.mutate({ c, action: "conge_accorder" })} disabled={decision.isPending}>{t("conges.accorder")}</Bouton>
                            <Bouton variante="secondaire" onClick={() => decision.mutate({ c, action: "conge_refuser" })} disabled={decision.isPending}>{t("conges.refuser")}</Bouton>
                          </>
                        ) : (
                          <Bouton variante="secondaire" onClick={() => decision.mutate({ c, action: "conge_annuler" })} disabled={decision.isPending}>{t("conges.annuler")}</Bouton>
                        )}
                      </div>
                    </div>
                  ) : null}
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
      <MessageErreur erreur={decision.error} />
    </div>
  );
}

function NoterAbsence({ membres }: { membres: readonly MembreAffiche[] }) {
  const rafraichir = useRafraichir();
  const [membre, setMembre] = useState("");
  const [date, setDate] = useState(aujourdHui);
  const [nature, setNature] = useState<NatureAbsencePersonnel>("journee");
  const [minutes, setMinutes] = useState("");
  const [motif, setMotif] = useState("");
  const [justifiee, setJustifiee] = useState(false);
  const note = useMutation({
    mutationFn: () =>
      appeler("absence_personnel_noter", {
        membre_id: membre,
        date,
        nature,
        minutes: nature === "retard" && minutes.trim() !== "" ? Number(minutes) : null,
        motif,
        justifiee,
      }),
    onSuccess: () => {
      setMinutes("");
      setMotif("");
      setJustifiee(false);
      rafraichir();
    },
  });
  return (
    <div className="flex flex-col gap-2 rounded border border-muet/30 p-3">
      <h3 className="text-sm font-semibold text-encre">{t("conges.noter_absence")}</h3>
      <div className="flex flex-wrap items-end gap-3">
        <Selection
          id="absence-membre"
          libelle={t("conges.membre")}
          valeur={membre}
          surChangement={setMembre}
          options={[{ valeur: "", libelle: t("conges.choisir_membre") }, ...membres.map((m) => ({ valeur: m.membre.id, libelle: `${m.prenoms} ${m.nom} · ${m.membre.fonction}` }))]}
        />
        <Champ id="absence-date" type="date" libelle={t("conges.date")} value={date} max={aujourdHui()} onChange={(ev) => setDate(ev.target.value)} />
        <Selection
          id="absence-nature"
          libelle={t("conges.nature")}
          valeur={nature}
          surChangement={(v) => setNature(v as NatureAbsencePersonnel)}
          options={(Object.keys(NATURES) as NatureAbsencePersonnel[]).map((n) => ({ valeur: n, libelle: t(NATURES[n]) }))}
        />
        {nature === "retard" ? (
          <div className="w-28">
            <Champ id="absence-minutes" type="number" min={1} max={240} libelle={t("conges.minutes")} value={minutes} onChange={(ev) => setMinutes(ev.target.value)} />
          </div>
        ) : null}
        <div className="min-w-60 flex-1">
          <Champ id="absence-motif" libelle={t("conges.motif")} value={motif} onChange={(ev) => setMotif(ev.target.value)} />
        </div>
      </div>
      <CaseACocher id="absence-justifiee" libelle={t("conges.deja_justifiee")} coche={justifiee} surChangement={setJustifiee} />
      <div>
        <Bouton onClick={() => note.mutate()} disabled={membre === "" || note.isPending}>
          {t("conges.noter")}
        </Bouton>
      </div>
      <p className="text-sm text-muet">{t("conges.aide_absence")}</p>
      {note.isSuccess ? <p role="status" className="text-sm text-vert">{t("conges.absence_notee")}</p> : null}
      <MessageErreur erreur={note.error} />
    </div>
  );
}

function natureLisible(a: AbsencePersonnelAffichee): string {
  const x = a.absence;
  return x.nature === "retard" && x.minutes !== null ? t("conges.retard_de", { minutes: x.minutes }) : t(NATURES[x.nature]);
}

function ListeAbsences({ absences, peutModifier }: { absences: readonly AbsencePersonnelAffichee[]; peutModifier: boolean }) {
  const rafraichir = useRafraichir();
  const [motifs, setMotifs] = useState<Record<string, string>>({});
  const action = useMutation({
    mutationFn: ({ id, commande }: { id: string; commande: "absence_personnel_justifier" | "absence_personnel_annuler" }) => appeler(commande, { id, motif: motifs[id] ?? "" }),
    onSuccess: rafraichir,
  });
  if (absences.length === 0) return <p className="text-sm text-muet">{t("conges.aucune_absence")}</p>;
  return (
    <div className="flex flex-col gap-2">
      <table className="w-full text-left text-sm">
        <thead>
          <tr className="text-muet">
            <th className="px-2 py-1 font-normal">{t("conges.date")}</th>
            <th className="px-2 py-1 font-normal">{t("conges.membre")}</th>
            <th className="px-2 py-1 font-normal">{t("conges.nature")}</th>
            <th className="px-2 py-1 font-normal">{t("conges.motif")}</th>
            <th className="px-2 py-1 font-normal">{t("conges.etat")}</th>
          </tr>
        </thead>
        <tbody>
          {absences.map((a) => {
            const x = a.absence;
            // Une absence annulée reste visible, barrée (R-14).
            const barre = x.annulee ? "line-through" : "";
            return (
              <tr key={x.id} className={`border-b border-muet/30 align-top ${x.annulee ? "text-muet" : "text-encre"}`}>
                <td className={`px-2 py-1 ${barre}`}>{dateCivile(x.date)}</td>
                <td className={`px-2 py-1 ${barre}`}>
                  <div>{a.membre}</div>
                  <div className="text-xs text-muet">{a.matricule}</div>
                </td>
                <td className={`px-2 py-1 ${barre}`}>{natureLisible(a)}</td>
                <td className={`px-2 py-1 ${barre}`}>{x.motif}</td>
                <td className="px-2 py-1">
                  {x.annulee ? (
                    <span className="text-muet">{t("conges.absence_annulee", { motif: x.motif_annulation })}</span>
                  ) : (
                    <span className={x.justifiee ? "text-vert" : "text-ocre"}>{t(x.justifiee ? "conges.justifiee" : "conges.non_justifiee")}</span>
                  )}
                  {peutModifier && !x.annulee ? (
                    <div className="mt-1 flex flex-col gap-1">
                      <input
                        id={`absence-motif-${x.id}`}
                        aria-label={t("conges.motif_absence_de", { membre: a.membre })}
                        placeholder={t("conges.motif")}
                        className="w-48 rounded border border-muet bg-surface px-2 py-1 text-xs text-primaire"
                        value={motifs[x.id] ?? ""}
                        onChange={(ev) => setMotifs((m) => ({ ...m, [x.id]: ev.target.value }))}
                      />
                      <div className="flex flex-wrap gap-1">
                        {x.justifiee ? null : (
                          <Bouton variante="secondaire" onClick={() => action.mutate({ id: x.id, commande: "absence_personnel_justifier" })} disabled={action.isPending}>
                            {t("conges.justifier")}
                          </Bouton>
                        )}
                        <Bouton variante="secondaire" onClick={() => action.mutate({ id: x.id, commande: "absence_personnel_annuler" })} disabled={action.isPending}>
                          {t("conges.annuler_absence")}
                        </Bouton>
                      </div>
                    </div>
                  ) : null}
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
      <MessageErreur erreur={action.error} />
    </div>
  );
}

function TypesConge({ types }: { types: readonly TypeConge[] }) {
  const rafraichir = useRafraichir();
  const [libelles, setLibelles] = useState<Record<string, string>>({});
  const [nouveau, setNouveau] = useState("");
  const enregistrer = useMutation({
    mutationFn: (d: { id: string | null; libelle: string; actif: boolean; deduit_du_solde: boolean }) => appeler("type_conge_enregistrer", d),
    onSuccess: () => {
      setNouveau("");
      rafraichir();
    },
  });
  const libelle = (x: TypeConge) => libelles[x.id] ?? x.libelle;
  return (
    <details className="rounded border border-muet/30 p-3">
      <summary className="cursor-pointer text-sm font-semibold text-primaire">{t("conges.types")}</summary>
      <div className="mt-3 flex flex-col gap-3">
        <p className="text-sm text-muet">{t("conges.types_aide")}</p>
        {types.map((x) => (
          <div key={x.id} className="flex flex-wrap items-end gap-2">
            <Champ id={`type-conge-${x.id}`} libelle={t("conges.type")} value={libelle(x)} onChange={(ev) => setLibelles((l) => ({ ...l, [x.id]: ev.target.value }))} />
            <CaseACocher
              id={`type-conge-deduit-${x.id}`}
              libelle={t("conges.deduit_du_solde")}
              coche={x.deduit_du_solde}
              surChangement={(c) => enregistrer.mutate({ id: x.id, libelle: libelle(x), actif: x.actif, deduit_du_solde: c })}
            />
            <Bouton variante="secondaire" onClick={() => enregistrer.mutate({ id: x.id, libelle: libelle(x), actif: x.actif, deduit_du_solde: x.deduit_du_solde })}>{t("commun.enregistrer")}</Bouton>
            <Bouton variante="secondaire" onClick={() => enregistrer.mutate({ id: x.id, libelle: x.libelle, actif: !x.actif, deduit_du_solde: x.deduit_du_solde })}>
              {t(x.actif ? "commun.retirer" : "personnel.remettre")}
            </Bouton>
            {x.actif ? null : <span className="text-sm text-muet">{t("conges.type_retire")}</span>}
          </div>
        ))}
        <div className="flex flex-wrap items-end gap-2">
          <Champ id="type-conge-nouveau" libelle={t("conges.nouveau_type")} value={nouveau} onChange={(ev) => setNouveau(ev.target.value)} />
          <Bouton variante="secondaire" onClick={() => enregistrer.mutate({ id: null, libelle: nouveau, actif: true, deduit_du_solde: false })} disabled={nouveau.trim() === ""}>
            {t("commun.ajouter")}
          </Bouton>
        </div>
        <MessageErreur erreur={enregistrer.error} />
      </div>
    </details>
  );
}
