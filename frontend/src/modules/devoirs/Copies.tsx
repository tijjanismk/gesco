import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { ApercuPdf } from "../../composants/ApercuPdf";
import { Bouton } from "../../composants/Bouton";
import { Champ } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { type Cle, dateHeure, t } from "../../i18n";
import type { CopiesDevoir, DevoirAffiche, EtatCopie, LigneCopie, PhotoCopie, RenduAffiche } from "../../types/gesco";

const CHAMP = "rounded border border-muet bg-surface px-2 py-1.5 text-primaire focus:outline-2 focus:outline-primaire";

const ETATS_COPIE: Record<EtatCopie, Cle> = {
  rendu: "devoirs.copie.rendu",
  en_retard: "devoirs.copie.en_retard",
  sur_papier: "devoirs.copie.sur_papier",
  pas_rendu: "devoirs.copie.pas_rendu",
};

type Filtre = "tous" | "a_corriger" | "pas_rendu";

const FILTRES: readonly Filtre[] = ["tous", "a_corriger", "pas_rendu"];

const LIBELLES_FILTRES: Record<Filtre, Cle> = {
  tous: "devoirs.filtre.tous",
  a_corriger: "devoirs.filtre.a_corriger",
  pas_rendu: "devoirs.filtre.pas_rendu",
};

/** Points lisibles : « 13,5 ». */
function points(centiemes: number): string {
  return (centiemes / 100).toLocaleString("fr-FR", { maximumFractionDigits: 2 });
}

function garde(filtre: Filtre, l: LigneCopie): boolean {
  if (filtre === "a_corriger") return l.a_corriger;
  if (filtre === "pas_rendu") return l.etat === "pas_rendu";
  return true;
}

/** Une photo d'une copie : en petit, puis en grand avec agrandir, tourner, suivante. */
function PhotoGrande({ devoirId, photos, depart, surFermer }: { devoirId: string; photos: readonly PhotoCopie[]; depart: number; surFermer: () => void }) {
  const [index, setIndex] = useState(depart);
  const [tours, setTours] = useState(0);
  const [grand, setGrand] = useState(false);
  const photo = photos[index] ?? photos[0];
  const lue = useQuery({
    queryKey: ["devoirs", "photo", devoirId, photo?.rendu_id, photo?.ordre],
    queryFn: () => appeler("devoir_photo", { devoir_id: devoirId, rendu_id: photo?.rendu_id ?? "", ordre: photo?.ordre ?? 0 }),
    enabled: photo !== undefined && !photo.retiree,
  });
  if (photo === undefined) return null;
  const donnees = lue.data?.donnees ?? null;
  return (
    <figure className="flex flex-col gap-2 rounded border border-muet/40 p-2">
      <figcaption className="flex flex-wrap items-center gap-2 text-sm text-encre">
        <span>{t("devoirs.photo", { ordre: String(photo.ordre) })}</span>
        <Bouton variante="secondaire" aria-pressed={grand} onClick={() => setGrand((g) => !g)}>
          {grand ? t("devoirs.reduire") : t("devoirs.agrandir")}
        </Bouton>
        <Bouton variante="secondaire" onClick={() => setTours((n) => (n + 1) % 4)}>
          {t("devoirs.tourner")}
        </Bouton>
        {photos.length > 1 ? (
          <Bouton
            variante="secondaire"
            onClick={() => {
              setIndex((i) => (i + 1) % photos.length);
              setTours(0);
            }}
          >
            {t("devoirs.photo_suivante")}
          </Bouton>
        ) : null}
        <Bouton variante="secondaire" onClick={surFermer}>
          {t("commun.fermer")}
        </Bouton>
      </figcaption>
      <MessageErreur erreur={lue.error} />
      {photo.retiree || (lue.data !== undefined && donnees === null) ? (
        <p className="text-sm text-muet">{t("devoirs.photo_retiree")}</p>
      ) : donnees === null || lue.data === undefined ? null : (
        <div className={`overflow-auto ${grand ? "" : "max-h-screen"}`}>
          <img
            src={`data:${lue.data.type_mime};base64,${donnees}`}
            alt={t("devoirs.photo", { ordre: String(photo.ordre) })}
            className={grand ? "max-w-none" : "mx-auto max-h-screen max-w-full"}
            style={{ transform: `rotate(${tours * 90}deg)` }}
          />
        </div>
      )}
    </figure>
  );
}

/** Un rendu d'un élève : texte, photos. */
function Rendu({ devoirId, rendu, compte }: { devoirId: string; rendu: RenduAffiche; compte: boolean }) {
  const [ouverte, setOuverte] = useState<number | null>(null);
  return (
    <div className="flex flex-col gap-2">
      <p className="text-sm text-encre">
        {t("devoirs.rendu_numero", { numero: String(rendu.numero), date: dateHeure(rendu.rendu_le) })}
        {compte ? ` · ${t("devoirs.rendu_compte")}` : ""}
        {rendu.en_retard ? ` · ${t("devoirs.rendu_retard")}` : ""}
      </p>
      {rendu.sur_papier ? (
        <p className="text-sm text-muet">{t("devoirs.rendu_papier_texte")}</p>
      ) : rendu.texte.trim() === "" ? (
        rendu.photos.length === 0 ? <p className="text-sm text-muet">{t("devoirs.sans_texte")}</p> : null
      ) : (
        <p className="whitespace-pre-wrap rounded border border-muet/40 p-3 text-encre">{rendu.texte}</p>
      )}
      {!compte && rendu.note !== null ? <p className="text-sm text-muet">{t("devoirs.ancienne_note", { note: points(rendu.note) })}</p> : null}
      {rendu.photos.length === 0 ? null : ouverte === null ? (
        <ul className="flex flex-wrap gap-2">
          {rendu.photos.map((p, i) => (
            <li key={p.ordre}>
              <Bouton variante="secondaire" onClick={() => setOuverte(i)}>
                {p.retiree ? t("devoirs.photo_retiree") : t("devoirs.photo", { ordre: String(p.ordre) })}
              </Bouton>
            </li>
          ))}
        </ul>
      ) : (
        <PhotoGrande devoirId={devoirId} photos={rendu.photos} depart={ouverte} surFermer={() => setOuverte(null)} />
      )}
    </div>
  );
}

/** La copie d'un élève : historique des rendus, note sur le barème, commentaire, « Élève suivant ». */
function CopieOuverte({
  devoirId,
  ligne,
  copies,
  surSuivant,
  surFermer,
}: {
  devoirId: string;
  ligne: LigneCopie;
  copies: CopiesDevoir;
  surSuivant: (() => void) | null;
  surFermer: () => void;
}) {
  const client = useQueryClient();
  const lue = useQuery({
    queryKey: ["devoirs", "copie", devoirId, ligne.eleve_id],
    queryFn: () => appeler("devoir_copie", { devoir_id: devoirId, eleve_id: ligne.eleve_id }),
  });
  const [note, setNote] = useState(ligne.note === null ? "" : points(ligne.note));
  const [commentaire, setCommentaire] = useState(ligne.commentaire);
  const noter = useMutation({
    mutationFn: () => appeler("devoir_noter", { devoir_id: devoirId, eleve_id: ligne.eleve_id, note, commentaire }),
    onSuccess: (c) => {
      client.setQueryData(["devoirs", "copies", devoirId], c);
      void client.invalidateQueries({ queryKey: ["devoirs", "copie", devoirId, ligne.eleve_id] });
    },
  });
  const rendus = lue.data?.rendus ?? [];
  const dernier = rendus.at(-1);
  return (
    <Section titre={ligne.nom}>
      <div className="flex flex-wrap gap-2">
        <Bouton variante="secondaire" onClick={surFermer}>
          {t("devoirs.fermer_copie")}
        </Bouton>
        {surSuivant === null ? null : (
          <Bouton variante="secondaire" onClick={surSuivant}>
            {t("devoirs.eleve_suivant")}
          </Bouton>
        )}
      </div>
      <MessageErreur erreur={lue.error} />
      <div className="flex flex-col gap-4">
        {[...rendus].reverse().map((r) => (
          <Rendu key={r.id} devoirId={devoirId} rendu={r} compte={r.id === dernier?.id} />
        ))}
      </div>
      {copies.peut_corriger && dernier !== undefined ? (
        <form
          className="mt-3 flex flex-col gap-2"
          onSubmit={(e) => {
            e.preventDefault();
            noter.mutate();
          }}
        >
          <Champ
            id="copie-note"
            libelle={t("devoirs.note_sur", { points: points(copies.bareme) })}
            value={note}
            onChange={(e) => {
              setNote(e.target.value);
              noter.reset();
            }}
            inputMode="decimal"
            className="w-28"
          />
          <label className="flex flex-col gap-1 text-sm text-encre">
            <span>{t("devoirs.commentaire")}</span>
            <textarea
              rows={3}
              maxLength={2000}
              className={CHAMP}
              value={commentaire}
              onChange={(e) => {
                setCommentaire(e.target.value);
                noter.reset();
              }}
            />
          </label>
          <MessageErreur erreur={noter.error} />
          {noter.isSuccess ? (
            <p role="status" className="text-sm text-vert">
              {t("devoirs.note_enregistree")}
            </p>
          ) : null}
          <div className="flex flex-wrap gap-2">
            <Bouton type="submit" disabled={noter.isPending}>
              {t("devoirs.enregistrer_note")}
            </Bouton>
          </div>
        </form>
      ) : null}
    </Section>
  );
}

/** Les copies d'un devoir donné : élèves de la classe, état, note, commentaire ; corriger, rendre les copies, imprimer. */
export function Copies({ devoir, surDevoir }: { devoir: DevoirAffiche; surDevoir: (d: DevoirAffiche) => void }) {
  const client = useQueryClient();
  const cle = ["devoirs", "copies", devoir.id];
  const lues = useQuery({ queryKey: cle, queryFn: () => appeler("devoir_copies", { id: devoir.id }) });
  const [filtre, setFiltre] = useState<Filtre>("tous");
  const [ouverte, setOuverte] = useState<string | null>(null);
  const [liste, setListe] = useState<string | null>(null);
  const papier = useMutation({
    mutationFn: (eleveId: string) => appeler("devoir_rendu_papier", { devoir_id: devoir.id, eleve_id: eleveId }),
    onSuccess: (c, eleveId) => {
      client.setQueryData(cle, c);
      setOuverte(eleveId);
    },
  });
  const rendre = useMutation({
    mutationFn: () => appeler("devoir_copies_rendre", { id: devoir.id }),
    onSuccess: (d) => {
      surDevoir(d);
      void client.invalidateQueries({ queryKey: cle });
    },
  });
  const imprimer = useMutation({
    mutationFn: () => appeler("devoir_notes_imprimer", { id: devoir.id }),
    onSuccess: (r) => setListe(r.pdf),
  });
  const c = lues.data;
  if (c === undefined) return <MessageErreur erreur={lues.error} />;
  const visibles = c.lignes.filter((l) => garde(filtre, l));
  const ligneOuverte = c.lignes.find((l) => l.eleve_id === ouverte);
  // « Élève suivant » : le prochain de la liste montrée qui a rendu quelque chose.
  const suivant = (() => {
    if (ligneOuverte === undefined) return null;
    const i = visibles.findIndex((l) => l.eleve_id === ligneOuverte.eleve_id);
    const apres = [...visibles.slice(i + 1), ...visibles.slice(0, Math.max(i, 0))];
    return apres.find((l) => l.etat !== "pas_rendu" && l.eleve_id !== ligneOuverte.eleve_id) ?? null;
  })();
  return (
    <Section titre={t("devoirs.copies")}>
      <div className="flex flex-wrap items-center gap-2">
        {c.peut_corriger && c.copies_rendues_le === null ? (
          <Bouton onClick={() => rendre.mutate()} disabled={rendre.isPending} title={t("devoirs.rendre_copies_aide")}>
            {t("devoirs.rendre_copies")}
          </Bouton>
        ) : null}
        <Bouton variante="secondaire" onClick={() => imprimer.mutate()} disabled={imprimer.isPending}>
          {t("devoirs.imprimer_notes")}
        </Bouton>
      </div>
      {c.copies_rendues_le === null ? (
        c.peut_corriger ? <p className="text-sm text-muet">{t("devoirs.rendre_copies_aide")}</p> : null
      ) : (
        <p className="text-sm text-encre">{t("devoirs.copies_rendues", { date: dateHeure(c.copies_rendues_le) })}</p>
      )}
      <MessageErreur erreur={rendre.error ?? imprimer.error ?? papier.error} />
      {ligneOuverte === undefined ? null : (
        <CopieOuverte
          key={ligneOuverte.eleve_id}
          devoirId={devoir.id}
          ligne={ligneOuverte}
          copies={c}
          surSuivant={suivant === null ? null : () => setOuverte(suivant.eleve_id)}
          surFermer={() => setOuverte(null)}
        />
      )}
      <div role="group" aria-label={t("devoirs.filtre")} className="flex flex-wrap gap-1">
        {FILTRES.map((f) => (
          <Bouton key={f} variante="secondaire" aria-pressed={filtre === f} onClick={() => setFiltre(f)}>
            {t(LIBELLES_FILTRES[f])}
          </Bouton>
        ))}
      </div>
      {visibles.length === 0 ? (
        <p className="text-sm text-muet">{t("devoirs.aucun_eleve")}</p>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full text-left text-sm text-encre">
            <caption className="sr-only">{t("devoirs.copies")}</caption>
            <thead>
              <tr className="border-b border-muet">
                <th scope="col" className="py-1 pr-2">{t("devoirs.eleve")}</th>
                <th scope="col" className="py-1 pr-2">{t("devoirs.etat_copie")}</th>
                <th scope="col" className="py-1 pr-2 text-right">{t("devoirs.note")}</th>
                <th scope="col" className="py-1 pr-2">{t("devoirs.commentaire")}</th>
                <th scope="col" className="py-1"><span className="sr-only">{t("devoirs.ouvrir_copie")}</span></th>
              </tr>
            </thead>
            <tbody>
              {visibles.map((l) => (
                <tr key={l.eleve_id} className={`border-b border-muet/40 ${l.eleve_id === ouverte ? "bg-primaire/5" : ""}`}>
                  <td className="py-1 pr-2">{l.nom}</td>
                  <td className="py-1 pr-2">
                    {t(ETATS_COPIE[l.etat])}
                    {l.rendu_le === null || l.etat === "sur_papier" ? "" : ` · ${dateHeure(l.rendu_le)}`}
                    {l.a_corriger ? ` · ${t("devoirs.a_corriger")}` : ""}
                  </td>
                  <td className="py-1 pr-2 text-right">{l.note === null ? "" : points(l.note)}</td>
                  <td className="max-w-xs truncate py-1 pr-2" title={l.commentaire}>
                    {l.commentaire}
                  </td>
                  <td className="py-1">
                    <div className="flex flex-wrap justify-end gap-1">
                      {l.etat === "pas_rendu" ? null : (
                        <Bouton variante="secondaire" onClick={() => setOuverte(l.eleve_id)}>
                          {t("devoirs.ouvrir_copie")}
                        </Bouton>
                      )}
                      {c.peut_corriger && l.etat !== "sur_papier" ? (
                        <Bouton variante="secondaire" onClick={() => papier.mutate(l.eleve_id)} disabled={papier.isPending}>
                          {t("devoirs.rendu_papier")}
                        </Bouton>
                      ) : null}
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      {liste === null ? null : (
        <Section titre={t("devoirs.liste_notes")}>
          <ApercuPdf base64={liste} />
        </Section>
      )}
    </Section>
  );
}
