import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { commandeAutorisee } from "../../app/navigation";
import { Bouton } from "../../composants/Bouton";
import { CaseACocher, Champ, Selection } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { Tableau } from "../../composants/Tableau";
import { type Cle, t } from "../../i18n";
import type { FicheEnseignant, StatutEnseignant } from "../../types/gesco";
import { telephoneLisible } from "../scolarite/Identite";

const STATUTS: Record<StatutEnseignant, Cle> = {
  titulaire: "enseignant.statut.titulaire",
  vacataire: "enseignant.statut.vacataire",
};

const AUCUN = "";

function useRafraichir() {
  const client = useQueryClient();
  return () => {
    for (const cle of ["enseignants", "affectations"]) void client.invalidateQueries({ queryKey: [cle] });
  };
}

/** Enseignants et affectations (SPEC_PHASE_4 §3) : qui enseigne quelle matière dans quelle classe. */
export function Enseignants() {
  const session = useQuery({ queryKey: ["session"], queryFn: () => appeler("session_courante", null) });
  const annees = useQuery({ queryKey: ["annees"], queryFn: () => appeler("annees_lister", null) });
  const [annee, setAnnee] = useState<string | null>(null);
  const anneeId = annee ?? annees.data?.find((a) => a.est_courante)?.id ?? annees.data?.[0]?.id ?? null;
  const enseignants = useQuery({ queryKey: ["enseignants"], queryFn: () => appeler("enseignants_lister", null) });
  const permissions = session.data?.permissions ?? [];
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("enseignants.titre")}</h1>
      <MessageErreur erreur={enseignants.error} />
      <ListeEnseignants fiches={enseignants.data ?? []} voitComptes={commandeAutorisee("utilisateurs_lister", permissions)} />
      <div className="w-48">
      <Selection
        id="affectations-annee"
        libelle={t("pedagogie.annee")}
        valeur={anneeId ?? ""}
        surChangement={setAnnee}
        options={(annees.data ?? []).map((a) => ({ valeur: a.id, libelle: a.libelle }))}
      />
      </div>
      {anneeId === null ? null : <Affectations anneeId={anneeId} fiches={(enseignants.data ?? []).filter((f) => f.enseignant.actif)} />}
    </div>
  );
}

const FICHE_VIDE = { id: null as string | null, nom: "", prenoms: "", telephone: "", statut: "titulaire" as StatutEnseignant, compte: AUCUN, actif: true };

function ListeEnseignants({ fiches, voitComptes }: { fiches: readonly FicheEnseignant[]; voitComptes: boolean }) {
  const rafraichir = useRafraichir();
  const comptes = useQuery({ queryKey: ["utilisateurs"], queryFn: () => appeler("utilisateurs_lister", null), enabled: voitComptes });
  const [fiche, setFiche] = useState(FICHE_VIDE);
  const enregistrer = useMutation({
    mutationFn: () =>
      appeler("enseignant_enregistrer", {
        id: fiche.id,
        nom: fiche.nom,
        prenoms: fiche.prenoms,
        telephones: fiche.telephone.trim() === "" ? [] : [fiche.telephone],
        statut: fiche.statut,
        utilisateur_id: fiche.compte === AUCUN ? null : fiche.compte,
        actif: fiche.actif,
      }),
    onSuccess: () => {
      setFiche(FICHE_VIDE);
      rafraichir();
    },
  });
  return (
    <Section titre={t("enseignants.liste")}>
      <Tableau
        nomExport="enseignants"
        titre={t("enseignants.liste")}
        lignes={fiches}
        cle={(f) => f.enseignant.id}
        surSelection={(f) =>
          setFiche({
            id: f.enseignant.id,
            nom: f.nom,
            prenoms: f.prenoms,
            telephone: f.telephones[0] === undefined ? "" : telephoneLisible(f.telephones[0]),
            statut: f.enseignant.statut,
            compte: f.enseignant.utilisateur_id ?? AUCUN,
            actif: f.enseignant.actif,
          })
        }
        colonnes={[
          { titre: t("enseignants.prenoms"), valeur: (f) => f.prenoms },
          { titre: t("enseignants.nom"), valeur: (f) => f.nom },
          { titre: t("enseignants.statut"), valeur: (f) => t(STATUTS[f.enseignant.statut]) },
          { titre: t("enseignants.telephone"), valeur: (f) => f.telephones.map(telephoneLisible).join(", ") },
          { titre: t("enseignants.compte"), valeur: (f) => f.compte ?? "" },
          { titre: t("matiere.etat"), valeur: (f) => t(f.enseignant.actif ? "enseignants.actif" : "enseignants.parti") },
        ]}
      />
      <h3 className="text-base font-semibold text-encre">{t(fiche.id === null ? "enseignants.nouveau" : "enseignants.modifier")}</h3>
      <div className="flex flex-wrap items-end gap-3">
        <Champ id="enseignant-prenoms" libelle={t("enseignants.prenoms")} value={fiche.prenoms} onChange={(e) => setFiche({ ...fiche, prenoms: e.target.value })} />
        <Champ id="enseignant-nom" libelle={t("enseignants.nom")} value={fiche.nom} onChange={(e) => setFiche({ ...fiche, nom: e.target.value })} />
        <Champ id="enseignant-telephone" libelle={t("enseignants.telephone")} value={fiche.telephone} onChange={(e) => setFiche({ ...fiche, telephone: e.target.value })} />
        <Selection id="enseignant-statut" libelle={t("enseignants.statut")} valeur={fiche.statut} surChangement={(s) => setFiche({ ...fiche, statut: s as StatutEnseignant })} options={Object.entries(STATUTS).map(([v, c]) => ({ valeur: v, libelle: t(c) }))} />
        {voitComptes ? (
          <Selection
            id="enseignant-compte"
            libelle={t("enseignants.compte")}
            valeur={fiche.compte}
            surChangement={(c) => setFiche({ ...fiche, compte: c })}
            options={[{ valeur: AUCUN, libelle: t("enseignants.sans_compte") }, ...(comptes.data ?? []).map((u) => ({ valeur: u.utilisateur.id, libelle: u.utilisateur.nom_affiche }))]}
          />
        ) : null}
        <CaseACocher id="enseignant-actif" libelle={t("enseignants.actif")} coche={fiche.actif} surChangement={(c) => setFiche({ ...fiche, actif: c })} />
      </div>
      <p className="text-sm text-muet">{t("enseignants.aide_compte")}</p>
      <MessageErreur erreur={enregistrer.error} />
      <div className="flex flex-wrap gap-2">
        <Bouton onClick={() => enregistrer.mutate()} disabled={enregistrer.isPending}>{t("commun.enregistrer")}</Bouton>
        {fiche.id === null ? null : <Bouton variante="secondaire" onClick={() => setFiche(FICHE_VIDE)}>{t("commun.annuler")}</Bouton>}
      </div>
    </Section>
  );
}

function Affectations({ anneeId, fiches }: { anneeId: string; fiches: readonly FicheEnseignant[] }) {
  const rafraichir = useRafraichir();
  const classes = useQuery({ queryKey: ["classes", anneeId], queryFn: () => appeler("classes_lister", { annee_id: anneeId }) });
  const pedagogie = useQuery({ queryKey: ["pedagogie", anneeId], queryFn: () => appeler("pedagogie_lire", { id: anneeId }) });
  const affectations = useQuery({ queryKey: ["affectations", anneeId], queryFn: () => appeler("affectations_lister", { id: anneeId }) });
  const [classeChoisie, setClasseChoisie] = useState<string | null>(null);
  const classe = (classes.data ?? []).find((c) => c.classe.id === classeChoisie) ?? classes.data?.[0];
  const [choix, setChoix] = useState<Record<string, string>>({});
  const [maitre, setMaitre] = useState(AUCUN);
  const options = [{ valeur: AUCUN, libelle: t("enseignants.choisir") }, ...fiches.map((f) => ({ valeur: f.enseignant.id, libelle: `${f.prenoms} ${f.nom}` }))];
  const affecter = useMutation({
    mutationFn: ({ matiere, seul }: { matiere: string; seul: boolean }) =>
      appeler(seul ? "affectation_confier" : "affectation_ajouter", { classe_id: classe?.classe.id ?? "", matiere_id: matiere, enseignant_id: choix[matiere] ?? "" }),
    onSuccess: rafraichir,
  });
  const retirer = useMutation({ mutationFn: (id: string) => appeler("affectation_retirer", { id }), onSuccess: rafraichir });
  const toutConfier = useMutation({
    mutationFn: () => appeler("maitre_affecter", { classe_id: classe?.classe.id ?? "", enseignant_id: maitre }),
    onSuccess: rafraichir,
  });
  if (classe === undefined) return <p className="text-sm text-muet">{t("enseignants.aucune_classe")}</p>;
  const programme = (pedagogie.data?.programme ?? []).filter((l) => l.niveau_id === classe.classe.niveau_id && l.serie_id === classe.classe.serie_id);
  const deLaClasse = (affectations.data ?? []).filter((a) => a.affectation.classe_id === classe.classe.id);
  return (
    <Section titre={t("enseignants.affectations")}>
      <div className="flex flex-wrap items-end gap-3">
        <Selection
          id="affectations-classe"
          libelle={t("enseignants.classe")}
          valeur={classe.classe.id}
          surChangement={setClasseChoisie}
          options={(classes.data ?? []).map((c) => ({ valeur: c.classe.id, libelle: `${c.classe.libelle} · ${c.niveau}` }))}
        />
        <Selection id="affectations-maitre" libelle={t("enseignants.maitre")} valeur={maitre} surChangement={setMaitre} options={options} />
        <Bouton variante="secondaire" onClick={() => toutConfier.mutate()} disabled={maitre === AUCUN || toutConfier.isPending}>{t("enseignants.toute_la_classe")}</Bouton>
      </div>
      <p className="text-sm text-muet">{t("enseignants.aide_affectations")}</p>
      {toutConfier.data === undefined ? null : <p role="status" className="text-sm text-vert">{t("enseignants.matieres_recues", { nombre: String(toutConfier.data) })}</p>}
      <table className="w-full text-left text-sm">
        <thead>
          <tr className="text-muet">
            <th className="px-2 py-1 font-normal">{t("programme.matiere")}</th>
            <th className="px-2 py-1 font-normal">{t("enseignants.enseigne_par")}</th>
            <th className="px-2 py-1 font-normal">{t("enseignants.changer")}</th>
          </tr>
        </thead>
        <tbody>
          {programme.map((l) => {
            const m = pedagogie.data?.matieres.find((x) => x.id === l.matiere_id);
            const deLaMatiere = deLaClasse.filter((a) => a.affectation.matiere_id === l.matiere_id);
            return (
              <tr key={l.id} className="border-b border-muet/30 align-top">
                <td className="px-2 py-1 text-encre">{m?.libelle}</td>
                <td className="px-2 py-1">
                  {deLaMatiere.length === 0 ? <span className="text-ocre">{t("enseignants.personne")}</span> : null}
                  <ul className="flex flex-col gap-1">
                    {deLaMatiere.map((a) => (
                      <li key={a.affectation.id} className="flex items-center gap-2 text-encre">
                        <span>{a.enseignant}</span>
                        <button type="button" className="text-xs text-primaire underline" onClick={() => retirer.mutate(a.affectation.id)}>
                          {t("commun.retirer")}
                        </button>
                      </li>
                    ))}
                  </ul>
                </td>
                <td className="px-2 py-1">
                  <div className="flex flex-wrap items-center gap-2">
                    <select
                      id={`affectation-${m?.code ?? l.matiere_id}`}
                      aria-label={`${t("enseignants.changer")} ${m?.libelle ?? ""}`}
                      className="rounded border border-muet bg-surface px-2 py-1 text-primaire"
                      value={choix[l.matiere_id] ?? AUCUN}
                      onChange={(e) => setChoix({ ...choix, [l.matiere_id]: e.target.value })}
                    >
                      {options.map((o) => (
                        <option key={o.valeur} value={o.valeur}>{o.libelle}</option>
                      ))}
                    </select>
                    <Bouton variante="secondaire" onClick={() => affecter.mutate({ matiere: l.matiere_id, seul: true })} disabled={(choix[l.matiere_id] ?? AUCUN) === AUCUN}>{t("enseignants.confier")}</Bouton>
                    <Bouton variante="secondaire" onClick={() => affecter.mutate({ matiere: l.matiere_id, seul: false })} disabled={(choix[l.matiere_id] ?? AUCUN) === AUCUN}>{t("enseignants.partager")}</Bouton>
                  </div>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
      <MessageErreur erreur={affecter.error ?? retirer.error ?? toutConfier.error} />
    </Section>
  );
}
