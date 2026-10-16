// Liste de la cantine du jour (SPEC_PHASE_8 §9, T-47) : les abonnés par classe, ce qui reste
// échu de leur cantine, une case à cocher pour le repas quand l'école les compte.
#import "/commun/entete.typ": page-standard

#let d = json("/donnees.json")

#page-standard(d.etablissement, "Cantine — liste du jour")[
  #text(size: 12pt, weight: "bold")[#d.date · #d.total #if d.total == 1 [abonné] else [abonnés]]

  #for c in d.classes [
    #v(3mm)
    #text(size: 11pt, weight: "bold")[#c.classe]
    #table(
      columns: if d.repas_comptes { (1fr, auto, 18mm) } else { (1fr, auto) },
      stroke: 0.4pt,
      inset: 4pt,
      align: (left, right, center),
      ..if d.repas_comptes {
        (table.header[*Élève*][*Reste échu*][*Repas*],)
      } else {
        (table.header[*Élève*][*Reste échu*],)
      },
      ..c.eleves.map(e => if d.repas_comptes {
        ([#e.nom], [#e.reste], [#box(width: 4mm, height: 4mm, stroke: 0.5pt)])
      } else {
        ([#e.nom], [#e.reste])
      }).flatten(),
    )
  ]
]
