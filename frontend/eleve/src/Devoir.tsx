import "katex/dist/katex.min.css";

import { useContext, useMemo, useState } from "react";

import { envoyer, Refus, SortieDemandee } from "./api";
import { Attente, Chargement, Retour } from "./communs";
import { Blocs, ContexteMedias, type Medias } from "./Lecon";
import { ContexteSortie, useLecture } from "./lecture";
import { PHOTOS_MAX, reduirePhoto, TAILLE_MAX, tailleDe } from "./photos";
import { aRendreLe, dateEtHeure, etatDevoir, noteSur, refusRendu, renduLe, T } from "./textes";
import type { DevoirOuvertEleve, RefusRendu, RenduFait } from "./types";

/** Clé du brouillon d'un devoir dans le navigateur du téléphone. */
function cleBrouillon(id: string): string {
  return `gesco-eleve-devoir-${id}`;
}

function brouillonGarde(id: string): string {
  try {
    return window.localStorage.getItem(cleBrouillon(id)) ?? "";
  } catch {
    return "";
  }
}

function garderBrouillon(id: string, texte: string) {
  try {
    if (texte === "") window.localStorage.removeItem(cleBrouillon(id));
    else window.localStorage.setItem(cleBrouillon(id), texte);
  } catch {
    // Le brouillon n'est pas gardé : rien de grave.
  }
}

/** Le devoir ouvert par l'élève : consignes, ce qu'il a rendu, sa note, et rendre. */
export function Devoir({ id, avecCode }: { id: string; avecCode: boolean }) {
  const [cle, setCle] = useState(0);
  const [confirme, setConfirme] = useState<RenduFait | null>(null);
  const lu = useLecture<DevoirOuvertEleve>(avecCode ? `devoir/${encodeURIComponent(id)}` : null, cle);
  if (!avecCode) {
    return (
      <>
        <Retour vers="#/devoirs" texte={T.mesDevoirs} />
        <p className="mt-4 text-lg">{T.entrerPourRendre}</p>
      </>
    );
  }
  if (lu.etat === "occupe") return <Attente />;
  if (lu.etat === "chargement") return <Chargement />;
  if (lu.etat === "introuvable") {
    return (
      <>
        <Retour vers="#/devoirs" texte={T.mesDevoirs} />
        <p className="mt-4 text-muet">{T.devoirIntrouvable}</p>
      </>
    );
  }
  const surRendu = (fait: RenduFait) => {
    setConfirme(fait);
    setCle((c) => c + 1);
  };
  return <DevoirOuvert key={lu.donnee.id} devoir={lu.donnee} confirme={confirme} surRendu={surRendu} />;
}

function DevoirOuvert({
  devoir,
  confirme,
  surRendu,
}: {
  devoir: DevoirOuvertEleve;
  confirme: RenduFait | null;
  surRendu: (fait: RenduFait) => void;
}) {
  const medias = useMemo<Medias>(
    () => ({
      images: new Map(devoir.images.map((i) => [i.cle, `data:${i.type_mime};base64,${i.donnees}`])),
      sons: new Map(),
      videos: new Map(),
    }),
    [devoir],
  );
  const papier = devoir.forme_rendu === "papier";
  return (
    <article>
      <Retour vers="#/devoirs" texte={T.mesDevoirs} />
      <p className="mt-2 text-muet">{devoir.matiere}</p>
      <h1 className="mb-1 text-2xl font-bold">{devoir.titre}</h1>
      <p className="font-bold">{aRendreLe(devoir.date_limite)}</p>
      <p className="mb-3 text-muet">{etatDevoir(devoir.etat, devoir.rendus.at(-1)?.rendu_le ?? null, devoir.rendus.at(-1)?.apres_limite ?? false)}</p>

      <ContexteMedias.Provider value={medias}>
        <div className="lecon text-lg">
          <Blocs blocs={devoir.lecon.blocs} />
        </div>
      </ContexteMedias.Provider>

      {devoir.lecon_liee !== null && (
        <p className="my-4">
          <a href={`#/lecon/${encodeURIComponent(devoir.lecon_liee.id)}`} className="cible inline-flex items-center font-bold text-primaire underline">
            {T.leconLiee} : {devoir.lecon_liee.titre}
          </a>
        </p>
      )}

      {confirme !== null && (
        <p role="status" className="my-4 rounded bg-surface px-4 py-3 text-lg font-bold text-vert">
          <span aria-hidden="true">✓&nbsp;</span>
          {renduLe(confirme.rendu_le)}
          {confirme.apres_limite ? ` ${T.apresLimite}.` : ""}
        </p>
      )}

      {devoir.correction !== null && (
        <section className="my-4 rounded border-2 border-vert bg-surface px-4 py-3">
          <h2 className="text-xl font-bold">{T.correction}</h2>
          <p className="text-2xl font-bold">{noteSur(devoir.correction.note, devoir.correction.bareme)}</p>
          {devoir.correction.commentaire !== "" && (
            <>
              <h3 className="mt-2 font-bold">{T.commentaireMaitre}</h3>
              <p className="whitespace-pre-wrap">{devoir.correction.commentaire}</p>
            </>
          )}
        </section>
      )}

      {devoir.rendus.length > 0 && <MesRendus devoir={devoir} />}

      {papier ? (
        <p className="mt-6 rounded bg-surface px-4 py-3 text-lg font-bold">{T.aRendrePapier}</p>
      ) : devoir.clos ? (
        <p className="mt-6 rounded bg-surface px-4 py-3 text-lg font-bold">{T.devoirClos}</p>
      ) : (
        <Rendre devoir={devoir} surRendu={surRendu} />
      )}
    </article>
  );
}

function MesRendus({ devoir }: { devoir: DevoirOuvertEleve }) {
  return (
    <section className="my-4">
      <h2 className="mb-2 text-xl font-bold">{T.mesRendus}</h2>
      <ol className="flex flex-col gap-3">
        {[...devoir.rendus].reverse().map((r) => (
          <li key={r.id} className="rounded bg-surface px-4 py-3 shadow">
            <p className="font-bold">
              {r.sur_papier ? T.surPapier : `Rendu le ${dateEtHeure(r.rendu_le)}`}
              {r.apres_limite && !r.sur_papier ? ` · ${T.apresLimite}` : ""}
            </p>
            {r.texte !== "" && <p className="mt-1 whitespace-pre-wrap">{r.texte}</p>}
            {r.photos.length > 0 && (
              <ul className="mt-2 grid grid-cols-2 gap-2">
                {r.photos.map((p) => (
                  <li key={p.ordre}>
                    {p.retiree ? (
                      <p className="text-muet">{T.photoRetiree}</p>
                    ) : (
                      <a href={`/eleve/api/rendu/${encodeURIComponent(r.id)}/photo/${p.ordre}`} target="_blank" rel="noopener">
                        <img
                          src={`/eleve/api/rendu/${encodeURIComponent(r.id)}/photo/${p.ordre}`}
                          alt={`Photo ${p.ordre}`}
                          loading="lazy"
                          className="w-full rounded border border-muet"
                        />
                      </a>
                    )}
                  </li>
                ))}
              </ul>
            )}
          </li>
        ))}
      </ol>
    </section>
  );
}

type Envoi = { etat: "pret" } | { etat: "envoi" } | { etat: "pas_parti" } | { etat: "refuse"; message: string };

/** Le champ de texte, les photos et « Rendre mon devoir ». Le texte se garde dans le téléphone
 *  tant qu'il n'est pas rendu ; si l'envoi ne passe pas, texte et photos restent et la page
 *  propose « Réessayer ». */
function Rendre({ devoir, surRendu }: { devoir: DevoirOuvertEleve; surRendu: (fait: RenduFait) => void }) {
  const sortie = useContext(ContexteSortie);
  const texteOk = devoir.forme_rendu === "texte" || devoir.forme_rendu === "texte_ou_photos";
  const photosOk = devoir.forme_rendu === "photos" || devoir.forme_rendu === "texte_ou_photos";
  const [texte, setTexte] = useState(() => brouillonGarde(devoir.id));
  const [photos, setPhotos] = useState<string[]>([]);
  const [preparation, setPreparation] = useState(false);
  const [envoi, setEnvoi] = useState<Envoi>({ etat: "pret" });
  const [messagePhoto, setMessagePhoto] = useState<string | null>(null);

  const ecrire = (valeur: string) => {
    setTexte(valeur);
    garderBrouillon(devoir.id, valeur);
  };

  const ajouter = async (fichiers: FileList | null) => {
    if (fichiers === null) return;
    setMessagePhoto(null);
    setPreparation(true);
    const nouvelles: string[] = [];
    for (const f of Array.from(fichiers)) {
      if (photos.length + nouvelles.length >= PHOTOS_MAX) {
        setMessagePhoto(refusRendu("trop_de_photos"));
        break;
      }
      try {
        const reduite = await reduirePhoto(f);
        if (tailleDe(reduite) > TAILLE_MAX) setMessagePhoto(refusRendu("photo_trop_grosse"));
        else nouvelles.push(reduite);
      } catch {
        setMessagePhoto(T.photoIllisible);
      }
    }
    setPhotos((p) => [...p, ...nouvelles].slice(0, PHOTOS_MAX));
    setPreparation(false);
  };

  const rendre = () => {
    setEnvoi({ etat: "envoi" });
    envoyer<RenduFait>("rendu", { devoir_id: devoir.id, texte: texteOk ? texte : "", photos: photosOk ? photos : [] })
      .then((fait) => {
        garderBrouillon(devoir.id, "");
        setTexte("");
        setPhotos([]);
        setEnvoi({ etat: "pret" });
        if (fait !== null) surRendu(fait);
      })
      .catch((e: unknown) => {
        if (e instanceof SortieDemandee) {
          sortie();
        } else if (e instanceof Refus && e.statut === 422) {
          const corps = e.corps as { refus?: RefusRendu } | null;
          setEnvoi({ etat: "refuse", message: corps?.refus === undefined ? T.pasParti : refusRendu(corps.refus) });
        } else {
          // Poste occupé, Wi-Fi perdu, panne : texte et photos restent, « Réessayer ».
          setEnvoi({ etat: "pas_parti" });
        }
      });
  };

  const vide = texte.trim() === "" && photos.length === 0;
  const dejaRendu = devoir.rendus.length > 0;
  return (
    <section className="mt-6">
      {texteOk && (
        <label className="block">
          <span className="mb-1 block text-lg font-bold">{T.taReponseDevoir}</span>
          <textarea
            value={texte}
            onChange={(e) => ecrire(e.target.value)}
            rows={6}
            className="w-full rounded border-2 border-muet bg-surface p-3 text-lg text-encre"
          />
          <span className="block text-sm text-muet">{T.brouillonGarde}</span>
        </label>
      )}
      {photosOk && (
        <div className="mt-4">
          {photos.length > 0 && (
            <ul className="mb-3 grid grid-cols-2 gap-2">
              {photos.map((p, i) => (
                <li key={`${i}-${p.length}`} className="flex flex-col gap-1">
                  <img src={p} alt={`Photo ${i + 1}`} className="w-full rounded border border-muet" />
                  <button
                    type="button"
                    onClick={() => setPhotos((liste) => liste.filter((_, j) => j !== i))}
                    className="cible rounded border-2 border-rouge px-3 font-bold text-rouge"
                  >
                    {T.retirer}
                  </button>
                </li>
              ))}
            </ul>
          )}
          {photos.length < PHOTOS_MAX && (
            <label className="cible flex cursor-pointer items-center justify-center rounded border-2 border-primaire bg-surface px-4 text-lg font-bold text-primaire">
              {preparation ? T.reduction : T.ajouterPhoto}
              <input
                type="file"
                accept="image/*"
                multiple
                className="sr-only"
                disabled={preparation}
                onChange={(e) => {
                  void ajouter(e.target.files);
                  e.target.value = "";
                }}
              />
            </label>
          )}
          {messagePhoto !== null && (
            <p role="alert" className="mt-2 font-bold text-rouge">
              {messagePhoto}
            </p>
          )}
        </div>
      )}
      {envoi.etat === "refuse" && (
        <p role="alert" className="mt-3 font-bold text-rouge">
          {envoi.message}
        </p>
      )}
      {envoi.etat === "pas_parti" && (
        <p role="alert" className="mt-3 font-bold text-rouge">
          {T.pasParti}
        </p>
      )}
      <button
        type="button"
        onClick={rendre}
        disabled={vide || preparation || envoi.etat === "envoi"}
        className="cible mt-4 w-full rounded bg-primaire px-4 py-3 text-lg font-bold text-surface disabled:opacity-60"
        style={{ minHeight: "44px" }}
      >
        {envoi.etat === "envoi" ? T.envoiEnCours : envoi.etat === "pas_parti" ? T.reessayer : dejaRendu ? T.rendreDeNouveau : T.rendreMonDevoir}
      </button>
    </section>
  );
}
