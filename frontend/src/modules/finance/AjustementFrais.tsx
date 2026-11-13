import type { ApercuAjustement } from "../../types/gesco";
import { dateCivile, francs, t } from "../../i18n";

/** Écart entre les frais de la facture et ceux d'une autre classe, tranche à venir par tranche
 *  à venir, montré avant que la direction l'applique : rien ne s'écrit ici. */
export function EcartsFrais({ apercu }: { apercu: ApercuAjustement }) {
  if (apercu.sans_facture) return <p className="text-muet">{t("ajustement.sans_facture")}</p>;
  if (apercu.ecarts.length === 0) return <p className="text-muet">{t("ajustement.aucun_ecart", { classe: apercu.classe_apres })}</p>;
  return (
    <div className="flex flex-col gap-1">
      <p className="text-encre">
        {t("ajustement.resume", { avant: apercu.classe_avant, apres: apercu.classe_apres, hausse: francs(apercu.hausse), baisse: francs(apercu.baisse) })}
      </p>
      <ul className="font-condensee tabular-nums text-encre">
        {apercu.ecarts.map((e) => (
          <li key={`${e.type_frais}-${e.date}`}>
            {t("ajustement.ligne", { libelle: e.libelle, date: dateCivile(e.date), avant: francs(e.avant), apres: francs(e.apres) })}
          </li>
        ))}
      </ul>
      {apercu.avoir === 0 ? null : <p className="text-encre">{t("ajustement.avoir", { avoir: francs(apercu.avoir) })}</p>}
    </div>
  );
}
