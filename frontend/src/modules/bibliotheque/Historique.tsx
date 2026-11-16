import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { Bouton } from "../../composants/Bouton";
import { Selection } from "../../composants/Champ";
import { MessageErreur } from "../../composants/Messages";
import { date, heure, t } from "../../i18n";
import type { DepartBrouillon, EtatVersion, HistoriqueContenu, RangeeComparaison, TexteCompare, VersionAffichee } from "../../types/gesco";
import { LignePlusRecente } from "./Editeur";

function libelleEtat(etat: EtatVersion): string {
  switch (etat) {
    case "brouillon":
      return t("bibliotheque.etat.brouillon");
    case "soumise":
      return t("bibliotheque.etat.soumise");
    case "renvoyee":
      return t("bibliotheque.etat.renvoyee");
    case "publiee":
      return t("bibliotheque.etat.publiee");
    case "remplacee":
      return t("bibliotheque.etat.remplacee");
    case "abandonnee":
      return t("bibliotheque.etat.abandonnee");
  }
}

const quand = (millis: number) => `${date(millis)} ${heure(millis)}`;

/** Valeur d'un choix de texte à comparer. */
function versValeur(c: TexteCompare): string {
  if (c.type === "version") return `v:${c.version_id}`;
  return c.type === "recue" ? `r:${c.version_recue_id}` : c.type;
}

function depuisValeur(v: string): TexteCompare {
  if (v.startsWith("v:")) return { type: "version", version_id: v.slice(2) };
  if (v.startsWith("r:")) return { type: "recue", version_recue_id: v.slice(2) };
  return v === "livree" ? { type: "livree" } : { type: "visible" };
}

/** Choix proposés : la version livrée, la version visible, chaque version de l'école, chaque
 *  version reçue d'une autre école. */
function choixTextes(h: HistoriqueContenu): { valeur: string; libelle: string }[] {
  const choix = [];
  if (h.a_une_version_livree) choix.push({ valeur: "livree", libelle: t("bibliotheque.vue.livree") });
  choix.push({ valeur: "visible", libelle: t("bibliotheque.vue.visible") });
  for (const v of h.versions)
    choix.push({ valeur: `v:${v.id}`, libelle: t("bibliotheque.historique.version_etat", { numero: v.numero, etat: libelleEtat(v.etat) }) });
  for (const r of h.recues) choix.push({ valeur: `r:${r.id}`, libelle: t("bibliotheque.historique.recue_le", { quand: quand(r.recue_le) }) });
  return choix;
}

/** Une ligne de la comparaison, le signe dit ce qu'elle est sans compter sur la couleur. */
function Cellule({ ligne, signe, change }: { ligne: RangeeComparaison["gauche"]; signe: string; change: boolean }) {
  if (ligne === null) return <td className="border-t border-muet/30 bg-fond" />;
  return (
    <td className={`border-t border-muet/30 px-2 align-top font-condensee whitespace-pre-wrap ${change ? "bg-fond font-semibold text-encre" : "text-encre"}`}>
      <span className="mr-2 inline-block w-10 text-right text-xs text-muet select-none">
        {change ? signe : ""} {ligne.numero}
      </span>
      {ligne.texte}
    </td>
  );
}

/** Deux versions côte à côte : lignes retirées à gauche, ajoutées à droite. */
export function Comparaison({ historique, initial }: { historique: HistoriqueContenu; initial: [TexteCompare, TexteCompare] }) {
  const [gauche, setGauche] = useState<TexteCompare>(initial[0]);
  const [droite, setDroite] = useState<TexteCompare>(initial[1]);
  const comparaison = useQuery({
    queryKey: ["bibliotheque", "comparer", historique.contenu_id, gauche, droite],
    queryFn: () => appeler("bibliotheque_comparer", { contenu_id: historique.contenu_id, gauche, droite }),
  });
  const choix = choixTextes(historique);
  const donnees = comparaison.data;
  return (
    <section className="flex flex-col gap-2">
      <h3 className="text-base font-semibold text-marine">{t("bibliotheque.comparer")}</h3>
      <div className="flex flex-wrap gap-3">
        <Selection id="comparer-gauche" libelle={t("bibliotheque.comparer.gauche")} valeur={versValeur(gauche)} surChangement={(v) => setGauche(depuisValeur(v))} options={choix} />
        <Selection id="comparer-droite" libelle={t("bibliotheque.comparer.droite")} valeur={versValeur(droite)} surChangement={(v) => setDroite(depuisValeur(v))} options={choix} />
      </div>
      <MessageErreur erreur={comparaison.error} />
      {donnees === undefined ? null : donnees.retirees === 0 && donnees.ajoutees === 0 ? (
        <p className="text-sm text-muet">{t("bibliotheque.comparer.pareilles")}</p>
      ) : (
        <>
          <p className="text-sm text-muet">{t("bibliotheque.comparer.bilan", { retirees: donnees.retirees, ajoutees: donnees.ajoutees })}</p>
          <div className="overflow-x-auto">
            <table className="w-full table-fixed border-collapse text-sm">
              <thead>
                <tr>
                  <th scope="col" className="px-2 text-left text-muet">{choix.find((c) => c.valeur === versValeur(gauche))?.libelle}</th>
                  <th scope="col" className="px-2 text-left text-muet">{choix.find((c) => c.valeur === versValeur(droite))?.libelle}</th>
                </tr>
              </thead>
              <tbody>
                {donnees.rangees.map((r, i) => (
                  <tr key={i}>
                    <Cellule ligne={r.gauche} signe="−" change={r.change} />
                    <Cellule ligne={r.droite} signe="+" change={r.change} />
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}
    </section>
  );
}

function LigneVersion({
  v,
  peutEcrire,
  surRepartir,
  enCours,
}: {
  v: VersionAffichee;
  peutEcrire: boolean;
  surRepartir: (id: string) => void;
  enCours: boolean;
}) {
  return (
    <li className="flex flex-col gap-1 border-b border-muet/30 py-2 text-sm text-encre">
      <p className="font-semibold">
        {t("bibliotheque.historique.version_etat", { numero: v.numero, etat: libelleEtat(v.etat) })}
        {v.retour_livre ? ` · ${t("bibliotheque.historique.retour_livree")}` : ""}
      </p>
      <p className="text-muet">
        {t("bibliotheque.historique.ecrite_par", { auteur: v.auteur, quand: quand(v.cree_le) })}
        {v.depart_livree ? ` · ${t("bibliotheque.historique.depart_livree")}` : ""}
        {v.depart_numero === null ? "" : ` · ${t("bibliotheque.historique.depart_version", { numero: v.depart_numero })}`}
      </p>
      {v.etat === "brouillon" ? <p className="text-muet">{t("bibliotheque.historique.enregistre", { quand: quand(v.enregistre_le) })}</p> : null}
      {v.soumise_le === null ? null : <p className="text-muet">{t("bibliotheque.historique.soumise", { quand: quand(v.soumise_le) })}</p>}
      {v.decidee_le === null || v.etat === "soumise" ? null : (
        <p className="text-muet">
          {t("bibliotheque.historique.decidee", { etat: libelleEtat(v.etat), quand: quand(v.decidee_le), qui: v.decidee_par ?? "" })}
        </p>
      )}
      {v.commentaire === "" ? null : <p>{t("bibliotheque.historique.commentaire", { commentaire: v.commentaire })}</p>}
      {peutEcrire && !v.retour_livre && v.etat !== "brouillon" ? (
        <div>
          <Bouton variante="secondaire" onClick={() => surRepartir(v.id)} disabled={enCours}>
            {t("bibliotheque.historique.repartir")}
          </Bouton>
        </div>
      ) : null}
    </li>
  );
}

/** Historique d'un contenu : versions, retraits et remises, comparaison, repartir d'une version. */
export function Historique({ contenuId, surEcrire, comparerLivree }: { contenuId: string; surEcrire: () => void; comparerLivree: boolean }) {
  const client = useQueryClient();
  const [comparer, setComparer] = useState(comparerLivree);
  // Comparer une version reçue : la version visible à gauche, la version reçue à droite.
  const [initialRecue, setInitialRecue] = useState<[TexteCompare, TexteCompare] | null>(null);
  const historique = useQuery({
    queryKey: ["bibliotheque", "historique", contenuId],
    queryFn: () => appeler("bibliotheque_historique", { contenu_id: contenuId }),
  });
  const repartir = useMutation({
    mutationFn: (depart: DepartBrouillon) => appeler("bibliotheque_brouillon_commencer", { contenu_id: contenuId, depart }),
    onSuccess: (e) => {
      client.setQueryData(["bibliotheque", "ecriture", contenuId], e);
      void client.invalidateQueries({ queryKey: ["bibliotheque"] });
      surEcrire();
    },
  });
  const h = historique.data;
  if (h === undefined) return <MessageErreur erreur={historique.error} />;
  const derniere = h.versions[0];
  const initial: [TexteCompare, TexteCompare] = h.a_une_version_livree
    ? [{ type: "livree" }, derniere === undefined ? { type: "visible" } : { type: "version", version_id: derniere.id }]
    : [{ type: "visible" }, derniere === undefined ? { type: "visible" } : { type: "version", version_id: derniere.id }];
  return (
    <div className="flex flex-col gap-3">
      {h.livree_plus_recente ? <LignePlusRecente surComparer={() => setComparer(true)} /> : null}
      <div>
        <Bouton
          variante="secondaire"
          onClick={() => {
            setInitialRecue(null);
            setComparer(!comparer);
          }}
          aria-pressed={comparer}
        >
          {t("bibliotheque.comparer")}
        </Bouton>
      </div>
      {comparer ? (
        <Comparaison
          key={initialRecue === null ? "versions" : versValeur(initialRecue[1])}
          historique={h}
          initial={initialRecue ?? (comparerLivree ? [{ type: "livree" }, { type: "visible" }] : initial)}
        />
      ) : null}
      <MessageErreur erreur={repartir.error} />
      {h.versions.length === 0 ? (
        <p className="text-sm text-muet">{t("bibliotheque.historique.aucune")}</p>
      ) : (
        <ul className="flex flex-col">
          {h.versions.map((v) => (
            <LigneVersion key={v.id} v={v} peutEcrire={h.peut_ecrire} surRepartir={(id) => repartir.mutate({ type: "version", version_id: id })} enCours={repartir.isPending} />
          ))}
        </ul>
      )}
      {h.recues.length === 0 ? null : (
        <ul className="flex flex-col" aria-label={t("bibliotheque.historique.recues")}>
          {h.recues.map((r) => (
            <li key={r.id} className="flex flex-col gap-1 border-b border-muet/30 py-2 text-sm text-encre">
              <p className="font-semibold">{t("bibliotheque.historique.recue")}</p>
              <p className="text-muet">{t("bibliotheque.historique.recue_par", { quand: quand(r.recue_le), qui: r.importe_par })}</p>
              <div className="flex flex-wrap gap-2">
                <Bouton
                  variante="secondaire"
                  onClick={() => {
                    setInitialRecue([{ type: "visible" }, { type: "recue", version_recue_id: r.id }]);
                    setComparer(true);
                  }}
                >
                  {t("bibliotheque.comparer")}
                </Bouton>
                {h.peut_ecrire ? (
                  <Bouton variante="secondaire" onClick={() => repartir.mutate({ type: "recue", version_recue_id: r.id })} disabled={repartir.isPending}>
                    {t("bibliotheque.historique.repartir")}
                  </Bouton>
                ) : null}
              </div>
            </li>
          ))}
        </ul>
      )}
      {h.mouvements.length === 0 ? null : (
        <ul className="flex flex-col gap-1 text-sm text-muet">
          {h.mouvements.map((m, i) => (
            <li key={i}>
              {m.sorte === "retrait"
                ? t("bibliotheque.historique.retire", { quand: quand(m.quand), qui: m.qui })
                : t("bibliotheque.historique.remis", { quand: quand(m.quand), qui: m.qui })}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
