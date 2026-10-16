import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { Bouton } from "../../composants/Bouton";
import { Champ, Formulaire, Selection } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { t } from "../../i18n";
import type { Cycle, PhraseAffichee, StatutEtablissement } from "../../types/gesco";
import { ChoixCycles } from "../scolarite/Cycles";
import { PhraseSecours } from "./PhraseSecours";
import { Restauration } from "./Restauration";

const STATUTS: readonly StatutEtablissement[] = ["prive", "public", "communautaire", "medersa"];
const LIBELLES_STATUT = {
  public: "demarrage.statut.public",
  prive: "demarrage.statut.prive",
  communautaire: "demarrage.statut.communautaire",
  medersa: "demarrage.statut.medersa",
} as const;

export function PremierDemarrage() {
  const client = useQueryClient();
  const [mode, setMode] = useState<"creer" | "restaurer">("creer");
  const [phrase, setPhrase] = useState<PhraseAffichee | null>(null);
  const [cycles, setCycles] = useState<Cycle[]>([]);
  const [champs, setChamps] = useState({
    organisation: "",
    etablissement_nom: "",
    statut: "prive" as StatutEtablissement,
    code_academie: "",
    code_cap: "",
    adresse: "",
    telephone: "",
    administrateur_identifiant: "",
    administrateur_nom: "",
    administrateur_mot_de_passe: "",
  });
  const creation = useMutation({
    mutationFn: () => {
      const { telephone, ...reste } = champs;
      return appeler("premier_demarrage", {
        ...reste,
        telephones: telephone.trim() === "" ? [] : [telephone.trim()],
        cycles,
      });
    },
    onSuccess: (r) => setPhrase(r.phrase),
  });
  const terminer = () => void client.invalidateQueries({ queryKey: ["etat"] });

  if (phrase !== null) {
    return (
      <div className="mx-auto mt-8 max-w-3xl">
        <PhraseSecours phrase={phrase} surTermine={terminer} />
      </div>
    );
  }
  const champ = (cle: keyof typeof champs, libelle: string, type = "text") => (
    <Champ
      id={cle}
      libelle={libelle}
      type={type}
      value={champs[cle]}
      onChange={(e) => setChamps({ ...champs, [cle]: e.target.value })}
    />
  );
  return (
    <div className="mx-auto mt-8 flex max-w-2xl flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("demarrage.titre")}</h1>
      <p className="text-sm text-encre">{t("demarrage.explication")}</p>
      <div className="flex gap-2">
        <Bouton variante={mode === "creer" ? "principal" : "secondaire"} onClick={() => setMode("creer")}>{t("demarrage.creer")}</Bouton>
        <Bouton variante={mode === "restaurer" ? "principal" : "secondaire"} onClick={() => setMode("restaurer")}>{t("demarrage.restaurer")}</Bouton>
      </div>
      {mode === "restaurer" ? (
        <Restauration commande="sauvegarde_restaurer_initiale" surTermine={terminer} />
      ) : (
        <Section titre={t("demarrage.creer")}>
          <Formulaire surEnvoi={() => creation.mutate()}>
            {champ("organisation", t("demarrage.organisation"))}
            {champ("etablissement_nom", t("demarrage.etablissement_nom"))}
            <Selection
              id="statut"
              libelle={t("demarrage.statut")}
              valeur={champs.statut}
              surChangement={(v) => {
                const statut = STATUTS.find((s) => s === v);
                if (statut !== undefined) setChamps({ ...champs, statut });
              }}
              options={STATUTS.map((s) => ({ valeur: s, libelle: t(LIBELLES_STATUT[s]) }))}
            />
            {champ("code_academie", t("demarrage.code_academie"))}
            {champ("code_cap", t("demarrage.code_cap"))}
            {champ("adresse", t("demarrage.adresse"))}
            {champ("telephone", t("demarrage.telephone"), "tel")}
            <ChoixCycles choisis={cycles} surChangement={setCycles} />
            <h3 className="text-sm font-medium text-encre">{t("demarrage.administrateur")}</h3>
            {champ("administrateur_identifiant", t("demarrage.administrateur_identifiant"))}
            {champ("administrateur_nom", t("demarrage.administrateur_nom"))}
            {champ("administrateur_mot_de_passe", t("demarrage.administrateur_mot_de_passe"), "password")}
            <MessageErreur erreur={creation.error} />
            <div>
              <Bouton type="submit" disabled={creation.isPending}>{t("demarrage.creer")}</Bouton>
            </div>
          </Formulaire>
        </Section>
      )}
    </div>
  );
}
