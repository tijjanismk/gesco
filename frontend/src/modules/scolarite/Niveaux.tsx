import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { Bouton } from "../../composants/Bouton";
import { CaseACocher, Champ, Formulaire } from "../../composants/Champ";
import { MessageEnregistre, MessageErreur, Section } from "../../composants/Messages";
import { Tableau } from "../../composants/Tableau";
import { t } from "../../i18n";
import type { Cycle, Niveau, Serie } from "../../types/gesco";
import { ChoixCycles, CYCLES } from "./Cycles";

type Choix = { nature: "niveau"; id: string } | { nature: "serie"; id: string };

/** Niveaux et séries livrés (SPEC_PHASE_2 §2) : l'école les renomme ou les désactive ; elle crée
 *  les filières de son enseignement technique (SPEC_PHASE_6 §9). */
export function Niveaux() {
  const referentiel = useQuery({ queryKey: ["referentiel"], queryFn: () => appeler("niveaux_lister", null) });
  const [choix, setChoix] = useState<Choix | null>(null);
  const niveaux = referentiel.data?.niveaux ?? [];
  const series = referentiel.data?.series ?? [];
  const niveau = choix?.nature === "niveau" ? niveaux.find((n) => n.id === choix.id) : undefined;
  const serie = choix?.nature === "serie" ? series.find((s) => s.id === choix.id) : undefined;
  const libelleNiveau = (id: string) => niveaux.find((n) => n.id === id)?.libelle ?? "";
  const techniques = niveaux.filter((n) => n.actif && n.cycle === "technique");
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("niveaux.titre")}</h1>
      <p className="text-sm text-muet">{t("niveaux.explication")}</p>
      <MessageErreur erreur={referentiel.error} />
      {referentiel.data === undefined ? null : (
        <CyclesEnseignes key={niveaux.map((x) => `${x.id}${String(x.actif)}`).join()} niveaux={niveaux} />
      )}
      {techniques.length === 0 ? null : <NouvelleFiliere niveaux={techniques} />}
      <div className="flex flex-wrap items-start gap-4">
        <div className="flex min-w-0 flex-1 flex-col gap-4">
          <Tableau
            nomExport="niveaux"
            titre={t("niveaux.titre")}
            lignes={niveaux}
            cle={(n) => n.id}
            surSelection={(n) => setChoix({ nature: "niveau", id: n.id })}
            colonnes={[
              { titre: t("niveaux.code"), valeur: (n) => n.code },
              { titre: t("niveaux.libelle"), valeur: (n) => n.libelle },
              { titre: t("niveaux.cycle"), valeur: (n) => t(CYCLES[n.cycle]) },
              { titre: t("niveaux.etat"), valeur: (n) => t(n.actif ? "niveaux.actif" : "niveaux.inactif") },
            ]}
          />
          <h2 className="text-base font-semibold text-encre">{t("niveaux.series")}</h2>
          <Tableau
            nomExport="series"
            titre={t("niveaux.series")}
            lignes={series}
            cle={(s) => s.id}
            surSelection={(s) => setChoix({ nature: "serie", id: s.id })}
            colonnes={[
              { titre: t("niveaux.niveau"), valeur: (s) => libelleNiveau(s.niveau_id) },
              { titre: t("niveaux.code"), valeur: (s) => s.code },
              { titre: t("niveaux.libelle"), valeur: (s) => s.libelle },
              { titre: t("niveaux.etat"), valeur: (s) => t(s.actif ? "niveaux.actif" : "niveaux.inactif") },
            ]}
          />
        </div>
        <aside className="w-80">
          {niveau === undefined ? null : <Edition key={niveau.id} element={niveau} nature="niveau" />}
          {serie === undefined ? null : <Edition key={serie.id} element={serie} nature="serie" />}
        </aside>
      </div>
    </div>
  );
}

function CyclesEnseignes({ niveaux }: { niveaux: readonly Niveau[] }) {
  const client = useQueryClient();
  const [cycles, setCycles] = useState<Cycle[]>(() => [...new Set(niveaux.filter((x) => x.actif).map((x) => x.cycle))]);
  const enregistrement = useMutation({
    mutationFn: () => appeler("cycles_definir", { cycles }),
    onSuccess: () => void client.invalidateQueries({ queryKey: ["referentiel"] }),
  });
  return (
    <Section titre={t("niveaux.cycles_enseignes")}>
      <p className="text-sm text-muet">{t("niveaux.cycles_aide")}</p>
      <Formulaire surEnvoi={() => enregistrement.mutate()}>
        <ChoixCycles choisis={cycles} surChangement={setCycles} />
        <div><Bouton type="submit" disabled={enregistrement.isPending}>{t("commun.enregistrer")}</Bouton></div>
        <MessageErreur erreur={enregistrement.error} />
      </Formulaire>
    </Section>
  );
}

/** Filière de l'enseignement technique : un code, un libellé, les années où elle s'enseigne. Une
 *  classe porte ensuite son année et sa filière (« CAP 1 Électricité »). */
function NouvelleFiliere({ niveaux }: { niveaux: readonly Niveau[] }) {
  const client = useQueryClient();
  const [code, setCode] = useState("");
  const [libelle, setLibelle] = useState("");
  const [annees, setAnnees] = useState<readonly string[]>([]);
  const [enregistre, setEnregistre] = useState<number | null>(null);
  const creation = useMutation({
    mutationFn: () => appeler("filiere_creer", { code, libelle, niveaux: [...annees] }),
    onSuccess: () => {
      setCode("");
      setLibelle("");
      setAnnees([]);
      setEnregistre(Date.now());
      // Les programmes des années ouvertes reçoivent la filière.
      void client.invalidateQueries();
    },
  });
  return (
    <Section titre={t("niveaux.filieres")}>
      <p className="text-sm text-muet">{t("niveaux.filieres_aide")}</p>
      <Formulaire surEnvoi={() => creation.mutate()}>
        <div className="flex flex-wrap items-end gap-3">
          <div className="w-40">
            <Champ id="filiere-code" libelle={t("niveaux.filiere_code")} value={code} onChange={(e) => setCode(e.target.value.toUpperCase())} />
          </div>
          <div className="w-72">
            <Champ id="filiere-libelle" libelle={t("niveaux.libelle")} value={libelle} onChange={(e) => setLibelle(e.target.value)} />
          </div>
        </div>
        <fieldset className="flex flex-wrap gap-x-4 gap-y-1">
          <legend className="text-sm text-encre">{t("niveaux.filiere_annees")}</legend>
          {niveaux.map((n) => (
            <CaseACocher
              key={n.id}
              id={`filiere-niveau-${n.code}`}
              libelle={n.libelle}
              coche={annees.includes(n.id)}
              surChangement={(coche) => setAnnees((a) => (coche ? [...a, n.id] : a.filter((x) => x !== n.id)))}
            />
          ))}
        </fieldset>
        <div><Bouton type="submit" disabled={creation.isPending}>{t("niveaux.filiere_creer")}</Bouton></div>
        <MessageErreur erreur={creation.error} />
        <MessageEnregistre instant={enregistre} />
      </Formulaire>
    </Section>
  );
}

function Edition({ element, nature }: { element: Niveau | Serie; nature: "niveau" | "serie" }) {
  const client = useQueryClient();
  const [libelle, setLibelle] = useState(element.libelle);
  const [actif, setActif] = useState(element.actif);
  const [enregistre, setEnregistre] = useState<number | null>(null);
  const modification = useMutation({
    mutationFn: async (): Promise<void> => {
      if (nature === "niveau") await appeler("niveau_modifier", { id: element.id, libelle, actif });
      else await appeler("serie_modifier", { id: element.id, libelle, actif });
    },
    onSuccess: () => {
      setEnregistre(Date.now());
      void client.invalidateQueries({ queryKey: ["referentiel"] });
    },
  });
  return (
    <Section titre={t("niveaux.modifier", { code: element.code })}>
      <Formulaire surEnvoi={() => modification.mutate()}>
        <Champ id="referentiel-libelle" libelle={t("niveaux.libelle")} value={libelle} onChange={(e) => setLibelle(e.target.value)} />
        <CaseACocher id="referentiel-actif" libelle={t(nature === "niveau" ? "niveaux.enseigne" : "niveaux.ouverte")} coche={actif} surChangement={setActif} />
        <div><Bouton type="submit" disabled={modification.isPending}>{t("commun.enregistrer")}</Bouton></div>
        <MessageErreur erreur={modification.error} />
        <MessageEnregistre instant={enregistre} />
      </Formulaire>
    </Section>
  );
}
