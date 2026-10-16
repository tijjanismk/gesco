import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { Bouton } from "../../composants/Bouton";
import { CaseACocher, Champ } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { type Cle, dateCivile, t } from "../../i18n";
import type { ApercuFusion, FicheEleve } from "../../types/gesco";
import { telephoneLisible } from "./Identite";

/** Champs comparés avant la fusion : ce que chaque fiche porte, et ce qui restera. */
const COMPARES: readonly [Cle, (f: Pick<ApercuFusion, "eleve" | "identite">) => string][] = [
  ["eleves.matricule", (f) => f.eleve.matricule_interne],
  ["personne.prenoms", (f) => f.identite.prenoms],
  ["personne.nom", (f) => f.identite.nom],
  ["personne.date_naissance", (f) => (f.identite.date_naissance === null ? "" : dateCivile(f.identite.date_naissance))],
  ["personne.lieu_naissance", (f) => f.identite.lieu_naissance],
  ["eleves.matricule_national", (f) => f.eleve.matricule_national ?? ""],
  ["personne.adresse", (f) => f.identite.adresse],
  ["personne.telephone", (f) => f.identite.telephones.map(telephoneLisible).join(" · ")],
  ["eleves.ecole_origine", (f) => f.eleve.ecole_origine],
];

/** Fusion de deux fiches en doublon (SPEC_PHASE_2 §8) : on choisit l'autre fiche, on voit le
 *  résultat, on confirme. Rien ne se fusionne sans ce geste (T-07). */
export function FusionEleve({ fiche, surOuvrir }: { fiche: FicheEleve; surOuvrir: (id: string) => void }) {
  const client = useQueryClient();
  const [ouverte, setOuverte] = useState(false);
  const [recherche, setRecherche] = useState("");
  const [autre, setAutre] = useState<string | null>(null);
  const [garderCelleCi, setGarderCelleCi] = useState(true);
  const [verifie, setVerifie] = useState(false);
  const { nom, prenoms, date_naissance } = fiche.identite;
  const doublons = useQuery({
    queryKey: ["doublons", nom, prenoms, date_naissance, fiche.eleve.id],
    queryFn: () => appeler("eleve_doublons_probables", { nom, prenoms, date_naissance: date_naissance ?? "", sauf: fiche.eleve.id }),
    enabled: ouverte,
  });
  const trouves = useQuery({
    queryKey: ["fusion-candidats", recherche],
    queryFn: () => appeler("eleves_rechercher", { texte: recherche }),
    enabled: ouverte && recherche.trim().length >= 2,
  });
  const demande = autre === null ? null : garderCelleCi ? { conservee_id: fiche.eleve.id, absorbee_id: autre } : { conservee_id: autre, absorbee_id: fiche.eleve.id };
  const apercu = useQuery({
    queryKey: ["fusion", demande],
    queryFn: () => appeler("eleves_fusion_apercu", demande ?? { conservee_id: "", absorbee_id: "" }),
    enabled: demande !== null,
  });
  const fusion = useMutation({
    mutationFn: () => appeler("eleves_fusionner", demande ?? { conservee_id: "", absorbee_id: "" }),
    onSuccess: (conservee) => {
      for (const cle of ["eleve", "eleves", "inscriptions", "classes"]) void client.invalidateQueries({ queryKey: [cle] });
      surOuvrir(conservee.eleve.id);
    },
  });
  if (!ouverte) {
    return (
      <div>
        <Bouton variante="secondaire" onClick={() => setOuverte(true)}>{t("fusion.ouvrir")}</Bouton>
      </div>
    );
  }
  const candidats = [...(doublons.data ?? []), ...(trouves.data ?? [])].filter((e, i, tous) => e.id !== fiche.eleve.id && tous.findIndex((x) => x.id === e.id) === i);
  return (
    <Section titre={t("fusion.titre")}>
      <p className="text-sm text-muet">{t("fusion.explication")}</p>
      <Champ id="fusion-recherche" type="search" libelle={t("fusion.chercher")} value={recherche} onChange={(e) => setRecherche(e.target.value)} />
      <ul className="flex flex-col gap-1">
        {candidats.map((e) => (
          <li key={e.id}>
            <button type="button" className={`text-sm underline ${e.id === autre ? "font-semibold text-encre" : "text-primaire"}`} onClick={() => setAutre(e.id)}>
              {`${e.prenoms} ${e.nom} · ${e.matricule_interne}${e.date_naissance === null ? "" : ` · ${dateCivile(e.date_naissance)}`}`}
            </button>
          </li>
        ))}
      </ul>
      {apercu.data === undefined ? null : (
        <>
          <CaseACocher id="fusion-garder" libelle={t("fusion.garder_celle_ci", { matricule: fiche.eleve.matricule_interne })} coche={garderCelleCi} surChangement={setGarderCelleCi} />
          <table className="w-full text-left text-sm">
            <thead>
              <tr className="text-muet">
                <th className="px-2 py-1 font-normal">{t("journal.champ")}</th>
                <th className="px-2 py-1 font-normal">{t("fusion.conservee")}</th>
                <th className="px-2 py-1 font-normal">{t("fusion.absorbee")}</th>
                <th className="px-2 py-1 font-normal">{t("fusion.resultat")}</th>
              </tr>
            </thead>
            <tbody>
              {COMPARES.map(([cle, valeur]) => (
                <tr key={cle} className="border-b border-muet/30">
                  <td className="px-2 py-1 text-muet">{t(cle)}</td>
                  <td className="px-2 py-1 text-encre">{valeur(apercu.data.conservee)}</td>
                  <td className="px-2 py-1 text-encre">{valeur(apercu.data.absorbee)}</td>
                  <td className="px-2 py-1 font-medium text-encre">{valeur(apercu.data)}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <p className="text-sm text-muet">
            {t("fusion.suite", { responsables: apercu.data.absorbee.responsables.length, inscriptions: apercu.data.absorbee.inscriptions.length })}
          </p>
          {apercu.data.annees_concurrentes.length === 0 ? null : (
            <p role="alert" className="text-sm text-rouge">{t("fusion.concurrentes", { annees: apercu.data.annees_concurrentes.join(", ") })}</p>
          )}
          <CaseACocher id="fusion-verifie" libelle={t("fusion.verifie")} coche={verifie} surChangement={setVerifie} />
          <div>
            <Bouton variante="danger" onClick={() => fusion.mutate()} disabled={!verifie || fusion.isPending || apercu.data.annees_concurrentes.length > 0}>
              {t("fusion.confirmer")}
            </Bouton>
          </div>
        </>
      )}
      <MessageErreur erreur={apercu.error ?? fusion.error} />
    </Section>
  );
}

/** Bandeau d'une fiche absorbée : elle reste lisible et mène à la fiche conservée. */
export function FicheAbsorbee({ conservee, surOuvrir }: { conservee: string; surOuvrir: (id: string) => void }) {
  return (
    <div role="status" className="flex flex-wrap items-center gap-3 rounded border border-ocre p-3 text-sm">
      <p className="text-ocre">{t("fusion.fiche_absorbee")}</p>
      <Bouton variante="secondaire" onClick={() => surOuvrir(conservee)}>{t("fusion.ouvrir_conservee")}</Bouton>
    </div>
  );
}
