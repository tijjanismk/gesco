import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { Bouton } from "../../composants/Bouton";
import { CaseACocher } from "../../composants/Champ";
import { MessageEnregistre, MessageErreur, Section } from "../../composants/Messages";
import { type Cle, libelleParametre, t } from "../../i18n";
import type { Parametre, ValeurParametre } from "../../types/gesco";
import { DocumentEssai } from "../accueil/Accueil";
import { FicheEtablissement } from "./FicheEtablissement";
import { ReglagesBulletin } from "./ReglagesBulletin";

/** Thèmes des réglages, dans l'ordre d'affichage, reconnus au début de la clé. Un paramètre
 *  nouveau sans thème tombe dans « Autres réglages ». */
const THEMES: readonly { titre: Cle; prefixes: readonly string[] }[] = [
  { titre: "parametres.theme.eleves", prefixes: ["matricule.", "inscription.", "fiche_inscription.", "transfert."] },
  { titre: "parametres.theme.securite", prefixes: ["session.", "connexion.", "mot_de_passe."] },
  { titre: "parametres.theme.sauvegarde", prefixes: ["sauvegarde."] },
  { titre: "parametres.theme.finance", prefixes: ["montant.", "facturation."] },
  { titre: "parametres.theme.personnel", prefixes: ["conge."] },
  { titre: "parametres.theme.messages", prefixes: ["sms."] },
  { titre: "parametres.theme.documents", prefixes: ["numerotation.", "document.format."] },
];

/** Ordre de lecture des réglages des SMS : l'envoi, puis chaque message suivi de ses délais.
 *  Les autres réglages gardent l'ordre de leur clé. */
const ORDRE = [
  "sms.canal",
  "sms.signature",
  "sms.fenetre_envoi",
  "sms.debit_par_minute",
  "sms.plafond_mensuel",
  "sms.auto_bulletin",
  "sms.auto_impaye",
  "sms.impaye_delai_jours",
  "sms.impaye_intervalle_jours",
  "sms.impaye_nombre_max",
  "sms.impaye_montant_min",
  "sms.auto_absence",
  "sms.absence_delai_grace_min",
  "sms.sanction_preparer",
];

function rang(cle: string): number {
  const i = ORDRE.indexOf(cle);
  return i === -1 ? ORDRE.length : i;
}

function theme(cle: string): Cle {
  return THEMES.find((th) => th.prefixes.some((p) => cle.startsWith(p)))?.titre ?? "parametres.theme.autres";
}

/** Paramètres d'établissement (§3.8 : rien en dur, tout est une donnée), regroupés par thème. */
export function Parametres() {
  const parametres = useQuery({ queryKey: ["parametres"], queryFn: () => appeler("parametres_lister", null) });
  const titres: Cle[] = [...THEMES.map((th) => th.titre), "parametres.theme.autres"];
  return (
    <div className="flex max-w-3xl flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("parametres.titre")}</h1>
      <FicheEtablissement />
      <MessageErreur erreur={parametres.error} />
      {titres.map((titre) => {
        // Les réglages du bulletin, par cycle, ont leur bloc à eux.
        const lignes = (parametres.data ?? []).filter((p) => !p.cle.startsWith("bulletin.") && !p.cle.startsWith("paie.") && !p.cle.startsWith("passage.") && !p.cle.startsWith("depense.") && !p.cle.startsWith("cantine.") && !p.cle.startsWith("sms.modele_") && theme(p.cle) === titre).sort((a, b) => rang(a.cle) - rang(b.cle));
        return lignes.length === 0 ? null : (
          <Section key={titre} titre={t(titre)}>
            {lignes.map((p) => (
              // Clé seule : la ligne garde sa confirmation « Enregistré » quand la liste se recharge.
              <LigneParametre key={p.cle} parametre={p} />
            ))}
          </Section>
        );
      })}
      <ReglagesBulletin />
      <DocumentEssai />
    </div>
  );
}

function enTexte(valeur: ValeurParametre): string {
  return valeur.type === "booleen" ? "" : String(valeur.valeur);
}

/** Paramètres dont la valeur se choisit dans une liste fermée, que le noyau vérifie. */
/** Format du papier, pour chaque document qui en a le réglage. */
const FORMATS: readonly { valeur: string; libelle: Cle }[] = [
  { valeur: "a5", libelle: "document.papier.a5" },
  { valeur: "a4", libelle: "document.papier.a4" },
];

function choixDe(cle: string): readonly { valeur: string; libelle: Cle }[] | undefined {
  return CHOIX[cle] ?? (cle.startsWith("document.format.") ? FORMATS : undefined);
}

const CHOIX: Record<string, readonly { valeur: string; libelle: Cle }[]> = {
  "sms.canal": [
    { valeur: "aucun", libelle: "sms.canal.aucun" },
    { valeur: "simule", libelle: "sms.canal.simule" },
  ],
  "facturation.arrivee_en_cours": [
    { valeur: "echeances_restantes", libelle: "facturation.arrivee.echeances_restantes" },
    { valeur: "prorata_mensuel", libelle: "facturation.arrivee.prorata_mensuel" },
    { valeur: "plein_tarif", libelle: "facturation.arrivee.plein_tarif" },
  ],
};

function LigneParametre({ parametre }: { parametre: Parametre }) {
  const client = useQueryClient();
  const [texte, setTexte] = useState(enTexte(parametre.valeur));
  const [booleen, setBooleen] = useState(parametre.valeur.type === "booleen" && parametre.valeur.valeur);
  const [enregistre, setEnregistre] = useState<number | null>(null);
  const valeur = (): ValeurParametre => {
    switch (parametre.valeur.type) {
      case "booleen":
        return { type: "booleen", valeur: booleen };
      case "entier":
        return { type: "entier", valeur: Number.parseInt(texte, 10) };
      case "texte":
        return { type: "texte", valeur: texte };
    }
  };
  const definition = useMutation({
    mutationFn: () => appeler("parametre_definir", { cle: parametre.cle, valeur: valeur() }),
    onSuccess: () => {
      setEnregistre(Date.now());
      void client.invalidateQueries({ queryKey: ["parametres"] });
      void client.invalidateQueries({ queryKey: ["scolarite-reglages"] });
    },
  });
  const modifiable = parametre.modifiable_par_utilisateur;
  const libelle = libelleParametre(parametre.cle);
  return (
    <div className="flex flex-wrap items-center gap-3 border-b border-muet/30 py-2">
      {parametre.valeur.type === "booleen" ? (
        <>
          <div className="w-72">
            <CaseACocher id={`param-${parametre.cle}`} libelle={libelle} coche={booleen} desactive={!modifiable} surChangement={setBooleen} />
          </div>
          <span className="w-48" />
        </>
      ) : (
        <>
          <label htmlFor={`param-${parametre.cle}`} className="w-72 text-sm text-encre">{libelle}</label>
          {choixDe(parametre.cle) !== undefined ? (
            <select
              id={`param-${parametre.cle}`}
              className="w-48 rounded border border-muet bg-surface px-2 py-1 text-primaire disabled:text-encre"
              value={texte}
              disabled={!modifiable}
              onChange={(e) => setTexte(e.target.value)}
            >
              {(choixDe(parametre.cle) ?? []).map((c) => (
                <option key={c.valeur} value={c.valeur}>{t(c.libelle)}</option>
              ))}
            </select>
          ) : (
          <input
            id={`param-${parametre.cle}`}
            className="w-48 rounded border border-muet bg-surface px-2 py-1 text-primaire disabled:text-encre"
            inputMode={parametre.valeur.type === "entier" ? "numeric" : "text"}
            value={texte}
            disabled={!modifiable}
            onChange={(e) => setTexte(e.target.value)}
          />
          )}
        </>
      )}
      {modifiable ? (
        <Bouton variante="secondaire" onClick={() => definition.mutate()} disabled={definition.isPending}>{t("commun.enregistrer")}</Bouton>
      ) : (
        <span className="text-sm text-muet">{t("parametres.non_modifiable")}</span>
      )}
      <MessageErreur erreur={definition.error} />
      <MessageEnregistre instant={enregistre} />
    </div>
  );
}
