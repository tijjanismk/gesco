// État de caisse (SPEC_PHASE_3 §6.2) : ce que le caissier remet avec l'argent, signé par lui et
// par le contrôleur. Les espèces seules sont dans le tiroir ; les autres modes sont à part.
#import "/commun/entete.typ": page-standard

#let d = json("/donnees.json")
#let ligne(libelle, valeur) = if valeur != none and valeur != "" { ([#libelle], [#valeur]) } else { () }

#page-standard(d.etablissement, "État de caisse")[
  #text(size: 11pt)[Caissier : *#d.caissier* · session #lower(d.statut)] \
  #text(size: 10pt)[Ouverte le #d.ouverture#if d.cloture != "" [, clôturée le #d.cloture]]

  #v(4mm)
  == Espèces
  #table(
    columns: (1fr, 45mm),
    stroke: 0.4pt,
    inset: 5pt,
    align: (left, right),
    ..ligne("Fonds de caisse initial", d.fonds),
    ..ligne("Entrées en espèces", d.entrees_especes),
    ..ligne("Sorties en espèces", d.sorties_especes),
    [*Total théorique*], [*#d.theorique*],
    ..ligne("Total compté", d.compte),
    ..ligne("Écart", d.ecart),
  )
  #if d.motif_ecart != "" [Motif de l'écart : #d.motif_ecart]

  #if d.modes.len() > 0 {
    v(3mm)
    [== Encaissements par mode]
    table(
      columns: (1fr, 45mm),
      stroke: 0.4pt,
      inset: 5pt,
      align: (left, right),
      ..d.modes.map(m => ([#m.mode], [#m.montant])).flatten(),
    )
  }
  #text(size: 9pt)[Reçus émis pendant la session : #d.nombre_recus]

  #if d.at("sorties", default: ()).len() > 0 {
    v(3mm)
    [== Mouvements hors encaissement]
    table(
      columns: (28mm, 1fr, 28mm, 32mm),
      stroke: 0.4pt,
      inset: 5pt,
      align: (left, left, left, right),
      table.header[*Nature*][*Motif*][*Mode*][*Montant*],
      ..d.sorties.map(s => ([#s.at("nature", default: "")], [#s.motif], [#s.mode], [#s.montant])).flatten(),
    )
  }

  #if d.vise_par != "" {
    v(3mm)
    text(size: 9pt)[Visée par #d.vise_par le #d.visa#if d.commentaire_visa != "" [ : #d.commentaire_visa]]
  }

  #v(8mm)
  #grid(
    columns: (1fr, 1fr, auto),
    column-gutter: 6mm,
    [Le caissier #v(18mm)],
    [Le contrôleur #v(18mm)],
    align(center)[
      #image(d.qr, width: 24mm) \
      #text(size: 6pt, d.reference)
    ],
  )
]
