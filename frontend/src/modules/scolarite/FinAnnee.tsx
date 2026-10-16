import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { commandeAutorisee } from "../../app/navigation";
import { Bouton } from "../../composants/Bouton";
import { Champ, Formulaire, Selection } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { Tableau } from "../../composants/Tableau";
import { type Cle, t } from "../../i18n";
import type { AnneeSuivante, BilanReinscriptions, Cycle, DecisionFinAnnee, DecisionsClasse, FinAnnee as Fin, PassageCycle } from "../../types/gesco";
import { lirePoints, moyenne, points } from "../pedagogie/saisies";

const DECISIONS: Record<DecisionFinAnnee, Cle> = {
  non_statue: "fin_annee.decision.non_statue",
  admis: "fin_annee.decision.admis",
  redouble: "fin_annee.decision.redouble",
  exclu: "fin_annee.decision.exclu",
  rattrapage: "fin_annee.decision.rattrapage",
};

const CYCLES: Record<Cycle, Cle> = {
  jardin: "niveaux.cycle.jardin",
  premier_cycle: "niveaux.cycle.premier_cycle",
  second_cycle: "niveaux.cycle.second_cycle",
  lycee: "niveaux.cycle.lycee",
  technique: "niveaux.cycle.technique",
};

function useRafraichir() {
  const client = useQueryClient();
  return () => {
    for (const cle of ["fin-annee", "decisions", "annees", "session", "avertissements", "inscriptions"]) void client.invalidateQueries({ queryKey: [cle] });
  };
}

/** Fin d'année (SPEC_PHASE_10 §3 et §4) : la moyenne de passage reprise du CAP ou de l'académie,
 *  la décision de chaque élève proposée puis validée par la direction, la clôture de l'année. */
export function FinAnnee() {
  const session = useQuery({ queryKey: ["session"], queryFn: () => appeler("session_courante", null) });
  const fin = useQuery({ queryKey: ["fin-annee"], queryFn: () => appeler("fin_annee_lire", null) });
  const [choisie, setChoisie] = useState<string | null>(null);
  const permissions = session.data?.permissions ?? [];
  const donnees = fin.data;
  const cloturee = donnees?.annee.statut === "cloturee";
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">
        {donnees === undefined ? t("fin_annee.titre") : t("fin_annee.titre_annee", { annee: donnees.annee.libelle })}
      </h1>
      <p className="text-sm text-muet">{t("fin_annee.explication")}</p>
      <MessageErreur erreur={fin.error} />
      {donnees === undefined ? null : (
        <>
          {cloturee ? <p className="text-sm font-medium text-ocre">{t("fin_annee.cloturee")}</p> : null}
          {donnees.passages.length === 0 || cloturee ? null : <Passages passages={donnees.passages} />}
          <div className="flex flex-wrap items-start gap-4">
            <div className="min-w-0 flex-1">
              <Tableau
                nomExport="fin-annee"
                titre={t("fin_annee.classes")}
                lignes={donnees.classes}
                cle={(c) => c.classe_id}
                surSelection={(c) => setChoisie(c.classe_id)}
                colonnes={[
                  { titre: t("fin_annee.classe"), valeur: (c) => c.classe },
                  { titre: t("fin_annee.eleves"), valeur: (c) => String(c.eleves) },
                  { titre: t("fin_annee.admis"), valeur: (c) => String(c.admis) },
                  { titre: t("fin_annee.redoublent"), valeur: (c) => String(c.redoublent) },
                  { titre: t("fin_annee.exclus"), valeur: (c) => String(c.exclus) },
                  { titre: t("fin_annee.en_attente"), valeur: (c) => (c.en_attente === 0 ? "" : String(c.en_attente)) },
                ]}
              />
            </div>
          </div>
          {choisie === null ? null : <Decisions key={choisie} classeId={choisie} />}
          {cloturee ? (
            commandeAutorisee("annee_rouvrir", permissions) ? <Reouverture fin={donnees} /> : null
          ) : (
            <Cloture fin={donnees} />
          )}
          {donnees.suivante === null ? (
            <p className="text-sm text-muet">{t("fin_annee.sans_suivante")}</p>
          ) : (
            <Suivante suivante={donnees.suivante} />
          )}
        </>
      )}
    </div>
  );
}

function Passages({ passages }: { passages: readonly PassageCycle[] }) {
  return (
    <Section titre={t("fin_annee.passage")}>
      <p className="text-sm text-muet">{t("fin_annee.passage_aide")}</p>
      <div className="flex flex-wrap gap-6">
        {passages.map((p) => (
          <Passage key={p.cycle} passage={p} />
        ))}
      </div>
    </Section>
  );
}

function Passage({ passage }: { passage: PassageCycle }) {
  const rafraichir = useRafraichir();
  const [texte, setTexte] = useState(points(passage.passage));
  const lu = lirePoints(texte);
  const regler = useMutation({
    mutationFn: (valeur: number) => appeler("passage_regler", { cycle: passage.cycle, passage: valeur }),
    onSuccess: rafraichir,
  });
  const id = `passage-${passage.cycle}`;
  return (
    <Formulaire surEnvoi={() => (lu === null ? undefined : regler.mutate(lu))}>
      <div className="flex items-end gap-2">
        <Champ
          id={id}
          libelle={t("fin_annee.passage_cycle", { cycle: t(CYCLES[passage.cycle]), bareme: points(passage.bareme) })}
          value={texte}
          onChange={(e) => setTexte(e.target.value)}
          autoComplete="off"
        />
        <Bouton type="submit" variante="secondaire" disabled={lu === null || lu === passage.passage || regler.isPending}>
          {t("commun.enregistrer")}
        </Bouton>
      </div>
      <MessageErreur erreur={regler.error} />
    </Formulaire>
  );
}

function Decisions({ classeId }: { classeId: string }) {
  const rafraichir = useRafraichir();
  const client = useQueryClient();
  const decisions = useQuery({
    queryKey: ["decisions", classeId],
    queryFn: () => appeler("decisions_classe_lire", { classe_id: classeId }),
  });
  const apres = (d: DecisionsClasse) => {
    client.setQueryData(["decisions", classeId], d);
    rafraichir();
  };
  const poser = useMutation({
    mutationFn: (demande: { inscription_id: string; decision: DecisionFinAnnee }) => appeler("decision_poser", demande),
    onSuccess: apres,
  });
  const valider = useMutation({
    mutationFn: () => appeler("decisions_valider_classe", { classe_id: classeId }),
    onSuccess: apres,
  });
  const d = decisions.data;
  const texteMoyenne = (m: number | null) => (m === null || d?.bareme == null ? "" : `${moyenne(m)} / ${points(d.bareme)}`);
  const a_valider = (d?.lignes ?? []).filter((l) => l.decision === "non_statue" && l.proposee !== "non_statue").length;
  return (
    <Section titre={d === undefined ? t("fin_annee.decisions") : t("fin_annee.decisions_classe", { classe: d.classe })}>
      <MessageErreur erreur={decisions.error ?? poser.error ?? valider.error} />
      {d === undefined ? null : (
        <>
          <p className="text-sm text-muet">
            {d.passage === null
              ? t("fin_annee.jardin_aide")
              : t("fin_annee.proposition_aide", { passage: moyenne(d.passage), bareme: points(d.bareme ?? 0) })}
          </p>
          {d.figee ? null : (
            <div>
              <Bouton onClick={() => valider.mutate()} disabled={a_valider === 0 || valider.isPending}>
                {t("fin_annee.valider_propositions", { nombre: a_valider })}
              </Bouton>
            </div>
          )}
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-muet text-left text-muet">
                <th className="py-1 pr-3 font-normal">{t("fin_annee.eleve")}</th>
                <th className="py-1 pr-3 font-normal">{t("fin_annee.moyenne_annuelle")}</th>
                <th className="py-1 pr-3 font-normal">{t("fin_annee.proposee")}</th>
                <th className="py-1 pr-3 font-normal">{t("fin_annee.decision")}</th>
              </tr>
            </thead>
            <tbody>
              {d.lignes.map((l) => (
                <tr key={l.inscription_id} className="border-b border-fond">
                  <td className="py-1 pr-3 text-encre">
                    {l.eleve} <span className="text-muet">{l.matricule}</span>
                  </td>
                  <td className="py-1 pr-3 text-encre">{texteMoyenne(l.moyenne_annuelle)}</td>
                  <td className="py-1 pr-3 text-muet">{l.proposee === "non_statue" ? t("fin_annee.sans_proposition") : t(DECISIONS[l.proposee])}</td>
                  <td className="py-1 pr-3">
                    {d.figee ? (
                      <span className="text-encre">{t(DECISIONS[l.decision])}</span>
                    ) : (
                      <Selection
                        id={`decision-${l.matricule}`}
                        libelle=""
                        valeur={l.decision}
                        surChangement={(v) => poser.mutate({ inscription_id: l.inscription_id, decision: v as DecisionFinAnnee })}
                        options={(Object.keys(DECISIONS) as DecisionFinAnnee[]).map((v) => ({ valeur: v, libelle: t(DECISIONS[v]) }))}
                      />
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </>
      )}
    </Section>
  );
}

function Cloture({ fin }: { fin: Fin }) {
  const rafraichir = useRafraichir();
  const [confirmation, setConfirmation] = useState(false);
  const cloturer = useMutation({
    mutationFn: () => appeler("annee_cloturer", { id: fin.annee.id }),
    onSuccess: () => {
      setConfirmation(false);
      rafraichir();
    },
  });
  return (
    <Section titre={t("fin_annee.cloture")}>
      <p className="text-sm text-muet">{t("fin_annee.cloture_aide")}</p>
      {fin.en_attente > 0 ? (
        <p className="text-sm text-ocre">{t("fin_annee.encore_en_attente", { nombre: fin.en_attente })}</p>
      ) : null}
      <div className="flex flex-wrap gap-2">
        {confirmation ? (
          <>
            <Bouton variante="danger" onClick={() => cloturer.mutate()} disabled={cloturer.isPending}>
              {t("fin_annee.confirmer_cloture", { annee: fin.annee.libelle })}
            </Bouton>
            <Bouton variante="secondaire" onClick={() => setConfirmation(false)}>
              {t("commun.annuler")}
            </Bouton>
          </>
        ) : (
          <Bouton onClick={() => setConfirmation(true)} disabled={fin.en_attente > 0}>
            {t("fin_annee.cloturer", { annee: fin.annee.libelle })}
          </Bouton>
        )}
      </div>
      <MessageErreur erreur={cloturer.error} />
    </Section>
  );
}

function Reouverture({ fin }: { fin: Fin }) {
  const rafraichir = useRafraichir();
  const [motif, setMotif] = useState("");
  const rouvrir = useMutation({
    mutationFn: () => appeler("annee_rouvrir", { annee_id: fin.annee.id, motif }),
    onSuccess: () => {
      setMotif("");
      rafraichir();
    },
  });
  return (
    <Section titre={t("fin_annee.reouverture")}>
      <p className="text-sm text-muet">{t("fin_annee.reouverture_aide")}</p>
      <Formulaire surEnvoi={() => rouvrir.mutate()}>
        <Champ id="reouverture-motif" libelle={t("fin_annee.motif")} value={motif} onChange={(e) => setMotif(e.target.value)} autoComplete="off" />
        <div>
          <Bouton type="submit" variante="secondaire" disabled={motif.trim() === "" || rouvrir.isPending}>
            {t("fin_annee.rouvrir", { annee: fin.annee.libelle })}
          </Bouton>
        </div>
      </Formulaire>
      <MessageErreur erreur={rouvrir.error} />
    </Section>
  );
}

function Suivante({ suivante }: { suivante: AnneeSuivante }) {
  const rafraichir = useRafraichir();
  const [bilan, setBilan] = useState<BilanReinscriptions | null>(null);
  const reprendre = useMutation({
    mutationFn: () => appeler("classes_reprendre", { id: suivante.annee.id }),
    onSuccess: rafraichir,
  });
  const preparer = useMutation({
    mutationFn: () => appeler("reinscriptions_preparer", { id: suivante.annee.id }),
    onSuccess: (b) => {
      setBilan(b);
      rafraichir();
    },
  });
  return (
    <Section titre={t("fin_annee.suivante", { annee: suivante.annee.libelle })}>
      <p className="text-sm text-muet">{t("fin_annee.suivante_aide")}</p>
      <p className="text-sm text-encre">
        {t("fin_annee.suivante_etat", { classes: suivante.classes, a_valider: suivante.a_valider, validees: suivante.validees })}
      </p>
      <div className="flex flex-wrap gap-2">
        <Bouton variante="secondaire" onClick={() => reprendre.mutate()} disabled={reprendre.isPending}>
          {t("fin_annee.reprendre_classes")}
        </Bouton>
        <Bouton onClick={() => preparer.mutate()} disabled={suivante.classes === 0 || preparer.isPending}>
          {t("fin_annee.preparer_reinscriptions")}
        </Bouton>
      </div>
      <MessageErreur erreur={reprendre.error ?? preparer.error} />
      {bilan === null ? null : (
        <div role="status" className="flex flex-col gap-1 text-sm text-encre">
          <p>{t("fin_annee.bilan_preparees", { nombre: bilan.preparees })}</p>
          {bilan.deja_inscrits > 0 ? <p>{t("fin_annee.bilan_deja", { nombre: bilan.deja_inscrits })}</p> : null}
          {bilan.sortants > 0 ? <p>{t("fin_annee.bilan_sortants", { nombre: bilan.sortants })}</p> : null}
          {bilan.sans_classe.length > 0 ? <p className="text-ocre">{t("fin_annee.bilan_sans_classe", { niveaux: bilan.sans_classe.join(", ") })}</p> : null}
        </div>
      )}
    </Section>
  );
}
