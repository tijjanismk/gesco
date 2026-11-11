# Orchestration de GESCO — tableau de bord

Tenu par la session « Orchestrateur GESCO ». Tout ce qu'il faut pour reprendre est ici : aucune
conversation passée n'est nécessaire. Dernière mise à jour : 11 novembre 2026.

## Règles de l'orchestrateur

1. **Une étape, une session neuve.** Chaque étape de code ou lot de leçons part d'une session
   neuve, avec un brief court qui renvoie aux fichiers. La session finit son étape, met son
   fichier d'état à jour et s'arrête. On ne prolonge jamais une session au-delà de son étape.
2. **Deux files au plus, une session par file** : le code (dépôt `gesco`) et les contenus
   (`C:\Users\USER\Documents\gesco-bibliotheque`). Elles ne touchent pas aux mêmes fichiers.
3. **Ouvrir les sessions sans demander** au porteur, et lui dire en une ligne ce qui est lancé.
   Les sessions rendent compte à l'orchestrateur, qui enchaîne sans poser de question au porteur,
   sauf pour un choix qui engage le produit ou ce que lui seul peut fournir.
4. **Modèle** : Fable 5.2. Effort moyen pour les leçons et pour l'orchestrateur ; effort élevé
   pour le code.
5. **Vérifier dans les fichiers, pas dans les conversations.** Une étape est finie quand
   `git log` et `ETAT_AVANCEMENT.md` (code) ou `ETAT.md` de la matière (leçons) le montrent. Ne
   jamais écrire à une ancienne session pour lui demander où elle en est.
6. **Archiver** chaque session dès que son étape est vérifiée.
7. **Garder son propre contexte petit** : ne lire que ce tableau, `git log`, les tableaux d'état.
   Ne pas lire le code ni les leçons. Quand un compte rendu dit seulement « en cours », répondre
   en une ligne sans rien lancer. Au-delà de 150 000 jetons de contexte, mettre ce tableau à
   jour, ouvrir un orchestrateur neuf et s'archiver.
8. **Commit et push** : une session de code commite et pousse seule quand tout est vert (fmt,
   clippy, essais, tsc, ESLint, vitest, knip, Playwright), seulement sur `developpement`. Jamais
   sur `1.2.x`, jamais l'étiquette `v1.2.7`, jamais de push forcé, jamais `docs/classeurs-notes/`
   ni `outils/classeurs-notes/`, jamais de noms réels (le dépôt est public). Les captures
   d'écran restent montrées au porteur.
9. **Disque** : environ 11 Go libres, `target` pèse 25 Go. Avant une étape de code, lancer
   `outils/elaguer-target.ps1` ; compiler avec `-j 4`.
10. **Limites d'usage** : avant de lancer une session, lire les limites. Au-dessus de 70 % de la
    fenêtre de 5 heures, ne rien lancer de neuf et le dire au porteur.
11. Répondre en français. Périmètre de la 2.0 entier : ne pas proposer de le réduire.

## État au 11 novembre 2026

### Code (branche `developpement`, à jour avec GitHub, dernier commit `5207478`)

- Version 1.2.7 figée : branche `1.2.x`, étiquette `v1.2.7`, commit `c0e2a6a`, installeur dans
  `Documents\GESCO-1.2.7`. On n'y touche plus.
- Phase 11, lot A (`SPEC_PHASE_11_A.md`, tableau des étapes dans `ETAT_AVANCEMENT.md`) :
  étapes 1 à 3 faites ; étape 4 faite pour sa première partie.
- Phase 9 : décisions du 11 novembre dans `ETAT_AVANCEMENT.md` ; elle vient après le lot A,
  spécification d'abord.
- Phase 12 : plan dans `docs/plan-phase-12.md`, encore hors du dépôt (non suivi).
- Hors du dépôt, à ne jamais commiter : `docs/classeurs-notes/`, `outils/classeurs-notes/`.

### Contenus (`gesco-bibliotheque\contenus\9e-annee`, mode d'emploi dans `LISEZMOI.md`)

| Matière | Fait | Reste | Fichier d'état |
|---|---|---|---|
| Biologie | 22 leçons sur 22 | rien | `biologie\ETAT.md` |
| Physique-chimie | 12 sur 26 | P-13 et P-14 coupées en rédaction, P-15 à P-17, C-01 à C-09 | `physique-chimie\ETAT.md` |
| Mathématiques | 17 fichiers sur 31 prévues | une leçon de géométrie coupée en rédaction, puis la suite | aucun : découpage dans `LISEZMOI.md`, « Où en est la rédaction » |

Aucune leçon n'a été relue par un enseignant ; chaque leçon le dit.

## File du code (dans l'ordre)

1. Étape 4, seconde partie : les cinq lenteurs (P-19, P-21, P-2, C-8, P-4), la mesure à dix
   postes simulés, et les reprises listées dans `ETAT_AVANCEMENT.md` (« Reste pour la seconde
   partie de l'étape 4 »).
2. Étape 5 : saisie des notes par les maîtres, éteinte à la livraison.
3. Étape 6 : livraison (pare-feu, installeur, guide, essai sur deux machines). Attend que le
   porteur fournisse deux ordinateurs Windows.
4. Lot B : livret du jardin.
5. Phase 9 : spécification, puis étapes.
6. Phase 12 dans l'application, selon `docs/plan-phase-12.md`.

## File des contenus (dans l'ordre, lots de cinq ou six leçons)

1. Physique : P-13 à P-17 (regarder d'abord si les fichiers de P-13 et P-14 existent).
2. Chimie : C-01 à C-05.
3. Chimie : C-06 à C-09.
4. Mathématiques : écrire `mathematiques\ETAT.md` d'après `LISEZMOI.md` et les fichiers présents,
   puis un premier lot de cinq leçons.
5. Mathématiques : lots suivants jusqu'à 31.

## Ce que seul le porteur peut fournir

- Deux ordinateurs Windows pour l'essai de l'étape 6.
- Le questionnaire CPS du fondamental (phase 9), les prix.
- Les réponses aux questions de terrain en attente dans `docs/plan-phase-12.md`.
- Un enseignant pour relire les leçons.

## Journal

- 2026-11-11 : tableau créé. Dix anciennes sessions archivées. Rien en cours.
