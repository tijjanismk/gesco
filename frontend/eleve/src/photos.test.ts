import { describe, expect, it } from "vitest";

import { COTE_MAX, dimensionsReduites, tailleDe } from "./photos";

describe("réduction des photos de copie", () => {
  it("ramène le grand côté à 1 600 pixels et garde les proportions", () => {
    expect(dimensionsReduites(4000, 3000)).toEqual({ largeur: COTE_MAX, hauteur: 1200 });
    expect(dimensionsReduites(3000, 4000)).toEqual({ largeur: 1200, hauteur: COTE_MAX });
  });

  it("n'agrandit pas une petite photo", () => {
    expect(dimensionsReduites(800, 600)).toEqual({ largeur: 800, hauteur: 600 });
    expect(dimensionsReduites(0, 600)).toEqual({ largeur: 0, hauteur: 0 });
  });

  it("mesure une adresse data en octets", () => {
    expect(tailleDe("data:image/jpeg;base64,QUJD")).toBe(3);
    expect(tailleDe("data:image/jpeg;base64,QUI=")).toBe(2);
  });
});
