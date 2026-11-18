import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler, ErreurApi } from "../../api";
import { aujourdHui } from "../../app/aujourdhui";
import { commandeAutorisee } from "../../app/navigation";
import { Bouton } from "../../composants/Bouton";
import { CaseACocher, Champ, Selection } from "../../composants/Champ";
import { MessageErreur } from "../../composants/Messages";
import { type Cle, dateCivile, dateHeure, francs, t } from "../../i18n";
import type { FactureAffichee, ModePaiement, Remboursement, RepriseDeLInscription, SituationInscription } from "../../types/gesco";
import { lireMontant } from "../paie/calcul";
import { ChoixCompte, modesDu } from "./Depenses";
import { MODES } from "./Encaissement";
import { CATEGORIE_REMBOURSEMENT, gesteDeLaSaisie, lendemain, remboursementPrerempli } from "./reprise";

/** Refus de la reprise qui ont un plancher : le message rappelle le minimum. */
const REFUS_AVEC_MINIMUM = new Set(["reprise.deja_remboursee", "reprise.avoir_a_couvrir"]);

const STATUTS_REMBOURSEMENT = new Map<string, Cle>([
  ["a_approuver", "paye_avant.remboursement_en_attente"],
  ["approuvee", "paye_avant.remboursement_a_payer"],
  ["payee", "paye_avant.remboursement_paye"],
]);

/** État d'un remboursement : son mode une fois payé, sinon l'accord ou le paiement attendu. */
function etatRemboursement(r: Remboursement): string {
  const mode = (Object.keys(MODES) as ModePaiement[]).find((m) => m === r.mode);
  if (r.statut === "payee" && mode !== undefined) return t(MODES[mode]);
  return t(STATUTS_REMBOURSEMENT.get(r.statut) ?? "paye_avant.remboursement_paye");
}

function arret(reprise: RepriseDeLInscription): string {
  return reprise.a_ce_jour ? t("paye_avant.arrete_a_ce_jour_court") : t("paye_avant.arrete_court", { date: dateCivile(lendemain(reprise.date_arret)) });
}

/** « Payé avant GESCO » d'une inscription, dans la section « Frais » de la fiche : le montant
 *  et sa note, Modifier et Annuler pour qui en a le droit, l'historique, l'excédent à rendre au
 *  parent, ses remboursements et le bouton « Rembourser ». */
export function RepriseInscription({
  situation,
  eleve,
  payeurs,
  permissions,
  surChange,
}: {
  situation: SituationInscription;
  eleve: string;
  payeurs: readonly string[];
  permissions: readonly string[];
  surChange: () => void;
}) {
  const reprise = situation.reprise;
  const inscriptionId = situation.inscription.id;
  const modifie = commandeAutorisee("reprise_enregistrer", permissions);
  const [edition, setEdition] = useState<"montant" | "annulation" | null>(null);
  const [montant, setMontant] = useState("");
  const [note, setNote] = useState("");
  const [motif, setMotif] = useState("");
  const [historique, setHistorique] = useState(false);
  const [rembourser, setRembourser] = useState(false);
  const anciennes = useQuery({
    queryKey: ["reprise-historique", inscriptionId],
    queryFn: () => appeler("reprise_historique", { inscription_id: inscriptionId }),
    enabled: historique,
  });
  const fini = () => {
    setEdition(null);
    surChange();
  };
  const enregistrement = useMutation({
    // La fiche ne garde pas l'instant de lecture de la facture : depuis la fiche, le dernier geste
    // gagne ; l'écran « Payé avant GESCO » envoie le sien.
    mutationFn: (valeur: number) => appeler("reprise_enregistrer", { inscription_id: inscriptionId, montant: valeur, note, motif: "", lue_le: null }),
    onSuccess: fini,
  });
  const annulation = useMutation({
    mutationFn: () => appeler("reprise_annuler", { inscription_id: inscriptionId, motif, lue_le: null }),
    onSuccess: fini,
  });
  if (reprise === null) return null;
  const actuel = reprise.chiffres.montant;
  if (actuel === 0 && reprise.remboursements.length === 0 && !modifie) return null;
  const geste = gesteDeLaSaisie(montant, actuel);
  const erreur = enregistrement.error ?? annulation.error;
  const refus = erreur instanceof ErreurApi ? erreur.cle : null;
  return (
    <div className="flex flex-col gap-1 border-t border-muet/30 pt-1">
      <div className="flex flex-wrap items-center gap-3">
        <span className="text-encre">
          {actuel === 0 ? t("paye_avant.aucun") : t("paye_avant.ligne", { montant: francs(actuel), arret: arret(reprise) })}
          {reprise.active === null || reprise.active.note === "" ? "" : ` · ${reprise.active.note}`}
        </span>
        {modifie && edition === null ? (
          <>
            <Bouton
              variante="secondaire"
              onClick={() => {
                setMontant(actuel === 0 ? "" : String(actuel));
                setNote(reprise.active?.note ?? "");
                setEdition("montant");
              }}
            >
              {t(actuel === 0 ? "paye_avant.saisir" : "paye_avant.modifier")}
            </Bouton>
            {reprise.active === null ? null : (
              <Bouton
                variante="secondaire"
                onClick={() => {
                  setMotif(t("paye_avant.motif_propose"));
                  setEdition("annulation");
                }}
              >
                {t("paye_avant.annuler")}
              </Bouton>
            )}
          </>
        ) : null}
        <Bouton variante="secondaire" onClick={() => setHistorique(!historique)}>
          {t(historique ? "paye_avant.masquer_historique" : "paye_avant.historique")}
        </Bouton>
      </div>
      {edition === "montant" ? (
        <div className="flex flex-wrap items-end gap-2">
          <Champ id={`reprise-montant-${inscriptionId}`} libelle={t("paye_avant.paye_avant")} inputMode="numeric" value={montant} onChange={(e) => setMontant(e.target.value)} autoFocus />
          <Champ id={`reprise-note-${inscriptionId}`} libelle={t("paye_avant.note")} aide={t("paye_avant.note_aide")} maxLength={120} value={note} onChange={(e) => setNote(e.target.value)} />
          <Bouton
            onClick={() => {
              if (geste.geste === "annuler") {
                setMotif(t("paye_avant.motif_propose"));
                setEdition("annulation");
              } else enregistrement.mutate(geste.geste === "enregistrer" ? geste.montant : actuel);
            }}
            disabled={enregistrement.isPending || geste.geste === "illisible" || (geste.geste === "rien" && note === (reprise.active?.note ?? ""))}
          >
            {t("commun.enregistrer")}
          </Bouton>
          <Bouton variante="secondaire" onClick={() => setEdition(null)}>{t("commun.annuler")}</Bouton>
          {geste.geste === "illisible" ? <p className="w-full text-rouge">{t("paye_avant.illisible")}</p> : null}
        </div>
      ) : null}
      {edition === "annulation" ? (
        <div className="flex flex-col gap-2 rounded border border-ocre p-2">
          <p className="text-encre">{t("paye_avant.annuler_explication", { eleve, montant: francs(actuel) })}</p>
          <div className="flex flex-wrap items-end gap-2">
            <Champ id={`reprise-motif-${inscriptionId}`} libelle={t("paye_avant.motif")} value={motif} onChange={(e) => setMotif(e.target.value)} />
            <Bouton onClick={() => annulation.mutate()} disabled={annulation.isPending || motif.trim() === ""}>{t("paye_avant.confirmer_annulation")}</Bouton>
            <Bouton variante="secondaire" onClick={() => setEdition(null)}>{t("commun.annuler")}</Bouton>
          </div>
        </div>
      ) : null}
      <MessageErreur erreur={erreur} surRelire={surChange} />
      {refus !== null && REFUS_AVEC_MINIMUM.has(refus) && reprise.chiffres.rembourse > 0 ? (
        <p className="text-rouge">{t("paye_avant.deja_rembourse", { montant: francs(reprise.chiffres.rembourse) })}</p>
      ) : null}
      {historique ? (
        <ul className="flex flex-col gap-1 pl-3 text-muet">
          <MessageErreur erreur={anciennes.error} />
          {(anciennes.data ?? []).filter((r) => r.statut === "annulee").map((r) => (
            <li key={r.id}>
              {t("paye_avant.ancienne", { montant: francs(r.montant), saisie: dateHeure(r.cree_le), annulee: dateHeure(r.mis_a_jour_le), motif: r.motif_annulation })}
              {r.origine === "import" ? ` · ${t("paye_avant.importee")}` : ""}
            </li>
          ))}
          {anciennes.isSuccess && anciennes.data.every((r) => r.statut !== "annulee") ? <li>{t("paye_avant.historique_vide")}</li> : null}
        </ul>
      ) : null}
      {reprise.remboursements.map((r, i) => (
        <p key={`${r.date}-${String(i)}`} className="text-encre">
          {t("paye_avant.remboursement", {
            date: dateCivile(r.date),
            montant: francs(r.montant),
            etat: etatRemboursement(r),
          })}
        </p>
      ))}
      {reprise.chiffres.a_rembourser === 0 ? null : (
        <div className="flex flex-wrap items-center gap-3">
          <span className="font-medium text-ocre">{t("paye_avant.a_rembourser_ligne", { montant: francs(reprise.chiffres.a_rembourser) })}</span>
          {commandeAutorisee("depense_saisir", permissions) && !rembourser ? (
            <Bouton variante="secondaire" onClick={() => setRembourser(true)}>{t("paye_avant.rembourser")}</Bouton>
          ) : null}
        </div>
      )}
      {rembourser && reprise.chiffres.a_rembourser > 0 ? (
        <RembourserParent
          inscriptionId={inscriptionId}
          eleve={eleve}
          payeurs={payeurs}
          aRembourser={reprise.chiffres.a_rembourser}
          surFini={() => {
            setRembourser(false);
            surChange();
          }}
        />
      ) : null}
    </div>
  );
}

/** Formulaire de dépense prérempli pour rendre l'excédent au parent : les règles ordinaires des
 *  dépenses s'appliquent (seuil d'accord, paiement par la caisse ouverte ou un compte). */
function RembourserParent({
  inscriptionId,
  eleve,
  payeurs,
  aRembourser,
  surFini,
}: {
  inscriptionId: string;
  eleve: string;
  payeurs: readonly string[];
  aRembourser: number;
  surFini: () => void;
}) {
  const client = useQueryClient();
  const mois = aujourdHui().slice(0, 7);
  const etat = useQuery({ queryKey: ["depenses", mois], queryFn: () => appeler("depenses_lire", { mois }) });
  const e = etat.data;
  const categorie = e?.categories.find((c) => c.code === CATEGORIE_REMBOURSEMENT);
  const prerempli = remboursementPrerempli({
    inscriptionId,
    eleve,
    payeur: payeurs[0] ?? "",
    aRembourser,
    categorieId: categorie?.id ?? "",
    libelle: (nom) => t("paye_avant.libelle_remboursement", { eleve: nom }),
  });
  const [beneficiaire, setBeneficiaire] = useState(prerempli.beneficiaire);
  const [montant, setMontant] = useState(String(prerempli.montant));
  const [payee, setPayee] = useState(true);
  const caisse = e?.comptes.find((c) => c.nature === "caisse");
  const parDefaut = e?.caisse_ouverte === true ? caisse : (e?.comptes.find((c) => c.nature !== "caisse") ?? caisse);
  const [compteId, setCompteId] = useState<string | null>(null);
  const [mode, setMode] = useState<ModePaiement | null>(null);
  const [reference, setReference] = useState("");
  const compte = compteId ?? parDefaut?.id ?? "";
  const modeChoisi = mode ?? modesDu(parDefaut)[0] ?? "especes";
  const lu = lireMontant(montant);
  const attendra = e !== undefined && lu !== null && e.seuil > 0 && lu > e.seuil;
  const paie = e !== undefined && e.droits.payer && payee && !attendra;
  const saisie = useMutation({
    mutationFn: () =>
      appeler("depense_saisir", {
        ...prerempli,
        beneficiaire,
        montant: lu ?? 0,
        paiement: paie ? { compte_id: compte, mode: modeChoisi, reference } : null,
      }),
    onSuccess: () => {
      for (const cle of ["depenses", "caisse", "reprises"]) void client.invalidateQueries({ queryKey: [cle] });
      surFini();
    },
  });
  return (
    <div className="flex flex-col gap-2 rounded border border-muet/40 p-2">
      <p className="font-medium text-encre">{t("paye_avant.rembourser_titre", { eleve })}</p>
      <MessageErreur erreur={etat.error} />
      {e === undefined ? null : categorie === undefined ? (
        <p className="text-rouge">{t("paye_avant.categorie_absente")}</p>
      ) : (
        <>
          <p className="text-muet">{t("paye_avant.rembourser_aide", { categorie: categorie.libelle, libelle: prerempli.libelle })}</p>
          <div className="flex flex-wrap items-end gap-2">
            {payeurs.length > 1 ? (
              <Selection id={`rembourser-payeur-${inscriptionId}`} libelle={t("depense.beneficiaire")} valeur={beneficiaire} surChangement={setBeneficiaire} options={payeurs.map((p) => ({ valeur: p, libelle: p }))} />
            ) : (
              <Champ id={`rembourser-beneficiaire-${inscriptionId}`} libelle={t("depense.beneficiaire")} value={beneficiaire} onChange={(ev) => setBeneficiaire(ev.target.value)} autoComplete="off" />
            )}
            <Champ
              id={`rembourser-montant-${inscriptionId}`}
              libelle={t("paie.mois.montant")}
              aide={t("paye_avant.rembourser_au_plus", { montant: francs(aRembourser) })}
              inputMode="numeric"
              value={montant}
              onChange={(ev) => setMontant(ev.target.value)}
            />
          </div>
          {attendra ? (
            <p className="text-ocre">{t("depense.attendra", { seuil: francs(e.seuil) })}</p>
          ) : e.droits.payer ? (
            <>
              <CaseACocher id={`rembourser-payee-${inscriptionId}`} libelle={t("depense.payee_maintenant")} coche={payee} surChangement={setPayee} />
              {payee ? (
                <ChoixCompte
                  prefixe={`rembourser-${inscriptionId}`}
                  comptes={e.comptes}
                  compteId={compte}
                  surCompte={(id, m) => {
                    setCompteId(id);
                    setMode(m);
                  }}
                  mode={modeChoisi}
                  surMode={setMode}
                  reference={reference}
                  surReference={setReference}
                />
              ) : null}
            </>
          ) : null}
          <div className="flex flex-wrap gap-2">
            <Bouton onClick={() => saisie.mutate()} disabled={saisie.isPending || lu === null || lu === 0 || lu > aRembourser || beneficiaire.trim() === ""}>
              {t("paye_avant.rembourser_confirmer")}
            </Bouton>
            <Bouton variante="secondaire" onClick={surFini}>{t("commun.annuler")}</Bouton>
          </div>
          <MessageErreur erreur={saisie.error} />
        </>
      )}
    </div>
  );
}

/** Frais facultatifs que l'école avait oublié de cocher : ceux de la grille de la facture qui n'y
 *  sont pas encore, avec les échéances qu'ils créeraient ; et le retrait d'un frais ajouté par
 *  erreur (ses échéances que rien n'a payées). */
export function AjouterFrais({ inscriptionId, facture, surChange }: { inscriptionId: string; facture: FactureAffichee; surChange: () => void }) {
  const [ouvert, setOuvert] = useState(false);
  const ajoutables = useQuery({
    queryKey: ["frais-ajoutables", inscriptionId],
    queryFn: () => appeler("frais_ajouter_apercu", { inscription_id: inscriptionId }),
    enabled: ouvert,
  });
  const ajout = useMutation({
    mutationFn: (ligneTarifaireId: string) => appeler("frais_ajouter", { inscription_id: inscriptionId, ligne_tarifaire_id: ligneTarifaireId }),
    onSuccess: () => {
      void ajoutables.refetch();
      surChange();
    },
  });
  const retrait = useMutation({
    mutationFn: (ligneFactureId: string) => appeler("frais_retirer", { inscription_id: inscriptionId, ligne_facture_id: ligneFactureId }),
    onSuccess: () => {
      void ajoutables.refetch();
      surChange();
    },
  });
  // Les frais d'inscription et la scolarité sont toujours dus ; les autres peuvent se retirer.
  const retirables = facture.lignes.filter((l) => l.type_frais !== "inscription" && l.type_frais !== "scolarite" && l.montant > 0);
  if (!ouvert) {
    return <Bouton variante="secondaire" onClick={() => setOuvert(true)}>{t("frais_ajoute.ajouter")}</Bouton>;
  }
  return (
    <div className="flex w-full flex-col gap-2 rounded border border-muet/40 p-2">
      <p className="font-medium text-encre">{t("frais_ajoute.titre")}</p>
      <MessageErreur erreur={ajoutables.error ?? ajout.error ?? retrait.error} />
      {ajoutables.isSuccess && ajoutables.data.length === 0 ? <p className="text-muet">{t("frais_ajoute.aucun")}</p> : null}
      {(ajoutables.data ?? []).map((f) => (
        <div key={f.ligne_tarifaire_id} className="flex flex-wrap items-center gap-3">
          <span className="text-encre">
            {t("frais_ajoute.propose", { libelle: f.libelle, montant: francs(f.montant) })}
            {f.echeances.length === 0 ? "" : ` · ${f.echeances.map((x) => t("frais_ajoute.echeance", { date: dateCivile(x.date), montant: francs(x.montant) })).join(", ")}`}
          </span>
          <Bouton variante="secondaire" onClick={() => ajout.mutate(f.ligne_tarifaire_id)} disabled={ajout.isPending}>{t("commun.ajouter")}</Bouton>
        </div>
      ))}
      {retirables.length === 0 ? null : (
        <>
          <p className="text-muet">{t("frais_ajoute.retirer_aide")}</p>
          {retirables.map((l) => (
            <div key={l.id} className="flex flex-wrap items-center gap-3">
              <span className="text-encre">{t("frais_ajoute.present", { libelle: l.libelle, montant: francs(l.montant) })}</span>
              <Bouton variante="secondaire" onClick={() => retrait.mutate(l.id)} disabled={retrait.isPending}>{t("commun.retirer")}</Bouton>
            </div>
          ))}
        </>
      )}
      <div>
        <Bouton variante="secondaire" onClick={() => setOuvert(false)}>{t("frais_ajoute.fermer")}</Bouton>
      </div>
    </div>
  );
}
