import { describe, expect, it } from "vitest";

import { apercuLigne, filtrerLignes, gesteDeLaSaisie, lendemain, lireSaisieReprise, remboursementPrerempli } from "./reprise";

describe("case « Payé avant GESCO »", () => {
  it("lit les montants comme le noyau", () => {
    expect(lireSaisieReprise("75000")).toEqual({ sorte: "montant", montant: 75_000 });
    expect(lireSaisieReprise("75 000")).toEqual({ sorte: "montant", montant: 75_000 });
    expect(lireSaisieReprise("75 000 F")).toEqual({ sorte: "montant", montant: 75_000 });
    expect(lireSaisieReprise("75\u202f000\u00a0FCFA")).toEqual({ sorte: "montant", montant: 75_000 });
    expect(lireSaisieReprise("75.000")).toEqual({ sorte: "montant", montant: 75_000 });
    expect(lireSaisieReprise("0")).toEqual({ sorte: "montant", montant: 0 });
    expect(lireSaisieReprise("")).toEqual({ sorte: "vide" });
    expect(lireSaisieReprise("   ")).toEqual({ sorte: "vide" });
  });

  it("refuse les décimales, les signes et le texte", () => {
    expect(lireSaisieReprise("75,5")).toEqual({ sorte: "illisible" });
    expect(lireSaisieReprise("-5000")).toEqual({ sorte: "illisible" });
    expect(lireSaisieReprise("beaucoup")).toEqual({ sorte: "illisible" });
  });

  it("dit ce que la saisie demande", () => {
    expect(gesteDeLaSaisie("75 000", 0)).toEqual({ geste: "enregistrer", montant: 75_000 });
    expect(gesteDeLaSaisie("75 000", 50_000)).toEqual({ geste: "enregistrer", montant: 75_000 });
    expect(gesteDeLaSaisie("75.000", 75_000)).toEqual({ geste: "rien" });
    // Vider la case ou taper 0 sur un élève qui a un montant : on demande le motif.
    expect(gesteDeLaSaisie("", 50_000)).toEqual({ geste: "annuler" });
    expect(gesteDeLaSaisie("0", 50_000)).toEqual({ geste: "annuler" });
    // Sur un élève sans montant, rien à faire.
    expect(gesteDeLaSaisie("", 0)).toEqual({ geste: "rien" });
    expect(gesteDeLaSaisie("0", 0)).toEqual({ geste: "rien" });
    expect(gesteDeLaSaisie("75,5", 0)).toEqual({ geste: "illisible" });
  });
});

describe("chiffres affichés", () => {
  const ligne = { du: 185_000, paye_gesco: 0, rembourse: 0 };

  it("la reprise remplit le reste, le surplus est à rembourser", () => {
    expect(apercuLigne(ligne, 60_000)).toEqual({ reste: 125_000, a_rembourser: 0 });
    expect(apercuLigne(ligne, 185_000)).toEqual({ reste: 0, a_rembourser: 0 });
    expect(apercuLigne(ligne, 200_000)).toEqual({ reste: 0, a_rembourser: 15_000 });
  });

  it("compte les versements faits dans GESCO et ce qui a déjà été rendu", () => {
    expect(apercuLigne({ ...ligne, paye_gesco: 10_000 }, 60_000)).toEqual({ reste: 115_000, a_rembourser: 0 });
    expect(apercuLigne({ ...ligne, paye_gesco: 10_000 }, 200_000)).toEqual({ reste: 0, a_rembourser: 25_000 });
    expect(apercuLigne({ ...ligne, rembourse: 10_000 }, 200_000)).toEqual({ reste: 0, a_rembourser: 5_000 });
    // Jamais de reste ni d'excédent négatif.
    expect(apercuLigne({ du: 50_000, paye_gesco: 60_000, rembourse: 0 }, 0)).toEqual({ reste: 0, a_rembourser: 0 });
  });

  it("dit le jour de la première caisse, lendemain de la date d'arrêt", () => {
    expect(lendemain("2026-10-06")).toBe("2026-10-07");
    expect(lendemain("2026-10-31")).toBe("2026-11-01");
    expect(lendemain("2026-12-31")).toBe("2027-01-01");
  });

  it("cherche par nom ou matricule, sans tenir compte des accents", () => {
    const lignes = [
      { eleve: "Awa Diarra", matricule: "2026-0001" },
      { eleve: "Fanta Kéita", matricule: "2026-0002" },
    ];
    expect(filtrerLignes(lignes, "keita").map((l) => l.matricule)).toEqual(["2026-0002"]);
    expect(filtrerLignes(lignes, "0001").map((l) => l.eleve)).toEqual(["Awa Diarra"]);
    expect(filtrerLignes(lignes, "")).toHaveLength(2);
  });
});

describe("remboursement d'un excédent", () => {
  it("prérempli : catégorie, payeur, excédent, libellé et inscription liée", () => {
    const demande = remboursementPrerempli({
      inscriptionId: "ins-1",
      eleve: "Awa Diarra",
      payeur: "Fanta Sangaré",
      aRembourser: 15_000,
      categorieId: "cat-remboursement",
      libelle: (eleve) => `Trop-payé avant GESCO — ${eleve}`,
    });
    expect(demande).toEqual({
      date: "",
      categorie_id: "cat-remboursement",
      libelle: "Trop-payé avant GESCO — Awa Diarra",
      beneficiaire: "Fanta Sangaré",
      montant: 15_000,
      paiement: null,
      inscription_id: "ins-1",
    });
  });
});
