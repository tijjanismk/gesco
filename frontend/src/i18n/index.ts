import { catalogue, type Cle } from "./fr-ML";

export type { Cle };

/** Locale unique livrée (ADR-015) ; elle porte aussi les formats de date et de nombre. */
const LOCALE = "fr-ML";

/** Texte d'une clé, avec ses valeurs : « Utilisateur : {nom} ». */
export function t(cle: Cle, valeurs: Record<string, string | number> = {}): string {
  return catalogue[cle].replace(/\{(\w+)\}/g, (entier, nom: string) =>
    nom in valeurs ? String(valeurs[nom]) : entier,
  );
}

function estCle(cle: string): cle is Cle {
  return Object.hasOwn(catalogue, cle);
}

/** Message d'une erreur ou d'un avertissement rendu par le noyau, par sa clé. */
export function texteDeCle(cle: string, valeurs: Record<string, string | number> = {}): string {
  return estCle(cle) ? t(cle, valeurs) : t("erreur.inconnue");
}

/** Libellé d'une permission du catalogue ; son code si elle n'en a pas encore. */
export function libellePermission(code: string): string {
  return estCle(code) ? t(code) : code;
}

/** Libellé d'un paramètre d'établissement ; sa clé technique s'il n'en a pas encore. */
export function libelleParametre(cle: string): string {
  return estCle(cle) ? t(cle) : cle;
}

// Le Mali vit à l'heure UTC : l'affichage est celui du poste et du noyau.
const formatDateHeure = new Intl.DateTimeFormat(LOCALE, {
  dateStyle: "medium",
  timeStyle: "short",
  timeZone: "UTC",
});
const formatDate = new Intl.DateTimeFormat(LOCALE, { dateStyle: "long", timeZone: "UTC" });
const formatHeure = new Intl.DateTimeFormat(LOCALE, { timeStyle: "short", timeZone: "UTC" });
const formatNombre = new Intl.NumberFormat(LOCALE, { maximumFractionDigits: 1 });

export const dateHeure = (millis: number): string => formatDateHeure.format(millis);

/** Date civile `AAAA-MM-JJ` rendue par le noyau, affichée « 2 mars 2014 », et « 1er juin 2026 »
 *  le premier du mois, comme sur les documents imprimés. */
export function dateCivile(iso: string): string {
  const [a, m, j] = iso.split("-").map(Number);
  const texte = formatDate.format(Date.UTC(a ?? 0, (m ?? 1) - 1, j ?? 1));
  return j === 1 ? texte.replace(/^1 /, "1er ") : texte;
}

const formatMois = new Intl.DateTimeFormat(LOCALE, { month: "long", year: "numeric", timeZone: "UTC" });
const formatNomMois = new Intl.DateTimeFormat(LOCALE, { month: "long", timeZone: "UTC" });

/** Nom d'un mois, de 1 à 12 : « juillet ». */
export function nomMois(numero: number): string {
  return formatNomMois.format(Date.UTC(2026, numero - 1, 1));
}

/** Mois `AAAA-MM` rendu par le noyau, affiché « octobre 2026 ». */
export function moisCivil(mois: string): string {
  const [a, m] = mois.split("-").map(Number);
  return formatMois.format(Date.UTC(a ?? 0, (m ?? 1) - 1, 1));
}
export const date = (millis: number): string => formatDate.format(millis);
export const heure = (millis: number): string => formatHeure.format(millis);

/** Taille de fichier lisible : « 1,2 Mo ». */
/** Montant en francs CFA : « 150 000 F », séparateur de milliers en espace fine insécable. */
export function francs(montant: number): string {
  const signe = montant < 0 ? "−" : "";
  const chiffres = String(Math.abs(Math.trunc(montant))).replace(/\B(?=(\d{3})+(?!\d))/g, "\u202F");
  return `${signe}${chiffres}\u202FF`;
}

export function taille(octets: number): string {
  const unites = ["o", "Ko", "Mo", "Go"];
  let valeur = octets;
  let rang = 0;
  while (valeur >= 1024 && rang < unites.length - 1) {
    valeur /= 1024;
    rang += 1;
  }
  return `${formatNombre.format(valeur)} ${unites[rang] ?? "o"}`;
}
