import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { aujourdHui } from "../../app/aujourdhui";
import { Bouton } from "../../composants/Bouton";
import { CaseACocher, Champ, Formulaire, Selection } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { Tableau } from "../../composants/Tableau";
import { dateCivile, francs, t } from "../../i18n";
import type { FactureFournisseurAffichee, FournisseurAffiche, Fournisseurs as Etat, ModePaiement } from "../../types/gesco";
import { lireMontant } from "../paie/calcul";
import { ChoixCompte, STATUTS, modesDu } from "./Depenses";

/** État d'une facture en un mot. */
function etatFacture(f: FactureFournisseurAffichee): string {
  if (f.facture.annulee) return t("fournisseur.facture.annulee");
  if (f.reste === 0) return t("fournisseur.facture.soldee");
  if (f.en_retard) return t("fournisseur.facture.en_retard");
  return t("fournisseur.facture.a_payer");
}

/** Fournisseurs (SPEC_PHASE_8 §4) : leurs factures, payées en une ou plusieurs fois par des
 *  dépenses, et ce que l'école leur doit. */
export function Fournisseurs() {
  const client = useQueryClient();
  const etat = useQuery({ queryKey: ["fournisseurs"], queryFn: () => appeler("fournisseurs_lire", null) });
  const apres = (e: Etat) => {
    client.setQueryData(["fournisseurs"], e);
  };
  const apresPaiement = () => {
    void client.invalidateQueries({ queryKey: ["fournisseurs"] });
    void client.invalidateQueries({ queryKey: ["depenses"] });
    void client.invalidateQueries({ queryKey: ["caisse"] });
  };
  const [choisi, setChoisi] = useState<string | null>(null);
  const [facture, setFacture] = useState<string | null>(null);
  // Chaque enregistrement remet le formulaire du fournisseur à neuf.
  const [tour, setTour] = useState(0);
  const e = etat.data;
  const fournisseur = e?.fournisseurs.find((f) => f.fournisseur.id === choisi);
  const ouvertes = e?.factures.filter((f) => f.reste > 0) ?? [];
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("fournisseur.titre")}</h1>
      <MessageErreur erreur={etat.error} />
      {e === undefined ? null : (
        <>
          <Section titre={t("fournisseur.liste", { total: francs(e.total_du) })}>
            <Tableau
              nomExport="fournisseurs"
              titre={t("fournisseur.liste", { total: francs(e.total_du) })}
              lignes={e.fournisseurs}
              cle={(f) => f.fournisseur.id}
              surSelection={(f) => {
                setChoisi(f.fournisseur.id);
                setFacture(null);
              }}
              colonnes={[
                { titre: t("fournisseur.nom"), valeur: (f) => f.fournisseur.nom },
                { titre: t("fournisseur.telephone"), valeur: (f) => f.fournisseur.telephone },
                { titre: t("fournisseur.fourniture"), valeur: (f) => f.fournisseur.fourniture },
                { titre: t("fournisseur.factures_ouvertes"), valeur: (f) => String(f.factures_ouvertes) },
                { titre: t("fournisseur.du"), valeur: (f) => (f.en_retard ? t("fournisseur.du_en_retard", { montant: francs(f.du) }) : francs(f.du)) },
                { titre: t("tresorerie.etat"), valeur: (f) => t(f.fournisseur.actif ? "fournisseur.actif_etat" : "fournisseur.retire") },
              ]}
            />
            {e.droits.saisir ? (
              <FicheFournisseur
                key={`${choisi ?? "nouveau"}-${String(tour)}`}
                f={fournisseur}
                surFait={(x) => {
                  setTour(tour + 1);
                  apres(x);
                }}
                surNouveau={() => {
                  setChoisi(null);
                  setFacture(null);
                }}
              />
            ) : null}
          </Section>
          {fournisseur === undefined ? (
            <Section titre={t("fournisseur.factures_a_payer")}>
              <ListeFactures
                nom="factures-a-payer"
                factures={ouvertes}
                avecFournisseur
                surSelection={(f) => {
                  setChoisi(f.facture.fournisseur_id);
                  setFacture(f.facture.id);
                }}
              />
            </Section>
          ) : (
            <FacturesDu f={fournisseur} e={e} facture={facture} surFacture={setFacture} surFait={apres} surPaiement={apresPaiement} />
          )}
        </>
      )}
    </div>
  );
}

function ListeFactures({
  nom,
  factures,
  avecFournisseur,
  surSelection,
}: {
  nom: string;
  factures: readonly FactureFournisseurAffichee[];
  avecFournisseur: boolean;
  surSelection: (f: FactureFournisseurAffichee) => void;
}) {
  return (
    <Tableau
      nomExport={nom}
      lignes={factures}
      cle={(f) => f.facture.id}
      surSelection={surSelection}
      colonnes={[
        ...(avecFournisseur ? [{ titre: t("fournisseur.nom"), valeur: (f: FactureFournisseurAffichee) => f.fournisseur }] : []),
        { titre: t("fournisseur.facture.numero"), valeur: (f) => f.facture.numero },
        { titre: t("depense.date"), valeur: (f) => dateCivile(f.facture.date) },
        { titre: t("fournisseur.facture.echeance"), valeur: (f) => (f.facture.echeance === null ? "" : dateCivile(f.facture.echeance)) },
        { titre: t("depense.libelle"), valeur: (f) => f.facture.libelle },
        { titre: t("paie.mois.montant"), valeur: (f) => francs(f.facture.montant) },
        { titre: t("fournisseur.facture.paye"), valeur: (f) => francs(f.paye) },
        { titre: t("fournisseur.facture.reste"), valeur: (f) => francs(f.reste) },
        { titre: t("depense.etat"), valeur: etatFacture },
      ]}
    />
  );
}

function FicheFournisseur({ f, surFait, surNouveau }: { f: FournisseurAffiche | undefined; surFait: (e: Etat) => void; surNouveau: () => void }) {
  const [nom, setNom] = useState(f?.fournisseur.nom ?? "");
  const [telephone, setTelephone] = useState(f?.fournisseur.telephone ?? "");
  const [fourniture, setFourniture] = useState(f?.fournisseur.fourniture ?? "");
  const [actif, setActif] = useState(f?.fournisseur.actif ?? true);
  const enregistrer = useMutation({
    mutationFn: () => appeler("fournisseur_enregistrer", { id: f?.fournisseur.id ?? null, nom, telephone, fourniture, actif }),
    onSuccess: surFait,
  });
  return (
    <Formulaire surEnvoi={() => enregistrer.mutate()}>
      <p className="text-sm font-medium text-encre">{f === undefined ? t("fournisseur.nouveau") : f.fournisseur.nom}</p>
      <div className="flex flex-wrap items-end gap-3">
        <Champ id="fournisseur-nom" libelle={t("fournisseur.nom")} value={nom} onChange={(ev) => setNom(ev.target.value)} autoComplete="off" />
        <Champ id="fournisseur-telephone" libelle={t("fournisseur.telephone")} value={telephone} onChange={(ev) => setTelephone(ev.target.value)} autoComplete="off" />
        <Champ id="fournisseur-fourniture" libelle={t("fournisseur.fourniture")} value={fourniture} onChange={(ev) => setFourniture(ev.target.value)} autoComplete="off" />
        {f === undefined ? null : <CaseACocher id="fournisseur-actif" libelle={t("fournisseur.actif")} coche={actif} surChangement={setActif} />}
        <Bouton type="submit" disabled={nom.trim() === "" || enregistrer.isPending}>
          {t("fournisseur.enregistrer")}
        </Bouton>
        {f === undefined ? null : (
          <Bouton variante="secondaire" onClick={surNouveau}>
            {t("fournisseur.nouveau")}
          </Bouton>
        )}
      </div>
      <MessageErreur erreur={enregistrer.error} />
    </Formulaire>
  );
}

function FacturesDu({
  f,
  e,
  facture,
  surFacture,
  surFait,
  surPaiement,
}: {
  f: FournisseurAffiche;
  e: Etat;
  facture: string | null;
  surFacture: (id: string | null) => void;
  surFait: (e: Etat) => void;
  surPaiement: () => void;
}) {
  const siennes = e.factures.filter((x) => x.facture.fournisseur_id === f.fournisseur.id);
  const choisie = siennes.find((x) => x.facture.id === facture);
  const [tour, setTour] = useState(0);
  return (
    <Section titre={t("fournisseur.factures_de", { nom: f.fournisseur.nom, du: francs(f.du) })}>
      <ListeFactures nom={`factures-${f.fournisseur.nom}`} factures={siennes} avecFournisseur={false} surSelection={(x) => surFacture(x.facture.id)} />
      {choisie === undefined ? null : <DetailFacture key={`${choisie.facture.id}-${String(choisie.paye)}-${String(choisie.en_cours)}`} f={choisie} e={e} surFait={surFait} surPaiement={surPaiement} />}
      {e.droits.saisir && f.fournisseur.actif ? (
        <NouvelleFacture
          key={tour}
          f={f}
          e={e}
          surFait={(x) => {
            setTour(tour + 1);
            surFait(x);
          }}
        />
      ) : null}
    </Section>
  );
}

function NouvelleFacture({ f, e, surFait }: { f: FournisseurAffiche; e: Etat; surFait: (e: Etat) => void }) {
  const [numero, setNumero] = useState("");
  const [date, setDate] = useState(aujourdHui());
  const [echeance, setEcheance] = useState("");
  const [libelle, setLibelle] = useState("");
  // Une facture de fournisseur n'est jamais un salaire : la première autre catégorie d'abord.
  const [categorie, setCategorie] = useState((e.categories.find((c) => c.code !== "salaires") ?? e.categories[0])?.id ?? "");
  const [montant, setMontant] = useState("");
  const lu = lireMontant(montant);
  const enregistrer = useMutation({
    mutationFn: () =>
      appeler("facture_fournisseur_enregistrer", {
        fournisseur_id: f.fournisseur.id,
        numero,
        date,
        echeance,
        libelle,
        categorie_id: categorie,
        montant: lu ?? 0,
      }),
    onSuccess: surFait,
  });
  return (
    <Formulaire surEnvoi={() => enregistrer.mutate()}>
      <p className="text-sm font-medium text-encre">{t("fournisseur.facture.nouvelle")}</p>
      <div className="flex flex-wrap items-end gap-3">
        <Champ id="facture-numero" libelle={t("fournisseur.facture.numero")} value={numero} onChange={(ev) => setNumero(ev.target.value)} autoComplete="off" />
        <Champ id="facture-date" type="date" libelle={t("depense.date")} value={date} onChange={(ev) => setDate(ev.target.value)} />
        <Champ id="facture-echeance" type="date" libelle={t("fournisseur.facture.echeance")} value={echeance} onChange={(ev) => setEcheance(ev.target.value)} />
        <Champ id="facture-libelle" libelle={t("depense.libelle")} value={libelle} onChange={(ev) => setLibelle(ev.target.value)} autoComplete="off" />
        <Selection id="facture-categorie" libelle={t("depense.categorie")} valeur={categorie} surChangement={setCategorie} options={e.categories.map((c) => ({ valeur: c.id, libelle: c.libelle }))} />
        <Champ id="facture-montant" libelle={t("paie.mois.montant")} value={montant} onChange={(ev) => setMontant(ev.target.value)} autoComplete="off" />
        <Bouton type="submit" disabled={libelle.trim() === "" || lu === null || categorie === "" || enregistrer.isPending}>
          {t("fournisseur.facture.enregistrer")}
        </Bouton>
      </div>
      <MessageErreur erreur={enregistrer.error} />
    </Formulaire>
  );
}

function DetailFacture({ f, e, surFait, surPaiement }: { f: FactureFournisseurAffichee; e: Etat; surFait: (e: Etat) => void; surPaiement: () => void }) {
  const caisse = e.comptes.find((c) => c.nature === "caisse");
  const parDefaut = e.caisse_ouverte ? caisse : e.comptes.find((c) => c.nature !== "caisse") ?? caisse;
  const [date, setDate] = useState(aujourdHui());
  const [montant, setMontant] = useState(String(f.a_payer));
  const [payee, setPayee] = useState(e.droits.payer);
  const [compteId, setCompteId] = useState(parDefaut?.id ?? "");
  const [mode, setMode] = useState<ModePaiement>(modesDu(parDefaut)[0] ?? "especes");
  const [reference, setReference] = useState("");
  const [motif, setMotif] = useState("");
  const lu = lireMontant(montant);
  const attendra = lu !== null && e.seuil > 0 && lu > e.seuil;
  const payer = useMutation({
    mutationFn: () =>
      appeler("depense_saisir", {
        date,
        categorie_id: f.facture.categorie_id,
        libelle: t("fournisseur.facture.libelle_paiement", { numero: f.facture.numero, libelle: f.facture.libelle }).slice(0, 120),
        beneficiaire: f.fournisseur,
        montant: lu ?? 0,
        paiement: payee && !attendra ? { compte_id: compteId, mode, reference } : null,
        facture_id: f.facture.id,
      }),
    onSuccess: surPaiement,
  });
  const annuler = useMutation({
    mutationFn: () => appeler("facture_fournisseur_annuler", { id: f.facture.id, motif }),
    onSuccess: surFait,
  });
  const annulable = e.droits.payer && !f.facture.annulee && f.paye === 0 && f.en_cours === 0;
  return (
    <div className="flex flex-col gap-2 rounded border border-muet p-3">
      <p className="text-sm font-medium text-encre">
        {[f.facture.numero, dateCivile(f.facture.date), f.facture.libelle, f.categorie, francs(f.facture.montant)].filter((x) => x !== "").join(" · ")}
      </p>
      <p className="text-sm text-muet">
        {t("fournisseur.facture.situation", { paye: francs(f.paye), en_cours: francs(f.en_cours), reste: francs(f.reste) })}
        {f.facture.annulee ? ` · ${f.facture.motif_annulation}` : ""}
      </p>
      <ul className="flex flex-col gap-1 text-sm text-encre">
        {f.reglements.map((r) => (
          <li key={r.depense_id}>
            {dateCivile(r.date)} · {francs(r.montant)} · {t(STATUTS[r.statut])}
          </li>
        ))}
      </ul>
      {e.droits.saisir && f.a_payer > 0 ? (
        <Formulaire surEnvoi={() => payer.mutate()}>
          <p className="text-sm font-medium text-encre">{t("fournisseur.facture.payer_titre", { a_payer: francs(f.a_payer) })}</p>
          <div className="flex flex-wrap items-end gap-3">
            <Champ id="facture-paiement-date" type="date" libelle={t("depense.date")} value={date} onChange={(ev) => setDate(ev.target.value)} />
            <Champ id="facture-paiement-montant" libelle={t("paie.mois.montant")} value={montant} onChange={(ev) => setMontant(ev.target.value)} autoComplete="off" />
          </div>
          {attendra ? (
            <p className="text-sm text-ocre">{t("depense.attendra", { seuil: francs(e.seuil) })}</p>
          ) : e.droits.payer ? (
            <>
              <CaseACocher id="facture-payee" libelle={t("depense.payee_maintenant")} coche={payee} surChangement={setPayee} />
              {payee ? (
                <ChoixCompte
                  prefixe="facture"
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
            <Bouton type="submit" disabled={lu === null || payer.isPending}>
              {t("fournisseur.facture.payer")}
            </Bouton>
          </div>
          <MessageErreur erreur={payer.error} />
        </Formulaire>
      ) : null}
      {annulable ? (
        <div className="flex flex-wrap items-end gap-3">
          <Champ id="facture-annulation" libelle={t("depense.motif_annulation")} value={motif} onChange={(ev) => setMotif(ev.target.value)} autoComplete="off" />
          <Bouton variante="danger" onClick={() => annuler.mutate()} disabled={motif.trim() === "" || annuler.isPending}>
            {t("fournisseur.facture.annuler")}
          </Bouton>
          <MessageErreur erreur={annuler.error} />
        </div>
      ) : null}
    </div>
  );
}
