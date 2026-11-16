# Portail des élèves : un second serveur dans `gesco-reseau`, avec son propre contrat, jamais `executer`

**Date** : 16 novembre 2026
**Statut** : proposé le 16 novembre 2026
**Précise** : décision 1 de `docs/plan-phase-12.md` §5 ; ADR « serveur du réseau local » et
« sécurité du réseau local » (10 novembre 2026) ; blueprint §16.3 (mode 1), §16.5
**Spécification** : `SPEC_PHASE_12_LOT_4.md`

## Contexte

Le lot 4 sert la bibliothèque aux téléphones et aux ordinateurs des élèves, sur le Wi-Fi de
l'école (T-87 : « tout cela à la fois »). C'est le poste principal qui sert : il détient aussi les
notes, la caisse et les données de tous les élèves.

- Aujourd'hui `gesco-reseau` écoute un seul port (47 800), en TLS, et ne répond qu'aux postes
  appairés : trois routes (`/api/etat`, `/api/appairage`, `/api/commandes/{nom}`). La route des
  commandes appelle `Application::executer`, qui connaît environ 330 commandes de gestion.
- Un téléphone n'est pas appairé et ne le sera jamais : n'importe quel appareil du Wi-Fi pourra
  joindre le portail (élèves, visiteurs, voisins qui ont le mot de passe).
- `gesco-services` a déjà une origine d'appel (`Origine::Locale`, `Origine::Relie(Poste)`) qui
  signe le journal d'audit.
- Ce que l'élève fait est court et connu d'avance : lire le catalogue de sa classe, lire une
  leçon visible, ouvrir une image ou un son, noter qu'il a lu, passer un quiz (lot 3), rendre un
  devoir (lot 5).
- Exigence E-13 : par le portail, aucune commande de gestion ne répond ; l'élève ne voit que sa
  classe ; on ne garde de lui que son matricule et son avancement.

## Options

| | 1. Second serveur dans `gesco-reseau`, port à part, contrat du portail | 2. Routes du portail sur le port des postes (47 800) | 3. Processus à part, qui lit une copie des contenus |
|---|---|---|---|
| Apprentissage | Faible : axum déjà là | Faible | Élevé : synchroniser une copie |
| Exploitation | Un port de plus, une règle de pare-feu de plus | Rien de plus | Un programme de plus à démarrer et à surveiller |
| Maturité | Même code que les postes reliés | Idem | À écrire |
| Pour un agent de code | Bon : essais sans Tauri, liste fermée de routes | Moyen : une erreur de routage ouvre la gestion | Moyen |
| Plus gros risque | Une route du portail qui appelle trop large | Un téléphone qui atteint `/api/commandes` | Une copie en retard sur ce que l'école a publié |

Écartées sans tableau : passer par `executer` avec une session « élève » et une liste de
commandes permises (une seule erreur de liste ouvre la gestion à tout le Wi-Fi) ; un serveur web
du système (IIS), qui demande un administrateur et une configuration par école.

## Décision

Le portail est un second serveur axum dans `gesco-reseau`, sur un port à part, avec ses propres
routes ; chaque route appelle une fonction d'un service `portail` de `gesco-services`, écrite pour
lui, sous une origine nouvelle `Origine::Portail` ; aucune route du portail n'atteint `executer`.

| Brique | Version | Sortie | Suite prévue |
|---|---|---|---|
| axum | 0.8.9 (déjà dans le dépôt) | 14 avril 2026 | Dernière stable sur crates.io le 16 novembre 2026 |
| tokio | 1.53.2 (déjà dans le dépôt) | — | 1.54.1 existe (21 octobre 2026), sans obligation de monter |

Vérifié le 16 novembre 2026 sur crates.io. Aucune dépendance nouvelle.

Ce que la décision fixe :

- **Deux écoutes, deux routeurs.** Le routeur des postes reste tel qu'il est. Le routeur du
  portail est construit à part et n'a que ses routes : pages (`/`, fichiers du frontend des
  élèves), `/eleve/api/…` (entrer, sortir, catalogue, leçon, fichier, avancement, puis quiz et
  devoirs). Un essai liste les routes du portail et échoue si l'une d'elles n'est pas dans la
  liste écrite.
- **Port** : 80 s'il est libre, pour que l'adresse à taper soit courte (`http://192.168.1.10`) ;
  sinon 47 801. Le port retenu est affiché dans les réglages et sur l'affiche de la classe.
  Windows ne demande pas de droit d'administrateur pour écouter le port 80 ; l'installeur ajoute
  la règle de pare-feu des deux ports, réseau privé seulement.
- **Éteint tant que la direction ne l'allume pas** ; ne démarre que sur le poste principal, base
  ouverte, module Bibliothèque allumé ; écoute les seules adresses du réseau privé, comme le
  serveur des postes.
- **Service `portail`** : fonctions propres, qui ne lisent que ce qu'un élève peut voir (versions
  visibles, non retirées, du niveau de sa classe, dans la portée d'un ajout de l'école) et
  n'écrivent que l'avancement, les tentatives de quiz et les rendus. Elles ne reçoivent jamais de
  jeton de session du personnel.
- **`Origine::Portail`** : le journal d'audit écrit « portail des élèves » comme poste ; jamais
  confondue avec un poste relié (`est_reliee` reste faux).
- **Ce qui ne passe jamais** : nom, date de naissance, notes, absences, paiements, responsables ;
  la réponse d'une leçon ne contient ni note pour le maître, ni liste de faits, ni brouillon.
- **Corps bornés** par route ; seules les routes des devoirs (lot 5) acceptent un fichier.

## Justification

La sécurité de la gestion ne doit pas tenir à une liste de permissions bien tenue : avec un
routeur à part, un téléphone ne peut pas joindre ce qui n'existe pas sur son port. Les fonctions
écrites pour le portail sont peu nombreuses et s'essaient une à une. Rester dans `gesco-reseau`
réutilise le serveur, l'arrêt propre et les essais du lot A. Un processus à part ajouterait une
synchronisation sans rien protéger de plus, puisqu'il lirait les mêmes données.

## Conséquences

Positives : le contrat de gestion ne change pas ; E-13 se vérifie par un essai simple ; le même
socle servira le portail des élèves en ligne, plus tard.

Négatives : un second port et sa règle de pare-feu (risque de livraison) ; des fonctions de
lecture écrites deux fois (bureau et portail) ; le port 80 peut être pris par un autre logiciel,
d'où le second port et une adresse plus longue.

## Contre-argument conservé

Passer par `executer` aurait donné au portail la garde, l'audit et la portée déjà écrites, sans
rien dupliquer. Il n'a pas gagné parce que le portail s'ouvre à tout appareil du Wi-Fi : la
moindre commande mal marquée y deviendrait publique.

## Conditions de reconsidération

- Le portail en ligne (édition Connectée) demande un contrat commun avec le portail local.
- Les fonctions du portail dépassent une vingtaine : regrouper ce qu'elles partagent avec le
  bureau.
- Le port 80 se révèle souvent pris dans les écoles (essais du porteur) : n'utiliser que 47 801.
