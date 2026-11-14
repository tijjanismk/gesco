import { t } from "../../i18n";
import type { FeuilleStatistique, FicheEcole, RubriqueFiche, ThemeRubrique, TypeReponse } from "../../types/gesco";

/** Thèmes de la fiche, dans l'ordre d'affichage. */
export const THEMES_RUBRIQUES: readonly ThemeRubrique[] = ["localisation", "identite", "actes", "eau_hygiene", "energie", "equipements", "vie_ecole", "accessibilite", "autres"];

/** Types de réponse qu'une rubrique de l'école peut prendre. */
export const TYPES_REPONSE: readonly TypeReponse[] = ["texte", "nombre", "oui_non", "date", "choix", "plusieurs_choix"];

/** Réponse telle qu'elle se lit : « Oui », une date à la française, plusieurs choix séparés par
 *  une virgule ; rien pour une case vide. */
export function texteReponse(rubrique: RubriqueFiche, valeur: string | null): string {
  if (valeur === null) return "";
  switch (rubrique.type_reponse) {
    case "oui_non":
      return valeur === "oui" ? t("commun.oui") : t("commun.non");
    case "date": {
      const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(valeur);
      return m === null ? valeur : `${m[3]}/${m[2]}/${m[1]}`;
    }
    case "nombre":
      return valeur.replace(".", ",");
    case "plusieurs_choix":
      return valeur.split("\n").join(", ");
    default:
      return valeur;
  }
}

/** Feuille Excel de la fiche : une ligne par rubrique en service, thème par thème ; une case
 *  vide quand la rubrique n'a pas de réponse. */
export function feuilleFiche(fiche: FicheEcole): FeuilleStatistique {
  const lignes: string[][] = [];
  for (const theme of THEMES_RUBRIQUES) {
    for (const r of fiche.rubriques) {
      if (r.theme !== theme || r.retiree_le !== null) continue;
      const valeur = fiche.reponses.find((x) => x.rubrique_id === r.id)?.valeur ?? null;
      lignes.push([t(`fiche.theme.${theme}`), r.libelle, texteReponse(r, valeur)]);
    }
  }
  return { titre: t("fiche.titre"), entetes: [t("fiche.theme"), t("fiche.rubrique"), t("fiche.reponse")], lignes };
}
