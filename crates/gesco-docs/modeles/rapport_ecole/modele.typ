// Rapports de rentrée (SPEC_PHASE_9 §9.4) : en-tête administratif, parties rédigées en
// portrait, tableaux larges en paysage. Les intitulés et l'ordre suivent les modèles ; une case
// vide reste vide. Emplacement laissé libre pour la signature et le cachet.
#import "/commun/entete.typ": entete, pied

#let d = json("/donnees.json")
#let e = d.etablissement

#set document(title: d.titre, author: e.nom)
#set page(paper: "a4", margin: (x: 16mm, top: 14mm, bottom: 18mm), footer: pied(e))
#set text(font: "Charis SIL", size: 10pt, lang: "fr")
#set par(justify: true)

#let tableau(t) = {
  let n = t.colonnes.len()
  let petit = n > 9
  set text(size: if petit { 7pt } else { 8.5pt })
  block(breakable: true)[
    #text(size: 10.5pt, weight: "bold")[#t.titre]
    #v(1mm)
    #table(
      columns: (auto,) + (1fr,) * (n - 1),
      stroke: 0.4pt,
      inset: if petit { 2.5pt } else { 3.5pt },
      align: (x, y) => if x == 0 { left } else { center },
      table.header(..t.colonnes.map(c => [*#c*])),
      ..t.lignes.map(l => {
        let gras = l.at(0).starts-with("Total")
        l.map(c => if gras [*#c*] else [#c])
      }).flatten(),
    )
  ]
  v(3mm)
}

#entete(e)
#v(2mm)
#grid(
  columns: (1fr, 1fr),
  column-gutter: 8mm,
  row-gutter: 1.5mm,
  ..d.entete.map(l => [#text(weight: "bold")[#l.libelle :] #l.valeur]),
)
#v(4mm)
#align(center)[
  #text(size: 15pt, weight: "bold")[#d.titre]
  #linebreak()
  #text(size: 10pt)[Année scolaire #d.annee · chiffres arrêtés au #d.arret]
]
#v(4mm)

#for s in d.sections [
  #text(size: 11pt, weight: "bold")[#s.titre]
  #v(1mm)
  #s.texte
  #v(3mm)
]

#for t in d.tableaux.filter(t => not t.paysage) [
  #tableau(t)
]

#let signature = align(right)[
  #v(6mm)
  #d.fait_a
  #linebreak()
  #text(weight: "bold")[#d.qualite]
  #v(22mm)
]

#let larges = d.tableaux.filter(t => t.paysage)
#if larges.len() == 0 {
  signature
} else {
  page(flipped: true)[
    #for t in larges [
      #tableau(t)
    ]
    #signature
  ]
}
