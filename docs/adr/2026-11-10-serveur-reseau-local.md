# Serveur du réseau local : axum, dans une crate `gesco-reseau`

**Date** : 10 novembre 2026
**Statut** : proposé le 10 novembre 2026, accepté le 10 novembre 2026 par le porteur du projet — décision 1 de `docs/plan-phase-11.md`, étape A1
**Précise** : ADR-014 du blueprint (multi-poste par serveur local) et l'ADR du 4 octobre 2026 (frontière à commande unique)

## Contexte

ADR-014 a décidé le principe : un poste principal détient la base, les autres s'y relient, un seul
binaire. Il reste à dire avec quoi le poste principal répond, et où ce code vit.

- La frontière existe : `Application::executer(nom, jeton, entree)` dans `gesco-services`, environ
  317 commandes, toutes gardées là. Une route réseau n'a qu'à l'appeler.
- axum 0.8.9 était dans l'espace de travail jusqu'au 31 octobre 2026, retiré avec les deux crates
  vides (D-4) parce qu'il était compilé pour rien.
- Le disque du porteur se remplit aux compilations : chaque exécutable de test de
  `gesco-services` pèse environ 140 Mo, et compiler `gesco-app` (Tauri) a pris jusqu'à 58 minutes.
  Ce qui s'essaie souvent ne doit dépendre ni de Tauri ni d'un cadre web de plus.
- `hyper`, `tower` et `http` sont déjà dans `Cargo.lock`, apportés par Tauri : axum n'ajoute que
  cinq paquets (ceux retirés le 31 octobre).
- `Application` porte un seul nom de poste, celui de l'ordinateur : tout appel reçu du réseau
  serait signé du poste principal dans le journal d'audit et dans `outbox`.

## Options

| | 1. axum dans une crate `gesco-reseau` | 2. axum dans `gesco-services` | 3. `hyper` seul, dans `gesco-app` |
|---|---|---|---|
| Apprentissage | Faible : axum est le choix du blueprint (§5.2) | Faible | Moyen : routage, limites et erreurs écrits à la main |
| Exploitation | Une crate de plus, un exécutable de test de plus | Rien de plus à ranger | Rien de plus à ranger |
| Maturité | axum 0.8, très répandu | Idem | hyper 1.x, très répandu |
| Pour un agent de code | Bon : petite crate, essais sans Tauri | Moyen : alourdit la crate que tout le monde compile | Mauvais : essais liés à la compilation de Tauri |
| Plus gros risque | Une frontière de crate de plus à tenir | axum recompilé à chaque essai de services | Bogues de protocole écrits à la main ; rien à réutiliser pour le serveur en ligne |

Écartées sans tableau : une option de compilation qui retirerait le serveur des compilations de
développement (ce qu'on essaie ne serait plus ce qu'on livre) ; un autre cadre (actix-web, warp),
sans raison de quitter le choix du blueprint.

## Décision

Le serveur du réseau local est écrit avec axum dans une nouvelle crate `gesco-reseau`, qui dépend
de `gesco-services` ; `gesco-app` la démarre quand le poste est principal.

| Brique | Version | Sortie | Suite prévue |
|---|---|---|---|
| axum | 0.8.9 | 14 avril 2026 | Série 0.8 ; pas de date de fin publiée pour une crate 0.x ; revérifier à chaque version de GESCO |
| tokio | 1.53.2 (déjà là) | — | Ajouter la fonctionnalité `net` ; la 1.54.1 existe (21 octobre 2026), sans obligation de monter |

Vérifié le 10 novembre 2026 sur crates.io. Non vérifiable : une date de fin de support, que ces
crates ne publient pas.

Ce que la décision fixe :

- **Une seule route de commandes**, `POST /api/commandes/{nom}`, même entrée et même sortie JSON
  que la commande Tauri, mêmes clés d'erreur ; c'est la forme déjà déclarée dans
  `frontend/src/api/http.ts`. Une route d'état rend la version de GESCO, pour refuser un poste
  d'une autre version.
- **Le poste appelant accompagne chaque appel** jusqu'à `executer`, et signe l'audit et les
  événements. Son nom vient de l'appairage (ADR « sécurité du réseau local »), jamais d'un
  en-tête que le poste relié choisirait.
- **Éteint à la livraison.** Le serveur ne démarre que si le poste est déclaré principal et que
  la base est ouverte. À l'étape A1, il n'écoute que la machine elle-même.
- **Taille des corps bornée** (photos, fichiers d'import) ; la borne se fixe dans la spécification.
- `gesco-reseau` n'est pas dans ce qu'un essai de `gesco-services` compile.

## Justification

Ce qui s'essaie le plus souvent doit rester léger : la crate à part garde axum hors de
`gesco-services` et les essais du réseau hors de Tauri. Le même code servira deux fois : au poste
principal maintenant, au serveur en ligne et aux cours servis sur le Wi-Fi plus tard. axum
n'ajoute que cinq paquets à un verrou qui contient déjà hyper et tower. Écrire le protocole à la
main économiserait ces cinq paquets au prix de bogues que personne d'autre n'aura relus.

## Conséquences

Positives : la route est un adaptateur de quelques dizaines de lignes ; les essais existants de
services peuvent être rejoués par le réseau dans le même processus ; une école à un seul poste ne
voit rien changer.

Négatives : une crate et un exécutable de test de plus sur un disque juste ; la signature de
`executer` change (le poste appelant), donc `gesco-app` et les essais de frontière aussi ; le
blueprint (§6) disait « gestionnaires dans `gesco-services` » : à corriger, la frontière y reste
mais l'adaptateur réseau vit à côté.

## Contre-argument conservé

Pour une seule route, un cadre web entier est beaucoup : `hyper`, déjà là, suffirait. Il n'a pas
gagné parce que la route unique ne restera pas seule (état, appairage, puis les lots C et D), et
que le temps du porteur vaut plus que cinq paquets.

## Conditions de reconsidération

- axum cesse d'être maintenu, ou sa série suivante casse la compatibilité avec le tokio du projet.
- La crate à part coûte plus de disque ou de temps qu'elle n'en épargne, mesuré à l'étape A1.
- Le serveur en ligne (lot C) s'avère si différent qu'il ne réutilise rien de `gesco-reseau`.
