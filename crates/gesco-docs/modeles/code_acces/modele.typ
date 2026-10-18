// Billet du code d'accès (SPEC_PHASE_5 §9) : remis en main propre au guichet, il est le seul
// endroit où le code est écrit ; la base n'en garde que l'empreinte. Il dit comment interroger
// l'école par SMS.
#import "/commun/entete.typ": page-standard

#let d = json("/donnees.json")

#page-standard(d.etablissement, "Code d'accès")[
  Remis à *#d.responsable*#if d.enfants != "" [, pour #d.enfants].

  #v(4mm)
  #align(center, block(
    stroke: 1pt,
    inset: (x: 10mm, y: 5mm),
    radius: 2mm,
    text(size: 24pt, weight: "bold", tracking: 3pt, d.code),
  ))
  #v(4mm)

  Ce code est personnel : gardez-le pour vous. Envoyez par SMS
  #if d.numero_ecole != "" [au *#d.numero_ecole*] else [au numéro de l'école] :

  #v(2mm)
  #table(
    columns: (auto, 1fr),
    stroke: none,
    inset: (x: 0pt, y: 3pt),
    [*SOLDE* code], [ce qui reste à payer et la prochaine échéance],
    [*ABS* code], [les absences de la période en cours],
    [*NOTE* code], [la moyenne du dernier bulletin publié],
    [*AIDE*], [la liste des mots],
  )

  Exemple : *SOLDE #d.code.replace(" ", "")*

  #v(2mm)
  #text(size: 0.9em)[
    Seuls vos numéros enregistrés à l'école reçoivent une réponse. Un code perdu se remplace au
    guichet : l'ancien ne sert plus.
  ]

  #v(6mm)
  #align(right)[Remis le #d.remis_le \ #d.remis_par]
]
