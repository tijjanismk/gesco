import { useMutation, useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { useSession } from "../../app/session";
import { Bouton } from "../../composants/Bouton";
import { Champ, Formulaire } from "../../composants/Champ";
import { MessageErreur } from "../../composants/Messages";
import { t } from "../../i18n";

/** Écran verrouillé et bascule d'utilisateur (§6.2, RA-13) : choisir sa session, taper son PIN,
 *  ou son mot de passe s'il n'a pas encore de PIN (SPEC_PHASE_1 §7). */
export function EcranVerrouille({ surAutreUtilisateur }: { surAutreUtilisateur: () => void }) {
  const session = useSession();
  const sessions = useQuery({ queryKey: ["sessions-poste"], queryFn: () => appeler("sessions_du_poste", null) });
  const [choisie, setChoisie] = useState<string | null>(session.jeton);
  const [secret, setSecret] = useState("");
  const avecPin = sessions.data?.find((s) => s.jeton === choisie)?.pin_defini ?? true;
  const deverrouillage = useMutation({
    mutationFn: (jeton: string) =>
      avecPin
        ? appeler("deverrouiller_pin", { jeton, pin: secret })
        : appeler("deverrouiller_mot_de_passe", { jeton, mot_de_passe: secret }),
    onSuccess: (ouverte) => {
      setSecret("");
      session.ouvrir(ouverte.jeton);
    },
  });
  return (
    <div className="mx-auto mt-24 flex max-w-md flex-col gap-4 rounded border border-muet/30 bg-surface p-6">
      <h1 className="text-xl font-semibold text-encre">{t("verrou.titre")}</h1>
      <p className="text-sm text-muet">{t("verrou.choisir")}</p>
      <ul className="flex flex-col gap-1">
        {(sessions.data ?? []).map((s) => (
          <li key={s.jeton}>
            <button
              type="button"
              onClick={() => {
                setChoisie(s.jeton);
                setSecret("");
              }}
              className={`w-full rounded border px-3 py-2 text-left text-sm ${choisie === s.jeton ? "border-primaire text-primaire" : "border-muet/40 text-encre"}`}
            >
              {s.nom_affiche}
              <span className="ml-2 text-muet">
                {s.verrouillee ? t("verrou.session_verrouillee") : t("verrou.session_active")}
              </span>
            </button>
          </li>
        ))}
      </ul>
      <Formulaire surEnvoi={() => choisie !== null && deverrouillage.mutate(choisie)}>
        {avecPin ? (
          <Champ id="pin" libelle={t("verrou.pin")} type="password" inputMode="numeric" autoComplete="off" value={secret} onChange={(e) => setSecret(e.target.value)} />
        ) : (
          <Champ
            id="mot-de-passe-verrou"
            libelle={t("verrou.mot_de_passe")}
            aide={t("verrou.sans_pin")}
            type="password"
            autoComplete="current-password"
            value={secret}
            onChange={(e) => setSecret(e.target.value)}
          />
        )}
        <MessageErreur erreur={deverrouillage.error} />
        <div className="flex gap-2">
          <Bouton type="submit" disabled={choisie === null || deverrouillage.isPending}>{t("verrou.deverrouiller")}</Bouton>
          <Bouton variante="secondaire" onClick={surAutreUtilisateur}>{t("verrou.autre_utilisateur")}</Bouton>
        </div>
      </Formulaire>
    </div>
  );
}
