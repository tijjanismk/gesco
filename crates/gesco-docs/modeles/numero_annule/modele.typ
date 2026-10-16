// Un numéro attribué puis abandonné laisse un document annulé, jamais un trou (R-46).
#import "/commun/entete.typ": page-standard

#let d = json("/donnees.json")

#page-standard(d.etablissement, "Numéro annulé : " + d.numero)[
  #table(
    columns: (auto, 1fr),
    stroke: 0.5pt,
    inset: 6pt,
    [*Numéro*], d.numero,
    [*Type de document*], d.type_document,
    [*Exercice*], d.exercice,
    [*Annulé le*], d.date,
    [*Motif*], d.motif,
  )

  #v(10mm)
  #align(center, text(size: 28pt, weight: "bold")[ANNULÉ])
]
