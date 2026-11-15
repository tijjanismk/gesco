import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { choisirClasseur } from "../../api/fichiers";
import type { IdSection } from "../../app/navigation";
import { ApercuPdf } from "../../composants/ApercuPdf";
import { Bouton } from "../../composants/Bouton";
import { Champ, Selection } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { dateCivile, t } from "../../i18n";
import type { RapportEcole, SectionRapport, TableauRapport, TypeRapport } from "../../types/gesco";

/** Rapports de rentrée (rapport succinct, rapport de rentrée) : la liste de l'année, puis un
 *  rapport ouvert — réglages, en-tête, textes rédigés, tableaux calculés à la date d'arrêt —,
 *  avec l'aperçu, l'impression, l'export Excel des tableaux, l'arrêt et la réouverture. */
export function Rapports({ surAller }: { surAller: (section: IdSection) => void }) {
  const [anneeId, setAnneeId] = useState<string | null>(null);
  const [ouvert, setOuvert] = useState<string | null>(null);
  const liste = useQuery({ queryKey: ["rapports", anneeId], queryFn: () => appeler("rapports_lister", { annee_id: anneeId }) });
  const client = useQueryClient();
  const preparer = useMutation({
    mutationFn: (v: { annee: string; type: TypeRapport }) => appeler("rapport_preparer", { annee_id: v.annee, type_rapport: v.type }),
    onSuccess: (r) => {
      client.setQueryData(["rapport", r.id], r);
      void client.invalidateQueries({ queryKey: ["rapports"] });
      setOuvert(r.id);
    },
  });
  const l = liste.data;
  if (ouvert !== null) return <RapportOuvert id={ouvert} surFermer={() => setOuvert(null)} surAller={surAller} />;
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("rapports.titre")}</h1>
      <MessageErreur erreur={liste.error ?? preparer.error} />
      {l === undefined ? null : (
        <>
          {l.annees.length > 0 ? (
            <div className="flex flex-wrap items-end gap-3">
              <Selection id="rapports-annee" libelle={t("rapports.annee")} valeur={l.annee_id ?? ""} surChangement={(v) => setAnneeId(v)} options={l.annees.map((a) => ({ valeur: a.id, libelle: a.libelle }))} />
            </div>
          ) : null}
          <ul className="flex flex-col gap-2">
            {l.rapports.map((r) => (
              <li key={r.type_rapport} className="flex flex-wrap items-center gap-3 rounded border border-muet/30 bg-surface p-3">
                <span className="font-medium text-encre">{t(`rapports.type.${r.type_rapport}`)}</span>
                <span className="text-sm text-muet">
                  {r.etat === null ? t("rapports.pas_prepare") : t(`rapports.etat.${r.etat}`)}
                  {r.date_arret === null ? "" : ` · ${t("rapports.arrete_au", { date: dateCivile(r.date_arret) })}`}
                </span>
                {r.id !== null ? (
                  <Bouton variante="secondaire" onClick={() => setOuvert(r.id)}>
                    {t("rapports.ouvrir")}
                  </Bouton>
                ) : l.annee_id !== null ? (
                  <Bouton variante="secondaire" disabled={preparer.isPending} onClick={() => l.annee_id !== null && preparer.mutate({ annee: l.annee_id, type: r.type_rapport })}>
                    {t("rapports.preparer")}
                  </Bouton>
                ) : null}
              </li>
            ))}
          </ul>
        </>
      )}
    </div>
  );
}

type Exportation = { chemin: string } | { erreur: unknown } | null;

function RapportOuvert({ id, surFermer, surAller }: { id: string; surFermer: () => void; surAller: (section: IdSection) => void }) {
  const cle = ["rapport", id] as const;
  const rapport = useQuery({ queryKey: cle, queryFn: () => appeler("rapport_lire", { id }) });
  const client = useQueryClient();
  const [pdf, setPdf] = useState<string | null>(null);
  const [exportation, setExportation] = useState<Exportation>(null);
  const poser = (r: RapportEcole) => {
    client.setQueryData(cle, r);
    void client.invalidateQueries({ queryKey: ["rapports"] });
  };
  const regler = useMutation({
    mutationFn: (v: { date_arret: string; lieu: string; date_signature: string | null; qualite_signataire: string }) => appeler("rapport_regler", { id, ...v }),
    onSuccess: poser,
  });
  const ecrire = useMutation({
    mutationFn: (v: { section: string; texte: string; lue_le: number }) => appeler("rapport_texte_enregistrer", { id, ...v }),
    onSuccess: poser,
  });
  const apercu = useMutation({ mutationFn: () => appeler("rapport_apercu", { id }), onSuccess: (d) => setPdf(d.pdf) });
  const arreter = useMutation({ mutationFn: () => appeler("rapport_arreter", { id }), onSuccess: poser });
  const rouvrir = useMutation({ mutationFn: () => appeler("rapport_rouvrir", { id }), onSuccess: poser });
  const r = rapport.data;
  if (r === undefined) return <MessageErreur erreur={rapport.error} />;
  const fige = r.etat === "arrete";
  const dernier = r.documents.at(-1);
  const exporter = async () => {
    setExportation(null);
    try {
      const titre = `${r.contenu.titre} ${r.contenu.annee}`;
      const chemin = await choisirClasseur(titre, t("commun.classeur_excel"));
      if (chemin === null) return;
      const feuilles = r.contenu.tableaux.map((tb) => ({ titre: tb.titre, entetes: tb.colonnes, lignes: tb.lignes }));
      const ecrit = await appeler("statistiques_exporter", { chemin, titre, feuilles });
      setExportation({ chemin: ecrit.chemin });
    } catch (erreur) {
      setExportation({ erreur });
    }
  };
  const reglages = { date_arret: r.date_arret, lieu: r.lieu, date_signature: r.date_signature, qualite_signataire: r.qualite_signataire };
  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center gap-3">
        <Bouton variante="secondaire" onClick={surFermer}>
          {t("rapports.retour")}
        </Bouton>
        <h1 className="text-xl font-semibold text-encre">
          {r.contenu.titre} · {r.contenu.annee}
        </h1>
        <span className="text-sm text-muet">
          {t(`rapports.etat.${r.etat}`)} · {t("rapports.version", { n: String(r.version) })}
        </span>
      </div>
      <MessageErreur erreur={rapport.error ?? regler.error ?? ecrire.error ?? apercu.error ?? arreter.error ?? rouvrir.error} />
      <Section titre={t("rapports.reglages")}>
        <div className="flex flex-wrap items-end gap-3">
          <Champ
            id="rapport-date-arret"
            type="date"
            libelle={t("rapports.date_arret")}
            defaultValue={r.date_arret}
            disabled={fige}
            onBlur={(e) => e.target.value !== "" && e.target.value !== r.date_arret && regler.mutate({ ...reglages, date_arret: e.target.value })}
          />
          <Champ id="rapport-lieu" libelle={t("rapports.lieu")} defaultValue={r.lieu} disabled={fige} onBlur={(e) => e.target.value !== r.lieu && regler.mutate({ ...reglages, lieu: e.target.value })} />
          <Champ
            id="rapport-date-signature"
            type="date"
            libelle={t("rapports.date_signature")}
            defaultValue={r.date_signature ?? ""}
            disabled={fige}
            onBlur={(e) => e.target.value !== (r.date_signature ?? "") && regler.mutate({ ...reglages, date_signature: e.target.value === "" ? null : e.target.value })}
          />
          <Champ
            id="rapport-qualite"
            libelle={t("rapports.qualite")}
            defaultValue={r.qualite_signataire}
            disabled={fige}
            list="rapport-qualites"
            onBlur={(e) => e.target.value !== r.qualite_signataire && regler.mutate({ ...reglages, qualite_signataire: e.target.value })}
          />
          <datalist id="rapport-qualites">
            {["Le Directeur", "La Directrice", "Le Proviseur", "Le Promoteur"].map((q) => (
              <option key={q} value={q} />
            ))}
          </datalist>
        </div>
      </Section>
      <Section titre={t("rapports.entete")}>
        <dl className="grid grid-cols-1 gap-x-6 gap-y-1 text-sm sm:grid-cols-2">
          {r.contenu.entete.map((e) => (
            <div key={e.libelle} className="flex gap-2">
              <dt className="font-medium text-encre">{e.libelle} :</dt>
              <dd className="text-encre">{e.valeur}</dd>
            </div>
          ))}
        </dl>
        <Bouton variante="secondaire" onClick={() => surAller("statistiques")}>
          {t("rapports.corriger_fiche")}
        </Bouton>
      </Section>
      {r.sections.map((s) => (
        <Redaction key={`${s.code}-${s.lue_le}`} section={s} fige={fige} enCours={ecrire.isPending} surEnregistrer={(texte) => ecrire.mutate({ section: s.code, texte, lue_le: s.lue_le })} />
      ))}
      {r.contenu.tableaux.map((tb) => (
        <TableauLu key={tb.code} tableau={tb} surAller={surAller} />
      ))}
      <div className="flex flex-wrap items-center gap-3">
        <Bouton variante="secondaire" onClick={() => apercu.mutate()} disabled={apercu.isPending}>
          {fige ? t("rapports.reimprimer") : t("rapports.apercu")}
        </Bouton>
        <Bouton variante="secondaire" onClick={() => void exporter()}>
          {t("rapports.exporter")}
        </Bouton>
        {fige ? (
          <Bouton variante="secondaire" onClick={() => rouvrir.mutate()} disabled={rouvrir.isPending}>
            {t("rapports.rouvrir")}
          </Bouton>
        ) : (
          <Bouton onClick={() => arreter.mutate()} disabled={arreter.isPending}>
            {t("rapports.arreter")}
          </Bouton>
        )}
      </div>
      {exportation === null ? null : "chemin" in exportation ? <p className="text-sm text-encre">{t("rapports.exporte", { chemin: exportation.chemin })}</p> : <MessageErreur erreur={exportation.erreur} />}
      {dernier === undefined ? null : <p className="text-sm text-muet">{t("rapports.historique", { n: String(r.documents.length), date: new Date(dernier.produit_le).toLocaleDateString("fr-FR") })}</p>}
      {pdf === null ? null : <ApercuPdf base64={pdf} />}
    </div>
  );
}

/** Une section rédigée : enregistrée en quittant la zone ; le texte de l'an dernier et les
 *  chiffres du jour s'insèrent d'un bouton. */
function Redaction({ section, fige, enCours, surEnregistrer }: { section: SectionRapport; fige: boolean; enCours: boolean; surEnregistrer: (texte: string) => void }) {
  const [texte, setTexte] = useState(section.texte);
  const id = `rapport-section-${section.code}`;
  const garder = (v: string) => {
    setTexte(v);
    if (v !== section.texte) surEnregistrer(v);
  };
  return (
    <Section titre={section.titre}>
      <textarea
        id={id}
        aria-label={section.titre}
        rows={5}
        className="rounded border border-muet bg-surface px-2 py-1.5 text-primaire focus:outline-2 focus:outline-primaire disabled:bg-fond disabled:text-encre disabled:opacity-70"
        value={texte}
        disabled={fige}
        onChange={(e) => setTexte(e.target.value)}
        onBlur={() => texte !== section.texte && surEnregistrer(texte)}
      />
      {fige ? null : (
        <div className="flex flex-wrap gap-2">
          {section.an_dernier !== null ? (
            <Bouton variante="secondaire" disabled={enCours} onClick={() => section.an_dernier !== null && garder(section.an_dernier)}>
              {t("rapports.reprendre_an_dernier")}
            </Bouton>
          ) : null}
          {section.propose !== null ? (
            <>
              <p className="w-full text-sm text-muet">{section.propose}</p>
              <Bouton variante="secondaire" disabled={enCours} onClick={() => section.propose !== null && garder(texte === "" ? section.propose : `${texte}\n${section.propose}`)}>
                {t("rapports.inserer_chiffres")}
              </Bouton>
            </>
          ) : null}
        </div>
      )}
    </Section>
  );
}

/** Un tableau calculé, en lecture : la donnée se corrige dans son écran. */
function TableauLu({ tableau, surAller }: { tableau: TableauRapport; surAller: (section: IdSection) => void }) {
  return (
    <Section titre={tableau.titre}>
      <div className="overflow-x-auto">
        <table className="min-w-full border-collapse text-sm">
          <thead>
            <tr>
              {tableau.colonnes.map((c, i) => (
                <th key={i} scope="col" className="border border-muet/40 px-2 py-1 text-left font-medium text-encre">
                  {c}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {tableau.lignes.map((l, i) => (
              <tr key={i} className={l[0]?.startsWith("Total") ? "font-semibold" : ""}>
                {l.map((c, j) => (
                  <td key={j} className={`border border-muet/40 px-2 py-1 text-encre ${j === 0 ? "text-left" : "text-center"}`}>
                    {c}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div>
        <Bouton variante="secondaire" onClick={() => surAller(tableau.lien === "inventaire" ? "inventaire" : "statistiques")}>
          {tableau.lien === "inventaire" ? t("rapports.corriger_inventaire") : t("rapports.corriger_statistiques")}
        </Bouton>
      </div>
    </Section>
  );
}
