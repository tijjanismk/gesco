import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { commandeAutorisee } from "../../app/navigation";
import { ApercuPdf } from "../../composants/ApercuPdf";
import { Bouton } from "../../composants/Bouton";
import { Champ, Selection } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { type Cle, date as dateDuJour, dateCivile, t } from "../../i18n";
import type { BulletinAffiche, BulletinsClasse, LigneReprise, StatutBulletin } from "../../types/gesco";
import { lirePoints, moyenne, points, texteRang } from "./saisies";

const STATUTS: Record<StatutBulletin, Cle> = {
  calcule: "bulletins.statut.calcule",
  valide: "bulletins.statut.valide",
  publie: "bulletins.statut.publie",
  remplace: "bulletins.statut.remplace",
};

/** Appréciations types, proposées au clavier ; le texte reste libre (T-31). */
const APPRECIATIONS = [
  "Excellent travail",
  "Très bon travail",
  "Bon travail",
  "Assez bon travail",
  "Travail passable",
  "Peut mieux faire",
  "Travail insuffisant",
  "Doit redoubler d'efforts",
];

/** Part du barème qui ouvre chaque appréciation type : 16, 14, 12, 11, 10, 8 et 6 sur 20 ; en
 *  dessous, la dernière. */
const SEUILS = [0.8, 0.7, 0.6, 0.55, 0.5, 0.4, 0.3];

function appreciationType(moyenne: number, bareme: number): string {
  const rang = SEUILS.findIndex((s) => moyenne >= s * bareme);
  return APPRECIATIONS[rang === -1 ? APPRECIATIONS.length - 1 : rang] ?? "";
}

function valeur(c: number | null): string {
  return c === null ? "—" : moyenne(c);
}

/** Bulletins (SPEC_PHASE_4 §7, UC-04) : calcul d'une classe pour une période, appréciations,
 *  validation, publication en un PDF, correction d'un bulletin publié. */
export function Bulletins() {
  const session = useQuery({ queryKey: ["session"], queryFn: () => appeler("session_courante", null) });
  const annees = useQuery({ queryKey: ["annees"], queryFn: () => appeler("annees_lister", null) });
  const anneeId = annees.data?.find((a) => a.est_courante)?.id ?? null;
  const classes = useQuery({ queryKey: ["classes", anneeId], queryFn: () => appeler("classes_lister", { annee_id: anneeId ?? "" }), enabled: anneeId !== null });
  const pedagogie = useQuery({ queryKey: ["pedagogie", anneeId], queryFn: () => appeler("pedagogie_lire", { id: anneeId ?? "" }), enabled: anneeId !== null });
  const [classeChoisie, setClasseChoisie] = useState<string | null>(null);
  const [periodeChoisie, setPeriodeChoisie] = useState<string | null>(null);
  const permissions = session.data?.permissions ?? [];
  const niveauxNotes = new Set((pedagogie.data?.niveaux ?? []).filter((n) => n.cycle !== "jardin").map((n) => n.id));
  const visibles = (classes.data ?? []).filter((c) => niveauxNotes.has(c.classe.niveau_id));
  const classe = visibles.find((c) => c.classe.id === classeChoisie) ?? visibles[0];
  const periodes = (pedagogie.data?.periodes ?? []).filter((p) => classe !== undefined && p.periode.niveau_id === classe.classe.niveau_id);
  const periode = periodes.find((p) => p.periode.id === periodeChoisie) ?? periodes[0];
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("bulletins.titre")}</h1>
      <MessageErreur erreur={classes.error ?? pedagogie.error} />
      {classe === undefined ? (
        <p className="text-sm text-muet">{t("bulletins.aucune_classe")}</p>
      ) : (
        <>
          <div className="flex flex-wrap items-end gap-3">
            <Selection id="bulletins-classe" libelle={t("enseignants.classe")} valeur={classe.classe.id} surChangement={setClasseChoisie} options={visibles.map((c) => ({ valeur: c.classe.id, libelle: `${c.classe.libelle} · ${c.niveau}` }))} />
            <Selection id="bulletins-periode" libelle={t("notes.periode")} valeur={periode?.periode.id ?? ""} surChangement={setPeriodeChoisie} options={periodes.map((p) => ({ valeur: p.periode.id, libelle: p.libelle }))} />
          </div>
          {periode === undefined ? null : <BulletinsDeLaClasse key={`${classe.classe.id}-${periode.periode.id}`} classeId={classe.classe.id} periodeId={periode.periode.id} permissions={permissions} />}
        </>
      )}
    </div>
  );
}

function BulletinsDeLaClasse({ classeId, periodeId, permissions }: { classeId: string; periodeId: string; permissions: readonly string[] }) {
  const client = useQueryClient();
  const demande = { classe_id: classeId, periode_id: periodeId };
  const cle = ["bulletins", classeId, periodeId];
  const lecture = useQuery({ queryKey: cle, queryFn: () => appeler("bulletins_lire", demande) });
  const [ouvert, setOuvert] = useState<string | null>(null);
  const [pdf, setPdf] = useState<string | null>(null);
  const [bilan, setBilan] = useState<string | null>(null);
  const rafraichir = () => void client.invalidateQueries({ queryKey: cle });
  const calcul = useMutation({
    mutationFn: () => appeler("resultats_calculer", demande),
    onSuccess: (donnees) => {
      client.setQueryData(cle, donnees);
      setBilan(t("bulletins.calcules", { nombre: String(donnees.bulletins.length) }));
    },
  });
  const validation = useMutation({
    mutationFn: () => appeler("bulletins_valider", demande),
    onSuccess: (n) => {
      setBilan(t("bulletins.valides", { nombre: String(n) }));
      rafraichir();
    },
  });
  const publication = useMutation({
    mutationFn: () => appeler("bulletins_publier", demande),
    onSuccess: (document) => {
      setPdf(document.pdf);
      setBilan(t("bulletins.publies"));
      rafraichir();
    },
  });
  const apercu = useMutation({ mutationFn: () => appeler("bulletins_produire", demande), onSuccess: (document) => setPdf(document.pdf) });
  // Remplit d'après la moyenne les appréciations encore vides des matières de l'utilisateur, pour
  // toute la classe ; ce qu'un enseignant a écrit reste, et l'appréciation générale aussi.
  const remplissage = useMutation({
    mutationFn: async () => {
      const donnees = lecture.data;
      let remplies = 0;
      for (const a of donnees?.bulletins ?? []) {
        if (donnees === undefined || a.bulletin.statut === "publie" || a.bulletin.statut === "remplace") continue;
        for (const l of a.bulletin.contenu.lignes) {
          if (l.moyenne === null || l.appreciation.trim() !== "" || !donnees.matieres_appreciables.includes(l.matiere_id)) continue;
          await appeler("appreciation_saisir", {
            inscription_id: a.bulletin.inscription_id,
            periode_id: a.bulletin.periode_id,
            matiere_id: l.matiere_id,
            texte: appreciationType(l.moyenne, donnees.bareme),
          });
          remplies += 1;
        }
      }
      return remplies;
    },
    onSuccess: (n) => {
      setBilan(t("bulletins.remplies", { nombre: String(n) }));
      rafraichir();
    },
    onError: rafraichir,
  });
  const d = lecture.data;
  if (d === undefined) return <MessageErreur erreur={lecture.error} />;
  const premier = d.bulletins[0]?.bulletin.contenu;
  const choisi = d.bulletins.find((b) => b.bulletin.inscription_id === ouvert);
  const erreur = calcul.error ?? validation.error ?? publication.error ?? apercu.error ?? remplissage.error;
  return (
    <>
      <Section titre={t("bulletins.classe", { classe: d.classe, periode: d.periode })}>
        <div className="flex flex-wrap gap-2">
          {commandeAutorisee("resultats_calculer", permissions) ? (
            <Bouton onClick={() => calcul.mutate()} disabled={calcul.isPending}>{t("bulletins.calculer")}</Bouton>
          ) : null}
          {d.matieres_appreciables.length > 0 ? (
            <Bouton variante="secondaire" title={t("bulletins.remplir_aide")} onClick={() => remplissage.mutate()} disabled={remplissage.isPending || d.bulletins.length === 0}>
              {t(remplissage.isPending ? "bulletins.remplissage" : "bulletins.remplir")}
            </Bouton>
          ) : null}
          {commandeAutorisee("bulletins_valider", permissions) ? (
            <Bouton variante="secondaire" onClick={() => validation.mutate()} disabled={validation.isPending || d.bulletins.length === 0}>{t("bulletins.valider")}</Bouton>
          ) : null}
          {commandeAutorisee("bulletins_publier", permissions) ? (
            <Bouton variante="secondaire" onClick={() => publication.mutate()} disabled={publication.isPending || d.bulletins.length === 0}>{t("bulletins.publier")}</Bouton>
          ) : null}
          {commandeAutorisee("bulletins_produire", permissions) ? (
            <Bouton variante="secondaire" onClick={() => apercu.mutate()} disabled={apercu.isPending || d.bulletins.length === 0}>{t("bulletins.apercu")}</Bouton>
          ) : null}
        </div>
        {d.evaluations_en_saisie > 0 ? <p className="text-sm text-ocre">{t("bulletins.evaluations_en_saisie", { nombre: String(d.evaluations_en_saisie) })}</p> : null}
        {bilan === null ? null : <p role="status" className="text-sm text-vert">{bilan}</p>}
        <MessageErreur erreur={erreur} />
        {d.bulletins.length === 0 ? (
          <p className="text-sm text-muet">{t("bulletins.aucun")}</p>
        ) : (
          <>
            {premier === undefined ? null : (
              <p className="text-sm text-encre">
                {t("bulletins.resume", {
                  ont_compose: String(premier.ont_compose),
                  effectif: String(premier.effectif),
                  premier: valeur(premier.premier),
                  dernier: valeur(premier.dernier),
                  classe: valeur(premier.moyenne_classe),
                })}
              </p>
            )}
            <table className="w-full max-w-4xl text-left text-sm">
              <thead>
                <tr className="text-muet">
                  <th className="px-2 py-1 font-normal">{t("bulletins.rang")}</th>
                  <th className="px-2 py-1 font-normal">{t("impayes.eleve")}</th>
                  <th className="px-2 py-1 font-normal">{t("bulletins.moyenne_sur", { bareme: moyenne(d.bareme).replace(",00", "") })}</th>
                  <th className="px-2 py-1 font-normal">{t("bulletins.mention")}</th>
                  <th className="px-2 py-1 font-normal">{t("bulletins.etat")}</th>
                  <th className="px-2 py-1 font-normal" />
                </tr>
              </thead>
              <tbody>
                {[...d.bulletins]
                  .sort((a, b) => (a.bulletin.contenu.rang?.rang ?? 9999) - (b.bulletin.contenu.rang?.rang ?? 9999))
                  .map((a) => {
                    const c = a.bulletin.contenu;
                    return (
                      <tr key={a.bulletin.id} className="border-b border-muet/30">
                        <td className="px-2 py-1 text-encre">{texteRang(c.rang)}</td>
                        <td className="px-2 py-1 text-encre">{c.eleve}</td>
                        <td className="px-2 py-1 text-encre">{valeur(c.moyenne)}</td>
                        <td className="px-2 py-1 text-encre">{c.mention ?? ""}</td>
                        <td className="px-2 py-1 text-muet">
                          {t(STATUTS[a.bulletin.statut])}
                          {a.bulletin.version > 1 ? ` · ${t("bulletins.version", { version: String(a.bulletin.version) })}` : ""}
                        </td>
                        <td className="px-2 py-1">
                          <button type="button" className="text-sm text-primaire underline" onClick={() => setOuvert(ouvert === a.bulletin.inscription_id ? null : a.bulletin.inscription_id)}>
                            {t(ouvert === a.bulletin.inscription_id ? "bulletins.fermer" : "bulletins.ouvrir")}
                          </button>
                        </td>
                      </tr>
                    );
                  })}
              </tbody>
            </table>
          </>
        )}
      </Section>
      {choisi === undefined ? null : <BulletinEleve key={choisi.bulletin.id} affiche={choisi} classe={d} permissions={permissions} surChange={rafraichir} />}
      {pdf === null ? null : <ApercuPdf base64={pdf} />}
      {commandeAutorisee("moyennes_reprises_lire", permissions) ? <MoyennesReprises classeId={classeId} periodeId={periodeId} /> : null}
    </>
  );
}

function BulletinEleve({ affiche, classe, permissions, surChange }: { affiche: BulletinAffiche; classe: BulletinsClasse; permissions: readonly string[]; surChange: () => void }) {
  const b = affiche.bulletin;
  const c = b.contenu;
  const fige = b.statut === "publie" || b.statut === "remplace";
  const [textes, setTextes] = useState<Record<string, string>>({});
  const [motif, setMotif] = useState("");
  const [pdf, setPdf] = useState<string | null>(null);
  const appreciation = useMutation({
    mutationFn: ({ matiere, texte }: { matiere: string | null; texte: string }) =>
      appeler("appreciation_saisir", { inscription_id: b.inscription_id, periode_id: b.periode_id, matiere_id: matiere, texte }),
    onSuccess: surChange,
  });
  const correction = useMutation({ mutationFn: () => appeler("bulletin_corriger", { id: b.id, motif }), onSuccess: surChange });
  const impression = useMutation({ mutationFn: (id: string) => appeler("bulletin_produire", { id }), onSuccess: (document) => setPdf(document.pdf) });
  const enregistrer = (matiere: string | null, enCours: string, texte: string | undefined) => {
    if (texte === undefined || texte === enCours) return;
    appreciation.mutate({ matiere, texte });
  };
  const premierCycle = c.cycle === "premier_cycle";
  return (
    <Section titre={t("bulletins.de", { eleve: c.eleve, matricule: c.matricule })}>
      {b.motif_correction === "" ? null : <p className="text-sm text-ocre">{t("bulletins.corrige", { version: String(b.version), motif: b.motif_correction })}</p>}
      <datalist id="appreciations-types">
        {APPRECIATIONS.map((a) => (
          <option key={a} value={a} />
        ))}
      </datalist>
      <table className="w-full text-left text-sm">
        <thead>
          <tr className="text-muet">
            <th className="px-2 py-1 font-normal">{t("programme.matiere")}</th>
            {premierCycle ? null : <th className="px-2 py-1 font-normal">{t("bulletins.note_de_classe")}</th>}
            {premierCycle ? null : <th className="px-2 py-1 font-normal">{t("bulletins.composition")}</th>}
            <th className="px-2 py-1 font-normal">{t("bulletins.moyenne")}</th>
            {premierCycle ? null : <th className="px-2 py-1 font-normal">{t("bulletins.coefficient")}</th>}
            {premierCycle ? null : <th className="px-2 py-1 font-normal">{t("bulletins.points")}</th>}
            <th className="px-2 py-1 font-normal">{t("bulletins.appreciation")}</th>
          </tr>
        </thead>
        <tbody>
          {c.lignes.map((l, i) => {
            const modifiable = !fige && classe.matieres_appreciables.includes(l.matiere_id);
            return (
              <tr key={l.matiere_id} className="border-b border-muet/30">
                <td className="px-2 py-1 text-encre">
                  {l.matiere}
                  {l.enseignant === "" ? null : <span className="block text-xs text-muet">{l.enseignant}</span>}
                </td>
                {premierCycle ? null : <td className="px-2 py-1 text-encre">{valeur(l.note_de_classe)}</td>}
                {premierCycle ? null : <td className="px-2 py-1 text-encre">{valeur(l.composition)}</td>}
                <td className="px-2 py-1 font-semibold text-encre">{valeur(l.moyenne)}</td>
                {premierCycle ? null : <td className="px-2 py-1 text-encre">{l.coefficient}</td>}
                {premierCycle ? null : <td className="px-2 py-1 text-encre">{l.points === null ? "—" : moyenne(l.points)}</td>}
                <td className="px-2 py-1">
                  {modifiable ? (
                    <input
                      id={`appreciation-${i + 1}`}
                      list="appreciations-types"
                      aria-label={t("bulletins.appreciation_de", { matiere: l.matiere })}
                      className="w-full rounded border border-muet bg-surface px-2 py-1 text-primaire"
                      value={textes[l.matiere_id] ?? l.appreciation}
                      onChange={(e) => setTextes((x) => ({ ...x, [l.matiere_id]: e.target.value }))}
                      onBlur={() => enregistrer(l.matiere_id, l.appreciation, textes[l.matiere_id])}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") enregistrer(l.matiere_id, l.appreciation, textes[l.matiere_id]);
                      }}
                    />
                  ) : (
                    <span className="text-encre">{l.appreciation}</span>
                  )}
                </td>
              </tr>
            );
          })}
          {/* La note de conduite qui compte dans la moyenne : une ligne comme une matière. */}
          {c.conduite === undefined || c.conduite === null || c.conduite.points === null ? null : (
            <tr className="border-b border-muet/30">
              <td className="px-2 py-1 text-encre">{t("bulletins.conduite")}</td>
              <td className="px-2 py-1" />
              <td className="px-2 py-1" />
              <td className="px-2 py-1 font-semibold text-encre">{moyenne(c.conduite.note)}</td>
              <td className="px-2 py-1 text-encre">{c.conduite.coefficient}</td>
              <td className="px-2 py-1 text-encre">{moyenne(c.conduite.points)}</td>
              <td className="px-2 py-1" />
            </tr>
          )}
        </tbody>
      </table>
      {c.conduite === undefined || c.conduite === null || c.conduite.points !== null ? null : (
        <p className="text-sm text-encre">{t("bulletins.conduite_a_part", { note: moyenne(c.conduite.note), bareme: points(c.bareme) })}</p>
      )}
      <p className="text-sm text-encre">
        {t("bulletins.total", { points: moyenne(c.total_points), coefficients: String(c.total_coefficients) })}{" "}
        <strong>{t("bulletins.generale", { moyenne: valeur(c.moyenne), rang: texteRang(c.rang), sur: String(c.ont_compose) })}</strong>
        {c.mention === null ? "" : ` · ${c.mention}`}
      </p>
      <p className="text-sm text-muet">{t("bulletins.absences", { demi_journees: String(c.absences.demi_journees), justifiees: String(c.absences.justifiees), retards: String(c.absences.retards) })}</p>
      {c.arrive_le === undefined || c.arrive_le === null ? null : <p className="text-sm text-ocre">{t("bulletins.arrive_le", { date: dateCivile(c.arrive_le) })}</p>}
      {c.moyenne_annuelle === null ? null : (
        <p className="text-sm text-encre">
          <strong>{t("bulletins.annuelle", { moyenne: valeur(c.moyenne_annuelle), rang: texteRang(c.rang_annuel ?? null) })}</strong>{" "}
          {t(c.moyenne_annuelle_imprimee === true ? "bulletins.annuelle_imprimee" : "bulletins.annuelle_enregistree")}
        </p>
      )}
      {!fige && classe.appreciation_generale ? (
        <Champ
          id="appreciation-generale"
          libelle={t("bulletins.appreciation_generale")}
          list="appreciations-types"
          value={textes["generale"] ?? c.appreciation_generale}
          onChange={(e) => setTextes((x) => ({ ...x, generale: e.target.value }))}
          onBlur={() => enregistrer(null, c.appreciation_generale, textes["generale"])}
        />
      ) : (
        <p className="text-sm text-encre">{`${t("bulletins.appreciation_generale")} : ${c.appreciation_generale}`}</p>
      )}
      <MessageErreur erreur={appreciation.error ?? correction.error ?? impression.error} />
      <div className="flex flex-wrap items-end gap-2">
        {commandeAutorisee("bulletin_produire", permissions) ? (
          <Bouton variante="secondaire" onClick={() => impression.mutate(b.id)} disabled={impression.isPending}>{t("bulletins.imprimer_un")}</Bouton>
        ) : null}
        {b.statut === "publie" && commandeAutorisee("bulletin_corriger", permissions) ? (
          <>
            <Champ id="correction-motif" libelle={t("bulletins.motif_correction")} value={motif} onChange={(e) => setMotif(e.target.value)} />
            <Bouton variante="secondaire" onClick={() => correction.mutate()} disabled={correction.isPending || motif.trim() === ""}>{t("bulletins.corriger")}</Bouton>
          </>
        ) : null}
      </div>
      {affiche.anterieures.length === 0 ? null : (
        <ul className="flex flex-col gap-1 text-sm">
          {affiche.anterieures.map((v) => (
            <li key={v.id}>
              <button type="button" className="text-primaire underline" onClick={() => impression.mutate(v.id)}>
                {t("bulletins.ancienne_version", { version: String(v.version), statut: t(STATUTS[v.statut]), date: v.publie_le === null ? "" : dateDuJour(v.publie_le) })}
              </button>
            </li>
          ))}
        </ul>
      )}
      {pdf === null ? null : <ApercuPdf base64={pdf} />}
    </Section>
  );
}

/** Moyennes reprises (Q-28) : pour un élève arrivé après la période, la moyenne de son relevé
 *  officiel d'une autre école, qui entre dans sa moyenne annuelle. */
function MoyennesReprises({ classeId, periodeId }: { classeId: string; periodeId: string }) {
  const lecture = useQuery({ queryKey: ["reprises", classeId, periodeId], queryFn: () => appeler("moyennes_reprises_lire", { classe_id: classeId, periode_id: periodeId }) });
  const d = lecture.data;
  if (d === undefined) return <MessageErreur erreur={lecture.error} />;
  if (d.lignes.length === 0) return null;
  return (
    <Section titre={t("reprises.titre")}>
      <p className="text-sm text-muet">{t("reprises.aide", { bareme: points(d.bareme) })}</p>
      {d.lignes.map((l, i) => (
        <LigneMoyenneReprise key={l.inscription_id} ligne={l} rang={i + 1} classeId={classeId} periodeId={periodeId} />
      ))}
    </Section>
  );
}

function LigneMoyenneReprise({ ligne, rang, classeId, periodeId }: { ligne: LigneReprise; rang: number; classeId: string; periodeId: string }) {
  const client = useQueryClient();
  const [texte, setTexte] = useState(ligne.reprise === null ? "" : points(ligne.reprise.moyenne));
  const [origine, setOrigine] = useState(ligne.reprise?.etablissement_origine ?? "");
  const [piece, setPiece] = useState(ligne.reprise?.piece ?? "");
  const saisie = useMutation({
    mutationFn: () => appeler("moyenne_reprise_saisir", { classe_id: classeId, inscription_id: ligne.inscription_id, periode_id: periodeId, moyenne: texte, etablissement_origine: origine, piece }),
    onSuccess: () => void client.invalidateQueries({ queryKey: ["reprises", classeId, periodeId] }),
  });
  const lisible = texte.trim() === "" || lirePoints(texte) !== null;
  return (
    <div className="flex flex-wrap items-end gap-3 border-b border-muet/30 py-2">
      <div className="w-56 text-sm text-encre">
        {ligne.eleve}
        <span className="block text-xs text-muet">{t("reprises.arrive", { date: dateCivile(ligne.inscrit_le) })}</span>
      </div>
      <div className="w-24">
        <Champ id={`reprise-moyenne-${rang}`} libelle={t("reprises.moyenne")} value={texte} onChange={(e) => setTexte(e.target.value)} />
      </div>
      <Champ id={`reprise-origine-${rang}`} libelle={t("reprises.origine")} value={origine} onChange={(e) => setOrigine(e.target.value)} />
      <Champ id={`reprise-piece-${rang}`} libelle={t("reprises.piece")} value={piece} onChange={(e) => setPiece(e.target.value)} />
      <Bouton variante="secondaire" onClick={() => saisie.mutate()} disabled={saisie.isPending || !lisible}>{t("commun.enregistrer")}</Bouton>
      {saisie.isSuccess ? <p role="status" className="text-sm text-vert">{t(saisie.data === null ? "reprises.retiree" : "reprises.enregistree")}</p> : null}
      <MessageErreur erreur={saisie.error} />
    </div>
  );
}
