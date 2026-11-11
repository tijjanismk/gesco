import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";

import { fermerQuandMeme, quitterPrincipal, surFermetureDemandee } from "../api/poste";
import { Bouton } from "../composants/Bouton";
import { MessageErreur } from "../composants/Messages";
import { t } from "../i18n";
import { usePoste } from "./poste";
import { useSession } from "./session";

const ETATS = {
  etablie: "bandeau_poste.etablie",
  coupee: "bandeau_poste.coupee",
  refusee: "bandeau_poste.refusee",
} as const;

/** Bandeau d'un poste relié : son nom, le poste principal, et l'état de la liaison. Rien sur un
 *  poste qui a sa base. */
export function BandeauPoste() {
  const { info, liaison } = usePoste();
  if (info === null || !info.relie) return null;
  return (
    <div
      role="status"
      className={`flex flex-wrap items-center gap-x-4 gap-y-1 px-4 py-1.5 text-sm ${liaison === "etablie" ? "bg-marine text-sur-marine" : "bg-rouge text-surface"}`}
    >
      <p className="font-semibold">{t("bandeau_poste.nom", { nom: info.nom })}</p>
      <p>{t("bandeau_poste.principal", { adresse: info.adresse ?? "" })}</p>
      <p className="ml-auto font-medium">{t(ETATS[liaison])}</p>
    </div>
  );
}

/** « Quitter le poste principal » : efface la liaison et ramène le poste au premier lancement.
 *  Demande confirmation ; ne touche à aucune donnée de l'école. */
export function QuitterPoste() {
  const client = useQueryClient();
  const session = useSession();
  const [aConfirmer, setAConfirmer] = useState(false);
  const depart = useMutation({
    mutationFn: quitterPrincipal,
    onSuccess: (poste) => {
      session.fermer();
      client.setQueryData(["poste"], poste);
      void client.invalidateQueries({ queryKey: ["etat"] });
    },
  });
  if (!aConfirmer) {
    return (
      <button type="button" className="self-start text-sm text-primaire underline" onClick={() => setAConfirmer(true)}>
        {t("quitter_poste.lien")}
      </button>
    );
  }
  return (
    <div className="flex flex-col gap-2 rounded border border-muet/30 bg-surface p-3">
      <p className="text-sm text-encre">{t("quitter_poste.explication")}</p>
      <MessageErreur erreur={depart.error} />
      <div className="flex flex-wrap gap-2">
        <Bouton variante="danger" disabled={depart.isPending} onClick={() => depart.mutate()}>{t("quitter_poste.confirmer")}</Bouton>
        <Bouton variante="secondaire" onClick={() => setAConfirmer(false)}>{t("commun.annuler")}</Bouton>
      </div>
    </div>
  );
}

/** Écran d'un poste relié qui n'obtient rien du poste principal : éteint, d'une autre version,
 *  ou qui ne le reconnaît plus. Le poste réessaie de lui-même tant que la liaison est coupée. */
export function LiaisonEnPanne({ erreur, surReessayer }: { erreur: unknown; surReessayer: () => void }) {
  const { liaison } = usePoste();
  return (
    <div className="mx-auto mt-16 flex max-w-xl flex-col gap-4 rounded border border-muet/30 bg-surface p-6">
      <h1 className="text-xl font-semibold text-encre">{t("liaison.panne_titre")}</h1>
      <MessageErreur erreur={erreur} />
      {liaison === "coupee" ? <p className="text-sm text-muet">{t("liaison.reprise")}</p> : null}
      <div>
        <Bouton onClick={surReessayer}>{t("liaison.reessayer")}</Bouton>
      </div>
      <QuitterPoste />
    </div>
  );
}

/** Sur le poste principal : fermer GESCO alors que des postes reliés ont une session ouverte
 *  coupe leur travail. La fenêtre reste ouverte et demande confirmation. */
export function AvertissementFermeture() {
  const [postes, setPostes] = useState<number | null>(null);
  useEffect(() => surFermetureDemandee(setPostes), []);
  if (postes === null) return null;
  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center bg-encre/40 p-6">
      <div role="alertdialog" aria-modal="true" aria-labelledby="fermeture-titre" className="mt-24 flex max-w-lg flex-col gap-3 rounded border border-muet/30 bg-surface p-6 shadow-lg">
        <h2 id="fermeture-titre" className="text-lg font-semibold text-encre">{t("fermeture.titre")}</h2>
        <p className="text-sm text-encre">
          {postes === 1 ? t("fermeture.un_poste") : t("fermeture.des_postes", { nombre: postes })}
        </p>
        <p className="text-sm text-muet">{t("fermeture.aide")}</p>
        <div className="flex flex-wrap gap-2">
          <Bouton variante="secondaire" autoFocus onClick={() => setPostes(null)}>{t("fermeture.rester")}</Bouton>
          <Bouton variante="danger" onClick={() => void fermerQuandMeme()}>{t("fermeture.fermer")}</Bouton>
        </div>
      </div>
    </div>
  );
}
