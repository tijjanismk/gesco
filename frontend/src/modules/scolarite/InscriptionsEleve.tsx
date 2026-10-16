import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { type ReactNode, useState } from "react";

import { appeler } from "../../api";
import { commandeAutorisee } from "../../app/navigation";
import { ApercuPdf } from "../../composants/ApercuPdf";
import { Bouton } from "../../composants/Bouton";
import { CaseACocher, Champ, Formulaire, Selection } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { type Cle, dateCivile, t, texteDeCle } from "../../i18n";
import type { ChoixFrais, FicheEleve, InscriptionAffichee, StatutInscription, TypeInscription } from "../../types/gesco";
import { ChoixFraisChamps } from "../finance/ChoixFrais";
import { DepartEleve, DocumentsDepart, ExclusionsEleve } from "./Departs";

export const STATUTS: Record<StatutInscription, Cle> = {
  brouillon: "inscriptions.statut.brouillon",
  validee: "inscriptions.statut.validee",
  transferee: "inscriptions.statut.transferee",
  abandonnee: "inscriptions.statut.abandonnee",
  radiee: "inscriptions.statut.radiee",
  achevee: "inscriptions.statut.achevee",
};

export const TYPES: Record<TypeInscription, Cle> = {
  nouvelle: "inscriptions.type.nouvelle",
  reinscription: "inscriptions.type.reinscription",
  transfert_entrant: "inscriptions.type.transfert_entrant",
};

/** Classes de l'année courante, pour inscrire ou changer de classe. */
function useClassesCourantes() {
  const annees = useQuery({ queryKey: ["annees"], queryFn: () => appeler("annees_lister", null) });
  const courante = annees.data?.find((a) => a.est_courante);
  const classes = useQuery({
    queryKey: ["classes", courante?.id ?? null],
    queryFn: () => appeler("classes_lister", { annee_id: courante?.id ?? "" }),
    enabled: courante !== undefined,
  });
  return { courante, classes: classes.data ?? [] };
}

function useRafraichir(eleveId: string) {
  const client = useQueryClient();
  return () => {
    void client.invalidateQueries({ queryKey: ["eleve", eleveId] });
    void client.invalidateQueries({ queryKey: ["classes"] });
    void client.invalidateQueries({ queryKey: ["inscriptions"] });
    void client.invalidateQueries({ queryKey: ["situation", eleveId] });
    void client.invalidateQueries({ queryKey: ["exclusions"] });
  };
}

/** Ce que la dernière action a rendu : son refus, ou ses avertissements. Gardé par la section,
 *  car le formulaire qui l'a provoqué peut disparaître (un brouillon refusé remplace le
 *  formulaire d'inscription). */
interface Retour {
  erreur: unknown;
  avertissements: string[];
}

const SANS_RETOUR: Retour = { erreur: null, avertissements: [] };

function AfficherRetour({ retour }: { retour: Retour }) {
  return (
    <>
      <MessageErreur erreur={retour.erreur} />
      {retour.avertissements.length === 0 ? null : (
        <ul role="status" className="text-sm text-ocre">
          {retour.avertissements.map((c) => <li key={c}>{texteDeCle(c)}</li>)}
        </ul>
      )}
    </>
  );
}

/** Inscriptions d'un élève (SPEC_PHASE_2 §5) : historique, brouillon, validation, abandon,
 *  changement de classe. Les règles sont celles du noyau ; l'écran affiche ses refus. */
export function InscriptionsEleve({ fiche, permissions }: { fiche: FicheEleve; permissions: readonly string[] }) {
  const { courante } = useClassesCourantes();
  const [retour, setRetour] = useState<Retour>(SANS_RETOUR);
  const rafraichir = useRafraichir(fiche.eleve.id);
  const [annonce, setAnnonce] = useState<string | null>(null);
  const vivante = fiche.inscriptions.find((i) => i.inscription.annee_id === courante?.id && (i.inscription.statut === "brouillon" || i.inscription.statut === "validee"));
  return (
    <Section titre={t("inscriptions.titre")}>
      {fiche.inscriptions.length === 0 ? <p className="text-sm text-muet">{t("inscriptions.aucune")}</p> : null}
      <ul className="flex flex-col gap-2">
        {fiche.inscriptions.map((i) => (
          <li key={i.inscription.id} className="rounded border border-muet/30 p-3 text-sm">
            <p className="font-medium text-encre">{`${i.annee} · ${i.classe} · ${t(STATUTS[i.inscription.statut])}`}</p>
            <p className="text-muet">
              {`${t(TYPES[i.inscription.type_inscription])} · ${dateCivile(i.inscription.date_inscription)}`}
              {i.inscription.motif_abandon === "" ? "" : ` · ${i.inscription.motif_abandon}`}
            </p>
            {i.inscription.date_depart === null ? null : (
              <p className="text-muet">
                {t("depart.parti_le", { date: dateCivile(i.inscription.date_depart) })}
                {i.inscription.etablissement_destination === "" ? "" : ` · ${i.inscription.etablissement_destination}`}
                {i.inscription.motif_depart === "" ? "" : ` · ${i.inscription.motif_depart}`}
              </p>
            )}
            {i.inscription.statut === "validee" ? <FicheImprimee inscriptionId={i.inscription.id} /> : null}
            {i.inscription.statut === "transferee" ? <DocumentsDepart inscriptionId={i.inscription.id} permissions={permissions} /> : null}
            {i.inscription.statut === "validee" || i.inscription.statut === "radiee" ? (
              <ExclusionsEleve inscriptionId={i.inscription.id} permissions={permissions} surChange={rafraichir} />
            ) : null}
            {i === vivante ? (
              <ActionsInscription
                eleveId={fiche.eleve.id}
                inscription={i}
                permissions={permissions}
                surRetour={setRetour}
                depart={i.inscription.statut === "validee" ? <DepartEleve inscriptionId={i.inscription.id} permissions={permissions} surChange={rafraichir} surAnnonce={setAnnonce} /> : null}
              />
            ) : null}
          </li>
        ))}
      </ul>
      <AfficherRetour retour={retour} />
      {annonce === null ? null : <p role="status" className="text-sm text-vert">{annonce}</p>}
      {vivante === undefined && courante !== undefined && commandeAutorisee("inscription_creer", permissions) ? (
        <NouvelleInscription eleveId={fiche.eleve.id} permissions={permissions} surRetour={setRetour} />
      ) : null}
    </Section>
  );
}

/** Fiche d'inscription imprimée (SPEC_PHASE_2 §9) : composée par le noyau, enregistrée avec
 *  son empreinte, réimprimable. */
function FicheImprimee({ inscriptionId }: { inscriptionId: string }) {
  const production = useMutation({ mutationFn: () => appeler("inscription_fiche_produire", { id: inscriptionId }) });
  return (
    <div className="mt-2 flex flex-col gap-2">
      <div><Bouton variante="secondaire" onClick={() => production.mutate()} disabled={production.isPending}>{t("inscriptions.imprimer_fiche")}</Bouton></div>
      <MessageErreur erreur={production.error} />
      {production.data === undefined ? null : <ApercuPdf base64={production.data.pdf} />}
    </div>
  );
}

/** Inscription dans une classe de l'année courante : brouillon, ou inscription validée en une
 *  action pour qui détient les deux permissions (le parcours devant le parent, UC-01). */
function NouvelleInscription({ eleveId, permissions, surRetour }: { eleveId: string; permissions: readonly string[]; surRetour: (r: Retour) => void }) {
  const rafraichir = useRafraichir(eleveId);
  const { courante, classes } = useClassesCourantes();
  const [classeId, setClasseId] = useState("");
  const [redoublant, setRedoublant] = useState(false);
  const [origine, setOrigine] = useState("");
  const [choix, setChoix] = useState<ChoixFrais>({});
  const peutValider = commandeAutorisee("inscription_valider", permissions);
  const classe = classes.find((c) => c.classe.id === classeId)?.classe;
  const inscription = useMutation({
    mutationFn: async (valider: boolean) => {
      const brouillon = await appeler("inscription_creer", { eleve_id: eleveId, classe_id: classeId, date_inscription: "", redoublant, etablissement_origine: origine });
      return valider ? appeler("inscription_valider", { id: brouillon.inscription.id, choix }) : brouillon;
    },
    onMutate: () => surRetour(SANS_RETOUR),
    onSuccess: (resultat) => surRetour({ erreur: null, avertissements: resultat.avertissements }),
    onError: (erreur) => surRetour({ erreur, avertissements: [] }),
    onSettled: rafraichir,
  });
  if (courante === undefined) return null;
  return (
    <Formulaire surEnvoi={() => inscription.mutate(peutValider)}>
      <h3 className="text-sm font-semibold text-encre">{t("inscriptions.nouvelle", { annee: courante.libelle })}</h3>
      <Selection
        id="inscription-classe"
        libelle={t("inscriptions.classe")}
        valeur={classeId}
        surChangement={(v) => {
          setClasseId(v);
          setChoix({});
        }}
        options={[{ valeur: "", libelle: t("inscriptions.choisir_classe") }, ...classes.map((c) => ({ valeur: c.classe.id, libelle: `${c.classe.libelle} · ${c.niveau}` }))]}
      />
      {peutValider && classe !== undefined ? (
        <ChoixFraisChamps prefixe="inscription" anneeId={courante.id} niveauId={classe.niveau_id} serieId={classe.serie_id} valeur={choix} surChangement={setChoix} />
      ) : null}
      <CaseACocher id="inscription-redoublant" libelle={t("inscriptions.redoublant")} coche={redoublant} surChangement={setRedoublant} />
      <Champ id="inscription-origine" libelle={t("inscriptions.origine")} aide={t("inscriptions.origine_aide")} value={origine} onChange={(e) => setOrigine(e.target.value)} />
      <div className="flex flex-wrap gap-2">
        {peutValider ? <Bouton type="submit" disabled={inscription.isPending}>{t("inscriptions.inscrire_et_valider")}</Bouton> : null}
        <Bouton variante={peutValider ? "secondaire" : "principal"} onClick={() => inscription.mutate(false)} disabled={inscription.isPending}>
          {t("inscriptions.enregistrer_brouillon")}
        </Bouton>
      </div>
    </Formulaire>
  );
}

function ActionsInscription({
  eleveId,
  inscription,
  permissions,
  surRetour,
  depart,
}: {
  eleveId: string;
  inscription: InscriptionAffichee;
  permissions: readonly string[];
  surRetour: (r: Retour) => void;
  /** Transfert et exclusion, rangés avec les autres actions rares. */
  depart: ReactNode;
}) {
  const rafraichir = useRafraichir(eleveId);
  const { classes } = useClassesCourantes();
  const [motif, setMotif] = useState("");
  const [classeId, setClasseId] = useState("");
  const [choix, setChoix] = useState<ChoixFrais>({});
  const id = inscription.inscription.id;
  const classe = classes.find((c) => c.classe.id === inscription.inscription.classe_id)?.classe;
  const suivi = {
    onMutate: () => surRetour(SANS_RETOUR),
    onError: (erreur: unknown) => surRetour({ erreur, avertissements: [] }),
    onSettled: rafraichir,
  };
  const brouillon = inscription.inscription.statut === "brouillon";
  const validation = useMutation({
    mutationFn: () => appeler("inscription_valider", { id, choix }),
    ...suivi,
    onSuccess: (r) => surRetour({ erreur: null, avertissements: r.avertissements }),
  });
  const abandon = useMutation({ mutationFn: () => appeler("inscription_abandonner", { id, motif }), ...suivi });
  const changement = useMutation({ mutationFn: () => appeler("inscription_changer_classe", { id, classe_id: classeId, motif }), ...suivi });
  return (
    <div className="mt-2 flex flex-col gap-2">
      {brouillon && commandeAutorisee("inscription_valider", permissions) ? (
        <>
          {classe === undefined ? null : (
            <ChoixFraisChamps prefixe={`validation-${id}`} anneeId={inscription.inscription.annee_id} niveauId={classe.niveau_id} serieId={classe.serie_id} valeur={choix} surChangement={setChoix} />
          )}
          <div><Bouton onClick={() => validation.mutate()} disabled={validation.isPending}>{t("inscriptions.valider")}</Bouton></div>
        </>
      ) : null}
      {/* Ce qui ne sert pas tous les jours est replié : changer de classe, abandonner, partir. */}
      <details className="rounded border border-muet/30 p-2">
        <summary className="cursor-pointer text-sm text-primaire">{t("inscriptions.autres_actions")}</summary>
        <div className="mt-2 flex flex-col gap-2">
      <Champ id={`motif-${id}`} libelle={t("inscriptions.motif")} aide={t("inscriptions.motif_aide")} value={motif} onChange={(e) => setMotif(e.target.value)} />
      {!brouillon && commandeAutorisee("inscription_changer_classe", permissions) ? (
        <div className="flex flex-wrap items-end gap-2">
          <Selection
            id={`nouvelle-classe-${id}`}
            libelle={t("inscriptions.changer_classe")}
            valeur={classeId}
            surChangement={setClasseId}
            options={[{ valeur: "", libelle: t("inscriptions.choisir_classe") }, ...classes.filter((c) => c.classe.id !== inscription.inscription.classe_id).map((c) => ({ valeur: c.classe.id, libelle: c.classe.libelle }))]}
          />
          <Bouton variante="secondaire" onClick={() => changement.mutate()} disabled={changement.isPending || classeId === ""}>{t("inscriptions.changer")}</Bouton>
        </div>
      ) : null}
      {commandeAutorisee("inscription_abandonner", permissions) ? (
        <div><Bouton variante="danger" onClick={() => abandon.mutate()} disabled={abandon.isPending}>{t("inscriptions.abandonner")}</Bouton></div>
      ) : null}
      {depart}
        </div>
      </details>
    </div>
  );
}
