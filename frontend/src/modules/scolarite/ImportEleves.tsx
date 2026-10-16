import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { choisirTableur } from "../../api/fichiers";
import { Bouton } from "../../composants/Bouton";
import { Selection } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { type Cle, dateCivile, t, texteDeCle } from "../../i18n";
import type { Analyse, Apercu, Champ, Decoupe, Disposition, EtatLigne, Rapport } from "../../types/gesco";
import { libelleSexe } from "./Identite";

const CHAMPS: Record<Champ, Cle> = {
  ignorer: "import.champ.ignorer",
  nom: "import.champ.nom",
  prenoms: "import.champ.prenoms",
  nom_et_prenoms: "import.champ.nom_et_prenoms",
  sexe: "import.champ.sexe",
  date_naissance: "import.champ.date_naissance",
  lieu_naissance: "import.champ.lieu_naissance",
  classe: "import.champ.classe",
  matricule_interne: "import.champ.matricule_interne",
  matricule_national: "import.champ.matricule_national",
  adresse: "import.champ.adresse",
  responsable_nom: "import.champ.responsable_nom",
  responsable_telephone: "import.champ.responsable_telephone",
};

const DISPOSITIONS: Record<Disposition, Cle> = {
  colonne_classe: "import.disposition.colonne_classe",
  feuille_par_classe: "import.disposition.feuille_par_classe",
};

const DECOUPES: Record<Decoupe, Cle> = {
  nom_d_abord: "import.decoupe.nom_d_abord",
  nom_en_dernier: "import.decoupe.nom_en_dernier",
};

const ETATS: Record<EtatLigne, Cle> = {
  valide: "import.etat.valide",
  avertissement: "import.etat.avertissement",
  erreur: "import.etat.erreur",
};

const ETAPES = ["import.etape.fichier", "import.etape.colonnes", "import.etape.apercu", "import.etape.confirmation"] as const;

/** Import des élèves en quatre temps (SPEC_PHASE_2 §10, R-47) : fichier, colonnes, aperçu,
 *  confirmation. Le fichier est lu par le noyau ; l'écran ne fait que montrer et corriger. */
export function ImportEleves({ surTermine }: { surTermine: () => void }) {
  const client = useQueryClient();
  const [analyse, setAnalyse] = useState<Analyse | null>(null);
  const [champs, setChamps] = useState<Champ[]>([]);
  const [disposition, setDisposition] = useState<Disposition>("colonne_classe");
  const [decoupe, setDecoupe] = useState<Decoupe>("nom_d_abord");
  const [apercu, setApercu] = useState<Apercu | null>(null);
  const [rapport, setRapport] = useState<Rapport | null>(null);
  const lecture = useMutation({
    mutationFn: async () => {
      const chemin = await choisirTableur(t("import.filtre"));
      return chemin === null ? null : appeler("import_eleves_analyser", { chemin });
    },
    onSuccess: (resultat) => {
      if (resultat === null) return;
      setAnalyse(resultat);
      setChamps(resultat.champs);
      setDisposition(resultat.disposition);
      setApercu(null);
    },
  });
  const reglages = analyse === null ? null : { fichier_id: analyse.fichier_id, champs, disposition, decoupe };
  const validation = useMutation({
    mutationFn: () => appeler("import_eleves_valider", reglages ?? { fichier_id: "", champs: [], disposition, decoupe }),
    onSuccess: setApercu,
  });
  const application = useMutation({
    mutationFn: () => appeler("import_eleves_appliquer", reglages ?? { fichier_id: "", champs: [], disposition, decoupe }),
    onSuccess: (resultat) => {
      setRapport(resultat);
      for (const cle of ["eleves", "inscriptions", "classes", "sauvegardes"]) void client.invalidateQueries({ queryKey: [cle] });
    },
  });
  const etape = rapport !== null ? 3 : apercu !== null ? 2 : analyse !== null ? 1 : 0;
  const premiereFeuille = analyse?.feuilles[0];
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("import.titre")}</h1>
      <ol className="flex flex-wrap gap-4 text-sm">
        {ETAPES.map((cle, i) => (
          <li key={cle} className={i === etape ? "font-semibold text-primaire" : "text-muet"}>{`${i + 1}. ${t(cle)}`}</li>
        ))}
      </ol>
      {rapport !== null ? (
        <Section titre={t("import.termine")}>
          <ul role="status" className="list-inside list-disc text-sm text-encre">
            <li>{t("import.rapport.importes", { nombre: rapport.importes })}</li>
            <li>{t("import.rapport.responsables", { crees: rapport.responsables_crees, retrouves: rapport.responsables_retrouves })}</li>
            <li>{t("import.rapport.ignorees", { nombre: rapport.ignorees })}</li>
          </ul>
          <p className="text-sm text-muet">{t("import.rapport.suite")}</p>
          <div><Bouton onClick={surTermine}>{t("import.fermer")}</Bouton></div>
        </Section>
      ) : (
        <>
          <Section titre={t("import.etape.fichier")}>
            <p className="text-sm text-muet">{t("import.explication")}</p>
            <div className="flex flex-wrap items-center gap-3">
              <Bouton variante={analyse === null ? "principal" : "secondaire"} disabled={lecture.isPending} onClick={() => lecture.mutate()}>
                {t("import.choisir")}
              </Bouton>
              {analyse === null ? null : (
                <p className="text-sm text-encre">
                  {t("import.fichier_lu", { feuilles: analyse.feuilles.length, lignes: analyse.feuilles.reduce((total, f) => total + f.lignes, 0) })}
                </p>
              )}
            </div>
            <MessageErreur erreur={lecture.error} />
          </Section>
          {analyse !== null && premiereFeuille !== undefined ? (
            <Section titre={t("import.etape.colonnes")}>
              <div className="flex flex-wrap gap-3">
                <Selection
                  id="import-disposition"
                  libelle={t("import.disposition")}
                  valeur={disposition}
                  surChangement={(v) => setDisposition(v as Disposition)}
                  options={(Object.keys(DISPOSITIONS) as Disposition[]).map((d) => ({ valeur: d, libelle: t(DISPOSITIONS[d]) }))}
                />
                {champs.includes("nom_et_prenoms") ? (
                  <Selection
                    id="import-decoupe"
                    libelle={t("import.decoupe")}
                    valeur={decoupe}
                    surChangement={(v) => setDecoupe(v as Decoupe)}
                    options={(Object.keys(DECOUPES) as Decoupe[]).map((d) => ({ valeur: d, libelle: t(DECOUPES[d]) }))}
                  />
                ) : null}
              </div>
              {disposition === "feuille_par_classe" ? (
                <p className="text-sm text-muet">{t("import.feuilles", { feuilles: analyse.feuilles.map((f) => f.nom).join(", ") })}</p>
              ) : null}
              <div className="overflow-x-auto">
                <table className="w-full text-left text-sm">
                  <thead>
                    <tr className="text-muet">
                      <th className="px-2 py-1 font-normal">{t("import.colonne")}</th>
                      <th className="px-2 py-1 font-normal">{t("import.exemples")}</th>
                      <th className="px-2 py-1 font-normal">{t("import.devient")}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {premiereFeuille.entetes.map((entete, i) => (
                      <tr key={`${i}-${entete}`} className="border-b border-muet/30">
                        <td className="px-2 py-1 font-medium text-encre">{entete}</td>
                        <td className="px-2 py-1 text-muet">{premiereFeuille.apercu.map((l) => l[i] ?? "").filter((v) => v !== "").slice(0, 3).join(" · ")}</td>
                        <td className="px-2 py-1">
                          <Selection
                            id={`import-champ-${i}`}
                            libelle=""
                            valeur={champs[i] ?? "ignorer"}
                            surChangement={(v) => setChamps(champs.map((c, j) => (j === i ? (v as Champ) : c)))}
                            options={(Object.keys(CHAMPS) as Champ[]).map((c) => ({ valeur: c, libelle: t(CHAMPS[c]) }))}
                          />
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
              <p className="text-sm text-muet">
                {analyse.matricule_manuel ? t("import.matricule_manuel") : t("import.matricule_genere")}
                {` ${t("import.classes_connues", { classes: analyse.classes.join(", ") })}`}
              </p>
              <MessageErreur erreur={validation.error} />
              <div><Bouton disabled={validation.isPending} onClick={() => validation.mutate()}>{t("import.verifier")}</Bouton></div>
            </Section>
          ) : null}
          {apercu !== null ? (
            <Section titre={t("import.etape.apercu")}>
              <p role="status" className="text-sm text-encre">
                {t("import.bilan", { valides: apercu.valides, avertissements: apercu.avertissements, erreurs: apercu.erreurs })}
              </p>
              {apercu.classes_absentes.length === 0 ? null : (
                <p role="alert" className="text-sm text-rouge">{t("import.classes_absentes", { classes: apercu.classes_absentes.join(", ") })}</p>
              )}
              <div className="max-h-96 overflow-auto">
                <table className="w-full text-left font-condensee text-sm">
                  <thead>
                    <tr className="text-muet">
                      {(["import.ligne", "personne.prenoms", "personne.nom", "personne.sexe", "personne.date_naissance", "inscriptions.classe", "import.etat", "import.motifs"] as const).map((cle) => (
                        <th key={cle} className="px-2 py-1 font-normal">{t(cle)}</th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {apercu.lignes.map((l) => (
                      <tr key={`${l.feuille}-${l.numero}`} className={`border-b border-muet/30 ${l.etat === "erreur" ? "text-rouge" : l.etat === "avertissement" ? "text-ocre" : "text-encre"}`}>
                        <td className="px-2 py-1">{apercu.lignes.some((x) => x.feuille !== l.feuille) ? `${l.feuille} · ${l.numero}` : String(l.numero)}</td>
                        <td className="px-2 py-1">{l.prenoms}</td>
                        <td className="px-2 py-1">{l.nom}</td>
                        <td className="px-2 py-1">{libelleSexe(l.sexe)}</td>
                        <td className="px-2 py-1">{l.date_naissance === null ? "" : dateCivile(l.date_naissance)}</td>
                        <td className="px-2 py-1">{l.classe}</td>
                        <td className="px-2 py-1">{t(ETATS[l.etat])}</td>
                        <td className="px-2 py-1">{[...l.motifs.map((m) => texteDeCle(m, {})), ...(l.doublon_de === null ? [] : [l.doublon_de])].join(" · ")}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
              <p className="text-sm text-muet">{t("import.confirmation_aide")}</p>
              <MessageErreur erreur={application.error} />
              <div className="flex flex-wrap gap-2">
                <Bouton disabled={application.isPending || apercu.valides + apercu.avertissements === 0} onClick={() => application.mutate()}>
                  {t("import.appliquer", { nombre: apercu.valides + apercu.avertissements })}
                </Bouton>
                <Bouton variante="secondaire" onClick={surTermine}>{t("commun.annuler")}</Bouton>
              </div>
            </Section>
          ) : null}
        </>
      )}
    </div>
  );
}
