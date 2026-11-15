import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useMemo, useRef, useState } from "react";

import { appeler, ErreurApi } from "../../api";
import { Bouton } from "../../composants/Bouton";
import { MessageEnregistre, MessageErreur } from "../../composants/Messages";
import { t } from "../../i18n";
import type { BrouillonOuvert, DepartBrouillon, EcritureOuverte } from "../../types/gesco";
import { imagesParCle, RenduLecon } from "./RenduLecon";

/** Délai entre la dernière frappe et l'aperçu. */
const DELAI_APERCU = 500;

/** Un bouton de la barre : ce qu'il écrit autour de la sélection, ou en tête de ligne. */
interface Balise {
  libelle: string;
  avant: string;
  apres: string;
  /** Texte mis à la place d'une sélection vide. */
  exemple: string;
  /** Le balisage commence une ligne. */
  ligne?: boolean;
}

function balises(): Balise[] {
  return [
    { libelle: t("bibliotheque.editeur.titre"), avant: "## ", apres: "", exemple: t("bibliotheque.editeur.exemple_titre"), ligne: true },
    { libelle: t("bibliotheque.editeur.gras"), avant: "**", apres: "**", exemple: t("bibliotheque.editeur.exemple_texte") },
    { libelle: t("bibliotheque.editeur.italique"), avant: "*", apres: "*", exemple: t("bibliotheque.editeur.exemple_texte") },
    { libelle: t("bibliotheque.editeur.liste"), avant: "- ", apres: "", exemple: t("bibliotheque.editeur.exemple_element"), ligne: true },
    { libelle: t("bibliotheque.editeur.liste_numerotee"), avant: "1. ", apres: "", exemple: t("bibliotheque.editeur.exemple_element"), ligne: true },
    { libelle: t("bibliotheque.editeur.tableau"), avant: t("bibliotheque.editeur.modele_tableau"), apres: "", exemple: "", ligne: true },
    { libelle: t("bibliotheque.editeur.formule"), avant: "$", apres: "$", exemple: "x^2" },
    { libelle: t("bibliotheque.editeur.formule_seule"), avant: "$$\n", apres: "\n$$\n", exemple: "\\frac{a}{b}", ligne: true },
    { libelle: t("bibliotheque.editeur.note"), avant: t("bibliotheque.editeur.modele_note"), apres: "", exemple: "", ligne: true },
  ];
}

/** Écrit une balise autour de la sélection de la zone de texte ; rend le texte et la sélection. */
function baliser(texte: string, debut: number, fin: number, b: Balise): { texte: string; debut: number; fin: number } {
  const choisi = texte.slice(debut, fin) || b.exemple;
  const avantLigne = b.ligne === true && debut > 0 && texte[debut - 1] !== "\n" ? "\n" : "";
  const insere = `${avantLigne}${b.avant}${choisi}${b.apres}`;
  const nouveau = texte.slice(0, debut) + insere + texte.slice(fin);
  const depart = debut + avantLigne.length + b.avant.length;
  return { texte: nouveau, debut: depart, fin: depart + choisi.length };
}

/** Le texte tapé, rendu comme la lecture, avec les notes de l'auteur dessous. */
function Apercu({ contenuId, texte }: { contenuId: string; texte: string }) {
  const [vu, setVu] = useState(texte);
  useEffect(() => {
    const minuterie = window.setTimeout(() => setVu(texte), DELAI_APERCU);
    return () => window.clearTimeout(minuterie);
  }, [texte]);
  const apercu = useQuery({
    queryKey: ["bibliotheque", "apercu", contenuId, vu],
    queryFn: () => appeler("bibliotheque_apercu", { contenu_id: contenuId, texte: vu }),
    placeholderData: (avant) => avant,
  });
  const donnees = apercu.data;
  const images = useMemo(() => imagesParCle(donnees?.images ?? []), [donnees?.images]);
  return (
    <div className="flex flex-col gap-3">
      <MessageErreur erreur={apercu.error} />
      {donnees === undefined ? null : (
        <>
          <div className="min-h-64 rounded border border-muet/40 p-3">
            <RenduLecon lecon={donnees.lecon} images={images} />
          </div>
          {donnees.formules_brutes.length === 0 && donnees.images_a_ajouter.length === 0 ? null : (
            <div className="flex flex-col gap-1 rounded border border-muet/40 bg-fond p-3 text-sm text-encre">
              <p className="font-semibold">{t("bibliotheque.editeur.notes_auteur")}</p>
              {donnees.formules_brutes.map((f) => (
                <p key={`f-${f}`}>
                  {t("bibliotheque.editeur.formule_brute")} <code className="font-condensee">{f}</code>
                </p>
              ))}
              {donnees.images_a_ajouter.map((i, n) => (
                <p key={`i-${n}`}>{t("bibliotheque.editeur.image_a_ajouter", { description: i })}</p>
              ))}
            </div>
          )}
        </>
      )}
    </div>
  );
}

/** Le brouillon de l'utilisateur, ouvert : texte à gauche, aperçu à droite. */
function EditeurBrouillon({
  contenuId,
  brouillon,
  surAutreVersion,
}: {
  contenuId: string;
  brouillon: BrouillonOuvert;
  surAutreVersion: () => void;
}) {
  const client = useQueryClient();
  const zone = useRef<HTMLTextAreaElement>(null);
  const [texte, setTexte] = useState(brouillon.texte);
  const [revision, setRevision] = useState(brouillon.revision);
  const [enregistre, setEnregistre] = useState(brouillon.texte);
  const [enregistreLe, setEnregistreLe] = useState<number | null>(null);
  const [abandon, setAbandon] = useState(false);
  const modifie = texte !== enregistre;
  const enregistrer = useMutation({
    mutationFn: (envoye: string) =>
      appeler("bibliotheque_brouillon_enregistrer", {
        contenu_id: contenuId,
        version_id: brouillon.version_id,
        revision,
        texte: envoye,
      }),
    onSuccess: (e, envoye) => {
      if (e.brouillon !== null) setRevision(e.brouillon.revision);
      setEnregistre(envoye);
      setEnregistreLe(Date.now());
      void client.invalidateQueries({ queryKey: ["bibliotheque", "lire", contenuId] });
      void client.invalidateQueries({ queryKey: ["bibliotheque", "historique", contenuId] });
    },
  });
  const abandonner = useMutation({
    mutationFn: () => appeler("bibliotheque_brouillon_abandonner", { contenu_id: contenuId, version_id: brouillon.version_id }),
    onSuccess: (e) => {
      client.setQueryData(["bibliotheque", "ecriture", contenuId], e);
      void client.invalidateQueries({ queryKey: ["bibliotheque"] });
    },
  });
  const conflit = enregistrer.error instanceof ErreurApi && enregistrer.error.cle === "bibliotheque.modifiee_ailleurs";
  const appliquer = (b: Balise) => {
    const z = zone.current;
    const r = baliser(texte, z?.selectionStart ?? texte.length, z?.selectionEnd ?? texte.length, b);
    setTexte(r.texte);
    window.requestAnimationFrame(() => {
      z?.focus();
      z?.setSelectionRange(r.debut, r.fin);
    });
  };
  return (
    <div className="flex flex-col gap-3">
      <div role="toolbar" aria-label={t("bibliotheque.editeur.barre")} className="flex flex-wrap gap-1">
        {balises().map((b) => (
          <Bouton key={b.libelle} variante="secondaire" onClick={() => appliquer(b)}>
            {b.libelle}
          </Bouton>
        ))}
      </div>
      <div className="grid gap-4 lg:grid-cols-2">
        <label htmlFor="bibliotheque-editeur" className="flex flex-col gap-1 text-sm text-encre">
          <span>{t("bibliotheque.editeur.texte")}</span>
          <textarea
            id="bibliotheque-editeur"
            ref={zone}
            value={texte}
            onChange={(e) => setTexte(e.target.value)}
            spellCheck
            rows={28}
            className="min-h-96 rounded border border-muet bg-surface px-2 py-1.5 font-condensee text-primaire focus:outline-2 focus:outline-primaire"
          />
        </label>
        <div className="flex flex-col gap-1 text-sm text-encre">
          <span>{t("bibliotheque.editeur.apercu")}</span>
          <Apercu contenuId={contenuId} texte={texte} />
        </div>
      </div>
      <div className="flex flex-wrap items-center gap-2">
        <Bouton onClick={() => enregistrer.mutate(texte)} disabled={!modifie || enregistrer.isPending}>
          {t("bibliotheque.editeur.enregistrer")}
        </Bouton>
        <Bouton variante="secondaire" onClick={() => setAbandon(true)}>
          {t("bibliotheque.editeur.abandonner")}
        </Bouton>
        <MessageEnregistre instant={enregistreLe} />
      </div>
      {conflit ? (
        <div role="alert" className="flex flex-col items-start gap-2 rounded border border-rouge/40 p-3 text-sm text-rouge">
          <p>{t("bibliotheque.modifiee_ailleurs")}</p>
          <p className="text-encre">{t("bibliotheque.editeur.conflit_aide")}</p>
          <Bouton variante="secondaire" onClick={surAutreVersion}>
            {t("bibliotheque.editeur.voir_autre")}
          </Bouton>
        </div>
      ) : (
        <MessageErreur erreur={enregistrer.error} />
      )}
      {abandon ? (
        <div role="alertdialog" aria-live="polite" className="flex flex-col gap-2 rounded border border-ocre bg-fond p-3 text-sm text-encre">
          <p>{t("bibliotheque.editeur.abandonner_confirmer")}</p>
          <div className="flex gap-2">
            <Bouton variante="danger" onClick={() => abandonner.mutate()} disabled={abandonner.isPending}>
              {t("bibliotheque.editeur.abandonner")}
            </Bouton>
            <Bouton variante="secondaire" onClick={() => setAbandon(false)}>
              {t("bibliotheque.retirer.annuler")}
            </Bouton>
          </div>
          <MessageErreur erreur={abandonner.error} />
        </div>
      ) : null}
    </div>
  );
}

/** L'autre version du brouillon, lue après un conflit : le texte de l'éditeur ne bouge pas. */
function AutreVersion({ contenuId, surFermer }: { contenuId: string; surFermer: () => void }) {
  const autre = useQuery({
    queryKey: ["bibliotheque", "ecriture-autre", contenuId],
    queryFn: () => appeler("bibliotheque_ecriture_lire", { contenu_id: contenuId }),
  });
  const texte = autre.data?.brouillon?.texte ?? autre.data?.texte_depart ?? "";
  return (
    <section className="flex flex-col gap-2 rounded border border-muet p-3">
      <h3 className="text-sm font-semibold text-marine">{t("bibliotheque.editeur.autre_version")}</h3>
      <MessageErreur erreur={autre.error} />
      {autre.data?.brouillon == null ? null : (
        <p className="text-sm text-muet">{t("bibliotheque.editeur.brouillon_de", { auteur: autre.data.brouillon.auteur })}</p>
      )}
      <textarea
        readOnly
        value={texte}
        rows={14}
        aria-label={t("bibliotheque.editeur.autre_version")}
        className="rounded border border-muet bg-fond px-2 py-1.5 font-condensee text-encre"
      />
      <div>
        <Bouton variante="secondaire" onClick={surFermer}>
          {t("bibliotheque.editeur.fermer_autre")}
        </Bouton>
      </div>
    </section>
  );
}

/** Le contenu sans brouillon : d'où partir. */
function Commencer({ ecriture, surComparer }: { ecriture: EcritureOuverte; surComparer: () => void }) {
  const client = useQueryClient();
  const commencer = useMutation({
    mutationFn: (depart: DepartBrouillon) =>
      appeler("bibliotheque_brouillon_commencer", { contenu_id: ecriture.contenu_id, depart }),
    onSuccess: (e) => {
      client.setQueryData(["bibliotheque", "ecriture", ecriture.contenu_id], e);
      void client.invalidateQueries({ queryKey: ["bibliotheque", "catalogue"] });
      void client.invalidateQueries({ queryKey: ["bibliotheque", "lire", ecriture.contenu_id] });
    },
  });
  const retour = useMutation({
    mutationFn: () => appeler("bibliotheque_retour_livree", { contenu_id: ecriture.contenu_id }),
    onSuccess: (e) => {
      client.setQueryData(["bibliotheque", "ecriture", ecriture.contenu_id], e);
      void client.invalidateQueries({ queryKey: ["bibliotheque"] });
    },
  });
  return (
    <div className="flex flex-col gap-3">
      {ecriture.livree_plus_recente ? <LignePlusRecente surComparer={surComparer} surPartir={() => commencer.mutate({ type: "livree" })} /> : null}
      <div className="flex flex-wrap gap-2">
        <Bouton onClick={() => commencer.mutate({ type: "visible" })} disabled={commencer.isPending}>
          {t("bibliotheque.editeur.commencer")}
        </Bouton>
        {ecriture.version_ecole && ecriture.a_une_version_livree ? (
          <Bouton variante="secondaire" onClick={() => commencer.mutate({ type: "livree" })} disabled={commencer.isPending}>
            {t("bibliotheque.editeur.partir_livree")}
          </Bouton>
        ) : null}
        {ecriture.version_ecole && ecriture.peut_valider && ecriture.origine === "livre" ? (
          <Bouton variante="secondaire" onClick={() => retour.mutate()} disabled={retour.isPending}>
            {t("bibliotheque.editeur.retour_livree")}
          </Bouton>
        ) : null}
      </div>
      <MessageErreur erreur={commencer.error} />
      <MessageErreur erreur={retour.error} />
    </div>
  );
}

/** « La bibliothèque a une version plus récente de cette leçon », discrète. */
export function LignePlusRecente({ surComparer, surPartir }: { surComparer: () => void; surPartir?: () => void }) {
  return (
    <div role="note" className="flex flex-wrap items-center gap-2 text-sm text-muet">
      <span>{t("bibliotheque.plus_recente")}</span>
      <Bouton variante="secondaire" onClick={surComparer}>
        {t("bibliotheque.comparer")}
      </Bouton>
      {surPartir === undefined ? null : (
        <Bouton variante="secondaire" onClick={surPartir}>
          {t("bibliotheque.editeur.partir_nouvelle_livree")}
        </Bouton>
      )}
    </div>
  );
}

/** L'éditeur d'un contenu : son brouillon, ou de quoi en commencer un. */
export function Editeur({ contenuId, surComparer }: { contenuId: string; surComparer: () => void }) {
  const client = useQueryClient();
  const [autre, setAutre] = useState(false);
  const ecriture = useQuery({
    queryKey: ["bibliotheque", "ecriture", contenuId],
    queryFn: () => appeler("bibliotheque_ecriture_lire", { contenu_id: contenuId }),
  });
  const reprendre = useMutation({
    mutationFn: (versionId: string) => appeler("bibliotheque_brouillon_reprendre", { contenu_id: contenuId, version_id: versionId }),
    onSuccess: (e) => {
      client.setQueryData(["bibliotheque", "ecriture", contenuId], e);
      void client.invalidateQueries({ queryKey: ["bibliotheque", "historique", contenuId] });
    },
  });
  const e = ecriture.data;
  if (e === undefined) return <MessageErreur erreur={ecriture.error} />;
  const b = e.brouillon;
  return (
    <div className="flex flex-col gap-3">
      <MessageErreur erreur={ecriture.error} />
      {b === null ? (
        <Commencer ecriture={e} surComparer={surComparer} />
      ) : !b.mien ? (
        <div className="flex flex-col items-start gap-2 text-sm text-encre">
          <p>{t("bibliotheque.editeur.brouillon_de", { auteur: b.auteur })}</p>
          <Bouton onClick={() => reprendre.mutate(b.version_id)} disabled={reprendre.isPending}>
            {t("bibliotheque.editeur.reprendre")}
          </Bouton>
          <MessageErreur erreur={reprendre.error} />
        </div>
      ) : b.retour_livre ? (
        <RetourLivree contenuId={contenuId} brouillon={b} />
      ) : (
        <>
          {e.livree_plus_recente ? <LignePlusRecente surComparer={surComparer} /> : null}
          <EditeurBrouillon key={b.version_id} contenuId={contenuId} brouillon={b} surAutreVersion={() => setAutre(true)} />
        </>
      )}
      {autre ? <AutreVersion contenuId={contenuId} surFermer={() => setAutre(false)} /> : null}
    </div>
  );
}

/** Un brouillon « revenir à la version livrée » : rien à écrire, il peut s'abandonner. */
function RetourLivree({ contenuId, brouillon }: { contenuId: string; brouillon: BrouillonOuvert }) {
  const client = useQueryClient();
  const abandonner = useMutation({
    mutationFn: () => appeler("bibliotheque_brouillon_abandonner", { contenu_id: contenuId, version_id: brouillon.version_id }),
    onSuccess: (e) => {
      client.setQueryData(["bibliotheque", "ecriture", contenuId], e);
      void client.invalidateQueries({ queryKey: ["bibliotheque"] });
    },
  });
  return (
    <div className="flex flex-col items-start gap-2 text-sm text-encre">
      <p>{t("bibliotheque.editeur.retour_livree_aide")}</p>
      <Bouton variante="secondaire" onClick={() => abandonner.mutate()} disabled={abandonner.isPending}>
        {t("bibliotheque.editeur.abandonner")}
      </Bouton>
      <MessageErreur erreur={abandonner.error} />
    </div>
  );
}
