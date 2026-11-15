// Fiche scolaire de l'élève (SPEC_PHASE_9 §5.3, §9.4) : une page par élève. Ce que l'école
// connaît est imprimé ; une rubrique sans valeur laisse une ligne de points à remplir à la main.
#import "/commun/entete.typ": entete, pied

#let d = json("/donnees.json")
#let e = d.etablissement

#set document(title: "Fiche scolaire", author: e.nom)
#set page(paper: "a4", margin: (x: 18mm, top: 14mm, bottom: 18mm), footer: pied(e))
#set text(font: "Charis SIL", size: 10.5pt, lang: "fr")

#let champ(libelle, valeur) = {
  [#libelle : ]
  if valeur == "" {
    box(width: 1fr, repeat[.])
  } else {
    text(weight: "bold", valeur)
  }
}

#let deux(..champs) = grid(
  columns: (1fr, 1fr),
  column-gutter: 8mm,
  row-gutter: 3mm,
  ..champs,
)

#let fiche(f) = [
  #entete(e)
  #v(3mm)
  #align(center, text(size: 15pt, weight: "bold")[FICHE SCOLAIRE])
  #v(4mm)
  #deux(
    champ("École", f.ecole), champ("Commune de", f.commune),
    champ("Cercle de", f.cercle), champ("Région de", f.region),
  )
  #v(4mm)
  #deux(
    champ("Nom", f.nom), champ("Prénom", f.prenoms),
    champ("Né(e) le", f.naissance), champ("À", f.lieu),
    champ("Pièce d'état civil", f.piece), champ("N°", f.piece_numero),
  )
  #v(3mm)
  #champ("Délivré(e) par", f.delivree)
  #v(3mm)
  #deux(
    champ("Date de recrutement", f.recrutement), champ("N° matricule", f.matricule),
    champ("Fils / Fille de", f.pere), champ("Profession", f.pere_profession),
    champ("Et de", f.mere), champ("Profession", f.mere_profession),
  )
  #v(3mm)
  #champ("Domicilié(e) à", f.domicile)
  #v(4mm)
  #text(weight: "bold")[TUTEUR]
  #v(2mm)
  #champ("Nom et prénom", f.tuteur)
  #v(2mm)
  #deux(champ("Lien de parenté", f.tuteur_lien), champ("Adresse", f.tuteur_adresse))
  #v(5mm)
  #align(center, text(size: 12pt, weight: "bold")[SCOLARITÉ])
  #v(2mm)
  #for c in f.scolarite [
    #text(weight: "bold")[#c.cycle]
    #v(1mm)
    #set text(size: 9pt)
    #table(
      columns: (auto,) + (1fr,) * c.niveaux.len(),
      stroke: 0.4pt,
      inset: 4pt,
      align: center + horizon,
      [*Classe*], ..c.niveaux.map(n => [*#n*]),
      [Période], ..c.periodes.map(x => [#x]),
      [École], ..c.ecoles.map(x => [#x]),
      [Cercle], ..c.cercles.map(x => [#x]),
    )
    #v(1mm)
    #champ("Classes doublées", c.doublees)
    #v(3mm)
  ]
  #if f.origine != "" [
    #champ("École fréquentée auparavant", f.origine)
    #v(3mm)
  ]
  #text(weight: "bold")[Diplôme d'études fondamentales (DEF)]
  #v(2mm)
  #deux(champ("Date", f.def_date), champ("Résultat", f.def_resultat))
  #v(6mm)
  #align(right, text(weight: "bold", f.signataire))
]

#for (i, f) in d.fiches.enumerate() {
  if i > 0 {
    pagebreak()
  }
  fiche(f)
}
