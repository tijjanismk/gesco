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

### Code (branche `developpement`, à jour avec GitHub, dernier commit de code `eb7f598`)

- Version 1.2.7 figée : branche `1.2.x`, étiquette `v1.2.7`, commit `c0e2a6a`, installeur dans
  `Documents\GESCO-1.2.7`. On n'y touche plus.
- Phase 11, lot A (`SPEC_PHASE_11_A.md`, tableau des étapes dans `ETAT_AVANCEMENT.md`) :
  étapes 1 à 4 faites (étape 4 : commits `0a9f3c4` et `eb7f598`, tout vert, 96,6 % des
  commandes sous une seconde à dix postes simulés). L'étape 4 reste à valider par le porteur,
  qui doit confirmer trois choix : la cible de la mesure (95 % sous une seconde à une commande
  par seconde et par poste), la file d'attente des écritures (`gesco-db`), la connexion par
  commande gardée telle quelle.
- Suites à donner de l'étape 4 (dans `ETAT_AVANCEMENT.md`) : avertissement « Aucune année
  scolaire n'est ouverte » resté sur un poste relié ; versement lent à dix postes (1,7 s au
  95e centile) ; deux essais fragiles (`gesco-reseau`, SMS d'essai Playwright).
- Disque : `target/release` pèse 10 Go ; élaguer avec `-Release` avant l'étape 5.
- Phase 9 : décisions du 11 novembre dans `ETAT_AVANCEMENT.md` ; elle vient après le lot A,
  spécification d'abord.
- Phase 12 : plan dans `docs/plan-phase-12.md`, encore hors du dépôt (non suivi).
- Hors du dépôt, à ne jamais commiter : `docs/classeurs-notes/`, `outils/classeurs-notes/`.

### Contenus (`gesco-bibliotheque\contenus\9e-annee`, mode d'emploi dans `LISEZMOI.md`)

| Matière | Fait | Reste | Fichier d'état |
|---|---|---|---|
| Biologie | 22 leçons sur 22 | rien | `biologie\ETAT.md` |
| Physique-chimie | 22 sur 26 (physique finie ; chimie C-01 à C-05 faites) | C-06 à C-09 | `physique-chimie\ETAT.md` |
| Mathématiques | 17 fichiers sur 31 prévues | une leçon de géométrie coupée en rédaction, puis la suite | aucun : découpage dans `LISEZMOI.md`, « Où en est la rédaction » |

Aucune leçon n'a été relue par un enseignant ; chaque leçon le dit.

### Décisions du porteur à reporter par la prochaine session de code

- 2026-11-12 : étape 4 validée, avec ses trois choix (cible de la mesure, file d'attente des
  écritures, connexion par commande gardée). À écrire dans `ETAT_AVANCEMENT.md`.
- 2026-11-12 : des enseignants refusent parfois de remettre leur dossier à l'école.
  L'application n'en parle pas (ni mention, ni case « refus »), et un dossier absent ou
  incomplet ne bloque jamais rien : ni la création de la fiche, ni l'affectation, ni la paie,
  ni les rapports de la phase 9. À vérifier dans le personnel existant et à tenir dans la
  spécification de la phase 9 ; à écrire dans `ETAT_AVANCEMENT.md`.

- 2026-11-12, même principe (« ne pas bloquer, ne pas le mentionner ») : les frais impayés ne
  bloquent jamais l'inscription ; une école qui ne se sert pas de la paie de l'application
  (certaines ne lui feront pas confiance) n'est bloquée nulle part, et rien ne le lui fait
  remarquer. À vérifier dans l'existant et à écrire dans `ETAT_AVANCEMENT.md`.

## File du code (dans l'ordre)

1. Étape 5 : saisie des notes par les maîtres, éteinte à la livraison ; avec, en reprise, les
   suites à donner de l'étape 4.
2. Étape 6 : livraison (pare-feu, installeur, guide, essai sur deux machines). Attend que le
   porteur fournisse deux ordinateurs Windows.
3. Lot B : livret du jardin.
4. Phase 9 : spécification, puis étapes.
5. Phase 12 dans l'application, selon `docs/plan-phase-12.md`.

## File des contenus (dans l'ordre, lots de cinq ou six leçons)

1. Chimie : C-06 à C-09.
2. Mathématiques : écrire `mathematiques\ETAT.md` d'après `LISEZMOI.md` et les fichiers présents,
   puis un premier lot de cinq leçons.
3. Mathématiques : lots suivants jusqu'à 31.

## Ce que seul le porteur peut fournir

- Deux ordinateurs Windows pour l'essai de l'étape 6.
- Le questionnaire CPS du fondamental (phase 9), les prix.
- Les réponses aux questions de terrain en attente dans `docs/plan-phase-12.md`.
- Un enseignant pour relire les leçons.

## Journal

- 2026-11-11 : tableau créé. Dix anciennes sessions archivées. Rien en cours.
- 2026-11-11 : étape 4, seconde partie, faite et poussée (`0a9f3c4`, `eb7f598`) ; physique P-13 à
  P-17 faite (17 sur 26). Les deux sessions ont coûté environ 50 % de la fenêtre de 5 heures.
- 2026-11-12 : le porteur valide l'étape 4 et ses trois choix ; décision sur les dossiers des
  enseignants. Étape 5 et chimie C-01 à C-05 lancées.
- 2026-11-12 : chimie C-01 à C-05 faite (22 sur 26). Chimie C-06 à C-09 lancée ; étape 5 en cours.
