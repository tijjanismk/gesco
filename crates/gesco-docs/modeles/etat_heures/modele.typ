// État mensuel des heures des vacataires (SPEC_PHASE_6 §4) : heures prévues et faites de chaque
// vacation du mois, au taux de l'heure, visé par la direction. Une ligne non visée est marquée.
#import "/commun/entete.typ": page-standard

#let d = json("/donnees.json")

#page-standard(d.etablissement, "État des heures des vacataires")[
  #text(size: 12pt, weight: "bold")[Mois : #d.mois]

  #v(3mm)
  #table(
    columns: (1fr, auto, auto, auto, auto, auto),
    stroke: 0.4pt,
    inset: 4pt,
    align: (left, right, right, right, right, left),
    table.header[*Vacataire*][*Taux de l'heure*][*Heures prévues*][*Heures faites*][*Montant*][*Observation*],
    ..d.lignes.map(l => (
      [#l.membre \ #text(size: 8pt)[#l.matricule · #l.fonction]],
      [#l.taux],
      [#l.prevues],
      [#if l.faites == none [—] else [#l.faites]],
      [#if l.montant == none [—] else [#l.montant]],
      [#l.observation#if not l.visee [ #text(size: 8pt, style: "italic")[(non visé)]]],
    )).flatten(),
    [*Total*], [], [], [*#d.total_heures*], [*#d.total_montant*], [],
  )

  #v(10mm)
  #grid(
    columns: (1fr, 1fr),
    [Arrêté le #d.date], [La direction #v(18mm)],
  )
]
