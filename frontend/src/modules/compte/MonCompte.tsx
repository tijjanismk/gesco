import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { ApercuPdf } from "../../composants/ApercuPdf";
import { Bouton } from "../../composants/Bouton";
import { Champ, Formulaire } from "../../composants/Champ";
import { MessageEnregistre, MessageErreur, Section } from "../../composants/Messages";
import { dateCivile, francs, moisCivil, t } from "../../i18n";
import { telephoneLisible } from "../scolarite/Identite";
import type { MaFiche } from "../../types/gesco";
import { ListeEngagements } from "../rh/Engagements";

/** Fiche du personnel liée au compte (SPEC_PHASE_6 §7) : chacun lit la sienne et ses
 *  engagements, montants compris, sans aucun droit sur le personnel. */
function FichePersonnelle({ f }: { f: MaFiche }) {
  const { membre, personne, categorie } = f.fiche;
  const lignes: [string, string][] = [
    [t("personnel.matricule"), membre.matricule],
    [t("personnel.prenoms_nom"), `${personne.prenoms} ${personne.nom}`],
    [t("personnel.fonction"), membre.fonction],
    [t("personnel.categorie"), categorie],
    [t("personnel.arrive_le"), membre.arrive_le === null ? "" : dateCivile(membre.arrive_le)],
    [t("personne.telephone"), personne.telephones.map(telephoneLisible).join(", ")],
  ];
  return (
    <Section titre={t("compte.ma_fiche")}>
      <dl className="grid grid-cols-1 gap-x-4 gap-y-1 text-sm sm:grid-cols-2">
        {lignes
          .filter(([, valeur]) => valeur !== "")
          .map(([libelle, valeur]) => (
            <div key={libelle} className="flex flex-col">
              <dt className="text-muet">{libelle}</dt>
              <dd className="text-encre">{valeur}</dd>
            </div>
          ))}
      </dl>
      <h3 className="text-base font-semibold text-encre">{t("engagement.titre")}</h3>
      <ListeEngagements engagements={f.engagements} />
      <p className="text-sm text-muet">{t("compte.ma_fiche_aide")}</p>
    </Section>
  );
}

/** Code PIN de la session rapide (§6.2) et changement de mot de passe (§6.1) ; la fiche du
 *  personnel du compte, s'il en a une. */
/** Mes bulletins de paie validés, à relire et imprimer (SPEC_PHASE_7 §8). */
function MesBulletinsPaie() {
  const bulletins = useQuery({ queryKey: ["mes-bulletins-paie"], queryFn: () => appeler("mes_bulletins_paie_lire", null) });
  const [pdf, setPdf] = useState<string | null>(null);
  const imprimer = useMutation({ mutationFn: (id: string) => appeler("mon_bulletin_paie_produire", { id }), onSuccess: (d) => setPdf(d.pdf) });
  if (bulletins.data === undefined || bulletins.data.length === 0) return null;
  return (
    <Section titre={t("compte.bulletins_paie")}>
      <ul className="flex flex-col gap-1">
        {bulletins.data.map((b) => (
          <li key={b.id} className="flex flex-wrap items-center gap-3 text-sm text-encre">
            <span>
              {moisCivil(b.mois)} · {b.numero} · {francs(b.net)}
            </span>
            <Bouton variante="secondaire" onClick={() => imprimer.mutate(b.id)} disabled={imprimer.isPending}>
              {t("paie.document.bulletin")}
            </Bouton>
          </li>
        ))}
      </ul>
      <MessageErreur erreur={imprimer.error} />
      {pdf === null ? null : <ApercuPdf base64={pdf} />}
    </Section>
  );
}

export function MonCompte({ changementExige }: { changementExige: boolean }) {
  const client = useQueryClient();
  const maFiche = useQuery({ queryKey: ["ma-fiche"], queryFn: () => appeler("ma_fiche_lire", null), enabled: !changementExige });
  const [pin, setPin] = useState({ mot_de_passe: "", pin: "" });
  const [mdp, setMdp] = useState({ ancien: "", nouveau: "" });
  const [pinEnregistre, setPinEnregistre] = useState<number | null>(null);
  const [mdpEnregistre, setMdpEnregistre] = useState<number | null>(null);
  const definirPin = useMutation({
    mutationFn: () => appeler("definir_pin", pin),
    onSuccess: () => {
      setPin({ mot_de_passe: "", pin: "" });
      setPinEnregistre(Date.now());
      void client.invalidateQueries({ queryKey: ["session"] });
    },
  });
  const changerMdp = useMutation({
    mutationFn: () => appeler("changer_mot_de_passe", mdp),
    onSuccess: () => {
      setMdp({ ancien: "", nouveau: "" });
      setMdpEnregistre(Date.now());
      void client.invalidateQueries({ queryKey: ["session"] });
    },
  });
  return (
    <div className="flex max-w-xl flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("compte.titre")}</h1>
      {changementExige || maFiche.data === undefined || maFiche.data === null ? null : <FichePersonnelle f={maFiche.data} />}
      {changementExige || maFiche.data === undefined || maFiche.data === null ? null : <MesBulletinsPaie />}
      <Section titre={t("compte.changer_mot_de_passe")}>
        {changementExige ? <p className="text-sm text-ocre">{t("compte.changement_exige")}</p> : null}
        <Formulaire surEnvoi={() => changerMdp.mutate()}>
          <Champ id="ancien-mdp" type="password" libelle={t("compte.mot_de_passe_actuel")} value={mdp.ancien} autoComplete="current-password" onChange={(e) => setMdp({ ...mdp, ancien: e.target.value })} />
          <Champ id="nouveau-mdp" type="password" libelle={t("compte.nouveau_mot_de_passe")} value={mdp.nouveau} autoComplete="new-password" onChange={(e) => setMdp({ ...mdp, nouveau: e.target.value })} />
          <MessageErreur erreur={changerMdp.error} />
          <MessageEnregistre instant={mdpEnregistre} />
          <div><Bouton type="submit" disabled={changerMdp.isPending}>{t("commun.enregistrer")}</Bouton></div>
        </Formulaire>
      </Section>
      {changementExige ? null : (
        <Section titre={t("compte.pin")}>
          <p className="text-sm text-muet">{t("compte.pin_aide")}</p>
          <Formulaire surEnvoi={() => definirPin.mutate()}>
            <Champ id="pin-mdp" type="password" libelle={t("compte.mot_de_passe_actuel")} value={pin.mot_de_passe} autoComplete="current-password" onChange={(e) => setPin({ ...pin, mot_de_passe: e.target.value })} />
            <Champ id="nouveau-pin" type="password" inputMode="numeric" libelle={t("compte.nouveau_pin")} value={pin.pin} autoComplete="off" onChange={(e) => setPin({ ...pin, pin: e.target.value })} />
            <MessageErreur erreur={definirPin.error} />
            <MessageEnregistre instant={pinEnregistre} />
            <div><Bouton type="submit" disabled={definirPin.isPending}>{t("commun.enregistrer")}</Bouton></div>
          </Formulaire>
        </Section>
      )}
    </div>
  );
}
