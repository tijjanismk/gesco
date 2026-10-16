import { useMutation } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { useSession } from "../../app/session";
import { Bouton } from "../../composants/Bouton";
import { Champ, Formulaire } from "../../composants/Champ";
import { MessageErreur } from "../../composants/Messages";
import { t } from "../../i18n";

export function Connexion({
  etablissement,
  version,
  surConnexion,
}: {
  etablissement: string;
  version: string;
  surConnexion: () => void;
}) {
  const session = useSession();
  const [identifiant, setIdentifiant] = useState("");
  const [motDePasse, setMotDePasse] = useState("");
  const connexion = useMutation({
    mutationFn: () => appeler("connexion", { identifiant, mot_de_passe: motDePasse }),
    onSuccess: (ouverte) => {
      setMotDePasse("");
      session.ouvrir(ouverte.jeton);
      surConnexion();
    },
  });
  return (
    <div className="mx-auto mt-24 flex max-w-sm flex-col gap-4 rounded border border-muet/30 bg-surface p-6">
      <h1 className="text-xl font-semibold text-encre">{t("connexion.titre")}</h1>
      <p className="text-sm text-muet">{etablissement}</p>
      <Formulaire surEnvoi={() => connexion.mutate()}>
        <Champ id="identifiant" libelle={t("connexion.identifiant")} value={identifiant} autoComplete="username" onChange={(e) => setIdentifiant(e.target.value)} />
        <Champ id="mot-de-passe" libelle={t("connexion.mot_de_passe")} type="password" value={motDePasse} autoComplete="current-password" onChange={(e) => setMotDePasse(e.target.value)} />
        <MessageErreur erreur={connexion.error} />
        <Bouton type="submit" disabled={connexion.isPending}>{t("connexion.valider")}</Bouton>
      </Formulaire>
      <p className="text-xs text-muet">{t("app.version", { version })}</p>
    </div>
  );
}
