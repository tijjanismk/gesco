import { useQuery, useQueryClient } from "@tanstack/react-query";
import { type KeyboardEvent, useEffect, useRef, useState } from "react";

import { appeler, ErreurApi } from "../../api";
import { Selection } from "../../composants/Champ";
import { MessageEnregistre, MessageErreur, Section } from "../../composants/Messages";
import { heure, t } from "../../i18n";
import type { GrilleObservations, NiveauAcquisition, Observation } from "../../types/gesco";
import { caseVoisine, cleCase, initiales, NIVEAUX, niveauDeTouche, PICTOGRAMMES } from "./grilleObservations";

const CHAMP =
  "rounded border border-muet bg-surface px-2 py-1.5 text-primaire focus:outline-2 focus:outline-primaire disabled:bg-fond disabled:text-encre disabled:opacity-70";

const SANS_NIVEAU = "";

/** Refus d'une case ou d'une colonne écrite par un autre poste depuis la lecture. */
type Conflit = { inscription: string | null; competence: string; par: string | null; le: number };

/**
 * Observations d'une classe du jardin (SPEC_PHASE_11_B §4.2) : une ligne par enfant, une colonne
 * par compétence du domaine choisi. Les cases vides sont vides, sans compteur ni alerte (RB-2).
 */
export function Observations({ classeId, periodeId }: { classeId: string; periodeId: string }) {
  const grille = useQuery({
    queryKey: ["observations", classeId, periodeId],
    queryFn: () => appeler("observations_grille", { classe_id: classeId, periode_id: periodeId }),
  });
  if (grille.data === undefined) return <MessageErreur erreur={grille.error} />;
  return <Grille key={`${classeId}:${periodeId}`} g={grille.data} classeId={classeId} periodeId={periodeId} />;
}

function Grille({ g, classeId, periodeId }: { g: GrilleObservations; classeId: string; periodeId: string }) {
  const client = useQueryClient();
  const [domaineChoisi, setDomaineChoisi] = useState<string | null>(null);
  const domaine = g.domaines.find((d) => d.id === domaineChoisi) ?? g.domaines[0];
  const competences = g.competences.filter((c) => c.domaine_id === domaine?.id);
  // Cases écrites par ce poste depuis la lecture, confirmées par le serveur : l'état pour
  // l'affichage, la référence pour les saisies en vol, qui doivent renvoyer l'instant le plus
  // récent même si l'écran n'a pas encore été redessiné.
  const [ecrites, setEcrites] = useState<Record<string, Observation>>({});
  const confirmees = useRef<Record<string, Observation>>({});
  const grilleLue = useRef(g);
  useEffect(() => {
    grilleLue.current = g;
  }, [g]);
  const confirmer = (maj: (x: Record<string, Observation>) => Record<string, Observation>) => {
    confirmees.current = maj(confirmees.current);
    setEcrites(confirmees.current);
  };
  // Niveau tapé, affiché aussitôt, le temps que le serveur réponde.
  const [tapees, setTapees] = useState<Record<string, NiveauAcquisition | null>>({});
  const enCours = useRef<Record<string, Promise<void>>>({});
  const [erreur, setErreur] = useState<unknown>(null);
  const [conflit, setConflit] = useState<Conflit | null>(null);
  const [enfantChoisi, setEnfantChoisi] = useState<string | null>(null);
  const cases = useRef<Record<string, HTMLButtonElement | null>>({});
  const lettres = initiales(g.echelle);
  const libelleDe = (n: NiveauAcquisition) => g.echelle[NIVEAUX.indexOf(n)] ?? n;
  const lue = (source: GrilleObservations, deCePoste: Record<string, Observation>, inscription: string, competence: string): Observation | null =>
    deCePoste[cleCase(inscription, competence)] ?? source.observations.find((o) => o.inscription_id === inscription && o.competence_id === competence) ?? null;
  const enregistree = (inscription: string, competence: string) => lue(g, ecrites, inscription, competence);
  const instantLu = (inscription: string, competence: string) => lue(grilleLue.current, confirmees.current, inscription, competence)?.modifiee_le ?? 0;
  const niveauAffiche = (inscription: string, competence: string): NiveauAcquisition | null => {
    const cle = cleCase(inscription, competence);
    return cle in tapees ? (tapees[cle] ?? null) : (enregistree(inscription, competence)?.niveau ?? null);
  };
  const sans = <T,>(x: Record<string, T>, garder: (cle: string) => boolean) => Object.fromEntries(Object.entries(x).filter(([k]) => garder(k)));
  const oublier = (cle: string) => {
    setTapees((x) => sans(x, (k) => k !== cle));
    confirmer((x) => sans(x, (k) => k !== cle));
  };
  const relire = () => void client.invalidateQueries({ queryKey: ["observations", classeId, periodeId] });

  const envoyer = async (inscription: string, competence: string, niveau: NiveauAcquisition | null) => {
    const cle = cleCase(inscription, competence);
    try {
      const o = await appeler("observation_saisir", {
        classe_id: classeId,
        periode_id: periodeId,
        competence_id: competence,
        inscription_id: inscription,
        niveau,
        lue_le: instantLu(inscription, competence),
      });
      confirmer((x) => ({ ...x, [cle]: o }));
      setErreur(null);
    } catch (e) {
      oublier(cle);
      if (e instanceof ErreurApi && e.modifiee !== null) {
        setConflit({ inscription, competence, par: e.modifiee.par, le: e.modifiee.le });
        relire();
      } else {
        setErreur(e);
      }
    }
  };
  // Une case à la fois : la seconde saisie d'une même case attend la réponse de la première.
  const poser = (inscription: string, competence: string, niveau: NiveauAcquisition | null) => {
    const cle = cleCase(inscription, competence);
    setTapees((x) => ({ ...x, [cle]: niveau }));
    setConflit(null);
    const precedente = enCours.current[cle] ?? Promise.resolve();
    const suivante = precedente.then(() => envoyer(inscription, competence, niveau));
    enCours.current[cle] = suivante;
  };
  const colonne = async (competence: string, niveau: NiveauAcquisition) => {
    await Promise.all(Object.values(enCours.current));
    const lues = grilleLue.current.enfants.map((e) => instantLu(e.inscription_id, competence));
    const deLaColonne = (k: string) => k.endsWith(`:${competence}`);
    try {
      const apres = await appeler("observations_colonne", {
        classe_id: classeId,
        periode_id: periodeId,
        competence_id: competence,
        niveau,
        lue_le: Math.max(0, ...lues),
      });
      confirmer((x) => ({ ...x, ...Object.fromEntries(apres.map((o) => [cleCase(o.inscription_id, o.competence_id), o])) }));
      setTapees((x) => sans(x, (k) => !deLaColonne(k)));
      setErreur(null);
      setConflit(null);
    } catch (e) {
      if (e instanceof ErreurApi && e.modifiee !== null) {
        setConflit({ inscription: null, competence, par: e.modifiee.par, le: e.modifiee.le });
        confirmer((x) => sans(x, (k) => !deLaColonne(k)));
        relire();
      } else {
        setErreur(e);
      }
    }
  };
  const aller = (ligne: number, col: number) => {
    const enfant = g.enfants[ligne];
    const competence = competences[col];
    if (enfant !== undefined && competence !== undefined) cases.current[cleCase(enfant.inscription_id, competence.id)]?.focus();
  };
  const surTouche = (ligne: number, col: number) => (e: KeyboardEvent<HTMLButtonElement>) => {
    const enfant = g.enfants[ligne];
    const competence = competences[col];
    if (enfant === undefined || competence === undefined) return;
    const niveau = g.modifiable ? niveauDeTouche(e.key, lettres) : null;
    if (niveau !== null) {
      e.preventDefault();
      poser(enfant.inscription_id, competence.id, niveau);
      const v = caseVoisine(ligne, col, g.enfants.length, competences.length, "suivante");
      if (v !== null) aller(...v);
      return;
    }
    if (g.modifiable && (e.key === "Delete" || e.key === "Backspace")) {
      e.preventDefault();
      poser(enfant.inscription_id, competence.id, null);
      return;
    }
    if (e.key === "Enter" || e.key === "ArrowDown" || e.key === "ArrowUp" || e.key === "ArrowLeft" || e.key === "ArrowRight") {
      e.preventDefault();
      const v = caseVoisine(ligne, col, g.enfants.length, competences.length, e.key === "Enter" ? "suivante" : e.key);
      if (v !== null) aller(...v);
    }
  };

  const enfant = g.enfants.find((e) => e.inscription_id === enfantChoisi) ?? g.enfants[0];
  const touches =
    lettres.length === 3 ? t("observations.touches", { a: lettres[0] ?? "", e: lettres[1] ?? "", n: lettres[2] ?? "" }) : t("observations.touches_chiffres");
  const texteConflit = (() => {
    if (conflit === null) return null;
    const competence = g.competences.find((c) => c.id === conflit.competence)?.libelle ?? "";
    const personne = conflit.par ?? t("fiche.quelqu_un");
    if (conflit.inscription === null) return t("observations.colonne_modifiee", { personne, heure: heure(conflit.le), competence });
    const eleve = g.enfants.find((e) => e.inscription_id === conflit.inscription)?.eleve ?? "";
    const niveau = enregistree(conflit.inscription, conflit.competence)?.niveau ?? null;
    return t("observations.modifiee", {
      personne,
      heure: heure(conflit.le),
      eleve,
      competence,
      valeur: niveau === null ? t("observations.vide") : libelleDe(niveau),
    });
  })();

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-end gap-3">
        <Selection
          id="observations-domaine"
          libelle={t("observations.domaine")}
          valeur={domaine?.id ?? ""}
          surChangement={setDomaineChoisi}
          options={g.domaines.map((d) => ({ valeur: d.id, libelle: d.libelle }))}
        />
      </div>
      {g.modifiable ? <p className="text-sm text-muet">{t("observations.aide", { touches })}</p> : <p className="text-sm text-ocre">{t("observations.verrouillee")}</p>}
      {domaine === undefined ? (
        <p className="text-sm text-muet">{t("observations.aucune_competence")}</p>
      ) : g.enfants.length === 0 ? (
        <p className="text-sm text-muet">{t("observations.aucun_enfant")}</p>
      ) : (
        <div className="overflow-x-auto">
          <table className="text-left text-sm">
            <thead>
              <tr className="align-bottom text-muet">
                <th className="px-2 py-1 font-normal">{t("notes.numero")}</th>
                <th className="px-2 py-1 font-normal">{t("observations.enfant")}</th>
                {competences.map((c) => (
                  <th key={c.id} className="w-40 px-2 py-1 font-normal">
                    <span className="block text-encre">{c.libelle}</span>
                    {g.modifiable ? (
                      <select
                        id={`colonne-${c.id}`}
                        aria-label={t("observations.toute_la_colonne_de", { competence: c.libelle })}
                        className="mt-1 w-full rounded border border-muet bg-surface px-1 py-0.5 text-xs text-primaire"
                        value={SANS_NIVEAU}
                        onChange={(e) => {
                          const n = NIVEAUX.find((x) => x === e.target.value);
                          if (n !== undefined) void colonne(c.id, n);
                        }}
                      >
                        <option value={SANS_NIVEAU}>{t("observations.toute_la_colonne")}</option>
                        {NIVEAUX.map((n) => (
                          <option key={n} value={n}>{`${PICTOGRAMMES[n]} ${libelleDe(n)}`}</option>
                        ))}
                      </select>
                    ) : null}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {g.enfants.map((e, ligne) => (
                <tr key={e.inscription_id} className={`border-b border-muet/30 ${e.inscription_id === enfant?.inscription_id ? "bg-fond" : ""}`}>
                  <td className="px-2 py-1 text-muet">{ligne + 1}</td>
                  <td className="px-2 py-1 text-encre">
                    <span className="flex items-center gap-2">
                      {e.photo === null ? null : <img src={e.photo} alt="" className="h-8 w-8 rounded object-cover" />}
                      {e.eleve}
                    </span>
                  </td>
                  {competences.map((c, col) => {
                    const n = niveauAffiche(e.inscription_id, c.id);
                    return (
                      <td key={c.id} className="px-1 py-0.5">
                        <button
                          type="button"
                          id={`obs-${ligne + 1}-${col + 1}`}
                          ref={(el) => {
                            cases.current[cleCase(e.inscription_id, c.id)] = el;
                          }}
                          aria-label={t("observations.case", { competence: c.libelle, eleve: e.eleve })}
                          className="h-8 w-full rounded border border-muet/40 bg-surface px-2 text-left text-primaire focus:outline-2 focus:outline-primaire"
                          onKeyDown={surTouche(ligne, col)}
                          onFocus={() => setEnfantChoisi(e.inscription_id)}
                        >
                          {n === null ? null : (
                            <>
                              <span aria-hidden="true" className="mr-1 text-encre">{PICTOGRAMMES[n]}</span>
                              {libelleDe(n)}
                            </>
                          )}
                        </button>
                      </td>
                    );
                  })}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      {texteConflit === null ? null : (
        <p role="alert" className="rounded border border-rouge/40 p-3 text-sm text-rouge">{texteConflit}</p>
      )}
      <MessageErreur erreur={erreur} />
      {enfant === undefined ? null : (
        <Section titre={t("observations.pour", { eleve: enfant.eleve })}>
          {g.commentaire_domaine && domaine !== undefined ? (
            <Appreciation
              key={`${enfant.inscription_id}:${domaine.id}`}
              id="observation-commentaire"
              libelle={t("observations.commentaire_domaine", { domaine: domaine.libelle })}
              g={g}
              classeId={classeId}
              periodeId={periodeId}
              inscription={enfant.inscription_id}
              domaine={domaine.id}
            />
          ) : null}
          <Appreciation
            key={`${enfant.inscription_id}:generale`}
            id="observation-appreciation"
            libelle={t("observations.appreciation_generale")}
            g={g}
            classeId={classeId}
            periodeId={periodeId}
            inscription={enfant.inscription_id}
            domaine={null}
          />
        </Section>
      )}
    </div>
  );
}

/** Appréciation générale ou commentaire d'un domaine, enregistré à la sortie du champ. */
function Appreciation({
  id,
  libelle,
  g,
  classeId,
  periodeId,
  inscription,
  domaine,
}: {
  id: string;
  libelle: string;
  g: GrilleObservations;
  classeId: string;
  periodeId: string;
  inscription: string;
  domaine: string | null;
}) {
  const client = useQueryClient();
  const lu = g.appreciations.find((a) => a.inscription_id === inscription && a.domaine_id === domaine)?.texte ?? "";
  const [texte, setTexte] = useState(lu);
  const [enregistre, setEnregistre] = useState(lu);
  const [instant, setInstant] = useState<number | null>(null);
  const [erreur, setErreur] = useState<unknown>(null);
  const enregistrer = async () => {
    if (texte.trim() === enregistre.trim()) return;
    try {
      const ecrit = await appeler("observation_apprecier", { classe_id: classeId, periode_id: periodeId, inscription_id: inscription, domaine_id: domaine, texte });
      setEnregistre(ecrit);
      setInstant(Date.now());
      setErreur(null);
      void client.invalidateQueries({ queryKey: ["observations", classeId, periodeId] });
    } catch (e) {
      setErreur(e);
    }
  };
  return (
    <div className="flex flex-col gap-1">
      <label htmlFor={id} className="flex flex-col gap-1 text-sm text-encre">
        <span>{libelle}</span>
        <textarea
          id={id}
          rows={2}
          maxLength={300}
          className={CHAMP}
          value={texte}
          disabled={!g.modifiable}
          onChange={(e) => setTexte(e.target.value)}
          onBlur={() => void enregistrer()}
        />
      </label>
      <MessageEnregistre instant={instant} />
      <MessageErreur erreur={erreur} />
    </div>
  );
}
