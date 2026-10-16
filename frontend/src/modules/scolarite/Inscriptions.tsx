import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { commandeAutorisee } from "../../app/navigation";
import { Bouton } from "../../composants/Bouton";
import { Champ, Selection } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { Tableau } from "../../composants/Tableau";
import { dateCivile, t, texteDeCle } from "../../i18n";
import type { StatutInscription } from "../../types/gesco";
import { EtapeFrais } from "../finance/EtapeFrais";
import { NouvelEleve } from "./Eleves";
import { InscriptionsEleve, STATUTS, TYPES } from "./InscriptionsEleve";
import { ResponsablesEleve } from "./Responsables";

/** Inscriptions de l'année (SPEC_PHASE_2 §5) et assistant d'inscription (UC-01, §19.4). */
export function Inscriptions({ surOuvrirEleve }: { surOuvrirEleve: (id: string) => void }) {
  const session = useQuery({ queryKey: ["session"], queryFn: () => appeler("session_courante", null) });
  const annees = useQuery({ queryKey: ["annees"], queryFn: () => appeler("annees_lister", null) });
  const [annee, setAnnee] = useState<string | null>(null);
  const anneeId = annee ?? annees.data?.find((a) => a.est_courante)?.id ?? annees.data?.[0]?.id ?? null;
  const [classe, setClasse] = useState("");
  const [statut, setStatut] = useState("");
  const [assistant, setAssistant] = useState(false);
  const liste = useQuery({
    queryKey: ["inscriptions", anneeId],
    queryFn: () => appeler("inscriptions_lister", { annee_id: anneeId ?? "" }),
    enabled: anneeId !== null,
  });
  const permissions = session.data?.permissions ?? [];
  const lignes = (liste.data ?? []).filter((l) => (classe === "" || l.classe === classe) && (statut === "" || l.inscription.statut === statut));
  const classes = [...new Set((liste.data ?? []).map((l) => l.classe))];
  if (assistant) return <AssistantInscription permissions={permissions} surTermine={() => setAssistant(false)} />;
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("inscriptions.titre")}</h1>
      <div className="flex flex-wrap items-end gap-3">
        <Selection
          id="inscriptions-annee"
          libelle={t("classes.annee")}
          valeur={anneeId ?? ""}
          surChangement={setAnnee}
          options={(annees.data ?? []).map((a) => ({ valeur: a.id, libelle: a.libelle }))}
        />
        <Selection
          id="inscriptions-classe"
          libelle={t("inscriptions.classe")}
          valeur={classe}
          surChangement={setClasse}
          options={[{ valeur: "", libelle: t("journal.toutes") }, ...classes.map((c) => ({ valeur: c, libelle: c }))]}
        />
        <Selection
          id="inscriptions-statut"
          libelle={t("inscriptions.statut")}
          valeur={statut}
          surChangement={setStatut}
          options={[{ valeur: "", libelle: t("journal.tous") }, ...(Object.keys(STATUTS) as StatutInscription[]).map((s) => ({ valeur: s, libelle: t(STATUTS[s]) }))]}
        />
        {commandeAutorisee("inscription_creer", permissions) ? <Bouton onClick={() => setAssistant(true)}>{t("inscriptions.assistant")}</Bouton> : null}
      </div>
      <MessageErreur erreur={annees.error ?? liste.error} />
      {classe !== "" && commandeAutorisee("inscriptions_valider_classe", permissions) ? (
        <ValidationEnLot classeId={lignes[0]?.inscription.classe_id ?? null} brouillons={lignes.filter((l) => l.inscription.statut === "brouillon").length} />
      ) : null}
      <Tableau
        nomExport="inscriptions"
        titre={t("inscriptions.titre")}
        lignes={lignes}
        cle={(l) => l.inscription.id}
        surSelection={(l) => surOuvrirEleve(l.inscription.eleve_id)}
        colonnes={[
          { titre: t("inscriptions.classe"), valeur: (l) => l.classe },
          { titre: t("eleves.matricule"), valeur: (l) => l.matricule },
          { titre: t("inscriptions.eleve"), valeur: (l) => l.eleve },
          { titre: t("inscriptions.statut"), valeur: (l) => t(STATUTS[l.inscription.statut]) },
          { titre: t("inscriptions.type"), valeur: (l) => t(TYPES[l.inscription.type_inscription]) },
          { titre: t("inscriptions.date"), valeur: (l) => dateCivile(l.inscription.date_inscription) },
        ]}
      />
    </div>
  );
}

const ETAPES = ["inscriptions.etape.eleve", "inscriptions.etape.responsables", "inscriptions.etape.classe", "inscriptions.etape.frais"] as const;

/** Assistant UC-01 : élève (retrouvé ou créé), responsables, classe et validation, puis frais et
 *  premier versement (SPEC_PHASE_3 §10), sans quitter l'écran. Chaque étape réutilise les écrans de la fiche élève : une seule règle, au noyau. */
function AssistantInscription({ permissions, surTermine }: { permissions: readonly string[]; surTermine: () => void }) {
  const [etape, setEtape] = useState(0);
  const [eleveId, setEleveId] = useState<string | null>(null);
  const [recherche, setRecherche] = useState("");
  const trouves = useQuery({
    queryKey: ["assistant-eleves", recherche],
    queryFn: () => appeler("eleves_rechercher", { texte: recherche }),
    enabled: recherche.trim().length >= 2,
  });
  const fiche = useQuery({ queryKey: ["eleve", eleveId], queryFn: () => appeler("eleve_lire", { id: eleveId ?? "" }), enabled: eleveId !== null });
  const choisir = (id: string) => {
    setEleveId(id);
    setEtape(1);
  };
  return (
    <div className="flex max-w-3xl flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("inscriptions.assistant")}</h1>
      <ol className="flex flex-wrap gap-4 text-sm">
        {ETAPES.map((cle, i) => (
          <li key={cle} className={i === etape ? "font-semibold text-primaire" : "text-muet"}>{`${i + 1}. ${t(cle)}`}</li>
        ))}
      </ol>
      {fiche.data === undefined ? null : <p className="text-sm text-encre">{t("inscriptions.pour", { eleve: `${fiche.data.identite.prenoms} ${fiche.data.identite.nom}`, matricule: fiche.data.eleve.matricule_interne })}</p>}
      {etape === 0 ? (
        <>
          <Section titre={t("inscriptions.eleve_connu")}>
            <Champ id="assistant-recherche" type="search" autoFocus libelle={t("eleves.rechercher")} aide={t("eleves.rechercher_aide")} value={recherche} onChange={(e) => setRecherche(e.target.value)} />
            <ul className="flex flex-col gap-1">
              {(trouves.data ?? []).slice(0, 10).map((e) => (
                <li key={e.id}>
                  <button type="button" className="text-sm text-primaire underline" onClick={() => choisir(e.id)}>
                    {`${e.prenoms} ${e.nom} · ${e.matricule_interne}${e.date_naissance === null ? "" : ` · ${dateCivile(e.date_naissance)}`}`}
                  </button>
                </li>
              ))}
            </ul>
          </Section>
          <NouvelEleve surCree={choisir} />
        </>
      ) : null}
      {etape === 1 && fiche.data !== undefined ? <ResponsablesEleve fiche={fiche.data} modifiable={commandeAutorisee("responsable_rattacher", permissions)} /> : null}
      {etape === 2 && fiche.data !== undefined ? <InscriptionsEleve fiche={fiche.data} permissions={permissions} /> : null}
      {etape === 3 && eleveId !== null ? <EtapeFrais eleveId={eleveId} permissions={permissions} /> : null}
      <div className="flex flex-wrap gap-2">
        {etape > 0 ? <Bouton variante="secondaire" onClick={() => setEtape(etape - 1)}>{t("inscriptions.precedent")}</Bouton> : null}
        {etape === 1 || etape === 2 ? <Bouton onClick={() => setEtape(etape + 1)}>{t("commun.continuer")}</Bouton> : null}
        <Bouton variante="secondaire" onClick={surTermine}>{t(etape >= 2 ? "inscriptions.terminer" : "commun.annuler")}</Bouton>
      </div>
    </div>
  );
}

/** Valide d'un coup les brouillons d'une classe, après un import (SPEC_PHASE_4, Q-62) : chaque
 *  brouillon refusé reste brouillon, avec sa raison. */
function ValidationEnLot({ classeId, brouillons }: { classeId: string | null; brouillons: number }) {
  const client = useQueryClient();
  const validation = useMutation({
    mutationFn: () => appeler("inscriptions_valider_classe", { id: classeId ?? "" }),
    onSuccess: () => void client.invalidateQueries({ queryKey: ["inscriptions"] }),
  });
  if (classeId === null) return null;
  return (
    <div className="flex flex-col gap-2">
      {brouillons === 0 ? null : (
        <div>
          <Bouton variante="secondaire" onClick={() => validation.mutate()} disabled={validation.isPending}>
            {t("inscriptions.valider_en_lot", { nombre: String(brouillons) })}
          </Bouton>
        </div>
      )}
      {validation.data === undefined ? null : (
        <div role="status" className="text-sm text-encre">
          <p>{t("inscriptions.bilan_lot", { validees: String(validation.data.validees), refusees: String(validation.data.refusees.length) })}</p>
          <ul className="list-disc pl-5 text-muet">
            {validation.data.refusees.map((r) => (
              <li key={r.eleve}>{`${r.eleve} : ${texteDeCle(r.cle)}`}</li>
            ))}
          </ul>
        </div>
      )}
      <MessageErreur erreur={validation.error} />
    </div>
  );
}
