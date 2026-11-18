import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { type KeyboardEvent, useState } from "react";

import { appeler } from "../../api";
import { commandeAutorisee } from "../../app/navigation";
import { Bouton } from "../../composants/Bouton";
import { Champ, Selection } from "../../composants/Champ";
import { MessageErreur } from "../../composants/Messages";
import { Tableau } from "../../composants/Tableau";
import { francs, t, texteDeCle } from "../../i18n";
import type { LignePriseEnCharge, ListePrisesEnCharge } from "../../types/gesco";
import { APRES_PRISE_EN_CHARGE, lireFrancs, litLaListe, resumerApercu } from "./prise_en_charge";
import { filtrerLignes } from "./reprise";
import { DuEtat } from "./DuEtat";
import { VersementsEtat } from "./VersementsEtat";

const TOUTES = "";

/** Un geste en attente de confirmation, pour un élève ou une sélection. */
export type SorteGeste = "cocher" | "decocher" | "modifier" | "appliquer" | "abandonner" | "defaire";

export interface GesteEnCours {
  sorte: SorteGeste;
  lignes: readonly { inscription_id: string; eleve: string; lue_le: number | null }[];
  /** Montant proposé dans le formulaire (montant attendu à modifier). */
  montant?: number;
}

/** Vrai si l'entrée « Prise en charge par l'État » se montre : l'année courante a une classe du
 *  lycée ou du technique, ou une prise en charge existe. Qui ne lit que les versements de l'État
 *  la voit toujours. */
export function usePriseEnChargeMontree(permissions: readonly string[] | undefined): boolean {
  const lit = permissions !== undefined && litLaListe(permissions);
  const liste = useQuery({
    queryKey: ["prises-en-charge", "entree"],
    queryFn: () => appeler("prises_en_charge_lister", {}),
    enabled: lit,
    retry: false,
  });
  if (permissions === undefined) return false;
  if (!lit) return commandeAutorisee("versements_etat_lister", permissions);
  return liste.data !== undefined && (liste.data.classes.length > 0 || liste.data.lignes.length > 0);
}

/** « Prise en charge par l'État » (Finance) : les élèves orientés et leurs cases, puis les
 *  versements de l'État et leur répartition. */
export function PriseEnChargeEtat() {
  const session = useQuery({ queryKey: ["session"], queryFn: () => appeler("session_courante", null) });
  const permissions = session.data?.permissions ?? [];
  const annees = useQuery({ queryKey: ["annees"], queryFn: () => appeler("annees_lister", null) });
  const [annee, setAnnee] = useState<string | null>(null);
  const anneeId = annee ?? annees.data?.find((a) => a.est_courante)?.id ?? annees.data?.[0]?.id ?? null;
  const eleves = litLaListe(permissions);
  const versements = commandeAutorisee("versements_etat_lister", permissions);
  const [onglet, setOnglet] = useState<"eleves" | "versements" | "du">("eleves");
  // Qui ne lit pas la liste des élèves ne lit pas non plus ce que doit l'État.
  const actif = eleves ? onglet : "versements";
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("pec.titre")}</h1>
      <p className="text-sm text-muet">{t("pec.aide")}</p>
      <MessageErreur erreur={annees.error ?? session.error} />
      <div className="flex flex-wrap items-end gap-3">
        <Selection
          id="pec-annee"
          libelle={t("pec.annee")}
          valeur={anneeId ?? ""}
          surChangement={setAnnee}
          options={(annees.data ?? []).map((a) => ({ valeur: a.id, libelle: a.libelle }))}
        />
      </div>
      <div role="tablist" className="flex flex-wrap gap-2 border-b border-muet/40">
        {eleves ? (
          <OngletBouton actif={actif === "eleves"} surClic={() => setOnglet("eleves")}>{t("pec.onglet.eleves")}</OngletBouton>
        ) : null}
        {versements ? (
          <OngletBouton actif={actif === "versements"} surClic={() => setOnglet("versements")}>{t("pec.onglet.versements")}</OngletBouton>
        ) : null}
        {eleves ? (
          <OngletBouton actif={actif === "du"} surClic={() => setOnglet("du")}>{t("pec.onglet.du")}</OngletBouton>
        ) : null}
      </div>
      {anneeId === null || session.data === undefined ? null : actif === "eleves" && eleves ? (
        <OngletEleves key={anneeId} anneeId={anneeId} permissions={permissions} />
      ) : actif === "du" && eleves ? (
        <DuEtat key={anneeId} anneeId={anneeId} />
      ) : versements ? (
        <VersementsEtat key={anneeId} anneeId={anneeId} permissions={permissions} />
      ) : null}
    </div>
  );
}

function OngletBouton({ actif, surClic, children }: { actif: boolean; surClic: () => void; children: string }) {
  return (
    <button
      type="button"
      role="tab"
      aria-selected={actif}
      onClick={surClic}
      className={`-mb-px border-b-2 px-3 py-1.5 text-sm ${actif ? "border-primaire font-medium text-encre" : "border-transparent text-muet hover:text-encre"}`}
    >
      {children}
    </button>
  );
}

/** Onglet « Élèves » : une ligne par inscription des classes du lycée et du technique, la case,
 *  le montant attendu modifiable dans la ligne, la sélection et ses gestes, les totaux. */
function OngletEleves({ anneeId, permissions }: { anneeId: string; permissions: readonly string[] }) {
  const [classe, setClasse] = useState(TOUTES);
  const [recherche, setRecherche] = useState("");
  const liste = useQuery({
    queryKey: ["prises-en-charge", anneeId, classe],
    queryFn: () => appeler("prises_en_charge_lister", { annee_id: anneeId, classe_id: classe === TOUTES ? null : classe }),
  });
  const regle = commandeAutorisee("prise_en_charge_regler", permissions);
  const [choisis, setChoisis] = useState<readonly string[]>([]);
  const [geste, setGeste] = useState<GesteEnCours | null>(null);
  const [montantSelection, setMontantSelection] = useState("");
  const donnees: ListePrisesEnCharge | undefined = liste.data;
  const lignes = filtrerLignes(donnees?.lignes ?? [], recherche);
  const close = donnees?.annee_close ?? false;
  const choisies = lignes.filter((l) => choisis.includes(l.inscription_id));
  const aCocher = (ls: readonly LignePriseEnCharge[]) => ls.filter((l) => !l.pris_en_charge && l.case_possible);
  const cochees = (ls: readonly LignePriseEnCharge[]) => ls.filter((l) => l.pris_en_charge);
  const ouvrir = (sorte: SorteGeste, ls: readonly LignePriseEnCharge[], montant?: number) => {
    if (ls.length === 0) return;
    setGeste({ sorte, lignes: ls, montant });
  };
  const basculer = (id: string) => setChoisis((c) => (c.includes(id) ? c.filter((x) => x !== id) : [...c, id]));
  const montantLu = lireFrancs(montantSelection);
  return (
    <div className="flex flex-col gap-3">
      <MessageErreur erreur={liste.error} />
      <div className="flex flex-wrap items-end gap-3">
        <Selection
          id="pec-classe"
          libelle={t("pec.classe")}
          valeur={classe}
          surChangement={(v) => {
            setClasse(v);
            setChoisis([]);
          }}
          options={[{ valeur: TOUTES, libelle: t("pec.toutes_classes") }, ...(donnees?.classes ?? []).map((c) => ({ valeur: c.id, libelle: c.libelle }))]}
        />
        <Champ id="pec-recherche" type="search" libelle={t("pec.rechercher")} value={recherche} onChange={(e) => setRecherche(e.target.value)} autoComplete="off" />
      </div>
      {donnees === undefined ? null : (
        <>
          <div className="flex flex-col gap-1 text-sm text-encre">
            <p className="font-medium">
              {t("pec.totaux", {
                nombre: donnees.nombre,
                attendu: francs(donnees.total_attendu),
                recu: francs(donnees.total_recu),
                du: francs(donnees.total_du_etat),
                echu: francs(donnees.total_du_etat_echu),
              })}
            </p>
            {donnees.total_a_rembourser === 0 ? null : <p className="text-ocre">{t("pec.totaux_a_rembourser", { montant: francs(donnees.total_a_rembourser) })}</p>}
            {donnees.total_recu_en_plus === 0 ? null : <p className="text-muet">{t("pec.totaux_recu_en_plus", { montant: francs(donnees.total_recu_en_plus) })}</p>}
            {close ? <p className="text-muet">{t("pec.annee_close")}</p> : null}
            {regle ? null : <p className="text-muet">{t("pec.lecture_seule")}</p>}
          </div>
          {regle ? (
            <div className="flex flex-wrap items-end gap-2">
              {!close && aCocher(lignes).length > 0 ? (
                <Bouton variante="secondaire" onClick={() => ouvrir("cocher", aCocher(lignes))}>
                  {t("pec.cocher_affiches", { nombre: aCocher(lignes).length })}
                </Bouton>
              ) : null}
              <Bouton variante="secondaire" onClick={() => setChoisis(choisies.length === lignes.length ? [] : lignes.map((l) => l.inscription_id))}>
                {t(choisies.length === lignes.length && lignes.length > 0 ? "pec.rien_choisir" : "pec.tout_choisir")}
              </Bouton>
              {choisies.length === 0 ? null : (
                <>
                  <span className="text-sm text-muet">{t("pec.choisis", { nombre: choisies.length })}</span>
                  {!close && aCocher(choisies).length > 0 ? (
                    <Bouton variante="secondaire" onClick={() => ouvrir("cocher", aCocher(choisies))}>{t("pec.cocher_choisis", { nombre: aCocher(choisies).length })}</Bouton>
                  ) : null}
                  {cochees(choisies).length > 0 ? (
                    <>
                      <Champ id="pec-montant-selection" libelle={t("pec.montant_selection")} inputMode="numeric" value={montantSelection} onChange={(e) => setMontantSelection(e.target.value)} />
                      <Bouton variante="secondaire" disabled={montantLu === null} onClick={() => ouvrir("modifier", cochees(choisies), montantLu ?? undefined)}>
                        {t("pec.appliquer_montant", { nombre: cochees(choisies).length })}
                      </Bouton>
                      <Bouton variante="secondaire" onClick={() => ouvrir("abandonner", cochees(choisies))}>{t("pec.ne_plus_attendre")}</Bouton>
                    </>
                  ) : null}
                </>
              )}
            </div>
          ) : null}
          {geste === null ? null : (
            <PanneauGeste
              key={`${geste.sorte}-${geste.lignes.map((l) => l.inscription_id).join(",")}`}
              geste={geste}
              surFini={() => {
                setGeste(null);
                setChoisis([]);
              }}
              surAnnuler={() => setGeste(null)}
            />
          )}
          <Tableau
            nomExport="prise-en-charge-etat"
            titre={t("pec.titre")}
            lignes={lignes}
            cle={(l) => l.inscription_id}
            colonnes={[
              ...(regle
                ? [
                    {
                      titre: t("pec.choisir"),
                      valeur: () => "",
                      sansExport: true,
                      rendu: (l: LignePriseEnCharge) => (
                        <input
                          type="checkbox"
                          className="accent-primaire"
                          aria-label={t("pec.choisir_eleve", { eleve: l.eleve })}
                          checked={choisis.includes(l.inscription_id)}
                          onChange={() => basculer(l.inscription_id)}
                        />
                      ),
                    },
                  ]
                : []),
              {
                titre: t("pec.pris_en_charge"),
                valeur: (l: LignePriseEnCharge) => (l.pris_en_charge ? t("commun.oui") : ""),
                rendu: (l: LignePriseEnCharge) => (
                  <input
                    type="checkbox"
                    className="accent-primaire"
                    aria-label={t("pec.case", { eleve: l.eleve })}
                    checked={l.pris_en_charge}
                    disabled={!regle || close || (!l.pris_en_charge && !l.case_possible)}
                    onChange={(e) => ouvrir(e.target.checked ? "cocher" : "decocher", [l])}
                  />
                ),
              },
              { titre: t("pec.classe"), valeur: (l: LignePriseEnCharge) => l.classe },
              { titre: t("pec.eleve"), valeur: (l: LignePriseEnCharge) => l.eleve },
              { titre: t("pec.matricule"), valeur: (l: LignePriseEnCharge) => l.matricule },
              { titre: t("pec.reference"), valeur: (l: LignePriseEnCharge) => l.reference },
              {
                titre: t("pec.attendu"),
                valeur: (l: LignePriseEnCharge) => (l.pris_en_charge ? francs(l.montant_attendu) : ""),
                rendu: (l: LignePriseEnCharge) =>
                  l.pris_en_charge && regle ? (
                    <CaseAttendu key={`${l.inscription_id}-${String(l.montant_attendu)}-${String(l.lue_le)}`} ligne={l} />
                  ) : l.pris_en_charge ? (
                    francs(l.montant_attendu)
                  ) : (
                    ""
                  ),
              },
              { titre: t("pec.recu"), valeur: (l: LignePriseEnCharge) => (l.recu_etat === 0 ? "" : francs(l.recu_etat)) },
              {
                titre: t("pec.reste_etat"),
                valeur: (l: LignePriseEnCharge) =>
                  l.reste_etat === 0 ? "" : l.reste_etat_echu === 0 ? francs(l.reste_etat) : t("pec.dont_echu", { reste: francs(l.reste_etat), echu: francs(l.reste_etat_echu) }),
              },
              { titre: t("pec.reste_parent"), valeur: (l: LignePriseEnCharge) => francs(l.reste_parent) },
              { titre: t("pec.a_rembourser"), valeur: (l: LignePriseEnCharge) => (l.a_rembourser === 0 ? "" : francs(l.a_rembourser)) },
            ]}
          />
        </>
      )}
    </div>
  );
}

/** Montant attendu modifiable dans la ligne : Entrée enregistre. */
function CaseAttendu({ ligne }: { ligne: LignePriseEnCharge }) {
  const client = useQueryClient();
  const [texte, setTexte] = useState(String(ligne.montant_attendu));
  const lu = lireFrancs(texte);
  const modification = useMutation({
    mutationFn: (montant: number) =>
      appeler("prise_en_charge_regler", { cibles: [{ inscription_id: ligne.inscription_id, lue_le: ligne.lue_le }], geste: "modifier", montant_attendu: montant }),
    onSuccess: () => {
      for (const cle of APRES_PRISE_EN_CHARGE) void client.invalidateQueries({ queryKey: [cle] });
    },
  });
  return (
    <div className="flex flex-col">
      <input
        aria-label={t("pec.attendu_de", { eleve: ligne.eleve })}
        className="w-28 rounded border border-muet bg-surface px-2 py-0.5 text-right text-primaire focus:outline-2 focus:outline-primaire"
        inputMode="numeric"
        autoComplete="off"
        value={texte}
        onChange={(e) => setTexte(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && lu !== null && lu !== ligne.montant_attendu) {
            e.preventDefault();
            modification.mutate(lu);
          }
        }}
      />
      {lu === null && texte.trim() !== "" ? <span className="text-xs text-rouge">{t("pec.illisible")}</span> : null}
      {ligne.abandonne === 0 ? null : <span className="text-xs text-muet">{t("pec.abandonne", { montant: francs(ligne.abandonne) })}</span>}
      <MessageErreur erreur={modification.error} />
    </div>
  );
}

/** Aperçu d'un geste et sa confirmation : ce que le parent ne doit plus ou devra, le trop-payé
 *  et ce qui est à lui rembourser, ce que l'État devra. Entrée confirme. */
export function PanneauGeste({ geste, surFini, surAnnuler }: { geste: GesteEnCours; surFini: () => void; surAnnuler: () => void }) {
  const client = useQueryClient();
  const { sorte, lignes } = geste;
  const [montant, setMontant] = useState(geste.montant === undefined ? "" : String(geste.montant));
  const [motif, setMotif] = useState(sorte === "decocher" ? t("pec.motif_propose") : "");
  const [reference, setReference] = useState("");
  const lu = lireFrancs(montant);
  const ids = lignes.map((l) => l.inscription_id);
  const apercu = useQuery({
    queryKey: ["pec-apercu", sorte, ids, lu],
    queryFn: () =>
      appeler("prise_en_charge_apercu", {
        inscriptions: ids,
        geste: sorte === "defaire" ? "defaire_abandon" : sorte === "appliquer" ? "appliquer_montant_propose" : sorte,
        montant_attendu: sorte === "cocher" || sorte === "modifier" ? lu : null,
        montant_abandon: sorte === "abandonner" ? lu : null,
      }),
  });
  const cibles = lignes.map((l) => ({ inscription_id: l.inscription_id, lue_le: l.lue_le }));
  const confirmation = useMutation({
    mutationFn: () =>
      sorte === "abandonner" || sorte === "defaire"
        ? appeler("prise_en_charge_abandonner", { cibles, montant: sorte === "abandonner" ? lu : null, defaire: sorte === "defaire", motif })
        : appeler("prise_en_charge_regler", {
            cibles,
            geste: sorte === "appliquer" ? "appliquer_montant_propose" : sorte,
            montant_attendu: sorte === "decocher" || sorte === "appliquer" ? null : lu,
            reference: (sorte === "cocher" || sorte === "modifier") && reference.trim() !== "" ? reference.trim() : null,
            motif,
          }),
    onSuccess: () => {
      for (const cle of APRES_PRISE_EN_CHARGE) void client.invalidateQueries({ queryKey: [cle] });
      surFini();
    },
  });
  const resume = apercu.data === undefined ? null : resumerApercu(apercu.data.eleves);
  const refuses = (apercu.data?.eleves ?? []).filter((e) => e.refus !== null);
  const seul = lignes.length === 1 ? lignes[0] : undefined;
  const propose = apercu.data?.eleves[0]?.montant_propose;
  const motifExige = sorte === "decocher" || sorte === "abandonner" || sorte === "defaire";
  const pret = !confirmation.isPending && (!motifExige || motif.trim() !== "") && (sorte !== "modifier" || lu !== null) && (resume === null || resume.refuses < lignes.length);
  const titres: Record<SorteGeste, string> = {
    cocher: seul === undefined ? t("pec.geste.cocher_selection", { nombre: lignes.length }) : t("pec.geste.cocher", { eleve: seul.eleve }),
    decocher: seul === undefined ? t("pec.geste.decocher_selection", { nombre: lignes.length }) : t("pec.geste.decocher", { eleve: seul.eleve }),
    modifier: t("pec.geste.modifier", { nombre: lignes.length }),
    appliquer: t("pec.geste.appliquer", { nombre: lignes.length }),
    abandonner: t("pec.geste.abandonner", { nombre: lignes.length }),
    defaire: t("pec.geste.defaire", { nombre: lignes.length }),
  };
  const confirmer = () => {
    if (pret) confirmation.mutate();
  };
  const surEntree = (e: KeyboardEvent) => {
    if (e.key === "Enter") {
      e.preventDefault();
      confirmer();
    }
  };
  return (
    <div className="flex flex-col gap-2 rounded border border-ocre p-3 text-sm">
      <p className="font-medium text-encre">{titres[sorte]}</p>
      <div className="flex flex-wrap items-end gap-2">
        {sorte === "cocher" || sorte === "modifier" || sorte === "abandonner" ? (
          <Champ
            id="pec-geste-montant"
            libelle={t(sorte === "abandonner" ? "pec.montant_abandon" : "pec.montant_attendu")}
            aide={
              sorte === "cocher" && propose !== undefined && seul !== undefined
                ? t("pec.montant_propose", { montant: francs(propose) })
                : sorte === "cocher"
                  ? t("pec.montant_propose_selection")
                  : sorte === "abandonner"
                    ? t("pec.montant_abandon_aide")
                    : undefined
            }
            inputMode="numeric"
            value={montant}
            onChange={(e) => setMontant(e.target.value)}
            onKeyDown={surEntree}
            autoComplete="off"
          />
        ) : null}
        {(sorte === "cocher" || sorte === "modifier") && seul !== undefined ? (
          <Champ id="pec-geste-reference" libelle={t("pec.reference")} aide={t("pec.reference_aide")} maxLength={60} value={reference} onChange={(e) => setReference(e.target.value)} onKeyDown={surEntree} autoComplete="off" />
        ) : null}
        {motifExige ? <Champ id="pec-geste-motif" libelle={t("pec.motif")} value={motif} onChange={(e) => setMotif(e.target.value)} onKeyDown={surEntree} autoComplete="off" autoFocus /> : null}
      </div>
      <MessageErreur erreur={apercu.error} />
      {resume === null ? null : (
        <div className="flex flex-col gap-1 text-encre">
          {resume.ne_doit_plus > 0 ? <p>{t("pec.apercu.ne_doit_plus", { montant: francs(resume.ne_doit_plus) })}</p> : null}
          {resume.devra_en_plus > 0 ? (
            <p className="text-ocre">
              {resume.dont_deja_rembourse > 0
                ? t("pec.apercu.devra_dont", { montant: francs(resume.devra_en_plus), rembourse: francs(resume.dont_deja_rembourse) })
                : t("pec.apercu.devra", { montant: francs(resume.devra_en_plus) })}
            </p>
          ) : null}
          {resume.trop_paye > 0 ? (
            <p>{t("pec.apercu.trop_paye", { trop: francs(resume.trop_paye), reporte: francs(resume.reporte), rembourser: francs(resume.a_rembourser) })}</p>
          ) : null}
          <p className="text-muet">{t("pec.apercu.etat", { avant: francs(resume.etat_avant), apres: francs(resume.etat_apres) })}</p>
          {resume.nombre === 0 && resume.refuses === 0 ? <p className="text-muet">{t("pec.apercu.rien")}</p> : null}
          {refuses.map((e) => (
            <p key={e.inscription_id} className="text-rouge">
              {t("pec.apercu.refus", { eleve: e.eleve, raison: texteDeCle(e.refus ?? "") })}
            </p>
          ))}
        </div>
      )}
      <MessageErreur erreur={confirmation.error} />
      <div className="flex flex-wrap gap-2">
        <Bouton onClick={confirmer} disabled={!pret} autoFocus={!motifExige}>
          {t("pec.confirmer")}
        </Bouton>
        <Bouton variante="secondaire" onClick={surAnnuler}>{t("commun.annuler")}</Bouton>
      </div>
    </div>
  );
}
