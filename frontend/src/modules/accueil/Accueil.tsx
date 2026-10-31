import { useMutation, useQuery } from "@tanstack/react-query";
import type { ReactNode } from "react";

import { appeler } from "../../api";
import { ApercuPdf } from "../../composants/ApercuPdf";
import { Bouton } from "../../composants/Bouton";
import { MessageErreur, Section } from "../../composants/Messages";
import { dateCivile, francs, moisCivil, t } from "../../i18n";
import type { AbsentDuJour, BlocPaie, BlocPersonnel, BlocTresorerie, Cycle, Effectifs, Finances, Scolarite } from "../../types/gesco";
import { libelleTranche } from "../finance/Recouvrement";
import { NATURES } from "../rh/Engagements";
import { CYCLES } from "../scolarite/Cycles";

/** Un chiffre clé : sa valeur en grand, son libellé, et une précision. */
function Chiffre({ libelle, valeur, detail, alerte = false }: { libelle: string; valeur: string; detail?: string; alerte?: boolean }) {
  return (
    <div className="flex min-w-40 flex-1 flex-col gap-1 rounded border border-muet/30 bg-surface p-3">
      <span className="text-sm text-muet">{libelle}</span>
      <span className={`text-2xl font-semibold ${alerte ? "text-ocre" : "text-encre"}`}>{valeur}</span>
      {detail === undefined ? null : <span className="text-xs text-muet">{detail}</span>}
    </div>
  );
}

function Rangee({ children }: { children: ReactNode }) {
  return <div className="flex flex-wrap gap-3">{children}</div>;
}

/** Tableau de bord : effectifs, finances et scolarité de l'année en cours, et personnel du jour,
 *  selon les droits du compte. */
export function Accueil({ nom }: { nom: string }) {
  const tableau = useQuery({ queryKey: ["tableau-de-bord"], queryFn: () => appeler("tableau_de_bord_lire", null) });
  const d = tableau.data;
  return (
    <div className="flex max-w-5xl flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("accueil.titre")}</h1>
      <p className="text-sm text-encre">
        {t("accueil.bienvenue", { nom })}
        {d === undefined ? "" : ` ${t("tableau.jour", { jour: dateCivile(d.jour) })}`}
      </p>
      <MessageErreur erreur={tableau.error} />
      {d !== undefined && d.annee === null ? <p className="text-sm text-muet">{t("tableau.sans_annee")}</p> : null}
      {d?.effectifs === undefined || d.effectifs === null ? null : <BlocEffectifs e={d.effectifs} />}
      {d?.finances === undefined || d.finances === null ? null : <BlocFinances f={d.finances} />}
      {d?.scolarite === undefined || d.scolarite === null ? null : <BlocScolarite s={d.scolarite} />}
      {d?.personnel === undefined || d.personnel === null ? null : <BlocDuPersonnel p={d.personnel} />}
      {d?.paie === undefined || d.paie === null ? null : <BlocDeLaPaie p={d.paie} />}
      {d?.tresorerie === undefined || d.tresorerie === null ? null : <BlocDeLaTresorerie b={d.tresorerie} />}
    </div>
  );
}

function BlocEffectifs({ e }: { e: Effectifs }) {
  return (
    <Section titre={t("tableau.effectifs")}>
      <Rangee>
        <Chiffre libelle={t("tableau.eleves_inscrits")} valeur={String(e.total)} detail={t("tableau.filles_garcons", { filles: String(e.filles), garcons: String(e.garcons) })} />
        {e.par_cycle.map((c) => (
          <Chiffre key={c.cycle} libelle={t(CYCLES[c.cycle as Cycle])} valeur={String(c.eleves)} />
        ))}
        <Chiffre libelle={t("tableau.a_valider")} valeur={String(e.brouillons)} alerte={e.brouillons > 0} detail={t("tableau.a_valider_detail")} />
      </Rangee>
      {e.par_classe.length === 0 ? null : (
        <table className="w-full max-w-3xl text-left text-sm">
          <thead>
            <tr className="text-muet">
              <th className="px-2 py-1 font-normal">{t("tableau.classe")}</th>
              <th className="px-2 py-1 font-normal">{t("tableau.niveau")}</th>
              <th className="px-2 py-1 font-normal">{t("tableau.eleves")}</th>
              <th className="px-2 py-1 font-normal">{t("tableau.filles")}</th>
              <th className="px-2 py-1 font-normal">{t("tableau.garcons")}</th>
            </tr>
          </thead>
          <tbody>
            {e.par_classe.map((c) => (
              <tr key={`${c.niveau}-${c.classe}`} className="border-b border-muet/30">
                <td className="px-2 py-1 text-encre">{c.classe}</td>
                <td className="px-2 py-1 text-muet">{c.niveau}</td>
                <td className="px-2 py-1 text-encre">{c.eleves}</td>
                <td className="px-2 py-1 text-encre">{c.filles}</td>
                <td className="px-2 py-1 text-encre">{c.garcons}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </Section>
  );
}

function BlocFinances({ f }: { f: Finances }) {
  return (
    <Section titre={t("tableau.finances")}>
      <Rangee>
        <Chiffre libelle={t("tableau.encaisse_jour")} valeur={francs(f.encaisse_jour)} detail={t("tableau.recus", { nombre: String(f.recus_jour) })} />
        <Chiffre libelle={t("tableau.encaisse_mois")} valeur={francs(f.encaisse_mois)} />
        <Chiffre libelle={t("tableau.reste_echu")} valeur={francs(f.reste_echu)} alerte={f.reste_echu > 0} detail={t("tableau.eleves_en_retard", { nombre: String(f.eleves_en_retard) })} />
      </Rangee>
      {f.par_tranche.every((x) => x.eleves === 0) ? null : (
        <Rangee>
          {f.par_tranche.map((x) => (
            <Chiffre key={x.tranche} libelle={libelleTranche(x)} valeur={francs(x.echu)} detail={t("tableau.eleves_en_retard", { nombre: String(x.eleves) })} />
          ))}
        </Rangee>
      )}
    </Section>
  );
}

function BlocScolarite({ s }: { s: Scolarite }) {
  return (
    <Section titre={t("tableau.scolarite")}>
      <Rangee>
        <Chiffre libelle={t("tableau.absences_jour")} valeur={String(s.absences_jour)} detail={t("tableau.retards_jour", { nombre: String(s.retards_jour) })} />
        <Chiffre
          libelle={t("tableau.releves")}
          valeur={`${s.classes_relevees} / ${s.classes}`}
          alerte={s.classes_relevees < s.classes}
          detail={t("tableau.releves_detail")}
        />
        <Chiffre libelle={t("tableau.evaluations_en_saisie")} valeur={String(s.evaluations_en_saisie)} />
        <Chiffre libelle={t("tableau.bulletins_a_valider")} valeur={String(s.bulletins_a_valider)} alerte={s.bulletins_a_valider > 0} />
        <Chiffre libelle={t("tableau.bulletins_a_publier")} valeur={String(s.bulletins_a_publier)} alerte={s.bulletins_a_publier > 0} />
        <Chiffre libelle={t("tableau.exclusions")} valeur={String(s.exclusions_en_cours)} />
      </Rangee>
    </Section>
  );
}

function absence(a: AbsentDuJour): string {
  if (a.nature === "retard") return t("conges.retard_de", { minutes: a.minutes ?? 0 });
  return t(a.nature === "journee" ? "conges.nature.journee" : "conges.nature.demi_journee");
}

/** Une liste courte sous un chiffre : un nom par ligne, et son détail. */
function Noms({ titre, lignes }: { titre: string; lignes: readonly { cle: string; nom: string; detail: string }[] }) {
  if (lignes.length === 0) return null;
  return (
    <div className="flex min-w-60 flex-1 flex-col gap-1">
      <span className="text-sm text-muet">{titre}</span>
      <ul className="flex flex-col gap-1 text-sm">
        {lignes.map((l) => (
          <li key={l.cle} className="text-encre">
            <span className="font-medium">{l.nom}</span>
            <span className="text-muet">{` · ${l.detail}`}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}

/** La paie (SPEC_PHASE_7 §10) : celle du mois écoulé et les salaires encore dus. */
function BlocDeLaPaie({ p }: { p: BlocPaie }) {
  const etat = p.statut === null ? t("tableau.paie_a_faire") : p.statut === "validee" ? t("tableau.paie_validee") : t("tableau.paie_calculee");
  return (
    <Section titre={t("tableau.paie")}>
      <Rangee>
        <Chiffre libelle={t("tableau.paie_du_mois", { mois: moisCivil(p.mois) })} valeur={etat} alerte={p.a_valider} />
        <Chiffre libelle={t("tableau.bulletins_payes")} valeur={`${p.payes} / ${p.bulletins}`} />
        <Chiffre libelle={t("tableau.salaires_dus")} valeur={francs(p.reste_du)} alerte={p.reste_du > 0} />
      </Rangee>
    </Section>
  );
}

/** La trésorerie (SPEC_PHASE_8) : le solde de chaque compte, les dépenses du mois, ce qui
 *  attend l'accord, ce qui est dû aux fournisseurs. */
function BlocDeLaTresorerie({ b }: { b: BlocTresorerie }) {
  return (
    <Section titre={t("tableau.tresorerie")}>
      <Rangee>
        {b.soldes.map((s) => (
          <Chiffre key={s.compte} libelle={s.compte} valeur={francs(s.solde)} alerte={s.solde < 0} />
        ))}
      </Rangee>
      <Rangee>
        <Chiffre libelle={t("tableau.depenses_du_mois")} valeur={francs(b.depenses_du_mois)} />
        <Chiffre libelle={t("tableau.depenses_a_approuver")} valeur={String(b.a_approuver)} alerte={b.a_approuver > 0} />
        <Chiffre libelle={t("tableau.du_fournisseurs")} valeur={francs(b.du_fournisseurs)} detail={b.factures_echues > 0 ? t("tableau.factures_echues", { nombre: String(b.factures_echues) }) : undefined} alerte={b.factures_echues > 0} />
        <Chiffre libelle={t("tableau.budget")} valeur={b.budget_depasse > 0 ? t("tableau.budget_depasse", { nombre: String(b.budget_depasse) }) : t("tableau.budget_tenu")} alerte={b.budget_depasse > 0} />
      </Rangee>
    </Section>
  );
}

/** Personnel du jour (SPEC_PHASE_6 §8) : qui est là, ce qui attend une décision. */
function BlocDuPersonnel({ p }: { p: BlocPersonnel }) {
  return (
    <Section titre={t("tableau.personnel")}>
      <Rangee>
        <Chiffre
          libelle={t("tableau.presents")}
          valeur={String(p.presents)}
          detail={p.par_categorie.map((c) => `${c.categorie} : ${c.membres}`).join(" · ")}
        />
        <Chiffre
          libelle={t("tableau.vacataires")}
          valeur={String(p.vacataires_du_mois)}
          detail={t("tableau.heures_a_viser", { nombre: p.heures_a_viser, mois: moisCivil(p.mois) })}
        />
        {p.heures_a_viser_mois_precedent === 0 ? null : (
          <Chiffre libelle={t("tableau.heures_mois_precedent", { mois: moisCivil(p.mois_precedent) })} valeur={String(p.heures_a_viser_mois_precedent)} alerte />
        )}
        <Chiffre libelle={t("tableau.absences_personnel")} valeur={String(p.absents.length)} />
        <Chiffre libelle={t("tableau.conges_en_attente")} valeur={String(p.conges_en_attente)} alerte={p.conges_en_attente > 0} />
        <Chiffre libelle={t("tableau.en_conge")} valeur={String(p.en_conge.length)} />
      </Rangee>
      <Rangee>
        <Noms
          titre={t("tableau.echeances")}
          lignes={p.echeances.map((e, i) => ({ cle: `e${i}`, nom: e.membre, detail: t("tableau.echeance", { nature: t(NATURES[e.nature]), fin: dateCivile(e.fin) }) }))}
        />
        <Noms titre={t("tableau.absences_personnel")} lignes={p.absents.map((a, i) => ({ cle: `a${i}`, nom: a.membre, detail: absence(a) }))} />
        <Noms
          titre={t("tableau.en_conge")}
          lignes={p.en_conge.map((c, i) => ({ cle: `c${i}`, nom: c.membre, detail: t("tableau.en_conge_jusqua", { type: c.type_libelle, fin: dateCivile(c.fin) }) }))}
        />
      </Rangee>
    </Section>
  );
}

/** Document d'essai : vérifie que l'impression fonctionne sur ce poste. Rarement utile : il est
 *  rangé en bas de l'écran « Établissement ». */
export function DocumentEssai() {
  const essai = useMutation({ mutationFn: () => appeler("document_essai_produire", null) });
  return (
    <Section titre={t("accueil.document_essai")}>
      <p className="text-sm text-muet">{t("accueil.document_essai_aide")}</p>
      <div>
        <Bouton variante="secondaire" onClick={() => essai.mutate()} disabled={essai.isPending}>{t("accueil.document_essai")}</Bouton>
      </div>
      <MessageErreur erreur={essai.error} />
      {essai.data === undefined ? null : (
        <>
          <p className="font-condensee text-sm text-encre">
            {t("accueil.document_produit", {
              modele: essai.data.document.modele_id,
              version: essai.data.document.modele_version,
              empreinte: essai.data.document.empreinte_sha256.slice(0, 16),
            })}
          </p>
          <ApercuPdf base64={essai.data.pdf} />
        </>
      )}
    </Section>
  );
}
