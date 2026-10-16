import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { Bouton } from "../../composants/Bouton";
import { CaseACocher, Champ, Formulaire, Selection } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { type Cle, t } from "../../i18n";
import type { FicheEleve, Lien, Qualites, ResponsableDeLEleve, SaisieResponsable, Sexe } from "../../types/gesco";
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
                  <p className="font-condensee text-encre">{r.identite.telephones.map(telephoneLisible).join(" · ")}</p>
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
