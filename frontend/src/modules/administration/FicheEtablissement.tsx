import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { Bouton } from "../../composants/Bouton";
import { Champ, Formulaire, Selection, ZoneTexte } from "../../composants/Champ";
import { MessageEnregistre, MessageErreur, Section } from "../../composants/Messages";
import { type Cle, t } from "../../i18n";
import type { Etablissement, FicheEtablissement as Fiche, StatutEtablissement } from "../../types/gesco";

const STATUTS: Record<StatutEtablissement, Cle> = {
  prive: "demarrage.statut.prive",
  public: "demarrage.statut.public",
  communautaire: "demarrage.statut.communautaire",
  medersa: "demarrage.statut.medersa",
};

function estStatut(valeur: string): valeur is StatutEtablissement {
  return Object.hasOwn(STATUTS, valeur);
}

/** Fiche de l'établissement et logo (SPEC_PHASE_1 §2) : ce que l'en-tête des documents imprime. */
export function FicheEtablissement() {
  const fiche = useQuery({ queryKey: ["etablissement"], queryFn: () => appeler("etablissement_lire", null) });
  if (fiche.data === undefined) return <MessageErreur erreur={fiche.error} />;
  return (
    <>
      <EditeurFiche key={fiche.data.etablissement.id} etablissement={fiche.data.etablissement} />
      <Logo fiche={fiche.data} />
    </>
  );
}

function useRafraichir() {
  const client = useQueryClient();
  return (nouvelle: Fiche) => {
    client.setQueryData(["etablissement"], nouvelle);
    void client.invalidateQueries({ queryKey: ["etat"] });
  };
}

function EditeurFiche({ etablissement }: { etablissement: Etablissement }) {
  const rafraichir = useRafraichir();
  const [champs, setChamps] = useState({
    nom: etablissement.nom,
    statut: etablissement.statut,
    code_academie: etablissement.code_academie,
    code_cap: etablissement.code_cap,
    adresse: etablissement.adresse,
    telephones: etablissement.telephones.join(", "),
    mentions_legales: etablissement.mentions_legales,
  });
  const [enregistre, setEnregistre] = useState<number | null>(null);
  const modification = useMutation({
    mutationFn: () => appeler("etablissement_modifier", { ...champs, telephones: champs.telephones.split(",") }),
    onSuccess: (nouvelle) => {
      setEnregistre(Date.now());
      rafraichir(nouvelle);
    },
  });
  const champ = (cle: "nom" | "code_academie" | "code_cap" | "adresse" | "telephones", libelle: string, aide?: string) => (
    <Champ
      id={`etablissement-${cle}`}
      libelle={libelle}
      aide={aide}
      value={champs[cle]}
      onChange={(e) => setChamps({ ...champs, [cle]: e.target.value })}
    />
  );
  const codesManquants = etablissement.code_academie === "" || etablissement.code_cap === "";
  return (
    <Section titre={t("etablissement.fiche")}>
      {codesManquants ? <p className="text-sm text-ocre">{t("etablissement.codes_manquants")}</p> : null}
      <Formulaire surEnvoi={() => modification.mutate()}>
        {champ("nom", t("demarrage.etablissement_nom"))}
        <Selection
          id="etablissement-statut"
          libelle={t("demarrage.statut")}
          valeur={champs.statut}
          surChangement={(v) => {
            if (estStatut(v)) setChamps({ ...champs, statut: v });
          }}
          options={Object.entries(STATUTS).map(([valeur, cle]) => ({ valeur, libelle: t(cle) }))}
        />
        {champ("code_academie", t("demarrage.code_academie"))}
        {champ("code_cap", t("demarrage.code_cap"))}
        {champ("adresse", t("demarrage.adresse"))}
        {champ("telephones", t("etablissement.telephones"), t("etablissement.telephones_aide"))}
        <ZoneTexte
          id="etablissement-mentions"
          libelle={t("etablissement.mentions")}
          aide={t("etablissement.mentions_aide")}
          valeur={champs.mentions_legales}
          surChangement={(mentions_legales) => setChamps({ ...champs, mentions_legales })}
        />
        <MessageErreur erreur={modification.error} />
        <MessageEnregistre instant={enregistre} />
        <div><Bouton type="submit" disabled={modification.isPending}>{t("commun.enregistrer")}</Bouton></div>
      </Formulaire>
    </Section>
  );
}

function Logo({ fiche }: { fiche: Fiche }) {
  const rafraichir = useRafraichir();
  const envoi = useMutation({
    mutationFn: (contenu_base64: string | null) => appeler("etablissement_logo_definir", { contenu_base64 }),
    onSuccess: rafraichir,
  });
  const lire = (fichier: File) => {
    const lecteur = new FileReader();
    lecteur.onload = () => {
      const url = typeof lecteur.result === "string" ? lecteur.result : "";
      envoi.mutate(url.slice(url.indexOf(",") + 1));
    };
    lecteur.readAsDataURL(fichier);
  };
  return (
    <Section titre={t("etablissement.logo")}>
      <p className="text-sm text-muet">{t("etablissement.logo_aide")}</p>
      {fiche.logo === null ? (
        <p className="text-sm text-muet">{t("etablissement.logo_absent")}</p>
      ) : (
        <img src={fiche.logo} alt={t("etablissement.logo")} className="h-20 w-fit rounded border border-muet/30 bg-surface p-1" />
      )}
      <label htmlFor="etablissement-logo" className="flex flex-col gap-1 text-sm text-encre">
        <span>{t("etablissement.logo_choisir")}</span>
        <input
          id="etablissement-logo"
          type="file"
          accept="image/png,image/jpeg"
          disabled={envoi.isPending}
          className="text-sm text-encre file:mr-3 file:rounded file:border file:border-muet file:bg-surface file:px-3 file:py-1.5 file:text-encre"
          onChange={(e) => {
            const fichier = e.target.files?.[0];
            e.target.value = "";
            if (fichier !== undefined) lire(fichier);
          }}
        />
      </label>
      {fiche.logo === null ? null : (
        <div>
          <Bouton variante="secondaire" onClick={() => envoi.mutate(null)} disabled={envoi.isPending}>
            {t("etablissement.logo_retirer")}
          </Bouton>
        </div>
      )}
      <MessageErreur erreur={envoi.error} />
    </Section>
  );
}
