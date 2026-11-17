import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { ApercuPdf } from "../../composants/ApercuPdf";
import { Bouton } from "../../composants/Bouton";
import { MessageErreur, Section } from "../../composants/Messages";
import { dateCivile, t } from "../../i18n";


/** Codes du portail d'une classe : combien d'élèves ont leur code, billets pour ceux qui n'en
 *  ont pas ou pour tous. Le code n'est écrit que sur le billet ; réimprimer remplace les codes. */
export function CodesPortailClasse({ classeId }: { classeId: string }) {
  const client = useQueryClient();
  const codes = useQuery({
    queryKey: ["portail-codes", classeId],
    queryFn: () => appeler("portail_codes_classe", { classe_id: classeId }),
  });
  const [pdf, setPdf] = useState<string | null>(null);
  const [rien, setRien] = useState(false);
  const billets = useMutation({
    mutationFn: (tous: boolean) => appeler("portail_billets_produire", { classe_id: classeId, tous }),
    onSuccess: (b) => {
      setPdf(b.pdf === "" ? null : b.pdf);
      setRien(b.billets === 0);
      void client.invalidateQueries({ queryKey: ["portail-codes", classeId] });
    },
  });
  const c = codes.data;
  return (
    <Section titre={t("portail.codes.titre")}>
      <MessageErreur erreur={codes.error ?? billets.error} />
      {c === undefined ? null : (
        <p className="text-sm text-encre">{t("portail.codes.compte", { avec: c.avec_code, eleves: c.eleves })}</p>
      )}
      <div className="flex flex-wrap gap-2">
        <Bouton variante="secondaire" onClick={() => billets.mutate(false)} disabled={billets.isPending}>
          {t("portail.codes.sans_code")}
        </Bouton>
        <Bouton variante="secondaire" onClick={() => billets.mutate(true)} disabled={billets.isPending}>
          {t("portail.codes.tous")}
        </Bouton>
      </div>
      <p className="text-sm text-encre">{t("portail.codes.remplace")}</p>
      {rien ? <p className="text-sm text-encre">{t("portail.codes.aucun_a_faire")}</p> : null}
      {pdf === null ? null : <ApercuPdf base64={pdf} />}
    </Section>
  );
}

/** Code du portail d'un élève, sur sa fiche : remis le …, « Remettre un code ». */
export function CodePortailEleve({ eleveId }: { eleveId: string }) {
  const client = useQueryClient();
  const etat = useQuery({
    queryKey: ["portail-code", eleveId],
    queryFn: () => appeler("portail_code_lire", { eleve_id: eleveId }),
  });
  const [pdf, setPdf] = useState<string | null>(null);
  const remise = useMutation({
    mutationFn: () => appeler("portail_code_remettre", { eleve_id: eleveId }),
    onSuccess: (b) => {
      setPdf(b.pdf === "" ? null : b.pdf);
      void client.invalidateQueries({ queryKey: ["portail-code", eleveId] });
    },
  });
  const e = etat.data;
  if (e === undefined) return <MessageErreur erreur={etat.error} />;
  return (
    <Section titre={t("portail.code.titre")}>
      <MessageErreur erreur={remise.error} />
      <div className="flex flex-wrap items-center gap-3">
        <span className="text-sm text-encre">
          {e.remis_le === null ? t("portail.code.aucun") : t("portail.code.remis_le", { date: dateCivile(e.remis_le) })}
        </span>
        {e.inscrit ? (
          <Bouton variante="secondaire" onClick={() => remise.mutate()} disabled={remise.isPending}>
            {t("portail.code.remettre")}
          </Bouton>
        ) : null}
      </div>
      {pdf === null ? null : <ApercuPdf base64={pdf} />}
    </Section>
  );
}

/** Affiche de classe : l'adresse du portail en grand et son code QR. */
export function AffichePortail() {
  const [pdf, setPdf] = useState<string | null>(null);
  const affiche = useMutation({
    mutationFn: () => appeler("portail_affiche", null),
    onSuccess: (a) => setPdf(a.pdf),
  });
  return (
    <div className="flex flex-col gap-2">
      <MessageErreur erreur={affiche.error} />
      <div>
        <Bouton variante="secondaire" onClick={() => affiche.mutate()} disabled={affiche.isPending}>
          {t("portail.affiche.imprimer")}
        </Bouton>
      </div>
      {pdf === null ? null : <ApercuPdf base64={pdf} />}
    </div>
  );
}
