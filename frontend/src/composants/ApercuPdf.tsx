import { useEffect, useMemo } from "react";

import { t } from "../i18n";

/** Aperçu d'un PDF composé par le noyau (R-44 : le webview affiche, il ne compose jamais). */
export function ApercuPdf({ base64 }: { base64: string }) {
  const url = useMemo(() => {
    const octets = Uint8Array.from(atob(base64), (c) => c.charCodeAt(0));
    return URL.createObjectURL(new Blob([octets], { type: "application/pdf" }));
  }, [base64]);
  useEffect(() => () => URL.revokeObjectURL(url), [url]);
  // `view=FitH` : la page occupe la largeur du cadre au lieu de déborder à sa taille réelle.
  return <iframe title={t("document.apercu")} src={`${url}#view=FitH`} className="h-160 w-full rounded border border-muet" />;
}
