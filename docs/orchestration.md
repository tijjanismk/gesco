# Orchestration de GESCO — tableau de bord

Tenu par la session « Orchestrateur GESCO ». Tout ce qu'il faut pour reprendre est ici : aucune
conversation passée n'est nécessaire. Dernière mise à jour : 11 novembre 2026.

## Règles de l'orchestrateur

1. **Une étape, une session neuve.** Chaque étape de code ou lot de leçons part d'une session
   neuve, avec un brief court qui renvoie aux fichiers. La session finit son étape, met son
   fichier d'état à jour et s'arrête. On ne prolonge jamais une session au-delà de son étape.
2. **Aller vite, en parallèle** (décision du porteur du 12 novembre 2026, qui remplace « deux
   files au plus ») : lancer ensemble tout ce qui ne touche pas aux mêmes fichiers. Bornes :
   une seule session qui compile à la fois (même checkout, disque presque plein) ; quatre
   sessions au plus, car deux consomment déjà la moitié d'une fenêtre de 5 heures.
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
- Phase 9 : `SPEC_PHASE_9.md` v1.0 commitée (`cf17dca`), huit étapes d'une session. Onze
  questions au porteur, T-98 à T-108, chacune avec une recommandation, posées le 12 novembre ;
  seule T-99 est nécessaire, avant l'étape 6 de la phase 9. Trois PDF scannés et la fin du
  rapport de fermeture n'ont pas pu être lus (tableaux à vérifier à l'étape 7 de la phase 9).
- Phase 12 : plan dans `docs/plan-phase-12.md`, encore hors du dépôt (non suivi).
- Hors du dépôt, à ne jamais commiter : `docs/classeurs-notes/`, `outils/classeurs-notes/`.

### Contenus (`gesco-bibliotheque\contenus\9e-annee`, mode d'emploi dans `LISEZMOI.md`)

| Matière | Fait | Reste | Fichier d'état |
|---|---|---|---|
| Biologie | 22 leçons sur 22 | rien | `biologie\ETAT.md` |
| Physique-chimie | 26 leçons sur 26 | rien | `physique-chimie\ETAT.md` |
| Mathématiques | 31 leçons sur 31 (paragraphe « Écart à lever » de `ETAT.md` à relire) | rien | `mathematiques/ETAT.md` |

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

- 2026-11-12, précision du porteur : « la paie » voulait dire aussi le paiement des frais de
  l'année. Une école qui n'enregistre pas les frais de scolarité dans l'application (ni
  factures suivies, ni encaissements) n'est bloquée nulle part — inscription, bulletins,
  passage, rapports — et rien ne le lui fait remarquer (pas d'alerte d'impayés imposée).

- 2026-11-12, réponse du porteur sur les frais non suivis : « on suit notre logique depuis le
  début ». Compris ainsi : comme la cantine ou la paie, le suivi des frais est un choix de
  l'école en réglage (module qu'elle allume ou éteint) ; éteint, il quitte les écrans et le
  tableau de bord, ses données restent, et aucune facture ne se crée à l'inscription.

- 2026-11-12 : le porteur valide toutes les recommandations de T-98 à T-108
  (`SPEC_PHASE_9.md`), avec une réserve sur T-103 : l'inventaire n'est pas forcément tenu par
  le secrétaire — toute personne que l'école en charge (droit à part, pas lié au rôle). À
  reporter dans `SPEC_PHASE_9.md` (v1.1) et `ETAT_AVANCEMENT.md` par la première session de la
  phase 9.

## File du code (dans l'ordre)

1. Étape 5 : faite (`160bc4f`, tout vert). Quatre choix à confirmer par le porteur : compte de
   maître = seul rôle Enseignant ; liste sans discipline ni finances ; session fermée à
   l'extinction ; réglage allumé si un maître a déjà saisi.
2. Étape 6 (en cours, `local_01afc8c1-06f6-45dc-9206-fd73d93a32e6`) : livraison (pare-feu, installeur, guide). Le porteur n'a qu'un ordinateur
   (2026-11-12) : tout faire et vérifier sur une seule machine, écrire la marche à suivre de
   l'essai sur deux machines dans les essais à la main, et laisser cet essai au porteur, qui
   vérifiera après. Ne pas attendre.
3. Lot B : livret du jardin.
4. Phase 9 : étapes 1 à 8 de `SPEC_PHASE_9.md`.
5. Phase 12 dans l'application, selon `docs/plan-phase-12.md`.

## File des contenus (dans l'ordre, lots de cinq ou six leçons)

Décision du porteur du 12 novembre 2026 : après la 9e, la 8e année, puis les autres classes,
vite. Une session par matière, en parallèle ; chaque matière de 8e a son `ETAT.md` dans
`contenus\8e-annee\<matière>` (découpage à confirmer sur le programme officiel s'il manque).

1. 9e année : entièrement rédigée (biologie 22, physique-chimie 26, mathématiques 31).
2. 8e, mathématiques : 5 sur 44 ; A-06 à A-10 en cours (`local_9f1d8743-cc93-4236-857b-99cd43fce5e8`), puis la suite par lots de cinq.
3. 8e, physique-chimie : 5 sur 23 (P-01 à P-05) ; lot 2 (P-06 à P-10) à lancer dès qu'une place se libère.
4. 8e, biologie : 5 sur 15 ; leçons 6 à 10 en cours (`local_2cd5374a-2e07-451b-9ba1-0f44ea5eed9d`), puis 11 à 15.
5. Terminale : confiée à l'autre compte du porteur (décision du 12 novembre 2026), qui
   travaille dans `gesco-bibliotheque\contenus	erminale` avec le prompt donné par le relais.
   Ce compte-ci n'écrit plus de leçons de terminale. État laissé : mathématiques de TSE 5 sur
   38, physique-chimie et biologie pas commencées.
6. 7e année, mêmes matières, puis les autres classes.

## Ce que l'usage a appris (12 novembre 2026)

- L'application ne laisse l'orchestrateur démarrer une session que dans le tour qui suit un
  message du porteur. Sur un compte rendu de session ou un réveil programmé, le démarrage est
  refusé : déposer la carte (`own_initiative`) et la démarrer par son `proposal_id` au prochain
  message du porteur (« on continue » suffit).
- Les sessions de leçons démarrent dans le dépôt `gesco` : leur brief donne les chemins absolus
  de `gesco-bibliotheque`.
- Une session de code peut rester à l'arrêt en attendant une commande d'arrière-plan qui ne la
  réveille pas (étape 5 : 1 h 45 perdue). Écrire dans le brief « ne termine pas ton tour en
  attendant une commande d'arrière-plan » ; si `list_sessions` la montre arrêtée sans commit
  depuis longtemps, lui écrire de reprendre.
- `ETAT_AVANCEMENT.md` a des lignes très longues : ne jamais le lire avec un `grep` large.
- Coût observé : une étape de code et un lot de cinq leçons en parallèle prennent environ 50 %
  d'une fenêtre de 5 heures.
- L'archivage passe au second appel, avec l'entrée que le premier refus renvoie.

## Sessions en cours au passage de relais (12 novembre 2026, vers 2 h 45)

- Étape 5 : session `local_372feff7-8ccd-4284-a67e-4596267608b8`, relancée à 2 h 35, pas encore
  commitée. Elle doit aussi reporter les décisions du porteur dans `ETAT_AVANCEMENT.md`. Finie
  quand `git log` montre son commit ; alors l'archiver et lancer l'étape 6.
- Mathématiques G-11 à G-15 : session lancée par l'ancien orchestrateur ; finie quand
  `mathematiques\ETAT.md` dit 25 sur 31.
- Ces deux sessions rendent compte à l'ancien orchestrateur, pas au nouveau : le nouveau vérifie
  dans les fichiers, ou s'abonne à leur fin (`SendMessage` avec `notify_when_idle`, sans message).

## Ce que seul le porteur peut fournir

- L'essai de l'étape 6 sur deux machines, quand il aura un second ordinateur (il vérifie après).
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
- 2026-11-12 : chimie C-06 à C-09 faite ; physique-chimie finie (26 sur 26). Mathématiques
  (ETAT.md et premier lot) lancées ; étape 5 en cours (Rust et Playwright restent à passer).
- 2026-11-12 : le porteur n'a qu'un ordinateur ; l'étape 6 se code sans attendre, il vérifie après.
- 2026-11-12 : mathématiques, `ETAT.md` écrit et lot 1 fait (20 sur 31) ; G-11 à G-15 lancées.
  Étape 5 relancée après 1 h 45 d'arrêt. Orchestrateur à 153 000 jetons : relais passé à un neuf.
- 2026-11-12 : le porteur veut aller très vite. Lancées en plus de l'étape 5 et de G-11 à G-15 :
  mathématiques G-16 à G-21 (`local_81da8b5d-2a01-48e0-82b0-37037f4fd296`) et la spécification
  de la phase 9, `SPEC_PHASE_9.md` (`local_75a1b1aa-2460-448f-837b-08f08e09d725`).
- 2026-11-12 : spécification de la phase 9 faite (`cf17dca`), session archivée ; T-98 à T-108
  posées au porteur.
- 2026-11-12 : mathématiques G-11 à G-15 faites, session archivée ; T-98 à T-108 validées.
- 2026-11-12 : 8e année lancée (mathématiques et physique-chimie) ; biologie de 8e à suivre.
- 2026-11-12 : étape 5 faite (`160bc4f`) ; mathématiques de 9e finies (31 sur 31) ; biologie de
  8e, lot 1 fait. Lancées : étape 6 et biologie de 8e, leçons 6 à 10. Sessions archivées.
- 2026-11-12 : 8e, lots 1 de mathématiques (5 sur 44) et de physique-chimie (5 sur 23) faits ;
  terminale ouverte (mathématiques TSE). Semaine Fable à 71 %, fenêtre de 5 heures à 52 %.
- 2026-11-12 : le porteur met la 8e année en pause. Sessions arrêtées en cours de tour, non
  archivées : biologie leçons 6 à 10 (`local_2cd5374a-2e07-451b-9ba1-0f44ea5eed9d`) et
  mathématiques A-06 à A-10 (`local_9f1d8743-cc93-4236-857b-99cd43fce5e8`) ; une leçon peut être
  restée à moitié écrite dans chacune. Ne rien lancer en 8e sans un mot du porteur. Continuent :
  étape 6 et mathématiques de TSE.
- 2026-11-12 : pause de la 8e levée par le porteur ; les deux sessions arrêtées reprennent
  (biologie 6 à 10, mathématiques A-06 à A-10). Terminale confiée à l'autre compte ; lot 1 des
  mathématiques de TSE fait (5 sur 38), session archivée. Semaine Fable à 79 %.
