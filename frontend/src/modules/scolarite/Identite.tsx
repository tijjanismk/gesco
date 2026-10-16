import { CaseACocher, Champ, Selection } from "../../composants/Champ";
import { type Cle, t } from "../../i18n";
import type { Personne, SaisieIdentite, Sexe } from "../../types/gesco";

const SEXES: Record<Sexe, Cle> = { masculin: "personne.sexe.masculin", feminin: "personne.sexe.feminin" };

export function libelleSexe(sexe: Sexe | null): string {
  return sexe === null ? "" : t(SEXES[sexe]);
}

export function identiteVide(): SaisieIdentite {
  return {
    nom: "",
    prenoms: "",
    sexe: null,
    date_naissance: "",
    date_naissance_estimee: false,
    lieu_naissance: "",
    nationalite: t("personne.nationalite_defaut"),
    adresse: "",
    telephones: [],
  };
}

export function saisieDepuis(personne: Personne): SaisieIdentite {
  return {
    nom: personne.nom,
    prenoms: personne.prenoms,
    sexe: personne.sexe,
    date_naissance: personne.date_naissance ?? "",
    date_naissance_estimee: personne.date_naissance_estimee,
    lieu_naissance: personne.lieu_naissance,
    nationalite: personne.nationalite,
    adresse: personne.adresse,
    telephones: personne.telephones,
  };
}

/** Téléphone canonique lisible : `+223 76 12 34 56`. */
export function telephoneLisible(numero: string): string {
  const malien = /^\+223(\d{2})(\d{2})(\d{2})(\d{2})$/.exec(numero);
  return malien === null ? numero : `+223 ${malien.slice(1).join(" ")}`;
}

/** Champs d'identité communs à l'élève et au responsable. Le noyau nettoie et vérifie (R-02). */
export function ChampsIdentite({
  prefixe,
  valeur,
  surChangement,
  eleve,
  sexeImpose = null,
}: {
  prefixe: string;
  valeur: SaisieIdentite;
  surChangement: (valeur: SaisieIdentite) => void;
  eleve: boolean;
  /** Sexe qu'impose le lien d'un responsable (père, mère) : il ne se redemande pas. */
  sexeImpose?: Sexe | null;
}) {
  const champ = (cle: "nom" | "prenoms" | "lieu_naissance" | "nationalite" | "adresse", libelle: Cle) => (
    <Champ id={`${prefixe}-${cle}`} libelle={t(libelle)} value={valeur[cle]} onChange={(e) => surChangement({ ...valeur, [cle]: e.target.value })} />
  );
  const telephone = (rang: 0 | 1, libelle: Cle) => (
    <Champ
      id={`${prefixe}-telephone-${rang}`}
      type="tel"
      libelle={t(libelle)}
      value={valeur.telephones[rang] ?? ""}
      onChange={(e) => {
        const telephones = [valeur.telephones[0] ?? "", valeur.telephones[1] ?? ""];
        telephones[rang] = e.target.value;
        surChangement({ ...valeur, telephones: telephones.filter((x) => x !== "") });
      }}
    />
  );
  return (
    <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
      {champ("prenoms", "personne.prenoms")}
      {champ("nom", "personne.nom")}
      {sexeImpose !== null ? null : <Selection
        id={`${prefixe}-sexe`}
        libelle={t("personne.sexe")}
        valeur={valeur.sexe ?? ""}
        surChangement={(v) => surChangement({ ...valeur, sexe: v === "masculin" || v === "feminin" ? v : null })}
        options={[
          { valeur: "", libelle: t("personne.sexe.choisir") },
          { valeur: "masculin", libelle: t(SEXES.masculin) },
          { valeur: "feminin", libelle: t(SEXES.feminin) },
        ]}
      />}
      {eleve ? (
        <>
          <Champ
            id={`${prefixe}-naissance`}
            type="date"
            libelle={t("personne.date_naissance")}
            value={valeur.date_naissance}
            onChange={(e) => surChangement({ ...valeur, date_naissance: e.target.value })}
          />
          <CaseACocher
            id={`${prefixe}-estimee`}
            libelle={t("personne.date_estimee")}
            coche={valeur.date_naissance_estimee}
            surChangement={(coche) => surChangement({ ...valeur, date_naissance_estimee: coche })}
          />
          {champ("lieu_naissance", "personne.lieu_naissance")}
          {champ("nationalite", "personne.nationalite")}
        </>
      ) : null}
      {telephone(0, "personne.telephone")}
      {telephone(1, "personne.telephone_2")}
      {champ("adresse", "personne.adresse")}
    </div>
  );
}
