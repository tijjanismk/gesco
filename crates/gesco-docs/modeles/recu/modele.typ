// Reçu de versement (SPEC_PHASE_3 §7.4, T-16) : une feuille A4, deux exemplaires A5, l'un pour
// le parent, l'autre en souche. Le montant en lettres fait foi ; le QR code renvoie au document
// enregistré. Un reçu annulé ou réimprimé le dit en grand.
#let d = json("/donnees.json")
#let e = d.etablissement

#set document(title: "Reçu " + d.numero, author: e.nom)
// En A5 (réglage de l'école) : une feuille A5 à l'italienne par exemplaire. En A4 : les deux
// exemplaires sur la même feuille, à couper.
#let a5 = e.papier == "a5"
#set page(paper: if a5 { "a5" } else { "a4" }, flipped: a5, margin: (x: if a5 { 10mm } else { 14mm }, y: if a5 { 8mm } else { 10mm }))
#set text(font: "Charis SIL", size: 9.5pt, lang: "fr")

#let exemplaire(destinataire) = block(height: 132mm, width: 100%, {
  grid(
    columns: (auto, 1fr, auto),
    column-gutter: 8pt,
    align: horizon,
    if e.logo != none { image(e.logo, height: 13mm) },
    {
      text(size: 11pt, weight: "bold", e.nom)
      if e.adresse != "" { linebreak(); text(size: 8pt, e.adresse) }
      if e.telephones.len() > 0 { linebreak(); text(size: 8pt, e.telephones.join(" · ")) }
    },
    align(right)[
      #text(size: 12pt, weight: "bold")[REÇU N° #d.numero] \
      #text(size: 8pt)[#d.date] \
      #text(size: 8pt, style: "italic", destinataire)
    ],
  )
  v(1mm)
  line(length: 100%, stroke: 0.5pt)
  if d.annule {
    align(center, text(size: 14pt, weight: "bold", fill: red)[ANNULÉ — #d.motif_annulation])
  } else if d.duplicata {
    align(center, text(size: 12pt, weight: "bold")[DUPLICATA])
  }
  v(1mm)
  grid(
    columns: (1fr, auto),
    column-gutter: 6mm,
    [
      Reçu de *#d.payeur* la somme de : \
      #text(size: 13pt, weight: "bold", d.montant) \
      #text(style: "italic")[#d.montant_lettres] \
      #v(1mm)
      Pour *#d.eleve* (#d.matricule), classe *#d.classe*, année #d.annee. \
      Mode : #d.mode#if d.reference_paiement != "" [ · référence #d.reference_paiement]
    ],
    align(center)[
      #image(d.qr, width: 20mm) \
      #text(size: 5.5pt, d.reference)
    ],
  )
  v(1mm)
  table(
    columns: (1fr, 32mm),
    stroke: 0.4pt,
    inset: 3.5pt,
    align: (left, right),
    table.header[*Imputé sur*][*Montant*],
    ..d.imputations.map(i => ([#i.libelle], [#i.montant])).flatten(),
  )
  [Reste dû après ce versement : *#d.reste*]
  v(1fr)
  grid(
    columns: (1fr, 1fr),
    [Le caissier : #d.caissier],
    align(right)[Signature et cachet],
  )
})

#exemplaire("Exemplaire du parent")
#if a5 {
  pagebreak()
} else {
  line(length: 100%, stroke: (dash: "dashed", thickness: 0.5pt))
  v(4mm)
}
#exemplaire("Souche de l'école")
