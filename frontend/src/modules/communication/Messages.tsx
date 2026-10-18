import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";

import { appeler } from "../../api";
import { Bouton } from "../../composants/Bouton";
import { Champ, Formulaire, ZoneTexte } from "../../composants/Champ";
import { MessageErreur, MessageEnregistre, Section } from "../../composants/Messages";
import { Tableau } from "../../composants/Tableau";
import { type Cle, dateHeure, t } from "../../i18n";
import type { ApercuSms, ErreurEnvoi, EtatMessage, FileMessages, MessageAffiche, ModeleSms, NatureMessage } from "../../types/gesco";
import { telephoneLisible } from "../scolarite/Identite";

const ETATS: Record<EtatMessage, Cle> = {
  a_decider: "messages.etat_message.a_decider",
  en_attente: "messages.etat_message.en_attente",
  envoye: "messages.etat_message.envoye",
  remis: "messages.etat_message.remis",
  echoue: "messages.etat_message.echoue",
  annule: "messages.etat_message.annule",
  expire: "messages.etat_message.expire",
};

const ORDRE_ETATS: readonly EtatMessage[] = ["a_decider", "en_attente", "envoye", "remis", "echoue", "annule", "expire"];

const NATURES: Record<NatureMessage, Cle> = {
  bulletin: "messages.nature.bulletin",
  impaye: "messages.nature.impaye",
  absence: "messages.nature.absence",
  convocation: "messages.nature.convocation",
  sanction: "messages.nature.sanction",
  essai: "messages.nature.essai",
};

const ERREURS: Record<ErreurEnvoi, Cle> = {
  reseau: "messages.erreur.reseau",
  credit: "messages.erreur.credit",
  numero_refuse: "messages.erreur.numero_refuse",
  numero_injoignable: "messages.erreur.numero_injoignable",
  non_remis: "messages.erreur.non_remis",
};

function etatAffiche(m: MessageAffiche): string {
  const etat = t(ETATS[m.message.etat]);
  return m.message.erreur === null ? etat : `${etat} : ${t(ERREURS[m.message.erreur])}`;
}

/** Messages aux familles (SPEC_PHASE_5 §3) : la file des SMS, ce qui attend une décision, ce qui
 *  est parti ou a échoué ; les modèles des messages ; un SMS d'essai. La file se relit d'elle-même
 *  pendant que les SMS partent. */
export function Messages() {
  const client = useQueryClient();
  const [filtre, setFiltre] = useState<EtatMessage | null>(null);
  const [choisi, setChoisi] = useState<string | null>(null);
  const file = useQuery({
    queryKey: ["messages", filtre],
    queryFn: () => appeler("messages_lire", { etat: filtre }),
    refetchInterval: 5_000,
  });
  const poser = (f: FileMessages) => {
    client.setQueryData(["messages", null], f);
    void client.invalidateQueries({ queryKey: ["messages"] });
  };
  const f = file.data;
  const message = f?.messages.find((m) => m.message.id === choisi);
  const nombre = (etat: EtatMessage) => f?.compteurs.find((c) => c.etat === etat)?.nombre ?? 0;
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("messages.titre")}</h1>
      <MessageErreur erreur={file.error} />
      {f === undefined ? null : (
        <>
          <EtatCanal f={f} />
          <div className="flex flex-wrap gap-2" role="group" aria-label={t("messages.filtrer")}>
            <Bouton variante={filtre === null ? "principal" : "secondaire"} onClick={() => setFiltre(null)}>{t("messages.tous")}</Bouton>
            {ORDRE_ETATS.map((e) => (
              <Bouton key={e} variante={filtre === e ? "principal" : "secondaire"} onClick={() => setFiltre(e)}>
                {`${t(ETATS[e])} (${String(nombre(e))})`}
              </Bouton>
            ))}
          </div>
          <Section titre={t("messages.file")}>
            {f.messages.length === 0 ? <p className="text-sm text-muet">{t("messages.file_vide")}</p> : null}
            <Tableau
              nomExport="messages"
              titre={t("messages.file")}
              lignes={f.messages}
              cle={(m) => m.message.id}
              surSelection={(m) => setChoisi(m.message.id)}
              colonnes={[
                { titre: t("messages.colonne.date"), valeur: (m) => dateHeure(m.message.cree_le) },
                { titre: t("messages.colonne.nature"), valeur: (m) => t(NATURES[m.message.nature]) },
                { titre: t("messages.colonne.destinataire"), valeur: (m) => m.destinataire },
                { titre: t("messages.colonne.eleve"), valeur: (m) => m.eleve },
                { titre: t("messages.colonne.numero"), valeur: (m) => telephoneLisible(m.message.numero) },
                { titre: t("messages.colonne.texte"), valeur: (m) => m.message.texte },
                { titre: t("messages.colonne.sms"), valeur: (m) => String(m.message.sms) },
                { titre: t("messages.colonne.etat"), valeur: etatAffiche },
              ]}
            />
          </Section>
          {message === undefined ? null : <MessageChoisi key={message.message.id} m={message} f={f} surFait={poser} />}
          {f.droits.configurer ? <SmsEssai surFait={poser} /> : null}
          {f.droits.configurer ? <Modeles f={f} surFait={poser} /> : null}
        </>
      )}
    </div>
  );
}

function EtatCanal({ f }: { f: FileMessages }) {
  const consommation =
    f.plafond === 0
      ? t("messages.consommation", { sms: f.sms_du_mois })
      : t("messages.consommation_plafond", { sms: f.sms_du_mois, plafond: f.plafond });
  return (
    <div className={`rounded border p-3 text-sm ${f.canal === "aucun" ? "border-ocre text-ocre" : "border-muet/30 text-encre"}`}>
      <p>{f.canal === "simule" ? t("messages.canal_simule") : t("messages.canal_aucun")}</p>
      <p className="text-muet">{`${t("messages.heures", { fenetre: f.fenetre })} · ${consommation}`}</p>
    </div>
  );
}

/** Aperçu d'un texte tel qu'il partirait : sans accents, avec son nombre de SMS. */
function useApercu(texte: string) {
  const [differe, setDiffere] = useState(texte);
  useEffect(() => {
    const minuterie = setTimeout(() => setDiffere(texte), 250);
    return () => clearTimeout(minuterie);
  }, [texte]);
  return useQuery({ queryKey: ["apercu-sms", differe], queryFn: () => appeler("message_apercu", { texte: differe }) });
}

function LigneApercu({ apercu }: { apercu: ApercuSms | undefined }) {
  if (apercu === undefined) return null;
  return (
    <p className={`text-sm ${apercu.sms > 3 ? "text-rouge" : "text-muet"}`}>
      {t("messages.apercu", { caracteres: apercu.caracteres, sms: apercu.sms })}
    </p>
  );
}

function MessageChoisi({ m, f, surFait }: { m: MessageAffiche; f: FileMessages; surFait: (f: FileMessages) => void }) {
  const [texte, setTexte] = useState(m.message.texte);
  const [motif, setMotif] = useState("");
  const apercu = useApercu(texte);
  const decision = useMutation({ mutationFn: () => appeler("message_decider", { id: m.message.id, texte }), onSuccess: surFait });
  const annulation = useMutation({ mutationFn: () => appeler("message_annuler", { id: m.message.id, motif }), onSuccess: surFait });
  const renvoi = useMutation({ mutationFn: () => appeler("message_renvoyer", { id: m.message.id }), onSuccess: surFait });
  const etat = m.message.etat;
  const annulable = f.droits.decider && (etat === "a_decider" || etat === "en_attente");
  return (
    <Section titre={t("messages.choisi", { nature: t(NATURES[m.message.nature]), numero: telephoneLisible(m.message.numero) })}>
      <p className="text-sm text-muet">{etatAffiche(m)}</p>
      {m.message.motif_annulation === "" ? null : <p className="text-sm text-muet">{t("messages.motif_annulation", { motif: m.message.motif_annulation })}</p>}
      {etat === "a_decider" && f.droits.decider ? (
        <Formulaire surEnvoi={() => decision.mutate()}>
          <p className="text-sm text-encre">{t("messages.a_relire")}</p>
          <ZoneTexte id="message-texte" libelle={t("messages.texte_relu")} valeur={texte} surChangement={setTexte} />
          <LigneApercu apercu={apercu.data} />
          <div><Bouton type="submit" disabled={decision.isPending}>{t("messages.envoyer")}</Bouton></div>
        </Formulaire>
      ) : (
        <p className="rounded bg-fond p-2 font-condensee text-sm text-encre">{m.message.texte}</p>
      )}
      {annulable ? (
        <Formulaire surEnvoi={() => annulation.mutate()}>
          <Champ id="message-motif" libelle={t("messages.motif")} value={motif} onChange={(e) => setMotif(e.target.value)} />
          <div><Bouton type="submit" variante="danger" disabled={annulation.isPending}>{t("messages.annuler")}</Bouton></div>
        </Formulaire>
      ) : null}
      {f.droits.renvoyer && (etat === "echoue" || etat === "expire") ? (
        <div><Bouton onClick={() => renvoi.mutate()} disabled={renvoi.isPending}>{t("messages.renvoyer")}</Bouton></div>
      ) : null}
      <MessageErreur erreur={decision.error ?? annulation.error ?? renvoi.error} />
    </Section>
  );
}

function SmsEssai({ surFait }: { surFait: (f: FileMessages) => void }) {
  const [numero, setNumero] = useState("");
  const [enregistre, setEnregistre] = useState<number | null>(null);
  const essai = useMutation({
    mutationFn: () => appeler("message_essai_envoyer", { numero }),
    onSuccess: (f) => {
      surFait(f);
      setEnregistre(Date.now());
    },
  });
  return (
    <Section titre={t("messages.essai")}>
      <Formulaire surEnvoi={() => essai.mutate()}>
        <Champ id="message-essai-numero" libelle={t("messages.essai_numero")} aide={t("messages.essai_aide")} value={numero} onChange={(e) => setNumero(e.target.value)} />
        <MessageErreur erreur={essai.error} />
        <div className="flex items-center gap-3">
          <Bouton type="submit" disabled={essai.isPending}>{t("messages.essai_envoyer")}</Bouton>
          <MessageEnregistre instant={enregistre} />
        </div>
      </Formulaire>
    </Section>
  );
}

function Modeles({ f, surFait }: { f: FileMessages; surFait: (f: FileMessages) => void }) {
  return (
    <Section titre={t("messages.modeles")}>
      <p className="text-sm text-muet">{t("messages.modeles_aide")}</p>
      {f.modeles.map((m) => (
        <Modele key={`${m.nature}-${m.texte}`} m={m} surFait={surFait} />
      ))}
    </Section>
  );
}

function Modele({ m, surFait }: { m: ModeleSms; surFait: (f: FileMessages) => void }) {
  const [texte, setTexte] = useState(m.texte);
  const definition = useMutation({
    mutationFn: (nouveau: string) => appeler("message_modele_definir", { nature: m.nature, texte: nouveau }),
    onSuccess: surFait,
  });
  return (
    <div className="flex flex-col gap-2 border-t border-muet/30 pt-3">
      <ZoneTexte
        id={`modele-${m.nature}`}
        libelle={t(NATURES[m.nature])}
        aide={t("messages.variables", { variables: m.variables.map((v) => `{${v}}`).join(" ") })}
        valeur={texte}
        surChangement={setTexte}
      />
      <p className="text-sm text-muet">{t("messages.exemple", { exemple: m.exemple, sms: m.sms })}</p>
      <MessageErreur erreur={definition.error} />
      <div className="flex flex-wrap gap-2">
        <Bouton variante="secondaire" onClick={() => definition.mutate(texte)} disabled={definition.isPending || texte === m.texte}>{t("commun.enregistrer")}</Bouton>
        <Bouton variante="secondaire" onClick={() => definition.mutate("")} disabled={definition.isPending}>{t("messages.modele_livre")}</Bouton>
      </div>
    </div>
  );
}
