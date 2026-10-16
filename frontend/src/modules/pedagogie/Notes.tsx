import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { type KeyboardEvent, useRef, useState } from "react";

import { appeler, ErreurApi } from "../../api";
import { choisirClasseur, choisirTableur } from "../../api/fichiers";
import { commandeAutorisee } from "../../app/navigation";
import { Bouton } from "../../composants/Bouton";
import { Champ, Selection } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { type Cle, dateCivile, t, texteDeCle } from "../../i18n";
import type { ApercuImportNotes, EtatImportNote, LigneGrille, NoteEleve, TypeEvaluation } from "../../types/gesco";
import { lirePoints, points } from "./saisies";
import { aujourdHui } from "../../app/aujourdhui";

const ETATS_IMPORT: Record<EtatImportNote, Cle> = {
  nouvelle: "import_notes.etat.nouvelle",
  modifiee: "import_notes.etat.modifiee",
  inchangee: "import_notes.etat.inchangee",
  vide: "import_notes.etat.vide",
  erreur: "import_notes.etat.erreur",
};

const TYPES: Record<TypeEvaluation, Cle> = {
  devoir: "evaluation.type.devoir",
  composition: "evaluation.type.composition",
};

/** Ce que la case affiche pour une note enregistrée. */
function texteNote(note: NoteEleve | null): string {
  if (note === null) return "";
  if (note.dispense) return "d";
  if (note.absent) return note.justifie ? "aj" : "a";
  return note.valeur === null ? "" : points(note.valeur);
}

/** Notes (SPEC_PHASE_4 §4, UC-03) : évaluations d'une classe, grille de saisie au clavier. */
export function Notes() {
  const session = useQuery({ queryKey: ["session"], queryFn: () => appeler("session_courante", null) });
  const annees = useQuery({ queryKey: ["annees"], queryFn: () => appeler("annees_lister", null) });
  const anneeId = annees.data?.find((a) => a.est_courante)?.id ?? null;
  const classes = useQuery({ queryKey: ["classes", anneeId], queryFn: () => appeler("classes_lister", { annee_id: anneeId ?? "" }), enabled: anneeId !== null });
  const pedagogie = useQuery({ queryKey: ["pedagogie", anneeId], queryFn: () => appeler("pedagogie_lire", { id: anneeId ?? "" }), enabled: anneeId !== null });
  const portee = useQuery({ queryKey: ["portee-notes"], queryFn: () => appeler("portee_notes_lire", null) });
  const [classeChoisie, setClasseChoisie] = useState<string | null>(null);
  const [periodeChoisie, setPeriodeChoisie] = useState<string | null>(null);
  const [matiereChoisie, setMatiereChoisie] = useState<string | null>(null);
  const [ouverte, setOuverte] = useState<string | null>(null);
  const permissions = session.data?.permissions ?? [];
  const autorise = (classe: string, matiere: string) =>
    portee.data === undefined || portee.data.type === "etablissement" || portee.data.couples.some(([c, m]) => c === classe && m === matiere);
  const visibles = (classes.data ?? []).filter((c) => portee.data === undefined || portee.data.type === "etablissement" || portee.data.couples.some(([cl]) => cl === c.classe.id));
  const classe = visibles.find((c) => c.classe.id === classeChoisie) ?? visibles[0];
  const periodes = (pedagogie.data?.periodes ?? []).filter((p) => classe !== undefined && p.periode.niveau_id === classe.classe.niveau_id);
  const periode = periodes.find((p) => p.periode.id === periodeChoisie) ?? periodes[0];
  const programme = (pedagogie.data?.programme ?? []).filter(
    (l) => classe !== undefined && l.niveau_id === classe.classe.niveau_id && l.serie_id === classe.classe.serie_id && autorise(classe.classe.id, l.matiere_id),
  );
  const matieres = programme.map((l) => pedagogie.data?.matieres.find((m) => m.id === l.matiere_id)).filter((m) => m !== undefined);
  const matiere = matieres.find((m) => m.id === matiereChoisie) ?? matieres[0];
  if (ouverte !== null) return <Grille evaluationId={ouverte} permissions={permissions} surFermer={() => setOuverte(null)} />;
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("notes.titre")}</h1>
      <MessageErreur erreur={classes.error ?? pedagogie.error ?? portee.error} />
      {classe === undefined ? (
        <p className="text-sm text-muet">{t("notes.aucune_classe")}</p>
      ) : (
        <>
          <div className="flex flex-wrap items-end gap-3">
            <Selection id="notes-classe" libelle={t("enseignants.classe")} valeur={classe.classe.id} surChangement={setClasseChoisie} options={visibles.map((c) => ({ valeur: c.classe.id, libelle: `${c.classe.libelle} · ${c.niveau}` }))} />
            <Selection id="notes-periode" libelle={t("notes.periode")} valeur={periode?.periode.id ?? ""} surChangement={setPeriodeChoisie} options={periodes.map((p) => ({ valeur: p.periode.id, libelle: p.libelle }))} />
            <Selection id="notes-matiere" libelle={t("programme.matiere")} valeur={matiere?.id ?? ""} surChangement={setMatiereChoisie} options={matieres.map((m) => ({ valeur: m.id, libelle: m.libelle }))} />
          </div>
          {periode === undefined || matiere === undefined ? null : (
            <Evaluations
              classeId={classe.classe.id}
              periodeId={periode.periode.id}
              matiereId={matiere.id}
              cycle={pedagogie.data?.niveaux.find((n) => n.id === classe.classe.niveau_id)?.cycle ?? null}
              regles={pedagogie.data?.regles ?? []}
              surOuvrir={setOuverte}
            />
          )}
        </>
      )}
    </div>
  );
}

function Evaluations({
  classeId,
  periodeId,
  matiereId,
  cycle,
  regles,
  surOuvrir,
}: {
  classeId: string;
  periodeId: string;
  matiereId: string;
  cycle: string | null;
  regles: readonly { cycle: string; contenu: { types: readonly { type_evaluation: TypeEvaluation; bareme: number }[] } }[];
  surOuvrir: (id: string) => void;
}) {
  const client = useQueryClient();
  const liste = useQuery({ queryKey: ["evaluations", classeId, periodeId], queryFn: () => appeler("evaluations_lister", { classe_id: classeId, periode_id: periodeId }) });
  const types = regles.find((r) => r.cycle === cycle)?.contenu.types ?? [];
  const deLaMatiere = (liste.data ?? []).filter((e) => e.evaluation.matiere_id === matiereId);
  const [type, setType] = useState<TypeEvaluation | null>(null);
  const typeChoisi = type ?? types[0]?.type_evaluation ?? "composition";
  const numero = deLaMatiere.filter((e) => e.evaluation.type_evaluation === typeChoisi).length + 1;
  const [libelle, setLibelle] = useState("");
  const [date, setDate] = useState(aujourdHui);
  const [bareme, setBareme] = useState("");
  const baremeDefaut = types.find((x) => x.type_evaluation === typeChoisi)?.bareme ?? 2000;
  const creation = useMutation({
    mutationFn: () =>
      appeler("evaluation_creer", {
        classe_id: classeId,
        periode_id: periodeId,
        matiere_id: matiereId,
        type_evaluation: typeChoisi,
        libelle: libelle.trim() === "" ? `${t(TYPES[typeChoisi])} ${numero}` : libelle,
        date,
        bareme: bareme.trim() === "" ? null : lirePoints(bareme),
      }),
    onSuccess: (e) => {
      void client.invalidateQueries({ queryKey: ["evaluations"] });
      setLibelle("");
      setBareme("");
      surOuvrir(e.id);
    },
  });
  return (
    <>
      <Section titre={t("notes.evaluations")}>
        {deLaMatiere.length === 0 ? <p className="text-sm text-muet">{t("notes.aucune_evaluation")}</p> : null}
        <ul className="flex flex-col gap-1">
          {deLaMatiere.map((e) => (
            <li key={e.evaluation.id}>
              <button type="button" className="text-left text-sm text-primaire underline" onClick={() => surOuvrir(e.evaluation.id)}>
                {t("notes.resume", {
                  libelle: e.evaluation.libelle,
                  date: dateCivile(e.evaluation.date),
                  bareme: points(e.evaluation.bareme),
                  saisies: String(e.saisies),
                  effectif: String(e.effectif),
                  statut: t(e.evaluation.statut === "validee" ? "notes.statut.validee" : "notes.statut.en_saisie"),
                })}
              </button>
            </li>
          ))}
        </ul>
      </Section>
      <Section titre={t("notes.nouvelle")}>
        <div className="flex flex-wrap items-end gap-3">
          <Selection id="evaluation-type" libelle={t("notes.type")} valeur={typeChoisi} surChangement={(v) => setType(v as TypeEvaluation)} options={types.map((x) => ({ valeur: x.type_evaluation, libelle: t(TYPES[x.type_evaluation]) }))} />
          <Champ id="evaluation-libelle" libelle={t("notes.libelle")} placeholder={`${t(TYPES[typeChoisi])} ${numero}`} value={libelle} onChange={(e) => setLibelle(e.target.value)} />
          <Champ id="evaluation-date" type="date" libelle={t("notes.date")} value={date} onChange={(e) => setDate(e.target.value)} />
          <div className="w-24">
            <Champ id="evaluation-bareme" libelle={t("regle.bareme")} placeholder={points(baremeDefaut)} value={bareme} onChange={(e) => setBareme(e.target.value)} />
          </div>
          <Bouton onClick={() => creation.mutate()} disabled={creation.isPending}>{t("notes.creer")}</Bouton>
        </div>
        <MessageErreur erreur={creation.error ?? liste.error} />
      </Section>
    </>
  );
}

function Grille({ evaluationId, permissions, surFermer }: { evaluationId: string; permissions: readonly string[]; surFermer: () => void }) {
  const client = useQueryClient();
  const grille = useQuery({ queryKey: ["grille", evaluationId], queryFn: () => appeler("notes_grille", { id: evaluationId }) });
  const [textes, setTextes] = useState<Record<string, string>>({});
  const [erreurs, setErreurs] = useState<Record<string, string>>({});
  const [enregistrees, setEnregistrees] = useState<Record<string, NoteEleve | null>>({});
  const cases = useRef<Record<string, HTMLInputElement | null>>({});
  // Notes parties au serveur et pas encore revenues : la validation les attend.
  const enCours = useRef(new Set<Promise<void>>());
  const [motif, setMotif] = useState("");
  const rafraichir = () => {
    void client.invalidateQueries({ queryKey: ["grille", evaluationId] });
    void client.invalidateQueries({ queryKey: ["evaluations"] });
  };
  const validation = useMutation({
    mutationFn: async () => {
      await Promise.all([...enCours.current]);
      return appeler("evaluation_valider", { id: evaluationId });
    },
    onSuccess: rafraichir,
  });
  const deverrouillage = useMutation({ mutationFn: () => appeler("evaluation_deverrouiller", { id: evaluationId, motif }), onSuccess: rafraichir });
  const g = grille.data;
  if (g === undefined) return <MessageErreur erreur={grille.error} />;
  const noteDe = (l: LigneGrille) => (l.inscription_id in enregistrees ? (enregistrees[l.inscription_id] ?? null) : l.note);
  const texteDe = (l: LigneGrille) => textes[l.inscription_id] ?? texteNote(noteDe(l));
  const enregistrer = (i: number, suivante: boolean) => {
    const envoi = envoyer(i, suivante);
    enCours.current.add(envoi);
    void envoi.finally(() => enCours.current.delete(envoi));
    return envoi;
  };
  const envoyer = async (i: number, suivante: boolean) => {
    const l = g.lignes[i];
    if (l === undefined) return;
    const texte = texteDe(l);
    if (texte === texteNote(noteDe(l))) {
      if (suivante) cases.current[g.lignes[i + 1]?.inscription_id ?? ""]?.focus();
      return;
    }
    try {
      const note = await appeler("note_saisir", { evaluation_id: evaluationId, inscription_id: l.inscription_id, saisie: texte });
      setEnregistrees((e) => ({ ...e, [l.inscription_id]: note }));
      setTextes((x) => ({ ...x, [l.inscription_id]: texteNote(note) }));
      setErreurs((x) => ({ ...x, [l.inscription_id]: "" }));
      if (suivante) cases.current[g.lignes[i + 1]?.inscription_id ?? ""]?.focus();
    } catch (erreur) {
      setErreurs((x) => ({ ...x, [l.inscription_id]: erreur instanceof ErreurApi ? texteDeCle(erreur.cle) : t("erreur.transport") }));
    }
  };
  const surTouche = (i: number) => (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") {
      e.preventDefault();
      void enregistrer(i, true);
    }
  };
  const saisies = g.lignes.filter((l) => noteDe(l) !== null).length;
  // Moyenne des notes saisies, tenue à jour à chaque Entrée ; tronquée au centième.
  const valeurs = g.lignes.map((l) => noteDe(l)?.valeur ?? null).filter((v) => v !== null);
  const moyenne = valeurs.length === 0 ? null : Math.trunc(valeurs.reduce((a, b) => a + b, 0) / valeurs.length);
  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center gap-3">
        <h1 className="text-xl font-semibold text-encre">{`${g.classe} · ${g.matiere} · ${g.evaluation.libelle}`}</h1>
        <Bouton variante="secondaire" onClick={surFermer}>{t("notes.retour")}</Bouton>
      </div>
      <p className="text-sm text-muet">{t("notes.aide_saisie", { bareme: points(g.evaluation.bareme) })}</p>
      {g.modifiable ? null : <p className="text-sm text-ocre">{t("notes.validee")}</p>}
      <table className="w-full max-w-3xl text-left text-sm">
        <thead>
          <tr className="text-muet">
            <th className="px-2 py-1 font-normal">{t("notes.numero")}</th>
            <th className="px-2 py-1 font-normal">{t("impayes.eleve")}</th>
            <th className="px-2 py-1 font-normal">{t("impayes.matricule")}</th>
            <th className="px-2 py-1 font-normal">{t("notes.note_sur", { bareme: points(g.evaluation.bareme) })}</th>
            <th className="px-2 py-1 font-normal" />
          </tr>
        </thead>
        <tbody>
          {g.lignes.map((l, i) => {
            const note = noteDe(l);
            const erreur = erreurs[l.inscription_id] ?? "";
            return (
              <tr key={l.inscription_id} className="border-b border-muet/30">
                <td className="px-2 py-1 text-muet">{i + 1}</td>
                <td className="px-2 py-1 text-encre">{l.eleve}</td>
                <td className="px-2 py-1 text-muet">{l.matricule}</td>
                <td className="px-2 py-1">
                  <input
                    id={`note-${i + 1}`}
                    ref={(el) => {
                      cases.current[l.inscription_id] = el;
                    }}
                    aria-label={t("notes.note_de", { eleve: l.eleve })}
                    autoFocus={i === 0 && g.modifiable}
                    disabled={!g.modifiable}
                    className={`w-20 rounded border bg-surface px-2 py-1 text-primaire ${erreur === "" ? "border-muet" : "border-rouge"}`}
                    value={texteDe(l)}
                    onChange={(e) => setTextes((x) => ({ ...x, [l.inscription_id]: e.target.value }))}
                    onKeyDown={surTouche(i)}
                    onBlur={() => void enregistrer(i, false)}
                  />
                </td>
                <td className="px-2 py-1 text-sm">
                  {erreur !== "" ? (
                    <span className="text-rouge">{erreur}</span>
                  ) : note === null ? null : note.dispense ? (
                    <span className="text-muet">{t("notes.dispense")}</span>
                  ) : note.absent ? (
                    <span className="text-ocre">{t(note.justifie ? "notes.absent_justifie" : "notes.absent")}</span>
                  ) : (
                    <span className="text-vert">{t("notes.enregistree")}</span>
                  )}
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
      <p role="status" className="text-sm text-encre">
        {t("notes.bilan", { saisies: String(saisies), effectif: String(g.lignes.length) })}
        {moyenne === null ? "" : ` ${t("notes.moyenne", { moyenne: points(moyenne) })}`}
      </p>
      <div className="flex flex-wrap items-end gap-2">
        {g.modifiable && commandeAutorisee("evaluation_valider", permissions) ? (
          <Bouton onClick={() => validation.mutate()} disabled={validation.isPending}>{t("notes.valider")}</Bouton>
        ) : null}
        {!g.modifiable && commandeAutorisee("evaluation_deverrouiller", permissions) ? (
          <>
            <Champ id="evaluation-motif" libelle={t("notes.motif")} value={motif} onChange={(e) => setMotif(e.target.value)} />
            <Bouton variante="secondaire" onClick={() => deverrouillage.mutate()} disabled={deverrouillage.isPending}>{t("notes.deverrouiller")}</Bouton>
          </>
        ) : null}
      </div>
      <MessageErreur erreur={validation.error ?? deverrouillage.error} />
      <GrilleExcel
        evaluationId={evaluationId}
        titre={`${g.classe} — ${g.matiere} — ${g.evaluation.libelle}`}
        entetes={[t("notes.numero"), t("impayes.matricule"), t("impayes.eleve"), t("notes.note_sur", { bareme: points(g.evaluation.bareme) })]}
        lignes={g.lignes.map((l, i) => [String(i + 1), l.matricule, l.eleve, texteDe(l)])}
        importable={g.modifiable && commandeAutorisee("notes_importer", permissions)}
        surImporte={() => {
          setTextes({});
          setEnregistrees({});
          setErreurs({});
          rafraichir();
        }}
      />
    </div>
  );
}

/** Grille Excel (SPEC_PHASE_4 §4.5) : la liste de la classe exportée pour l'enseignant, puis sa
 *  grille remplie relue, montrée avant d'être enregistrée. */
function GrilleExcel({
  evaluationId,
  titre,
  entetes,
  lignes,
  importable,
  surImporte,
}: {
  evaluationId: string;
  titre: string;
  entetes: string[];
  lignes: string[][];
  importable: boolean;
  surImporte: () => void;
}) {
  const [chemin, setChemin] = useState<string | null>(null);
  const [apercu, setApercu] = useState<ApercuImportNotes | null>(null);
  const exporter = useMutation({
    mutationFn: async () => {
      const cible = await choisirClasseur(titre.replaceAll(/[\\/:*?"<>|]/g, "-"), t("commun.classeur_excel"));
      if (cible === null) return null;
      return appeler("exporter_xlsx", { chemin: cible, titre, entetes, lignes });
    },
  });
  const analyser = useMutation({
    mutationFn: async () => {
      const fichier = await choisirTableur(t("import_notes.filtre"));
      if (fichier === null) return null;
      setChemin(fichier);
      return appeler("notes_import_analyser", { evaluation_id: evaluationId, chemin: fichier });
    },
    onSuccess: (a) => setApercu(a),
  });
  const importer = useMutation({
    mutationFn: () => appeler("notes_importer", { evaluation_id: evaluationId, chemin: chemin ?? "" }),
    onSuccess: (bilan) => {
      setApercu(null);
      surImporte();
      return bilan;
    },
  });
  return (
    <Section titre={t("import_notes.titre")}>
      <p className="text-sm text-muet">{t("import_notes.aide")}</p>
      <div className="flex flex-wrap gap-2">
        <Bouton variante="secondaire" onClick={() => exporter.mutate()} disabled={exporter.isPending}>{t("import_notes.exporter")}</Bouton>
        {importable ? <Bouton variante="secondaire" onClick={() => analyser.mutate()} disabled={analyser.isPending}>{t("import_notes.ouvrir")}</Bouton> : null}
      </div>
      {exporter.data ? <p role="status" className="text-sm text-vert">{t("import_notes.exportee", { chemin: exporter.data.chemin })}</p> : null}
      {importer.data ? <p role="status" className="text-sm text-vert">{t("import_notes.enregistrees", { nombre: String(importer.data.a_ecrire) })}</p> : null}
      <MessageErreur erreur={exporter.error ?? analyser.error ?? importer.error} />
      {apercu === null ? null : (
        <div className="flex flex-col gap-2">
          <p role="status" className="text-sm text-encre">{t("import_notes.resume", { a_ecrire: String(apercu.a_ecrire), erreurs: String(apercu.erreurs) })}</p>
          <table className="w-full max-w-3xl text-left text-sm">
            <thead>
              <tr className="text-muet">
                <th className="px-2 py-1 font-normal">{t("import_notes.ligne")}</th>
                <th className="px-2 py-1 font-normal">{t("impayes.eleve")}</th>
                <th className="px-2 py-1 font-normal">{t("import_notes.avant")}</th>
                <th className="px-2 py-1 font-normal">{t("import_notes.apres")}</th>
                <th className="px-2 py-1 font-normal">{t("import_notes.etat")}</th>
              </tr>
            </thead>
            <tbody>
              {apercu.lignes.map((l) => (
                <tr key={l.ligne} className="border-b border-muet/30">
                  <td className="px-2 py-1 text-muet">{l.ligne}</td>
                  <td className="px-2 py-1 text-encre">{l.eleve}</td>
                  <td className="px-2 py-1 text-muet">{l.avant}</td>
                  <td className="px-2 py-1 text-encre">{l.apres}</td>
                  <td className={`px-2 py-1 ${l.etat === "erreur" ? "text-rouge" : l.etat === "nouvelle" || l.etat === "modifiee" ? "text-vert" : "text-muet"}`}>
                    {l.erreur === null ? t(ETATS_IMPORT[l.etat]) : `${t(ETATS_IMPORT[l.etat])} : ${texteDeCle(l.erreur)}`}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          {apercu.inconnus.length === 0 ? null : <p className="text-sm text-ocre">{t("import_notes.inconnus", { matricules: apercu.inconnus.join(", ") })}</p>}
          {apercu.absents_du_fichier.length === 0 ? null : <p className="text-sm text-muet">{t("import_notes.absents_du_fichier", { eleves: apercu.absents_du_fichier.join(", ") })}</p>}
          <div className="flex flex-wrap gap-2">
            <Bouton onClick={() => importer.mutate()} disabled={importer.isPending || apercu.a_ecrire === 0}>{t("import_notes.enregistrer", { nombre: String(apercu.a_ecrire) })}</Bouton>
            <Bouton variante="secondaire" onClick={() => setApercu(null)}>{t("commun.annuler")}</Bouton>
          </div>
        </div>
      )}
    </Section>
  );
}
