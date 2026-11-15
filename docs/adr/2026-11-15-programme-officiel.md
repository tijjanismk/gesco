# Le programme officiel dans GESCO : des chapitres, livrés avec la bibliothèque, complétés par l'école

**Date** : 15 novembre 2026
**Statut** : proposé le 15 novembre 2026 — décision 11 de `docs/plan-phase-12.md` §5, lot 1 de la phase 12 (`SPEC_PHASE_12_LOT_1.md`)

## Contexte

Le programme malien est la colonne vertébrale de la bibliothèque (plan de la phase 12, §0). Trois
questions restaient : à quel grain on le saisit (chapitre seul, ou objectifs et compétences),
quelle version fait foi, qui le saisit.

Ce qui existe le 15 novembre 2026 :
- `gesco-bibliotheque\squelette-programme-malien.csv` : 9e année (87 lignes en douze matières,
  programmes de juin 1990) et terminales (1 134 lignes en 53 couples série et matière, programme
  du 4 septembre 2013, écrit par compétences). 829 lignes des terminales ont été retrouvées par un
  contrôle automatique ; 339 lignes d'arabe, de bamanankan et de russe ne sont pas contrôlées ;
  aucune ligne n'a été relue par un enseignant.
- Manques du document officiel : mathématiques de TSE et TSExp, histoire-géographie de ces
  séries, histoire de TSEco. Les mathématiques de TSE ont été rédigées d'après les épreuves du
  BAC, chaque leçon marquée « programme à confirmer ».
- Les leçons rédigées sont rangées par dossiers de chapitres (`01-nombres-reels`, `02-cercle`…)
  qui suivent le programme de leur classe ; pour la 7e et la 8e, d'après les programmes de
  `gesco-bibliotheque\textes\programmes`.
- Le programme par niveau existe déjà dans GESCO, mais au grain de la matière (coefficient,
  heures par semaine), pas du chapitre.
- Décision du porteur du 14 novembre 2026 : tout ce qui est écrit entre dans GESCO, sans ordre de
  classes, matières incomplètes comprises ; l'école complète dans l'application. Principe : « la
  flexibilité sans incohérence » — ne pas faire remarquer ce qui manque.

## Options

| | 1. Chapitres de la bibliothèque, l'école ajoute les siens | 2. Programme entier du squelette, chapitres vides compris | 3. Objectifs et compétences, sous chaque chapitre |
|---|---|---|---|
| Apprentissage | Faible | Faible | Élevé : un référentiel par compétences à saisir |
| Exploitation | Un chapitre livré a toujours au moins un contenu | Des centaines de chapitres vides, dont 339 lignes non contrôlées | Saisie longue, à faire relire |
| Maturité | Le découpage des leçons est déjà fait | Le squelette n'est pas relu | Le programme de 2013 seul est écrit par compétences |
| Pour un agent de code | Bon | Bon | Moyen |
| Plus gros risque | Un chapitre du programme absent du catalogue tant que personne ne l'écrit | Une matière entière affichée vide, qui fait remarquer ce qui manque ; des intitulés faux | Un travail énorme pour un usage que rien ne demande encore |

## Décision

Option 1.

- **Grain** : le chapitre, rangé dans une partie (« Algèbre », « Géométrie », « Physique »,
  « Chimie », « Examens »…), dans l'ordre du programme. Une leçon appartient à un chapitre.
  Objectifs et compétences ne sont pas saisis ; le « point du programme » que chaque leçon livrée
  porte en tête reste dans le texte de la leçon, sous son titre.
- **Ce qui est livré** : les chapitres qui ont au moins un contenu, tirés du découpage de la
  bibliothèque par l'outil de fabrication du paquet ; pour chaque matière d'examen qui a des
  sujets, une partie « Examens » avec un chapitre « Sujets du DEF » ou « Sujets du BAC ». Les
  chapitres du squelette qui n'ont encore aucun contenu ne sont pas livrés. Cela revoit
  l'exigence E-2 (« le programme entier, même vide »), par la règle du 14 novembre 2026.
- **Version qui fait foi** : celle que suit le découpage de la bibliothèque. Le manifeste du
  paquet la nomme pour l'éditeur ; elle n'est pas affichée (aucun texte affiché ne cite de
  source). Un chapitre bâti sans programme officiel porte « programme à confirmer » dans le
  manifeste ; seul le personnel qui écrit le voit, dans la liste de relecture.
- **Qui saisit** : l'éditeur, par le paquet, pour les chapitres livrés. L'école ajoute ses
  chapitres (origine `ecole`) sous ses niveaux et ses matières, avec le droit d'écrire
  (`library:edit`), et les ordonne parmi les chapitres livrés ; elle ne renomme pas un chapitre
  livré, elle le retire et en ajoute un.
- **Mise à jour** : un paquet nouveau peut ajouter des chapitres, en renommer ou en réordonner
  par leur clé ; les chapitres de l'école gardent leur place relative.

Aucune dépendance nouvelle.

## Justification

1. Le chapitre est le grain qu'un maître, un élève et un sujet d'examen partagent ; c'est aussi
   celui du découpage déjà fait.
2. Ne livrer que des chapitres qui ont un contenu respecte la règle du porteur : rien ne
   s'affiche vide pour faire remarquer un manque, et l'école ajoute ce qu'elle veut où elle veut.
3. Le squelette n'est pas relu ; livrer ses intitulés tels quels, dont 339 lignes non
   contrôlées, mettrait des erreurs sous les yeux des écoles.
4. Les objectifs et compétences ne servent à aucun écran du lot 1 ; ils pourront s'ajouter
   sous le chapitre sans rien reprendre.
5. Garder la version de référence dans le manifeste et hors de l'écran tient à la fois la trace
   et la règle de l'interface.

## Conséquences

Positives : un catalogue où chaque chapitre ouvre sur quelque chose ; aucun intitulé non relu ;
l'école complète au même endroit que la bibliothèque.

Négatives : une matière sans aucun contenu livré (philosophie de terminale, par exemple)
n'apparaît pas tant que l'école n'y a pas ajouté de chapitre ; on ne mesure pas, dans le lot 1, la
couverture du programme par les cours d'une école ; une école ne peut pas corriger un intitulé
livré autrement qu'en le retirant et en écrivant le sien.

## Contre-argument conservé

Afficher le programme entier, même vide, montre à l'école ce qu'elle doit enseigner et l'invite à
écrire chapitre par chapitre : c'est ce que disait l'exigence E-2. Il n'a pas gagné parce que le
porteur a posé ensuite que l'application ne fait pas remarquer ce qui manque, et que les
intitulés vides viendraient d'un squelette qu'aucun enseignant n'a relu.

## Conditions de reconsidération

- Si le porteur demande que le programme entier paraisse, chapitres vides compris (question posée
  dans `SPEC_PHASE_12_LOT_1.md`).
- Si le squelette est relu par un enseignant : ses chapitres vides pourront être livrés.
- Si le lot 3 ou le portail ont besoin d'objectifs par chapitre (quiz par compétence, suivi).
