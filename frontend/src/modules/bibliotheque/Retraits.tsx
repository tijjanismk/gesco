import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { Bouton } from "../../composants/Bouton";
import { MessageErreur } from "../../composants/Messages";
import { date, t } from "../../i18n";
import type { ApercuRetrait, CatalogueBibliotheque, DemandeRetrait, RetraitAffiche } from "../../types/gesco";

/** Où un contenu paraît au catalogue : ce que la direction peut retirer autour de lui. */
export interface PlaceAuCatalogue {
  contenuCle: string;
  chapitreCle: string;
  chapitreId: string;
  chapitre: string;
  matiereCle: string;
  matiere: string;
  niveauCode: string;
  niveau: string;
  serieCode: string | null;
  serie: string | null;
}

/** Première place d'un contenu dans le catalogue. */
export function trouverPlace(catalogue: CatalogueBibliotheque, contenuId: string): PlaceAuCatalogue | null {
  for (const n of catalogue.niveaux)
    for (const s of n.series)
      for (const m of s.matieres)
        for (const p of m.parties)
          for (const c of p.chapitres)
            for (const x of c.contenus)
              if (x.id === contenuId)
                return {
                  contenuCle: x.cle,
                  chapitreCle: c.cle,
                  chapitreId: c.id,
                  chapitre: c.titre,
                  matiereCle: m.cle,
                  matiere: m.libelle,
                  niveauCode: n.code,
                  niveau: n.libelle,
                  serieCode: s.code,
                  serie: s.libelle,
                };
  return null;
}

/** « 31 leçons et 5 sujets ne seront plus montrés ; vous pourrez les remettre. » */
function phraseApercu(a: ApercuRetrait): string {
  const parties: string[] = [];
  const ajouter = (n: number, un: string, plusieurs: string) => {
    if (n > 0) parties.push(n === 1 ? un : plusieurs);
  };
  ajouter(a.lecons, t("bibliotheque.retrait.lecon", { n: a.lecons }), t("bibliotheque.retrait.lecons", { n: a.lecons }));
  ajouter(a.sujets, t("bibliotheque.retrait.sujet", { n: a.sujets }), t("bibliotheque.retrait.sujets", { n: a.sujets }));
  ajouter(a.corriges, t("bibliotheque.retrait.corrige", { n: a.corriges }), t("bibliotheque.retrait.corriges", { n: a.corriges }));
  ajouter(a.documents, t("bibliotheque.retrait.document", { n: a.documents }), t("bibliotheque.retrait.documents", { n: a.documents }));
  if (parties.length === 0) return t("bibliotheque.retrait.apercu_rien");
  const liste = parties.length === 1 ? (parties[0] ?? "") : `${parties.slice(0, -1).join(", ")} ${t("bibliotheque.retrait.et")} ${parties.at(-1) ?? ""}`;
  const total = a.lecons + a.sujets + a.corriges + a.documents;
  return total === 1 ? t("bibliotheque.retrait.apercu_un", { liste }) : t("bibliotheque.retrait.apercu", { liste });
}

/** Confirmation d'un retrait : ce qui sera caché, puis Retirer ou Annuler. */
export function ConfirmerRetrait({ demande, surFin }: { demande: DemandeRetrait; surFin: (retire: boolean) => void }) {
  const client = useQueryClient();
  const apercu = useQuery({
    queryKey: ["bibliotheque", "apercu-retrait", demande],
    queryFn: () => appeler("bibliotheque_retrait_apercu", demande),
  });
  const retrait = useMutation({
    mutationFn: () => appeler("bibliotheque_retirer", demande),
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: ["bibliotheque"] });
      surFin(true);
    },
  });
  return (
    <div role="alertdialog" aria-live="polite" className="flex flex-col gap-2 rounded border border-ocre bg-fond p-3 text-sm text-encre">
      <MessageErreur erreur={apercu.error} />
      {apercu.data === undefined ? null : <p>{phraseApercu(apercu.data)}</p>}
      <div className="flex gap-2">
        <Bouton variante="danger" onClick={() => retrait.mutate()} disabled={apercu.data === undefined || retrait.isPending}>
          {t("bibliotheque.retirer.confirmer")}
        </Bouton>
        <Bouton variante="secondaire" onClick={() => surFin(false)}>
          {t("bibliotheque.retirer.annuler")}
        </Bouton>
      </div>
      <MessageErreur erreur={retrait.error} />
    </div>
  );
}

/** Le bouton « Retirer… » d'un contenu ouvert : ce cours, son chapitre, sa matière, son niveau. */
export function RetirerAutour({ place }: { place: PlaceAuCatalogue }) {
  const [ouvert, setOuvert] = useState(false);
  const [demande, setDemande] = useState<DemandeRetrait | null>(null);
  const [fait, setFait] = useState(false);
  const choix: { libelle: string; demande: DemandeRetrait }[] = [
    { libelle: t("bibliotheque.retirer.contenu"), demande: { grain: "contenu", cible: place.contenuCle, niveau_code: null, serie_code: null } },
    {
      libelle: t("bibliotheque.retirer.chapitre", { titre: place.chapitre }),
      demande: { grain: "chapitre", cible: place.chapitreCle, niveau_code: null, serie_code: null },
    },
    {
      libelle:
        place.serie === null
          ? t("bibliotheque.retirer.matiere", { matiere: place.matiere, niveau: place.niveau })
          : t("bibliotheque.retirer.matiere_serie", { matiere: place.matiere, niveau: place.niveau, serie: place.serie }),
      demande: { grain: "matiere", cible: place.matiereCle, niveau_code: place.niveauCode, serie_code: place.serieCode },
    },
    {
      libelle: t("bibliotheque.retirer.niveau", { niveau: place.niveau }),
      demande: { grain: "niveau", cible: place.niveauCode, niveau_code: null, serie_code: null },
    },
  ];
  if (!ouvert)
    return fait ? (
      <p role="status" className="text-sm text-vert">
        {t("bibliotheque.retirer.fait")}
      </p>
    ) : (
      <Bouton variante="secondaire" onClick={() => setOuvert(true)}>
        {t("bibliotheque.retirer")}
      </Bouton>
    );
  return (
    <div className="flex w-full flex-col gap-2 rounded border border-muet p-3">
      <p className="text-sm font-medium text-encre">{t("bibliotheque.retirer.titre")}</p>
      <div role="group" className="flex flex-wrap gap-2">
        {choix.map((c) => (
          <Bouton
            key={c.demande.grain}
            variante={demande?.grain === c.demande.grain ? "principal" : "secondaire"}
            aria-pressed={demande?.grain === c.demande.grain}
            onClick={() => setDemande(c.demande)}
          >
            {c.libelle}
          </Bouton>
        ))}
        <Bouton
          variante="secondaire"
          onClick={() => {
            setDemande(null);
            setOuvert(false);
          }}
        >
          {t("bibliotheque.retirer.annuler")}
        </Bouton>
      </div>
      {demande === null ? null : (
        <ConfirmerRetrait
          key={demande.grain}
          demande={demande}
          surFin={(retire) => {
            setDemande(null);
            if (retire) {
              setOuvert(false);
              setFait(true);
            }
          }}
        />
      )}
    </div>
  );
}

function libelleRetrait(r: RetraitAffiche): string {
  const avecNiveau = (texte: string) => (r.niveau === null ? texte : `${texte} · ${r.niveau}${r.serie_code === null ? "" : ` (${r.serie_code})`}`);
  switch (r.grain) {
    case "contenu":
      return avecNiveau(t("bibliotheque.retires.contenu", { titre: r.titre }));
    case "chapitre":
      return avecNiveau(t("bibliotheque.retires.chapitre", { titre: r.titre }));
    case "matiere":
      return avecNiveau(t("bibliotheque.retires.matiere", { titre: r.titre }));
    case "niveau":
      return t("bibliotheque.retires.niveau", { titre: r.titre });
    case "paquet":
      return t("bibliotheque.retires.paquet");
  }
}

/** Liste « Retirés » : chaque retrait, qui, quand, ce qui le cache aussi, et Remettre. */
export function ListeRetraits() {
  const client = useQueryClient();
  const retraits = useQuery({
    queryKey: ["bibliotheque", "retraits"],
    queryFn: () => appeler("bibliotheque_retraits_lister", null),
  });
  const remise = useMutation({
    mutationFn: (retrait_id: string) => appeler("bibliotheque_remettre", { retrait_id }),
    onSuccess: (liste) => {
      client.setQueryData(["bibliotheque", "retraits"], liste);
      void client.invalidateQueries({ queryKey: ["bibliotheque"] });
    },
  });
  const liste = retraits.data ?? [];
  const parId = new Map(liste.map((r) => [r.id, r]));
  return (
    <section className="flex flex-col gap-3" aria-label={t("bibliotheque.retires")}>
      <MessageErreur erreur={retraits.error} />
      <MessageErreur erreur={remise.error} />
      {retraits.data === undefined ? null : liste.length === 0 ? (
        <p className="text-sm text-muet">{t("bibliotheque.retires.aucun")}</p>
      ) : (
        <ul className="flex flex-col gap-2">
          {liste.map((r) => {
            const aussi = r.aussi.flatMap((id) => {
              const autre = parId.get(id);
              return autre === undefined ? [] : [libelleRetrait(autre)];
            });
            return (
              <li key={r.id} className="flex flex-wrap items-center justify-between gap-2 rounded border border-muet p-3">
                <div className="flex flex-col gap-0.5 text-sm">
                  <span className="font-medium text-encre">{libelleRetrait(r)}</span>
                  <span className="text-muet">{t("bibliotheque.retires.par", { date: date(r.retire_le), nom: r.retire_par })}</span>
                  {aussi.length === 0 ? null : <span className="text-encre">{t("bibliotheque.retires.aussi", { liste: aussi.join(" ; ") })}</span>}
                </div>
                <Bouton variante="secondaire" onClick={() => remise.mutate(r.id)} disabled={remise.isPending}>
                  {t("bibliotheque.retires.remettre")}
                </Bouton>
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
