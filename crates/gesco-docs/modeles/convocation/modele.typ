// Billet de convocation (SPEC_PHASE_5 §7) : le responsable convoqué, l'élève, le rendez-vous et
// son motif ; un coupon à rapporter signé, qui permet de confirmer la réception.
#import "/commun/entete.typ": page-standard

#let d = json("/donnees.json")

#page-standard(d.etablissement, "Convocation")[
  #align(center, text(size: 14pt, weight: "bold")[CONVOCATION])
  #v(4mm)

  À l'attention de *#d.responsable*, au sujet de *#d.eleve* (#d.classe).

  #v(2mm)
  Vous êtes prié(e) de vous présenter à l'école le *#d.date* à *#d.heure*.

  #v(2mm)
  *Motif :* #d.motif

  #v(6mm)
  #align(right)[Fait le #d.emise_le \ #d.emise_par]

  #v(8mm)
  #line(length: 100%, stroke: (dash: "dashed", thickness: 0.5pt))
  #v(2mm)
  #text(size: 0.9em)[
    *Coupon à rapporter à l'école*

    Je soussigné(e) #box(width: 1fr, repeat[.]), parent de #d.eleve (#d.classe), ai bien reçu la
    convocation du #d.date à #d.heure.

    #v(6mm)
    Date et signature : #box(width: 1fr, repeat[.])
  ]
]
