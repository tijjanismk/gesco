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
      <SecoursAdministrateur />
      <p className="text-xs text-muet">{t("app.version", { version })}</p>
    </div>
  );
}

/** L'administrateur qui a perdu son mot de passe le remplace avec les douze mots du certificat :
 *  qui les détient peut déjà tout restaurer. Le remplacement est tracé et signalé. */
function SecoursAdministrateur() {
  const [ouvert, setOuvert] = useState(false);
  const [phrase, setPhrase] = useState("");
  const [identifiant, setIdentifiant] = useState("");
  const [nouveau, setNouveau] = useState("");
  const [fait, setFait] = useState(false);
  const remplacement = useMutation({
    mutationFn: () => appeler("administrateur_reinitialiser_par_phrase", { phrase, identifiant, nouveau_mot_de_passe: nouveau }),
    onSuccess: () => {
      setPhrase("");
      setNouveau("");
      setOuvert(false);
      setFait(true);
    },
  });
  if (!ouvert) {
    return (
      <div className="flex flex-col gap-1">
        {fait ? <p role="status" className="text-sm text-vert">{t("secours.fait")}</p> : null}
        <button type="button" className="self-start text-sm text-primaire underline" onClick={() => { setOuvert(true); setFait(false); }}>
          {t("secours.lien")}
        </button>
      </div>
    );
  }
  return (
    <Formulaire surEnvoi={() => remplacement.mutate()}>
      <h2 className="text-base font-semibold text-encre">{t("secours.titre")}</h2>
      <p className="text-sm text-muet">{t("secours.explication")}</p>
      <Champ id="secours-phrase" libelle={t("phrase.saisie")} value={phrase} autoComplete="off" onChange={(e) => setPhrase(e.target.value)} />
      <Champ id="secours-identifiant" libelle={t("secours.identifiant")} value={identifiant} autoComplete="username" onChange={(e) => setIdentifiant(e.target.value)} />
      <Champ id="secours-nouveau" libelle={t("secours.nouveau")} type="password" value={nouveau} autoComplete="new-password" onChange={(e) => setNouveau(e.target.value)} />
      <MessageErreur erreur={remplacement.error} />
      <div className="flex flex-wrap gap-2">
        <Bouton type="submit" disabled={remplacement.isPending || phrase.trim() === "" || identifiant.trim() === "" || nouveau === ""}>{t("secours.valider")}</Bouton>
        <Bouton variante="secondaire" onClick={() => setOuvert(false)}>{t("commun.annuler")}</Bouton>
      </div>
    </Formulaire>
  );
}
