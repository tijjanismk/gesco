import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { commandeAutorisee } from "../../app/navigation";
import { Bouton } from "../../composants/Bouton";
import { Champ, Selection } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { type Cle, dateCivile, dateHeure, francs, t } from "../../i18n";
import { ApercuPdf } from "../../composants/ApercuPdf";
import type { Remise, SituationInscription, StatutRemise, TypeFrais, TypeRemise, ValeurRemise, Versement } from "../../types/gesco";
import { EcartsFrais } from "./AjustementFrais";
import { lireJour } from "./echeances";
import { MODES } from "./Encaissement";

export const TYPES_REMISE: Record<TypeRemise, Cle> = {
  bourse: "remise.type.bourse",
  demi_bourse: "remise.type.demi_bourse",
  fratrie: "remise.type.fratrie",
  enfant_personnel: "remise.type.enfant_personnel",
  arrangement: "remise.type.arrangement",
};

const STATUTS_REMISE: Record<StatutRemise, Cle> = {
  proposee: "remise.statut.proposee",
  accordee: "remise.statut.accordee",
  refusee: "remise.statut.refusee",
  annulee: "remise.statut.annulee",
};

const FRAIS: Record<TypeFrais, Cle> = {
  inscription: "frais.type.inscription",
  scolarite: "frais.type.scolarite",
  cantine: "frais.type.cantine",
  transport: "frais.type.transport",
  tenue: "frais.type.tenue",
  examen: "frais.type.examen",
  autre: "frais.type.autre",
};

export function valeurRemise(valeur: ValeurRemise): string {
  return valeur.type === "taux" ? `${valeur.pour_cent} %` : francs(valeur.montant);
}

/** Section « Frais » de la fiche élève (SPEC_PHASE_3 §4.4, §5) : facture, échéances nettes de
 *  remise, reste à payer, changement de plan, remises proposées et accordées. */
export function FraisEleve({ eleveId, permissions }: { eleveId: string; permissions: readonly string[] }) {
  const situation = useQuery({ queryKey: ["situation", eleveId], queryFn: () => appeler("situation_financiere", { id: eleveId }) });
  const vivantes = (situation.data ?? []).filter((s) => s.inscription.statut === "brouillon" || s.inscription.statut === "validee" || s.facture !== null);
  const releve = useMutation({ mutationFn: () => appeler("releve_compte_produire", { id: eleveId }) });
  // Avoirs de l'élève, toutes années : ce qui reste à utiliser.
  const avoirDisponible = (situation.data ?? []).flatMap((s) => s.avoirs).reduce((somme, a) => somme + a.montant - a.utilise, 0);
  return (
    <Section titre={t("facture.titre")}>
      <MessageErreur erreur={situation.error} />
      {vivantes.some((s) => s.facture !== null) ? (
        <div className="flex flex-col gap-2">
          <div><Bouton variante="secondaire" onClick={() => releve.mutate()} disabled={releve.isPending}>{t("facture.imprimer_releve")}</Bouton></div>
          <MessageErreur erreur={releve.error} />
          {releve.data === undefined ? null : <ApercuPdf base64={releve.data.pdf} />}
        </div>
      ) : null}
      {situation.isSuccess && vivantes.length === 0 ? <p className="text-sm text-muet">{t("facture.aucune")}</p> : null}
      {vivantes.map((s) => (
        <FraisInscription key={s.inscription.id} situation={s} eleveId={eleveId} permissions={permissions} avoirDisponible={avoirDisponible} />
      ))}
    </Section>
  );
}

function FraisInscription({ situation, eleveId, permissions, avoirDisponible }: { situation: SituationInscription; eleveId: string; permissions: readonly string[]; avoirDisponible: number }) {
  const client = useQueryClient();
  const rafraichir = () => {
    void client.invalidateQueries({ queryKey: ["situation", eleveId] });
    void client.invalidateQueries({ queryKey: ["remises-en-attente"] });
  };
  const facture = situation.facture;
  const [plan, setPlan] = useState(facture?.facture.plan_paiement_id ?? "");
  const changement = useMutation({
    mutationFn: () => appeler("echeancier_changer_plan", { inscription_id: situation.inscription.id, plan_paiement_id: plan }),
    onSuccess: rafraichir,
  });
  const [jour, setJour] = useState("");
  const jourLu = lireJour(jour);
  const inscriptionId = situation.inscription.id;
  // Frais de la classe actuelle, pour un élève qui a changé de classe : l'écart avant tout.
  const ajustable =
    facture !== null && situation.inscription.statut === "validee" && situation.inscription.classe_precedente_id !== null && commandeAutorisee("facture_ajuster", permissions);
  const apercu = useQuery({
    queryKey: ["ajustement", inscriptionId, "actuelle"],
    queryFn: () => appeler("facture_ajustement_apercu", { inscription_id: inscriptionId, classe_id: null }),
    enabled: ajustable,
  });
  const ajustement = useMutation({
    mutationFn: () => appeler("facture_ajuster", { inscription_id: inscriptionId }),
    onSuccess: () => {
      rafraichir();
      void client.invalidateQueries({ queryKey: ["ajustement", inscriptionId] });
    },
  });
  const avoir = useMutation({ mutationFn: () => appeler("avoirs_utiliser", { inscription_id: inscriptionId }), onSuccess: rafraichir });
  const decalage = useMutation({
    mutationFn: () => appeler("echeancier_changer_jour", { inscription_id: situation.inscription.id, jour: jourLu ?? 0 }),
    onSuccess: () => {
      setJour("");
      rafraichir();
    },
  });
  return (
    <div className="flex flex-col gap-2 rounded border border-muet/30 p-3 text-sm">
      {facture === null ? (
        <p className="text-muet">{t("facture.pas_encore", { annee: situation.annee, classe: situation.classe })}</p>
      ) : (
        <>
          <p className="font-medium text-encre">
            {t("facture.entete", { annee: situation.annee, classe: situation.classe, numero: facture.facture.numero, plan: facture.plan })}
          </p>
          <p className="text-encre">
            {facture.ajustement === 0
              ? t("facture.totaux", { total: francs(facture.facture.total), remise: francs(facture.remise), paye: francs(facture.paye), reste: francs(facture.reste) })
              : t("facture.totaux_ajustes", {
                  total: francs(facture.facture.total),
                  remise: francs(facture.remise),
                  baisse: francs(facture.ajustement),
                  paye: francs(facture.paye),
                  reste: francs(facture.reste),
                })}
          </p>
          {facture.ajustements.filter((a) => !a.remplace).map((a) => (
            <p key={a.id} className="text-muet">
              {t("facture.ajustement_fait", { date: dateCivile(a.date), hausse: francs(a.hausse), baisse: francs(a.baisse) })}
              {a.avoir === 0 ? "" : ` ${t("facture.ajustement_avoir", { avoir: francs(a.avoir) })}`}
            </p>
          ))}
          <table className="w-full text-left font-condensee tabular-nums">
            <thead>
              <tr className="text-muet">
                {(["facture.echeance", "facture.date", "facture.montant", "facture.remise", "facture.ajustement", "facture.net"] as const).map((cle) => (
                  <th key={cle} className="px-2 py-1 font-normal">{t(cle)}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {facture.echeances.map((e) => (
                <tr key={e.echeance.id} className="border-b border-muet/30 text-encre">
                  <td className="px-2 py-1">{e.echeance.libelle}</td>
                  <td className="px-2 py-1">{dateCivile(e.echeance.date)}</td>
                  <td className="px-2 py-1">{francs(e.echeance.montant)}</td>
                  <td className="px-2 py-1">{e.remise === 0 ? "" : `− ${francs(e.remise)}`}</td>
                  <td className="px-2 py-1">{e.ajustement === 0 ? "" : `− ${francs(e.ajustement)}`}</td>
                  <td className="px-2 py-1 font-medium">{francs(e.net)}</td>
                </tr>
              ))}
            </tbody>
          </table>
          {commandeAutorisee("echeancier_changer_plan", permissions) && situation.plans.length > 1 ? (
            <div className="flex flex-wrap items-end gap-2">
              <Selection
                id={`plan-${situation.inscription.id}`}
                libelle={t("facture.changer_plan")}
                valeur={plan}
                surChangement={setPlan}
                options={situation.plans.map((p) => ({ valeur: p.id, libelle: p.libelle }))}
              />
              <Bouton variante="secondaire" onClick={() => changement.mutate()} disabled={changement.isPending || plan === facture.facture.plan_paiement_id}>
                {t("facture.appliquer_plan")}
              </Bouton>
            </div>
          ) : null}
          <MessageErreur erreur={changement.error} />
          {commandeAutorisee("echeancier_changer_jour", permissions) ? (
            <div className="flex flex-wrap items-end gap-2">
              <div className="w-48">
                <Champ id={`jour-${situation.inscription.id}`} libelle={t("facture.jour_echeance")} inputMode="numeric" value={jour} onChange={(e) => setJour(e.target.value)} />
              </div>
              <Bouton variante="secondaire" onClick={() => decalage.mutate()} disabled={decalage.isPending || jourLu === null}>
                {t("facture.appliquer_jour")}
              </Bouton>
              <p className="w-full text-muet">{t("facture.jour_aide")}</p>
            </div>
          ) : null}
          <MessageErreur erreur={decalage.error} />
          {ajustable && apercu.data !== undefined && apercu.data.ecarts.length > 0 ? (
            <div className="flex flex-col gap-2 rounded border border-muet/30 p-2">
              <EcartsFrais apercu={apercu.data} />
              <div>
                <Bouton variante="secondaire" onClick={() => ajustement.mutate()} disabled={ajustement.isPending}>
                  {t("facture.appliquer_frais_classe", { classe: apercu.data.classe_apres })}
                </Bouton>
              </div>
            </div>
          ) : null}
          <MessageErreur erreur={apercu.error ?? ajustement.error} />
          {avoirDisponible > 0 && facture.reste > 0 && commandeAutorisee("avoirs_utiliser", permissions) ? (
            <div className="flex flex-wrap items-center gap-2">
              <Bouton variante="secondaire" onClick={() => avoir.mutate()} disabled={avoir.isPending}>
                {t("facture.utiliser_avoir", { avoir: francs(avoirDisponible) })}
              </Bouton>
            </div>
          ) : null}
          <MessageErreur erreur={avoir.error} />
        </>
      )}
      {situation.avoirs.map((a) => (
        <p key={a.id} className="text-encre">
          {t("facture.avoir", { montant: francs(a.montant), solde: francs(a.montant - a.utilise) })}
        </p>
      ))}
      {situation.versements.length === 0 ? null : (
        <ul className="flex flex-col gap-1">
          {situation.versements.map((v) => (
            <LigneVersement
              key={v.id}
              versement={v}
              peutReimprimer={commandeAutorisee("recu_produire", permissions)}
              peutAnnuler={commandeAutorisee("versement_annuler", permissions)}
              surAnnule={rafraichir}
            />
          ))}
        </ul>
      )}
      {situation.remises.length === 0 ? null : (
        <ul className="flex flex-col gap-1">
          {situation.remises.map((r) => (
            <LigneRemise key={r.id} remise={r} permissions={permissions} surDecision={rafraichir} />
          ))}
        </ul>
      )}
      {commandeAutorisee("remise_proposer", permissions) && (situation.inscription.statut === "brouillon" || situation.inscription.statut === "validee") ? (
        <NouvelleRemise inscriptionId={situation.inscription.id} accorde={commandeAutorisee("remise_accorder", permissions)} surCree={rafraichir} />
      ) : null}
    </div>
  );
}

function LigneRemise({ remise, permissions, surDecision }: { remise: Remise; permissions: readonly string[]; surDecision: () => void }) {
  const [motif, setMotif] = useState("");
  const decision = useMutation({
    mutationFn: (action: "accorder" | "refuser" | "annuler") =>
      action === "accorder"
        ? appeler("remise_accorder", { id: remise.id })
        : appeler(action === "refuser" ? "remise_refuser" : "remise_annuler", { id: remise.id, motif }),
    onSuccess: surDecision,
  });
  const decide = commandeAutorisee("remise_accorder", permissions);
  return (
    <li className="flex flex-col gap-1 border-t border-muet/30 pt-1">
      <p className="text-encre">
        {t("remise.resume", {
          type: t(TYPES_REMISE[remise.type_remise]),
          valeur: valeurRemise(remise.valeur),
          frais: t(FRAIS[remise.type_frais]),
          statut: t(STATUTS_REMISE[remise.statut]),
        })}
      </p>
      <p className="text-muet">{remise.motif_decision === "" ? remise.motif : `${remise.motif} · ${remise.motif_decision}`}</p>
      {decide && (remise.statut === "proposee" || remise.statut === "accordee") ? (
        <div className="flex flex-wrap items-end gap-2">
          {remise.statut === "proposee" ? (
            <Bouton onClick={() => decision.mutate("accorder")} disabled={decision.isPending}>{t("remise.accorder")}</Bouton>
          ) : null}
          <Champ id={`remise-motif-${remise.id}`} libelle={t("remise.motif_decision")} value={motif} onChange={(e) => setMotif(e.target.value)} />
          <Bouton variante="secondaire" onClick={() => decision.mutate(remise.statut === "proposee" ? "refuser" : "annuler")} disabled={decision.isPending}>
            {t(remise.statut === "proposee" ? "remise.refuser" : "remise.annuler")}
          </Bouton>
        </div>
      ) : null}
      <MessageErreur erreur={decision.error} />
    </li>
  );
}

function NouvelleRemise({ inscriptionId, accorde, surCree }: { inscriptionId: string; accorde: boolean; surCree: () => void }) {
  const [ouverte, setOuverte] = useState(false);
  const [type, setType] = useState<TypeRemise>("fratrie");
  const [nature, setNature] = useState<"taux" | "montant">("taux");
  const [quantite, setQuantite] = useState("");
  const [frais, setFrais] = useState<TypeFrais>("scolarite");
  const [motif, setMotif] = useState("");
  const creation = useMutation({
    mutationFn: async () => {
      const nombre = Number.parseInt(quantite.replace(/\D/g, "") || "0", 10);
      const valeur: ValeurRemise = nature === "taux" ? { type: "taux", pour_cent: nombre } : { type: "montant", montant: nombre };
      const remise = await appeler("remise_proposer", { inscription_id: inscriptionId, type_remise: type, valeur, type_frais: frais, motif });
      // Qui propose et décide à la fois accorde d'un seul geste (§5.1).
      return accorde ? appeler("remise_accorder", { id: remise.id }) : remise;
    },
    onSuccess: () => {
      setOuverte(false);
      setQuantite("");
      setMotif("");
      surCree();
    },
  });
  if (!ouverte) {
    return (
      <div>
        <Bouton variante="secondaire" onClick={() => setOuverte(true)}>{t(accorde ? "remise.nouvelle_accordee" : "remise.nouvelle")}</Bouton>
      </div>
    );
  }
  return (
    <div className="flex flex-col gap-2 border-t border-muet/30 pt-2">
      <div className="flex flex-wrap items-end gap-2">
        <Selection
          id={`remise-type-${inscriptionId}`}
          libelle={t("remise.type")}
          valeur={type}
          surChangement={(v) => setType((Object.keys(TYPES_REMISE) as TypeRemise[]).find((x) => x === v) ?? "arrangement")}
          options={(Object.keys(TYPES_REMISE) as TypeRemise[]).map((x) => ({ valeur: x, libelle: t(TYPES_REMISE[x]) }))}
        />
        <Selection
          id={`remise-nature-${inscriptionId}`}
          libelle={t("remise.nature")}
          valeur={nature}
          surChangement={(v) => setNature(v === "montant" ? "montant" : "taux")}
          options={[
            { valeur: "taux", libelle: t("remise.nature.taux") },
            { valeur: "montant", libelle: t("remise.nature.montant") },
          ]}
        />
        <Champ id={`remise-valeur-${inscriptionId}`} libelle={t(nature === "taux" ? "remise.pour_cent" : "remise.francs")} inputMode="numeric" value={quantite} onChange={(e) => setQuantite(e.target.value)} />
        <Selection
          id={`remise-frais-${inscriptionId}`}
          libelle={t("remise.frais")}
          valeur={frais}
          surChangement={(v) => setFrais((Object.keys(FRAIS) as TypeFrais[]).find((x) => x === v) ?? "scolarite")}
          options={(Object.keys(FRAIS) as TypeFrais[]).map((x) => ({ valeur: x, libelle: t(FRAIS[x]) }))}
        />
      </div>
      <Champ id={`remise-motif-${inscriptionId}`} libelle={t("remise.motif")} aide={t("remise.motif_aide")} value={motif} onChange={(e) => setMotif(e.target.value)} />
      <MessageErreur erreur={creation.error} />
      <div className="flex flex-wrap gap-2">
        <Bouton onClick={() => creation.mutate()} disabled={creation.isPending}>{t(accorde ? "remise.proposer_accorder" : "remise.proposer")}</Bouton>
        <Bouton variante="secondaire" onClick={() => setOuverte(false)}>{t("commun.annuler")}</Bouton>
      </div>
    </div>
  );
}

/** Versement de l'élève : reçu, date, mode, montant ; réimpression en duplicata ; annulation
 * motivée pour l'Administrateur et le Comptable (SPEC_PHASE_3 §8). */
function LigneVersement({
  versement,
  peutReimprimer,
  peutAnnuler,
  surAnnule,
}: {
  versement: Versement;
  peutReimprimer: boolean;
  peutAnnuler: boolean;
  surAnnule: () => void;
}) {
  const client = useQueryClient();
  const [annulation, setAnnulation] = useState(false);
  const [motif, setMotif] = useState("");
  const duplicata = useMutation({ mutationFn: () => appeler("recu_produire", { id: versement.id }) });
  const annuler = useMutation({
    mutationFn: () => appeler("versement_annuler", { id: versement.id, motif }),
    onSuccess: () => {
      setAnnulation(false);
      for (const cle of ["caisse", "sessions-caisse", "avertissements"]) void client.invalidateQueries({ queryKey: [cle] });
      surAnnule();
    },
  });
  const annule = versement.statut === "annule";
  const recu = annuler.data?.recu ?? duplicata.data;
  return (
    <li className="flex flex-col gap-1 border-t border-muet/30 pt-1">
      <div className="flex flex-wrap items-center gap-3">
        <span className={annule ? "text-muet line-through" : "text-encre"}>
          {t("versement.resume", {
            numero: versement.numero_recu,
            date: dateHeure(versement.encaisse_le),
            mode: t(MODES[versement.mode]),
            montant: francs(versement.montant),
            payeur: versement.payeur_nom,
          })}
        </span>
        {peutReimprimer ? (
          <Bouton variante="secondaire" onClick={() => duplicata.mutate()} disabled={duplicata.isPending}>{t("versement.reimprimer")}</Bouton>
        ) : null}
        {peutAnnuler && !annule && !annulation ? (
          <Bouton variante="secondaire" onClick={() => setAnnulation(true)}>{t("versement.annuler")}</Bouton>
        ) : null}
      </div>
      {annule ? <p className="text-muet">{t("versement.annule_motif", { motif: versement.motif_annulation })}</p> : null}
      {annulation ? (
        <div className="flex flex-col gap-2 rounded border border-ocre p-2">
          <p className="text-encre">{t("versement.annuler_explication")}</p>
          <div className="flex flex-wrap items-end gap-2">
            <Champ id={`versement-motif-${versement.id}`} libelle={t("versement.motif")} value={motif} onChange={(e) => setMotif(e.target.value)} />
            <Bouton onClick={() => annuler.mutate()} disabled={annuler.isPending || motif.trim() === ""}>{t("versement.confirmer_annulation")}</Bouton>
            <Bouton variante="secondaire" onClick={() => setAnnulation(false)}>{t("commun.annuler")}</Bouton>
          </div>
        </div>
      ) : null}
      {annuler.data?.regularisation ? (
        <p className="text-encre">{t("versement.regularise", { montant: francs(annuler.data.regularisation.montant) })}</p>
      ) : null}
      <MessageErreur erreur={duplicata.error ?? annuler.error} />
      {recu === undefined ? null : <ApercuPdf base64={recu.pdf} />}
    </li>
  );
}
