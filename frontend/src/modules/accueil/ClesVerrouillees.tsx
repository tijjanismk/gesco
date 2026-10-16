import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { Bouton } from "../../composants/Bouton";
import { Formulaire, ZoneTexte } from "../../composants/Champ";
import { MessageErreur } from "../../composants/Messages";
import { t } from "../../i18n";

/** Clé du poste introuvable : la phrase du certificat rouvre les données (§6.3). */
export function ClesVerrouillees() {
  const client = useQueryClient();
  const [phrase, setPhrase] = useState("");
  const ouverture = useMutation({
    mutationFn: () => appeler("cles_deverrouiller_phrase", { phrase }),
    onSuccess: (etat) => {
      setPhrase("");
      client.setQueryData(["etat"], etat);
    },
  });
  return (
    <div className="mx-auto mt-24 flex max-w-xl flex-col gap-4 rounded border border-muet/30 bg-surface p-6">
      <h1 className="text-xl font-semibold text-encre">{t("cles.titre")}</h1>
      <p className="text-sm text-encre">{t("cles.explication")}</p>
      <Formulaire surEnvoi={() => ouverture.mutate()}>
        <ZoneTexte id="phrase" libelle={t("phrase.saisie")} aide={t("phrase.saisie_aide")} valeur={phrase} surChangement={setPhrase} />
        <MessageErreur erreur={ouverture.error} />
        <div>
          <Bouton type="submit" disabled={ouverture.isPending}>{t("cles.ouvrir")}</Bouton>
        </div>
      </Formulaire>
    </div>
  );
}
