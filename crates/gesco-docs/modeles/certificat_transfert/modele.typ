// Certificat de transfert (SPEC_PHASE_4 §9) : numéroté CTR-…, il atteste que l'élève a fréquenté
// l'école et qu'il la quitte pour une autre. Au lycée, il porte le matricule national (Q-32).
#import "/commun/entete.typ": page-standard

#let d = json("/donnees.json")

#page-standard(d.etablissement, "Certificat de transfert")[
  #align(right)[*N° #d.numero*]
  #v(6mm)
  Le directeur de l'établissement soussigné certifie que l'élève

  #align(center)[#text(size: 14pt, weight: "bold", d.eleve)]

  #if d.naissance != "" [né(e) le #d.naissance, ]
  matricule #d.matricule#if d.matricule_national != none [, matricule national #d.matricule_national],
  a fréquenté l'établissement en classe de *#d.classe* pendant l'année scolaire #d.annee,
  du #d.inscrit_le au #d.date_depart, et le quitte pour poursuivre sa scolarité à
  *#d.destination*.

  #v(4mm)
  En foi de quoi le présent certificat lui est délivré pour servir et valoir ce que de droit.

  #v(10mm)
  #grid(
    columns: (1fr, auto),
    [Fait le #d.date \ Le directeur #v(18mm)],
    align(center)[
      #image(d.qr, width: 24mm) \
      #text(size: 6pt, d.reference)
    ],
  )
]
