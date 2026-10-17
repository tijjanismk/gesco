// Bulletins d'une classe (SPEC_PHASE_4 §7.3, T-32) : une page par élève, un seul PDF à
// imprimer. Deux mises en page : 1er cycle, mensuel, notes sur 10 sans coefficient ; 2e cycle et
// lycée, trimestriel, note de classe, composition, moyenne, coefficient et points. L'école choisit
// les colonnes et les lignes du bilan qui s'impriment (`d.afficher`, réponse du 12 octobre 2026) ;
// la matière et sa moyenne s'impriment toujours.
#let d = json("/donnees.json")
#let e = d.etablissement
#let a = d.afficher

#set document(title: "Bulletins", author: e.nom)
// En A5 (réglage de l'école, le format de la plupart des écoles) : marges et texte réduits.
#let a5 = e.papier == "a5"
#set page(paper: if a5 { "a5" } else { "a4" }, margin: if a5 { (x: 7mm, top: 7mm, bottom: 8mm) } else { (x: 12mm, top: 10mm, bottom: 12mm) })
#set text(font: "Charis SIL", size: if a5 { 6.8pt } else { 8.5pt }, lang: "fr")

#let entete-bulletin(b) = {
  grid(
    columns: (auto, 1fr, auto),
    column-gutter: 8pt,
    align: horizon,
    if e.logo != none { image(e.logo, height: if a5 { 11mm } else { 15mm }) },
    {
      text(size: if a5 { 9pt } else { 11pt }, weight: "bold", e.nom)
      if e.adresse != "" { linebreak(); text(size: 7.5pt, e.adresse) }
      if e.telephones.len() > 0 { linebreak(); text(size: 7.5pt, e.telephones.join(" · ")) }
    },
    align(right)[
      #text(size: if a5 { 9.5pt } else { 12pt }, weight: "bold")[BULLETIN DE NOTES] \
      #text(size: 9pt)[#b.periode · #b.annee]
    ],
  )
  v(1mm)
  line(length: 100%, stroke: 0.5pt)
  if b.etat != "" {
    align(center, text(size: 10pt, weight: "bold", fill: rgb("#9a3412"), upper(b.etat)))
  }
  v(1mm)
  grid(
    columns: (1fr, auto),
    [
      Élève : *#b.eleve* · matricule #b.matricule \
      Classe : *#b.classe* (#b.niveau) · effectif #b.effectif
      #if b.arrive_le != none [ \ Arrivée dans l'école : #b.arrive_le]
    ],
    align(right)[Notes sur #b.bareme],
  )
  v(2mm)
}

#let cellule(t) = if t == "" { [—] } else { [#t] }

// Une colonne : son titre, sa largeur, son alignement, sa cellule pour une matière et sa case
// de la ligne des totaux.
// Les colonnes de chiffres ont une largeur fixe : elles s'alignent d'un bulletin à l'autre.
#let chiffres = if a5 { 12mm } else { 17mm }

#let colonne(titre, cellule-de, largeur: chiffres, aligne: right, total: b => []) = (
  titre: titre, largeur: largeur, aligne: aligne, cellule: cellule-de, total: total,
)

#let colonnes(b) = {
  let premier = b.premier_cycle
  let cols = (
    // La matière prend la place de son nom quand l'appréciation, imprimée, prend le reste de la
    // ligne ; sinon toute la largeur restante.
    colonne([*Matière*], l => [#l.matiere], largeur: if a.colonne_appreciation { auto } else { 1fr },
      aligne: left, total: b => if premier [*Total*] else [*Totaux*]),
  )
  if not premier and a.colonne_note_de_classe { cols.push(colonne([*Note de classe*], l => cellule(l.note_de_classe))) }
  if not premier and a.colonne_composition { cols.push(colonne([*Compo.*], l => cellule(l.composition))) }
  cols.push(colonne(if premier [*Note*] else [*Moyenne*], l => cellule(l.moyenne),
    total: b => if premier [*#b.total_points*] else []))
  if not premier and a.colonne_coefficient {
    cols.push(colonne([*Coef.*], l => [#l.coefficient], largeur: if a5 { 8mm } else { 11mm }, aligne: center, total: b => [*#b.total_coefficients*]))
  }
  if not premier and a.colonne_points {
    cols.push(colonne([*Moy. × coef.*], l => cellule(l.points), total: b => [*#b.total_points*]))
  }
  if a.colonne_moyenne_classe { cols.push(colonne([*Moy. classe*], l => cellule(l.moyenne_classe))) }
  if a.colonne_appreciation { cols.push(colonne([*Appréciation*], l => [#l.appreciation], largeur: 1fr, aligne: left)) }
  cols
}

#let tableau(b) = {
  let cols = colonnes(b)
  table(
    columns: cols.map(c => c.largeur),
    stroke: 0.4pt,
    inset: 3.5pt,
    align: cols.map(c => c.aligne),
    table.header(..cols.map(c => c.titre)),
    ..b.lignes.map(l => cols.map(c => (c.cellule)(l))).flatten(),
    // La ligne des totaux n'a de sens qu'avec un total à montrer.
    ..if b.premier_cycle or a.colonne_coefficient or a.colonne_points { cols.map(c => (c.total)(b)) } else { () },
  )
}

#let bilan(b) = {
  let gauche = ()
  if a.moyenne_generale { gauche += ([*Moyenne générale*], [*#cellule(b.moyenne) / #b.bareme*]) }
  if a.rang { gauche += ([Rang], [#cellule(b.rang) sur #b.ont_compose]) }
  if a.mention { gauche += ([Mention], [#cellule(b.mention)]) }
  // L'avertissement et le blâme du conseil de classe de la période.
  let sanctions = b.at("sanctions", default: ())
  if sanctions.len() > 0 { gauche += ([Conseil de classe], [#sanctions.join(linebreak())]) }
  if b.moyenne_annuelle != none {
    gauche += ([*Moyenne annuelle*], [*#b.moyenne_annuelle / #b.bareme*])
    if a.rang { gauche += ([Rang annuel], [#cellule(b.rang_annuel)]) }
  }
  let droite = ()
  if a.ont_compose { droite += ([Élèves qui ont composé], [#b.ont_compose / #b.effectif]) }
  if a.premier_dernier {
    droite += ([Moyenne du premier], [#cellule(b.premier)], [Moyenne du dernier], [#cellule(b.dernier)])
  }
  if a.moyenne_classe { droite += ([Moyenne de la classe], [#cellule(b.moyenne_classe)]) }
  if a.absences {
    droite += (
      [Absences (demi-journées)], [#b.absences.demi_journees, dont #b.absences.justifiees justifiées],
      [Retards], [#b.absences.retards],
    )
  }
  grid(
    columns: (1fr, 1fr),
    column-gutter: 6mm,
    // Une colonne du bilan sans aucune ligne choisie ne trace pas de cadre vide.
    if gauche.len() > 0 { table(columns: (1fr, auto), stroke: 0.4pt, inset: 3.5pt, ..gauche) },
    if droite.len() > 0 { table(columns: (1fr, auto), stroke: 0.4pt, inset: 3.5pt, ..droite) },
  )
}

#for (i, b) in d.bulletins.enumerate() {
  if i > 0 { pagebreak() }
  entete-bulletin(b)
  tableau(b)
  v(3mm)
  bilan(b)
  v(3mm)
  if a.appreciation_generale {
    block(width: 100%, stroke: 0.4pt, inset: 5pt)[
      *Appréciation de la direction :* #b.appreciation_generale
      #v(4mm)
    ]
  }
  v(4mm)
  grid(
    columns: (1fr, 1fr, auto),
    column-gutter: 6mm,
    [Le directeur #v(if a5 { 9mm } else { 14mm })],
    [Le parent #v(if a5 { 9mm } else { 14mm })],
    align(center)[
      #image(b.qr, width: if a5 { 14mm } else { 18mm }) \
      #text(size: 5.5pt, b.reference)
    ],
  )
}
