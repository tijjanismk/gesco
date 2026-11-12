import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { Bouton } from "../../composants/Bouton";
import { CaseACocher, Champ, Selection } from "../../composants/Champ";
import { MessageEnregistre, MessageErreur, Section } from "../../composants/Messages";
import { t } from "../../i18n";
import type { Competence, CompetencesJardin as Donnees, DomaineCompetence } from "../../types/gesco";

/** Réglages des trois libellés de l'échelle, dans l'ordre acquis, en cours, non acquis. */
const ECHELLE = ["jardin.echelle_acquis", "jardin.echelle_en_cours", "jardin.echelle_non_acquis"] as const;

function maintenant(): number {
  return Date.now();
}

/** Compétences du jardin (SPEC_PHASE_11_B §4.1) : une liste par section, rangée par domaine, que
 *  la direction modifie librement ; rien ne se supprime, une compétence retirée se remet. Absent
 *  pour une école sans jardin. */
export function CompetencesJardin() {
  const donnees = useQuery({ queryKey: ["jardin"], queryFn: () => appeler("jardin_lire", null) });
  const [sectionChoisie, setSection] = useState<string | null>(null);
  const [retirees, setRetirees] = useState(false);
  const sections = donnees.data?.sections ?? [];
  if (donnees.data === undefined || sections.length === 0) return <MessageErreur erreur={donnees.error} />;
  const section = sections.find((s) => s.id === sectionChoisie) ?? sections[0];
  if (section === undefined) return null;
  const domaines = donnees.data.domaines.filter((d) => d.retire_le === null);
  return (
    <Section titre={t("jardin.titre")}>
      <p className="text-sm text-muet">{t("jardin.aide")}</p>
      <Echelle donnees={donnees.data} />
      <div className="flex flex-wrap items-center gap-2" role="tablist" aria-label={t("jardin.section")}>
        {sections.map((s) => (
          <Bouton key={s.id} role="tab" aria-selected={s.id === section.id} variante={s.id === section.id ? "principal" : "secondaire"} onClick={() => setSection(s.id)}>
            {s.libelle}
          </Bouton>
        ))}
      </div>
      {domaines.map((d, i) => (
        <BlocDomaine key={d.id} domaine={d} premier={i === 0} dernier={i === domaines.length - 1} sectionId={section.id} competences={donnees.data.competences} retirees={retirees} />
      ))}
      <NouveauDomaine />
      <div className="flex flex-col gap-2 border-t border-muet/30 pt-3">
        <CaseACocher id="jardin-retirees" libelle={t("jardin.afficher_retirees")} coche={retirees} surChangement={setRetirees} />
        {retirees ? <DomainesRetires domaines={donnees.data.domaines.filter((d) => d.retire_le !== null)} /> : null}
      </div>
      <Recopie sections={sections.filter((s) => s.id !== section.id).map((s) => ({ valeur: s.id, libelle: s.libelle }))} cibleId={section.id} />
    </Section>
  );
}

function useRafraichir() {
  const client = useQueryClient();
  return () => void client.invalidateQueries({ queryKey: ["jardin"] });
}

/** Les libellés des trois niveaux et le commentaire par domaine, enregistrés ensemble. */
function Echelle({ donnees }: { donnees: Donnees }) {
  const client = useQueryClient();
  const cle = `${donnees.echelle.join("|")}|${String(donnees.commentaire_domaine)}`;
  const [saisie, setSaisie] = useState<{ cle: string; libelles: string[]; commentaire: boolean }>({ cle, libelles: donnees.echelle, commentaire: donnees.commentaire_domaine });
  const courante = saisie.cle === cle ? saisie : { cle, libelles: donnees.echelle, commentaire: donnees.commentaire_domaine };
  const [enregistre, setEnregistre] = useState<number | null>(null);
  const enregistrer = useMutation({
    mutationFn: async () => {
      for (const [i, parametre] of ECHELLE.entries()) {
        await appeler("parametre_definir", { cle: parametre, valeur: { type: "texte", valeur: courante.libelles[i] ?? "" } });
      }
      await appeler("parametre_definir", { cle: "jardin.commentaire_domaine", valeur: { type: "booleen", valeur: courante.commentaire } });
    },
    onSuccess: () => {
      setEnregistre(maintenant());
      void client.invalidateQueries({ queryKey: ["jardin"] });
      void client.invalidateQueries({ queryKey: ["parametres"] });
    },
  });
  return (
    <div className="flex flex-col gap-2 rounded border border-muet/30 p-3">
      <h3 className="text-sm font-semibold text-encre">{t("jardin.echelle_titre")}</h3>
      <p className="text-sm text-muet">{t("jardin.echelle_aide")}</p>
      <div className="flex flex-wrap items-end gap-3">
        {ECHELLE.map((parametre, i) => (
          <Champ
            key={parametre}
            id={`param-${parametre}`}
            libelle={t(parametre)}
            value={courante.libelles[i] ?? ""}
            maxLength={30}
            onChange={(e) => setSaisie({ ...courante, libelles: courante.libelles.map((l, j) => (j === i ? e.target.value : l)) })}
          />
        ))}
      </div>
      <CaseACocher id="param-jardin.commentaire_domaine" libelle={t("jardin.commentaire_domaine")} coche={courante.commentaire} surChangement={(c) => setSaisie({ ...courante, commentaire: c })} />
      <div className="flex flex-wrap items-center gap-3">
        <Bouton variante="secondaire" onClick={() => enregistrer.mutate()} disabled={enregistrer.isPending}>{t("commun.enregistrer")}</Bouton>
        <MessageErreur erreur={enregistrer.error} />
        <MessageEnregistre instant={enregistre} />
      </div>
    </div>
  );
}

function BlocDomaine({ domaine, premier, dernier, sectionId, competences, retirees }: { domaine: DomaineCompetence; premier: boolean; dernier: boolean; sectionId: string; competences: readonly Competence[]; retirees: boolean }) {
  const rafraichir = useRafraichir();
  const [nom, setNom] = useState<string | null>(null);
  const [nouvelle, setNouvelle] = useState("");
  const deplacer = useMutation({
    mutationFn: (monter: boolean) => appeler("domaine_competence_deplacer", { id: domaine.id, monter }),
    onSuccess: rafraichir,
  });
  const enregistrer = useMutation({
    mutationFn: ({ libelle, retire }: { libelle: string; retire: boolean }) => appeler("domaine_competence_enregistrer", { id: domaine.id, libelle, retire }),
    onSuccess: () => {
      setNom(null);
      rafraichir();
    },
  });
  const ajouter = useMutation({
    mutationFn: () => appeler("competence_enregistrer", { id: null, domaine_id: domaine.id, niveau_id: sectionId, libelle: nouvelle, retiree: false }),
    onSuccess: () => {
      setNouvelle("");
      rafraichir();
    },
  });
  const de_la_section = competences.filter((c) => c.domaine_id === domaine.id && c.niveau_id === sectionId).sort((a, b) => a.ordre - b.ordre);
  const actives = de_la_section.filter((c) => c.retiree_le === null);
  const enRetrait = de_la_section.filter((c) => c.retiree_le !== null);
  return (
    <div className="flex flex-col gap-2 rounded border border-muet/30 p-3">
      <div className="flex flex-wrap items-end gap-2">
        {nom === null ? (
          <h3 className="mr-auto text-sm font-semibold text-encre">{domaine.libelle}</h3>
        ) : (
          <div className="mr-auto flex flex-wrap items-end gap-2">
            <Champ id={`domaine-${domaine.id}-libelle`} libelle={t("jardin.libelle_domaine")} value={nom} maxLength={60} onChange={(e) => setNom(e.target.value)} />
            <Bouton onClick={() => enregistrer.mutate({ libelle: nom, retire: false })} disabled={enregistrer.isPending}>{t("commun.enregistrer")}</Bouton>
            <Bouton variante="secondaire" onClick={() => setNom(null)}>{t("commun.annuler")}</Bouton>
          </div>
        )}
        <Bouton variante="secondaire" aria-label={t("jardin.monter_nom", { nom: domaine.libelle })} onClick={() => deplacer.mutate(true)} disabled={premier || deplacer.isPending}>↑</Bouton>
        <Bouton variante="secondaire" aria-label={t("jardin.descendre_nom", { nom: domaine.libelle })} onClick={() => deplacer.mutate(false)} disabled={dernier || deplacer.isPending}>↓</Bouton>
        {nom === null ? <Bouton variante="secondaire" onClick={() => setNom(domaine.libelle)}>{t("jardin.renommer")}</Bouton> : null}
        <Bouton variante="secondaire" onClick={() => enregistrer.mutate({ libelle: domaine.libelle, retire: true })} disabled={enregistrer.isPending}>{t("jardin.retirer_domaine")}</Bouton>
      </div>
      <ol className="flex flex-col">
        {actives.map((c, i) => (
          <LigneCompetence key={c.id} competence={c} premiere={i === 0} derniere={i === actives.length - 1} />
        ))}
      </ol>
      {retirees && enRetrait.length > 0 ? (
        <ul className="flex flex-col" aria-label={t("jardin.retirees")}>
          {enRetrait.map((c) => (
            <LigneRetiree key={c.id} competence={c} />
          ))}
        </ul>
      ) : null}
      <div className="flex flex-wrap items-end gap-2">
        <div className="min-w-72 flex-1">
          <Champ id={`domaine-${domaine.id}-nouvelle`} libelle={t("jardin.nouvelle_competence")} value={nouvelle} maxLength={120} onChange={(e) => setNouvelle(e.target.value)} />
        </div>
        <Bouton variante="secondaire" onClick={() => ajouter.mutate()} disabled={ajouter.isPending || nouvelle.trim() === ""}>{t("jardin.ajouter")}</Bouton>
      </div>
      <MessageErreur erreur={deplacer.error ?? enregistrer.error ?? ajouter.error} />
    </div>
  );
}

function LigneCompetence({ competence, premiere, derniere }: { competence: Competence; premiere: boolean; derniere: boolean }) {
  const rafraichir = useRafraichir();
  const [nom, setNom] = useState<string | null>(null);
  const deplacer = useMutation({
    mutationFn: (monter: boolean) => appeler("competence_deplacer", { id: competence.id, monter }),
    onSuccess: rafraichir,
  });
  const enregistrer = useMutation({
    mutationFn: ({ libelle, retiree }: { libelle: string; retiree: boolean }) =>
      appeler("competence_enregistrer", { id: competence.id, domaine_id: competence.domaine_id, niveau_id: competence.niveau_id, libelle, retiree }),
    onSuccess: () => {
      setNom(null);
      rafraichir();
    },
  });
  return (
    <li className="flex flex-col gap-1 border-b border-muet/20 py-1.5">
      <div className="flex items-center gap-2">
        {nom === null ? (
          <span className="min-w-0 flex-1 text-sm text-encre">{competence.libelle}</span>
        ) : (
          <div className="mr-auto flex min-w-72 flex-1 flex-wrap items-end gap-2">
            <div className="flex-1">
              <Champ id={`competence-${competence.id}-libelle`} libelle={t("jardin.libelle_competence")} value={nom} maxLength={120} onChange={(e) => setNom(e.target.value)} />
            </div>
            <Bouton onClick={() => enregistrer.mutate({ libelle: nom, retiree: false })} disabled={enregistrer.isPending}>{t("commun.enregistrer")}</Bouton>
            <Bouton variante="secondaire" onClick={() => setNom(null)}>{t("commun.annuler")}</Bouton>
          </div>
        )}
        <div className="flex shrink-0 gap-2">
          <Bouton variante="secondaire" aria-label={t("jardin.monter_nom", { nom: competence.libelle })} onClick={() => deplacer.mutate(true)} disabled={premiere || deplacer.isPending}>↑</Bouton>
          <Bouton variante="secondaire" aria-label={t("jardin.descendre_nom", { nom: competence.libelle })} onClick={() => deplacer.mutate(false)} disabled={derniere || deplacer.isPending}>↓</Bouton>
          {nom === null ? <Bouton variante="secondaire" aria-label={t("jardin.renommer_nom", { nom: competence.libelle })} onClick={() => setNom(competence.libelle)}>{t("jardin.renommer")}</Bouton> : null}
          <Bouton variante="secondaire" aria-label={t("jardin.retirer_nom", { nom: competence.libelle })} onClick={() => enregistrer.mutate({ libelle: competence.libelle, retiree: true })} disabled={enregistrer.isPending}>{t("jardin.retirer")}</Bouton>
        </div>
      </div>
      <MessageErreur erreur={deplacer.error ?? enregistrer.error} />
    </li>
  );
}

function LigneRetiree({ competence }: { competence: Competence }) {
  const rafraichir = useRafraichir();
  const remettre = useMutation({
    mutationFn: () => appeler("competence_enregistrer", { id: competence.id, domaine_id: competence.domaine_id, niveau_id: competence.niveau_id, libelle: competence.libelle, retiree: false }),
    onSuccess: rafraichir,
  });
  return (
    <li className="flex items-center gap-2 border-b border-muet/20 py-1.5">
      <span className="min-w-0 flex-1 text-sm text-muet">{competence.libelle}</span>
      <Bouton variante="secondaire" aria-label={t("jardin.remettre_nom", { nom: competence.libelle })} onClick={() => remettre.mutate()} disabled={remettre.isPending}>{t("jardin.remettre")}</Bouton>
      <MessageErreur erreur={remettre.error} />
    </li>
  );
}

function DomainesRetires({ domaines }: { domaines: readonly DomaineCompetence[] }) {
  const rafraichir = useRafraichir();
  const remettre = useMutation({
    mutationFn: (d: DomaineCompetence) => appeler("domaine_competence_enregistrer", { id: d.id, libelle: d.libelle, retire: false }),
    onSuccess: rafraichir,
  });
  if (domaines.length === 0) return null;
  return (
    <>
      <ul className="flex flex-col" aria-label={t("jardin.domaines_retires")}>
        {domaines.map((d) => (
          <li key={d.id} className="flex flex-wrap items-center gap-2 border-b border-muet/20 py-1.5">
            <span className="mr-auto text-sm text-muet">{d.libelle}</span>
            <Bouton variante="secondaire" aria-label={t("jardin.remettre_nom", { nom: d.libelle })} onClick={() => remettre.mutate(d)} disabled={remettre.isPending}>{t("jardin.remettre")}</Bouton>
          </li>
        ))}
      </ul>
      <MessageErreur erreur={remettre.error} />
    </>
  );
}

function NouveauDomaine() {
  const rafraichir = useRafraichir();
  const [libelle, setLibelle] = useState("");
  const ajouter = useMutation({
    mutationFn: () => appeler("domaine_competence_enregistrer", { id: null, libelle, retire: false }),
    onSuccess: () => {
      setLibelle("");
      rafraichir();
    },
  });
  return (
    <div className="flex flex-wrap items-end gap-2">
      <Champ id="jardin-nouveau-domaine" libelle={t("jardin.nouveau_domaine")} value={libelle} maxLength={60} onChange={(e) => setLibelle(e.target.value)} />
      <Bouton variante="secondaire" onClick={() => ajouter.mutate()} disabled={ajouter.isPending || libelle.trim() === ""}>{t("jardin.ajouter")}</Bouton>
      <MessageErreur erreur={ajouter.error} />
    </div>
  );
}

/** Prend les compétences d'une autre section pour partir d'elles ; rien n'est doublé. */
function Recopie({ sections, cibleId }: { sections: readonly { valeur: string; libelle: string }[]; cibleId: string }) {
  const rafraichir = useRafraichir();
  const [sourceChoisie, setSource] = useState<string | null>(null);
  const sourceId = sections.some((s) => s.valeur === sourceChoisie) ? sourceChoisie : (sections[0]?.valeur ?? null);
  const recopier = useMutation({
    mutationFn: () => appeler("competences_recopier", { source_id: sourceId ?? "", cible_id: cibleId }),
    onSuccess: rafraichir,
  });
  if (sourceId === null) return null;
  return (
    <div className="flex flex-wrap items-end gap-2 border-t border-muet/30 pt-3">
      <Selection id="jardin-recopie-source" libelle={t("jardin.recopier_depuis")} valeur={sourceId} surChangement={setSource} options={sections} />
      <Bouton variante="secondaire" onClick={() => recopier.mutate()} disabled={recopier.isPending}>{t("jardin.recopier")}</Bouton>
      {recopier.data === undefined ? null : <p role="status" className="text-sm text-vert">{t("jardin.recopiees", { nombre: String(recopier.data) })}</p>}
      <MessageErreur erreur={recopier.error} />
    </div>
  );
}
