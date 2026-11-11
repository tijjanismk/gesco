import { useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";

import { ErreurApi, appeler } from "../api";
import { MessageErreur } from "../composants/Messages";
import { t } from "../i18n";
import { ClesVerrouillees } from "../modules/accueil/ClesVerrouillees";
import { Connexion } from "../modules/accueil/Connexion";
import { EcranVerrouille } from "../modules/accueil/EcranVerrouille";
import { PremierDemarrage } from "../modules/accueil/PremierDemarrage";
import type { EtatDemarrage } from "../types/gesco";
import { AvertissementFermeture, BandeauPoste, LiaisonEnPanne, QuitterPoste } from "./BandeauPoste";
import { Espace } from "./Espace";
import { usePoste } from "./poste";
import { surRefusDeSession } from "./requetes";
import { useSession } from "./session";

/** Aiguillage des écrans selon l'état du poste et de la session. */
export function Application() {
  const session = useSession();
  const poste = usePoste();
  const etat = useQuery({ queryKey: ["etat"], queryFn: () => appeler("etat_demarrage", null) });
  const [autreUtilisateur, setAutreUtilisateur] = useState(false);
  const { verrouiller, fermer } = session;

  useEffect(() => {
    surRefusDeSession((cle) => (cle === "session.verrouillee" ? verrouiller() : fermer()));
    return () => surRefusDeSession(null);
  }, [verrouiller, fermer]);

  if (etat.isPending || poste.info === null) return <p className="p-6 text-muet">{t("commun.chargement")}</p>;
  // Une relecture qui échoue parce que la liaison est coupée laisse l'écran en place, avec ce
  // qui y est saisi : le bandeau dit la coupure, et l'état se relit au retour de la liaison.
  const coupee =
    etat.data !== undefined && etat.error instanceof ErreurApi && etat.error.cle === "reseau.poste_principal_injoignable";
  // Un poste relié qui n'obtient rien du poste principal, ou qu'il refuse, le dit, garde son
  // bandeau et réessaie.
  if (etat.isError && !coupee && poste.info.relie) {
    return (
      <>
        <BandeauPoste />
        <LiaisonEnPanne erreur={etat.error} surReessayer={() => void etat.refetch()} />
      </>
    );
  }
  if ((etat.isError && !coupee) || etat.data === undefined) return <div className="p-6"><MessageErreur erreur={etat.error} /></div>;
  return (
    <>
      <Ecran
        etat={etat.data}
        relie={poste.info.relie}
        autreUtilisateur={autreUtilisateur}
        surAutreUtilisateur={setAutreUtilisateur}
      />
      <AvertissementFermeture />
    </>
  );
}

function Ecran({
  etat,
  relie,
  autreUtilisateur,
  surAutreUtilisateur,
}: {
  etat: EtatDemarrage;
  relie: boolean;
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
        return (
          <>
            <BandeauPoste />
            <EcranVerrouille surAutreUtilisateur={() => surAutreUtilisateur(true)} />
          </>
        );
      }
      if (session.jeton === null || autreUtilisateur) {
        return (
          <>
            <BandeauPoste />
            <Connexion
              etablissement={etat.etablissement?.nom ?? ""}
              version={etat.version_applicative}
              surConnexion={() => surAutreUtilisateur(false)}
            />
            {relie ? <div className="mx-auto mt-4 flex max-w-sm flex-col"><QuitterPoste /></div> : null}
          </>
        );
      }
      return <Espace etablissement={etat.etablissement?.nom ?? ""} />;
  }
}
