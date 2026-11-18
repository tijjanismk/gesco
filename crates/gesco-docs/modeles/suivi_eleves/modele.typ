// Suivi des élèves (SPEC_PHASE_12_LOT_4 §6) : pour une classe et une matière, combien d'élèves
// ont ouvert et fini chaque leçon, puis, quand la direction le permet, les leçons de chaque élève
// dans l'ordre alphabétique. Aucun classement, aucune note.
#import "/commun/entete.typ": page-standard

#let d = json("/donnees.json")

#page-standard(d.etablissement, "Suivi des élèves dans les leçons")[
  #text(size: 12pt, weight: "bold")[#d.classe · #d.matiere]
  #linebreak()
  #text(size: 10pt)[#d.date · #d.eleves #if d.eleves == 1 [élève] else [élèves]]

  #v(3mm)
  #if d.lecons.len() == 0 [
    Aucune leçon de cette matière n'est ouverte aux élèves de la classe.
  ] else [
    #table(
      columns: (1fr, 1.4fr, auto, auto),
      stroke: 0.4pt,
      inset: 4pt,
      align: (left, left, right, right),
      table.header[*Chapitre*][*Leçon*][*Ouverte par*][*Finie par*],
      ..d.lecons.map(l => ([#l.chapitre], [#l.titre], [#l.ouvertes], [#l.finies])).flatten(),
    )
  ]

  #if d.par_eleve and d.liste.len() > 0 [
    #v(4mm)
    #text(size: 11pt, weight: "bold")[Par élève]
    #table(
      columns: (1fr, auto, auto, auto),
      stroke: 0.4pt,
      inset: 4pt,
      align: (left, right, right, left),
      table.header[*Élève*][*Leçons ouvertes*][*Leçons finies*][*Dernière visite*],
      ..d.liste.map(e => ([#e.nom], [#e.ouvertes], [#e.finies], [#e.visite])).flatten(),
    )
  ]
]
