import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { choisirDossier } from "../../api/fichiers";
import { Bouton } from "../../composants/Bouton";
import { MessageEnregistre, MessageErreur, Section } from "../../composants/Messages";
import { dateHeure, t, taille } from "../../i18n";
import type { Destination, DossierHorsEcole, RythmeHorsEcole } from "../../types/gesco";
import { CLES_ETAT, RYTHMES, aProposer, avecDossier, sansDossier, volumeCopie, volumeMensuel } from "./copieHorsEcole";

/**
 * Copie hors de l'école : un dossier que Google Drive, OneDrive ou Dropbox envoie sur le compte
 * de l'école. Rien n'est choisi à la livraison ; sans dossier trouvé, seul « Choisir un autre
 * dossier… » apparaît, sans message.
 */
export function HorsEcole() {
  const client = useQueryClient();
  const etat = useQuery({ queryKey: ["hors_ecole"], queryFn: () => appeler("sauvegarde_hors_ecole_etat", null) });
  const trouves = useQuery({
    queryKey: ["dossiers_synchronises"],
    queryFn: () => appeler("sauvegarde_dossiers_synchronises", null),
  });
  const destinations = useQuery({
    queryKey: ["destinations"],
    queryFn: () => appeler("sauvegarde_destinations_lister", null),
  });
  const [envoye, setEnvoye] = useState<number | null>(null);
  const rafraichir = () => {
    void client.invalidateQueries({ queryKey: ["hors_ecole"] });
    void client.invalidateQueries({ queryKey: ["destinations"] });
    void client.invalidateQueries({ queryKey: ["sauvegardes"] });
    void client.invalidateQueries({ queryKey: ["avertissements"] });
  };
  // La première destination est le dossier du poste : la commande la rajoute d'elle-même.
  const configurees = (destinations.data ?? []).slice(1);
  const definition = useMutation({
    mutationFn: (liste: Destination[]) => appeler("sauvegarde_destinations_definir", liste),
    onSuccess: rafraichir,
  });
  const reglage = useMutation({
    mutationFn: (rythme: RythmeHorsEcole) => appeler("sauvegarde_hors_ecole_regler", { rythme }),
    onSuccess: rafraichir,
  });
  const envoi = useMutation({
    mutationFn: () => appeler("sauvegarde_hors_ecole_envoyer", null),
    onSuccess: () => {
      setEnvoye(Date.now());
      rafraichir();
    },
  });
  const choisir = (chemin: string) => definition.mutate(avecDossier(configurees, chemin));
  const choisirAilleurs = async () => {
    const chemin = await choisirDossier();
    if (chemin !== null) choisir(chemin);
  };
  if (etat.data === undefined) return <MessageErreur erreur={etat.error} />;
  const { rythme, dossiers, taille_copie: tailleCopie, prochaine_copie: prochaine } = etat.data;
  const proposes = aProposer(trouves.data ?? [], dossiers.map((d) => d.chemin));
  const parCopie = volumeCopie(tailleCopie, Math.max(dossiers.length, 1));
  const parMois = volumeMensuel(tailleCopie, Math.max(dossiers.length, 1), rythme);
  const occupe = definition.isPending || destinations.data === undefined;
  return (
    <Section titre={t("sauvegarde.hors_ecole.titre")}>
      <p className="text-sm text-muet">{t("sauvegarde.hors_ecole.explication")}</p>

      <h3 className="text-sm font-medium text-encre">{t("sauvegarde.hors_ecole.dossier")}</h3>
      {dossiers.map((d) => (
        <DossierChoisi
          key={d.chemin}
          dossier={d}
          surRetrait={() => definition.mutate(sansDossier(configurees, d.chemin))}
          occupe={occupe}
        />
      ))}
      {proposes.map((d) => (
        <div key={d.propose} className="flex flex-wrap items-center gap-3">
          <span className="font-condensee text-sm text-encre">{t("sauvegarde.hors_ecole.trouve", { nom: d.nom, chemin: d.propose })}</span>
          <Bouton variante="secondaire" onClick={() => choisir(d.propose)} disabled={occupe}>{t("sauvegarde.hors_ecole.choisir")}</Bouton>
        </div>
      ))}
      <div>
        <Bouton variante="secondaire" onClick={() => void choisirAilleurs()} disabled={occupe}>{t("sauvegarde.hors_ecole.autre_dossier")}</Bouton>
      </div>
      <MessageErreur erreur={definition.error} />

      <fieldset className="flex flex-col gap-1">
        <legend className="text-sm font-medium text-encre">{t("sauvegarde.hors_ecole.rythme")}</legend>
        <div className="flex flex-wrap gap-4 text-sm text-encre">
          {RYTHMES.map((r) => (
            <label key={r.rythme} className="flex items-center gap-1">
              <input
                type="radio"
                name="rythme-hors-ecole"
                checked={rythme === r.rythme}
                disabled={reglage.isPending}
                onChange={() => reglage.mutate(r.rythme)}
              />
              {t(r.cle)}
            </label>
          ))}
        </div>
        {tailleCopie === null || tailleCopie <= 0 ? null : (
          <p className="text-sm text-muet">{t("sauvegarde.hors_ecole.poids", { taille: taille(tailleCopie) })}</p>
        )}
        {parMois === null ? null : (
          <p className="text-sm text-muet">{t("sauvegarde.hors_ecole.par_mois", { taille: taille(parMois) })}</p>
        )}
      </fieldset>
      <MessageErreur erreur={reglage.error} />

      {dossiers.length === 0 ? null : (
        <div className="flex flex-col gap-1">
          {parCopie === null ? null : (
            <p className="text-sm text-encre">{t("sauvegarde.hors_ecole.volume", { taille: taille(parCopie) })}</p>
          )}
          <div>
            <Bouton onClick={() => envoi.mutate()} disabled={envoi.isPending}>{t("sauvegarde.hors_ecole.envoyer")}</Bouton>
          </div>
          <MessageErreur erreur={envoi.error} />
          <MessageEnregistre instant={envoye} />
          {prochaine === null ? null : (
            <p className="text-sm text-muet">{t("sauvegarde.hors_ecole.prochaine", { date: dateHeure(prochaine) })}</p>
          )}
        </div>
      )}

      <p className="text-sm text-encre">{t("sauvegarde.hors_ecole.rappel")}</p>
    </Section>
  );
}

/** Un dossier choisi, sa dernière copie et son état. */
function DossierChoisi({ dossier, surRetrait, occupe }: { dossier: DossierHorsEcole; surRetrait: () => void; occupe: boolean }) {
  const derniere = dossier.derniere;
  return (
    <div className="flex flex-col gap-1">
      <div className="flex flex-wrap items-center gap-3">
        <span className="font-condensee text-sm text-encre">{dossier.chemin}</span>
        <Bouton variante="secondaire" onClick={surRetrait} disabled={occupe}>{t("commun.retirer")}</Bouton>
      </div>
      {derniere === null || derniere.etat_envoi === null ? null : (
        <p className="text-sm text-muet">
          {t("sauvegarde.hors_ecole.derniere", {
            date: dateHeure(derniere.horodatage),
            taille: taille(derniere.taille),
            etat: t(CLES_ETAT[derniere.etat_envoi]),
          })}
        </p>
      )}
    </div>
  );
}
