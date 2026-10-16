// Fiche d'inscription (SPEC_PHASE_2 §9, T-08) : remise au parent et signée. Le bloc des
// responsables n'est imprimé que si l'école le veut ; le QR code renvoie au document enregistré.
#import "/commun/entete.typ": page-standard

#let d = json("/donnees.json")
#let e = d.eleve

#let ligne(libelle, valeur) = if valeur != none and valeur != "" { ([#libelle], [#valeur]) } else { () }

#page-standard(d.etablissement, "Fiche d'inscription")[
  #grid(
    columns: (1fr, auto),
    column-gutter: 8mm,
    [
      #text(size: 12pt)[Année scolaire *#d.annee* · Classe *#d.classe*] \
      #text(size: 10pt)[#d.nature, le #d.date]
    ],
    if d.at("photo", default: none) != none {
      box(stroke: 0.5pt, image(d.photo, width: 28mm, height: 34mm, fit: "cover"))
    },
  )

  #v(4mm)
  == Élève
  #table(
    columns: (45mm, 1fr),
    stroke: 0.4pt,
    inset: 5pt,
    ..ligne("Prénoms", e.prenoms),
    ..ligne("Nom", e.nom),
    ..ligne("Matricule", e.matricule),
    ..ligne("Matricule national", e.at("matricule_national", default: none)),
    ..ligne("Sexe", e.sexe),
    ..ligne("Né(e) le", e.date_naissance),
    ..ligne("À", e.lieu_naissance),
    ..ligne("Nationalité", e.nationalite),
    ..ligne("Pièce d'état civil", e.at("acte", default: none)),
    ..ligne("École d'origine", e.at("ecole_origine", default: none)),
  )

  #let responsables = d.at("responsables", default: none)
  #if responsables != none and responsables.len() > 0 {
    v(3mm)
    [== Responsables]
    table(
      columns: (1fr, auto, auto),
      stroke: 0.4pt,
      inset: 5pt,
      table.header[*Nom*][*Lien*][*Téléphones*],
      ..responsables.map(r => ([#r.nom \ #text(size: 8pt, r.qualites)], [#r.lien], [#r.telephones])).flatten(),
    )
  }

  #v(8mm)
  #grid(
    columns: (1fr, 1fr, auto),
    column-gutter: 6mm,
    [Signature du parent ou du responsable #v(18mm)],
    [Le secrétariat #v(18mm)],
    align(center)[
      #image(d.qr, width: 24mm) \
      #text(size: 6pt, d.reference)
    ],
  )
]
