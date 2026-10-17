import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { Bouton } from "../../composants/Bouton";
import { CaseACocher, Champ, Formulaire, Selection } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { type Cle, t } from "../../i18n";
import type { EtatJoignabilite, FicheEleve, Lien, MessagesResponsable, Qualites, ResponsableDeLEleve, SaisieResponsable, Sexe, TypeMessage } from "../../types/gesco";
import { ChampsIdentite, identiteVide, saisieDepuis, telephoneLisible } from "./Identite";

const LIENS: Record<Lien, Cle> = {
  pere: "responsables.lien.pere",
  mere: "responsables.lien.mere",
  tuteur: "responsables.lien.tuteur",
  autre: "responsables.lien.autre",
};

/** Sexe qu'impose le lien : le noyau l'applique de même (`Lien::sexe`). */
const SEXE_DU_LIEN: Record<Lien, Sexe | null> = { pere: "masculin", mere: "feminin", tuteur: null, autre: null };

const CASES = [
  ["est_responsable_legal", "responsables.legal"],
  ["est_payeur", "responsables.payeur"],
  ["recoit_notifications", "responsables.sms"],
  ["vient_chercher", "responsables.vient_chercher"],
] as const satisfies readonly (readonly [keyof Qualites, Cle])[];

function qualitesParDefaut(ordre: number): Qualites {
  return {
    lien: "pere",
    est_responsable_legal: true,
    est_payeur: true,
    recoit_notifications: true,
    vient_chercher: false,
    ordre_priorite: ordre,
  };
}

function ChampsQualites({ prefixe, valeur, surChangement }: { prefixe: string; valeur: Qualites; surChangement: (q: Qualites) => void }) {
  return (
    <div className="flex flex-col gap-2">
      <Selection
        id={`${prefixe}-lien`}
        libelle={t("responsables.lien")}
        valeur={valeur.lien}
        surChangement={(v) => {
          const lien = (Object.keys(LIENS) as Lien[]).find((l) => l === v);
          if (lien !== undefined) surChangement({ ...valeur, lien });
        }}
        options={(Object.keys(LIENS) as Lien[]).map((l) => ({ valeur: l, libelle: t(LIENS[l]) }))}
      />
      {CASES.map(([cle, libelle]) => (
        <CaseACocher key={cle} id={`${prefixe}-${cle}`} libelle={t(libelle)} coche={valeur[cle]} surChangement={(coche) => surChangement({ ...valeur, [cle]: coche })} />
      ))}
    </div>
  );
}

function resume(q: Qualites): string {
  return [t(LIENS[q.lien]), ...CASES.filter(([cle]) => q[cle]).map(([, libelle]) => t(libelle))].join(" · ");
}

/** Responsables d'un élève (SPEC_PHASE_2 §4.3) : un responsable déjà connu se retrouve par son
 *  téléphone ou son nom, et se lie sans ressaisie (§25.5). */
export function ResponsablesEleve({ fiche, modifiable }: { fiche: FicheEleve; modifiable: boolean }) {
  const [edite, setEdite] = useState<string | null>(null);
  return (
    <Section titre={t("responsables.titre")}>
      {fiche.responsables.length === 0 ? <p className="text-sm text-ocre">{t("responsables.aucun")}</p> : null}
      <ul className="flex flex-col gap-3">
        {fiche.responsables.map((r) => (
          <li key={r.responsable.id} className="rounded border border-muet/30 p-3">
            {edite === r.responsable.id ? (
              <EditionResponsable fiche={fiche} responsable={r} surTermine={() => setEdite(null)} />
            ) : (
              <div className="flex flex-wrap items-start gap-3">
                <div className="flex-1 text-sm">
                  <p className="font-medium text-encre">{`${r.identite.prenoms} ${r.identite.nom}`}</p>
                  <p className="text-muet">{resume(r.qualites)}</p>
                  <MessagesDuResponsable responsableId={r.responsable.id} telephones={r.identite.telephones} recoitNotifications={r.qualites.recoit_notifications} />
                </div>
                {modifiable ? <Bouton variante="secondaire" onClick={() => setEdite(r.responsable.id)}>{t("responsables.modifier")}</Bouton> : null}
              </div>
            )}
          </li>
        ))}
      </ul>
      {modifiable ? <AjoutResponsable fiche={fiche} /> : null}
    </Section>
  );
}

const TYPES_MESSAGE: Record<TypeMessage, Cle> = {
  bulletin: "messages.type.bulletin",
  impaye: "messages.type.impaye",
  absence: "messages.type.absence",
  convocation: "messages.type.convocation",
};

/** Un numéro pas encore vérifié n'affiche rien : c'est l'état de tous avant le premier envoi. */
const ETATS: Partial<Record<EtatJoignabilite, { libelle: Cle; couleur: string }>> = {
  joignable: { libelle: "messages.etat.joignable", couleur: "text-vert" },
  injoignable: { libelle: "messages.etat.injoignable", couleur: "text-rouge" },
  invalide: { libelle: "messages.etat.invalide", couleur: "text-rouge" },
};

/** Les numéros d'un responsable avec leur état, et les types de messages qu'il accepte. Un refus
 *  vaut pour tous ses enfants ; un numéro mort se remet à l'essai une fois corrigé. */
function MessagesDuResponsable({ responsableId, telephones, recoitNotifications }: { responsableId: string; telephones: string[]; recoitNotifications: boolean }) {
  const client = useQueryClient();
  const cle = ["responsable-messages", responsableId, telephones.join(",")];
  const messages = useQuery({ queryKey: cle, queryFn: () => appeler("responsable_messages_lire", { responsable_id: responsableId }) });
  const poser = (m: MessagesResponsable) => client.setQueryData(cle, m);
  const consentement = useMutation({
    mutationFn: ({ type, accepte }: { type: TypeMessage; accepte: boolean }) =>
      appeler("responsable_consentement_definir", { responsable_id: responsableId, type_message: type, accepte }),
    // La case suit le clic aussitôt ; un refus du noyau la remet comme elle était.
    onMutate: ({ type, accepte }) => {
      const avant = client.getQueryData<MessagesResponsable>(cle);
      if (avant !== undefined) {
        poser({ ...avant, consentements: avant.consentements.map((c) => (c.type_message === type ? { ...c, accepte } : c)) });
      }
      return avant;
    },
    onError: (_erreur, _variables, avant) => {
      if (avant !== undefined) poser(avant);
    },
    onSuccess: poser,
  });
  const essai = useMutation({
    mutationFn: (numero: string) => appeler("numero_remettre_a_l_essai", { responsable_id: responsableId, numero }),
    onSuccess: poser,
  });
  const m = messages.data;
  const etat = (numero: string) => m?.numeros.find((n) => n.numero === numero)?.etat;
  return (
    <>
      <ul className="flex flex-wrap gap-x-4 gap-y-1">
        {telephones.map((numero) => {
          const e = etat(numero);
          return (
            <li key={numero} className="flex flex-wrap items-center gap-2">
              <span className="font-condensee text-encre">{telephoneLisible(numero)}</span>
              {e === undefined || ETATS[e] === undefined ? null : <span className={ETATS[e].couleur}>{t(ETATS[e].libelle)}</span>}
              {m?.modifiable === true && (e === "injoignable" || e === "invalide") ? (
                <Bouton variante="secondaire" onClick={() => essai.mutate(numero)} disabled={essai.isPending}>{t("messages.remettre_a_l_essai")}</Bouton>
              ) : null}
            </li>
          );
        })}
      </ul>
      {m === undefined ? null : recoitNotifications ? (
        <fieldset className="mt-1 flex flex-wrap gap-x-4 gap-y-1">
          <legend className="text-muet">{t("messages.recoit")}</legend>
          {m.consentements.map((c) => (
            <CaseACocher
              key={c.type_message}
              id={`messages-${responsableId}-${c.type_message}`}
              libelle={t(TYPES_MESSAGE[c.type_message])}
              coche={c.accepte}
              desactive={!m.modifiable}
              surChangement={(accepte) => consentement.mutate({ type: c.type_message, accepte })}
            />
          ))}
        </fieldset>
      ) : (
        <p className="text-muet">{t("messages.ne_recoit_rien")}</p>
      )}
      <MessageErreur erreur={messages.error ?? consentement.error ?? essai.error} />
    </>
  );
}

function useFiche(id: string) {
  const client = useQueryClient();
  return (fiche: FicheEleve) => client.setQueryData(["eleve", id], fiche);
}

function AjoutResponsable({ fiche }: { fiche: FicheEleve }) {
  const poser = useFiche(fiche.eleve.id);
  const [recherche, setRecherche] = useState("");
  const [saisie, setSaisie] = useState<SaisieResponsable>({ identite: { ...identiteVide(), nationalite: "" }, profession: "", employeur: "" });
  const [qualites, setQualites] = useState(() => qualitesParDefaut(fiche.responsables.length + 1));
  const trouves = useQuery({
    queryKey: ["responsables", recherche],
    queryFn: () => appeler("responsables_rechercher", { texte: recherche }),
    enabled: recherche.trim().length >= 2,
  });
  const lies = new Set(fiche.responsables.map((r) => r.responsable.id));
  const rattachement = useMutation({
    mutationFn: (responsableId: string | null) =>
      appeler("responsable_rattacher", {
        eleve_id: fiche.eleve.id,
        responsable_id: responsableId,
        saisie: responsableId === null ? saisie : null,
        qualites,
      }),
    onSuccess: (nouvelle) => {
      poser(nouvelle);
      setRecherche("");
      setSaisie({ identite: { ...identiteVide(), nationalite: "" }, profession: "", employeur: "" });
      setQualites(qualitesParDefaut(nouvelle.responsables.length + 1));
    },
  });
  return (
    <div className="flex flex-col gap-3 border-t border-muet/30 pt-3">
      <h3 className="text-sm font-semibold text-encre">{t("responsables.ajouter")}</h3>
      <ChampsQualites prefixe="ajout" valeur={qualites} surChangement={setQualites} />
      <Champ id="responsable-recherche" type="search" libelle={t("responsables.retrouver")} aide={t("responsables.retrouver_aide")} value={recherche} onChange={(e) => setRecherche(e.target.value)} />
      {(trouves.data ?? [])
        .filter((r) => !lies.has(r.responsable.id))
        .map((r) => (
          <div key={r.responsable.id} className="flex flex-wrap items-center gap-3 rounded bg-fond p-2 text-sm">
            <div className="flex-1">
              <p className="font-medium text-encre">{`${r.identite.prenoms} ${r.identite.nom}`}</p>
              <p className="font-condensee text-muet">{r.identite.telephones.map(telephoneLisible).join(" · ")}</p>
              {r.enfants.length === 0 ? null : <p className="text-muet">{t("responsables.deja_parent_de", { enfants: r.enfants.map((e) => e.nom).join(", ") })}</p>}
            </div>
            <Bouton variante="secondaire" onClick={() => rattachement.mutate(r.responsable.id)} disabled={rattachement.isPending}>{t("responsables.lier")}</Bouton>
          </div>
        ))}
      <Formulaire surEnvoi={() => rattachement.mutate(null)}>
        <p className="text-sm text-muet">{t("responsables.nouveau_aide")}</p>
        <ChampsIdentite prefixe="nouveau-responsable" valeur={saisie.identite} surChangement={(identite) => setSaisie({ ...saisie, identite })} eleve={false} sexeImpose={SEXE_DU_LIEN[qualites.lien]} />
        <Champ id="nouveau-responsable-profession" libelle={t("responsables.profession")} value={saisie.profession} onChange={(e) => setSaisie({ ...saisie, profession: e.target.value })} />
        <MessageErreur erreur={rattachement.error} />
        <div><Bouton type="submit" disabled={rattachement.isPending}>{t("responsables.creer_et_lier")}</Bouton></div>
      </Formulaire>
    </div>
  );
}

function EditionResponsable({ fiche, responsable, surTermine }: { fiche: FicheEleve; responsable: ResponsableDeLEleve; surTermine: () => void }) {
  const poser = useFiche(fiche.eleve.id);
  const [saisie, setSaisie] = useState<SaisieResponsable>({
    identite: saisieDepuis(responsable.identite),
    profession: responsable.responsable.profession,
    employeur: responsable.responsable.employeur,
  });
  const [qualites, setQualites] = useState(responsable.qualites);
  const termine = (nouvelle: FicheEleve) => {
    poser(nouvelle);
    surTermine();
  };
  const modification = useMutation({
    mutationFn: () => appeler("responsable_modifier", { eleve_id: fiche.eleve.id, responsable_id: responsable.responsable.id, saisie, qualites }),
    onSuccess: termine,
  });
  const retrait = useMutation({
    mutationFn: () => appeler("responsable_detacher", { eleve_id: fiche.eleve.id, responsable_id: responsable.responsable.id }),
    onSuccess: termine,
  });
  return (
    <Formulaire surEnvoi={() => modification.mutate()}>
      <ChampsIdentite prefixe="responsable" valeur={saisie.identite} surChangement={(identite) => setSaisie({ ...saisie, identite })} eleve={false} sexeImpose={SEXE_DU_LIEN[qualites.lien]} />
      <Champ id="responsable-profession" libelle={t("responsables.profession")} value={saisie.profession} onChange={(e) => setSaisie({ ...saisie, profession: e.target.value })} />
      <ChampsQualites prefixe="responsable" valeur={qualites} surChangement={setQualites} />
      <MessageErreur erreur={modification.error ?? retrait.error} />
      <div className="flex flex-wrap gap-2">
        <Bouton type="submit" disabled={modification.isPending}>{t("commun.enregistrer")}</Bouton>
        <Bouton variante="secondaire" onClick={surTermine}>{t("commun.annuler")}</Bouton>
        <Bouton variante="danger" onClick={() => retrait.mutate()} disabled={retrait.isPending}>{t("responsables.detacher")}</Bouton>
      </div>
    </Formulaire>
  );
}
