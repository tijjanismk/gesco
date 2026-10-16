import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { Bouton } from "../../composants/Bouton";
import { CaseACocher, Selection } from "../../composants/Champ";
import { MessageEnregistre, MessageErreur, Section } from "../../composants/Messages";
import { type Cle, t } from "../../i18n";
import type { Cycle } from "../../types/gesco";
import { CYCLES } from "../scolarite/Cycles";

const CYCLES_A_BULLETIN: readonly Cycle[] = ["premier_cycle", "second_cycle", "lycee", "technique"];

/** Libellé de chaque réglage, d'après la fin de sa clé (`bulletin.<cycle>.colonne.points`). */
const LIBELLES: Record<string, Cle> = {
  "colonne.note_de_classe": "bulletin.colonne.note_de_classe",
  "colonne.composition": "bulletin.colonne.composition",
  "colonne.coefficient": "bulletin.colonne.coefficient",
  "colonne.points": "bulletin.colonne.points",
  "colonne.moyenne_classe": "bulletin.colonne.moyenne_classe",
  "colonne.appreciation": "bulletin.colonne.appreciation",
  "afficher.moyenne_generale": "bulletin.afficher.moyenne_generale",
  "afficher.ont_compose": "bulletin.afficher.ont_compose",
  "afficher.rang": "bulletin.afficher.rang",
  "afficher.mention": "bulletin.afficher.mention",
  "afficher.premier_dernier": "bulletin.afficher.premier_dernier",
  "afficher.moyenne_classe": "bulletin.afficher.moyenne_classe",
  "afficher.absences": "bulletin.afficher.absences",
  "afficher.appreciation_generale": "bulletin.afficher.appreciation_generale",
};

function maintenant(): number {
  return Date.now();
}

/** Bulletin imprimé : pour chaque cycle, les colonnes et les lignes du bilan qui s'impriment.
 *  Les notes et les moyennes n'en dépendent pas. */
export function ReglagesBulletin() {
  const client = useQueryClient();
  const parametres = useQuery({ queryKey: ["parametres"], queryFn: () => appeler("parametres_lister", null) });
  const referentiel = useQuery({ queryKey: ["referentiel"], queryFn: () => appeler("niveaux_lister", null) });
  // Le cycle technique ne paraît que dans l'école qui l'enseigne.
  const technique = (referentiel.data?.niveaux ?? []).some((n) => n.actif && n.cycle === "technique");
  const cycles = CYCLES_A_BULLETIN.filter((c) => c !== "technique" || technique);
  const [cycle, setCycle] = useState<Cycle>("second_cycle");
  const [choix, setChoix] = useState<Record<string, boolean>>({});
  const [enregistre, setEnregistre] = useState<number | null>(null);
  const prefixe = `bulletin.${cycle}.`;
  // Dans l'ordre du bulletin : les colonnes, puis les lignes du bilan.
  const ordre = Object.keys(LIBELLES);
  const rang = (cle: string) => ordre.indexOf(cle.slice(prefixe.length));
  const reglages = (parametres.data ?? [])
    .filter((p) => p.cle.startsWith(prefixe) && p.valeur.type === "booleen")
    .sort((a, b) => rang(a.cle) - rang(b.cle));
  const valeur = (cle: string, enBase: boolean) => choix[cle] ?? enBase;
  const changes = reglages.filter((p) => p.valeur.type === "booleen" && choix[p.cle] !== undefined && choix[p.cle] !== p.valeur.valeur);
  const enregistrement = useMutation({
    mutationFn: async () => {
      for (const p of changes) {
        await appeler("parametre_definir", { cle: p.cle, valeur: { type: "booleen", valeur: choix[p.cle] ?? false } });
      }
    },
    onSuccess: () => {
      setChoix({});
      setEnregistre(maintenant());
      void client.invalidateQueries({ queryKey: ["parametres"] });
    },
  });
  if (reglages.length === 0) return <MessageErreur erreur={parametres.error} />;
  return (
    <Section titre={t("parametres.theme.bulletin")}>
      <p className="text-sm text-muet">{t("bulletin.reglages_aide")}</p>
      <div className="w-56">
        <Selection
          id="bulletin-cycle"
          libelle={t("niveaux.cycle")}
          valeur={cycle}
          surChangement={(v) => {
            setCycle(v as Cycle);
            setChoix({});
          }}
          options={cycles.map((c) => ({ valeur: c, libelle: t(CYCLES[c]) }))}
        />
      </div>
      <div className="grid gap-x-6 gap-y-2 sm:grid-cols-2">
        {reglages.map((p) => {
          const suffixe = p.cle.slice(prefixe.length);
          return (
            <CaseACocher
              key={p.cle}
              id={`bulletin-${suffixe}`}
              libelle={LIBELLES[suffixe] === undefined ? suffixe : t(LIBELLES[suffixe])}
              coche={valeur(p.cle, p.valeur.type === "booleen" && p.valeur.valeur)}
              desactive={!p.modifiable_par_utilisateur}
              surChangement={(c) => setChoix((x) => ({ ...x, [p.cle]: c }))}
            />
          );
        })}
      </div>
      <div className="flex items-center gap-3">
        <Bouton onClick={() => enregistrement.mutate()} disabled={enregistrement.isPending || changes.length === 0}>{t("commun.enregistrer")}</Bouton>
        <MessageEnregistre instant={enregistre} />
      </div>
      <MessageErreur erreur={enregistrement.error} />
    </Section>
  );
}
