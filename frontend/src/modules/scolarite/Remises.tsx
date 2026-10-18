import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { useSmsActif } from "../../app/modules";
import { Bouton } from "../../composants/Bouton";
import { CaseACocher, Champ, Formulaire, Selection } from "../../composants/Champ";
import { MessageEnregistre, MessageErreur, Section } from "../../composants/Messages";
import { Tableau } from "../../composants/Tableau";
import { type Cle, dateHeure, francs, t } from "../../i18n";
import type { LigneRemise, RemisA, StatutRemiseDocument } from "../../types/gesco";

const STATUTS: Record<StatutRemiseDocument, Cle> = {
  a_remettre: "remises.statut.a_remettre",
  remis: "remises.statut.remis",
  signe: "remises.statut.signe",
  non_retire: "remises.statut.non_retire",
};

const DESTINATAIRES: Record<RemisA, Cle> = {
  responsable: "remises.a.responsable",
  eleve: "remises.a.eleve",
  personne_autorisee: "remises.a.personne_autorisee",
  tiers: "remises.a.tiers",
};

function etat(l: LigneRemise): string {
  const statut = t(STATUTS[l.remise.statut]);
  return l.recu_par === "" ? statut : `${statut} · ${l.recu_par}`;
}

/** Remise des bulletins (SPEC_PHASE_5 §6) : un bulletin publié n'est pas un bulletin remis.
 *  Chaque bulletin se remet et se signe ; ceux qui ne sont pas retirés au bout d'une semaine
 *  forment une liste, avec ce que la famille doit. */
export function Remises() {
  const annees = useQuery({ queryKey: ["annees"], queryFn: () => appeler("annees_lister", null) });
  const anneeId = annees.data?.find((a) => a.est_courante)?.id ?? null;
  const classes = useQuery({ queryKey: ["classes", anneeId], queryFn: () => appeler("classes_lister", { annee_id: anneeId ?? "" }), enabled: anneeId !== null });
  const ped = useQuery({ queryKey: ["pedagogie", anneeId], queryFn: () => appeler("pedagogie_lire", { id: anneeId ?? "" }), enabled: anneeId !== null });
  const [classeId, setClasseId] = useState("");
  const [periodeId, setPeriodeId] = useState("");
  const classe = (classes.data ?? []).find((c) => c.classe.id === classeId);
  const periodes = (ped.data?.periodes ?? []).filter((p) => classe !== undefined && p.periode.niveau_id === classe.classe.niveau_id);
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("remises.titre")}</h1>
      <MessageErreur erreur={annees.error ?? classes.error ?? ped.error} />
      <Section titre={t("remises.par_classe")}>
        <div className="flex flex-wrap gap-3">
          <Selection
            id="remises-classe"
            libelle={t("remises.classe")}
            valeur={classeId}
            surChangement={(v) => {
              setClasseId(v);
              setPeriodeId("");
            }}
            options={[{ valeur: "", libelle: t("remises.choisir") }, ...(classes.data ?? []).map((c) => ({ valeur: c.classe.id, libelle: c.classe.libelle }))]}
          />
          <Selection
            id="remises-periode"
            libelle={t("remises.periode")}
            valeur={periodeId}
            surChangement={setPeriodeId}
            options={[{ valeur: "", libelle: t("remises.choisir") }, ...periodes.map((p) => ({ valeur: p.periode.id, libelle: p.libelle }))]}
          />
        </div>
        {classeId !== "" && periodeId !== "" ? <RemisesDeLaClasse classeId={classeId} periodeId={periodeId} /> : null}
      </Section>
      <NonRetires />
    </div>
  );
}

function RemisesDeLaClasse({ classeId, periodeId }: { classeId: string; periodeId: string }) {
  const remises = useQuery({ queryKey: ["remises", classeId, periodeId], queryFn: () => appeler("remises_classe_lire", { classe_id: classeId, periode_id: periodeId }) });
  const [choisie, setChoisie] = useState<string | null>(null);
  const r = remises.data;
  if (r === undefined) return <MessageErreur erreur={remises.error} />;
  const ligne = r.lignes.find((l) => l.remise.id === choisie);
  return (
    <>
      {r.lignes.length === 0 ? <p className="text-sm text-muet">{t("remises.aucun_bulletin")}</p> : null}
      <Tableau
        nomExport="remise-des-bulletins"
        titre={t("remises.titre")}
        lignes={r.lignes}
        cle={(l) => l.remise.id}
        surSelection={r.droits.remettre ? (l) => setChoisie(l.remise.id) : undefined}
        colonnes={[
          { titre: t("remises.colonne.eleve"), valeur: (l) => l.eleve },
          { titre: t("remises.colonne.etat"), valeur: etat },
          { titre: t("remises.colonne.remis_le"), valeur: (l) => (l.remise.remis_le === null ? "" : dateHeure(l.remise.remis_le)) },
          { titre: t("remises.colonne.signataire"), valeur: (l) => l.remise.signataire },
        ]}
      />
      {ligne === undefined ? null : <FormulaireRemise key={ligne.remise.id} ligne={ligne} />}
    </>
  );
}

/** Remise d'un bulletin, ou sa signature s'il est déjà remis. Le guichet s'en sert aussi. */
export function FormulaireRemise({ ligne, surFait }: { ligne: LigneRemise; surFait?: (l: LigneRemise) => void }) {
  const client = useQueryClient();
  const [remisA, setRemisA] = useState<RemisA>(ligne.responsables.length > 0 ? "responsable" : "eleve");
  const [responsableId, setResponsableId] = useState(ligne.responsables[0]?.id ?? "");
  const [nom, setNom] = useState("");
  const [signe, setSigne] = useState(true);
  const [signataire, setSignataire] = useState(ligne.responsables[0]?.nom ?? "");
  const [enregistre, setEnregistre] = useState<number | null>(null);
  const fini = (l: LigneRemise) => {
    setEnregistre(Date.now());
    void client.invalidateQueries({ queryKey: ["remises"] });
    void client.invalidateQueries({ queryKey: ["non-retires"] });
    surFait?.(l);
  };
  const remise = useMutation({
    mutationFn: () =>
      appeler("bulletin_remettre", {
        id: ligne.remise.id,
        remis_a: remisA,
        responsable_id: remisA === "responsable" && responsableId !== "" ? responsableId : null,
        nom_retrait: nom,
        signe,
        signataire,
      }),
    onSuccess: fini,
  });
  const signature = useMutation({ mutationFn: () => appeler("bulletin_signer", { id: ligne.remise.id, signataire }), onSuccess: fini });
  const titre = t("remises.remettre_titre", { eleve: ligne.eleve, periode: ligne.periode });
  if (ligne.remise.statut === "signe") {
    return <p className="text-sm text-muet">{t("remises.deja_signe", { eleve: ligne.eleve })}</p>;
  }
  if (ligne.remise.statut === "remis") {
    return (
      <Formulaire surEnvoi={() => signature.mutate()}>
        <h3 className="text-sm font-semibold text-encre">{titre}</h3>
        <Champ id={`signataire-${ligne.remise.id}`} libelle={t("remises.signataire")} value={signataire} onChange={(e) => setSignataire(e.target.value)} />
        <MessageErreur erreur={signature.error} />
        <div className="flex items-center gap-3">
          <Bouton type="submit" disabled={signature.isPending}>{t("remises.enregistrer_signature")}</Bouton>
          <MessageEnregistre instant={enregistre} />
        </div>
      </Formulaire>
    );
  }
  return (
    <Formulaire surEnvoi={() => remise.mutate()}>
      <h3 className="text-sm font-semibold text-encre">{titre}</h3>
      <Selection
        id={`remis-a-${ligne.remise.id}`}
        libelle={t("remises.remis_a")}
        valeur={remisA}
        surChangement={(v) => setRemisA((Object.keys(DESTINATAIRES) as RemisA[]).find((r) => r === v) ?? "responsable")}
        options={(Object.keys(DESTINATAIRES) as RemisA[]).map((r) => ({ valeur: r, libelle: t(DESTINATAIRES[r]) }))}
      />
      {remisA === "responsable" ? (
        <Selection
          id={`remis-responsable-${ligne.remise.id}`}
          libelle={t("remises.responsable")}
          valeur={responsableId}
          surChangement={(v) => {
            setResponsableId(v);
            setSignataire(ligne.responsables.find((r) => r.id === v)?.nom ?? signataire);
          }}
          options={ligne.responsables.map((r) => ({ valeur: r.id, libelle: r.nom }))}
        />
      ) : null}
      {remisA === "personne_autorisee" || remisA === "tiers" ? (
        <Champ id={`remis-nom-${ligne.remise.id}`} libelle={t("remises.nom_retrait")} value={nom} onChange={(e) => setNom(e.target.value)} />
      ) : null}
      <CaseACocher id={`remis-signe-${ligne.remise.id}`} libelle={t("remises.signe_sur_place")} coche={signe} surChangement={setSigne} />
      {signe ? <Champ id={`remis-signataire-${ligne.remise.id}`} libelle={t("remises.signataire")} value={signataire} onChange={(e) => setSignataire(e.target.value)} /> : null}
      <MessageErreur erreur={remise.error} />
      <div className="flex items-center gap-3">
        <Bouton type="submit" disabled={remise.isPending}>{t("remises.remettre")}</Bouton>
        <MessageEnregistre instant={enregistre} />
      </div>
    </Formulaire>
  );
}

function NonRetires() {
  const smsActif = useSmsActif();
  const liste = useQuery({ queryKey: ["non-retires"], queryFn: () => appeler("bulletins_non_retires_lire", null) });
  const [renvoye, setRenvoye] = useState<string | null>(null);
  const renvoi = useMutation({
    mutationFn: (id: string) => appeler("bulletin_avis_renvoyer", { id }),
    onSuccess: (n) => setRenvoye(t("remises.avis_renvoye", { messages: n })),
  });
  const l = liste.data;
  return (
    <Section titre={t("remises.non_retires")}>
      <p className="text-sm text-muet">{t("remises.non_retires_aide")}</p>
      <MessageErreur erreur={liste.error ?? renvoi.error} />
      {renvoye === null ? null : <p role="status" className="text-sm text-vert">{renvoye}</p>}
      {l === undefined ? null : l.lignes.length === 0 ? (
        <p className="text-sm text-muet">{t("remises.aucun_non_retire")}</p>
      ) : (
        <>
          <Tableau
            nomExport="bulletins-non-retires"
            titre={t("remises.non_retires")}
            lignes={l.lignes}
            cle={(x) => x.remise.id}
            surSelection={l.droits.remettre && smsActif ? (x) => renvoi.mutate(x.remise.id) : undefined}
            colonnes={[
              { titre: t("remises.colonne.eleve"), valeur: (x) => x.eleve },
              { titre: t("remises.colonne.classe"), valeur: (x) => x.classe },
              { titre: t("remises.colonne.periode"), valeur: (x) => x.periode },
              { titre: t("remises.colonne.jours"), valeur: (x) => String(x.jours) },
              { titre: t("remises.colonne.impaye"), valeur: (x) => (x.impaye_echu === null ? "" : francs(x.impaye_echu)) },
            ]}
          />
          {l.droits.remettre && smsActif ? <p className="text-sm text-muet">{t("remises.renvoyer_aide")}</p> : null}
        </>
      )}
    </Section>
  );
}
