# Report d'une note de devoir au bulletin : une évaluation ordinaire, créée par un geste, marquée d'où elle vient

**Date** : 16 novembre 2026
**Statut** : proposé le 16 novembre 2026
**Précise** : décision 12 de `docs/plan-phase-12.md` §5 ; blueprint §16.2 (« une note
d'e-learning n'entre jamais seule dans le bulletin ») ; exigence E-13 ; `SPEC_PHASE_4.md`
(évaluations, notes, validation) ; `SPEC_PHASE_11_A.md` §7 (saisie des notes par les maîtres)
**Spécification** : `SPEC_PHASE_12_LOT_5.md`

## Contexte

- Le lot 5 permet au maître de donner un devoir, à l'élève de le rendre par le portail, au maître
  de le noter. Le maître peut ensuite faire entrer ces notes dans le bulletin, jamais d'elles-mêmes.
- Le bulletin se calcule depuis `evaluation` (sorte « devoir » ou « composition », période, classe,
  matière, barème, pondération, état « en saisie » puis « validée ») et `note` (valeur, absent,
  justifié, dispensé). Les règles de calcul, de validation et de déverrouillage existent et sont
  essayées depuis la phase 4.
- La saisie des notes par les maîtres est éteinte à la livraison (`notes.saisie_maitres`) :
  dans ce cas un compte de maître n'ouvre pas de session et c'est le secrétariat ou la direction
  qui saisit.
- Une note de devoir rendu peut être changée après coup (copie revue, réclamation).

## Options

| | 1. Le report crée une évaluation « devoir » ordinaire, en saisie, marquée « devoir en ligne » | 2. Le report écrit dans une évaluation existante choisie par le maître | 3. Une colonne à part dans le bulletin pour les devoirs en ligne |
|---|---|---|---|
| Apprentissage | Faible : règles existantes | Moyen : notes déjà saisies à ne pas écraser | Élevé : calcul du bulletin à changer |
| Exploitation | Rien de plus | Conflits entre deux sources de notes | Modèles de bulletin à refaire |
| Maturité | Circuit de la phase 4 | Idem, avec des cas nouveaux | À écrire |
| Pour un agent de code | Bon | Moyen | Mauvais |
| Plus gros risque | Une évaluation reportée deux fois | Une note saisie à la main écrasée par un report | Un bulletin qui ne ressemble plus aux bulletins officiels |

## Décision

« Reporter au bulletin » crée, pour la classe et la matière du devoir, une évaluation de sorte
« devoir » en saisie dans la période choisie, avec les notes du devoir ; elle porte une origine
« devoir en ligne » et le lien vers son devoir ; ensuite, tout suit les règles existantes.

Aucune brique nouvelle. Une colonne `origine` et une colonne `devoir_id` sur `evaluation`
(migration du lot 5, ADR « tables des lots 2 à 5 »).

Ce que la décision fixe :

- **Un geste, jamais automatique** : le bouton est sur l'écran du devoir corrigé ; une
  confirmation dit la période, le barème, la pondération (1 par défaut, modifiable) et le nombre
  de notes qui entreront. Sans ce geste, aucune note de devoir n'entre au bulletin (essai).
- **Qui** : celui qui a le droit de créer une évaluation de cette classe et de cette matière
  (`grades:create` et sa portée). Quand la saisie des maîtres est éteinte, la direction ou le
  secrétariat reporte depuis l'écran du devoir.
- **Ce qui passe** : chaque note du devoir sur le même barème ; un élève qui n'a pas rendu reste
  sans note (ni zéro, ni absent) : le maître le complète dans l'évaluation comme d'habitude.
- **Un devoir, une évaluation** : reporter de nouveau met à jour l'évaluation tant qu'elle est en
  saisie ; validée, elle ne bouge plus et l'écran le dit (« Cette évaluation est validée : la
  modifier demande de la déverrouiller »), avec le circuit de déverrouillage existant.
- **Une note changée dans le devoir** après le report ne change pas le bulletin d'elle-même ;
  l'écran du devoir montre « Notes changées depuis le report » et le bouton « Reporter de
  nouveau ».
- **Dans le bulletin**, l'évaluation est un devoir comme un autre : aucun texte du bulletin ne
  dit « en ligne ». Dans l'écran des évaluations, une marque discrète « Devoir rendu par les
  élèves » et un lien vers le devoir.
- **Journal** : report et nouveau report vont au journal d'audit avec le nombre de notes.

## Justification

Le bulletin est le document le plus sensible de l'école : n'y faire entrer qu'une évaluation
ordinaire garde ses règles de calcul, de validation et de déverrouillage, essayées depuis la
phase 4. Écrire dans une évaluation existante mêlerait deux sources de notes ; une colonne à part
changerait les bulletins officiels. Le geste du maître et la confirmation tiennent E-13.

## Conséquences

Positives : rien ne change au calcul ni aux modèles de bulletin ; l'origine se voit dans l'écran
des évaluations ; la direction garde la validation.

Négatives : deux colonnes nouvelles sur une table centrale (migration à essayer sur les bases
1.2 et 2.0) ; un maître peut reporter un devoir dans la mauvaise période (corrigeable tant que
l'évaluation est en saisie).

## Contre-argument conservé

Reporter dans une évaluation existante éviterait de multiplier les évaluations quand un maître
fait un devoir en ligne et un devoir sur table pour la même note. Il n'a pas gagné parce qu'il
faudrait décider, élève par élève, quelle note l'emporte, et qu'une erreur écraserait une note
déjà saisie.

## Conditions de reconsidération

- Les écoles veulent qu'un même devoir mêle des copies rendues en ligne et sur papier dans une
  seule note : le maître note déjà le papier dans l'écran du devoir (lot 5) ; si cela ne suffit
  pas, revoir.
- Le ministère impose une forme de bulletin qui distingue les devoirs en ligne.
