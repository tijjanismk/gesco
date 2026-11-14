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
10. **Limites d'usage** : ne plus les regarder avant de lancer (décision du porteur du
    12 novembre 2026) ; si une limite arrête les sessions, on attend sa remise à zéro et on les relance.
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

- 2026-11-12, décisions du porteur pour la phase 12 (e-learning), à inscrire dans
  `docs/plan-phase-12.md` et dans la spécification de la phase 12 : les leçons peuvent recevoir
  des vidéos. Elles sont ajoutées par l'école (fichiers locaux, hors de la base, lues sans
  Internet et depuis les postes reliés) ; elles ne font jamais partie de la sauvegarde ;
  GESCO ne fournit aucune vidéo pour le moment ; l'application et sa documentation donnent
  des guides pour que l'école fasse et ajoute ses vidéos et que les maîtres s'en servent.
  Précisions acceptées par le porteur le même jour : (1) un bouton pour copier les vidéos
  vers une clé et les reprendre, pour changer d'ordinateur, annoncé à l'ajout de la première
  vidéo ; (2) une vidéo absente ne bloque rien, la leçon s'ouvre sans elle ; (3) un seul
  format accepté (MP4), avec un avertissement sans refus au-delà d'une taille conseillée ;
  (4) deux guides courts dans l'application : filmer une leçon avec un téléphone, ajouter la
  vidéo à une leçon et la montrer en classe.

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
2. 8e, mathématiques : 10 sur 44 ; suivant : A-11 à A-15, à lancer après la remise à zéro de la semaine (13 novembre, 9 h UTC).
3. 8e, physique-chimie : 5 sur 23 (P-01 à P-05) ; la suite (P-06 et après) est confiée à
   l'autre compte du porteur (12 novembre 2026). Ne rien y lancer d'ici.
4. 8e, biologie : 10 sur 15 ; suivant : leçons 11 à 15 (géologie), à lancer après la remise à zéro de la semaine.
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
- 2026-11-12 : physique-chimie de 8e confiée à l'autre compte, comme la terminale.
- 2026-11-12 : 8e, biologie 10 sur 15 et mathématiques 10 sur 44 ; sessions archivées. Étape 6
  arrêtée par une coupure d'Internet, message de reprise mis en file. Semaine Fable à 87 % :
  rien de neuf avant le 13 novembre à 9 h UTC, le reste va à l'étape 6.
- 2026-11-12 (soir) : le porteur arrête la 8e sur ce compte (10 sur 44 en mathématiques, 10 sur 15
  en biologie, 5 sur 23 en physique-chimie, en pause) ; l'autre compte ne fait que la terminale.
  Il demande ensuite les autres matières de 9e (français, histoire-géographie, anglais, ECM…)
  et « les premiers » (à lui faire préciser). Étape 6 : deux coupures d'Internet, relancée
  deux fois, pas encore commitée. `gesco-bibliotheque/contenus/REPARTITION.md` dit qui fait quoi.
- 2026-11-12 (soir), correction : l'autre compte se charge du lycée et de la 8e année ; ce
  compte-ci des autres matières de 9e. Rien de ce qui est écrit n'est effacé ; la fusion des
  deux travaux se décidera plus tard avec le porteur.
- 2026-11-12 (soir) : « les premiers » = la 7e année, matières scientifiques, pour ce compte-ci,
  avec les autres matières de 9e. À lancer après la remise à zéro de la semaine (13 novembre,
  9 h UTC) : 7e mathématiques, physique-chimie, biologie (découpage et lot 1 chacune).
- 2026-11-12 (soir) : le porteur demande de continuer sur Claude Opus quand la semaine Fable est
  épuisée. Lancées sur Opus 5.5, effort très élevé : 7e mathématiques
  (`local_1490ba64-36a0-4369-a1e4-bbea9f50595f`), 7e physique-chimie
  (`local_a4103519-9f2a-407c-8e2e-ec4bf16dbdd6`), 7e biologie
  (`local_feaa0bbc-c3b6-4f99-bddb-9ea63bb145f3`) ; découpage et lot 1 chacune. À suivre : autres
  matières de 9e (français, histoire-géographie, anglais, ECM).
- 2026-11-12 (nuit) : 7e mathématiques, lot 1 fait (5 sur 32, reprendre à A-06). Lancées : 9e
  français (`local_fb302456-5e7e-472b-97f3-80d44aa9a599`) et 9e histoire-géographie
  (`local_fd5bed0a-0775-4393-8018-05abdf4cf78c`), celle-ci sans la Révolution française (décision
  du porteur). Anglais et ECM de 9e à lancer ensuite. Pannes d'Internet répétées : étape 6, 7e
  physique-chimie, 7e biologie et 9e français arrêtées par des erreurs de connexion, messages de
  reprise envoyés. L'application ne laisse tourner que cinq sessions à la fois. Commits du
  tableau non poussés tant que GitHub ne répond pas.
- 2026-11-12 (nuit) : étape 6 faite et poussée (`017b1d3`), version 2.0.0, installeur d'essai dans
  `Documents\GESCO-2.0.0-essai`. Lot A de la phase 11 fini côté code. Restent au porteur : la
  règle du pare-feu posée pour de vrai, l'essai sur deux machines, sept choix à confirmer (dans
  `ETAT_AVANCEMENT.md`, « Choix de l'étape 6 »), et les quatre choix de l'étape 5.
  Suite du code : lot B (livret du jardin), puis phase 9, étape 1 ; avant, une étape « ne pas
  bloquer » (dossier d'enseignant, frais éteignables, paie) est à spécifier.
- Leçons, lots 1 faits : 7e biologie 5 sur 15, 7e physique-chimie 5 sur 28, 7e mathématiques 5
  sur 32, 9e français 5 sur 46 (leçons trop longues, 331 à 412 lignes ; découpage à confirmer).
  9e histoire-géographie : 2 sur 20 seulement (G-01, G-02) ; trois leçons sur la Révolution
  française écrites puis supprimées sur décision du porteur. Sessions archivées. À lancer au
  prochain message du porteur : lots 2 de ces cinq matières, anglais et ECM de 9e.
- 2026-11-12 (nuit) : semaine Fable à 96 % ; tout passe sur Opus 5.5 (élevé pour le code,
  très élevé pour les leçons). Lancées : spécification du lot B, `SPEC_PHASE_11_B.md`
  (`local_7713524b-4f6e-4076-811e-66a142c6129c`) ; 9e histoire-géographie lot suivant
  (`local_8b7b8254-c570-436d-bfa5-d4ca0b13002b`) ; 9e français 6 à 10
  (`local_a91abf7f-f6db-4e42-aa90-c3394ec24fac`) ; 7e mathématiques A-06 à A-10
  (`local_3ded24e5-b56e-41ac-8863-373b7b35a63a`) ; 7e physique-chimie P-06 à P-10
  (`local_94df1157-2125-4e02-9695-0dd5a5151b60`). En attente d'une place : 7e biologie 6 à
  10, anglais et ECM de 9e.
- 2026-11-12, 22 h : spécification du lot B faite (`fe450be`, quatre étapes), session archivée.
  Questions T-109 à T-115 posées au porteur ; l'étape 1 du lot B les attend.
- L'autre compte travaille dans une copie de la bibliothèque,
  `Documents\gesco-bibliotheque\gesco-bibliotheque` (mathématiques de TSE à 15 sur 38, physique
  de 8e P-06 à P-10, physique-chimie de terminale en découpage). Rien n'en est encore dans la
  bibliothèque ; la fusion se décidera avec le porteur. Son estimation : `estimation.txt` dans
  cette copie.
- 2026-11-12, 22 h 05 : le porteur valide T-109 à T-115 (recommandations) et le travail de nuit.
  7e physique-chimie 10 sur 28 et 9e français 10 sur 46 faits, sessions archivées. Lancées sur
  Opus : lot B étape 1 (`local_2003de89-17c8-40dc-a198-0dcb95a46dce`), 7e physique-chimie P-11 à
  P-15 (`local_88c37389-53bd-458c-97c5-7e6fab916b3a`), 9e français 11 à 15
  (`local_5609fa5e-e0ce-414b-a62f-0edc35be9159`). En cours : 9e histoire-géographie, 7e
  mathématiques A-06 à A-10.
- Nuit du 12 au 13 seulement (exception à la règle 1, accord du porteur) : les sessions de leçons
  qui finissent ne sont pas archivées ; à la remise à zéro de la fenêtre (1 h 20), l'orchestrateur
  leur écrit « lot suivant » (l'application refuse de démarrer une session sans message du
  porteur). À 9 h, retour à la règle : sessions neuves, Fable effort moyen pour les leçons.
- 2026-11-12, 22 h 15 : 9e histoire-géographie 7 sur 20 et 7e mathématiques 10 sur 32 faits ;
  leurs sessions gardées pour la nuit (`local_8b7b8254-c570-436d-bfa5-d4ca0b13002b`,
  `local_3ded24e5-b56e-41ac-8863-373b7b35a63a`). Lancées : 7e biologie 6 à 10
  (`local_ee5810b6-50b5-4e33-ad28-ecbcb9fae3f1`), 9e anglais découpage et lot 1
  (`local_d59bae7a-5ddf-410f-88af-ee280360e48d`). Reste à ouvrir : ECM de 9e.
- 2026-11-12, 22 h 30 : le porteur choisit LaTeX pour les formules des leçons, affichées par KaTeX
  dans l'application (phase 12 : dépendance à ajouter avec un ADR). Règles changées dans
  `LISEZMOI.md`, `outils/consigne-lecon.md` et `DECISIONS.md` de la bibliothèque et de la copie de
  l'autre compte ; notations maliennes confirmées (x, y ; (uₙ) ; k ∈ ℤ ; paramètre m ; a ≡ b [n]).
  Les leçons déjà écrites sont en texte simple : chaque session de leçons convertit d'abord sa
  matière avant son lot suivant ; matières finies (9e sciences, TSE de l'autre compte) : sessions
  de conversion à prévoir. 9e français 15 sur 46 fait.
- 2026-11-12, 22 h 45 : 9e anglais 5 sur 41 (découpage à confirmer) et 7e biologie 10 sur 15 faits.
  Fenêtre de 5 heures à 71 % : rien de neuf avant 1 h 20 (règle 10). À 1 h 20, relancer par
  message, en commençant par la conversion LaTeX de la matière s'il y a des formules : 7e
  mathématiques, 7e physique-chimie, 7e biologie (11 à 15), 9e français, 9e histoire-géographie,
  9e anglais. Le lot B, étape 1, tourne toujours.
- 2026-11-12, 22 h 50 : lancées sur Opus : 7e mathématiques (LaTeX puis lot,
  `local_f74990e8-7f88-4471-8e3b-c162caebb2fb`), 7e physique-chimie (LaTeX puis lot,
  `local_7d388cff-eda8-4f8e-9fb4-f3b1a6d71de8`), 7e biologie 11 à 15
  (`local_e5d7dfb2-a7a1-4f9b-8adc-586c2b0162dd`), 9e français 16 à 20
  (`local_99de4515-56b0-4388-86f3-bdf3b2f3fdc6`) ; relancées par message : 9e histoire-géographie
  et 9e anglais. Lot B, étape 1, toujours en cours.
- 2026-11-13, 1 h 40 : lot B étape 1 faite (`633be9a`) ; six choix à confirmer (dans
  `ETAT_AVANCEMENT.md`), dont l'ADR `docs/adr/2026-11-12-competences-du-jardin.md`. 9e ECM 5 sur 22.
  Décision du porteur : seules les formes géométriques sont dessinées, en SVG par un `.fig.py`
  (règle dans les deux `DECISIONS.md`). Le porteur a refusé l'archivage des sessions finies : ne
  plus archiver sans son accord. Lancées : lot B étape 2 (`local_f6f8d8f3-0fc5-4742-8ae3-c5d51c4d0494`),
  7e mathématiques avec figures (`local_3a73bf9f-c510-4e32-8502-d6591d9fb0bc`), 7e physique-chimie
  jusqu'à 28 (`local_0030694b-b0cb-464e-b759-32611741377c`) ; relancées par message : 9e ECM 06 à
  10, 9e histoire-géographie à partir de G-05. En cours : 9e français 23 à 25, 9e anglais 18 à 20.
- 2026-11-13 : le porteur valide les six choix de l'étape 1 du lot B (transmis à la session de
  l'étape 2). Changement de classe en cours d'année, y compris vers un niveau inférieur, jamais
  nommé « redoublement » ni « rétrogradation » : la facture reste telle quelle, la direction peut
  l'ajuster ; les notes restent dans l'historique, le bulletin de la nouvelle classe ne compte que
  les siennes. Session proposée en carte (cinq sessions tournent déjà) ; phase 1 en lecture seule, phase 2 après
  le commit de l'étape 2 du lot B. 9e français et anglais relancés par message.
- 2026-11-13, précision du porteur : au changement de classe, la direction choisit de garder les
  frais actuels (par défaut) ou d'appliquer ceux de la nouvelle classe, écart montré avant. Le
  chiffre d'affaires ne doit pas être faussé : ligne d'ajustement datée et tracée, jamais de
  facture réécrite, paiements inchangés, trop-perçu en avoir. Transmis à la session du changement
  de classe (`local_6cd59c7c-46f0-4b13-bf99-ee7e8a76db88`, démarrée par le porteur).
- 2026-11-13 : toutes les sessions arrêtées par une erreur 403 (Cloudflare). Décision du porteur :
  les sessions de leçons passent sur Sonnet 5.5, le code de la phase 11 reste sur Opus 5.5. Six
  sessions de leçons basculées et relancées, lot B étape 2 relancé. Images à chercher : règle
  dans `DECISIONS.md`, liste en cours dans `contenus/IMAGES-A-CHERCHER.md` (sous-agent).
- 2026-11-13 : finies, 7e physique-chimie (28 sur 28) et 9e histoire-géographie (20 sur 20).
  7e mathématiques 20 sur 32 (41 figures SVG), 9e français 35 sur 46, anglais 30 sur 41, ECM
  15 sur 22 : lots suivants relancés par message. Liste des images : 399, dont 102 en priorité
  haute, dans `contenus/IMAGES-A-CHERCHER.md`. Lot B étape 2 : tour fini sans commit, relancée.
- 2026-11-13, 13 h : fusion de la sortie de l'autre compte (`gesco-bibliotheque/2`) dans la
  bibliothèque : 32 fichiers ajoutés (8e physique P-06 à P-10, TSE 10 leçons, physique-chimie de
  terminale : découpage et consigne), 3 remplacés (anciens gardés), nos règles gardées ;
  versions écartées dans `contenus/_fusion-2026-11-13`. Sauvegarde avant fusion dans le
  dossier temporaire de l'orchestrateur. 25 vérifications ajoutées : toutes justes. Les
  leçons de TSE sont en texte simple (conversion LaTeX à faire). Physique-chimie de terminale :
  découpage de 59 leçons qui attend l'accord du porteur.
- 2026-11-13 : 9e anglais fini (41 sur 41), 7e mathématiques 25 sur 32 (dernier lot lancé).
  Le porteur n'utilise plus l'autre compte : toutes ses matières reviennent ici (`REPARTITION.md`).
  Lancées sur Sonnet, effort élevé : TSE mathématiques, LaTeX puis GE-04 à
  (`local_1fcdb84c-00f7-4a5d-8a37-c4fe4d65d3c7`) ; 8e mathématiques, LaTeX puis A-11
  (`local_a4cc7d1e-71f2-4293-a145-80082e3b37ed`). À suivre : 8e biologie 11 à 15, 8e
  physique-chimie, terminale physique-chimie (découpage à faire valider), terminale biologie.
- 2026-11-13 : lot B étape 2 faite (`bccebbe`), six choix à confirmer ; changement de classe passé
  en phase 2. 9e français fini (46 sur 46) : toute la 9e est rédigée (208 leçons). Règle du
  porteur : rédiger toute matière qui a un programme dans la bibliothèque, même non confirmé ;
  laisser les autres (dans `DECISIONS.md`). Terminale physique-chimie lancée
  (`local_21baffd7-608a-48ca-bcf7-55cd09a46658`). File des leçons : 8e biologie et
  physique-chimie, terminale SVT, philosophie, français, histoire, géographie, ECM, langues
  et autres matières du squelette ; 9e dessin, EPS, économie familiale. Code ensuite : lot B
  étape 3, après le commit du changement de classe.
- 2026-11-13 : 7e mathématiques finies (32 sur 32, 121 figures SVG) : toute la 7e scientifique est
  rédigée. Le porteur a passé la physique-chimie de terminale sur Opus 5.5. Lancée : 8e biologie
  11 à 15 (Sonnet).
- 2026-11-13 : 8e biologie finie (15 sur 15) ; 8e mathématiques 15 sur 44. Décision du porteur :
  matières scientifiques sur Opus 5.5, les autres sur Sonnet. TSE et 8e mathématiques passées
  sur Opus ; 8e mathématiques relancée (A-16 à A-20) ; 8e physique-chimie lancée sur Opus.
- 2026-11-13 : changement de classe fait (`2c4595e`, tout vert), sept choix à confirmer. Leçons :
  TSE 25 sur 38, terminale physique-chimie 15 sur 59, 8e mathématiques 25 sur 44, 8e
  physique-chimie 15 ; lots suivants relancés par message. Lot B étape 3 confiée par message à
  la session de l'étape 2 (`local_f6f8d8f3-0fc5-4742-8ae3-c5d51c4d0494`). Disque : 18 Go libres
  après suppression par le porteur des cours ivoiriens (10_CI_Lycee) et des fichiers d'essai.
- 2026-11-13 : choix du changement de classe validés ; 5 et 6 changés : le plan de paiement peut
  encore changer après un ajustement (recalcul, jamais de refus) ; la moyenne annuelle ne compte
  que les périodes de la nouvelle classe. Transmis à `local_6cd59c7c-46f0-4b13-bf99-ee7e8a76db88`,
  qui code après le commit de l'étape 3 du lot B (à lui réécrire alors).
- 2026-11-14 : décision du porteur : la rédaction s'arrête quand les quatre lots en cours finissent
  (TSE, terminale physique-chimie, 8e mathématiques, 8e physique-chimie). Ne plus relancer de
  lot ni ouvrir de matière sans un mot de lui. Le code continue (lot B, changement de classe).
