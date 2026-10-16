import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { Bouton } from "../../composants/Bouton";
import { Champ, Formulaire } from "../../composants/Champ";
import { MessageEnregistre, MessageErreur, Section } from "../../composants/Messages";
import { Tableau } from "../../composants/Tableau";
import { type Cle, date, t } from "../../i18n";
import type { AnneeScolaire, StatutAnnee } from "../../types/gesco";

const STATUTS: Record<StatutAnnee, Cle> = {
  preparation: "annees.statut.preparation",
  active: "annees.statut.active",
  cloturee: "annees.statut.cloturee",
  archivee: "annees.statut.archivee",
};

/** Jour saisi (`AAAA-MM-JJ`) vers minuit UTC, l'heure du Mali. */
function versMillis(jour: string): number {
  const [a, m, j] = jour.split("-").map(Number);
  return Date.UTC(a ?? 0, (m ?? 1) - 1, j ?? 1);
}

function versJour(millis: number): string {
  return new Date(millis).toISOString().slice(0, 10);
}

function useRafraichir() {
  const client = useQueryClient();
  return () => {
    for (const cle of ["annees", "session", "avertissements"]) void client.invalidateQueries({ queryKey: [cle] });
  };
}

/** Années scolaires (SPEC_PHASE_1 §3) : préparer, corriger, ouvrir. La clôture viendra en phase 10. */
export function Annees() {
  const annees = useQuery({ queryKey: ["annees"], queryFn: () => appeler("annees_lister", null) });
  const [choisie, setChoisie] = useState<string | null>(null);
  const annee = annees.data?.find((a) => a.id === choisie);
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("annees.titre")}</h1>
      <p className="text-sm text-muet">{t("annees.explication")}</p>
      <MessageErreur erreur={annees.error} />
      <div className="flex flex-wrap items-start gap-4">
        <div className="min-w-0 flex-1">
          <Tableau
            nomExport="annees"
            titre={t("annees.titre")}
            lignes={annees.data ?? []}
            cle={(a) => a.id}
            surSelection={(a) => setChoisie(a.id)}
            colonnes={[
              { titre: t("annees.libelle"), valeur: (a) => a.libelle },
              { titre: t("annees.debut"), valeur: (a) => date(a.debut) },
              { titre: t("annees.fin"), valeur: (a) => date(a.fin) },
              { titre: t("annees.statut"), valeur: (a) => t(STATUTS[a.statut]) },
              { titre: t("annees.courante"), valeur: (a) => (a.est_courante ? t("commun.oui") : "") },
            ]}
          />
        </div>
        <aside className="flex w-80 flex-col gap-4">
          {annee === undefined ? null : <FicheAnnee key={`${annee.id}-${annee.statut}`} annee={annee} surSupprimee={() => setChoisie(null)} />}
          <NouvelleAnnee />
        </aside>
      </div>
    </div>
  );
}

function ChampsAnnee({
  prefixe,
  valeurs,
  surChangement,
}: {
  prefixe: string;
  valeurs: { libelle: string; debut: string; fin: string };
  surChangement: (valeurs: { libelle: string; debut: string; fin: string }) => void;
}) {
  return (
    <>
      <Champ
        id={`${prefixe}-libelle`}
        libelle={t("annees.libelle")}
        aide={t("annees.libelle_aide")}
        value={valeurs.libelle}
        onChange={(e) => surChangement({ ...valeurs, libelle: e.target.value })}
      />
      <Champ id={`${prefixe}-debut`} type="date" libelle={t("annees.debut")} value={valeurs.debut} onChange={(e) => surChangement({ ...valeurs, debut: e.target.value })} />
      <Champ id={`${prefixe}-fin`} type="date" libelle={t("annees.fin")} value={valeurs.fin} onChange={(e) => surChangement({ ...valeurs, fin: e.target.value })} />
    </>
  );
}

function NouvelleAnnee() {
  const rafraichir = useRafraichir();
  const vide = { libelle: "", debut: "", fin: "" };
  const [valeurs, setValeurs] = useState(vide);
  const creation = useMutation({
    mutationFn: () =>
      appeler("annee_creer", { libelle: valeurs.libelle, debut: versMillis(valeurs.debut), fin: versMillis(valeurs.fin) }),
    onSuccess: () => {
      setValeurs(vide);
      rafraichir();
    },
  });
  return (
    <Section titre={t("annees.nouvelle")}>
      <Formulaire surEnvoi={() => creation.mutate()}>
        <ChampsAnnee prefixe="nouvelle-annee" valeurs={valeurs} surChangement={setValeurs} />
        <MessageErreur erreur={creation.error} />
        <div><Bouton type="submit" disabled={creation.isPending}>{t("annees.creer")}</Bouton></div>
      </Formulaire>
    </Section>
  );
}

function FicheAnnee({ annee, surSupprimee }: { annee: AnneeScolaire; surSupprimee: () => void }) {
  const rafraichir = useRafraichir();
  const [valeurs, setValeurs] = useState({ libelle: annee.libelle, debut: versJour(annee.debut), fin: versJour(annee.fin) });
  const [enregistre, setEnregistre] = useState<number | null>(null);
  const modification = useMutation({
    mutationFn: () =>
      appeler("annee_modifier", { id: annee.id, libelle: valeurs.libelle, debut: versMillis(valeurs.debut), fin: versMillis(valeurs.fin) }),
    onSuccess: () => {
      setEnregistre(Date.now());
      rafraichir();
    },
  });
  const activation = useMutation({ mutationFn: () => appeler("annee_activer", { id: annee.id }), onSuccess: rafraichir });
  const suppression = useMutation({
    mutationFn: () => appeler("annee_supprimer", { id: annee.id }),
    onSuccess: () => {
      rafraichir();
      surSupprimee();
    },
  });
  if (annee.statut !== "preparation") {
    return (
      <Section titre={annee.libelle}>
        <p className="text-sm text-muet">{t("annees.figee")}</p>
      </Section>
    );
  }
  return (
    <Section titre={annee.libelle}>
      <Formulaire surEnvoi={() => modification.mutate()}>
        <ChampsAnnee prefixe="annee" valeurs={valeurs} surChangement={setValeurs} />
        <div><Bouton type="submit" variante="secondaire" disabled={modification.isPending}>{t("commun.enregistrer")}</Bouton></div>
      </Formulaire>
      <p className="text-sm text-ocre">{t("annees.ouvrir_aide")}</p>
      <div className="flex flex-wrap gap-2">
        <Bouton onClick={() => activation.mutate()} disabled={activation.isPending}>{t("annees.ouvrir")}</Bouton>
        <Bouton variante="danger" onClick={() => suppression.mutate()} disabled={suppression.isPending}>{t("annees.supprimer")}</Bouton>
      </div>
      <MessageErreur erreur={modification.error ?? activation.error ?? suppression.error} />
      <MessageEnregistre instant={enregistre} />
    </Section>
  );
}
