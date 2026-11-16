# Format des quiz : une convention écrite dans le texte de la leçon, reconnue par le noyau

**Date** : 16 novembre 2026
**Statut** : proposé le 16 novembre 2026, accepté le 16 novembre 2026 par le porteur du projet
**Précise** : ADR « format des leçons » (15 novembre 2026) ; blueprint §16.1 (quiz, question,
réponse) ; étape 3.1 de `docs/plan-phase-12.md` §3.5
**Spécification** : `SPEC_PHASE_12_LOT_3.md`

## Contexte

- Les quiz sont écrits à neuf pour GESCO, jamais tirés des anciennes bases (décision du porteur ;
  les 15 129 questions au gabarit ne sont pas livrées).
- **Ce qui existe déjà** (relevé du 16 novembre 2026 dans `gesco-bibliotheque\contenus`) : les
  398 leçons livrées ont une section « 7. Quiz » ; dans 392, elle suit exactement la même forme —
  « **Question n.** » puis l'énoncé, une ligne « A) … B) … C) … D) … », puis une sous-section
  « Réponses » où chaque ligne donne « n. **B.** » et une explication qui nomme l'erreur d'élève
  que cache chaque mauvaise réponse. 1 990 questions en tout. Les six autres leçons (physique de
  8e et de 9e) ont trois choix ou une ligne de choix coupée.
- Plus aucun cours n'est écrit ni repris (décision du porteur, 15 novembre 2026) : le lot 3 doit
  rendre ces quiz interactifs sans réécrire les leçons.
- Les leçons ont déjà tout un circuit : version livrée intacte, versions de l'école, brouillon,
  soumission, publication, historique, comparaison, impression « sans les corrigés » qui retire
  déjà la sous-section « Réponses ».
- Un maître peu à l'aise au clavier doit pouvoir écrire un quiz ; certains ont leurs questions
  dans Excel.

## Options

| | 1. Convention dans le texte de la leçon | 2. Tables quiz, question, choix (blueprint §16.1) | 3. Bloc de données (JSON) dans le texte |
|---|---|---|---|
| Apprentissage | Faible : la forme que les leçons ont déjà | Moyen : écrans d'édition propres | Faible pour la machine, mauvais pour un maître |
| Exploitation | Rien de plus | Versions et publication à refaire pour les quiz | Rien de plus |
| Maturité | Analyseur du lot 1 (`pulldown-cmark`) | Classique | Classique |
| Pour un agent de code | Bon : essais sur les 1 990 questions | Moyen : deux circuits à tenir d'accord | Bon |
| Plus gros risque | Une question mal écrite qui n'est pas reconnue | Un quiz qui ne suit plus sa leçon après une version de l'école | Un maître qui casse le bloc d'une virgule |

## Décision

Un quiz est écrit dans le texte, selon la convention que les leçons livrées suivent déjà ; le
noyau la reconnaît en un bloc « quiz » de l'arbre de la leçon ; ce qu'il ne reconnaît pas reste
affiché comme du texte, sans jamais bloquer.

Aucune brique nouvelle : `pulldown-cmark` (déjà dans le dépôt depuis le lot 1) ; l'import depuis
Excel utilise `calamine` 0.36.1, déjà dans le dépôt (0.38.0 est sortie le 2 novembre 2026, trop
récente pour monter maintenant). Vérifié le 16 novembre 2026 sur crates.io.

Ce que la décision fixe :

- **Où** : dans la section d'une leçon dont le titre est « Quiz » (numéro et accents ignorés), et
  dans un contenu nouveau de nature « quiz », rangé sous un chapitre, dont tout le texte suit la
  convention.
- **La convention** :
  - une question commence par « **Question n.** » ; l'énoncé suit, avec formules et images ;
  - ses choix sont des lignes ou morceaux de ligne « A) … B) … » (deux à six choix) ;
  - la sous-section « Réponses » donne, par question, « n. **B.** » puis l'explication ;
  - **plusieurs bonnes réponses** : « n. **A, C.** » ;
  - **vrai ou faux** : deux choix « A) Vrai B) Faux » ; rien de plus ;
  - **réponse courte** : pas de choix ; la réponse est « n. **« 3,5 » ou « 7/2 ».** » ; la
    comparaison ignore majuscules, accents et espaces, et lit « 3,5 » et « 3.5 » comme le même
    nombre ;
  - **appariement** : choix « A) … B) … » et « 1) … 2) … » ; la réponse est « n. **A-2, B-1.** ».
- **Ce que donne l'analyse** : une liste de questions, chacune avec sa sorte, ses choix, ses
  bonnes réponses et son explication ; une question incomplète (pas de réponse, réponse qui
  n'existe pas) n'entre pas dans le quiz et reste du texte ; sous l'aperçu de l'éditeur, l'auteur
  seul lit « Cette question ne sera pas proposée en quiz : la réponse manque. ».
- **Versions** : un quiz suit sa leçon. Une version de l'école de la leçon porte sa version du
  quiz ; aucune version propre au quiz.
- **Tentatives** : une tentative garde l'empreinte du texte de la version qu'elle a passée ; une
  nouvelle version ne rend pas faux les résultats passés.
- **Écrire un quiz** : un bouton « Question » de l'éditeur écrit le gabarit d'une question ; un
  import Excel (une ligne par question : énoncé, choix A à F, bonne réponse, explication) écrit le
  même texte dans un brouillon, avec l'analyse ligne par ligne des imports existants.
- **Livré** : l'outil du paquet n'a rien à changer ; il compte dans le tableau de couverture les
  questions reconnues et signale, pour l'intégration des cours, celles qui ne le sont pas.

## Justification

Les 1 990 questions écrites suivent déjà cette forme : la reconnaître coûte un analyseur, alors
que des tables demanderaient une conversion et un second circuit de versions. Un quiz qui vit dans
le texte de sa leçon ne peut pas se désaccorder d'elle, s'imprime comme elle, se compare et se
publie comme elle. Une convention lisible par un maître vaut mieux qu'un bloc de données exact
mais fragile. Ce qui n'est pas reconnu reste lisible, ce qui tient « ne jamais bloquer ».

## Conséquences

Positives : rien à convertir dans la bibliothèque ; aucun écran d'édition nouveau en dehors de
deux boutons ; l'historique et la comparaison couvrent les quiz.

Négatives : l'analyse devient plus exigeante (essais sur toutes les leçons) ; une faute de frappe
retire une question du quiz sans que l'élève le sache ; les six leçons hors convention restent du
texte jusqu'à l'intégration des cours ; une table de questions ferait des statistiques par
question plus simplement (ici, par numéro et empreinte de version).

## Contre-argument conservé

Le blueprint prévoyait des tables, qui permettent de réutiliser une question dans plusieurs
quiz, de tirer des questions au hasard dans une banque et de suivre une question d'une version à
l'autre. Il n'a pas gagné parce que les quiz existants sont attachés chacun à une leçon et que le
tirage « Réviser le chapitre » se fait aussi bien depuis les quiz des leçons.

## Conditions de reconsidération

- Les écoles demandent une banque de questions partagée entre leçons, ou des examens blancs tirés
  au hasard : passer à des tables, alimentées par cette même analyse.
- Plus d'une question sur vingt écrites par les écoles n'est pas reconnue (mesure dans l'école
  pilote) : un éditeur de quiz par formulaire devient nécessaire.
