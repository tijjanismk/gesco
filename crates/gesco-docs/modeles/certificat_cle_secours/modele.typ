// Certificat de clé de secours (blueprint §17.3, spécification §6.3). Il porte les douze mots
// qui, avec un fichier de sauvegarde, suffisent à reconstituer la base sur un autre poste (R-57).
#import "/commun/entete.typ": page-standard

#let d = json("/donnees.json")

#page-standard(d.etablissement, "Certificat de clé de secours")[
  Émis le #d.date par #d.emis_par. Identifiant de l'enveloppe : #raw(d.enveloppe_id).

  #v(4mm)
  Ces douze mots permettent de restaurer les sauvegardes de l'établissement sur un autre
  ordinateur, par exemple après une panne ou un vol du poste. Sans eux, une sauvegarde ne peut
  pas être ouverte ailleurs que sur le poste qui l'a produite.

  #v(6mm)
  #grid(
    columns: (1fr, 1fr, 1fr),
    row-gutter: 6mm,
    column-gutter: 4mm,
    ..d.mots.enumerate().map(((i, mot)) => box(
      width: 100%,
      inset: 8pt,
      stroke: 0.6pt,
      radius: 2pt,
      [#text(size: 9pt)[#(i + 1).] #h(4pt) #text(size: 15pt, weight: "bold", mot)],
    )),
  )

  #v(8mm)
  *Consignes de conservation*

  - Rangez ce certificat hors du poste, dans un lieu fermé à clé : coffre de la direction ou
    chez le promoteur.
  - Toute personne qui détient ces mots et un fichier de sauvegarde peut lire les données de
    l'établissement. Ne les photographiez pas, ne les envoyez pas par message.
  - Si le certificat est perdu ou a pu être vu, régénérez une nouvelle phrase depuis le poste :
    l'ancienne cessera de fonctionner pour les sauvegardes suivantes.
]
