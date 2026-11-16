import { useMutation } from "@tanstack/react-query";
import { createContext, useContext, useEffect, useState } from "react";

import { appeler } from "../../api";
import { Bouton } from "../../composants/Bouton";
import { MessageErreur } from "../../composants/Messages";
import { t } from "../../i18n";
import type { SonLecon } from "../../types/gesco";

/** Sons de la leçon ouverte, par empreinte, et la façon de les montrer. */
export type SonsDeLaLecon = {
  sons: ReadonlyMap<string, SonLecon>;
  /** Mode classe : un grand bouton de lecture ; un son absent ne laisse rien. */
  classe: boolean;
};

export const ContexteSons = createContext<SonsDeLaLecon | null>(null);

/** Lit un son par morceaux, par la même route que les vidéos, sur le poste principal comme
 *  depuis un poste relié ; le rend en adresse locale : rien ne se charge par Internet. */
async function chargerSon(empreinte: string, typeMime: string): Promise<string> {
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
  return URL.createObjectURL(new Blob(morceaux as BlobPart[], { type: typeMime }));
}

function LecteurSon({ son, titre, classe }: { son: SonLecon; titre: string; classe: boolean }) {
  const [adresse, setAdresse] = useState<string | null>(null);
  const charger = useMutation({ mutationFn: () => chargerSon(son.empreinte, son.type_mime), onSuccess: setAdresse });
  useEffect(
    () => () => {
      if (adresse !== null) URL.revokeObjectURL(adresse);
    },
    [adresse],
  );
  return (
    <span className="my-2 flex flex-col gap-1">
      <span className="font-semibold text-marine">{titre}</span>
      {adresse !== null ? (
        // Lecture, pause et avancer : les commandes du lecteur du système.
        <audio controls autoPlay src={adresse} className="w-full max-w-xl" aria-label={titre} />
      ) : (
        <span className="flex flex-wrap items-center gap-2">
          <Bouton
            variante={classe ? "principal" : "secondaire"}
            onClick={() => charger.mutate()}
            disabled={charger.isPending}
            className={classe ? "px-8! py-4! text-2xl!" : undefined}
          >
            {charger.isPending ? t("bibliotheque.son.chargement") : t("bibliotheque.son.ecouter")}
          </Bouton>
          <MessageErreur erreur={charger.error} />
        </span>
      )}
    </span>
  );
}

/** Un son appelé par la leçon. Absent, le personnel lit « Son non disponible » ; en mode
 *  classe, devant les élèves, il ne laisse rien. La leçon s'ouvre toujours sans lui. */
export function SonDansLecon({ empreinte, titre }: { empreinte: string; titre: string }) {
  const contexte = useContext(ContexteSons);
  const son = contexte?.sons.get(empreinte);
  if (son === undefined) return <span className="my-2 block text-muet">{t("bibliotheque.son.ligne", { titre })}</span>;
  if (!son.disponible) return contexte?.classe ? null : <span className="my-2 block text-muet">{t("bibliotheque.son_absent")}</span>;
  return <LecteurSon son={son} titre={titre} classe={contexte?.classe ?? false} />;
}
