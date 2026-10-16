import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { aujourdHui } from "../../app/aujourdhui";
import { commandeAutorisee } from "../../app/navigation";
import { ApercuPdf } from "../../composants/ApercuPdf";
import { Bouton } from "../../composants/Bouton";
import { Champ, Formulaire, Selection } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { Tableau } from "../../composants/Tableau";
import { type Cle, dateCivile, dateHeure, francs, moisCivil, t } from "../../i18n";
import type { BulletinPaieAffiche, LigneCalculee, PaieDuMois as Paie, SorteLigne } from "../../types/gesco";
import { duMois, lireMontant } from "./calcul";

/** Montant saisi, négatif s'il commence par un signe moins : une régularisation qui reprend un
 *  trop-perçu. */
function lireMontantSigne(texte: string): number | null {
  const lu = lireMontant(texte);
  if (lu === null) return null;
  return /^\s*[-−]/.test(texte) ? -lu : lu;
}

const SORTES: Record<SorteLigne, Cle> = {
  base: "paie.sorte.base",
  heures: "paie.sorte.heures",
  gain: "paie.sorte.gain",
  retenue: "paie.sorte.retenue",
  absence: "paie.sorte.absence",
  avance: "paie.sorte.avance",
  patronale: "paie.sorte.patronale",
};

/** Mois proposé : la paie se fait en fin de mois et se paie jusqu'au 5 du suivant (T-51) ;
 *  avant le 20, c'est le mois précédent qui se prépare encore. */
function moisPropose(): string {
  const jour = aujourdHui();
  const [a, m, j] = jour.split("-").map(Number);
  if ((j ?? 1) >= 20) return jour.slice(0, 7);
  const precedent = new Date(Date.UTC(a ?? 2026, (m ?? 1) - 2, 1));
  return precedent.toISOString().slice(0, 7);
}

/** Paie du mois (SPEC_PHASE_7 §5) : calculer, revoir (primes du mois, absences à retenir ou
 *  non), valider. */
export function PaieDuMois({ permissions }: { permissions: readonly string[] }) {
  const client = useQueryClient();
  const [mois, setMois] = useState(moisPropose);
  const [choisi, setChoisi] = useState<string | null>(null);
  const cle = ["paie-mois", mois];
  const paie = useQuery({ queryKey: cle, queryFn: () => appeler("paie_mois_lire", { mois }), enabled: /^\d{4}-\d{2}$/.test(mois) });
  const apres = (p: Paie) => client.setQueryData(cle, p);
  const calculer = useMutation({ mutationFn: () => appeler("paie_calculer", { mois }), onSuccess: apres });
  const [confirmation, setConfirmation] = useState(false);
  const valider = useMutation({
    mutationFn: () => appeler("paie_valider", { mois }),
    onSuccess: (p) => {
      setConfirmation(false);
      apres(p);
    },
  });
  const p = paie.data;
  const enRevue = p?.paie?.statut === "calculee";
  const bulletin = p?.bulletins.find((b) => b.bulletin.membre_id === choisi);
  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-end gap-3">
        <Champ id="paie-mois" type="month" libelle={t("paie.mois.choisir")} value={mois} onChange={(e) => setMois(e.target.value)} />
        {p === undefined ? null : <p className="text-sm text-encre">{etatPaie(p)}</p>}
      </div>
      <MessageErreur erreur={paie.error ?? calculer.error ?? valider.error} />
      {p === undefined ? null : (
        <>
          {p.precedente_a_valider === null ? null : (
            <p className="text-sm text-ocre">{t("paie.mois.precedente", { mois: duMois(p.precedente_a_valider) })}</p>
          )}
          {p.heures_en_saisie > 0 ? <p className="text-sm text-ocre">{t("paie.mois.heures_en_saisie", { nombre: p.heures_en_saisie })}</p> : null}
          <div className="flex flex-wrap gap-2">
            {p.paie === null || enRevue ? (
              <Bouton onClick={() => calculer.mutate()} disabled={calculer.isPending}>
                {t(p.paie === null ? "paie.mois.calculer" : "paie.mois.recalculer", { mois: duMois(mois) })}
              </Bouton>
            ) : null}
            {enRevue && commandeAutorisee("paie_valider", permissions) ? (
              confirmation ? (
                <>
                  <Bouton variante="danger" onClick={() => valider.mutate()} disabled={valider.isPending}>
                    {t("paie.mois.confirmer_validation", { nombre: p.bulletins.length, net: francs(p.net) })}
                  </Bouton>
                  <Bouton variante="secondaire" onClick={() => setConfirmation(false)}>
                    {t("commun.annuler")}
                  </Bouton>
                </>
              ) : (
                <Bouton variante="secondaire" onClick={() => setConfirmation(true)}>
                  {t("paie.mois.valider", { mois: duMois(mois) })}
                </Bouton>
              )
            ) : null}
          </div>
          {p.paie === null ? null : (
            <>
              <Tableau
                nomExport={`paie-${mois}`}
                titre={t("paie.mois.bulletins")}
                lignes={p.bulletins}
                cle={(b) => b.bulletin.id}
                surSelection={(b) => setChoisi(b.bulletin.membre_id)}
                colonnes={[
                  { titre: t("paie.mois.membre"), valeur: (b) => b.membre },
                  { titre: t("paie.mois.fonction"), valeur: (b) => b.fonction },
                  { titre: t("paie.mois.brut"), valeur: (b) => francs(b.bulletin.resultat.brut) },
                  { titre: t("paie.mois.retenues"), valeur: (b) => francs(b.bulletin.resultat.retenues) },
                  { titre: t("paie.mois.avances"), valeur: (b) => francs(b.bulletin.resultat.avances) },
                  { titre: t("paie.mois.net"), valeur: (b) => francs(b.bulletin.resultat.net) },
                  { titre: t("paie.mois.numero"), valeur: (b) => b.bulletin.numero ?? "" },
                ]}
              />
              <p className="text-sm text-encre">
                {t("paie.mois.totaux", { brut: francs(p.brut), net: francs(p.net), patronales: francs(p.patronales) })}
              </p>
              {commandeAutorisee("bulletins_paie_produire", permissions) ? <Documents mois={mois} /> : null}
              {bulletin === undefined ? null : <Detail key={bulletin.bulletin.id} b={bulletin} surFermer={() => setChoisi(null)} />}
              <Absences p={p} enRevue={enRevue} surMiseAJour={apres} />
              <Saisies p={p} enRevue={enRevue} surMiseAJour={apres} />
            </>
          )}
        </>
      )}
    </div>
  );
}

/** Les documents du mois : bulletins de paie, état à signer pour acquit, état des retenues. */
function Documents({ mois }: { mois: string }) {
  const [pdf, setPdf] = useState<string | null>(null);
  const produire = useMutation({
    mutationFn: (commande: "bulletins_paie_produire" | "etat_paie_produire" | "etat_retenues_produire") => appeler(commande, { mois }),
    onSuccess: (d) => setPdf(d.pdf),
  });
  return (
    <div className="flex flex-col gap-2">
      <div className="flex flex-wrap gap-2">
        <Bouton variante="secondaire" onClick={() => produire.mutate("bulletins_paie_produire")} disabled={produire.isPending}>
          {t("paie.document.bulletins")}
        </Bouton>
        <Bouton variante="secondaire" onClick={() => produire.mutate("etat_paie_produire")} disabled={produire.isPending}>
          {t("paie.document.etat_paie")}
        </Bouton>
        <Bouton variante="secondaire" onClick={() => produire.mutate("etat_retenues_produire")} disabled={produire.isPending}>
          {t("paie.document.etat_retenues")}
        </Bouton>
      </div>
      <MessageErreur erreur={produire.error} />
      {pdf === null ? null : <ApercuPdf base64={pdf} />}
    </div>
  );
}

function etatPaie(p: Paie): string {
  if (p.paie === null) return t("paie.mois.non_calculee");
  if (p.paie.statut === "validee" && p.paie.validee_le !== null) return t("paie.mois.validee_le", { le: dateHeure(p.paie.validee_le) });
  return t("paie.mois.calculee_le", { le: dateHeure(p.paie.calculee_le) });
}

function detailLigne(l: LigneCalculee): string {
  const parts: string[] = [];
  if (l.jours !== null) parts.push(t("paie.ligne.jours", { payes: l.jours[0], sur: l.jours[1] }));
  if (l.minutes !== null) parts.push(t("paie.ligne.heures", { heures: (l.minutes / 60).toLocaleString("fr-FR") }));
  if (l.taux !== null) parts.push(`${(l.taux / 100).toLocaleString("fr-FR")} %`);
  if (l.base !== null) parts.push(t("paie.ligne.base", { base: francs(l.base) }));
  if (l.mois_rappel !== null) parts.push(t("paie.ligne.rappel", { mois: moisCivil(l.mois_rappel) }));
  if (l.date !== null) parts.push(dateCivile(l.date));
  if (l.reste !== null) parts.push(t("paie.ligne.reste", { reste: francs(l.reste) }));
  return parts.join(" · ");
}

function Detail({ b, surFermer }: { b: BulletinPaieAffiche; surFermer: () => void }) {
  const r = b.bulletin.resultat;
  const [pdf, setPdf] = useState<string | null>(null);
  const imprimer = useMutation({ mutationFn: () => appeler("bulletin_paie_produire", { id: b.bulletin.id }), onSuccess: (d) => setPdf(d.pdf) });
  return (
    <Section titre={t("paie.mois.bulletin_de", { membre: b.membre })}>
      <table className="w-full text-sm">
        <thead>
          <tr className="border-b border-muet text-left text-muet">
            <th className="py-1 pr-3 font-normal">{t("paie.ligne.libelle")}</th>
            <th className="py-1 pr-3 font-normal">{t("paie.ligne.detail")}</th>
            <th className="py-1 pr-3 text-right font-normal">{t("paie.ligne.montant")}</th>
          </tr>
        </thead>
        <tbody>
          {r.lignes.map((l, i) => (
            <tr key={`${l.libelle}-${String(i)}`} className="border-b border-fond">
              <td className="py-1 pr-3 text-encre">
                {l.libelle} {l.sorte === "base" || l.sorte === "heures" ? null : <span className="text-muet">{t(SORTES[l.sorte])}</span>}
              </td>
              <td className="py-1 pr-3 text-muet">{detailLigne(l)}</td>
              <td className="py-1 pr-3 text-right text-encre">{francs(l.montant)}</td>
            </tr>
          ))}
        </tbody>
      </table>
      <p className="text-sm font-medium text-encre">
        {t("paie.mois.resume", { brut: francs(r.brut), retenues: francs(r.retenues), avances: francs(r.avances), net: francs(r.net) })}
      </p>
      <p className="text-sm text-muet">{t("paie.mois.cumuls", { brut: francs(r.cumuls.brut), net: francs(r.cumuls.net) })}</p>
      {r.avertissements.map((a) => (
        <p key={a} className="text-sm text-ocre">
          {t(a as Cle)}
        </p>
      ))}
      <div className="flex gap-2">
        <Bouton variante="secondaire" onClick={() => imprimer.mutate()} disabled={imprimer.isPending}>
          {t("paie.document.bulletin")}
        </Bouton>
        <Bouton variante="secondaire" onClick={surFermer}>
          {t("commun.fermer")}
        </Bouton>
      </div>
      <MessageErreur erreur={imprimer.error} />
      {pdf === null ? null : <ApercuPdf base64={pdf} />}
    </Section>
  );
}

function Absences({ p, enRevue, surMiseAJour }: { p: Paie; enRevue: boolean; surMiseAJour: (p: Paie) => void }) {
  const confirmer = useMutation({
    mutationFn: (d: { absence_id: string; retenue: boolean }) => appeler("paie_absence_confirmer", { mois: p.mois, ...d }),
    onSuccess: surMiseAJour,
  });
  if (p.absences.length === 0) return null;
  return (
    <Section titre={t("paie.mois.absences")}>
      <p className="text-sm text-muet">{t("paie.mois.absences_aide")}</p>
      <MessageErreur erreur={confirmer.error} />
      <ul className="flex flex-col gap-2">
        {p.absences.map((a) => (
          <li key={a.absence.id} className="flex flex-wrap items-center gap-3 text-sm text-encre">
            <span>
              {a.membre} · {dateCivile(a.absence.date)} · {t(a.absence.nature === "demi_journee" ? "paie.mois.demi_journee" : "paie.mois.journee")}
            </span>
            <span className={a.retenue === true ? "text-ocre" : "text-muet"}>
              {t(a.retenue === null ? "paie.mois.a_decider" : a.retenue ? "paie.mois.retenue" : "paie.mois.laissee")}
            </span>
            {enRevue ? (
              <>
                <Bouton variante="secondaire" disabled={a.retenue === true || confirmer.isPending} onClick={() => confirmer.mutate({ absence_id: a.absence.id, retenue: true })}>
                  {t("paie.mois.retenir")}
                </Bouton>
                <Bouton variante="secondaire" disabled={a.retenue === false || confirmer.isPending} onClick={() => confirmer.mutate({ absence_id: a.absence.id, retenue: false })}>
                  {t("paie.mois.laisser")}
                </Bouton>
              </>
            ) : null}
          </li>
        ))}
      </ul>
    </Section>
  );
}

function Saisies({ p, enRevue, surMiseAJour }: { p: Paie; enRevue: boolean; surMiseAJour: (p: Paie) => void }) {
  const [membreId, setMembreId] = useState("");
  const [elementId, setElementId] = useState("");
  const [montant, setMontant] = useState("");
  const [libelle, setLibelle] = useState("");
  const lu = lireMontantSigne(montant);
  const saisir = useMutation({
    mutationFn: () => appeler("paie_saisir", { mois: p.mois, membre_id: membreId, element_id: elementId, montant: lu ?? 0, libelle }),
    onSuccess: (r) => {
      setMontant("");
      setLibelle("");
      surMiseAJour(r);
    },
  });
  const retirer = useMutation({ mutationFn: (id: string) => appeler("paie_saisie_retirer", { id }), onSuccess: surMiseAJour });
  return (
    <Section titre={t("paie.mois.saisies")}>
      <p className="text-sm text-muet">{t("paie.mois.saisies_aide")}</p>
      <ul className="flex flex-col gap-1">
        {p.saisies.map((s) => (
          <li key={s.saisie.id} className="flex flex-wrap items-center gap-3 text-sm text-encre">
            <span>
              {s.membre} · {s.element} · {s.saisie.libelle} · {francs(s.saisie.montant)}
            </span>
            {enRevue ? (
              <Bouton variante="secondaire" onClick={() => retirer.mutate(s.saisie.id)} disabled={retirer.isPending}>
                {t("paie.mois.retirer")}
              </Bouton>
            ) : null}
          </li>
        ))}
      </ul>
      {enRevue ? (
        <Formulaire surEnvoi={() => saisir.mutate()}>
          <div className="flex flex-wrap items-end gap-3">
            <Selection
              id="saisie-membre"
              libelle={t("paie.mois.membre")}
              valeur={membreId}
              surChangement={setMembreId}
              options={[{ valeur: "", libelle: "" }, ...p.membres.map(([id, nom]) => ({ valeur: id, libelle: nom }))]}
            />
            <Selection
              id="saisie-element"
              libelle={t("paie.mois.element")}
              valeur={elementId}
              surChangement={setElementId}
              options={[{ valeur: "", libelle: "" }, ...p.elements_saisis.map((e) => ({ valeur: e.id, libelle: e.libelle }))]}
            />
            <Champ id="saisie-montant" libelle={t("paie.mois.montant")} value={montant} onChange={(e) => setMontant(e.target.value)} autoComplete="off" />
            <Champ id="saisie-motif" libelle={t("paie.mois.motif")} value={libelle} onChange={(e) => setLibelle(e.target.value)} autoComplete="off" />
            <Bouton type="submit" disabled={membreId === "" || elementId === "" || lu === null || libelle.trim() === "" || saisir.isPending}>
              {t("paie.mois.ajouter")}
            </Bouton>
          </div>
          <MessageErreur erreur={saisir.error ?? retirer.error} />
        </Formulaire>
      ) : null}
    </Section>
  );
}
