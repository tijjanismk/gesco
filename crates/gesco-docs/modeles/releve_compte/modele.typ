// Relevé de compte (SPEC_PHASE_3 §4.4) : par année, la facture, ses échéances avec remise,
// payé et reste, ce qui était payé avant GESCO, les versements avec leur reçu. Remis au parent qui demande où il en est.
#import "/commun/entete.typ": page-standard

#let d = json("/donnees.json")
#let e = d.etablissement
// Largeur d'une colonne réglée par l'école, en millimètres ; sinon celle du modèle.
#let largeur(nom, defaut) = {
  let mm = e.at("largeurs", default: (:)).at(nom, default: 0)
  if mm > 0 { mm * 1mm } else { defaut }
}

#page-standard(d.etablissement, "Relevé de compte")[
  #text(size: 11pt)[*#d.eleve* · matricule #d.matricule] \
  #text(size: 9pt)[Établi le #d.date]

  #for i in d.inscriptions {
    v(4mm)
    [== #i.annee · #i.classe]
    if i.facture == none {
      [Aucune facture : l'inscription n'est pas validée.]
    } else {
      let f = i.facture
      [Facture #f.numero · plan « #f.plan » · total #f.total · remises et baisses #f.remise · payé #f.paye · *reste #f.reste*]
      table(
        columns: (largeur("echeance", 1fr), largeur("date", auto), largeur("du", auto), largeur("remise", auto), largeur("paye", auto), largeur("reste", auto)),
        stroke: 0.4pt,
        inset: 4pt,
        align: (left, left, right, right, right, right),
        table.header[*Échéance*][*Date*][*Dû*][*Remise ou baisse*][*Payé*][*Reste*],
        ..f.echeances.map(e => ([#e.libelle], [#e.date], [#e.montant], [#e.remise], [#e.paye], [#e.reste])).flatten(),
      )
      // Payé avant GESCO : en tête des paiements, sans reçu, avec sa date d'arrêt.
      let r = i.at("reprise", default: none)
      let avant = if r == none or r.montant == none { () } else {
        ([Payé avant GESCO], [#r.arret], [—], [#r.montant])
      }
      if i.versements.len() > 0 or avant.len() > 0 {
        table(
          columns: (largeur("recu", auto), largeur("date_versement", auto), largeur("mode", 1fr), largeur("montant", auto)),
          stroke: 0.4pt,
          inset: 4pt,
          align: (left, left, left, right),
          table.header[*Reçu*][*Date*][*Mode*][*Montant*],
          ..avant,
          ..i.versements.map(v => ([#v.numero], [#v.date], [#v.mode], [#v.montant])).flatten(),
        )
      }
      if r != none {
        for x in r.rembourse {
          [Remboursé au parent le #x.date (#x.mode) : #x.montant \ ]
        }
        if r.a_rembourser != none {
          [*Reste à rembourser au parent : #r.a_rembourser*]
        }
      }
    }
  }

  #v(8mm)
  #grid(
    columns: (1fr, auto),
    [La comptabilité #v(14mm)],
    align(center)[
      #image(d.qr, width: 22mm) \
      #text(size: 6pt, d.reference)
    ],
  )
]
