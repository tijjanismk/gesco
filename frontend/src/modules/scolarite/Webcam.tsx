import { useEffect, useRef, useState } from "react";

import { Bouton } from "../../composants/Bouton";
import { t } from "../../i18n";
import { capturerVideo } from "./photo";

/** Prise de photo à la webcam du poste (T-10). La caméra s'éteint dès la photo prise ou
 *  l'abandon ; sans caméra ou sans autorisation, un message le dit. */
export function Webcam({ surPhoto, surFermer }: { surPhoto: (base64: string) => void; surFermer: () => void }) {
  const video = useRef<HTMLVideoElement>(null);
  const [indisponible, setIndisponible] = useState(false);
  useEffect(() => {
    let flux: MediaStream | null = null;
    let arrete = false;
    navigator.mediaDevices
      .getUserMedia({ video: { width: 640, height: 480 }, audio: false })
      .then((ouvert) => {
        if (arrete) {
          ouvert.getTracks().forEach((piste) => piste.stop());
          return;
        }
        flux = ouvert;
        if (video.current !== null) {
          video.current.srcObject = ouvert;
          void video.current.play();
        }
      })
      .catch(() => setIndisponible(true));
    return () => {
      arrete = true;
      flux?.getTracks().forEach((piste) => piste.stop());
    };
  }, []);
  return (
    <div className="flex flex-col gap-2">
      {indisponible ? (
        <p role="alert" className="text-sm text-rouge">{t("eleves.webcam_indisponible")}</p>
      ) : (
        <video ref={video} muted playsInline className="w-64 rounded border border-muet/30" />
      )}
      <div className="flex flex-wrap gap-2">
        {indisponible ? null : (
          <Bouton
            onClick={() => {
              if (video.current !== null && video.current.videoWidth > 0) surPhoto(capturerVideo(video.current));
            }}
          >
            {t("eleves.prendre_photo")}
          </Bouton>
        )}
        <Bouton variante="secondaire" onClick={surFermer}>{t("commun.annuler")}</Bouton>
      </div>
    </div>
  );
}
