import { useMutation, useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { Bouton } from "../../composants/Bouton";
import { Champ, Selection } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { Tableau } from "../../composants/Tableau";
import { type Cle, dateHeure, t } from "../../i18n";
import type { LigneAudit, Operation, RechercheJournal, ValeurJson } from "../../types/gesco";

const OPERATIONS = {
  creation: "operation.creation",
  modification: "operation.modification",
  suppression: "operation.suppression",
  transition: "operation.transition",
  annulation: "operation.annulation",
  session: "operation.session",
  acces: "operation.acces",
} as const satisfies Record<Operation, Cle>;

/** Entités du journal par leur nom français ; une entité nouvelle garde son nom technique. */
const ENTITES: Record<string, Cle> = {
  annee_scolaire: "entite.annee_scolaire",
  classe: "entite.classe",
  document: "entite.document",
  eleve: "entite.eleve",
  eleve_photo: "entite.eleve_photo",
  eleve_responsable: "entite.eleve_responsable",
  enveloppe_secours: "entite.enveloppe_secours",
  etablissement: "entite.etablissement",
  etablissement_logo: "entite.etablissement_logo",
  import: "entite.import",
  facture: "entite.facture",
  grille_tarifaire: "entite.grille_tarifaire",
  inscription: "entite.inscription",
  licence: "entite.licence",
  niveau: "entite.niveau",
  numero: "entite.numero",
  organisation: "entite.organisation",
  parametre: "entite.parametre",
  reduction: "entite.reduction",
  responsable: "entite.responsable",
  role: "entite.role",
  sauvegarde: "entite.sauvegarde",
  serie: "entite.serie",
  session_caisse: "entite.session_caisse",
  utilisateur: "entite.utilisateur",
  versement: "entite.versement",
  matiere: "entite.matiere",
  enseignant: "entite.enseignant",
  evaluation: "entite.evaluation",
  note: "entite.note",
  affectation_enseignement: "entite.affectation_enseignement",
  programme_matiere: "entite.programme_matiere",
  periode_academique: "entite.periode_academique",
  regle_evaluation: "entite.regle_evaluation",
  mouvement_caisse: "entite.mouvement_caisse",
  utilisateur_role: "entite.utilisateur_role",
};

function libelleEntite(entite: string): string {
  const cle = ENTITES[entite];
  return cle === undefined ? entite : t(cle);
}

function estOperation(valeur: string): valeur is Operation {
  return Object.hasOwn(OPERATIONS, valeur);
}

/** Jour saisi (`AAAA-MM-JJ`) vers minuit UTC ; `jourSuivant` pour une borne de fin incluse. */
function versMillis(jour: string, jourSuivant = false): number | null {
  if (jour === "") return null;
  const [a, m, j] = jour.split("-").map(Number);
  return Date.UTC(a ?? 0, (m ?? 1) - 1, (j ?? 1) + (jourSuivant ? 1 : 0));
}

/** Journal d'audit (SPEC_PHASE_1 §8) : filtres, noms lisibles, détail avant et après, chaîne
 *  vérifiable. */
export function Journal() {
  const [filtres, setFiltres] = useState({ utilisateur: "", entite: "", operation: "", du: "", au: "" });
  const [page, setPage] = useState(0);
  const [choisie, setChoisie] = useState<string | null>(null);
  const recherche: RechercheJournal = {
    utilisateur_id: filtres.utilisateur === "" ? null : filtres.utilisateur,
    entite: filtres.entite === "" ? null : filtres.entite,
    operation: estOperation(filtres.operation) ? filtres.operation : null,
    depuis: versMillis(filtres.du),
    jusqua: versMillis(filtres.au, true),
    page,
  };
  const journal = useQuery({ queryKey: ["journal", recherche], queryFn: () => appeler("audit_lister", recherche) });
  const verification = useMutation({ mutationFn: () => appeler("audit_verifier", null) });
  const filtrer = (cle: keyof typeof filtres, valeur: string) => {
    setFiltres({ ...filtres, [cle]: valeur });
    setPage(0);
  };
  const auteurs = journal.data?.auteurs ?? [];
  const nomAuteur = (ligne: LigneAudit) =>
    ligne.utilisateur_id === null ? t("journal.systeme") : (auteurs.find((a) => a.id === ligne.utilisateur_id)?.nom ?? ligne.utilisateur_id);
  const ligne = journal.data?.lignes.find((l) => l.id === choisie);
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("journal.titre")}</h1>
      <div className="flex flex-wrap items-end gap-3">
        <Selection
          id="journal-auteur"
          libelle={t("journal.auteur")}
          valeur={filtres.utilisateur}
          surChangement={(v) => filtrer("utilisateur", v)}
          options={[{ valeur: "", libelle: t("journal.tous") }, ...auteurs.map((a) => ({ valeur: a.id, libelle: a.nom }))]}
        />
        <Selection
          id="journal-entite"
          libelle={t("journal.entite")}
          valeur={filtres.entite}
          surChangement={(v) => filtrer("entite", v)}
          options={[
            { valeur: "", libelle: t("journal.toutes") },
            ...(journal.data?.entites ?? []).map((e) => ({ valeur: e, libelle: libelleEntite(e) })),
          ]}
        />
        <Selection
          id="journal-operation"
          libelle={t("journal.operation")}
          valeur={filtres.operation}
          surChangement={(v) => filtrer("operation", v)}
          options={[
            { valeur: "", libelle: t("journal.toutes") },
            ...Object.entries(OPERATIONS).map(([valeur, cle]) => ({ valeur, libelle: t(cle) })),
          ]}
        />
        <Champ id="journal-du" type="date" libelle={t("journal.du")} value={filtres.du} onChange={(e) => filtrer("du", e.target.value)} />
        <Champ id="journal-au" type="date" libelle={t("journal.au")} value={filtres.au} onChange={(e) => filtrer("au", e.target.value)} />
        <Bouton variante="secondaire" onClick={() => verification.mutate()} disabled={verification.isPending}>{t("journal.verifier")}</Bouton>
      </div>
      {verification.data === undefined ? null : (
        <p role="status" className="text-sm text-vert">{t("journal.integre", { lignes: verification.data.lignes_verifiees })}</p>
      )}
      <MessageErreur erreur={verification.error ?? journal.error} />
      <div className="flex flex-wrap items-start gap-4">
        <div className="min-w-0 flex-1">
          <Tableau
            nomExport="journal-audit"
            titre={t("journal.titre")}
            lignes={journal.data?.lignes ?? []}
            cle={(l) => l.id}
            surSelection={(l) => setChoisie(l.id)}
            colonnes={[
              { titre: t("journal.horodatage"), valeur: (l) => dateHeure(l.horodatage) },
              { titre: t("journal.auteur"), valeur: nomAuteur },
              { titre: t("journal.entite"), valeur: (l) => libelleEntite(l.entite) },
              { titre: t("journal.operation"), valeur: (l) => t(OPERATIONS[l.operation]) },
              { titre: t("journal.origine"), valeur: (l) => l.contexte },
              { titre: t("journal.poste"), valeur: (l) => l.poste },
            ]}
          />
          <div className="mt-2 flex items-center gap-2">
            <Bouton variante="secondaire" onClick={() => setPage(page - 1)} disabled={page === 0}>{t("journal.precedente")}</Bouton>
            <p className="text-sm text-muet">{t("journal.page", { page: page + 1 })}</p>
            <Bouton variante="secondaire" onClick={() => setPage(page + 1)} disabled={journal.data?.suivante !== true}>{t("journal.suivante")}</Bouton>
          </div>
        </div>
        {ligne === undefined ? null : <DetailLigne ligne={ligne} auteur={nomAuteur(ligne)} />}
      </div>
    </div>
  );
}

function enTexte(valeur: ValeurJson | undefined): string {
  if (valeur === undefined || valeur === null) return "";
  return typeof valeur === "string" ? valeur : JSON.stringify(valeur);
}

function champs(valeur: ValeurJson | null): Record<string, ValeurJson> {
  return valeur !== null && typeof valeur === "object" && !Array.isArray(valeur) ? valeur : {};
}

/** Avant et après, champ par champ ; les champs changés ressortent. */
function DetailLigne({ ligne, auteur }: { ligne: LigneAudit; auteur: string }) {
  const avant = champs(ligne.avant);
  const apres = champs(ligne.apres);
  const noms = [...new Set([...Object.keys(avant), ...Object.keys(apres)])];
  return (
    <aside className="w-full max-w-xl">
      <Section titre={t("journal.detail", { entite: libelleEntite(ligne.entite), auteur })}>
        <p className="text-sm text-muet">{`${dateHeure(ligne.horodatage)} · ${ligne.entite_id}`}</p>
        <table className="w-full text-left text-sm">
          <thead>
            <tr className="text-muet">
              <th className="px-2 py-1 font-normal">{t("journal.champ")}</th>
              <th className="px-2 py-1 font-normal">{t("journal.avant")}</th>
              <th className="px-2 py-1 font-normal">{t("journal.apres")}</th>
            </tr>
          </thead>
          <tbody>
            {noms.map((nom) => {
              const change = enTexte(avant[nom]) !== enTexte(apres[nom]);
              return (
                <tr key={nom} className={change ? "bg-fond font-medium" : ""}>
                  <td className="border-b border-muet/30 px-2 py-1 text-muet">{nom}</td>
                  <td className="break-all border-b border-muet/30 px-2 py-1 text-encre">{enTexte(avant[nom])}</td>
                  <td className="break-all border-b border-muet/30 px-2 py-1 text-encre">{enTexte(apres[nom])}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </Section>
    </aside>
  );
}
