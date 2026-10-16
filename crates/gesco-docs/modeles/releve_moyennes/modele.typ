// Relevé des moyennes (SPEC_PHASE_4 §9) : les moyennes des périodes suivies dans l'école, par
// matière et générale, avec le rang sauf pour la période du départ ; une moyenne reprise d'une
// autre école porte sa source. Il accompagne le certificat de transfert.
#import "/commun/entete.typ": page-standard

#let d = json("/donnees.json")

#page-standard(d.etablissement, "Relevé des moyennes")[
  #text(size: 11pt)[*#d.eleve* · matricule #d.matricule#if d.matricule_national != none [ · matricule national #d.matricule_national]] \
  #text(size: 9pt)[#if d.naissance != "" [Né(e) le #d.naissance · ]Classe #d.classe · année #d.annee · notes sur #d.bareme]

  #if d.periodes.len() == 0 [
    #v(4mm)
    Aucune moyenne de période n'est encore calculée.
  ]
  #for p in d.periodes {
    v(4mm)
    [== #p.periode]
    if p.source != "" { text(size: 8pt, style: "italic", p.source); linebreak() }
    if p.matieres.len() > 0 {
      table(
        columns: (1fr, auto),
        stroke: 0.4pt,
        inset: 4pt,
        align: (left, right),
        table.header[*Matière*][*Moyenne*],
        ..p.matieres.map(m => ([#m.matiere], [#m.moyenne])).flatten(),
      )
    }
    [*Moyenne générale : #p.moyenne*#if p.rang != "" [ · rang #p.rang]]
  }

  #v(10mm)
  #grid(
    columns: (1fr, auto),
    [Fait le #d.date \ Le directeur #v(18mm)],
    align(center)[
      #image(d.qr, width: 22mm) \
      #text(size: 6pt, d.reference)
    ],
  )
]
