import { useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";

import { appeler } from "../api";
import { MessageErreur } from "../composants/Messages";
import { t } from "../i18n";
import { ClesVerrouillees } from "../modules/accueil/ClesVerrouillees";
import { Connexion } from "../modules/accueil/Connexion";
import { EcranVerrouille } from "../modules/accueil/EcranVerrouille";
import { PremierDemarrage } from "../modules/accueil/PremierDemarrage";
import type { EtatDemarrage } from "../types/gesco";
import { Espace } from "./Espace";
import { surRefusDeSession } from "./requetes";
import { useSession } from "./session";

/** Aiguillage des écrans selon l'état du poste et de la session. */
export function Application() {
  const session = useSession();
  const etat = useQuery({ queryKey: ["etat"], queryFn: () => appeler("etat_demarrage", null) });
  const [autreUtilisateur, setAutreUtilisateur] = useState(false);
  const { verrouiller, fermer } = session;

  useEffect(() => {
    surRefusDeSession((cle) => (cle === "session.verrouillee" ? verrouiller() : fermer()));
    return () => surRefusDeSession(null);
  }, [verrouiller, fermer]);

  if (etat.isPending) return <p className="p-6 text-muet">{t("commun.chargement")}</p>;
  if (etat.isError) return <div className="p-6"><MessageErreur erreur={etat.error} /></div>;
  return (
    <Ecran
      etat={etat.data}
      autreUtilisateur={autreUtilisateur}
      surAutreUtilisateur={setAutreUtilisateur}
    />
  );
}

function Ecran({
  etat,
  autreUtilisateur,
  surAutreUtilisateur,
}: {
  etat: EtatDemarrage;
  autreUtilisateur: boolean;
  surAutreUtilisateur: (oui: boolean) => void;
}) {
  const session = useSession();
  switch (etat.phase) {
    case "cles_verrouillees":
      return <ClesVerrouillees />;
    case "premier_demarrage":
      return <PremierDemarrage />;
    case "pret":
      if (session.jeton !== null && session.verrouillee && !autreUtilisateur) {
        return <EcranVerrouille surAutreUtilisateur={() => surAutreUtilisateur(true)} />;
      }
      if (session.jeton === null || autreUtilisateur) {
        return (
          <Connexion
            etablissement={etat.etablissement?.nom ?? ""}
            version={etat.version_applicative}
            surConnexion={() => surAutreUtilisateur(false)}
          />
        );
      }
      return <Espace etablissement={etat.etablissement?.nom ?? ""} />;
  }
}
