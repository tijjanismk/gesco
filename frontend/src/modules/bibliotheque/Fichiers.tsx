import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { choisirDocumentBibliotheque, choisirDossier, enregistrerPdfSous, enregistrerPieceSous } from "../../api/fichiers";
import { Bouton } from "../../composants/Bouton";
import { MessageErreur } from "../../composants/Messages";
import { t } from "../../i18n";
import type { BilanVideos, FichierDocument, FormeImpression, LectureContenu } from "../../types/gesco";
import type { PlaceAuCatalogue } from "./Retraits";

/** Nom proposé d'un PDF : « Classe — Matière — Titre.pdf », sans caractère interdit. */
export function nomDuPdf(parties: readonly string[]): string {
  const nom = parties
    .filter((p) => p.trim() !== "")
    .join(" — ")
    .replace(/[\\/:*?"<>|]/g, " ")
    .trim();
  return `${nom === "" ? "Leçon" : nom}.pdf`;
}

/** « Imprimer… » : la leçon, un sujet, un corrigé, son chapitre entier ou ses faits à vérifier,
 *  enregistrés en PDF où l'utilisateur le choisit. */
export function Imprimer({ contenu, place }: { contenu: LectureContenu; place: PlaceAuCatalogue | null }) {
  const [ouvert, setOuvert] = useState(false);
  const [forme, setForme] = useState<FormeImpression>("complete");
  const [brouillon, setBrouillon] = useState(false);
  const [fait, setFait] = useState(false);
  const aBrouillon = contenu.vues.includes("brouillon");
  const debut = place === null ? [] : [place.niveau, place.matiere];
  const imprimer = useMutation({
    mutationFn: async (quoi: "contenu" | "chapitre" | "faits") => {
      setFait(false);
      const titre =
        quoi === "chapitre" ? (place?.chapitre ?? contenu.titre) : quoi === "faits" ? `${t("bibliotheque.faits")} — ${contenu.titre}` : contenu.titre;
      const chemin = await enregistrerPdfSous(nomDuPdf([...debut, titre]), t("bibliotheque.imprimer.filtre"));
      if (chemin === null) return false;
      if (quoi === "chapitre" && place !== null) {
        await appeler("bibliotheque_chapitre_imprimer", { chapitre_id: place.chapitreId, forme, chemin });
      } else if (quoi === "faits") {
        await appeler("bibliotheque_faits_imprimer", { contenu_id: contenu.id, brouillon: false, forme: "complete", chemin });
      } else {
        await appeler("bibliotheque_imprimer", { contenu_id: contenu.id, brouillon: brouillon && aBrouillon, forme, chemin });
      }
      return true;
    },
    onSuccess: setFait,
  });
  if (!ouvert)
    return (
      <Bouton variante="secondaire" onClick={() => setOuvert(true)}>
        {t("bibliotheque.imprimer")}
      </Bouton>
    );
  return (
    <div role="group" aria-label={t("bibliotheque.imprimer")} className="flex w-full flex-col gap-2 rounded border border-muet/40 bg-fond p-3 text-sm text-encre">
      <fieldset className="flex flex-wrap gap-3">
        <legend className="sr-only">{t("bibliotheque.imprimer.forme")}</legend>
        {(["complete", "sans_corriges"] as const).map((f) => (
          <label key={f} className="flex items-center gap-1">
            <input type="radio" name="forme-impression" checked={forme === f} onChange={() => setForme(f)} />
            {f === "complete" ? t("bibliotheque.imprimer.complete") : t("bibliotheque.imprimer.sans_corriges")}
          </label>
        ))}
        {aBrouillon ? (
          <label className="flex items-center gap-1">
            <input type="checkbox" checked={brouillon} onChange={(e) => setBrouillon(e.target.checked)} />
            {t("bibliotheque.imprimer.brouillon")}
          </label>
        ) : null}
      </fieldset>
      <div className="flex flex-wrap gap-2">
        <Bouton onClick={() => imprimer.mutate("contenu")} disabled={imprimer.isPending}>
          {t("bibliotheque.imprimer.contenu")}
        </Bouton>
        {place === null ? null : (
          <Bouton variante="secondaire" onClick={() => imprimer.mutate("chapitre")} disabled={imprimer.isPending}>
            {t("bibliotheque.imprimer.chapitre")}
          </Bouton>
        )}
        {contenu.faits === null ? null : (
          <Bouton variante="secondaire" onClick={() => imprimer.mutate("faits")} disabled={imprimer.isPending}>
            {t("bibliotheque.imprimer.faits")}
          </Bouton>
        )}
        <Bouton variante="secondaire" onClick={() => setOuvert(false)}>
          {t("commun.fermer")}
        </Bouton>
      </div>
      {imprimer.isPending ? <p role="status">{t("bibliotheque.imprimer.en_cours")}</p> : null}
      {fait ? <p role="status">{t("bibliotheque.imprimer.fait")}</p> : null}
      <MessageErreur erreur={imprimer.error} />
    </div>
  );
}

function taille(octets: number): string {
  return octets >= 1024 * 1024 ? `${(octets / (1024 * 1024)).toFixed(1)} Mo` : `${Math.max(1, Math.round(octets / 1024))} Ko`;
}

/** Fichiers d'un document de l'école : les enregistrer ; pour qui l'écrit, en joindre et en
 *  retirer. */
export function FichiersDocument({ contenu }: { contenu: LectureContenu }) {
  const client = useQueryClient();
  const [fichiers, setFichiers] = useState<readonly FichierDocument[]>(contenu.fichiers);
  const rafraichir = (liste: FichierDocument[]) => {
    setFichiers(liste);
    void client.invalidateQueries({ queryKey: ["bibliotheque", "lire", contenu.id] });
  };
  const joindre = useMutation({
    mutationFn: async () => {
      const chemin = await choisirDocumentBibliotheque(t("bibliotheque.document.filtre"));
      if (chemin === null) return null;
      return appeler("bibliotheque_document_fichier_ajouter", { contenu_id: contenu.id, chemin });
    },
    onSuccess: (liste) => {
      if (liste !== null) rafraichir(liste);
    },
  });
  const retirer = useMutation({
    mutationFn: (lien_id: string) => appeler("bibliotheque_document_fichier_retirer", { contenu_id: contenu.id, lien_id }),
    onSuccess: rafraichir,
  });
  const enregistrer = useMutation({
    mutationFn: async (f: FichierDocument) => {
      const chemin = await enregistrerPieceSous(f.nom);
      if (chemin !== null) await appeler("bibliotheque_fichier_enregistrer_sous", { empreinte: f.empreinte, chemin });
    },
  });
  if (fichiers.length === 0 && !contenu.peut_modifier) return null;
  return (
    <section aria-label={t("bibliotheque.document.fichiers")} className="flex flex-col gap-2">
      <h3 className="font-semibold text-marine">{t("bibliotheque.document.fichiers")}</h3>
      <ul className="flex flex-col gap-1">
        {fichiers.map((f) => (
          <li key={f.lien_id} className="flex flex-wrap items-center gap-2">
            <span className="text-encre">
              {f.nom} <span className="text-muet">({taille(f.taille)})</span>
            </span>
            <Bouton variante="secondaire" onClick={() => enregistrer.mutate(f)} disabled={enregistrer.isPending}>
              {t("bibliotheque.document.enregistrer")}
            </Bouton>
            {contenu.peut_modifier ? (
              <Bouton variante="secondaire" onClick={() => retirer.mutate(f.lien_id)} disabled={retirer.isPending}>
                {t("bibliotheque.document.retirer")}
              </Bouton>
            ) : null}
          </li>
        ))}
      </ul>
      {contenu.peut_modifier ? (
        <div>
          <Bouton variante="secondaire" onClick={() => joindre.mutate()} disabled={joindre.isPending}>
            {t("bibliotheque.document.joindre")}
          </Bouton>
        </div>
      ) : null}
      <MessageErreur erreur={joindre.error ?? retirer.error ?? enregistrer.error} />
    </section>
  );
}

/** Copier les vidéos vers une clé, et les en reprendre. */
export function VideosCle({ reprendre = true }: { reprendre?: boolean }) {
  const [bilan, setBilan] = useState<BilanVideos | null>(null);
  const geste = useMutation({
    mutationFn: async (quoi: "copier" | "reprendre") => {
      const dossier = await choisirDossier();
      if (dossier === null) return null;
      return appeler(quoi === "copier" ? "bibliotheque_videos_copier" : "bibliotheque_videos_reprendre", { dossier });
    },
    onSuccess: (b) => {
      if (b !== null) setBilan(b);
    },
  });
  return (
    <div className="flex flex-col gap-2">
      <div className="flex flex-wrap gap-2">
        <Bouton variante="secondaire" onClick={() => geste.mutate("copier")} disabled={geste.isPending}>
          {t("bibliotheque.videos.copier")}
        </Bouton>
        {reprendre ? (
          <Bouton variante="secondaire" onClick={() => geste.mutate("reprendre")} disabled={geste.isPending}>
            {t("bibliotheque.videos.reprendre")}
          </Bouton>
        ) : null}
      </div>
      {bilan === null ? null : (
        <p role="status" className="text-sm text-encre">
          {t("bibliotheque.videos.bilan", { copiees: bilan.copiees, deja: bilan.deja_la })}
        </p>
      )}
      <MessageErreur erreur={geste.error} />
    </div>
  );
}
