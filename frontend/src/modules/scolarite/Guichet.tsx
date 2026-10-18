import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { Bouton } from "../../composants/Bouton";
import { Champ } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { type Cle, dateCivile, dateHeure, francs, t } from "../../i18n";
import type { Guichet as FamilleGuichet } from "../../types/gesco";
import { FormulaireEncaissement } from "../finance/Encaissement";
import { telephoneLisible } from "./Identite";
import { FormulaireRemise } from "./Remises";

const DEMI_JOURNEES: Record<"matin" | "apres_midi", Cle> = {
  matin: "guichet.matin",
  apres_midi: "guichet.apres_midi",
};

/** Le guichet (SPEC_PHASE_5 §8) : le parent au comptoir est le moment où tout converge. Une
 *  recherche, et toute la famille est là : ce que doit chaque enfant, les bulletins à remettre ou
 *  à signer, les convocations, les absences récentes, les messages ; tout se règle sans changer
 *  d'écran. */
export function Guichet() {
  const [texte, setTexte] = useState("");
  const [eleveId, setEleveId] = useState<string | null>(null);
  const trouves = useQuery({
    queryKey: ["guichet-eleves", texte],
    queryFn: () => appeler("eleves_rechercher", { texte }),
    enabled: texte.trim().length >= 2,
  });
  const famille = useQuery({
    queryKey: ["guichet", eleveId],
    queryFn: () => appeler("guichet_lire", { eleve_id: eleveId ?? "" }),
    enabled: eleveId !== null,
  });
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("guichet.titre")}</h1>
      <Section titre={t("guichet.rechercher")}>
        <Champ
          id="guichet-recherche"
          type="search"
          autoFocus
          libelle={t("guichet.eleve")}
          aide={t("guichet.rechercher_aide")}
          value={texte}
          onChange={(e) => {
            setTexte(e.target.value);
            setEleveId(null);
          }}
        />
        {eleveId === null ? (
          <ul className="flex flex-col gap-1">
            {(trouves.data ?? []).slice(0, 8).map((e) => (
              <li key={e.id}>
                <button type="button" className="text-sm text-primaire underline" onClick={() => setEleveId(e.id)}>
                  {`${e.prenoms} ${e.nom} · ${e.matricule_interne}`}
                </button>
              </li>
            ))}
          </ul>
        ) : null}
        <MessageErreur erreur={trouves.error ?? famille.error} />
      </Section>
      {famille.data === undefined ? null : <Famille f={famille.data} />}
    </div>
  );
}

function Famille({ f }: { f: FamilleGuichet }) {
  const client = useQueryClient();
  const [encaisser, setEncaisser] = useState<string | null>(null);
  const [remettre, setRemettre] = useState<string | null>(null);
  const relire = () => void client.invalidateQueries({ queryKey: ["guichet", f.eleve_id] });
  const justification = useMutation({
    mutationFn: (a: { inscription_id: string; date: string; demi_journee: "matin" | "apres_midi" }) =>
      appeler("absences_justifier", { absences: [a], motif: t("guichet.motif_guichet"), justificatif: t("guichet.justificatif_guichet") }),
    onSuccess: relire,
  });
  const venue = useMutation({ mutationFn: (id: string) => appeler("convocation_clore", { id, honoree: true }), onSuccess: relire });
  const document = f.documents.find((d) => d.remise.id === remettre);
  return (
    <>
      <Section titre={t("guichet.famille")}>
        <ul className="flex flex-col gap-1 text-sm">
          {f.responsables.map((r) => (
            <li key={`${r.nom}-${r.lien}`}>
              <span className="font-medium text-encre">{r.nom}</span>
              <span className="text-muet">{` · ${r.telephones.map(telephoneLisible).join(" · ")}`}</span>
            </li>
          ))}
        </ul>
        <table className="w-full text-sm">
          <thead>
            <tr className="text-left text-muet">
              <th className="py-1">{t("guichet.colonne.enfant")}</th>
              <th>{t("guichet.colonne.classe")}</th>
              <th className="text-right">{t("guichet.colonne.echu")}</th>
              <th className="text-right">{t("guichet.colonne.reste")}</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {f.enfants.map((e) => (
              <tr key={e.eleve_id} className="border-t border-muet/20">
                <td className="py-1 text-encre">{`${e.nom} · ${e.matricule}`}</td>
                <td>{e.classe}</td>
                <td className={`text-right ${e.echu > 0 ? "text-rouge" : "text-vert"}`}>{francs(e.echu)}</td>
                <td className="text-right">{francs(e.reste)}</td>
                <td className="text-right">
                  {f.droits.encaisser && e.reste > 0 ? (
                    <Bouton variante="secondaire" onClick={() => setEncaisser(e.inscription_id)}>{t("guichet.encaisser")}</Bouton>
                  ) : null}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
        <p className="text-sm font-medium text-encre">{t("guichet.echu_famille", { montant: francs(f.echu_famille) })}</p>
        {encaisser === null ? null : (
          <FormulaireEncaissement
            key={encaisser}
            inscriptionId={encaisser}
            autreEleve={false}
            surTermine={() => {
              setEncaisser(null);
              relire();
            }}
          />
        )}
      </Section>

      <Section titre={t("guichet.documents")}>
        {f.documents.length === 0 ? <p className="text-sm text-muet">{t("guichet.aucun_document")}</p> : null}
        <ul className="flex flex-col gap-2">
          {f.documents.map((d) => (
            <li key={d.remise.id} className="flex flex-wrap items-center gap-3 text-sm">
              <span className="text-encre">{t("guichet.bulletin", { eleve: d.eleve, periode: d.periode })}</span>
              <span className={d.remise.statut === "a_remettre" ? "text-muet" : "text-ocre"}>
                {d.remise.statut === "remis"
                  ? t("guichet.a_signer")
                  : d.remise.statut === "non_retire"
                    ? t("guichet.non_retire")
                    : t("guichet.a_remettre", { jours: d.jours })}
              </span>
              {f.droits.remettre ? (
                <Bouton variante="secondaire" onClick={() => setRemettre(d.remise.id)}>
                  {d.remise.statut === "remis" ? t("guichet.faire_signer") : t("guichet.remettre")}
                </Bouton>
              ) : null}
            </li>
          ))}
        </ul>
        {document === undefined ? null : (
          <FormulaireRemise
            key={document.remise.id}
            ligne={document}
            surFait={() => {
              setRemettre(null);
              relire();
            }}
          />
        )}
      </Section>

      <Section titre={t("guichet.convocations")}>
        {f.convocations.length === 0 ? <p className="text-sm text-muet">{t("guichet.aucune_convocation")}</p> : null}
        <ul className="flex flex-col gap-2">
          {f.convocations.map((c) => (
            <li key={c.convocation.id} className="flex flex-wrap items-center gap-3 text-sm">
              <span className="text-encre">{t("guichet.convocation", { eleve: c.eleve, date: dateCivile(c.convocation.date), heure: c.convocation.heure, motif: c.convocation.motif })}</span>
              {c.convocation.accuse === "transmis_non_confirme" ? <span className="text-ocre">{t("guichet.non_confirmee")}</span> : null}
              {f.droits.convocations ? (
                <Bouton variante="secondaire" onClick={() => venue.mutate(c.convocation.id)} disabled={venue.isPending}>{t("guichet.venu")}</Bouton>
              ) : null}
            </li>
          ))}
        </ul>
      </Section>

      <Section titre={t("guichet.absences")}>
        {f.absences.length === 0 ? <p className="text-sm text-muet">{t("guichet.aucune_absence")}</p> : null}
        <ul className="flex flex-col gap-2">
          {f.absences.map((a) => (
            <li key={`${a.inscription_id}-${a.date}-${a.demi_journee}`} className="flex flex-wrap items-center gap-3 text-sm">
              <span className="text-encre">{`${a.eleve} · ${dateCivile(a.date)} · ${t(DEMI_JOURNEES[a.demi_journee])}`}</span>
              {f.droits.justifier ? (
                <Bouton
                  variante="secondaire"
                  onClick={() => justification.mutate({ inscription_id: a.inscription_id, date: a.date, demi_journee: a.demi_journee })}
                  disabled={justification.isPending}
                >
                  {t("guichet.justifier")}
                </Bouton>
              ) : null}
            </li>
          ))}
        </ul>
      </Section>

      <Section titre={t("guichet.messages")}>
        {f.messages.length === 0 ? <p className="text-sm text-muet">{t("guichet.aucun_message")}</p> : null}
        <ul className="flex flex-col gap-1 text-sm">
          {f.messages.map((m) => (
            <li key={m.message.id}>
              <span className="text-muet">{`${dateHeure(m.message.cree_le)} · `}</span>
              <span className="text-encre">{m.message.texte}</span>
            </li>
          ))}
        </ul>
      </Section>
      <MessageErreur erreur={justification.error ?? venue.error} />
    </>
  );
}
