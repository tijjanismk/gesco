// Affiche de classe du portail des élèves (phase 12, lot 4) : l'adresse en grand, son code QR
// et trois étapes.
#import "/commun/entete.typ": entete

#let d = json("/donnees.json")

#set document(title: "Les cours sur ton téléphone", author: d.etablissement.nom)
#set page(paper: "a4", margin: (x: 18mm, y: 16mm))
#set text(font: "Charis SIL", size: 16pt, lang: "fr")

#entete(d.etablissement)
#v(10mm)
#align(center, text(size: 32pt, weight: "bold")[Les cours sur ton téléphone])
#v(10mm)
#align(center, image(d.qr, width: 70mm))
#v(6mm)
#align(center, text(size: 30pt, weight: "bold", d.adresse))
#v(12mm)
#set enum(numbering: "1.", spacing: 8mm)
+ Connecte-toi au Wi-Fi de l'école.
+ Ouvre l'adresse #text(weight: "bold", d.adresse) ou photographie le code.
+ Entre ton matricule et ton code.
