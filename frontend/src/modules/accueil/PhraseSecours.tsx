import { useState } from "react";

import { ApercuPdf } from "../../composants/ApercuPdf";
import { Bouton } from "../../composants/Bouton";
import { CaseACocher } from "../../composants/Champ";
import { Section } from "../../composants/Messages";
import { t } from "../../i18n";
import type { PhraseAffichee } from "../../types/gesco";

/** Affichage unique de la phrase de secours et de son certificat (§6.3). */
export function PhraseSecours({ phrase, surTermine }: { phrase: PhraseAffichee; surTermine: () => void }) {
  const [range, setRange] = useState(false);
  return (
    <Section titre={t("phrase.titre")}>
      <p className="text-sm text-encre">{t("phrase.explication")}</p>
      <ol className="grid grid-cols-3 gap-2">
        {phrase.mots.map((mot, rang) => (
          <li key={rang} className="rounded border border-muet/40 px-3 py-2 text-base">
            <span className="mr-2 text-sm text-muet">{rang + 1}</span>
            <span className="font-semibold text-encre">{mot}</span>
          </li>
        ))}
      </ol>
      <h3 className="text-sm font-medium text-encre">{t("phrase.certificat")}</h3>
      <ApercuPdf base64={phrase.certificat_pdf} />
      <CaseACocher id="certificat-range" libelle={t("phrase.confirmation")} coche={range} surChangement={setRange} />
      <div>
        <Bouton disabled={!range} onClick={surTermine}>{t("commun.continuer")}</Bouton>
      </div>
    </Section>
  );
}
