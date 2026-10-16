// Bulletins de paie (SPEC_PHASE_7 §8, T-57) : une page par membre, un seul PDF à imprimer.
// Gains, retenues, avances, net à payer, paiements reçus et reste ; cumuls de l'année. Un bulletin
// d'une paie pas encore validée s'imprime marqué « projet ».
#let d = json("/donnees.json")
#let e = d.etablissement

#set document(title: "Bulletins de paie", author: e.nom)
#let a5 = e.papier == "a5"
#set page(paper: if a5 { "a5" } else { "a4" }, margin: if a5 { (x: 9mm, top: 9mm, bottom: 10mm) } else { (x: 18mm, top: 15mm, bottom: 16mm) })
#set text(font: "Charis SIL", size: if a5 { 7.5pt } else { 9.5pt }, lang: "fr")

#let rubrique(titre, lignes) = if lignes.len() > 0 {
  v(2mm)
  table(
    columns: (1fr, auto, auto),
    stroke: 0.4pt,
    inset: 4pt,
    align: (left, left, right),
    table.header[*#titre*][*Détail*][*Montant*],
    ..lignes.map(l => ([#l.libelle], [#l.detail], [#l.montant])).flatten(),
  )
}

#for (i, b) in d.bulletins.enumerate() {
  if i > 0 { pagebreak() }
  grid(
    columns: (auto, 1fr, auto),
    column-gutter: 8pt,
    align: horizon,
    if e.logo != none { image(e.logo, height: if a5 { 12mm } else { 16mm }) },
    {
      text(size: if a5 { 9.5pt } else { 12pt }, weight: "bold", e.nom)
      if e.adresse != "" { linebreak(); text(size: 8pt, e.adresse) }
      if e.telephones.len() > 0 { linebreak(); text(size: 8pt, e.telephones.join(" · ")) }
    },
    align(right)[
      #text(size: if a5 { 10pt } else { 13pt }, weight: "bold")[BULLETIN DE PAIE] \
      #text(size: 9pt)[#b.mois] \
      #text(size: 8pt)[#b.numero]
    ],
  )
  v(1mm)
  line(length: 100%, stroke: 0.5pt)
  if b.projet {
    align(center, text(size: 11pt, weight: "bold", fill: rgb("#9a3412"))[PROJET — PAIE NON VALIDÉE])
  }
  v(1mm)
  grid(
    columns: (1fr, 1fr),
    column-gutter: 10pt,
    [
      *#b.membre.nom* \
      Matricule #b.membre.matricule \
      #b.membre.fonction
    ],
    [
      #if b.membre.categorie != "" [Catégorie : #b.membre.categorie \ ]
      #if b.membre.inps != "" [N° INPS : #b.membre.inps \ ]
      #if b.membre.fonctionnaire != "" [Fonctionnaire n° #b.membre.fonctionnaire \ ]
      #if b.membre.situation != "" [#b.membre.situation]
    ],
  )

  rubrique("Gains", b.gains)
  table(
    columns: (1fr, auto),
    stroke: 0.4pt,
    inset: 4pt,
    align: (left, right),
    [*Salaire brut*], [*#b.brut*],
  )
  rubrique("Retenues", b.retenues)
  rubrique("Avances", b.avances)

  v(2mm)
  table(
    columns: (1fr, auto),
    stroke: 0.6pt,
    inset: 5pt,
    align: (left, right),
    [Total des retenues], [#b.total_retenues],
    [Avances retenues], [#b.total_avances],
    [*Net à payer*], [*#b.net*],
    [Déjà payé], [#b.paye],
    [*Reste à payer*], [*#b.reste*],
  )

  if b.patronales.len() > 0 {
    v(2mm)
    text(size: 8pt)[*Cotisations de l'école, pour mémoire* (elles ne touchent pas le net) :
      #b.patronales.map(l => [#l.libelle #l.montant]).join(" · ")]
  }

  v(2mm)
  text(size: 8pt)[*Cumuls de l'année #b.annee* : brut #b.cumuls.brut · retenues #b.cumuls.retenues · net #b.cumuls.net]

  for a in b.avertissements {
    v(1mm)
    text(size: 8pt, style: "italic")[#a]
  }

  v(8mm)
  grid(
    columns: (1fr, 1fr),
    [Le membre du personnel #v(14mm)], [La direction #v(14mm)],
  )
}
