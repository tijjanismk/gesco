import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { commandeAutorisee } from "../../app/navigation";
import { Bouton } from "../../composants/Bouton";
import { Champ, Formulaire, Selection } from "../../composants/Champ";
import { MessageEnregistre, MessageErreur, Section } from "../../composants/Messages";
import { Tableau } from "../../composants/Tableau";
import { t } from "../../i18n";
import type { FicheClasse, Referentiel } from "../../types/gesco";

/** Classes d'une année (SPEC_PHASE_2 §3) : aucun effectif maximal, la série suit le niveau. */
export function Classes() {
  const session = useQuery({ queryKey: ["session"], queryFn: () => appeler("session_courante", null) });
  const annees = useQuery({ queryKey: ["annees"], queryFn: () => appeler("annees_lister", null) });
  const referentiel = useQuery({ queryKey: ["referentiel"], queryFn: () => appeler("niveaux_lister", null) });
  const [annee, setAnnee] = useState<string | null>(null);
  const anneeId = annee ?? annees.data?.find((a) => a.est_courante)?.id ?? annees.data?.[0]?.id ?? null;
  const classes = useQuery({
    queryKey: ["classes", anneeId],
    queryFn: () => appeler("classes_lister", { annee_id: anneeId ?? "" }),
    enabled: anneeId !== null,
  });
  const [choisie, setChoisie] = useState<string | null>(null);
  const fiche = classes.data?.find((c) => c.classe.id === choisie);
  const organise = commandeAutorisee("classe_creer", session.data?.permissions ?? []);
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("classes.titre")}</h1>
      <div className="w-60">
        <Selection
          id="classes-annee"
          libelle={t("classes.annee")}
          valeur={anneeId ?? ""}
          surChangement={(v) => {
            setAnnee(v);
            setChoisie(null);
          }}
          options={(annees.data ?? []).map((a) => ({ valeur: a.id, libelle: a.libelle }))}
        />
      </div>
      {anneeId === null ? <p className="text-sm text-ocre">{t("classes.aucune_annee")}</p> : null}
      <MessageErreur erreur={annees.error ?? referentiel.error ?? classes.error} />
      <div className="flex flex-wrap items-start gap-4">
        <div className="min-w-0 flex-1">
          <Tableau
            nomExport="classes"
            titre={t("classes.titre")}
            lignes={classes.data ?? []}
            cle={(c) => c.classe.id}
            surSelection={(c) => setChoisie(c.classe.id)}
            colonnes={[
              { titre: t("classes.libelle"), valeur: (c) => c.classe.libelle },
              { titre: t("classes.niveau"), valeur: (c) => c.niveau },
              { titre: t("classes.serie"), valeur: (c) => c.serie ?? "" },
              { titre: t("classes.effectif"), valeur: (c) => String(c.effectif) },
            ]}
          />
        </div>
        {organise && anneeId !== null && referentiel.data !== undefined ? (
          <aside className="flex w-80 flex-col gap-4">
            {fiche === undefined ? null : <FicheClasseEdition key={fiche.classe.id} fiche={fiche} surSupprimee={() => setChoisie(null)} />}
            <NouvelleClasse anneeId={anneeId} referentiel={referentiel.data} />
          </aside>
        ) : null}
      </div>
    </div>
  );
}

function useRafraichir() {
  const client = useQueryClient();
  return () => void client.invalidateQueries({ queryKey: ["classes"] });
}

function NouvelleClasse({ anneeId, referentiel }: { anneeId: string; referentiel: Referentiel }) {
  const rafraichir = useRafraichir();
  const niveaux = referentiel.niveaux.filter((n) => n.actif);
  const [niveauId, setNiveauId] = useState(niveaux[0]?.id ?? "");
  const [serieId, setSerieId] = useState("");
  const [libelle, setLibelle] = useState("");
  const series = referentiel.series.filter((s) => s.niveau_id === niveauId && s.actif);
  const technique = niveaux.find((n) => n.id === niveauId)?.cycle === "technique";
  const creation = useMutation({
    mutationFn: () =>
      appeler("classe_creer", { annee_id: anneeId, niveau_id: niveauId, serie_id: serieId === "" ? null : serieId, libelle }),
    onSuccess: () => {
      setLibelle("");
      rafraichir();
    },
  });
  return (
    <Section titre={t("classes.nouvelle")}>
      <Formulaire surEnvoi={() => creation.mutate()}>
        <Selection
          id="classe-niveau"
          libelle={t("classes.niveau")}
          valeur={niveauId}
          surChangement={(v) => {
            setNiveauId(v);
            setSerieId("");
          }}
          options={niveaux.map((n) => ({ valeur: n.id, libelle: n.libelle }))}
        />
        {series.length === 0 ? null : (
          <Selection
            id="classe-serie"
            libelle={t(technique ? "classes.filiere" : "classes.serie")}
            valeur={serieId}
            surChangement={setSerieId}
            options={[{ valeur: "", libelle: t("classes.choisir_serie") }, ...series.map((s) => ({ valeur: s.id, libelle: `${s.code} — ${s.libelle}` }))]}
          />
        )}
        <Champ id="classe-libelle" libelle={t("classes.libelle")} aide={t("classes.libelle_aide")} value={libelle} onChange={(e) => setLibelle(e.target.value)} />
        <MessageErreur erreur={creation.error} />
        <div><Bouton type="submit" disabled={creation.isPending}>{t("classes.creer")}</Bouton></div>
      </Formulaire>
    </Section>
  );
}

function FicheClasseEdition({ fiche, surSupprimee }: { fiche: FicheClasse; surSupprimee: () => void }) {
  const rafraichir = useRafraichir();
  const [libelle, setLibelle] = useState(fiche.classe.libelle);
  const [enregistre, setEnregistre] = useState<number | null>(null);
  const modification = useMutation({
    mutationFn: () => appeler("classe_modifier", { id: fiche.classe.id, libelle }),
    onSuccess: () => {
      setEnregistre(Date.now());
      rafraichir();
    },
  });
  const suppression = useMutation({
    mutationFn: () => appeler("classe_supprimer", { id: fiche.classe.id }),
    onSuccess: () => {
      rafraichir();
      surSupprimee();
    },
  });
  return (
    <Section titre={fiche.classe.libelle}>
      <p className="text-sm text-muet">{fiche.serie === null ? fiche.niveau : `${fiche.niveau} · ${fiche.serie}`}</p>
      <Formulaire surEnvoi={() => modification.mutate()}>
        <Champ id="classe-renommer" libelle={t("classes.libelle")} value={libelle} onChange={(e) => setLibelle(e.target.value)} />
        <div className="flex flex-wrap gap-2">
          <Bouton type="submit" variante="secondaire" disabled={modification.isPending}>{t("commun.enregistrer")}</Bouton>
          <Bouton variante="danger" onClick={() => suppression.mutate()} disabled={suppression.isPending}>{t("classes.supprimer")}</Bouton>
        </div>
      </Formulaire>
      <MessageErreur erreur={modification.error ?? suppression.error} />
      <MessageEnregistre instant={enregistre} />
    </Section>
  );
}
