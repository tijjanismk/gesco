// État des sommes dues par l'État (prise en charge des élèves orientés) : par classe, les élèves
// pris en charge avec l'attendu, le reçu, le reste dû et l'échu, un sous-total par classe et le
// total général ; puis les versements de l'État reçus pour l'année et ce qui n'en est pas
// réparti. À joindre à une réclamation : lieu, date, chef d'établissement, signature et cachet,
// QR de vérification. En A4 paysage, comme l'état de paie.
#import "/commun/entete.typ": entete, pied

#let d = json("/donnees.json")
#let e = d.etablissement

#set document(title: d.titre, author: e.nom)
#set page(paper: "a4", flipped: true, margin: (x: 14mm, top: 14mm, bottom: 16mm), footer: pied(e))
#set text(font: "Charis SIL", size: 9pt, lang: "fr")

#entete(e)
#v(2mm)
#text(size: 14pt, weight: "bold")[#d.titre]
#linebreak()
#text(size: 10pt)[Année scolaire #d.annee · arrêté au #d.arrete_au]
#if d.precisions != "" [ \ #text(size: 9pt)[#d.precisions]]

#let colonnes = (1fr, auto, auto, auto, auto, auto, auto, auto, auto)
#let alignement = (left, left, center, left, left, right, right, right, right)
#let entetes = table.header[*Élève*][*Matricule*][*Sexe*][*Série*][*Référence*][*Attendu*][*Reçu*][*Reste dû*][*Dont échu*]

#if d.classes.len() == 0 [
  #v(4mm)
  Aucun élève pris en charge ne correspond à ces critères.
]

#for c in d.classes {
  v(3mm)
  block(breakable: true)[
    #text(size: 10.5pt, weight: "bold")[#c.classe]
    #v(1mm)
    #table(
      columns: colonnes,
      stroke: 0.4pt,
      inset: 3.5pt,
      align: alignement,
      entetes,
      ..c.lignes.map(l => (
        [#l.eleve], [#l.matricule], [#l.sexe], [#l.serie], [#l.reference],
        [#l.attendu], [#l.recu], [*#l.reste*], [#l.echu],
      )).flatten(),
      table.cell(colspan: 5)[*Sous-total #c.classe* (#c.sous_total.nombre élève(s))],
      [*#c.sous_total.attendu*], [*#c.sous_total.recu*], [*#c.sous_total.reste*], [*#c.sous_total.echu*],
    )
  ]
}

#v(4mm)
#table(
  columns: (1fr, auto, auto, auto, auto),
  stroke: 0.6pt,
  inset: 4pt,
  align: (left, right, right, right, right),
  table.header[*Total général*][*Élèves*][*Attendu*][*Reçu*][*Reste dû (dont échu)*],
  [Toutes les classes de cet état], [*#d.total.nombre*], [*#d.total.attendu*], [*#d.total.recu*], [*#d.total.reste* (#d.total.echu)],
)
#if d.at("recu_en_plus", default: none) != none [
  #text(size: 8.5pt)[Reçu de l'État au-delà de ce qu'il doit pour certains élèves : #d.recu_en_plus.]
]

#v(4mm)
#text(size: 10.5pt, weight: "bold")[Versements de l'État reçus pour l'année]
#v(1mm)
#if d.versements.len() == 0 [
  Aucun versement de l'État reçu pour cette année.
] else {
  table(
    columns: (auto, auto, 1fr, auto),
    stroke: 0.4pt,
    inset: 3.5pt,
    align: (left, left, left, right),
    table.header[*Date*][*Référence*][*Libellé*][*Montant*],
    ..d.versements.map(v => ([#v.date], [#v.reference], [#v.libelle], [#v.montant])).flatten(),
    table.cell(colspan: 3)[*Total reçu*], [*#d.total_verse*],
  )
}
Reçu de l'État pas encore réparti sur des élèves : *#d.non_reparti*

#v(6mm)
#block(breakable: false)[
  #grid(
    columns: (1fr, auto, 1fr),
    column-gutter: 8mm,
    align(left)[
      #image(d.qr, width: 22mm) \
      #text(size: 6pt, d.reference)
    ],
    [],
    align(center)[
      #d.fait_a
      #v(2mm)
      *Le chef d'établissement*
      #v(4mm)
      #text(size: 8pt)[Signature et cachet]
      #v(22mm)
    ],
  )
]
