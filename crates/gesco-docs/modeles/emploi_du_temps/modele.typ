// Emploi du temps (SPEC_PHASE_8 §8, T-64) : d'une classe ou d'un enseignant, une colonne par
// jour de classe, une ligne par horaire ; chaque case porte la matière et la classe ou
// l'enseignant, et la salle. En A4 paysage.
#let d = json("/donnees.json")
#let e = d.etablissement

#set document(title: d.titre, author: e.nom)
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
#text(size: 14pt, weight: "bold")[#d.titre]
#linebreak()
#text(size: 10pt)[#d.sous_titre]

#v(3mm)
#table(
  columns: (auto, ..d.jours.map(_ => 1fr)),
  stroke: 0.4pt,
  inset: 5pt,
  align: (left + horizon, ..d.jours.map(_ => center + horizon)),
  table.header([*Horaire*], ..d.jours.map(j => [*#j*])),
  ..d.lignes.map(l => (
    [*#l.horaire*],
    ..l.cases.map(c => if c == none [] else [
      *#c.matiere*
      #if c.detail != "" [ \ #text(size: 8pt)[#c.detail]]
      #if c.salle != "" [ \ #text(size: 7.5pt, style: "italic")[#c.salle]]
    ]),
  )).flatten(),
)

#v(6mm)
#text(size: 8pt)[Édité le #d.date]
