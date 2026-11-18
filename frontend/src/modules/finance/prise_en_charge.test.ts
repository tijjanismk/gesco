import { describe, expect, it } from "vitest";

import type { ApercuEleve } from "../../types/gesco";
import { depassementAttendu, importRepartition, lireFrancs, memeMontantPourChaque, partsDeLaSaisie, piedRepartition, resumerApercu, saisieInitiale } from "./prise_en_charge";

/** Élève de l'exemple : inscription 15 000 F, scolarité 120 000 F, cantine 30 000 F ; le parent a
 *  payé 63 000 F (inscription et première tranche) avant la case. */
function eleve(modifs: Partial<ApercuEleve>): ApercuEleve {
  return {
    inscription_id: "ins-1",
    eleve: "Awa Diarra",
    classe: "10e",
    refus: null,
    change: true,
    sans_facture: false,
    montant_propose: 120_000,
    attendu_avant: 0,
    attendu_apres: 120_000,
    abandonne_avant: 0,
    abandonne_apres: 0,
    du_etat_avant: 0,
    du_etat_apres: 120_000,
    doit_encore_etat_avant: 0,
    doit_encore_etat_apres: 120_000,
    reste_parent_avant: 102_000,
    reste_parent_apres: 0,
    trop_paye_apres: 63_000,
    trop_paye_reporte_apres: 30_000,
    a_rembourser_avant: 0,
    a_rembourser_apres: 33_000,
    rendu_apres: 0,
    ...modifs,
  };
}

describe("aperçu du cochage", () => {
  it("dit ce que le parent ne doit plus, son trop-payé et ce qui est à lui rembourser", () => {
    expect(resumerApercu([eleve({})])).toEqual({
      nombre: 1,
      refuses: 0,
      ne_doit_plus: 102_000,
      devra_en_plus: 0,
      dont_deja_rembourse: 0,
      trop_paye: 63_000,
      reporte: 30_000,
      a_rembourser: 33_000,
      etat_avant: 0,
      etat_apres: 120_000,
    });
  });

  it("additionne une sélection et laisse de côté les élèves refusés", () => {
    const resume = resumerApercu([
      eleve({}),
      eleve({ inscription_id: "ins-2", reste_parent_avant: 150_000, reste_parent_apres: 30_000, trop_paye_apres: 0, trop_paye_reporte_apres: 0, a_rembourser_apres: 0 }),
      eleve({ inscription_id: "ins-3", refus: "prise_en_charge.classe_non_concernee", change: false }),
    ]);
    expect(resume.nombre).toBe(2);
    expect(resume.refuses).toBe(1);
    expect(resume.ne_doit_plus).toBe(222_000);
    expect(resume.a_rembourser).toBe(33_000);
    expect(resume.etat_apres).toBe(240_000);
  });

  it("au décochage, dit ce que le parent devra, dont ce qui lui a déjà été rendu", () => {
    const resume = resumerApercu([
      eleve({
        reste_parent_avant: 0,
        reste_parent_apres: 135_000,
        trop_paye_apres: 0,
        trop_paye_reporte_apres: 0,
        a_rembourser_avant: 0,
        a_rembourser_apres: 0,
        rendu_apres: 33_000,
        doit_encore_etat_avant: 120_000,
        doit_encore_etat_apres: 0,
      }),
    ]);
    expect(resume.devra_en_plus).toBe(135_000);
    expect(resume.dont_deja_rembourse).toBe(33_000);
    expect(resume.ne_doit_plus).toBe(0);
    expect(resume.etat_apres).toBe(0);
  });
});

describe("pied de la répartition", () => {
  it("dit le réparti et le non réparti", () => {
    expect(piedRepartition(2_400_000, [{ montant: 1_200_000 }, { montant: 600_000 }])).toEqual({
      recu: 2_400_000,
      reparti: 1_800_000,
      non_reparti: 600_000,
      depassement: 0,
      nombre: 2,
      refuse: false,
    });
  });

  it("refuse au-delà du reçu, avec le dépassement", () => {
    const pied = piedRepartition(2_400_000, [{ montant: 2_000_000 }, { montant: 500_000 }]);
    expect(pied.refuse).toBe(true);
    expect(pied.depassement).toBe(100_000);
    expect(pied.non_reparti).toBe(0);
  });

  it("accepte une répartition sans élève : tout est non réparti", () => {
    expect(piedRepartition(500_000, [])).toMatchObject({ reparti: 0, non_reparti: 500_000, nombre: 0, refuse: false });
  });

  it("ne refuse rien tant que le montant reçu n'est pas lisible", () => {
    expect(piedRepartition(null, [{ montant: 120_000 }])).toMatchObject({ reparti: 120_000, refuse: false });
  });

  it("n'envoie que les lignes cochées dont le montant est positif", () => {
    const parts = partsDeLaSaisie({
      a: { coche: true, texte: "120 000 F" },
      b: { coche: false, texte: "120000" },
      c: { coche: true, texte: "" },
      d: { coche: true, texte: "0" },
      e: { coche: true, texte: "36.000" },
    });
    expect(parts).toEqual([
      { inscription_id: "a", montant: 120_000 },
      { inscription_id: "e", montant: 36_000 },
    ]);
  });
});

describe("même montant pour chaque élève coché", () => {
  it("remplace le montant des seules lignes cochées", () => {
    const apres = memeMontantPourChaque(
      {
        a: { coche: true, texte: "120000" },
        b: { coche: false, texte: "90000" },
        c: { coche: true, texte: "" },
      },
      126_000,
    );
    expect(apres).toEqual({
      a: { coche: true, texte: "126000" },
      b: { coche: false, texte: "90000" },
      c: { coche: true, texte: "126000" },
    });
  });

  it("préremplit le reste dû par l'État, et coche les élèves qui ont déjà une part", () => {
    expect(
      saisieInitiale([
        { inscription_id: "a", part: 0, propose: 120_000 },
        { inscription_id: "b", part: 60_000, propose: 60_000 },
        { inscription_id: "c", part: 0, propose: 0 },
      ]),
    ).toEqual({
      a: { coche: false, texte: "120000" },
      b: { coche: true, texte: "60000" },
      c: { coche: false, texte: "" },
    });
  });

  it("lit les montants comme le noyau", () => {
    expect(lireFrancs("126 000 F")).toBe(126_000);
    expect(lireFrancs("")).toBeNull();
    expect(lireFrancs("douze")).toBeNull();
  });
});

describe("part au-delà de ce que l'État doit", () => {
  it("dit le reçu en plus et l'attendu qui égale la somme reçue", () => {
    expect(depassementAttendu({ attendu: 120_000, reste_du_etat: 120_000, pris_en_charge: true }, 130_000)).toEqual({ en_plus: 10_000, attendu_porte: 130_000 });
    expect(depassementAttendu({ attendu: 120_000, reste_du_etat: 120_000, pris_en_charge: true }, 120_000)).toBeNull();
    // Un élève décoché n'a plus d'attendu à porter.
    expect(depassementAttendu({ attendu: 120_000, reste_du_etat: 0, pris_en_charge: false }, 40_000)).toEqual({ en_plus: 40_000, attendu_porte: null });
  });
});

describe("importRepartition", () => {
  it("écrit les lignes valides et en avertissement", () => {
    expect(importRepartition({ valides: 3, avertissements: 2, retirees: 0, refus: null })).toEqual({ aEcrire: 5, possible: true });
  });
  it("refuse tout au-delà du montant reçu", () => {
    expect(importRepartition({ valides: 3, avertissements: 0, retirees: 0, refus: "versement_etat.reparti_superieur" })).toEqual({ aEcrire: 3, possible: false });
  });
  it("un fichier vide retire encore les parts actuelles", () => {
    expect(importRepartition({ valides: 0, avertissements: 0, retirees: 2, refus: null })).toEqual({ aEcrire: 0, possible: true });
  });
  it("rien à écrire ni à retirer : rien à faire", () => {
    expect(importRepartition({ valides: 0, avertissements: 0, retirees: 0, refus: null }).possible).toBe(false);
  });
});
