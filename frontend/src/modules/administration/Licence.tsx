import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { Bouton } from "../../composants/Bouton";
import { MessageErreur, Section } from "../../composants/Messages";
import { type Cle, date, dateHeure, heure, t } from "../../i18n";
import type { EtatLicenceAffiche, StatutLicence } from "../../types/gesco";

const STATUTS: Record<StatutLicence, Cle> = {
  absente: "licence.statut.absente",
  valide: "licence.statut.valide",
  a_reemettre: "licence.statut.a_reemettre",
  echue: "licence.statut.echue",
};

/** Licence du poste (§26.5) : l'empreinte à transmettre à l'éditeur et l'import du fichier reçu.
 *  Aucun état de la licence ne bloque l'application ; l'écran informe seulement. */
export function Licence() {
  const client = useQueryClient();
  const etat = useQuery({ queryKey: ["licence"], queryFn: () => appeler("licence_etat", null) });
  const installation = useMutation({
    mutationFn: (contenu: string) => appeler("licence_installer", { contenu }),
    onSuccess: (nouvel) => {
      client.setQueryData(["licence"], nouvel);
      void client.invalidateQueries({ queryKey: ["avertissements"] });
    },
  });
  return (
    <div className="flex max-w-3xl flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("licence.titre")}</h1>
      <MessageErreur erreur={etat.error} />
      {etat.data === undefined ? null : (
        <>
          <Empreinte empreinte={etat.data.empreinte_poste} />
          <Situation affiche={etat.data} />
        </>
      )}
      <Section titre={t("licence.installer")}>
        <p className="text-sm text-muet">{t("licence.installer_aide")}</p>
        <label htmlFor="fichier-licence" className="flex flex-col gap-1 text-sm text-encre">
          <span>{t("licence.fichier")}</span>
          <input
            id="fichier-licence"
            type="file"
            disabled={installation.isPending}
            className="text-sm text-encre file:mr-3 file:rounded file:border file:border-muet file:bg-surface file:px-3 file:py-1.5 file:text-encre"
            onChange={(e) => {
              const fichier = e.target.files?.[0];
              e.target.value = "";
              if (fichier !== undefined) void fichier.text().then((contenu) => installation.mutate(contenu));
            }}
          />
        </label>
        <MessageErreur erreur={installation.error} />
        {installation.data?.installee_le == null ? null : (
          <p role="status" className="text-sm text-vert">{t("licence.installee_a", { heure: heure(installation.data.installee_le) })}</p>
        )}
      </Section>
    </div>
  );
}

function Empreinte({ empreinte }: { empreinte: string }) {
  const [copiee, setCopiee] = useState(false);
  return (
    <Section titre={t("licence.empreinte")}>
      <p className="text-sm text-muet">{t("licence.empreinte_aide")}</p>
      <div className="flex flex-wrap items-center gap-3">
        <p className="select-all font-condensee text-lg text-encre">{empreinte}</p>
        <Bouton
          variante="secondaire"
          onClick={() => void navigator.clipboard.writeText(empreinte).then(() => setCopiee(true))}
        >
          {t("licence.copier")}
        </Bouton>
        {copiee ? <p role="status" className="text-sm text-vert">{t("licence.copiee")}</p> : null}
      </div>
    </Section>
  );
}

function Situation({ affiche }: { affiche: EtatLicenceAffiche }) {
  const { etat, licence, installee_le, empreinte_attachee } = affiche;
  const libre = licence?.empreinte_machine === "LIBRE";
  const lignes: [Cle, string][] = [[
    "licence.connectes",
    etat.modules_connectes_actifs ? t("commun.oui") : t("commun.non"),
  ]];
  if (etat.fin_grace !== null) lignes.push(["licence.fin_grace", date(etat.fin_grace)]);
  if (licence !== null) {
    const titulaire: [Cle, string][] = licence.etablissement === "" ? [] : [["licence.etablissement", licence.etablissement]];
    lignes.unshift(
      ["licence.identifiant", licence.licence_id],
      ...titulaire,
      ["licence.edition", licence.edition === "connectee" ? t("licence.edition.connectee") : t("licence.edition.essentielle")],
      ...(libre ? [["licence.nature", t("licence.libre")] as [Cle, string]] : []),
      ["licence.modules", licence.modules.length === 0 ? t("licence.modules_aucun") : licence.modules.join(", ")],
      ["licence.emise_le", date(licence.emise_le)],
      ["licence.echeance", licence.echeance === null ? t("licence.perpetuelle") : date(licence.echeance)],
    );
  }
  if (installee_le !== null) lignes.push(["licence.installee_le", dateHeure(installee_le)]);
  if (empreinte_attachee !== null) lignes.push(["licence.attachee", empreinte_attachee]);
  const statut = libre && etat.statut === "a_reemettre" ? "licence.statut.a_reinstaller" : STATUTS[etat.statut];
  return (
    <Section titre={t("licence.situation")}>
      <p className={`text-sm font-medium ${etat.statut === "valide" ? "text-vert" : "text-ocre"}`}>{t(statut)}</p>
      <dl className="grid grid-cols-2 gap-x-4 gap-y-1 text-sm">
        {lignes.map(([cle, valeur]) => (
          <div key={cle} className="contents">
            <dt className="text-muet">{t(cle)}</dt>
            <dd className="text-encre">{valeur}</dd>
          </div>
        ))}
      </dl>
    </Section>
  );
}
