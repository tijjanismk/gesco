import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { aujourdHui } from "../../app/aujourdhui";
import { commandeAutorisee } from "../../app/navigation";
import { Bouton } from "../../composants/Bouton";
import { Champ, Formulaire, Selection } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { Tableau } from "../../composants/Tableau";
import { dateCivile, francs, moisCivil, t } from "../../i18n";
import type { ModePaiement, RecetteAffichee, RecettesDuMois as Etat } from "../../types/gesco";
import { lireMontant } from "../paie/calcul";
import { ChoixCompte, modesDu } from "./Depenses";

/** Autres recettes (SPEC_VERSION_1_1 §1) : un don, une location de salle, reçus hors des
 *  versements des parents, sur un compte de l'école ; leur annulation motivée. */
export function Recettes() {
  const client = useQueryClient();
  const session = useQuery({ queryKey: ["session"], queryFn: () => appeler("session_courante", null) });
  const gere = commandeAutorisee("recette_saisir", session.data?.permissions ?? []);
  const [mois, setMois] = useState(aujourdHui().slice(0, 7));
  const etat = useQuery({ queryKey: ["recettes", mois], queryFn: () => appeler("recettes_lire", { mois }), enabled: /^\d{4}-\d{2}$/.test(mois) });
  const apres = (e: Etat) => {
    client.setQueryData(["recettes", e.mois], e);
    void client.invalidateQueries({ queryKey: ["recettes"] });
    void client.invalidateQueries({ queryKey: ["caisse"] });
    void client.invalidateQueries({ queryKey: ["comptes-soldes"] });
  };
  const [choisie, setChoisie] = useState<string | null>(null);
  const e = etat.data;
  const recette = e?.recettes.find((r) => r.recette.id === choisie);
  const titre = t("recette.du_mois", { mois: moisCivil(mois), total: francs(e?.total ?? 0) });
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("recette.titre")}</h1>
      <MessageErreur erreur={etat.error} />
      {e === undefined ? null : (
        <>
          {gere ? <NouvelleRecette e={e} surFait={apres} /> : null}
          <Section titre={titre}>
            <Champ id="recettes-mois" type="month" libelle={t("paie.mois.choisir")} value={mois} onChange={(ev) => setMois(ev.target.value)} />
            <Tableau
              nomExport={`autres-recettes-${mois}`}
              titre={titre}
              lignes={e.recettes}
              cle={(r) => r.recette.id}
              surSelection={gere ? (r) => setChoisie(r.recette.id) : undefined}
              colonnes={[
                { titre: t("depense.date"), valeur: (r) => dateCivile(r.recette.date) },
                { titre: t("depense.categorie"), valeur: (r) => r.categorie },
                { titre: t("depense.libelle"), valeur: (r) => r.recette.libelle },
                { titre: t("recette.payeur"), valeur: (r) => r.recette.payeur },
                { titre: t("paie.mois.montant"), valeur: (r) => francs(r.recette.montant) },
                { titre: t("depense.compte"), valeur: (r) => r.compte },
                { titre: t("depense.etat"), valeur: (r) => (r.recette.annulee ? t("recette.annulee", { motif: r.recette.motif_annulation }) : t("recette.recue")) },
              ]}
            />
            {recette === undefined || recette.recette.annulee ? null : <Annulation key={recette.recette.id} r={recette} surFait={apres} />}
          </Section>
        </>
      )}
    </div>
  );
}

function NouvelleRecette({ e, surFait }: { e: Etat; surFait: (e: Etat) => void }) {
  const caisse = e.comptes.find((c) => c.nature === "caisse");
  const parDefaut = e.caisse_ouverte ? caisse : e.comptes.find((c) => c.nature !== "caisse") ?? caisse;
  const [date, setDate] = useState(aujourdHui());
  const [categorie, setCategorie] = useState(e.categories[0]?.id ?? "");
  const [libelle, setLibelle] = useState("");
  const [payeur, setPayeur] = useState("");
  const [montant, setMontant] = useState("");
  const [compteId, setCompteId] = useState(parDefaut?.id ?? "");
  const [mode, setMode] = useState<ModePaiement>(modesDu(parDefaut)[0] ?? "especes");
  const [reference, setReference] = useState("");
  const lu = lireMontant(montant);
  const saisir = useMutation({
    mutationFn: () => appeler("recette_saisir", { date, categorie_id: categorie, libelle, payeur, montant: lu ?? 0, compte_id: compteId, mode, reference }),
    onSuccess: (x) => {
      setLibelle("");
      setPayeur("");
      setMontant("");
      setReference("");
      surFait(x);
    },
  });
  return (
    <Section titre={t("recette.nouvelle")}>
      <p className="text-sm text-muet">{t("recette.aide")}</p>
      <Formulaire surEnvoi={() => saisir.mutate()}>
        <div className="flex flex-wrap items-end gap-3">
          <Champ id="recette-date" type="date" libelle={t("depense.date")} value={date} onChange={(ev) => setDate(ev.target.value)} />
          <Selection id="recette-categorie" libelle={t("depense.categorie")} valeur={categorie} surChangement={setCategorie} options={e.categories.map((c) => ({ valeur: c.id, libelle: c.libelle }))} />
          <Champ id="recette-libelle" libelle={t("depense.libelle")} value={libelle} onChange={(ev) => setLibelle(ev.target.value)} autoComplete="off" />
          <Champ id="recette-payeur" libelle={t("recette.payeur")} value={payeur} onChange={(ev) => setPayeur(ev.target.value)} autoComplete="off" />
          <Champ id="recette-montant" libelle={t("paie.mois.montant")} value={montant} onChange={(ev) => setMontant(ev.target.value)} autoComplete="off" />
        </div>
        <ChoixCompte
          prefixe="recette"
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
        <div>
          <Bouton type="submit" disabled={libelle.trim() === "" || payeur.trim() === "" || lu === null || categorie === "" || saisir.isPending}>
            {t("recette.enregistrer")}
          </Bouton>
        </div>
        <MessageErreur erreur={saisir.error} />
      </Formulaire>
    </Section>
  );
}

function Annulation({ r, surFait }: { r: RecetteAffichee; surFait: (e: Etat) => void }) {
  const [motif, setMotif] = useState("");
  const annuler = useMutation({
    mutationFn: () => appeler("recette_annuler", { id: r.recette.id, motif }),
    onSuccess: surFait,
  });
  return (
    <div className="flex flex-wrap items-end gap-3 rounded border border-muet p-3">
      <p className="text-sm font-medium text-encre">
        {`${dateCivile(r.recette.date)} · ${r.recette.libelle} · ${r.recette.payeur} · ${francs(r.recette.montant)}`}
      </p>
      <Champ id="recette-annulation" libelle={t("depense.motif_annulation")} value={motif} onChange={(ev) => setMotif(ev.target.value)} autoComplete="off" />
      <Bouton variante="danger" onClick={() => annuler.mutate()} disabled={motif.trim() === "" || annuler.isPending}>
        {t("recette.annuler")}
      </Bouton>
      <MessageErreur erreur={annuler.error} />
    </div>
  );
}
