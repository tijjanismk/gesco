import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { Bouton } from "../../composants/Bouton";
import { Selection } from "../../composants/Champ";
import { MessageEnregistre, MessageErreur, Section } from "../../composants/Messages";
import { type Cle, t } from "../../i18n";

/** Documents dont l'école règle les colonnes, et leurs colonnes dans l'ordre imprimé ; le noyau
 *  en tient la même liste (`colonnes.<document>.<colonne>`). */
const DOCUMENTS: readonly { document: string; libelle: Cle; colonnes: readonly { colonne: string; libelle: Cle }[] }[] = [
  {
    document: "bulletin",
    libelle: "colonnes.document.bulletin",
    colonnes: [
      { colonne: "matiere", libelle: "colonnes.bulletin.matiere" },
      { colonne: "note_de_classe", libelle: "colonnes.bulletin.note_de_classe" },
      { colonne: "composition", libelle: "colonnes.bulletin.composition" },
      { colonne: "moyenne", libelle: "colonnes.bulletin.moyenne" },
      { colonne: "coefficient", libelle: "colonnes.bulletin.coefficient" },
      { colonne: "points", libelle: "colonnes.bulletin.points" },
      { colonne: "moyenne_classe", libelle: "colonnes.bulletin.moyenne_classe" },
      { colonne: "appreciation", libelle: "colonnes.bulletin.appreciation" },
    ],
  },
  {
    document: "recu",
    libelle: "colonnes.document.recu",
    colonnes: [
      { colonne: "imputation", libelle: "colonnes.recu.imputation" },
      { colonne: "montant", libelle: "colonnes.recu.montant" },
    ],
  },
  {
    document: "releve_compte",
    libelle: "colonnes.document.releve_compte",
    colonnes: [
      { colonne: "echeance", libelle: "colonnes.releve_compte.echeance" },
      { colonne: "date", libelle: "colonnes.releve_compte.date" },
      { colonne: "du", libelle: "colonnes.releve_compte.du" },
      { colonne: "remise", libelle: "colonnes.releve_compte.remise" },
      { colonne: "paye", libelle: "colonnes.releve_compte.paye" },
      { colonne: "reste", libelle: "colonnes.releve_compte.reste" },
      { colonne: "recu", libelle: "colonnes.releve_compte.recu" },
      { colonne: "date_versement", libelle: "colonnes.releve_compte.date_versement" },
      { colonne: "mode", libelle: "colonnes.releve_compte.mode" },
      { colonne: "montant", libelle: "colonnes.releve_compte.montant" },
    ],
  },
  {
    document: "bulletin_paie",
    libelle: "colonnes.document.bulletin_paie",
    colonnes: [
      { colonne: "libelle", libelle: "colonnes.bulletin_paie.libelle" },
      { colonne: "detail", libelle: "colonnes.bulletin_paie.detail" },
      { colonne: "montant", libelle: "colonnes.bulletin_paie.montant" },
    ],
  },
];

function maintenant(): number {
  return Date.now();
}

/** Largeur des colonnes des documents imprimés, en millimètres, colonne par colonne ; 0 ou vide
 *  garde la largeur livrée. Elle vaut pour chaque impression, réimpression comprise. */
export function LargeursColonnes() {
  const client = useQueryClient();
  const parametres = useQuery({ queryKey: ["parametres"], queryFn: () => appeler("parametres_lister", null) });
  const [document, setDocument] = useState("bulletin");
  const [saisies, setSaisies] = useState<Record<string, string>>({});
  const [enregistre, setEnregistre] = useState<number | null>(null);
  const choisi = DOCUMENTS.find((d) => d.document === document) ?? DOCUMENTS[0];
  const enBase = (cle: string): number => {
    const v = (parametres.data ?? []).find((p) => p.cle === cle)?.valeur;
    return v?.type === "entier" ? v.valeur : 0;
  };
  const cle = (colonne: string) => `colonnes.${choisi?.document ?? ""}.${colonne}`;
  const texte = (c: string) => saisies[c] ?? (enBase(c) === 0 ? "" : String(enBase(c)));
  const enregistrement = useMutation({
    mutationFn: async (valeurs: Record<string, number>) => {
      for (const [c, mm] of Object.entries(valeurs)) {
        if (mm !== enBase(c)) await appeler("parametre_definir", { cle: c, valeur: { type: "entier", valeur: mm } });
      }
    },
    onSuccess: () => {
      setSaisies({});
      setEnregistre(maintenant());
      void client.invalidateQueries({ queryKey: ["parametres"] });
    },
  });
  if (choisi === undefined || parametres.data === undefined) return <MessageErreur erreur={parametres.error} />;
  const cles = choisi.colonnes.map((c) => cle(c.colonne));
  const lues = Object.fromEntries(cles.map((c) => [c, texte(c).trim() === "" ? 0 : Number.parseInt(texte(c), 10)]));
  const illisible = Object.values(lues).some((mm) => !Number.isInteger(mm) || mm < 0);
  return (
    <Section titre={t("colonnes.titre")}>
      <p className="text-sm text-muet">{t("colonnes.aide")}</p>
      <div className="w-64">
        <Selection
          id="colonnes-document"
          libelle={t("colonnes.document")}
          valeur={choisi.document}
          surChangement={(v) => {
            setDocument(v);
            setSaisies({});
          }}
          options={DOCUMENTS.map((d) => ({ valeur: d.document, libelle: t(d.libelle) }))}
        />
      </div>
      <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
        {choisi.colonnes.map(({ colonne, libelle }) => {
          const c = cle(colonne);
          return (
            <div key={c} className="flex items-center gap-2">
              <label htmlFor={`colonne-${c}`} className="w-48 text-sm text-encre">{t(libelle)}</label>
              <input
                id={`colonne-${c}`}
                inputMode="numeric"
                placeholder={t("colonnes.livree")}
                className="w-24 rounded border border-muet bg-surface px-2 py-1 text-primaire"
                value={texte(c)}
                onChange={(e) => setSaisies((s) => ({ ...s, [c]: e.target.value }))}
              />
              <span className="text-sm text-muet">{t("colonnes.unite")}</span>
            </div>
          );
        })}
      </div>
      <div className="flex flex-wrap items-center gap-3">
        <Bouton onClick={() => enregistrement.mutate(lues)} disabled={illisible || enregistrement.isPending}>
          {t("commun.enregistrer")}
        </Bouton>
        <Bouton variante="secondaire" onClick={() => enregistrement.mutate(Object.fromEntries(cles.map((c) => [c, 0])))} disabled={enregistrement.isPending}>
          {t("colonnes.revenir")}
        </Bouton>
        <MessageEnregistre instant={enregistre} />
      </div>
      {illisible ? <p className="text-sm text-rouge">{t("colonnes.illisible")}</p> : null}
      <MessageErreur erreur={enregistrement.error} />
    </Section>
  );
}
