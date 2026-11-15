# Le repère commun : un contenu livré vise des codes nationaux, et une correspondance de l'école le place sous ses matières

**Date** : 15 novembre 2026
**Statut** : proposé le 15 novembre 2026 — décision 9 de `docs/plan-phase-12.md` §5, lot 1 de la phase 12 (`SPEC_PHASE_12_LOT_1.md`)

## Contexte

Un même paquet entre dans toutes les écoles ; chaque école a son propre référentiel, avec ses
identifiants. Un contenu livré ne peut donc pas viser une matière ou un niveau de l'école par son
identifiant.

Vérifié dans le code le 15 novembre 2026 :
- Les niveaux livrés ont des codes stables : `7`, `8`, `9` pour le 2e cycle, `10`, `11`, `12`
  pour le lycée (la terminale est `12`), avec leurs séries (`S`, `SES`, `LL` en 11e ; `TSE`,
  `TSExp`, `TSEco`, `TSS`, `TLL` en terminale).
- Les matières livrées ont des codes : `MATH`, `PC`, `SVT`, `FR`, `ANG`, `HG`, `ECM`, `PHILO`,
  `ECO`, `EPS`, `DESS`, `ECOF`… La biologie du 2e cycle est la matière `SVT`.
- L'école peut changer le code d'une matière (seule règle : deux matières n'ont pas le même
  code), la renommer, en désactiver, en créer.
- La bibliothèque livrée couvre aujourd'hui la 7e et la 8e (mathématiques, physique-chimie,
  biologie), la 9e (sept matières), la terminale (mathématiques de TSE, physique-chimie de TSE et
  TSExp) et les sujets du DEF et du BAC. Aucun contenu ivoirien n'est livré : la correspondance
  des séries ivoriennes n'a plus d'objet.
- Le 1er cycle, dont les matières ne correspondent pas à la bibliothèque, n'a aucun contenu
  livré aujourd'hui.

## Options

| | 1. Codes nationaux, et une correspondance de l'école réglable | 2. Codes nationaux seuls, rangement par code exact | 3. Identifiants de l'école, posés à l'installation du paquet |
|---|---|---|---|
| Apprentissage | Faible | Très faible | Moyen |
| Exploitation | Juste après un renommage ou un changement de code | Un code changé fait disparaître des contenus du catalogue | Chaque paquet doit être réconcilié école par école |
| Maturité | Même démarche que les matières et compétences livrées, reconnues par leur code | — | — |
| Pour un agent de code | Bon | Très bon | Mauvais |
| Plus gros risque | Une correspondance fausse posée par l'école | Des leçons invisibles sans que personne sache pourquoi | Un paquet mis à jour qui ne retrouve plus ses matières |

## Décision

Option 1.

- **Ce que vise un contenu livré** : le code national du niveau, la liste des codes de séries
  (vide = toutes les séries du niveau) et un **code de discipline de référence**, qui est le code
  de la matière livrée (`MATH`, `PC`, `SVT`…). Le paquet porte aussi le libellé de référence de
  chaque discipline (« Mathématiques », « Biologie »…).
- **Niveaux** : un contenu paraît si l'école a un niveau actif de ce code. Un niveau dont
  l'école a changé le libellé reste reconnu par son code ; un niveau sans équivalent dans
  l'école ne montre rien, ce n'est pas un retrait.
- **Matières, par une correspondance** : une table `correspondance_discipline` (code de
  référence, niveau, matière de l'école) dit sous quelle matière de l'école se range chaque
  discipline de référence. Elle est **posée seule** à l'installation du paquet, par code exact,
  et la direction peut la changer dans les réglages de la bibliothèque (« La biologie de la
  bibliothèque va sous : … »). Une mise à jour du paquet ne la réécrit jamais.
- **Sans correspondance** (code changé, matière désactivée, matière absente) : les contenus
  restent dans le catalogue, sous le libellé de référence de la bibliothèque. Rien ne disparaît
  et rien n'est signalé comme une faute ; la direction peut poser la correspondance quand elle
  le veut.
- **Ce que l'école ajoute** vise directement ses propres niveaux et matières, par leurs
  identifiants : c'est son référentiel.
- **Le droit d'écrire d'un maître** suit cette correspondance : un maître affecté à la matière
  de l'école qui reçoit `SVT` en 9e peut écrire les versions de la biologie de 9e livrée.

Aucune dépendance nouvelle. Une table de plus que l'ADR « tables de la bibliothèque » :
`correspondance_discipline`.

## Justification

1. Les codes nationaux sont déjà le repère des matières, des niveaux et des compétences livrés :
   rien à inventer.
2. Une correspondance réglable tient la flexibilité : l'école qui a fusionné ou renommé ses
   matières range la bibliothèque où elle veut.
3. Sans correspondance, rien ne disparaît : la règle « flexibilité sans incohérence » demande
   qu'aucun contenu ne devienne invisible par un réglage fait ailleurs.
4. Une mise à jour qui ne touche jamais la correspondance tient E-9 : ce que l'école a réglé
   reste réglé.
5. Le droit d'écrire du maître se déduit de ses affectations, comme pour les notes depuis la
   phase 4.

## Conséquences

Positives : un même paquet pour toutes les écoles ; une école qui change le code d'une matière ne
perd rien ; le 1er cycle pourra recevoir des contenus livrés plus tard en réglant sa
correspondance (cinq domaines du curriculum ou matières de l'emploi du temps).

Négatives : une table et un écran de réglage de plus ; une école peut ranger la biologie sous les
mathématiques par erreur (elle le voit au catalogue et le corrige) ; un catalogue peut montrer à
la fois des libellés de l'école et des libellés de référence.

## Contre-argument conservé

Le rangement par code exact, sans correspondance, est plus simple et suffit à toute école qui
garde les codes livrés, c'est-à-dire presque toutes. Il n'a pas gagné parce qu'un seul code
changé ferait disparaître des dizaines de leçons sans explication, ce que la règle du porteur
interdit.

## Conditions de reconsidération

- Si des contenus du 1er cycle sont livrés : la correspondance devra savoir répartir une
  discipline de référence sur plusieurs matières de l'école (« Français » vers lecture,
  orthographe, rédaction).
- Si des contenus d'un autre pays sont livrés : il faudra une correspondance des séries.
- Si aucune école ne règle jamais la correspondance : la simplifier en code exact.
