import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { commandeAutorisee } from "../../app/navigation";
import { Bouton } from "../../composants/Bouton";
import { Champ, Formulaire, Selection } from "../../composants/Champ";
import { MessageEnregistre, MessageErreur, Section } from "../../composants/Messages";
import { type Cle, dateCivile, t } from "../../i18n";
import type { FicheEleve, NatureActe, SaisieEleve } from "../../types/gesco";
import { ChampsIdentite, identiteVide, saisieDepuis } from "./Identite";
import { reduirePhoto } from "./photo";
import { FicheAbsorbee, FusionEleve } from "./Fusion";
import { FraisEleve } from "../finance/FraisEleve";
import { InscriptionsEleve } from "./InscriptionsEleve";
import { ResponsablesEleve } from "./Responsables";
import { Webcam } from "./Webcam";

const ACTES: Record<NatureActe, Cle> = { acte: "eleves.acte.acte", jugement_suppletif: "eleves.acte.jugement_suppletif" };

export function saisieEleveVide(): SaisieEleve {
  return { identite: identiteVide(), matricule_interne: "", matricule_national: "", acte_nature: null, acte_numero: "", acte_date: "", ecole_origine: "" };
}

function saisieDeLaFiche(fiche: FicheEleve): SaisieEleve {
  return {
    identite: saisieDepuis(fiche.identite),
    matricule_interne: fiche.eleve.matricule_interne,
    matricule_national: fiche.eleve.matricule_national ?? "",
    acte_nature: fiche.eleve.acte_nature,
    acte_numero: fiche.eleve.acte_numero,
    acte_date: fiche.eleve.acte_date ?? "",
    ecole_origine: fiche.eleve.ecole_origine,
  };
}

/** Champs de la fiche élève (T-02) : identité, puis état civil et provenance. Le matricule
 *  interne ne se saisit que si l'école le choisit ; sinon GESCO le génère. */
export function ChampsEleve({ prefixe, valeur, surChangement }: { prefixe: string; valeur: SaisieEleve; surChangement: (v: SaisieEleve) => void }) {
  const reglages = useQuery({ queryKey: ["scolarite-reglages"], queryFn: () => appeler("scolarite_reglages", null) });
  return (
    <div className="flex flex-col gap-3">
      {reglages.data?.matricule_manuel === true ? (
        <Champ id={`${prefixe}-matricule`} libelle={t("eleves.matricule")} aide={t("eleves.matricule_aide")} value={valeur.matricule_interne} onChange={(e) => surChangement({ ...valeur, matricule_interne: e.target.value })} />
      ) : null}
      <ChampsIdentite prefixe={prefixe} valeur={valeur.identite} surChangement={(identite) => surChangement({ ...valeur, identite })} eleve />
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <Champ id={`${prefixe}-matricule-national`} libelle={t("eleves.matricule_national")} aide={t("eleves.matricule_national_aide")} value={valeur.matricule_national} onChange={(e) => surChangement({ ...valeur, matricule_national: e.target.value })} />
        <Selection
          id={`${prefixe}-acte-nature`}
          libelle={t("eleves.acte")}
          valeur={valeur.acte_nature ?? ""}
          surChangement={(v) => surChangement({ ...valeur, acte_nature: v === "acte" || v === "jugement_suppletif" ? v : null })}
          options={[
            { valeur: "", libelle: t("eleves.acte.aucun") },
            { valeur: "acte", libelle: t(ACTES.acte) },
            { valeur: "jugement_suppletif", libelle: t(ACTES.jugement_suppletif) },
          ]}
        />
        <Champ id={`${prefixe}-acte-numero`} libelle={t("eleves.acte_numero")} value={valeur.acte_numero} onChange={(e) => surChangement({ ...valeur, acte_numero: e.target.value })} />
        <Champ id={`${prefixe}-acte-date`} type="date" libelle={t("eleves.acte_date")} value={valeur.acte_date} onChange={(e) => surChangement({ ...valeur, acte_date: e.target.value })} />
        <Champ id={`${prefixe}-origine`} libelle={t("eleves.ecole_origine")} value={valeur.ecole_origine} onChange={(e) => surChangement({ ...valeur, ecole_origine: e.target.value })} />
      </div>
    </div>
  );
}

/** Fiche d'un élève : identité, photo, responsables. */
export function FicheEleveEcran({ id, permissions, surOuvrir }: { id: string; permissions: readonly string[]; surOuvrir: (id: string) => void }) {
  const fiche = useQuery({ queryKey: ["eleve", id], queryFn: () => appeler("eleve_lire", { id }) });
  if (fiche.isError) return <MessageErreur erreur={fiche.error} />;
  if (fiche.data === undefined) return <p className="text-sm text-muet">{t("commun.chargement")}</p>;
  const { identite, eleve } = fiche.data;
  const modifiable = commandeAutorisee("eleve_modifier", permissions) && eleve.fusionne_dans === null;
  return (
    <div className="flex flex-col gap-4">
      {eleve.fusionne_dans === null ? null : <FicheAbsorbee conservee={eleve.fusionne_dans} surOuvrir={surOuvrir} />}
      <Section titre={`${identite.prenoms} ${identite.nom}`}>
        <div className="flex flex-wrap items-start gap-4">
          <Photo fiche={fiche.data} modifiable={modifiable} />
          <dl className="grid grid-cols-2 gap-x-3 gap-y-1 text-sm">
            <dt className="text-muet">{t("eleves.matricule")}</dt>
            <dd className="font-condensee text-encre">{eleve.matricule_interne}</dd>
            <dt className="text-muet">{t("personne.date_naissance")}</dt>
            <dd className="text-encre">
              {identite.date_naissance === null ? "" : dateCivile(identite.date_naissance)}
              {identite.date_naissance_estimee ? ` (${t("personne.estimee")})` : ""}
            </dd>
            <dt className="text-muet">{t("personne.lieu_naissance")}</dt>
            <dd className="text-encre">{identite.lieu_naissance}</dd>
          </dl>
        </div>
      </Section>
      {modifiable ? <EditionEleve key={JSON.stringify(fiche.data.eleve) + JSON.stringify(identite)} fiche={fiche.data} /> : null}
      <ResponsablesEleve fiche={fiche.data} modifiable={modifiable} />
      {eleve.fusionne_dans === null ? <InscriptionsEleve fiche={fiche.data} permissions={permissions} /> : null}
      <FraisEleve eleveId={eleve.id} permissions={permissions} />
      {modifiable && commandeAutorisee("eleves_fusionner", permissions) ? <FusionEleve fiche={fiche.data} surOuvrir={surOuvrir} /> : null}
    </div>
  );
}

function useRafraichir(id: string) {
  const client = useQueryClient();
  return (fiche: FicheEleve) => {
    client.setQueryData(["eleve", id], fiche);
    void client.invalidateQueries({ queryKey: ["eleves"] });
  };
}

function EditionEleve({ fiche }: { fiche: FicheEleve }) {
  const rafraichir = useRafraichir(fiche.eleve.id);
  const [saisie, setSaisie] = useState(() => saisieDeLaFiche(fiche));
  const [enregistre, setEnregistre] = useState<number | null>(null);
  const modification = useMutation({
    mutationFn: () => appeler("eleve_modifier", { id: fiche.eleve.id, saisie }),
    onSuccess: (nouvelle) => {
      setEnregistre(Date.now());
      rafraichir(nouvelle);
    },
  });
  return (
    <Section titre={t("eleves.identite")}>
      <Formulaire surEnvoi={() => modification.mutate()}>
        <ChampsEleve prefixe="eleve" valeur={saisie} surChangement={setSaisie} />
        <MessageErreur erreur={modification.error} />
        <MessageEnregistre instant={enregistre} />
        <div><Bouton type="submit" disabled={modification.isPending}>{t("commun.enregistrer")}</Bouton></div>
      </Formulaire>
    </Section>
  );
}

/** Photo choisie dans un fichier, prise à la webcam (déjà réduite), ou retirée. */
type SourcePhoto = { fichier: File } | { base64: string } | null;

function Photo({ fiche, modifiable }: { fiche: FicheEleve; modifiable: boolean }) {
  const rafraichir = useRafraichir(fiche.eleve.id);
  const [webcam, setWebcam] = useState(false);
  const pose = useMutation({
    mutationFn: async (source: SourcePhoto) =>
      appeler("eleve_photo_definir", {
        id: fiche.eleve.id,
        contenu_base64: source === null ? null : "fichier" in source ? await reduirePhoto(source.fichier) : source.base64,
      }),
    onSuccess: (nouvelle) => {
      setWebcam(false);
      rafraichir(nouvelle);
    },
  });
  if (webcam) {
    return <Webcam surPhoto={(base64) => pose.mutate({ base64 })} surFermer={() => setWebcam(false)} />;
  }
  return (
    <div className="flex w-36 flex-col gap-2">
      {fiche.photo === null ? (
        <div className="flex h-44 w-36 items-center justify-center rounded border border-dashed border-muet text-center text-xs text-muet">{t("eleves.sans_photo")}</div>
      ) : (
        <img src={fiche.photo} alt={t("eleves.photo")} className="h-44 w-36 rounded border border-muet/30 object-cover" />
      )}
      {modifiable ? (
        <>
          <label htmlFor="eleve-photo" className="cursor-pointer text-sm text-primaire underline">{t("eleves.choisir_photo")}</label>
          <input
            id="eleve-photo"
            type="file"
            accept="image/png,image/jpeg"
            className="sr-only"
            onChange={(e) => {
              const fichier = e.target.files?.[0];
              if (fichier !== undefined) pose.mutate({ fichier });
            }}
          />
          <Bouton variante="secondaire" onClick={() => setWebcam(true)}>{t("eleves.webcam")}</Bouton>
          {fiche.photo === null ? null : (
            <Bouton variante="secondaire" onClick={() => pose.mutate(null)}>{t("eleves.retirer_photo")}</Bouton>
          )}
        </>
      ) : null}
      <MessageErreur erreur={pose.error} />
    </div>
  );
}
