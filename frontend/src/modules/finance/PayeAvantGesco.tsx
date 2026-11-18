import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler, ErreurApi } from "../../api";
import { Bouton } from "../../composants/Bouton";
import { Champ, Selection } from "../../composants/Champ";
import { MessageErreur } from "../../composants/Messages";
import { Tableau } from "../../composants/Tableau";
import { dateCivile, francs, t } from "../../i18n";
import type { LigneRepriseAvantGesco, ListeReprises } from "../../types/gesco";
import { filtrerLignes, gesteDeLaSaisie, lendemain } from "./reprise";

const TOUTES = "";

/** Refus de la reprise qui ont un plancher : le message rappelle le minimum de la ligne. */
const REFUS_AVEC_MINIMUM = new Set(["reprise.deja_remboursee", "reprise.avoir_a_couvrir"]);

/** Remplace une ligne de la liste après un geste ; les totaux se relisent ensuite. */
function remplacer(liste: ListeReprises | undefined, ligne: LigneRepriseAvantGesco): ListeReprises | undefined {
  if (liste === undefined) return liste;
  return { ...liste, lignes: liste.lignes.map((l) => (l.inscription_id === ligne.inscription_id ? ligne : l)) };
}

/** Met le curseur dans la case de la ligne suivante, comme une grille de saisie. */
function caseSuivante(index: number) {
  const suivante = document.querySelector<HTMLInputElement>(`[data-reprise="${String(index + 1)}"]`);
  suivante?.focus();
  suivante?.select();
}

/** « Payé avant GESCO » : ce que chaque parent avait payé avant l'installation, saisi au clavier,
 *  ligne après ligne. Ni versement, ni reçu, ni caisse : le reste à payer seul change. */
export function PayeAvantGesco() {
  const client = useQueryClient();
  const annees = useQuery({ queryKey: ["annees"], queryFn: () => appeler("annees_lister", null) });
  const [annee, setAnnee] = useState<string | null>(null);
  const anneeId = annee ?? annees.data?.find((a) => a.est_courante)?.id ?? annees.data?.[0]?.id ?? null;
  const classes = useQuery({ queryKey: ["classes", anneeId], queryFn: () => appeler("classes_lister", { annee_id: anneeId ?? "" }), enabled: anneeId !== null });
  const [classe, setClasse] = useState(TOUTES);
  const cle = ["reprises", anneeId, classe];
  const liste = useQuery({
    queryKey: cle,
    queryFn: () => appeler("reprises_lister", { annee_id: anneeId, classe_id: classe === TOUTES ? null : classe }),
    enabled: anneeId !== null,
  });
  const [recherche, setRecherche] = useState("");
  // Ligne vidée ou mise à zéro alors qu'elle a une reprise : on demande le motif.
  const [aAnnuler, setAAnnuler] = useState<{ ligne: LigneRepriseAvantGesco; index: number } | null>(null);
  const [motif, setMotif] = useState(t("paye_avant.motif_propose"));
  const [enErreur, setEnErreur] = useState<{ ligne: LigneRepriseAvantGesco; erreur: unknown } | null>(null);
  const [illisible, setIllisible] = useState<string | null>(null);
  const apres = (ligne: LigneRepriseAvantGesco) => {
    client.setQueryData<ListeReprises>(cle, (l) => remplacer(l, ligne));
    void client.invalidateQueries({ queryKey: ["reprises"] });
    void client.invalidateQueries({ queryKey: ["situation"] });
    void client.invalidateQueries({ queryKey: ["impayes"] });
  };
  const enregistrement = useMutation({
    mutationFn: ({ ligne, montant }: { ligne: LigneRepriseAvantGesco; montant: number; index: number }) =>
      appeler("reprise_enregistrer", { inscription_id: ligne.inscription_id, montant, note: ligne.note, lue_le: ligne.lue_le }),
    onSuccess: (ligne, { index }) => {
      setEnErreur(null);
      apres(ligne);
      caseSuivante(index);
    },
    onError: (erreur, { ligne }) => setEnErreur({ ligne, erreur }),
  });
  const annulation = useMutation({
    mutationFn: ({ ligne }: { ligne: LigneRepriseAvantGesco; index: number }) =>
      appeler("reprise_annuler", { inscription_id: ligne.inscription_id, motif, lue_le: ligne.lue_le }),
    onSuccess: (ligne, { index }) => {
      setAAnnuler(null);
      setEnErreur(null);
      apres(ligne);
      caseSuivante(index);
    },
    onError: (erreur, { ligne }) => setEnErreur({ ligne, erreur }),
  });
  const donnees = liste.data;
  const lignes = filtrerLignes(donnees?.lignes ?? [], recherche);
  const rang = new Map(lignes.map((l, i) => [l.inscription_id, i]));
  const saisir = (ligne: LigneRepriseAvantGesco, texte: string) => {
    const index = rang.get(ligne.inscription_id) ?? 0;
    const geste = gesteDeLaSaisie(texte, ligne.reprise);
    setIllisible(geste.geste === "illisible" ? ligne.inscription_id : null);
    if (geste.geste === "rien") caseSuivante(index);
    else if (geste.geste === "enregistrer") enregistrement.mutate({ ligne, montant: geste.montant, index });
    else if (geste.geste === "annuler") {
      setMotif(t("paye_avant.motif_propose"));
      setAAnnuler({ ligne, index });
    }
  };
  const lectureSeule = donnees?.lecture_seule ?? true;
  const refus = enErreur?.erreur instanceof ErreurApi ? enErreur.erreur.cle : null;
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("paye_avant.titre")}</h1>
      <p className="text-sm text-muet">{t("paye_avant.aide")}</p>
      <MessageErreur erreur={annees.error ?? classes.error ?? liste.error} />
      <div className="flex flex-wrap items-end gap-3">
        <Selection
          id="paye-avant-annee"
          libelle={t("paye_avant.annee")}
          valeur={anneeId ?? ""}
          surChangement={(v) => {
            setAnnee(v);
            setClasse(TOUTES);
          }}
          options={(annees.data ?? []).map((a) => ({ valeur: a.id, libelle: a.libelle }))}
        />
        <Selection
          id="paye-avant-classe"
          libelle={t("paye_avant.classe")}
          valeur={classe}
          surChangement={setClasse}
          options={[{ valeur: TOUTES, libelle: t("paye_avant.toutes_classes") }, ...(classes.data ?? []).map((c) => ({ valeur: c.classe.id, libelle: c.classe.libelle }))]}
        />
        <Champ id="paye-avant-recherche" type="search" libelle={t("paye_avant.rechercher")} value={recherche} onChange={(e) => setRecherche(e.target.value)} autoComplete="off" />
      </div>
      {donnees === undefined ? null : (
        <>
          <div className="flex flex-col gap-1 text-sm text-encre">
            <p className="font-medium">
              {donnees.a_ce_jour ? t("paye_avant.arrete_a_ce_jour") : t("paye_avant.arrete", { date: dateCivile(lendemain(donnees.date_arret)) })}
            </p>
            <p>{t("paye_avant.totaux", { total: francs(donnees.total), nombre: String(donnees.nombre) })}</p>
            {donnees.nombre_a_rembourser === 0 ? null : (
              <p className="text-ocre">{t("paye_avant.totaux_a_rembourser", { total: francs(donnees.total_a_rembourser), nombre: String(donnees.nombre_a_rembourser) })}</p>
            )}
            {donnees.brouillons === 0 ? null : <p className="text-muet">{t("paye_avant.brouillons", { nombre: String(donnees.brouillons) })}</p>}
            {donnees.lecture_seule ? <p className="text-muet">{t("paye_avant.lecture_seule")}</p> : null}
          </div>
          {aAnnuler === null ? null : (
            <div className="flex flex-col gap-2 rounded border border-ocre p-2 text-sm">
              <p className="text-encre">{t("paye_avant.annuler_explication", { eleve: aAnnuler.ligne.eleve, montant: francs(aAnnuler.ligne.reprise) })}</p>
              <div className="flex flex-wrap items-end gap-2">
                <Champ id="paye-avant-motif" libelle={t("paye_avant.motif")} value={motif} onChange={(e) => setMotif(e.target.value)} autoFocus />
                <Bouton onClick={() => annulation.mutate(aAnnuler)} disabled={annulation.isPending || motif.trim() === ""}>
                  {t("paye_avant.confirmer_annulation")}
                </Bouton>
                <Bouton variante="secondaire" onClick={() => setAAnnuler(null)}>{t("commun.annuler")}</Bouton>
              </div>
            </div>
          )}
          {enErreur === null ? null : (
            <div className="flex flex-col gap-1">
              <p className="text-sm text-encre">{enErreur.ligne.eleve}</p>
              <MessageErreur erreur={enErreur.erreur} surRelire={() => void liste.refetch()} />
              {refus !== null && REFUS_AVEC_MINIMUM.has(refus) ? (
                <p className="text-sm text-rouge">{t("paye_avant.au_moins", { montant: francs(enErreur.ligne.minimum) })}</p>
              ) : null}
            </div>
          )}
          <Tableau
            nomExport="paye-avant-gesco"
            titre={t("paye_avant.titre")}
            lignes={lignes}
            cle={(l) => l.inscription_id}
            colonnes={[
              { titre: t("paye_avant.classe"), valeur: (l) => l.classe },
              { titre: t("paye_avant.eleve"), valeur: (l) => l.eleve },
              { titre: t("paye_avant.matricule"), valeur: (l) => l.matricule },
              { titre: t("paye_avant.du"), valeur: (l) => francs(l.du) },
              {
                titre: t("paye_avant.paye_avant"),
                valeur: (l) => (l.reprise === 0 ? "" : francs(l.reprise)),
                rendu: (l) => (
                  <CaseReprise
                    key={`${l.inscription_id}-${String(l.reprise)}-${String(l.lue_le)}`}
                    ligne={l}
                    index={rang.get(l.inscription_id) ?? 0}
                    lectureSeule={lectureSeule}
                    illisible={illisible === l.inscription_id}
                    surSaisie={(texte) => saisir(l, texte)}
                  />
                ),
              },
              { titre: t("paye_avant.paye_gesco"), valeur: (l) => francs(l.paye_gesco) },
              { titre: t("paye_avant.reste"), valeur: (l) => francs(l.reste) },
              { titre: t("paye_avant.a_rembourser"), valeur: (l) => (l.a_rembourser === 0 ? "" : francs(l.a_rembourser)) },
            ]}
          />
        </>
      )}
    </div>
  );
}

/** Case « Payé avant GESCO » d'une ligne : Entrée enregistre et passe à la ligne suivante. */
function CaseReprise({
  ligne,
  index,
  lectureSeule,
  illisible,
  surSaisie,
}: {
  ligne: LigneRepriseAvantGesco;
  index: number;
  lectureSeule: boolean;
  illisible: boolean;
  surSaisie: (texte: string) => void;
}) {
  const [texte, setTexte] = useState(ligne.reprise === 0 ? "" : String(ligne.reprise));
  return (
    <div className="flex flex-col">
      <input
        data-reprise={index}
        aria-label={t("paye_avant.case", { eleve: ligne.eleve })}
        className="w-32 rounded border border-muet bg-surface px-2 py-0.5 text-right text-primaire focus:outline-2 focus:outline-primaire disabled:bg-fond disabled:text-encre"
        inputMode="numeric"
        autoComplete="off"
        disabled={lectureSeule}
        value={texte}
        onChange={(e) => setTexte(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            surSaisie(texte);
          }
        }}
      />
      {illisible ? <span className="text-xs text-rouge">{t("paye_avant.illisible")}</span> : null}
      {ligne.note === "" ? null : <span className="text-xs text-muet">{ligne.note}</span>}
    </div>
  );
}
