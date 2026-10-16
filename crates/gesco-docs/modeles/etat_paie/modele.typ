// État de paie du mois (SPEC_PHASE_7 §8, T-57) : une ligne par membre payé, ce qui lui est dû
// et ce qu'il a reçu, une colonne pour signer « pour acquit » ; les totaux. En A4 paysage.
#let d = json("/donnees.json")
#let e = d.etablissement

#set document(title: "État de paie", author: e.nom)
#set page(paper: "a4", flipped: true, margin: (x: 14mm, top: 14mm, bottom: 16mm))
#set text(font: "Charis SIL", size: 9pt, lang: "fr")

#grid(
  columns: (auto, 1fr),
  column-gutter: 10pt,
  align: horizon,
  if e.logo != none { image(e.logo, height: 14mm) },
  {
    text(size: 12pt, weight: "bold", e.nom)
    if e.adresse != "" { linebreak(); text(size: 8pt, e.adresse) }
  },
)
#v(1mm)
#line(length: 100%, stroke: 0.5pt)
#v(2mm)
#text(size: 14pt, weight: "bold")[État de paie — #d.mois]
#if d.projet [ #text(size: 11pt, weight: "bold", fill: rgb("#9a3412"))[(projet, paie non validée)]]

#v(3mm)
#table(
  columns: (auto, 1fr, auto, auto, auto, auto, auto, auto, 38mm),
  stroke: 0.4pt,
  inset: 4pt,
  align: (left, left, right, right, right, right, right, right, left),
  table.header[*N°*][*Membre*][*Brut*][*Retenues*][*Avances*][*Net*][*Payé*][*Reste*][*Signature pour acquit*],
  ..d.lignes.map(l => (
    [#l.numero],
    [#l.membre \ #text(size: 7.5pt)[#l.matricule · #l.fonction]],
    [#l.brut], [#l.retenues], [#l.avances], [*#l.net*], [#l.paye], [#l.reste],
    [#v(7mm)],
  )).flatten(),
  [], [*Total* (#d.lignes.len() personnes)], [*#d.totaux.brut*], [*#d.totaux.retenues*], [*#d.totaux.avances*], [*#d.totaux.net*], [*#d.totaux.paye*], [*#d.totaux.reste*], [],
)

#v(8mm)
#grid(
  columns: (1fr, 1fr),
  [Arrêté le #d.date], [La direction #v(16mm)],
)
