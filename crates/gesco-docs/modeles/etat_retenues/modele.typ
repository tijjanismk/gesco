// État des retenues et cotisations du mois (SPEC_PHASE_7 §8, T-57) : pour chaque retenue
// salariale et chaque cotisation de l'école, une ligne par membre concerné, son assiette, le taux
// et le montant ; les totaux, pour les déclarations faites hors du logiciel.
#import "/commun/entete.typ": page-standard

#let d = json("/donnees.json")

#page-standard(d.etablissement, "État des retenues et cotisations")[
  #text(size: 12pt, weight: "bold")[Mois : #d.mois]
  #if d.projet [ #text(weight: "bold", fill: rgb("#9a3412"))[(projet, paie non validée)]]

  #if d.elements.len() == 0 [
    #v(4mm)
    Aucune retenue ni cotisation ce mois-ci.
  ]
  #for el in d.elements {
    v(4mm)
    text(size: 11pt, weight: "bold")[#el.libelle]
    text(size: 9pt)[ — #el.nature]
    table(
      columns: (1fr, auto, auto, auto),
      stroke: 0.4pt,
      inset: 4pt,
      align: (left, right, right, right),
      table.header[*Membre*][*Assiette*][*Taux*][*Montant*],
      ..el.lignes.map(l => ([#l.membre], [#l.assiette], [#l.taux], [#l.montant])).flatten(),
      [*Total*], [], [], [*#el.total*],
    )
  }

  #v(10mm)
  #grid(
    columns: (1fr, 1fr),
    [Arrêté le #d.date], [La direction #v(18mm)],
  )
]
