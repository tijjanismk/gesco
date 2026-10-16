// Document d'essai (spécification §10.1) : valide la chaîne composition, écriture,
// enregistrement, aperçu et impression, sans aucune donnée métier.
#import "/commun/entete.typ": page-standard

#let d = json("/donnees.json")

#page-standard(d.etablissement, d.titre)[
  Ce document vérifie que le moteur documentaire de l'établissement compose, enregistre et
  imprime correctement. Il a été produit le #d.date par #d.auteur.

  #v(4mm)
  #let lignes = d.at("lignes", default: ())
  #if lignes.len() > 0 {
    table(
      columns: (1fr, auto),
      stroke: 0.5pt,
      inset: 6pt,
      table.header[*Élément*][*Valeur*],
      ..lignes.map(l => (l.libelle, l.valeur)).flatten(),
    )
  }

  #v(6mm)
  Contrôle des caractères du bambara : ɛ ɔ ɲ ŋ Ɛ Ɔ Ɲ Ŋ.
]
