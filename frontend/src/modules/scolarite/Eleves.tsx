import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { commandeAutorisee } from "../../app/navigation";
import { Bouton } from "../../composants/Bouton";
import { Champ, Formulaire } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { Tableau } from "../../composants/Tableau";
import { dateCivile, t } from "../../i18n";
import type { SaisieEleve, StatutInscription } from "../../types/gesco";
import { ChampsEleve, FicheEleveEcran, saisieEleveVide } from "./FicheEleve";
import { libelleSexe } from "./Identite";
import { ImportEleves } from "./ImportEleves";
import { STATUTS } from "./InscriptionsEleve";

/** Élèves (SPEC_PHASE_2 §4) : recherche, fiche, création. */
export function Eleves({ ouvert }: { ouvert: string | null }) {
  const session = useQuery({ queryKey: ["session"], queryFn: () => appeler("session_courante", null) });
  const [texte, setTexte] = useState("");
  const [choisi, setChoisi] = useState<string | null>(ouvert);
  const [creation, setCreation] = useState(false);
  const [importation, setImportation] = useState(false);
  const trouves = useQuery({ queryKey: ["eleves", texte], queryFn: () => appeler("eleves_rechercher", { texte }) });
  const peutCreer = commandeAutorisee("eleve_creer", session.data?.permissions ?? []);
  const peutImporter = commandeAutorisee("import_eleves_analyser", session.data?.permissions ?? []);
  if (importation) return <ImportEleves surTermine={() => setImportation(false)} />;
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("eleves.titre")}</h1>
      {/* L'aide passe sous la rangée : sous le champ, elle décalait les boutons vers le bas. */}
      <div className="flex flex-col gap-1">
        <div className="flex flex-wrap items-end gap-3">
          <div className="w-96">
            <Champ id="eleves-recherche" type="search" autoFocus libelle={t("eleves.rechercher")} aria-describedby="eleves-recherche-aide" value={texte} onChange={(e) => setTexte(e.target.value)} />
          </div>
          {peutCreer ? (
            <Bouton
              onClick={() => {
                setCreation(true);
                setChoisi(null);
              }}
            >
              {t("eleves.nouveau")}
            </Bouton>
          ) : null}
          {peutImporter ? <Bouton variante="secondaire" onClick={() => setImportation(true)}>{t("import.ouvrir")}</Bouton> : null}
        </div>
        <p id="eleves-recherche-aide" className="text-sm text-muet">{t("eleves.rechercher_aide")}</p>
      </div>
      <MessageErreur erreur={trouves.error} />
      <div className="flex flex-wrap items-start gap-4">
        <div className="min-w-0 flex-1">
          <Tableau
            nomExport="eleves"
            titre={t("eleves.titre")}
            lignes={trouves.data ?? []}
            cle={(e) => e.id}
            surSelection={(e) => {
              setChoisi(e.id);
              setCreation(false);
            }}
            colonnes={[
              { titre: t("eleves.matricule"), valeur: (e) => e.matricule_interne },
              { titre: t("personne.prenoms"), valeur: (e) => e.prenoms },
              { titre: t("personne.nom"), valeur: (e) => e.nom },
              {
                titre: t("eleves.classe"),
                valeur: (e) =>
                  e.statut_inscription === "" || e.statut_inscription === "validee"
                    ? e.classe
                    : `${e.classe} (${t(STATUTS[e.statut_inscription as StatutInscription]).toLowerCase()})`,
              },
              { titre: t("personne.sexe"), valeur: (e) => libelleSexe(e.sexe) },
              { titre: t("personne.date_naissance"), valeur: (e) => (e.date_naissance === null ? "" : dateCivile(e.date_naissance)) },
              { titre: t("eleves.matricule_national"), valeur: (e) => e.matricule_national ?? "" },
            ]}
          />
        </div>
        <aside className="w-full max-w-2xl">
          {creation ? (
            <NouvelEleve
              surCree={(id) => {
                setCreation(false);
                setChoisi(id);
              }}
            />
          ) : null}
          {choisi === null ? null : <FicheEleveEcran key={choisi} id={choisi} permissions={session.data?.permissions ?? []} surOuvrir={setChoisi} />}
        </aside>
      </div>
    </div>
  );
}

export function NouvelEleve({ surCree }: { surCree: (id: string) => void }) {
  const client = useQueryClient();
  const [saisie, setSaisie] = useState<SaisieEleve>(saisieEleveVide);
  const { nom, prenoms, date_naissance } = saisie.identite;
  const complete = nom.trim() !== "" && prenoms.trim() !== "" && date_naissance !== "";
  const doublons = useQuery({
    queryKey: ["doublons", nom, prenoms, date_naissance],
    queryFn: () => appeler("eleve_doublons_probables", { nom, prenoms, date_naissance, sauf: null }),
    enabled: complete,
  });
  const creation = useMutation({
    mutationFn: () => appeler("eleve_creer", saisie),
    onSuccess: (fiche) => {
      void client.invalidateQueries({ queryKey: ["eleves"] });
      surCree(fiche.eleve.id);
    },
  });
  return (
    <Section titre={t("eleves.nouveau")}>
      <Formulaire surEnvoi={() => creation.mutate()}>
        <ChampsEleve prefixe="nouvel-eleve" valeur={saisie} surChangement={setSaisie} />
        {complete && (doublons.data ?? []).length > 0 ? (
          <div role="status" className="rounded border border-ocre p-3 text-sm">
            <p className="font-medium text-ocre">{t("eleves.doublons")}</p>
            <ul className="mt-1 list-inside list-disc text-encre">
              {(doublons.data ?? []).map((d) => (
                <li key={d.id}>
                  <button type="button" className="text-primaire underline" onClick={() => surCree(d.id)}>
                    {`${d.prenoms} ${d.nom} · ${d.matricule_interne}${d.date_naissance === null ? "" : ` · ${dateCivile(d.date_naissance)}`}`}
                  </button>
                </li>
              ))}
            </ul>
            <p className="mt-1 text-muet">{t("eleves.doublons_aide")}</p>
          </div>
        ) : null}
        <MessageErreur erreur={creation.error} />
        <div><Bouton type="submit" disabled={creation.isPending}>{t("eleves.creer")}</Bouton></div>
      </Formulaire>
    </Section>
  );
}
