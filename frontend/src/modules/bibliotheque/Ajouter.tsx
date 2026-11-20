import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useMemo, useRef, useState } from "react";

import { appeler } from "../../api";
import { Bouton } from "../../composants/Bouton";
import { CaseACocher, Champ, Formulaire, Selection } from "../../composants/Champ";
import { MessageErreur } from "../../composants/Messages";
import { t } from "../../i18n";
import type { CatalogueBibliotheque, ChoixEcriture, DemandeAjoutContenu, EcritureOuverte, ExamenSujet, NatureContenu } from "../../types/gesco";
import { choisirWord } from "../../api/fichiers";
import { cleNotesWord } from "./Editeur";

type Sorte = "chapitre" | "lecon" | "quiz" | "sujet" | "document";

/** Libellé du choix et du bouton d'ajout de chaque sorte. */
function libelles(sorte: Sorte): { choix: string; bouton: string } {
  switch (sorte) {
    case "chapitre":
      return { choix: t("bibliotheque.ajouter.sorte.chapitre"), bouton: t("bibliotheque.ajouter.chapitre") };
    case "lecon":
      return { choix: t("bibliotheque.ajouter.sorte.lecon"), bouton: t("bibliotheque.ajouter.lecon") };
    case "quiz":
      return { choix: t("bibliotheque.ajouter.sorte.quiz"), bouton: t("bibliotheque.ajouter.quiz") };
    case "sujet":
      return { choix: t("bibliotheque.ajouter.sorte.sujet"), bouton: t("bibliotheque.ajouter.sujet") };
    case "document":
      return { choix: t("bibliotheque.ajouter.sorte.document"), bouton: t("bibliotheque.ajouter.document") };
  }
}

/** Un chapitre où l'utilisateur peut écrire, avec son chemin dans le catalogue. */
interface ChapitreEcrivable {
  id: string;
  cle: string;
  chemin: string;
  niveauCode: string;
  matiereId: string | null;
}

/** Chapitres où l'utilisateur écrit : toutes les matières, ou les siennes pour un maître. */
function chapitresEcrivables(catalogue: CatalogueBibliotheque): ChapitreEcrivable[] {
  const vus = new Set<string>();
  const liste: ChapitreEcrivable[] = [];
  for (const n of catalogue.niveaux)
    for (const s of n.series)
      for (const m of s.matieres) {
        if (!catalogue.ecrit_partout && !m.mienne) continue;
        for (const p of m.parties)
          for (const c of p.chapitres) {
            if (vus.has(c.id)) continue;
            vus.add(c.id);
            const serie = s.libelle === null ? "" : ` (${s.libelle})`;
            liste.push({ id: c.id, cle: c.cle, chemin: `${n.libelle}${serie} › ${m.libelle} › ${c.titre}`, niveauCode: n.code, matiereId: m.matiere_id });
          }
      }
  return liste;
}

/** Niveaux et matières où l'utilisateur écrit. */
function permis(choix: ChoixEcriture, niveauCode: string, matiereId: string): boolean {
  return choix.partout || choix.places.some((p) => p.niveau_code === niveauCode && p.matiere_id === matiereId);
}

/** Choix du niveau et de la matière, bornés à la portée de l'utilisateur. */
function NiveauMatiere({
  choix,
  niveauId,
  matiereId,
  surNiveau,
  surMatiere,
}: {
  choix: ChoixEcriture;
  niveauId: string;
  matiereId: string;
  surNiveau: (id: string) => void;
  surMatiere: (id: string) => void;
}) {
  const niveaux = choix.niveaux.filter((n) => choix.partout || choix.places.some((p) => p.niveau_code === n.code));
  const niveau = choix.niveaux.find((n) => n.id === niveauId);
  const matieres = choix.matieres.filter((m) => niveau === undefined || permis(choix, niveau.code, m.id));
  return (
    <div className="flex flex-wrap gap-3">
      <Selection
        id="ajout-niveau"
        libelle={t("bibliotheque.ajouter.niveau")}
        valeur={niveauId}
        surChangement={surNiveau}
        options={[{ valeur: "", libelle: t("bibliotheque.ajouter.choisir") }, ...niveaux.map((n) => ({ valeur: n.id, libelle: n.libelle }))]}
      />
      <Selection
        id="ajout-matiere"
        libelle={t("bibliotheque.ajouter.matiere")}
        valeur={matiereId}
        surChangement={surMatiere}
        options={[{ valeur: "", libelle: t("bibliotheque.ajouter.choisir") }, ...matieres.map((m) => ({ valeur: m.id, libelle: m.libelle }))]}
      />
    </div>
  );
}

/** Ajouter un chapitre de l'école : niveau, séries, matière, partie, titre et place. */
function AjoutChapitre({ catalogue, choix, surFait }: { catalogue: CatalogueBibliotheque; choix: ChoixEcriture; surFait: () => void }) {
  const client = useQueryClient();
  const [niveauId, setNiveauId] = useState("");
  const [matiereId, setMatiereId] = useState("");
  const [series, setSeries] = useState<string[]>([]);
  const [partie, setPartie] = useState("");
  const [titre, setTitre] = useState("");
  const [apres, setApres] = useState("");
  const niveau = choix.niveaux.find((n) => n.id === niveauId);
  // Chapitres de la matière dans le niveau, dans l'ordre du catalogue, et leurs parties.
  const { chapitres, parties } = useMemo(() => {
    const vus = new Map<string, string>();
    const p = new Set<string>();
    for (const n of catalogue.niveaux) {
      if (n.code !== niveau?.code) continue;
      for (const s of n.series)
        for (const m of s.matieres) {
          if (m.matiere_id !== matiereId) continue;
          for (const x of m.parties) {
            if (x.titre !== "") p.add(x.titre);
            for (const c of x.chapitres) vus.set(c.cle, c.titre);
          }
        }
    }
    return { chapitres: [...vus.entries()], parties: [...p] };
  }, [catalogue, niveau?.code, matiereId]);
  const ajouter = useMutation({
    mutationFn: () =>
      appeler("bibliotheque_chapitre_ajouter", {
        niveau_id: niveauId,
        matiere_id: matiereId,
        series,
        partie,
        titre,
        apres_cle: apres === "" ? null : apres,
      }),
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: ["bibliotheque", "catalogue"] });
      surFait();
    },
  });
  return (
    <Formulaire surEnvoi={() => ajouter.mutate()}>
      <NiveauMatiere choix={choix} niveauId={niveauId} matiereId={matiereId} surNiveau={(v) => { setNiveauId(v); setSeries([]); }} surMatiere={setMatiereId} />
      {niveau === undefined || niveau.series.length === 0 ? null : (
        <fieldset className="flex flex-col gap-1">
          <legend className="text-sm text-encre">{t("bibliotheque.ajouter.series")}</legend>
          {niveau.series.map((s) => (
            <CaseACocher
              key={s.code}
              id={`ajout-serie-${s.code}`}
              libelle={s.libelle}
              coche={series.includes(s.code)}
              surChangement={(c) => setSeries(c ? [...series, s.code] : series.filter((x) => x !== s.code))}
            />
          ))}
        </fieldset>
      )}
      <Champ id="ajout-partie" libelle={t("bibliotheque.ajouter.partie")} value={partie} onChange={(e) => setPartie(e.target.value)} list="ajout-parties" maxLength={100} />
      <datalist id="ajout-parties">
        {parties.map((p) => (
          <option key={p} value={p} />
        ))}
      </datalist>
      <Champ id="ajout-titre" libelle={t("bibliotheque.ajouter.titre")} value={titre} onChange={(e) => setTitre(e.target.value)} maxLength={200} required />
      <Selection
        id="ajout-place"
        libelle={t("bibliotheque.ajouter.place")}
        valeur={apres}
        surChangement={setApres}
        options={[
          { valeur: "", libelle: t("bibliotheque.ajouter.en_tete") },
          ...chapitres.map(([cle, titreChapitre]) => ({ valeur: cle, libelle: t("bibliotheque.ajouter.apres", { titre: titreChapitre }) })),
        ]}
      />
      <div>
        <Bouton type="submit" disabled={niveauId === "" || matiereId === "" || titre.trim() === "" || ajouter.isPending}>
          {libelles("chapitre").bouton}
        </Bouton>
      </div>
      <MessageErreur erreur={ajouter.error} />
    </Formulaire>
  );
}

/** Ajouter une leçon, un sujet ou un document ; il s'ouvre dans l'éditeur. */
function AjoutContenu({
  sorte,
  catalogue,
  choix,
  surOuvrir,
}: {
  sorte: Exclude<Sorte, "chapitre">;
  catalogue: CatalogueBibliotheque;
  choix: ChoixEcriture;
  surOuvrir: (id: string) => void;
}) {
  const client = useQueryClient();
  const chapitres = useMemo(() => chapitresEcrivables(catalogue), [catalogue]);
  const [chapitreId, setChapitreId] = useState("");
  const [niveauId, setNiveauId] = useState("");
  const [matiereId, setMatiereId] = useState("");
  const [titre, setTitre] = useState("");
  const [presentation, setPresentation] = useState("");
  const [examen, setExamen] = useState<ExamenSujet>("DEF");
  const [serie, setSerie] = useState("");
  const [session, setSession] = useState("");
  const niveau = choix.niveaux.find((n) => n.id === niveauId);
  const nature: NatureContenu = sorte;
  const sujetSansChapitre = sorte === "sujet";
  const demande = (): DemandeAjoutContenu => ({
    nature,
    chapitre_id: sujetSansChapitre ? null : chapitreId,
    niveau_id: sujetSansChapitre ? niveauId : null,
    matiere_id: sujetSansChapitre ? matiereId : null,
    titre,
    presentation,
    examen: sorte === "sujet" ? examen : null,
    serie_code: sorte === "sujet" && serie !== "" ? serie : null,
    session: sorte === "sujet" && session !== "" ? Number(session) : null,
    sujet_id: null,
    apres_cle: null,
  });
  const ajouter = useMutation({
    mutationFn: () => appeler("bibliotheque_contenu_ajouter", demande()),
    onSuccess: (e) => {
      void client.invalidateQueries({ queryKey: ["bibliotheque", "catalogue"] });
      client.setQueryData(["bibliotheque", "ecriture", e.contenu_id], e);
      surOuvrir(e.contenu_id);
    },
  });
  // La fiche se choisit d'abord : annuler le choix n'ajoute rien. Puis l'ajout, l'import et
  // l'éditeur, le brouillon déjà écrit. Si la fiche est refusée, le cours ajouté est gardé : un
  // nouvel essai l'utilise au lieu d'en ajouter un second.
  const ajoutWord = useRef<EcritureOuverte | null>(null);
  const depuisWord = useMutation({
    mutationFn: async () => {
      const chemin = await choisirWord(t("bibliotheque.word.filtre"));
      if (chemin === null) return null;
      const ajoute = ajoutWord.current ?? (await appeler("bibliotheque_contenu_ajouter", demande()));
      ajoutWord.current = ajoute;
      void client.invalidateQueries({ queryKey: ["bibliotheque", "catalogue"] });
      // Le cours ajouté naît avec son brouillon : la fiche en remplace le texte.
      return appeler("bibliotheque_word_importer", {
        contenu_id: ajoute.contenu_id,
        chemin,
        version_id: ajoute.brouillon?.version_id ?? null,
        revision: ajoute.brouillon?.revision ?? null,
      });
    },
    onSuccess: (r) => {
      if (r === null) return;
      ajoutWord.current = null;
      client.setQueryData(cleNotesWord(r.ecriture.contenu_id), r.notes);
      client.setQueryData(["bibliotheque", "ecriture", r.ecriture.contenu_id], r.ecriture);
      surOuvrir(r.ecriture.contenu_id);
    },
  });
  const pret = titre.trim() !== "" && (sujetSansChapitre ? niveauId !== "" && matiereId !== "" : chapitreId !== "");
  return (
    <Formulaire surEnvoi={() => ajouter.mutate()}>
      {sujetSansChapitre ? (
        <>
          <NiveauMatiere choix={choix} niveauId={niveauId} matiereId={matiereId} surNiveau={(v) => { setNiveauId(v); setSerie(""); }} surMatiere={setMatiereId} />
          <div className="flex flex-wrap gap-3">
            <Selection
              id="ajout-examen"
              libelle={t("bibliotheque.ajouter.examen")}
              valeur={examen}
              surChangement={(v) => setExamen(v === "BAC" ? "BAC" : "DEF")}
              options={[
                { valeur: "DEF", libelle: t("bibliotheque.ajouter.examen_def") },
                { valeur: "BAC", libelle: t("bibliotheque.ajouter.examen_bac") },
              ]}
            />
            {niveau === undefined || niveau.series.length === 0 ? null : (
              <Selection
                id="ajout-serie"
                libelle={t("bibliotheque.ajouter.serie")}
                valeur={serie}
                surChangement={setSerie}
                options={[{ valeur: "", libelle: t("bibliotheque.ajouter.toutes_series") }, ...niveau.series.map((s) => ({ valeur: s.code, libelle: s.libelle }))]}
              />
            )}
            <Champ id="ajout-session" libelle={t("bibliotheque.ajouter.session")} type="number" min={1950} max={2200} value={session} onChange={(e) => setSession(e.target.value)} />
          </div>
        </>
      ) : (
        <Selection
          id="ajout-chapitre"
          libelle={t("bibliotheque.ajouter.sous_chapitre")}
          valeur={chapitreId}
          surChangement={setChapitreId}
          options={[{ valeur: "", libelle: t("bibliotheque.ajouter.choisir") }, ...chapitres.map((c) => ({ valeur: c.id, libelle: c.chemin }))]}
        />
      )}
      <Champ id="ajout-titre-contenu" libelle={t("bibliotheque.ajouter.titre")} value={titre} onChange={(e) => setTitre(e.target.value)} maxLength={200} required />
      {sorte === "document" ? (
        <Champ id="ajout-presentation" libelle={t("bibliotheque.ajouter.presentation")} value={presentation} onChange={(e) => setPresentation(e.target.value)} maxLength={500} />
      ) : null}
      <div className="flex flex-wrap gap-2">
        <Bouton type="submit" disabled={!pret || ajouter.isPending || depuisWord.isPending}>
          {libelles(sorte).bouton}
        </Bouton>
        {sorte === "document" || sorte === "quiz" ? null : (
          <Bouton variante="secondaire" onClick={() => depuisWord.mutate()} disabled={!pret || ajouter.isPending || depuisWord.isPending}>
            {depuisWord.isPending ? t("bibliotheque.word.en_cours") : t("bibliotheque.ajouter.depuis_word")}
          </Bouton>
        )}
      </div>
      <MessageErreur erreur={ajouter.error ?? depuisWord.error} />
    </Formulaire>
  );
}

/** Le panneau « Ajouter » : chapitre, leçon, sujet ou document de l'école. */
export function Ajouter({
  catalogue,
  surOuvrir,
  surFermer,
}: {
  catalogue: CatalogueBibliotheque;
  surOuvrir: (id: string) => void;
  surFermer: () => void;
}) {
  const [sorte, setSorte] = useState<Sorte>("lecon");
  const choix = useQuery({
    queryKey: ["bibliotheque", "choix-ecriture"],
    queryFn: () => appeler("bibliotheque_ecriture_choix", null),
  });
  const sortes: Sorte[] = ["chapitre", "lecon", "quiz", "sujet", "document"];
  return (
    <section aria-label={t("bibliotheque.ajouter")} className="flex flex-col gap-3 rounded border border-muet bg-surface p-4">
      <div role="group" aria-label={t("bibliotheque.ajouter.quoi")} className="flex flex-wrap gap-1">
        {sortes.map((s) => (
          <Bouton key={s} variante={s === sorte ? "principal" : "secondaire"} aria-pressed={s === sorte} onClick={() => setSorte(s)}>
            {libelles(s).choix}
          </Bouton>
        ))}
        <Bouton variante="secondaire" onClick={surFermer}>
          {t("bibliotheque.retirer.annuler")}
        </Bouton>
      </div>
      <MessageErreur erreur={choix.error} />
      {choix.data === undefined ? null : sorte === "chapitre" ? (
        <AjoutChapitre catalogue={catalogue} choix={choix.data} surFait={surFermer} />
      ) : (
        <AjoutContenu key={sorte} sorte={sorte} catalogue={catalogue} choix={choix.data} surOuvrir={surOuvrir} />
      )}
    </section>
  );
}
