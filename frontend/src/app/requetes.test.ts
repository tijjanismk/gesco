// Écrans qui se relisent à plusieurs postes : les listes et les fiches à chaque ouverture, les
// données de référence après un délai court, et tout de suite après un refus.
import { afterEach, describe, expect, it } from "vitest";

import { ErreurApi } from "../api";

import { clientRequetes } from "./requetes";

function compteur() {
  let lectures = 0;
  return { lire: () => Promise.resolve(++lectures), nombre: () => lectures };
}

async function ecrire(resultat: () => Promise<unknown>): Promise<void> {
  const ecriture = clientRequetes.getMutationCache().build(clientRequetes, { mutationFn: resultat });
  await ecriture.execute(undefined).catch(() => undefined);
}

afterEach(() => clientRequetes.clear());

describe("relecture des écrans", () => {
  it("relit une liste à chaque ouverture, une donnée de référence après son délai", async () => {
    const classes = compteur();
    const eleves = compteur();
    for (let i = 0; i < 2; i += 1) {
      await clientRequetes.fetchQuery({ queryKey: ["classes", "2026"], queryFn: classes.lire });
      await clientRequetes.fetchQuery({ queryKey: ["eleves"], queryFn: eleves.lire });
    }
    expect(classes.nombre()).toBe(1);
    expect(eleves.nombre()).toBe(2);
  });

  it("ne garde pas le délai après une écriture de ce poste", async () => {
    const classes = compteur();
    await clientRequetes.fetchQuery({ queryKey: ["classes", "2026"], queryFn: classes.lire });
    await ecrire(() => Promise.resolve(null));
    await clientRequetes.fetchQuery({ queryKey: ["classes", "2026"], queryFn: classes.lire });
    expect(classes.nombre()).toBe(2);
  });

  it("ne garde pas le délai après un refus, sauf si la liaison ou la session est en cause", async () => {
    const classes = compteur();
    const lire = () => clientRequetes.fetchQuery({ queryKey: ["classes", "2026"], queryFn: classes.lire });
    const refus = (cle: string) => () => Promise.reject(new ErreurApi({ cle, jusqua: null, detail: null }));
    await lire();
    for (const cle of ["reseau.poste_principal_injoignable", "session.verrouillee", "poste.occupe.cloture"]) {
      await ecrire(refus(cle));
      await lire();
    }
    expect(classes.nombre()).toBe(1);
    await ecrire(refus("classe.libelle_pris"));
    await lire();
    expect(classes.nombre()).toBe(2);
  });
});
