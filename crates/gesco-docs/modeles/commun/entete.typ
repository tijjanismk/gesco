// En-tête et pied communs à tous les modèles. Nom, adresse, téléphones, logo et mentions
// viennent des données injectées par le moteur (blueprint §13.2, niveau 2) : changer d'école ne
// modifie aucun modèle.

#let entete(etablissement) = {
  grid(
    columns: (auto, 1fr),
    column-gutter: 10pt,
    align: horizon,
    if etablissement.logo != none { image(etablissement.logo, height: 18mm) },
    {
      text(size: 13pt, weight: "bold", etablissement.nom)
      if etablissement.adresse != "" {
        linebreak()
        text(size: 9pt, etablissement.adresse)
      }
      if etablissement.telephones.len() > 0 {
        linebreak()
        text(size: 9pt, etablissement.telephones.join(" · "))
      }
    },
  )
  v(2mm)
  line(length: 100%, stroke: 0.6pt)
}

#let pied(etablissement) = if etablissement.mentions != "" {
  text(size: 8pt, etablissement.mentions)
}

#let page-standard(etablissement, titre, corps) = {
  set document(title: titre, author: etablissement.nom)
  // Le format suit le réglage de l'école pour ce document (A4 par défaut).
  let a5 = etablissement.papier == "a5"
  set page(
    paper: if a5 { "a5" } else { "a4" },
    margin: if a5 { (x: 12mm, top: 12mm, bottom: 16mm) } else { (x: 20mm, top: 18mm, bottom: 22mm) },
    footer: pied(etablissement),
  )
  set text(font: "Charis SIL", size: if a5 { 9pt } else { 11pt }, lang: "fr")
  set par(justify: true)
  entete(etablissement)
  v(8mm)
  text(size: 16pt, weight: "bold", titre)
  v(4mm)
  corps
}
