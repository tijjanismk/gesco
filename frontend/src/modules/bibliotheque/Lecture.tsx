import { useQuery } from "@tanstack/react-query";
import { useEffect, useMemo, useRef, useState } from "react";

import { appeler } from "../../api";
import { Bouton } from "../../composants/Bouton";
import { MessageErreur } from "../../composants/Messages";
import { t } from "../../i18n";
import type { ExamenSujet, LectureContenu, NatureContenu, VueContenu } from "../../types/gesco";
import { imagesParCle, RenduLecon } from "./RenduLecon";
import { type PlaceAuCatalogue, RetirerAutour } from "./Retraits";

const CLE_TAILLE = "gesco.bibliotheque.taille";
const TAILLE_MIN = 80;
const TAILLE_MAX = 200;

/** Taille du texte retenue sur ce poste ; 100 % si le navigateur intégré ne garde rien. */
function tailleRetenue(): number {
  try {
    const n = Number(window.localStorage.getItem(CLE_TAILLE));
    return Number.isFinite(n) && n >= TAILLE_MIN && n <= TAILLE_MAX ? n : 100;
  } catch {
    return 100;
  }
}

function retenirTaille(n: number): void {
  try {
    window.localStorage.setItem(CLE_TAILLE, String(n));
  } catch {
    // Rien à faire : la taille vaut pour cette ouverture.
  }
}

/** Nature d'un contenu telle que le catalogue et la leçon la montrent. */
export function libelleNature(nature: NatureContenu, examen: ExamenSujet | null, session: number | null): string {
  switch (nature) {
    case "lecon":
      return t("bibliotheque.nature.lecon");
    case "sujet":
      return session === null
        ? t("bibliotheque.nature.sujet_sans_session", { examen: examen ?? "" })
        : t("bibliotheque.nature.sujet", { examen: examen ?? "", session });
    case "corrige":
      return t("bibliotheque.nature.corrige");
    case "document":
      return t("bibliotheque.nature.document");
  }
}

function libelleVue(vue: VueContenu): string {
  switch (vue) {
    case "visible":
      return t("bibliotheque.vue.visible");
    case "livree":
      return t("bibliotheque.vue.livree");
    case "brouillon":
      return t("bibliotheque.vue.brouillon");
  }
}

/** Mode classe : la version visible en plein écran, en grand, sans barre ni menu ; Échap pour
 *  sortir. Les notes pour le maître n'y paraissent pas. */
function ModeClasse({ contenu, surFermer }: { contenu: LectureContenu; surFermer: () => void }) {
  const cadre = useRef<HTMLDivElement>(null);
  const images = useMemo(() => imagesParCle(contenu.images), [contenu.images]);
  useEffect(() => {
    const element = cadre.current;
    element?.focus();
    const touche = (e: KeyboardEvent) => {
      if (e.key === "Escape") surFermer();
    };
    // Le plein écran du système, s'il est permis ; Échap en sort et ferme le mode classe.
    const changement = () => {
      if (document.fullscreenElement === null) surFermer();
    };
    window.addEventListener("keydown", touche);
    document.addEventListener("fullscreenchange", changement);
    element?.requestFullscreen?.().catch(() => undefined);
    return () => {
      window.removeEventListener("keydown", touche);
      document.removeEventListener("fullscreenchange", changement);
      if (document.fullscreenElement !== null) void document.exitFullscreen().catch(() => undefined);
    };
  }, [surFermer]);
  return (
    <div
      ref={cadre}
      tabIndex={-1}
      role="dialog"
      aria-modal="true"
      aria-label={contenu.titre}
      className="fixed inset-0 z-50 overflow-auto bg-surface px-8 py-6 focus:outline-none"
    >
      <button
        type="button"
        onClick={surFermer}
        className="fixed top-2 right-2 rounded border border-muet bg-surface px-2 py-1 text-sm text-encre focus-visible:outline-2 focus-visible:outline-primaire"
      >
        {t("bibliotheque.mode_classe.quitter")}
      </button>
      <div className="mx-auto max-w-5xl" style={{ fontSize: "170%" }}>
        <h1 className="mb-4 text-3xl font-semibold text-marine">{contenu.titre}</h1>
        <RenduLecon lecon={contenu.lecon} images={images} sansNotes />
      </div>
    </div>
  );
}

/** Un contenu ouvert : barre de la leçon (versions, contenu lié, taille du texte, mode classe)
 *  et le texte. */
export function Lecture({
  contenuId,
  surOuvrir,
  place,
  peutGerer,
}: {
  contenuId: string;
  surOuvrir: (id: string) => void;
  /** Place du contenu au catalogue, pour la direction qui retire. */
  place: PlaceAuCatalogue | null;
  peutGerer: boolean;
}) {
  const [vue, setVue] = useState<VueContenu | null>(null);
  const [taille, setTaille] = useState(tailleRetenue);
  const [classe, setClasse] = useState(false);
  const lecture = useQuery({
    queryKey: ["bibliotheque", "lire", contenuId, vue],
    queryFn: () => appeler("bibliotheque_lire", { contenu_id: contenuId, vue }),
  });
  const contenu = lecture.data;
  const images = useMemo(() => imagesParCle(contenu?.images ?? []), [contenu?.images]);
  const changerTaille = (pas: number) => {
    const n = Math.min(TAILLE_MAX, Math.max(TAILLE_MIN, taille + pas));
    setTaille(n);
    retenirTaille(n);
  };
  if (contenu === undefined) return <MessageErreur erreur={lecture.error} />;
  return (
    <article className="flex flex-col gap-3">
      <header className="flex flex-col gap-2 border-b border-muet pb-3">
        <p className="text-sm text-muet">{libelleNature(contenu.nature, contenu.examen, contenu.session)}</p>
        <h2 className="text-xl font-semibold text-marine">{contenu.titre}</h2>
        <div className="flex flex-wrap items-center gap-2">
          {contenu.vues.length > 1 ? (
            <div role="group" aria-label={t("bibliotheque.vue.libelle")} className="flex gap-1">
              {contenu.vues.map((v) => (
                <Bouton
                  key={v}
                  variante={v === contenu.vue ? "principal" : "secondaire"}
                  aria-pressed={v === contenu.vue}
                  onClick={() => setVue(v)}
                >
                  {libelleVue(v)}
                </Bouton>
              ))}
            </div>
          ) : null}
          {contenu.lie === null ? null : (
            <Bouton variante="secondaire" onClick={() => contenu.lie !== null && surOuvrir(contenu.lie.id)}>
              {contenu.lie.nature === "corrige" ? t("bibliotheque.voir_corrige") : t("bibliotheque.voir_sujet")}
            </Bouton>
          )}
          <Bouton variante="secondaire" onClick={() => changerTaille(-10)} disabled={taille <= TAILLE_MIN} aria-label={t("bibliotheque.texte.plus_petit")}>
            A−
          </Bouton>
          <Bouton variante="secondaire" onClick={() => changerTaille(10)} disabled={taille >= TAILLE_MAX} aria-label={t("bibliotheque.texte.plus_grand")}>
            A+
          </Bouton>
          <Bouton variante="secondaire" onClick={() => setClasse(true)}>
            {t("bibliotheque.mode_classe")}
          </Bouton>
          {peutGerer && place !== null ? <RetirerAutour place={place} /> : null}
        </div>
      </header>
      <MessageErreur erreur={lecture.error} />
      {contenu.presentation === "" ? null : <p className="text-encre">{contenu.presentation}</p>}
      <div style={{ fontSize: `${taille}%` }}>
        {contenu.lecon.blocs.length === 0 ? (
          <p className="text-muet">{t("bibliotheque.contenu_vide")}</p>
        ) : (
          <RenduLecon lecon={contenu.lecon} images={images} />
        )}
      </div>
      {classe ? <ModeClasse contenu={contenu} surFermer={() => setClasse(false)} /> : null}
    </article>
  );
}
