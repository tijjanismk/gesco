# Retirer un contenu : le cacher, à cinq grains, par sa clé stable, et le remettre quand on veut

**Date** : 15 novembre 2026
**Statut** : accepté le 15 novembre 2026 (étape 6 du lot : retraits à cinq grains, liste et remise) ; proposé le 15 novembre 2026 — décision 7 de `docs/plan-phase-12.md` §5, lot 1 de la phase 12 (`SPEC_PHASE_12_LOT_1.md`)

## Contexte

Le porteur a décidé que l'école retire de la bibliothèque ce qu'elle ne veut pas (10 novembre
2026). L'exigence E-9 précise : retirer pour de bon, une mise à jour ne fait jamais revenir ce
que l'école a retiré et ne touche jamais ce qu'elle a ajouté.

- Le paquet livré est un fichier signé, jamais modifié (ADR « stockage des contenus ») :
  effacer un contenu du disque casserait la signature.
- La bibliothèque pèse quelques mégaoctets : effacer ne rend presque aucune place.
- Rien ne se supprime dans GESCO (R-14) ; un module éteint garde ses données.
- Une école retire souvent par blocs : une matière qu'elle n'enseigne pas comme la
  bibliothèque, une série qu'elle n'a pas, un chapitre qu'elle traite autrement.
- La décision « flexibilité sans incohérence » (14 novembre 2026) : ne jamais bloquer, tout
  doit pouvoir revenir, et ce que l'écran montre doit rester juste.

## Options

| | 1. Cacher, à cinq grains, par clé stable | 2. Effacer du disque | 3. Cacher contenu par contenu seulement |
|---|---|---|---|
| Apprentissage | Faible | Faible | Très faible |
| Exploitation | Une table de retraits ; une mise à jour les respecte | Le paquet doit être réécrit, donc re-signé ou non vérifiable | Retirer une matière = des dizaines de gestes |
| Maturité | Même règle que les matières et les rubriques désactivées | Contraire à R-14 | — |
| Pour un agent de code | Bon : un filtre unique sur le catalogue | Mauvais : signature, reprise, sauvegarde | Bon |
| Plus gros risque | Un nouveau contenu livré dans un chapitre retiré qui apparaîtrait | Une erreur sans retour | Une école qui renonce à trier |

## Décision

Option 1. Retirer, c'est cacher ; rien n'est effacé ni du paquet ni de la base.

- **Cinq grains** : un contenu, un chapitre, une matière d'un niveau (avec sa série au lycée),
  un niveau entier, un paquet entier. Chaque retrait est une ligne de `retrait_bibliotheque` :
  grain, cible désignée par sa **clé stable** (clé du contenu ou du chapitre, codes du niveau,
  de la série et de la matière, code du paquet), qui, quand ; la remise ajoute qui et quand à
  la même ligne. Retirer de nouveau crée une ligne nouvelle.
- **Ce que retirer cache** : le contenu disparaît du catalogue, de la recherche, de
  l'impression par chapitre, et plus tard du portail et des quiz. Il reste dans la liste
  « Retirés », où l'on voit qui l'a retiré et quand, et d'où on le remet d'un geste.
- **Retirer un bloc couvre ce qui viendra** : un contenu qu'une mise à jour ajoute dans un
  chapitre, une matière ou un niveau retiré reste caché, puisque le retrait vise la clé du bloc.
- **Les ajouts de l'école se retirent aussi**, par les mêmes gestes ; ce qu'elle a écrit reste
  dans la base et dans la sauvegarde.
- **Une version de l'école suit son contenu** : retirer une leçon livrée cache aussi la version
  de l'école ; la remettre fait revenir la version qui était visible.
- **Qui** : la direction (droit `library:manage`, Directeur et Administrateur à la livraison).
  Un maître ne retire rien ; il peut demander à la direction, hors de l'application.
- **Les niveaux que l'école n'a pas** ne sont pas des retraits : le catalogue ne montre que les
  niveaux actifs de l'école. Ouvrir un niveau plus tard fait apparaître sa bibliothèque.

Aucune dépendance nouvelle.

## Justification

1. Cacher se défait toujours : la règle « flexibilité sans incohérence » demande qu'aucun geste
   ne soit sans retour.
2. La signature du paquet reste vérifiable, et le paquet reste le même dans toutes les écoles :
   une mise à jour remplace un fichier, elle ne recolle pas des morceaux.
3. Viser des clés stables fait tenir E-9 par construction : une mise à jour ne connaît pas les
   retraits, elle ne peut donc pas les défaire.
4. Les blocs évitent à une école de trier quatre cents leçons une à une.
5. Effacer du disque ne rendrait presque rien : la bibliothèque pèse quelques mégaoctets.

## Conséquences

Positives : un retrait se remet d'un geste ; les mises à jour n'ont aucun cas particulier ; la
liste « Retirés » dit qui a décidé quoi.

Négatives : la place des contenus retirés n'est jamais rendue ; un contenu retiré à deux grains
(lui-même et son chapitre) ne revient qu'une fois les deux retraits remis, ce que la liste
« Retirés » doit montrer clairement ; un contenu livré que la direction retire alors qu'un maître
en avait une version publiée disparaît aussi pour les élèves.

## Contre-argument conservé

Une école au disque presque plein voudrait que retirer libère de la place. Il n'a pas gagné
parce que la bibliothèque livrée tient en quelques mégaoctets depuis qu'elle est rédigée pour
GESCO : ce sont les vidéos de l'école qui prennent de la place, et elles se gèrent à part.

## Conditions de reconsidération

- Si un paquet dépasse 500 Mo (manuels numérisés, images lourdes).
- Si des écoles demandent qu'un maître puisse retirer dans ses propres matières.
- Si la liste « Retirés » devient illisible parce que les écoles retirent surtout contenu par
  contenu.

## Mise en œuvre

- 15 novembre 2026, étape 6 : la table `retrait_bibliotheque` (migration 0061) garde chaque retrait ; le noyau (`gesco-core/src/bibliotheque/lecture.rs`, `GrainRetrait`, `CibleRetrait`) cache au rangement du catalogue ce que visent les retraits en cours, et la recherche ne montre que ce que le catalogue montre. La cible d'une matière est sa clé au catalogue : `m:<identifiant>` pour la matière de l'école qui reçoit la discipline par la correspondance, `d:<code>` pour une discipline de référence sans correspondance ; à valider par le porteur du projet (si la direction change la correspondance après coup, le retrait suit la matière qu'elle voyait). Retirer tout le paquet cache les contenus livrés ; les contenus de l'école rangés dans un chapitre livré restent visibles (à valider). Un retrait identique déjà en cours n'est pas doublé. Commandes `bibliotheque_retrait_apercu`, `bibliotheque_retirer`, `bibliotheque_remettre`, `bibliotheque_retraits_lister` (`library:manage`), auditées, événements `BibliothequeRetiree` et `BibliothequeRemise`.
