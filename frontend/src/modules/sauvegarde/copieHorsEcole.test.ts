import { describe, expect, it } from "vitest";

import type { Destination, DossierSynchronise } from "../../types/gesco";
import { aProposer, avecDossier, copiesParMois, sansDossier, volumeCopie, volumeMensuel } from "./copieHorsEcole";

const ONEDRIVE: DossierSynchronise = {
  fournisseur: "one_drive",
  nom: "OneDrive",
  racine: "C:\\Users\\ecole\\OneDrive",
  propose: "C:\\Users\\ecole\\OneDrive\\GESCO",
};
const DRIVE: DossierSynchronise = {
  fournisseur: "google_drive",
  nom: "Google Drive",
  racine: "G:\\Mon Drive",
  propose: "G:\\Mon Drive\\GESCO",
};
const USB: Destination = { chemin: "E:\\sauvegardes", sorte: "amovible", amovible: true };

describe("copie hors de l'école", () => {
  it("compte les copies d'un mois selon le rythme", () => {
    expect(copiesParMois("demande")).toBe(0);
    expect(copiesParMois("mois")).toBe(1);
    expect(copiesParMois("semaine")).toBeCloseTo(4.33, 2);
  });

  it("dit ce qui partira par Internet, une copie par dossier choisi", () => {
    expect(volumeCopie(4_500_000, 1)).toBe(4_500_000);
    expect(volumeCopie(4_500_000, 2)).toBe(9_000_000);
    expect(volumeCopie(null, 1)).toBeNull();
    expect(volumeCopie(4_500_000, 0)).toBeNull();
  });

  it("donne l'ordre de grandeur par mois, rien à la demande", () => {
    expect(volumeMensuel(1_200_000, 1, "semaine")).toBeCloseTo(5_200_000, 0);
    expect(volumeMensuel(1_200_000, 2, "mois")).toBe(2_400_000);
    expect(volumeMensuel(1_200_000, 1, "demande")).toBeNull();
    expect(volumeMensuel(null, 1, "semaine")).toBeNull();
  });

  it("ne propose pas un dossier déjà choisi, quelle que soit sa casse", () => {
    expect(aProposer([DRIVE, ONEDRIVE], [])).toEqual([DRIVE, ONEDRIVE]);
    expect(aProposer([DRIVE, ONEDRIVE], ["c:\\users\\ecole\\onedrive\\gesco\\"])).toEqual([DRIVE]);
  });

  it("ajoute un dossier choisi sans toucher aux autres destinations", () => {
    expect(avecDossier([USB], ONEDRIVE.propose)).toEqual([
      USB,
      { chemin: ONEDRIVE.propose, sorte: "synchronise", amovible: false },
    ]);
    // Un dossier de la 1.2 au même endroit ne se double pas.
    const ancien: Destination = { chemin: ONEDRIVE.propose, sorte: "dossier", amovible: false };
    expect(avecDossier([ancien, USB], ONEDRIVE.propose)).toEqual([
      { chemin: ONEDRIVE.propose, sorte: "synchronise", amovible: false },
      USB,
    ]);
  });

  it("retire seulement le dossier synchronisé désigné", () => {
    const choisi: Destination = { chemin: ONEDRIVE.propose, sorte: "synchronise", amovible: false };
    expect(sansDossier([USB, choisi], ONEDRIVE.propose)).toEqual([USB]);
    expect(sansDossier([USB], USB.chemin)).toEqual([USB]);
  });
});
