// Livrets du jardin d'une classe (SPEC_PHASE_11_B §5) : une page par enfant, un seul PDF à
// imprimer, en A4 portrait. Un bloc par domaine, chaque compétence observée sur une ligne, une
// colonne par période déjà passée ; dans chaque case le niveau en toutes lettres et un
// pictogramme noir sur blanc (rond plein, demi-rond, rond vide), jamais la couleur seule : le
// livret reste lisible en photocopie et par un parent qui lit peu. Ce qui n'existe pas ne
// s'imprime pas, sans cadre vide ni mention.
#let d = json("/donnees.json")
#let e = d.etablissement

#set document(title: "Livrets", author: e.nom)
#set page(paper: "a4", margin: (x: 14mm, top: 12mm, bottom: 14mm))
#set text(font: "Charis SIL", size: 9.5pt, lang: "fr")

// Les trois pictogrammes, tracés : ils ne dépendent d'aucune police.
#let r = 1.5mm
#let picto(nom) = box(width: 2 * r, height: 2 * r, baseline: 0.3mm, {
  if nom == "plein" {
    circle(radius: r, fill: black, stroke: 0.6pt + black)
  } else if nom == "vide" {
    circle(radius: r, stroke: 0.6pt + black)
  } else {
    place(circle(radius: r, stroke: 0.6pt + black))
    // Moitié gauche pleine : un demi-disque en polygone.
    place(polygon(fill: black, ..range(0, 13).map(k => {
      let a = 90deg + k * 15deg
      (r + r * calc.cos(a), r - r * calc.sin(a))
    })))
  }
})

#let niveau(n) = if n == none { [] } else { [#picto(n.picto) #h(1.5mm) #n.libelle] }

#let entete(l) = {
  grid(
    columns: (auto, 1fr, auto),
    column-gutter: 8pt,
    align: horizon,
    if e.logo != none { image(e.logo, height: 16mm) },
    {
      text(size: 12pt, weight: "bold", e.nom)
      if e.adresse != "" { linebreak(); text(size: 8pt, e.adresse) }
      if e.telephones.len() > 0 { linebreak(); text(size: 8pt, e.telephones.join(" · ")) }
    },
    align(right)[
      #text(size: 13pt, weight: "bold")[LIVRET DU JARDIN] \
      #text(size: 9.5pt)[#l.periode · #l.annee]
    ],
  )
  v(1mm)
  line(length: 100%, stroke: 0.6pt)
  if l.etat != "" {
    align(center, text(size: 10pt, weight: "bold", upper(l.etat)))
  }
  v(1mm)
  // La photo seulement si elle existe : aucun cadre à sa place.
  grid(
    columns: if l.photo != none { (1fr, auto) } else { (1fr,) },
    column-gutter: 6mm,
    [
      #text(size: 11pt)[*#l.eleve*] \
      #if l.naissance != none [Né(e) le #l.naissance \ ]
      Classe : *#l.classe* · #l.section \
      Année scolaire #l.annee · #l.periode
    ],
    ..if l.photo != none { (image(l.photo, height: 26mm),) } else { () },
  )
  v(2mm)
  // Légende des trois niveaux.
  text(size: 8.5pt, l.echelle.map(n => [#picto(n.picto) #h(1mm) #n.libelle]).join(h(6mm)))
  v(2mm)
}

#let domaine(l, dom) = {
  block(breakable: false, width: 100%, {
    table(
      columns: (1fr, ..l.periodes.map(_ => 32mm)),
      stroke: 0.4pt + black,
      inset: 4pt,
      align: (left + horizon, ..l.periodes.map(_ => left + horizon)),
      table.header(
        table.cell(fill: luma(225))[*#dom.libelle*],
        ..l.periodes.map(p => table.cell(fill: luma(225))[*#p*]),
      ),
      ..dom.competences.map(c => ([#c.libelle], ..c.niveaux.map(niveau))).flatten(),
    )
    if dom.commentaire != "" {
      v(-1mm)
      block(width: 100%, stroke: (x: 0.4pt, bottom: 0.4pt), inset: 4pt, emph(dom.commentaire))
    }
  })
  v(2mm)
}

#let bas(l) = {
  if l.appreciation_generale != "" {
    block(width: 100%, stroke: 0.4pt, inset: 6pt)[*Appréciation :* #l.appreciation_generale]
  }
  if l.absences != none {
    block[Absences de la période : #l.absences demi-journée(s).]
  }
  if l.decision != none {
    block[*Décision de fin d'année :* #l.decision]
  }
  v(4mm)
  grid(
    columns: (1fr, 1fr, 1fr, auto),
    column-gutter: 6mm,
    [#strong(underline[La maîtresse ou le maître]) #v(16mm)],
    [#strong(underline[La direction]) #v(16mm)],
    [#strong(underline[Les parents]) #v(16mm)],
    align(center)[
      #image(l.qr, width: 16mm) \
      #text(size: 5.5pt, l.reference)
    ],
  )
}

#for (i, l) in d.livrets.enumerate() {
  if i > 0 { pagebreak() }
  entete(l)
  for dom in l.domaines { domaine(l, dom) }
  // Le bas de page reste d'un seul tenant, sur la dernière page de l'enfant.
  block(breakable: false, width: 100%, bas(l))
}
