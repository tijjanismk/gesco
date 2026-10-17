import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { aujourdHui } from "../../app/aujourdhui";
import { Bouton } from "../../composants/Bouton";
import { Champ, Formulaire, Selection } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { Tableau } from "../../composants/Tableau";
import { type Cle, dateCivile, t } from "../../i18n";
import type { DossierDiscipline, NatureSanction, RegistreDiscipline, SanctionAffichee } from "../../types/gesco";

const NATURES: Record<NatureSanction, Cle> = {
  avertissement: "sanction.nature.avertissement",
  blame: "sanction.nature.blame",
  retenue: "sanction.nature.retenue",
  convocation_parents: "sanction.nature.convocation_parents",
};

const ORDRE: readonly NatureSanction[] = ["avertissement", "blame", "retenue", "convocation_parents"];

function etat(s: SanctionAffichee): string {
  if (s.sanction.annulee) return t("sanction.annulee", { motif: s.sanction.motif_annulation });
  return s.periode === null ? t("sanction.au_dossier") : t("sanction.au_bulletin", { periode: s.periode });
}

/** Discipline (SPEC_VERSION_1_1 §1) : le registre de l'année et le dossier de chaque élève ;
 *  l'avertissement et le blâme du conseil de classe vont au bulletin de leur période. */
export function Discipline() {
  const client = useQueryClient();
  const registre = useQuery({ queryKey: ["discipline"], queryFn: () => appeler("discipline_registre_lire", null) });
  const [inscription, setInscription] = useState<string | null>(null);
  const r = registre.data;
  const apres = (d: DossierDiscipline) => {
    client.setQueryData(["dossier-discipline", d.inscription_id], d);
    void client.invalidateQueries({ queryKey: ["discipline"] });
  };
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("discipline.titre")}</h1>
      <MessageErreur erreur={registre.error} />
      {r === undefined ? null : (
        <>
          {r.droits.sanctionner ? <NouvelleSanction r={r} surEleve={setInscription} surFait={apres} /> : null}
          <Section titre={t("discipline.registre")}>
            <Tableau
              nomExport="registre-de-discipline"
              titre={t("discipline.registre")}
              lignes={r.sanctions}
              cle={(s) => s.sanction.id}
              surSelection={(s) => setInscription(s.sanction.inscription_id)}
              colonnes={[
                { titre: t("depense.date"), valeur: (s) => dateCivile(s.sanction.date) },
                { titre: t("discipline.eleve"), valeur: (s) => s.eleve },
                { titre: t("discipline.classe"), valeur: (s) => s.classe },
                { titre: t("discipline.nature"), valeur: (s) => t(NATURES[s.sanction.nature]) },
                { titre: t("discipline.motif"), valeur: (s) => s.sanction.motif },
                { titre: t("discipline.decidee_par"), valeur: (s) => s.sanction.decidee_par },
                { titre: t("depense.etat"), valeur: etat },
              ]}
            />
          </Section>
          {inscription === null ? null : <Dossier key={inscription} inscriptionId={inscription} surFait={apres} />}
        </>
      )}
    </div>
  );
}

function NouvelleSanction({ r, surEleve, surFait }: { r: RegistreDiscipline; surEleve: (id: string) => void; surFait: (d: DossierDiscipline) => void }) {
  const inscrits = useQuery({ queryKey: ["inscrits", r.annee_id], queryFn: () => appeler("inscriptions_lister", { annee_id: r.annee_id }) });
  const valides = (inscrits.data ?? []).filter((l) => l.inscription.statut === "validee");
  const classes = [...new Set(valides.map((l) => l.classe))].sort();
  const [classe, setClasse] = useState("");
  const [eleve, setEleve] = useState("");
  const [nature, setNature] = useState<NatureSanction>("avertissement");
  const [date, setDate] = useState(aujourdHui());
  const [motif, setMotif] = useState("");
  const [decidee, setDecidee] = useState(t("discipline.decidee_par_defaut"));
  const [periode, setPeriode] = useState("");
  const dossier = useQuery({ queryKey: ["dossier-discipline", eleve], queryFn: () => appeler("discipline_dossier_lire", { inscription_id: eleve }), enabled: eleve !== "" });
  const duConseil = nature === "avertissement" || nature === "blame";
  const noter = useMutation({
    mutationFn: () => appeler("sanction_noter", { inscription_id: eleve, nature, date, motif, decidee_par: decidee, periode_id: duConseil && periode !== "" ? periode : null }),
    onSuccess: (d) => {
      setMotif("");
      surEleve(d.inscription_id);
      surFait(d);
    },
  });
  const eleves = valides.filter((l) => l.classe === classe);
  return (
    <Section titre={t("discipline.nouvelle")}>
      <Formulaire surEnvoi={() => noter.mutate()}>
        <div className="flex flex-wrap items-end gap-3">
          <Selection
            id="sanction-classe"
            libelle={t("discipline.classe")}
            valeur={classe}
            surChangement={(v) => {
              setClasse(v);
              setEleve("");
            }}
            options={[{ valeur: "", libelle: t("discipline.choisir") }, ...classes.map((c) => ({ valeur: c, libelle: c }))]}
          />
          <Selection id="sanction-eleve" libelle={t("discipline.eleve")} valeur={eleve} surChangement={setEleve} options={[{ valeur: "", libelle: t("discipline.choisir") }, ...eleves.map((l) => ({ valeur: l.inscription.id, libelle: l.eleve }))]} />
          <Selection id="sanction-nature" libelle={t("discipline.nature")} valeur={nature} surChangement={(v) => setNature(ORDRE.find((n) => n === v) ?? "avertissement")} options={ORDRE.map((n) => ({ valeur: n, libelle: t(NATURES[n]) }))} />
          <Champ id="sanction-date" type="date" libelle={t("depense.date")} value={date} onChange={(ev) => setDate(ev.target.value)} />
          <Champ id="sanction-motif" libelle={t("discipline.motif")} value={motif} onChange={(ev) => setMotif(ev.target.value)} autoComplete="off" />
          <Champ id="sanction-decidee" libelle={t("discipline.decidee_par")} value={decidee} onChange={(ev) => setDecidee(ev.target.value)} autoComplete="off" />
          {duConseil ? (
            <Selection
              id="sanction-periode"
              libelle={t("discipline.bulletin")}
              valeur={periode}
              surChangement={setPeriode}
              options={[{ valeur: "", libelle: t("discipline.pas_au_bulletin") }, ...(dossier.data?.periodes ?? []).map((p) => ({ valeur: p.periode_id, libelle: p.libelle }))]}
            />
          ) : null}
          <Bouton type="submit" disabled={eleve === "" || motif.trim() === "" || decidee.trim() === "" || noter.isPending}>
            {t("discipline.noter")}
          </Bouton>
        </div>
        <MessageErreur erreur={noter.error ?? inscrits.error} />
      </Formulaire>
    </Section>
  );
}

function Dossier({ inscriptionId, surFait }: { inscriptionId: string; surFait: (d: DossierDiscipline) => void }) {
  const dossier = useQuery({ queryKey: ["dossier-discipline", inscriptionId], queryFn: () => appeler("discipline_dossier_lire", { inscription_id: inscriptionId }) });
  const [choisie, setChoisie] = useState<string | null>(null);
  const [motif, setMotif] = useState("");
  const annuler = useMutation({
    mutationFn: () => appeler("sanction_annuler", { id: choisie ?? "", motif }),
    onSuccess: (d) => {
      setChoisie(null);
      setMotif("");
      surFait(d);
    },
  });
  const d = dossier.data;
  if (d === undefined) return <MessageErreur erreur={dossier.error} />;
  const sanction = d.sanctions.find((s) => s.sanction.id === choisie);
  return (
    <Section titre={t("discipline.dossier", { eleve: d.eleve, classe: d.classe })}>
      <ul className="flex flex-col gap-1 text-sm text-encre">
        {d.sanctions.length === 0 ? <li className="text-muet">{t("discipline.dossier_vide")}</li> : null}
        {d.sanctions.map((s) => (
          <li key={s.sanction.id}>
            <button type="button" className={`text-left ${s.sanction.annulee ? "text-muet line-through" : ""}`} onClick={() => setChoisie(s.sanction.id)} disabled={!d.droits.annuler || s.sanction.annulee}>
              {`${dateCivile(s.sanction.date)} · ${t(NATURES[s.sanction.nature])} · ${s.sanction.motif} · ${s.sanction.decidee_par} · ${etat(s)}`}
            </button>
          </li>
        ))}
      </ul>
      {sanction === undefined || !d.droits.annuler ? null : (
        <div className="flex flex-wrap items-end gap-3">
          <Champ id="sanction-annulation" libelle={t("depense.motif_annulation")} value={motif} onChange={(ev) => setMotif(ev.target.value)} autoComplete="off" />
          <Bouton variante="danger" onClick={() => annuler.mutate()} disabled={motif.trim() === "" || annuler.isPending}>
            {t("discipline.annuler")}
          </Bouton>
        </div>
      )}
      <MessageErreur erreur={annuler.error} />
    </Section>
  );
}
