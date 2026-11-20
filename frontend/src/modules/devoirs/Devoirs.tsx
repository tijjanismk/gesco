import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useMemo, useRef, useState } from "react";

import { appeler } from "../../api";
import { choisirImageBibliotheque } from "../../api/fichiers";
import { ApercuPdf } from "../../composants/ApercuPdf";
import { Bouton } from "../../composants/Bouton";
import { Champ, Selection } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { type Cle, dateHeure, t } from "../../i18n";
import type { ClasseDevoir, DemandeDevoir, DevoirAffiche, DevoirResume, EtatDevoir, FormeRendu } from "../../types/gesco";
import { imagesParCle, RenduLecon } from "../bibliotheque/RenduLecon";
import { Copies } from "./Copies";

const CHAMP = "rounded border border-muet bg-surface px-2 py-1.5 text-primaire focus:outline-2 focus:outline-primaire";

/** Délai entre la dernière frappe et l'aperçu des consignes. */
const DELAI_APERCU = 500;

export const ETATS: Record<EtatDevoir, Cle> = {
  brouillon: "devoirs.etat.brouillon",
  donne: "devoirs.etat.donne",
  clos: "devoirs.etat.clos",
};

export const FORMES: Record<FormeRendu, Cle> = {
  texte_ou_photos: "devoirs.forme.texte_ou_photos",
  texte: "devoirs.forme.texte",
  photos: "devoirs.forme.photos",
  papier: "devoirs.forme.papier",
};

/** Barème en points : « 20 », « 12,5 ». */
export function points(centiemes: number): string {
  return (centiemes / 100).toLocaleString("fr-FR", { maximumFractionDigits: 2 });
}

/** Barème tapé (« 20 », « 12,5 ») en centièmes ; null s'il n'est pas un nombre. */
function lireBareme(texte: string): number | null {
  const n = Number(texte.replace(",", ".").trim());
  return Number.isFinite(n) && texte.trim() !== "" ? Math.round(n * 100) : null;
}

/** Instant en millisecondes vers la valeur d'un champ « date et heure » (heure du poste). */
function versChamp(millis: number): string {
  const d = new Date(millis);
  const deux = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${deux(d.getMonth() + 1)}-${deux(d.getDate())}T${deux(d.getHours())}:${deux(d.getMinutes())}`;
}

/** Date limite proposée : dans une semaine, en fin de journée. */
function dateLimiteParDefaut(): number {
  const d = new Date();
  d.setDate(d.getDate() + 7);
  d.setHours(23, 59, 0, 0);
  return d.getTime();
}

/** Un bouton de la barre des consignes : ce qu'il écrit autour de la sélection. */
interface Balise {
  cle: Cle;
  avant: string;
  apres: string;
  exemple: string;
}

const BALISES: readonly Balise[] = [
  { cle: "bibliotheque.editeur.gras", avant: "**", apres: "**", exemple: "texte" },
  { cle: "bibliotheque.editeur.liste_numerotee", avant: "\n1. ", apres: "", exemple: "" },
  { cle: "bibliotheque.editeur.formule", avant: "$", apres: "$", exemple: "x^2" },
  { cle: "bibliotheque.editeur.formule_seule", avant: "\n$$\n", apres: "\n$$\n", exemple: "\\frac{a}{b}" },
];

/** Consignes : texte au format des leçons (formules, images), avec un aperçu tel que l'élève le lira. */
function Consignes({ valeur, surChangement }: { valeur: string; surChangement: (v: string) => void }) {
  const zone = useRef<HTMLTextAreaElement>(null);
  const [differe, setDiffere] = useState(valeur);
  useEffect(() => {
    const minuterie = window.setTimeout(() => setDiffere(valeur), DELAI_APERCU);
    return () => window.clearTimeout(minuterie);
  }, [valeur]);
  const apercu = useQuery({
    queryKey: ["devoirs", "apercu", differe],
    queryFn: () => appeler("devoir_apercu", { texte: differe }),
  });
  const images = useMemo(() => imagesParCle(apercu.data?.images ?? []), [apercu.data?.images]);
  const inserer = (avant: string, apres: string, exemple: string) => {
    const el = zone.current;
    const debut = el?.selectionStart ?? valeur.length;
    const fin = el?.selectionEnd ?? valeur.length;
    const choisi = valeur.slice(debut, fin);
    const milieu = choisi === "" ? exemple : choisi;
    surChangement(valeur.slice(0, debut) + avant + milieu + apres + valeur.slice(fin));
  };
  const image = useMutation({
    mutationFn: async () => {
      const chemin = await choisirImageBibliotheque(t("bibliotheque.editeur.image_filtre"));
      return chemin === null ? null : appeler("bibliotheque_image_ajouter", { chemin });
    },
    onSuccess: (i) => {
      if (i !== null) inserer("\n![", `](${i.reference})\n`, i.nom);
    },
  });
  return (
    <div className="flex flex-col gap-2">
      <span className="text-sm text-encre">{t("devoirs.consignes")}</span>
      <div role="toolbar" aria-label={t("devoirs.barre")} className="flex flex-wrap gap-1">
        {BALISES.map((b) => (
          <Bouton key={b.cle} variante="secondaire" onClick={() => inserer(b.avant, b.apres, b.exemple)}>
            {t(b.cle)}
          </Bouton>
        ))}
        <Bouton variante="secondaire" onClick={() => image.mutate()} disabled={image.isPending}>
          {t("devoirs.image")}
        </Bouton>
      </div>
      <MessageErreur erreur={image.error} />
      <div className="grid gap-3 lg:grid-cols-2">
        <textarea
          ref={zone}
          aria-label={t("devoirs.consignes")}
          rows={12}
          className={`${CHAMP} font-mono`}
          value={valeur}
          onChange={(e) => surChangement(e.target.value)}
          spellCheck={false}
        />
        <div className="rounded border border-muet/40 p-3" aria-live="polite">
          <p className="mb-2 text-xs text-muet">{t("devoirs.apercu")}</p>
          {apercu.data === undefined ? null : <RenduLecon lecon={apercu.data.lecon} images={images} />}
        </div>
      </div>
    </div>
  );
}

/** Écrire ou récrire un devoir. */
export function FormulaireDevoir({
  classes,
  devoir,
  contenuId = null,
  titreInitial = "",
  surFini,
  surAnnuler,
}: {
  classes: readonly ClasseDevoir[];
  /** Le devoir récrit ; absent : un devoir nouveau. */
  devoir?: DevoirAffiche;
  /** Leçon liée d'un devoir nouveau (« Donner un devoir sur cette leçon »). */
  contenuId?: string | null;
  titreInitial?: string;
  surFini: (d: DevoirAffiche) => void;
  surAnnuler: () => void;
}) {
  const client = useQueryClient();
  const [classeId, setClasseId] = useState(devoir?.classe_id ?? classes[0]?.id ?? "");
  const matieres = classes.find((c) => c.id === classeId)?.matieres ?? [];
  const [matiereChoisie, setMatiere] = useState(devoir?.matiere_id ?? "");
  const matiereId = matieres.some((m) => m.id === matiereChoisie) ? matiereChoisie : (matieres[0]?.id ?? "");
  const [titre, setTitre] = useState(devoir?.titre ?? titreInitial);
  const [consignes, setConsignes] = useState(devoir?.consignes ?? "");
  const [limite, setLimite] = useState(versChamp(devoir?.date_limite ?? dateLimiteParDefaut()));
  const [bareme, setBareme] = useState(points(devoir?.bareme ?? 2000));
  const [forme, setForme] = useState<FormeRendu>(devoir?.forme_rendu ?? "texte_ou_photos");
  const lie = devoir === undefined ? contenuId : devoir.contenu_id;
  const fixe = devoir !== undefined && devoir.etat !== "brouillon";
  const enregistrer = useMutation({
    mutationFn: () => {
      const ecrit: DemandeDevoir = {
        classe_id: classeId,
        matiere_id: matiereId,
        titre,
        consignes,
        contenu_id: lie,
        date_limite: new Date(limite).getTime(),
        bareme: lireBareme(bareme) ?? 0,
        forme_rendu: forme,
      };
      return devoir === undefined
        ? appeler("devoir_creer", ecrit)
        : appeler("devoir_enregistrer", { id: devoir.id, lue_le: devoir.modifie_le, devoir: ecrit });
    },
    onSuccess: (d) => {
      void client.invalidateQueries({ queryKey: ["devoirs"] });
      surFini(d);
    },
  });
  if (classes.length === 0) return <p className="text-sm text-muet">{t("devoirs.aucune_classe")}</p>;
  const baremeLu = lireBareme(bareme);
  const baremeJuste = baremeLu !== null && baremeLu >= 1000 && baremeLu <= 10000;
  return (
    <form
      className="flex flex-col gap-3"
      onSubmit={(e) => {
        e.preventDefault();
        enregistrer.mutate();
      }}
    >
      <div className="flex flex-wrap items-end gap-3">
        {fixe ? (
          <p className="text-sm text-encre">
            {devoir.classe} · {devoir.matiere}
          </p>
        ) : (
          <>
            <Selection
              id="devoir-classe"
              libelle={t("devoirs.classe")}
              valeur={classeId}
              surChangement={setClasseId}
              options={classes.map((c) => ({ valeur: c.id, libelle: c.libelle }))}
            />
            <Selection
              id="devoir-matiere"
              libelle={t("devoirs.matiere")}
              valeur={matiereId}
              surChangement={setMatiere}
              options={matieres.map((m) => ({ valeur: m.id, libelle: m.libelle }))}
            />
          </>
        )}
      </div>
      <Champ id="devoir-titre" libelle={t("devoirs.titre_devoir")} value={titre} onChange={(e) => setTitre(e.target.value)} maxLength={200} required />
      {lie === null ? null : <p className="text-sm text-muet">{t("devoirs.lecon_liee", { titre: devoir?.lecon_liee ?? titreInitial })}</p>}
      <Consignes valeur={consignes} surChangement={setConsignes} />
      <div className="flex flex-wrap items-end gap-3">
        <Champ id="devoir-limite" type="datetime-local" libelle={t("devoirs.date_limite")} value={limite} onChange={(e) => setLimite(e.target.value)} required />
        <Champ
          id="devoir-bareme"
          libelle={t("devoirs.bareme")}
          value={bareme}
          onChange={(e) => setBareme(e.target.value)}
          inputMode="decimal"
          aide={baremeJuste ? undefined : t("devoirs.bareme_aide")}
        />
        <Selection
          id="devoir-forme"
          libelle={t("devoirs.forme")}
          valeur={forme}
          surChangement={(v) => setForme(v as FormeRendu)}
          options={(Object.keys(FORMES) as FormeRendu[]).map((f) => ({ valeur: f, libelle: t(FORMES[f]) }))}
        />
      </div>
      <MessageErreur erreur={enregistrer.error} />
      <div className="flex flex-wrap gap-2">
        <Bouton type="submit" disabled={enregistrer.isPending || !baremeJuste || matiereId === ""}>
          {devoir === undefined ? t("devoirs.creer") : t("devoirs.enregistrer")}
        </Bouton>
        <Bouton variante="secondaire" onClick={surAnnuler}>
          {t("commun.annuler")}
        </Bouton>
      </div>
    </form>
  );
}

/** « Donner aussi à une autre classe ». */
function DonnerAussi({ devoir, surFini }: { devoir: DevoirAffiche; surFini: (d: DevoirAffiche) => void }) {
  const client = useQueryClient();
  const [classe, setClasse] = useState(devoir.autres_classes[0]?.id ?? "");
  const copier = useMutation({
    mutationFn: () => appeler("devoir_copier", { id: devoir.id, classe_id: classe }),
    onSuccess: (d) => {
      void client.invalidateQueries({ queryKey: ["devoirs"] });
      surFini(d);
    },
  });
  if (devoir.autres_classes.length === 0) return null;
  return (
    <div className="flex flex-wrap items-end gap-2">
      <Selection
        id="devoir-autre-classe"
        libelle={t("devoirs.autre_classe")}
        valeur={classe}
        surChangement={setClasse}
        options={devoir.autres_classes.map((c) => ({ valeur: c.id, libelle: c.libelle }))}
      />
      <Bouton variante="secondaire" onClick={() => copier.mutate()} disabled={copier.isPending || classe === ""}>
        {t("devoirs.donner_aussi")}
      </Bouton>
      <MessageErreur erreur={copier.error} />
    </div>
  );
}

/** Un devoir ouvert : consignes, état, gestes du maître, feuille à imprimer. */
export function DevoirOuvert({ id, classes, surRetour, surOuvrir }: { id: string; classes: readonly ClasseDevoir[]; surRetour: () => void; surOuvrir: (id: string) => void }) {
  const client = useQueryClient();
  const lu = useQuery({ queryKey: ["devoirs", "devoir", id], queryFn: () => appeler("devoir_lire", { id }) });
  const [ecrire, setEcrire] = useState(false);
  const [feuille, setFeuille] = useState<string | null>(null);
  const relire = (d: DevoirAffiche) => {
    client.setQueryData(["devoirs", "devoir", d.id], d);
    void client.invalidateQueries({ queryKey: ["devoirs", "liste"] });
  };
  const geste = useMutation({
    mutationFn: (nom: "devoir_donner" | "devoir_clore" | "devoir_rouvrir") => appeler(nom, { id }),
    onSuccess: relire,
  });
  const imprimer = useMutation({
    mutationFn: () => appeler("devoir_feuille", { id }),
    onSuccess: (r) => setFeuille(r.pdf),
  });
  const images = useMemo(() => imagesParCle(lu.data?.images ?? []), [lu.data?.images]);
  const d = lu.data;
  if (d === undefined) return <MessageErreur erreur={lu.error} />;
  return (
    <article className="flex flex-col gap-3">
      <div>
        <Bouton variante="secondaire" onClick={surRetour}>
          {t("devoirs.retour")}
        </Bouton>
      </div>
      <header className="flex flex-col gap-1 border-b border-muet pb-3">
        <p className="text-sm text-muet">
          {d.classe} · {d.matiere}
        </p>
        <h2 className="text-xl font-semibold text-marine">{d.titre}</h2>
        <p className="text-sm text-encre">
          {t(ETATS[d.etat])} · {t("devoirs.a_rendre_le", { date: dateHeure(d.date_limite) })} · {t("devoirs.sur", { points: points(d.bareme) })} · {t(FORMES[d.forme_rendu])}
        </p>
        {d.lecon_liee === null ? null : <p className="text-sm text-muet">{t("devoirs.lecon_liee", { titre: d.lecon_liee })}</p>}
        {d.etat === "brouillon" ? <p className="text-sm text-muet">{t("devoirs.brouillon_aide")}</p> : null}
      </header>
      {d.peut_modifier ? (
        <div className="flex flex-wrap gap-2">
          {d.etat === "brouillon" ? (
            <Bouton onClick={() => geste.mutate("devoir_donner")} disabled={geste.isPending}>
              {t("devoirs.donner")}
            </Bouton>
          ) : null}
          {d.etat === "donne" ? (
            <Bouton variante="secondaire" onClick={() => geste.mutate("devoir_clore")} disabled={geste.isPending}>
              {t("devoirs.clore")}
            </Bouton>
          ) : null}
          {d.etat === "clos" ? (
            <Bouton variante="secondaire" onClick={() => geste.mutate("devoir_rouvrir")} disabled={geste.isPending}>
              {t("devoirs.rouvrir")}
            </Bouton>
          ) : null}
          <Bouton variante="secondaire" aria-pressed={ecrire} onClick={() => setEcrire((e) => !e)}>
            {t("devoirs.modifier")}
          </Bouton>
          <Bouton variante="secondaire" onClick={() => imprimer.mutate()} disabled={imprimer.isPending}>
            {t("devoirs.feuille")}
          </Bouton>
        </div>
      ) : (
        <div>
          <Bouton variante="secondaire" onClick={() => imprimer.mutate()} disabled={imprimer.isPending}>
            {t("devoirs.feuille")}
          </Bouton>
        </div>
      )}
      <MessageErreur erreur={geste.error ?? imprimer.error} />
      {d.peut_modifier ? <DonnerAussi devoir={d} surFini={(c) => surOuvrir(c.id)} /> : null}
      {ecrire ? (
        <Section titre={t("devoirs.modifier")}>
          <FormulaireDevoir
            classes={classes}
            devoir={d}
            surFini={(n) => {
              relire(n);
              setEcrire(false);
            }}
            surAnnuler={() => setEcrire(false)}
          />
        </Section>
      ) : (
        <Section titre={t("devoirs.consignes")}>
          {d.consignes.trim() === "" ? <p className="text-sm text-muet">{t("devoirs.sans_consignes")}</p> : <RenduLecon lecon={d.lecon} images={images} />}
        </Section>
      )}
      {feuille === null ? null : (
        <Section titre={t("devoirs.feuille")}>
          <ApercuPdf base64={feuille} />
        </Section>
      )}
      {d.etat === "brouillon" ? null : <Copies devoir={d} surDevoir={relire} />}
    </article>
  );
}

function LigneDevoir({ devoir, surOuvrir }: { devoir: DevoirResume; surOuvrir: () => void }) {
  return (
    <tr className="border-b border-muet/40">
      <td className="py-1 pr-2">
        <button type="button" className="text-left text-primaire underline" onClick={surOuvrir}>
          {devoir.titre}
        </button>
      </td>
      <td className="py-1 pr-2">{devoir.classe}</td>
      <td className="py-1 pr-2">{devoir.matiere}</td>
      <td className="py-1 pr-2">{dateHeure(devoir.date_limite)}</td>
      <td className="py-1 pr-2">{t(ETATS[devoir.etat])}</td>
      <td className="py-1 text-right">{devoir.rendus}</td>
    </tr>
  );
}

/** Devoirs à rendre (SPEC_PHASE_12_LOT_5 §3, §11) : la liste par classe et matière, « Donner un devoir ». */
export function Devoirs() {
  const [classeId, setClasseId] = useState("");
  const [matiereId, setMatiereId] = useState("");
  const [ouvert, setOuvert] = useState<string | null>(null);
  const [nouveau, setNouveau] = useState(false);
  const liste = useQuery({
    queryKey: ["devoirs", "liste", classeId, matiereId],
    queryFn: () => appeler("devoirs_lister", { classe_id: classeId === "" ? null : classeId, matiere_id: matiereId === "" ? null : matiereId }),
  });
  const donnees = liste.data;
  const matieres = useMemo(() => {
    const toutes = new Map<string, string>();
    for (const c of donnees?.classes ?? []) {
      if (classeId !== "" && c.id !== classeId) continue;
      for (const m of c.matieres) toutes.set(m.id, m.libelle);
    }
    return [...toutes].map(([id, libelle]) => ({ id, libelle })).sort((a, b) => a.libelle.localeCompare(b.libelle));
  }, [donnees?.classes, classeId]);
  if (ouvert !== null && donnees !== undefined) {
    return <DevoirOuvert id={ouvert} classes={donnees.classes} surRetour={() => setOuvert(null)} surOuvrir={setOuvert} />;
  }
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("devoirs.titre")}</h1>
      <MessageErreur erreur={liste.error} />
      {donnees === undefined ? null : !donnees.actif ? (
        <p className="text-sm text-muet">{t("devoirs.coupes")}</p>
      ) : (
        <>
          <div className="flex flex-wrap items-end gap-3">
            <Selection
              id="devoirs-classe"
              libelle={t("devoirs.classe")}
              valeur={classeId}
              surChangement={(v) => {
                setClasseId(v);
                setMatiereId("");
              }}
              options={[{ valeur: "", libelle: t("devoirs.toutes") }, ...donnees.classes.map((c) => ({ valeur: c.id, libelle: c.libelle }))]}
            />
            <Selection
              id="devoirs-matiere"
              libelle={t("devoirs.matiere")}
              valeur={matiereId}
              surChangement={setMatiereId}
              options={[{ valeur: "", libelle: t("devoirs.toutes") }, ...matieres.map((m) => ({ valeur: m.id, libelle: m.libelle }))]}
            />
            {donnees.peut_donner && donnees.classes.length > 0 ? (
              <Bouton onClick={() => setNouveau(true)} disabled={nouveau}>
                {t("devoirs.donner_un")}
              </Bouton>
            ) : null}
          </div>
          {nouveau ? (
            <Section titre={t("devoirs.donner_un")}>
              <FormulaireDevoir
                classes={classeId === "" ? donnees.classes : donnees.classes.filter((c) => c.id === classeId)}
                surFini={(d) => {
                  setNouveau(false);
                  setOuvert(d.id);
                }}
                surAnnuler={() => setNouveau(false)}
              />
            </Section>
          ) : null}
          {donnees.devoirs.length === 0 ? (
            <p className="text-sm text-muet">{t("devoirs.aucun")}</p>
          ) : (
            <div className="overflow-x-auto">
              <table className="w-full text-left text-sm text-encre">
                <caption className="sr-only">{t("devoirs.titre")}</caption>
                <thead>
                  <tr className="border-b border-muet">
                    <th scope="col" className="py-1 pr-2">{t("devoirs.titre_devoir")}</th>
                    <th scope="col" className="py-1 pr-2">{t("devoirs.classe")}</th>
                    <th scope="col" className="py-1 pr-2">{t("devoirs.matiere")}</th>
                    <th scope="col" className="py-1 pr-2">{t("devoirs.date_limite")}</th>
                    <th scope="col" className="py-1 pr-2">{t("devoirs.etat")}</th>
                    <th scope="col" className="py-1 text-right">{t("devoirs.rendus")}</th>
                  </tr>
                </thead>
                <tbody>
                  {donnees.devoirs.map((d) => (
                    <LigneDevoir key={d.id} devoir={d} surOuvrir={() => setOuvert(d.id)} />
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </>
      )}
    </div>
  );
}

/** « Donner un devoir sur cette leçon », dans la barre d'une leçon. */
export function DonnerSurLecon({ contenuId, titre }: { contenuId: string; titre: string }) {
  const [ouvert, setOuvert] = useState(false);
  const [donne, setDonne] = useState<DevoirAffiche | null>(null);
  const liste = useQuery({
    queryKey: ["devoirs", "liste", "", ""],
    queryFn: () => appeler("devoirs_lister", { classe_id: null, matiere_id: null }),
    enabled: ouvert,
  });
  return (
    <>
      <Bouton
        variante="secondaire"
        aria-expanded={ouvert}
        onClick={() => {
          setOuvert((o) => !o);
          setDonne(null);
        }}
      >
        {t("devoirs.sur_cette_lecon")}
      </Bouton>
      {ouvert ? (
        <div className="basis-full">
          <Section titre={t("devoirs.sur_cette_lecon")}>
            <MessageErreur erreur={liste.error} />
            {donne !== null ? (
              <p role="status" className="text-sm text-vert">
                {t("devoirs.ecrit_en_brouillon", { titre: donne.titre, classe: donne.classe })}
              </p>
            ) : liste.data === undefined ? null : !liste.data.actif ? (
              <p className="text-sm text-muet">{t("devoirs.coupes")}</p>
            ) : (
              <FormulaireDevoir classes={liste.data.classes} contenuId={contenuId} titreInitial={titre} surFini={setDonne} surAnnuler={() => setOuvert(false)} />
            )}
          </Section>
        </div>
      ) : null}
    </>
  );
}
