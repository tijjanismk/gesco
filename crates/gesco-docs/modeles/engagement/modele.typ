// Lettre d'engagement (SPEC_PHASE_6 §3) : imprimable si l'école le veut, jamais exigée (T-40,
// T-42). Un fonctionnaire de l'État reçoit un engagement de vacation, pas un contrat de travail.
#import "/commun/entete.typ": page-standard

#let d = json("/donnees.json")

#let titres = (
  duree_indeterminee: "Lettre d'engagement",
  duree_determinee: "Lettre d'engagement à durée déterminée",
  vacation: "Engagement de vacation",
  stage: "Convention de stage",
  benevole: "Engagement bénévole",
)

#page-standard(d.etablissement, titres.at(d.nature))[
  #v(4mm)
  La direction de l'établissement engage

  #align(center)[#text(size: 14pt, weight: "bold", d.membre)]

  matricule #d.matricule#if d.fonctionnaire [, fonctionnaire de l'État#if d.numero_fonctionnaire != "" [ (n° #d.numero_fonctionnaire)]],
  en qualité de *#d.fonction*,
  #if d.fin == none [à compter du #d.debut.] else [du #d.debut au #d.fin.]

  #if d.nature == "vacation" [
    Les heures sont payées au taux de *#d.remuneration*, pour *#d.heures* prévues par semaine ;
    seules les heures effectuées, arrêtées chaque mois par la direction, sont dues.
  ] else if d.remuneration != "" [
    Rémunération : *#d.remuneration*.
  ]

  #if d.note != "" [
    #v(2mm)
    #d.note
  ]

  #v(12mm)
  #grid(
    columns: (1fr, 1fr),
    [Fait le #d.date \ La direction #v(18mm)],
    [Lu et accepté \ L'intéressé(e) #v(18mm)],
  )
]
