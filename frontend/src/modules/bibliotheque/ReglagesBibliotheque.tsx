import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { choisirPaquetBibliotheque } from "../../api/fichiers";
import { Bouton } from "../../composants/Bouton";
import { MessageErreur, Section } from "../../composants/Messages";
import { date, t } from "../../i18n";
import { ConfirmerRetrait } from "./Retraits";

/** Réglages de la bibliothèque de cours, pour la direction : la bibliothèque installée et
 *  l'installation d'une mise à jour depuis un fichier (une clé USB). Sans bibliothèque
 *  installée, l'écran le dit en une ligne ; rien ne bloque. */
export function ReglagesBibliotheque() {
  const client = useQueryClient();
  const [aRetirer, setARetirer] = useState<string | null>(null);
  const etat = useQuery({ queryKey: ["bibliotheque-etat"], queryFn: () => appeler("bibliotheque_etat", null) });
  const installation = useMutation({
    mutationFn: (chemin: string) => appeler("bibliotheque_installer", { chemin }),
    onSuccess: (r) => {
      client.setQueryData(["bibliotheque-etat"], r.etat);
      void client.invalidateQueries({ queryKey: ["bibliotheque"] });
    },
  });
  const choisir = async () => {
    const chemin = await choisirPaquetBibliotheque(t("bibliotheque.reglages.filtre"));
    if (chemin !== null) installation.mutate(chemin);
  };
  const paquets = etat.data?.paquets ?? [];
  return (
    <Section titre={t("bibliotheque.reglages.titre")}>
      <MessageErreur erreur={etat.error} />
      {etat.data === undefined ? null : paquets.length === 0 ? (
        <p className="text-sm text-encre">{t("bibliotheque.reglages.aucune")}</p>
      ) : (
        paquets.map((p) => (
          <div key={p.code} className="flex flex-col gap-1 text-sm text-encre">
            <p>
              {t("bibliotheque.reglages.installee", {
                version: p.version,
                date: date(p.installe_le),
                lecons: p.lecons,
                sujets: p.sujets,
              })}
            </p>
            {p.version_plus_recente === null ? null : (
              <p>{t("bibliotheque.reglages.plus_recente", { version: p.version_plus_recente })}</p>
            )}
            {aRetirer === p.code ? (
              <ConfirmerRetrait
                demande={{ grain: "paquet", cible: p.code, niveau_code: null, serie_code: null }}
                surFin={() => setARetirer(null)}
              />
            ) : (
              <div>
                <Bouton variante="secondaire" onClick={() => setARetirer(p.code)}>
                  {t("bibliotheque.retirer.paquet")}
                </Bouton>
              </div>
            )}
          </div>
        ))
      )}
      <div>
        <Bouton variante="secondaire" onClick={() => void choisir()} disabled={installation.isPending}>
          {t("bibliotheque.reglages.installer")}
        </Bouton>
      </div>
      <MessageErreur erreur={installation.error} />
      {installation.data === undefined ? null : (
        <p role="status" className="text-sm text-vert">
          {t("bibliotheque.reglages.installee_ok", { version: installation.data.version })}
        </p>
      )}
    </Section>
  );
}
