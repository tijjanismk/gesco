// Billets des élèves pour le portail de l'école (phase 12, lot 4) : dix billets par page A4, à
// découper. Le billet est le seul endroit où le code est écrit ; la base n'en garde que
// l'empreinte. Réimprimer, c'est remettre de nouveaux codes.
#let d = json("/donnees.json")

#set document(title: "Billets des élèves", author: d.etablissement.nom)
#set page(paper: "a4", margin: (x: 10mm, top: 10mm, bottom: 12mm), footer: align(center, text(
  size: 7pt,
)[Réimprimer les billets remet de nouveaux codes : les anciens ne marchent plus.]))
#set text(font: "Charis SIL", size: 9pt, lang: "fr")

#let billet(b) = {
  let texte = {
    text(size: 7.5pt, d.etablissement.nom)
    linebreak()
    text(size: 11pt, weight: "bold")[#b.prenoms #b.nom]
    linebreak()
    [Classe : #b.classe]
    linebreak()
    [Matricule : *#b.matricule*]
    v(1mm)
    [Code : ]
    text(size: 14pt, weight: "bold", tracking: 2pt, b.code)
    if d.adresse != "" {
      linebreak()
      text(size: 8pt)[Sur le Wi-Fi de l'école : *#d.adresse*]
    }
    linebreak()
    text(size: 7pt)[Remis le #d.remis_le. Garde ton code pour toi.]
  }
  box(
    width: 100%,
    height: 100%,
    stroke: (paint: luma(120), dash: "dashed", thickness: 0.5pt),
    inset: 3mm,
    if d.qr != none {
      grid(
        columns: (1fr, 22mm),
        column-gutter: 2mm,
        texte,
        align(horizon, image(d.qr, width: 22mm)),
      )
    } else {
      texte
    },
  )
}

#for (i, page) in d.billets.chunks(d.par_page).enumerate() {
  if i > 0 { pagebreak() }
  grid(
    columns: (1fr, 1fr),
    rows: (1fr,) * 5,
    gutter: 0mm,
    ..page.map(billet),
  )
}
