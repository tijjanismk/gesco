import { useQuery } from "@tanstack/react-query";
import { useEffect, useMemo, useState, type ReactNode } from "react";

import { appeler } from "../../api";
import { commandeAutorisee } from "../../app/navigation";
import { Bouton } from "../../composants/Bouton";
import { CaseACocher } from "../../composants/Champ";
import { MessageErreur } from "../../composants/Messages";
import { t } from "../../i18n";
import type {
  CatalogueBibliotheque,
  ContenuCatalogue,
  NiveauCatalogue,
  ResultatRechercheBibliotheque,
  SelectionExport,
} from "../../types/gesco";
import { RevisionPoste } from "./PasserQuiz";
import { Ajouter } from "./Ajouter";
import { DroitsPublication } from "./DroitsPublication";
import { ExporterLecons } from "./Echange";
import { ListeEnAttente, useLeconsAValider } from "./EnAttente";
import { SuiviEleves } from "./SuiviEleves";
import { Lecture, libelleNature } from "./Lecture";
import { type CiblePourQui, MarqueClasses, PourQui } from "./PourQui";
import { ListeRetraits, trouverPlace } from "./Retraits";

interface Filtres {
  mesMatieres: boolean;
  versionEcole: boolean;
  enAttente: boolean;
}

/** Garde du catalogue ce que les filtres retiennent ; une branche vide disparaît. */
function filtrer(niveaux: readonly NiveauCatalogue[], f: Filtres): NiveauCatalogue[] {
  const garde = (c: ContenuCatalogue) => (!f.versionEcole || c.version_ecole) && (!f.enAttente || c.en_attente);
  const filtrerContenus = !f.versionEcole && !f.enAttente;
  return niveaux
    .map((n) => ({
      ...n,
      series: n.series
        .map((s) => ({
          ...s,
          matieres: s.matieres
            .filter((m) => !f.mesMatieres || m.mienne)
            .map((m) => ({
              ...m,
              parties: m.parties
                .map((p) => ({
                  ...p,
                  chapitres: p.chapitres
                    .map((c) => ({ ...c, contenus: filtrerContenus ? c.contenus : c.contenus.filter(garde) }))
                    .filter((c) => filtrerContenus || c.contenus.length > 0),
                }))
                .filter((p) => p.chapitres.length > 0),
            }))
            .filter((m) => m.parties.length > 0),
        }))
        .filter((s) => s.matieres.length > 0),
    }))
    .filter((n) => n.series.length > 0);
}

/** Le texte d'un extrait, les mots trouvés en évidence. */
function Extrait({ texte }: { texte: string }) {
  const morceaux: ReactNode[] = [];
  let reste = texte;
  let i = 0;
  while (reste.length > 0) {
    const debut = reste.indexOf("\u0002");
    if (debut < 0) {
      morceaux.push(reste);
      break;
    }
    const fin = reste.indexOf("\u0003", debut);
    morceaux.push(reste.slice(0, debut));
    const mot = reste.slice(debut + 1, fin < 0 ? undefined : fin);
    morceaux.push(
      <mark key={i++} className="bg-fond font-semibold text-encre">
        {mot}
      </mark>,
    );
    reste = fin < 0 ? "" : reste.slice(fin + 1);
  }
  return <p className="text-sm text-encre">{morceaux}</p>;
}

/** Leçons cochées pour l'export, quand l'utilisateur choisit des leçons à envoyer. */
interface ChoixExport {
  coches: ReadonlySet<string>;
  basculer: (id: string) => void;
}

function LigneContenu({
  contenu,
  peutEcrire,
  choisi,
  surOuvrir,
  choix,
}: {
  contenu: ContenuCatalogue;
  peutEcrire: boolean;
  choisi: boolean;
  surOuvrir: (id: string) => void;
  choix: ChoixExport | null;
}) {
  return (
    <li className="flex items-start gap-1">
      {choix === null ? null : (
        <input
          type="checkbox"
          className="mt-2 size-4 shrink-0 accent-primaire"
          checked={choix.coches.has(contenu.id)}
          onChange={() => choix.basculer(contenu.id)}
          aria-label={t("bibliotheque.echange.cocher", { titre: contenu.titre })}
        />
      )}
      <button
        type="button"
        onClick={() => surOuvrir(contenu.id)}
        aria-current={choisi ? "true" : undefined}
        className={`flex w-full flex-col items-start rounded px-2 py-1 text-left text-sm focus-visible:outline-2 focus-visible:outline-primaire ${choisi ? "bg-primaire text-surface" : "text-encre hover:bg-fond"}`}
      >
        <span>{contenu.titre}</span>
        <span className={`text-xs ${choisi ? "text-surface" : "text-muet"}`}>
          {libelleNature(contenu.nature, contenu.examen, contenu.session)}
          {peutEcrire && contenu.version_ecole ? ` · ${t("bibliotheque.marque.version_ecole")}` : ""}
          {peutEcrire && contenu.en_attente ? ` · ${t("bibliotheque.marque.en_attente")}` : ""}
          {contenu.classes_seulement.length > 0 ? ` · ${t("bibliotheque.pour_qui.marque", { classes: contenu.classes_seulement.join(", ") })}` : ""}
        </span>
      </button>
    </li>
  );
}

/** Arbre du catalogue : niveau, série au lycée, matière, partie, chapitre, contenus. */
function Arbre({
  niveaux,
  peutEcrire,
  choisi,
  surOuvrir,
  choix,
  surExporter,
}: {
  niveaux: readonly NiveauCatalogue[];
  peutEcrire: boolean;
  choisi: string | null;
  surOuvrir: (id: string) => void;
  choix: ChoixExport | null;
  surExporter: (s: SelectionExport) => void;
}) {
  const ouvert = niveaux.length === 1;
  const [pourQui, setPourQui] = useState<CiblePourQui | null>(null);
  const [revision, setRevision] = useState<string | null>(null);
  return (
    <nav aria-label={t("bibliotheque.titre")} className="flex flex-col gap-1">
      {pourQui === null ? null : <PourQui cible={pourQui} surFermer={() => setPourQui(null)} />}
      {revision === null ? null : <RevisionPoste chapitreId={revision} surFermer={() => setRevision(null)} />}
      {niveaux.map((n) => (
        <details key={n.code} open={ouvert}>
          <summary className="cursor-pointer rounded px-1 py-1 font-semibold text-marine focus-visible:outline-2 focus-visible:outline-primaire">
            {n.libelle}
          </summary>
          <div className="ml-3 flex flex-col gap-1">
            {n.series.map((s) => {
              const matieres = s.matieres.map((m) => (
                <details key={m.cle}>
                  <summary className="cursor-pointer rounded px-1 py-1 text-encre focus-visible:outline-2 focus-visible:outline-primaire">
                    {m.libelle}
                  </summary>
                  <div className="ml-3 flex flex-col gap-1">
                    {peutEcrire ? (
                      <div className="flex flex-wrap gap-1 py-0.5">
                        <Bouton
                          variante="secondaire"
                          className="px-2! py-0.5! text-xs!"
                          onClick={() => surExporter({ type: "matiere", niveau_code: n.code, matiere_cle: m.cle })}
                        >
                          {t("bibliotheque.echange.exporter_matiere")}
                        </Bouton>
                      </div>
                    ) : null}
                    {m.parties.map((p) => (
                      <div key={p.titre}>
                        {p.titre === "" ? null : <p className="px-1 pt-1 text-xs font-semibold text-muet">{p.titre}</p>}
                        {p.chapitres.map((c) => (
                          <details key={c.id}>
                            <summary className="cursor-pointer rounded px-1 py-0.5 text-sm text-encre focus-visible:outline-2 focus-visible:outline-primaire">
                              {c.titre} <MarqueClasses classes={c.classes_seulement} />
                            </summary>
                            <div className="ml-3 flex flex-wrap gap-1 py-0.5">
                              {c.contenus.some((x) => x.nature === "lecon" || x.nature === "quiz") ? (
                                <Bouton variante="secondaire" className="px-2! py-0.5! text-xs!" onClick={() => setRevision(c.id)}>
                                  {t("quiz.reviser")}
                                </Bouton>
                              ) : null}
                            </div>
                            {peutEcrire ? (
                              <div className="ml-3 flex flex-wrap gap-1 py-0.5">
                                {c.origine === "ecole" ? (
                                  <Bouton variante="secondaire" className="px-2! py-0.5! text-xs!" onClick={() => setPourQui({ contenu_id: null, chapitre_id: c.id })}>
                                    {t("bibliotheque.pour_qui.bouton")}
                                  </Bouton>
                                ) : null}
                                <Bouton
                                  variante="secondaire"
                                  className="px-2! py-0.5! text-xs!"
                                  onClick={() => surExporter({ type: "chapitre", chapitre_cle: c.cle })}
                                >
                                  {t("bibliotheque.echange.exporter")}
                                </Bouton>
                              </div>
                            ) : null}
                            <ul className="ml-3">
                              {c.contenus.map((x) => (
                                <LigneContenu key={x.id} contenu={x} peutEcrire={peutEcrire} choisi={x.id === choisi} surOuvrir={surOuvrir} choix={choix} />
                              ))}
                            </ul>
                          </details>
                        ))}
                      </div>
                    ))}
                  </div>
                </details>
              ));
              return s.code === null ? (
                <div key="sans-serie" className="flex flex-col gap-1">
                  {matieres}
                </div>
              ) : (
                <details key={s.code}>
                  <summary className="cursor-pointer rounded px-1 py-1 text-encre focus-visible:outline-2 focus-visible:outline-primaire">
                    {s.libelle ?? s.code}
                  </summary>
                  <div className="ml-3 flex flex-col gap-1">{matieres}</div>
                </details>
              );
            })}
          </div>
        </details>
      ))}
    </nav>
  );
}

/** Résultats d'une recherche, groupés par niveau et matière. */
function Resultats({
  resultats,
  choisi,
  surOuvrir,
}: {
  resultats: readonly ResultatRechercheBibliotheque[];
  choisi: string | null;
  surOuvrir: (id: string) => void;
}) {
  if (resultats.length === 0) return <p className="text-sm text-muet">{t("bibliotheque.rechercher.aucun")}</p>;
  const groupes: { titre: string; lignes: ResultatRechercheBibliotheque[] }[] = [];
  for (const r of resultats) {
    const titre = `${r.niveau} · ${r.matiere}`;
    const dernier = groupes.at(-1);
    if (dernier?.titre === titre) dernier.lignes.push(r);
    else groupes.push({ titre, lignes: [r] });
  }
  return (
    <div className="flex flex-col gap-3">
      {groupes.map((g) => (
        <section key={g.titre} className="flex flex-col gap-1">
          <h3 className="text-sm font-semibold text-marine">{g.titre}</h3>
          <ul className="flex flex-col gap-1">
            {g.lignes.map((r) => (
              <li key={r.contenu_id}>
                <button
                  type="button"
                  onClick={() => surOuvrir(r.contenu_id)}
                  aria-current={r.contenu_id === choisi ? "true" : undefined}
                  className="flex w-full flex-col items-start gap-0.5 rounded px-2 py-1 text-left hover:bg-fond focus-visible:outline-2 focus-visible:outline-primaire"
                >
                  <span className="text-sm font-medium text-encre">{r.titre}</span>
                  <span className="text-xs text-muet">
                    {libelleNature(r.nature, null, null)} · {r.chapitre}
                  </span>
                  <Extrait texte={r.extrait} />
                </button>
              </li>
            ))}
          </ul>
        </section>
      ))}
    </div>
  );
}

/** Filtres proposés selon ce que l'utilisateur fait : ses matières pour un maître, la version de
 *  l'école pour qui écrit, l'attente de validation pour qui valide. */
function BarreFiltres({
  catalogue,
  filtres,
  surChanger,
}: {
  catalogue: CatalogueBibliotheque;
  filtres: Filtres;
  surChanger: (f: Filtres) => void;
}) {
  const aDesMatieres = catalogue.niveaux.some((n) => n.series.some((s) => s.matieres.some((m) => m.mienne)));
  return (
    <div className="flex flex-col gap-1">
      {aDesMatieres ? (
        <CaseACocher
          id="bibliotheque-mes-matieres"
          libelle={t("bibliotheque.filtre.mes_matieres")}
          coche={filtres.mesMatieres}
          surChangement={(v) => surChanger({ ...filtres, mesMatieres: v })}
        />
      ) : null}
      {catalogue.peut_ecrire ? (
        <CaseACocher
          id="bibliotheque-version-ecole"
          libelle={t("bibliotheque.filtre.version_ecole")}
          coche={filtres.versionEcole}
          surChangement={(v) => surChanger({ ...filtres, versionEcole: v })}
        />
      ) : null}
      {catalogue.peut_valider ? (
        <CaseACocher
          id="bibliotheque-en-attente"
          libelle={t("bibliotheque.filtre.en_attente")}
          coche={filtres.enAttente}
          surChangement={(v) => surChanger({ ...filtres, enAttente: v })}
        />
      ) : null}
    </div>
  );
}

/** Bibliothèque de cours : recherche en tête, catalogue à gauche, contenu à droite. */
export function Bibliotheque() {
  const catalogue = useQuery({
    queryKey: ["bibliotheque", "catalogue"],
    queryFn: () => appeler("bibliotheque_catalogue", null),
  });
  const [filtres, setFiltres] = useState<Filtres | null>(null);
  const [choisi, setChoisi] = useState<string | null>(null);
  const [saisie, setSaisie] = useState("");
  const [cherche, setCherche] = useState("");
  const [retires, setRetires] = useState(false);
  const [ajout, setAjout] = useState(false);
  const [enEcriture, setEnEcriture] = useState(false);
  const [attente, setAttente] = useState(false);
  const [droits, setDroits] = useState(false);
  const [suiviOuvert, setSuiviOuvert] = useState(false);
  const session = useQuery({ queryKey: ["session"], queryFn: () => appeler("session_courante", null) });
  const peutSuivre =
    session.data !== undefined && commandeAutorisee("bibliotheque_suivi_lire", session.data.permissions, session.data.maitre);
  // Leçons cochées pour l'export (null : pas de choix en cours) et export ouvert.
  const [coches, setCoches] = useState<ReadonlySet<string> | null>(null);
  const [exportant, setExportant] = useState<SelectionExport | null>(null);
  const choix: ChoixExport | null =
    coches === null
      ? null
      : {
          coches,
          basculer: (id) => {
            const suivant = new Set(coches);
            if (suivant.has(id)) suivant.delete(id);
            else suivant.add(id);
            setCoches(suivant);
          },
        };
  const aValider = useLeconsAValider(catalogue.data?.peut_valider === true);
  const ouvrir = (id: string) => {
    setEnEcriture(false);
    setChoisi(id);
  };
  const ecrire = (id: string) => {
    setAjout(false);
    setEnEcriture(true);
    setChoisi(id);
  };
  useEffect(() => {
    const minuterie = window.setTimeout(() => setCherche(saisie.trim()), 250);
    return () => window.clearTimeout(minuterie);
  }, [saisie]);
  const recherche = useQuery({
    queryKey: ["bibliotheque", "recherche", cherche],
    queryFn: () => appeler("bibliotheque_rechercher", { texte: cherche }),
    enabled: cherche !== "",
  });
  const donnees = catalogue.data;
  // Le maître arrive sur ses matières ; la direction sur tout.
  const maitre = donnees?.maitre ?? false;
  const enCours: Filtres = useMemo(
    () => filtres ?? { mesMatieres: maitre, versionEcole: false, enAttente: false },
    [filtres, maitre],
  );
  const niveaux = useMemo(() => (donnees === undefined ? [] : filtrer(donnees.niveaux, enCours)), [donnees, enCours]);
  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h1 className="text-2xl font-semibold text-marine">{t("bibliotheque.titre")}</h1>
        <div className="flex flex-wrap gap-2">
        {donnees?.peut_valider === true ? (
          <button
            type="button"
            onClick={() => setAttente(!attente)}
            aria-pressed={attente}
            className="rounded border border-muet px-3 py-1.5 text-sm text-encre hover:bg-fond focus-visible:outline-2 focus-visible:outline-primaire"
          >
            {aValider > 0 ? t("bibliotheque.en_attente.compte", { nombre: aValider }) : t("bibliotheque.en_attente")}
          </button>
        ) : null}
        {donnees?.peut_valider === true ? (
          <button
            type="button"
            onClick={() => setDroits(!droits)}
            aria-pressed={droits}
            className="rounded border border-muet px-3 py-1.5 text-sm text-encre hover:bg-fond focus-visible:outline-2 focus-visible:outline-primaire"
          >
            {t("bibliotheque.droits.titre")}
          </button>
        ) : null}
        {peutSuivre ? (
          <button
            type="button"
            onClick={() => setSuiviOuvert(!suiviOuvert)}
            aria-pressed={suiviOuvert}
            className="rounded border border-muet px-3 py-1.5 text-sm text-encre hover:bg-fond focus-visible:outline-2 focus-visible:outline-primaire"
          >
            {t("suivi.titre")}
          </button>
        ) : null}
        {donnees?.peut_ecrire === true ? (
          <button
            type="button"
            onClick={() => setAjout(!ajout)}
            aria-pressed={ajout}
            className="rounded border border-muet px-3 py-1.5 text-sm text-encre hover:bg-fond focus-visible:outline-2 focus-visible:outline-primaire"
          >
            {t("bibliotheque.ajouter")}
          </button>
        ) : null}
        {donnees?.peut_ecrire === true ? (
          <button
            type="button"
            onClick={() => setCoches(coches === null ? new Set() : null)}
            aria-pressed={coches !== null}
            className="rounded border border-muet px-3 py-1.5 text-sm text-encre hover:bg-fond focus-visible:outline-2 focus-visible:outline-primaire"
          >
            {t("bibliotheque.echange.choisir")}
          </button>
        ) : null}
        {donnees?.peut_gerer === true ? (
          <button
            type="button"
            onClick={() => setRetires(!retires)}
            aria-pressed={retires}
            className="rounded border border-muet px-3 py-1.5 text-sm text-encre hover:bg-fond focus-visible:outline-2 focus-visible:outline-primaire"
          >
            {retires ? t("bibliotheque.retires.retour") : t("bibliotheque.retires")}
          </button>
        ) : null}
        </div>
      </div>
      {ajout && donnees !== undefined ? <Ajouter catalogue={donnees} surOuvrir={ecrire} surFermer={() => setAjout(false)} /> : null}
      {retires ? <ListeRetraits /> : null}
      {exportant === null ? null : <ExporterLecons key={JSON.stringify(exportant)} selection={exportant} surFin={() => setExportant(null)} />}
      {coches === null ? null : (
        <div className="flex flex-wrap items-center gap-2 rounded border border-muet/40 p-2 text-sm text-encre">
          <span>{t("bibliotheque.echange.cochees", { nombre: coches.size })}</span>
          <Bouton onClick={() => setExportant({ type: "contenus", contenu_ids: [...coches] })} disabled={coches.size === 0}>
            {t("bibliotheque.echange.exporter_selection")}
          </Bouton>
          <Bouton variante="secondaire" onClick={() => setCoches(null)}>
            {t("bibliotheque.echange.choix_fin")}
          </Bouton>
        </div>
      )}
      {droits ? <DroitsPublication /> : null}
      {suiviOuvert && peutSuivre ? <SuiviEleves /> : null}
      {attente ? (
        <ListeEnAttente
          surOuvrir={(id) => {
            setAttente(false);
            ecrire(id);
          }}
        />
      ) : null}
      <div className="flex flex-wrap items-end gap-2">
        <label htmlFor="bibliotheque-recherche" className="flex min-w-64 flex-1 flex-col gap-1 text-sm text-encre">
          <span>{t("bibliotheque.rechercher")}</span>
          <input
            id="bibliotheque-recherche"
            type="search"
            value={saisie}
            onChange={(e) => setSaisie(e.target.value)}
            placeholder={t("bibliotheque.rechercher.aide")}
            className="rounded border border-muet bg-surface px-2 py-1.5 text-primaire focus:outline-2 focus:outline-primaire"
          />
        </label>
        {saisie === "" ? null : (
          <button
            type="button"
            onClick={() => setSaisie("")}
            className="rounded border border-muet px-3 py-1.5 text-sm text-encre hover:bg-fond focus-visible:outline-2 focus-visible:outline-primaire"
          >
            {t("bibliotheque.rechercher.effacer")}
          </button>
        )}
      </div>
      <MessageErreur erreur={catalogue.error} />
      <div className={`grid gap-4 md:grid-cols-3 ${retires ? "hidden" : ""}`}>
        <div className="flex flex-col gap-3 md:col-span-1">
          {cherche !== "" ? (
            <>
              <MessageErreur erreur={recherche.error} />
              {recherche.data === undefined ? null : <Resultats resultats={recherche.data} choisi={choisi} surOuvrir={ouvrir} />}
            </>
          ) : donnees === undefined ? null : (
            <>
              <BarreFiltres catalogue={donnees} filtres={enCours} surChanger={setFiltres} />
              {niveaux.length === 0 ? (
                <p className="text-sm text-muet">
                  {donnees.niveaux.length === 0 ? t("bibliotheque.vide") : t("bibliotheque.vide.filtre")}
                </p>
              ) : (
                <Arbre niveaux={niveaux} peutEcrire={donnees.peut_ecrire} choisi={choisi} surOuvrir={ouvrir} choix={choix} surExporter={setExportant} />
              )}
            </>
          )}
        </div>
        <div className="rounded border border-muet bg-surface p-4 md:col-span-2">
          {choisi === null ? (
            <p className="text-sm text-muet">{t("bibliotheque.choisir")}</p>
          ) : (
            <Lecture
              key={`${choisi}-${enEcriture ? "e" : "l"}`}
              contenuId={choisi}
              surOuvrir={ouvrir}
              surEcrire={ecrire}
              modeInitial={enEcriture ? "ecrire" : "lire"}
              place={donnees === undefined ? null : trouverPlace(donnees, choisi)}
              peutGerer={donnees?.peut_gerer === true}
            />
          )}
        </div>
      </div>
    </div>
  );
}
