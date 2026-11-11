# Deux personnes sur la même fiche : refus si elle a changé depuis sa lecture

**Date** : 10 novembre 2026
**Statut** : proposé le 10 novembre 2026, accepté le 10 novembre 2026 par le porteur du projet — décision 4 de `docs/plan-phase-11.md`, étape A4

## Contexte

Avec un seul clavier, deux personnes ne modifient jamais la même fiche au même moment. Avec dix
postes (T-81), la secrétaire peut corriger un numéro de téléphone pendant que le directeur des
études corrige la date de naissance du même élève.

- Vérifié dans `gesco-db` (`depots/personnes.rs`, modification d'une personne et d'un élève) : la
  mise à jour ne regarde que l'identifiant et l'établissement. Celui qui enregistre en second
  écrase, sans le savoir, ce que le premier vient de corriger. Aucune recherche dans les dépôts
  et les services ne montre de contrôle « changé entre-temps ».
- Chaque table porte déjà `mis_a_jour_le` (à la milliseconde) et `mis_a_jour_par`.
- Tout ne se ressemble pas :
  - les **fiches** (élève, responsable, personnel, classe, grille de tarifs, réglages) se
    modifient en bloc : c'est là que l'écrasement se produit ;
  - les **grilles de saisie** (notes, absences, appréciations) s'enregistrent case par case ;
  - les **écritures d'argent** ne se modifient jamais, elles s'ajoutent et s'annulent ; la
    numérotation est déjà tenue dans une transaction ;
  - les **changements d'état** (valider une inscription, publier un bulletin, clôturer une
    caisse) vérifient déjà l'état courant dans leur transaction : le second reçoit un refus.
- Les écritures se suivent une à une (un seul verrou d'écriture) : pas de corruption possible,
  seulement une perte silencieuse.

## Options

| | 1. La dernière écriture gagne | 2. Refus si la fiche a changé depuis sa lecture | 3. Verrou : « en cours de modification par… » |
|---|---|---|---|
| Apprentissage | Nul : c'est l'état actuel | Faible : une condition de plus à l'enregistrement | Moyen : table de verrous, durée, libération |
| Exploitation | Rien | Rien | Verrous oubliés (poste éteint, pause de midi) à libérer |
| Maturité | — | Pratique courante | Pratique courante, plus fragile |
| Pour un agent de code | — | Bon : mécanique, un essai par fiche | Moyen : beaucoup d'états à essayer |
| Plus gros risque | Une correction perdue sans que personne le sache | Une saisie à refaire après un refus | Une fiche bloquée par quelqu'un qui n'est plus devant son poste |

## Décision

Une fiche modifiée par quelqu'un d'autre depuis qu'on l'a lue ne s'enregistre pas : l'écran dit
qui l'a modifiée et quand, garde ce qui a été saisi, et propose de relire.

Aucune dépendance nouvelle, donc aucune version à retenir.

Ce que la décision fixe :

- **Fiches** : la lecture rend l'instant de dernière modification ; la commande de modification
  le renvoie ; l'enregistrement ne se fait que s'il n'a pas bougé, sinon une erreur
  « modifiée entre-temps » nomme la personne et l'heure. La saisie n'est pas perdue à l'écran.
- **Grilles de saisie** : la dernière écriture d'une case gagne, comme aujourd'hui ; la grille se
  relit quand on y revient.
- **Écritures d'argent et changements d'état** : rien à changer.
- **Relecture des écrans** : pas de message poussé par le poste principal dans cette version ;
  les listes et les fiches se relisent à l'ouverture, au retour sur l'écran et après un refus.
- Les opérations qui demandent d'être seul (clôture de l'année, restauration, import, mise à
  niveau) relèvent d'un autre mécanisme, à écrire dans la spécification du lot A.

## Justification

Ce que l'école ne pardonne pas, c'est une correction qui disparaît sans trace : l'option 1 le
permet. Le refus coûte peu : la colonne existe déjà partout, la condition est mécanique. Il ne
crée aucun état durable, donc rien à libérer quand un poste s'éteint, ce qui compte dans une
école où le courant se coupe. Le verrou protège mieux la saisie en cours, mais il bloque les
collègues à la place de les prévenir, et il s'oublie.

## Conséquences

Positives : plus de perte silencieuse ; l'écran explique ; aucune table nouvelle ; le
comportement à un seul poste ne change pas.

Négatives : chaque commande « modifier » et son type changent (contrat, écrans, essais) : un
travail mécanique mais large, à compter dans l'étape A4 ; celui qui enregistre en second doit
reporter sa correction sur la fiche relue ; deux modifications dans la même milliseconde ne se
distingueraient pas (les écritures se suivent une à une, le cas est théorique).

## Contre-argument conservé

Dans une école de trois postes où chacun a son domaine, le cas n'arrivera presque jamais : tout
ce travail pour un message que personne ne verra. Il n'a pas gagné parce que le jour où il
arrive, rien ne le signale, et qu'une erreur sur un numéro de téléphone ou une date de naissance
se retrouve sur des documents officiels.

## Conditions de reconsidération

- Les écoles se plaignent de saisies à refaire : ajouter un simple avis « un collègue consulte
  cette fiche », sans verrou.
- Les grilles de notes à plusieurs (étape A5, maîtres) montrent des cases écrasées : y appliquer
  le même refus, case par case.
- La synchronisation en ligne (lot C) demande un numéro de version par ligne : l'adopter alors
  pour ce contrôle aussi.
