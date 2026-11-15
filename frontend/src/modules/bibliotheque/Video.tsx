import { useMutation } from "@tanstack/react-query";
import { createContext, useContext, useEffect, useState } from "react";

import { appeler } from "../../api";
import { Bouton } from "../../composants/Bouton";
import { MessageErreur } from "../../composants/Messages";
import { t } from "../../i18n";
import type { VideoLecon } from "../../types/gesco";

/** Vidéos de la leçon ouverte, par empreinte ; absent dans l'aperçu de l'éditeur. */
export const ContexteVideos = createContext<ReadonlyMap<string, VideoLecon> | null>(null);

/** Lit une vidéo par morceaux, sur le poste principal comme depuis un poste relié, et la rend
 *  en adresse locale : rien ne se charge par Internet. */
async function chargerVideo(empreinte: string): Promise<string> {
  const morceaux: Uint8Array[] = [];
  let position = 0;
  for (;;) {
    const m = await appeler("bibliotheque_video_morceau", { empreinte, position });
    const binaire = atob(m.contenu);
    const octets = new Uint8Array(binaire.length);
    for (let i = 0; i < binaire.length; i += 1) octets[i] = binaire.charCodeAt(i);
    if (octets.length === 0) break;
    morceaux.push(octets);
    position += octets.length;
    if (position >= m.taille) break;
  }
  return URL.createObjectURL(new Blob(morceaux as BlobPart[], { type: "video/mp4" }));
}

function LecteurVideo({ empreinte, titre }: { empreinte: string; titre: string }) {
  const [adresse, setAdresse] = useState<string | null>(null);
  const charger = useMutation({ mutationFn: () => chargerVideo(empreinte), onSuccess: setAdresse });
  useEffect(
    () => () => {
      if (adresse !== null) URL.revokeObjectURL(adresse);
    },
    [adresse],
  );
  if (adresse !== null)
    return (
      <video controls src={adresse} className="my-2 block max-h-screen w-full bg-encre" aria-label={titre}>
        <track kind="captions" />
      </video>
    );
  return (
    <span className="my-2 flex flex-wrap items-center gap-2">
      <Bouton variante="secondaire" onClick={() => charger.mutate()} disabled={charger.isPending}>
        {charger.isPending ? t("bibliotheque.video.chargement") : t("bibliotheque.video.lire", { titre })}
      </Bouton>
      <MessageErreur erreur={charger.error} />
    </span>
  );
}

/** Une vidéo appelée par la leçon. Absente du poste principal, une ligne le dit au personnel ;
 *  la leçon s'ouvre sans elle. */
export function VideoDansLecon({ empreinte, titre }: { empreinte: string; titre: string }) {
  const videos = useContext(ContexteVideos);
  const video = videos?.get(empreinte);
  if (video === undefined) return <span className="my-2 block text-muet">{t("bibliotheque.video.ligne", { titre })}</span>;
  if (!video.disponible) return <span className="my-2 block text-muet">{t("bibliotheque.video_absente")}</span>;
  return <LecteurVideo empreinte={empreinte} titre={titre} />;
}

/** Les deux guides courts : filmer une leçon, ajouter et montrer une vidéo. */
export function GuidesVideo({ surFermer }: { surFermer: () => void }) {
  return (
    <div role="dialog" aria-modal="true" aria-labelledby="guides-video" className="fixed inset-0 z-50 flex items-center justify-center bg-encre/40 p-4">
      <div className="flex max-h-full w-full max-w-2xl flex-col gap-3 overflow-y-auto rounded bg-surface p-5 text-encre">
        <h2 id="guides-video" className="text-lg font-semibold text-marine">
          {t("bibliotheque.guide.titre")}
        </h2>
        <section className="flex flex-col gap-1">
          <h3 className="font-semibold text-marine">{t("bibliotheque.guide.filmer.titre")}</h3>
          <ul className="list-disc pl-5">
            {(
              [
                "bibliotheque.guide.filmer.1",
                "bibliotheque.guide.filmer.2",
                "bibliotheque.guide.filmer.3",
                "bibliotheque.guide.filmer.4",
                "bibliotheque.guide.filmer.5",
                "bibliotheque.guide.filmer.6",
              ] as const
            ).map((cle) => (
              <li key={cle}>{t(cle)}</li>
            ))}
          </ul>
        </section>
        <section className="flex flex-col gap-1">
          <h3 className="font-semibold text-marine">{t("bibliotheque.guide.montrer.titre")}</h3>
          <ol className="list-decimal pl-5">
            {(
              [
                "bibliotheque.guide.montrer.1",
                "bibliotheque.guide.montrer.2",
                "bibliotheque.guide.montrer.3",
                "bibliotheque.guide.montrer.4",
              ] as const
            ).map((cle) => (
              <li key={cle}>{t(cle)}</li>
            ))}
          </ol>
        </section>
        <div className="flex justify-end">
          <Bouton onClick={surFermer}>{t("commun.fermer")}</Bouton>
        </div>
      </div>
    </div>
  );
}
