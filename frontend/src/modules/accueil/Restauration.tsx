import { useMutation } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { choisirArchive } from "../../api/fichiers";
import { Bouton } from "../../composants/Bouton";
import { Champ, Formulaire, ZoneTexte } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { date, t } from "../../i18n";
import type { RapportRestauration } from "../../types/gesco";

/** Assistant de restauration (§11.4) : lire l'archive, vérifier qu'elle est la bonne, saisir la
 *  phrase si le poste n'a pas la clé, restaurer. */
export function Restauration({
  commande,
  surTermine,
}: {
  commande: "sauvegarde_restaurer" | "sauvegarde_restaurer_initiale";
  surTermine: (rapport: RapportRestauration) => void;
}) {
  const [chemin, setChemin] = useState("");
  const [phrase, setPhrase] = useState("");
  const apercu = useMutation({ mutationFn: (archive: string) => appeler("sauvegarde_apercu", { chemin: archive }) });
  const parcourir = useMutation({
    mutationFn: () => choisirArchive(t("restauration.filtre")),
    onSuccess: (choisi) => {
      if (choisi === null) return;
      setChemin(choisi);
      apercu.mutate(choisi);
    },
  });
  const restauration = useMutation({
    mutationFn: () => appeler(commande, { chemin, phrase: phrase.trim() === "" ? null : phrase }),
    onSuccess: (rapport) => {
      setPhrase("");
      surTermine(rapport);
    },
  });
  return (
    <Section titre={t("restauration.titre")}>
      <Formulaire surEnvoi={() => apercu.mutate(chemin)}>
        <Champ id="archive" libelle={t("restauration.chemin")} value={chemin} onChange={(e) => setChemin(e.target.value)} />
        <MessageErreur erreur={parcourir.error ?? apercu.error} />
        <div className="flex gap-2">
          <Bouton variante="secondaire" onClick={() => parcourir.mutate()} disabled={parcourir.isPending}>{t("restauration.parcourir")}</Bouton>
          <Bouton variante="secondaire" type="submit" disabled={chemin.trim() === ""}>{t("restauration.lire")}</Bouton>
        </div>
      </Formulaire>
      {apercu.data === undefined ? null : (
        <Formulaire surEnvoi={() => restauration.mutate()}>
          <p className="text-sm text-encre">
            {t("restauration.apercu", {
              etablissement: apercu.data.etablissement_nom,
              date: date(apercu.data.horodatage),
              schema: apercu.data.version_schema,
            })}
          </p>
          <p className="text-sm text-muet">{t("restauration.enveloppe", { id: apercu.data.enveloppe_secours_id })}</p>
          <ZoneTexte id="phrase-restauration" libelle={t("restauration.phrase_facultative")} aide={t("phrase.saisie_aide")} valeur={phrase} surChangement={setPhrase} />
          <p className="text-sm text-ocre">{t("restauration.avertissement")}</p>
          <MessageErreur erreur={restauration.error} />
          <div>
            <Bouton type="submit" disabled={restauration.isPending}>{t("restauration.lancer")}</Bouton>
          </div>
        </Formulaire>
      )}
    </Section>
  );
}
