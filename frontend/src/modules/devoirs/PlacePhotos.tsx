import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { Bouton } from "../../composants/Bouton";
import { MessageErreur, Section } from "../../composants/Messages";
import { t } from "../../i18n";
import type { PlaceAnnee } from "../../types/gesco";

/** Octets lisibles : « 312 Ko », « 4,2 Mo », « 1,3 Go ». */
export function taille(octets: number): string {
  const unites = ["octets", "Ko", "Mo", "Go"];
  let valeur = octets;
  let i = 0;
  while (valeur >= 1024 && i < unites.length - 1) {
    valeur /= 1024;
    i += 1;
  }
  return `${valeur.toLocaleString("fr-FR", { maximumFractionDigits: i === 0 ? 0 : 1 })} ${unites[i] ?? ""}`;
}

function Annee({ annee }: { annee: PlaceAnnee }) {
  const client = useQueryClient();
  const [confirmer, setConfirmer] = useState(false);
  const liberer = useMutation({
    mutationFn: () => appeler("devoirs_photos_liberer", { annee_id: annee.annee_id }),
    onSuccess: (p) => {
      client.setQueryData(["devoirs", "place"], p);
      setConfirmer(false);
    },
  });
  return (
    <li className="flex flex-col gap-1 border-b border-muet/40 py-2">
      <p className="text-sm text-encre">
        {t("devoirs.place_annee", { annee: annee.libelle, photos: String(annee.photos), taille: taille(annee.octets) })}
        {annee.retirees > 0 ? ` · ${t("devoirs.place_retirees", { nombre: String(annee.retirees) })}` : ""}
      </p>
      {!annee.close ? (
        <p className="text-xs text-muet">{t("devoirs.place_ouverte")}</p>
      ) : annee.photos === 0 ? null : confirmer ? (
        <div role="alertdialog" aria-label={t("devoirs.liberer")} className="flex flex-col gap-2 rounded border border-ocre p-2">
          <p className="text-sm text-encre">{t("devoirs.liberer_confirmer", { annee: annee.libelle })}</p>
          <div className="flex flex-wrap gap-2">
            <Bouton variante="danger" onClick={() => liberer.mutate()} disabled={liberer.isPending}>
              {t("devoirs.liberer_oui")}
            </Bouton>
            <Bouton variante="secondaire" onClick={() => setConfirmer(false)}>
              {t("commun.annuler")}
            </Bouton>
          </div>
        </div>
      ) : (
        <div>
          <Bouton variante="secondaire" onClick={() => setConfirmer(true)}>
            {t("devoirs.liberer")}
          </Bouton>
        </div>
      )}
      <MessageErreur erreur={liberer.error} />
    </li>
  );
}

/** Réglages des devoirs pour la direction : la place des photos des copies, année par année. */
export function PlacePhotos() {
  const lue = useQuery({ queryKey: ["devoirs", "place"], queryFn: () => appeler("devoirs_place_lire", null) });
  return (
    <Section titre={t("devoirs.place")}>
      <MessageErreur erreur={lue.error} />
      {lue.data === undefined ? null : lue.data.annees.length === 0 ? (
        <p className="text-sm text-muet">{t("devoirs.place_aucune")}</p>
      ) : (
        <ul className="flex flex-col">
          {lue.data.annees.map((a) => (
            <Annee key={a.annee_id} annee={a} />
          ))}
        </ul>
      )}
    </Section>
  );
}
