import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { Selection } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { t } from "../../i18n";
import type { EleveExamen, Examen, Examens, Sexe, TotalAffiche } from "../../types/gesco";

type Champ = "inscrit" | "present" | "admis";

/** Saisie des résultats aux examens, en tête de l'onglet Examens et fin d'année : la grille
 *  d'une classe d'examen — inscrit, présent, admis, mention —, et les totaux par sexe pour qui ne
 *  saisit pas élève par élève. Une case s'enregistre dès qu'on la coche ou qu'on la quitte. */
export function ResultatsExamens({ anneeId }: { anneeId: string | null }) {
  const [classeId, setClasseId] = useState<string | null>(null);
  const cle = ["examens", anneeId, classeId] as const;
  const examens = useQuery({ queryKey: cle, queryFn: () => appeler("examens_lire", { annee_id: anneeId, classe_id: classeId }) });
  const client = useQueryClient();
  const rafraichir = () => {
    void client.invalidateQueries({ queryKey: ["examens"] });
    void client.invalidateQueries({ queryKey: ["statistiques"] });
  };
  const entree7e = useMutation({ mutationFn: (actif: boolean) => appeler("examens_entree_7e_regler", { actif }), onSuccess: rafraichir });
  const d = examens.data;
  if (d === undefined) return <MessageErreur erreur={examens.error} />;
  return (
    <Section titre={t("examens.saisie")}>
      <label className="flex items-center gap-2 self-start text-sm text-encre">
        <input type="checkbox" checked={d.entree_7e} onChange={(e) => entree7e.mutate(e.target.checked)} disabled={entree7e.isPending} />
        {t("examens.entree_7e")}
      </label>
      <MessageErreur erreur={entree7e.error} />
      {d.classes.length === 0 ? null : (
        <>
          <Selection id="examens-classe" libelle={t("examens.classe")} valeur={d.classe_id ?? ""} surChangement={setClasseId} options={d.classes.map((c) => ({ valeur: c.id, libelle: `${c.libelle} · ${t(`examens.examen.${c.examen}`)}` }))} />
          <GrilleClasse eleves={d.eleves} surEnregistre={rafraichir} />
        </>
      )}
      {d.annee_id === null ? null : <Totaux donnees={d} anneeId={d.annee_id} surEnregistre={rafraichir} />}
    </Section>
  );
}

function GrilleClasse({ eleves, surEnregistre }: { eleves: readonly EleveExamen[]; surEnregistre: () => void }) {
  if (eleves.length === 0) return <p className="text-sm text-muet">{t("commun.aucune_ligne")}</p>;
  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse text-sm">
        <thead>
          <tr>
            {(["examens.eleve", "examens.inscrit", "examens.present", "examens.admis", "examens.mention"] as const).map((c) => (
              <th key={c} className="border-b border-muet px-2 py-1 text-left font-medium text-muet">
                {t(c)}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {eleves.map((e) => (
            <LigneEleve key={`${e.inscription_id}-${String(e.inscrit)}-${String(e.present)}-${String(e.admis)}-${e.mention}`} eleve={e} surEnregistre={surEnregistre} />
          ))}
        </tbody>
      </table>
    </div>
  );
}

function LigneEleve({ eleve, surEnregistre }: { eleve: EleveExamen; surEnregistre: () => void }) {
  const [mention, setMention] = useState(eleve.mention);
  const regler = useMutation({
    mutationFn: (e: Pick<EleveExamen, Champ | "mention">) => appeler("resultat_examen_regler", { inscription_id: eleve.inscription_id, ...e }),
    onSuccess: surEnregistre,
  });
  const valeurs = { inscrit: eleve.inscrit, present: eleve.present, admis: eleve.admis, mention: eleve.mention };
  const cocher = (champ: Champ, oui: boolean) => regler.mutate({ ...valeurs, [champ]: oui });
  return (
    <tr>
      <th scope="row" className="border-b border-muet/30 px-2 py-1 text-left font-normal text-encre">
        {eleve.nom}
        <MessageErreur erreur={regler.error} />
      </th>
      {(["inscrit", "present", "admis"] as const).map((champ) => (
        <td key={champ} className="border-b border-muet/30 px-2 py-1">
          <input type="checkbox" aria-label={`${eleve.nom} · ${t(`examens.${champ}`)}`} checked={eleve[champ] === true} onChange={(e) => cocher(champ, e.target.checked)} disabled={regler.isPending} />
        </td>
      ))}
      <td className="border-b border-muet/30 px-2 py-1">
        <input
          type="text"
          maxLength={40}
          aria-label={`${eleve.nom} · ${t("examens.mention")}`}
          className="w-36 rounded border border-muet bg-surface px-1.5 py-1 text-primaire"
          value={mention}
          onChange={(e) => setMention(e.target.value)}
          onBlur={() => {
            if (mention.trim() !== eleve.mention) regler.mutate({ ...valeurs, mention });
          }}
        />
      </td>
    </tr>
  );
}

const SEXES: readonly Sexe[] = ["masculin", "feminin"];
const CHAMPS_TOTAL = ["inscrits", "presentes", "admis"] as const;

/** Totaux par examen et par sexe ; un examen saisi élève par élève les ignore, et le dit. */
function Totaux({ donnees, anneeId, surEnregistre }: { donnees: Examens; anneeId: string; surEnregistre: () => void }) {
  const examens: Examen[] = donnees.examens.length > 0 ? donnees.examens : ["def", "bac"];
  return (
    <div className="flex flex-col gap-2">
      <h3 className="text-sm font-medium text-encre">{t("examens.totaux")}</h3>
      <div className="overflow-x-auto">
        <table className="border-collapse text-sm">
          <thead>
            <tr>
              <th className="border-b border-muet px-2 py-1 text-left font-medium text-muet">{t("examens.examen")}</th>
              <th className="border-b border-muet px-2 py-1 text-left font-medium text-muet">{t("examens.sexe")}</th>
              {CHAMPS_TOTAL.map((c) => (
                <th key={c} className="border-b border-muet px-2 py-1 text-left font-medium text-muet">
                  {t(`examens.total.${c}`)}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {examens.flatMap((examen) =>
              SEXES.map((sexe) => {
                const total = donnees.totaux.find((x) => x.examen === examen && x.sexe === sexe) ?? null;
                return <LigneTotal key={`${examen}-${sexe}-${JSON.stringify(total)}`} anneeId={anneeId} examen={examen} sexe={sexe} total={total} ignore={donnees.avec_detail.includes(examen)} surEnregistre={surEnregistre} />;
              }),
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}

function LigneTotal({ anneeId, examen, sexe, total, ignore, surEnregistre }: { anneeId: string; examen: Examen; sexe: Sexe; total: TotalAffiche | null; ignore: boolean; surEnregistre: () => void }) {
  const depart = { inscrits: total?.inscrits ?? null, presentes: total?.presentes ?? null, admis: total?.admis ?? null };
  const [valeurs, setValeurs] = useState(depart);
  const regler = useMutation({ mutationFn: () => appeler("total_examen_regler", { annee_id: anneeId, examen, sexe, ...valeurs }), onSuccess: surEnregistre });
  return (
    <tr className={ignore ? "text-muet" : ""}>
      <td className="border-b border-muet/30 px-2 py-1">
        {t(`examens.examen.${examen}`)}
        {ignore ? <span className="block text-xs">{t("examens.detail_l_emporte")}</span> : null}
      </td>
      <td className="border-b border-muet/30 px-2 py-1">{t(sexe === "masculin" ? "statistiques.colonne.garcons" : "statistiques.colonne.filles")}</td>
      {CHAMPS_TOTAL.map((c) => (
        <td key={c} className="border-b border-muet/30 px-1 py-1">
          <input
            type="number"
            min={0}
            aria-label={`${t(`examens.examen.${examen}`)} · ${sexe} · ${t(`examens.total.${c}`)}`}
            className="w-20 rounded border border-muet bg-surface px-1.5 py-1 text-right text-primaire"
            value={valeurs[c] ?? ""}
            onChange={(e) => {
              const n = e.target.value.trim() === "" ? null : Math.max(0, Math.floor(Number(e.target.value)));
              setValeurs({ ...valeurs, [c]: n !== null && Number.isFinite(n) ? n : null });
            }}
            onBlur={() => {
              if (JSON.stringify(valeurs) !== JSON.stringify(depart)) regler.mutate();
            }}
          />
        </td>
      ))}
      <td>
        <MessageErreur erreur={regler.error} />
      </td>
    </tr>
  );
}
