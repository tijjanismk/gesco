import { describe, expect, it, vi } from "vitest";

import { ErreurApi, versErreurApi } from "./erreur";
import { transportHttp } from "./http";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async (commande: string, args: { nom: string }) => {
    if (args.nom === "audit_lister") throw { cle: "erreur.permission_refusee", jusqua: null, detail: null };
    return { commande, args };
  }),
}));

describe("couche d'accès (R-38, ADR-014)", () => {
  it("passe toutes les commandes par la commande Tauri unique `executer`", async () => {
    const { transportTauri } = await import("./tauri");
    await expect(transportTauri.appeler("etat_demarrage", null, null)).resolves.toEqual({
      commande: "executer",
      args: { nom: "etat_demarrage", jeton: null, entree: null },
    });
  });

  it("rend les refus de la garde en erreurs à clé de traduction", async () => {
    const { transportTauri } = await import("./tauri");
    const erreur = await transportTauri.appeler("audit_lister", "jeton", { limite: 10 }).catch((e: unknown) => e);
    expect(erreur).toBeInstanceOf(ErreurApi);
    expect((erreur as ErreurApi).cle).toBe("erreur.permission_refusee");
  });

  it("déclare le transport HTTP du mode réseau local", async () => {
    const appels: { url: string; init: RequestInit | undefined }[] = [];
    const recuperer = (async (url: string, init?: RequestInit) => {
      appels.push({ url, init });
      return new Response(JSON.stringify({ cle: "session.invalide", jusqua: null, detail: null }), { status: 401 });
    }) as typeof fetch;
    const http = transportHttp("https://serveur-ecole.local:8443", recuperer);
    const erreur = await http.appeler("sauvegarde_creer", "abc", null).catch((e: unknown) => e);
    expect((erreur as ErreurApi).cle).toBe("session.invalide");
    expect(appels[0]?.url).toBe("https://serveur-ecole.local:8443/api/commandes/sauvegarde_creer");
    expect(new Headers(appels[0]?.init?.headers).get("Authorization")).toBe("Bearer abc");
  });

  it("tolère une erreur de transport sans clé", () => {
    expect(versErreurApi(new Error("rompu")).cle).toBe("erreur.transport");
  });
});
