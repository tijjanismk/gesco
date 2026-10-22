# GESCO 1.2.1 — notes de version

Version du 19 octobre 2026. Elle ajoute à la 1.2.0 l'import des classes et des matières, et
rend un poste plus facile à dépanner.

## Ce qui est nouveau

**Import des classes et des matières.** En bas de **Paramètres → Établissement**, la section
repliée « Importer les classes et les matières (Excel) » crée en une fois les classes de l'année
en cours et les matières qui manquent, sans toucher au programme de chaque niveau. Le modèle a
deux feuilles, déjà remplies de ce que l'école a : « Classes » (niveau, série au lycée, nom de
la classe) et « Matières » (matière, code, nom court, groupe). Chaque feuille est facultative ;
un CSV qui porte l'une ou l'autre est reconnu à ses colonnes. Ce qui existe déjà reste tel
quel. GESCO montre chaque ligne avant d'importer : niveau inconnu ou d'un cycle fermé, série
manquante au lycée, groupe inconnu, code déjà pris, ligne en double. Une sauvegarde est faite
avant l'import.

Deux classeurs d'exemple accompagnent la version : `classes-et-matieres.xlsx` et
`programme-cycles-1-et-2.xlsx` (programme des cycles 1 et 2 ; ses coefficients et heures du
2e cycle sont à remplacer par ceux de l'école).

**Guide d'installation.** `GESCO-guide-installation.pdf` décrit l'installation sur un
ordinateur, de la machine vide à l'école prête : vérifier le fichier téléchargé, le message de
Windows, le premier démarrage et la phrase de secours, la licence, la sauvegarde, la mise à
jour, le changement d'ordinateur.

**Plus facile à dépanner.** Après un premier essai sur un poste neuf, où « Créer
l'établissement » n'a mené nulle part :
- une panne interne pendant une opération s'affiche en message d'erreur, au lieu d'un bouton
  grisé qui attend sans fin ;
- une création d'établissement qui dépasse vingt secondes le dit, et dit quoi faire ;
- sur l'écran des douze mots, le bouton « Continuer » reste grisé tant que la case « J'ai
  imprimé le certificat et je l'ai rangé hors du poste » n'est pas cochée ; une phrase le dit
  maintenant sous le bouton ;
- le bouton « Créer l'établissement » affiche « Création de l'établissement en cours… » pendant
  le travail ;
- chaque erreur est notée, avec l'heure, dans le fichier `journal-technique.txt` du dossier
  `%APPDATA%\ml.gesco.poste` : la commande et la nature de l'erreur, jamais les données
  saisies ni les mots de passe. Le lancement de GESCO et chaque étape de la création de
  l'établissement y sont notés aussi, avec leur durée. En cas de souci, c'est ce fichier qu'on
  envoie à l'assistance ;
- une panne inattendue d'un écran affiche un message et un bouton « Recharger l'écran », au
  lieu d'une page figée.

## Mettre à jour

Installez la 1.2.1 par-dessus la version en place : les données sont gardées. Depuis la 1.2.0,
rien d'autre ne change. Depuis la 1.0 ou la 1.1, voir aussi les notes de version 1.2.0 : les
rôles livrés reçoivent les nouveaux droits, et les messages aux familles ne paraissent que si
l'école les allume.
