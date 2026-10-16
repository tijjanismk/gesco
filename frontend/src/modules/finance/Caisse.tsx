import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { commandeAutorisee } from "../../app/navigation";
import { ApercuPdf } from "../../composants/ApercuPdf";
import { Bouton } from "../../composants/Bouton";
import { Champ, Formulaire } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { Tableau } from "../../composants/Tableau";
import { type Cle, dateHeure, francs, t } from "../../i18n";
import type { NatureMouvement, SessionAffichee, StatutSession } from "../../types/gesco";
import { Encaissement, EncaissementFamille, MODES } from "./Encaissement";

const NATURES_MOUVEMENT: Record<NatureMouvement, Cle> = {
  sortie_regularisation: "caisse.regularisation",
  sortie_salaire: "caisse.sortie_salaire",
  sortie_avance: "caisse.sortie_avance",
  sortie_depense: "caisse.sortie_depense",
  sortie_transfert: "caisse.sortie_transfert",
  entree_regularisation: "caisse.entree_regularisation",
  entree_transfert: "caisse.entree_transfert",
};


const STATUTS: Record<StatutSession, Cle> = {
  ouverte: "caisse.statut.ouverte",
  cloturee: "caisse.statut.cloturee",
  rapprochee: "caisse.statut.rapprochee",
};

function lireFrancs(texte: string): number {
  const chiffres = texte.replace(/\D/g, "");
  return chiffres === "" ? 0 : Number.parseInt(chiffres, 10);
}

/** Ma caisse (SPEC_PHASE_3 §6) : ouverture avec le fonds, totaux, clôture avec le total compté. */
export function Caisse() {
  const client = useQueryClient();
  const courante = useQuery({ queryKey: ["caisse"], queryFn: () => appeler("caisse_session_courante", null) });
  const [derniere, setDerniere] = useState<SessionAffichee | null>(null);
  const rafraichir = () => {
    void client.invalidateQueries({ queryKey: ["caisse"] });
    void client.invalidateQueries({ queryKey: ["sessions-caisse"] });
  };
  return (
    <div className="flex max-w-3xl flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("caisse.titre")}</h1>
      <MessageErreur erreur={courante.error} />
      {courante.isSuccess && courante.data === null ? (
        <Ouverture
          surOuverte={() => {
            setDerniere(null);
            rafraichir();
          }}
        />
      ) : null}
      {courante.data === null || courante.data === undefined ? null : <Encaisser />}
      {courante.data === null || courante.data === undefined ? null : (
        <SessionOuverte
          affichee={courante.data}
          surCloturee={(s) => {
            setDerniere(s);
            rafraichir();
          }}
        />
      )}
      {derniere === null ? null : <EtatImprime affichee={derniere} titre={t("caisse.cloturee_ok")} />}
    </div>
  );
}

/** Encaisser : un élève, ou plusieurs enfants d'un même parent (SPEC_PHASE_3 §7). */
function Encaisser() {
  const [famille, setFamille] = useState(false);
  return (
    <Section titre={t("encaissement.titre")}>
      <div className="flex flex-wrap gap-2">
        <Bouton variante={famille ? "secondaire" : "principal"} onClick={() => setFamille(false)}>{t("encaissement.un_eleve")}</Bouton>
        <Bouton variante={famille ? "principal" : "secondaire"} onClick={() => setFamille(true)}>{t("encaissement.plusieurs_enfants")}</Bouton>
      </div>
      {famille ? <EncaissementFamille /> : <Encaissement />}
    </Section>
  );
}

function Ouverture({ surOuverte }: { surOuverte: () => void }) {
  const [fonds, setFonds] = useState("");
  const ouverture = useMutation({ mutationFn: () => appeler("caisse_ouvrir", { fonds_initial: lireFrancs(fonds) }), onSuccess: surOuverte });
  return (
    <Section titre={t("caisse.ouvrir")}>
      <p className="text-sm text-muet">{t("caisse.ouvrir_aide")}</p>
      <Formulaire surEnvoi={() => ouverture.mutate()}>
        <div className="w-60">
          <Champ id="caisse-fonds" libelle={t("caisse.fonds")} inputMode="numeric" autoFocus value={fonds} onChange={(e) => setFonds(e.target.value)} />
        </div>
        <MessageErreur erreur={ouverture.error} />
        <div><Bouton type="submit" disabled={ouverture.isPending}>{t("caisse.ouvrir_bouton")}</Bouton></div>
      </Formulaire>
    </Section>
  );
}

function Totaux({ affichee }: { affichee: SessionAffichee }) {
  const { totaux } = affichee;
  return (
    <dl className="grid grid-cols-2 gap-x-6 gap-y-1 text-sm">
      <dt className="text-muet">{t("caisse.fonds")}</dt>
      <dd className="text-encre">{francs(totaux.fonds)}</dd>
      <dt className="text-muet">{t("caisse.entrees_especes")}</dt>
      <dd className="text-encre">{francs(totaux.entrees_especes)}</dd>
      <dt className="text-muet">{t("caisse.sorties_especes")}</dt>
      <dd className="text-encre">{francs(totaux.sorties_especes)}</dd>
      <dt className="font-medium text-encre">{t("caisse.theorique")}</dt>
      <dd className="font-medium text-encre">{francs(totaux.theorique)}</dd>
      {totaux.par_mode
        .filter((m) => m.mode !== "especes")
        .map((m) => (
          <div key={m.mode} className="contents">
            <dt className="text-muet">{t(MODES[m.mode])}</dt>
            <dd className="text-encre">{francs(m.montant)}</dd>
          </div>
        ))}
      <dt className="text-muet">{t("caisse.recus")}</dt>
      <dd className="text-encre">{String(totaux.nombre_recus)}</dd>
      {totaux.regularisations.map((m) => (
        <p key={m.id} className="col-span-2 text-ocre">
          {t(NATURES_MOUVEMENT[m.nature], { motif: m.motif, mode: t(MODES[m.mode]), montant: francs(m.montant) })}
        </p>
      ))}
    </dl>
  );
}

function SessionOuverte({ affichee, surCloturee }: { affichee: SessionAffichee; surCloturee: (s: SessionAffichee) => void }) {
  const [compte, setCompte] = useState("");
  const [motif, setMotif] = useState("");
  const cloture = useMutation({
    mutationFn: () => appeler("caisse_cloturer", { total_compte: lireFrancs(compte), motif_ecart: motif }),
    onSuccess: surCloturee,
  });
  const ecart = compte === "" ? null : lireFrancs(compte) - affichee.totaux.theorique;
  return (
    <Section titre={t("caisse.ouverte_depuis", { date: dateHeure(affichee.session.ouverte_le) })}>
      <Totaux affichee={affichee} />
      <p className="text-sm text-muet">{t("caisse.cloturer_aide")}</p>
      <Formulaire surEnvoi={() => cloture.mutate()}>
        <div className="flex flex-wrap items-end gap-3">
          <div className="w-60">
            <Champ id="caisse-compte" libelle={t("caisse.compte")} inputMode="numeric" value={compte} onChange={(e) => setCompte(e.target.value)} />
          </div>
          {ecart === null ? null : (
            <p role="status" className={`text-sm ${ecart === 0 ? "text-encre" : "text-rouge"}`}>
              {ecart === 0 ? t("caisse.sans_ecart") : t("caisse.ecart", { ecart: francs(ecart) })}
            </p>
          )}
        </div>
        {ecart !== null && ecart !== 0 ? (
          <Champ id="caisse-motif" libelle={t("caisse.motif_ecart")} value={motif} onChange={(e) => setMotif(e.target.value)} />
        ) : null}
        <MessageErreur erreur={cloture.error} />
        <div><Bouton type="submit" disabled={cloture.isPending || compte === ""}>{t("caisse.cloturer")}</Bouton></div>
      </Formulaire>
    </Section>
  );
}

function EtatImprime({ affichee, titre }: { affichee: SessionAffichee; titre: string }) {
  const production = useMutation({ mutationFn: () => appeler("etat_caisse_produire", { id: affichee.session.id }) });
  return (
    <Section titre={titre}>
      <p className="text-sm text-encre">
        {t("caisse.resume", {
          theorique: francs(affichee.session.total_theorique ?? 0),
          compte: francs(affichee.session.total_compte ?? 0),
          ecart: francs(affichee.session.ecart ?? 0),
        })}
      </p>
      <div><Bouton variante="secondaire" onClick={() => production.mutate()} disabled={production.isPending}>{t("caisse.imprimer_etat")}</Bouton></div>
      <MessageErreur erreur={production.error} />
      {production.data === undefined ? null : <ApercuPdf base64={production.data.pdf} />}
    </Section>
  );
}

/** Sessions de caisse de l'école, pour le visa de la direction ou du comptable (§6.1). */
export function SessionsCaisse() {
  const client = useQueryClient();
  const session = useQuery({ queryKey: ["session"], queryFn: () => appeler("session_courante", null) });
  const sessions = useQuery({ queryKey: ["sessions-caisse"], queryFn: () => appeler("caisse_sessions_lister", null) });
  const [choisie, setChoisie] = useState<string | null>(null);
  const [commentaire, setCommentaire] = useState("");
  const fiche = sessions.data?.find((s) => s.session.id === choisie);
  const visa = useMutation({
    mutationFn: () => appeler("caisse_rapprocher", { id: choisie ?? "", commentaire }),
    onSuccess: () => {
      setCommentaire("");
      void client.invalidateQueries({ queryKey: ["sessions-caisse"] });
    },
  });
  const peutViser = commandeAutorisee("caisse_rapprocher", session.data?.permissions ?? []);
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("caisse.sessions")}</h1>
      <MessageErreur erreur={sessions.error} />
      <div className="flex flex-wrap items-start gap-4">
        <div className="min-w-0 flex-1">
          <Tableau
            nomExport="sessions-caisse"
            titre={t("caisse.sessions")}
            lignes={sessions.data ?? []}
            cle={(s) => s.session.id}
            surSelection={(s) => setChoisie(s.session.id)}
            colonnes={[
              { titre: t("caisse.caissier"), valeur: (s) => s.caissier },
              { titre: t("caisse.ouverte_le"), valeur: (s) => dateHeure(s.session.ouverte_le) },
              { titre: t("caisse.cloturee_le"), valeur: (s) => (s.session.cloturee_le === null ? "" : dateHeure(s.session.cloturee_le)) },
              { titre: t("caisse.etat"), valeur: (s) => t(STATUTS[s.session.statut]) },
              { titre: t("caisse.theorique"), valeur: (s) => (s.session.total_theorique === null ? "" : francs(s.session.total_theorique)) },
              { titre: t("caisse.compte"), valeur: (s) => (s.session.total_compte === null ? "" : francs(s.session.total_compte)) },
              { titre: t("caisse.ecart_colonne"), valeur: (s) => (s.session.ecart === null ? "" : francs(s.session.ecart)) },
            ]}
          />
        </div>
        {fiche === undefined ? null : (
          <aside className="flex w-96 flex-col gap-3">
            <Section titre={`${fiche.caissier} · ${t(STATUTS[fiche.session.statut])}`}>
              <Totaux affichee={fiche} />
              {fiche.session.motif_ecart === "" ? null : <p className="text-sm text-encre">{t("caisse.motif_affiche", { motif: fiche.session.motif_ecart })}</p>}
              {fiche.vise_par === null ? null : <p className="text-sm text-muet">{t("caisse.vise", { nom: fiche.vise_par, commentaire: fiche.session.commentaire_visa })}</p>}
              {peutViser && fiche.session.statut === "cloturee" ? (
                <div className="flex flex-col gap-2">
                  <Champ id="caisse-visa" libelle={t("caisse.commentaire_visa")} value={commentaire} onChange={(e) => setCommentaire(e.target.value)} />
                  <div><Bouton onClick={() => visa.mutate()} disabled={visa.isPending}>{t("caisse.viser")}</Bouton></div>
                </div>
              ) : null}
              <MessageErreur erreur={visa.error} />
            </Section>
            {fiche.session.statut === "ouverte" ? null : <EtatImprime key={fiche.session.id} affichee={fiche} titre={t("caisse.etat_titre")} />}
          </aside>
        )}
      </div>
    </div>
  );
}
