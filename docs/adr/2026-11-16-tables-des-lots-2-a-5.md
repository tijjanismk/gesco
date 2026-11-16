# Tables des lots 2 à 5 de la bibliothèque : quatre migrations, ce qui vit par année à part de ce qui dure

**Date** : 16 novembre 2026
**Statut** : proposé le 16 novembre 2026, accepté le 16 novembre 2026 par le porteur du projet
**Précise** : ADR « tables de la bibliothèque » (15 novembre 2026, migration 0061) ; décision 6
de `docs/plan-phase-12.md` §5 ; blueprint §16.1
**Spécifications** : `SPEC_PHASE_12_LOT_2.md` à `SPEC_PHASE_12_LOT_5.md`

## Contexte

- La migration 0061 a posé les tables du lot 1, sans année scolaire : programme, contenus,
  versions, relectures, retraits, fichiers, vidéos, droit de publier, correspondance, index FTS5.
  Aucune ligne ne s'y supprime (déclencheurs). La prochaine migration libre est la 0063.
- Les lots suivants ajoutent ce qui **vit par année scolaire** (avancement des élèves, tentatives
  de quiz, devoirs et rendus) et quelques précisions de ce qui **dure** (sons, quiz de chapitre,
  portée d'un ajout, versions reçues).
- Deux contraintes `CHECK` du lot 1 bornent des natures : `fichier_bibliotheque.nature` (image,
  document) et `contenu.nature` (leçon, sujet, corrigé, document). SQLite ne modifie pas une
  contrainte : il faut reconstruire la table.
- Le blueprint §16.1 prévoyait des tables quiz, question, réponse, devoir en ligne, soumission,
  progression ; l'ADR « format des quiz » garde les questions dans le texte des leçons.
- Les lots se livrent dans l'ordre 2, 4, 3, 5, tous dans la 2.0.0.

## Options

| | 1. Une migration par lot, tables par année à part, reconstruction des deux tables bornées | 2. Une seule migration pour les quatre lots, écrite d'avance | 3. Tables du blueprint telles quelles |
|---|---|---|---|
| Apprentissage | Faible | Faible | Moyen |
| Exploitation | Quatre mises à niveau, chacune essayée | Une mise à niveau, mais des tables vides pendant des semaines | Tables de questions inutiles (ADR « format des quiz ») |
| Maturité | Procédure des migrations du dépôt | Idem | — |
| Pour un agent de code | Bon : chaque lot porte sa migration | Mauvais : une migration à corriger après coup | Moyen |
| Plus gros risque | Une reconstruction qui perd un déclencheur | Une migration figée avant que le lot soit écrit | Deux sources pour un même quiz |

## Décision

Chaque lot ajoute sa migration, la suivante libre au moment de l'écrire ; les tables qui vivent par
année portent `annee_id` ; les deux tables bornées du lot 1 sont reconstruites quand leur nature
s'élargit, avec leurs index et leurs déclencheurs refaits et un essai qui les compare avant et
après.

Aucune brique nouvelle.

Ce que la décision fixe (noms de travail ; le détail des colonnes est dans chaque spécification) :

| Lot | Migration (prévue) | Tables et changements |
|---|---|---|
| 2 | 0063 | `fichier_bibliotheque` reconstruite : nature `son` en plus ; `contenu_classe` (un ajout de l'école réservé à certaines classes du niveau) ; `version_recue` (version d'une leçon reçue d'une autre école quand un brouillon existe déjà) ; `import_lecons` (fichier importé, empreinte, quand, qui, nombre) |
| 4 | 0064 | `code_eleve` (empreinte du code, actif, remis le, par, révoqué le ; un seul actif par élève) ; `essai_code_eleve` (matricule, jour, nombre d'essais faux) ; `avancement_lecon` (élève, année, contenu, empreinte de la version lue, ouvert le, fini le) ; réglages du portail |
| 3 | 0065 | `contenu` reconstruite : nature `quiz` en plus ; `tentative_quiz` (élève, année, contenu ou chapitre, empreinte de la version, origine, commencée le, finie le, score, sur) ; `reponse_quiz` (tentative, contenu, empreinte, numéro de question, réponse donnée, juste) |
| 5 | 0066 | `devoir` (année, classe, matière, auteur, titre, consignes, date limite, barème, forme du rendu, leçon liée, état, copies rendues le) ; `rendu_devoir` (devoir, élève, numéro, origine portail ou papier, texte, rendu le, en retard, note, commentaire, noté le, par) ; `fichier_rendu` (empreinte, contenu, taille, sorte, libéré le) et `photo_rendu` (rendu, ordre, empreinte) ; `evaluation` : colonnes `origine` (`saisie` par défaut) et `devoir_id` |

Règles communes :

- **Aucune ligne ne se supprime** (déclencheurs), comme au lot 1 ; seule exception, le contenu
  d'une photo de rendu peut être vidé après la clôture de l'année (ADR « copies
  photographiées ») : la ligne reste, `libere_le` est rempli.
- **Ce qui vit par année** porte `annee_id` et suit la clôture d'année existante : une année close
  ne reçoit plus d'avancement, de tentative ni de rendu.
- **L'élève** est désigné par son identifiant d'élève ; la classe se lit par son inscription de
  l'année. Aucune table du portail ne copie un nom.
- **Les empreintes de version** (SHA-256 du texte visible) lient un avancement ou une tentative au
  texte réellement lu, sans copier ce texte.
- **Reconstruction d'une table** : nouvelle table, copie de toutes les lignes, ancienne retirée,
  nouvelle renommée, index, index uniques partiels et déclencheurs refaits, `foreign_key_check`
  passé ; essai sur une base 2.0 remplie (école d'essai) qui compare le nombre de lignes, les
  index et les déclencheurs avant et après, et sur une base 1.2 mise à niveau en une fois.
- **Index plein texte** : la reconstruction de `contenu` refait les déclencheurs de
  `recherche_bibliotheque` ; un essai cherche un mot avant et après.

## Justification

Une migration par lot suit la façon de travailler du projet (une étape, un commit) et ne fige pas
les colonnes d'un lot avant qu'il soit écrit. Séparer ce qui vit par année de ce qui dure garde la
bibliothèque hors de la clôture d'année, comme au lot 1, et range l'activité des élèves avec le
reste de l'année. Reconstruire deux tables est une opération connue, à condition de refaire les
déclencheurs qui empêchent les suppressions et ceux de l'index : les essais le vérifient.

## Conséquences

Positives : chaque lot se livre seul ; les tables par année se clôturent avec l'année ; aucune
donnée d'élève n'est copiée.

Négatives : deux reconstructions de tables, les opérations de migration les plus risquées du lot ;
quatre mises à niveau de plus à essayer depuis une base 1.2 ; une colonne nouvelle sur
`evaluation`, table centrale.

## Contre-argument conservé

Une seule migration écrite maintenant éviterait quatre mises à niveau et donnerait une vue
d'ensemble. Elle n'a pas gagné parce que les lots peuvent encore changer après les réponses du
porteur, et qu'une migration livrée ne se corrige plus.

## Conditions de reconsidération

- Une reconstruction échoue sur une base réelle (essai de l'école d'essai ou d'une école pilote) :
  remplacer l'élargissement de la nature par une table à part.
- L'activité des élèves dépasse ce que la base supporte en taille ou en temps (mesure du lot 4) :
  ranger l'avancement dans une base à part, hors de la sauvegarde.
