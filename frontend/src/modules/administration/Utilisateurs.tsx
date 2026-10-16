import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { Bouton } from "../../composants/Bouton";
import { CaseACocher, Champ, Formulaire } from "../../composants/Champ";
import { MessageEnregistre, MessageErreur, Section } from "../../composants/Messages";
import { Tableau } from "../../composants/Tableau";
import { dateHeure, heure, t } from "../../i18n";
import type { FicheRole, FicheUtilisateur } from "../../types/gesco";

/** Utilisateurs et rôles (spécification §12.1). Les actions rapides se font dans un panneau
 *  latéral, jamais dans une modale (R-40). */
export function Utilisateurs() {
  const utilisateurs = useQuery({ queryKey: ["utilisateurs"], queryFn: () => appeler("utilisateurs_lister", null) });
  const roles = useQuery({ queryKey: ["roles"], queryFn: () => appeler("roles_lister", null) });
  const [choisi, setChoisi] = useState<string | null>(null);
  const fiche = utilisateurs.data?.find((f) => f.utilisateur.id === choisi);
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("utilisateurs.titre")}</h1>
      <MessageErreur erreur={utilisateurs.error ?? roles.error} />
      <div className="flex flex-wrap items-start gap-4">
        <div className="min-w-0 flex-1">
          <Tableau
            nomExport="utilisateurs"
            titre={t("utilisateurs.titre")}
            lignes={utilisateurs.data ?? []}
            cle={(f) => f.utilisateur.id}
            surSelection={(f) => setChoisi(f.utilisateur.id)}
            colonnes={[
              { titre: t("utilisateurs.identifiant"), valeur: (f) => f.utilisateur.identifiant },
              { titre: t("utilisateurs.nom"), valeur: (f) => f.utilisateur.nom_affiche },
              {
                titre: t("utilisateurs.roles"),
                valeur: (f) => f.roles.map((code) => roles.data?.find((r) => r.role.code === code)?.role.libelle ?? code).join(", "),
              },
              { titre: t("utilisateurs.actif"), valeur: (f) => (f.utilisateur.actif ? t("commun.oui") : t("commun.non")) },
              {
                titre: t("utilisateurs.derniere_connexion"),
                valeur: (f) => (f.utilisateur.derniere_connexion === null ? t("utilisateurs.jamais") : dateHeure(f.utilisateur.derniere_connexion)),
              },
            ]}
          />
        </div>
        <aside className="flex w-80 flex-col gap-4">
          {fiche === undefined ? null : (
            <FicheUtilisateurPanneau key={fiche.utilisateur.id} fiche={fiche} roles={roles.data ?? []} />
          )}
          <NouvelUtilisateur roles={roles.data ?? []} />
        </aside>
      </div>
    </div>
  );
}

function FicheUtilisateurPanneau({ fiche, roles }: { fiche: FicheUtilisateur; roles: readonly FicheRole[] }) {
  const client = useQueryClient();
  const [nom, setNom] = useState(fiche.utilisateur.nom_affiche);
  const [actif, setActif] = useState(fiche.utilisateur.actif);
  const [enregistre, setEnregistre] = useState<number | null>(null);
  const rafraichir = () => {
    setEnregistre(Date.now());
    void client.invalidateQueries({ queryKey: ["utilisateurs"] });
  };
  const modification = useMutation({
    mutationFn: () => appeler("utilisateur_modifier", { id: fiche.utilisateur.id, nom_affiche: nom, actif }),
    onSuccess: rafraichir,
  });
  const attribution = useMutation({
    mutationFn: (demande: { role: string; attribue: boolean }) =>
      appeler("role_attribuer", { utilisateur_id: fiche.utilisateur.id, ...demande }),
    onSuccess: rafraichir,
  });
  const [motDePasse, setMotDePasse] = useState("");
  const reinitialisation = useMutation({
    mutationFn: () =>
      appeler("utilisateur_reinitialiser_mot_de_passe", { id: fiche.utilisateur.id, mot_de_passe_initial: motDePasse }),
    onSuccess: () => {
      setMotDePasse("");
      rafraichir();
    },
  });
  const deverrouillage = useMutation({
    mutationFn: () => appeler("utilisateur_deverrouiller", { id: fiche.utilisateur.id }),
    onSuccess: rafraichir,
  });
  const { verrouille_jusqua: verrouJusqua, doit_changer_mot_de_passe: doitChanger } = fiche.utilisateur;
  return (
    <Section titre={t("utilisateurs.fiche", { nom: fiche.utilisateur.nom_affiche })}>
      <Formulaire surEnvoi={() => modification.mutate()}>
        <Champ id="fiche-nom" libelle={t("utilisateurs.nom")} value={nom} onChange={(e) => setNom(e.target.value)} />
        <CaseACocher id="fiche-actif" libelle={t("utilisateurs.actif")} coche={actif} surChangement={setActif} />
        <div><Bouton type="submit" disabled={modification.isPending}>{t("commun.enregistrer")}</Bouton></div>
      </Formulaire>
      {doitChanger ? <p className="text-sm text-ocre">{t("utilisateurs.changement_attendu")}</p> : null}
      {verrouJusqua === null ? null : (
        <div className="flex flex-col gap-2">
          <p className="text-sm text-rouge">{t("utilisateurs.verrouille_jusqua", { heure: heure(verrouJusqua) })}</p>
          <div>
            <Bouton variante="secondaire" onClick={() => deverrouillage.mutate()} disabled={deverrouillage.isPending}>
              {t("utilisateurs.deverrouiller")}
            </Bouton>
          </div>
        </div>
      )}
      <Formulaire surEnvoi={() => reinitialisation.mutate()}>
        <Champ
          id="fiche-reinitialisation"
          type="password"
          autoComplete="new-password"
          libelle={t("utilisateurs.nouveau_mot_de_passe_initial")}
          aide={t("utilisateurs.reinitialiser_aide")}
          value={motDePasse}
          onChange={(e) => setMotDePasse(e.target.value)}
        />
        <div>
          <Bouton variante="secondaire" type="submit" disabled={reinitialisation.isPending || motDePasse === ""}>
            {t("utilisateurs.reinitialiser")}
          </Bouton>
        </div>
      </Formulaire>
      <p className="text-sm font-medium text-encre">{t("utilisateurs.roles")}</p>
      {roles.map((r) => (
        <CaseACocher
          key={r.role.id}
          id={`role-${r.role.code}`}
          libelle={r.role.libelle}
          coche={fiche.roles.includes(r.role.code)}
          desactive={attribution.isPending}
          surChangement={(attribue) => attribution.mutate({ role: r.role.code, attribue })}
        />
      ))}
      <MessageErreur erreur={modification.error ?? attribution.error ?? reinitialisation.error ?? deverrouillage.error} />
      <MessageEnregistre instant={enregistre} />
    </Section>
  );
}

function NouvelUtilisateur({ roles }: { roles: readonly FicheRole[] }) {
  const client = useQueryClient();
  const vide = { identifiant: "", nom_affiche: "", mot_de_passe_initial: "", roles: [] as string[] };
  const [champs, setChamps] = useState(vide);
  const [enregistre, setEnregistre] = useState<number | null>(null);
  const creation = useMutation({
    mutationFn: () => appeler("utilisateur_creer", champs),
    onSuccess: () => {
      setChamps(vide);
      setEnregistre(Date.now());
      void client.invalidateQueries({ queryKey: ["utilisateurs"] });
    },
  });
  return (
    <Section titre={t("utilisateurs.nouveau")}>
      <Formulaire surEnvoi={() => creation.mutate()}>
        <Champ id="nouvel-identifiant" libelle={t("utilisateurs.identifiant")} value={champs.identifiant} onChange={(e) => setChamps({ ...champs, identifiant: e.target.value })} />
        <Champ id="nouveau-nom" libelle={t("utilisateurs.nom")} value={champs.nom_affiche} onChange={(e) => setChamps({ ...champs, nom_affiche: e.target.value })} />
        <Champ id="nouveau-mdp-initial" type="password" autoComplete="new-password" libelle={t("utilisateurs.mot_de_passe_initial")} value={champs.mot_de_passe_initial} onChange={(e) => setChamps({ ...champs, mot_de_passe_initial: e.target.value })} />
        {roles.map((r) => (
          <CaseACocher
            key={r.role.id}
            id={`nouveau-role-${r.role.code}`}
            libelle={r.role.libelle}
            coche={champs.roles.includes(r.role.code)}
            surChangement={(coche) =>
              setChamps({
                ...champs,
                roles: coche ? [...champs.roles, r.role.code] : champs.roles.filter((c) => c !== r.role.code),
              })
            }
          />
        ))}
        <MessageErreur erreur={creation.error} />
        <MessageEnregistre instant={enregistre} />
        <div><Bouton type="submit" disabled={creation.isPending}>{t("utilisateurs.creer")}</Bouton></div>
      </Formulaire>
    </Section>
  );
}
