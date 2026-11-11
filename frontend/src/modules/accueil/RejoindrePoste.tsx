import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { apercevoirPrincipal, rejoindrePrincipal } from "../../api/poste";
import { usePoste } from "../../app/poste";
import { Bouton } from "../../composants/Bouton";
import { Champ, Formulaire } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { t } from "../../i18n";

/** Troisième chemin du premier lancement : ce poste n'aura pas de base, il travaille sur celle du
 *  poste principal de l'école. On saisit l'adresse lue sur le poste principal, on compare
 *  l'empreinte affichée des deux côtés, puis on recopie le code. */
export function RejoindrePoste() {
  const client = useQueryClient();
  const { info } = usePoste();
  const [adresse, setAdresse] = useState("");
  const [nom, setNom] = useState<string | null>(null);
  const [code, setCode] = useState("");
  const recherche = useMutation({ mutationFn: () => apercevoirPrincipal(adresse) });
  const liaison = useMutation({
    mutationFn: () => rejoindrePrincipal(code, nomSaisi),
    onSuccess: (poste) => {
      client.setQueryData(["poste"], poste);
      void client.invalidateQueries({ queryKey: ["etat"] });
    },
  });
  const nomSaisi = nom ?? info?.nom_ordinateur ?? "";
  const apercu = recherche.data;

  return (
    <Section titre={t("rejoindre.titre")}>
      <p className="text-sm text-muet">{t("rejoindre.presentation")}</p>
      <Formulaire surEnvoi={() => recherche.mutate()}>
        <Champ
          id="rejoindre-adresse"
          libelle={t("rejoindre.adresse")}
          aide={t("rejoindre.adresse_aide")}
          value={adresse}
          autoComplete="off"
          inputMode="decimal"
          onChange={(e) => {
            setAdresse(e.target.value);
            recherche.reset();
          }}
        />
        <MessageErreur erreur={recherche.error} />
        <div>
          <Bouton type="submit" variante={apercu === undefined ? "principal" : "secondaire"} disabled={recherche.isPending || adresse.trim() === ""}>
            {recherche.isPending ? t("rejoindre.recherche_en_cours") : t("rejoindre.chercher")}
          </Bouton>
        </div>
      </Formulaire>
      {apercu === undefined ? null : (
        <>
          <div className="flex flex-col gap-1 rounded border border-muet/30 bg-fond p-3">
            <p className="text-sm text-encre">{t("rejoindre.trouve", { adresse: apercu.adresse, version: apercu.version })}</p>
            <p className="text-sm text-muet">{t("rejoindre.empreinte")}</p>
            <p id="rejoindre-empreinte" className="font-condensee text-xl font-semibold tracking-wide text-encre">{apercu.empreinte_courte}</p>
            <p className="text-sm text-encre">{t("rejoindre.empreinte_aide")}</p>
          </div>
          {apercu.meme_version ? (
            <Formulaire surEnvoi={() => liaison.mutate()}>
              <Champ id="rejoindre-nom" libelle={t("rejoindre.nom")} aide={t("rejoindre.nom_aide")} value={nomSaisi} autoComplete="off" onChange={(e) => setNom(e.target.value)} />
              <Champ id="rejoindre-code" libelle={t("rejoindre.code")} aide={t("rejoindre.code_aide")} value={code} autoComplete="off" onChange={(e) => {
                  setCode(e.target.value.toUpperCase());
                  liaison.reset();
                }} />
              <MessageErreur erreur={liaison.error} />
              <div>
                <Bouton type="submit" disabled={liaison.isPending || code.trim() === "" || nomSaisi.trim() === ""}>
                  {liaison.isPending ? t("rejoindre.liaison_en_cours") : t("rejoindre.relier")}
                </Bouton>
              </div>
            </Formulaire>
          ) : (
            <p role="alert" className="text-sm text-rouge">
              {t("rejoindre.autre_version", { ici: info?.version ?? "", principal: apercu.version })}
            </p>
          )}
        </>
      )}
    </Section>
  );
}
