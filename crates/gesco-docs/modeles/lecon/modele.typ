// Leçons de la bibliothèque (SPEC_PHASE_12_LOT_1 §11) : une leçon, un sujet, un corrigé, la
// liste des faits à vérifier ou un chapitre entier, en A4. Chaque contenu commence sur une page.
// En-tête : nom de l'école, niveau, matière ; pied : titre et numéro de page. Aucune autre
// mention. Le corps de chaque contenu est un balisage Typst préparé par GESCO, où tout texte
// est échappé ; les formules y sont déjà converties.
#let d = json("/donnees.json")
#let e = d.etablissement

// Le contenu en cours sur une page : la dernière marque posée jusqu'à elle.
#let courant() = {
  let p = here().page()
  let marques = query(<contenu>).filter(m => m.location().page() <= p)
  if marques.len() > 0 { marques.last().value } else { none }
}

#set document(title: d.titre, author: e.nom)
#set page(
  paper: "a4",
  margin: (x: 20mm, top: 24mm, bottom: 20mm),
  header: context {
    let c = courant()
    set text(size: 8.5pt)
    grid(
      columns: (1fr, auto),
      column-gutter: 8pt,
      e.nom,
      if c != none { (c.niveau, c.matiere).filter(t => t != "").join(" · ") },
    )
    v(-2mm)
    line(length: 100%, stroke: 0.4pt)
  },
  footer: context {
    let c = courant()
    set text(size: 8.5pt)
    grid(
      columns: (1fr, auto),
      column-gutter: 8pt,
      if c != none { c.titre },
      counter(page).display(),
    )
  },
  background: if d.brouillon {
    rotate(-40deg, text(size: 96pt, weight: "bold", fill: luma(228))[Brouillon])
  },
)
#set text(font: "Charis SIL", size: 11pt, lang: "fr")
#show math.equation: set text(font: "New Computer Modern Math")
#set par(leading: 0.7em, spacing: 1.1em)
#show heading.where(level: 1): set text(size: 16pt)
#show heading.where(level: 2): set text(size: 13pt)
#show heading.where(level: 3): set text(size: 11.5pt)
#show heading.where(level: 4): set text(size: 11pt)
#show heading: set block(above: 1.3em, below: 0.7em)
#set table(stroke: 0.5pt, inset: 5pt)
#show table: set block(width: 100%)
#show quote: set block(inset: (left: 4mm))

#for (i, c) in d.documents.enumerate() {
  if i > 0 { pagebreak() }
  [#metadata(c) <contenu>]
  heading(level: 1, c.titre)
  if c.sous_titre != "" {
    text(size: 10pt, style: "italic", c.sous_titre)
    v(2mm)
  }
  eval(c.corps, mode: "markup")
}
