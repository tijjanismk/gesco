// Appels au poste principal. Le témoin de session voyage seul (même adresse) ; l'élève ne voit
// jamais de mot technique : une réponse 401 le ramène à l'entrée, une réponse 503 dit d'attendre.

/** La session a fini : l'élève doit entrer de nouveau. */
export class SortieDemandee extends Error {}

/** Le poste principal est occupé ou fermé : la page réessaie seule. */
export class Occupe extends Error {}

/** Le poste principal a refusé (code faux, matricule bloqué, introuvable). */
export class Refus extends Error {
  constructor(
    readonly statut: number,
    readonly corps: unknown,
  ) {
    super(`refus ${statut}`);
  }
}

/** Adresse d'un fichier appelé par une leçon. */
export function adresseFichier(empreinte: string): string {
  return `/eleve/api/fichier/${empreinte}`;
}

/** Adresse du PDF d'une leçon. */
export function adressePdf(id: string): string {
  return `/eleve/api/pdf/${encodeURIComponent(id)}`;
}

async function demander(chemin: string, init?: RequestInit): Promise<Response> {
  let reponse: Response;
  try {
    reponse = await fetch(`/eleve/api/${chemin}`, {
      credentials: "same-origin",
      cache: "no-store",
      ...init,
    });
  } catch {
    // Le téléphone a perdu le Wi-Fi un instant : même conduite qu'un poste occupé.
    throw new Occupe();
  }
  if (reponse.status === 401) throw new SortieDemandee();
  if (reponse.status === 503 || reponse.status === 429) throw new Occupe();
  if (!reponse.ok) {
    const corps: unknown = await reponse.json().catch(() => null);
    throw new Refus(reponse.status, corps);
  }
  return reponse;
}

export async function lire<T>(chemin: string): Promise<T> {
  const reponse = await demander(chemin);
  return (await reponse.json()) as T;
}

export async function envoyer<T>(chemin: string, corps: unknown): Promise<T | null> {
  const reponse = await demander(chemin, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(corps),
  });
  if (reponse.status === 204) return null;
  return (await reponse.json()) as T;
}
