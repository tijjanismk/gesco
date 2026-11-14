import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { commandeAutorisee } from "../../app/navigation";
import { Bouton } from "../../composants/Bouton";
import { Selection } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { t } from "../../i18n";
import type {
  ComplementEleve,
  ComplementMembre,
  ComplementsEleves,
  DegreDifficulte,
  DiplomeEleve,
  Fonctionnement,
  PedagogieClasse,
  Specialite,
  StatutRapport,
  TypeDifficulte,
} from "../../types/gesco";

const FONCTIONNEMENTS: readonly Fonctionnement[] = [
  "simple",
  "double_division",
  "double_vacation",
];
const PEDAGOGIES: readonly PedagogieClasse[] = [
  "classique",
  "pedagogie_convergente",
  "curriculum",
];
const STATUTS: readonly StatutRapport[] = [
  "fonctionnaire_etat",
  "fonctionnaire_collectivites",
  "contractuel_etat",
  "contractuel_collectivites",
  "contractuel_ecole",
  "vacataire",
  "eleve_maitre",
  "autre",
];
const SPECIALITES: readonly Specialite[] = [
  "lhg",
  "mpc",
  "snpc",
  "ldm",
  "eps",
  "autre",
];
const DIPLOMES: readonly DiplomeEleve[] = [
  "doctorat",
  "master",
  "dea",
  "ensup",
  "ingenieur",
  "maitrise",
  "licence",
  "deug_bts",
  "bt",
  "baccalaureat",
  "cap",
  "ifm",
  "autre",
];
const TYPES: readonly TypeDifficulte[] = [
  "vue",
  "ouie",
  "motricite_globale",
  "motricite_fine",
  "intellect",
  "communication",
  "comportement",
  "multiple",
  "autre",
];
const DEGRES: readonly DegreDifficulte[] = [
  "quelques",
  "beaucoup",
  "impossible",
];

const SAISIE =
  "rounded border border-muet bg-surface px-2 py-1 text-sm text-primaire";

/** Onglet « Pour les rapports » : ce que les formulaires demandent et que les fiches ne disaient
 *  pas, saisi en grille ; chaque case est facultative, une valeur proposée se montre à côté sans
 *  être écrite d'elle-même. */
export function PourLesRapports({ anneeId }: { anneeId: string | null }) {
  const session = useQuery({
    queryKey: ["session"],
    queryFn: () => appeler("session_courante", null),
  });
  const droits = session.data?.permissions ?? [];
  return (
    <div className="flex flex-col gap-4">
      {commandeAutorisee("complements_eleves_lire", droits) ? (
        <GrilleEleves
          anneeId={anneeId}
          peutEleves={commandeAutorisee("complement_eleve_regler", droits)}
          peutClasse={commandeAutorisee("classe_rapports_regler", droits)}
          peutDifficultes={commandeAutorisee("difficulte_eleve_regler", droits)}
        />
      ) : null}
      {commandeAutorisee("complements_personnel_lire", droits) ? (
        <GrillePersonnel
          peutModifier={commandeAutorisee("complement_membre_regler", droits)}
        />
      ) : null}
    </div>
  );
}

function GrilleEleves({
  anneeId,
  peutEleves,
  peutClasse,
  peutDifficultes,
}: {
  anneeId: string | null;
  peutEleves: boolean;
  peutClasse: boolean;
  peutDifficultes: boolean;
}) {
  const [classeId, setClasseId] = useState<string | null>(null);
  const [ouvert, setOuvert] = useState<ComplementEleve | null>(null);
  const client = useQueryClient();
  const cle = ["complements-eleves", anneeId, classeId];
  const donnees = useQuery({
    queryKey: cle,
    queryFn: () =>
      appeler("complements_eleves_lire", {
        annee_id: anneeId,
        classe_id: classeId,
      }),
  });
  const garder = (d: ComplementsEleves) => client.setQueryData(cle, d);
  const classe = useMutation({
    mutationFn: (v: {
      fonctionnement: Fonctionnement | null;
      pedagogie: PedagogieClasse | null;
    }) =>
      appeler("classe_rapports_regler", {
        classe_id: donnees.data?.classe_id ?? "",
        ...v,
      }),
    onSuccess: garder,
  });
  const eleve = useMutation({
    mutationFn: (v: {
      inscription_id: string;
      annee_recrutement: number | null;
      prise_en_charge_etat: boolean;
    }) => appeler("complement_eleve_regler", v),
    onSuccess: garder,
  });
  const d = donnees.data;
  if (d === undefined) return <MessageErreur erreur={donnees.error} />;
  const choisie = d.classes.find((c) => c.id === d.classe_id);
  return (
    <Section titre={t("complements.eleves")}>
      {choisie === undefined ? (
        <p className="text-sm text-muet">{t("complements.aucune_classe")}</p>
      ) : (
        <>
          <div className="flex flex-wrap items-end gap-3">
            <Selection
              id="complements-classe"
              libelle={t("complements.classe")}
              valeur={choisie.id}
              surChangement={setClasseId}
              options={d.classes.map((c) => ({
                valeur: c.id,
                libelle: c.libelle,
              }))}
            />
            {peutClasse ? (
              <>
                <Selection
                  id="complements-fonctionnement"
                  libelle={t("complements.fonctionnement")}
                  valeur={choisie.fonctionnement ?? ""}
                  surChangement={(v) =>
                    classe.mutate({
                      fonctionnement: v === "" ? null : (v as Fonctionnement),
                      pedagogie: choisie.pedagogie,
                    })
                  }
                  options={[
                    {
                      valeur: "",
                      libelle: t("complements.par_defaut", {
                        valeur: t(
                          `complements.fonctionnement.${d.fonctionnement_defaut}`,
                        ),
                      }),
                    },
                    ...FONCTIONNEMENTS.map((f) => ({
                      valeur: f,
                      libelle: t(`complements.fonctionnement.${f}`),
                    })),
                  ]}
                />
                <Selection
                  id="complements-pedagogie"
                  libelle={t("complements.pedagogie")}
                  valeur={choisie.pedagogie ?? ""}
                  surChangement={(v) =>
                    classe.mutate({
                      fonctionnement: choisie.fonctionnement,
                      pedagogie: v === "" ? null : (v as PedagogieClasse),
                    })
                  }
                  options={[
                    {
                      valeur: "",
                      libelle: t("complements.par_defaut", {
                        valeur: t(
                          `complements.pedagogie.${d.pedagogie_defaut}`,
                        ),
                      }),
                    },
                    ...PEDAGOGIES.map((p) => ({
                      valeur: p,
                      libelle: t(`complements.pedagogie.${p}`),
                    })),
                  ]}
                />
              </>
            ) : (
              <p className="text-sm text-encre">
                {t(
                  `complements.fonctionnement.${choisie.fonctionnement ?? d.fonctionnement_defaut}`,
                )}{" "}
                ·{" "}
                {t(
                  `complements.pedagogie.${choisie.pedagogie ?? d.pedagogie_defaut}`,
                )}
              </p>
            )}
          </div>
          <MessageErreur erreur={classe.error ?? eleve.error} />
          {d.eleves.length === 0 ? (
            <p className="text-sm text-muet">{t("commun.aucune_ligne")}</p>
          ) : (
            <div className="overflow-x-auto">
              <table className="w-full border-collapse text-sm">
                <thead>
                  <tr className="text-left text-muet">
                    <th className="border-b border-muet px-2 py-1 font-medium">
                      {t("complements.eleve")}
                    </th>
                    <th className="border-b border-muet px-2 py-1 font-medium">
                      {t("complements.annee_recrutement")}
                    </th>
                    {d.prise_en_charge_possible ? (
                      <th className="border-b border-muet px-2 py-1 font-medium">
                        {t("complements.prise_en_charge")}
                      </th>
                    ) : null}
                    {peutDifficultes ? (
                      <th className="border-b border-muet px-2 py-1 font-medium">
                        {t("complements.difficultes")}
                      </th>
                    ) : null}
                  </tr>
                </thead>
                <tbody>
                  {d.eleves.map((e) => (
                    <LigneEleve
                      key={e.inscription_id}
                      eleve={e}
                      possible={d.prise_en_charge_possible}
                      peutModifier={peutEleves}
                      peutDifficultes={peutDifficultes}
                      surRegler={(v) => eleve.mutate(v)}
                      surOuvrir={() => setOuvert(e)}
                    />
                  ))}
                </tbody>
              </table>
            </div>
          )}
          {ouvert === null ? null : (
            <Difficultes eleve={ouvert} surFermer={() => setOuvert(null)} />
          )}
        </>
      )}
    </Section>
  );
}

function LigneEleve({
  eleve,
  possible,
  peutModifier,
  peutDifficultes,
  surRegler,
  surOuvrir,
}: {
  eleve: ComplementEleve;
  possible: boolean;
  peutModifier: boolean;
  peutDifficultes: boolean;
  surRegler: (v: {
    inscription_id: string;
    annee_recrutement: number | null;
    prise_en_charge_etat: boolean;
  }) => void;
  surOuvrir: () => void;
}) {
  const [brouillon, setBrouillon] = useState<string | null>(null);
  const regler = (annee: number | null, prise: boolean) =>
    surRegler({
      inscription_id: eleve.inscription_id,
      annee_recrutement: annee,
      prise_en_charge_etat: prise,
    });
  const quitter = (texte: string) => {
    setBrouillon(null);
    const n = texte.trim() === "" ? null : Number.parseInt(texte, 10);
    if (n !== null && Number.isNaN(n)) return;
    if (n !== eleve.annee_recrutement) regler(n, eleve.prise_en_charge_etat);
  };
  return (
    <tr>
      <th
        scope="row"
        className="border-b border-muet/30 px-2 py-1 text-left font-normal text-encre"
      >
        {eleve.nom}
      </th>
      <td className="border-b border-muet/30 px-2 py-1">
        <div className="flex flex-wrap items-center gap-2">
          {peutModifier ? (
            <input
              aria-label={`${t("complements.annee_recrutement")} · ${eleve.nom}`}
              className={`${SAISIE} w-24`}
              inputMode="numeric"
              value={
                brouillon ??
                (eleve.annee_recrutement === null
                  ? ""
                  : String(eleve.annee_recrutement))
              }
              onChange={(e) => setBrouillon(e.target.value)}
              onBlur={(e) => quitter(e.target.value)}
            />
          ) : (
            <span className="text-primaire">
              {eleve.annee_recrutement ?? ""}
            </span>
          )}
          {eleve.annee_proposee === null ? null : (
            <span className="text-muet">
              {t("complements.proposee", {
                valeur: String(eleve.annee_proposee),
              })}{" "}
              {peutModifier ? (
                <button
                  type="button"
                  className="underline decoration-dotted underline-offset-2 hover:text-encre"
                  onClick={() =>
                    regler(eleve.annee_proposee, eleve.prise_en_charge_etat)
                  }
                >
                  {t("complements.retenir")}
                </button>
              ) : null}
            </span>
          )}
        </div>
      </td>
      {possible ? (
        <td className="border-b border-muet/30 px-2 py-1">
          <input
            type="checkbox"
            aria-label={`${t("complements.prise_en_charge")} · ${eleve.nom}`}
            checked={eleve.prise_en_charge_etat}
            disabled={!peutModifier}
            onChange={(e) => regler(eleve.annee_recrutement, e.target.checked)}
          />
        </td>
      ) : null}
      {peutDifficultes ? (
        <td className="border-b border-muet/30 px-2 py-1">
          <Bouton variante="secondaire" onClick={surOuvrir}>
            {t("complements.difficultes")}
          </Bouton>
        </td>
      ) : null}
    </tr>
  );
}

/** Difficultés d'un élève : un degré par type, vide pour ne rien noter ; confidentiel. */
function Difficultes({
  eleve,
  surFermer,
}: {
  eleve: ComplementEleve;
  surFermer: () => void;
}) {
  const client = useQueryClient();
  const cle = ["difficultes", eleve.eleve_id];
  const liste = useQuery({
    queryKey: cle,
    queryFn: () =>
      appeler("difficultes_eleve_lire", { eleve_id: eleve.eleve_id }),
  });
  const regler = useMutation({
    mutationFn: (v: {
      type_difficulte: TypeDifficulte;
      degre: DegreDifficulte | null;
    }) =>
      appeler("difficulte_eleve_regler", {
        eleve_id: eleve.eleve_id,
        note: "",
        ...v,
      }),
    onSuccess: (d) => client.setQueryData(cle, d),
  });
  return (
    <Section titre={t("complements.difficultes_de", { nom: eleve.nom })}>
      <p className="text-sm text-muet">
        {t("complements.difficultes_confidentiel")}
      </p>
      <div className="grid gap-2 sm:grid-cols-2">
        {TYPES.map((ty) => (
          <Selection
            key={ty}
            id={`difficulte-${ty}`}
            libelle={t(`complements.difficulte.${ty}`)}
            valeur={
              liste.data?.find((x) => x.type_difficulte === ty)?.degre ?? ""
            }
            surChangement={(v) =>
              regler.mutate({
                type_difficulte: ty,
                degre: v === "" ? null : (v as DegreDifficulte),
              })
            }
            options={[
              { valeur: "", libelle: "" },
              ...DEGRES.map((de) => ({
                valeur: de,
                libelle: t(`complements.degre.${de}`),
              })),
            ]}
          />
        ))}
      </div>
      <MessageErreur erreur={liste.error ?? regler.error} />
      <div>
        <Bouton variante="secondaire" onClick={surFermer}>
          {t("commun.fermer")}
        </Bouton>
      </div>
    </Section>
  );
}

function GrillePersonnel({ peutModifier }: { peutModifier: boolean }) {
  const client = useQueryClient();
  const liste = useQuery({
    queryKey: ["complements-personnel"],
    queryFn: () => appeler("complements_personnel_lire", null),
  });
  const regler = useMutation({
    mutationFn: (v: {
      membre_id: string;
      statut: StatutRapport | null;
      specialite: Specialite | null;
      diplome: DiplomeEleve | null;
    }) => appeler("complement_membre_regler", v),
    onSuccess: (d) => client.setQueryData(["complements-personnel"], d),
  });
  const membres = (liste.data ?? []).filter((m) => !m.parti);
  const choix = <T extends string>(
    valeurs: readonly T[],
    prefixe: "statut" | "specialite" | "diplome",
    vide: string,
  ) => [
    { valeur: "", libelle: vide },
    ...valeurs.map((v) => ({
      valeur: v,
      libelle: t(`complements.${prefixe}.${v}` as never),
    })),
  ];
  const envoyer = (
    m: ComplementMembre,
    change: Partial<{
      statut: StatutRapport | null;
      specialite: Specialite | null;
      diplome: DiplomeEleve | null;
    }>,
  ) =>
    regler.mutate({
      membre_id: m.membre_id,
      statut: m.statut,
      specialite: m.specialite,
      diplome: m.diplome,
      ...change,
    });
  return (
    <Section titre={t("complements.personnel")}>
      <MessageErreur erreur={liste.error ?? regler.error} />
      {membres.length === 0 ? (
        <p className="text-sm text-muet">{t("commun.aucune_ligne")}</p>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full border-collapse text-sm">
            <thead>
              <tr className="text-left text-muet">
                <th className="border-b border-muet px-2 py-1 font-medium">
                  {t("complements.personnel")}
                </th>
                <th className="border-b border-muet px-2 py-1 font-medium">
                  {t("complements.statut")}
                </th>
                <th className="border-b border-muet px-2 py-1 font-medium">
                  {t("complements.specialite")}
                </th>
                <th className="border-b border-muet px-2 py-1 font-medium">
                  {t("complements.diplome")}
                </th>
              </tr>
            </thead>
            <tbody>
              {membres.map((m) => (
                <tr key={m.membre_id}>
                  <th
                    scope="row"
                    className="border-b border-muet/30 px-2 py-1 text-left font-normal text-encre"
                  >
                    {m.nom}
                  </th>
                  <td className="border-b border-muet/30 px-2 py-1">
                    <select
                      aria-label={`${t("complements.statut")} · ${m.nom}`}
                      className={SAISIE}
                      disabled={!peutModifier}
                      value={m.statut ?? ""}
                      onChange={(e) =>
                        envoyer(m, {
                          statut:
                            e.target.value === ""
                              ? null
                              : (e.target.value as StatutRapport),
                        })
                      }
                    >
                      {choix(
                        STATUTS,
                        "statut",
                        t("complements.statut_deduit", {
                          valeur: t(`complements.statut.${m.statut_deduit}`),
                        }),
                      ).map((o) => (
                        <option key={o.valeur} value={o.valeur}>
                          {o.libelle}
                        </option>
                      ))}
                    </select>
                  </td>
                  <td className="border-b border-muet/30 px-2 py-1">
                    <select
                      aria-label={`${t("complements.specialite")} · ${m.nom}`}
                      className={SAISIE}
                      disabled={!peutModifier}
                      value={m.specialite ?? ""}
                      onChange={(e) =>
                        envoyer(m, {
                          specialite:
                            e.target.value === ""
                              ? null
                              : (e.target.value as Specialite),
                        })
                      }
                    >
                      {choix(
                        SPECIALITES,
                        "specialite",
                        m.specialite_proposee === null
                          ? ""
                          : t("complements.proposee", {
                              valeur: t(
                                `complements.specialite.${m.specialite_proposee}`,
                              ),
                            }),
                      ).map((o) => (
                        <option key={o.valeur} value={o.valeur}>
                          {o.libelle}
                        </option>
                      ))}
                    </select>
                  </td>
                  <td className="border-b border-muet/30 px-2 py-1">
                    <select
                      aria-label={`${t("complements.diplome")} · ${m.nom}`}
                      className={SAISIE}
                      disabled={!peutModifier}
                      value={m.diplome ?? ""}
                      onChange={(e) =>
                        envoyer(m, {
                          diplome:
                            e.target.value === ""
                              ? null
                              : (e.target.value as DiplomeEleve),
                        })
                      }
                    >
                      {choix(
                        DIPLOMES,
                        "diplome",
                        m.diplome_propose === null
                          ? ""
                          : t("complements.proposee", {
                              valeur: t(
                                `complements.diplome.${m.diplome_propose}`,
                              ),
                            }),
                      ).map((o) => (
                        <option key={o.valeur} value={o.valeur}>
                          {o.libelle}
                        </option>
                      ))}
                    </select>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </Section>
  );
}
