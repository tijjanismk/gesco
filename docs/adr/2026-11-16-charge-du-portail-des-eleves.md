# Charge du portail des élèves : la gestion d'abord, écritures groupées, fichiers par morceaux

**Date** : 16 novembre 2026
**Statut** : proposé le 16 novembre 2026
**Précise** : décision 14 de `docs/plan-phase-12.md` §5 ; ADR « portail séparé de la gestion »
**Spécification** : `SPEC_PHASE_12_LOT_4.md`

## Contexte

- Une classe compte souvent plus de soixante élèves ; le nombre d'élèves en même temps n'a pas
  été dit par le porteur (T-87) : « il se mesurera ».
- Le poste principal est souvent un vieil ordinateur ; il sert en même temps la caisse, la saisie
  des notes et jusqu'à dix postes reliés (mesure du lot A).
- La base SQLite n'a qu'un écrivain à la fois : soixante petites écritures d'avancement à la même
  seconde passent avant ou après un encaissement.
- Un routeur d'école ordinaire tient mal plus d'une trentaine d'appareils actifs.
- Une leçon livrée pèse quelques dizaines de kilo-octets avec ses figures ; une vidéo de l'école
  peut peser 200 Mo (T-121) ; un PDF se fabrique par Typst en une à plusieurs secondes.
- Les fichiers de la bibliothèque sont adressés par leur empreinte : ils ne changent jamais.

## Options

| | 1. Priorité à la gestion, écritures groupées, bornes mesurées | 2. Rien de particulier : on mesure après | 3. Le portail sur un autre ordinateur de l'école |
|---|---|---|---|
| Apprentissage | Moyen | Nul | Élevé : copie et synchronisation |
| Exploitation | Rien | Plaintes de la caisse un jour de forte affluence | Un ordinateur de plus à tenir allumé |
| Maturité | Mécanismes de tokio et d'axum | — | À écrire |
| Pour un agent de code | Bon : essais de charge simulés | Bon | Moyen |
| Plus gros risque | Des bornes mal fixées refusent des élèves trop tôt | Un encaissement lent devant un parent | Une copie en retard sur ce que l'école a publié |

## Décision

Le portail passe toujours après la gestion : ses lectures sont servies sans toucher à la base
quand c'est possible, ses écritures sont groupées, ses requêtes simultanées sont bornées, et
au-delà il répond poliment « réessaie dans un instant ».

Aucune brique nouvelle (tokio et axum déjà dans le dépôt ; vérifiés le 16 novembre 2026 sur
crates.io). Les nombres ci-dessous sont des valeurs de départ, à régler à l'étape de mesure.

Ce que la décision fixe :

- **Lectures sans base** : le catalogue d'un niveau et l'arbre de blocs d'une leçon visible sont
  gardés en mémoire, recalculés à chaque publication, retrait ou installation de paquet. Les
  fichiers (figures, images, sons) se servent par leur empreinte avec un en-tête « ne change
  jamais » : un téléphone ne les demande qu'une fois.
- **Écritures groupées** : l'avancement et les tentatives de quiz entrent dans une file en
  mémoire, écrite en une seule transaction toutes les cinq secondes au plus ; une file pleine
  (dix mille lignes) refuse les nouvelles écritures d'avancement sans erreur visible pour l'élève.
  Un rendu de devoir (lot 5) s'écrit tout de suite : c'est un geste rare et attendu.
- **Requêtes simultanées** : 48 au plus pour les pages et l'API, 4 pour les vidéos, 2 pour les
  PDF ; au-delà, l'élève voit « Beaucoup d'élèves sont connectés. Réessaie dans un instant. » et
  la page se relance seule après quelques secondes.
- **Fichiers par morceaux** : vidéos et sons se servent avec les requêtes partielles (`Range`)
  que les navigateurs emploient pour avancer dans une vidéo ; aucun fichier n'est lu en entier en
  mémoire.
- **PDF** : fabriqué une fois par version visible et par forme, gardé dans un dossier de travail
  du poste principal (hors sauvegarde) ; refait seulement si la version change.
- **Pages compressées** à la construction (gzip) ; le frontend des élèves pèse 300 Ko compressés
  au plus au premier chargement, polices comprises.
- **Promesse écrite dans le guide** : une classe entière (soixante élèves) qui lit en même temps,
  pendant que deux postes travaillent, si le routeur le permet ; le chiffre se corrige après la
  mesure de l'étape 4.6.

## Justification

L'encaissement devant un parent compte plus qu'une leçon ouverte une seconde plus tard. Ce que
lit un élève change rarement : le garder en mémoire et le faire garder par le téléphone retire
presque toute la charge. Grouper les écritures transforme soixante transactions en une. Un
second ordinateur dédié réglerait la charge mais ajouterait une copie à tenir à jour et un
appareil de plus qu'une école n'a pas.

## Conséquences

Positives : la gestion ne ralentit pas (mesuré) ; peu de mémoire ; les vieux téléphones ne
retéléchargent rien.

Négatives : un avancement peut se perdre si le poste principal s'arrête dans les cinq secondes
(perte acceptée : ce n'est pas une note) ; des bornes à régler sur le terrain ; un cache en
mémoire à invalider exactement, sinon un élève voit une version remplacée.

## Contre-argument conservé

Écrire chaque avancement tout de suite serait plus simple et ne perdrait rien. Il n'a pas gagné
parce que soixante écritures à la même seconde se mettent en file devant celles de la caisse, sur
la seule base de l'école.

## Conditions de reconsidération

- La mesure de l'étape 4.6 montre qu'un encaissement dépasse une seconde pendant qu'une classe
  lit : baisser les bornes, ou servir le portail depuis un autre ordinateur.
- Les écoles demandent plus que deux classes à la fois : revoir les bornes et le routeur conseillé.
- Le portail en ligne prend le relais des lectures : la charge locale baisse d'elle-même.
