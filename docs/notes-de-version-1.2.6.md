# GESCO 1.2.6 — notes de version

Version du 31 octobre 2026. Elle laisse chaque école régler les tranches d'ancienneté de ses
impayés, retrouve l'élève d'une convocation par son nom et remplit les appréciations vides des
bulletins d'après la moyenne.

## Ce qui change

**Tranches des impayés.** Dans **Paramètres → Établissement**, section « Frais et caisse », la
ligne « Tranches d'ancienneté des impayés, en jours » prend trois nombres croissants, par exemple
`15,30,60` (réglage livré) : 0 à 15 jours, 16 à 30, 31 à 60, plus de 60. Une école plus stricte
écrit `7,15,30`, une école plus souple `30,60,90`. L'écran des impayés et le tableau de bord
affichent les tranches de l'école. GESCO montre les retards ; il ne renvoie personne et
n'envoie aucune relance de lui-même.

**Convocation.** L'élève ne se choisit plus dans une longue liste : on tape quelques lettres de
son nom, de ses prénoms ou de son matricule, comme au guichet, puis on clique sur le bon.

**Appréciations des bulletins.** Le bouton « Remplir les appréciations vides » met, pour toute la
classe, l'appréciation qui correspond à la moyenne de chaque matière (16, 14, 12, 11, 10, 8 et 6
sur 20). Ce qu'un enseignant a déjà écrit est gardé ; l'appréciation de la direction reste à la
main ; tout reste modifiable.

## Mettre à jour

Installez la 1.2.6 par-dessus la version en place : les données sont gardées. Les impayés se
rangent aussitôt dans les nouvelles tranches livrées.
