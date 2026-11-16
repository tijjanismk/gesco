import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { useSession } from "../../app/session";
import { Bouton } from "../../composants/Bouton";
import { Champ, Formulaire, Selection } from "../../composants/Champ";
import { MessageEnregistre, MessageErreur, Section } from "../../composants/Messages";
import { Tableau } from "../../composants/Tableau";
import { dateHeure, t, taille } from "../../i18n";
import type { Destination, PhraseAffichee, SorteDestination } from "../../types/gesco";
import { PhraseSecours } from "../accueil/PhraseSecours";
import { Restauration } from "../accueil/Restauration";
import { HorsEcole } from "./HorsEcole";
import { CLES_ETAT, CLES_SORTE } from "./copieHorsEcole";

/** Sauvegarde et restauration (spécification §11) : le module le plus important de la phase. */
export function Sauvegardes() {
  const client = useQueryClient();
  const session = useSession();
  const historique = useQuery({ queryKey: ["sauvegardes"], queryFn: () => appeler("sauvegardes_lister", null) });
  const rafraichir = () => {
    void client.invalidateQueries({ queryKey: ["sauvegardes"] });
    void client.invalidateQueries({ queryKey: ["avertissements"] });
  };
  const creation = useMutation({ mutationFn: () => appeler("sauvegarde_creer", null), onSuccess: rafraichir });
  const test = useMutation({ mutationFn: () => appeler("sauvegarde_tester", null), onSuccess: rafraichir });
  return (
    <div className="flex max-w-5xl flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("sauvegarde.titre")}</h1>
      <div className="flex flex-wrap items-center gap-2">
        <Bouton onClick={() => creation.mutate()} disabled={creation.isPending}>{t("sauvegarde.maintenant")}</Bouton>
        <Bouton variante="secondaire" onClick={() => test.mutate()} disabled={test.isPending}>{t("sauvegarde.tester")}</Bouton>
      </div>
      <MessageErreur erreur={creation.error ?? test.error} />
      {test.data === undefined ? null : test.data.reussi ? (
        <p role="status" className="text-sm text-vert">{t("sauvegarde.test_reussi", { destination: test.data.destination })}</p>
      ) : (
        <p role="alert" className="text-sm text-rouge">{t("sauvegarde.test_echoue", { destination: test.data.destination })}</p>
      )}
      <Section titre={t("sauvegarde.historique")}>
        <MessageErreur erreur={historique.error} />
        <Tableau
          nomExport="sauvegardes"
          titre={t("sauvegarde.titre")}
          lignes={historique.data ?? []}
          cle={(s) => s.id}
          colonnes={[
            { titre: t("sauvegarde.date"), valeur: (s) => dateHeure(s.horodatage) },
            { titre: t("sauvegarde.destination"), valeur: (s) => s.destination },
            { titre: t("sauvegarde.sorte"), valeur: (s) => t(CLES_SORTE[s.sorte]) },
            { titre: t("sauvegarde.taille"), valeur: (s) => taille(s.taille) },
            {
              titre: t("sauvegarde.resultat"),
              valeur: (s) => (s.resultat === "reussie" ? t("sauvegarde.resultat.reussie") : t("sauvegarde.resultat.echouee")),
            },
            { titre: t("sauvegarde.envoi"), valeur: (s) => (s.etat_envoi === null ? "" : t(CLES_ETAT[s.etat_envoi])) },
            {
              titre: t("sauvegarde.dernier_test"),
              valeur: (s) => (s.dernier_test_restauration === null ? "" : dateHeure(s.dernier_test_restauration)),
            },
          ]}
        />
      </Section>
      <HorsEcole />
      <Destinations />
      <Certificat />
      <Restauration
        commande="sauvegarde_restaurer"
        surTermine={() => {
          session.fermer();
          void client.invalidateQueries({ queryKey: ["etat"] });
        }}
      />
    </div>
  );
}

function Destinations() {
  const client = useQueryClient();
  const destinations = useQuery({
    queryKey: ["destinations"],
    queryFn: () => appeler("sauvegarde_destinations_lister", null),
  });
  if (destinations.data === undefined) return <MessageErreur erreur={destinations.error} />;
  return (
    <EditeurDestinations
      key={JSON.stringify(destinations.data)}
      initiales={destinations.data}
      surEnregistre={() => {
        void client.invalidateQueries({ queryKey: ["destinations"] });
        void client.invalidateQueries({ queryKey: ["avertissements"] });
      }}
    />
  );
}

/** Les trois sortes, dans l'ordre de la liste. */
const SORTES: readonly SorteDestination[] = ["dossier", "amovible", "synchronise"];

function EditeurDestinations({ initiales, surEnregistre }: { initiales: readonly Destination[]; surEnregistre: () => void }) {
  // La première destination est le dossier du poste, toujours présent.
  const [locale, ...autres] = initiales;
  const [liste, setListe] = useState<Destination[]>(autres);
  const [enregistre, setEnregistre] = useState<number | null>(null);
  const enregistrement = useMutation({
    mutationFn: () => appeler("sauvegarde_destinations_definir", liste),
    onSuccess: () => {
      setEnregistre(Date.now());
      surEnregistre();
    },
  });
  const modifier = (rang: number, d: Destination) => setListe(liste.map((x, i) => (i === rang ? d : x)));
  return (
    <Section titre={t("sauvegarde.destinations")}>
      <p className="text-sm text-muet">{t("sauvegarde.destinations_aide")}</p>
      {locale === undefined ? null : (
        <p className="font-condensee text-sm text-encre">{`${t("sauvegarde.locale")} : ${locale.chemin}`}</p>
      )}
      {liste.map((d, rang) => (
        <div key={rang} className="flex flex-wrap items-end gap-3">
          <Champ id={`destination-${rang}`} libelle={t("sauvegarde.chemin")} value={d.chemin} onChange={(e) => modifier(rang, { ...d, chemin: e.target.value })} />
          <Selection
            id={`sorte-${rang}`}
            libelle={t("sauvegarde.sorte")}
            valeur={d.sorte}
            surChangement={(valeur) => {
              const sorte = valeur as SorteDestination;
              modifier(rang, { ...d, sorte, amovible: sorte === "amovible" });
            }}
            options={SORTES.map((sorte) => ({ valeur: sorte, libelle: t(CLES_SORTE[sorte]) }))}
          />
          <Bouton variante="secondaire" onClick={() => setListe(liste.filter((_, i) => i !== rang))}>{t("commun.retirer")}</Bouton>
        </div>
      ))}
      <div className="flex gap-2">
        <Bouton variante="secondaire" onClick={() => setListe([...liste, { chemin: "", sorte: "amovible", amovible: true }])}>{t("commun.ajouter")}</Bouton>
        <Bouton onClick={() => enregistrement.mutate()} disabled={enregistrement.isPending}>{t("commun.enregistrer")}</Bouton>
      </div>
      <MessageErreur erreur={enregistrement.error} />
      <MessageEnregistre instant={enregistre} />
    </Section>
  );
}

function Certificat() {
  const client = useQueryClient();
  const [motDePasse, setMotDePasse] = useState("");
  const [phrase, setPhrase] = useState<PhraseAffichee | null>(null);
  const [confirme, setConfirme] = useState<number | null>(null);
  const confirmation = useMutation({
    mutationFn: () => appeler("certificat_confirmer", null),
    onSuccess: () => {
      setConfirme(Date.now());
      void client.invalidateQueries({ queryKey: ["avertissements"] });
    },
  });
  const regeneration = useMutation({
    mutationFn: () => appeler("phrase_secours_regenerer", { mot_de_passe: motDePasse }),
    onSuccess: (nouvelle) => {
      setMotDePasse("");
      setPhrase(nouvelle);
      void client.invalidateQueries({ queryKey: ["avertissements"] });
    },
  });
  if (phrase !== null) {
    return (
      <PhraseSecours
        phrase={phrase}
        surTermine={() => {
          setPhrase(null);
          confirmation.mutate();
        }}
      />
    );
  }
  return (
    <Section titre={t("sauvegarde.certificat")}>
      <div>
        <Bouton variante="secondaire" onClick={() => confirmation.mutate()} disabled={confirmation.isPending}>{t("sauvegarde.certificat_confirmer")}</Bouton>
      </div>
      <MessageErreur erreur={confirmation.error} />
      <MessageEnregistre instant={confirme} />
      <h3 className="text-sm font-medium text-encre">{t("sauvegarde.regenerer")}</h3>
      <p className="text-sm text-muet">{t("sauvegarde.regenerer_aide")}</p>
      <Formulaire surEnvoi={() => regeneration.mutate()}>
        <Champ id="regeneration-mdp" type="password" autoComplete="current-password" libelle={t("sauvegarde.mot_de_passe")} value={motDePasse} onChange={(e) => setMotDePasse(e.target.value)} />
        <MessageErreur erreur={regeneration.error} />
        <div><Bouton variante="danger" type="submit" disabled={regeneration.isPending}>{t("sauvegarde.regenerer")}</Bouton></div>
      </Formulaire>
    </Section>
  );
}
