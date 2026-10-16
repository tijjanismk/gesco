import { useQuery } from "@tanstack/react-query";

import { appeler } from "../../api";
import { CaseACocher, Selection } from "../../composants/Champ";
import { francs, t } from "../../i18n";
import type { ChoixFrais } from "../../types/gesco";

/** Grille en vigueur d'une année : la même requête pour tous les écrans qui la montrent. */
function useGrilleEnVigueur(anneeId: string | null) {
  const grilles = useQuery({
    queryKey: ["grilles", anneeId],
    queryFn: () => appeler("grilles_lister", { annee_id: anneeId ?? "" }),
    enabled: anneeId !== null,
  });
  return { grille: grilles.data?.find((g) => g.grille.etat === "en_vigueur"), chargee: grilles.isSuccess };
}

/** Choix du parent avant la validation (SPEC_PHASE_3 §4.1) : plan de paiement et frais
 *  facultatifs de sa classe. Le noyau recalcule tout ; l'écran ne fait que proposer. */
export function ChoixFraisChamps({
  prefixe,
  anneeId,
  niveauId,
  serieId,
  valeur,
  surChangement,
}: {
  prefixe: string;
  anneeId: string | null;
  niveauId: string | null;
  serieId: string | null;
  valeur: ChoixFrais;
  surChangement: (choix: ChoixFrais) => void;
}) {
  const { grille, chargee } = useGrilleEnVigueur(anneeId);
  if (niveauId === null) return null;
  if (grille === undefined) {
    return chargee ? <p className="text-sm text-ocre">{t("facture.aucune_grille_en_vigueur")}</p> : null;
  }
  const plans = grille.contenu.plans;
  const options = grille.contenu.lignes.filter((l) => l.niveau_id === niveauId && (l.serie_id === null || l.serie_id === serieId) && !l.obligatoire);
  const cochees = valeur.frais_optionnels ?? [];
  return (
    <div className="flex flex-col gap-2">
      <Selection
        id={`${prefixe}-plan`}
        libelle={t("facture.plan")}
        valeur={valeur.plan_paiement_id ?? plans.find((p) => p.par_defaut)?.id ?? ""}
        surChangement={(v) => surChangement({ ...valeur, plan_paiement_id: v })}
        options={plans.map((p) => ({ valeur: p.id, libelle: p.libelle }))}
      />
      {options.map((l) => (
        <CaseACocher
          key={l.id}
          id={`${prefixe}-option-${l.id}`}
          libelle={`${l.libelle} · ${francs(l.montant)}`}
          coche={cochees.includes(l.id)}
          surChangement={(c) => surChangement({ ...valeur, frais_optionnels: c ? [...cochees, l.id] : cochees.filter((x) => x !== l.id) })}
        />
      ))}
    </div>
  );
}
