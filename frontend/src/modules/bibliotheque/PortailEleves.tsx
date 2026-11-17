import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { appeler } from "../../api";
import { Bouton } from "../../composants/Bouton";
import { CaseACocher, Champ, Selection } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { t } from "../../i18n";
import type { ReglagesPortail } from "../../types/gesco";

/** Suivi : « par élève », « par classe seulement » ou éteint ; un seul choix pour la direction. */
type ChoixSuivi = "eleve" | "classe" | "eteint";

function choixSuivi(r: ReglagesPortail): ChoixSuivi {
  if (!r.suivi) return "eteint";
  return r.grain === "classe" ? "classe" : "eleve";
}

/** Réglages du portail des élèves, pour la direction : l'interrupteur, l'adresse à donner aux
 *  élèves, la durée sans activité, les vidéos, le PDF et ce que voient les maîtres du suivi.
 *  Sans le réseau local allumé, une ligne dit ce qu'il faut allumer d'abord ; rien ne bloque. */
export function PortailEleves({ peutAllumerReseau }: { peutAllumerReseau: boolean }) {
  const client = useQueryClient();
  const reseau = useMutation({
    mutationFn: () => appeler("reseau_activer", null),
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: ["portail-etat"] });
      void client.invalidateQueries({ queryKey: ["reseau"] });
      window.setTimeout(() => void client.invalidateQueries({ queryKey: ["portail-etat"] }), 1500);
    },
  });
  const etat = useQuery({ queryKey: ["portail-etat"], queryFn: () => appeler("portail_etat", null) });
  const regler = useMutation({
    mutationFn: (reglages: ReglagesPortail) => appeler("portail_regler", reglages),
    onSuccess: (r) => {
      client.setQueryData(["portail-etat"], r);
      // Le serveur du portail prend le port un instant après : l'adresse se relit.
      window.setTimeout(() => void client.invalidateQueries({ queryKey: ["portail-etat"] }), 800);
    },
  });
  const donnees = etat.data;
  if (donnees === undefined) {
    return (
      <Section titre={t("portail.reglages.titre")}>
        <MessageErreur erreur={etat.error} />
      </Section>
    );
  }
  const r = donnees.reglages;
  const changer = (modif: Partial<ReglagesPortail>) => regler.mutate({ ...r, ...modif });
  const suivi = choixSuivi(r);
  return (
    <Section titre={t("portail.reglages.titre")}>
      <CaseACocher
        id="portail-eleves"
        libelle={t("portail.reglages.allumer")}
        coche={r.eleves}
        desactive={regler.isPending}
        surChangement={(eleves) => changer({ eleves })}
      />
      {r.eleves && !donnees.reseau_allume ? (
        <div className="flex flex-col gap-2">
          <p className="text-sm text-encre">{t("portail.reglages.reseau_d_abord")}</p>
          {peutAllumerReseau ? (
            <div>
              <Bouton variante="secondaire" onClick={() => reseau.mutate()} disabled={reseau.isPending}>
                {t("portail.reglages.allumer_reseau")}
              </Bouton>
            </div>
          ) : null}
          <MessageErreur erreur={reseau.error} />
        </div>
      ) : null}
      {r.eleves && donnees.reseau_allume && donnees.port !== null && donnees.adresses.length > 0 ? (
        <div className="flex flex-col gap-1">
          <p className="text-sm text-encre">{t("portail.reglages.adresse")}</p>
          <p className="text-2xl font-semibold text-marine" data-testid="adresse-portail">
            {donnees.adresses[0]}
          </p>
          {donnees.adresses.length > 1 ? (
            <p className="text-sm text-encre">
              {t("portail.reglages.autres_adresses", { adresses: donnees.adresses.slice(1).join(", ") })}
            </p>
          ) : null}
        </div>
      ) : null}
      {r.eleves && donnees.reseau_allume && donnees.port === null ? (
        <p className="text-sm text-encre">{t("portail.reglages.demarrage")}</p>
      ) : null}
      <Champ
        id="portail-duree"
        libelle={t("portail.reglages.duree")}
        type="number"
        min={15}
        max={1440}
        defaultValue={r.duree_session_min}
        key={`duree-${r.duree_session_min}`}
        onBlur={(e) => {
          const minutes = Number.parseInt(e.target.value, 10);
          if (Number.isFinite(minutes) && minutes !== r.duree_session_min) changer({ duree_session_min: minutes });
        }}
      />
      <CaseACocher
        id="portail-videos"
        libelle={t("portail.reglages.videos")}
        coche={r.videos}
        desactive={regler.isPending}
        surChangement={(videos) => changer({ videos })}
      />
      <CaseACocher
        id="portail-pdf"
        libelle={t("portail.reglages.pdf")}
        coche={r.pdf}
        desactive={regler.isPending}
        surChangement={(pdf) => changer({ pdf })}
      />
      <Selection
        id="portail-suivi"
        libelle={t("portail.reglages.suivi")}
        valeur={suivi}
        options={[
          { valeur: "eleve", libelle: t("portail.reglages.suivi_eleve") },
          { valeur: "classe", libelle: t("portail.reglages.suivi_classe") },
          { valeur: "eteint", libelle: t("portail.reglages.suivi_eteint") },
        ]}
        surChangement={(v) =>
          changer(v === "eteint" ? { suivi: false } : { suivi: true, grain: v === "classe" ? "classe" : "eleve" })
        }
      />
      <MessageErreur erreur={regler.error} />
    </Section>
  );
}
